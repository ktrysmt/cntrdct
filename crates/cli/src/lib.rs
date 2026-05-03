//! cntrdct CLI library entry point.
//!
//! Specs:
//! - `cntrdct/docs/spec/cli-v0.md` — base scan command.
//! - `cntrdct/docs/spec/ranker-v1.md` — calibrate subcommand and ranker
//!   selection (calibrated when priors are available, uncalibrated otherwise).

use std::fs;
use std::path::{Path, PathBuf};

use cntrdct_adjudicator_llm::{AnthropicAdjudicator, ReqwestClient};
use cntrdct_calibration::{
    compute_priors, load_corpus, CalibrationError, DetectorPrior,
};
use cntrdct_core::{
    register_detector, Adjudicator, CorpusStats, DetectContext, Detector, DetectorConfig,
    Finding, ParsedFile, RankedFinding, Ranker,
};
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;
use cntrdct_detector_config_interaction::ConfigInteraction;
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;
use cntrdct_eval::{evaluate, load_manifest, EvalError, EvalReport};
use cntrdct_ranker::{CalibratedRanker, UncalibratedRanker};
use std::collections::HashMap;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("path not found: {0}")]
    PathNotFound(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("detector error: {0}")]
    Detector(#[from] cntrdct_core::DetectorError),
}

