# cntrdct rebuild plan

Last updated: 2026-05-25. Supersedes the retired `ROADMAP.md` and
the retired `REBUILD-handoff.md` (the latter's R-1 procedure and
verification rules are absorbed into this file below).

Current shipped version: v0.5.2 (Rust + Python, six detectors). This
file replaces the old Q-series / T-series / M-series roadmap with a
single forward-looking plan oriented around the v0.6.0 rebuild:
a language-agnostic IR layer plus a low-cost, single-repo PR
contribution path for new languages.

For historical detector / corpus / quality-audit milestones (P-1
through Q-16, T1-1 through T2-11, M-1 through M-6, T3-14 through
T3-16, T4-17 through T4-19), see `CHANGELOG.md` and the per-detector
specs under `docs/spec/`. They are not duplicated here.

## Status legend

- `[x]` completed
- `[~]` in progress
- `[ ]` pending
- `(carry-over)` — inherited from the retired `ROADMAP.md`,
  re-prioritised under the rebuild plan

REBUILD.md doubles as the task tracker for the rebuild. When a
phase or sub-step advances, update its `[ ]` / `[~]` / `[x]`
marker in place and append a one-line `Status:` entry on the same
sub-step recording the landing date plus the commit / PR (or
"uncommitted in working tree" when no commit exists yet) that did
the work. A session picking up the rebuild then sees where it
stopped without re-reading the diff. The R-1 release gate (§9)
reads from these markers; stale entries silently weaken the gate,
so do not skip updates.

## 0. Why rebuild

The retired `ROADMAP.md` accumulated three structural premises that
have since broken:

1. Q-15 "SOTA baseline comparators" assumed pinned Docker images of
   PyBugLab and SourcererCC could be wired against the corpus on
   release. The 2026-05-22 evidence audit (`gh api`, HuggingFace,
   DockerHub, fork survey) showed PyBugLab pre-trained weights are
   not distributed in any form, the upstream README explicitly says
   the infrastructure cannot be shared, and no academic alternative
   ships an installable arg-swap detector. The "external SOTA
   comparator" framing was structurally unrealisable.
2. The M-series serial language-extension plan scaled by editing
   every detector's `match Language::*` arms for each new language.
   The marginal cost per language was ~1500-1900 LOC of detector
   source plus per-(detector, language) literature surveys, which
   does not invite outside contribution at the rate the
   peer-reviewed-citation differentiator deserves.
3. The closed `Language` enum + compile-time tree-sitter linkage
   model assumed cntrdct was the only place new languages could
   land. There was no concept of a contribution path for a language
   the maintainers had not personally surveyed.

The rebuild keeps every shipping behaviour (P1-P5 constraints, the
four-layer architecture, the SARIF / eval / calibrate / scan /
cross-model-kappa CLI surface, the `network-isolation` CI gate) but
replaces (1)-(3) with a single architectural pivot: language-agnostic
IR + PR-driven per-language source under one repo.

## 1. Architecture goals

Goal G1. Language-agnostic IR

A new module `src/ir.rs` defines a normalised AST shape
(`IrCallSite`, `IrBlock`, `IrComment`, `IrExpr`, `IrPath`, etc.) that
every cross-cutting detector consumes. Per-language code lives only
in `src/parsers/<lang>.rs` which converts the language's tree-sitter
AST into IR. After landing, the marginal cost of adding a new
language drops from ~1500-1900 LOC across five files (one per
cross-cutting detector) to ~300-500 LOC in one file (the parser
converter).

Goal G2. Two-tier detector layout

```
src/detectors/
├── arg_swap.rs                  # cross-cutting, IR-based
├── clone_drift.rs
├── comment_code.rs
├── pr_miner/
├── unreachable_after_terminator.rs
└── lang/                        # language-specific
    ├── rust_config_interaction.rs
    ├── python_unreachable_except.rs
    └── ...
```

Cross-cutting detectors are written once on IR. Language-specific
detectors (Rust `#[cfg]` interaction, Python `except` reachability,
future Go build-tag interaction, etc.) live under `src/detectors/lang/`
and continue to read tree-sitter ASTs directly. The two-tier split
keeps the IR small (no concession to single-language AST quirks) and
keeps the language-specific detectors honest.

Goal G3. Single-repo contribution model

cntrdct stays a single crates.io artefact and a single GitHub repo.
There is no separate `cntrdct-plugins` repo, no out-of-process plugin
discovery, no WASM modules, no `.so` loading. Adding a new language
is a PR against this repo that adds:

- `src/parsers/<lang>.rs` (the IR converter)
- `Cargo.toml` dependency on `tree-sitter-<lang>`
- `benchmarks/wild-corpus-<lang>/` fixture set (≥ 8 positive / 3
  negative per cross-cutting detector that opts in)
- `CITATIONS.md` entries with `Languages:` lines (P1 + per-language
  SHOULD per `docs/spec/citations-policy.md`)
- `docs/surveys/<detector>-<lang>-<YYYY-MM>.md` per detector
  surveyed
- Optionally, `src/detectors/lang/<lang>_<id>.rs` for any
  language-specific detector the contributor wants to ship alongside

`CONTRIBUTING.md` documents this workflow as the canonical path.

Goal G4. P1-P5 preserved

The rebuild touches none of the design constraints:

- P1 (peer-reviewed citation per detector) — IR-based detectors keep
  their existing citation sets; new languages add `Languages:` lines
  and per-language surveys per existing policy.
- P3 (LLM only on Layer 3) — the IR layer is in Layer 1; no new
  socket-opening code paths. The `network-isolation` CI gate
  continues to cover `scan` / `calibrate` / `eval`.
- P4 (priors from labelled corpora) — `cntrdct calibrate` continues
  to feed `benchmarks/priors-default.json`. Per-language priors are
  re-fitted after detectors migrate to IR (a Layer 2 reconciliation
  task inside R-1).
- P5 (severities map to IEEE 1044-2009) — SARIF emission is
  unchanged. IR carries no severity; severity stays detector-defined.

## 2. Design constraints (carried from CLAUDE.md, unchanged)

- P1 / P3 / P4 / P5: as written in `CLAUDE.md` "Design constraints".
- Boundary contract between root `cntrdct` and `research/`: as
  written in `CLAUDE.md` "Boundary contract (do not violate)".
  Research promotions remain explicit `promote(<area>): ...` commits.
- Citation policy: as written in `docs/spec/citations-policy.md`.
  Per-language survey requirement applies to every new language
  added under G3.
- Release procedure: as written in `CLAUDE.md` "Release procedure".
  Annotated tag + version match + `tests/fixtures/baselines/baselines/v<release>/`
  rename discipline continues. The rename step disappears for v0.6.0
  if R-1 retires the baselines fixture tree (see R-1 scope).

## 3. Internal layout target (v0.6.0)

