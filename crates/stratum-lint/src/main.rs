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

fn hook(max: Option<usize>) -> u8 {
    match commands::hook::run(max) {
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
        Some(cli::Command::Visualize {
            root,
            config,
            no_open,
        }) => commands::visualize::run(&root, config.as_deref(), !no_open).map(|()| 0),
        Some(cli::Command::Baseline {
            root,
            config,
            out,
            prune,
        }) => commands::baseline::run(&root, config.as_deref(), out.as_deref(), prune).map(|()| 0),
        Some(cli::Command::Hook { max }) => Ok(hook(max)),
        Some(cli::Command::Explain {
            slug,
            root,
            format,
            config,
        }) => commands::rules::explain(&slug, &root, format, config.as_deref()).map(exit_code),
        Some(cli::Command::Rules {
            root,
            format,
            config,
        }) => commands::rules::list(&root, format, config.as_deref()).map(exit_code),
        Some(cli::Command::Context {
            file,
            root,
            format,
            config,
        }) => {
            commands::context::run(&file, root.as_deref(), format, config.as_deref()).map(exit_code)
        }
        Some(cli::Command::Lint { root, args }) => lint(&root, &args),
        None => lint(&cli.root, &cli.lint),
    }
}

fn lint(root: &camino::Utf8Path, args: &cli::LintArgs) -> miette::Result<u8> {
    if args.watch {
        watch::run(root, || commands::lint::run(root, args))?;
        return Ok(0);
    }
    commands::lint::run(root, args).map(exit_code)
}

fn exit_code(code: i32) -> u8 {
    u8::try_from(code).unwrap_or(TOOL_FAILURE)
}
