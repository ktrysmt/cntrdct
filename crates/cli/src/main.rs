use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use cntrdct_core::Detector;
use cntrdct_detector_arg_swap::ArgSwap;
use cntrdct_detector_clone_drift::CloneDrift;
use cntrdct_detector_comment_code::CommentCode;
use cntrdct_detector_config_interaction::ConfigInteraction;
use cntrdct_detector_unreachable_after_terminator::UnreachableAfterTerminator;

#[derive(Parser)]
#[command(name = "cntrdct", about = "Evidence-based contradiction linter")]
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
    Calibrate {
        /// Path to a labelled JSONL corpus.
        corpus: PathBuf,
        /// Output path for the priors file. Defaults to
        /// `<cache_dir>/cntrdct/priors.json`.
        #[arg(long)]
        output: Option<PathBuf>,
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
    /// Run `cargo clippy --message-format=json` over every crate listed
    /// in a corpus manifest and emit per-crate diagnostic JSON files.
    ///
    /// Refuses to start without `--accept-arbitrary-code`. cargo invokes
    /// build.rs and proc macros from the target crate, which are
    /// arbitrary code from crates.io. Run this only inside an isolated
    /// environment (container, VM, dedicated user). The flag is the
    /// caller's acknowledgement of that contract.
    Clippy {
        /// Path to the corpus manifest (`<out>/manifest.csv` from
        /// `cntrdct fetch`).
        #[arg(long)]
        manifest: PathBuf,
        /// Output directory for per-crate JSON files plus
        /// `summary.json`. Created if absent.
        #[arg(long)]
        out: PathBuf,
        /// Acknowledge that this command compiles third-party crates,
        /// which executes build scripts and proc macros from crates.io.
        #[arg(long, default_value_t = false)]
        accept_arbitrary_code: bool,
    },
    /// Aggregate scan findings into per-(crate, detector) firing counts.
    ///
    /// Reads a JSON `Vec<RankedFinding>` (the output of
    /// `cntrdct scan ... --format json`) and bins each finding by the
    /// `<name>-<version>` directory under `--corpus-root`. Emits a CSV
    /// with columns `crate_dir, detector, count`, suitable for joining
    /// against `manifest.csv` to attach license / downloads columns.
    Aggregate {
        /// Path to a JSON findings file produced by `cntrdct scan`.
        #[arg(long)]
        findings: PathBuf,
        /// Corpus root that the findings' `primary.file` paths live under.
        #[arg(long)]
        corpus_root: PathBuf,
        /// Output CSV path. Stdout when omitted.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the cntrdct × clippy overlap matrix on a long-format CSV.
    ///
    /// For each cntrdct finding, looks up the clippy diagnostics produced
    /// by `cntrdct clippy` at the same `(crate_dir, rel_path, start_line)`
    /// triple. Emits one row per (detector, clippy_lint) pair with a
    /// nonzero co-occurrence count, sorted lexicographically.
    Overlap {
        /// Path to a JSON findings file produced by `cntrdct scan`.
        #[arg(long)]
        findings: PathBuf,
        /// Directory of `<name>-<version>.clippy.json` files produced by
        /// `cntrdct clippy`.
        #[arg(long)]
        clippy_dir: PathBuf,
        /// Corpus root that the findings' `primary.file` paths live under.
        #[arg(long)]
        corpus_root: PathBuf,
        /// Output CSV path. Stdout when omitted.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Stratified random sample N findings per detector for manual labelling.
    ///
    /// Reproducible across runs given the same `--seed`. The sample is
    /// emitted as a JSON `Vec<RankedFinding>` so it can be fed back into
    /// any tool that consumes the original findings format (a labelling
    /// UI, a diff against the next run, etc.).
    Sample {
        /// Path to a JSON findings file produced by `cntrdct scan`.
        #[arg(long)]
        findings: PathBuf,
        /// Number of findings to draw from each detector group.
        #[arg(long, default_value_t = 30)]
        per_detector: usize,
        /// Seed for the deterministic shuffle.
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// Output JSON path. Stdout when omitted.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the top-N crates by lifetime download count from a
    /// previously downloaded crates.io DB dump and emit a crate-list file
    /// suitable for `cntrdct fetch`.
    ///
    /// The dump itself is not fetched here — download it once with curl
    /// (or any HTTP client) from `https://static.crates.io/db-dump.tar.gz`,
    /// then point `--dump` at the saved archive. Keeping download and rank
    /// as separate operations means a slow 1 GB pull is paid exactly once
    /// per study iteration.
    Rank {
        /// Path to a saved `db-dump.tar.gz`.
        #[arg(long)]
        dump: PathBuf,
        /// Number of top crates to emit.
        #[arg(long, default_value_t = 100)]
        top: usize,
        /// Where to write the ranking. Stdout when omitted.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Fetch crate sources from crates.io into an analysis corpus.
    ///
    /// Reads a list of crate names (one per line; `#` comments allowed),
    /// pulls the latest non-yanked version of each via the Sparse Index,
    /// applies a permissive-license allowlist, downloads + verifies the
    /// `.crate` tarball against its `cksum`, extracts the filtered subset
    /// (`.rs` files <= 50 KB outside `tests/target/examples/benches`) into
    /// `<out>/<name>-<version>/`, and appends rows to `<out>/manifest.csv`.
    ///
    /// First slice of the Track A empirical study; the DB-dump-driven
    /// downloads ranking lands in a separate change.
    Fetch {
        /// Path to a newline-delimited list of crate names.
        crates_list: PathBuf,
        /// Output corpus directory (created if absent).
        #[arg(long, default_value = "corpus/wild")]
        out: PathBuf,
        /// Comma-separated SPDX identifiers to allow. Defaults to the
        /// study's narrow list (MIT, Apache-2.0, BSD-3-Clause, ISC).
        #[arg(long)]
        licenses: Option<String>,
        /// Maximum kept-file size in kilobytes.
        #[arg(long, default_value_t = 50)]
        max_file_kb: u64,
        /// Maximum concurrent crate fetches. The default 8 is a politeness
        /// ceiling for crates.io's static endpoint; set to 1 for a fully
        /// sequential run.
        #[arg(long, default_value_t = 8)]
        jobs: usize,
        /// Skip crates that already appear in `<out>/manifest.csv`. Lets
        /// an interrupted run pick up where it left off without
        /// re-downloading.
        #[arg(long, default_value_t = false)]
        resume: bool,
        /// Progress format on stderr. `text` is human-readable; `ndjson`
        /// emits one JSON object per event for piping into log
        /// aggregators.
        #[arg(long, value_enum, default_value_t = ProgressFormat::Text)]
        progress: ProgressFormat,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    /// Pretty-printed JSON of `Vec<RankedFinding>`.
    Json,
    /// SARIF 2.1.0, ordered by rank_score descending.
    Sarif,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ProgressFormat {
    Text,
    Ndjson,
}

impl From<ProgressFormat> for cntrdct_cli::FetchProgress {
    fn from(p: ProgressFormat) -> Self {
        match p {
            ProgressFormat::Text => cntrdct_cli::FetchProgress::Text,
            ProgressFormat::Ndjson => cntrdct_cli::FetchProgress::NdJson,
        }
    }
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
        } => match cntrdct_cli::scan_full(&path) {
            Ok((raw_findings, parsed_files)) => {
                let findings = match cntrdct_cli::apply_suppression(
                    config.as_deref(),
                    &path,
                    &parsed_files,
                    raw_findings,
                ) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("error: {}", e);
                        return ExitCode::from(1);
                    }
                };

                let mut ranked = match cntrdct_cli::rank_with_calibration(
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
                    match cntrdct_cli::read_anthropic_api_key() {
                        Some(key) => match cntrdct_cli::build_default_adjudicator(key) {
                            Ok(mut adj) => {
                                if let Ok(url) = std::env::var("ANTHROPIC_API_URL_OVERRIDE") {
                                    adj = adj.with_url(url);
                                }
                                if let Err(e) =
                                    cntrdct_cli::adjudicate_top_n(&mut ranked, &adj, adjudicate_top)
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
                }

                let output = match format {
                    OutputFormat::Json => serde_json::to_string_pretty(&ranked)
                        .expect("ranked findings serialize cleanly"),
                    OutputFormat::Sarif => {
                        // Mirror the detector set registered by `cntrdct_cli::scan`
                        // so the rules taxonomy in the SARIF output matches the
                        // detectors that produced the findings.
                        let clone_drift = CloneDrift::new();
                        let arg_swap = ArgSwap::new();
                        let comment_code = CommentCode::new();
                        let unreachable = UnreachableAfterTerminator::new();
                        let config_interaction = ConfigInteraction::new();
                        let detectors: Vec<&dyn Detector> = vec![
                            &clone_drift,
                            &arg_swap,
                            &comment_code,
                            &unreachable,
                            &config_interaction,
                        ];
                        cntrdct_sarif::to_sarif_with_rules_pretty_ranked(&ranked, &detectors)
                    }
                };
                println!("{}", output);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Eval {
            corpus_dir,
            manifest,
        } => {
            let manifest_path = manifest.unwrap_or_else(|| corpus_dir.join("manifest.jsonl"));
            match cntrdct_cli::run_eval(&corpus_dir, &manifest_path) {
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
        Commands::Calibrate { corpus, output } => {
            let output_path = output
                .or_else(cntrdct_cli::default_priors_path)
                .unwrap_or_else(|| PathBuf::from("priors.json"));
            match cntrdct_cli::calibrate(&corpus, &output_path) {
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
        Commands::Clippy {
            manifest,
            out,
            accept_arbitrary_code,
        } => match cntrdct_cli::run_clippy_harness(&manifest, &out, accept_arbitrary_code) {
            Ok(summary) => {
                let body = serde_json::to_string_pretty(&summary)
                    .expect("ClippyHarnessSummary serializes cleanly");
                println!("{}", body);
                if summary.failed_compile.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Aggregate {
            findings,
            corpus_root,
            out,
        } => match cntrdct_cli::run_aggregate(&findings, &corpus_root, out.as_deref()) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Overlap {
            findings,
            clippy_dir,
            corpus_root,
            out,
        } => match cntrdct_cli::run_overlap(&findings, &clippy_dir, &corpus_root, out.as_deref()) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Sample {
            findings,
            per_detector,
            seed,
            out,
        } => match cntrdct_cli::run_sample(&findings, per_detector, seed, out.as_deref()) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Rank { dump, top, output } => {
            match cntrdct_cli::run_rank(&dump, top, output.as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Commands::Fetch {
            crates_list,
            out,
            licenses,
            max_file_kb,
            jobs,
            resume,
            progress,
        } => {
            let allowlist_owned: Vec<String> = match licenses {
                Some(s) => s
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect(),
                None => cntrdct_corpus_fetch::DEFAULT_LICENSE_ALLOWLIST
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            };
            let allowlist: Vec<&str> = allowlist_owned.iter().map(|s| s.as_str()).collect();
            let extract_opts = cntrdct_corpus_fetch::ExtractOptions {
                max_file_bytes: max_file_kb.saturating_mul(1024),
                ..cntrdct_corpus_fetch::ExtractOptions::default()
            };
            match cntrdct_cli::run_fetch(
                &crates_list,
                &out,
                &allowlist,
                &extract_opts,
                jobs,
                resume,
                progress.into(),
            ) {
                Ok(summary) => {
                    let body = serde_json::to_string_pretty(&summary)
                        .expect("FetchSummary serializes cleanly");
                    println!("{}", body);
                    if summary.errors > 0 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
    }
}
