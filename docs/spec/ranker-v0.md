# cntrdct ranker v0 spec

Status: active draft, approved for TDD implementation 2026-05-03.

## Background

Layer 2 of the architecture per `cntrdct/docs/spec/clone-drift-v0.md` and the
overall design docs. Reference algorithms:

- Z-Ranking, Kremenek & Engler, SAS 2003
- Bayesian post-analysis, Jung et al., SAS 2005

These algorithms require a labelled corpus that yields per-detector TP/FP counts.
v0 ships before any labelled data is collected; therefore the ranker is
intentionally **uncalibrated** in v0 and degrades gracefully.

## Scope

- Input: `Vec<Finding>` from Layer 1
- Output: `Vec<RankedFinding>` ordered for triage
- Trait: implements `cntrdct_core::Ranker`
- No labelled corpus, no statistical learning in v0

## Functional requirements

### F1 — API

- `cntrdct_ranker::rank(findings: Vec<Finding>) -> Vec<RankedFinding>` (free function)
- `cntrdct_ranker::UncalibratedRanker` struct that implements `cntrdct_core::Ranker`

### F2 — Calibration columns

For v0 (no labels):
- `posterior_tp = None`
- `wilson_lower = None`

These become `Some(p)` once `cntrdct-calibration` (post-v0) ships and a labelled
corpus is loaded. Using `Option<f64>` instead of `f64::NAN` so the values
serialize cleanly to JSON `null`.

### F3 — rank_score formula

`rank_score = finding.related.len() as f64`

Rationale: a finding whose drifted sibling diverges from a larger consensus
group is more salient than one diverging from a 2-member majority. Generic
across detectors; future detectors may override via different ranker impls.

### F4 — Output ordering

1. `rank_score` descending
2. `primary.file` ascending lexicographically
3. `primary.start_line` ascending

NaN-safe: `rank_score` is derived from `usize` so cannot be NaN.

### F5 — Determinism and purity

`rank` is a pure function of its input. No I/O, no network, no clock, no random.

## Non-functional requirements

- N1. P3 preserved: no LLM in ranker
- N2. P4 preserved: no hardcoded empirical priors. v0 ships with `f64::NAN`
  rather than guessed values

## Test plan

| ID | Description | Expected |
|---|---|---|
| T1 | empty input | empty output |
| T2 | one finding | exactly one ranked finding |
| T3 | order by rank_score desc | finding with more `related` ranks first |
| T4 | tie break by file path | equal rank_score → ascending file path |
| T5 | tie break by line | equal rank_score and file → ascending start_line |
| T6 | posterior_tp None in v0 | every output has `is_none()` posterior |
| T7 | wilson_lower None in v0 | every output has `is_none()` wilson |
| T8 | rank_score equals related length | each output's rank_score == finding.related.len() |
| T9 | UncalibratedRanker implements core::Ranker | trait dispatch works |

## Non-goals (v0)

- Statistical priors from labelled data
- Per-detector calibration tables
- Bayesian update on user feedback
- Active learning loop
- Brier score reporting

## References

- `kremenek-engler-sas-2003` — Z-Ranking, SAS 2003
- `jung-kim-shin-yi-sas-2005` — Bayesian post-analysis, SAS 2005
- `spiess-icse-2025` — Calibration of LMs for code (post-v0 reference for
  Layer 3 adjudicator integration)
