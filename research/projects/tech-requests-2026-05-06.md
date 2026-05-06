# Technical-side requests from research track (2026-05-06)

Author: ktrysmt
Date: 2026-05-06
Source: research-track work over 2026-05-04..2026-05-06 sessions
Status: pending technical-side action
Layout context: technical project is now a single `[package]` at the
repo root with `src/` layout (per CLAUDE.md root §1, restated 2026-05-06
after the workspace collapse for v0.2.0-beta.0). All file references
below use the post-collapse paths.

This document captures specific changes the research track needs on the
technical project, in a form a technical-side session can execute
end-to-end without back-and-forth. Each request is self-contained: it
lists the scope, acceptance criteria, the technical files involved
(post-collapse paths), the test plan, and the trigger condition that
determines when the request becomes blocking.

The request set includes only changes that materially affect
research-track outputs: preregistration test gates and detector trace
paths. Pure research-side tooling extensions are excluded; see
"Excluded from this document" at the end.

## TECH-1 — prereg_consistency skip filter for failure-modes

### Trigger

When `prereg/<YYYY-MM-DD>-failure-modes-v1.md` lands at the repo root.
Promotion is downstream of USR-1 (rubric v1 §0 confirmation) and
USR-2 (failure-modes v1 promotion, synced with USR-1) per
`research/projects/PLAN.md`.

### Background

`tests/prereg_consistency.rs` picks the alphabetically last `*.md` in
`prereg/` as the canonical OSF preregistration and validates it
against the OSF schema. Sibling artefacts that do not follow the OSF
schema (labelling rubrics, addenda, failure-mode taxonomies) MUST be
filtered out by stem pattern; otherwise the test trips when a
non-OSF file sorts last and the consistency check applies the OSF
schema to it.

The current filter at `tests/prereg_consistency.rs:91` excludes
stems containing `-rubric-` and `-addendum`. After the v1 promotion,
the filter must also exclude `-failure-modes-`.

This need was foreseen in
`research/projects/A_1000_crate/failure-modes-v1.md` §8 (promotion
checklist, item 3): "the skip filter is currently `-rubric-` only.
`-failure-modes-` is OSF-schema-incompatible, so the skip filter
must be extended on the technical side at promote time".

### Scope

Edit one line in `tests/prereg_consistency.rs` to add
`-failure-modes-` to the existing filter:

```rust
// Current (tests/prereg_consistency.rs:91):
!stem.contains("-rubric-") && !stem.contains("-addendum")

// After:
!stem.contains("-rubric-")
    && !stem.contains("-addendum")
    && !stem.contains("-failure-modes-")
```

No other code changes. The test machinery itself is correct; only
the filter list needs the extension.

### Acceptance

- The test continues to pass with the canonical OSF prereg
  (`prereg/2026-05-03-osf-prereg.md` plus the `-phase0-addendum`).
- The test correctly skips the new
  `prereg/<DATE>-failure-modes-v1.md` file.
- The test correctly skips the existing
  `prereg/<DATE>-labelling-rubric-v1.md` file (regression check;
  the `-rubric-` exclusion must remain functional).
- A smoke step: introduce a stub
  `prereg/9999-12-31-failure-modes-v1.md` with arbitrary content,
  confirm the test passes, then remove the stub. Optional but
  recommended; verifies the filter actually fires on the new
  pattern.

### File references (post-collapse paths)

- `tests/prereg_consistency.rs` — the consistency test (filter at
  line 91 today; line numbers may drift if the file is touched
  between now and promotion).
- `prereg/<DATE>-failure-modes-v1.md` — the new artefact, lands
  with the v1 promotion commit.
- `research/projects/A_1000_crate/failure-modes-v1.md` §8 — the
  promotion checklist that anticipates this change.

### Effort

5 min change + 5 min smoke test. Total under 15 min.

### Commit

Convention: bundle with the promotion commit (or as the immediately
following commit). Recommended message:

