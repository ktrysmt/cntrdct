//! Tests for `cntrdct-research overlap`.
//!
//! Exercises the full pipeline against synthetic findings.json + a tiny
//! `<clippy_dir>/<crate>-<ver>.clippy.json` directory. No network, no
//! cargo, no real Rust toolchain.

use cntrdct_research::run_overlap;
use serde_json::json;

fn cntrdct_finding(detector: &str, file: &str, line: u32) -> serde_json::Value {
    json!({
        "finding": {
            "detector_id": detector,
            "primary": {
                "file": file,
                "start_line": line,
                "start_col": 1,
                "end_line": line + 1,
                "end_col": 1,
            },
            "related": [],
            "message": "synthetic",
            "raw_severity": "Warning",
            "anomaly_class": "Logic",
            "evidence": {"citation_keys": [], "raw": {}},
        },
        "posterior_tp": null,
        "wilson_lower": null,
        "rank_score": 1.0,
    })
}

fn clippy_diagnostic(lint: &str, file_name: &str, line_start: u32) -> serde_json::Value {
    json!({
        "reason": "compiler-message",
        "package_id": "synthetic",
        "manifest_path": "/tmp/Cargo.toml",
        "target": {"name": "synthetic"},
        "message": {
            "rendered": "warning: synthetic",
            "children": [],
            "code": {"code": lint, "explanation": null},
            "level": "warning",
            "message": "synthetic",
            "spans": [{
                "file_name": file_name,
                "line_start": line_start,
                "column_start": 1,
                "line_end": line_start,
                "column_end": 1,
            }],
        },
    })
}

fn write_corpus(tmp: &std::path::Path) -> std::path::PathBuf {
    let corpus = tmp.join("corpus");
    std::fs::create_dir_all(corpus.join("serde-1.0.228/src")).unwrap();
    std::fs::create_dir_all(corpus.join("log-0.4.20/src")).unwrap();
    std::fs::write(corpus.join("serde-1.0.228/src/lib.rs"), "").unwrap();
    std::fs::write(corpus.join("log-0.4.20/src/lib.rs"), "").unwrap();
    corpus
}

fn write_findings(tmp: &std::path::Path, findings: &serde_json::Value) -> std::path::PathBuf {
    let p = tmp.join("findings.json");
    std::fs::write(&p, serde_json::to_string(findings).unwrap()).unwrap();
    p
}

fn write_clippy(
    tmp: &std::path::Path,
    crate_dir: &str,
    diagnostics: &serde_json::Value,
) -> std::path::PathBuf {
    let dir = tmp.join("clippy");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{crate_dir}.clippy.json"));
    std::fs::write(&p, serde_json::to_string(diagnostics).unwrap()).unwrap();
    dir
}

#[test]
fn overlap_counts_intersections_and_emits_long_format_csv() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = write_corpus(tmp.path());

    // cntrdct findings:
    //   - clone-drift @ serde-1.0.228/src/lib.rs:42  (overlaps with two clippy lints)
    //   - clone-drift @ log-0.4.20/src/lib.rs:7      (overlaps with one clippy lint)
    //   - arg-swap   @ serde-1.0.228/src/lib.rs:42   (same line as detection #1)
    //   - clone-drift @ serde-1.0.228/src/lib.rs:99  (no clippy match)
    let findings = json!([
        cntrdct_finding(
            "clone-drift",
            corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
            42,
        ),
        cntrdct_finding(
            "clone-drift",
            corpus.join("log-0.4.20/src/lib.rs").to_str().unwrap(),
            7,
        ),
        cntrdct_finding(
            "arg-swap",
            corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
            42,
        ),
        cntrdct_finding(
            "clone-drift",
            corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
            99,
        ),
    ]);
    let findings_path = write_findings(tmp.path(), &findings);

    // clippy for serde-1.0.228 has two lints at line 42 and one unrelated.
    let serde_clippy = json!([
        clippy_diagnostic("clippy::needless_borrow", "src/lib.rs", 42),
        clippy_diagnostic("clippy::unused_unit", "src/lib.rs", 42),
        clippy_diagnostic("clippy::needless_borrow", "src/lib.rs", 200),
    ]);
    write_clippy(tmp.path(), "serde-1.0.228", &serde_clippy);

    let log_clippy = json!([clippy_diagnostic(
        "clippy::needless_borrow",
        "src/lib.rs",
        7
    )]);
    let clippy_dir = write_clippy(tmp.path(), "log-0.4.20", &log_clippy);

    let out = tmp.path().join("overlap.csv");
    let rows = run_overlap(&findings_path, &clippy_dir, &corpus, Some(&out)).unwrap();

    // BTreeMap-ordered: detector asc, lint asc.
    // Expected pairings:
    //   (arg-swap, clippy::needless_borrow)  count=1  [serde:42]
    //   (arg-swap, clippy::unused_unit)      count=1  [serde:42]
    //   (clone-drift, clippy::needless_borrow) count=2 [serde:42, log:7]
    //   (clone-drift, clippy::unused_unit)   count=1  [serde:42]
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[0],
        cntrdct_research::OverlapRow {
            detector: "arg-swap".into(),
            clippy_lint: "clippy::needless_borrow".into(),
            count: 1
        }
    );
    assert_eq!(
        rows[1],
        cntrdct_research::OverlapRow {
            detector: "arg-swap".into(),
            clippy_lint: "clippy::unused_unit".into(),
            count: 1
        }
    );
    assert_eq!(
        rows[2],
        cntrdct_research::OverlapRow {
            detector: "clone-drift".into(),
            clippy_lint: "clippy::needless_borrow".into(),
            count: 2
        }
    );
    assert_eq!(
        rows[3],
        cntrdct_research::OverlapRow {
            detector: "clone-drift".into(),
            clippy_lint: "clippy::unused_unit".into(),
            count: 1
        }
    );

    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.starts_with("detector,clippy_lint,count\n"));
    assert!(body.contains("clone-drift,clippy::needless_borrow,2\n"));
}

