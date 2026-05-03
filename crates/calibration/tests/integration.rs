//! Integration tests for the cntrdct calibration crate.
//!
//! Spec: `cntrdct/docs/spec/ranker-v1.md` (Layer 2 calibration).
//! References:
//! - `kremenek-engler-sas-2003` — Z-Ranking (TP/FP statistical priors).
//! - `jung-kim-shin-yi-sas-2005` — Bayesian post-analysis (Laplace smoothing).

use std::fs;

use cntrdct_calibration::{
    compute_priors, load_corpus, wilson_lower_95, CalibrationError, LabelledFinding, Verdict,
};
use cntrdct_core::AnomalyClass;
use tempfile::tempdir;

// ---------- Wilson 95% lower bound reference values ----------

/// Reference values are computed from the standard Wilson score interval at
/// z = 1.96 (95% confidence). They are stable to 1e-3 across implementations.
#[test]
fn wilson_80_tp_20_fp() {
    let got = wilson_lower_95(80, 20);
    let expected = 0.7111;
    assert!(
        (got - expected).abs() < 1e-3,
        "wilson_lower_95(80, 20) = {} (expected ~{})",
        got,
        expected
    );
}

#[test]
fn wilson_50_tp_50_fp() {
    let got = wilson_lower_95(50, 50);
    let expected = 0.4038;
    assert!(
        (got - expected).abs() < 1e-3,
        "wilson_lower_95(50, 50) = {} (expected ~{})",
        got,
        expected
    );
}

#[test]
fn wilson_1_tp_0_fp() {
    let got = wilson_lower_95(1, 0);
    let expected = 0.2065;
    assert!(
        (got - expected).abs() < 1e-3,
        "wilson_lower_95(1, 0) = {} (expected ~{})",
        got,
        expected
    );
}

#[test]
fn wilson_zero_zero_is_zero_by_convention() {
    // Document the convention: an observation-free detector has no evidence,
    // so its Wilson lower bound is defined as 0.0 (not NaN).
    let got = wilson_lower_95(0, 0);
    assert_eq!(got, 0.0, "wilson_lower_95(0, 0) must be 0.0 by convention");
}

// ---------- compute_priors ----------

#[test]
fn compute_priors_groups_by_detector_id_and_counts_tp_fp() {
    let corpus = vec![
        LabelledFinding {
            detector_id: "clone-drift".to_string(),
            repo: "demo".to_string(),
            file: "a.rs".to_string(),
            line: 1,
            verdict: Verdict::TruePositive,
            anomaly_class: None,
        },
        LabelledFinding {
            detector_id: "clone-drift".to_string(),
            repo: "demo".to_string(),
            file: "b.rs".to_string(),
            line: 2,
            verdict: Verdict::FalsePositive,
            anomaly_class: None,
        },
        LabelledFinding {
            detector_id: "arg-swap".to_string(),
            repo: "demo".to_string(),
            file: "c.rs".to_string(),
            line: 3,
            verdict: Verdict::TruePositive,
            anomaly_class: Some(AnomalyClass::Interface),
        },
    ];
    let priors = compute_priors(&corpus);

    let cd = priors
        .get("clone-drift")
        .expect("clone-drift prior present");
    assert_eq!(cd.tp, 1);
    assert_eq!(cd.fp, 1);

    let asw = priors.get("arg-swap").expect("arg-swap prior present");
    assert_eq!(asw.tp, 1);
    assert_eq!(asw.fp, 0);
}

#[test]
fn compute_priors_uses_laplace_posterior() {
    // Posterior_TP per Jung et al. (SAS 2005) Bayesian post-analysis with
    // a uniform Beta(1, 1) prior reduces to (TP + 1) / (TP + FP + 2).
    let corpus = vec![
        LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: "a.rs".to_string(),
            line: 1,
            verdict: Verdict::TruePositive,
            anomaly_class: None,
        },
        LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: "b.rs".to_string(),
            line: 2,
            verdict: Verdict::FalsePositive,
            anomaly_class: None,
        },
        LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: "c.rs".to_string(),
            line: 3,
            verdict: Verdict::TruePositive,
            anomaly_class: None,
        },
    ];
    let priors = compute_priors(&corpus);
    let p = priors.get("d").expect("prior present");
    // (2 + 1) / (3 + 2) = 0.6
    assert!((p.posterior_tp - 0.6).abs() < 1e-9);
}

