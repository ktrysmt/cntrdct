//! Q-13 cross-model κ audit.
//!
//! Spec: `docs/spec/cross-model-kappa-v0.md`. Routes the same finding
//! set through every configured Layer 3 adjudicator and computes
//! pairwise Cohen's κ on the resulting verdicts per
//! `(detector_id, anomaly_class)` cell. Cells with κ < 0.6 (the lower
//! edge of the Landis & Koch (1977) "substantial agreement" band that
//! Wataoka et al. (2024) adopt) are flagged as low-reliability
//! adjudication regions.
//!
//! Per design constraint P3, the adjudicators are the only network-
//! reachable component invoked here. This module itself is allocation
//! only: pure math, pure JSON I/O against the supplied paths, no LLM
//! calls and no network.
//!
//! References:
//! - `wataoka-2024` — K. Wataoka, T. Takahashi, R. Ri, "Self-Preference
//!   Bias in LLM-as-a-Judge", arXiv:2410.21819, 2024.
//! - `zheng-neurips-2023` — L. Zheng et al., "Judging LLM-as-a-Judge
//!   with MT-Bench and Chatbot Arena", NeurIPS 36, 46595–46623, 2023.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::adjudicator::PromptDispatch;
use crate::core::{AdjudicationVerdict, AnomalyClass, DetectorError, LanguageCitationStatus};

// ---------- Constants ----------

/// Q-13: the Landis & Koch (1977) "substantial agreement" lower bound.
/// Cells whose worst pairwise κ falls below this value are flagged as
/// low-reliability adjudication regions in the audit report.
pub const SUBSTANTIAL_AGREEMENT_THRESHOLD: f64 = 0.6;

/// Q-13: cells with fewer than this many findings are reported with
/// `low_n: true` and excluded from `low_reliability` flagging — sparse
/// cells produce noisy κ estimates and the audit log should not blame
/// them for instability the sample size cannot rule out.
pub const MIN_N: usize = 5;

// ---------- Verdict alias ----------

/// Spec alias for [`AdjudicationVerdict`] used at κ call sites for
/// clarity. Three nominal classes: `LikelyTruePositive`,
/// `LikelyFalsePositive`, `Uncertain`.
pub type Verdict3 = AdjudicationVerdict;

// ---------- Cohen's κ ----------

/// Compute Cohen's κ for two equal-length sequences of three-class
/// verdicts.
///
/// Returns `None` when:
/// - the two slices have different lengths;
/// - either slice is empty;
/// - the expected agreement `p_e` collapses onto 1.0 (every observation
///   maps to the same class for both raters — κ is mathematically
///   undefined).
///
/// Otherwise returns the standard formula
/// `κ = (p_o - p_e) / (1 - p_e)` where `p_o` is observed agreement and
/// `p_e` is the chance-agreement floor under independence.
pub fn cohen_kappa(a: &[Verdict3], b: &[Verdict3]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let n = a.len() as f64;
    let mut row_a: [f64; 3] = [0.0; 3];
    let mut col_b: [f64; 3] = [0.0; 3];
    let mut agree: f64 = 0.0;
    for (va, vb) in a.iter().zip(b.iter()) {
        let ia = verdict_idx(*va) as usize;
        let ib = verdict_idx(*vb) as usize;
        row_a[ia] += 1.0;
        col_b[ib] += 1.0;
        if ia == ib {
            agree += 1.0;
        }
    }
    let p_o = agree / n;
    let mut p_e = 0.0;
    for k in 0..3 {
        p_e += (row_a[k] * col_b[k]) / (n * n);
    }
    if (1.0 - p_e).abs() < 1e-12 {
        return None;
    }
    Some((p_o - p_e) / (1.0 - p_e))
}

fn verdict_idx(v: Verdict3) -> u8 {
    match v {
        AdjudicationVerdict::LikelyTruePositive => 0,
        AdjudicationVerdict::LikelyFalsePositive => 1,
        AdjudicationVerdict::Uncertain => 2,
    }
}

// ---------- Audit input shapes ----------

/// Severity tag carried on an [`AuditInputFinding`]. Mirrors
/// [`crate::core::Severity`] but adds `Deserialize` without disturbing
/// the rest of the codebase's serialise-only Severity surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditInputSeverity {
    /// Lowest level; informational signal that does not warrant action.
    Info,
    /// Slightly higher than `Info`; flagged for awareness.
    Note,
    /// Likely defect; should usually be addressed.
    Warning,
    /// Definite defect; must be addressed.
    Error,
}

/// Source-code location carried on an [`AuditInputFinding`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditInputLocation {
    /// Path to the file containing the location.
    pub file: PathBuf,
    /// 1-based line number of the first character of the span.
    pub start_line: u32,
    /// 1-based column number of the first character of the span.
    pub start_col: u32,
    /// 1-based line number of the character immediately after the span.
    pub end_line: u32,
    /// 1-based column number of the character immediately after the span.
    pub end_col: u32,
}

