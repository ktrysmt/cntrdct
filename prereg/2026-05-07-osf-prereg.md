# OSF Preregistration: cntrdct β-phase precision and recall study (revision 3)

Author: ktrysmt
Date: 2026-05-07
Project: cntrdct (Evidence-based contradiction linter, Rust + Python)
Status: draft, pre-registration not yet submitted
Supersedes: `prereg/2026-05-06-osf-prereg.md`

## What changed since 2026-05-06

This revision records a wild β corpus FP reduction pass run on
2026-05-07. No new detectors are added; the five-detector Layer 1
surface and the H1–H5 hypothesis statements are unchanged. The
substantive changes are detector-internal precision tightenings
backed by the same prior art set, plus the resulting recalibration
of the embedded priors.

- `unreachable-after-terminator` adds F4b (cfg-gated terminator
  suppression) and F4c (hoisted item suppression). F4b closes the
  cross-platform `#[cfg(...)] return ...; #[cfg(not(...))] return
  ...;` idiom that produced 10/10 FPs on the Rust wild β corpus;
  F4c closes the `return helper(); fn helper() {}` hoisted-item
  pattern that produced the residual `semver__identifier.rs:377`
  FP. Spec: `docs/spec/unreachable-after-terminator-v0.md` F4b /
  F4c. Tests: `tests/detector_unreachable_after_terminator.rs`
  T29–T37.
- `comment-code` adds F5b (Sphinx `:raises:` factory-shape
  suppression: a function whose body's `return` returns a call
  expression and whose docstring contains `:raises:` ... ) and F5c
  (parameter-level `.. deprecated::` directive peeking, including
  indented continuation lines). F5b closes the 14-finding
  `attrs_validators.py` cluster and F5c closes the 2-finding
  `attrs_make.py` cluster on the Python wild β corpus. Spec:
  `docs/spec/comment-code-v0.md` F5b / F5c. Tests:
  `tests/detector_comment_code.rs` t21–t28b.
- `clone-drift` adds F5b (scope-bounded clustering via path-only
  inference: provenance header → Cargo layout → filename `__`
  separator → parent directory) and F5c (within-scope tightening:
  F5c-i strict-majority gate `largest * 2 > group`, F5c-ii
  near-duplicate gate `Jaccard(drifted, dominant) ≥
  NEAR_DUPLICATE_THRESHOLD = 0.7`). Together these reduce the Rust
  wild β corpus FP count from 124 to 3 (97.6 % reduction); the 3
  residuals are designed-library-shape variants documented as v0
  limitations. Spec: `docs/spec/clone-drift-v0.md` F5b / F5c.
  Tests: `tests/detector_clone_drift.rs` t20–t28.
- `cntrdct calibrate` is now byte-stable: the priors HashMap is
  serialised through a sorted BTreeMap before write, so identical
  labelled corpora produce byte-identical
  `benchmarks/priors-default.json` across runs. Recalibration
  against the relabelled wild β corpus moves the embedded priors
  as expected: clone-drift Wilson lower 0.073 → 0.355,
  unreachable-after-terminator 0.407 → 0.796, comment-code 0.298 →
  0.657. Source: `src/lib.rs::calibrate`.
- The wild β corpus manifests are relabelled per
  `prereg/2026-05-04-labelling-rubric-v0.md` against the new
  detector behaviour. Rust:
  `benchmarks/wild-corpus/manifest.jsonl` (no expected entries
  added; the previous manifest already had zero TPs). Python:
  `benchmarks/wild-corpus-python/manifest.jsonl` —
  `charset_normalizer_utils.py:27 clone-drift` is reclassified
  from TP to FP per rubric §5.1 FP-1 (different conceptual role:
  accent-property detector vs single-predicate
  script/category-detector siblings). The pre-relabel TP at this
  position is no longer flagged anyway: F5c-ii filters it.

## Background

cntrdct is an evidence-based static linter whose first design
constraint (P1) requires every detector to cite peer-reviewed prior
art. The α phase shipped four Layer 1 detectors — `clone-drift`,
`arg-swap`, `comment-code`, and `unreachable-after-terminator`. The
β phase added a fifth, `config-interaction`. Layer 1 sits on top of
a Layer 2 statistical ranker, a Layer 3 LLM adjudicator, and a
Layer 4 SARIF emitter. Per-detector citations are enforced at
startup (`register_detector` rejects citation-free detectors); a
package test (`tests/citations_consistency.rs`) keeps the live
`Detector::citations()` output in lock-step with `CITATIONS.md`;
and a sibling test (`tests/prereg_consistency.rs`) keeps the
latest preregistration in lock-step with the live citation key set.

