//! Layer 3 post-hoc LLM confidence calibration via Platt scaling.
//!
//! Spec: `docs/spec/llm-calibration-v0.md` (Q-12). Replaces the
//! adjudicator's earlier reliance on a self-reported `calibration_tag`
//! with a Platt fit per `(detector_id, anomaly_class)` cell on a
//! labelled corpus of LLM confidence values.
//!
//! Per design constraint P3, this module is pure: no network, no
//! filesystem access beyond the supplied paths in [`load_corpus`] and
//! [`load_registry`]. The only `Adjudicator` interaction is via the
//! library helper `cntrdct::apply_llm_calibration`, which post-
//! processes the verdicts the adjudicator already produced.
//!
//! References:
//! - `platt-1999` — Platt, "Probabilistic Outputs for Support Vector
//!   Machines and Comparisons to Regularized Likelihood Methods",
//!   Advances in Large Margin Classifiers (MIT Press), 1999.
//! - `spiess-icse-2025` — empirical motivation for post-hoc
//!   calibration of LLM-emitted confidence on code tasks.
//! - `spiess-koohestani-sergeyuk-2025` — large-scale evidence that
//!   verbalised confidence is not better calibrated than raw output.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::calibration::Verdict;
use crate::core::AnomalyClass;

// ---------- Input row (corpus JSONL) ----------

/// One labelled LLM-confidence observation read from a JSONL corpus.
///
/// Mirrors the role [`crate::calibration::LabelledFinding`] plays for
/// the P-4 priors pipeline: a single labelled row that the calibrator
/// aggregates into per-cell parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabelledLlmConfidence {
    /// Detector that produced the underlying finding.
    pub detector_id: String,
    /// IEEE 1044-2009 anomaly class carried by the finding.
    pub anomaly_class: AnomalyClass,
    /// LLM-emitted confidence in `[0.0, 1.0]`. Out-of-range values
    /// are clamped at fit time to match the runtime parser policy in
    /// [`crate::adjudicator::parse_response`].
    pub raw_confidence: f64,
    /// Ground-truth verdict (`TruePositive` / `FalsePositive`).
    pub verdict: Verdict,
}

// ---------- Platt parameters ----------

/// Sigmoid parameters fit by Platt scaling. Calibrated probability
/// is `sigmoid(a * raw + b)` per the original paper's notation
/// (Platt 1999 §1, eq. 1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PlattParams {
    /// Slope of the calibrated sigmoid.
    pub a: f64,
    /// Intercept of the calibrated sigmoid.
    pub b: f64,
}

/// Composite key for the Platt registry: per-`(detector_id,
/// anomaly_class)` cell. Stored alongside the registry rather than
/// inside `PlattParams` so the registry's JSON shape is a flat object
/// keyed by composite strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PlattKey {
    /// Detector id (e.g. `"clone-drift"`).
    pub detector_id: String,
    /// Finding's IEEE 1044-2009 anomaly class.
    pub anomaly_class: AnomalyClass,
}

impl PlattKey {
    /// Composite serde key, e.g. `"clone-drift:Logic"`. The detector
    /// id portion never contains a colon (alphanumeric + dashes), so
    /// the split point is unambiguous.
    pub fn composite(&self) -> String {
        format!(
            "{}:{}",
            self.detector_id,
            anomaly_class_str(self.anomaly_class)
        )
    }

    /// Inverse of [`composite`]. Returns `None` when the string does
    /// not split cleanly into a known anomaly class suffix.
    pub fn from_composite(s: &str) -> Option<PlattKey> {
        let (det, cls) = s.rsplit_once(':')?;
        let cls = anomaly_class_from_str(cls)?;
        Some(PlattKey {
            detector_id: det.to_string(),
            anomaly_class: cls,
        })
    }
}

fn anomaly_class_str(c: AnomalyClass) -> &'static str {
    match c {
        AnomalyClass::Logic => "Logic",
        AnomalyClass::Interface => "Interface",
        AnomalyClass::Data => "Data",
        AnomalyClass::Documentation => "Documentation",
        AnomalyClass::Performance => "Performance",
        AnomalyClass::Standards => "Standards",
        AnomalyClass::Other => "Other",
    }
}

