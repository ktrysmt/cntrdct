# Wild-corpus failure-modes dry-run (2026-05-06)

Author: ktrysmt
Date: 2026-05-06
Project: cntrdct Track A
Status: pre-promote audit. The exercise's outputs (v1 vocabulary
additions) have been merged into `failure-modes-v1.md` in the same
commit as this report; the report itself is retained as audit
trail of what was tested before promotion.

## 1. Why this dry-run

`failure-modes-v1.md` defines a controlled vocabulary of FP failure
modes per detector. Before the v1 vocabulary is promoted to
`prereg/`, we want evidence that the vocabulary actually covers the
FPs the detectors produce on real Rust code. The Phase 0 pilot
(top-100 crates, single rater) was the nominal source for this
dry-run, but `data/phase0/labelling-rated.csv` is gitignored and
not present in the working tree; the rated CSV either lives on
another machine or has not been produced. We substitute the P-1 β
wild corpus (`benchmarks/wild-corpus/`), which has a more
substantial labelled FP set: 124 hand-labelled FPs across 270
files, fully tracked in the repository. The substitution is sound
because both data sources are FP-only collections from the same
five Layer 1 detectors; only the corpus shape and rater protocol
differ.

## 2. Method

The wild-corpus README (`benchmarks/wild-corpus/README.md`
sections "Labelling triage (v0, 2026-05-06)") records 124 FPs
across three detectors. We classify each cluster against the
`failure-modes-v1.md` section 3 vocabulary and record which v1
modes apply, which clusters lack a clean fit, and what additions
would close the gap.

The mapping is at the cluster level rather than per finding:
within each cluster the README's prose makes the failure mode
unambiguous (e.g. all 10 unreachable-after-terminator FPs share
the "cfg-gated alternative returns" pattern). Cluster-level
mapping is consistent with how the v1 aggregator
(`scripts/phase1_failure_modes_aggregate.py`) groups its output;
per-finding labelling will happen in Phase 1 against this
vocabulary.

## 3. Per-detector classification

### 3.1 unreachable-after-terminator (10/10 FP)

All ten findings fire on Rust's `cfg`-gated alternative-return
idiom: a `#[cfg(feature = "preserve_order")] return ...;` followed
by `#[cfg(not(feature = "preserve_order"))] return ...;`. The
detector treats both as sequential statements; in reality exactly
one is alive per cfg evaluation.

v1 mode: `cfg-gated-divergence` (failure-modes-v1.md section
3.4). Direct match. No vocabulary gap.

Coverage: 10/10 (100%).

### 3.2 comment-code (2/2 FP)

Both FPs fire on `parking_lot_core__parking_lot.rs` lines 736 and
892. The doc text describes the contract of the `validate` /
`callback` parameter ("must not panic"); the detector reads the
"panic" token as a function-level claim about the function's own
behaviour. The function does not panic; the comment is correct;
the detector misattributed the comment's subject.

v1 candidates considered:

- `higher-abstract-intent`: about scope (comment describes whole
  function, visible code is one branch). The wild FP is about
  subject (which entity the contract is on), not scope. Mismatch.
- `future-work-marker`, `doctest-divergence`, `translation-
  ambiguity`, `stale-but-harmless`: none describes a subject-
  attribution error. Mismatch.

Vocabulary gap. Added in this dry-run as
`parameter-contract-misread` (failure-modes-v1.md section 3.3).

Coverage after addition: 2/2 (100%).

### 3.3 clone-drift (112/112 FP)

The 112 findings concentrate on parser combinators and iterator
combinators across crates: nom (37), winnow (28), itertools (21),
regex_syntax (7), memchr (5), base64 (6), serde_json (5), plus
single-finding outliers in 14 other files. Every finding reports
"function diverged from 24 similar siblings" because the global
similarity pool caps at `MAX_RELATED = 24` and spans the entire
corpus across crate boundaries.

v1 candidates considered:

- `boilerplate-shape-only`: defined for short `match` arms,
  `From` impls, trait forwards. Parser combinators are
  substantive functions, not boilerplate. The shape similarity
  is real but the conceptual roles are not "different" — both
  are parsing combinators. The mode does not fit because the
  problem is pool boundary, not boilerplate-vs-real-logic.
- `type-or-cfg-justified-drift`, `metadata-only-drift`,
  `auto-generated-clone`: none describes a cross-crate pool
  scope error. Mismatch.
- `cross-file-context-resolved`: requires adjudicator context
  (phase1-context.json + neighbouring crate inspection) to
  resolve the drift as intentional. The wild FPs do not require
  any such resolution; the comparison itself is invalid because
  the pool boundary is wrong. Mismatch.

