//! M-5 acceptance: `[languages.<canonical>]` in `cntrdct.toml` controls
//! discovery and per-language detector suppression, and SARIF output for
//! a mixed Rust+Python repo carries findings from both languages.
//!
//! Three required scenarios:
//! (a) Default config scans both languages and emits findings from each.
//! (b) `[languages.python] enabled = false` skips Python files at the
//!     walker — no Python findings appear at all.
//! (c) `[languages.python] suppress = ["arg-swap"]` keeps the walker on
//!     for Python but drops arg-swap findings whose primary file is .py;
//!     Rust arg-swap findings stay.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

const RUST_ARG_SWAP: &str = "\
fn copy(dst: i32, src: i32) -> i32 { dst + src }
fn driver_rust() {
    let dst = 1;
    let src = 2;
    let _ = copy(src, dst);
}
";

const PYTHON_ARG_SWAP: &str = "\
def copy(dst, src):
    return dst + src

def driver_py():
    dst = 1
    src = 2
    _ = copy(src, dst)
";

fn write_mixed_corpus(dir: &Path) {
    fs::write(dir.join("a.rs"), RUST_ARG_SWAP).unwrap();
    fs::write(dir.join("b.py"), PYTHON_ARG_SWAP).unwrap();
}

fn run_scan(dir: &Path) -> Vec<cntrdct_core::Finding> {
    let cfg = cntrdct_cli::load_config(None, dir).expect("load_config");
    let (raw, files) =
        cntrdct_cli::scan_full_with_config(dir, &cfg).expect("scan_full_with_config");
    cntrdct_config::apply(&cfg, &files, raw).expect("apply")
}

fn has_finding(findings: &[cntrdct_core::Finding], detector: &str, ext: &str) -> bool {
    findings.iter().any(|f| {
        f.detector_id == detector
            && f.primary
                .file
                .extension()
                .and_then(|s| s.to_str())
                .map(|e| e == ext)
                .unwrap_or(false)
    })
}

#[test]
fn default_config_scans_rust_and_python() {
    let dir = TempDir::new().unwrap();
    write_mixed_corpus(dir.path());
    let findings = run_scan(dir.path());
    assert!(
        has_finding(&findings, "arg-swap", "rs"),
        "default config must keep Rust arg-swap findings; got: {:?}",
        findings
    );
    assert!(
        has_finding(&findings, "arg-swap", "py"),
        "default config must keep Python arg-swap findings; got: {:?}",
        findings
    );
}

#[test]
fn languages_python_disabled_skips_python_files_at_walker() {
    let dir = TempDir::new().unwrap();
    write_mixed_corpus(dir.path());
    fs::write(
        dir.path().join("cntrdct.toml"),
        "[languages.python]\nenabled = false\n",
    )
    .unwrap();
    let findings = run_scan(dir.path());
    assert!(
        has_finding(&findings, "arg-swap", "rs"),
        "Rust findings must survive when only Python is disabled"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.primary.file.extension().and_then(|s| s.to_str()) == Some("py")),
        "[languages.python] enabled = false must drop every .py finding; got: {:?}",
        findings
    );
}

#[test]
fn languages_python_suppress_drops_only_python_arg_swap_findings() {
    let dir = TempDir::new().unwrap();
    write_mixed_corpus(dir.path());
    fs::write(
        dir.path().join("cntrdct.toml"),
        "[languages.python]\nsuppress = [\"arg-swap\"]\n",
    )
    .unwrap();
    let findings = run_scan(dir.path());
    assert!(
        has_finding(&findings, "arg-swap", "rs"),
        "Rust arg-swap finding must survive per-language suppression scoped to Python"
    );
    assert!(
        !has_finding(&findings, "arg-swap", "py"),
        "Python arg-swap finding must be dropped by [languages.python] suppress; got: {:?}",
        findings
    );
}

#[test]
fn sarif_emitter_handles_mixed_rust_and_python_unchanged() {
    use cntrdct_core::Detector;
    use cntrdct_detector_arg_swap::ArgSwap;
    use cntrdct_detector_clone_drift::CloneDrift;
    use cntrdct_detector_comment_code::CommentCode;
    use cntrdct_detector_config_interaction::ConfigInteraction;
    use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;

    let dir = TempDir::new().unwrap();
    write_mixed_corpus(dir.path());
    let findings = run_scan(dir.path());
    let ranked = cntrdct_cli::rank_with_calibration(findings, true, None).expect("rank");

    let clone_drift = CloneDrift::new();
    let arg_swap = ArgSwap::new();
    let comment_code = CommentCode::new();
    let unreachable = UnreachableAfterTerminator::new();
    let config_interaction = ConfigInteraction::new();
    let detectors: Vec<&dyn Detector> = vec![
        &clone_drift,
        &arg_swap,
        &comment_code,
        &unreachable,
        &config_interaction,
    ];
    let sarif = cntrdct_sarif::to_sarif_with_rules_ranked(&ranked, &detectors);

    let results = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results array");
    let mut has_rs = false;
    let mut has_py = false;
    for r in results {
        let uri = r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .unwrap_or("");
        if uri.ends_with(".rs") {
            has_rs = true;
        }
        if uri.ends_with(".py") {
            has_py = true;
        }
    }
    assert!(
        has_rs && has_py,
        "SARIF output must surface both Rust and Python findings; rs={} py={}\nresults={:#?}",
        has_rs,
        has_py,
        results
    );
}
