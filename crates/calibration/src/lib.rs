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

use cntrdct_core::AnomalyClass;
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

// ---------- DetectorPrior ----------

/// Per-detector priors aggregated from a labelled corpus.
///
/// - `posterior_tp`: Laplace-smoothed Beta(1,1) posterior mean — `(TP+1)/(TP+FP+2)`.
///   Cited: jung-kim-shin-yi-sas-2005.
/// - `wilson_lower_95`: 95% Wilson score lower bound for the true-positive rate.
///   Cited: kremenek-engler-sas-2003 (Z-Ranking uses confidence-bound style
///   reasoning to avoid penalising rare detectors with little data).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectorPrior {
    pub tp: u32,
    pub fp: u32,
    pub posterior_tp: f64,
    pub wilson_lower_95: f64,
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
/// - `wilson_lower_95 = wilson_lower_95(TP, FP)` (95% Wilson lower bound).
pub fn compute_priors(corpus: &[LabelledFinding]) -> HashMap<String, DetectorPrior> {
    let mut counts: HashMap<String, (u32, u32)> = HashMap::new();
    for entry in corpus {
        let slot = counts
            .entry(entry.detector_id.clone())
            .or_insert((0, 0));
        match entry.verdict {
            Verdict::TruePositive => slot.0 += 1,
            Verdict::FalsePositive => slot.1 += 1,
        }
    }

    let mut out: HashMap<String, DetectorPrior> = HashMap::with_capacity(counts.len());
    for (id, (tp, fp)) in counts {
        let n = (tp + fp) as f64;
        let posterior_tp = (tp as f64 + 1.0) / (n + 2.0);
        let wilson = wilson_lower_95(tp, fp);
        out.insert(
            id,
            DetectorPrior {
                tp,
                fp,
                posterior_tp,
                wilson_lower_95: wilson,
            },
        );
    }
    out
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
        };
        let s = serde_json::to_string(&prior).unwrap();
        let back: DetectorPrior = serde_json::from_str(&s).unwrap();
        assert_eq!(back, prior);
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
