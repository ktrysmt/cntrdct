//! Q-13 acceptance test: deterministic mock providers exercise the
//! full cross-model κ pipeline.
//!
//! Spec: `docs/spec/cross-model-kappa-v0.md` F10. PR CI runs this test
//! on every push without third-party API access; nightly CI exercises
//! the same code path against live providers.
//!
//! Fixture: `tests/fixtures/cross-model-kappa/sample-corpus.jsonl`
//! ships 8 ranked findings (6 in `clone-drift:Logic`, 2 in
//! `arg-swap:Interface`). Three canned providers are wired up with
//! verdict streams chosen so that:
//!
//! - `clone-drift:Logic` has perfect κ between alpha and beta but
//!   substantial disagreement on the alpha-gamma pair (worst κ).
//! - `arg-swap:Interface` is `low_n` (n = 2 < MIN_N) so it is reported
//!   but excluded from `low_reliability` flagging and from
//!   worst-cell selection.

use std::path::PathBuf;
use std::sync::Mutex;

use cntrdct::adjudicator::PromptDispatch;
use cntrdct::core::{AdjudicationResult, AdjudicationVerdict, DetectorError};
use cntrdct::cross_model_kappa::{
    aggregate, load_corpus, run_audit, AuditError, AuditReport, ProviderHandle, ProviderStatus,
    Verdict3, MIN_N, SUBSTANTIAL_AGREEMENT_THRESHOLD,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cross-model-kappa")
        .join("sample-corpus.jsonl")
}

/// Canned `PromptDispatch` impl that returns verdicts from a
/// pre-configured queue. Tests parameterise the verdict stream per
/// provider so the κ matrix is deterministic.
struct CannedDispatch {
    provider_id: &'static str,
    model: String,
    verdicts: Mutex<Vec<Verdict3>>,
    cursor: Mutex<usize>,
}

impl CannedDispatch {
    fn new(provider_id: &'static str, model: &str, verdicts: Vec<Verdict3>) -> Self {
        Self {
            provider_id,
            model: model.to_string(),
            verdicts: Mutex::new(verdicts),
            cursor: Mutex::new(0),
        }
    }
}

impl PromptDispatch for CannedDispatch {
    fn provider_id(&self) -> &'static str {
        self.provider_id
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn dispatch(&self, _prompt: &str) -> Result<AdjudicationResult, DetectorError> {
        let mut cursor = self.cursor.lock().unwrap();
        let verdicts = self.verdicts.lock().unwrap();
        let v = verdicts
            .get(*cursor)
            .copied()
            .unwrap_or(AdjudicationVerdict::Uncertain);
        *cursor += 1;
        Ok(AdjudicationResult {
            verdict: v,
            confidence: 0.5,
            rationale: "canned".to_string(),
            calibration_tag: None,
            calibrated_confidence: None,
        })
    }
}

fn vt() -> Verdict3 {
    AdjudicationVerdict::LikelyTruePositive
}
fn vf() -> Verdict3 {
    AdjudicationVerdict::LikelyFalsePositive
}

/// Build the three-provider stack used by every test in this file.
/// alpha and beta agree on every clone-drift finding; gamma disagrees
/// substantially on the clone-drift cell. The arg-swap cell (last 2
/// findings) is below MIN_N so its κ never feeds worst_cell.
fn three_provider_stack() -> Vec<ProviderHandle> {
    // 8 findings total (6 clone-drift + 2 arg-swap).
    // alpha: T,T,F,T,T,F | T,F
    // beta : T,T,F,T,T,F | F,T   <-- perfect on clone-drift cell
    // gamma: F,F,T,F,F,T | T,F   <-- inverted on clone-drift cell
    let alpha = vec![vt(), vt(), vf(), vt(), vt(), vf(), vt(), vf()];
    let beta = vec![vt(), vt(), vf(), vt(), vt(), vf(), vf(), vt()];
    let gamma = vec![vf(), vf(), vt(), vf(), vf(), vt(), vt(), vf()];

    vec![
        ProviderHandle {
            provider_id: "alpha".to_string(),
            model: "alpha-model".to_string(),
            adjudicator: Some(Box::new(CannedDispatch::new("alpha", "alpha-model", alpha))),
            status: ProviderStatus::Mocked,
        },
        ProviderHandle {
            provider_id: "beta".to_string(),
            model: "beta-model".to_string(),
            adjudicator: Some(Box::new(CannedDispatch::new("beta", "beta-model", beta))),
            status: ProviderStatus::Mocked,
        },
        ProviderHandle {
            provider_id: "gamma".to_string(),
            model: "gamma-model".to_string(),
            adjudicator: Some(Box::new(CannedDispatch::new("gamma", "gamma-model", gamma))),
            status: ProviderStatus::Mocked,
        },
    ]
}