/// Evidence payload carried on an [`AuditInputFinding`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditInputEvidence {
    /// Citation keys referenced by the detector for this finding.
    pub citation_keys: Vec<String>,
    /// Detector-defined raw evidence payload.
    #[serde(default)]
    pub raw: Value,
    /// Per-language citation grounding for this finding.
    pub language_citation_status: LanguageCitationStatus,
}

/// Detector finding shape accepted by the cross-model audit. Mirrors
/// [`crate::core::Finding`]'s wire shape but is deserialise-friendly:
/// `citation_keys` is `Vec<String>` rather than `Vec<&'static str>` so
/// no string leaks are required to reconstruct findings from disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditInputFinding {
    /// Identifier of the detector that produced this finding.
    pub detector_id: String,
    /// Primary location the finding refers to.
    pub primary: AuditInputLocation,
    /// Additional related locations (e.g., the sibling that drifted).
    #[serde(default)]
    pub related: Vec<AuditInputLocation>,
    /// Human-readable description of the finding.
    pub message: String,
    /// Detector-supplied severity.
    pub raw_severity: AuditInputSeverity,
    /// IEEE 1044-2009 anomaly classification supplied by the detector.
    pub anomaly_class: AnomalyClass,
    /// Supporting evidence (citations + opaque payload).
    pub evidence: AuditInputEvidence,
}

/// Ranked finding shape accepted by the cross-model audit. Tolerates
/// any extra fields a future `RankedFinding` may carry (serde ignores
/// unknown fields by default), so a tag bump that adds a new ranker
/// signal does not silently break the audit corpus contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditInputRanked {
    /// The underlying detector finding.
    pub finding: AuditInputFinding,
    /// Layer 2 calibrated posterior, if the corpus was produced under
    /// the calibrated ranker.
    #[serde(default)]
    pub posterior_tp: Option<f64>,
    /// Layer 2 lower bound, mirroring the calibrated ranker's output.
    #[serde(default)]
    pub wilson_lower: Option<f64>,
    /// Final ranking score used to order the input. Defaults to `0.0`
    /// when absent so a hand-authored fixture corpus does not need to
    /// pre-compute ranking metadata.
    #[serde(default)]
    pub rank_score: f64,
}

// ---------- Build prompt without RankedFinding ----------

/// Build the adjudication prompt directly from an [`AuditInputRanked`].
///
/// Mirrors [`crate::adjudicator::__sample_build_prompt`] format byte for
/// byte so the prompt content is identical between the standard scan
/// path and the cross-model audit. Pure / deterministic /
/// allocation-only.
pub fn build_audit_prompt(input: &AuditInputRanked) -> String {
    let f = &input.finding;
    let location = format!(
        "{}:{}",
        f.primary.file.to_string_lossy(),
        f.primary.start_line
    );
    let citations = if f.evidence.citation_keys.is_empty() {
        "(none)".to_string()
    } else {
        f.evidence.citation_keys.join(",")
    };
    let prior = match (input.posterior_tp, input.wilson_lower) {
        (Some(p), Some(w)) => format!("posterior_tp={:.4}, wilson_lower={:.4}", p, w),
        _ => "uncalibrated".to_string(),
    };
    let raw_pretty =
        serde_json::to_string_pretty(&f.evidence.raw).unwrap_or_else(|_| "{}".to_string());
    format!(
        "You are evaluating a static analysis finding from cntrdct. Decide whether it is a true bug or a false positive.\n\
         \n\
         DETECTOR: {detector}\n\
         MESSAGE: {message}\n\
         SEVERITY: {severity:?}\n\
         ANOMALY_CLASS: {anomaly:?}\n\
         LOCATION: {location}\n\
         CITATIONS: {citations}\n\
         STATISTICAL_PRIOR: {prior}\n\
         EVIDENCE_RAW:\n{raw_pretty}\n\
         \n\
         Respond with a single JSON object on one line, exactly this shape:\n\
         {{\"verdict\": \"LikelyTruePositive\"|\"LikelyFalsePositive\"|\"Uncertain\", \"confidence\": <0.0-1.0>, \"rationale\": \"<one to three sentences>\"}}\n",
        detector = f.detector_id,
        message = f.message,
        severity = f.raw_severity,
        anomaly = f.anomaly_class,
        location = location,
        citations = citations,
        prior = prior,
        raw_pretty = raw_pretty,
    )
}

// ---------- Audit report shapes ----------

/// Provenance status of one provider in the audit run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ProviderStatus {
    /// Provider invoked against its real production endpoint.
    Live,
    /// Provider invoked against a mock transport (PR-CI fixture path).
    Mocked,
    /// Provider was not invoked. The associated string explains why
    /// (e.g. `"no API key in environment"`); nightly runs surface this
    /// in the audit log so a missing key is visible without breaking
    /// the cadence.
    Skipped(String),
}

/// Provider record carried in the audit report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRecord {
    /// Stable provider id (`"anthropic"`, `"openai"`, `"gemini"`).
    pub provider_id: String,
    /// Model id passed in the wire-format body.
    pub model: String,
    /// Whether the provider was actually invoked.
    pub status: ProviderStatus,
}