#[derive(Debug, Error)]
pub enum EvalRunError {
    #[error("eval error: {0}")]
    Eval(#[from] EvalError),
    #[error("scan error: {0}")]
    Scan(#[from] ScanError),
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum CalibrateError {
    #[error("calibration error: {0}")]
    Calibration(#[from] CalibrationError),
    #[error("io error writing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn scan(path: &Path) -> Result<Vec<Finding>, ScanError> {
    if !path.exists() {
        return Err(ScanError::PathNotFound(path.to_path_buf()));
    }

    let rust_paths = collect_rust_files(path);

    let mut parsed: Vec<ParsedFile> = Vec::with_capacity(rust_paths.len());
    for p in &rust_paths {
        let source = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        parsed.push(ParsedFile {
            path: p.clone(),
            language: "rust".to_string(),
            source,
        });
    }

    let clone_drift = CloneDrift::new();
    let arg_swap = ArgSwap::new();
    let comment_code = CommentCode::new();
    let unreachable = UnreachableAfterTerminator::new();
    let config_interaction = ConfigInteraction::new();
    register_detector(&clone_drift)?;
    register_detector(&arg_swap)?;
    register_detector(&comment_code)?;
    register_detector(&unreachable)?;
    register_detector(&config_interaction)?;

    let stats = CorpusStats {
        file_count: parsed.len(),
        total_loc: parsed.iter().map(|p| p.source.lines().count()).sum(),
    };
    let config = DetectorConfig::default();
    let ctx = DetectContext {
        files: &parsed,
        stats: &stats,
        config: &config,
    };

    let mut findings = clone_drift.detect(&ctx)?;
    findings.extend(arg_swap.detect(&ctx)?);
    findings.extend(comment_code.detect(&ctx)?);
    findings.extend(unreachable.detect(&ctx)?);
    findings.extend(config_interaction.detect(&ctx)?);
    Ok(findings)
}

// ---------- Calibration discovery ----------

/// Default location of the cached priors file:
/// `dirs::cache_dir().join("cntrdct").join("priors.json")`.
///
/// Returns `None` only when the platform exposes no cache dir (extremely rare;
/// CI containers and most Unix shells provide one).
pub fn default_priors_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("cntrdct").join("priors.json"))
}

/// Try to load priors from `path`. Returns `Ok(Some(map))` when the file
/// exists AND parses cleanly; `Ok(None)` when the file does not exist (silent
/// fallback); `Err` only on I/O or JSON failure for a file that does exist.
pub fn try_load_priors(
    path: &Path,
) -> Result<Option<HashMap<String, DetectorPrior>>, CalibrateError> {
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read_to_string(path).map_err(|e| CalibrateError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let parsed: HashMap<String, DetectorPrior> = serde_json::from_str(&body)?;
    Ok(Some(parsed))
}

/// Build the appropriate ranker according to caller intent.
///
/// - `force_uncalibrated == true` → always returns `UncalibratedRanker`.
/// - else if `priors_override` is `Some` and exists → load from there.
/// - else if `default_priors_path()` exists → load from there.
/// - else → `UncalibratedRanker` (silent fallback).
pub fn pick_ranker(
    force_uncalibrated: bool,
    priors_override: Option<&Path>,
) -> Result<Box<dyn Ranker>, CalibrateError> {
    if force_uncalibrated {
        return Ok(Box::new(UncalibratedRanker::new()));
    }

    let chosen: Option<HashMap<String, DetectorPrior>> = if let Some(p) = priors_override {
        try_load_priors(p)?
    } else if let Some(p) = default_priors_path() {
        try_load_priors(&p)?
    } else {
        None
    };

    Ok(match chosen {
        Some(priors) => Box::new(CalibratedRanker::new(priors)),
        None => Box::new(UncalibratedRanker::new()),
    })
}

/// One-shot helper: rank `findings` according to calibration discovery rules.
pub fn rank_with_calibration(
    findings: Vec<Finding>,
    force_uncalibrated: bool,
    priors_override: Option<&Path>,
) -> Result<Vec<RankedFinding>, CalibrateError> {
    let ranker = pick_ranker(force_uncalibrated, priors_override)?;
    Ok(ranker.rank(findings))
}

// ---------- Adjudication orchestration (Layer 3) ----------

/// Read `ANTHROPIC_API_KEY` from the environment. Returns `None` for both
/// "unset" and "set to empty string" so a stray `export ANTHROPIC_API_KEY=` in
/// a shell profile does not look like an enabled key.
pub fn read_anthropic_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Mutate `ranked` in place: adjudicate the top-N findings (by current order)
/// with the supplied adjudicator. Findings beyond `top_n` keep
/// `adjudication = None`.
///
/// Per design constraint P3, this is the SOLE entry point in the CLI that
/// invokes an adjudicator (and therefore the LLM). All `Detector` and `Ranker`
/// code remains network-free.
pub fn adjudicate_top_n<A: Adjudicator>(
    ranked: &mut [RankedFinding],
    adjudicator: &A,
    top_n: usize,
) -> Result<(), cntrdct_core::DetectorError> {
    for rf in ranked.iter_mut().take(top_n) {
        let result = adjudicator.adjudicate(rf)?;
        rf.adjudication = Some(result);
    }
    Ok(())
}

/// Build the production Anthropic adjudicator with the given API key.
///
/// Wires `ReqwestClient` (rustls-backed `reqwest::blocking`) into
/// `AnthropicAdjudicator` with default model / temperature / token cap. The
/// returned adjudicator hits the live Anthropic Messages endpoint; tests must
/// substitute their own `AnthropicAdjudicator` built around a mock client or
/// `with_url()` pointing at a mock server.
pub fn build_default_adjudicator(
    api_key: String,
) -> Result<AnthropicAdjudicator<ReqwestClient>, cntrdct_core::DetectorError> {
    let client = ReqwestClient::new().map_err(|e| {
        cntrdct_core::DetectorError::Config(format!("reqwest init: {}", e))
    })?;
    Ok(AnthropicAdjudicator::new(client, api_key))
}

// ---------- Calibrate subcommand ----------

/// Read a JSONL corpus, compute per-detector priors, write them as pretty JSON
/// to `output_path`. Creates parent directories as needed.
///
/// Returns the number of detectors written (so the caller can print a friendly
/// message to stderr).
pub fn calibrate(
    corpus_path: &Path,
    output_path: &Path,
) -> Result<usize, CalibrateError> {
    let corpus = load_corpus(corpus_path)?;
    let priors = compute_priors(&corpus);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| CalibrateError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    let body = serde_json::to_string_pretty(&priors)?;
    fs::write(output_path, body).map_err(|e| CalibrateError::Io {
        path: output_path.to_path_buf(),
        source: e,
    })?;
    Ok(priors.len())
}

// ---------- Eval subcommand ----------

/// Load a manifest, run `scan` over the corpus directory, and compute the
/// `EvalReport`. Spec: `docs/spec/eval-v0.md` F7. Pure orchestration —
/// matching and metric arithmetic live in `cntrdct-eval`.
pub fn run_eval(corpus_dir: &Path, manifest_path: &Path) -> Result<EvalReport, EvalRunError> {
    let manifest = load_manifest(manifest_path)?;
    for entry in &manifest.entries {
        let abs = corpus_dir.join(&entry.file);
        if !abs.exists() {
            return Err(EvalRunError::Eval(EvalError::MissingSource(abs)));
        }
    }
    let findings = scan(corpus_dir)?;
    Ok(evaluate(&manifest, &findings, corpus_dir))
}

// ---------- File discovery ----------

fn collect_rust_files(path: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    if path.is_file() {
        if has_rs_extension(path) {
            paths.push(path.to_path_buf());
        }
        return paths;
    }

    for entry in WalkDir::new(path).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_file() && has_rs_extension(entry.path()) {
            paths.push(entry.path().to_path_buf());
        }
    }
    paths
}

fn has_rs_extension(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("rs")
}
