# Rice 2017 replication specification v0 (DRAFT scaffold)

Author: ktrysmt
Date: 2026-MM-DD (set when section 0 placeholders are resolved)
Project: cntrdct Track B (replication of Rice et al., ICSE 2017)
Status: DRAFT scaffold. Drafted in
`research/projects/B_rice_replication/replication-spec-v0.md` without
direct access to the original paper. All metric definitions, section
references, and quantitative claims attributed to Rice et al. are
marked with `[verify Rice §X]` until the user (or any reader with
paper access) cross-checks them against
`rice-icse-2017` (CITATIONS.md). The expected output of that pass is
`research/projects/B_rice_replication/replication-spec-v1.md`, with
`v0` retained as the audit-trail copy of the unverified draft.

## 0. Verification status (load-bearing)

This document was drafted by an LLM agent (Claude) that did not have
the Rice 2017 PDF in its context. The scaffold therefore relies on:

- The cntrdct arg-swap detector spec (`docs/spec/arg-swap-v0.md`),
  which encodes the Rice approach in its simplest form (F5 of that
  spec).
- The `Background` and `Method outline` sections of
  `research/projects/B_rice_replication/README.md`, which list
  candidate Rice metrics at a high level and cite one quantitative
  datapoint (38 percent type-check survival rate).
- The CITATIONS.md entry confirming author list and venue.

What this scaffold cannot supply without paper access:

- The exact metric set Rice et al. report (table numbers, precise
  definitions, denominators).
- Section numbers used in `[verify Rice §X]` tags.
- The precision / recall / firing-rate values reported per language
  (C++ vs. Java) so a Rust replication can be calibrated against
  them.
- The exact filter list Rice et al. apply before swap detection
  (e.g., overloading handling, alias resolution, type substitution).

Resolution path: the user (or a reviewer with paper access) reads
Rice 2017 once, walks each `[verify Rice §X]` tag in this scaffold,
and either confirms or corrects the placeholder text. The corrected
file is committed as
`research/projects/B_rice_replication/replication-spec-v1.md` with a
`Supersedes:` header pointing to v0. v0 is retained verbatim as the
audit trail of "what we claimed before reading the paper", which is
itself a useful artefact for the eventual replication paper's
methodology section.

## 1. Purpose

Define, before any replication code runs, the exact contract under
which Track B reports its results:

- Which metrics from Rice 2017 will be reported with Rust analogues.
- Which Rice 2017 measurements are deliberately not replicated
  (substitution rule, with rationale).
- How cntrdct's current arg-swap detector
  (`crates/detector-arg-swap`, spec at `docs/spec/arg-swap-v0.md`)
  maps onto the Rice detection rule, and what extension is required
  to surface the candidate-call population (not only the
  swap-confirmed subset).

The contract is intentionally narrow. cntrdct's arg-swap detector is
the simplest faithful realisation of the Rice rule (F5 of arg-swap-v0
spec); the replication does not aim to reproduce Rice's full pipeline
(alias analysis, type-aware substitution, abstract-name expansion).
Instead it asks: when the simplest form of the rule runs on Rust
corpora, how do the headline numbers compare?

## 2. Original paper digest (scaffold placeholders)

Citation key: `rice-icse-2017`. Authors: A. Rice, E. Aftandilian,
C. Jaspan, E. Johnston, M. Pradel, Y. Arroyo-Paredes. Venue: ICSE
2017.

Subject corpora (per CITATIONS.md and Track B README):

- C++ codebase, Google internal monorepo.
- Java codebase, Google internal monorepo.

Detection rule (per arg-swap-v0 §F5; the cntrdct port that paraphrases
the paper):

- Two-parameter functions only.
- For a call site with two simple-identifier arguments
  `(a0, a1)` and a single same-name definition with parameter names
  `(p0, p1)`, emit a finding iff
  `a0 == p1 && a1 == p0` AND NOT `(a0 == p0 && a1 == p1)`.

Publicly cited quantitative claim (per Track B README):

- 38 percent of swap candidates were ruled out by type checking
  before reaching code reviewers.
  `[verify Rice §X — exact wording, denominator, language breakdown]`

