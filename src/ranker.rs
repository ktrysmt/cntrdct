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

use crate::calibration::DetectorPrior;
use crate::core::{Finding, RankedFinding};

// ---------- Uncalibrated (v0) ----------

#[derive(Debug, Default)]
pub struct UncalibratedRanker;

impl UncalibratedRanker {
    pub fn new() -> Self {
        Self
    }
}

impl crate::core::Ranker for UncalibratedRanker {
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding> {
        let mut ranked: Vec<RankedFinding> = findings
            .into_iter()
            .map(|f| {
                let rank_score = f.related.len() as f64;
                RankedFinding {
                    finding: f,
                    posterior_tp: None,
                    wilson_lower: None,
                    prior_method: None,
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
    use crate::core::Ranker;
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
///
/// R-4 prior separation (B3, `docs/spec/p3-amendment-v0.md` §6 P4): a
/// Layer 0 LLM candidate ([`crate::core::Origin::Layer0Llm`]) must NOT
/// inherit the Layer 1 detector's prior — an LLM-originated candidate has
/// a different base rate than a deterministic detector hit, even when it
/// carries the same `detector_id` (e.g. `arg-swap`). v0 ships no Layer-0
/// prior, so such candidates always take the `related.len()` fallback; the
/// full `(detector_id, origin)`-keyed prior map lands in Phase B when
/// labelled Layer-0 entries exist. This keeps `priors-default.json`
/// byte-identical (its entries describe Layer-1 base rates only).
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

impl crate::core::Ranker for CalibratedRanker {
    fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding> {
        let mut ranked: Vec<RankedFinding> = findings
            .into_iter()
            .map(|f| {
                // R-4 (B3): Layer 0 LLM candidates never consult a
                // Layer-1 prior; v0 has no Layer-0 prior so they take the
                // related.len() fallback below.
                let prior = if f.origin.is_default() {
                    self.priors.get(&f.detector_id)
                } else {
                    None
                };
                match prior {
                    Some(prior) => {
                        let related = f.related.len() as f64;
                        let rank_score = prior.wilson_lower_95 * (1.0 + (1.0 + related).log2());
                        RankedFinding {
                            finding: f,
                            posterior_tp: Some(prior.posterior_tp),
                            wilson_lower: Some(prior.wilson_lower_95),
                            prior_method: Some(prior.prior_method),
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
                            prior_method: None,
                            rank_score,
                            adjudication: None,
                        }
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
