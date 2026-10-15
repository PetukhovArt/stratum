use std::io::Write;
use std::time::Instant;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::Config;

use crate::{
    baseline::{self, Baseline},
    cli::{Format, LintArgs},
    engine::EngineInput,
    pipeline,
    report::{EdgeRef, Finding, Location, Report, Summary, relative},
    reporters::{AgentReporter, JsonReporter, Reporter, SarifReporter, TerminalReporter},
    scope::Scope,
    zero_config,
};

pub fn run(root: &Utf8Path, args: &LintArgs) -> miette::Result<i32> {
    let report = gate(root, args)?;
    let mut stdout = std::io::stdout().lock();
    render(args, &report, &mut stdout).map_err(|e| miette::miette!("{e}"))?;
    stdout.flush().ok();
    Ok(report.exit_code())
}

pub fn render(args: &LintArgs, report: &Report, w: &mut dyn Write) -> std::io::Result<()> {
    match args.format {
        Format::Terminal => TerminalReporter.write(report, w),
        Format::Json => JsonReporter.write(report, w),
        Format::Sarif => SarifReporter.write(report, w),
        Format::Agent => AgentReporter {
            max: args.max.unwrap_or(AgentReporter::DEFAULT_MAX),
        }
        .write(report, w),
    }
}

pub fn gate(root: &Utf8Path, args: &LintArgs) -> miette::Result<Report> {
    let started = Instant::now();
    let (config, zero_config) = load_config(root, args.config.as_deref())?;
    let baseline = load_baseline(root, args)?;
    let root_abs = root
        .canonicalize_utf8()
        .map_err(|e| miette::miette!("Project root {root}: {e}"))?;
    let scope = Scope::resolve(&root_abs, &args.files, args.changed.as_deref())?;
    let mut summary = Summary {
        scope_files: scope.as_ref().map(Scope::len),
        zero_config,
        ..Summary::default()
    };
    if scope.as_ref().is_some_and(Scope::is_empty) {
        summary.elapsed = started.elapsed();
        return Ok(Report::new(Vec::new(), summary));
    }

    let input = pipeline::build(root, &config).map_err(|e| miette::miette!("{e}"))?;
    let all = findings(&input)?;
    let matched = baseline.unwrap_or_default().matches(&all);
    summary.fixed = matched.fixed;
    let mut kept = Vec::new();
    for (f, baselined) in all.into_iter().zip(matched.baselined) {
        if scope.as_ref().is_some_and(|s| !s.touches(&f)) {
            continue;
        }
        if baselined {
            summary.baselined += 1;
        } else {
            kept.push(f);
        }
    }
    summary.elapsed = started.elapsed();
    Ok(Report::new(kept, summary))
}

pub fn load_config(root: &Utf8Path, path: Option<&Utf8Path>) -> miette::Result<(Config, bool)> {
    let parse = |p: &Utf8Path| stratum_config::parse_file(p).map_err(|e| miette::miette!("{e}"));
    if let Some(p) = path {
        return Ok((parse(p)?, false));
    }
    let default = root.join("stratum.config.jsonc");
    if default.exists() {
        Ok((parse(&default)?, false))
    } else {
        Ok((zero_config::infer(root), true))
    }
}

fn load_baseline(root: &Utf8Path, args: &LintArgs) -> miette::Result<Option<Baseline>> {
    if args.no_baseline {
        return Ok(None);
    }
    if let Some(p) = &args.baseline {
        return Baseline::load(p).map(Some);
    }
    let default = default_baseline_path(root);
    if default.exists() {
        Baseline::load(&default).map(Some)
    } else {
        Ok(None)
    }
}

#[must_use]
pub fn default_baseline_path(root: &Utf8Path) -> Utf8PathBuf {
    root.join(baseline::FILE_NAME)
}

pub fn findings(input: &EngineInput) -> miette::Result<Vec<Finding>> {
    let violations = stratum_rules::run_all_with_registry(
        &input.graph,
        &input.config,
        &input.project_root,
        &input.registry,
    )
    .map_err(|e| miette::miette!("{e}"))?;
    let root = &input.project_root;
    let module_path = |id| {
        input
            .graph
            .modules
            .get(&id)
            .map_or_else(|| format!("<module {id:?}>"), |m| relative(&m.path, root))
    };
    Ok(violations
        .into_iter()
        .map(|v| Finding {
            rule: input
                .registry
                .by_id(v.rule)
                .map(stratum_rules::DynRule::slug)
                .or_else(|| stratum_rules::slug_for(v.rule))
                .unwrap_or("unknown")
                .to_owned(),
            severity: v.severity,
            file: relative(&v.file, root),
            location: Location {
                line: v.location.line,
                column: v.location.column,
            },
            edge: v.edge.map(|e| EdgeRef {
                from: module_path(e.from),
                to: module_path(e.to),
                kind: e.kind,
            }),
            members: v.modules.iter().map(|&id| module_path(id)).collect(),
            message: v.message,
            suggestion: v.suggestion,
        })
        .collect())
}
