//! ir-v0.md §F6 T1 — per-detector pinning tests.
//!
//! For each cross-cutting detector (`arg-swap`, `clone-drift`,
//! `comment-code`, `pr-miner`, `unreachable-after-terminator`),
//! re-serialise the findings produced against the audit-corpus +
//! wild-corpus (Rust) + wild-corpus-python fixtures and compare
//! byte-for-byte against the golden snapshot captured from v0.5.2 at
//! `tests/fixtures/ir-pinning/<detector>/{audit,wild-rust,wild-python}.json`
//! (commit b6c6540).
//!
//! Capture invocation:
//!
//! ```sh
//! cntrdct scan --no-calibration --format json <corpus> \
//!   | jq --indent 2 'map(select(.finding.detector_id == "<det>"))'
//! ```
//!
//! The test reproduces that pipeline in Rust: scan the corpus, rank
//! uncalibrated (Layer 1 identity is what we measure here; Layer 2
//! ranker identity is the concern of `tests/ranker_lib.rs`), filter to
//! the detector under test, serialise as a JSON array via
//! `serde_json::to_string_pretty` (which produces 2-space indent
//! matching `jq --indent 2`), and assert byte equality with the
//! fixture.

use std::path::{Path, PathBuf};

use cntrdct::core::{Finding, RankedFinding};
use cntrdct::ranker::UncalibratedRanker;
use cntrdct::scan;

fn corpus_path(kind: &str) -> PathBuf {
    // Use a relative path so the file walker records relative
    // `Finding.primary.file` entries that match the v0.5.2 capture
    // invocation (`cntrdct scan ... benchmarks/audit-corpus`). Cargo
    // runs `cargo test` with CWD = `CARGO_MANIFEST_DIR`, so a relative
    // path resolves against the package root the same way the CLI does.
    match kind {
        "audit" => PathBuf::from("benchmarks/audit-corpus"),
        "wild-rust" => PathBuf::from("benchmarks/wild-corpus"),
        "wild-python" => PathBuf::from("benchmarks/wild-corpus-python"),
        other => panic!("unknown corpus kind: {other}"),
    }
}

fn fixture_path(detector: &str, kind: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("tests/fixtures/ir-pinning")
        .join(detector)
        .join(format!("{kind}.json"))
}

fn rank_uncalibrated(findings: Vec<Finding>) -> Vec<RankedFinding> {
    use cntrdct::core::Ranker;
    let ranker = UncalibratedRanker::new();
    ranker.rank(findings)
}

fn run_and_compare(detector: &str, kind: &str) {
    let corpus = corpus_path(kind);
    let fixture = fixture_path(detector, kind);

    let all_findings = scan(&corpus).expect("scan succeeds");
    let ranked = rank_uncalibrated(all_findings);
    let filtered: Vec<&RankedFinding> = ranked
        .iter()
        .filter(|rf| rf.finding.detector_id == detector)
        .collect();

    let actual = serde_json::to_string_pretty(&filtered).expect("serialize");
    // serde_json's pretty printer does not add a trailing newline; the
    // jq-captured fixtures don't either. If the byte comparison drifts
    // it surfaces in the assert below.

    let expected =
        std::fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("read {fixture:?}: {e}"));
    // Fixture files were captured with `jq --indent 2` and so end in a
    // trailing newline. Strip it for the byte comparison so the
    // serde_json output (which lacks the trailing newline) matches.
    let expected = expected.trim_end_matches('\n');

    if actual != expected {
        let actual_path = std::env::temp_dir().join(format!("ir_pinning_{detector}_{kind}.json"));
        std::fs::write(&actual_path, &actual).ok();
        panic!(
            "T1 pinning drift for {detector} / {kind}\n\
             expected (fixture): {fixture:?}\n\
             actual (written):   {actual_path:?}\n\
             diff with: diff {actual_path:?} {fixture:?}"
        );
    }
}

#[test]
fn t1_arg_swap_audit_byte_identical() {
    run_and_compare("arg-swap", "audit");
}

#[test]
fn t1_arg_swap_wild_rust_byte_identical() {
    run_and_compare("arg-swap", "wild-rust");
}

#[test]
fn t1_arg_swap_wild_python_byte_identical() {
    run_and_compare("arg-swap", "wild-python");
}

#[test]
fn t1_clone_drift_audit_byte_identical() {
    run_and_compare("clone-drift", "audit");
}

#[test]
fn t1_clone_drift_wild_rust_byte_identical() {
    run_and_compare("clone-drift", "wild-rust");
}

#[test]
fn t1_clone_drift_wild_python_byte_identical() {
    run_and_compare("clone-drift", "wild-python");
}

#[test]
fn t1_comment_code_audit_byte_identical() {
    run_and_compare("comment-code", "audit");
}

#[test]
fn t1_comment_code_wild_rust_byte_identical() {
    run_and_compare("comment-code", "wild-rust");
}

#[test]
fn t1_comment_code_wild_python_byte_identical() {
    run_and_compare("comment-code", "wild-python");
}

#[test]
fn t1_pr_miner_audit_byte_identical() {
    run_and_compare("pr-miner", "audit");
}

#[test]
fn t1_pr_miner_wild_rust_byte_identical() {
    run_and_compare("pr-miner", "wild-rust");
}

#[test]
fn t1_pr_miner_wild_python_byte_identical() {
    run_and_compare("pr-miner", "wild-python");
}

#[test]
fn t1_unreachable_after_terminator_audit_byte_identical() {
    run_and_compare("unreachable-after-terminator", "audit");
}

#[test]
fn t1_unreachable_after_terminator_wild_rust_byte_identical() {
    run_and_compare("unreachable-after-terminator", "wild-rust");
}

#[test]
fn t1_unreachable_after_terminator_wild_python_byte_identical() {
    run_and_compare("unreachable-after-terminator", "wild-python");
}
