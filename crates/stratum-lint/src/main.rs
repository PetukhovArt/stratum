#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use stratum_lint::{cli, commands, watch};

const TOOL_FAILURE: u8 = 2;
const HOOK_NON_BLOCKING_ERROR: u8 = 1;

fn main() -> ExitCode {
    match run(cli::Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::from(TOOL_FAILURE)
        }
    }
}

fn hook() -> u8 {
    match commands::hook::run() {
        Ok(Some(decision)) => {
            println!("{decision}");
            0
        }
        Ok(None) => 0,
        Err(e) => {
            eprintln!("{e:?}");
            HOOK_NON_BLOCKING_ERROR
        }
    }
}

fn run(cli: cli::Cli) -> miette::Result<u8> {
    match cli.command {
        Some(cli::Command::Init { root }) => commands::init::run(&root).map(|()| 0),
        Some(cli::Command::Snapshot { root, out }) => {
            commands::snapshot::run(&root, &out).map(|()| 0)
        }
        Some(cli::Command::Visualize { root }) => commands::visualize::run(&root).map(|()| 0),
        Some(cli::Command::Diff { prev, now }) => commands::diff::run(&prev, &now).map(|()| 0),
        Some(cli::Command::Baseline {
            root,
            config,
            out,
            prune,
        }) => commands::baseline::run(&root, config.as_deref(), out.as_deref(), prune).map(|()| 0),
        Some(cli::Command::Hook) => Ok(hook()),
        Some(cli::Command::Lint { root, args }) => lint(&root, &args),
        None => lint(&cli.root, &cli.lint),
    }
}

fn lint(root: &camino::Utf8Path, args: &cli::LintArgs) -> miette::Result<u8> {
    if args.watch {
        watch::run(root, || commands::lint::run(root, args))?;
        return Ok(0);
    }
    commands::lint::run(root, args).map(|code| u8::from(code != 0))
}
