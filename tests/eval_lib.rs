//! Integration tests for the cntrdct eval harness v0 spec.
//!
//! Each test maps to a row in `cntrdct/docs/spec/eval-v0.md` test plan.

use std::fs;
use std::path::{Path, PathBuf};

use cntrdct::core::{AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, Severity};
use cntrdct::eval::{evaluate, load_manifest, EvalError, ExpectedFinding, ManifestEntry};
use tempfile::tempdir;

fn finding_at(detector_id: &str, file: &Path, line: u32) -> Finding {
    Finding {
        detector_id: detector_id.to_string(),
        primary: Location {
            file: file.to_path_buf(),
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 1,
        },
        related: vec![],
        message: "test".to_string(),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["test"],
            raw: serde_json::Value::Null,
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
    }
}

// ---------- L1-L4: load_manifest ----------

#[test]
fn l1_load_manifest_parses_valid_jsonl() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("manifest.jsonl");
    fs::write(
        &path,
        r#"{"file":"files/a.rs","expected":[{"detector_id":"clone-drift","line":3}]}
{"file":"files/b.rs","expected":[]}
"#,
    )
    .unwrap();
    let m = load_manifest(&path).expect("manifest loads");
    assert_eq!(m.entries.len(), 2);
    assert_eq!(m.entries[0].file, PathBuf::from("files/a.rs"));
    assert_eq!(m.entries[0].expected.len(), 1);
    assert_eq!(m.entries[0].expected[0].detector_id, "clone-drift");
    assert_eq!(m.entries[0].expected[0].line, 3);
    assert!(m.entries[1].expected.is_empty());
}

#[test]
fn l2_load_manifest_skips_blank_and_comment_lines() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("manifest.jsonl");
    fs::write(
        &path,
        "// header comment\n\n{\"file\":\"a.rs\",\"expected\":[]}\n  // indented comment\n\n{\"file\":\"b.rs\",\"expected\":[]}\n",
    )
    .unwrap();
    let m = load_manifest(&path).expect("manifest loads");
    assert_eq!(m.entries.len(), 2, "blank and // lines must be skipped");
}

#[test]
fn l3_load_manifest_reports_one_based_line_on_parse_failure() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("manifest.jsonl");
    fs::write(
        &path,
        "{\"file\":\"a.rs\",\"expected\":[]}\n{ this is not json\n",
    )
    .unwrap();
    let result = load_manifest(&path);
    match result {
        Err(EvalError::Parse { line, .. }) => assert_eq!(line, 2),
        other => panic!("expected Parse error on line 2, got {:?}", other),
    }
}

#[test]
fn l4_load_manifest_empty_file_yields_zero_entries() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("manifest.jsonl");
    fs::write(&path, "").unwrap();
    let m = load_manifest(&path).expect("empty manifest loads");
    assert!(m.entries.is_empty());
}

// ---------- E1-E10: evaluate ----------

fn manifest(entries: Vec<ManifestEntry>) -> cntrdct::eval::Manifest {
    cntrdct::eval::Manifest { entries }
}

fn entry(file: &str, expected: Vec<(&str, u32)>) -> ManifestEntry {
    ManifestEntry {
        file: PathBuf::from(file),
        expected: expected
            .into_iter()
            .map(|(id, line)| ExpectedFinding {
                detector_id: id.to_string(),
                line,
            })
            .collect(),
        source: None,
        license: None,
        sha256: None,
    }
}

#[test]
fn e1_counts_tp_fp_fn_correctly() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![
        entry("files/a.rs", vec![("clone-drift", 10)]),
        entry("files/b.rs", vec![]),
    ]);
    let actual = vec![
        finding_at("clone-drift", &dir.path().join("files/a.rs"), 10), // TP
        finding_at("arg-swap", &dir.path().join("files/b.rs"), 5),     // FP
    ];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.tp, 1);
    assert_eq!(report.overall.fp, 1);
    assert_eq!(report.overall.fn_, 0);
}

#[test]
fn e2_breakdown_by_detector_id() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![
        entry("a.rs", vec![("clone-drift", 1), ("arg-swap", 7)]),
        entry("b.rs", vec![("comment-code", 3)]),
    ]);
    let actual = vec![
        finding_at("clone-drift", &dir.path().join("a.rs"), 1), // TP for clone-drift
        finding_at("arg-swap", &dir.path().join("a.rs"), 99),   // FP for arg-swap (wrong line)
        finding_at("comment-code", &dir.path().join("b.rs"), 3), // TP for comment-code
    ];
    let report = evaluate(&m, &actual, dir.path());
    let cd = report.per_detector.get("clone-drift").expect("has key");
    assert_eq!(cd.tp, 1);
    assert_eq!(cd.fp, 0);
    assert_eq!(cd.fn_, 0);
    let asw = report.per_detector.get("arg-swap").expect("has key");
    assert_eq!(asw.tp, 0);
    assert_eq!(asw.fp, 1);
    assert_eq!(asw.fn_, 1, "expected line 7 was missed");
    let cc = report.per_detector.get("comment-code").expect("has key");
    assert_eq!(cc.tp, 1);
}