Other Rice metrics that the scaffold expects to find but cannot
enumerate without the paper:

- Per-language firing density (candidates per KLOC or per file).
  `[verify Rice §X — expect Table X.Y]`
- Per-language precision on a manually labelled sample.
  `[verify Rice §X — expect Table X.Y]`
- Distribution of swap candidates by parameter-type relationship
  (same type, related type, distinct type).
  `[verify Rice §X — expect a stacked bar or table]`
- Filtering / substitution rules applied before the binary swap
  test (overloading, generic instantiation, callable lookup).
  `[verify Rice §X — expect a Methodology section enumeration]`

If any of the above turns out to be absent from the actual paper,
delete the row in v1 and document the deletion in the v0 → v1 diff
section of v1.

## 3. Candidate metrics × Rust analogue

Each row is a Rice metric (placeholder), the proposed Rust analogue,
the cntrdct artefact that produces (or would produce) it, and the
verification tag.

| Rice metric (placeholder) | Rust analogue | cntrdct artefact | verification |
|---|---|---|---|
| candidate-call count per KLOC | Same. Rust SLOC counted via `tokei` (excluding `tests/` per Track A convention). Numerator = arg-swap candidates emitted by trace mode (Track B README Step 3). | extend `crates/detector-arg-swap` with `RICE_TRACE=1` candidate-emission path; aggregate via a dedicated cli-research subcommand (deferred to v1.1) | `[verify Rice §X — denominator (KLOC vs lines vs files)]` |
| candidate-call density per file-size bucket | Same. Bucket boundaries: `[<200, 200-1k, 1k-5k, >=5k]` SLOC per file. | Post-processing of trace output. Bucket boundaries are a Rust convention (small modules dominate Rust corpora); document the deviation in the v1 diff if Rice uses different boundaries. | `[verify Rice §X — bucket boundaries, file vs translation unit]` |
| post-type-check survival rate | Substitution rule (see §4.1). Default: report all current findings as "post-type-check survivors" since rustc compiled the corpus. Optional add-on: query rust-analyzer or chalk for parameter types and report a "type-distinct" subset that the simple rule should never have flagged. | The default reading needs no extension. The optional add-on is deferred (no v0 / v1.1 spec line item until the user decides). | `[verify Rice §X — exact denominator, whether 38 % is over candidates or over emitted swaps]` |
| manual precision on stratified sample | Same. Reuse Track A's rubric v1 (`research/projects/A_1000_crate/rubric-v1-draft.md`), `phase1_kappa_wilson.py`, and `phase1_precision.py`. Sample size 100-200 (Track B README Step 4); per-detector stratification collapses to detector = arg-swap. | rubric v1 (research-side, draft) + Track A scripts (research-side, shipped). The Phase 1 CSV schema applies unchanged. | `[verify Rice §X — Rice's sample size and rater protocol]` |
| per-language comparison (C++ vs Java vs Rust) | Reportable but exploratory only. Direct comparison is invalid for confirmatory claims (different corpora, different languages, different reviewer populations); Track B README §Method outline Step 5 already constrains this to "exploratory context". | None new; the comparison is a write-up activity, not a tooling activity. | `[verify Rice §X — ensure we cite their explicit numbers, not derived figures]` |

Metrics deliberately not replicated (substitution rule documented in
§4.2):

- Rice's alias analysis / pointer-name resolution (no direct
  analogue for Rust references; Rust's borrow checker subsumes the
  defect class differently).
- Rice's overload resolution heuristic (Rust has no function
  overloading; this filter is vacuous).
- Rice's abstract-name expansion (matching `srcPtr` against
  parameter `src` after stripping a hungarian-style prefix).
  cntrdct does straight lowercase comparison; document this as a
  conscious narrowing, not an oversight.

## 4. Substitution rules

### 4.1 Type-check survival

Rice 2017's 38 percent figure measures the fraction of detected
candidates that the host language's type system rejects, leaving
only the genuinely callable swap calls for the swap rule to
evaluate.

In Rust, every candidate the detector emits is, by definition, a
type-checked call: rustc compiled the corpus, so the call expression
type-checks under whatever inference happened at the call site. This
makes Rice's "post-type-check survival rate" not directly
replicable as defined, but it does enable a cleaner alternative:

- Default substitution: report `post_type_check_survival_rate = 1.0`
  for every candidate, with a footnote that Rust's borrow checker +
  type system collapse this filter to a triviality.
- Optional add-on: if the user wants a Rust-side analogue of the
  filter, instrument the detector with a "would-have-been-rejected"
  predicate by querying rust-analyzer for parameter types and
  excluding candidates where `type_of(p0) != type_of(p1)`. Report
  the fraction of candidates with `type_of(p0) == type_of(p1)` as
  the Rust analogue of post-type-check survival.

The optional add-on is the more interesting datapoint scientifically
(it estimates how much Rust's type system suppresses the defect
class), but it requires rust-analyzer integration that is out of
scope for v0 / v1.

### 4.2 Language-specific filters

Java / C++ filters in Rice 2017 that have no Rust analogue:

- Method overloading: Rust has none. Filter is vacuous.
- Macro expansion: Rust has procedural macros and declarative
  macros. cntrdct's arg-swap-v0 spec §F2 walks top-level `fn` items;
  it does not expand macros. Document this as `notes:
  "macro-call-site only"` in the trace output. Rice's analogue (if
  any) goes in v1 once the paper is read.
- Pointer / reference aliasing: Rust borrow checker constrains
  aliasing differently. The replication does not attempt a
  Rust-equivalent alias analysis.

Rust-specific filters added by cntrdct that have no Rice analogue:

- Skip definitions with `_`-prefixed parameter names
  (arg-swap-v0 §F2). Rationale: Rust convention for unused
  parameters; the Rice rule is undefined when one of the parameter
  names is conventionally meaningless.
- Skip non-identifier callees (qualified paths, method calls,
  closures stored in variables; arg-swap-v0 §F3). Rationale: the
  same-file resolution rule (§F4) cannot disambiguate these without
  full name resolution; Rice 2017 has the equivalent constraint
  but with different syntactic boundaries.

These deliberate narrowings are documented in arg-swap-v0
spec §`Non-goals`. The v1 replication-spec must explicitly state
that these narrowings shrink the comparable subset (so the
replication's firing density should be a lower bound relative to a
hypothetical full Rice-faithful Rust port).

## 5. cntrdct artefacts that already realise (or will realise) each Rice step

### 5.1 Already realised in `crates/detector-arg-swap`

- F2 (definition extraction): top-level 2-arg `fn` items.
- F3 (call-site extraction): 2-arg calls with simple-identifier
  callees and simple-identifier arguments.
- F4 (resolution): same-file, single-match.
- F5 (swap detection): the Rice binary swap rule (lowercased name
  comparison; permits-no-correct-order clause).
- F6 (finding shape): emits `citation_keys` including
  `rice-icse-2017`; this binds the replication to the original
  paper at the artefact level.
- T1-T11 (test plan): seed-level positive and negative cases
  including cross-file resolution.

### 5.2 To be added under Track B Step 3

- A trace-mode emission path that surfaces every call considered as
  a candidate and the reason it was kept or skipped. The Track B
  README specifies this as `RICE_TRACE=1` env flag or a separate
  `detect_with_trace` API. Public Detector trait stays unchanged.
- A reason taxonomy: at minimum
  `kept`, `skipped:single-arg`, `skipped:non-identifier-arg`,
  `skipped:no-definition`, `skipped:multi-definition`,
  `skipped:correct-order`, `skipped:no-name-match`. The taxonomy is
  the input to the candidate-call density metric.

The trace path is technical-side work (modifies `crates/`); it is
out of scope for the research-side scaffold. The contract above is
specified here so the technical-side patch knows what fields must
appear in trace output for the replication's metric computation to
be unambiguous.

### 5.3 To be added under Track B Step 4

- Run the Phase 1 labelling pipeline from Track A:
  `cntrdct-research stratified-sample` over the trace output,
  `build_phase1_csv.py` to produce a blind labelling CSV,
  rubric v1 + two raters, `phase1_kappa_wilson.py` for kappa,
  `phase1_precision.py` for per-detector precision (collapses to a
  single detector here).
- The Phase 1 protocol (rubric v1 §7) reuses the same round-1 /
  round-2 / round-3 adjudication ladder. No new rubric is needed.

## 6. Seed corpus calibration

Before running the replication on any wild corpus, the trace path
must reproduce the seed corpus expectations:

- `arg_swap_001` ... `arg_swap_010` (10 positive cases shipped under
  `benchmarks/corpus/`): every fixture must produce exactly one
  swap-confirmed finding and a non-empty trace record per call.
- T1-T11 from arg-swap-v0 spec: every test must still pass with
  the trace path enabled (regression on the public detector
  contract is unacceptable).

If either calibration fails, the trace path is incorrect and the
replication run is paused. This is a hard gate, not a warning.

## 7. Out of scope (v0)

- Comparison with PR-Miner (`li-zhou-fse-2005`). PR-Miner is a
  superset of arg-swap; replication of its full association-rule
  miner is a separate research question and not part of Track B.
- N-ary swap detection (3+ parameters). Out of scope by
  arg-swap-v0 §`Non-goals`. Rice's treatment of n-ary
  `[verify Rice §X]` may differ; if so, document in v1 and either
  expand cntrdct's detector under a feature flag or accept the
  narrower comparable subset.
- Cross-crate / cross-module resolution. Out of scope by
  arg-swap-v0 §`Non-goals`. Rice's same-translation-unit
  scope `[verify Rice §X]` is the equivalent constraint.
- Type-aware swap detection. The optional add-on in §4.1 is the
  only scope window where types are queried.
- Defect persistence study (do swaps survive in the codebase
  long-term?). Out of scope; Track B reports cross-sectional firing
  rate only.

## 8. Promotion path (v0 → v1)

1. User (or any reader with paper access) reads Rice 2017 once.
2. For each `[verify Rice §X]` tag in this file, the reader either:
   - Confirms the placeholder text and replaces the tag with a
     concrete Rice section / table / page number, or
   - Corrects the placeholder text and records the correction in
     the v0 → v1 diff section of v1.
3. The reader copies this file to
   `research/projects/B_rice_replication/replication-spec-v1.md`,
   with a `Supersedes:` header pointing to
   `research/projects/B_rice_replication/replication-spec-v0.md`,
   and applies the corrections.
4. v0 is retained verbatim. The audit trail of "what we claimed
   before reading the paper" becomes a section of the eventual
   replication paper's methodology (or a Threats to Validity
   bullet).
5. Track B Step 2 (corpus assembly) starts only after v1 is
   committed. Do not run replication code against the v0 scaffold;
   any numbers produced under v0 are unverified and must be
   discarded.

## 9. Out-of-scope follow-ups

- A v1.1 of this spec that adds the rust-analyzer-based optional
  add-on (§4.1) for type-aware filtering. Whether to pursue this
  depends on the v1 reader's judgement about the scientific value of
  the Rust-specific type-check survival figure.
- Aggregation tooling: a `cntrdct-research` subcommand that ingests
  the trace output and emits the per-KLOC, per-bucket, and
  precision tables described in §3. Schema is specified in §5.2;
  implementation is deferred.
- Paper-level write-up of the per-language comparison (§3 row 5).
  Track B README Step 5 already commits this to "exploratory
  context"; no new spec material here.

## 10. References

- `rice-icse-2017` — Rice et al., "Detecting Argument Selection
  Defects", ICSE 2017. (CITATIONS.md.)
- `li-zhou-fse-2005` — Li & Zhou, "PR-Miner", ESEC/FSE 2005.
  Cited by arg-swap-v0 §F6 alongside Rice; not replicated by
  Track B.
- `docs/spec/arg-swap-v0.md` — cntrdct's arg-swap detector
  contract. The replication is built on top of this contract; any
  change to the spec must be reflected here (or in v1).
- `research/projects/B_rice_replication/README.md` — Track B plan
  and effort estimate. The First concrete step section (line 109)
  is the trigger that produced this scaffold.
