//! cntrdct library entry point.
//!
//! Layer modules:
//! - [`core`]: shared types and traits (Detector, Ranker, Adjudicator).
//! - [`parsers`]: language detection and tree-sitter providers.
//! - [`detectors`]: Layer 1 deterministic detectors.
//! - [`ranker`] / [`calibration`]: Layer 2 statistical ranking.
//! - [`adjudicator`]: Layer 3 LLM adjudicator (Anthropic Messages API).
//! - [`sarif`]: SARIF 2.1.0 emitter.
//! - [`eval`]: precision/recall/F1 harness.
//! - [`config`]: `cntrdct.toml` and in-source suppression.
//!
//! The binary entry point is `src/main.rs`; the library glue
//! (CLI orchestration helpers) lives in this file.
//!
//! Specs:
//! - `docs/spec/cli-v0.md` — base scan command.
//! - `docs/spec/ranker-v1.md` — calibrate subcommand and ranker
//!   selection (calibrated when priors are available, uncalibrated
//!   otherwise).

pub mod adjudicator;
pub mod calibration;
pub mod config;
pub mod core;
pub mod detectors;
pub mod eval;
pub mod parsers;
pub mod ranker;
pub mod sarif;

use std::fs;
use std::path::{Path, PathBuf};

use crate::adjudicator::{AnthropicAdjudicator, ReqwestClient};
use crate::calibration::{compute_priors, load_corpus, CalibrationError, DetectorPrior};
use crate::core::{
    register_detector, Adjudicator, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    ParsedFile, RankedFinding, Ranker,
};
use crate::detectors::arg_swap::ArgSwap;
use crate::detectors::clone_drift::CloneDrift;
use crate::detectors::comment_code::CommentCode;
use crate::detectors::config_interaction::ConfigInteraction;
use crate::detectors::pr_miner::PrMinerDetector;
use crate::detectors::unreachable_after_terminator::UnreachableAfterTerminator;
use crate::eval::{evaluate, load_manifest, EvalError, EvalReport};
use crate::parsers::{detect_language, Language};
use crate::ranker::{CalibratedRanker, UncalibratedRanker};
use rayon::prelude::*;
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
    Detector(#[from] crate::core::DetectorError),
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
    scan_full(path).map(|(findings, _)| findings)
}

/// Like [`scan`] but also returns the parsed files. Callers that want to run
/// the suppression filter (`crate::config::apply`) need both. Equivalent to
/// [`scan_full_with_config`] called with the default (empty) config.
pub fn scan_full(path: &Path) -> Result<(Vec<Finding>, Vec<ParsedFile>), ScanError> {
    scan_full_with_config(path, &crate::config::Config::default())
}

/// Like [`scan_full`] but consults `config.languages` to decide which
/// languages the file walker discovers. A `[languages.<canonical>]` table
/// with `enabled = false` causes the walker to skip files of that language;
/// every other language stays enabled. Spec: M-5
/// (`docs/spec/multilang-v0.md`).
pub fn scan_full_with_config(
    path: &Path,
    config: &crate::config::Config,
) -> Result<(Vec<Finding>, Vec<ParsedFile>), ScanError> {
    if !path.exists() {
        return Err(ScanError::PathNotFound(path.to_path_buf()));
    }

    let source_paths: Vec<(PathBuf, Language)> = collect_supported_files(path)
        .into_iter()
        .filter(|(_, lang)| config.language_enabled(*lang))
        .collect();

    // Read files in parallel. Unreadable files (permission errors, transient
    // races) are silently skipped, matching the previous serial behaviour.
    let parsed: Vec<ParsedFile> = source_paths
        .par_iter()
        .filter_map(|(p, lang)| {
            fs::read_to_string(p).ok().map(|source| ParsedFile {
                path: p.clone(),
                language: *lang,
                source,
            })
        })
        .collect();

    let clone_drift = CloneDrift::new();
    let arg_swap = ArgSwap::new();
    let comment_code = CommentCode::new();
    let unreachable = UnreachableAfterTerminator::new();
    let config_interaction = ConfigInteraction::new();
    let pr_miner = PrMinerDetector::new();
    register_detector(&clone_drift)?;
    register_detector(&arg_swap)?;
    register_detector(&comment_code)?;
    register_detector(&unreachable)?;
    register_detector(&config_interaction)?;
    register_detector(&pr_miner)?;

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

    // Run all five detectors in parallel against the shared context. Each
    // detector implementation is `Send + Sync` per the trait bound, so this
    // is sound. Output ordering is restored via a deterministic post-hoc
    // sort below so the ranker (and snapshot tests) see stable input.
    let detectors: Vec<&(dyn Detector + Sync)> = vec![
        &clone_drift,
        &arg_swap,
        &comment_code,
        &unreachable,
        &config_interaction,
        &pr_miner,
    ];
    let nested: Result<Vec<Vec<Finding>>, crate::core::DetectorError> =
        detectors.par_iter().map(|d| d.detect(&ctx)).collect();
    let mut findings: Vec<Finding> = nested?.into_iter().flatten().collect();

    findings.sort_by(|a, b| {
        a.detector_id
            .cmp(&b.detector_id)
            .then_with(|| a.primary.file.cmp(&b.primary.file))
            .then_with(|| a.primary.start_line.cmp(&b.primary.start_line))
            .then_with(|| a.primary.start_col.cmp(&b.primary.start_col))
    });

    Ok((findings, parsed))
}

// ---------- Suppression / config integration ----------

