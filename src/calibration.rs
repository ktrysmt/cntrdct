//! Layer 2 calibration: labelled corpus loader and per-detector priors.
//!
//! Spec: `cntrdct/docs/spec/ranker-v1.md`.
//!
//! References:
//! - `kremenek-engler-sas-2003` — Z-Ranking: per-detector TP/FP statistics.
//! - `jung-kim-shin-yi-sas-2005` — Bayesian post-analysis with Laplace smoothing.
//!
//! Design notes:
//! - This crate is a pure data layer. It MUST NOT depend on tree-sitter or any
//!   detector crate. The only `cntrdct_*` dependency is `cntrdct-core` for the
//!   shared `AnomalyClass` enum.
//! - Per design constraint P4, priors are derived from a labelled corpus rather
//!   than hardcoded. `compute_priors` therefore takes the corpus as input and
//!   returns the priors computed from it.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::core::AnomalyClass;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------- Verdict ----------

/// Verdict assigned to a finding by a human reviewer during corpus labelling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    TruePositive,
    FalsePositive,
}

// ---------- LabelledFinding ----------

/// A single labelled finding read from a JSONL corpus file.
///
/// `anomaly_class` is optional for backward-compatibility with corpora collected
/// before Phase 5 (when `AnomalyClass` was added to `Finding`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LabelledFinding {
    pub detector_id: String,
    pub repo: String,
    pub file: String,
    pub line: u32,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anomaly_class: Option<AnomalyClass>,
}

// ---------- PriorMethod ----------

/// Which 95% lower-bound method produced [`DetectorPrior::wilson_lower_95`].
///
/// The calibrator selects the method based on the
/// labelled aggregate size: Wilson at `n >= 30`, a Bayes-Laplace
/// (Beta(1,1)) lower 95% bound at `n < 30`. BCD (2001) showed that the
/// Wilson interval enters a coverage-oscillation regime at small `n`,
/// while a Beta(α, β) credible-interval lower bound stays close to
/// nominal — this is the auditable signal of which regime we used.
///
/// The struct field name `wilson_lower_95` is retained for JSON
/// backward compatibility with priors files written before Q-11
/// (where every entry was Wilson by construction). New code should
/// inspect `prior_method` to know which formula produced the value.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PriorMethod {
    /// Wilson score interval (z = 1.96), used at `n >= 30`.
    /// Default so pre-Q-11 prior files (which carried no method
    /// field) round-trip as Wilson by construction.
    #[default]
    Wilson,
    /// Bayes-Laplace lower bound — 2.5th percentile of `Beta(tp+1, fp+1)`,
    /// used at `n < 30`. Cited: brown-cai-dasgupta-stat-sci-2001;
    /// thulin-electronic-journal-statistics-2014.
    Jeffreys,
}

/// Threshold below which `compute_priors` switches from Wilson to
/// Jeffreys. Brown / Cai / DasGupta (Statistical Science, 2001) show
/// the Wilson interval's actual coverage starts oscillating below
/// nominal once `n` falls under roughly 30; we use this exact cutoff.
pub const SMALL_SAMPLE_THRESHOLD: u32 = 30;

// ---------- DetectorPrior ----------

/// Per-detector priors aggregated from a labelled corpus.
///
/// `posterior_tp` is the Laplace-smoothed Beta(1, 1) posterior mean
/// `(TP + 1) / (TP + FP + 2)` (cited: jung-kim-shin-yi-sas-2005).
///
/// `wilson_lower_95` is the chosen 95% lower bound for the
/// true-positive rate. The actual computation depends on
/// `prior_method`: [`PriorMethod::Wilson`] gives the Wilson score
/// lower bound at z = 1.96 (cited: kremenek-engler-sas-2003);
/// [`PriorMethod::Jeffreys`] gives the 2.5th percentile of
/// `Beta(tp+1, fp+1)` and is used when `tp + fp <
/// SMALL_SAMPLE_THRESHOLD` (cited: brown-cai-dasgupta-stat-sci-2001
/// and thulin-ejs-2014). The field name is historical (every shipped
/// value before Q-11 was Wilson); the JSON shape is kept for
/// backward compatibility with user-cached priors files.
///
/// `prior_method` records which formula produced `wilson_lower_95`.
/// Defaulted to [`PriorMethod::Wilson`] when absent so old JSON files
/// keep loading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectorPrior {
    pub tp: u32,
    pub fp: u32,
    pub posterior_tp: f64,
    pub wilson_lower_95: f64,
    #[serde(default)]
    pub prior_method: PriorMethod,
}

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum CalibrationError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error at {path}:{line}: {source}")]
    Parse {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}

