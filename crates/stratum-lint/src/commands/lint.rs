use std::io::Write;
use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_engine::{
    BASELINE_FILE, Baseline, EngineInput, Finding, LoadedConfig, paths, pipeline,
};

use crate::{
    cli::{Format, LintArgs},
    git,
    report::{Report, Summary},
    reporters::{
        AgentReporter, GithubReporter, GitlabReporter, JsonReporter, JsonlReporter, Reporter,
        SarifReporter, TerminalReporter,
    },
    scope::Scope,
};

pub fn run(root: &Utf8Path, args: &LintArgs) -> miette::Result<i32> {
    let report = gate(root, args)?;
    let mut stdout = std::io::stdout().lock();
    render(root, args, &report, &mut stdout).map_err(|e| miette::miette!("{e}"))?;
    stdout.flush().ok();
    Ok(report.exit_code())
}

fn render(
    root: &Utf8Path,
    args: &LintArgs,
    report: &Report,
    w: &mut dyn Write,
) -> std::io::Result<()> {
    let prefix = || git::prefix(root).unwrap_or_default();
    match args.format {
        Format::Gitlab => GitlabReporter::new(prefix()).write(report, w),
        Format::Github => GithubReporter::new(prefix()).write(report, w),
        Format::Terminal => TerminalReporter.write(report, w),
        Format::Json => JsonReporter.write(report, w),
        Format::Jsonl => JsonlReporter.write(report, w),
        Format::Sarif => SarifReporter.write(report, w),
        Format::Agent => AgentReporter {
            max: args.max.unwrap_or(AgentReporter::DEFAULT_MAX),
        }
        .write(report, w),
    }
}

fn gate(root: &Utf8Path, args: &LintArgs) -> miette::Result<Report> {
    let started = Instant::now();
    let loaded = load_config(root, args.config.as_deref())?;
    let baseline = load_baseline(root, args)?;
    let root_abs =
        paths::canonicalize(root).map_err(|e| miette::miette!("Project root {root}: {e}"))?;
    let scope = Scope::resolve(&root_abs, &args.files, args.changed.as_deref())?;
    let mut summary = Summary {
        scope_files: scope.as_ref().map(Scope::len),
        zero_config: loaded.is_inferred(),
        ..Summary::default()
    };
    if scope.as_ref().is_some_and(Scope::is_empty) {
        summary.elapsed = started.elapsed();
        return Ok(Report::new(Vec::new(), Vec::new(), summary));
    }

    let input = build(root, &loaded)?;
    summary.cache = Some(input.cache);
    let analysis = input.analyze().map_err(|e| miette::miette!("{e}"))?;
    let in_scope = |f: &Finding| scope.as_ref().is_none_or(|s| s.touches(f));
    let matched = baseline.unwrap_or_default().apply(analysis.findings);
    summary.fixed = matched.fixed();
    summary.fixed_in_scope =
        matched.fixed_where(|file| scope.as_ref().is_none_or(|s| s.covers(file)));
    summary.baselined = matched.baselined.iter().filter(|f| in_scope(f)).count();
    let kept = matched.new.into_iter().filter(|f| in_scope(f)).collect();
    let suppressed = analysis
        .suppressed
        .into_iter()
        .filter(|s| in_scope(&s.finding))
        .collect();
    summary.elapsed = started.elapsed();
    Ok(Report::new(kept, suppressed, summary))
}

pub fn load_config(root: &Utf8Path, path: Option<&Utf8Path>) -> miette::Result<LoadedConfig> {
    stratum_engine::config::load(root, path).map_err(|e| miette::miette!("{e}"))
}

pub fn build(root: &Utf8Path, loaded: &LoadedConfig) -> miette::Result<EngineInput> {
    pipeline::build(root, loaded).map_err(|e| miette::miette!("{e}"))
}

pub fn findings(input: &EngineInput) -> miette::Result<Vec<Finding>> {
    input.violations().map_err(|e| miette::miette!("{e}"))
}

fn load_baseline(root: &Utf8Path, args: &LintArgs) -> miette::Result<Option<Baseline>> {
    let loaded = match (&args.baseline, args.no_baseline) {
        (_, true) => Ok(None),
        (Some(p), false) => Baseline::load(p).map(Some),
        (None, false) => Baseline::load_if_exists(&default_baseline_path(root)),
    };
    loaded.map_err(|e| miette::miette!("{e}"))
}

#[must_use]
pub fn default_baseline_path(root: &Utf8Path) -> Utf8PathBuf {
    root.join(BASELINE_FILE)
}
