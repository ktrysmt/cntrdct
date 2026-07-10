use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use cntrdct::core::Detector;
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::lang::go_build_tag_interaction::GoBuildTagInteraction;
use cntrdct::detectors::lang::python_unreachable_except::PythonUnreachableExcept;
use cntrdct::detectors::lang::rust_config_interaction::ConfigInteraction;
use cntrdct::detectors::pr_miner::PrMinerDetector;
use cntrdct::detectors::unreachable_after_terminator::UnreachableAfterTerminator;

#[derive(Parser)]
#[command(
    name = "cntrdct",
    version,
    about = "Evidence-based contradiction linter"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a path for contradiction findings.
    ///
    /// Findings are ranked by Layer 2. When a calibration priors file is
    /// available (default: `<cache_dir>/cntrdct/priors.json`), the calibrated
    /// ranker is used; otherwise the uncalibrated baseline (sibling-count
    /// ordering) is used. Pass `--no-calibration` to force the baseline.
    Scan {
        /// Path to scan (file or directory).
        path: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        /// Force the uncalibrated ranker even if priors are present.
        #[arg(long, default_value_t = false)]
        no_calibration: bool,
        /// Override the priors file path (bypasses default cache lookup).
        /// Spec'd in `docs/spec/ranker-v1.md` for testability.
        #[arg(long)]
        priors: Option<PathBuf>,
        /// Run the Layer 3 LLM adjudicator on the top-N findings.
        /// Requires `ANTHROPIC_API_KEY` in the environment; if unset, the
        /// run continues without adjudication and a note is printed to stderr.
        /// Spec: `docs/spec/adjudicator-v0.md`.
        #[arg(long, default_value_t = false)]
        adjudicate: bool,
        /// Number of top-ranked findings to adjudicate when `--adjudicate`
        /// is set.
        #[arg(long, default_value_t = 5)]
        adjudicate_top: usize,
        /// Which Layer 3 backend runs the adjudicator. `claude-cli`
        /// (DEFAULT) shells out to `claude --print` on the Haiku model
        /// using the subscription login (no API key), and falls back to
        /// `agy` (Gemini) when the Claude subscription hits its usage cap.
        /// `anthropic` uses the Anthropic Messages API and needs
        /// `ANTHROPIC_API_KEY`. `agy-cli` forces a non-Anthropic Gemini
        /// model. Spec: `docs/spec/adjudicator-v0.md`,
        /// `docs/spec/cross-model-kappa-v0.md`.
        #[arg(long, value_enum, default_value_t = AdjudicateVia::ClaudeCli)]
        adjudicate_via: AdjudicateVia,
        /// Override the `cntrdct.toml` config file. When omitted, the scan
        /// root is searched for `cntrdct.toml`; absent file is silently
        /// treated as an empty config.
        #[arg(long)]
        config: Option<PathBuf>,
        /// R-4: run the opt-in Layer 0 LLM candidate generator (arg-swap
        /// Bound B) before Layer 1. REQUIRES `--adjudicate`: candidates
        /// flow through Layer 3, and any candidate left unadjudicated is
        /// suppressed from the output (an unadjudicated LLM proposal has
        /// no precision floor). The optional value selects the provider
        /// CLI (`claude-cli` default, or `agy-cli`); auth is delegated
        /// to that CLI's own login. Excluded from the `network-isolation`
        /// netns gate by design. Spec: `docs/spec/p3-amendment-v0.md`.
        #[arg(
            long,
            value_enum,
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "claude-cli",
            requires = "adjudicate"
        )]
        candidate_llm: Option<CandidateProvider>,
        /// R-4: hard ceiling on Layer 0 LLM dispatches per scan. The
        /// deterministic pre-filter narrows the LLM to the Bound B
        /// residue; this caps fan-out on large trees. Call sites beyond
        /// the cap are logged as skipped, never silently dropped.
        #[arg(long, default_value_t = cntrdct::candidate_llm::DEFAULT_MAX_CALLS)]
        candidate_llm_max_calls: usize,
        /// R-4 (R3): override the self-preference guard. By default the
        /// scan refuses when the Layer 0 proposer and the Layer 3
        /// adjudicator are the same model family (e.g.
        /// `--candidate-llm=claude-cli` with the Anthropic adjudicator),
        /// because a judge over-accepts its own family's proposals
        /// (`wataoka-2024`). Pass this to proceed anyway.
        #[arg(long)]
        allow_self_preference: bool,
        /// B-1: filter output through a baseline file — only findings
        /// NOT recorded in the baseline are reported (ratchet mode for
        /// adopting cntrdct in an existing codebase). The filter runs
        /// after Layer 2 ranking and before Layer 3 adjudication, so
        /// the opt-in LLM budget is spent on new findings only. A
        /// missing or malformed baseline file is a hard error.
        /// Spec: `docs/spec/baseline-v0.md`.
        #[arg(long)]
        baseline: Option<PathBuf>,
        /// B-1: record the current finding set as a baseline file and
        /// exit 0 (`--fail-on` is not enforced on the run that accepts
        /// the findings). Mutually exclusive with `--baseline`; to
        /// update a baseline, simply regenerate it.
        /// Spec: `docs/spec/baseline-v0.md`.
        #[arg(long, conflicts_with = "baseline")]
        write_baseline: Option<PathBuf>,
        /// B-1: exit-code policy. `error` exits 3 when any reported
        /// finding has raw_severity Error; `warning` exits 3 on Error
        /// or Warning; `never` (default) always exits 0 on a
        /// successful scan. Applied AFTER baseline filtering, so with
        /// `--baseline` only new findings can fail the run. Exit code
        /// 3 is distinct from 1 (operational error) and 2 (usage
        /// error). Spec: `docs/spec/baseline-v0.md`.
        #[arg(long, value_enum, default_value_t = FailOn::Never)]
        fail_on: FailOn,
    },
    /// Build calibration priors from a labelled JSONL corpus.
    ///
    /// Reads the corpus, computes per-detector TP/FP / posterior_tp /
    /// wilson_lower_95, and writes the resulting JSON to `--output` (default:
    /// `<cache_dir>/cntrdct/priors.json`).
    ///
    /// Pass `--fit-platt` to switch to Q-12 mode: the corpus is read as a
    /// JSONL of `LabelledLlmConfidence` rows, Platt parameters are fit per
    /// `(detector_id, anomaly_class)` cell, and the resulting registry is
    /// written to `--output` (default:
    /// `benchmarks/llm-calibration/platt-default.json`). Spec:
    /// `docs/spec/llm-calibration-v0.md`.
    ///
    /// Pass `--audit-recall` to switch to Q-14 mode: the corpus argument is
    /// interpreted as an audit-corpus DIRECTORY (not a JSONL file), and the
    /// per-detector recall upper bound is computed against externally-sourced
    /// ground truth recorded in `<corpus>/manifest.jsonl`. Output defaults to
    /// stdout (or `--output PATH` to write to disk). Spec:
    /// `docs/spec/recall-audit-v0.md`.
    Calibrate {
        /// Path to a labelled JSONL corpus (default mode and `--fit-platt`),
        /// or to an audit-corpus directory (`--audit-recall`).
        corpus: PathBuf,
        /// Output path. Defaults to `<cache_dir>/cntrdct/priors.json` for
        /// the priors mode, and `benchmarks/llm-calibration/platt-default.json`
        /// for the Platt mode. For `--audit-recall`, defaults to stdout
        /// (the report JSON is printed) when omitted.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Q-12: fit Platt parameters from a labelled LLM-confidence
        /// corpus instead of computing per-detector priors.
        #[arg(long, default_value_t = false, conflicts_with = "audit_recall")]
        fit_platt: bool,
        /// Q-14: compute per-detector recall against an audit corpus whose
        /// `expected` entries cite externally-sourced ground truth. The
        /// `corpus` argument is read as a DIRECTORY in this mode.
        #[arg(long, default_value_t = false, conflicts_with = "fit_platt")]
        audit_recall: bool,
        /// Audit-recall only: override the manifest path. Defaults to
        /// `<corpus>/manifest.jsonl`. Ignored unless `--audit-recall` is set.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Evaluate detectors against a labelled corpus and print the
    /// precision/recall/F1 report as JSON.
    ///
    /// Spec: `docs/spec/eval-v0.md`.
    Eval {
        /// Corpus root directory. Must contain `manifest.jsonl` unless
        /// `--manifest` is given.
        corpus_dir: PathBuf,
        /// Override the manifest file path. Defaults to
        /// `<corpus_dir>/manifest.jsonl`.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Self-replication delta: read a previous release's snapshot
        /// (a JSONL of `EvalReport` lines, e.g.
        /// `benchmarks/self-replication/v<prev>/cntrdct.jsonl`) and print
        /// the precision/recall/F1 delta of this corpus against the
        /// matching line instead of the plain `EvalReport`. When no line
        /// matches the corpus, the delta is reported as a baseline.
        #[arg(long)]
        against: Option<PathBuf>,
    },
    /// Q-13: cross-model κ audit. Routes the same finding set through
    /// `claude --print` and `agy -p` (Antigravity, a non-Anthropic Gemini
    /// model), then reports pairwise Cohen's κ per
    /// `(detector_id, anomaly_class)` cell. Spec:
    /// `docs/spec/cross-model-kappa-v0.md`.
    ///
    /// Auth is delegated to each CLI's own login (no API keys read by
    /// cntrdct). A missing CLI surfaces as a `skipped` provider in
    /// the audit JSON; at least two live providers are required to
    /// compute κ. The audit is on-demand only — there is no nightly
    /// CI cadence behind it.
    CrossModelKappa {
        /// Path to a JSONL or JSON-array corpus of `RankedFinding`
        /// rows. The shape `cntrdct scan --format json` emits is
        /// accepted directly.
        corpus: PathBuf,
        /// Optional output path. When omitted, the audit JSON is
        /// printed to stdout (composes cleanly with `> file.json`).
        /// When set, the JSON is written to disk and a one-line
        /// summary is printed to stderr.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    /// Pretty-printed JSON of `Vec<RankedFinding>`.
    Json,
    /// SARIF 2.1.0, ordered by rank_score descending.
    Sarif,
}

/// B-1: exit-code policy for `scan`. Mirrors the `fail-on` input of the
/// GitHub Action (`.github/actions/scan`), so local runs, pre-commit
/// hooks, and CI can share one severity threshold vocabulary.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
enum FailOn {
    /// Exit 3 when any reported finding has raw_severity Error.
    Error,
    /// Exit 3 when any reported finding has raw_severity Error or Warning.
    Warning,
    /// Always exit 0 on a successful scan (the default; preserves the
    /// pre-B-1 behaviour).
    #[default]
    Never,
}

/// R-4: provider CLI backing the Layer 0 candidate generator. Both shell
/// out via `PromptDispatch` (no `reqwest`); auth is delegated to each
/// CLI's own login.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum CandidateProvider {
    /// Claude Code's `claude --print`.
    ClaudeCli,
    /// Google Antigravity's `agy -p` (multi-model; forced to a Gemini
    /// model — replaces the retired `gemini` CLI).
    AgyCli,
}

