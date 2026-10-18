mod budgets;
mod check;
mod generate;
mod peak;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Release gate: stratum-lint against the budgets in docs/harness.md"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write the deterministic synthetic project (a git repository) into DIR
    Generate {
        dir: PathBuf,
        #[arg(long, default_value_t = generate::MODULES)]
        modules: usize,
    },
    /// Measure the four budget scenarios; exit 1 when any is over budget
    Check {
        /// Measure this project as it is, instead of a generated one
        #[arg(long)]
        project: Option<PathBuf>,
        /// stratum-lint to measure, instead of a fresh release build
        #[arg(long)]
        bin: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        runs: usize,
    },
}

fn run(cli: Cli) -> Result<bool> {
    match cli.command {
        Command::Generate { dir, modules } => {
            generate::generate(&dir, modules)?;
            Ok(true)
        }
        Command::Check { project, bin, runs } => check::check(project, bin, runs),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("stratum-budget: {e}");
            ExitCode::from(2)
        }
    }
}