/// One pairwise κ entry inside an [`AuditCell`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KappaEntry {
    /// Cohen's κ for this pair on this cell. `None` only when the cell
    /// is `degenerate` (one-class collapse) or otherwise undefined.
    pub kappa: Option<f64>,
    /// `true` iff `p_e` collapsed onto 1.0. The audit log surfaces this
    /// as a flag rather than a misleading numeric.
    pub degenerate: bool,
}

/// One `(detector_id, anomaly_class)` cell of the audit report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditCell {
    /// Detector that produced the findings in this cell.
    pub detector_id: String,
    /// IEEE 1044-2009 anomaly class shared by every finding in the cell.
    pub anomaly_class: AnomalyClass,
    /// Number of findings aggregated into this cell.
    pub n: usize,
    /// Pairwise κ keyed by alphabetised `<provider_a>-<provider_b>`.
    pub pairwise_kappa: BTreeMap<String, KappaEntry>,
    /// Smallest non-degenerate κ across pairs.
    pub min_kappa: Option<f64>,
    /// `true` iff `n >= MIN_N` AND `min_kappa < SUBSTANTIAL_AGREEMENT_THRESHOLD`.
    pub low_reliability: bool,
    /// `true` iff `n < MIN_N`. Cells flagged `low_n` are excluded from
    /// `low_reliability` regardless of the κ value.
    pub low_n: bool,
}

/// Worst-κ cell across the entire audit. Surfaced separately in the
/// report so a README badge can render the single most pessimistic
/// signal without re-scanning the cell list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorstCell {
    /// Detector hosting the worst κ.
    pub detector_id: String,
    /// Anomaly class of the worst-κ cell.
    pub anomaly_class: AnomalyClass,
    /// Pair label producing the worst κ.
    pub pair: String,
    /// κ value (for the badge text).
    pub kappa: f64,
}

/// Top-level audit report serialised under
/// `benchmarks/cross-model-kappa/<date>.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditReport {
    /// UTC date of the audit run (`YYYY-MM-DD`).
    pub date: String,
    /// ISO 8601 UTC timestamp of generation.
    pub generated_at: String,
    /// One record per provider declared at run time. Skipped providers
    /// are present so a missing API key is surfaced in the artefact.
    pub providers: Vec<ProviderRecord>,
    /// Per-cell audit results, sorted by `(detector_id, anomaly_class)`.
    pub cells: Vec<AuditCell>,
    /// Worst κ cell across the report, or `None` when no cell carries a
    /// non-degenerate κ at `n >= MIN_N`.
    pub worst_cell: Option<WorstCell>,
}

impl AuditReport {
    /// Pretty JSON, byte-stable across runs given identical inputs.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("AuditReport serialises cleanly")
    }
}

// ---------- Errors ----------