/// Task 1: Layer 3 adjudicator backend for `scan --adjudicate`. The HTTP
/// Anthropic path needs `ANTHROPIC_API_KEY`; the two CLI backends use the
/// respective CLI's own subscription login instead, which is what lets the
/// end-to-end recall measurement run without an API key.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
enum AdjudicateVia {
    /// `claude --print` (Haiku) subscription login (no API key) — the
    /// DEFAULT. Falls back to `agy` (Gemini) when the Claude subscription
    /// hits its usage cap. This is the normal `claude -p` adjudication path.
    #[default]
    ClaudeCli,
    /// Anthropic Messages API over HTTP (`reqwest`); requires
    /// `ANTHROPIC_API_KEY`. Explicit opt-in.
    Anthropic,
    /// `agy -p` (Antigravity) subscription login, non-Anthropic Gemini
    /// model (no API key). Explicit opt-in (also the claude-cli fallback).
    AgyCli,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan {
            path,
            format,
            no_calibration,
            priors,
            adjudicate,
            adjudicate_top,
            adjudicate_via,
            config,
            candidate_llm,
            candidate_llm_max_calls,
            allow_self_preference,
            baseline,
            write_baseline,
            fail_on,
        } => {
            let cfg = match cntrdct::load_config(config.as_deref(), &path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::from(1);
                }
            };
            match cntrdct::scan_full_with_config(&path, &cfg) {
                Ok((raw_findings, parsed_files)) => {
                    let mut findings =
                        match cntrdct::config::apply(&cfg, &parsed_files, raw_findings) {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!("error: {}", e);
                                return ExitCode::from(1);
                            }
                        };

                    // R-4 Layer 0: originate LLM candidates BEFORE Layer 2,
                    // so they flow through ranking + adjudication. Opt-in,
                    // requires --adjudicate (enforced by clap). The provider
                    // CLI shells out via PromptDispatch (no reqwest).
                    let mut layer0_ran = false;
                    if let Some(provider) = candidate_llm {
                        // R-4 (R3): refuse same-family proposer+confirmer
                        // (self-preference, wataoka-2024). The guard keys on
                        // the MODEL string of each side, not the provider id:
                        // `agy` is multi-model, so only its selected model
                        // (a forced Gemini by default) carries the family,
                        // and the Layer 3 adjudicator's family now depends on
                        // --adjudicate-via, not a hardcoded Anthropic.
                        let proposer_model = match provider {
                            CandidateProvider::ClaudeCli => {
                                cntrdct::adjudicator::CLAUDE_CLI_MODEL.to_string()
                            }
                            CandidateProvider::AgyCli => cntrdct::agy_cli_model(),
                        };
                        // The claude-cli adjudicator runs the Haiku model
                        // (still anthropic family). Note the claude-cli
                        // backend also carries an `agy` (google) usage-limit
                        // fallback, so a same-family verdict here may become
                        // cross-family at runtime if the cap is hit; the
                        // guard classifies on the PRIMARY adjudicator.
                        let adjudicator_model = match adjudicate_via {
                            AdjudicateVia::Anthropic => {
                                cntrdct::adjudicator::DEFAULT_MODEL.to_string()
                            }
                            AdjudicateVia::ClaudeCli => {
                                cntrdct::adjudicator::CLAUDE_CLI_ADJUDICATE_MODEL.to_string()
                            }
                            AdjudicateVia::AgyCli => cntrdct::agy_cli_model(),
                        };
                        if cntrdct::candidate_llm::is_self_preference_conflict(
                            &proposer_model,
                            &adjudicator_model,
                        ) {
                            if allow_self_preference {
                                eprintln!(
                                    "warning: Layer 0 proposer `{}` and the Layer 3 adjudicator `{}` share a model family; self-preference bias possible (wataoka-2024). Proceeding (--allow-self-preference).",
                                    proposer_model, adjudicator_model,
                                );
                            } else {
                                eprintln!(
                                    "error: Layer 0 proposer `{}` and the Layer 3 adjudicator `{}` are the same model family (self-preference bias, wataoka-2024). Use a cross-family pairing (e.g. `--candidate-llm=claude-cli --adjudicate-via=agy-cli`), or pass `--allow-self-preference` to override.",
                                    proposer_model, adjudicator_model,
                                );
                                return ExitCode::from(2);
                            }
                        }
                        let handle = match provider {
                            CandidateProvider::ClaudeCli => {
                                cntrdct::build_audit_claude_cli_provider()
                            }
                            CandidateProvider::AgyCli => cntrdct::build_audit_agy_cli_provider(),
                        };
                        match handle.adjudicator {
                            Some(dispatch) => {
                                let outcome = cntrdct::candidate_llm::run_candidate_llm(
                                    &parsed_files,
                                    dispatch.as_ref(),
                                    candidate_llm_max_calls,
                                );
                                eprintln!(
                                    "note: Layer 0 candidate generator ({}): {} dispatched, {} candidate(s), {} skipped over cap, {} dropped",
                                    handle.provider_id,
                                    outcome.dispatched,
                                    outcome.candidates.len(),
                                    outcome.skipped_over_cap,
                                    outcome.dropped,
                                );
                                findings.extend(outcome.candidates);
                                layer0_ran = true;
                            }
                            None => {
                                // R-4 (R9): a missing optional provider must
                                // not fail the scan — continue Layer-1-only.
                                eprintln!(
                                    "note: --candidate-llm requested but provider `{}` unavailable ({:?}); continuing with Layer 1 findings only",
                                    handle.provider_id, handle.status,
                                );
                            }
                        }
                    }

                    // S-1: resolve the priors ONCE and hand the same map to
                    // both the ranker and the scan summary, so the precision
                    // numbers shown to the user are exactly the ones that
                    // ranked the findings (P4).
                    let resolved_priors =
                        match cntrdct::resolve_priors(no_calibration, priors.as_deref()) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("error: {}", e);
                                return ExitCode::from(1);
                            }
                        };
                    let mut ranked =
                        cntrdct::ranker_from_priors(resolved_priors.clone()).rank(findings);

                    // B-1: apply the baseline filter BEFORE Layer 3
                    // adjudication so `--adjudicate-top` budget goes to new
                    // findings, not to findings the project has already
                    // accepted into the baseline.
                    let mut baseline_suppressed: Option<usize> = None;
                    if let Some(baseline_path) = baseline.as_deref() {
                        let loaded = match cntrdct::baseline::load(baseline_path) {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!("error: {}", e);
                                return ExitCode::from(1);
                            }
                        };
                        let outcome = cntrdct::baseline::filter_ranked(&loaded, ranked, &path);
                        ranked = outcome.kept;
                        baseline_suppressed = Some(outcome.suppressed);
                    }

                    if adjudicate {
                        // Task 1: build the Layer 3 adjudicator per the
                        // selected backend. `anthropic` reads the API key
                        // (HTTP); the CLI backends use subscription auth and
                        // need no key. A missing key / unavailable CLI yields
                        // `None` → the scan continues without verdicts (and
                        // any Layer 0 candidate is suppressed downstream).
                        let adjudicator: Option<Box<dyn cntrdct::core::Adjudicator>> =
                            match adjudicate_via {
                                AdjudicateVia::Anthropic => match cntrdct::read_anthropic_api_key()
                                {
                                    Some(key) => match cntrdct::build_default_adjudicator(key) {
                                        Ok(mut adj) => {
                                            if let Ok(url) =
                                                std::env::var("ANTHROPIC_API_URL_OVERRIDE")
                                            {
                                                adj = adj.with_url(url);
                                            }
                                            Some(Box::new(adj))
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "note: adjudicator init failed; continuing without verdicts ({})",
                                                e
                                            );
                                            None
                                        }
                                    },
                                    None => {
                                        eprintln!(
                                            "note: --adjudicate requested but ANTHROPIC_API_KEY not set; skipping adjudication"
                                        );
                                        None
                                    }
                                },
                                AdjudicateVia::ClaudeCli => {
                                    // Default: claude -p (Haiku) primary with
                                    // an agy (Gemini) usage-cap fallback.
                                    match cntrdct::build_claude_cli_adjudicator_with_agy_fallback()
                                    {
                                        Some(adj) => Some(adj),
                                        None => {
                                            eprintln!(
                                                "note: --adjudicate-via=claude-cli but neither the `claude` nor `agy` CLI is available on PATH; skipping adjudication"
                                            );
                                            None
                                        }
                                    }
                                }
                                AdjudicateVia::AgyCli => match cntrdct::build_agy_cli_adjudicator()
                                {
                                    Some(adj) => Some(adj),
                                    None => {
                                        eprintln!(
                                            "note: --adjudicate-via=agy-cli but the `agy` CLI is unavailable on PATH; skipping adjudication"
                                        );
                                        None
                                    }
                                },
                            };

                        if let Some(adj) = adjudicator.as_deref() {
                            if let Err(e) =
                                cntrdct::adjudicate_top_n(&mut ranked, adj, adjudicate_top)
                            {
                                eprintln!(
                                    "note: adjudication failed; continuing without verdicts ({})",
                                    e
                                );
                            }
                            // R-4 (B5): every Layer 0 candidate is adjudicated
                            // regardless of --adjudicate-top.
                            if layer0_ran {
                                if let Err(e) =
                                    cntrdct::adjudicate_layer0_candidates(&mut ranked, adj)
                                {
                                    eprintln!(
                                        "note: Layer 0 adjudication failed; affected candidates will be suppressed ({})",
                                        e
                                    );
                                }
                            }
                        }
                        // Q-12: post-hoc Platt calibration of LLM confidence.
                        // No-op when the embedded registry is empty (v0
                        // ships {}), so adjudicated findings without a
                        // matching cell keep `calibrated_confidence = None`
                        // and consumers fall back to raw `confidence`.
                        let registry = cntrdct::embedded_platt_registry();
                        cntrdct::apply_llm_calibration(&mut ranked, &registry);
                    }

                    // R-4 (B5 / §3.3): suppress any Layer 0 candidate that
                    // was not adjudicated — an unadjudicated LLM proposal has
                    // no precision floor. Layer 1 findings are never affected.
                    if layer0_ran {
                        let before = ranked.len();
                        ranked.retain(|rf| {
                            rf.finding.origin != cntrdct::core::Origin::Layer0Llm
                                || rf.adjudication.is_some()
                        });
                        let suppressed = before - ranked.len();
                        if suppressed > 0 {
                            eprintln!(
                                "note: suppressed {} unadjudicated Layer 0 candidate(s) from output",
                                suppressed
                            );
                        }
                    }

                    // B-1: record the baseline at the END of the pipeline so
                    // it captures exactly the finding set this scan would
                    // have reported (post-suppression, post-adjudication).
                    if let Some(write_path) = write_baseline.as_deref() {
                        let doc = cntrdct::baseline::build(&ranked, &path);
                        if let Err(e) = cntrdct::baseline::save(write_path, &doc) {
                            eprintln!("error: {}", e);
                            return ExitCode::from(1);
                        }
                        eprintln!(
                            "wrote baseline ({} entr{}, {} finding(s)) to {}",
                            doc.entries.len(),
                            if doc.entries.len() == 1 { "y" } else { "ies" },
                            ranked.len(),
                            write_path.display()
                        );
                    }

                    // S-1: per-detector summary with the corpus-derived
                    // precision the ranker used. Always on stderr so stdout
                    // stays a clean JSON / SARIF document for pipes.
                    eprintln!(
                        "{}",
                        cntrdct::render_scan_summary(
                            &ranked,
                            resolved_priors.as_ref(),
                            parsed_files.len(),
                            baseline_suppressed,
                        )
                    );

                    let output = match format {
                        OutputFormat::Json => serde_json::to_string_pretty(&ranked)
                            .expect("ranked findings serialize cleanly"),
                        OutputFormat::Sarif => {
                            // Mirror the detector set registered by `cntrdct::scan`
                            // so the rules taxonomy in the SARIF output matches the
                            // detectors that produced the findings. Kept in sync
                            // with `cntrdct::ALL_DETECTOR_IDS` via Q-4 wiring test.
                            let clone_drift = CloneDrift::new();
                            let arg_swap = ArgSwap::new();
                            let comment_code = CommentCode::new();
                            let unreachable = UnreachableAfterTerminator::new();
                            let config_interaction = ConfigInteraction::new();
                            let pr_miner = PrMinerDetector::new();
                            let python_unreachable_except = PythonUnreachableExcept::new();
                            let go_build_tag_interaction = GoBuildTagInteraction::new();
                            let detectors: Vec<&dyn Detector> = vec![
                                &clone_drift,
                                &arg_swap,
                                &comment_code,
                                &unreachable,
                                &config_interaction,
                                &pr_miner,
                                &python_unreachable_except,
                                &go_build_tag_interaction,
                            ];
                            cntrdct::sarif::to_sarif_with_rules_pretty_ranked(&ranked, &detectors)
                        }
                    };
                    println!("{}", output);

                    // B-1: exit-code policy, applied after baseline
                    // filtering (only reported findings can fail the run).
                    // The run that WRITES a baseline accepts its findings by
                    // definition, so enforcement is skipped with a note.
                    if write_baseline.is_some() {
                        if fail_on != FailOn::Never {
                            eprintln!(
                                "note: --fail-on is not enforced on the run that writes the baseline"
                            );
                        }
                        return ExitCode::SUCCESS;
                    }
                    let offenders = ranked
                        .iter()
                        .filter(|rf| match fail_on {
                            FailOn::Never => false,
                            FailOn::Error => {
                                matches!(rf.finding.raw_severity, cntrdct::core::Severity::Error)
                            }
                            FailOn::Warning => matches!(
                                rf.finding.raw_severity,
                                cntrdct::core::Severity::Error | cntrdct::core::Severity::Warning
                            ),
                        })
                        .count();
                    if offenders > 0 {
                        eprintln!(
                            "fail-on: {} finding(s) at or above the configured severity",
                            offenders
                        );
                        return ExitCode::from(3);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Commands::Eval {
            corpus_dir,
            manifest,
            against,
        } => {
            let manifest_path = manifest.unwrap_or_else(|| corpus_dir.join("manifest.jsonl"));
            let report = match cntrdct::run_eval(&corpus_dir, &manifest_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::from(1);
                }
            };
            match against {
                None => {
                    let body = serde_json::to_string_pretty(&report)
                        .expect("EvalReport serializes cleanly");
                    println!("{}", body);
                    ExitCode::SUCCESS
                }
                Some(prev_path) => {
                    let previous = match cntrdct::self_replication::load_eval_snapshot(&prev_path) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("error: {}", e);
                            return ExitCode::from(1);
                        }
                    };
                    let delta = cntrdct::self_replication::assemble_report(&report, &previous);
                    let body = serde_json::to_string_pretty(&delta)
                        .expect("SelfReplicationDelta serializes cleanly");
                    println!("{}", body);
                    if !delta.has_baseline {
                        eprintln!(
                            "note: no snapshot line for corpus `{}` in {}; reported as baseline",
                            delta.corpus,
                            prev_path.display()
                        );
                    }
                    ExitCode::SUCCESS
                }
            }
        }
        Commands::CrossModelKappa { corpus, output } => {
            match cntrdct::run_cross_model_audit(&corpus) {
                Ok(report) => {
                    let body = report.to_json_pretty();
                    match output {
                        Some(path) => {
                            if let Err(e) = cntrdct::write_cross_model_audit(&path, &report) {
                                eprintln!("error: {}", e);
                                return ExitCode::from(1);
                            }
                            eprintln!(
                                "wrote cross-model κ audit ({} cells) to {}",
                                report.cells.len(),
                                path.display()
                            );
                        }
                        None => {
                            println!("{}", body);
                        }
                    }
                    if let Some(worst) = &report.worst_cell {
                        eprintln!(
                            "worst cell: {}:{:?} pair={} κ={:.3}",
                            worst.detector_id, worst.anomaly_class, worst.pair, worst.kappa,
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Commands::Calibrate {
            corpus,
            output,
            fit_platt,
            audit_recall,
            manifest,
        } => {
            if audit_recall {
                let manifest_path = manifest.unwrap_or_else(|| corpus.join("manifest.jsonl"));
                match cntrdct::run_recall_audit(&corpus, &manifest_path) {
                    Ok(report) => {
                        let body = serde_json::to_string_pretty(&report)
                            .expect("RecallAuditReport serializes cleanly");
                        match output {
                            Some(path) => {
                                if let Err(e) = std::fs::write(&path, &body) {
                                    eprintln!("error: writing {}: {}", path.display(), e);
                                    return ExitCode::from(1);
                                }
                                eprintln!(
                                    "wrote recall audit ({} detectors, recall_upper_bound={:.3}) to {}",
                                    report.per_detector.len(),
                                    report.overall.recall_upper_bound,
                                    path.display()
                                );
                            }
                            None => println!("{}", body),
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: {}", e);
                        ExitCode::from(1)
                    }
                }
            } else if fit_platt {
                let output_path = output.unwrap_or_else(|| {
                    PathBuf::from("benchmarks/llm-calibration/platt-default.json")
                });
                match cntrdct::fit_platt_calibration(&corpus, &output_path) {
                    Ok(n) => {
                        eprintln!(
                            "wrote Platt parameters for {} cells to {}",
                            n,
                            output_path.display()
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: {}", e);
                        ExitCode::from(1)
                    }
                }
            } else {
                let output_path = output
                    .or_else(cntrdct::default_priors_path)
                    .unwrap_or_else(|| PathBuf::from("priors.json"));
                match cntrdct::calibrate(&corpus, &output_path) {
                    Ok(n) => {
                        eprintln!(
                            "wrote priors for {} detectors to {}",
                            n,
                            output_path.display()
                        );
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: {}", e);
                        ExitCode::from(1)
                    }
                }
            }
        }
    }
}
