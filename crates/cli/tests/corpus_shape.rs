//! Corpus-shape contract for `benchmarks/corpus/`.
//!
//! The OSF preregistration (prereg/2026-05-03-osf-prereg.md) commits to a β
//! corpus of at least 50 labelled source files with at least 8 positive cases
//! per registered detector, and with negative-only files (entries whose
//! `expected` array is empty) capped at 30 percent of the total. This test
//! enforces those numeric commitments and so doubles as a stop-gap against
//! corpus shopping during β data collection.
//!
//! The test parses the manifest directly (no JSON dependency on
//! `cntrdct-eval`) so it remains independent of the eval harness's
//! implementation.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cntrdct_core::Detector;
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;
use cntrdct_detector_config_interaction::ConfigInteraction;
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;
use serde_json::Value;

const MIN_CORPUS_SIZE: usize = 50;
const MIN_POSITIVES_PER_DETECTOR: usize = 8;
const MAX_NEGATIVE_FRACTION: f64 = 0.30;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above crates/cli")
        .to_path_buf()
}

fn registered_detector_ids() -> Vec<String> {
    let detectors: Vec<Box<dyn Detector>> = vec![
        Box::new(CloneDrift::new()),
        Box::new(ArgSwap::new()),
        Box::new(CommentCode::new()),
        Box::new(UnreachableAfterTerminator::new()),
        Box::new(ConfigInteraction::new()),
    ];
    detectors.iter().map(|d| d.id().to_string()).collect()
}

struct ManifestStats {
    entries: usize,
    negatives: usize,
    positives_per_detector: BTreeMap<String, usize>,
}

fn load_manifest_stats() -> ManifestStats {
    let manifest_path = workspace_root().join("benchmarks/corpus/manifest.jsonl");
    let text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {}", manifest_path.display(), e));

    let mut stats = ManifestStats {
        entries: 0,
        negatives: 0,
        positives_per_detector: BTreeMap::new(),
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("parse manifest line {:?}: {}", line, e));
        stats.entries += 1;
        let expected = value
            .get("expected")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("entry missing `expected` array: {}", line));
        if expected.is_empty() {
            stats.negatives += 1;
            continue;
        }
        for finding in expected {
            let id = finding
                .get("detector_id")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("expected.detector_id missing: {}", line));
            *stats
                .positives_per_detector
                .entry(id.to_string())
                .or_insert(0) += 1;
        }
    }

    stats
}

#[test]
fn corpus_meets_minimum_size() {
    let stats = load_manifest_stats();
    assert!(
        stats.entries >= MIN_CORPUS_SIZE,
        "corpus has {} entries, prereg requires at least {}",
        stats.entries,
        MIN_CORPUS_SIZE
    );
}

#[test]
fn corpus_has_minimum_positives_per_registered_detector() {
    let stats = load_manifest_stats();
    let ids = registered_detector_ids();
    let shortfalls: Vec<String> = ids
        .iter()
        .filter_map(|id| {
            let count = stats.positives_per_detector.get(id).copied().unwrap_or(0);
            if count < MIN_POSITIVES_PER_DETECTOR {
                Some(format!("{}: {} (need {})", id, count, MIN_POSITIVES_PER_DETECTOR))
            } else {
                None
            }
        })
        .collect();
    assert!(
        shortfalls.is_empty(),
        "corpus is short on positives for: {:?}",
        shortfalls
    );
}

#[test]
fn corpus_negative_fraction_is_within_cap() {
    let stats = load_manifest_stats();
    if stats.entries == 0 {
        panic!("empty corpus — earlier tests should have caught this");
    }
    let frac = stats.negatives as f64 / stats.entries as f64;
    assert!(
        frac <= MAX_NEGATIVE_FRACTION,
        "negative-only entries account for {:.2} of corpus, prereg cap is {:.2}",
        frac,
        MAX_NEGATIVE_FRACTION
    );
}

#[test]
fn every_manifest_file_exists() {
    let manifest_path = workspace_root().join("benchmarks/corpus/manifest.jsonl");
    let text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {}", manifest_path.display(), e));
    let corpus_root = workspace_root().join("benchmarks/corpus");
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("parse manifest line {:?}: {}", line, e));
        let rel = value
            .get("file")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("entry missing `file`: {}", line));
        let abs = corpus_root.join(rel);
        assert!(
            abs.is_file(),
            "manifest references missing file: {}",
            abs.display()
        );
    }
}
