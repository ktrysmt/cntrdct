//! Tests for the clippy harness pieces that do not require an actual
//! cargo invocation. The end-to-end path is covered by manual runs in an
//! isolated environment — running cargo clippy in unit tests would need
//! both a network and a working rustup toolchain, both of which the
//! workspace's CI does not currently guarantee for arbitrary crates.

use cntrdct_research::{parse_clippy_diagnostics, run_clippy_harness, ClippyHarnessError};

const SAMPLE_STDOUT: &[u8] = br#"{"reason":"compiler-artifact","package_id":"foo 0.1.0","manifest_path":"/tmp/Cargo.toml"}
{"reason":"compiler-message","package_id":"foo 0.1.0","manifest_path":"/tmp/Cargo.toml","target":{"name":"foo"},"message":{"rendered":"warning: needless borrow","children":[],"code":{"code":"clippy::needless_borrow","explanation":null},"level":"warning","message":"needless borrow","spans":[{"file_name":"src/lib.rs","line_start":42,"column_start":1}]}}
{"reason":"compiler-message","package_id":"foo 0.1.0","target":{"name":"foo"},"message":{"rendered":"warning: unused","children":[],"code":{"code":"unused_variables","explanation":null},"level":"warning","message":"unused","spans":[]}}
{"reason":"compiler-message","package_id":"foo 0.1.0","target":{"name":"foo"},"message":{"rendered":"info","children":[],"code":null,"level":"warning","message":"info","spans":[]}}
{"reason":"build-finished","success":true}
"#;

#[test]
fn parse_keeps_only_clippy_compiler_messages() {
    let kept = parse_clippy_diagnostics(SAMPLE_STDOUT);
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0]["message"]["code"]["code"].as_str(),
        Some("clippy::needless_borrow")
    );
}

#[test]
fn parse_tolerates_blank_and_unparseable_lines() {
    let stdout = b"\n\n   not json   \n\n{\"reason\":\"build-finished\"}\n";
    let kept = parse_clippy_diagnostics(stdout);
    assert!(kept.is_empty());
}

#[test]
fn parse_returns_empty_for_empty_input() {
    let kept = parse_clippy_diagnostics(b"");
    assert!(kept.is_empty());
}

#[test]
fn parse_handles_lines_without_trailing_newline() {
    // Real cargo output usually ends with a trailing newline, but the
    // helper must still cope with a missing one — otherwise a stdout
    // capture from a busy crate could drop the final diagnostic.
    let stdout = b"{\"reason\":\"compiler-message\",\"message\":{\"code\":{\"code\":\"clippy::unused_unit\"},\"level\":\"warning\",\"message\":\"\"}}";
    let kept = parse_clippy_diagnostics(stdout);
    assert_eq!(kept.len(), 1);
}

#[test]
fn run_clippy_harness_refuses_without_consent_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = tmp.path().join("manifest.csv");
    std::fs::write(
        &manifest,
        "crate,version,license,downloads,sha256\nfoo,0.1.0,MIT,,abc\n",
    )
    .unwrap();
    let out = tmp.path().join("out");
    let err = run_clippy_harness(&manifest, &out, false).unwrap_err();
    assert!(matches!(err, ClippyHarnessError::ConsentRequired));
    // The output directory must NOT have been created — refusal happens
    // before any side effect.
    assert!(!out.exists());
}
