//! Integration tests for the ranker v0 + v1 spec.

use std::collections::HashMap;
use std::path::PathBuf;

use cntrdct::calibration::DetectorPrior;
use cntrdct::core::{
    AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, Ranker, Severity,
};
use cntrdct::ranker::{rank, CalibratedRanker, UncalibratedRanker};

fn loc(file: &str, line: u32) -> Location {
    Location {
        file: PathBuf::from(file),
        start_line: line,
        start_col: 1,
        end_line: line + 1,
        end_col: 1,
    }
}

fn make_finding(file: &str, line: u32, related_count: usize) -> Finding {
    make_finding_for("clone-drift", file, line, related_count)
}

fn make_finding_for(detector_id: &str, file: &str, line: u32, related_count: usize) -> Finding {
    Finding {
        detector_id: detector_id.to_string(),
        primary: loc(file, line),
        related: (0..related_count)
            .map(|i| loc("rel.rs", i as u32 + 1))
            .collect(),
        message: "diverged".to_string(),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["cordy-roy-icpc-2008"],
            raw: serde_json::Value::Null,
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
        origin: Default::default(),
    }
}

fn prior(tp: u32, fp: u32, posterior_tp: f64, wilson_lower_95: f64) -> DetectorPrior {
    DetectorPrior {
        tp,
        fp,
        posterior_tp,
        wilson_lower_95,
        prior_method: cntrdct::calibration::PriorMethod::Wilson,
    }
}

#[test]
fn t1_empty_input_returns_empty() {
    let out = rank(vec![]);
    assert!(out.is_empty());
}

#[test]
fn t2_one_finding_in_one_finding_out() {
    let out = rank(vec![make_finding("a.rs", 1, 3)]);
    assert_eq!(out.len(), 1);
}

#[test]
fn t3_orders_by_rank_score_desc() {
    let small = make_finding("a.rs", 10, 2);
    let large = make_finding("b.rs", 20, 5);
    let medium = make_finding("c.rs", 30, 3);
    let out = rank(vec![small, large, medium]);
    assert_eq!(out[0].rank_score, 5.0);
    assert_eq!(out[1].rank_score, 3.0);
    assert_eq!(out[2].rank_score, 2.0);
}

#[test]
fn t4_ties_break_by_file_path_asc() {
    let z = make_finding("z.rs", 1, 3);
    let a = make_finding("a.rs", 1, 3);
    let m = make_finding("m.rs", 1, 3);
    let out = rank(vec![z, a, m]);
    assert_eq!(out[0].finding.primary.file, PathBuf::from("a.rs"));
    assert_eq!(out[1].finding.primary.file, PathBuf::from("m.rs"));
    assert_eq!(out[2].finding.primary.file, PathBuf::from("z.rs"));
}

#[test]
fn t5_ties_break_by_line_asc_when_files_equal() {
    let f30 = make_finding("a.rs", 30, 3);
    let f10 = make_finding("a.rs", 10, 3);
    let f20 = make_finding("a.rs", 20, 3);
    let out = rank(vec![f30, f10, f20]);
    assert_eq!(out[0].finding.primary.start_line, 10);
    assert_eq!(out[1].finding.primary.start_line, 20);
    assert_eq!(out[2].finding.primary.start_line, 30);
}

#[test]
fn t6_posterior_is_none_in_v0() {
    let out = rank(vec![make_finding("a.rs", 1, 3)]);
    assert!(out[0].posterior_tp.is_none());
}

#[test]
fn t7_wilson_is_none_in_v0() {
    let out = rank(vec![make_finding("a.rs", 1, 3)]);
    assert!(out[0].wilson_lower.is_none());
}

#[test]
fn t8_rank_score_equals_related_count() {
    let f = make_finding("a.rs", 1, 7);
    let out = rank(vec![f]);
    assert_eq!(out[0].rank_score, 7.0);
}

#[test]
fn t9_uncalibrated_ranker_implements_trait() {
    let ranker: Box<dyn Ranker> = Box::new(UncalibratedRanker::new());
    let out = ranker.rank(vec![make_finding("a.rs", 1, 3)]);
    assert_eq!(out.len(), 1);
}