/// Errors produced by the cross-model κ audit pipeline.
#[derive(Debug, Error)]
pub enum AuditError {
    /// I/O failure while reading or writing an audit artefact.
    #[error("io error reading {path}: {source}")]
    Io {
        /// Path being read or written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Malformed JSONL row in the input corpus.
    #[error("parse error at {path}:{line}: {source}")]
    ParseLine {
        /// Path of the file being read.
        path: PathBuf,
        /// 1-based line number of the offending row.
        line: usize,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },
    /// Failed to parse the JSON-array form of the corpus.
    #[error("parse error: {0}")]
    Parse(String),
    /// Audit invoked on an empty corpus.
    #[error("empty corpus: nothing to audit")]
    EmptyCorpus,
    /// Live provider returned a `DetectorError` mid-dispatch.
    #[error("adjudicator error from {provider}: {source}")]
    Provider {
        /// Provider id (`"anthropic"` / `"openai"` / `"gemini"`).
        provider: String,
        /// Underlying detector / adjudicator error.
        #[source]
        source: DetectorError,
    },
    /// Audit requires at least two live providers to compute pairwise κ.
    #[error("at least two live providers required (got {0})")]
    InsufficientProviders(usize),
}

// ---------- Corpus loader ----------

/// Read an audit corpus from `path`. Accepts both newline-delimited
/// JSONL (one `RankedFinding` JSON object per line) and a single JSON
/// array (the shape `cntrdct scan --format json` produces). The first
/// non-whitespace character determines the parse path; blank lines are
/// skipped in the JSONL branch.
pub fn load_corpus(path: &Path) -> Result<Vec<AuditInputRanked>, AuditError> {
    let body = fs::read_to_string(path).map_err(|e| AuditError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let trimmed = body.trim_start();
    if trimmed.starts_with('[') {
        let parsed: Vec<AuditInputRanked> =
            serde_json::from_str(trimmed).map_err(|e| AuditError::Parse(e.to_string()))?;
        return Ok(parsed);
    }
    let mut out: Vec<AuditInputRanked> = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        let line_no = idx + 1;
        if line.trim().is_empty() {
            continue;
        }
        let parsed: AuditInputRanked =
            serde_json::from_str(line).map_err(|e| AuditError::ParseLine {
                path: path.to_path_buf(),
                line: line_no,
                source: e,
            })?;
        out.push(parsed);
    }
    Ok(out)
}

// ---------- Pair labels ----------

/// Sorted list of pair tuples for a slice of provider ids.
///
/// Each tuple is `(label, idx_a, idx_b)` where `label` is the
/// alphabetised `<lower>-<upper>` pair string and `idx_a` / `idx_b`
/// index into the original `providers` slice. The outer list is sorted
/// lexicographically by label so the audit JSON is byte-stable across
/// runs regardless of provider declaration order.
pub fn pair_labels(providers: &[&str]) -> Vec<(String, usize, usize)> {
    let mut sorted: Vec<(usize, &str)> =
        providers.iter().enumerate().map(|(i, s)| (i, *s)).collect();
    sorted.sort_by(|a, b| a.1.cmp(b.1));
    let mut out: Vec<(String, usize, usize)> = Vec::new();
    for i in 0..sorted.len() {
        for j in (i + 1)..sorted.len() {
            let (idx_a, name_a) = sorted[i];
            let (idx_b, name_b) = sorted[j];
            out.push((format!("{}-{}", name_a, name_b), idx_a, idx_b));
        }
    }
    out
}

// ---------- Pure aggregation ----------

/// Aggregate verdict matrices into per-cell κ summaries.
///
/// `verdict_matrix[i]` is the verdict vector produced by provider
/// `provider_ids[i]` over `inputs`; every entry must have length
/// `inputs.len()`. Returns the per-cell report sorted by
/// `(detector_id, anomaly_class)` and the worst-κ cell across all
/// cells (excluding `low_n` cells from worst-κ selection).
pub fn aggregate(
    inputs: &[AuditInputRanked],
    provider_ids: &[&str],
    verdict_matrix: &[Vec<Verdict3>],
) -> AuditCellSummary {
    type CellKey = (String, AnomalyClass);
    let mut by_cell: BTreeMap<CellKey, Vec<usize>> = BTreeMap::new();
    for (idx, row) in inputs.iter().enumerate() {
        let key = (row.finding.detector_id.clone(), row.finding.anomaly_class);
        by_cell.entry(key).or_default().push(idx);
    }

    let pairs = pair_labels(provider_ids);
    let mut cells: Vec<AuditCell> = Vec::new();
    let mut worst: Option<WorstCell> = None;

    for ((det_id, cls), indices) in by_cell.iter() {
        let n = indices.len();
        let mut pairwise: BTreeMap<String, KappaEntry> = BTreeMap::new();
        let mut min_kappa: Option<f64> = None;

        for (label, idx_a, idx_b) in &pairs {
            let mut va: Vec<Verdict3> = Vec::with_capacity(n);
            let mut vb: Vec<Verdict3> = Vec::with_capacity(n);
            for &row_idx in indices {
                va.push(verdict_matrix[*idx_a][row_idx]);
                vb.push(verdict_matrix[*idx_b][row_idx]);
            }
            let entry = match cohen_kappa(&va, &vb) {
                Some(k) => KappaEntry {
                    kappa: Some(k),
                    degenerate: false,
                },
                None => KappaEntry {
                    kappa: None,
                    degenerate: true,
                },
            };
            if let Some(k) = entry.kappa {
                let new_min = match min_kappa {
                    Some(m) => m.min(k),
                    None => k,
                };
                min_kappa = Some(new_min);
                let beats_worst = match &worst {
                    Some(w) => k < w.kappa,
                    None => true,
                };
                if beats_worst && n >= MIN_N {
                    worst = Some(WorstCell {
                        detector_id: det_id.clone(),
                        anomaly_class: *cls,
                        pair: label.clone(),
                        kappa: k,
                    });
                }
            }
            pairwise.insert(label.clone(), entry);
        }

        let low_n = n < MIN_N;
        let low_reliability = !low_n
            && match min_kappa {
                Some(k) => k < SUBSTANTIAL_AGREEMENT_THRESHOLD,
                None => false,
            };

        cells.push(AuditCell {
            detector_id: det_id.clone(),
            anomaly_class: *cls,
            n,
            pairwise_kappa: pairwise,
            min_kappa,
            low_reliability,
            low_n,
        });
    }

    AuditCellSummary {
        cells,
        worst_cell: worst,
    }
}

/// Output of [`aggregate`]. Exposed so tests can inspect both fields
/// without rebuilding the surrounding [`AuditReport`].
#[derive(Debug, Clone, PartialEq)]
pub struct AuditCellSummary {
    /// Per-cell results, sorted by `(detector_id, anomaly_class)`.
    pub cells: Vec<AuditCell>,
    /// Worst non-degenerate κ across cells with `n >= MIN_N`.
    pub worst_cell: Option<WorstCell>,
}

// ---------- Orchestrator ----------

/// One configured provider for [`run_audit`]. `adjudicator` is `Some`
/// for live and mocked providers, `None` for skipped ones; the
/// orchestrator records the status verbatim into the audit log.
pub struct ProviderHandle {
    /// Stable provider id (`"anthropic"`, `"openai"`, `"gemini"`).
    pub provider_id: String,
    /// Model id surfaced in the audit report.
    pub model: String,
    /// Live or mocked dispatch surface; `None` for skipped providers.
    pub adjudicator: Option<Box<dyn PromptDispatch>>,
    /// Provenance status to record in the report.
    pub status: ProviderStatus,
}

/// Run the full Q-13 audit: build prompts, dispatch through every live
/// provider, aggregate per-cell pairwise κ.
///
/// `date` and `generated_at` are accepted as parameters so tests can
/// pin them; the CLI computes them from `SystemTime::now()` via
/// [`current_utc_date`] / [`current_iso8601_utc`].
pub fn run_audit(
    date: String,
    generated_at: String,
    providers: Vec<ProviderHandle>,
    inputs: Vec<AuditInputRanked>,
) -> Result<AuditReport, AuditError> {
    if inputs.is_empty() {
        return Err(AuditError::EmptyCorpus);
    }
    let live_count = providers.iter().filter(|p| p.adjudicator.is_some()).count();
    if live_count < 2 {
        return Err(AuditError::InsufficientProviders(live_count));
    }

    // Build prompts once (one per input).
    let prompts: Vec<String> = inputs.iter().map(build_audit_prompt).collect();

    let mut live_ids: Vec<String> = Vec::new();
    let mut live_verdicts: Vec<Vec<Verdict3>> = Vec::new();
    let mut provider_records: Vec<ProviderRecord> = Vec::new();
    for handle in &providers {
        if let Some(adj) = handle.adjudicator.as_ref() {
            let mut row: Vec<Verdict3> = Vec::with_capacity(prompts.len());
            for prompt in &prompts {
                let res = adj.dispatch(prompt).map_err(|e| AuditError::Provider {
                    provider: handle.provider_id.clone(),
                    source: e,
                })?;
                row.push(res.verdict);
            }
            live_ids.push(handle.provider_id.clone());
            live_verdicts.push(row);
        }
        provider_records.push(ProviderRecord {
            provider_id: handle.provider_id.clone(),
            model: handle.model.clone(),
            status: handle.status.clone(),
        });
    }

    let live_id_refs: Vec<&str> = live_ids.iter().map(|s| s.as_str()).collect();
    let summary = aggregate(&inputs, &live_id_refs, &live_verdicts);

    Ok(AuditReport {
        date,
        generated_at,
        providers: provider_records,
        cells: summary.cells,
        worst_cell: summary.worst_cell,
    })
}

/// Write `report` as pretty JSON to `path`, creating parent directories
/// as needed.
pub fn write_report(path: &Path, report: &AuditReport) -> Result<(), AuditError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| AuditError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }
    fs::write(path, report.to_json_pretty()).map_err(|e| AuditError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(())
}