#[test]
fn e3_one_to_one_matching_no_double_count() {
    let dir = tempdir().unwrap();
    // Manifest has ONE expected finding at (a.rs, line 5).
    let m = manifest(vec![entry("a.rs", vec![("clone-drift", 5)])]);
    // Detector emits TWO findings at the same location — duplicate scenario.
    let actual = vec![
        finding_at("clone-drift", &dir.path().join("a.rs"), 5),
        finding_at("clone-drift", &dir.path().join("a.rs"), 5),
    ];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.tp, 1, "TP must not be double-counted");
    assert_eq!(report.overall.fp, 1, "the second duplicate is FP");
}

#[test]
fn e4_matches_by_file_relative_to_corpus_dir_and_line_and_detector() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry("files/a.rs", vec![("clone-drift", 42)])]);
    // Use absolute path to corpus_dir/files/a.rs — must still match.
    let actual = vec![finding_at(
        "clone-drift",
        &dir.path().join("files/a.rs"),
        42,
    )];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(
        report.overall.tp, 1,
        "absolute actual path must be relativised"
    );
}

#[test]
fn e5_precision_zero_when_no_predictions() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry("a.rs", vec![("clone-drift", 1)])]);
    let actual: Vec<Finding> = vec![];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.precision, 0.0);
}

#[test]
fn e6_recall_zero_when_no_expected() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry("a.rs", vec![])]);
    let actual = vec![finding_at("clone-drift", &dir.path().join("a.rs"), 1)];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.recall, 0.0);
}

#[test]
fn e7_f1_zero_when_precision_plus_recall_zero() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry("a.rs", vec![])]);
    let actual: Vec<Finding> = vec![];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.f1, 0.0);
}

#[test]
fn e8_f1_with_balanced_precision_and_recall() {
    let dir = tempdir().unwrap();
    // Construct: 1 TP, 1 FP, 1 FN → precision = 0.5, recall = 0.5, f1 = 0.5.
    let m = manifest(vec![entry(
        "a.rs",
        vec![("clone-drift", 1), ("clone-drift", 2)],
    )]);
    let actual = vec![
        finding_at("clone-drift", &dir.path().join("a.rs"), 1), // TP
        finding_at("clone-drift", &dir.path().join("a.rs"), 99), // FP
                                                                // line 2 is FN
    ];
    let report = evaluate(&m, &actual, dir.path());
    assert!(
        (report.overall.precision - 0.5).abs() < 1e-9,
        "got {}",
        report.overall.precision
    );
    assert!(
        (report.overall.recall - 0.5).abs() < 1e-9,
        "got {}",
        report.overall.recall
    );
    assert!(
        (report.overall.f1 - 0.5).abs() < 1e-9,
        "got {}",
        report.overall.f1
    );
}

#[test]
fn e9_overall_aggregates_across_detectors() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry(
        "a.rs",
        vec![("clone-drift", 1), ("arg-swap", 2)],
    )]);
    let actual = vec![
        finding_at("clone-drift", &dir.path().join("a.rs"), 1),
        finding_at("arg-swap", &dir.path().join("a.rs"), 2),
    ];
    let report = evaluate(&m, &actual, dir.path());
    assert_eq!(report.overall.tp, 2);
    assert_eq!(report.overall.fp, 0);
    assert_eq!(report.overall.fn_, 0);
    assert_eq!(report.expected_total, 2);
    assert_eq!(report.actual_total, 2);
    assert_eq!(report.corpus_size, 1);
}

#[test]
fn e10_identical_inputs_produce_identical_reports() {
    let dir = tempdir().unwrap();
    let m = manifest(vec![entry("a.rs", vec![("clone-drift", 1)])]);
    let actual = vec![finding_at("clone-drift", &dir.path().join("a.rs"), 1)];
    let r1 = evaluate(&m, &actual, dir.path());
    let r2 = evaluate(&m, &actual, dir.path());
    let j1 = serde_json::to_string(&r1).unwrap();
    let j2 = serde_json::to_string(&r2).unwrap();
    assert_eq!(j1, j2);
}
