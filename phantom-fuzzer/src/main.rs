use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use phantom_fuzzer::{plan_once, ChaosProfile, Corpus, FuzzerConfig, Result};

#[derive(Debug, Parser)]
#[command(
    name = "phantom-fuzz",
    version,
    about = "Phantom Engine fuzzer planner"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Creates the corpus directory layout.
    Init {
        #[arg(long, default_value = ".phantom-fuzzer")]
        root: PathBuf,
    },
    /// Imports one HTML file into the seed corpus.
    Import {
        #[arg(long, default_value = ".phantom-fuzzer")]
        root: PathBuf,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        label: String,
        #[arg(long, default_value = "manual")]
        source: String,
    },
    /// Emits a plan and payload files. This does not execute them.
    Plan {
        #[arg(long, default_value = ".phantom-fuzzer")]
        root: PathBuf,
        #[arg(long, default_value = ".phantom-fuzzer/plans")]
        out: PathBuf,
        #[arg(long, default_value_t = 12)]
        count: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = ChaosProfile::Responsible)]
        profile: ChaosProfile,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init { root } => {
            let corpus = Corpus::init(root)?;
            println!("initialized {}", corpus.root().display());
        }
        Cmd::Import {
            root,
            file,
            label,
            source,
        } => {
            let corpus = Corpus::init(root)?;
            let seed = corpus.import_html(file, label, source)?;
            println!("imported {}", seed.id);
        }
        Cmd::Plan {
            root,
            out,
            count,
            seed,
            profile,
        } => {
            let cfg = FuzzerConfig {
                corpus_root: root,
                out_root: out,
                plan_count: count,
                rng_seed: seed,
                profile,
                ..FuzzerConfig::default()
            };
            let path = plan_once(cfg)?;
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}
