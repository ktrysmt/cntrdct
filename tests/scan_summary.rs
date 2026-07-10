//! Integration tests for the S-1 scan summary printed to stderr.
//!
//! Spec: `docs/spec/scan-summary-v0.md`. The summary must show
//! per-detector finding counts plus the labelled-corpus precision the
//! ranker used (P4: read from the resolved priors verbatim), and must
//! never contaminate stdout (which stays a clean JSON / SARIF
//! document).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::tempdir;

const DEAD_CODE: &str = "fn f() { return; bar(); }\n";

/// A priors file with recognizable values so the test does not depend
/// on the embedded defaults or on a user's `~/.cache/cntrdct/priors.json`.
const PRIORS: &str = r#"{
  "unreachable-after-terminator": {
    "tp": 40,
    "fp": 10,
    "posterior_tp": 0.788,
    "wilson_lower_95": 0.67,
    "prior_method": "wilson"
  }
}"#;

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

fn scan(args: &[&str]) -> Output {
    Command::new(cntrdct_bin())
        .arg("scan")
        .args(args)
        .output()
        .expect("invoke cntrdct scan")
}

#[test]
fn summary_reports_count_and_corpus_derived_precision() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let priors_path = dir.path().join("priors.json");
    fs::write(&priors_path, PRIORS).unwrap();

    let out = scan(&[
        dir.path().to_str().unwrap(),
        "--priors",
        priors_path.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("scan summary: 1 finding(s) across 1 detector(s) in 1 file(s)"),
        "got: {}",
        stderr
    );
    assert!(
        stderr.contains("unreachable-after-terminator"),
        "got: {}",
        stderr
    );
    assert!(
        stderr.contains("est. precision >= 0.67 (wilson 95% lower bound, n=50 labelled)"),
        "precision must be read verbatim from the priors file (P4); got: {}",
        stderr
    );

    // stdout stays a clean JSON array.
    let findings: Vec<serde_json::Value> =
        serde_json::from_slice(&out.stdout).expect("stdout must stay parseable JSON");
    assert_eq!(findings.len(), 1);
}

#[test]
fn summary_marks_missing_calibration_data() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();

    let out = scan(&[dir.path().to_str().unwrap(), "--no-calibration"]);
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("(no calibration data)"),
        "uncalibrated runs must not invent precision numbers (P4); got: {}",
        stderr
    );
}

#[test]
fn summary_appears_for_clean_scans_too() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("clean.rs"), "fn f() -> i32 { 1 }\n").unwrap();

    let out = scan(&[dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("scan summary: 0 finding(s) across 0 detector(s) in 1 file(s)"),
        "got: {}",
        stderr
    );
}
