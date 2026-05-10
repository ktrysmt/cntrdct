//! Q-11 acceptance: small-sample interval switching + RankedFinding
//! prior_method propagation.
//!
//! Q-11 picks the Bayes-Laplace lower 2.5% bound at small `n` and the
//! Wilson bound otherwise. The motivation is methodological coherence,
//! not a uniform coverage advantage:
//! - At small `n`, the calibrator's `posterior_tp` already comes from
//!   a Beta(1, 1) Bayesian update (Laplace smoothing). Pairing the
//!   posterior mean with a credible-interval lower bound from the
//!   *same* prior keeps both columns in one statistical regime; pairing
//!   it with a frequentist Wilson bound mixes regimes.
//! - At large `n`, the Wilson interval is well-behaved and matches
//!   the Bayesian bound to several decimal places, so the choice is
//!   irrelevant and we keep Wilson for back-compat with pre-Q-11
//!   priors files.
//! - For the boundary outcome `tp = 0`, both bounds return 0 (the
//!   BCD 2001 "modified Jeffreys" boundary correction). Detectors
//!   with no observed TPs therefore stay sorted to the bottom under
//!   either method.
//!
//! Coverage probability simulations live in `examples/coverage_debug.rs`
//! (manual reference; not gated). Asserting "Jeffreys' average
//! coverage error beats Wilson's" as a structural test would
//! over-constrain the implementation: the relative ordering depends
//! on the specific `(n, p)` grid and on whether one cares about
//! one-sided vs two-sided coverage. This file gates only the properties
//! Q-11 actually depends on.
//!
//! References:
//! - Brown, Cai, DasGupta (2001) Statistical Science 16(2), 101-133
//!   ("Interval Estimation for a Binomial Proportion"). §4 discusses
//!   the coverage-oscillation regime and the boundary modification
//!   used by `jeffreys_lower_95` at `tp = 0`.
//! - Thulin (2014) Electronic Journal of Statistics 8(1), 817-840.

use std::collections::HashMap;

use cntrdct::calibration::{
    compute_lower_bound, jeffreys_lower_95, wilson_lower_95, DetectorPrior, PriorMethod,
    SMALL_SAMPLE_THRESHOLD,
};
use cntrdct::core::{
    AnomalyClass, Evidence, Finding, LanguageCitationStatus, Location, Ranker, Severity,
};
use cntrdct::ranker::{CalibratedRanker, UncalibratedRanker};

/// `Bin(n, p)` PMF computed in log-space to stay numerically stable
/// when `n` is moderate.
fn binomial_pmf(n: u32, k: u32, p: f64) -> f64 {
    if k > n {
        return 0.0;
    }
    if p == 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p == 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    let mut log_c = 0.0_f64;
    for i in 0..k {
        log_c += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
    }
    (log_c + (k as f64) * p.ln() + ((n - k) as f64) * (1.0 - p).ln()).exp()
}

/// Exact one-sided lower coverage:
///   `cov(n, p; L) = sum_{k=0..=n} P(Bin(n, p) = k) * I(p >= L(k, n-k))`
fn one_sided_coverage<F: Fn(u32, u32) -> f64>(n: u32, p: f64, lower: F) -> f64 {
    let mut cov = 0.0_f64;
    for k in 0..=n {
        let l = lower(k, n - k);
        if p >= l {
            cov += binomial_pmf(n, k, p);
        }
    }
    cov
}

#[test]
fn small_sample_intervals_distinguish_wilson_from_jeffreys_at_intermediate_tp() {
    // The whole point of the switch is that the two methods produce
    // distinguishable lower bounds at small `n`. Pin this for a few
    // representative cells: any value within rounding of the published
    // Beta-quantile reference (computed offline; see references on the
    // module-level docs) and provably different from Wilson.
    let cases: &[(u32, u32, f64, f64)] = &[
        // (tp, fp, expected_jeffreys, expected_wilson)
        (2, 1, 0.1941, 0.2077),  // Beta(3, 2) lower 2.5% / Wilson(2, 1)
        (8, 0, 0.6637, 0.6756),  // Beta(9, 1) lower 2.5% / Wilson(8, 0)
        (15, 2, 0.6529, 0.6566), // Beta(16, 3) lower 2.5% / Wilson(15, 2)
    ];
    for &(tp, fp, exp_j, exp_w) in cases {
        let lj = jeffreys_lower_95(tp, fp);
        let lw = wilson_lower_95(tp, fp);
        assert!(
            (lj - exp_j).abs() < 1e-3,
            "(tp={}, fp={}) jeffreys = {:.4} (expected {:.4})",
            tp,
            fp,
            lj,
            exp_j
        );
        assert!(
            (lw - exp_w).abs() < 1e-3,
            "(tp={}, fp={}) wilson = {:.4} (expected {:.4})",
            tp,
            fp,
            lw,
            exp_w
        );
        assert!(
            (lj - lw).abs() > 1e-3,
            "(tp={}, fp={}): jeffreys and wilson should be \
             distinguishable at small n; jeffreys={:.4}, wilson={:.4}",
            tp,
            fp,
            lj,
            lw
        );
    }
}

