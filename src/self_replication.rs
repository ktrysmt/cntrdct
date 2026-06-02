//! Self-replication ledger — per-release eval-snapshot deltas.
//!
//! The ledger lives at `benchmarks/self-replication/v<release>/cntrdct.jsonl`:
//! one [`crate::eval::EvalReport`] per line (one corpus per line), produced by
//! running `cntrdct eval` over each tracked corpus. This module computes the
//! precision / recall / F1 delta of a freshly-evaluated corpus against the
//! matching line in the previous release's snapshot, so a release reviewer can
//! confirm a change did not regress detection quality.
//!
//! It replaces the retired Q-15 "external SOTA comparator" framing: cntrdct
//! tracks its own metrics across releases rather than comparing head-to-head
//! against tools whose weights / infrastructure are not distributable.
//!
//! Spec: `REBUILD.md` R-1.f. No CI gate — the ledger is refreshed manually
//! per release.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::eval::{DetectorMetrics, EvalReport};

/// The three headline metrics for one cell (overall or per-detector).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Prf {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

impl From<&DetectorMetrics> for Prf {
    fn from(m: &DetectorMetrics) -> Self {
        Prf {
            precision: m.precision,
            recall: m.recall,
            f1: m.f1,
        }
    }
}

impl Prf {
    /// `self - other`, component-wise.
    fn minus(self, other: Prf) -> Prf {
        Prf {
            precision: self.precision - other.precision,
            recall: self.recall - other.recall,
            f1: self.f1 - other.f1,
        }
    }
}

/// One metric cell's current value, the matching previous value (if a prior
/// snapshot line covered it), and the current-minus-previous delta.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellDelta {
    pub current: Prf,
    /// `None` when no previous snapshot line / detector matched (e.g. the
    /// first release to carry a snapshot, or a newly-added detector).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<Prf>,
    /// `current - previous`; `None` when `previous` is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Prf>,
}

impl CellDelta {
    fn new(current: Prf, previous: Option<Prf>) -> Self {
        CellDelta {
            current,
            previous,
            delta: previous.map(|p| current.minus(p)),
        }
    }
}

/// Delta of one corpus's current eval against the previous release's snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReplicationDelta {
    pub corpus: String,
    /// `true` when a previous snapshot line for this corpus was found and the
    /// deltas are meaningful; `false` when this is a baseline (no prior).
    pub has_baseline: bool,
    pub overall: CellDelta,
    pub per_detector: BTreeMap<String, CellDelta>,
}

/// Compute the self-replication delta of `current` against the matching corpus
/// line in `previous` (matched by [`EvalReport::corpus`]). When no prior line
/// matches, every cell is reported as a baseline (`previous` / `delta` are
/// `None`, `has_baseline` is `false`).
pub fn assemble_report(current: &EvalReport, previous: &[EvalReport]) -> SelfReplicationDelta {
    let prior = previous.iter().find(|r| r.corpus == current.corpus);

    let overall = CellDelta::new(
        Prf::from(&current.overall),
        prior.map(|p| Prf::from(&p.overall)),
    );

    let detector_ids: BTreeSet<&str> = current
        .per_detector
        .keys()
        .map(String::as_str)
        .chain(
            prior
                .into_iter()
                .flat_map(|p| p.per_detector.keys().map(String::as_str)),
        )
        .collect();

    let mut per_detector = BTreeMap::new();
    for id in detector_ids {
        // A detector present only in the prior snapshot (dropped since) still
        // surfaces, with a zeroed current cell so the regression is visible.
        let cur = current.per_detector.get(id).map(Prf::from).unwrap_or(Prf {
            precision: 0.0,
            recall: 0.0,
            f1: 0.0,
        });
        let prev = prior.and_then(|p| p.per_detector.get(id)).map(Prf::from);
        per_detector.insert(id.to_string(), CellDelta::new(cur, prev));
    }

    SelfReplicationDelta {
        corpus: current.corpus.clone(),
        has_baseline: prior.is_some(),
        overall,
        per_detector,
    }
}

#[derive(Debug, Error)]
pub enum SnapshotError {
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
}