fn anomaly_class_from_str(s: &str) -> Option<AnomalyClass> {
    Some(match s {
        "Logic" => AnomalyClass::Logic,
        "Interface" => AnomalyClass::Interface,
        "Data" => AnomalyClass::Data,
        "Documentation" => AnomalyClass::Documentation,
        "Performance" => AnomalyClass::Performance,
        "Standards" => AnomalyClass::Standards,
        "Other" => AnomalyClass::Other,
        _ => return None,
    })
}

// ---------- Registry ----------

/// In-memory Platt registry: per-cell parameters keyed by
/// [`PlattKey`]. Serialises as a flat JSON object whose keys are
/// `<detector_id>:<anomaly_class>` composite strings, sorted by key
/// on write so the artefact is byte-stable across runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlattRegistry {
    inner: BTreeMap<PlattKey, PlattParams>,
}

impl PlattRegistry {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// Number of entries in the registry.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` iff the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Look up parameters for a `(detector_id, anomaly_class)` cell.
    pub fn get(&self, detector_id: &str, anomaly_class: AnomalyClass) -> Option<PlattParams> {
        let key = PlattKey {
            detector_id: detector_id.to_string(),
            anomaly_class,
        };
        self.inner.get(&key).copied()
    }

    /// Insert / replace parameters for a cell.
    pub fn insert(&mut self, key: PlattKey, params: PlattParams) {
        self.inner.insert(key, params);
    }

    /// Iterate sorted entries.
    pub fn iter(&self) -> impl Iterator<Item = (&PlattKey, &PlattParams)> {
        self.inner.iter()
    }

    /// Serialise to a sorted JSON object.
    pub fn to_json_pretty(&self) -> String {
        let flat: BTreeMap<String, &PlattParams> =
            self.inner.iter().map(|(k, v)| (k.composite(), v)).collect();
        serde_json::to_string_pretty(&flat).expect("PlattParams serialise cleanly")
    }

    /// Parse a flat JSON object produced by [`Self::to_json_pretty`].
    /// Unknown composite keys (anomaly class outside the documented
    /// set) cause [`PlattError::UnknownAnomalyClass`].
    pub fn from_json(body: &str) -> Result<Self, PlattError> {
        let flat: BTreeMap<String, PlattParams> =
            serde_json::from_str(body).map_err(|e| PlattError::Parse(e.to_string()))?;
        let mut inner: BTreeMap<PlattKey, PlattParams> = BTreeMap::new();
        for (k, v) in flat {
            let parsed = PlattKey::from_composite(&k)
                .ok_or_else(|| PlattError::UnknownAnomalyClass(k.clone()))?;
            inner.insert(parsed, v);
        }
        Ok(Self { inner })
    }
}

// ---------- Errors ----------