#[test]
fn fixture_corpus_loads_eight_rows() {
    let corpus = load_corpus(&fixture_path()).expect("load");
    assert_eq!(corpus.len(), 8, "fixture must have eight ranked findings");
    assert_eq!(corpus[0].finding.detector_id, "clone-drift");
    assert_eq!(corpus[6].finding.detector_id, "arg-swap");
}

#[test]
fn run_audit_produces_three_pairwise_kappa_entries_per_cell() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let providers = three_provider_stack();
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    assert_eq!(report.providers.len(), 3);
    assert_eq!(report.cells.len(), 2, "two cells: clone-drift + arg-swap");

    let clone_cell = report
        .cells
        .iter()
        .find(|c| c.detector_id == "clone-drift")
        .expect("clone-drift cell");
    assert_eq!(clone_cell.n, 6);
    assert_eq!(clone_cell.pairwise_kappa.len(), 3);
    assert!(clone_cell.pairwise_kappa.contains_key("alpha-beta"));
    assert!(clone_cell.pairwise_kappa.contains_key("alpha-gamma"));
    assert!(clone_cell.pairwise_kappa.contains_key("beta-gamma"));
}

#[test]
fn alpha_beta_perfect_agreement_on_clone_drift_cell() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let providers = three_provider_stack();
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    let clone_cell = report
        .cells
        .iter()
        .find(|c| c.detector_id == "clone-drift")
        .unwrap();
    let alpha_beta = &clone_cell.pairwise_kappa["alpha-beta"];
    let kappa = alpha_beta.kappa.expect("non-degenerate κ");
    assert!(
        (kappa - 1.0).abs() < 1e-9,
        "alpha-beta on clone-drift must be perfect agreement, got {}",
        kappa
    );
    assert!(!alpha_beta.degenerate);
}

#[test]
fn alpha_gamma_perfect_inversion_drives_worst_cell() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let providers = three_provider_stack();
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    let clone_cell = report
        .cells
        .iter()
        .find(|c| c.detector_id == "clone-drift")
        .unwrap();
    let alpha_gamma = &clone_cell.pairwise_kappa["alpha-gamma"];
    let kappa = alpha_gamma.kappa.expect("non-degenerate κ");
    assert!(
        kappa < 0.0,
        "alpha-gamma on clone-drift must be negative κ (inversion), got {}",
        kappa
    );

    let worst = report.worst_cell.expect("worst cell present");
    assert_eq!(worst.detector_id, "clone-drift");
    assert_eq!(worst.pair, "alpha-gamma");
    assert!(
        worst.kappa < SUBSTANTIAL_AGREEMENT_THRESHOLD,
        "worst κ must be below the substantial-agreement threshold"
    );

    assert!(
        clone_cell.low_reliability,
        "clone-drift:Logic must flag low_reliability when worst pair is far below threshold"
    );
    assert!(!clone_cell.low_n);
}

#[test]
fn arg_swap_cell_is_low_n_and_never_worst() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let providers = three_provider_stack();
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    let arg_swap = report
        .cells
        .iter()
        .find(|c| c.detector_id == "arg-swap")
        .expect("arg-swap cell");
    assert_eq!(arg_swap.n, 2);
    assert!(arg_swap.n < MIN_N);
    assert!(arg_swap.low_n);
    assert!(!arg_swap.low_reliability);

    let worst = report.worst_cell.unwrap();
    assert_ne!(
        worst.detector_id, "arg-swap",
        "low_n cells must never feed worst_cell selection"
    );
}

