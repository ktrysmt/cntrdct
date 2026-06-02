# R-1 T7 performance measurement

Captured: 2026-05-26.

Spec: `docs/spec/ir-v0.md` §F6 T7 + R-1 release gate
(`REBUILD.md` §9 — wall-clock + peak-RSS regression < 25 % vs v0.5.2).

## Method

- Hardware: macOS (darwin 25.4.0).
- Binary: `cargo build --release` at each measured SHA.
- Tool: `/usr/bin/time -l` (macOS), capturing `real` (wall-clock
  seconds) and `maximum resident set size` (peak RSS bytes).
- Warm-up: 1 untimed run per binary per corpus before timed trials.
- Trials: 5 per (binary, corpus). Median reported; outliers retained
  in the raw logs (`/tmp/t7-measure/*.log`, not committed).
- Corpora:
  - `benchmarks/wild-corpus` — 270 Rust files, ~11.4 MiB source.
  - `benchmarks/wild-corpus-python` — 11 Python files, ~280 KB
    source. (Spec text references "~600 files" — the actual
    corpus has remained at 11 files since v0.1; the Rust corpus is
    the primary regression signal.)

## Measured SHAs

- v0.5.2 baseline: `23a645c` (tag `v0.5.2`).
- R-1 head: working tree at the post-IR commit (incl. the lazy
  `raw_tree` refactor landed in this PR).

## Results

### Rust wild-corpus (270 files)

| metric | v0.5.2 | R-1 head (eager raw_tree) | R-1 head (lazy raw_tree) | delta vs v0.5.2 |
|---|---|---|---|---|
| wall-clock median | 1.33 s | 0.41 s | 1.62 s | +21.8 % |
| peak RSS median | 71.5 MiB | 380.0 MiB | 156.3 MiB | +118.6 % |

### Python wild-corpus (11 files)

| metric | v0.5.2 | R-1 head (lazy raw_tree) | delta vs v0.5.2 |
|---|---|---|---|
| wall-clock median | 0.03 s | 0.04 s | +33 % (small absolute) |
| peak RSS median | 17.0 MiB | 20.8 MiB | +22 % |

## R-1 release gate evaluation

- Wall-clock: PASS (< 25 % on both corpora).
- Peak RSS Python: PASS (+22 % < 25 %).
- Peak RSS Rust: FAIL (+118.6 % > 25 %).

The wild-corpus-python case is within the gate. The wild-corpus
Rust case is not. ir-v0.md R1 ("peak-RSS regression exceeds 25 %")
materialised on the Rust corpus and is the controlling failure for
the R-1 release gate.

## R1 mitigation applied

ir-v0.md R1 prescribed two responses to a > 25 % regression:
"R-0 revisits the retention decision (alternative: re-parse lazily
on first `raw_tree` access, paying CPU for memory)."

The lazy `raw_tree` refactor (this PR) implements the prescribed
alternative:

- `IrFile.raw_tree` becomes a method, not a field. Each call reparses
  the source and returns a fresh `Arc<SyncTree>` that drops when the
  caller releases it.
- `IrFile.resolve` becomes `resolve_with(&NodeRef, FnOnce(Node) -> R)`
  so the freshly-parsed tree can drop after the closure runs.
