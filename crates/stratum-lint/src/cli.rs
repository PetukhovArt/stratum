use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Terminal,
    Json,
    Sarif,
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
        /// Bind to a fixed TCP port. Default 0 = OS-assigned. Use 8080 to pair
        /// with the Vite dev server (which proxies /api to 127.0.0.1:8080).
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
}
