# cntrdct ranker v1 spec

Status: active draft, approved for TDD implementation 2026-05-03.

v0 was uncalibrated; v1 introduces calibration on top of v0's API. v0 remains
the no-corpus fallback and its spec (`docs/spec/ranker-v0.md`) is kept as
historical record. This document only describes deltas from v0 unless noted.

## Background

v0 left `RankedFinding.posterior_tp` and `RankedFinding.wilson_lower` as `None`
because no labelled corpus existed. v1 ships the calibration layer that
populates them, plus a CLI subcommand to derive priors from a JSONL corpus.
Reference algorithms:

- Z-Ranking, Kremenek & Engler, SAS 2003 — confidence-bound ordering of
  warnings to counter the impact of static-analysis approximations
- Bayesian post-analysis, Jung et al., SAS 2005 — TP rate posterior under a
  uniform Beta(1, 1) prior (Laplace smoothing)

## Crates

- `crates/calibration` — pure data layer; no detector / tree-sitter deps
- `crates/ranker` — gains `CalibratedRanker`; keeps `UncalibratedRanker`
- `crates/cli` — gains `cntrdct calibrate` subcommand and ranker auto-pick

## Functional requirements

### F1 — calibration crate API

```
pub enum Verdict { TruePositive, FalsePositive }

pub struct LabelledFinding {
    detector_id: String,
    repo: String,
    file: String,
    line: u32,
    verdict: Verdict,
    anomaly_class: Option<AnomalyClass>,  // optional for backward compat
}

pub struct DetectorPrior {
    tp: u32,
    fp: u32,
    posterior_tp: f64,     // Laplace
    wilson_lower_95: f64,  // Wilson 95% lower bound
}

pub fn load_corpus(path: &Path) -> Result<Vec<LabelledFinding>, CalibrationError>;
pub fn compute_priors(corpus: &[LabelledFinding]) -> HashMap<String, DetectorPrior>;
pub fn wilson_lower_95(tp: u32, fp: u32) -> f64;
```

`anomaly_class` is `Option` because corpora collected before Phase 5 do not
carry it. Loaders MUST treat the field as optional.

### F2 — formulas

- Laplace-smoothed posterior TP rate (Beta(1, 1) prior):
  `posterior_tp = (TP + 1) / (TP + FP + 2)`
- Wilson 95% lower bound (z = 1.96):
  ```
  let n = TP + FP
  if n == 0 { return 0.0 }            // convention
  let p = TP / n
  let z2 = 1.96 * 1.96
  let center = p + z2 / (2*n)
  let margin = 1.96 * sqrt((p*(1-p) + z2/(4*n)) / n)
  let denom  = 1 + z2 / n
  (center - margin) / denom
  ```
  Convention: `(0, 0) -> 0.0`. Returning a finite value (not NaN) keeps
  downstream sort and JSON serialisation well-defined for detectors with no
  labelled data.

Reference values used in the test suite:

| TP, FP | wilson_lower_95 |
|--------|-----------------|
| 80, 20 | 0.7111 |
| 50, 50 | 0.4038 |
| 1, 0   | 0.2065 |
| 0, 0   | 0.0    |

### F3 — `CalibratedRanker`

```
pub struct CalibratedRanker { priors: HashMap<String, DetectorPrior> }
impl CalibratedRanker { pub fn new(priors: ...) -> Self; }
impl cntrdct_core::Ranker for CalibratedRanker { ... }
```

Per finding `f`:

- If `priors.get(&f.detector_id)` is `Some(p)`:
  - `posterior_tp = Some(p.posterior_tp)`
  - `wilson_lower = Some(p.wilson_lower_95)`
  - `rank_score = p.wilson_lower_95 * (1.0 + (1.0 + f.related.len() as f64).log2())`
- Else (silent fallback for partially-calibrated corpora):
  - `posterior_tp = None`
  - `wilson_lower = None`
  - `rank_score = f.related.len() as f64`

Rationale for the formula:

- Using `wilson_lower_95` (not raw `tp/(tp+fp)`) is the Z-Ranking move: a
  rare-but-possibly-precise detector is not allowed to dominate the ranking
  before enough evidence has been accumulated.
- The `(1 + log2(1 + related.len()))` factor is monotone, sub-linear in
  group size, and equal to 1 when `related.len() == 0`. It rewards corroboration
  by sibling clones without letting one huge clone group flood the top of the
  ranking.

Sort order (unchanged from v0):

1. `rank_score` desc
2. `primary.file` asc
3. `primary.start_line` asc

### F4 — `UncalibratedRanker` retained

The free function `cntrdct_ranker::rank()` and `UncalibratedRanker` continue to
exist exactly as in v0. They are the no-corpus fallback; behaviour is unchanged.