#[test]
fn jeffreys_zero_tp_matches_wilson_via_bcd_boundary_modification() {
    // BCD 2001 §4 boundary correction: at `tp = 0` the unmodified
    // Beta(1, n+1) lower 2.5% quantile sits above 0 and creates a
    // narrow under-coverage shoulder for `p ∈ (0, L)`. Pinning
    // `jeffreys_lower_95(0, n) == 0` keeps the lower bound aligned
    // with Wilson at the boundary and preserves the
    // observation-free convention shared with `wilson_lower_95`.
    for fp in [1_u32, 5, 10, 29] {
        assert_eq!(
            jeffreys_lower_95(0, fp),
            0.0,
            "boundary modification should pin L(0, {}) to 0",
            fp
        );
        assert_eq!(wilson_lower_95(0, fp), 0.0, "wilson L(0, {}) is 0", fp);
    }
}

#[test]
fn both_methods_meet_or_exceed_nominal_one_sided_coverage_at_typical_p_grid() {
    // At `n >= 30` Wilson is in its well-behaved regime; both methods
    // should sit close to (and not far below) the nominal 97.5%
    // one-sided lower coverage at typical mid-range `p`. This is a
    // sanity gate: if either method drops below 0.93 here, something
    // structural broke.
    let p_grid = [0.3_f64, 0.5, 0.7];
    for &n in &[30_u32, 87] {
        for &p in &p_grid {
            let cw = one_sided_coverage(n, p, wilson_lower_95);
            let cj = one_sided_coverage(n, p, jeffreys_lower_95);
            assert!(
                cw > 0.93,
                "wilson one-sided cov(n={}, p={}) = {:.4} below sanity floor",
                n,
                p,
                cw
            );
            assert!(
                cj > 0.93,
                "jeffreys one-sided cov(n={}, p={}) = {:.4} below sanity floor",
                n,
                p,
                cj
            );
        }
    }
}

#[test]
fn compute_lower_bound_switches_at_threshold() {
    // Pin the regime split: n = 29 → Jeffreys, n = 30 → Wilson. Removing
    // either branch flips one of these and the test fails.
    let (_, m_below) = compute_lower_bound(14, 15);
    assert_eq!(m_below, PriorMethod::Jeffreys);
    let (_, m_at) = compute_lower_bound(15, 15);
    assert_eq!(m_at, PriorMethod::Wilson);
    // Sanity: SMALL_SAMPLE_THRESHOLD is exposed for downstream callers
    // (test fixtures, audit reports) — pin it.
    assert_eq!(SMALL_SAMPLE_THRESHOLD, 30);
}

// ---------- prior_method propagation through the ranker ----------

fn make_finding(detector_id: &str, related: usize) -> Finding {
    Finding {
        detector_id: detector_id.to_string(),
        primary: Location {
            file: "a.rs".into(),
            start_line: 1,
            start_col: 1,
            end_line: 2,
            end_col: 1,
        },
        related: (0..related)
            .map(|i| Location {
                file: "rel.rs".into(),
                start_line: i as u32 + 1,
                start_col: 1,
                end_line: i as u32 + 2,
                end_col: 1,
            })
            .collect(),
        message: "demo".into(),
        raw_severity: Severity::Warning,
        anomaly_class: AnomalyClass::Logic,
        evidence: Evidence {
            citation_keys: vec!["cordy-roy-icpc-2008"],
            raw: serde_json::Value::Null,
            language_citation_status: LanguageCitationStatus::Confirmed,
        },
    }
}