```
cntrdct/                                # single crate, single repo
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── cargo_subcommand.rs
│   ├── core.rs                         # Detector trait, Language enum, Finding, etc.
│   ├── ir.rs                           # NEW: language-agnostic AST shape
│   ├── parsers/                        # NEW: per-language tree-sitter → IR
│   │   ├── mod.rs                      # ParserProvider trait, parser_for()
│   │   ├── rust.rs
│   │   ├── python.rs
│   │   └── ...                         # one file per language
│   ├── detectors/
│   │   ├── arg_swap.rs                 # cross-cutting, IR-based
│   │   ├── clone_drift.rs
│   │   ├── comment_code.rs
│   │   ├── pr_miner/
│   │   ├── unreachable_after_terminator.rs
│   │   └── lang/                       # NEW: language-specific
│   │       └── rust_config_interaction.rs   # moved from src/detectors/config_interaction.rs
│   ├── eval.rs / sarif.rs / ranker.rs / adjudicator.rs / ...   # unchanged
│   └── ...
├── tests/                              # unchanged shape; tests gain IR-conversion coverage
├── benchmarks/
│   ├── audit-corpus/
│   ├── wild-corpus/                    # Rust (current)
│   ├── wild-corpus-python/             # current
│   └── wild-corpus-<lang>/             # added one per new language
├── docs/
│   ├── spec/                           # gains ir-v0.md alongside existing specs
│   └── surveys/                        # per-(detector, language) surveys
├── CITATIONS.md
├── CONTRIBUTING.md                     # documents the language-contribution workflow
├── CHANGELOG.md
└── REBUILD.md                          # this file
```

Boundary against `research/` is unchanged. Research members do not
gain IR access; promotion remains explicit re-implementation.

## 4. Phased execution sequence

The work is sequenced so each phase has a discrete review surface.

R-0. IR design spec — `[x]`

- Authoritative design at `docs/spec/ir-v0.md`. Drafted 2026-05-24,
  post-R-0-review revision 2026-05-25 absorbing 11 blockers from a
  4-axis parallel review (IR design / detector coverage / spec
  consistency / risk completeness). Reviewer-absorbed decisions
  are settled — `Arc<str>` source sharing,
  `struct IrStmt { kind, attributes, location }`,
  `IrStmtKind::{With, HoistedItem}` variants,
  `Location.{start_byte, end_byte}` for raw-source-slice consumers,
  `DivergentKind` / `BranchMergeKind` / `ConstantBranchKind` enums
  replacing stringly-typed `kind`, `IrExpr::{Return, Raise}`
  Option-wrapping, `IrFile::resolve(NodeRef)` lookup,
  `IrConvertError` production-runtime contract (warn-log +
  per-file skip, no synthetic SARIF finding), LSP cache requirement
  for partial-parse non-regression. See ir-v0.md "Risks and open
  questions" R8-R11 for derived decisions; do not re-litigate.

R-1. IR implementation + Rust/Python migration — `[x]`

Authoritative design: `docs/spec/ir-v0.md`. Sub-step ordering is
binding (ir-v0.md R11):

R-1.0. Capture T1 pinning fixtures (ir-v0.md F6 T1) at v0.5.2 HEAD
       before any rewrite begins. `cntrdct scan --json` against
       `benchmarks/audit-corpus` and `benchmarks/wild-corpus*` per
       detector, snapshot to `tests/fixtures/ir-pinning/<detector>/
       {audit,wild}.json`.
       Status: `[x]` 2026-05-26 (commit b6c6540).
R-1.a. Create `src/ir.rs` with the IR types per ir-v0.md F1.
       Status: `[x]` 2026-05-26 (commit 704eb59; `src/ir.rs`
       carries every F1 type + `IrConvertError` variants +
       `IrFile::resolve` + hand-written `NodeRef` `Serialize` impl,
       plus 7 inline unit tests).
R-1.b. Promote `src/parsers.rs` to `src/parsers/{mod,rust,python}.rs`
       and implement `to_ir` per ParserProvider per ir-v0.md F2.
       Status: `[x]` 2026-05-26 (commit 704eb59;
       `mod.rs` adds `to_ir` to the `ParserProvider` trait +
       `build_ir_shell` shared prelude, `rust.rs` / `python.rs`
       implement the full F1 surface — IrFn / IrParam / IrBlock /
       IrStmt + statement classification / IrCallSite / IrPath /
       IrExpr / IrLiteral / IrTerminator / IrComment / IrDecorator
       + byte-identical `walk_normalize_*` tokenisation. F6 T2 /
       T3 / T4 / T5 land in this sub-step under
       `tests/ir_{convert_error,recovery,convert,location}.rs` +
       `tests/fixtures/ir/{rust,python}/{impl_methods,
       class_methods,nested_calls,nested_if_match}.{rs,py,json}`;
       F6 T1 was captured in R-1.0, T6 and T7 still pending per
       ir-v0.md R11 ordering).
R-1.c. Rewrite the five cross-cutting detectors against IR.
       Suggested order: comment-code → arg-swap →
       unreachable-after-terminator → clone-drift → pr-miner.
       `cntrdct-lsp` LSP cache change (ir-v0.md F5 non-regression)
       lands in this commit.
       Status: `[x]` 2026-05-26 (commit 704eb59;
       R-1.c0 deletes `ParsedFile` from `src/core.rs` and switches
       `DetectContext.files` to `&[IrFile]`; `src/lib.rs` now parses
       each file once via `parser_for(lang).to_ir(...)` and the
       resulting `IrFile` flows through every detector — eliminates
       the v0.5.x per-detector double-parse; `src/ir.rs` adds a
       `SyncTree` newtype with `unsafe impl Sync` so
       `Arc<SyncTree>: Sync` lets the existing `par_iter()` paths
       compile; `src/config.rs` `collect_attribute_suppressions` and
       `apply()` take `&IrFile`; the five cross-cutting detectors
       (`arg_swap.rs`, `clone_drift.rs`, `comment_code.rs`,
       `pr_miner/{mod,extract_rust,extract_python}.rs`,
       `unreachable_after_terminator.rs`) and `config_interaction.rs`
       drop their internal `tree_sitter::Parser::set_language` +
       `parser.parse(&source, None)` prelude and read
       `file.raw_tree.root_node()` directly. LSP F5 non-regression:
       `src/lsp.rs` `UriState` gains `last_clean_findings:
       Option<Vec<Finding>>`, populated when `IrFile.parse_recovered
       == false` and re-published when a subsequent buffer state is
       recovered so the editor's problems pane no longer blinks
       mid-keystroke; eviction on `did_close`. The cross-cutting
       detectors still touch `IrFile.raw_tree` rather than consuming
       IR (`IrFn` / `IrBlock` / `IrStmt` / `IrCallSite`)
       semantically — the ir-v0.md F5 "cross-cutting detectors must
       not touch raw_tree" discipline is documentation-level (F5
       explicitly defers type-system enforcement) and a follow-up
       commit will rewrite the detector internals to consume IR
       fields, preserving the byte-identical T1 pinning. T1
       byte-identical against the v0.5.2 capture verified across
       all 5 detectors × 3 corpora (audit / wild-rust /
       wild-python); `tests/ir_pinning.rs` runs the comparison.
       The 6 detector integration test files (`tests/detector_*.rs`)
       and `src/detectors/pr_miner/extract_{rust,python}.rs` inline
       tests rebuild `IrFile` inputs via the new public helper
       `cntrdct::ir_from_source(path, lang, source)`. Spec sweep:
       `arg-swap-v0.md`, `clone-drift-v0.md`, `comment-code-v0.md`,
       `unreachable-after-terminator-v0.md`, `pr-miner-v0.md` F1
       input sections updated to `&[IrFile]`. `cargo test
       --all-targets`, `cargo clippy --all-targets -- -D warnings`,
       `cargo fmt --all -- --check` all green; the
       `cargo test --features lsp` variant likewise green
       (includes two new F5 cache unit tests).
