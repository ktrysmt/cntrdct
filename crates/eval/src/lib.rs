//! cntrdct-eval — Layer-1-output precision / recall / F1 harness.
//!
//! Spec: `cntrdct/docs/spec/eval-v0.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedFinding {
    pub detector_id: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: PathBuf,
    pub expected: Vec<ExpectedFinding>,
    /// Upstream source URL (PyPI / GitHub release tarball / paper appendix
    /// link, etc.). Optional — synthetic seed-corpus entries omit this.
    /// Spec: M-4 (`docs/spec/multilang-v0.md`), reuses the same field for
    /// the future P-1 Rust β corpus per ROADMAP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// SPDX license expression of the upstream source. Optional for the
    /// same reason as `source`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// SHA-256 of the file's content as committed under `files/`. Lets
    /// CI re-verify integrity without re-downloading the upstream
    /// tarball. Optional for synthetic fixtures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Manifest {
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DetectorMetrics {
    pub tp: u32,
    pub fp: u32,
    #[serde(rename = "fn")]
    pub fn_: u32,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EvalReport {
    pub per_detector: BTreeMap<String, DetectorMetrics>,
    pub overall: DetectorMetrics,
    pub corpus_size: u32,
    pub expected_total: u32,
    pub actual_total: u32,
}

#[derive(Debug, Error)]
pub enum EvalError {
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
    #[error("missing source file referenced by manifest: {0}")]
    MissingSource(PathBuf),
}

pub fn load_manifest(path: &Path) -> Result<Manifest, EvalError> {
    let body = fs::read_to_string(path).map_err(|e| EvalError::Io {
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
        let entry: ManifestEntry = serde_json::from_str(trimmed).map_err(|e| EvalError::Parse {
            line: line_no,
            source: e,
        })?;
        entries.push(entry);
    }
    Ok(Manifest { entries })
}

pub fn evaluate(
    manifest: &Manifest,
    actual: &[cntrdct_core::Finding],
    corpus_dir: &Path,
) -> EvalReport {
    let expected_keys = expected_index(manifest);
    let actual_keys = actual_index(actual, corpus_dir);

    let mut per_detector: BTreeMap<String, DetectorMetrics> = BTreeMap::new();

    let detector_ids: BTreeSet<&str> = expected_keys
        .iter()
        .map(|(d, _, _)| d.as_str())
        .chain(actual_keys.iter().map(|(d, _, _)| d.as_str()))
        .collect();

    for det in detector_ids {
        let exp_for_det: Vec<&Key> = expected_keys.iter().filter(|(d, _, _)| d == det).collect();
        let act_for_det: Vec<&Key> = actual_keys.iter().filter(|(d, _, _)| d == det).collect();

        let (tp, fp, fn_) = match_one_to_one(&exp_for_det, &act_for_det);
        per_detector.insert(det.to_string(), metrics(tp, fp, fn_));
    }

    let total_tp: u32 = per_detector.values().map(|m| m.tp).sum();
    let total_fp: u32 = per_detector.values().map(|m| m.fp).sum();
    let total_fn: u32 = per_detector.values().map(|m| m.fn_).sum();
    let overall = metrics(total_tp, total_fp, total_fn);

    EvalReport {
        per_detector,
        overall,
        corpus_size: manifest.entries.len() as u32,
        expected_total: expected_keys.len() as u32,
        actual_total: actual.len() as u32,
    }
}

type Key = (String, PathBuf, u32);

fn expected_index(manifest: &Manifest) -> Vec<Key> {
    let mut out = Vec::new();
    for entry in &manifest.entries {
        for exp in &entry.expected {
            out.push((exp.detector_id.clone(), entry.file.clone(), exp.line));
        }
    }
    out
}

fn actual_index(findings: &[cntrdct_core::Finding], corpus_dir: &Path) -> Vec<Key> {
    findings
        .iter()
        .map(|f| {
            let rel = relativize(&f.primary.file, corpus_dir);
            (f.detector_id.clone(), rel, f.primary.start_line)
        })
        .collect()
}

fn relativize(path: &Path, base: &Path) -> PathBuf {
    path.strip_prefix(base)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn match_one_to_one(expected: &[&Key], actual: &[&Key]) -> (u32, u32, u32) {
    let mut consumed_expected: Vec<bool> = vec![false; expected.len()];
    let mut tp: u32 = 0;
    let mut fp: u32 = 0;

    for a in actual {
        let mut matched = None;
        for (i, e) in expected.iter().enumerate() {
            if consumed_expected[i] {
                continue;
            }
            if e == a {
                matched = Some(i);
                break;
            }
        }
        match matched {
            Some(i) => {
                consumed_expected[i] = true;
                tp += 1;
            }
            None => fp += 1,
        }
    }

    let fn_ = consumed_expected.iter().filter(|c| !**c).count() as u32;
    (tp, fp, fn_)
}

fn metrics(tp: u32, fp: u32, fn_: u32) -> DetectorMetrics {
    let precision = if tp + fp == 0 {
        0.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        0.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    DetectorMetrics {
        tp,
        fp,
        fn_,
        precision,
        recall,
        f1,
    }
}