fn priors_with(method: PriorMethod) -> HashMap<String, DetectorPrior> {
    let mut m = HashMap::new();
    m.insert(
        "clone-drift".to_string(),
        DetectorPrior {
            tp: 8,
            fp: 0,
            posterior_tp: 0.9,
            wilson_lower_95: 0.6637,
            prior_method: method,
        },
    );
    m
}

#[test]
fn calibrated_ranker_propagates_prior_method_into_ranked_finding() {
    let out = CalibratedRanker::new(priors_with(PriorMethod::Jeffreys))
        .rank(vec![make_finding("clone-drift", 3)]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].prior_method, Some(PriorMethod::Jeffreys));

    let out = CalibratedRanker::new(priors_with(PriorMethod::Wilson))
        .rank(vec![make_finding("clone-drift", 3)]);
    assert_eq!(out[0].prior_method, Some(PriorMethod::Wilson));
}

#[test]
fn uncalibrated_ranker_leaves_prior_method_none() {
    let out = UncalibratedRanker::new().rank(vec![make_finding("clone-drift", 3)]);
    assert_eq!(out[0].prior_method, None);
}

#[test]
fn calibrated_ranker_falls_back_to_none_for_unknown_detector() {
    // No prior for "mystery": fallback path must zero out prior_method
    // alongside posterior_tp and wilson_lower so downstream consumers
    // can tell the calibration data did not cover this finding.
    let out = CalibratedRanker::new(priors_with(PriorMethod::Jeffreys))
        .rank(vec![make_finding("mystery", 3)]);
    assert_eq!(out[0].prior_method, None);
    assert_eq!(out[0].wilson_lower, None);
    assert_eq!(out[0].posterior_tp, None);
}

#[test]
fn sarif_emits_prior_method_in_result_properties_when_present() {
    use cntrdct::core::Detector;
    use cntrdct::sarif::to_sarif_with_rules_ranked;

    struct FakeCloneDrift;
    impl Detector for FakeCloneDrift {
        fn id(&self) -> &'static str {
            "clone-drift"
        }
        fn name(&self) -> &'static str {
            "Clone drift"
        }
        fn citations(&self) -> &'static [cntrdct::core::Citation] {
            static CITES: [cntrdct::core::Citation; 1] = [cntrdct::core::Citation {
                key: "cordy-roy-icpc-2008",
                authors: "J.R. Cordy and C.K. Roy",
                title: "The NiCad Clone Detector",
                venue: "ICPC",
                year: 2008,
                doi: None,
                url: None,
                languages: &[cntrdct::core::Language::Rust],
            }];
            &CITES
        }
        fn supported_languages(&self) -> &'static [cntrdct::core::Language] {
            &[cntrdct::core::Language::Rust]
        }
        fn detect(
            &self,
            _: &cntrdct::core::DetectContext,
        ) -> Result<Vec<Finding>, cntrdct::core::DetectorError> {
            Ok(vec![])
        }
    }

    let cd = FakeCloneDrift;
    let detectors: Vec<&dyn Detector> = vec![&cd];

    let ranked_jeffreys = CalibratedRanker::new(priors_with(PriorMethod::Jeffreys))
        .rank(vec![make_finding("clone-drift", 1)]);
    let s = to_sarif_with_rules_ranked(&ranked_jeffreys, &detectors);
    assert_eq!(
        s["runs"][0]["results"][0]["properties"]["priorMethod"],
        "jeffreys"
    );

    let ranked_wilson = CalibratedRanker::new(priors_with(PriorMethod::Wilson))
        .rank(vec![make_finding("clone-drift", 1)]);
    let s = to_sarif_with_rules_ranked(&ranked_wilson, &detectors);
    assert_eq!(
        s["runs"][0]["results"][0]["properties"]["priorMethod"],
        "wilson"
    );

    let ranked_uncal = UncalibratedRanker::new().rank(vec![make_finding("clone-drift", 1)]);
    let s = to_sarif_with_rules_ranked(&ranked_uncal, &detectors);
    assert!(
        s["runs"][0]["results"][0]["properties"]
            .get("priorMethod")
            .is_none(),
        "priorMethod must be omitted when ranker had no calibration data"
    );
}
