use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Terminal,
    Json,
    Sarif,
    /// Compact, token-cheap text for a model's context window.
    Agent,
    /// GitLab Code Quality report (JSON array)
    Gitlab,
    /// GitHub Actions workflow annotations, then the agent text
    Github,
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

    #[command(flatten)]
    pub lint: LintArgs,
}

#[derive(Debug, Clone, Args)]
pub struct LintArgs {
    #[arg(long, value_enum, default_value = "terminal")]
    pub format: Format,

    #[arg(long)]
    pub config: Option<Utf8PathBuf>,

    #[arg(long)]
    pub watch: bool,

    /// Cap printed violations in the agent format; the rest is summarised (default 20)
    #[arg(long)]
    pub max: Option<usize>,

    /// Report only violations whose file or import edge touches one of these files
    #[arg(long, num_args = 1..)]
    pub files: Vec<Utf8PathBuf>,

    /// Report only violations touching files changed vs REF (default HEAD) plus untracked files
    #[arg(long, value_name = "REF", num_args = 0..=1, require_equals = true, default_missing_value = "HEAD")]
    pub changed: Option<String>,

    /// Baseline file (default: ROOT/stratum.baseline.json if it exists)
    #[arg(long, conflicts_with = "no_baseline")]
    pub baseline: Option<Utf8PathBuf>,

    #[arg(long)]
    pub no_baseline: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Lint {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[command(flatten)]
        args: LintArgs,
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
        #[arg(long)]
        config: Option<Utf8PathBuf>,
    },
    Visualize {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
        /// Also serve the drift of the working tree vs REF at /api/drift (Merge request view)
        #[arg(long, value_name = "REF")]
        base: Option<String>,
        /// Serve without opening the default browser (for scripted or agent runs)
        #[arg(long)]
        no_open: bool,
    },
    /// Accept the current violations into a baseline file
    Baseline {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
        /// Default: ROOT/stratum.baseline.json
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        /// Only drop entries that are fixed; never add new ones
        #[arg(long)]
        prune: bool,
        /// Fail if a baseline entry count grew vs the baseline committed at REF (default HEAD)
        #[arg(
            long,
            value_name = "REF",
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "HEAD",
            conflicts_with_all = ["prune", "config"]
        )]
        check: Option<String>,
    },
    /// Architecture drift of the working tree vs REF, as markdown
    Diff {
        #[arg(default_value = ".")]
        root: Utf8PathBuf,
        #[arg(long, value_name = "REF")]
        base: String,
        #[arg(long)]
        config: Option<Utf8PathBuf>,
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