/// Discover `<root>/cntrdct.toml` (or load `--config` if given), then apply
/// path globs, attribute suppressions, and per-detector overrides to the
/// findings. Returns the surviving findings in source order.
///
/// Spec: T2-7 (`docs/spec/suppression-v0.md`).
pub fn apply_suppression(
    config_override: Option<&Path>,
    scan_root: &Path,
    files: &[ParsedFile],
    findings: Vec<Finding>,
) -> Result<Vec<Finding>, crate::config::ConfigError> {
    let config = load_config(config_override, scan_root)?;
    crate::config::apply(&config, files, findings)
}

/// Resolve the active config: prefer `--config <path>` when supplied,
/// otherwise fall back to `<scan_root>/cntrdct.toml` discovery, otherwise
/// return `Config::default()`. Pulled out of `apply_suppression` so callers
/// (notably `main.rs`) can load the config exactly once and feed it to both
/// `scan_full_with_config` and `crate::config::apply` without re-reading
/// the file. Spec: M-5 (`docs/spec/multilang-v0.md`).
pub fn load_config(
    config_override: Option<&Path>,
    scan_root: &Path,
) -> Result<crate::config::Config, crate::config::ConfigError> {
    if let Some(p) = config_override {
        crate::config::Config::load_from(p)
    } else {
        Ok(crate::config::Config::discover_in(scan_root)?.unwrap_or_default())
    }
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

/// Priors shipped with the binary. Embedded at compile time from
/// `benchmarks/priors-default.json`, which is regenerated by
/// `scripts/build_priors_corpus.py | cntrdct calibrate`. Spec: P-4
/// (`ROADMAP.md`).
///
/// Uses `include_str!` so a fresh `cargo install cntrdct` (or the
/// pre-built release binary) ships with calibrated rankings out of the
/// box — no `cntrdct calibrate` step required by end users. The
/// per-user cache file (`default_priors_path()`) still wins when
/// present, so users who ran `cntrdct calibrate` against their own
/// corpus keep their override.
const EMBEDDED_PRIORS_JSON: &str = include_str!("../benchmarks/priors-default.json");

/// Parse the [`EMBEDDED_PRIORS_JSON`] constant. Returns `None` when the
/// embedded JSON is empty (i.e. the file shipped is intentionally
/// blank, signalling "no default priors"); `Some` when parsing
/// succeeds; an `expect`-panic if the embedded JSON is malformed,
/// which would be a CI-caught build-time bug.
fn embedded_priors() -> Option<HashMap<String, DetectorPrior>> {
    let trimmed = EMBEDDED_PRIORS_JSON.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return None;
    }
    let parsed: HashMap<String, DetectorPrior> = serde_json::from_str(trimmed)
        .expect("embedded benchmarks/priors-default.json must parse cleanly");
    Some(parsed)
}

/// Build the appropriate ranker according to caller intent.
///
/// Lookup order, first hit wins:
/// 1. `force_uncalibrated == true` → always [`UncalibratedRanker`].
/// 2. `priors_override` is `Some` → load from there. If the path does
///    NOT exist, fall back to [`UncalibratedRanker`] (preserves the
///    pre-P-4 silent-fallback contract for explicit overrides — a user
///    pointing at a missing file is almost certainly debugging and
///    expects to NOT silently get embedded priors).
/// 3. `default_priors_path()` (per-user cache) exists → load it.
/// 4. [`EMBEDDED_PRIORS_JSON`] non-empty → load embedded priors. This
///    is the P-4 deliverable: a fresh `cargo install cntrdct` ships
///    with calibrated rankings out of the box.
/// 5. [`UncalibratedRanker`] (final fallback).
pub fn pick_ranker(
    force_uncalibrated: bool,
    priors_override: Option<&Path>,
) -> Result<Box<dyn Ranker>, CalibrateError> {
    if force_uncalibrated {
        return Ok(Box::new(UncalibratedRanker::new()));
    }

    if let Some(p) = priors_override {
        let chosen = try_load_priors(p)?;
        return Ok(match chosen {
            Some(priors) => Box::new(CalibratedRanker::new(priors)),
            None => Box::new(UncalibratedRanker::new()),
        });
    }

    let from_cache: Option<HashMap<String, DetectorPrior>> = match default_priors_path() {
        Some(p) => try_load_priors(&p)?,
        None => None,
    };

    let chosen = from_cache.or_else(embedded_priors);

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
) -> Result<(), crate::core::DetectorError> {
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
) -> Result<AnthropicAdjudicator<ReqwestClient>, crate::core::DetectorError> {
    let client = ReqwestClient::new()
        .map_err(|e| crate::core::DetectorError::Config(format!("reqwest init: {}", e)))?;
    Ok(AnthropicAdjudicator::new(client, api_key))
}

// ---------- Calibrate subcommand ----------

/// Read a JSONL corpus, compute per-detector priors, write them as pretty JSON
/// to `output_path`. Creates parent directories as needed.
///
/// Returns the number of detectors written (so the caller can print a friendly
/// message to stderr).
pub fn calibrate(corpus_path: &Path, output_path: &Path) -> Result<usize, CalibrateError> {
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

/// Walk `path` and return every file whose extension maps to a supported
/// `Language` per `crate::parsers::detect_language`. Files with unknown
/// extensions are silently dropped, mirroring the previous `.rs`-only
/// behaviour. Spec: `multilang-v0.md` F5.
fn collect_supported_files(path: &Path) -> Vec<(PathBuf, Language)> {
    let mut paths: Vec<(PathBuf, Language)> = Vec::new();

    if path.is_file() {
        if let Some(lang) = detect_language(path) {
            paths.push((path.to_path_buf(), lang));
        }
        return paths;
    }

    for entry in WalkDir::new(path).sort_by_file_name() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(lang) = detect_language(entry.path()) {
            paths.push((entry.path().to_path_buf(), lang));
        }
    }
    paths
}
