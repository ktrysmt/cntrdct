//! Integration tests for the cntrdct CLI v0 spec.

use std::fs;
use std::path::PathBuf;

use cntrdct_cli::{scan, ScanError};
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

#[test]
fn t1_scan_dir_with_drift_finds_one() {
    let dir = make_drift_dir();
    let findings = scan(dir.path()).expect("scan must succeed");
    assert_eq!(
        findings.len(),
        1,
        "expected 1 finding, got {:#?}",
        findings
    );
    assert!(
        findings[0].primary.file.ends_with("e.rs"),
        "primary should be e.rs, got {:?}",
        findings[0].primary.file
    );
}

#[test]
fn t2_empty_dir_returns_no_findings() {
    let dir = tempdir().unwrap();
    let findings = scan(dir.path()).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn t3_single_file_no_clones() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("only.rs"), FN_BASE).unwrap();
    let findings = scan(dir.path()).unwrap();
    assert!(
        findings.is_empty(),
        "single fn cannot drift, got {:#?}",
        findings
    );
}

#[test]
fn t4_path_not_found_errors() {
    let path = PathBuf::from("/nonexistent/cntrdct/test/path/does-not-exist");
    let result = scan(&path);
    match result {
        Err(ScanError::PathNotFound(_)) => {}
        other => panic!("expected PathNotFound, got {:?}", other),
    }
}

#[test]
fn t5_deterministic_across_runs() {
    let dir = make_drift_dir();
    let f1 = scan(dir.path()).unwrap();
    let f2 = scan(dir.path()).unwrap();
    let j1 = serde_json::to_string(&f1).unwrap();
    let j2 = serde_json::to_string(&f2).unwrap();
    assert_eq!(j1, j2, "scan output must be deterministic");
}

#[test]
fn unreachable_after_terminator_fires_through_cli_scan() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("dead.rs"),
        "fn f() { return; bar(); }\n",
    )
    .unwrap();
    let findings = scan(dir.path()).expect("scan must succeed");
    assert!(
        findings
            .iter()
            .any(|f| f.detector_id == "unreachable-after-terminator"),
        "expected at least one unreachable-after-terminator finding via CLI scan, got {:#?}",
        findings
    );
}

#[test]
fn t6_ignores_non_rs_files() {
    let dir = make_drift_dir();
    fs::write(dir.path().join("readme.md"), "# not rust").unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("config.yaml"), "key: value").unwrap();
    let findings = scan(dir.path()).unwrap();
    assert_eq!(
        findings.len(),
        1,
        "non-rust files must be ignored, got {:#?}",
        findings
    );
}
