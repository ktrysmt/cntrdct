//! Integration tests for Q-14 recall-audit harness.
//!
//! Spec: `docs/spec/recall-audit-v0.md` test plan rows L1-L4, A1-A4, C1-C2.
//!
//! Unit-level matching arithmetic is exercised in
//! `src/recall_audit.rs::tests`. This file covers:
//! - manifest loader edge cases (skip lines, parse errors, missing
//!   `external_source` rejection),
//! - the orchestrator running scan + audit_recall against a synthetic
//!   fixture and reporting non-trivial recall,
//! - CLI happy path through `cntrdct calibrate --audit-recall`,
//! - CLI clap conflict between `--fit-platt` and `--audit-recall`.

use std::path::PathBuf;
use std::process::Command;

use cntrdct::recall_audit::{load_audit_manifest, RecallAuditError};
use cntrdct::run_recall_audit;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_dir() -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("recall-audit")
}

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

#[test]
fn l1_load_audit_manifest_parses_valid_jsonl() {
    let path = fixture_dir().join("manifest.jsonl");
    let manifest = load_audit_manifest(&path).expect("load fixture manifest");
    assert_eq!(manifest.entries.len(), 2);
    assert_eq!(manifest.entries[0].expected[0].detector_id, "arg-swap");
    assert_eq!(
        manifest.entries[0].expected[0].external_source.kind,
        "semgrep"
    );
    assert_eq!(
        manifest.entries[1].expected[0].external_source.kind,
        "clippy"
    );
}

#[test]
fn l2_load_audit_manifest_skips_blank_and_comment_lines() {
    let tmp = tempdir();
    let manifest_path = tmp.path().join("manifest.jsonl");
    std::fs::write(
        &manifest_path,
        "// header comment\n\n{\"file\":\"a.rs\",\"expected\":[]}\n\n",
    )
    .unwrap();
    let manifest = load_audit_manifest(&manifest_path).expect("loads with comments / blanks");
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(manifest.entries[0].file, PathBuf::from("a.rs"));
}

#[test]
fn l3_parse_error_reports_one_based_line() {
    let tmp = tempdir();
    let manifest_path = tmp.path().join("manifest.jsonl");
    // Line 1 valid, line 2 garbage.
    std::fs::write(
        &manifest_path,
        "{\"file\":\"a.rs\",\"expected\":[]}\n{not json\n",
    )
    .unwrap();
    let err = load_audit_manifest(&manifest_path).expect_err("expect parse error");
    match err {
        RecallAuditError::Parse { line, .. } => {
            assert_eq!(line, 2, "1-based line number reported");
        }
        other => panic!("expected Parse, got {:?}", other),
    }
}

#[test]
fn l4_missing_external_source_field_fails_to_load() {
    let tmp = tempdir();
    let manifest_path = tmp.path().join("manifest.jsonl");
    // `expected[0]` is missing the required `external_source` field.
    std::fs::write(
        &manifest_path,
        "{\"file\":\"a.rs\",\"expected\":[{\"detector_id\":\"arg-swap\",\"line\":3}]}\n",
    )
    .unwrap();
    let err = load_audit_manifest(&manifest_path).expect_err("missing field must fail");
    match err {
        RecallAuditError::Parse { .. } => {}
        other => panic!(
            "expected Parse for missing external_source, got {:?}",
            other
        ),
    }
}

#[test]
fn fixture_yields_full_recall_on_both_detectors() {
    // The synthetic fixture is constructed so the corresponding
    // detectors fire on the labelled lines. recall_upper_bound = 1.0
    // is the acceptance signal that the orchestrator wires the
    // scan output into the audit matcher correctly.
    let dir = fixture_dir();
    let manifest_path = dir.join("manifest.jsonl");
    let report = run_recall_audit(&dir, &manifest_path).expect("run recall audit");
    let arg_swap = &report.per_detector["arg-swap"];
    let unreach = &report.per_detector["unreachable-after-terminator"];
    assert_eq!(arg_swap.tp, 1, "arg-swap TP");
    assert_eq!(arg_swap.fn_, 0, "arg-swap FN");
    assert!((arg_swap.recall_upper_bound - 1.0).abs() < 1e-9);
    assert_eq!(unreach.tp, 1, "unreachable-after-terminator TP");
    assert_eq!(unreach.fn_, 0, "unreachable-after-terminator FN");
    assert_eq!(report.expected_total, 2);
    assert!(report.sources.contains_key("semgrep"));
    assert!(report.sources.contains_key("clippy"));
}

#[test]
fn a4_report_is_byte_stable_across_runs() {
    let dir = fixture_dir();
    let manifest_path = dir.join("manifest.jsonl");
    let r1 = run_recall_audit(&dir, &manifest_path).unwrap();
    let r2 = run_recall_audit(&dir, &manifest_path).unwrap();
    let s1 = serde_json::to_string_pretty(&r1).unwrap();
    let s2 = serde_json::to_string_pretty(&r2).unwrap();
    assert_eq!(
        s1, s2,
        "two runs over the same corpus must serialise identically"
    );
}

#[test]
fn c1_cli_audit_recall_exits_zero_and_prints_parseable_json() {
    let output = Command::new(cntrdct_bin())
        .arg("calibrate")
        .arg("--audit-recall")
        .arg(fixture_dir())
        .output()
        .expect("invoke cntrdct calibrate --audit-recall");
    assert!(
        output.status.success(),
        "calibrate --audit-recall exited non-zero: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {}\n{}", e, stdout));
    assert!(
        value.get("per_detector").is_some(),
        "report has per_detector"
    );
    assert!(value.get("overall").is_some(), "report has overall");
    assert!(
        value.get("sources").is_some(),
        "report has top-level sources mix"
    );
}

#[test]
fn c2_fit_platt_and_audit_recall_are_mutually_exclusive() {
    let output = Command::new(cntrdct_bin())
        .arg("calibrate")
        .arg("--fit-platt")
        .arg("--audit-recall")
        .arg(fixture_dir())
        .output()
        .expect("invoke cntrdct calibrate with conflicting flags");
    assert!(
        !output.status.success(),
        "clap should reject --fit-platt + --audit-recall"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "stderr should mention the conflict, got: {}",
        stderr
    );
}

// Minimal stdlib-only tempdir so this test file does not pull in `tempfile`.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

static TEMPDIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tempdir() -> TempDir {
    // Combine PID + a process-wide monotonic counter so parallel test
    // threads cannot collide on the path (the SystemTime-only variant
    // was flaky under cargo test's default parallelism).
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let n = TEMPDIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    p.push(format!("cntrdct-recall-audit-test-{}-{}", pid, n));
    std::fs::create_dir_all(&p).unwrap();
    TempDir { path: p }
}