/// Errors produced by Platt calibration helpers.
#[derive(Debug, Error)]
pub enum PlattError {
    /// I/O failure while reading or writing a Platt artefact.
    #[error("io error reading {path}: {source}")]
    Io {
        /// Path being read or written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Malformed JSONL row in the input corpus.
    #[error("parse error at {path}:{line}: {source}")]
    ParseLine {
        /// Path of the file being read.
        path: PathBuf,
        /// 1-based line number of the offending row.
        line: usize,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to parse a Platt registry from JSON.
    #[error("registry parse error: {0}")]
    Parse(String),
    /// Composite registry key did not include a known anomaly class.
    #[error("unknown anomaly class in composite key: {0}")]
    UnknownAnomalyClass(String),
    /// `--fit-platt` was invoked on an empty corpus.
    #[error("empty corpus: nothing to fit")]
    EmptyCorpus,
}

// ---------- Corpus loading ----------

/// Read a JSONL corpus of [`LabelledLlmConfidence`] rows from `path`.
/// Blank lines are skipped; the first malformed line aborts.
pub fn load_corpus(path: &Path) -> Result<Vec<LabelledLlmConfidence>, PlattError> {
    let file = File::open(path).map_err(|e| PlattError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let reader = BufReader::new(file);

    let mut out: Vec<LabelledLlmConfidence> = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_no = idx + 1;
        let raw = line.map_err(|e| PlattError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if raw.trim().is_empty() {
            continue;
        }
        let parsed: LabelledLlmConfidence =
            serde_json::from_str(&raw).map_err(|e| PlattError::ParseLine {
                path: path.to_path_buf(),
                line: line_no,
                source: e,
            })?;
        out.push(parsed);
    }
    Ok(out)
}

/// Load a Platt registry from a JSON file. Returns `Ok(None)` when
/// the file does not exist (silent fallback, matching
/// [`crate::try_load_priors`]).
pub fn load_registry(path: &Path) -> Result<Option<PlattRegistry>, PlattError> {
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(path).map_err(|e| PlattError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Some(PlattRegistry::new()));
    }
    Ok(Some(PlattRegistry::from_json(trimmed)?))
}

// ---------- Aggregate corpus into per-cell registry ----------

/// Group a corpus by `(detector_id, anomaly_class)` and fit Platt
/// parameters for every non-empty cell.
///
/// Empty cells in the input are skipped; the resulting registry has
/// no entry for them and downstream `apply_platt` returns `None`.
pub fn fit_registry(corpus: &[LabelledLlmConfidence]) -> Result<PlattRegistry, PlattError> {
    if corpus.is_empty() {
        return Err(PlattError::EmptyCorpus);
    }
    let mut grouped: BTreeMap<PlattKey, Vec<(f64, bool)>> = BTreeMap::new();
    for row in corpus {
        let raw = row.raw_confidence.clamp(0.0, 1.0);
        let key = PlattKey {
            detector_id: row.detector_id.clone(),
            anomaly_class: row.anomaly_class,
        };
        let label = matches!(row.verdict, Verdict::TruePositive);
        grouped.entry(key).or_default().push((raw, label));
    }

    let mut registry = PlattRegistry::new();
    for (key, samples) in grouped {
        let params = fit_platt(&samples);
        registry.insert(key, params);
    }
    Ok(registry)
}

// ---------- Platt fit ----------

/// Newton-Raphson Platt fit per Platt 1999 §2 with the regularised
/// target shift (positives → `(N+ + 1) / (N+ + 2)`,
/// negatives → `1 / (N- + 2)`).
///
/// Returns `(a, b)` such that `sigmoid(a * raw + b)` is the
/// calibrated probability. On a degenerate sample (all-positive,
/// all-negative, or single-point) the fit gracefully returns
/// `(0.0, ln((N- + 1) / (N+ + 1)))` per Platt's initialisation
/// formula — the constant-output predictor at the empirical base
/// rate. Tested.
pub fn fit_platt(samples: &[(f64, bool)]) -> PlattParams {
    let n_pos = samples.iter().filter(|(_, y)| *y).count() as f64;
    let n_neg = samples.iter().filter(|(_, y)| !*y).count() as f64;

    // Platt 1999 init: A = 0, B = ln((N- + 1) / (N+ + 1)). Yields a
    // constant-output predictor at the empirical base rate.
    let mut a = 0.0_f64;
    let mut b = ((n_neg + 1.0) / (n_pos + 1.0)).ln();

    if samples.is_empty() {
        return PlattParams { a, b };
    }

    let t_pos = (n_pos + 1.0) / (n_pos + 2.0);
    let t_neg = 1.0 / (n_neg + 2.0);
    let targets: Vec<f64> = samples
        .iter()
        .map(|(_, y)| if *y { t_pos } else { t_neg })
        .collect();

    let mut prev_loss = neg_log_likelihood(samples, &targets, a, b);

    let max_iter = 100;
    let g_tol = 1e-7;
    let sigma = 1e-12; // Hessian regularisation to avoid singularity.

    for _ in 0..max_iter {
        let (g_a, g_b, h_aa, h_ab, h_bb) = gradient_and_hessian(samples, &targets, a, b, sigma);

        if g_a.abs() < g_tol && g_b.abs() < g_tol {
            break;
        }

        let det = h_aa * h_bb - h_ab * h_ab;
        if det.abs() < 1e-30 {
            // Should be excluded by the sigma regularisation, but
            // keep a defensive break so we never divide by zero.
            break;
        }
        // Newton step: solve H * delta = -g.
        let d_a = -(h_bb * g_a - h_ab * g_b) / det;
        let d_b = -(-h_ab * g_a + h_aa * g_b) / det;

        // Backtracking line search on the negative log-likelihood.
        let mut step = 1.0_f64;
        let min_step = 1e-10;
        let (new_a, new_b, new_loss) = loop {
            let candidate_a = a + step * d_a;
            let candidate_b = b + step * d_b;
            let candidate_loss = neg_log_likelihood(samples, &targets, candidate_a, candidate_b);
            if candidate_loss < prev_loss - 1e-12 || step < min_step {
                break (candidate_a, candidate_b, candidate_loss);
            }
            step *= 0.5;
        };

        a = new_a;
        b = new_b;
        prev_loss = new_loss;
    }

    PlattParams { a, b }
}

/// Apply fit parameters to a raw confidence value, returning the
/// calibrated probability. `raw` is clamped to `[0.0, 1.0]` before
/// the sigmoid is evaluated to mirror the parser policy.
pub fn apply_platt(p: PlattParams, raw: f64) -> f64 {
    let r = raw.clamp(0.0, 1.0);
    sigmoid(p.a * r + p.b)
}

/// Numerically stable sigmoid `1 / (1 + exp(-x))`.
fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        let z = (-x).exp();
        1.0 / (1.0 + z)
    } else {
        let z = x.exp();
        z / (1.0 + z)
    }
}

