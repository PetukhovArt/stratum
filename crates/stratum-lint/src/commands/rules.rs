use std::fmt::Write as _;

use camino::Utf8Path;
use stratum_config::Config;
use stratum_core::severity::Severity;
use stratum_rules::{DynRule, RuleRegistry};

use stratum_engine::pipeline;

use crate::{cli::BriefFormat, commands::lint};

pub fn list(
    root: &Utf8Path,
    format: BriefFormat,
    config_path: Option<&Utf8Path>,
) -> miette::Result<i32> {
    let (config, registry) = load(root, config_path)?;
    let (active, off): (Vec<&dyn DynRule>, Vec<&dyn DynRule>) = registry
        .rules()
        .iter()
        .map(AsRef::as_ref)
        .partition(|r| severity_of(&config, *r) != Severity::Off);
    match format {
        BriefFormat::Agent => {
            let mut out = format!("{} active rules", active.len());
            if !off.is_empty() {
                let names: Vec<&str> = off.iter().map(|r| r.slug()).collect();
                let _ = write!(out, " ({} off: {})", off.len(), names.join(", "));
            }
            out.push('\n');
            for r in &active {
                let _ = writeln!(
                    out,
                    "{:<7} {}  {}",
                    severity_of(&config, *r).to_string(),
                    r.slug(),
                    summary_of(*r)
                );
            }
            print!("{out}");
        }
        BriefFormat::Json => {
            let rules: Vec<serde_json::Value> = registry
                .rules()
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "slug": r.slug(),
                        "severity": severity_of(&config, r.as_ref()),
                        "default_severity": r.default_severity(),
                        "summary": r.doc().map(|d| d.summary),
                    })
                })
                .collect();
            print_json(&serde_json::json!({ "version": 1, "rules": rules }))?;
        }
    }
    Ok(0)
}

pub fn explain(
    slug: &str,
    root: &Utf8Path,
    format: BriefFormat,
    config_path: Option<&Utf8Path>,
) -> miette::Result<i32> {
    let (config, registry) = load(root, config_path)?;
    let Some(rule) = registry
        .by_slug(slug)
        .or_else(|| registry.by_slug(&format!("stratum/{slug}")))
    else {
        let valid: Vec<&str> = registry.rules().iter().map(|r| r.slug()).collect();
        return Err(miette::miette!(
            "unknown rule '{slug}'; valid slugs: {}",
            valid.join(", ")
        ));
    };
    let severity = severity_of(&config, rule);
    let doc = rule.doc();
    match format {
        BriefFormat::Agent => {
            let mut out = format!(
                "{} (severity {severity}, default {})\n",
                rule.slug(),
                rule.default_severity()
            );
            match &doc {
                Some(d) => {
                    let _ = writeln!(out, "what: {}\nwhy: {}\nfix: {}", d.summary, d.why, d.fix);
                    if d.options.is_empty() {
                        out.push_str("options: none\n");
                    } else {
                        out.push_str("options:\n");
                        for o in &d.options {
                            let _ = writeln!(out, "  {o}");
                        }
                    }
                }
                None => out.push_str("no documentation: this rule does not override doc()\n"),
            }
            print!("{out}");
        }
        BriefFormat::Json => print_json(&serde_json::json!({
            "version": 1,
            "slug": rule.slug(),
            "severity": severity,
            "default_severity": rule.default_severity(),
            "doc": doc,
        }))?,
    }
    Ok(0)
}

fn load(root: &Utf8Path, config_path: Option<&Utf8Path>) -> miette::Result<(Config, RuleRegistry)> {
    let config = lint::load_config(root, config_path)?.config;
    let registry = pipeline::registry(root, &config).map_err(|e| miette::miette!("{e}"))?;
    Ok((config, registry))
}

fn severity_of(config: &Config, rule: &dyn DynRule) -> Severity {
    config
        .rules
        .get(rule.slug())
        .map_or_else(|| rule.default_severity(), |rc| rc.severity)
}

fn summary_of(rule: &dyn DynRule) -> String {
    rule.doc()
        .map_or_else(|| "(no documentation)".to_string(), |d| d.summary)
}

fn print_json(value: &serde_json::Value) -> miette::Result<()> {
    let s = serde_json::to_string_pretty(value).map_err(|e| miette::miette!("{e}"))?;
    println!("{s}");
    Ok(())
}
