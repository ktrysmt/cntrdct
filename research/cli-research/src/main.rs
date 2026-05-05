use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "cntrdct-research",
    about = "Research / empirical-study CLI for cntrdct (corpus fetch, aggregate, overlap, clippy-harness, sample, rank)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run `cargo clippy --message-format=json` over every crate listed
    /// in a corpus manifest and emit per-crate diagnostic JSON files.
    Clippy {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = false)]
        accept_arbitrary_code: bool,
    },
    /// Aggregate scan findings into per-(crate, detector) firing counts.
    Aggregate {
        #[arg(long)]
        findings: PathBuf,
        #[arg(long)]
        corpus_root: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the cntrdct × clippy overlap matrix on a long-format CSV.
    Overlap {
        #[arg(long)]
        findings: PathBuf,
        #[arg(long)]
        clippy_dir: PathBuf,
        #[arg(long)]
        corpus_root: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Stratified random sample N findings per detector for manual labelling.
    Sample {
        #[arg(long)]
        findings: PathBuf,
        #[arg(long, default_value_t = 30)]
        per_detector: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the top-N crates by lifetime download count from a
    /// previously downloaded crates.io DB dump.
    Rank {
        #[arg(long)]
        dump: PathBuf,
        #[arg(long, default_value_t = 100)]
        top: usize,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Fetch crate sources from crates.io into an analysis corpus.
    Fetch {
        crates_list: PathBuf,
        #[arg(long, default_value = "corpus/wild")]
        out: PathBuf,
        #[arg(long)]
        licenses: Option<String>,
        #[arg(long, default_value_t = 50)]
        max_file_kb: u64,
        #[arg(long, default_value_t = 8)]
        jobs: usize,
        #[arg(long, default_value_t = false)]
        resume: bool,
        #[arg(long, value_enum, default_value_t = ProgressFormat::Text)]
        progress: ProgressFormat,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ProgressFormat {
    Text,
    Ndjson,
}

impl From<ProgressFormat> for cntrdct_research::FetchProgress {
    fn from(p: ProgressFormat) -> Self {
        match p {
            ProgressFormat::Text => cntrdct_research::FetchProgress::Text,
            ProgressFormat::Ndjson => cntrdct_research::FetchProgress::NdJson,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Clippy {
            manifest,
            out,
            accept_arbitrary_code,
        } => match cntrdct_research::run_clippy_harness(&manifest, &out, accept_arbitrary_code) {
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
        } => match cntrdct_research::run_aggregate(&findings, &corpus_root, out.as_deref()) {
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
        } => match cntrdct_research::run_overlap(&findings, &clippy_dir, &corpus_root, out.as_deref()) {
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
        } => match cntrdct_research::run_sample(&findings, per_detector, seed, out.as_deref()) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(1)
            }
        },
        Commands::Rank { dump, top, output } => {
            match cntrdct_research::run_rank(&dump, top, output.as_deref()) {
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
            match cntrdct_research::run_fetch(
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