/// Negative log-likelihood under shifted targets, for line search.
fn neg_log_likelihood(samples: &[(f64, bool)], targets: &[f64], a: f64, b: f64) -> f64 {
    let mut total = 0.0_f64;
    for ((s, _), t) in samples.iter().zip(targets.iter()) {
        let f = a * *s + b;
        // Numerically stable softplus form:
        //   loss = t * softplus(-f) + (1-t) * softplus(f)
        // where softplus(z) = ln(1 + exp(z)) computed without
        // overflow as max(z, 0) + ln(1 + exp(-|z|)).
        let sp_neg = softplus(-f);
        let sp_pos = softplus(f);
        total += t * sp_neg + (1.0 - t) * sp_pos;
    }
    total
}

fn softplus(z: f64) -> f64 {
    if z > 0.0 {
        z + (1.0 + (-z).exp()).ln()
    } else {
        (1.0 + z.exp()).ln()
    }
}

/// Gradient and Hessian of the negative log-likelihood at `(a, b)`.
/// Returns `(g_a, g_b, H_aa, H_ab, H_bb)`. `sigma` is added to the
/// Hessian diagonal to avoid singularity at degenerate inputs.
fn gradient_and_hessian(
    samples: &[(f64, bool)],
    targets: &[f64],
    a: f64,
    b: f64,
    sigma: f64,
) -> (f64, f64, f64, f64, f64) {
    let mut g_a = 0.0_f64;
    let mut g_b = 0.0_f64;
    let mut h_aa = sigma;
    let mut h_ab = 0.0_f64;
    let mut h_bb = sigma;
    for ((s, _), t) in samples.iter().zip(targets.iter()) {
        let f = a * *s + b;
        // p = sigmoid(f), q = 1 - p, computed in stable form.
        let p = sigmoid(f);
        let q = 1.0 - p;
        // d L / d f = p - t.
        let d1 = p - *t;
        // d^2 L / d f^2 = p * q.
        let d2 = p * q;
        g_a += d1 * *s;
        g_b += d1;
        h_aa += d2 * *s * *s;
        h_ab += d2 * *s;
        h_bb += d2;
    }
    (g_a, g_b, h_aa, h_ab, h_bb)
}

// ---------- Expected calibration error ----------