### F5 — CLI subcommand `cntrdct calibrate`

```
cntrdct calibrate <CORPUS_PATH> [--output <PATH>]
```

- Reads a JSONL corpus via `calibration::load_corpus`.
- Computes priors via `calibration::compute_priors`.
- Pretty-prints the resulting `HashMap<String, DetectorPrior>` to `--output`
  (default: `dirs::cache_dir().join("cntrdct").join("priors.json")`).
- Creates parent directories as needed.
- Writes `wrote priors for N detectors to <path>` to stderr.

### F6 — CLI scan auto-pick

`cntrdct scan` selects the ranker by:

1. If `--no-calibration` is set → `UncalibratedRanker` (forced).
2. Else, if `--priors <PATH>` is set → load that path; if it does not exist
   the run errors out via the file-load path; if it parses cleanly use
   `CalibratedRanker`.
3. Else, if the default cache path exists → use `CalibratedRanker`.
4. Else → `UncalibratedRanker` (silent fallback; no warning).

`--priors` is an internal flag intended primarily for testing and reproducible
runs. Production users typically rely on the default cache path.

### F7 — Determinism and purity

Unchanged from v0: `Ranker::rank` is a pure function of its input. All I/O
lives in `cntrdct-cli` (which calls `calibration::load_corpus` once at start
and constructs the ranker with the resulting `HashMap`).

## Non-functional requirements

- N1. P3 preserved: no LLM in ranker
- N2. P4 preserved: priors come from a labelled corpus, never hardcoded. The
  uncalibrated ranker continues to ship `None` for the calibration columns when
  no corpus is available rather than guessed values.
- N3. The calibration crate has no detector / tree-sitter dependency. It only
  depends on `cntrdct-core` (for the `AnomalyClass` enum), `serde`, `serde_json`
  and `thiserror`.

## Test plan

| ID  | Crate / file | Description |
|-----|---|---|
| W1  | calibration/tests/integration.rs | wilson_lower_95(80, 20) ≈ 0.7111 |
| W2  | calibration/tests/integration.rs | wilson_lower_95(50, 50) ≈ 0.4038 |
| W3  | calibration/tests/integration.rs | wilson_lower_95(1, 0) ≈ 0.2065 |
| W4  | calibration/tests/integration.rs | wilson_lower_95(0, 0) == 0.0 |
| C1  | calibration/tests/integration.rs | compute_priors groups by detector_id and counts TP/FP correctly |
| C2  | calibration/tests/integration.rs | compute_priors uses Laplace formula |
| C3  | calibration/tests/integration.rs | compute_priors populates wilson_lower_95 |
| C4  | calibration/tests/integration.rs | compute_priors of empty corpus is empty map |
| L1  | calibration/tests/integration.rs | load_corpus parses valid JSONL |
| L2  | calibration/tests/integration.rs | load_corpus skips blank lines |
| L3  | calibration/tests/integration.rs | load_corpus on empty file returns Ok empty |
| L4  | calibration/tests/integration.rs | load_corpus reports 1-based line number on parse failure |
| L5  | calibration/tests/integration.rs | example_corpus.jsonl loads and covers all three detectors |
| R1  | ranker/tests/integration.rs | CalibratedRanker(empty priors) matches UncalibratedRanker |
| R2  | ranker/tests/integration.rs | CalibratedRanker uses Wilson and log factor |
| R3  | ranker/tests/integration.rs | CalibratedRanker orders by wilson_lower across detectors |
| R4  | ranker/tests/integration.rs | CalibratedRanker falls back when detector unknown |
| R5  | ranker/tests/integration.rs | rank_score collapses to wilson_lower when related.len()==0 |
| K1  | cli/tests/calibrate.rs | calibrate writes priors with expected values |
| K2  | cli/tests/calibrate.rs | calibrate creates parent directories |
| K3  | cli/tests/calibrate.rs | scan + --priors override populates posterior_tp / wilson_lower |
| K4  | cli/tests/calibrate.rs | --no-calibration forces UncalibratedRanker even when priors exist |
| K5  | cli/tests/calibrate.rs | pick_ranker on missing priors path silently falls back |

## Non-goals (v1)

- Per-(detector, anomaly_class) calibration buckets
- Bayesian update on user feedback
- Active learning loop
- Brier score reporting
- Concurrent corpus mutation across runs

## References

- `kremenek-engler-sas-2003` — Z-Ranking, SAS 2003
- `jung-kim-shin-yi-sas-2005` — Bayesian post-analysis, SAS 2005
- `spiess-icse-2025` — Calibration of LMs for code (post-v1 reference for
  Layer 3 adjudicator integration)
