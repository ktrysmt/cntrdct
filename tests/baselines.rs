//! Integration tests for Q-15 baseline-comparator harness.
//!
//! Spec: `docs/spec/sota-baselines-v0.md` test plan rows
//! L1, L4, L5, L6, C1, C2, C3, C4, R1, R2, R3.
//!
//! Unit-level coverage of the load + compare path lives in
//! `src/baselines.rs::tests`. This file covers the orchestrator and
//! the CLI surface, ensuring the `--baseline` / `--baselines-out` /
//! `--baselines-skip-run` flags compose with the existing
//! `cntrdct eval` path without regressing the no-flag case.
//!
//! Phase B does not exercise live Docker invocation — see
//! `docs/spec/sota-baselines-v0.md` Out of scope. The R1 test uses
//! `--baselines-skip-run` so CI can run on every push without
//! depending on the registry image.
//!
//! Note: Phase B's REGISTRY ships only the `sourcerercc` entry.
//! `pybuglab` lands under Phase C; the L2 row from the spec test
//! plan ("parses valid pybuglab JSONL") is exercised at unit-test
//! level via `load_baseline_jsonl` taking the expected tool name as
//! a parameter (see `src/baselines.rs::tests`).

use std::path::{Path, PathBuf};
use std::process::Command;

use cntrdct::baselines::{find_baseline, load_baseline_jsonl, BaselineError};
use cntrdct::{embedded_priors_sha256, run_eval_with_baselines};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus_dir() -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("baselines")
        .join("corpus")
}

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

fn release_tag() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

// ---------- Loader (spec L1 / L5 / L6 — duplicated at integration scope) ----------

#[test]
fn l1_load_baseline_jsonl_parses_cached_sourcerercc() {
    // The shipped fixture is empty by design. The loader still has
    // to return Ok([]).
    let path = corpus_dir()
        .parent()
        .unwrap()
        .join("baselines")
        .join(release_tag())
        .join("sourcerercc.jsonl");
    let rows = load_baseline_jsonl(&path, "sourcerercc", "clone-drift")
        .expect("load cached sourcerercc JSONL");
    assert!(rows.is_empty());
}

#[test]
fn l4_load_baseline_jsonl_rejects_unknown_baseline_via_registry() {
    // `find_baseline` is the gate the CLI uses before reaching
    // `load_baseline_jsonl`. An unknown name produces None here and
    // a BaselineError::UnknownBaseline at the orchestrator level
    // (covered by R2 below).
    assert!(find_baseline("pybuglab").is_none(), "Phase C only");
    assert!(find_baseline("unknown").is_none());
    assert!(find_baseline("sourcerercc").is_some());
}

// ---------- Compare-one + report assembly (spec C1 / C3) ----------

#[test]
fn c1_orchestrator_produces_one_comparison_per_baseline() {
    let manifest = corpus_dir().join("manifest.jsonl");
    let (_eval, report) = run_eval_with_baselines(
        &corpus_dir(),
        &manifest,
        &["sourcerercc".to_string()],
        true, // skip_run reads the cached JSONL
        &release_tag(),
    )
    .expect("orchestrator runs against fixture");
    assert_eq!(report.comparisons.len(), 1);
    assert_eq!(report.comparisons[0].cntrdct.tool, "cntrdct");
    assert_eq!(report.comparisons[0].baseline.tool, "sourcerercc");
    assert_eq!(report.comparisons[0].detector_id, "clone-drift");
}

#[test]
fn c3_report_is_byte_stable_across_runs() {
    let manifest = corpus_dir().join("manifest.jsonl");
    let (_e1, r1) = run_eval_with_baselines(
        &corpus_dir(),
        &manifest,
        &["sourcerercc".to_string()],
        true,
        &release_tag(),
    )
    .unwrap();
    let (_e2, r2) = run_eval_with_baselines(
        &corpus_dir(),
        &manifest,
        &["sourcerercc".to_string()],
        true,
        &release_tag(),
    )
    .unwrap();
    let j1 = serde_json::to_string_pretty(&r1).unwrap();
    let j2 = serde_json::to_string_pretty(&r2).unwrap();
    assert_eq!(j1, j2);
}

#[test]
fn c4_report_priors_hash_matches_embedded() {
    let manifest = corpus_dir().join("manifest.jsonl");
    let (_eval, report) = run_eval_with_baselines(
        &corpus_dir(),
        &manifest,
        &["sourcerercc".to_string()],
        true,
        &release_tag(),
    )
    .unwrap();
    assert_eq!(
        report.priors_default_sha256,
        embedded_priors_sha256(),
        "H6.3 structural guard wiring: report hash must equal embedded hash",
    );
    // Defensive: the hash is non-empty and looks like a hex string.
    assert_eq!(report.priors_default_sha256.len(), 64);
    assert!(report
        .priors_default_sha256
        .chars()
        .all(|c| c.is_ascii_hexdigit()));
}

// ---------- CLI (spec R1 / R2 / R3) ----------

