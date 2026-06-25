//! Integration guard for the self-replication ledger.
//!
//! Asserts the committed `benchmarks/self-replication/v<release>/cntrdct.jsonl`
//! snapshot parses, covers the tracked corpora, and that `assemble_report`
//! reports a clean self-comparison (zero deltas, `has_baseline = true`) when a
//! snapshot is compared against itself.

use std::collections::BTreeSet;
use std::path::PathBuf;

use cntrdct::self_replication::{assemble_report, load_eval_snapshot};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The ledger version directory tracking the current release line. Bumped per
/// release alongside the snapshot refresh.
const LEDGER_VERSION: &str = "v0.8.0";

fn snapshot_path() -> PathBuf {
    workspace_root()
        .join("benchmarks")
        .join("self-replication")
        .join(LEDGER_VERSION)
        .join("cntrdct.jsonl")
}

#[test]
fn committed_snapshot_parses_and_covers_tracked_corpora() {
    let reports = load_eval_snapshot(&snapshot_path()).expect("committed snapshot parses as JSONL");
    let corpora: BTreeSet<&str> = reports.iter().map(|r| r.corpus.as_str()).collect();
    for expected in ["audit-corpus", "wild-corpus", "wild-corpus-python"] {
        assert!(
            corpora.contains(expected),
            "ledger is missing a line for corpus `{}`; found {:?}",
            expected,
            corpora
        );
    }
    // Every snapshot line must self-identify; an empty corpus name means an
    // older binary wrote the line (the `corpus` field predates v0.8.0).
    for r in &reports {
        assert!(!r.corpus.is_empty(), "snapshot line has empty corpus name");
    }
}

#[test]
fn self_comparison_has_baseline_and_zero_delta() {
    let reports = load_eval_snapshot(&snapshot_path()).expect("committed snapshot parses");
    let audit = reports
        .iter()
        .find(|r| r.corpus == "audit-corpus")
        .expect("audit-corpus line present");

    let delta = assemble_report(audit, &reports);
    assert!(delta.has_baseline, "self-comparison must find a baseline");

    let d = delta.overall.delta.expect("overall delta present");
    assert!(d.precision.abs() < 1e-9, "precision delta {}", d.precision);
    assert!(d.recall.abs() < 1e-9, "recall delta {}", d.recall);
    assert!(d.f1.abs() < 1e-9, "f1 delta {}", d.f1);
}

#[test]
fn missing_corpus_line_reports_baseline() {
    let reports = load_eval_snapshot(&snapshot_path()).expect("committed snapshot parses");
    // A corpus not present in the snapshot yields a baseline (no prior).
    let unseen = cntrdct::eval::EvalReport {
        corpus: "corpus-not-in-ledger".to_string(),
        ..Default::default()
    };
    let delta = assemble_report(&unseen, &reports);
    assert!(!delta.has_baseline);
    assert!(delta.overall.previous.is_none());
    assert!(delta.overall.delta.is_none());
}
