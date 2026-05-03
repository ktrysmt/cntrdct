//! Integration tests for `cntrdct eval` subcommand against the seed corpus.
//!
//! Spec: `docs/spec/eval-v0.md` test plan rows K1-K3.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above crates/cli")
        .to_path_buf()
}

fn cntrdct_bin() -> PathBuf {
    let exe = if cfg!(windows) {
        "cntrdct.exe"
    } else {
        "cntrdct"
    };
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
        .parent()
        .map(|p| p.join(exe))
        .unwrap_or_else(|| PathBuf::from("cntrdct"))
}

#[test]
fn k1_eval_against_seed_corpus_succeeds() {
    let corpus = workspace_root().join("benchmarks").join("corpus");
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(&corpus)
        .output()
        .expect("invoke cntrdct eval");
    assert!(
        output.status.success(),
        "eval exited non-zero: stdout={}, stderr={}",
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
    assert!(value.get("corpus_size").is_some(), "report has corpus_size");
}

#[test]
fn k2_seed_corpus_yields_nonzero_precision_and_recall() {
    let corpus = workspace_root().join("benchmarks").join("corpus");
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(&corpus)
        .output()
        .expect("invoke cntrdct eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {}\n{}", e, stdout));
    let precision = value["overall"]["precision"]
        .as_f64()
        .expect("overall.precision is a number");
    let recall = value["overall"]["recall"]
        .as_f64()
        .expect("overall.recall is a number");
    assert!(
        precision > 0.0,
        "seed corpus must produce > 0 precision, got {}",
        precision
    );
    assert!(
        recall > 0.0,
        "seed corpus must produce > 0 recall, got {}",
        recall
    );
}

#[test]
fn k3_eval_with_missing_manifest_exits_nonzero() {
    let bad = workspace_root()
        .join("benchmarks")
        .join("nonexistent_corpus_dir");
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(&bad)
        .output()
        .expect("invoke cntrdct eval");
    assert!(
        !output.status.success(),
        "eval against missing corpus must exit non-zero; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