// ---------- v1: CalibratedRanker ----------

#[test]
fn calibrated_with_empty_priors_matches_uncalibrated() {
    let findings = vec![
        make_finding("z.rs", 1, 2),
        make_finding("a.rs", 1, 5),
        make_finding("m.rs", 7, 5),
    ];
    let calibrated: Box<dyn Ranker> = Box::new(CalibratedRanker::new(HashMap::new()));
    let uncalibrated: Box<dyn Ranker> = Box::new(UncalibratedRanker::new());
    let cal = calibrated.rank(findings.clone());
    let unc = uncalibrated.rank(findings);
    assert_eq!(cal.len(), unc.len());
    for (a, b) in cal.iter().zip(unc.iter()) {
        assert_eq!(a.finding.primary.file, b.finding.primary.file);
        assert_eq!(a.finding.primary.start_line, b.finding.primary.start_line);
        assert_eq!(a.rank_score, b.rank_score);
        assert!(a.posterior_tp.is_none());
        assert!(a.wilson_lower.is_none());
    }
}

#[test]
fn calibrated_uses_wilson_and_log_factor() {
    let mut priors: HashMap<String, DetectorPrior> = HashMap::new();
    priors.insert("clone-drift".to_string(), prior(80, 20, 0.7941, 0.7111));
    let ranker = CalibratedRanker::new(priors);
    let out = ranker.rank(vec![make_finding("a.rs", 1, 3)]);
    assert_eq!(out.len(), 1);
    // 1 + log2(1 + 3) = 1 + 2 = 3.0; 0.7111 * 3 = 2.1333
    let expected = 0.7111 * 3.0_f64;
    assert!(
        (out[0].rank_score - expected).abs() < 1e-9,
        "rank_score = {} (expected {})",
        out[0].rank_score,
        expected
    );
    assert_eq!(out[0].posterior_tp, Some(0.7941));
    assert_eq!(out[0].wilson_lower, Some(0.7111));
}

#[test]
fn calibrated_orders_two_detectors_by_wilson_lower() {
    let mut priors: HashMap<String, DetectorPrior> = HashMap::new();
    // Same `related.len()` so the log factor is identical; ordering is decided
    // by wilson_lower_95 alone.
    priors.insert("high-precision".to_string(), prior(90, 10, 0.892, 0.836));
    priors.insert("low-precision".to_string(), prior(10, 90, 0.108, 0.060));
    let ranker = CalibratedRanker::new(priors);

    let lo = make_finding_for("low-precision", "a.rs", 1, 2);
    let hi = make_finding_for("high-precision", "z.rs", 1, 2);
    let out = ranker.rank(vec![lo, hi]);
    assert_eq!(out[0].finding.detector_id, "high-precision");
    assert_eq!(out[1].finding.detector_id, "low-precision");
}

#[test]
fn calibrated_unknown_detector_falls_back_to_related_len() {
    let priors: HashMap<String, DetectorPrior> = HashMap::new();
    let ranker = CalibratedRanker::new(priors);
    let out = ranker.rank(vec![make_finding_for("never-seen", "a.rs", 1, 4)]);
    assert_eq!(out[0].rank_score, 4.0);
    assert!(out[0].posterior_tp.is_none());
    assert!(out[0].wilson_lower.is_none());
}

#[test]
fn calibrated_with_no_related_uses_log_one_equals_one() {
    // With related.len() == 0, the multiplier 1 + log2(1) = 1, so rank_score
    // collapses to wilson_lower_95.
    let mut priors: HashMap<String, DetectorPrior> = HashMap::new();
    priors.insert("clone-drift".to_string(), prior(50, 50, 0.5, 0.4038));
    let ranker = CalibratedRanker::new(priors);
    let out = ranker.rank(vec![make_finding("a.rs", 1, 0)]);
    assert!((out[0].rank_score - 0.4038).abs() < 1e-9);
}
