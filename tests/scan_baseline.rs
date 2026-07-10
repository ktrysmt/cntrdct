//! Integration tests for the B-1 baseline (ratchet) surface of
//! `cntrdct scan`: `--baseline`, `--write-baseline`, and `--fail-on`.
//!
//! Spec: `docs/spec/baseline-v0.md`. Distinct from `tests/baselines.rs`,
//! which covers the Q-15 release-fixture pinning (an unrelated feature
//! that happens to share the word).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::tempdir;

/// `fn f() { return; bar(); }` triggers `unreachable-after-terminator`
/// (raw_severity Warning) whose message embeds the terminator's line
/// number — exactly the shape the digit-normalized fingerprint must
/// tolerate across line shifts.
const DEAD_CODE: &str = "fn f() { return; bar(); }\n";

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

fn stdout_findings(out: &Output) -> Vec<serde_json::Value> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "stdout must be a JSON findings array: {}\nstdout: {}",
            e, stdout
        )
    })
}

#[test]
fn write_baseline_then_rescan_reports_nothing() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let baseline = dir.path().join("cntrdct-baseline.json");
    let root = dir.path().to_str().unwrap();
    let baseline_arg = baseline.to_str().unwrap();

    let out = scan(&[root, "--write-baseline", baseline_arg]);
    assert!(out.status.success(), "write-baseline run must exit 0");
    assert!(
        !stdout_findings(&out).is_empty(),
        "the recording run still reports its findings"
    );
    let body = fs::read_to_string(&baseline).expect("baseline file written");
    let doc: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(doc["version"], 1);
    assert_eq!(
        doc["entries"][0]["detector_id"], "unreachable-after-terminator",
        "baseline entries must be reviewable: {}",
        body
    );
    assert_eq!(
        doc["entries"][0]["file"], "dead.rs",
        "path must be scan-root-relative: {}",
        body
    );

    let out = scan(&[root, "--baseline", baseline_arg]);
    assert!(out.status.success());
    assert!(
        stdout_findings(&out).is_empty(),
        "all findings are known; output must be empty"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("baseline: 1 known finding(s) suppressed; 0 new"),
        "suppression must be visible on stderr, got: {}",
        stderr
    );
}

#[test]
fn line_shift_does_not_resurrect_baselined_findings() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("dead.rs");
    fs::write(&src, DEAD_CODE).unwrap();
    let baseline = dir.path().join("b.json");
    let root = dir.path().to_str().unwrap();
    let baseline_arg = baseline.to_str().unwrap();

    let out = scan(&[root, "--write-baseline", baseline_arg]);
    assert!(out.status.success());

    // Shift the finding down by three lines without changing it. The
    // detector message embeds the terminator's (now different) line
    // number, so this fails unless fingerprints digit-normalize.
    fs::write(&src, format!("// moved\n// down\n\n{}", DEAD_CODE)).unwrap();
    let out = scan(&[root, "--baseline", baseline_arg]);
    assert!(out.status.success());
    assert!(
        stdout_findings(&out).is_empty(),
        "an unchanged finding must stay suppressed after a line shift; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn new_finding_surfaces_through_the_baseline() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let baseline = dir.path().join("b.json");
    let root = dir.path().to_str().unwrap();
    let baseline_arg = baseline.to_str().unwrap();

    let out = scan(&[root, "--write-baseline", baseline_arg]);
    assert!(out.status.success());

    // Introduce a NEW contradiction in a different file.
    fs::write(dir.path().join("fresh.rs"), "fn g() { return; qux(); }\n").unwrap();
    let out = scan(&[root, "--baseline", baseline_arg]);
    assert!(out.status.success());
    let findings = stdout_findings(&out);
    assert_eq!(
        findings.len(),
        1,
        "only the new finding may surface: {:#?}",
        findings
    );
    assert!(
        findings[0]["finding"]["primary"]["file"]
            .as_str()
            .unwrap()
            .ends_with("fresh.rs"),
        "the surfaced finding must be the new one: {:#?}",
        findings
    );
}

#[test]
fn baseline_filters_sarif_output_too() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let baseline = dir.path().join("b.json");
    let root = dir.path().to_str().unwrap();
    let baseline_arg = baseline.to_str().unwrap();

    let out = scan(&[root, "--write-baseline", baseline_arg]);
    assert!(out.status.success());

    let out = scan(&[root, "--baseline", baseline_arg, "--format", "sarif"]);
    assert!(out.status.success());
    let sarif: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be SARIF JSON");
    let results = sarif["runs"][0]["results"]
        .as_array()
        .expect("runs[0].results array");
    assert!(
        results.is_empty(),
        "baselined findings must not reach SARIF: {:#?}",
        results
    );
}

#[test]
fn missing_baseline_file_is_a_hard_error() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let root = dir.path().to_str().unwrap();

    let out = scan(&[root, "--baseline", "/nonexistent/cntrdct-baseline.json"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a typo'd baseline path must fail loudly, not report everything as new"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error:"), "got: {}", stderr);
}

#[test]
fn baseline_and_write_baseline_conflict() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    let out = scan(&[root, "--baseline", "a.json", "--write-baseline", "b.json"]);
    assert_eq!(out.status.code(), Some(2), "clap usage error expected");
}

#[test]
fn fail_on_warning_exits_3_and_baseline_makes_it_pass() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("dead.rs"), DEAD_CODE).unwrap();
    let baseline = dir.path().join("b.json");
    let root = dir.path().to_str().unwrap();
    let baseline_arg = baseline.to_str().unwrap();

    // Warning-level finding + --fail-on warning → exit 3.
    let out = scan(&[root, "--fail-on", "warning"]);
    assert_eq!(out.status.code(), Some(3), "findings must fail the run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("fail-on:"), "got: {}", stderr);

    // --fail-on error ignores Warning-level findings.
    let out = scan(&[root, "--fail-on", "error"]);
    assert_eq!(out.status.code(), Some(0));

    // Default (never) preserves the pre-B-1 behaviour.
    let out = scan(&[root]);
    assert_eq!(out.status.code(), Some(0));

    // The write-baseline run accepts its findings: exit 0 even with
    // --fail-on warning, with a note.
    let out = scan(&[
        root,
        "--write-baseline",
        baseline_arg,
        "--fail-on",
        "warning",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--fail-on is not enforced"),
        "got: {}",
        stderr
    );

    // Ratchet: with everything baselined, --fail-on warning passes.
    let out = scan(&[root, "--baseline", baseline_arg, "--fail-on", "warning"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "baselined findings must not fail the run; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