```
chore(prereg): extend skip filter for failure-modes-v1 promotion

The consistency test at tests/prereg_consistency.rs picks the
alphabetically last *.md in prereg/ as the canonical OSF prereg
and validates it against the OSF schema. failure-modes-v1.md
follows a sibling schema; add -failure-modes- to the existing
-rubric- / -addendum filter list so the test does not trip on
the new artefact.
```

## TECH-3 — RICE_TRACE trace path on the arg-swap detector

### Trigger

When USR-3 (Rice paper read + replication-spec v1) lands. Without
v1's metric verification the trace reason taxonomy could be wrong;
implementing the trace path against an unverified taxonomy would
require redoing the test fixtures and the trace records.

The pending Rust-analyzer integration (v1.1 addendum) is downstream
of this trace path, NOT a precondition: the v1.1 addendum's
`rice-types` subcommand consumes the trace JSONL output produced
by this patch.

### Background

The Track B Rice 2017 replication study needs not just the
swap-confirmed findings the arg-swap detector currently produces
on `Detector::detect()`, but every CALL the detector considered as
a swap candidate, plus the reason it was kept or skipped. This is
the input to the per-KLOC density and per-bucket distribution
metrics specified in
`research/projects/B_rice_replication/replication-spec-v0.md` §3
and §5.2.

The `Detector::detect()` public path stays unchanged. Existing
behaviour, existing fixture results, and existing test suite are
byte-identical before and after this patch. The trace path is a
parallel emission channel gated behind an env flag or a separate
API on the implementation type, never on the public trait.

### Scope

Add a trace mode to the arg-swap detector that emits, for every
call considered, a record with:

- the (file, span, callee identifier, arg-index) tuple,
- the reason: one of the controlled-vocabulary values in the next
  section,
- (deferred to v1.1 addendum) parameter type strings from
  rust-analyzer.

The trace path is gated either by `RICE_TRACE=1` env var (read
inside the detector module before the per-call work begins) or by
a separate `detect_with_trace(ctx, sink: &mut TraceSink)` method
on the impl type (NOT on the `Detector` trait). Choose whichever
is less invasive given the current detector wiring; the env-var
form is acceptable if `detect()` already constructs all candidate
information up to the swap-rule step.

### Reason taxonomy (frozen by replication-spec-v0 §5.2)

```
kept
skipped:single-arg
skipped:non-identifier-arg
skipped:no-definition
skipped:multi-definition
skipped:correct-order
skipped:no-name-match
```

The v1.1 addendum
(`research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`
§3) will add `skipped:lsp-no-type` to the taxonomy when the
rust-analyzer integration ships. This patch does NOT need to emit
that value yet; it can be added in a follow-up after the LSP
integration lands.

### Output format

JSONL on stdout (or to a sink the caller picks) when the trace
path is active. Each line is one JSON object with fields:

```json
{
  "file": "<relative path under scan root>",
  "start_line": 42,
  "start_col": 8,
  "end_line": 42,
  "end_col": 24,
  "callee": "frobnicate",
  "arg_index_pairs": [[0,1]],
  "arg_names": ["src", "dst"],
  "param_names": ["dst", "src"],
  "reason": "kept"
}
```

For `skipped:*` reasons, `arg_names` and `param_names` may be
empty (e.g., `skipped:single-arg` has no swap pair to report).
The exact field set is the technical-side implementer's
discretion as long as the reason taxonomy is honoured and each
record carries enough location information for the
research-side aggregator to join against the corpus.

### Acceptance

- Existing arg-swap unit tests (`tests/detector_arg_swap.rs`)
  pass byte-identically. The public `Detector::detect` path is
  unchanged.
- A new trace-mode test (suggested location:
  `tests/detector_arg_swap_trace.rs`, or extending the existing
  file) demonstrates each reason taxonomy value is emitted on a
  synthetic input that should produce that reason. Coverage at
  least one case per reason.
- The 10 existing seed fixtures
  (`benchmarks/corpus/files/arg_swap_001`..`_010`) each produce
  exactly one `kept` record with non-empty `arg_names` and
  `param_names` when the trace path is enabled. This is the
  calibration baseline for Track B (per
  `replication-spec-v0.md` §6).