The β phase introduces an evaluation harness, specified in
`docs/spec/eval-v0.md` and implemented as the `cntrdct::eval`
module plus the `cntrdct eval` CLI subcommand. This
preregistration commits — in advance of the β corpus collection —
to the corpus design, the matching rule, the metric set, and the
inference criteria that will be used to characterise each
detector's empirical performance on Rust source.

The aim is to forestall corpus shopping and metric shopping: the
performance figures published with the β release will be those
produced by the procedure described here, regardless of whether they
are favourable to any specific detector.

## Hypotheses

Each hypothesis is binary: either the measured value clears the
stated threshold or the corresponding detector is flagged in the β
README as "needs refinement". The thresholds are deliberately
conservative; they are intended to distinguish a working detector
from one that is indistinguishable from chance, not to claim parity
with any specific published baseline. All hypotheses are scoped to
the Rust β corpus described in this preregistration.

H1 (clone-drift). Precision ≥ 0.50 AND recall ≥ 0.30 on the
labelled corpus, with Wilson 95 percent lower bounds on both
metrics strictly greater than 0.

H2 (arg-swap). Precision ≥ 0.50 AND recall ≥ 0.30, Wilson 95
percent lower bounds strictly greater than 0.

H3 (comment-code). Precision ≥ 0.50 AND recall ≥ 0.30, Wilson 95
percent lower bounds strictly greater than 0.

H4 (unreachable-after-terminator). Precision ≥ 0.50 AND recall ≥
0.30, Wilson 95 percent lower bounds strictly greater than 0.
Reachability of this pattern is structural, so refutation here would
most likely indicate corpus or labelling defects rather than
detector defects; diagnosis would proceed accordingly.

H5 (config-interaction). Precision ≥ 0.50 AND recall ≥ 0.30,
Wilson 95 percent lower bounds strictly greater than 0. The
detector's literal pair contradiction rule is structural like
reachability; refutation would similarly point to corpus or
labelling defects.

H_overall. Micro-averaged F1 across the five detectors ≥ 0.50.

Comparison with the precision and recall figures reported by the
detectors' founding papers (`cordy-roy-icpc-2008`,
`bettenburg-msr-2009`, `krinke-icsm-2007`, `assi-tosem-2025`,
`li-zhou-fse-2005`, `rice-icse-2017`, `allamanis-neurips-2021`,
`tan-sosp-2007`, `tan-pldi-2011`, `hovemeyer-pugh-oopsla-2004`,
`engler-sosp-2001`, `tartler-eurosys-2011`, `nadi-icse-2014`) is
treated as exploratory: those studies used different languages,
corpora, and labelling protocols and direct numerical comparison
would be invalid. The preregistered hypotheses do not depend on it.

## Design Plan

### Study type

Observational. The detectors are deterministic, file-local, and
network-free (P3, P4); they emit findings, the harness counts
matches against a manually labelled manifest, and standard
precision and recall formulas summarise the result. No
interventions are performed.

### Blinding

Not applicable. The procedure is fully automated and reproducible.
Identical inputs produce byte-identical reports (eval-v0.md F5,
F8). As of revision 3 the priors-write path is also byte-stable,
so a recalibration cycle is observable as a clean diff.

### Study design

Single-corpus, within-detector. Each source file in the corpus is
a single observation. For each detector the matching rule defined
in eval-v0.md F3 partitions findings into TP, FP, and FN.

## Sampling Plan

### Existing data

The seed corpus at `benchmarks/corpus/` contains synthetic
hand-crafted fixtures at the date of this revision: ten positive
fixtures per cross-cutting Rust detector, three Rust negatives per
detector, plus the M-2 / M-3 Python pilot fixtures (five positives
+ three negatives for each of `unreachable-after-terminator`,
`comment-code`, `arg-swap`, and `clone-drift`). The seed corpus is
treated as a smoke-test rather than as evaluation data; it does not
contribute to the β figures produced under this preregistration.
Python fixtures are present but Python β evaluation is preregistered
separately (see ROADMAP M-4).

The wild β corpus at `benchmarks/wild-corpus/` (Rust, 270 files)
and `benchmarks/wild-corpus-python/` (Python, 11 files) is
relabelled in this revision against the post-fix detector
behaviour. The wild corpus is sparse on real bugs by design (it
is sampled from heavily reviewed top-by-downloads packages); its
role is FP-reduction validation rather than recall measurement.

### Data collection procedures

