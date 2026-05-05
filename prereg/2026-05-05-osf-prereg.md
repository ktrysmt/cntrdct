# OSF Preregistration: cntrdct β-phase precision and recall study (revision 1)

Author: ktrysmt
Date: 2026-05-05
Project: cntrdct (Evidence-based contradiction linter, Rust + Python)
Status: draft, pre-registration not yet submitted
Supersedes: `prereg/2026-05-03-osf-prereg.md`

## What changed since 2026-05-03

The original preregistration covered five Layer 1 detectors operating
on Rust source only. Between 2026-05-03 and the date of this revision
the project landed milestones M-1 (language abstraction foundation),
M-2 (Python pilot for `unreachable-after-terminator`), and the first
two parts of M-3 (Python extension of `comment-code` and `arg-swap`).
This revision restates the full preregistration so that the most
recent dated document continues to commit the entire detector and
citation surface that will be measured at β release time. The
substantive changes are:

- The detector surface still names the same five detectors, but two
  of them (`unreachable-after-terminator`, `comment-code`) and now
  three of them (`arg-swap` added in this revision) accept Python
  source in addition to Rust. The Rust-side hypotheses are
  unchanged.
- Python detection ships with a per-language citation status field
  (`LanguageCitationStatus`, see `docs/spec/citations-policy.md`).
  `arg-swap` Python is `Confirmed` via Allamanis, Jackson-Flux,
  Brockschmidt (NeurIPS 2021); `unreachable-after-terminator` and
  `comment-code` Python are `Unconfirmed` per their respective
  surveys under `docs/surveys/`.
- The β corpus described under "Sampling Plan" remains Rust-only at
  the date of this revision. Python β corpus collection is tracked
  by ROADMAP M-4 and will be preregistered separately under a future
  document; this preregistration's hypotheses are scoped to the
  Rust corpus.
- The Layer 1 citation key set gains `allamanis-neurips-2021`.

## Background

cntrdct is an evidence-based static linter whose first design
constraint (P1) requires every detector to cite peer-reviewed prior
art. The α phase shipped four Layer 1 detectors — `clone-drift`,
`arg-swap`, `comment-code`, and `unreachable-after-terminator`. The
β phase added a fifth, `config-interaction`. Layer 1 sits on top of
a Layer 2 statistical ranker, a Layer 3 LLM adjudicator, and a
Layer 4 SARIF emitter. Per-detector citations are enforced at
startup (`register_detector` rejects citation-free detectors); a
workspace test (`crates/cli/tests/citations_consistency.rs`) keeps
the live `Detector::citations()` output in lock-step with
`CITATIONS.md`; and a sibling test
(`crates/cli/tests/prereg_consistency.rs`) keeps the latest
preregistration in lock-step with the live citation key set.

The β phase introduces an evaluation harness, specified in
`docs/spec/eval-v0.md` and implemented as the `cntrdct-eval` crate
plus the `cntrdct eval` CLI subcommand. This preregistration commits
— in advance of the β corpus collection — to the corpus design, the
matching rule, the metric set, and the inference criteria that will
be used to characterise each detector's empirical performance on
Rust source.

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
`bettenburg-msr-2009`, `krinke-icsm-2007`, `li-zhou-fse-2005`,
`rice-icse-2017`, `allamanis-neurips-2021`, `tan-sosp-2007`,
`tan-pldi-2011`, `hovemeyer-pugh-oopsla-2004`, `engler-sosp-2001`,
`tartler-eurosys-2011`, `nadi-icse-2014`) is treated as
exploratory: those studies used different languages, corpora, and
labelling protocols and direct numerical comparison would be
invalid. The preregistered hypotheses do not depend on it.

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
F8).

### Study design

Single-corpus, within-detector. Each source file in the corpus is
a single observation. For each detector the matching rule defined
in eval-v0.md F3 partitions findings into TP, FP, and FN.

## Sampling Plan

### Existing data

The seed corpus at `benchmarks/corpus/` contains synthetic
hand-crafted fixtures at the date of this revision: ten positive
fixtures per cross-cutting Rust detector, three Rust negatives per
detector, plus the M-2 / M-3-partial Python pilot fixtures (five
positives + three negatives for each of `unreachable-after-terminator`,
`comment-code`, and `arg-swap`). The seed corpus is treated as a
smoke-test rather than as evaluation data; it does not contribute
to the β figures produced under this preregistration. Python
fixtures are present but Python β evaluation is preregistered
separately (see ROADMAP M-4).

### Data collection procedures

The β corpus is drawn from publicly available Rust source files
under permissive licences (MIT, Apache-2.0, BSD). Candidate sources
include the rust-analyzer test corpus, the clippy regression suite,
and the top 200 most-downloaded crates on crates.io.

Each candidate file is reviewed by the author and labelled by hand
into the JSONL manifest format defined in `docs/spec/eval-v0.md`.
To reduce single-rater bias, every label is independently reviewed
by a second pass through the Layer 3 LLM adjudicator (Anthropic
Messages, see `crates/adjudicator-llm`) used here strictly as a
labelling cross-check and never as an inference instrument.

A label is accepted into the manifest only when both passes agree.
A disagreement triggers a written reconciliation note recorded
under `benchmarks/disagreements.md`; the author's adjudicated
decision is final but the original disagreement is preserved for
auditability. The Phase 0 single-rater addendum
(`prereg/2026-05-05-osf-prereg-phase0-addendum.md`) clarifies the
fall-back protocol for the initial bootstrapping batch.

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

- TP(d), FP(d), FN(d), as defined by `cntrdct_eval::evaluate` per
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
  (`crates/calibration`).
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
  this revision but Python β evaluation is preregistered
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
  `MIN_GROUP_SIZE`, and the `comment-code` pattern set). Tuning
  these in response to the preregistered figures is a separate,
  post-hoc activity.
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
enforced by `crates/cli/tests/citations_consistency.rs`. The
per-language extension of P1 is governed by
`docs/spec/citations-policy.md` (M-6); detector findings carry a
`LanguageCitationStatus` field surfaced via SARIF.

P2 (preregistration metadata via
`DetectorConfig::preregistration_id`). This document's stable
identifier is `osf-prereg-2026-05-05`. The previous identifier
`osf-prereg-2026-05-03` continues to refer to the superseded
revision. Detectors that wish to reference this preregistration
set `preregistration_id` accordingly.

P3 (only Layer 3 may invoke an LLM). The eval harness performs no
LLM calls (eval-v0.md N1). The Layer 3 adjudicator is used only
for manifest cross-checking during labelling and not during
evaluation.

P4 (priors come from labelled corpora, not hardcoded guesses).
The labelled β corpus IS the source. No priors are baked into the
detectors themselves.

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
- `docs/surveys/unreachable-after-terminator-python-2026-05.md`,
  `docs/surveys/comment-code-python-2026-05.md`,
  `docs/surveys/arg-swap-python-2026-05.md` — per-detector
  language-extension surveys (M-2, M-3).
- Layer 1 citation keys committed to by this preregistration:
  `cordy-roy-icpc-2008`, `bettenburg-msr-2009`,
  `krinke-icsm-2007`, `li-zhou-fse-2005`, `rice-icse-2017`,
  `allamanis-neurips-2021`, `tan-sosp-2007`, `tan-pldi-2011`,
  `hovemeyer-pugh-oopsla-2004`, `engler-sosp-2001`,
  `tartler-eurosys-2011`, `nadi-icse-2014`.