- Public API surface: the `Detector` trait does not gain new
  methods. The trace mode is opaque to non-research consumers.

### File references (post-collapse paths)

- `src/detectors/arg_swap.rs` — the detector implementation.
- `tests/detector_arg_swap.rs` — existing detector-scope test
  file; trace tests can extend or live alongside.
- `docs/spec/arg-swap-v0.md` — the existing spec; the trace
  contract should be documented as a new section (suggested:
  §F7 "Trace mode (research-only)").
- `benchmarks/corpus/files/arg_swap_*.rs` — seed fixtures used
  for the calibration baseline.

### Effort

1-2 days. The reason taxonomy is the unfamiliar work — every
existing skip path in the detector needs to be tagged with the
right taxonomy value. The emission path is straightforward.

### Commit

Convention: `feat(detector-arg-swap): RICE_TRACE trace path` per
the existing scope naming (cf. `929ed4a feat(pr-miner): ...`).
The commit body should reference both
`replication-spec-v0.md` §5.2 (taxonomy frozen) and
`replication-spec-v1.1-rust-analyzer.md` §3 (downstream consumer)
so the v1.1 promotion path is auditable from the commit log
alone.

### Dependencies

Upstream:
- USR-3 (paper read + v1 promote). Required before this patch
  ships. The taxonomy is `[verify Rice §X]`-tagged in v0; v1
  resolves whether each value matches Rice's filter list.

Downstream (research-side, blocked on this patch):
- `cntrdct-research rice-types` subcommand (LSP integration per
  v1.1 addendum §5).
- `cntrdct-research rice-aggregate` subcommand (joins trace
  output and types side-car per v1.1 addendum §5 / v0 §9).
- Track B Step 3+ corpus run.

## Excluded from this document

### TECH-2 (build_phase1_csv.py extension) — research-side, not technical

The original handoff catalogued
`research/projects/A_1000_crate/scripts/build_phase1_csv.py`
extension (emit empty `consensus_label`, `round`, `tiebreak_rater`,
`failure_mode`, `failure_mode_notes` columns) as TECH-2. This was a
miscategorisation: the script lives under `research/`, depends only
on the research-side schema (rubric v1 + failure-modes v1), and has
no technical-package counterpart. The change will be done on the
research side around the time USR-1 lands, in the same commit as the
rubric v1 promotion (or as an immediately following research-side
commit).

### CLI flag for trace mode

Whether `cntrdct scan --trace=arg-swap` (or similar) is exposed at
the CLI is an implementation detail for the technical side to
decide. The research-side aggregator can call the detector's trace
API via a small Rust harness; CLI exposure is convenience, not
contract. If the technical side prefers env-var-only gating
(`RICE_TRACE=1 cntrdct scan ...`), that is acceptable to the
research side as long as the JSONL output reaches stdout.

## References

- `research/projects/PLAN.md` §"Gating user decisions (USR-1..3)"
  — full context for USR-1, USR-2, USR-3.
- `research/projects/A_1000_crate/failure-modes-v1.md` §8 —
  promotion checklist that triggers TECH-1.
- `research/projects/B_rice_replication/replication-spec-v0.md`
  §5.2 — frozen reason taxonomy for TECH-3.
- `research/projects/B_rice_replication/replication-spec-v1.1-rust-analyzer.md`
  §3, §5 — downstream rust-analyzer integration that consumes the
  TECH-3 trace output.
- `CLAUDE.md` (repo root) — current technical-package layout (§1)
  and commit conventions (§3).
- `tests/prereg_consistency.rs:91` — current skip filter (line
  number subject to drift).
- `src/detectors/arg_swap.rs` — current arg-swap detector.

## Status tracking

When each request is implemented, the technical-side commit should
add a status note here (or this file should be moved to a `done/`
subdirectory). Until then, the research side treats both as
pending.

| ID | Status | Last update |
|---|---|---|
| TECH-1 | done (landed pre-emptively; filter is no-op until `prereg/<DATE>-failure-modes-v1.md` is placed) | 2026-05-07 |
| TECH-3 | pending | 2026-05-06 |