#[test]
fn overlap_skips_findings_without_clippy_match() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = write_corpus(tmp.path());
    let findings = json!([cntrdct_finding(
        "clone-drift",
        corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
        99,
    )]);
    let findings_path = write_findings(tmp.path(), &findings);
    let serde_clippy = json!([clippy_diagnostic(
        "clippy::needless_borrow",
        "src/lib.rs",
        42
    )]);
    let clippy_dir = write_clippy(tmp.path(), "serde-1.0.228", &serde_clippy);

    let rows = run_overlap(&findings_path, &clippy_dir, &corpus, None).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn overlap_dedups_clippy_lints_emitted_multiple_times_at_same_location() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = write_corpus(tmp.path());
    let findings = json!([cntrdct_finding(
        "clone-drift",
        corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
        42,
    )]);
    let findings_path = write_findings(tmp.path(), &findings);
    // Same lint × line repeated three times — could happen when clippy
    // emits one diagnostic per macro expansion.
    let serde_clippy = json!([
        clippy_diagnostic("clippy::needless_borrow", "src/lib.rs", 42),
        clippy_diagnostic("clippy::needless_borrow", "src/lib.rs", 42),
        clippy_diagnostic("clippy::needless_borrow", "src/lib.rs", 42),
    ]);
    let clippy_dir = write_clippy(tmp.path(), "serde-1.0.228", &serde_clippy);

    let rows = run_overlap(&findings_path, &clippy_dir, &corpus, None).unwrap();
    assert_eq!(rows.len(), 1);
    // Dedup at the location level: a single (lint, location) → counts
    // once even though clippy reported it three times. The cntrdct
    // finding contributes a single observation to that bucket.
    assert_eq!(rows[0].count, 1);
}

#[test]
fn overlap_skips_findings_outside_corpus_root() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = write_corpus(tmp.path());
    let outside = tmp.path().join("not-corpus/foo.rs");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "").unwrap();

    let findings = json!([cntrdct_finding("clone-drift", outside.to_str().unwrap(), 1)]);
    let findings_path = write_findings(tmp.path(), &findings);
    let clippy_dir = tmp.path().join("clippy");
    std::fs::create_dir_all(&clippy_dir).unwrap();

    let rows = run_overlap(&findings_path, &clippy_dir, &corpus, None).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn overlap_ignores_unrelated_files_in_clippy_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = write_corpus(tmp.path());
    let findings = json!([cntrdct_finding(
        "clone-drift",
        corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
        42
    )]);
    let findings_path = write_findings(tmp.path(), &findings);

    let serde_clippy = json!([clippy_diagnostic(
        "clippy::needless_borrow",
        "src/lib.rs",
        42
    )]);
    let clippy_dir = write_clippy(tmp.path(), "serde-1.0.228", &serde_clippy);
    // Stray summary.json should not break the run.
    std::fs::write(clippy_dir.join("summary.json"), "{\"processed\":1}").unwrap();
    // Stray non-JSON file should also be tolerated.
    std::fs::write(clippy_dir.join("README.txt"), "ignore me").unwrap();

    let rows = run_overlap(&findings_path, &clippy_dir, &corpus, None).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].count, 1);
}
