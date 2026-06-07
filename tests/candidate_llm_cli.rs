//! R-4 (P3 amendment) end-to-end CLI integration tests for the Layer 0
//! candidate generator (`scan --candidate-llm`).
//!
//! These exercise the wiring in `src/main.rs` that the library-level unit
//! tests cannot reach: the clap flag contract (B5 `requires`), graceful
//! degradation when the provider CLI is absent (R9), the cost cap (B6),
//! and unadjudicated-candidate suppression (B5 / §3.3). A stub `claude`
//! script (selected via `CLAUDE_CLI_PROGRAM_OVERRIDE`) stands in for the
//! real CLI so no network or real LLM is involved.
//!
//! Spec: `docs/spec/p3-amendment-v0.md` §9.

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_cntrdct")
}

/// The flagship Bound B case: a 2-arg call with no lexical correlation,
/// nested in a list comprehension. Layer 1 emits nothing; Layer 0 (when
/// enabled) produces exactly one candidate.
fn write_flagship(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("totalsegmentator_statistics.py");
    std::fs::write(
        &path,
        "def get_radiomics_features(seg_file, img_file):\n    return 0\n\n\
         def run(ct_file, mask, masks):\n    return [get_radiomics_features(ct_file, mask) for _ in masks]\n",
    )
    .expect("write flagship fixture");
    path
}

/// Write an executable stub that mimics `claude`: answers the `--version`
/// availability probe, and on any dispatch prints the
/// `claude --output-format json` envelope whose `result` carries the
/// requested verdict JSON.
#[cfg(unix)]
fn write_stub_claude(dir: &TempDir, verdict: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.path().join("stub-claude.sh");
    let template = r#"#!/usr/bin/env bash
for a in "$@"; do
  if [ "$a" = "--version" ]; then echo "stub-claude 1.0"; exit 0; fi
done
printf '%s\n' '{"result": "{\"verdict\": \"__VERDICT__\", \"confidence\": 0.9, \"rationale\": \"stub\"}"}'
"#;
    std::fs::write(&path, template.replace("__VERDICT__", verdict)).expect("write stub");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod stub");
    path
}

#[cfg(unix)]
fn scan_with_stub(file: &Path, stub: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(bin());
    cmd.arg("scan")
        .arg(file)
        .arg("--candidate-llm")
        .arg("--adjudicate")
        .arg("--no-calibration")
        .args(extra)
        .env("CLAUDE_CLI_PROGRAM_OVERRIDE", stub)
        // No Layer 3 backend, so Layer 0 candidates stay unadjudicated.
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_API_URL_OVERRIDE");
    cmd.output().expect("run cntrdct")
}

/// B5: `--candidate-llm` without `--adjudicate` is a clap usage error
/// (exit 2). The candidate generator must never run unadjudicated.
#[test]
fn candidate_llm_requires_adjudicate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = write_flagship(&dir);
    let out = Command::new(bin())
        .arg("scan")
        .arg(&file)
        .arg("--candidate-llm")
        .output()
        .expect("run cntrdct");
    assert_eq!(
        out.status.code(),
        Some(2),
        "clap should reject --candidate-llm without --adjudicate"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--adjudicate"),
        "error should name the required flag; stderr: {stderr}"
    );
}

/// R9: when the provider CLI is unavailable, the scan degrades to
/// Layer-1-only, logs a note, and exits 0 — a missing optional provider
/// must not fail a scan the user opted into.
#[test]
fn provider_unavailable_degrades_gracefully() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = write_flagship(&dir);
    let out = Command::new(bin())
        .arg("scan")
        .arg(&file)
        .arg("--candidate-llm")
        .arg("--adjudicate")
        .arg("--no-calibration")
        .env("CLAUDE_CLI_PROGRAM_OVERRIDE", "/nonexistent/claude-xyz")
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .expect("run cntrdct");
    assert!(
        out.status.success(),
        "unavailable provider must not fail the scan (exit {:?})",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unavailable"),
        "should note the unavailable provider; stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim_start().starts_with('['),
        "stdout should be a (Layer-1-only) JSON array; got: {stdout}"
    );
    assert!(!stdout.contains("Layer0Llm"));
}

/// B5 / §3.3: a Layer 0 candidate that is generated but never
/// adjudicated (here: no `ANTHROPIC_API_KEY`, so Layer 3 is skipped) is
/// suppressed from the output — an unadjudicated LLM proposal has no
/// precision floor.
#[cfg(unix)]
#[test]
fn candidate_generated_then_suppressed_without_adjudication() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = write_flagship(&dir);
    let stub = write_stub_claude(&dir, "LikelyTruePositive");
    let out = scan_with_stub(&file, &stub, &[]);
    assert!(out.status.success(), "exit {:?}", out.status.code());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("1 candidate(s)"),
        "Layer 0 should report one candidate; stderr: {stderr}"
    );
    assert!(
        stderr.contains("suppressed 1"),
        "the unadjudicated candidate should be suppressed; stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Layer0Llm"),
        "suppressed candidate must not appear in output; stdout: {stdout}"
    );
}

/// B6 / R7: `--candidate-llm-max-calls 0` dispatches nothing and records
/// the residue as skipped (no silent caps) — the cap bounds fan-out.
#[cfg(unix)]
#[test]
fn cost_cap_zero_dispatches_nothing_and_records_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = write_flagship(&dir);
    let stub = write_stub_claude(&dir, "LikelyTruePositive");
    let out = scan_with_stub(&file, &stub, &["--candidate-llm-max-calls", "0"]);
    assert!(out.status.success(), "exit {:?}", out.status.code());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("0 dispatched"),
        "max-calls 0 should dispatch nothing; stderr: {stderr}"
    );
    assert!(
        stderr.contains("1 skipped over cap"),
        "the un-dispatched residue must be logged, not silently dropped; stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("Layer0Llm"));
}
