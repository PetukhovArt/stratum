#![forbid(unsafe_code)]

use clap::Parser;
use stratum_lint::{cli, commands, watch};

fn main() -> miette::Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        Some(cli::Command::Init { root }) => commands::init::run(&root),
        Some(cli::Command::Snapshot { root, out }) => commands::snapshot::run(&root, &out),
        Some(cli::Command::Visualize { root }) => commands::visualize::run(&root),
        Some(cli::Command::Diff { prev, now }) => commands::diff::run(&prev, &now),
        Some(cli::Command::Explain {
            slug,
            root,
            format,
            config,
        }) => exit_with(commands::rules::explain(
            &slug,
            &root,
            format,
            config.as_deref(),
        )),
        Some(cli::Command::Rules {
            root,
            format,
            config,
        }) => exit_with(commands::rules::list(&root, format, config.as_deref())),
        Some(cli::Command::Context {
            file,
            root,
            format,
            config,
        }) => exit_with(commands::context::run(
            &file,
            root.as_deref(),
            format,
            config.as_deref(),
        )),
        cmd => {
            let root = if let Some(cli::Command::Lint { root }) = cmd {
                root
            } else {
                cli.root.clone()
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

fn exit_with(result: miette::Result<i32>) -> ! {
    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{e:?}");
            std::process::exit(2);
        }
    }
}