// ---------- Loader ----------

/// Read a JSONL corpus from `path`, one `LabelledFinding` per non-blank line.
///
/// Behaviour:
/// - Blank lines (and whitespace-only lines) are skipped.
/// - The first malformed line aborts with `CalibrationError::Parse` carrying
///   the 1-based line number.
/// - An empty file returns `Ok(vec![])`.
pub fn load_corpus(path: &Path) -> Result<Vec<LabelledFinding>, CalibrationError> {
    let file = File::open(path).map_err(|e| CalibrationError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let reader = BufReader::new(file);

    let mut out: Vec<LabelledFinding> = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_no = idx + 1;
        let raw = line.map_err(|e| CalibrationError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if raw.trim().is_empty() {
            continue;
        }
        let parsed: LabelledFinding =
            serde_json::from_str(&raw).map_err(|e| CalibrationError::Parse {
                path: path.to_path_buf(),
                line: line_no,
                source: e,
            })?;
        out.push(parsed);
    }
    Ok(out)
}

// ---------- Priors ----------

/// Aggregate a labelled corpus into per-detector priors, keyed by `detector_id`.
///
/// For each detector:
/// - `posterior_tp = (TP + 1) / (TP + FP + 2)` (Laplace smoothing).
/// - `wilson_lower_95` and `prior_method` come from
///   [`compute_lower_bound`], which selects Wilson at
///   `n >= SMALL_SAMPLE_THRESHOLD` and Jeffreys (Beta(1,1) lower 2.5%
///   quantile) at smaller `n`.
pub fn compute_priors(corpus: &[LabelledFinding]) -> HashMap<String, DetectorPrior> {
    let mut counts: HashMap<String, (u32, u32)> = HashMap::new();
    for entry in corpus {
        let slot = counts.entry(entry.detector_id.clone()).or_insert((0, 0));
        match entry.verdict {
            Verdict::TruePositive => slot.0 += 1,
            Verdict::FalsePositive => slot.1 += 1,
        }
    }

    let mut out: HashMap<String, DetectorPrior> = HashMap::with_capacity(counts.len());
    for (id, (tp, fp)) in counts {
        let n = (tp + fp) as f64;
        let posterior_tp = (tp as f64 + 1.0) / (n + 2.0);
        let (lower, method) = compute_lower_bound(tp, fp);
        out.insert(
            id,
            DetectorPrior {
                tp,
                fp,
                posterior_tp,
                wilson_lower_95: lower,
                prior_method: method,
            },
        );
    }
    out
}

/// Choose the 95% lower bound formula for a `(tp, fp)` cell per Q-11.
///
/// Switching rule:
/// - `tp + fp >= SMALL_SAMPLE_THRESHOLD` (n=30): [`PriorMethod::Wilson`].
/// - `tp + fp <  SMALL_SAMPLE_THRESHOLD`: [`PriorMethod::Jeffreys`].
///
/// The cell-size threshold is the Brown / Cai / DasGupta (2001) rule
/// of thumb: below ~30 observations the Wilson interval enters a
/// coverage-oscillation regime, while a Beta-prior credible-interval
/// lower bound stays close to nominal. Returning the method alongside
/// the value keeps the choice auditable in [`DetectorPrior`] and in
/// every downstream `RankedFinding`.
pub fn compute_lower_bound(tp: u32, fp: u32) -> (f64, PriorMethod) {
    if tp + fp < SMALL_SAMPLE_THRESHOLD {
        (jeffreys_lower_95(tp, fp), PriorMethod::Jeffreys)
    } else {
        (wilson_lower_95(tp, fp), PriorMethod::Wilson)
    }
}

// ---------- Wilson 95% lower bound ----------

/// Wilson score interval lower bound at z = 1.96 (95% confidence).
///
/// Convention: when `tp + fp == 0` (no observations), returns 0.0. Returning a
/// finite value (not NaN) keeps downstream sort/serialisation well-defined for
/// detectors with no labelled data.
pub fn wilson_lower_95(tp: u32, fp: u32) -> f64 {
    let n = (tp + fp) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let p_hat = tp as f64 / n;
    let z = 1.96_f64;
    let z2 = z * z;
    let center = p_hat + z2 / (2.0 * n);
    let margin = z * ((p_hat * (1.0 - p_hat) + z2 / (4.0 * n)) / n).sqrt();
    let denom = 1.0 + z2 / n;
    (center - margin) / denom
}

// ---------- Jeffreys (Bayes-Laplace) 95% lower bound ----------

/// Bayes-Laplace 95% lower bound — the 2.5th percentile of
/// `Beta(tp+1, fp+1)`, with the BCD boundary modification.
///
/// Cited in [`PriorMethod::Jeffreys`]:
/// brown-cai-dasgupta-stat-sci-2001 §4 (boundary modification:
/// "when tp = 0, set L = 0"; the unmodified Beta-prior bound has
/// systematic under-coverage in a thin window of small `p` at the
/// `k = 0` outcome). thulin-ejs-2014 reaches the same conclusion via
/// a different route (admissibility of the modification).
///
/// Implementation:
/// - `tp == 0`: return 0.0 (boundary modification — matches Wilson at
///   the same input and avoids the under-coverage shoulder at small
///   `p`).
/// - otherwise: bisection on the regularized incomplete beta function
///   `I_x(a, b)` with `a = tp+1`, `b = fp+1`. For positive integer
///   `a, b` (always the case here), `I_x(a, b)` reduces to a finite
///   binomial sum so no continued-fraction / Lentz approximation is
///   needed; rounding error is within 1e-12. 100 bisection steps gives
///   ~ 7.9e-31 precision, well below the 1e-6 we need.
///
/// Convention: when `tp + fp == 0`, returns 0.0 (matches the
/// observation-free convention for `wilson_lower_95`).
pub fn jeffreys_lower_95(tp: u32, fp: u32) -> f64 {
    if tp == 0 {
        // Boundary modification per BCD 2001: an unmodified Beta(1, n+1)
        // 2.5% quantile is positive but creates an under-coverage
        // shoulder when the true `p` lies in `(0, L)` — coverage drops
        // from 0.95 to 0 there. Returning 0 (as Wilson does at the
        // same input) keeps coverage at the boundary and matches the
        // observation-free convention.
        return 0.0;
    }
    let a = tp + 1;
    let b = fp + 1;
    let target = 0.025_f64;
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if regularized_incomplete_beta_int(mid, a, b) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Regularized incomplete beta function `I_x(a, b)` for positive
/// integer `a, b`. Uses the binomial-sum identity
/// `I_x(a, b) = sum_{i=a}^{a+b-1} C(a+b-1, i) x^i (1-x)^(a+b-1-i)`
/// (Abramowitz & Stegun 26.5.4 / Wikipedia "Beta function").
///
/// Intermediate `C(n, i)` magnitudes are tracked in log-space so the
/// computation stays numerically stable for the small-N inputs Q-11
/// uses (`n < 30`); pure `f64` via Stirling-free direct ratio
/// updates.
fn regularized_incomplete_beta_int(x: f64, a: u32, b: u32) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let n = a + b - 1;
    let ln_x = x.ln();
    let ln_1mx = (1.0 - x).ln();
    let mut log_binomial = 0.0_f64;
    let mut sum = 0.0_f64;
    for i in 0..=n {
        if i >= a {
            let log_term = log_binomial + (i as f64) * ln_x + ((n - i) as f64) * ln_1mx;
            sum += log_term.exp();
        }
        if i < n {
            log_binomial += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_roundtrips_via_json() {
        let v = Verdict::TruePositive;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"TruePositive\"");
        let back: Verdict = serde_json::from_str(&s).unwrap();
        assert_eq!(back, Verdict::TruePositive);
    }

    #[test]
    fn detector_prior_roundtrips_via_json() {
        let prior = DetectorPrior {
            tp: 80,
            fp: 20,
            posterior_tp: 0.7941176470588235,
            wilson_lower_95: 0.7111,
            prior_method: PriorMethod::Wilson,
        };
        let s = serde_json::to_string(&prior).unwrap();
        let back: DetectorPrior = serde_json::from_str(&s).unwrap();
        assert_eq!(back, prior);
    }

    #[test]
    fn pre_q11_priors_json_deserialises_with_default_wilson_method() {
        // Priors files written before Q-11 carry no `prior_method` field;
        // serde must default it to Wilson so old user-cached priors keep
        // loading after upgrade.
        let body = r#"{"tp":80,"fp":20,"posterior_tp":0.7941,"wilson_lower_95":0.7111}"#;
        let parsed: DetectorPrior = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.prior_method, PriorMethod::Wilson);
    }

    #[test]
    fn prior_method_serialises_as_snake_case() {
        let s = serde_json::to_string(&PriorMethod::Jeffreys).unwrap();
        assert_eq!(s, "\"jeffreys\"");
        let s = serde_json::to_string(&PriorMethod::Wilson).unwrap();
        assert_eq!(s, "\"wilson\"");
    }

    #[test]
    fn compute_lower_bound_picks_jeffreys_below_threshold_and_wilson_at_or_above() {
        let (lo, m) = compute_lower_bound(2, 1);
        assert_eq!(m, PriorMethod::Jeffreys);
        // Beta(3, 2) lower 2.5% quantile is ≈ 0.1941.
        assert!(
            (lo - 0.1941).abs() < 1e-3,
            "jeffreys_lower_95(2, 1) = {} (expected ~0.1941)",
            lo
        );

        // n = 29: still small-sample.
        let (_, m) = compute_lower_bound(15, 14);
        assert_eq!(m, PriorMethod::Jeffreys);

        // n = 30: switches to Wilson.
        let (lo, m) = compute_lower_bound(15, 15);
        assert_eq!(m, PriorMethod::Wilson);
        let expected_wilson = wilson_lower_95(15, 15);
        assert!(
            (lo - expected_wilson).abs() < 1e-12,
            "Wilson value should pass through unchanged"
        );
    }

    #[test]
    fn jeffreys_lower_zero_zero_is_zero_by_convention() {
        // Match the wilson_lower_95(0, 0) == 0.0 convention so detectors
        // with no labelled data sort/serialize predictably.
        assert_eq!(jeffreys_lower_95(0, 0), 0.0);
    }

    #[test]
    fn jeffreys_lower_zero_tp_returns_zero_per_bcd_boundary_modification() {
        // BCD 2001 §4: at `tp = 0` an unmodified Beta(1, fp+1) lower
        // 2.5% quantile sits above 0, which creates a thin
        // under-coverage shoulder when true `p` is small. Q-11 applies
        // the boundary correction (return 0) so observation-free cells
        // serialise the same as Wilson and stay sorted to the bottom.
        for fp in [1_u32, 5, 10, 29] {
            assert_eq!(jeffreys_lower_95(0, fp), 0.0);
        }
    }

    #[test]
    fn jeffreys_lower_strictly_positive_when_any_tp() {
        // Past the boundary cell, the Beta(tp+1, fp+1) lower 2.5%
        // quantile is strictly positive. Spot-check a few small
        // configurations.
        for (tp, fp) in [(1_u32, 5_u32), (2, 1), (8, 0), (15, 2)] {
            let lj = jeffreys_lower_95(tp, fp);
            assert!(lj > 0.0, "L({}, {}) = {} should be > 0", tp, fp, lj);
            assert!(lj < 1.0, "L({}, {}) = {} should be < 1", tp, fp, lj);
        }
    }

    #[test]
    fn regularized_incomplete_beta_matches_known_closed_forms() {
        // Beta(1, 1) is uniform: I_x(1, 1) = x.
        for x in [0.0_f64, 0.1, 0.5, 0.9, 1.0] {
            let v = regularized_incomplete_beta_int(x, 1, 1);
            assert!(
                (v - x).abs() < 1e-12,
                "I_{}(1,1) = {} (expected {})",
                x,
                v,
                x
            );
        }
        // Beta(2, 1) has CDF x^2.
        for x in [0.1_f64, 0.4, 0.7] {
            let v = regularized_incomplete_beta_int(x, 2, 1);
            assert!(
                (v - x * x).abs() < 1e-12,
                "I_{}(2,1) = {} (expected {})",
                x,
                v,
                x * x
            );
        }
    }

    #[test]
    fn labelled_finding_optional_anomaly_class_is_omitted_when_none() {
        let lf = LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: "a.rs".to_string(),
            line: 1,
            verdict: Verdict::TruePositive,
            anomaly_class: None,
        };
        let s = serde_json::to_string(&lf).unwrap();
        assert!(!s.contains("anomaly_class"), "should omit when None: {}", s);
    }
}
