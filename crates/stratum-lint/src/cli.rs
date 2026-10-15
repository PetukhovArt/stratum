use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Terminal,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum BriefFormat {
    #[default]
    Agent,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "stratum-lint", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    pub root: Utf8PathBuf,

    #[arg(long, value_enum, default_value = "terminal")]
    pub format: Format,

    #[arg(long)]
    pub config: Option<Utf8PathBuf>,

    #[arg(long)]
    pub watch: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Lint {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
    },
    Init {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
    },
    Snapshot {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long, default_value = "stratum.snapshot.json")]
        out: Utf8PathBuf,
    },
    Diff {
        prev: Utf8PathBuf,
        now: Utf8PathBuf,
    },
    Visualize {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
    },
    /// Explain one rule: what it guards, why, how to fix a hit, its options
    Explain {
        slug: String,
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long, value_enum, default_value = "agent")]
        format: BriefFormat,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
    },
    /// List active rules with effective severity and a one-line summary
    Rules {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long, value_enum, default_value = "agent")]
        format: BriefFormat,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
    },
    /// Where a file (existing or planned) sits and what it may import
    Context {
        file: Utf8PathBuf,
        /// Project root; defaults to the nearest ancestor of FILE holding
        /// stratum.config.jsonc, else package.json, else the current directory
        #[arg(long)]
        root: Option<Utf8PathBuf>,
        #[arg(long, value_enum, default_value = "agent")]
        format: BriefFormat,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
    },
}