// ---------- Date helpers ----------

/// UTC date `YYYY-MM-DD` of "now" via `SystemTime`. Standard library
/// only (no `chrono` / `time` dep).
pub fn current_utc_date() -> String {
    let secs = now_secs();
    format_utc_date(secs)
}

/// ISO 8601 UTC timestamp `YYYY-MM-DDTHH:MM:SSZ` of "now". Standard
/// library only.
pub fn current_iso8601_utc() -> String {
    let secs = now_secs();
    let (y, mo, d) = ymd_from_epoch_secs(secs);
    let day_secs = secs.rem_euclid(86400);
    let h = (day_secs / 3600) as u32;
    let m = ((day_secs % 3600) / 60) as u32;
    let s = (day_secs % 60) as u32;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, s)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Format a Unix timestamp (UTC) as `YYYY-MM-DD`.
pub fn format_utc_date(secs: i64) -> String {
    let (y, mo, d) = ymd_from_epoch_secs(secs);
    format!("{:04}-{:02}-{:02}", y, mo, d)
}

/// Howard Hinnant's "civil_from_days" algorithm — proleptic Gregorian.
/// `secs` is a Unix timestamp; the returned tuple is `(year, month, day)`
/// with `month` in `1..=12` and `day` in `1..=31`. Verified against
/// known dates in the unit tests below.
fn ymd_from_epoch_secs(secs: i64) -> (i32, u32, u32) {
    let z = secs.div_euclid(86400) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 {
        (mp + 3) as u32
    } else {
        (mp - 9) as u32
    };
    let y_adj = if m <= 2 { y + 1 } else { y };
    (y_adj as i32, m, d)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn vt() -> Verdict3 {
        AdjudicationVerdict::LikelyTruePositive
    }
    fn vf() -> Verdict3 {
        AdjudicationVerdict::LikelyFalsePositive
    }
    fn vu() -> Verdict3 {
        AdjudicationVerdict::Uncertain
    }

    // ---- cohen_kappa ----

    #[test]
    fn kappa_rejects_mismatched_lengths() {
        assert!(cohen_kappa(&[vt(), vf()], &[vt()]).is_none());
    }

    #[test]
    fn kappa_rejects_empty_input() {
        assert!(cohen_kappa(&[], &[]).is_none());
    }

    #[test]
    fn kappa_perfect_agreement_is_one() {
        let a = vec![vt(), vf(), vu(), vt(), vf()];
        let k = cohen_kappa(&a, &a).unwrap();
        assert!((k - 1.0).abs() < 1e-12, "expected 1.0, got {}", k);
    }

    #[test]
    fn kappa_single_class_is_degenerate_returns_none() {
        // Both raters always say "Uncertain" — p_e = 1.0, κ undefined.
        let a = vec![vu(); 8];
        let b = vec![vu(); 8];
        assert!(cohen_kappa(&a, &b).is_none());
    }

    #[test]
    fn kappa_perfect_disagreement_is_negative() {
        // Two-class sequence with raters perfectly inverted: κ = -1.
        let a = vec![vt(), vf(), vt(), vf()];
        let b = vec![vf(), vt(), vf(), vt()];
        let k = cohen_kappa(&a, &b).unwrap();
        assert!((k + 1.0).abs() < 1e-12, "expected -1.0, got {}", k);
    }

    #[test]
    fn kappa_chance_agreement_is_zero() {
        // Two-class sequence where p_o equals p_e by construction.
        // a, b have identical marginals (50/50) but agreement only at
        // the chance rate of 0.5: κ = (0.5 - 0.5) / (1 - 0.5) = 0.
        let a = vec![vt(), vt(), vf(), vf()];
        let b = vec![vt(), vf(), vt(), vf()];
        let k = cohen_kappa(&a, &b).unwrap();
        assert!(k.abs() < 1e-12, "expected ~0.0, got {}", k);
    }

    #[test]
    fn kappa_three_class_intermediate() {
        // Cohen 1960 §3 worked example shape: 8 of 10 agree across
        // three classes. p_o = 0.8, p_e ≈ 0.34, κ ≈ 0.697.
        let a = vec![vt(), vt(), vt(), vt(), vf(), vf(), vf(), vu(), vu(), vu()];
        let b = vec![vt(), vt(), vt(), vt(), vf(), vf(), vu(), vu(), vu(), vt()];
        let k = cohen_kappa(&a, &b).unwrap();
        assert!(
            k > 0.6 && k < 0.8,
            "expected substantial agreement around 0.7, got {}",
            k
        );
    }

    // ---- pair_labels ----

    #[test]
    fn pair_labels_alphabetises_regardless_of_input_order() {
        let labels = pair_labels(&["openai", "anthropic", "gemini"]);
        let only_strings: Vec<String> = labels.iter().map(|(s, _, _)| s.clone()).collect();
        assert_eq!(
            only_strings,
            vec![
                "anthropic-gemini".to_string(),
                "anthropic-openai".to_string(),
                "gemini-openai".to_string(),
            ]
        );
    }

    // ---- AuditInputRanked deserialization ----

    #[test]
    fn audit_input_ignores_unknown_top_level_fields() {
        // RankedFinding may grow new fields (Q-11 added prior_method);
        // the audit corpus reader must not break when consumed against
        // an output produced under a newer schema.
        let json = r#"{
            "finding": {
                "detector_id": "clone-drift",
                "primary": {"file":"a.rs","start_line":1,"start_col":1,"end_line":2,"end_col":1},
                "related": [],
                "message": "msg",
                "raw_severity": "Warning",
                "anomaly_class": "Logic",
                "evidence": {
                    "citation_keys": ["k1"],
                    "raw": {"x": 1},
                    "language_citation_status": "Confirmed"
                }
            },
            "posterior_tp": 0.5,
            "wilson_lower": 0.3,
            "prior_method": "wilson",
            "rank_score": 1.2,
            "adjudication": {"verdict":"Uncertain","confidence":0.5,"rationale":"r"}
        }"#;
        let parsed: AuditInputRanked = serde_json::from_str(json).expect("parses");
        assert_eq!(parsed.finding.detector_id, "clone-drift");
        assert_eq!(parsed.posterior_tp, Some(0.5));
        assert_eq!(parsed.rank_score, 1.2);
    }

    #[test]
    fn audit_input_defaults_optional_ranker_metadata_to_none() {
        // Hand-authored fixtures may omit ranker metadata. The default
        // for posterior_tp / wilson_lower must be None and rank_score 0.0
        // so a minimal corpus still drives the audit pipeline.
        let json = r#"{
            "finding": {
                "detector_id": "arg-swap",
                "primary": {"file":"x.rs","start_line":10,"start_col":1,"end_line":11,"end_col":1},
                "related": [],
                "message": "m",
                "raw_severity": "Note",
                "anomaly_class": "Interface",
                "evidence": {
                    "citation_keys": [],
                    "language_citation_status": "Unconfirmed"
                }
            }
        }"#;
        let parsed: AuditInputRanked = serde_json::from_str(json).expect("parses");
        assert_eq!(parsed.posterior_tp, None);
        assert_eq!(parsed.wilson_lower, None);
        assert_eq!(parsed.rank_score, 0.0);
    }

    // ---- aggregate ----

    fn make_input(detector_id: &str, cls: AnomalyClass) -> AuditInputRanked {
        AuditInputRanked {
            finding: AuditInputFinding {
                detector_id: detector_id.to_string(),
                primary: AuditInputLocation {
                    file: PathBuf::from("a.rs"),
                    start_line: 1,
                    start_col: 1,
                    end_line: 2,
                    end_col: 1,
                },
                related: vec![],
                message: "msg".to_string(),
                raw_severity: AuditInputSeverity::Warning,
                anomaly_class: cls,
                evidence: AuditInputEvidence {
                    citation_keys: vec!["k".to_string()],
                    raw: serde_json::json!({}),
                    language_citation_status: LanguageCitationStatus::Confirmed,
                },
            },
            posterior_tp: None,
            wilson_lower: None,
            rank_score: 0.0,
        }
    }

    #[test]
    fn aggregate_groups_by_cell_and_ignores_low_n_for_worst_kappa() {
        // 6 inputs in clone-drift:Logic (>= MIN_N), 2 inputs in
        // arg-swap:Interface (< MIN_N). Even if the small cell has the
        // worst κ, worst_cell must point at the well-populated one.
        let inputs: Vec<AuditInputRanked> = vec![
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("clone-drift", AnomalyClass::Logic),
            make_input("arg-swap", AnomalyClass::Interface),
            make_input("arg-swap", AnomalyClass::Interface),
        ];
        // Two providers; provider B is perfectly inverted on the first
        // (large) cell but matches A on the second cell.
        let a: Vec<Verdict3> = vec![vt(), vt(), vf(), vf(), vt(), vf(), vt(), vf()];
        let b: Vec<Verdict3> = vec![vf(), vf(), vt(), vt(), vf(), vt(), vt(), vf()];
        let summary = aggregate(&inputs, &["alpha", "beta"], &[a, b]);
        assert_eq!(summary.cells.len(), 2);

        // arg-swap cell is low_n -- not flagged low_reliability and
        // never the worst cell.
        let small = summary
            .cells
            .iter()
            .find(|c| c.detector_id == "arg-swap")
            .unwrap();
        assert!(small.low_n);
        assert!(!small.low_reliability);

        let big = summary
            .cells
            .iter()
            .find(|c| c.detector_id == "clone-drift")
            .unwrap();
        assert_eq!(big.n, 6);
        assert!(
            big.low_reliability,
            "perfect inversion must flag low_reliability"
        );

        let worst = summary.worst_cell.expect("worst_cell present");
        assert_eq!(worst.detector_id, "clone-drift");
        assert_eq!(worst.pair, "alpha-beta");
    }

    #[test]
    fn aggregate_marks_one_class_collapse_as_degenerate() {
        let inputs: Vec<AuditInputRanked> = (0..5)
            .map(|_| make_input("clone-drift", AnomalyClass::Logic))
            .collect();
        // Both providers always emit Uncertain — degenerate cell.
        let a: Vec<Verdict3> = vec![vu(); 5];
        let b: Vec<Verdict3> = vec![vu(); 5];
        let summary = aggregate(&inputs, &["alpha", "beta"], &[a, b]);
        assert_eq!(summary.cells.len(), 1);
        let cell = &summary.cells[0];
        let entry = &cell.pairwise_kappa["alpha-beta"];
        assert!(entry.degenerate);
        assert!(entry.kappa.is_none());
        assert!(cell.min_kappa.is_none());
        assert!(!cell.low_reliability);
        assert!(summary.worst_cell.is_none());
    }

    // ---- date helpers ----

    #[test]
    fn ymd_from_epoch_secs_known_dates() {
        // 1970-01-01T00:00:00Z
        assert_eq!(ymd_from_epoch_secs(0), (1970, 1, 1));
        // 2000-01-01T00:00:00Z = 946_684_800
        assert_eq!(ymd_from_epoch_secs(946_684_800), (2000, 1, 1));
        // 2026-05-11T00:00:00Z = 1778457600
        assert_eq!(ymd_from_epoch_secs(1_778_457_600), (2026, 5, 11));
    }

    #[test]
    fn format_utc_date_known_value() {
        assert_eq!(format_utc_date(1_778_457_600), "2026-05-11");
    }

    // ---- ProviderStatus serde ----

    #[test]
    fn provider_status_serialises_with_kind_and_optional_detail() {
        let live = serde_json::to_string(&ProviderStatus::Live).unwrap();
        assert_eq!(live, r#"{"kind":"live"}"#);
        let mocked = serde_json::to_string(&ProviderStatus::Mocked).unwrap();
        assert_eq!(mocked, r#"{"kind":"mocked"}"#);
        let skipped =
            serde_json::to_string(&ProviderStatus::Skipped("no key".to_string())).unwrap();
        assert_eq!(skipped, r#"{"kind":"skipped","detail":"no key"}"#);
    }

    // ---- run_audit smoke ----

    struct CannedAdjudicator {
        verdicts: std::sync::Mutex<std::collections::VecDeque<Verdict3>>,
        provider_id: &'static str,
        model: String,
    }

    impl CannedAdjudicator {
        fn new(provider_id: &'static str, model: &str, verdicts: Vec<Verdict3>) -> Self {
            Self {
                verdicts: std::sync::Mutex::new(verdicts.into_iter().collect()),
                provider_id,
                model: model.to_string(),
            }
        }
    }

    impl PromptDispatch for CannedAdjudicator {
        fn provider_id(&self) -> &'static str {
            self.provider_id
        }
        fn model(&self) -> &str {
            &self.model
        }
        fn dispatch(
            &self,
            _prompt: &str,
        ) -> Result<crate::core::AdjudicationResult, DetectorError> {
            let v = self.verdicts.lock().unwrap().pop_front().unwrap_or(vu());
            Ok(crate::core::AdjudicationResult {
                verdict: v,
                confidence: 0.5,
                rationale: "canned".to_string(),
                calibration_tag: None,
                calibrated_confidence: None,
            })
        }
    }

    #[test]
    fn run_audit_aggregates_canned_verdicts_into_report() {
        let inputs: Vec<AuditInputRanked> = (0..6)
            .map(|_| make_input("clone-drift", AnomalyClass::Logic))
            .collect();
        let providers = vec![
            ProviderHandle {
                provider_id: "alpha".to_string(),
                model: "alpha-model".to_string(),
                adjudicator: Some(Box::new(CannedAdjudicator::new(
                    "alpha",
                    "alpha-model",
                    vec![vt(), vt(), vf(), vt(), vt(), vf()],
                ))),
                status: ProviderStatus::Mocked,
            },
            ProviderHandle {
                provider_id: "beta".to_string(),
                model: "beta-model".to_string(),
                adjudicator: Some(Box::new(CannedAdjudicator::new(
                    "beta",
                    "beta-model",
                    vec![vt(), vt(), vf(), vt(), vt(), vf()],
                ))),
                status: ProviderStatus::Mocked,
            },
        ];
        let report = run_audit(
            "2026-05-11".to_string(),
            "2026-05-11T00:00:00Z".to_string(),
            providers,
            inputs,
        )
        .expect("audit");
        assert_eq!(report.cells.len(), 1);
        let cell = &report.cells[0];
        assert_eq!(cell.n, 6);
        let entry = &cell.pairwise_kappa["alpha-beta"];
        assert!(
            (entry.kappa.unwrap() - 1.0).abs() < 1e-9,
            "perfect agreement on canned verdicts must yield κ=1, got {:?}",
            entry.kappa
        );
        assert!(!cell.low_reliability);
    }

    #[test]
    fn run_audit_rejects_empty_corpus() {
        let providers = vec![ProviderHandle {
            provider_id: "alpha".to_string(),
            model: "m".to_string(),
            adjudicator: Some(Box::new(CannedAdjudicator::new("alpha", "m", vec![]))),
            status: ProviderStatus::Mocked,
        }];
        let err = run_audit(
            "2026-05-11".to_string(),
            "2026-05-11T00:00:00Z".to_string(),
            providers,
            vec![],
        )
        .unwrap_err();
        assert!(matches!(err, AuditError::EmptyCorpus));
    }

    #[test]
    fn run_audit_rejects_fewer_than_two_live_providers() {
        let inputs = vec![make_input("clone-drift", AnomalyClass::Logic)];
        let providers = vec![ProviderHandle {
            provider_id: "alpha".to_string(),
            model: "m".to_string(),
            adjudicator: Some(Box::new(CannedAdjudicator::new("alpha", "m", vec![vt()]))),
            status: ProviderStatus::Mocked,
        }];
        let err = run_audit(
            "2026-05-11".to_string(),
            "2026-05-11T00:00:00Z".to_string(),
            providers,
            inputs,
        )
        .unwrap_err();
        assert!(matches!(err, AuditError::InsufficientProviders(1)));
    }

    #[test]
    fn audit_report_round_trips_through_json() {
        let inputs: Vec<AuditInputRanked> = (0..6)
            .map(|_| make_input("clone-drift", AnomalyClass::Logic))
            .collect();
        let providers = vec![
            ProviderHandle {
                provider_id: "alpha".to_string(),
                model: "alpha-model".to_string(),
                adjudicator: Some(Box::new(CannedAdjudicator::new(
                    "alpha",
                    "alpha-model",
                    vec![vt(), vt(), vf(), vt(), vt(), vf()],
                ))),
                status: ProviderStatus::Mocked,
            },
            ProviderHandle {
                provider_id: "beta".to_string(),
                model: "beta-model".to_string(),
                adjudicator: Some(Box::new(CannedAdjudicator::new(
                    "beta",
                    "beta-model",
                    vec![vf(), vf(), vt(), vf(), vf(), vt()],
                ))),
                status: ProviderStatus::Mocked,
            },
        ];
        let report = run_audit(
            "2026-05-11".to_string(),
            "2026-05-11T00:00:00Z".to_string(),
            providers,
            inputs,
        )
        .expect("audit");
        let body = report.to_json_pretty();
        let restored: AuditReport = serde_json::from_str(&body).expect("round-trip");
        assert_eq!(restored, report);
    }
}
