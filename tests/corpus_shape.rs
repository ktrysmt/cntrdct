//! Corpus-shape contract for `benchmarks/corpus/`.
//!
//! The β corpus contract is: at least 50 labelled source files with at
//! least 8 positive cases per registered detector, and with
//! negative-only files (entries whose `expected` array is empty)
//! capped at 30 percent of the total. This test enforces those
//! numeric commitments.
//!
//! The test parses the manifest directly (no JSON dependency on
//! `cntrdct-eval`) so it remains independent of the eval harness's
//! implementation.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cntrdct::core::Detector;
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::lang::python_unreachable_except::PythonUnreachableExcept;
use cntrdct::detectors::lang::rust_config_interaction::ConfigInteraction;
use cntrdct::detectors::pr_miner::PrMinerDetector;
use cntrdct::detectors::unreachable_after_terminator::UnreachableAfterTerminator;
use serde_json::Value;

const MIN_CORPUS_SIZE: usize = 50;
const MIN_POSITIVES_PER_DETECTOR: usize = 8;
const MAX_NEGATIVE_FRACTION: f64 = 0.30;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn registered_detector_ids() -> Vec<String> {
    let detectors: Vec<Box<dyn Detector>> = vec![
        Box::new(CloneDrift::new()),
        Box::new(ArgSwap::new()),
        Box::new(CommentCode::new()),
        Box::new(UnreachableAfterTerminator::new()),
        Box::new(ConfigInteraction::new()),
        Box::new(PrMinerDetector::new()),
        Box::new(PythonUnreachableExcept::new()),
    ];
    detectors.iter().map(|d| d.id().to_string()).collect()
}

struct ManifestStats {
    entries: usize,
    negatives: usize,
    positives_per_detector: BTreeMap<String, usize>,
    /// (detector_id, language) -> count. Language is the lowercase file
    /// extension stem (`rs`, `py`). Used by spec pr-miner-v0.md's
    /// per-language ≥ 8 positives requirement.
    positives_per_detector_language: BTreeMap<(String, String), usize>,
}

fn file_language_token(rel: &str) -> Option<&'static str> {
    if rel.ends_with(".rs") {
        Some("rs")
    } else if rel.ends_with(".py") {
        Some("py")
    } else if rel.ends_with(".ts") {
        Some("ts")
    } else {
        None
    }
}

fn load_manifest_stats() -> ManifestStats {
    let manifest_path = workspace_root().join("benchmarks/corpus/manifest.jsonl");
    let text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {}", manifest_path.display(), e));

    let mut stats = ManifestStats {
        entries: 0,
        negatives: 0,
        positives_per_detector: BTreeMap::new(),
        positives_per_detector_language: BTreeMap::new(),
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("parse manifest line {:?}: {}", line, e));
        stats.entries += 1;
        let file_path = value
            .get("file")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("entry missing `file`: {}", line));
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
            if let Some(lang) = file_language_token(file_path) {
                *stats
                    .positives_per_detector_language
                    .entry((id.to_string(), lang.to_string()))
                    .or_insert(0) += 1;
            }
        }
    }

    stats
}

#[test]
fn corpus_meets_minimum_size() {
    let stats = load_manifest_stats();
    assert!(
        stats.entries >= MIN_CORPUS_SIZE,
        "corpus has {} entries, contract requires at least {}",
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
                Some(format!(
                    "{}: {} (need {})",
                    id, count, MIN_POSITIVES_PER_DETECTOR
                ))
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
fn pr_miner_corpus_meets_per_language_positives() {
    // Spec: docs/spec/pr-miner-v0.md "Corpus contribution" — at least 8
    // positives per supported language for pr-miner, on top of the
    // global ≥ 8 enforced by `corpus_has_minimum_positives_per_registered_detector`.
    let stats = load_manifest_stats();
    let pr_miner = PrMinerDetector::new();
    let id = pr_miner.id().to_string();
    let supported = pr_miner.supported_languages();
    let mut shortfalls: Vec<String> = Vec::new();
    for language in supported {
        let lang_token = match language.canonical_name() {
            "rust" => "rs",
            "python" => "py",
            "typescript" => "ts",
            other => panic!(
                "pr-miner declares unsupported language token `{}` — extend file_language_token()",
                other
            ),
        };
        let count = stats
            .positives_per_detector_language
            .get(&(id.clone(), lang_token.to_string()))
            .copied()
            .unwrap_or(0);
        if count < MIN_POSITIVES_PER_DETECTOR {
            shortfalls.push(format!(
                "{}/{}: {} (need {})",
                id, lang_token, count, MIN_POSITIVES_PER_DETECTOR
            ));
        }
    }
    assert!(
        shortfalls.is_empty(),
        "pr-miner is short on per-language positives: {:?}",
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
        "negative-only entries account for {:.2} of corpus, cap is {:.2}",
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
