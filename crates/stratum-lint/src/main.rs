#![forbid(unsafe_code)]

use clap::Parser;

mod cli;
mod commands;
mod engine;
mod pipeline;
mod reporters;
mod watch;
mod zero_config;

fn main() -> miette::Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        Some(cli::Command::Init { root }) => commands::init::run(&root),
        Some(cli::Command::Snapshot { root, out }) => commands::snapshot::run(&root, &out),
        Some(cli::Command::Visualize { root }) => commands::visualize::run(&root),
        Some(cli::Command::Diff { prev, now }) => commands::diff::run(&prev, &now),
        cmd => {
            let root = match cmd {
                Some(cli::Command::Lint { root }) => root,
                _ => cli.root.clone(),
            };
            if cli.watch {
                let format = cli.format;
                let config_path = cli.config.clone();
                let root_for_lint = root.clone();
                return watch::run(&root, || {
                    commands::lint::run(&root_for_lint, format, config_path.as_deref())
                });
            }
            let code = commands::lint::run(&root, cli.format, cli.config.as_deref())?;
            std::process::exit(code);
        }
    }
}
