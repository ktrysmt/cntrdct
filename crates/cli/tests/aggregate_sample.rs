//! Integration tests for `cntrdct aggregate` and `cntrdct sample`.
//!
//! Both subcommands read a JSON `Vec<RankedFinding>` and produce per-study
//! artefacts. Tests construct synthetic findings with the minimal subset of
//! the schema that the CLI inspects (`finding.detector_id`,
//! `finding.primary.file`); the full schema validation lives in the core
//! crate's serde tests.

use cntrdct_cli::{run_aggregate, run_sample};
use serde_json::json;

fn write_findings(dir: &std::path::Path, findings: &serde_json::Value) -> std::path::PathBuf {
    let path = dir.join("findings.json");
    std::fs::write(&path, serde_json::to_string(findings).unwrap()).unwrap();
    path
}

fn finding(detector: &str, file: &str) -> serde_json::Value {
    json!({
        "finding": {
            "detector_id": detector,
            "primary": {
                "file": file,
                "start_line": 1,
                "start_col": 1,
                "end_line": 2,
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

#[test]
fn aggregate_bins_findings_by_crate_dir_and_detector() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = tmp.path().join("corpus/wild");
    // Materialise the per-crate directories so canonicalisation succeeds
    // and the findings' file paths line up with the corpus root.
    std::fs::create_dir_all(corpus.join("serde-1.0.228/src")).unwrap();
    std::fs::create_dir_all(corpus.join("log-0.4.20/src")).unwrap();
    std::fs::write(corpus.join("serde-1.0.228/src/lib.rs"), "").unwrap();
    std::fs::write(corpus.join("log-0.4.20/src/lib.rs"), "").unwrap();

    let serde_lib = corpus.join("serde-1.0.228/src/lib.rs");
    let log_lib = corpus.join("log-0.4.20/src/lib.rs");
    let findings = json!([
        finding("clone-drift", serde_lib.to_str().unwrap()),
        finding("clone-drift", serde_lib.to_str().unwrap()),
        finding("arg-swap", serde_lib.to_str().unwrap()),
        finding("clone-drift", log_lib.to_str().unwrap()),
    ]);
    let findings_path = write_findings(tmp.path(), &findings);

    let csv_out = tmp.path().join("aggregate.csv");
    let rows = run_aggregate(&findings_path, &corpus, Some(&csv_out)).unwrap();

    // BTreeMap ordering: crate_dir asc, detector asc.
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].crate_dir, "log-0.4.20");
    assert_eq!(rows[0].detector, "clone-drift");
    assert_eq!(rows[0].count, 1);
    assert_eq!(rows[1].crate_dir, "serde-1.0.228");
    assert_eq!(rows[1].detector, "arg-swap");
    assert_eq!(rows[1].count, 1);
    assert_eq!(rows[2].crate_dir, "serde-1.0.228");
    assert_eq!(rows[2].detector, "clone-drift");
    assert_eq!(rows[2].count, 2);

    let body = std::fs::read_to_string(&csv_out).unwrap();
    assert!(body.starts_with("crate_dir,detector,count\n"));
    assert!(body.contains("serde-1.0.228,clone-drift,2\n"));
}

#[test]
fn aggregate_skips_findings_outside_corpus_root() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = tmp.path().join("corpus/wild");
    std::fs::create_dir_all(corpus.join("serde-1.0.228/src")).unwrap();
    std::fs::write(corpus.join("serde-1.0.228/src/lib.rs"), "").unwrap();

    // Second finding lives somewhere unrelated; aggregator should ignore it.
    let outside = tmp.path().join("unrelated/foo.rs");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "").unwrap();

    let findings = json!([
        finding(
            "clone-drift",
            corpus.join("serde-1.0.228/src/lib.rs").to_str().unwrap(),
        ),
        finding("clone-drift", outside.to_str().unwrap()),
    ]);
    let findings_path = write_findings(tmp.path(), &findings);

    let rows = run_aggregate(&findings_path, &corpus, None).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].crate_dir, "serde-1.0.228");
    assert_eq!(rows[0].count, 1);
}

#[test]
fn sample_returns_at_most_n_findings_per_detector() {
    let tmp = tempfile::tempdir().unwrap();
    let mut findings_arr: Vec<serde_json::Value> = Vec::new();
    for i in 0..10 {
        findings_arr.push(finding("clone-drift", &format!("a/{i}.rs")));
    }
    for i in 0..5 {
        findings_arr.push(finding("arg-swap", &format!("b/{i}.rs")));
    }
    let findings = serde_json::Value::Array(findings_arr);
    let path = write_findings(tmp.path(), &findings);

    let out = tmp.path().join("sample.json");
    let sample = run_sample(&path, 3, 42, Some(&out)).unwrap();

    // 3 from clone-drift + 3 from arg-swap = 6.
    assert_eq!(sample.len(), 6);

    let written: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(written.len(), 6);
}

#[test]
fn sample_is_deterministic_given_same_seed() {
    let tmp = tempfile::tempdir().unwrap();
    let mut findings_arr: Vec<serde_json::Value> = Vec::new();
    for i in 0..20 {
        findings_arr.push(finding("clone-drift", &format!("x/{i}.rs")));
    }
    let findings = serde_json::Value::Array(findings_arr);
    let path = write_findings(tmp.path(), &findings);

    let a = run_sample(&path, 5, 7, None).unwrap();
    let b = run_sample(&path, 5, 7, None).unwrap();
    let c = run_sample(&path, 5, 8, None).unwrap();

    // Identical seed → identical sample.
    let pick_files = |v: &[serde_json::Value]| -> Vec<String> {
        v.iter()
            .map(|f| {
                f["finding"]["primary"]["file"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    assert_eq!(pick_files(&a), pick_files(&b));
    // Different seed → different ordering (extremely likely with 20 inputs
    // and a 5-pick; failure here would point at a bug in the seed plumbing).
    assert_ne!(pick_files(&a), pick_files(&c));
}

#[test]
fn sample_handles_groups_smaller_than_per_detector() {
    let tmp = tempfile::tempdir().unwrap();
    let findings = json!([
        finding("clone-drift", "a/0.rs"),
        finding("clone-drift", "a/1.rs"),
    ]);
    let path = write_findings(tmp.path(), &findings);

    let sample = run_sample(&path, 30, 0, None).unwrap();
    assert_eq!(sample.len(), 2);
}

#[test]
fn aggregate_rejects_non_array_input() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("findings.json");
    std::fs::write(&path, "{\"not\":\"an array\"}").unwrap();
    let err = run_aggregate(&path, tmp.path(), None).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not a JSON array"), "got: {msg}");
}