R-1.c'. T7 measurement (ir-v0.md F6 T7): capture wall-clock +
       peak-RSS on `benchmarks/wild-corpus-python` (the real corpus
       is 11 files, not the spec's ~600) and supplementally on
       `benchmarks/wild-corpus` (270 Rust files) via
       `/usr/bin/time -l`. MUST run before R-1.e retires the
       baseline harness so the comparison shape remains available.
       Status: `[x]` 2026-05-26 (this commit). The first measurement
       pass against the eager `IrFile.raw_tree` design (commit
       704eb59) showed a 5.4× peak-RSS regression on wild-corpus
       Rust (71 → 380 MiB), exceeding the R-1 release gate's 25 %
       threshold and triggering ir-v0.md R1's "R-0 revisits the
       retention decision" clause. The R1 mitigation landed in this
       commit: `IrFile.raw_tree` becomes a method (`raw_tree()`)
       that reparses the source on demand instead of a stored
       `Arc<SyncTree>` field; `IrBlock.normalised_tokens` is
       populated only on `IrFn.body` per ir-v0.md R2 (nested blocks
       carry an empty vector); `IrFile::resolve` becomes
       `resolve_with(node_ref, FnOnce(Node) -> R)` so a freshly-
       parsed tree drops when the closure returns. T4 golden
       fixtures re-blessed (nested-block `normalised_tokens` arrays
       are now empty). T1 byte-identical pinning held across all
       five detectors × three corpora. Re-measurement: Rust peak
       RSS 380 → 156 MiB (still +118 % over v0.5.2, but well under
       the eager-IR result); Rust wall-clock 0.41 → 1.62 s
       (within the 25 % gate vs v0.5.2's 1.33 s); Python within
       noise on both axes. The residual Rust RSS regression is
       dominated by IR struct overhead held across all 270 files
       for the scan duration — see R-1.c'' below for the follow-up.
       Full report: `benchmarks/self-replication/v0.6.0/t7-performance.md`.
R-1.c''. IR compaction follow-up. The R-1.c' lazy reparse cut the
       Rust wild-corpus RSS regression from 5.4× to 2.3× but did
       not close it to within the 25 % gate. Two paths to closing
       it, neither blocking R-1.h: (a) string-form IR compaction —
       `String` → `Box<str>` across `IrFn.name` / `IrParam.name` /
       `IrPath.{receiver, segments, raw}` / `IrLiteral` /
       `IrComment.text` / `IrDecorator.{name_path, raw}` and
       packed `NodeRef` (Range → 4 × u32, drop tree-sitter
       `Point`); (b) cross-cutting detector IR migration (the
       deferred R-1.c follow-up) — once detectors stop calling
       `IrFile::raw_tree()` and instead read `IrFn` / `IrBlock` /
       `IrStmt` / `IrCallSite` semantically, the reparse cost
       disappears and the wall-clock recovers the 3× win the
       eager design demonstrated. Both items may be sequenced
       independently. R-1.h's release gate can either be amended
       (per ir-v0.md R1 "R-0 revisits the retention decision") or
       wait on R-1.c''; do not tag v0.6.0 against the current
       Rust-corpus RSS without one of those resolutions.
       Status: `[x]` 2026-06-03. Path (b) cross-cutting migration
       COMPLETE (steps 1-3 below + the four detector migrations;
       `pr-miner` retained on `raw_tree()` by design — step 4). Path
       (a) CLOSED with a measurement-driven scope correction (see
       below). Both paths landed; R-1.c'' is done.

       Path (a) outcome (2026-06-03). The spec sketch above (`String`
       → `Box<str>` + packed `NodeRef`) was re-scoped after a floor
       study showed it targeted the wrong fields: per-node string/
       NodeRef shrink saves only single-digit MiB (names are short;
       NodeRefs are few), nowhere near the ~80 MiB the < 25 % gate
       would need. A `scan benchmarks/wild-corpus` (270 files)
       attribution measured the real contributors — `IrFn.
       normalised_tokens` ≈ 33 MiB and the per-node `Location.file`
       path duplication ≈ 16 MiB — and a combined floor of ~125 MiB
       even with BOTH emptied (+75 % over v0.5.2's 71.5 MiB). The
       gate is structurally unreachable by field compaction: the
       cross-file detectors (clone-drift, pr-miner) require the whole
       corpus's IR resident at once, so retention cannot be cut. The
       safe, T1-byte-identical win that path (a) could ship landed:
       `Location.file` moved from `PathBuf` (deep-copied per node) to
       a shared `Arc<Path>` (one alloc per file, refcount-cloned per
       node), serialised via a `serialize_with` shim so the T4 golden
       wire shape stays byte-identical (serde has no `Arc` Serialize
       without the unused `rc` feature). Result: Rust wild-corpus
       peak RSS ~169 → ~150 MiB (5-trial median, +109 % over v0.5.2);
       wall-clock +10.5 %; T1 byte-identical across all five detectors
       × three corpora; full gate green (`cargo test --all-targets`,
       `--features lsp`, `clippy -D warnings`, `fmt --check`). The
       § 9 peak-RSS gate was retired-and-replaced (relative < 25 % →
       absolute ≤ 175 MiB ceiling) per the measurement; see § 9 and
       ir-v0.md R1. The `String`→`Box<str>` / NodeRef-packing items
       were dropped as not worth the churn for a sub-gate win.
       Path (b) progress:
       - `comment-code` (2026-05-27, commit 2d90b3c) reads
         `IrFn.{leading_doc, return_type_text, decorators, body}` +
         `IrBlock.statements` + `IrStmtKind::{Raise, Return, If,
         While, With, Match, Loop}` + `IrFile.source` byte slice
         semantically; the per-detector `raw_tree()` call is gone.
       - `clone-drift` (2026-05-29, follow-up step 1) now reads
         `IrFn.normalised_tokens` (function-item-rooted) for the
         function-level clustering pipeline (top-level `!is_method`)
         and walks `IrStmtKind::If` / `IrExpr::If` +
         `IrBlock.normalised_token_count` + `IrFile.source` byte
         slices for F2b; both `raw_tree()` calls are gone. The
         IR-side prerequisite landed in the same change: the
         normalised token sequence moved from `IrBlock.normalised_tokens`
         (body-rooted) to `IrFn.normalised_tokens` (function-item-
         rooted) so the signature prefix participates in the n-gram
         set, and `IrBlock` carries a per-block
         `normalised_token_count: usize` (not the vector — O(1)
         per block) for F2b's consequence size gate.
         `NormalisedToken` derives `Hash`. T4 goldens re-blessed;
         ir-v0.md §F1 / R2 / §F4 / F6 T4 + clone-drift-v0.md
         §F2 / F2b / F4 updated.
       - `unreachable-after-terminator` (2026-06-02) consumes IR
         semantically: block-level terminator classification from
         `IrStmtKind` + `IrIfStmt.terminator` / `IrMatchStmt.terminator`
         (branch-merge) + `IrLoopStmt.has_break_to_self`; F4d-ii/iii/iv
         from `IrCallSite.args` / `IrStmtKind::Return` value /
         `IrIfStmt.condition` reading the per-`IrExpr` `location` (step 3
         below); F4e from the literal `IrExprKind::Literal` condition.
         Suppression modelled via `IrFn.decorators` + `IrStmt.attributes`
         (the corpus carries no real `#[allow(unreachable_code)]`, so
         this only affects the t6/t14 integration tests). Both
         `raw_tree()` calls gone. Bundled IR fix: the converter looked
         for the loop / break label node under the wrong kind
         (`loop_label`); tree-sitter-rust names it `label`, so
         `IrLoopStmt.has_break_to_self` and `IrLabel` were wrong for
         labelled loops (`'outer: loop { loop { break 'outer; } }`) —
         corrected, which the IR-reading detector requires for T1
         (audit `rustc_ui_expr_loop.rs:28` no longer over-reports
         loop-no-break). unreachable-after-terminator-v0.md §F2 / F3
         updated. No T4 re-bless (no labelled-loop golden fixture).
       - `arg-swap` (2026-06-01) now reads `IrFn.{name, params,
         is_method}` for definition extraction (Rust top-level
         `!is_method`; Python incl. class methods, `Receiver` dropped,
         `Unsupported` whole-fn reject) and recursively walks every
         function body's IR (`IrBlock.statements` descending through
         `IrStmtKind::{If, While, Loop, For, Match, With, Try}` +
         `Let` / `Assign` RHS + `Return` / `Raise` / `Assert` / `DivergentCall`
         payloads + `IrExpr::Call` arg nesting) to enumerate
         `IrCallSite`s; both `raw_tree()` calls are gone. The IR walk is
         a strict subset of the v0.5.x full-tree traversal — calls in
         still-`IrExpr::Other` shapes are unvisited but cannot
         manufacture a finding v0.5.x lacked, so T1 stays byte-identical.
         IR-side prerequisite landed in the same change: the converter
         now unwraps the transparent Python `await` wrapper (ir-v0.md
         §F2) so `_ = await copy(src, dst)` (arg-swap t20) stays
         reachable. ir-v0.md §F2 transparent-wrapper note +
         arg-swap-v0.md §F2 / F3 updated. No T4 re-bless (no `await`
         fixture).
         CORRECTION (2026-06-03): the "strict subset is harmless" claim
         above was wrong. A recall-audit FN triage showed the IR-only
         call walk silently regressed arg-swap recall on real code —
         name-correlating swaps nested in `IrExpr::Other` shapes (binary
         operands, closures, Python comprehensions / generators /
         conditional expressions, f-strings) produced no finding where
         v0.5.x did. The T1 gate missed it because the one such audit call
         (`totalsegmentator_statistics.py:10`, inside a list comprehension)
         has no name correlation and so fires in neither version. Fix:
         arg-swap call-site ENUMERATION reverted to the v0.5.x raw-tree
         walk over `IrFile::raw_tree()` (Pattern-B escape hatch, same as
         pr-miner); DEFINITION extraction stays on IR (`IrFn.params` /
         `is_method` are lossless). T1 byte-identical preserved; corpus
         `recall_upper_bound` unchanged (0.918 / arg-swap 0.25); regression
         guard `tests/detector_arg_swap.rs` T30 / T31. arg-swap-v0.md §F3
         + "Known recall upper bounds" (Bound A same-file resolution,
         Bound B name-correlation ceiling → R-4), recall-audit-v0.md
         "arg-swap / clone-drift FN triage", clone-drift-v0.md "Known
         recall bound — branches_sharing_code" (Bound C) updated.
       T1 byte-identical across audit / wild-rust / wild-python for
       both migrated detectors. T7 spot-check (3 trials, wild-corpus
       Rust): wall-clock 1.62 → ~1.29 s (now ≈ v0.5.2's 1.33 s — the
       reparse removal recovered the lazy-reparse penalty); peak RSS
       ~159 MiB, essentially unchanged (RSS is dominated by IR struct
       retention, which path (a) / full step-2 migration addresses,
       not clone-drift's transient reparse).
       The remaining three cross-cutting detectors were blocked on
       the IR-coverage extension below; step 2 (2026-05-30) closes it,
       so they are now ready to migrate:
       - `arg-swap` and `pr-miner` enumerate call sites across
         the full function body. IR currently classifies Python
         `for_statement` / `try_statement` / `assignment` and
         Rust `let_declaration` / `for_expression` as
         `IrStmtKind::Other` (opaque `NodeRef`), so calls hiding
         inside those shapes are invisible to an IR-only walk.
         Audit-corpus T1 fixtures place calls inside both shapes
         (`rarfile_set_attrs.py`'s swap is inside `for dst, inf
         in dirs:`; integration tests use `_ = copy(src, dst)`
         assignment-wrapped form).
       - `unreachable-after-terminator` hits the same gap.
         `rustc_ui_expr_return.rs` audit fixture's nested
         `let x: () = {return {return {return;}}};` is a single
         `let_declaration` → `Other` and the inner terminators
         are not reachable from IR.
       Follow-up sequence (each its own commit on the IR side
       before each detector migration):
       1. [done] Moved `IrBlock.normalised_tokens` →
          `IrFn.normalised_tokens` (+ `IrBlock.normalised_token_count`),
          re-blessed T4 goldens, updated ir-v0.md §F1 / R2 / §F4,
          migrated `clone-drift`.
       2. IR-side extension [done] 2026-05-30: added
          `IrStmtKind::{For(IrForStmt), Try(IrTryStmt), Assign{value},
          Let{value}}` covering Python `for_statement` /
          `try_statement` / `assignment` and Rust `let_declaration` /
          `for_expression`. Each carries the iterable / body / RHS as
          materialised `IrExpr` / `IrBlock` so call sites and nested
          terminators inside those statement shapes are reachable from
          an IR-only walk (verified in the new T4 goldens:
          `rust/let_for.rs`, `python/for_try_assign.py`). New variants
          contribute no block terminator (parity with the prior
          `Other` → `None`), so T1 stays byte-identical across all five
          detectors × three corpora (detectors still read `raw_tree()`
          until they migrate). Re-blessed `python/class_methods.json`
          (assignment → Assign); updated the two T5 `*_stmt_kind_other`
          location tests to a binary-expression statement (the
          canonical remaining `Other` shape). Updated ir-v0.md §F1
          (enum + the two new structs + the arg-swap / unreachable /
          pr-miner traceability rows) and §F6 T4. Scope boundary: this
          commit closes the *statement-shape* gap only; calls nested
          inside still-`Other` expressions (e.g. `x = a + foo()`'s
          `binary_operator` RHS) remain opaque and are the detector
          migrations' concern (NodeRef recovery or further IrExpr
          coverage). `arg-swap` migrated 2026-06-01 (pure IR walk +
          `await` unwrap).
       3. IR-side extension [done] 2026-06-02: `IrExpr` became
          `struct IrExpr { kind: IrExprKind, location: Location }`
          (was a bare enum). Every expression now carries its source
          span so `unreachable-after-terminator` can report the
          F4d-ii / F4d-iii / F4d-iv (divergent call arg / divergent
          return-or-break value / divergent `if` condition) and F4e
          (Python constant condition) finding endpoints at the v0.5.x
          raw-node spans without `raw_tree()`. The enum variants moved
          verbatim onto `IrExprKind`; all converter / detector match
          sites updated to `&expr.kind`; the converter wraps each
          `convert_*_expr` result with `node_location(node)` (transparent
          `parenthesized_expression` / `await` wrappers keep the inner
          node's location). All 7 expr-bearing T4 goldens re-blessed
          (additive: each expr gains a `location`); T1 stays
          byte-identical (findings do not serialise `IrExpr`). ir-v0.md
          §F1 updated. `unreachable-after-terminator` migrated
          2026-06-02 (consumes the new locations).
       4. `pr-miner` retained on `raw_tree()` by design (decided
          2026-06-02). Unlike the other cross-cutting detectors,
          pr-miner mines association rules over the SET of every
          call-head last-segment in each function body — a full
          recursive AST enumeration. The structured IR does not
          losslessly preserve that set: `IrCallSite.callee` (`IrPath`)
          flattens a method-chain receiver to name segments and drops
          the receiver CALL (`a.b().c()` exposes `c` but not `b`), and
          calls nested in still-`IrExpr::Other` shapes (`?`-try,
          binary, index, …) are invisible to an IR-only walk. An
          empirical probe over `benchmarks/wild-corpus` (270 files)
          showed 593 / 909 top-level functions (65 %) produce a
          DIFFERENT call-head set under a pure-IR walk, missing 1750
          call heads in total — pr-miner's global Apriori support /
          confidence would shift and T1 (audit: 2 findings) would not
          hold. Migrating it would require materialising call sites in
          receiver position AND every Other expression shape (plus a
          decorated-definition location fix), i.e. reconstructing the
          full AST inside IR, which defeats the IR-simplicity goal and
          carries high byte-identical risk. pr-miner therefore keeps a
          single per-file `IrFile::raw_tree()` reparse — the same
          Pattern-B escape hatch `src/detectors/lang/rust_config_interaction.rs`
          uses (ir-v0.md §F5) — and is NOT a Path (b) target.
          `extract_{rust,python}.rs` are unchanged. Path (b) cross-cutting
          migration is complete modulo this exception.
       Either follow-up can be sequenced with Path (a) (string-
       form IR compaction) in any order.
R-1.d. `git mv src/detectors/config_interaction.rs
       src/detectors/lang/rust_config_interaction.rs` + module
       path updates + test path rewrite
       (`tests/detector_config_interaction.rs` →
       `tests/detector_rust_config_interaction.rs`).
       Status: `[x]` 2026-05-26 (this commit; `src/detectors/lang/mod.rs`
       added, every import site updated, T1 fixtures byte-identical,
       `cntrdct ALL_DETECTOR_IDS` still lists `config-interaction`).
R-1.e. Retire Q-15 baseline scaffolding:

       ```sh
       git rm src/baselines.rs
       git rm -r baselines/sourcerercc baselines/pybuglab
       git rm tests/baselines.rs
       git rm -r tests/fixtures/baselines/
       ```

       Remove `pub mod baselines` from `src/lib.rs`. Remove
       `--baseline` / `--baselines-out` / `--baselines-skip-run`
       flags from `cntrdct eval` (clap + match). Rewrite the
       README "Baseline comparison" section as "Self-replication
       ledger" documenting `benchmarks/self-replication/v<release>/`.
       Status: `[x]` 2026-06-02 (uncommitted in working tree). Code
       side: `pub mod baselines` + the `use crate::baselines::*`
       block + `BaselineRunError` + the entire Q-15 baseline-comparator
       subcommand section (`embedded_priors_sha256`,
       `run_eval_with_baselines`, `load_or_run_baseline`,
       `expected_by_detector`) removed from `src/lib.rs`; the three
       `--baseline*` flags + the comparator match arm removed from
       `src/main.rs` (Eval now has only `--manifest`); the now-unused
       `sha2` dependency dropped from `Cargo.toml` (gone from
       `Cargo.lock`). README "Baseline comparison" → "Self-replication
       ledger"; `book/src/workflows/eval.md` + `book/src/detectors/
       pr-miner.md` baseline references rewritten. Deletions
       (`git rm`, run by the maintainer per the no-self-delete rule):
       `src/baselines.rs`, `tests/baselines.rs`,
       `baselines/{sourcerercc,pybuglab}/`,
       `tests/fixtures/baselines/`, `docs/spec/sota-baselines-v0.md`.
       Sweep done (§10): the now-orphan `sajnani-icse-2016`
       CITATIONS.md entry removed (its sole consumer
       `src/baselines.rs` is deleted; `allamanis-neurips-2021` stays —
       still cited by the live `arg-swap` detector); `prereg/
       2026-05-19-osf-prereg.md` keeps its H6.1 prose as a frozen
       historical record. Full gate green after deletions:
       `cargo test --all-targets`, `cargo clippy --all-targets --
       -D warnings`, `cargo fmt --all -- --check`.
R-1.f. Introduce `benchmarks/self-replication/v0.6.0/cntrdct.jsonl`
       as the eval snapshot. Add an `assemble_report` helper
       computing the delta against the previous tag's snapshot
       (F1 / P / R deltas). Manual refresh per release; no CI gate.
       Status: `[x]` 2026-06-02 (uncommitted in working tree). Landed
       under `v0.8.0/` (not the spec's `v0.6.0/` — the version moved on
       to 0.8.0 across the R-1.c'' path(b) migrations; the ledger
       tracks the current release line). `benchmarks/self-replication/
       v0.8.0/cntrdct.jsonl` holds three `EvalReport` lines (audit /
       wild-rust / wild-python), one per line; `EvalReport` gained a
       `corpus` field (+`Deserialize`) so the lines self-identify and
       match across releases. `assemble_report(current, previous)` +
       `SelfReplicationDelta` / `CellDelta` / `Prf` + `load_eval_snapshot`
       live in the new `src/self_replication.rs`; it computes the
       per-detector + overall F1/P/R `current`/`previous`/`delta`,
       matched by corpus, with a baseline fallback (`has_baseline =
       false`) when no prior line matches. Wired into the `eval`
       subcommand via `--against <prev.jsonl>` (per the chosen
       integration — eval is an existing allowed subcommand, no new
       CLI surface). v0.8.0 is the first snapshot, so the delta path is
       exercised by `tests/self_replication.rs` (self-comparison →
       zero deltas) + the module unit tests rather than against a real
       prior; the machinery is ready for v0.9.0. Wild corpora are
       unlabelled (P/R = 0); their signal is `actual_total` drift in
       the raw line. README + `book/src/workflows/eval.md` updated.
       Full gate green (`cargo test --all-targets`, `cargo clippy
       --all-targets -- -D warnings`, `cargo fmt --all -- --check`).
R-1.g. Recalibrate priors:

       ```sh
       cargo run --release -- calibrate
       cargo run --release -- calibrate --audit-recall benchmarks/audit-corpus
       ```

       Commit the `benchmarks/priors-default.json` update.
       Status: `[x]` 2026-06-03 (uncommitted in working tree). Run with
       the explicit corpus / output args the schematic commands omit:
       `cntrdct calibrate benchmarks/labelled-findings.jsonl --output
       benchmarks/priors-default.json` (priors mode requires the
       labelled-corpus arg and defaults its output to the OS cache dir,
       not the repo). The regenerated priors are byte-identical to the
       committed file — no `priors-default.json` change to commit. This
       is the expected no-op: detect() is T1 byte-identical to v0.5.2
       across the IR migration and `benchmarks/labelled-findings.jsonl`
       is unchanged, so the per-detector Jeffreys/Wilson priors are
       unchanged. T6 recall re-measurement (the §9 deferral folded into
       this step): `cntrdct calibrate --audit-recall
       benchmarks/audit-corpus` → `overall_recall_upper_bound` = 0.918
       (56 tp / 5 fn; per-detector arg-swap 0.25, clone-drift 0.5,
       comment-code 1.0, config-interaction 1.0, pr-miner 1.0,
       unreachable-after-terminator 0.941). This equals the v0.5.2
       baseline exactly — `benchmarks/audit-corpus` (manifest included)
       is byte-identical to `v0.5.2` HEAD and detect() is T1
       byte-identical, so the figure is not an IR regression; the §9 /
       ir-v0.md T6 floor was corrected from the rounded 0.92 to the
       provable 0.918 in this step (see §9 "Floor reconciliation").
       Full gate green: `cargo test --all-targets` (incl.
       `tests/ir_pinning.rs` T1 byte-identical ×15), `cargo clippy
       --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
R-1.h. `CHANGELOG.md` entry + `Cargo.toml` version → `0.6.0`.
       Conventional Commits prefix: `chore(release)` (cliff parses
       this and groups it under "Miscellaneous Chores"; the breaking
       IR commits 704eb59 / 92dafa3 already carry `feat(ir)!` /
       `perf(ir)!` and surface as the breaking-change headline in
       the auto-generated release notes). `CHANGELOG.md` is
       regenerated post-release by the `update-changelog` job, so
       no manual entry in this commit.
       Status: `[x]` 2026-05-26 (this commit; Cargo.toml 0.5.2 →
       0.6.0, Cargo.lock synced, Q-15 fixture pin renamed
       `tests/fixtures/baselines/baselines/v0.5.2/` →
       `v0.6.0/`).
R-1.i. Release per CLAUDE.md "Release procedure":

       ```sh
       cargo update -p cntrdct
       git add Cargo.toml Cargo.lock
       git commit -m "chore(release): bump version to 0.6.0"
       git tag -a v0.6.0 -m "release v0.6.0"
       git push --follow-tags
       ```

       Status: `[x]` 2026-05-26 (this commit + tag v0.6.0).

R-1 also resolves the historical references to the retired
`ROADMAP.md` and the v0.5.x `ParsedFile` type across the repo
(see §9 "R-1 cross-cutting sweep" below) and updates the five
detector specs + `lsp-v0.md` per ir-v0.md F4. The
`tests/fixtures/baselines/baselines/v<release>/` rename step from
CLAUDE.md "Release procedure" is retired by R-1.e — does not
apply to v0.6.0.

Cuts v0.6.0.

R-2. TypeScript pilot — `[ ]`

- Add `src/parsers/typescript.rs`, `tree-sitter-typescript` Cargo
  dep, `benchmarks/wild-corpus-typescript/`.
- Run the per-(detector, TypeScript) literature surveys under
  `docs/surveys/<detector>-typescript-2026-MM.md`. Citations land
  under `CITATIONS.md` with `Languages: TypeScript` lines (or the
  `unconfirmed:` form per `docs/spec/citations-policy.md`).
- Re-calibrate priors with the TypeScript corpus included.
- Cuts v0.7.0. This is the proof point that the G1+G3 promises hold:
  the PR diff should be parser + corpus + surveys, not detector
  edits.

R-3. Go pilot — `[ ]`

- Same shape as R-2 for Go. Validates that R-2 was not
  TypeScript-specific.
- Cuts v0.7.x.

R-4. P3 revisit for Layer 0 LLM (carry-over from Q-17) — `[ ]`

- Architectural amendment to P3 permitting a Layer 0 candidate
  generator that runs an LLM against IR call-site predicates
  before Layer 1 sees them. Out-of-scope for v0.6.0 but spec
  drafting starts after R-1 ships so the IR predicates are
  available to anchor the amendment.
- Required reading: `docs/spec/arg-swap-v0.md` "Name-correlation
  upper bound", `docs/spec/cross-model-kappa-v0.md` (Q-13 reuse
  precedent).
- Spec: `docs/spec/p3-amendment-v0.md` (new).

R-5. Python `except` handler reachability — `[x]`

- Implement F4f (raise-set extraction + class-hierarchy check)
  under `src/detectors/lang/python_unreachable_except.rs`. This is
  the first language-specific detector under the new layout and
  serves as the canonical example for contributors.
- Builtin contract table at `data/python-builtin-exceptions.json`
  with per-entry CPython doc URLs (provenance audit-able for
  future Python version drift).
- Spec: extend `docs/spec/unreachable-after-terminator-v0.md` with
  the F4f section.
  Status: `[x]` 2026-06-03 (uncommitted in working tree). v0 scope is
  the ordering/subsumption check only (confirmed): a handler is flagged
  iff every exception type it catches is provably a subclass-or-equal of
  a type caught by an earlier handler in the same `try`. The
  REBUILD-sketch "raise-set extraction" was scoped to mean the
  caught-type subsumption analysis; body raise-set inference (a handler
  for an exception the `try` body cannot raise) and PEP 654 `except*`
  groups are explicit non-goals (the F4f spec section preregisters
  both). New detector `python-unreachable-except`
  (`src/detectors/lang/python_unreachable_except.rs`, Pattern B raw
  tree-sitter per ir-v0.md §F5): reads `try_statement` → `except_clause`
  handlers, resolves subclass relationships via the embedded CPython
  hierarchy table (`data/python-builtin-exceptions.json`, Python 3.13,
  child→parent + per-entry doc URL, `include_str!` so `detect()` stays
  fs/socket-free per P3) chained with same-file `class Foo(Bar)`
  definitions; unresolvable (imported/unknown) names are INDETERMINATE
  and never flagged (precision-first). `anomaly_class = Logic`,
  `raw_severity = Warning`. Wiring: added to `ALL_DETECTOR_IDS`,
  `run_detectors_on` (six→seven), `src/main.rs` SARIF rule vec,
  `src/lsp.rs` citation registry, and the four hardcoded test lists
  (`wiring_consistency`, `citations_consistency`, `multilang_config`,
  `corpus_shape`). Citations: `hovemeyer-pugh-oopsla-2004` (FindBugs UR
  pattern) + `de-padua-shang-icpc-2017` (ICPC 2017 "Unreachable Handler"
  anti-pattern), both peer-reviewed concept grounding; Python coverage
  is `LanguageCitationStatus::Unconfirmed` per citations-policy.md —
  survey `docs/surveys/python-unreachable-except-python-2026-06.md`
  found no peer-reviewed Python-subject study of this anti-pattern (the
  SSRN "Slithering ... Python" study is the top revisit trigger).
  Corpus: 8 positive Python fixtures under `benchmarks/corpus/files/
  python_unreachable_except_00{1..8}.py` + manifest entries (meets the
  `corpus_shape` ≥8-positives-per-registered-detector contract) and 8
  matching `benchmarks/labelled-findings.jsonl` TruePositive lines;
  `benchmarks/priors-default.json` regenerated (additive: new
  `python-unreachable-except` prior tp=8/fp=0/jeffreys, existing priors
  byte-identical). Tests: `tests/detector_python_unreachable_except.rs`
  (15 cases T1-T15: superclass-before-subclass, ordering, tuple full/
  partial coverage, same-file user class, indeterminate import, bare
  `except` reachability, `except*` skip, citation/status, determinism,
  non-Python skip, anomaly class, duplicate, builtin chain). Full gate
  green: `cargo test --all-targets` (+ `--features lsp`),
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all --
  --check`; recall audit `overall_recall_upper_bound` = 0.918 (unchanged
  — no audit-corpus entries added, §9 floor held). Not yet released; a
  v0.9.0 bump + tag is the gated follow-up.

R-6. VS Code extension (carry-over from T3-12 Phase 2) — `[ ]`

- `vscode-cntrdct` extension scaffolding in a separate
  `ktrysmt/vscode-cntrdct` repo, bundling the LSP binary
  auto-downloaded from GitHub Releases. Phase 1 LSP already shipped
  in v0.5.2; Phase 2 is the extension itself.
- Independent of R-1 through R-5; can start any time after R-1
  ships v0.6.0 (the LSP binary tracks the same crate version).

## 5. Execution order

1. R-0 — IR spec. (done 2026-05-25; see §4 R-0)
2. R-1 — IR implementation, Rust + Python migration, Q-15 scaffolding retirement. Cuts v0.6.0.
3. R-2 — TypeScript pilot. Cuts v0.7.0.
4. R-3 — Go pilot.
5. R-4 — P3 amendment spec (R-1 IR enables predicate-level framing).
6. R-5 — Python F4f detector.
7. R-6 — VS Code extension (independent of the rest after R-1).

The order assumes R-1 is the bottleneck. R-2 / R-3 / R-5 / R-6 can
parallelise once R-1 lands.

## 6. Out of scope

Explicitly not part of this rebuild:

- External SOTA comparator framework (the retired Q-15 vision).
  The "compare against PyBugLab / SourcererCC head-to-head" idea
  was structurally unrealisable; the self-replication ledger
  under R-1 is the deliberate replacement.
- Out-of-process plugin model (WASM, dlopen, JSON-line protocol,
  separate `cntrdct-plugin-<lang>` crates on crates.io). Considered
  and dropped; the single-repo contribution model is the
  intentional design.
- Cargo features for opt-in language compilation. Reconsider only
  if binary size becomes a measurable complaint after R-2 or R-3.
- AUR distribution. Dropped from T3-14 scope; reopen as a new
  R-series item if external demand surfaces.
- Apriori → FP-growth migration in pr-miner. Internal quality
  improvement, tracked in `docs/spec/pr-miner-v0.md` future work,
  not in REBUILD scope.
- Layer 3 ML-detector ensemble (PyBugLab / GraphCodeBERT alongside
  the LLM judge). Crosses the P3 boundary like R-4; subsumed by
  the R-4 amendment when that lands.
- Translation of cntrdct output, docs, or surveys. English-only,
  as before.
- Social / outreach / promotion scaffolding. The former R-7
  (Code of Conduct, GitHub Discussions, pinned roadmap discussion)
  and R-8 (public mdBook Pages re-enable) were removed 2026-06-03.
  Community-foundation and dissemination work has no value until the
  product itself is proven worth promoting; it is premature to invest
  in it now. Priority is unambiguous product quality — detector
  correctness, recall / precision, and the IR migration's
  byte-identical guarantees — scrutinised hard enough to establish
  that cntrdct is worth publicising. Only once that bar is met does
  promotion / outreach earn a place back on the plan. Do not
  re-add CoC / Discussions / Pages tasks before then.

## 7. Glossary

- IR — Intermediate Representation. The language-agnostic AST shape
  defined in `src/ir.rs` that cross-cutting detectors consume.
  Specified in `docs/spec/ir-v0.md` (R-0).
- Cross-cutting detector — a detector whose concept transfers
  across languages (arg-swap, clone-drift, comment-code, pr-miner,
  unreachable-after-terminator). Written once on IR after R-1.
- Language-specific detector — a detector whose concept is bound to
  one language's syntax (Rust `#[cfg]` interaction, Python `except`
  reachability). Lives under `src/detectors/lang/<lang>_<id>.rs`,
  reads tree-sitter ASTs directly.
- Self-replication ledger — `benchmarks/self-replication/v<release>/`,
  the per-release snapshot of cntrdct's own eval against its
  corpora. Replaces the retired Q-15 "external SOTA comparator"
  framing.
- Layer 0 — proposed candidate-origination layer that runs an LLM
  against IR predicates before Layer 1 sees them. Requires the R-4
  P3 amendment before any implementation.
- R-series — the rebuild item numbering used in this file. Replaces
  the retired Q-series / T-series / M-series numbering.

## 8. Release sequence preview

- v0.6.0 — R-0 + R-1. Breaking change (the `Detector` trait
  migrates from `ParsedFile` to `IrFile`; the `cntrdct` crate
  bumps from v0.5.x to v0.6.0). Conventional Commits prefix
  `feat(ir)!`. Self-replication ledger replaces baseline scaffolding.
- v0.6.x — fast-follow: R-5 (Python F4f).
- v0.7.0 — R-2 (TypeScript pilot). First language added under the
  single-repo PR contribution model.
- v0.7.x — R-3 (Go pilot), R-6 (VS Code extension).

Version bumps follow `CLAUDE.md` "Release procedure" verbatim; the
`tests/fixtures/baselines/baselines/v<release>/` rename step is
retired by R-1 along with the baselines scaffolding.

## 9. R-1 release gate (verification rules)

Before tagging v0.6.0:

- `cargo test --all-targets` green.
- `cargo clippy --all-targets -- -D warnings` green.
- `cargo fmt --all -- --check` green.
- `cargo run --release -- calibrate --audit-recall benchmarks/audit-corpus`
  reports `overall_recall_upper_bound` ≥ 0.918 (v0.5.2 baseline floor;
  ir-v0.md F6 T6 codifies the threshold). A regression below 0.918
  with T1 green indicates corpus / labelling drift; below 0.918 with
  T1 red indicates converter drift — go back to R-0 spec and revise
  IR representation.
  **Floor reconciliation (R-1.g, 2026-06-03)**: the floor was stated
  as 0.92 through v0.6.0–v0.8.0; the R-1.g re-measurement showed the
  true v0.5.2 baseline is 0.918 (56/61). `benchmarks/audit-corpus`
  (manifest included) is byte-identical to `v0.5.2` HEAD
  (`git diff v0.5.2 HEAD -- benchmarks/audit-corpus/` is empty) and
  detect() is T1 byte-identical, so 0.918 IS what v0.5.2 itself
  produces — the prior 0.92 was an unmeetable rounding, not a real
  floor. Corrected to 0.918 here and in ir-v0.md F6 T6 / §"recall
  threshold". This is a factual correction, not a design change.
  **v0.6.0 deferral**: not re-run for the v0.6.0 tag. Detector
  detect() logic is byte-identical to v0.5.2 (T1 green across all
  five cross-cutting detectors × three corpora); a recall regression
  is therefore not possible without converter drift, which T1 would
  have surfaced. Re-measurement is folded into R-1.g (post-release
  follow-up) so a refreshed `overall_recall_upper_bound` lands with
  the next priors recalibration.
- `cntrdct ALL_DETECTOR_IDS` includes `config-interaction` (verifies
  R-1.d module relocation did not drop the registration).
- `tests/citations_consistency.rs` green (citation key set unchanged
  by R-1).
- `network-isolation` CI job green: `sudo unshare --net cntrdct scan ...`
  must not emit `ENETUNREACH` / `EAI_*` (CLAUDE.md P3 gate). The
  Q-13 `cross-model-kappa` subcommand stays excluded from this
  gate by design; it does not go through IR.
- T1 pinning fixtures (ir-v0.md F6 T1) byte-identical against the
  v0.5.2 capture per detector.
- T7 wall-clock regression < 25 % vs v0.5.2 (ir-v0.md R1): wall-clock
  Rust wild-corpus +10.5 % (median 1.47 s vs v0.5.2 1.33 s), within
  gate. Python within noise.
- T7 peak-RSS: the < 25 % relative rule is **retired for the Rust
  wild-corpus** and replaced by an absolute regression ceiling of
  **≤ 175 MiB** (R-1.c'' path (a), 2026-06-03). Rationale, with the
  measurement that forced it: the IR architecture must retain every
  file's IR for the whole scan because the cross-file detectors
  (clone-drift clusters across all functions; pr-miner mines
  association rules across all files) need the full corpus in memory
  at once. A measured floor study (`scan benchmarks/wild-corpus`,
  270 files) showed peak RSS stays ~125 MiB even after emptying the
  two largest per-node contributors (`IrFn.normalised_tokens` ≈ 33 MiB
  and the per-node `Location.file` path duplication ≈ 16 MiB), i.e.
  +75 % over the v0.5.2 71.5 MiB baseline — so the < 25 % target
  (≈ 89 MiB) is structurally unreachable by field compaction, not a
  missing optimisation. Path (a) shipped the safe, T1-byte-identical
  win it could: `Location.file` is now a shared `Arc<Path>` (one
  per-file allocation referenced by every node instead of a per-node
  `to_path_buf()` clone), cutting Rust wild-corpus peak RSS from
  ~169 MiB to a 5-trial median of ~150 MiB (+109 % over v0.5.2). The
  175 MiB ceiling sits above that with headroom and still catches the
  regression class that matters — the original eager-tree-retention
  design measured 380 MiB. Python wild-corpus stays under the < 25 %
  rule (14.1 MiB). Full analysis:
  `benchmarks/self-replication/v0.6.0/t7-performance.md`.
- T2 / T3 / T4 / T5 per ir-v0.md F6 (IrConvertError variants,
  parse_recovered carry-through, IR golden fixtures, Location
  equality).

## 10. R-1 cross-cutting sweep

The R-1 PR resolves historical cross-references that survive in
docs / book / benchmarks / research. Three-way choice per occurrence:
preserve as history, swap to `REBUILD.md` (or `ir-v0.md`), or delete
entirely. Decision logged in the R-1 PR description.

ROADMAP.md references (14 locations, none in code):

- `docs/spec/multilang-v0.md` (3)
- `docs/spec/pr-miner-v0.md` (3)
- `docs/spec/recall-audit-v0.md` (1)
- `docs/spec/citations-policy.md` (1)
- `docs/spec/cross-model-kappa-v0.md` (2)
- `docs/spec/llm-calibration-v0.md` (1)
- `docs/spec/sota-baselines-v0.md` (1) — file retired entirely by R-1
- `docs/spec/arg-swap-v0.md` (1)
- `docs/spec/lsp-v0.md` (1)
- `book/src/{introduction,releases}.md` (2)
- `docs/site/index.md` (1)
- `benchmarks/{audit-corpus,retraction-watch,wild-corpus-python}/README.md`
  (3)
- `research/projects/PLAN.md` (4)

ParsedFile references (per ir-v0.md F4 override):

- `docs/spec/{arg-swap-v0.md, clone-drift-v0.md, comment-code-v0.md,
  unreachable-after-terminator-v0.md, pr-miner-v0.md}` F1 input
  sections.
- `docs/spec/lsp-v0.md` `scan_buffer` signature and any other
  `ParsedFile` mention.

Verify after the sweep:

```sh
rg -l 'ROADMAP.md' -- docs/ book/ benchmarks/ research/
rg -l 'ParsedFile' -- docs/ src/ tests/
```

The first should return empty (or only `REBUILD.md` itself if its
historical commentary still references the retirement). The second
should return empty under `src/` and `tests/` (R-1 deletes the
type) and `docs/` (R-1 sweep).

## 11. R-1 reading list

A fresh session picking up R-1 needs:

Authoritative (read in full before R-1.a):

- This file (REBUILD.md), §3 layout target, §4 R-1, §9 verification,
  §10 sweep.
- `docs/spec/ir-v0.md` — IR types, conversion contract, test plan,
  risks. Settled design; do not re-litigate per §4 R-0.
- `CLAUDE.md` — design constraints (P1 / P3 / P4 / P5), boundary
  contract with `research/`, release procedure, repo layout rules.
- `docs/spec/multilang-v0.md` — the `ParserProvider` seam ir-v0.md
  extends with `to_ir`.
- `docs/spec/citations-policy.md` — unchanged by R-1 but governs
  the per-language survey requirement R-2 onwards.
- `src/parsers.rs` (149 LOC), `src/core.rs` (623 LOC) — the
  primary implementation seams R-1 modifies.

Context (skim before R-1.c):

- `src/detectors/{arg_swap, clone_drift, comment_code,
  unreachable_after_terminator}.rs` — the cross-cutting detectors
  R-1.c rewrites. Per-language `scan_rust` / `scan_python` helpers
  fold into IR converter responsibilities.
- `src/detectors/pr_miner/{mod, apriori, extract_rust,
  extract_python}.rs` — already split per language; IR converter
  replaces the `extract_*` files.
- `src/detectors/config_interaction.rs` — R-1.d target for
  relocation into `src/detectors/lang/`. Stays on raw tree-sitter
  (Pattern B, ir-v0.md F5 escape hatch).
- `src/baselines.rs` (931 LOC), `tests/baselines.rs` (364 LOC),
  `baselines/{sourcerercc,pybuglab}/`, `tests/fixtures/baselines/`
  — R-1.e retirement targets.
- `tests/citations_consistency.rs`, `tests/corpus_shape.rs`,
  `tests/wiring_consistency.rs` — touched by R-1 but shape
  preserved; only fixture / dispatch updates.
- `src/lsp.rs` — `scan_buffer` migrates to IrFile; the F5
  partial-parse cache change lands in R-1.c.
