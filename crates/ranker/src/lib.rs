//! Layer 2 statistical ranker.
//!
//! Specs:
//! - `cntrdct/docs/spec/ranker-v0.md` — uncalibrated baseline (still shipped as
//!   the no-corpus fallback).
//! - `cntrdct/docs/spec/ranker-v1.md` — calibrated ranker on top of v0's API.
//!
//! v0 ships uncalibrated. Without a labelled corpus, posterior_tp and
//! wilson_lower remain `None`; rank_score falls back to `related.len()`.
//! v1 introduces `CalibratedRanker`, which consumes per-detector priors
//! produced by `cntrdct-calibration` and applies Z-Ranking
//! (Kremenek-Engler SAS 2003) and Bayesian post-analysis
//! (Jung-Kim-Shin-Yi SAS 2005) on top of the same `Ranker` trait.

use std::collections::HashMap;

use cntrdct_calibration::DetectorPrior;
use cntrdct_core::{Finding, RankedFinding};

// ---------- Uncalibrated (v0) ----------

#[derive(Debug, Default)]
pub struct UncalibratedRanker;

impl UncalibratedRanker {
    pub fn new() -> Self {
        Self
    }
}

impl cntrdct_core::Ranker for UncalibratedRanker {
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding> {
        let mut ranked: Vec<RankedFinding> = findings
            .into_iter()
            .map(|f| {
                let rank_score = f.related.len() as f64;
                RankedFinding {
                    finding: f,
                    posterior_tp: None,
                    wilson_lower: None,
                    rank_score,
                    adjudication: None,
                }
            })
            .collect();

        sort_ranked(&mut ranked);
        ranked
    }
}

pub fn rank(findings: Vec<Finding>) -> Vec<RankedFinding> {
    use cntrdct_core::Ranker;
    UncalibratedRanker::new().rank(findings)
}

// ---------- Calibrated (v1) ----------

/// Ranks findings using per-detector priors produced by `cntrdct-calibration`.
///
/// rank_score formula: `wilson_lower_95 * (1 + log2(1 + related.len()))`.
///
/// Rationale:
/// - The Wilson lower bound is a confidence-discounted TP rate; using it (rather
///   than raw `tp/(tp+fp)`) prevents detectors with little data from
///   over-ranking against well-tested ones (Z-Ranking — Kremenek-Engler SAS 2003).
/// - The `(1 + log2(1 + related.len()))` factor is monotone, sub-linear in
///   sibling-group size, and equal to 1 when `related.len() == 0`. It rewards
///   findings whose drift is corroborated by more siblings without letting a
///   single huge clone group dominate the ranking.
///
/// When a finding's `detector_id` is missing from `priors`, the calibrated
/// ranker silently falls back to the uncalibrated rank_score
/// (`related.len() as f64`) for that finding. This keeps a partially-calibrated
/// corpus useful: detectors not yet covered are not penalised.
#[derive(Debug, Default)]
pub struct CalibratedRanker {
    priors: HashMap<String, DetectorPrior>,
}

impl CalibratedRanker {
    pub fn new(priors: HashMap<String, DetectorPrior>) -> Self {
        Self { priors }
    }

    pub fn priors(&self) -> &HashMap<String, DetectorPrior> {
        &self.priors
    }
}

impl cntrdct_core::Ranker for CalibratedRanker {
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding> {
        let mut ranked: Vec<RankedFinding> = findings
            .into_iter()
            .map(|f| match self.priors.get(&f.detector_id) {
                Some(prior) => {
                    let related = f.related.len() as f64;
                    let rank_score = prior.wilson_lower_95 * (1.0 + (1.0 + related).log2());
                    RankedFinding {
                        finding: f,
                        posterior_tp: Some(prior.posterior_tp),
                        wilson_lower: Some(prior.wilson_lower_95),
                        rank_score,
                        adjudication: None,
                    }
                }
                None => {
                    let rank_score = f.related.len() as f64;
                    RankedFinding {
                        finding: f,
                        posterior_tp: None,
                        wilson_lower: None,
                        rank_score,
                        adjudication: None,
                    }
                }
            })
            .collect();

        sort_ranked(&mut ranked);
        ranked
    }
}

// ---------- Shared sort ----------

fn sort_ranked(ranked: &mut [RankedFinding]) {
    ranked.sort_by(|a, b| {
        b.rank_score
            .partial_cmp(&a.rank_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.finding.primary.file.cmp(&b.finding.primary.file))
            .then_with(|| {
                a.finding
                    .primary
                    .start_line
                    .cmp(&b.finding.primary.start_line)
            })
    });
}
