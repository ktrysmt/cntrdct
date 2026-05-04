//! cntrdct CLI library entry point.
//!
//! Specs:
//! - `cntrdct/docs/spec/cli-v0.md` — base scan command.
//! - `cntrdct/docs/spec/ranker-v1.md` — calibrate subcommand and ranker
//!   selection (calibrated when priors are available, uncalibrated otherwise).

use std::fs;
use std::path::{Path, PathBuf};

use cntrdct_adjudicator_llm::{AnthropicAdjudicator, ReqwestClient};
use cntrdct_calibration::{compute_priors, load_corpus, CalibrationError, DetectorPrior};
use cntrdct_core::{
    register_detector, Adjudicator, CorpusStats, DetectContext, Detector, DetectorConfig, Finding,
    ParsedFile, RankedFinding, Ranker,
};
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;
use cntrdct_detector_config_interaction::ConfigInteraction;
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;
use cntrdct_eval::{evaluate, load_manifest, EvalError, EvalReport};
use cntrdct_parsers::{detect_language, Language};
use cntrdct_ranker::{CalibratedRanker, UncalibratedRanker};
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

#[derive(Debug, Error)]
pub enum FetchRunError {
    #[error("could not read crates list at {path}: {source}")]
    ReadList {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("io error under {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("http client init failed: {0}")]
    HttpInit(String),
    #[error("fetch error: {0}")]
    Fetch(#[from] cntrdct_corpus_fetch::FetchError),
    #[error("rayon thread pool init failed: {0}")]
    ThreadPool(String),
}

#[derive(Debug, Error)]
pub enum ClippyHarnessError {
    #[error("--accept-arbitrary-code is required: cntrdct clippy compiles third-party Rust source via cargo, which executes build.rs scripts and proc macros. Run only in an isolated environment (container, VM, dedicated user).")]
    ConsentRequired,
    #[error("io error under {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("manifest read failed: {0}")]
    ManifestRead(String),
    #[error("fetch error: {0}")]
    Fetch(#[from] cntrdct_corpus_fetch::FetchError),
    #[error("http client init failed: {0}")]
    HttpInit(String),
    #[error("cargo invocation failed for {crate_name}: {source}")]
    CargoSpawn {
        crate_name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClippyCrateResult {
    pub name: String,
    pub version: String,
    pub diagnostics: usize,
    pub compile_status: i32,
    pub output_file: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ClippyHarnessSummary {
    pub processed: usize,
    pub failed_compile: Vec<String>,
    pub crates: Vec<ClippyCrateResult>,
}

#[derive(Debug, Error)]
pub enum OverlapError {
    #[error("could not read findings at {path}: {source}")]
    ReadFindings {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("findings file at {path} is not a JSON array")]
    NotArray { path: PathBuf },
    #[error("parse error in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("io error under {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum AggregateError {
    #[error("could not read findings at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("findings file at {path} is not a JSON array")]
    NotArray { path: PathBuf },
    #[error("findings parse error in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("io error under {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn scan(path: &Path) -> Result<Vec<Finding>, ScanError> {
    scan_full(path).map(|(findings, _)| findings)
}

/// Like [`scan`] but also returns the parsed files. Callers that want to run
/// the suppression filter (`cntrdct_config::apply`) need both.
pub fn scan_full(path: &Path) -> Result<(Vec<Finding>, Vec<ParsedFile>), ScanError> {
    if !path.exists() {
        return Err(ScanError::PathNotFound(path.to_path_buf()));
    }

    let source_paths = collect_supported_files(path);

    // Read files in parallel. Unreadable files (permission errors, transient
    // races) are silently skipped, matching the previous serial behaviour.
    let parsed: Vec<ParsedFile> = source_paths
        .par_iter()
        .filter_map(|(p, lang)| {
            fs::read_to_string(p).ok().map(|source| ParsedFile {
                path: p.clone(),
                language: lang.canonical_name().to_string(),
                source,
            })
        })
        .collect();

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
    ];
    let nested: Result<Vec<Vec<Finding>>, cntrdct_core::DetectorError> =
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
) -> Result<Vec<Finding>, cntrdct_config::ConfigError> {
    let config = if let Some(p) = config_override {
        cntrdct_config::Config::load_from(p)?
    } else {
        cntrdct_config::Config::discover_in(scan_root)?.unwrap_or_default()
    };
    cntrdct_config::apply(&config, files, findings)
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
    let client = ReqwestClient::new()
        .map_err(|e| cntrdct_core::DetectorError::Config(format!("reqwest init: {}", e)))?;
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

// ---------- Overlap matrix (Phase 0 baseline) ----------

/// One row of the cntrdct × clippy overlap CSV emitted by [`run_overlap`].
/// `count` is the number of cntrdct findings produced by `detector` whose
/// `(crate_dir, rel_path, start_line)` triple co-occurs with at least one
/// clippy diagnostic carrying lint `clippy_lint` at the same location.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OverlapRow {
    pub detector: String,
    pub clippy_lint: String,
    pub count: usize,
}

/// Compute the cntrdct × clippy overlap matrix.
///
/// Inputs:
/// - `findings_path`: cntrdct's `Vec<RankedFinding>` JSON.
/// - `clippy_dir`: directory of `<name>-<version>.clippy.json` files
///   produced by [`run_clippy_harness`].
/// - `corpus_root`: corpus root used to interpret the findings' `file`
///   paths (same value passed to `cntrdct aggregate`).
///
/// The match key is exact-line: the cntrdct finding's
/// `(crate_dir, rel_path, start_line)` must be identical to one of the
/// clippy diagnostic's spans. Fuzzy line matching (±N) is left to a
/// downstream pivot — the long-format CSV here is small enough to feed
/// straight into pandas / R.
///
/// Findings whose path falls outside `corpus_root` and clippy
/// diagnostics with empty spans are silently dropped so the matrix
/// reflects only well-localised intersections.
pub fn run_overlap(
    findings_path: &Path,
    clippy_dir: &Path,
    corpus_root: &Path,
    output: Option<&Path>,
) -> Result<Vec<OverlapRow>, OverlapError> {
    use std::collections::BTreeMap;

    let canonical_root = canonicalise_or(corpus_root);
    let clippy_index = load_clippy_index(clippy_dir)?;

    let body = fs::read_to_string(findings_path).map_err(|e| OverlapError::ReadFindings {
        path: findings_path.to_path_buf(),
        source: e,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| OverlapError::Parse {
            path: findings_path.to_path_buf(),
            source: e,
        })?;
    let findings = value.as_array().ok_or_else(|| OverlapError::NotArray {
        path: findings_path.to_path_buf(),
    })?;

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for f in findings {
        let detector = match f
            .get("finding")
            .and_then(|v| v.get("detector_id"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s.to_string(),
            None => continue,
        };
        let primary = match f.get("finding").and_then(|v| v.get("primary")) {
            Some(p) => p,
            None => continue,
        };
        let file_str = match primary.get("file").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let line = match primary.get("start_line").and_then(|v| v.as_u64()) {
            Some(l) => l,
            None => continue,
        };

        let path = canonicalise_or(Path::new(file_str));
        let rel = match path.strip_prefix(&canonical_root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        let mut comps = rel.components();
        let crate_dir = match comps.next().and_then(|c| c.as_os_str().to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let rel_rest: PathBuf = comps.as_path().to_path_buf();
        let rel_path = rel_rest.to_string_lossy().replace('\\', "/");

        if let Some(lints) = clippy_index.get(&(crate_dir, rel_path, line)) {
            for lint in lints {
                *counts.entry((detector.clone(), lint.clone())).or_insert(0) += 1;
            }
        }
    }

    let rows: Vec<OverlapRow> = counts
        .into_iter()
        .map(|((detector, clippy_lint), count)| OverlapRow {
            detector,
            clippy_lint,
            count,
        })
        .collect();

    let mut buf = String::from("detector,clippy_lint,count\n");
    for r in &rows {
        buf.push_str(&format!("{},{},{}\n", r.detector, r.clippy_lint, r.count));
    }
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| OverlapError::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            fs::write(p, buf).map_err(|e| OverlapError::Io {
                path: p.to_path_buf(),
                source: e,
            })?;
        }
        None => print!("{buf}"),
    }
    Ok(rows)
}

type ClippyKey = (String, String, u64);
type ClippyIndex = std::collections::HashMap<ClippyKey, std::collections::HashSet<String>>;

/// Build a `(crate_dir, rel_path, line) -> Set<lint>` index from a
/// directory of `<name>-<version>.clippy.json` files. Files whose name
/// does not match that pattern are skipped silently so unrelated artefacts
/// in the directory (e.g. `summary.json`) do not poison the index.
fn load_clippy_index(clippy_dir: &Path) -> Result<ClippyIndex, OverlapError> {
    let mut index: ClippyIndex = std::collections::HashMap::new();
    let entries = fs::read_dir(clippy_dir).map_err(|e| OverlapError::Io {
        path: clippy_dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| OverlapError::Io {
            path: clippy_dir.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let crate_dir = match filename.strip_suffix(".clippy.json") {
            Some(c) => c.to_string(),
            None => continue,
        };

        let body = fs::read_to_string(&path).map_err(|e| OverlapError::Io {
            path: path.clone(),
            source: e,
        })?;
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| OverlapError::Parse {
                path: path.clone(),
                source: e,
            })?;
        let arr = match value.as_array() {
            Some(a) => a,
            None => continue,
        };
        for diag in arr {
            let lint = match diag
                .get("message")
                .and_then(|m| m.get("code"))
                .and_then(|c| c.get("code"))
                .and_then(|s| s.as_str())
            {
                Some(l) => l.to_string(),
                None => continue,
            };
            let spans = match diag
                .get("message")
                .and_then(|m| m.get("spans"))
                .and_then(|s| s.as_array())
            {
                Some(s) => s,
                None => continue,
            };
            for span in spans {
                let file = match span.get("file_name").and_then(|f| f.as_str()) {
                    Some(f) => f.replace('\\', "/"),
                    None => continue,
                };
                let line = match span.get("line_start").and_then(|l| l.as_u64()) {
                    Some(l) => l,
                    None => continue,
                };
                index
                    .entry((crate_dir.clone(), file, line))
                    .or_default()
                    .insert(lint.clone());
            }
        }
    }
    Ok(index)
}

// ---------- Clippy harness (Phase 0 baseline) ----------

/// Filter `cargo clippy --message-format=json` stdout to the JSON objects
/// representing clippy diagnostics.
///
/// Stdout is one JSON object per line. We keep entries whose
/// `reason == "compiler-message"` and whose `message.code.code` field
/// starts with the literal `clippy::` prefix. Everything else (build
/// progress, rustc warnings, unrelated noise) is dropped.
///
/// Pure helper, no I/O — exposed pub so it can be unit tested against a
/// fixture stdout payload without invoking cargo.
pub fn parse_clippy_diagnostics(stdout: &[u8]) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for line in stdout.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_slice(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }
        let code = v
            .get("message")
            .and_then(|m| m.get("code"))
            .and_then(|c| c.get("code"))
            .and_then(|s| s.as_str());
        if let Some(c) = code {
            if c.starts_with("clippy::") {
                out.push(v);
            }
        }
    }
    out
}

/// Run `cargo clippy --message-format=json` against every crate in the
/// supplied manifest, capturing the diagnostic stream into per-crate JSON
/// files.
///
/// Refuses to start unless `accept_arbitrary_code` is `true`; the flag
/// exists so the call site is forced to acknowledge that compiling
/// third-party crates means executing build scripts and proc macros from
/// crates.io. `cntrdct clippy --accept-arbitrary-code` is the only way to
/// reach this code path through the CLI.
pub fn run_clippy_harness(
    manifest_path: &Path,
    out_dir: &Path,
    accept_arbitrary_code: bool,
) -> Result<ClippyHarnessSummary, ClippyHarnessError> {
    use cntrdct_corpus_fetch::{extract_filtered, ExtractOptions, ReqwestClient, TarballClient};

    if !accept_arbitrary_code {
        return Err(ClippyHarnessError::ConsentRequired);
    }

    eprintln!("WARNING: cntrdct clippy compiles third-party Rust source via cargo, which");
    eprintln!("executes build.rs scripts and proc macros from those crates.");

    let rows = cntrdct_corpus_fetch::read_manifest_rows(manifest_path)
        .map_err(|e| ClippyHarnessError::ManifestRead(e.to_string()))?;
    fs::create_dir_all(out_dir).map_err(|e| ClippyHarnessError::Io {
        path: out_dir.to_path_buf(),
        source: e,
    })?;

    let tarball = TarballClient::new(
        ReqwestClient::new().map_err(|e| ClippyHarnessError::HttpInit(e.to_string()))?,
    );

    let permissive = ExtractOptions {
        max_file_bytes: u64::MAX,
        exclude_dirs: Vec::new(),
        include_extensions: Vec::new(),
    };

    let mut summary = ClippyHarnessSummary::default();

    for row in rows {
        let bytes = tarball.fetch_verified(&row.name, &row.version, &row.sha256)?;

        let temp = tempfile::tempdir().map_err(|e| ClippyHarnessError::Io {
            path: out_dir.to_path_buf(),
            source: e,
        })?;
        let crate_dir = temp.path().join(format!("{}-{}", row.name, row.version));
        fs::create_dir_all(&crate_dir).map_err(|e| ClippyHarnessError::Io {
            path: crate_dir.clone(),
            source: e,
        })?;
        extract_filtered(&bytes, &crate_dir, &permissive)?;

        let output = std::process::Command::new("cargo")
            .arg("clippy")
            .arg("--message-format=json")
            .arg("--quiet")
            .arg("--no-deps")
            .current_dir(&crate_dir)
            .output()
            .map_err(|e| ClippyHarnessError::CargoSpawn {
                crate_name: row.name.clone(),
                source: e,
            })?;

        let diagnostics = parse_clippy_diagnostics(&output.stdout);
        let out_file = out_dir.join(format!("{}-{}.clippy.json", row.name, row.version));
        let body = serde_json::to_string_pretty(&diagnostics)?;
        fs::write(&out_file, body).map_err(|e| ClippyHarnessError::Io {
            path: out_file.clone(),
            source: e,
        })?;

        let compile_status = output.status.code().unwrap_or(-1);
        let result = ClippyCrateResult {
            name: row.name.clone(),
            version: row.version.clone(),
            diagnostics: diagnostics.len(),
            compile_status,
            output_file: out_file,
        };
        eprintln!(
            "clippy: {} {} ({} diagnostics, exit {})",
            result.name, result.version, result.diagnostics, result.compile_status
        );
        if compile_status != 0 && diagnostics.is_empty() {
            summary.failed_compile.push(row.name.clone());
        }
        summary.crates.push(result);
        summary.processed += 1;
    }

    let summary_path = out_dir.join("summary.json");
    let body = serde_json::to_string_pretty(&summary)?;
    fs::write(&summary_path, body).map_err(|e| ClippyHarnessError::Io {
        path: summary_path,
        source: e,
    })?;

    Ok(summary)
}

// ---------- Aggregate / sample subcommands (study Phase 0) ----------

/// One row of the per-(crate, detector) firing-count table emitted by
/// [`run_aggregate`]. The crate identifier is the directory name as it
/// appears under `corpus_root` — typically `<name>-<version>`. Splitting
/// that into separate name and version columns is intentionally left to
/// the caller, who can JOIN against `manifest.csv` if they want them
/// separated.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AggregateRow {
    pub crate_dir: String,
    pub detector: String,
    pub count: usize,
}

/// Parse a `Vec<RankedFinding>` JSON file, attribute each finding to a
/// `<crate>-<version>` directory under `corpus_root`, and emit a CSV with
/// per-(crate, detector) firing counts. Findings whose `primary.file` does
/// not live inside `corpus_root` are skipped silently — the empirical
/// study scans the corpus directory only, so any external paths in the
/// stream are noise to the aggregator.
pub fn run_aggregate(
    findings_path: &Path,
    corpus_root: &Path,
    output: Option<&Path>,
) -> Result<Vec<AggregateRow>, AggregateError> {
    use std::collections::BTreeMap;

    let body = fs::read_to_string(findings_path).map_err(|e| AggregateError::Read {
        path: findings_path.to_path_buf(),
        source: e,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AggregateError::Parse {
            path: findings_path.to_path_buf(),
            source: e,
        })?;
    let findings = value.as_array().ok_or_else(|| AggregateError::NotArray {
        path: findings_path.to_path_buf(),
    })?;

    let canonical_root = canonicalise_or(corpus_root);

    // BTreeMap so the output order is deterministic.
    let mut bins: BTreeMap<(String, String), usize> = BTreeMap::new();
    for f in findings {
        let detector = match f
            .get("finding")
            .and_then(|v| v.get("detector_id"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s.to_string(),
            None => continue,
        };
        let file_str = match f
            .get("finding")
            .and_then(|v| v.get("primary"))
            .and_then(|v| v.get("file"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s,
            None => continue,
        };
        let path = canonicalise_or(Path::new(file_str));
        let rel = match path.strip_prefix(&canonical_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let crate_dir = match rel.components().next().and_then(|c| c.as_os_str().to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        *bins.entry((crate_dir, detector)).or_insert(0) += 1;
    }

    let rows: Vec<AggregateRow> = bins
        .into_iter()
        .map(|((crate_dir, detector), count)| AggregateRow {
            crate_dir,
            detector,
            count,
        })
        .collect();

    let mut buf = String::from("crate_dir,detector,count\n");
    for r in &rows {
        buf.push_str(&format!("{},{},{}\n", r.crate_dir, r.detector, r.count));
    }
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| AggregateError::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            fs::write(p, buf).map_err(|e| AggregateError::Io {
                path: p.to_path_buf(),
                source: e,
            })?;
        }
        None => {
            print!("{buf}");
        }
    }
    Ok(rows)
}

/// Read a `Vec<RankedFinding>` JSON file, group by detector_id, and
/// emit a deterministic stratified random sample of `per_detector`
/// findings per detector (or fewer if a group is shorter). The sample is
/// reproducible across runs given the same `seed`.
pub fn run_sample(
    findings_path: &Path,
    per_detector: usize,
    seed: u64,
    output: Option<&Path>,
) -> Result<Vec<serde_json::Value>, AggregateError> {
    use std::collections::BTreeMap;

    let body = fs::read_to_string(findings_path).map_err(|e| AggregateError::Read {
        path: findings_path.to_path_buf(),
        source: e,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AggregateError::Parse {
            path: findings_path.to_path_buf(),
            source: e,
        })?;
    let findings = value
        .as_array()
        .ok_or_else(|| AggregateError::NotArray {
            path: findings_path.to_path_buf(),
        })?
        .clone();

    let mut groups: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for f in findings {
        let detector = f
            .get("finding")
            .and_then(|v| v.get("detector_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)")
            .to_string();
        groups.entry(detector).or_default().push(f);
    }

    let mut rng = fastrand::Rng::with_seed(seed);
    let mut sample: Vec<serde_json::Value> = Vec::new();
    for (_detector, mut group) in groups {
        rng.shuffle(&mut group);
        group.truncate(per_detector);
        sample.extend(group);
    }

    let buf = serde_json::to_string_pretty(&sample).expect("findings array reserialises cleanly");
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| AggregateError::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            fs::write(p, buf).map_err(|e| AggregateError::Io {
                path: p.to_path_buf(),
                source: e,
            })?;
        }
        None => {
            println!("{buf}");
        }
    }
    Ok(sample)
}

fn canonicalise_or(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

// ---------- Fetch subcommand ----------

/// Per-skip record produced by [`run_fetch`]. Serialised into the summary
/// JSON the CLI prints to stdout.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchSkipRecord {
    pub name: String,
    pub reason: &'static str,
    /// SPDX expression that triggered a `license_rejected` skip; absent
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spdx: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchSummary {
    pub out_dir: PathBuf,
    pub manifest: PathBuf,
    pub fetched: usize,
    pub skipped: usize,
    pub errors: usize,
    /// Number of input crates skipped because they were already recorded in
    /// the existing manifest (only non-zero when `--resume` is set).
    pub resume_skipped: usize,
    pub skips: Vec<FetchSkipRecord>,
}

/// One entry in a crate-list file. The optional `downloads` field is
/// populated when the list was generated by `cntrdct rank` (each line then
/// has the form `<name> <downloads>`); plain `<name>` lines parse as
/// `downloads: None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateListEntry {
    pub name: String,
    pub downloads: Option<u64>,
}

/// Progress reporting format for `run_fetch`. Default `Text` is human-readable
/// (`fetched: serde 1.0.2`); `NdJson` emits one JSON object per line so
/// downstream tooling can ingest the stream without parsing free-form text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FetchProgress {
    #[default]
    Text,
    NdJson,
}

/// Run the corpus-fetch pipeline against a list of crate names.
///
/// The list file is read line-by-line. Blank lines and lines starting with
/// `#` are ignored so the file can document its own provenance. Each
/// remaining line is `<name>` or `<name> <downloads>` — the latter
/// populates the manifest's `downloads` column.
///
/// `jobs` bounds parallelism. The default of 8 is a politeness ceiling for
/// crates.io's static.crates.io endpoint; raise it if you have an
/// understanding with the registry.
///
/// Output structure under `out_dir`:
/// - `manifest.csv` — one row per fetched crate (header + name, version,
///   license, downloads, sha256), in input order.
/// - `<name>-<version>/` — extracted source tree per fetched crate, with
///   the corpus filters from `extract_opts` applied.
///
/// Per-crate skips (yanked, missing license, copyleft license, 404) are
/// recorded in the returned [`FetchSummary`] but never abort the run; only
/// transport failures or checksum mismatches surface as errors.
pub fn run_fetch(
    crates_list: &Path,
    out_dir: &Path,
    allowlist: &[&str],
    extract_opts: &cntrdct_corpus_fetch::ExtractOptions,
    jobs: usize,
    resume: bool,
    progress: FetchProgress,
) -> Result<FetchSummary, FetchRunError> {
    use cntrdct_corpus_fetch::{
        fetch_one, FetchOutcome, ReqwestClient, SkipReason, SparseIndexClient, TarballClient,
    };

    let (raw_entries, rank_source) = read_crate_list_with_provenance(crates_list)?;
    let license_hints = read_sidecar_licenses(crates_list)?;
    fs::create_dir_all(out_dir).map_err(|e| FetchRunError::Io {
        path: out_dir.to_path_buf(),
        source: e,
    })?;

    let manifest_path = out_dir.join("manifest.csv");
    let mut summary = FetchSummary {
        out_dir: out_dir.to_path_buf(),
        manifest: manifest_path.clone(),
        fetched: 0,
        skipped: 0,
        errors: 0,
        resume_skipped: 0,
        skips: Vec::new(),
    };

    let entries: Vec<CrateListEntry> = if resume {
        let already = cntrdct_corpus_fetch::read_manifest_names(&manifest_path).map_err(|e| {
            FetchRunError::Io {
                path: manifest_path.clone(),
                source: e,
            }
        })?;
        let mut kept = Vec::with_capacity(raw_entries.len());
        for entry in raw_entries {
            if already.contains(&entry.name) {
                emit_resume_skip(progress, &entry.name);
                summary.resume_skipped += 1;
            } else {
                kept.push(entry);
            }
        }
        kept
    } else {
        raw_entries
    };

    let sparse = SparseIndexClient::new(
        ReqwestClient::new().map_err(|e| FetchRunError::HttpInit(e.to_string()))?,
    );
    let tarball = TarballClient::new(
        ReqwestClient::new().map_err(|e| FetchRunError::HttpInit(e.to_string()))?,
    );

    // Per-crate work runs on the rayon pool. We collect outcomes back on the
    // main thread so the manifest serialises in input order — the manifest
    // file is append-only and not safe to write from multiple threads.
    type FetchResult = (
        CrateListEntry,
        Result<FetchOutcome, cntrdct_corpus_fetch::FetchError>,
    );
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs.max(1))
        .build()
        .map_err(|e| FetchRunError::ThreadPool(e.to_string()))?;
    let results: Vec<FetchResult> = pool.install(|| {
        use rayon::prelude::*;
        entries
            .par_iter()
            .map(|entry| {
                let hint = license_hints.get(&entry.name).map(|s| s.as_str());
                let r = fetch_one(
                    &sparse,
                    &tarball,
                    &entry.name,
                    out_dir,
                    allowlist,
                    extract_opts,
                    hint,
                );
                match &r {
                    Ok(FetchOutcome::Fetched { row, .. }) => {
                        emit_fetched(progress, &row.name, &row.version);
                    }
                    Ok(FetchOutcome::Skipped { name, reason }) => {
                        emit_skipped(progress, name, reason);
                    }
                    Err(e) => {
                        emit_error(progress, &entry.name, &format!("{e}"));
                    }
                }
                (entry.clone(), r)
            })
            .collect()
    });

    for (entry, r) in results {
        match r {
            Ok(FetchOutcome::Fetched { mut row, .. }) => {
                row.downloads = entry.downloads;
                cntrdct_corpus_fetch::append_row(&manifest_path, &row).map_err(|e| {
                    FetchRunError::Io {
                        path: manifest_path.clone(),
                        source: e,
                    }
                })?;
                summary.fetched += 1;
            }
            Ok(FetchOutcome::Skipped { name, reason }) => {
                let (reason_tag, spdx) = match reason {
                    SkipReason::LicenseRejected(s) => ("license_rejected", Some(s)),
                    SkipReason::LicenseMissing => ("license_missing", None),
                    SkipReason::NotFound => ("not_found", None),
                    SkipReason::AllVersionsYanked => ("all_versions_yanked", None),
                };
                summary.skipped += 1;
                summary.skips.push(FetchSkipRecord {
                    name,
                    reason: reason_tag,
                    spdx,
                });
            }
            Err(_) => {
                summary.errors += 1;
            }
        }
    }

    write_provenance(out_dir, &rank_source, &summary)?;

    Ok(summary)
}

fn write_provenance(
    out_dir: &Path,
    rank_source: &RankSource,
    summary: &FetchSummary,
) -> Result<(), FetchRunError> {
    let fetched_at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut provenance = serde_json::json!({
        "fetched_at_unix": fetched_at_unix,
        "cntrdct_corpus_fetch_version": env!("CARGO_PKG_VERSION"),
        "fetch_summary": {
            "fetched": summary.fetched,
            "skipped": summary.skipped,
            "errors": summary.errors,
            "resume_skipped": summary.resume_skipped,
        },
    });
    if !rank_source.is_empty() {
        provenance["rank_source"] =
            serde_json::to_value(rank_source).expect("RankSource serialises");
    }

    let path = out_dir.join("provenance.json");
    let body = serde_json::to_string_pretty(&provenance).expect("provenance JSON serialises");
    fs::write(&path, body).map_err(|e| FetchRunError::Io { path, source: e })
}

fn skip_reason_tag(reason: &cntrdct_corpus_fetch::SkipReason) -> &'static str {
    use cntrdct_corpus_fetch::SkipReason;
    match reason {
        SkipReason::LicenseRejected(_) => "license_rejected",
        SkipReason::LicenseMissing => "license_missing",
        SkipReason::NotFound => "not_found",
        SkipReason::AllVersionsYanked => "all_versions_yanked",
    }
}

fn skip_spdx(reason: &cntrdct_corpus_fetch::SkipReason) -> Option<&str> {
    use cntrdct_corpus_fetch::SkipReason;
    match reason {
        SkipReason::LicenseRejected(s) => Some(s.as_str()),
        _ => None,
    }
}

fn emit_fetched(progress: FetchProgress, name: &str, version: &str) {
    match progress {
        FetchProgress::Text => eprintln!("fetched: {name} {version}"),
        FetchProgress::NdJson => {
            let v = serde_json::json!({
                "event": "fetched",
                "name": name,
                "version": version,
            });
            eprintln!("{v}");
        }
    }
}

fn emit_skipped(progress: FetchProgress, name: &str, reason: &cntrdct_corpus_fetch::SkipReason) {
    let tag = skip_reason_tag(reason);
    let spdx = skip_spdx(reason);
    match progress {
        FetchProgress::Text => match spdx {
            Some(s) => eprintln!("skipped: {name} ({tag}: {s})"),
            None => eprintln!("skipped: {name} ({tag})"),
        },
        FetchProgress::NdJson => {
            let mut v = serde_json::json!({
                "event": "skipped",
                "name": name,
                "reason": tag,
            });
            if let Some(s) = spdx {
                v["spdx"] = serde_json::Value::String(s.to_string());
            }
            eprintln!("{v}");
        }
    }
}

fn emit_error(progress: FetchProgress, name: &str, message: &str) {
    match progress {
        FetchProgress::Text => eprintln!("error: {name}: {message}"),
        FetchProgress::NdJson => {
            let v = serde_json::json!({
                "event": "error",
                "name": name,
                "message": message,
            });
            eprintln!("{v}");
        }
    }
}

fn emit_resume_skip(progress: FetchProgress, name: &str) {
    match progress {
        FetchProgress::Text => eprintln!("resume-skip: {name}"),
        FetchProgress::NdJson => {
            let v = serde_json::json!({
                "event": "resume_skip",
                "name": name,
            });
            eprintln!("{v}");
        }
    }
}

/// Compute the top-N crates from a saved db-dump archive and emit a
/// crate-list file (or stdout) usable by [`run_fetch`].
pub fn run_rank(dump_path: &Path, top: usize, output: Option<&Path>) -> Result<(), FetchRunError> {
    let ranking = cntrdct_corpus_fetch::read_top_n_from_archive(dump_path, top)?;
    let metadata = cntrdct_corpus_fetch::read_metadata_from_archive(dump_path)?;
    let mut body = String::new();
    body.push_str("# generated by `cntrdct rank`\n");
    if let Some(ts) = &metadata.timestamp {
        body.push_str(&format!("# dump-timestamp: {ts}\n"));
    }
    if let Some(hash) = &metadata.commit_hash {
        body.push_str(&format!("# dump-commit-hash: {hash}\n"));
    }
    body.push_str("# columns: name downloads\n");
    for r in &ranking {
        body.push_str(&format!("{} {}\n", r.name, r.downloads));
    }
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| FetchRunError::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            fs::write(p, body).map_err(|e| FetchRunError::Io {
                path: p.to_path_buf(),
                source: e,
            })?;
            // Sidecar TSV: <output>.licenses.tsv. The crates.io sparse
            // index does not expose a license field, so `cntrdct fetch`
            // reads this file (when it sits next to the crate list) to
            // populate the license filter without round-tripping back to
            // the dump or the crates.io API.
            let mut licenses_tsv = String::from("name\tlicense\n");
            for r in &ranking {
                if let Some(lic) = &r.license {
                    licenses_tsv.push_str(&format!("{}\t{}\n", r.name, lic));
                }
            }
            let sidecar = sidecar_licenses_path(p);
            fs::write(&sidecar, licenses_tsv).map_err(|e| FetchRunError::Io {
                path: sidecar,
                source: e,
            })?;
        }
        None => {
            print!("{body}");
        }
    }
    Ok(())
}

/// Compute the sidecar licenses path next to a crate-list file.
/// `crates.txt` → `crates.txt.licenses.tsv`.
fn sidecar_licenses_path(crates_list: &Path) -> PathBuf {
    let mut s = crates_list.as_os_str().to_owned();
    s.push(".licenses.tsv");
    PathBuf::from(s)
}

/// Read a sibling `<crates_list>.licenses.tsv` if it exists. The TSV must
/// have a `name\tlicense` header; rows with empty license fields are
/// dropped. Missing sidecar is not an error — the map is empty and the
/// fetch loop falls back to whatever the sparse index returns (typically
/// nothing, which yields `LicenseMissing` skips).
fn read_sidecar_licenses(crates_list: &Path) -> Result<HashMap<String, String>, FetchRunError> {
    let path = sidecar_licenses_path(crates_list);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let body = fs::read_to_string(&path).map_err(|e| FetchRunError::ReadList {
        path: path.clone(),
        source: e,
    })?;
    let mut out = HashMap::new();
    for (idx, line) in body.lines().enumerate() {
        if idx == 0 && line.starts_with("name\t") {
            continue;
        }
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '\t');
        let name = match parts.next() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let license = match parts.next() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        out.insert(name, license);
    }
    Ok(out)
}

/// Optional rank-source pin extracted from a crate-list file's comment
/// header (`# dump-timestamp: ...`, `# dump-commit-hash: ...`). Both
/// fields are skipped during serialization when unset, so a manually
/// authored crate list produces an empty `rank_source` block in the
/// downstream `provenance.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct RankSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dump_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dump_commit_hash: Option<String>,
}

impl RankSource {
    fn is_empty(&self) -> bool {
        self.dump_timestamp.is_none() && self.dump_commit_hash.is_none()
    }
}

fn read_crate_list_with_provenance(
    path: &Path,
) -> Result<(Vec<CrateListEntry>, RankSource), FetchRunError> {
    let body = fs::read_to_string(path).map_err(|e| FetchRunError::ReadList {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut entries = Vec::new();
    let mut rank_source = RankSource::default();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix('#') {
            let comment = comment.trim();
            if let Some(v) = comment.strip_prefix("dump-timestamp:") {
                rank_source.dump_timestamp = Some(v.trim().to_string());
            } else if let Some(v) = comment.strip_prefix("dump-commit-hash:") {
                rank_source.dump_commit_hash = Some(v.trim().to_string());
            }
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        // SAFETY: `trimmed` is non-empty here, so the first token always exists.
        let name = parts.next().expect("non-empty line").to_string();
        let downloads = parts.next().and_then(|s| s.parse::<u64>().ok());
        entries.push(CrateListEntry { name, downloads });
    }
    Ok((entries, rank_source))
}

// ---------- File discovery ----------

/// Walk `path` and return every file whose extension maps to a supported
/// `Language` per `cntrdct_parsers::detect_language`. Files with unknown
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
