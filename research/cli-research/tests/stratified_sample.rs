//! Integration tests for `cntrdct-research stratified-sample`.
//!
//! Two-axis stratified sampling: `per_detector` cap on the per-detector
//! pool size, `max_per_crate` cap on each (detector, crate) bucket.
//! Reproducibility comes from a single seeded `fastrand::Rng` iterated
//! in `BTreeMap` order over (detector, crate).

use cntrdct_research::run_stratified_sample;
use serde_json::json;

fn write_findings(dir: &std::path::Path, findings: &serde_json::Value) -> std::path::PathBuf {
    let path = dir.join("findings.json");
    std::fs::write(&path, serde_json::to_string(findings).unwrap()).unwrap();
    path
}

fn finding(detector: &str, file: &str) -> serde_json::Value {
    finding_with_msg(detector, file, "synthetic")
}

fn finding_with_msg(detector: &str, file: &str, msg: &str) -> serde_json::Value {
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
            "message": msg,
            "raw_severity": "Warning",
            "anomaly_class": "Logic",
            "evidence": {"citation_keys": [], "raw": {}},
        },
        "posterior_tp": null,
        "wilson_lower": null,
        "rank_score": 1.0,
    })
}

fn make_corpus(tmp: &std::path::Path, crates: &[&str]) -> std::path::PathBuf {
    let corpus = tmp.join("corpus/wild");
    for c in crates {
        let dir = corpus.join(c).join("src");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lib.rs"), "").unwrap();
    }
    corpus
}

#[test]
fn caps_per_crate_within_each_detector() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["serde-1.0.228", "log-0.4.20"]);
    let serde_lib = corpus.join("serde-1.0.228/src/lib.rs");
    let log_lib = corpus.join("log-0.4.20/src/lib.rs");

    let mut items = Vec::new();
    for _ in 0..10 {
        items.push(finding("clone-drift", serde_lib.to_str().unwrap()));
        items.push(finding("clone-drift", log_lib.to_str().unwrap()));
    }
    let findings = serde_json::Value::Array(items);
    let path = write_findings(tmp.path(), &findings);

    // max_per_crate = 2, per_detector = 5 => 2 from each crate, total 4.
    let sample = run_stratified_sample(&path, &corpus, 5, 2, 42, None).unwrap();
    assert_eq!(sample.len(), 4);
    // The sampled JSON preserves the original (non-canonicalised) path
    // string as it appeared in the input findings.
    let serde_count = sample
        .iter()
        .filter(|f| f["finding"]["primary"]["file"] == serde_lib.to_str().unwrap())
        .count();
    let log_count = sample
        .iter()
        .filter(|f| f["finding"]["primary"]["file"] == log_lib.to_str().unwrap())
        .count();
    assert_eq!(serde_count, 2);
    assert_eq!(log_count, 2);
}

#[test]
fn down_samples_when_per_detector_exceeded() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0", "b-1.0.0", "c-1.0.0"]);
    let mut items = Vec::new();
    for c in ["a-1.0.0", "b-1.0.0", "c-1.0.0"] {
        let lib = corpus.join(c).join("src/lib.rs");
        for _ in 0..5 {
            items.push(finding("clone-drift", lib.to_str().unwrap()));
        }
    }
    let findings = serde_json::Value::Array(items);
    let path = write_findings(tmp.path(), &findings);

    // 4 per crate * 3 crates = 12 in pool, then down-sampled to 5.
    let sample = run_stratified_sample(&path, &corpus, 5, 4, 0, None).unwrap();
    assert_eq!(sample.len(), 5);
}

#[test]
fn deterministic_with_same_seed() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0", "b-1.0.0"]);
    // Findings carry distinct messages so that two seeds can be
    // distinguished by the actual subset selected, not just by the
    // order pattern of indistinguishable objects.
    let mut items = Vec::new();
    for (i, c) in ["a-1.0.0", "b-1.0.0"].iter().enumerate() {
        let lib = corpus.join(c).join("src/lib.rs");
        for j in 0..10 {
            items.push(finding_with_msg(
                "clone-drift",
                lib.to_str().unwrap(),
                &format!("syn-{}-{}", i, j),
            ));
        }
    }
    let findings = serde_json::Value::Array(items);
    let path = write_findings(tmp.path(), &findings);

    let s1 = run_stratified_sample(&path, &corpus, 5, 3, 7, None).unwrap();
    let s2 = run_stratified_sample(&path, &corpus, 5, 3, 7, None).unwrap();
    assert_eq!(s1, s2);
    let s3 = run_stratified_sample(&path, &corpus, 5, 3, 8, None).unwrap();
    assert_ne!(s1, s3);
}

#[test]
fn writes_to_output_path_and_creates_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0"]);
    let lib = corpus.join("a-1.0.0/src/lib.rs");
    let findings = serde_json::Value::Array(vec![finding("clone-drift", lib.to_str().unwrap())]);
    let path = write_findings(tmp.path(), &findings);
    let out = tmp.path().join("nested/dir/sample.json");

    run_stratified_sample(&path, &corpus, 30, 5, 0, Some(&out)).unwrap();
    let body = std::fs::read_to_string(&out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn skips_findings_outside_corpus_root() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0"]);
    let inside = corpus.join("a-1.0.0/src/lib.rs");
    let outside = tmp.path().join("unrelated/foo.rs");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "").unwrap();

    let findings = json!([
        finding("clone-drift", inside.to_str().unwrap()),
        finding("clone-drift", outside.to_str().unwrap()),
    ]);
    let path = write_findings(tmp.path(), &findings);
    let sample = run_stratified_sample(&path, &corpus, 30, 5, 0, None).unwrap();
    assert_eq!(sample.len(), 1);
}

#[test]
fn separates_by_detector() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0"]);
    let lib = corpus.join("a-1.0.0/src/lib.rs");
    let mut items = Vec::new();
    for _ in 0..5 {
        items.push(finding("clone-drift", lib.to_str().unwrap()));
        items.push(finding("arg-swap", lib.to_str().unwrap()));
    }
    let findings = serde_json::Value::Array(items);
    let path = write_findings(tmp.path(), &findings);

    // max_per_crate = 2, per_detector cap large => 2 per detector * 2 detectors = 4.
    let sample = run_stratified_sample(&path, &corpus, 30, 2, 0, None).unwrap();
    assert_eq!(sample.len(), 4);
    let drift = sample
        .iter()
        .filter(|f| f["finding"]["detector_id"] == "clone-drift")
        .count();
    let swap = sample
        .iter()
        .filter(|f| f["finding"]["detector_id"] == "arg-swap")
        .count();
    assert_eq!(drift, 2);
    assert_eq!(swap, 2);
}

#[test]
fn empty_findings_array_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let corpus = make_corpus(tmp.path(), &["a-1.0.0"]);
    let findings = serde_json::Value::Array(Vec::new());
    let path = write_findings(tmp.path(), &findings);
    let sample = run_stratified_sample(&path, &corpus, 30, 5, 0, None).unwrap();
    assert!(sample.is_empty());
}