#[test]
fn r1_cli_with_baseline_skip_run_exits_zero_and_prints_both_reports() {
    let manifest = corpus_dir().join("manifest.jsonl");
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(corpus_dir())
        .arg("--manifest")
        .arg(&manifest)
        .arg("--baseline")
        .arg("sourcerercc")
        .arg("--baselines-skip-run")
        .output()
        .expect("run cntrdct eval --baseline");
    assert!(
        output.status.success(),
        "cntrdct eval --baseline failed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // Stdout carries two pretty-printed JSON values concatenated.
    // serde_json::Deserializer streaming handles that cleanly.
    let mut stream = serde_json::Deserializer::from_str(&stdout).into_iter::<serde_json::Value>();
    let eval_report = stream
        .next()
        .expect("EvalReport present")
        .expect("EvalReport parses");
    let baseline_report = stream
        .next()
        .expect("BaselineComparisonReport present")
        .expect("BaselineComparisonReport parses");

    // EvalReport carries the eval-v0 §F4 shape; we don't pin
    // specific numbers (the corpus has zero expected findings).
    assert!(eval_report.get("overall").is_some());

    // BaselineComparisonReport: one comparison cell, priors hash
    // present, release_tag matches the binary's version.
    let comparisons = baseline_report
        .get("comparisons")
        .and_then(|v| v.as_array())
        .expect("comparisons array");
    assert_eq!(comparisons.len(), 1);
    assert_eq!(
        baseline_report
            .get("release_tag")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        release_tag()
    );
    assert!(baseline_report
        .get("priors_default_sha256")
        .and_then(|v| v.as_str())
        .map(|s| s.len() == 64)
        .unwrap_or(false));
}

#[test]
fn r2_cli_with_unknown_baseline_exits_non_zero() {
    let manifest = corpus_dir().join("manifest.jsonl");
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(corpus_dir())
        .arg("--manifest")
        .arg(&manifest)
        .arg("--baseline")
        .arg("definitely-not-a-baseline")
        .arg("--baselines-skip-run")
        .output()
        .expect("run cntrdct eval --baseline");
    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown baseline; stdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("definitely-not-a-baseline"),
        "stderr should name the unknown baseline; got:\n{stderr}"
    );
}

#[test]
fn r3_cli_without_baseline_runs_existing_eval_path_unchanged() {
    let manifest = corpus_dir().join("manifest.jsonl");
    // The --baselines-skip-run flag is set, but --baseline is empty;
    // the orchestrator falls back to the existing eval path and the
    // skip-run flag is a no-op.
    let output = Command::new(cntrdct_bin())
        .arg("eval")
        .arg(corpus_dir())
        .arg("--manifest")
        .arg(&manifest)
        .arg("--baselines-skip-run")
        .output()
        .expect("run cntrdct eval");
    assert!(
        output.status.success(),
        "cntrdct eval (no baseline) failed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut stream = serde_json::Deserializer::from_str(&stdout).into_iter::<serde_json::Value>();
    let report = stream
        .next()
        .expect("EvalReport present")
        .expect("EvalReport parses");
    assert!(report.get("overall").is_some());
    // No second value: BaselineComparisonReport must not be emitted
    // when --baseline is empty.
    assert!(
        stream.next().is_none(),
        "expected a single EvalReport JSON value when --baseline is empty"
    );
}

// ---------- Defensive: cached JSONL missing path ----------

#[test]
fn cached_jsonl_missing_returns_a_baseline_error() {
    // Point at a temp corpus dir whose cached JSONL does not exist.
    let tmp = tempfile::tempdir().unwrap();
    // The fixture corpus has a manifest the orchestrator can load;
    // copy it into the temp dir but skip building the cached JSONL.
    std::fs::create_dir_all(tmp.path().join("corpus/files")).unwrap();
    std::fs::copy(
        corpus_dir().join("manifest.jsonl"),
        tmp.path().join("corpus/manifest.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        corpus_dir().join("files/quiet_a.rs"),
        tmp.path().join("corpus/files/quiet_a.rs"),
    )
    .unwrap();
    let err = run_eval_with_baselines(
        &tmp.path().join("corpus"),
        &tmp.path().join("corpus/manifest.jsonl"),
        &["sourcerercc".to_string()],
        true,
        &release_tag(),
    )
    .expect_err("expect CachedJsonlMissing");
    let msg = format!("{err}");
    assert!(
        msg.contains("cached JSONL"),
        "expected CachedJsonlMissing error, got: {msg}"
    );
    // Defensive: the underlying error variant is BaselineError.
    match err {
        cntrdct::BaselineRunError::Baseline(BaselineError::CachedJsonlMissing { .. }) => {}
        other => panic!("expected CachedJsonlMissing variant, got {other:?}"),
    }
}

// ---------- Path resolution sanity ----------

#[test]
fn corpus_fixture_layout_is_what_orchestrator_expects() {
    // Pins the spec's cached-path layout against the fixture so a
    // future refactor of cached_jsonl_path is caught here.
    let cached = corpus_dir()
        .parent()
        .unwrap()
        .join("baselines")
        .join(release_tag())
        .join("sourcerercc.jsonl");
    assert!(
        cached.exists(),
        "fixture cached JSONL must exist at {}",
        cached.display()
    );
    assert!(Path::new(&corpus_dir().join("manifest.jsonl")).exists());
    assert!(Path::new(&corpus_dir().join("files/quiet_a.rs")).exists());
}