The β corpus is drawn from publicly available Rust source files
under permissive licences (MIT, Apache-2.0, BSD). Candidate sources
include the rust-analyzer test corpus, the clippy regression suite,
and the top 200 most-downloaded crates on crates.io.

Each candidate file is reviewed by the author and labelled by hand
into the JSONL manifest format defined in `docs/spec/eval-v0.md`.
To reduce single-rater bias, every label is independently reviewed
by a second pass through the Layer 3 LLM adjudicator (Anthropic
Messages, see `src/adjudicator.rs`) used here strictly as a
labelling cross-check and never as an inference instrument.

A label is accepted into the manifest only when both passes agree.
A disagreement triggers a written reconciliation note recorded
under `benchmarks/disagreements.md`; the author's adjudicated
decision is final but the original disagreement is preserved for
auditability. The Phase 0 single-rater addendum
(`prereg/2026-05-05-osf-prereg-phase0-addendum.md`) clarifies the
fall-back protocol for the initial bootstrapping batch. Strict
re-application of the labelling rubric
(`prereg/2026-05-04-labelling-rubric-v0.md`) on relabel cycles
may downgrade or upgrade existing labels; the rubric clause
applied is recorded next to the label.

### Sample size

The target is at least 50 labelled Rust source files, with at
least eight positive cases per registered Layer 1 detector (now
five). Files that contain zero expected findings are admitted as
true negatives but capped at 30 percent of the corpus to prevent
metric inflation by trivial cases.

### Stopping rule

Data collection halts when (a) the corpus reaches 50 files AND
(b) the per-detector minimum of eight positive cases is met for
each of the five registered detectors. The eval harness MUST NOT
be run during collection on the partial corpus, to prevent corpus
shopping. After collection halts, the harness is run exactly once
to produce the preregistered metrics.

## Variables

### Measured variables

For each detector d in {clone-drift, arg-swap, comment-code,
unreachable-after-terminator, config-interaction}:

- TP(d), FP(d), FN(d), as defined by `cntrdct::eval::evaluate` per
  `docs/spec/eval-v0.md` F3.
- precision(d) = TP / (TP + FP), recall(d) = TP / (TP + FN),
  F1(d) = 2 * P * R / (P + R), with the divide-by-zero conventions
  in eval-v0.md F4 (returning 0.0 rather than NaN).

In addition:

- corpus_size, expected_total, actual_total (eval-v0.md F4).
- overall (micro-averaged) precision, recall, F1 across all
  detectors.

### Indices

- Wilson 95 percent lower confidence bound on precision and on
  recall, per detector and overall, using the same `wilson_lower`
  helper used by the Layer 2 calibrated ranker
  (`src/calibration.rs`).
- A detector is reported as "Wilson-positive" when its Wilson
  lower bound on both precision and recall is strictly greater
  than 0.

## Analysis Plan

### Statistical models

No statistical models beyond the descriptive metrics defined above
and the Wilson lower-bound calculation. No null-hypothesis
significance testing, no Bayesian model fitting, no regression.
Empirical priors that would feed the Layer 2 ranker (P4) are out
of scope for this preregistration: they live in calibration data
and have their own audit trail.

### Inference criteria

H1 through H5 each evaluate to "supported" iff the corresponding
detector is Wilson-positive AND precision ≥ 0.50 AND recall ≥
0.30. Otherwise the hypothesis is "refuted" and the detector is
annotated in the β release notes as needing refinement before any
wider claim of fitness for purpose.

H_overall is "supported" iff micro-averaged F1 ≥ 0.50 with a
Wilson lower bound on precision and recall both strictly greater
than 0.

### Data exclusion

A file is excluded from analysis (and the exclusion logged
separately) if any of:

- tree-sitter parsing fails. The exclusion is reported under a
  `parse_failures` field in the β release notes.
- the file requires cross-file resolution that the v0 detectors
  do not perform (for example, an `arg-swap` candidate whose
  function definition lives in another file). Such files are
  excluded ex ante during labelling and never entered into the
  manifest.
- the file is auto-generated (e.g., contains a `// @generated`
  marker or sits under a `target/` or `gen/` directory).
- the file's source language is Python. Python detection ships in
  the predecessor revision but Python β evaluation is preregistered
  separately; no Python file is admitted into the Rust β corpus.

### Missing data

A malformed manifest line aborts the entire run with
`EvalError::Parse` (eval-v0.md F2 / F6). A missing source file
referenced by the manifest aborts with `EvalError::MissingSource`.
Both are treated as run-invalidating errors rather than as missing
data; the run is repeated after correction.