#[test]
fn compute_priors_populates_wilson_lower() {
    let corpus = (0..80)
        .map(|i| LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: format!("a{}.rs", i),
            line: 1,
            verdict: Verdict::TruePositive,
            anomaly_class: None,
        })
        .chain((0..20).map(|i| LabelledFinding {
            detector_id: "d".to_string(),
            repo: "r".to_string(),
            file: format!("b{}.rs", i),
            line: 1,
            verdict: Verdict::FalsePositive,
            anomaly_class: None,
        }))
        .collect::<Vec<_>>();
    let priors = compute_priors(&corpus);
    let p = priors.get("d").expect("prior present");
    assert!((p.wilson_lower_95 - 0.7111).abs() < 1e-3);
}

#[test]
fn compute_priors_empty_corpus_returns_empty_map() {
    let priors = compute_priors(&[]);
    assert!(priors.is_empty());
}

// ---------- load_corpus ----------

#[test]
fn load_corpus_parses_valid_jsonl() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("corpus.jsonl");
    let body = r#"{"detector_id":"clone-drift","repo":"r","file":"a.rs","line":10,"verdict":"TruePositive"}
{"detector_id":"arg-swap","repo":"r","file":"b.rs","line":20,"verdict":"FalsePositive","anomaly_class":"Interface"}
"#;
    fs::write(&path, body).unwrap();
    let corpus = load_corpus(&path).expect("loads cleanly");
    assert_eq!(corpus.len(), 2);
    assert_eq!(corpus[0].detector_id, "clone-drift");
    assert!(matches!(corpus[0].verdict, Verdict::TruePositive));
    assert_eq!(corpus[0].anomaly_class, None);
    assert_eq!(corpus[1].detector_id, "arg-swap");
    assert!(matches!(corpus[1].verdict, Verdict::FalsePositive));
    assert_eq!(corpus[1].anomaly_class, Some(AnomalyClass::Interface));
}

#[test]
fn load_corpus_skips_blank_lines() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("corpus.jsonl");
    let body = "\n{\"detector_id\":\"d\",\"repo\":\"r\",\"file\":\"a.rs\",\"line\":1,\"verdict\":\"TruePositive\"}\n\n   \n";
    fs::write(&path, body).unwrap();
    let corpus = load_corpus(&path).expect("loads cleanly");
    assert_eq!(corpus.len(), 1);
}

#[test]
fn load_corpus_empty_file_is_ok() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.jsonl");
    fs::write(&path, "").unwrap();
    let corpus = load_corpus(&path).expect("empty is ok");
    assert!(corpus.is_empty());
}

#[test]
fn load_corpus_malformed_line_errors_with_line_number() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.jsonl");
    let body = r#"{"detector_id":"d","repo":"r","file":"a.rs","line":1,"verdict":"TruePositive"}
not valid json
"#;
    fs::write(&path, body).unwrap();
    let result = load_corpus(&path);
    match result {
        Err(CalibrationError::Parse { line, .. }) => {
            assert_eq!(line, 2, "error must report 1-based line number");
        }
        Ok(_) => panic!("expected parse error"),
        Err(other) => panic!("expected Parse, got {:?}", other),
    }
}

#[test]
fn example_corpus_fixture_loads() {
    // The fixture lives under crates/calibration/fixtures/example_corpus.jsonl
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("fixtures").join("example_corpus.jsonl");
    let corpus = load_corpus(&fixture).expect("example corpus loads");
    assert!(
        corpus.len() >= 5,
        "expected >= 5 entries, got {}",
        corpus.len()
    );

    let priors = compute_priors(&corpus);
    // Must cover all three registered detectors.
    assert!(priors.contains_key("clone-drift"));
    assert!(priors.contains_key("arg-swap"));
    assert!(priors.contains_key("comment-code"));
}
