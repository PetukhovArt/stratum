#![forbid(unsafe_code)]

use clap::Parser;

mod cli;
mod pipeline;

fn main() -> miette::Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        Some(cli::Command::Init { root }) => {
            println!("stratum-lint init at {root}");
        }
        Some(cli::Command::Snapshot { root, out }) => {
            println!("stratum-lint snapshot at {root} → {out}");
        }
        Some(cli::Command::Visualize { root }) => {
            println!("stratum-lint visualize at {root} (Phase 8)");
        }
        Some(cli::Command::Diff { prev, now }) => {
            println!("stratum-lint diff {prev} {now}");
        }
        _ => {
            let root = match cli.command {
                Some(cli::Command::Lint { ref root }) => root.clone(),
                _ => cli.root.clone(),
            };
            let config_path = cli
                .config
                .clone()
                .unwrap_or_else(|| root.join("stratum.config.jsonc"));
            let config = stratum_config::parse_file(&config_path)
                .map_err(|e| miette::Report::msg(e.to_string()))?;
            let violations =
                pipeline::run(&root, &config).map_err(|e| miette::Report::msg(e.to_string()))?;
            println!("Found {} violations", violations.len());
        }
    }
    Ok(())
}
