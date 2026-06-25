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
pub mod candidate_llm;
pub mod config;
pub mod core;
pub mod cross_model_kappa;
pub mod detectors;
pub mod eval;
pub mod ir;
pub mod llm_calibration;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod parsers;
pub mod ranker;
pub mod recall_audit;
pub mod sarif;
pub mod self_replication;

/// Canonical set of `Detector::id()` values registered by
/// [`scan_full_with_config`] and emitted as SARIF `tool.driver.rules`
/// by `src/main.rs`. Single source of truth so a removal or
/// addition at any one site without updating the others is caught by
/// `tests/wiring_consistency.rs` (Q-4).
///
/// Order is alphabetical and not load-bearing; tests sort before
/// comparing.
pub const ALL_DETECTOR_IDS: &[&str] = &[
    "arg-swap",
    "build-tag-interaction-go",
    "clone-drift",
    "comment-code",
    "config-interaction",
    "pr-miner",
    "python-unreachable-except",
    "unreachable-after-terminator",
];

use std::fs;
use std::path::{Path, PathBuf};

use crate::adjudicator::{
    AgyCliAdjudicator, AnthropicAdjudicator, ClaudeCliAdjudicator, FallbackAdjudicator,
    ReqwestClient,
};
use crate::calibration::{compute_priors, load_corpus, CalibrationError, DetectorPrior};
use crate::core::{
    register_detector, Adjudicator, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    RankedFinding, Ranker,
};
use crate::cross_model_kappa::{
    current_iso8601_utc, current_utc_date, load_corpus as load_audit_corpus, run_audit,
    write_report, AuditError, AuditReport, ProviderHandle, ProviderStatus,
};
use crate::detectors::arg_swap::ArgSwap;
use crate::detectors::clone_drift::CloneDrift;
use crate::detectors::comment_code::CommentCode;
use crate::detectors::lang::go_build_tag_interaction::GoBuildTagInteraction;
use crate::detectors::lang::python_unreachable_except::PythonUnreachableExcept;
use crate::detectors::lang::rust_config_interaction::ConfigInteraction;
use crate::detectors::pr_miner::PrMinerDetector;
use crate::detectors::unreachable_after_terminator::UnreachableAfterTerminator;
use crate::eval::{evaluate, load_manifest, EvalError, EvalReport};
use crate::ir::IrFile;
use crate::llm_calibration::{
    apply_platt, fit_registry, load_corpus as load_llm_corpus, PlattError, PlattRegistry,
};
use crate::parsers::{detect_language, Language};
use crate::ranker::{CalibratedRanker, UncalibratedRanker};
use crate::recall_audit::{audit_recall, load_audit_manifest, RecallAuditError, RecallAuditReport};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
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
pub enum RecallAuditRunError {
    #[error("recall audit error: {0}")]
    Audit(#[from] RecallAuditError),
    #[error("scan error: {0}")]
    Scan(#[from] ScanError),
    #[error("io error writing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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

/// Like [`scan`] but also returns the parsed [`IrFile`] vector. Callers
/// that want to run the suppression filter (`crate::config::apply`) need
/// both. Equivalent to [`scan_full_with_config`] called with the default
/// (empty) config.
pub fn scan_full(path: &Path) -> Result<(Vec<Finding>, Vec<IrFile>), ScanError> {
    scan_full_with_config(path, &crate::config::Config::default())
}

/// Like [`scan_full`] but consults `config.languages` to decide which
/// languages the file walker discovers. A `[languages.<canonical>]` table
/// with `enabled = false` causes the walker to skip files of that language;
/// every other language stays enabled. Spec: M-5
/// (`docs/spec/multilang-v0.md`).
///
/// R-1 (ir-v0.md §F2): each file is parsed once via
/// [`ParserProvider::to_ir`] and the resulting [`IrFile`] flows through
/// every detector. Files whose conversion returns
/// [`crate::ir::IrConvertError`] are silently skipped per the F2
/// production-runtime contract — `EmptySource` swallows the file with
/// no log; `LanguageMismatch` / `StructuralInvariant` skip and continue
/// so a single pathological file does not abort the scan.
pub fn scan_full_with_config(
    path: &Path,
    config: &crate::config::Config,
) -> Result<(Vec<Finding>, Vec<IrFile>), ScanError> {
    if !path.exists() {
        return Err(ScanError::PathNotFound(path.to_path_buf()));
    }

    let source_paths: Vec<(PathBuf, Language)> = collect_supported_files(path)
        .into_iter()
        .filter(|(_, lang)| config.language_enabled(*lang))
        .collect();

    // Read + parse + convert each file to IR in parallel. Unreadable
    // files (permission errors, transient races) and conversion errors
    // are silently skipped per F2's production-runtime contract.
    let parsed: Vec<IrFile> = source_paths
        .par_iter()
        .filter_map(|(p, lang)| read_and_convert_to_ir(p, *lang))
        .collect();

    run_detectors_on(parsed)
}

/// Read `path` from disk and run it through the appropriate
/// [`ParserProvider::to_ir`] converter. Returns `None` for any failure
/// shape — unreadable file, parse failure, or [`crate::ir::IrConvertError`]
/// variant — matching the v0.5.x "silently skip" behaviour at the file
/// walker boundary.
fn read_and_convert_to_ir(path: &Path, lang: Language) -> Option<IrFile> {
    let source = fs::read_to_string(path).ok()?;
    convert_source_to_ir(path, lang, source)
}

/// Build an [`IrFile`] from in-memory source for the supplied
/// language. Returns `None` if parsing or conversion fails. Public so
/// integration tests under `tests/` can construct IR inputs without
/// re-implementing the parser_for + to_ir dance per test file.
pub fn ir_from_source(path: &Path, lang: Language, source: String) -> Option<IrFile> {
    convert_source_to_ir(path, lang, source)
}

/// Shared body of [`read_and_convert_to_ir`] / [`scan_buffer`]: given
/// already-loaded source text, produce an [`IrFile`] via the
/// language's parser provider. Returns `None` on any conversion error
/// (matches the v0.5.x silent-skip semantics at the walker boundary).
fn convert_source_to_ir(path: &Path, lang: Language, source: String) -> Option<IrFile> {
    let provider = crate::parsers::parser_for(lang);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&provider.ts_language()).ok()?;
    let tree = parser.parse(&source, None)?;
    let shared: Arc<str> = Arc::from(source);
    provider.to_ir(tree, shared, path.to_path_buf()).ok()
}

/// Buffer-only scan path used by the LSP server (T3-12 Phase 1.b).
///
/// Skips the file walker entirely: builds a one-file `DetectContext` from
/// the supplied source string and runs the full Layer 1 detector battery
/// the same way [`scan_full_with_config`] does after reading from disk.
/// Returns empty findings (and an empty parsed vec) for paths whose
/// extension does not map to a [`Language`] — the caller (e.g.
/// `cntrdct::lsp::CntrdctLsp`) does not need to gate on language up
/// front. Spec: `docs/spec/lsp-v0.md` Phase 1.b.
pub fn scan_buffer(path: &Path, source: String) -> Result<(Vec<Finding>, Vec<IrFile>), ScanError> {
    let Some(language) = detect_language(path) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let Some(ir_file) = convert_source_to_ir(path, language, source) else {
        // The LSP/scan_buffer callers prefer "no findings on convert
        // failure" over a hard error so a single broken keystroke does
        // not propagate up as a scan error.
        return Ok((Vec::new(), Vec::new()));
    };
    run_detectors_on(vec![ir_file])
}

/// Run all eight Layer 1 detectors over a pre-built [`IrFile`] vector.
/// Shared between the disk-walking [`scan_full_with_config`] and the
/// buffer-only [`scan_buffer`] (used by the LSP server) so the registration
/// list and ordering rules live in exactly one place.
fn run_detectors_on(parsed: Vec<IrFile>) -> Result<(Vec<Finding>, Vec<IrFile>), ScanError> {
    let clone_drift = CloneDrift::new();
    let arg_swap = ArgSwap::new();
    let comment_code = CommentCode::new();
    let unreachable = UnreachableAfterTerminator::new();
    let config_interaction = ConfigInteraction::new();
    let pr_miner = PrMinerDetector::new();
    let python_unreachable_except = PythonUnreachableExcept::new();
    let go_build_tag_interaction = GoBuildTagInteraction::new();
    register_detector(&clone_drift)?;
    register_detector(&arg_swap)?;
    register_detector(&comment_code)?;
    register_detector(&unreachable)?;
    register_detector(&config_interaction)?;
    register_detector(&pr_miner)?;
    register_detector(&python_unreachable_except)?;
    register_detector(&go_build_tag_interaction)?;

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

    // Run all eight detectors in parallel against the shared context. Each
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
        &python_unreachable_except,
        &go_build_tag_interaction,
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
    files: &[IrFile],
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
/// `scripts/build_priors_corpus.py | cntrdct calibrate`. Spec:
/// design constraint P4.
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
pub fn adjudicate_top_n(
    ranked: &mut [RankedFinding],
    adjudicator: &dyn Adjudicator,
    top_n: usize,
) -> Result<(), crate::core::DetectorError> {
    for rf in ranked.iter_mut().take(top_n) {
        let result = adjudicator.adjudicate(rf)?;
        rf.adjudication = Some(result);
    }
    Ok(())
}

/// R-4 (review B5, `docs/spec/p3-amendment-v0.md` §3.3): adjudicate every
/// `Origin::Layer0Llm` candidate in `ranked` that is not already
/// adjudicated, REGARDLESS of the `--adjudicate-top` cap. Layer 0
/// proposes; Layer 3 disposes — a candidate the adjudicator never sees
/// has no precision floor and is suppressed from output downstream. Layer
/// 1 findings are untouched here (they go through `adjudicate_top_n`).
///
/// On the first adjudicator error the function returns `Err`; the caller
/// treats any still-unadjudicated Layer 0 candidate as suppressed.
pub fn adjudicate_layer0_candidates(
    ranked: &mut [RankedFinding],
    adjudicator: &dyn Adjudicator,
) -> Result<(), crate::core::DetectorError> {
    for rf in ranked.iter_mut() {
        if rf.finding.origin == crate::core::Origin::Layer0Llm && rf.adjudication.is_none() {
            let result = adjudicator.adjudicate(rf)?;
            rf.adjudication = Some(result);
        }
    }
    Ok(())
}

// ---------- Q-12 Platt scaling for LLM calibration ----------

/// Platt parameters shipped with the binary, embedded at compile time
/// from `benchmarks/llm-calibration/platt-default.json`. v0 ships an
/// empty object — the registry returned by [`embedded_platt_registry`]
/// is empty, and downstream `apply_llm_calibration` becomes a no-op.
/// A future tag that fits Platt over a real labelled corpus replaces
/// the file contents in the same shape.
///
/// Spec: `docs/spec/llm-calibration-v0.md` F6.
const EMBEDDED_PLATT_JSON: &str = include_str!("../benchmarks/llm-calibration/platt-default.json");

/// Parse [`EMBEDDED_PLATT_JSON`] into a registry. Empty / `{}` JSON
/// yields an empty registry. Malformed JSON triggers an
/// `expect`-panic that would be caught at CI build time.
pub fn embedded_platt_registry() -> PlattRegistry {
    let trimmed = EMBEDDED_PLATT_JSON.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return PlattRegistry::new();
    }
    PlattRegistry::from_json(trimmed)
        .expect("embedded benchmarks/llm-calibration/platt-default.json must parse cleanly")
}

/// Apply Q-12 Platt scaling to every adjudicated finding in `ranked`.
///
/// Walks each `RankedFinding` whose `adjudication` is `Some`, looks up
/// `(detector_id, anomaly_class)` in `registry`, and writes
/// `adjudication.calibrated_confidence`. Findings without an
/// adjudication are untouched. Findings whose cell has no Platt entry
/// receive `calibrated_confidence = None`. Idempotent: the helper
/// always overwrites the field, so a stale value from a previous
/// registry cannot persist.
///
/// Per design constraint P3, this is post-processing of the verdict
/// the adjudicator already returned; the helper does not invoke the
/// LLM and does not touch the network.
///
/// Spec: `docs/spec/llm-calibration-v0.md` F7.
pub fn apply_llm_calibration(ranked: &mut [RankedFinding], registry: &PlattRegistry) {
    for rf in ranked.iter_mut() {
        let det_id = rf.finding.detector_id.clone();
        let class = rf.finding.anomaly_class;
        if let Some(adj) = rf.adjudication.as_mut() {
            adj.calibrated_confidence = registry
                .get(&det_id, class)
                .map(|p| apply_platt(p, adj.confidence));
        }
    }
}

/// Read an LLM-confidence corpus, fit Platt parameters per
/// `(detector_id, anomaly_class)` cell, and write the resulting
/// registry as pretty JSON to `output_path`. Creates parent
/// directories as needed; output is sorted by composite key on write.
///
/// Returns the number of cells written (so the caller can print a
/// friendly message to stderr).
///
/// Spec: `docs/spec/llm-calibration-v0.md` F5.
pub fn fit_platt_calibration(corpus_path: &Path, output_path: &Path) -> Result<usize, PlattError> {
    let corpus = load_llm_corpus(corpus_path)?;
    let registry = fit_registry(&corpus)?;
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| PlattError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }
    fs::write(output_path, registry.to_json_pretty()).map_err(|e| PlattError::Io {
        path: output_path.to_path_buf(),
        source: e,
    })?;
    Ok(registry.len())
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

// ---------- Q-13 cross-model audit orchestration ----------

/// Probe whether `program` is invokable on the current `PATH`. Used by
/// the audit's CLI provider builders to surface a `Skipped` status
/// when the user has not installed the corresponding CLI.
fn cli_is_available(program: &str) -> bool {
    std::process::Command::new(program)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build a [`ProviderHandle`] for Claude Code's `claude --print`. The
/// provider is `Skipped` when the `claude` binary is not on `PATH` (or
/// when its `--version` probe fails) so the audit log surfaces the
/// omission without erroring out.
pub fn build_audit_claude_cli_provider() -> ProviderHandle {
    let provider_id = crate::adjudicator::CLAUDE_CLI_PROVIDER_ID.to_string();
    let model = crate::adjudicator::CLAUDE_CLI_MODEL.to_string();
    let program = std::env::var("CLAUDE_CLI_PROGRAM_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::CLAUDE_CLI_PROGRAM.to_string());
    if !cli_is_available(&program) {
        return ProviderHandle {
            provider_id,
            model,
            adjudicator: None,
            status: ProviderStatus::Skipped(format!("{} CLI not available on PATH", program)),
        };
    }
    match ClaudeCliAdjudicator::new() {
        Ok(adj) => ProviderHandle {
            provider_id,
            model,
            adjudicator: Some(Box::new(adj.with_program(program))),
            status: ProviderStatus::Live,
        },
        Err(e) => ProviderHandle {
            provider_id,
            model,
            adjudicator: None,
            status: ProviderStatus::Skipped(format!("tempdir alloc: {}", e)),
        },
    }
}

/// Resolve the `agy` model string, honouring the `AGY_CLI_MODEL_OVERRIDE`
/// env var (e.g. a paid Antigravity account bumping the lightweight
/// free-tier default to a heavier Gemini variant). A non-Anthropic Gemini
/// model is assumed; overriding to a Claude model would re-introduce the
/// self-preference conflict the default avoids. Public so the CLI's
/// self-preference guard can classify the adjudicator's actual model.
pub fn agy_cli_model() -> String {
    std::env::var("AGY_CLI_MODEL_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::AGY_CLI_MODEL.to_string())
}

/// Build a [`ProviderHandle`] for Antigravity's `agy -p` (the multi-model
/// CLI that replaced the retired `gemini` binary). Skipped semantics
/// mirror [`build_audit_claude_cli_provider`].
pub fn build_audit_agy_cli_provider() -> ProviderHandle {
    let provider_id = crate::adjudicator::AGY_CLI_PROVIDER_ID.to_string();
    let model = agy_cli_model();
    let program = std::env::var("AGY_CLI_PROGRAM_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::AGY_CLI_PROGRAM.to_string());
    if !cli_is_available(&program) {
        return ProviderHandle {
            provider_id,
            model,
            adjudicator: None,
            status: ProviderStatus::Skipped(format!("{} CLI not available on PATH", program)),
        };
    }
    match AgyCliAdjudicator::new() {
        Ok(adj) => ProviderHandle {
            provider_id,
            model: model.clone(),
            adjudicator: Some(Box::new(adj.with_program(program).with_model(model))),
            status: ProviderStatus::Live,
        },
        Err(e) => ProviderHandle {
            provider_id,
            model,
            adjudicator: None,
            status: ProviderStatus::Skipped(format!("tempdir alloc: {}", e)),
        },
    }
}

/// Resolve the `claude-cli` ADJUDICATOR model — Haiku by default (the
/// normal cheap `claude -p` adjudication path), overridable via
/// `CLAUDE_CLI_ADJUDICATE_MODEL_OVERRIDE`. Distinct from
/// [`crate::adjudicator::CLAUDE_CLI_MODEL`] (Sonnet), which the Layer 0
/// PROPOSER keeps.
fn claude_cli_adjudicate_model() -> String {
    std::env::var("CLAUDE_CLI_ADJUDICATE_MODEL_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::CLAUDE_CLI_ADJUDICATE_MODEL.to_string())
}

/// Build the Layer 3 `Adjudicator` for
/// `scan --adjudicate --adjudicate-via=claude-cli` — `claude -p` on the
/// Haiku adjudication model. Returns `None` when the `claude` CLI is not
/// invokable on `PATH` (the caller degrades to no adjudication, mirroring
/// a missing `ANTHROPIC_API_KEY`). Auth is the user's `claude`
/// subscription login — no API key is read.
pub fn build_claude_cli_adjudicator() -> Option<Box<dyn Adjudicator>> {
    let program = std::env::var("CLAUDE_CLI_PROGRAM_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::CLAUDE_CLI_PROGRAM.to_string());
    if !cli_is_available(&program) {
        return None;
    }
    ClaudeCliAdjudicator::new().ok().map(|adj| {
        Box::new(
            adj.with_program(program)
                .with_model(claude_cli_adjudicate_model()),
        ) as Box<dyn Adjudicator>
    })
}

/// Build the DEFAULT scan adjudicator: `claude -p` (Haiku) as primary, with
/// `agy` (Gemini) as a usage-limit FALLBACK — when the Claude subscription
/// hits its `$200` cap, adjudication transparently continues on Antigravity
/// (see [`FallbackAdjudicator`]). Degradation:
/// - claude available + agy available → the fallback chain.
/// - claude available, agy not → claude alone (no fallback).
/// - claude not available, agy available → agy alone.
/// - neither → `None` (caller continues without verdicts).
pub fn build_claude_cli_adjudicator_with_agy_fallback() -> Option<Box<dyn Adjudicator>> {
    match (build_claude_cli_adjudicator(), build_agy_cli_adjudicator()) {
        (Some(primary), Some(fallback)) => {
            Some(Box::new(FallbackAdjudicator::new(primary, fallback)))
        }
        (Some(primary), None) => Some(primary),
        (None, Some(fallback)) => Some(fallback),
        (None, None) => None,
    }
}

/// Task 1: build the Layer 3 `Adjudicator` for
/// `scan --adjudicate --adjudicate-via=agy-cli`. Returns `None` when the
/// `agy` CLI is not invokable on `PATH`. Forces a non-Anthropic Gemini
/// model (see [`agy_model`]) so the verdict carries no self-preference
/// bias against a `claude-cli` proposer.
pub fn build_agy_cli_adjudicator() -> Option<Box<dyn Adjudicator>> {
    let program = std::env::var("AGY_CLI_PROGRAM_OVERRIDE")
        .unwrap_or_else(|_| crate::adjudicator::AGY_CLI_PROGRAM.to_string());
    if !cli_is_available(&program) {
        return None;
    }
    AgyCliAdjudicator::new().ok().map(|adj| {
        Box::new(adj.with_program(program).with_model(agy_cli_model())) as Box<dyn Adjudicator>
    })
}

/// Q-13: run the cross-model κ audit pipeline.
///
/// Routes the same finding set through `claude --print` and `agy -p`
/// (Antigravity, running a non-Anthropic Gemini model so the pair is
/// genuinely cross-family), both authenticated via their respective
/// CLI's own login (no API keys are read or forwarded by cntrdct).
/// Missing CLIs surface as `Skipped` provider records; the audit errors
/// out only when fewer than two live providers remain.
///
/// Per design constraint P3, this entry point is the only public path
/// that invokes the cross-model adjudicators. `scan` / `calibrate` /
/// `eval` remain network-free.
pub fn run_cross_model_audit(corpus_path: &Path) -> Result<AuditReport, AuditError> {
    let inputs = load_audit_corpus(corpus_path)?;
    let providers = vec![
        build_audit_claude_cli_provider(),
        build_audit_agy_cli_provider(),
    ];
    let date = current_utc_date();
    let generated_at = current_iso8601_utc();
    run_audit(date, generated_at, providers, inputs)
}

/// Q-13: write `report` as pretty JSON to `output_path`. Re-exported
/// from [`crate::cross_model_kappa::write_report`] for the CLI surface.
pub fn write_cross_model_audit(output_path: &Path, report: &AuditReport) -> Result<(), AuditError> {
    write_report(output_path, report)
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
    // Sort keys so the serialized priors file is byte-stable across runs.
    // compute_priors returns a HashMap (insertion order undefined), and
    // priors-default.json is checked into the repo: a non-deterministic
    // serialization would produce spurious diffs every recalibration.
    let sorted: std::collections::BTreeMap<&String, &DetectorPrior> = priors.iter().collect();
    let body = serde_json::to_string_pretty(&sorted)?;
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

// ---------- Recall-audit subcommand (Q-14) ----------

/// Load an audit manifest, run `scan` over the audit corpus directory, and
/// compute the `RecallAuditReport`. Spec: `docs/spec/recall-audit-v0.md` F7.
///
/// The audit corpus carries externally-sourced ground truth (CVEs / OSV.dev
/// advisories / Semgrep / CodeQL / Clippy testset entries) so the per-detector
/// recall numbers are not subject to the labeller-bias loop that affects
/// `priors-default.json` (Heckman & Williams IST 2011).
pub fn run_recall_audit(
    corpus_dir: &Path,
    manifest_path: &Path,
) -> Result<RecallAuditReport, RecallAuditRunError> {
    let manifest = load_audit_manifest(manifest_path)?;
    for entry in &manifest.entries {
        let abs = corpus_dir.join(&entry.file);
        if !abs.exists() {
            return Err(RecallAuditRunError::Audit(RecallAuditError::MissingSource(
                abs,
            )));
        }
    }
    let findings = scan(corpus_dir)?;
    Ok(audit_recall(&manifest, &findings, corpus_dir))
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
