//! Integration tests for the `cntrdct calibrate` flow and the
//! calibrated-vs-uncalibrated ranker selection used by `cntrdct scan`.
//!
//! Spec: `cntrdct/docs/spec/ranker-v1.md`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use cntrdct_calibration::DetectorPrior;
use cntrdct_cli::{calibrate, pick_ranker, rank_with_calibration, scan};
use cntrdct_core::{
    AnomalyClass, Evidence, Finding, Location, RankedFinding, Severity,
};
use tempfile::tempdir;

const FN_BASE: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
        }
    }
    result
}
"#;

const FN_DRIFTED: &str = r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 && item < 100 {
            result.push(item * 2);
        }
    }
    result
}
"#;

fn make_drift_dir() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    for (name, src) in [
        ("a.rs", FN_BASE),
        ("b.rs", FN_BASE),
        ("c.rs", FN_BASE),
        ("d.rs", FN_BASE),
        ("e.rs", FN_DRIFTED),
    ] {
        fs::write(dir.path().join(name), src).unwrap();
    }
    dir
}

fn make_finding(detector_id: &str, related: usize) -> Finding {
    Finding {
        detector_id: detector_id.to_string(),
        primary: Location {
            file: PathBuf::from("a.rs"),
            start_line: 1,
            start_col: 1,
            end_line: 2,
            end_col: 1,
        },
        related: (0..related)
            .map(|i| Location {
                file: PathBuf::from("rel.rs"),
                start_line: i as u32 + 1,
                start_col: 1,
                end_line: i as u32 + 2,
                end_col: 1,
            })
            .collect(),
        message: "demo".to_string(),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["cordy-roy-icpc-2008"],
            raw: serde_json::Value::Null,
        },
    }
}

fn write_corpus(path: &std::path::Path) {
    let body = "{\"detector_id\":\"clone-drift\",\"repo\":\"r\",\"file\":\"a.rs\",\"line\":1,\"verdict\":\"TruePositive\"}\n\
                {\"detector_id\":\"clone-drift\",\"repo\":\"r\",\"file\":\"b.rs\",\"line\":2,\"verdict\":\"TruePositive\"}\n\
                {\"detector_id\":\"clone-drift\",\"repo\":\"r\",\"file\":\"c.rs\",\"line\":3,\"verdict\":\"FalsePositive\"}\n\
                {\"detector_id\":\"arg-swap\",\"repo\":\"r\",\"file\":\"d.rs\",\"line\":4,\"verdict\":\"TruePositive\"}\n\
                {\"detector_id\":\"arg-swap\",\"repo\":\"r\",\"file\":\"e.rs\",\"line\":5,\"verdict\":\"FalsePositive\"}\n";
    fs::write(path, body).unwrap();
}

#[test]
fn calibrate_writes_priors_with_expected_values() {
    let dir = tempdir().unwrap();
    let corpus_path = dir.path().join("corpus.jsonl");
    write_corpus(&corpus_path);
    let out = dir.path().join("priors.json");

    let n = calibrate(&corpus_path, &out).expect("calibrate succeeds");
    assert_eq!(n, 2, "two detectors should be present");

    let body = fs::read_to_string(&out).unwrap();
    let priors: HashMap<String, DetectorPrior> = serde_json::from_str(&body).unwrap();

    let cd = priors.get("clone-drift").expect("clone-drift present");
    assert_eq!(cd.tp, 2);
    assert_eq!(cd.fp, 1);
    // (2 + 1) / (3 + 2) = 0.6
    assert!((cd.posterior_tp - 0.6).abs() < 1e-3);
    // wilson_lower_95(2, 1) ≈ 0.2077
    assert!(
        (cd.wilson_lower_95 - 0.2077).abs() < 1e-3,
        "got {}",
        cd.wilson_lower_95
    );

    let asw = priors.get("arg-swap").expect("arg-swap present");
    assert_eq!(asw.tp, 1);
    assert_eq!(asw.fp, 1);
    // (1 + 1) / (2 + 2) = 0.5
    assert!((asw.posterior_tp - 0.5).abs() < 1e-3);
}

#[test]
fn calibrate_creates_parent_dirs() {
    let dir = tempdir().unwrap();
    let corpus = dir.path().join("c.jsonl");
    write_corpus(&corpus);
    let nested = dir.path().join("nested").join("deep").join("priors.json");
    calibrate(&corpus, &nested).expect("calibrate succeeds with nested output");
    assert!(nested.exists());
}

#[test]
fn scan_with_priors_override_uses_calibrated_ranker() {
    // Pre-build a priors file via calibrate(), then run scan() and rank with
    // the override pointed at it.
    let dir = tempdir().unwrap();
    let corpus = dir.path().join("c.jsonl");
    write_corpus(&corpus);
    let priors = dir.path().join("priors.json");
    calibrate(&corpus, &priors).unwrap();

    let scan_dir = make_drift_dir();
    let findings = scan(scan_dir.path()).expect("scan succeeds");
    assert!(!findings.is_empty(), "must produce at least one finding");
    let ranked: Vec<RankedFinding> =
        rank_with_calibration(findings, false, Some(&priors)).expect("rank ok");
    assert!(
        ranked.iter().any(|r| r.posterior_tp.is_some()),
        "with priors, at least one finding should carry posterior_tp"
    );
    assert!(
        ranked.iter().any(|r| r.wilson_lower.is_some()),
        "with priors, at least one finding should carry wilson_lower"
    );
}

#[test]
fn no_calibration_flag_forces_uncalibrated_ranker() {
    // Build priors and then explicitly request the uncalibrated ranker —
    // posterior_tp / wilson_lower must remain None even though priors exist.
    let dir = tempdir().unwrap();
    let corpus = dir.path().join("c.jsonl");
    write_corpus(&corpus);
    let priors = dir.path().join("priors.json");
    calibrate(&corpus, &priors).unwrap();

    let f = make_finding("clone-drift", 3);
    let ranked = rank_with_calibration(vec![f], true, Some(&priors))
        .expect("rank ok with no_calibration=true");
    assert_eq!(ranked.len(), 1);
    assert!(
        ranked[0].posterior_tp.is_none(),
        "--no-calibration must zero out calibration columns"
    );
    assert!(ranked[0].wilson_lower.is_none());
    assert_eq!(ranked[0].rank_score, 3.0); // uncalibrated == related.len()
}

#[test]
fn pick_ranker_with_missing_priors_path_falls_back_silently() {
    let bogus = PathBuf::from("/nonexistent/cntrdct/test/priors.json");
    let ranker = pick_ranker(false, Some(&bogus)).expect("no error on missing priors");
    let f = make_finding("clone-drift", 4);
    let out = ranker.rank(vec![f]);
    assert!(out[0].posterior_tp.is_none(), "fallback must be uncalibrated");
}
