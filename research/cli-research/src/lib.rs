//! cntrdct-research library entry point.
//!
//! Research / empirical-study workflows extracted from the technical CLI in
//! the research-split refactor. Owns: corpus fetch, license filter,
//! clippy-harness, aggregate / overlap matrices, stratified sample, top-N
//! rank from a crates.io DB dump.
//!
//! Boundary: this crate has no dependency on `cntrdct-core`, the detectors,
//! the ranker, or any other technical workspace member. Research code that
//! ever needs core types must duplicate them rather than depend in.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

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
    #[error("--accept-arbitrary-code is required: cntrdct-research clippy compiles third-party Rust source via cargo, which executes build.rs scripts and proc macros. Run only in an isolated environment (container, VM, dedicated user).")]
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

// ---------- Overlap matrix (Phase 0 baseline) ----------

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OverlapRow {
    pub detector: String,
    pub clippy_lint: String,
    pub count: usize,
}

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

pub fn run_clippy_harness(
    manifest_path: &Path,
    out_dir: &Path,
    accept_arbitrary_code: bool,
) -> Result<ClippyHarnessSummary, ClippyHarnessError> {
    use cntrdct_corpus_fetch::{extract_filtered, ExtractOptions, ReqwestClient, TarballClient};

    if !accept_arbitrary_code {
        return Err(ClippyHarnessError::ConsentRequired);
    }

    eprintln!("WARNING: cntrdct-research clippy compiles third-party Rust source via cargo, which");
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AggregateRow {
    pub crate_dir: String,
    pub detector: String,
    pub count: usize,
}

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

/// Two-axis stratified random sample: detector × crate.
///
/// For each detector, group findings by the corpus crate directory
/// (extracted by stripping `corpus_root` from `finding.primary.file`).
/// From each (detector, crate) bucket, take up to `max_per_crate` at
/// random. If the per-detector concatenation exceeds `per_detector`,
/// down-sample to `per_detector`. All randomness is driven by a single
/// seeded `fastrand::Rng` iterated in `BTreeMap` (detector, crate)
/// order, so the output is reproducible for a fixed (input, seed, caps).
///
/// Findings whose `primary.file` does not resolve under `corpus_root`
/// are skipped (mirrors `run_aggregate`). Findings without
/// `detector_id` are grouped under `(unknown)` (mirrors `run_sample`).
pub fn run_stratified_sample(
    findings_path: &Path,
    corpus_root: &Path,
    per_detector: usize,
    max_per_crate: usize,
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

    let canonical_root = canonicalise_or(corpus_root);

    let mut buckets: BTreeMap<(String, String), Vec<serde_json::Value>> = BTreeMap::new();
    for f in findings {
        let detector = f
            .get("finding")
            .and_then(|v| v.get("detector_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)")
            .to_string();
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
        buckets.entry((detector, crate_dir)).or_default().push(f);
    }

    let mut rng = fastrand::Rng::with_seed(seed);
    let mut per_detector_pool: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for ((detector, _crate_dir), mut group) in buckets {
        rng.shuffle(&mut group);
        group.truncate(max_per_crate);
        per_detector_pool.entry(detector).or_default().extend(group);
    }

    let mut sample: Vec<serde_json::Value> = Vec::new();
    for (_detector, mut pool) in per_detector_pool {
        if pool.len() > per_detector {
            rng.shuffle(&mut pool);
            pool.truncate(per_detector);
        }
        sample.extend(pool);
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchSkipRecord {
    pub name: String,
    pub reason: &'static str,
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
    pub resume_skipped: usize,
    pub skips: Vec<FetchSkipRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateListEntry {
    pub name: String,
    pub downloads: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FetchProgress {
    #[default]
    Text,
    NdJson,
}

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

pub fn run_rank(dump_path: &Path, top: usize, output: Option<&Path>) -> Result<(), FetchRunError> {
    let ranking = cntrdct_corpus_fetch::read_top_n_from_archive(dump_path, top)?;
    let metadata = cntrdct_corpus_fetch::read_metadata_from_archive(dump_path)?;
    let mut body = String::new();
    body.push_str("# generated by `cntrdct-research rank`\n");
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

fn sidecar_licenses_path(crates_list: &Path) -> PathBuf {
    let mut s = crates_list.as_os_str().to_owned();
    s.push(".licenses.tsv");
    PathBuf::from(s)
}

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
        let name = parts.next().expect("non-empty line").to_string();
        let downloads = parts.next().and_then(|s| s.parse::<u64>().ok());
        entries.push(CrateListEntry { name, downloads });
    }
    Ok((entries, rank_source))
}
