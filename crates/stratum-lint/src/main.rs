#![forbid(unsafe_code)]

use clap::Parser;

mod cli;
mod commands;
mod engine;
mod pipeline;
mod reporters;
mod zero_config;

fn main() -> miette::Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        Some(cli::Command::Init { root }) => commands::init::run(&root),
        Some(cli::Command::Snapshot { root, out }) => commands::snapshot::run(&root, &out),
        Some(cli::Command::Visualize { root }) => {
            println!("stratum-lint visualize at {root} (Phase 8 stub)");
            Ok(())
        }
        Some(cli::Command::Diff { prev, now }) => {
            println!("stratum-lint diff {prev} {now} (Phase 4 stub)");
            Ok(())
        }
        cmd => {
            let root = match cmd {
                Some(cli::Command::Lint { root }) => root,
                _ => cli.root.clone(),
            };
            let code =
                commands::lint::run(&root, cli.format, cli.config.as_deref())?;
            std::process::exit(code);
        }
    }
}
