//! Q-4: assert that the three sites wiring detectors into cntrdct's
//! pipelines stay in sync.
//!
//! Drift between
//! - `src/lib.rs::scan_full_with_config` (scanner registration),
//! - `src/main.rs` (SARIF emitter `tool.driver.rules`), and
//! - `tests/prereg_consistency.rs::registered_detectors` (preregistration
//!   citation cross-check)
//!
//! is exactly the failure mode that landed `pr-miner` findings without a
//! matching SARIF rule in `v0.2.0-beta.1` (audit finding Q-1). All three
//! sites now read from `cntrdct::ALL_DETECTOR_IDS`; this test enforces
//! that contract end-to-end so a future site-by-site addition cannot
//! regress.

use std::path::PathBuf;
use std::process::Command;

use cntrdct::core::Detector;
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::config_interaction::ConfigInteraction;
use cntrdct::detectors::pr_miner::PrMinerDetector;
use cntrdct::detectors::unreachable_after_terminator::UnreachableAfterTerminator;

use tempfile::TempDir;

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

fn canonical_ids() -> Vec<String> {
    let mut ids: Vec<String> = cntrdct::ALL_DETECTOR_IDS
        .iter()
        .map(|s| s.to_string())
        .collect();
    ids.sort();
    ids
}

/// IDs returned by detector constructions inside
/// `src/lib.rs::scan_full_with_config`. Mirrors that block one-for-one
/// so a removal-without-update is caught here rather than at runtime.
fn scanner_registration_ids() -> Vec<String> {
    let mut ids: Vec<String> = vec![
        CloneDrift::new().id().to_string(),
        ArgSwap::new().id().to_string(),
        CommentCode::new().id().to_string(),
        UnreachableAfterTerminator::new().id().to_string(),
        ConfigInteraction::new().id().to_string(),
        PrMinerDetector::new().id().to_string(),
    ];
    ids.sort();
    ids
}

#[test]
fn scanner_registration_matches_canonical_set() {
    assert_eq!(
        scanner_registration_ids(),
        canonical_ids(),
        "scanner registration site (src/lib.rs::scan_full_with_config) and \
         cntrdct::ALL_DETECTOR_IDS disagree; update both together"
    );
}

#[test]
fn sarif_emitter_rules_match_canonical_set() {
    // SARIF `tool.driver.rules` is built from the detector list — not the
    // findings — so an empty scan root is sufficient to surface every
    // detector exactly once. We deliberately invoke the binary
    // (`src/main.rs`) so this test catches drift at THAT site rather than
    // the library helper alone.
    let scan_root = TempDir::new().expect("tempdir");
    let output = Command::new(cntrdct_bin())
        .arg("scan")
        .arg(scan_root.path())
        .arg("--format")
        .arg("sarif")
        .arg("--no-calibration")
        .output()
        .expect("invoke cntrdct scan");
    assert!(
        output.status.success(),
        "cntrdct scan exited non-zero: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sarif: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as SARIF JSON: {}\n{}", e, stdout));
    let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("SARIF runs[0].tool.driver.rules must be present");

    let mut emitted_ids: Vec<String> = rules
        .iter()
        .map(|r| r["id"].as_str().expect("rule.id is a string").to_string())
        .collect();
    emitted_ids.sort();

    assert_eq!(
        emitted_ids,
        canonical_ids(),
        "SARIF rules taxonomy emitted by `src/main.rs` diverges from \
         cntrdct::ALL_DETECTOR_IDS; update both together"
    );
}
