//! cntrdct recall-audit harness — Q-14.
//!
//! Spec: `docs/spec/recall-audit-v0.md`.
//!
//! Counters the labeller-bias loop in which cntrdct's priors are fit
//! against corpora that cntrdct labelled itself. The audit-recall
//! harness consumes a separate `benchmarks/audit-corpus/` whose
//! `expected` entries cite externally-sourced ground truth (CVEs, OSV
//! advisories, Semgrep / CodeQL / Clippy rule IDs, paper-appendix
//! anomaly sets) and reports per-detector recall upper bounds without
//! feeding back into the priors loop.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::Finding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSource {
    pub kind: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExpectedFinding {
    pub detector_id: String,
    pub line: u32,
    pub external_source: ExternalSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditManifestEntry {
    pub file: PathBuf,
    pub expected: Vec<AuditExpectedFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AuditManifest {
    pub entries: Vec<AuditManifestEntry>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SourceTally {
    pub tp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DetectorRecall {
    pub tp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub recall_upper_bound: f64,
    pub source_breakdown: BTreeMap<String, SourceTally>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RecallAuditReport {
    pub per_detector: BTreeMap<String, DetectorRecall>,
    pub overall: DetectorRecall,
    pub corpus_size: u32,
    pub expected_total: u32,
    pub sources: BTreeMap<String, u32>,
}

#[derive(Debug, Error)]
pub enum RecallAuditError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error at line {line}: {source}")]
    Parse {
        line: u32,
        #[source]
        source: serde_json::Error,
    },
    #[error("missing source file referenced by audit manifest: {0}")]
    MissingSource(PathBuf),
}

pub fn load_audit_manifest(path: &Path) -> Result<AuditManifest, RecallAuditError> {
    let body = fs::read_to_string(path).map_err(|e| RecallAuditError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut entries = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let line_no = (i + 1) as u32;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let entry: AuditManifestEntry =
            serde_json::from_str(trimmed).map_err(|e| RecallAuditError::Parse {
                line: line_no,
                source: e,
            })?;
        entries.push(entry);
    }
    Ok(AuditManifest { entries })
}

/// Per-(detector, file, line) key reused for matching. The kind string
/// is carried alongside so source-breakdown aggregation does not
/// require a second walk over the manifest.
type ExpectedKey = (String, PathBuf, u32, String);
type ActualKey = (String, PathBuf, u32);

pub fn audit_recall(
    manifest: &AuditManifest,
    actual: &[Finding],
    corpus_dir: &Path,
) -> RecallAuditReport {
    let expected = expected_index(manifest);
    let actual_keys = actual_index(actual, corpus_dir);

    let mut detector_ids: std::collections::BTreeSet<String> =
        expected.iter().map(|(d, _, _, _)| d.clone()).collect();
    // Detectors that fired but had zero expected entries still get an
    // entry (with tp = fn = 0 → recall_upper_bound = 0.0) so the
    // report makes the gap visible rather than silently dropping the
    // detector. This mirrors eval-v0 §F4's treatment.
    for (d, _, _) in &actual_keys {
        detector_ids.insert(d.clone());
    }

    let mut per_detector: BTreeMap<String, DetectorRecall> = BTreeMap::new();
    let mut sources: BTreeMap<String, u32> = BTreeMap::new();
    for (_, _, _, kind) in &expected {
        *sources.entry(kind.clone()).or_insert(0) += 1;
    }

    for det in &detector_ids {
        let exp_for_det: Vec<&ExpectedKey> =
            expected.iter().filter(|(d, _, _, _)| d == det).collect();
        let act_for_det: Vec<&ActualKey> =
            actual_keys.iter().filter(|(d, _, _)| d == det).collect();
        per_detector.insert(det.clone(), per_detector_recall(&exp_for_det, &act_for_det));
    }

    let overall = aggregate_overall(&per_detector);

    RecallAuditReport {
        per_detector,
        overall,
        corpus_size: manifest.entries.len() as u32,
        expected_total: expected.len() as u32,
        sources,
    }
}

fn expected_index(manifest: &AuditManifest) -> Vec<ExpectedKey> {
    let mut out = Vec::new();
    for entry in &manifest.entries {
        for exp in &entry.expected {
            out.push((
                exp.detector_id.clone(),
                entry.file.clone(),
                exp.line,
                exp.external_source.kind.clone(),
            ));
        }
    }
    out
}

fn actual_index(findings: &[Finding], corpus_dir: &Path) -> Vec<ActualKey> {
    findings
        .iter()
        .map(|f| {
            let rel = f
                .primary
                .file
                .strip_prefix(corpus_dir)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| f.primary.file.clone());
            (f.detector_id.clone(), rel, f.primary.start_line)
        })
        .collect()
}

fn per_detector_recall(expected: &[&ExpectedKey], actual: &[&ActualKey]) -> DetectorRecall {
    let mut consumed: Vec<bool> = vec![false; expected.len()];
    let mut tp: u32 = 0;

    for a in actual {
        for (i, e) in expected.iter().enumerate() {
            if consumed[i] {
                continue;
            }
            if e.0 == a.0 && e.1 == a.1 && e.2 == a.2 {
                consumed[i] = true;
                tp += 1;
                break;
            }
        }
    }

    let mut source_breakdown: BTreeMap<String, SourceTally> = BTreeMap::new();
    for (i, e) in expected.iter().enumerate() {
        let entry = source_breakdown.entry(e.3.clone()).or_default();
        if consumed[i] {
            entry.tp += 1;
        } else {
            entry.fn_ += 1;
        }
    }

    let fn_ = consumed.iter().filter(|c| !**c).count() as u32;
    DetectorRecall {
        tp,
        fn_,
        recall_upper_bound: recall_or_zero(tp, fn_),
        source_breakdown,
    }
}

fn aggregate_overall(per_detector: &BTreeMap<String, DetectorRecall>) -> DetectorRecall {
    let tp: u32 = per_detector.values().map(|d| d.tp).sum();
    let fn_: u32 = per_detector.values().map(|d| d.fn_).sum();
    let mut source_breakdown: BTreeMap<String, SourceTally> = BTreeMap::new();
    for d in per_detector.values() {
        for (kind, tally) in &d.source_breakdown {
            let entry = source_breakdown.entry(kind.clone()).or_default();
            entry.tp += tally.tp;
            entry.fn_ += tally.fn_;
        }
    }
    DetectorRecall {
        tp,
        fn_,
        recall_upper_bound: recall_or_zero(tp, fn_),
        source_breakdown,
    }
}

fn recall_or_zero(tp: u32, fn_: u32) -> f64 {
    if tp + fn_ == 0 {
        0.0
    } else {
        tp as f64 / (tp + fn_) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(kind: &str, ref_: &str, url: &str) -> ExternalSource {
        ExternalSource {
            kind: kind.into(),
            ref_: ref_.into(),
            url: url.into(),
        }
    }

    fn entry(file: &str, expected: Vec<AuditExpectedFinding>) -> AuditManifestEntry {
        AuditManifestEntry {
            file: PathBuf::from(file),
            expected,
            source: None,
            license: None,
            sha256: None,
        }
    }

    fn finding(detector: &str, file: &str, line: u32) -> Finding {
        use crate::core::{AnomalyClass, Evidence, LanguageCitationStatus, Location, Severity};
        Finding {
            detector_id: detector.into(),
            primary: Location {
                file: PathBuf::from(file),
                start_line: line,
                end_line: line,
                start_col: 1,
                end_col: 1,
            },
            related: vec![],
            message: "synthetic".into(),
            raw_severity: Severity::Note,
            anomaly_class: AnomalyClass::Logic,
            evidence: Evidence {
                citation_keys: vec![],
                raw: serde_json::json!({}),
                language_citation_status: LanguageCitationStatus::Confirmed,
            },
        }
    }

    #[test]
    fn recall_one_when_all_expected_caught() {
        let manifest = AuditManifest {
            entries: vec![entry(
                "files/a.rs",
                vec![AuditExpectedFinding {
                    detector_id: "arg-swap".into(),
                    line: 7,
                    external_source: ext("semgrep", "rule.x", "https://example.test/x"),
                }],
            )],
        };
        let actual = vec![finding("arg-swap", "files/a.rs", 7)];
        let r = audit_recall(&manifest, &actual, Path::new(""));
        let det = &r.per_detector["arg-swap"];
        assert_eq!(det.tp, 1);
        assert_eq!(det.fn_, 0);
        assert!((det.recall_upper_bound - 1.0).abs() < 1e-9);
    }

    #[test]
    fn recall_half_when_one_of_two_caught() {
        let manifest = AuditManifest {
            entries: vec![entry(
                "files/a.rs",
                vec![
                    AuditExpectedFinding {
                        detector_id: "arg-swap".into(),
                        line: 7,
                        external_source: ext("semgrep", "rule.x", "https://example.test/x"),
                    },
                    AuditExpectedFinding {
                        detector_id: "arg-swap".into(),
                        line: 12,
                        external_source: ext("nvd", "CVE-1", "https://example.test/cve"),
                    },
                ],
            )],
        };
        let actual = vec![finding("arg-swap", "files/a.rs", 7)];
        let r = audit_recall(&manifest, &actual, Path::new(""));
        let det = &r.per_detector["arg-swap"];
        assert_eq!(det.tp, 1);
        assert_eq!(det.fn_, 1);
        assert!((det.recall_upper_bound - 0.5).abs() < 1e-9);
    }

    #[test]
    fn source_breakdown_splits_by_kind() {
        let manifest = AuditManifest {
            entries: vec![entry(
                "files/a.rs",
                vec![
                    AuditExpectedFinding {
                        detector_id: "arg-swap".into(),
                        line: 7,
                        external_source: ext("semgrep", "rule.x", "https://example.test/x"),
                    },
                    AuditExpectedFinding {
                        detector_id: "arg-swap".into(),
                        line: 12,
                        external_source: ext("nvd", "CVE-1", "https://example.test/cve"),
                    },
                ],
            )],
        };
        let actual = vec![finding("arg-swap", "files/a.rs", 7)];
        let r = audit_recall(&manifest, &actual, Path::new(""));
        let det = &r.per_detector["arg-swap"];
        assert_eq!(det.source_breakdown["semgrep"].tp, 1);
        assert_eq!(det.source_breakdown["semgrep"].fn_, 0);
        assert_eq!(det.source_breakdown["nvd"].tp, 0);
        assert_eq!(det.source_breakdown["nvd"].fn_, 1);
    }

    #[test]
    fn empty_corpus_is_zero_recall() {
        let manifest = AuditManifest { entries: vec![] };
        let actual: Vec<Finding> = vec![];
        let r = audit_recall(&manifest, &actual, Path::new(""));
        assert_eq!(r.expected_total, 0);
        assert_eq!(r.overall.tp, 0);
        assert_eq!(r.overall.fn_, 0);
        assert!((r.overall.recall_upper_bound - 0.0).abs() < 1e-9);
    }
}
