//! Q-12 acceptance test: post-hoc Platt scaling lowers ECE relative
//! to the raw LLM-emitted confidence on a held-out fixture.
//!
//! Spec: `docs/spec/llm-calibration-v0.md` F10. Methodology:
//! `platt-1999` (post-hoc sigmoid fit) and `spiess-icse-2025`
//! (post-hoc calibration of code-LLM outputs).
//!
//! The fixture corpus is constructed-pathology: the LLM is heavily
//! over-confident in the way Spiess et al. (2025) §6 documents — raw
//! confidence sits at 0.95 / 0.85 / 0.75 / 0.65 / 0.55 with empirical
//! base rates of 0.50 / 0.50 / 0.55 / 0.50 / 0.50 across the bands.
//! Platt scaling is the canonical fix for this shape, so the test
//! fails (deliberately) if the fit ever stops correcting it.
//!
//! Mechanics:
//! - `train.jsonl` (210 rows) — fed to `cntrdct calibrate --fit-platt`.
//! - `holdout.jsonl` (90 rows, 30/70 split) — never seen by the
//!   fitter; 10-bin ECE is computed on it for raw vs calibrated.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use cntrdct::calibration::Verdict;
use cntrdct::core::AnomalyClass;
use cntrdct::llm_calibration::{
    apply_platt, ece, fit_registry, load_corpus, LabelledLlmConfidence, PlattRegistry,
};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("llm-calibration")
}

fn cntrdct_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cntrdct"))
}

fn load_holdout_pairs(path: &std::path::Path) -> Vec<(f64, bool)> {
    load_corpus(path)
        .unwrap_or_else(|e| panic!("load holdout {}: {}", path.display(), e))
        .into_iter()
        .map(|row| {
            (
                row.raw_confidence,
                matches!(row.verdict, Verdict::TruePositive),
            )
        })
        .collect()
}

#[test]
fn platt_calibration_lowers_holdout_ece_versus_raw() {
    let dir = fixture_dir();
    let train = dir.join("train.jsonl");
    let holdout = dir.join("holdout.jsonl");

    let train_corpus = load_corpus(&train).expect("load train corpus");
    assert!(
        !train_corpus.is_empty(),
        "fixture train.jsonl must not be empty"
    );
    let holdout_pairs = load_holdout_pairs(&holdout);
    assert!(
        !holdout_pairs.is_empty(),
        "fixture holdout.jsonl must not be empty"
    );

    // Sanity: fixture is the documented over-confidence shape — raw
    // mean confidence > raw accuracy. Without this property the test
    // tells us nothing about Platt.
    let mean_conf: f64 =
        holdout_pairs.iter().map(|(c, _)| *c).sum::<f64>() / holdout_pairs.len() as f64;
    let acc: f64 =
        holdout_pairs.iter().filter(|(_, y)| *y).count() as f64 / holdout_pairs.len() as f64;
    assert!(
        mean_conf - acc > 0.1,
        "fixture must be over-confidence-shaped (mean_conf={mean_conf}, acc={acc})"
    );

    let registry = fit_registry(&train_corpus).expect("fit registry");
    let cell = registry
        .get("clone-drift", AnomalyClass::Logic)
        .expect("registry must carry clone-drift:Logic from the train corpus");

    let raw_ece = ece(&holdout_pairs, 10);
    let calibrated_pairs: Vec<(f64, bool)> = holdout_pairs
        .iter()
        .map(|(c, y)| (apply_platt(cell, *c), *y))
        .collect();
    let calibrated_ece = ece(&calibrated_pairs, 10);

    assert!(
        calibrated_ece < raw_ece - 0.05,
        "Platt calibration must drop holdout ECE by a non-trivial \
         margin (>= 0.05). raw_ece={raw_ece}, calibrated_ece={calibrated_ece}"
    );
}

#[test]
fn fit_platt_cli_writes_registry_for_real_corpus() {
    // End-to-end smoke test of `cntrdct calibrate --fit-platt`: it
    // must read a real fixture corpus and produce a parseable
    // registry on disk. The registry's content is exercised
    // structurally above; this test pins the CLI surface.
    let train = fixture_dir().join("train.jsonl");
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("nested").join("platt.json");

    let status = Command::new(cntrdct_bin())
        .arg("calibrate")
        .arg(&train)
        .arg("--fit-platt")
        .arg("--output")
        .arg(&out)
        .status()
        .expect("spawn cntrdct");
    assert!(status.success(), "calibrate --fit-platt must succeed");
    assert!(out.exists(), "output file must be created");

    let body = fs::read_to_string(&out).expect("read registry");
    let registry = PlattRegistry::from_json(&body).expect("registry parses");
    assert!(
        registry.get("clone-drift", AnomalyClass::Logic).is_some(),
        "registry must contain clone-drift:Logic from the fixture"
    );
}

#[test]
fn fit_platt_cli_rejects_empty_corpus() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let empty = tmp.path().join("empty.jsonl");
    fs::write(&empty, b"").unwrap();
    let out = tmp.path().join("platt.json");

    let result = Command::new(cntrdct_bin())
        .arg("calibrate")
        .arg(&empty)
        .arg("--fit-platt")
        .arg("--output")
        .arg(&out)
        .output()
        .expect("spawn cntrdct");
    assert!(
        !result.status.success(),
        "empty corpus must fail loudly, not silently produce empty output"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("empty corpus"),
        "stderr should explain the failure: {}",
        stderr
    );
}

#[test]
fn registry_json_round_trip_preserves_predictions_on_real_corpus() {
    // Defend against future refactors that diverge the JSON
    // serialisation of `PlattRegistry` from the in-memory shape
    // `apply_llm_calibration` consumes. After round-tripping through
    // `to_json_pretty` / `from_json`, a fitted cell must produce
    // bitwise-identical calibrated values for every raw input on the
    // training corpus.
    let train = fixture_dir().join("train.jsonl");
    let corpus = load_corpus(&train).expect("load");
    let registry = fit_registry(&corpus).expect("fit");
    let body = registry.to_json_pretty();
    let restored = PlattRegistry::from_json(&body).expect("round-trip");

    let cell = registry
        .get("clone-drift", AnomalyClass::Logic)
        .expect("clone-drift:Logic must exist in registry");
    let restored_cell = restored
        .get("clone-drift", AnomalyClass::Logic)
        .expect("clone-drift:Logic must exist in restored registry");

    for row in &corpus {
        let before = apply_platt(cell, row.raw_confidence);
        let after = apply_platt(restored_cell, row.raw_confidence);
        assert_eq!(
            before.to_bits(),
            after.to_bits(),
            "round-trip must preserve calibrated value bit-for-bit; \
             input={:?} got before={} after={}",
            row,
            before,
            after,
        );
    }
}

#[test]
fn fit_is_deterministic_on_real_corpus() {
    // The Q-12 spec promises Platt fit is deterministic — same
    // training data must produce byte-identical PlattParams. Pin
    // it on the real fixture, not just synthetic small-N data.
    let train = fixture_dir().join("train.jsonl");
    let corpus = load_corpus(&train).expect("load");
    let a = fit_registry(&corpus).expect("fit a");
    let b = fit_registry(&corpus).expect("fit b");
    assert_eq!(
        a.to_json_pretty(),
        b.to_json_pretty(),
        "Platt fit must be deterministic"
    );
    // Reference unused symbol so the compiler does not drop the
    // import when only some assertions compile.
    let _: Option<&LabelledLlmConfidence> = corpus.first();
}