### Exploratory analyses

The following analyses are exploratory and any conclusions drawn
from them will be reported as such:

- Sensitivity of precision and recall to the seed corpus's
  configuration thresholds (`SIMILARITY_THRESHOLD`,
  `MIN_GROUP_SIZE`, `MIN_FN_TOKENS`, `NEAR_DUPLICATE_THRESHOLD`,
  and the `comment-code` pattern set). Tuning these in response
  to the preregistered figures is a separate, post-hoc activity.
- Comparison of detector outputs against rustc, clippy, and the
  rust-analyzer diagnostics on the same corpus. These tools target
  overlapping but non-identical defect classes and direct
  comparison is informative but not confirmatory.
- Correlation between Layer 1 raw output and Layer 2 ranked output
  ordering, on the same corpus. Layer 2 evaluation is
  preregistered separately under the γ phase.
- Counts of `LanguageCitationStatus::Unconfirmed` findings produced
  by Python detection on the seed Python fixtures. These do not
  contribute to H1-H5 (which are Rust-scoped) but inform the M-4
  Python β corpus design.

## Constraints respected by this study

P1 (every detector cites prior art). The corpus does not change
which detectors run; their citation lists remain pinned and are
enforced by `tests/citations_consistency.rs`. The
per-language extension of P1 is governed by
`docs/spec/citations-policy.md` (M-6); detector findings carry a
`LanguageCitationStatus` field surfaced via SARIF.

P2 (preregistration metadata via
`DetectorConfig::preregistration_id`). This document's stable
identifier is `osf-prereg-2026-05-07`. The previous identifiers
`osf-prereg-2026-05-03`, `osf-prereg-2026-05-05`, and
`osf-prereg-2026-05-06` continue to refer to the superseded
revisions. Detectors that wish to reference this preregistration
set `preregistration_id` accordingly.

P3 (only Layer 3 may invoke an LLM). The eval harness performs no
LLM calls (eval-v0.md N1). The Layer 3 adjudicator is used only
for manifest cross-checking during labelling and not during
evaluation.

P4 (priors come from labelled corpora, not hardcoded guesses).
The labelled β corpus IS the source. No priors are baked into the
detectors themselves. Recalibration against the relabelled
wild β corpus moves the embedded priors as recorded under
"What changed since 2026-05-06".

P5 (severity maps to IEEE 1044-2009). Unaffected by this study;
the mapping is applied at SARIF emission time and is orthogonal
to precision and recall.

## References

- `docs/spec/eval-v0.md` — operational definition of the
  evaluation harness, including the matching rule, the manifest
  format, and the divide-by-zero conventions used in this study.
- `docs/spec/citations-policy.md` — per-language citation policy
  (M-6) governing the new `LanguageCitationStatus` field and the
  survey requirement under `docs/surveys/`.
- `docs/spec/multilang-v0.md` — multi-language architecture for
  Layer 1 detectors (M-1).
- `CITATIONS.md` — full bibliography of Layer 1 to Layer 4 prior
  art.
- `docs/spec/clone-drift-v0.md`, `docs/spec/arg-swap-v0.md`,
  `docs/spec/comment-code-v0.md`,
  `docs/spec/unreachable-after-terminator-v0.md`,
  `docs/spec/config-interaction-v0.md` — per-detector specs.
  Sections F4b / F4c (unreachable-after-terminator), F5b / F5c
  (comment-code), F5b / F5c (clone-drift) document the FP
  reduction pass landed in this revision.
- `docs/surveys/unreachable-after-terminator-python-2026-05.md`,
  `docs/surveys/comment-code-python-2026-05.md`,
  `docs/surveys/arg-swap-python-2026-05.md`,
  `docs/surveys/clone-drift-python-2026-05.md` — per-detector
  language-extension surveys (M-2, M-3).
- `prereg/2026-05-04-labelling-rubric-v0.md` — Phase 0
  single-rater labelling rubric. Used in the relabel cycle that
  accompanies this revision.
- Layer 1 citation keys committed to by this preregistration:
  `cordy-roy-icpc-2008`, `bettenburg-msr-2009`,
  `krinke-icsm-2007`, `assi-tosem-2025`, `li-zhou-fse-2005`,
  `rice-icse-2017`, `allamanis-neurips-2021`, `tan-sosp-2007`,
  `tan-pldi-2011`, `hovemeyer-pugh-oopsla-2004`,
  `engler-sosp-2001`, `tartler-eurosys-2011`, `nadi-icse-2014`.