/// Load a self-replication snapshot: one [`EvalReport`] per non-empty line of a
/// JSONL file. Blank lines and `//`-prefixed comment lines are skipped,
/// mirroring [`crate::eval::load_manifest`].
pub fn load_eval_snapshot(path: &Path) -> Result<Vec<EvalReport>, SnapshotError> {
    let body = fs::read_to_string(path).map_err(|e| SnapshotError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut reports = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let report: EvalReport =
            serde_json::from_str(trimmed).map_err(|e| SnapshotError::Parse {
                line: (i + 1) as u32,
                source: e,
            })?;
        reports.push(report);
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(precision: f64, recall: f64, f1: f64) -> DetectorMetrics {
        DetectorMetrics {
            tp: 0,
            fp: 0,
            fn_: 0,
            precision,
            recall,
            f1,
        }
    }

    fn report(
        corpus: &str,
        overall: DetectorMetrics,
        dets: &[(&str, DetectorMetrics)],
    ) -> EvalReport {
        EvalReport {
            corpus: corpus.to_string(),
            per_detector: dets
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            overall,
            corpus_size: 0,
            expected_total: 0,
            actual_total: 0,
        }
    }

    #[test]
    fn baseline_when_no_previous_snapshot() {
        let cur = report(
            "audit-corpus",
            metrics(0.8, 0.7, 0.75),
            &[("arg-swap", metrics(0.9, 0.6, 0.72))],
        );
        let delta = assemble_report(&cur, &[]);
        assert_eq!(delta.corpus, "audit-corpus");
        assert!(!delta.has_baseline);
        assert!(delta.overall.previous.is_none());
        assert!(delta.overall.delta.is_none());
        assert!(delta.per_detector["arg-swap"].delta.is_none());
    }

    #[test]
    fn delta_matches_corpus_by_name() {
        let prev = vec![
            report(
                "wild-corpus",
                metrics(0.5, 0.5, 0.5),
                &[("clone-drift", metrics(0.4, 0.4, 0.4))],
            ),
            report(
                "audit-corpus",
                metrics(0.6, 0.6, 0.6),
                &[("arg-swap", metrics(0.6, 0.5, 0.55))],
            ),
        ];
        let cur = report(
            "audit-corpus",
            metrics(0.8, 0.7, 0.75),
            &[("arg-swap", metrics(0.9, 0.6, 0.72))],
        );
        let delta = assemble_report(&cur, &prev);
        assert!(delta.has_baseline);
        let d = delta.overall.delta.expect("overall delta present");
        assert!((d.f1 - 0.15).abs() < 1e-9, "f1 delta {}", d.f1);
        assert!(
            (d.precision - 0.2).abs() < 1e-9,
            "precision delta {}",
            d.precision
        );
        let asw = delta.per_detector["arg-swap"]
            .delta
            .expect("arg-swap delta");
        assert!(
            (asw.recall - 0.1).abs() < 1e-9,
            "recall delta {}",
            asw.recall
        );
    }

    #[test]
    fn dropped_detector_surfaces_with_zeroed_current() {
        let prev = vec![report(
            "audit-corpus",
            metrics(0.6, 0.6, 0.6),
            &[
                ("arg-swap", metrics(0.6, 0.5, 0.55)),
                ("clone-drift", metrics(0.7, 0.7, 0.7)),
            ],
        )];
        let cur = report(
            "audit-corpus",
            metrics(0.8, 0.7, 0.75),
            &[("arg-swap", metrics(0.9, 0.6, 0.72))],
        );
        let delta = assemble_report(&cur, &prev);
        let dropped = &delta.per_detector["clone-drift"];
        assert_eq!(dropped.current.f1, 0.0);
        let d = dropped
            .delta
            .expect("dropped detector still carries a delta");
        assert!((d.f1 - (-0.7)).abs() < 1e-9, "f1 delta {}", d.f1);
    }

    #[test]
    fn snapshot_round_trips_through_jsonl() {
        let r = report(
            "audit-corpus",
            metrics(0.8, 0.7, 0.75),
            &[("arg-swap", metrics(0.9, 0.6, 0.72))],
        );
        let line = serde_json::to_string(&r).unwrap();
        assert!(!line.contains('\n'), "compact JSON has no embedded newline");
        let parsed: Vec<EvalReport> = line
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].corpus, "audit-corpus");
    }
}