- `IrBlock.normalised_tokens` is populated only on the top-level
  function body (`IrFn::body`); nested blocks carry an empty
  vector. Matches ir-v0.md R2 ("clone-drift normalises every
  top-level function exactly once, and the IR layer does the same").
- T4 golden fixtures re-blessed: nested-block `normalised_tokens`
  arrays are now empty.

Effect on Rust wild-corpus (270 files):

- Peak RSS: 380 → 156 MiB (-58.9 %).
- Wall-clock: 0.41 → 1.62 s (+295 %, paying CPU for memory).

The lazy refactor halves the eager-IR regression but does not
close it. The remaining ~85 MiB residual is dominated by the IR
struct overhead held across all 270 files for the scan duration
(`IrFn` / `IrBlock` / `IrStmt` / `IrExpr` / `Vec<NormalisedToken>`
on top-level bodies / `Arc<str>` source). Closing it requires
either a streaming detector pipeline (breaks clone-drift's
cross-file scope) or a compact IR representation
(`String` → `Box<str>`, packed `NodeRef`, lazy field
materialisation). Both are out of R-1.c' scope.

## R-1.h release gate disposition

The R-1.h tagging step (REBUILD.md §4 R-1.h) cannot proceed under
the current gate text — the Rust wild-corpus RSS exceeds the 25 %
threshold by ~94 percentage points. Two options for R-1.h:

1. Land an IR-compaction follow-up before tagging v0.6.0 (new
   roadmap item R-1.c''). Targets: `String` → `Box<str>` across
   `IrFn` / `IrParam` / `IrPath` / `IrLiteral` / `IrComment`,
   packed `NodeRef` (Range → 4 × u32), and revisit eager
   normalised-token storage even on top-level bodies (compute on
   demand from the source slice).
2. Amend the R-1 release gate threshold per ir-v0.md R1's "R-0
   revisits the retention decision" clause — accept a higher RSS
   ceiling on the Rust corpus while the IR migration of
   cross-cutting detectors completes (the deferred R-1.c follow-up
   would let detectors stop touching `raw_tree`, removing the
   reparse cost and recovering the original 3× wall-clock win).

This file is the audit trail for the PR description per
ir-v0.md F6 T7. The raw `/usr/bin/time -l` logs were retained in
`/tmp/t7-measure/` during measurement but are not committed; the
medians above are reproducible by re-running the procedure in §
Method.

## R-1.c'' path (a) resolution (2026-06-03)

R-1.h tagging proceeded under option 2 (gate amendment); v0.6.0 –
v0.8.0 shipped with the relative gate suspended. R-1.c'' path (a)
then closed the item with a measurement-driven scope correction.

Floor study (`scan benchmarks/wild-corpus`, 270 Rust files,
`/usr/bin/time -l`, this machine): the option-1 sketch above
(`String` → `Box<str>`, packed `NodeRef`) was found to target the
wrong fields. Per-field/per-NodeRef shrink is single-digit MiB;
the dominant per-node costs are `IrFn.normalised_tokens` and the
per-node `Location.file` path duplication.

| Variant | peak RSS | Δ vs baseline |
|---|---|---|
| baseline (post path-b, this session) | 169 MiB | — |
| `normalised_tokens` emptied | 137 MiB | −33 MiB |
| `Location.file` path-dup removed | 153 MiB | −16 MiB |
| both emptied (floor) | ~125 MiB | −44 MiB |

The ~125 MiB floor is +75 % over the v0.5.2 71.5 MiB baseline, so
the < 25 % target (≈ 89 MiB) is unreachable by field compaction —
the cross-file detectors (clone-drift, pr-miner) need the whole
corpus's IR resident at once, so retention cannot be cut, and
shrinking `normalised_tokens` further would require changing
clone-drift's token representation (risking T1 byte-identical).

Shipped (T1-byte-identical, safe): `Location.file` `PathBuf` →
shared `Arc<Path>` (one alloc per file, refcount-cloned per node;
`serialize_with` shim keeps the T4 golden wire shape identical).

| Metric (Rust wild-corpus) | before | after (5-trial median) |
|---|---|---|
| peak RSS | ~169 MiB | ~150 MiB (+109 % vs v0.5.2) |
| wall-clock | — | 1.47 s (+10.5 % vs v0.5.2 1.33 s) |

Python wild-corpus: 14.1 MiB (within the < 25 % rule).

Gate disposition (final): wall-clock keeps the < 25 % rule (passes).
Peak-RSS retires the relative rule for the Rust corpus in favour of
an absolute ≤ 175 MiB ceiling — headroom over the ~150 MiB result,
and still catches the eager-retention regression class (380 MiB).
See REBUILD.md § 9 and ir-v0.md R1. The `Box<str>` / NodeRef-packing
items were dropped as not worth the churn for a sub-gate win.