/// Expected Calibration Error with equal-width bins. Standard
/// definition: split `[0, 1]` into `n_bins` equal-width buckets,
/// compute `|accuracy - mean_confidence|` per bin, average weighted
/// by bin support.
///
/// `n_bins` must be `>= 1`; the function returns `0.0` on an empty
/// sample.
pub fn ece(samples: &[(f64, bool)], n_bins: usize) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n_bins = n_bins.max(1);
    let mut bin_count = vec![0_usize; n_bins];
    let mut bin_conf = vec![0.0_f64; n_bins];
    let mut bin_acc = vec![0.0_f64; n_bins];

    for (conf, label) in samples {
        let c = conf.clamp(0.0, 1.0);
        // Map [0, 1] → [0, n_bins). Confidence == 1.0 lands in the
        // last bin rather than the out-of-range bin n_bins.
        let mut idx = (c * n_bins as f64).floor() as usize;
        if idx >= n_bins {
            idx = n_bins - 1;
        }
        bin_count[idx] += 1;
        bin_conf[idx] += c;
        if *label {
            bin_acc[idx] += 1.0;
        }
    }

    let total = samples.len() as f64;
    let mut sum = 0.0_f64;
    for i in 0..n_bins {
        if bin_count[i] == 0 {
            continue;
        }
        let n = bin_count[i] as f64;
        let mean_conf = bin_conf[i] / n;
        let acc = bin_acc[i] / n;
        sum += (n / total) * (mean_conf - acc).abs();
    }
    sum
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_is_stable_at_extremes() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-12);
        assert!(sigmoid(50.0) > 0.999);
        assert!(sigmoid(-50.0) < 0.001);
        assert!(sigmoid(50.0).is_finite());
        assert!(sigmoid(-50.0).is_finite());
    }

    #[test]
    fn softplus_is_stable_at_extremes() {
        // softplus(z) ~ z for large positive z.
        let v = softplus(100.0);
        assert!((v - 100.0).abs() < 1e-9, "got {}", v);
        // softplus(z) ~ 0 for large negative z.
        let v = softplus(-100.0);
        assert!(v < 1e-9, "got {}", v);
        assert!(v >= 0.0);
    }

    #[test]
    fn fit_platt_recovers_identity_on_well_calibrated_data() {
        // Construct samples whose raw confidence already matches
        // empirical accuracy: 80% positives at 0.8, 20% positives at
        // 0.2, etc. Platt should fit close to a → ~+infinity-ish but
        // with regularisation we expect a > 0 and the calibrated
        // value at 0.8 close to 0.8.
        let mut samples: Vec<(f64, bool)> = Vec::new();
        for _ in 0..80 {
            samples.push((0.8, true));
        }
        for _ in 0..20 {
            samples.push((0.8, false));
        }
        for _ in 0..20 {
            samples.push((0.2, true));
        }
        for _ in 0..80 {
            samples.push((0.2, false));
        }
        let p = fit_platt(&samples);
        let calibrated_high = apply_platt(p, 0.8);
        let calibrated_low = apply_platt(p, 0.2);
        assert!(
            calibrated_high > 0.65,
            "calibrated(0.8) should stay near 0.8, got {}",
            calibrated_high
        );
        assert!(
            calibrated_low < 0.35,
            "calibrated(0.2) should stay near 0.2, got {}",
            calibrated_low
        );
    }

    #[test]
    fn fit_platt_corrects_overconfidence() {
        // 100 samples at raw=0.9; only 50 are TPs. Calibrated 0.9
        // should drop toward ~0.5.
        let mut samples: Vec<(f64, bool)> = Vec::new();
        for i in 0..100 {
            samples.push((0.9, i < 50));
        }
        let p = fit_platt(&samples);
        let calibrated = apply_platt(p, 0.9);
        assert!(
            (calibrated - 0.5).abs() < 0.1,
            "calibrated(0.9) should sit near base rate 0.5, got {}",
            calibrated
        );
    }

    #[test]
    fn fit_platt_is_deterministic() {
        let samples = vec![
            (0.9, true),
            (0.8, true),
            (0.7, false),
            (0.4, true),
            (0.3, false),
            (0.1, false),
        ];
        let a = fit_platt(&samples);
        let b = fit_platt(&samples);
        assert_eq!(a, b);
    }

    #[test]
    fn fit_platt_handles_all_positive_input_gracefully() {
        let samples: Vec<(f64, bool)> = (0..10).map(|_| (0.7, true)).collect();
        let p = fit_platt(&samples);
        assert!(p.a.is_finite());
        assert!(p.b.is_finite());
    }

    #[test]
    fn apply_platt_clamps_out_of_range_inputs() {
        let p = PlattParams { a: 1.0, b: 0.0 };
        let lo = apply_platt(p, -0.5);
        let hi = apply_platt(p, 1.5);
        assert!((lo - sigmoid(0.0)).abs() < 1e-12);
        assert!((hi - sigmoid(1.0)).abs() < 1e-12);
    }

    #[test]
    fn ece_zero_for_perfectly_calibrated_samples() {
        // 50 samples at conf=0.5 with exactly 25 positives → bin
        // mean conf 0.5, acc 0.5, ECE 0.
        let mut samples: Vec<(f64, bool)> = Vec::new();
        for i in 0..50 {
            samples.push((0.5, i < 25));
        }
        let e = ece(&samples, 10);
        assert!(e < 1e-9, "expected zero ECE, got {}", e);
    }

    #[test]
    fn ece_detects_overconfidence() {
        // 100 samples at conf=0.95 but only 50% are positive. ECE
        // should pick up the 0.45 gap weighted by bin support.
        let mut samples: Vec<(f64, bool)> = Vec::new();
        for i in 0..100 {
            samples.push((0.95, i < 50));
        }
        let e = ece(&samples, 10);
        assert!((e - 0.45).abs() < 1e-9, "expected ECE ~0.45, got {}", e);
    }

    #[test]
    fn registry_round_trips_through_json() {
        let mut reg = PlattRegistry::new();
        reg.insert(
            PlattKey {
                detector_id: "clone-drift".to_string(),
                anomaly_class: AnomalyClass::Logic,
            },
            PlattParams { a: 1.5, b: -0.7 },
        );
        reg.insert(
            PlattKey {
                detector_id: "arg-swap".to_string(),
                anomaly_class: AnomalyClass::Interface,
            },
            PlattParams { a: 0.8, b: 0.2 },
        );

        let body = reg.to_json_pretty();
        let back = PlattRegistry::from_json(&body).expect("round-trip");
        assert_eq!(back, reg);
    }

    #[test]
    fn registry_rejects_unknown_anomaly_class() {
        let body = r#"{"clone-drift:Bogus":{"a":1.0,"b":0.0}}"#;
        let err = PlattRegistry::from_json(body).unwrap_err();
        assert!(matches!(err, PlattError::UnknownAnomalyClass(_)));
    }

    #[test]
    fn fit_registry_groups_by_cell() {
        let corpus = vec![
            LabelledLlmConfidence {
                detector_id: "clone-drift".to_string(),
                anomaly_class: AnomalyClass::Logic,
                raw_confidence: 0.9,
                verdict: Verdict::TruePositive,
            },
            LabelledLlmConfidence {
                detector_id: "clone-drift".to_string(),
                anomaly_class: AnomalyClass::Logic,
                raw_confidence: 0.3,
                verdict: Verdict::FalsePositive,
            },
            LabelledLlmConfidence {
                detector_id: "arg-swap".to_string(),
                anomaly_class: AnomalyClass::Interface,
                raw_confidence: 0.8,
                verdict: Verdict::TruePositive,
            },
        ];
        let reg = fit_registry(&corpus).expect("fit");
        assert_eq!(reg.len(), 2);
        assert!(reg.get("clone-drift", AnomalyClass::Logic).is_some());
        assert!(reg.get("arg-swap", AnomalyClass::Interface).is_some());
        assert!(reg.get("clone-drift", AnomalyClass::Interface).is_none());
    }

    #[test]
    fn fit_registry_rejects_empty_corpus() {
        let err = fit_registry(&[]).unwrap_err();
        assert!(matches!(err, PlattError::EmptyCorpus));
    }

    #[test]
    fn composite_key_round_trips() {
        let key = PlattKey {
            detector_id: "clone-drift".to_string(),
            anomaly_class: AnomalyClass::Logic,
        };
        let s = key.composite();
        assert_eq!(s, "clone-drift:Logic");
        let back = PlattKey::from_composite(&s).unwrap();
        assert_eq!(back, key);
    }
}
