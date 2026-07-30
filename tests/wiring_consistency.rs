//! Q-4: assert that the sites wiring detectors into cntrdct's
//! pipelines stay in sync.
//!
//! Drift between
//! - `src/lib.rs::scan_full_with_config` (scanner registration) and
//! - `src/main.rs` (SARIF emitter `tool.driver.rules`)
//!
//! is exactly the failure mode that landed `pr-miner` findings without a
//! matching SARIF rule in `v0.2.0-beta.1` (audit finding Q-1). Both
//! sites now read from `cntrdct::ALL_DETECTOR_IDS`; this test enforces
//! that contract end-to-end so a future site-by-site addition cannot
//! regress.

use std::path::PathBuf;
use std::process::Command;

use cntrdct::core::{Detector, Language};
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::lang::go_build_tag_interaction::GoBuildTagInteraction;
use cntrdct::detectors::lang::python_unreachable_except::PythonUnreachableExcept;
use cntrdct::detectors::lang::rust_config_interaction::ConfigInteraction;
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
        PythonUnreachableExcept::new().id().to_string(),
        GoBuildTagInteraction::new().id().to_string(),
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

/// Every registered detector, boxed so trait-level metadata contracts can
/// be asserted uniformly. Mirrors `scanner_registration_ids` above.
fn registered_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(CloneDrift::new()),
        Box::new(ArgSwap::new()),
        Box::new(CommentCode::new()),
        Box::new(UnreachableAfterTerminator::new()),
        Box::new(ConfigInteraction::new()),
        Box::new(PrMinerDetector::new()),
        Box::new(PythonUnreachableExcept::new()),
        Box::new(GoBuildTagInteraction::new()),
    ]
}

/// Run `cntrdct scan` over an empty tree and return the SARIF
/// `tool.driver.rules` array.
///
/// The rules taxonomy is built from the detector list — not the findings —
/// so an empty scan root surfaces every detector exactly once. We
/// deliberately invoke the binary (`src/main.rs`) so callers catch drift at
/// THAT site rather than at the library helper alone.
fn sarif_driver_rules() -> Vec<serde_json::Value> {
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
    sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("SARIF runs[0].tool.driver.rules must be present")
        .clone()
}

#[test]
fn sarif_emitter_rules_match_canonical_set() {
    let rules = sarif_driver_rules();

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

/// `(detector id, display name)` as the pair reaches users through SARIF
/// `tool.driver.rules[].shortDescription.text`
/// (`src/sarif.rs::detector_to_rule`). Sorted by id to match
/// `canonical_ids()`.
///
/// These are pinned as literals rather than read back from
/// `Detector::name()`. Comparing emitted output against `d.name()` would
/// be self-satisfying: both sides move together, so a silent rename would
/// still pass. SARIF `shortDescription` is published output — it is what a
/// SARIF viewer and GitHub code scanning display as the rule title — so
/// renaming a detector should be a deliberate two-site edit.
fn expected_rule_short_descriptions() -> Vec<(String, String)> {
    [
        ("arg-swap", "Argument Swap"),
        ("build-tag-interaction-go", "Go Build Tag Interaction"),
        ("clone-drift", "Clone Drift"),
        ("comment-code", "Comment/Code Mismatch"),
        ("config-interaction", "Config Interaction"),
        ("pr-miner", "Implicit Rule Violation (PR-Miner)"),
        (
            "python-unreachable-except",
            "Python Unreachable Except Handler",
        ),
        (
            "unreachable-after-terminator",
            "Unreachable After Terminator",
        ),
    ]
    .iter()
    .map(|(id, name)| (id.to_string(), name.to_string()))
    .collect()
}

#[test]
fn sarif_rule_short_descriptions_match_detector_display_names() {
    let rules = sarif_driver_rules();

    let mut emitted: Vec<(String, String)> = rules
        .iter()
        .map(|r| {
            let id = r["id"].as_str().expect("rule.id is a string").to_string();
            let text = r["shortDescription"]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("rule `{}` has no shortDescription.text", id))
                .to_string();
            (id, text)
        })
        .collect();
    emitted.sort();

    assert_eq!(
        emitted,
        expected_rule_short_descriptions(),
        "SARIF rule shortDescription.text emitted by `src/main.rs` diverges \
         from the pinned detector display names; a detector's `name()` is \
         published output, so update the detector and this table together"
    );
}

#[test]
fn every_detector_declares_at_least_one_supported_language() {
    // `supported_languages()` is a detector's declared reach, and several
    // contracts are *derived* from it by iterating the returned slice —
    // `tests/corpus_shape.rs::pr_miner_corpus_meets_per_language_positives`
    // being the live example. An empty slice does not fail those checks,
    // it makes them vacuous: the loop body never runs and the obligation
    // silently evaporates. Assert the precondition here, once, so a
    // detector that declares nothing is a hard failure rather than a
    // quietly-skipped set of downstream requirements.
    for d in registered_detectors() {
        let langs: &'static [Language] = d.supported_languages();
        assert!(
            !langs.is_empty(),
            "detector `{}` declares no supported language; it can never be \
             exercised and every per-language contract derived from this \
             slice degrades to a vacuous pass",
            d.id()
        );
    }
}