#[test]
fn audit_report_round_trips_through_pretty_json() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let providers = three_provider_stack();
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    let body = report.to_json_pretty();
    let restored: AuditReport = serde_json::from_str(&body).expect("round-trip");
    assert_eq!(restored, report, "round-trip must preserve every field");
}

#[test]
fn skipped_provider_is_recorded_but_excluded_from_pairs() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let alpha = vec![vt(), vt(), vf(), vt(), vt(), vf(), vt(), vf()];
    let beta = vec![vt(), vt(), vf(), vt(), vt(), vf(), vf(), vt()];
    let providers = vec![
        ProviderHandle {
            provider_id: "alpha".to_string(),
            model: "alpha-model".to_string(),
            adjudicator: Some(Box::new(CannedDispatch::new("alpha", "alpha-model", alpha))),
            status: ProviderStatus::Mocked,
        },
        ProviderHandle {
            provider_id: "beta".to_string(),
            model: "beta-model".to_string(),
            adjudicator: Some(Box::new(CannedDispatch::new("beta", "beta-model", beta))),
            status: ProviderStatus::Mocked,
        },
        ProviderHandle {
            provider_id: "gamma".to_string(),
            model: "gamma-model".to_string(),
            adjudicator: None,
            status: ProviderStatus::Skipped("no key".to_string()),
        },
    ];
    let report = run_audit(
        "2026-05-11".to_string(),
        "2026-05-11T00:00:00Z".to_string(),
        providers,
        inputs,
    )
    .expect("run_audit");

    // gamma is recorded with Skipped status...
    assert_eq!(report.providers.len(), 3);
    let gamma = report
        .providers
        .iter()
        .find(|p| p.provider_id == "gamma")
        .unwrap();
    assert_eq!(gamma.status, ProviderStatus::Skipped("no key".to_string()));

    // ...but no pairwise entry references it.
    let clone_cell = report
        .cells
        .iter()
        .find(|c| c.detector_id == "clone-drift")
        .unwrap();
    assert_eq!(
        clone_cell.pairwise_kappa.len(),
        1,
        "only alpha-beta pair should appear when gamma is skipped"
    );
    assert!(clone_cell.pairwise_kappa.contains_key("alpha-beta"));
}

#[test]
fn empty_corpus_yields_empty_corpus_error() {
    let providers = three_provider_stack();
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
fn cli_subcommand_reports_insufficient_providers_when_no_clis_available() {
    // PR-CI smoke test: pointing both CLI program overrides at
    // nonexistent paths is the canonical way to make the subcommand
    // fail deterministically across dev / CI hosts (avoids the
    // hazard of accidentally invoking a real `claude` if the
    // developer is logged in locally). The audit must fail loudly
    // with the documented "at least two live providers required"
    // error rather than silently producing an empty audit log.
    let corpus = fixture_path();
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("audit.json");

    let result = std::process::Command::new(env!("CARGO_BIN_EXE_cntrdct"))
        .arg("cross-model-kappa")
        .arg(&corpus)
        .arg("--output")
        .arg(&out)
        .env(
            "CLAUDE_CLI_PROGRAM_OVERRIDE",
            "/cntrdct-test-nonexistent-claude",
        )
        .env(
            "GEMINI_CLI_PROGRAM_OVERRIDE",
            "/cntrdct-test-nonexistent-gemini",
        )
        .output()
        .expect("spawn cntrdct");
    assert!(
        !result.status.success(),
        "no available CLIs must produce a non-zero exit"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("at least two live providers required"),
        "stderr must surface the documented error; got: {}",
        stderr
    );
    assert!(
        !out.exists(),
        "no audit log should be written on the failure path"
    );
}

#[test]
fn aggregate_is_pure_against_repeated_calls() {
    let inputs = load_corpus(&fixture_path()).expect("load");
    let alpha = vec![vt(), vt(), vf(), vt(), vt(), vf(), vt(), vf()];
    let beta = vec![vt(), vt(), vf(), vt(), vt(), vf(), vf(), vt()];
    let provider_ids = ["alpha", "beta"];
    let matrix = vec![alpha.clone(), beta.clone()];
    let s1 = aggregate(&inputs, &provider_ids, &matrix);
    let s2 = aggregate(&inputs, &provider_ids, &matrix);
    assert_eq!(s1, s2, "aggregate must be a pure function of its inputs");
}