Vocabulary gap. Added in this dry-run as
`cross-crate-pool-mismatch` (failure-modes-v1.md section 3.1).
Distinct from `boilerplate-shape-only` because:

- `boilerplate-shape-only` says "shape similar, conceptual roles
  differ". The pool comparison is meaningful but the result is FP
  because the entities are too small / formulaic to carry signal.
- `cross-crate-pool-mismatch` says "shape similar, conceptual
  roles match, but the comparison crosses a boundary the
  detector should respect". The pool comparison is not
  meaningful in the first place.

The new mode has implementation implications: Phase 1
adjudicators using `cross-crate-pool-mismatch` are flagging a
detector-config problem (pool too broad), not a per-finding
interpretation problem. The β paper's clone-drift v0.x roadmap
should treat high `cross-crate-pool-mismatch` count as the
strongest signal that pool restriction (intra-crate or
intra-file) is the right next iteration.

Coverage after addition: 112/112 (100%).

### 3.4 arg-swap (0/0)

Zero findings on the wild corpus. No signal. The v1 vocabulary
for arg-swap is unchanged; whether its modes will be exhausted by
Phase 1 data remains an open empirical question.

### 3.5 config-interaction (0/0)

Zero findings. Same conclusion as arg-swap.

## 4. Coverage summary

| detector | FPs in wild corpus | covered by v1 (pre-dry-run) | covered after additions |
|---|---|---|---|
| unreachable-after-terminator | 10 | 10/10 | 10/10 |
| comment-code | 2 | 0/2 | 2/2 |
| clone-drift | 112 | 0/112 | 112/112 |
| arg-swap | 0 | n/a | n/a |
| config-interaction | 0 | n/a | n/a |
| total | 124 | 10/124 (8%) | 124/124 (100%) |

Pre-dry-run coverage was 8 percent. The two additions
(`parameter-contract-misread` for comment-code,
`cross-crate-pool-mismatch` for clone-drift) close the gap to 100
percent on the wild corpus.

The 100 percent coverage figure is on a single corpus and is not
a guarantee that Phase 1 (top 1000 crates, fresh sample) will
also be fully covered. The vocabulary is calibrated against
parser-combinator / iterator-combinator FP shapes; library
domains the wild corpus under-samples (web frameworks, async
runtimes, databases) may surface modes neither corpus has seen.
The v1.x promotion path in `failure-modes-v1.md` section 5
remains the mechanism for closing those gaps when they appear.

## 5. Implications for Phase 1

- Phase 1 adjudicators starting from the post-addition v1
  vocabulary should expect a small but non-zero `other` rate
  driven by domain shift between the wild corpus and the top-
  1000 sample. Section 2.3 of `failure-modes-v1.md` already
  specifies that 5 or more `other` rows of the same shape
  trigger a v1.x file.
- The `cross-crate-pool-mismatch` mode is detector-config-shaped
  rather than finding-interpretation-shaped. Its appearance in
  Phase 1 data is a stronger signal for "fix the detector pool"
  than for "fix the finding's per-call rationale". The β paper's
  v0.x roadmap should weight detector-config fixes accordingly.
- The two additions are NEW (no v0 ancestor), tagged
  `(wild-corpus dry-run, 2026-05-06)` in the v1 doc so the
  audit trail of why each mode entered the vocabulary is
  preserved.

## 6. Artefacts updated by this dry-run

- `research/projects/A_1000_crate/failure-modes-v1.md` section
  3.1: added `cross-crate-pool-mismatch`.
- `research/projects/A_1000_crate/failure-modes-v1.md` section
  3.3: added `parameter-contract-misread`.
- `research/projects/A_1000_crate/scripts/phase1_failure_modes.py`:
  vocabulary mapping extended with both new modes.
- `research/projects/A_1000_crate/scripts/test_phase1_failure_modes_aggregate.py`:
  test_vocabulary_covers_five_detectors_with_shared_mode still
  green; assertion `len(modes) >= 4` is satisfied for both
  detectors at their new mode counts (clone-drift: 6, comment-
  code: 6).

All 56 Phase 1 Python tests continue to pass after the additions.

## 7. References

- `benchmarks/wild-corpus/README.md` sections "Labelling triage
  (v0, 2026-05-06)" and "Reported metrics" for the source data.
- `prereg/2026-05-05-osf-prereg-phase0-addendum.md` for the
  Phase 0 single-rater protocol the wild corpus substitutes.
- `failure-modes-v1.md` (this dry-run's calibration target).
- `phase1_failure_modes.py` for the runtime vocabulary
  enforcement.
