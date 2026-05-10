use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use cntrdct::core::Detector;
use cntrdct::detectors::arg_swap::ArgSwap;
use cntrdct::detectors::clone_drift::CloneDrift;
use cntrdct::detectors::comment_code::CommentCode;
use cntrdct::detectors::config_interaction::ConfigInteraction;
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
        /// Override the `cntrdct.toml` config file. When omitted, the scan
        /// root is searched for `cntrdct.toml`; absent file is silently
        /// treated as an empty config.
        #[arg(long)]
        config: Option<PathBuf>,
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
    Calibrate {
        /// Path to a labelled JSONL corpus.
        corpus: PathBuf,
        /// Output path. Defaults to `<cache_dir>/cntrdct/priors.json` for
        /// the priors mode, and `benchmarks/llm-calibration/platt-default.json`
        /// for the Platt mode.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Q-12: fit Platt parameters from a labelled LLM-confidence
        /// corpus instead of computing per-detector priors.
        #[arg(long, default_value_t = false)]
        fit_platt: bool,
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
    },
    /// Q-13: cross-model κ audit. Routes the same finding set through
    /// `claude --print` and `gemini -p`, then reports pairwise Cohen's
    /// κ per `(detector_id, anomaly_class)` cell. Spec:
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
            config,
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
                    let findings = match cntrdct::config::apply(&cfg, &parsed_files, raw_findings) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("error: {}", e);
                            return ExitCode::from(1);
                        }
                    };

                    let mut ranked = match cntrdct::rank_with_calibration(
                        findings,
                        no_calibration,
                        priors.as_deref(),
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("error: {}", e);
                            return ExitCode::from(1);
                        }
                    };

                    if adjudicate {
                        match cntrdct::read_anthropic_api_key() {
                            Some(key) => match cntrdct::build_default_adjudicator(key) {
                                Ok(mut adj) => {
                                    if let Ok(url) = std::env::var("ANTHROPIC_API_URL_OVERRIDE") {
                                        adj = adj.with_url(url);
                                    }
                                    if let Err(e) =
                                        cntrdct::adjudicate_top_n(&mut ranked, &adj, adjudicate_top)
                                    {
                                        eprintln!(
                                        "note: adjudication failed; continuing without verdicts ({})",
                                        e
                                    );
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                    "note: adjudicator init failed; continuing without verdicts ({})",
                                    e
                                );
                                }
                            },
                            None => {
                                eprintln!(
                                "note: --adjudicate requested but ANTHROPIC_API_KEY not set; skipping adjudication"
                            );
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
                            let detectors: Vec<&dyn Detector> = vec![
                                &clone_drift,
                                &arg_swap,
                                &comment_code,
                                &unreachable,
                                &config_interaction,
                                &pr_miner,
                            ];
                            cntrdct::sarif::to_sarif_with_rules_pretty_ranked(&ranked, &detectors)
                        }
                    };
                    println!("{}", output);
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
        } => {
            let manifest_path = manifest.unwrap_or_else(|| corpus_dir.join("manifest.jsonl"));
            match cntrdct::run_eval(&corpus_dir, &manifest_path) {
                Ok(report) => {
                    let body = serde_json::to_string_pretty(&report)
                        .expect("EvalReport serializes cleanly");
                    println!("{}", body);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::from(1)
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
        } => {
            if fit_platt {
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
