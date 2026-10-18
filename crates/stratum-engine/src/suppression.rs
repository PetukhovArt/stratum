use std::collections::HashMap;

use stratum_config::ConfigError;
use stratum_core::{ids::ModuleId, severity::Severity, types::Suppression, violation::Violation};
use stratum_rules::{EffectiveSeverities, SUPPRESSION};

use crate::finding::Finding;
use crate::pipeline::EngineInput;

const SLUG: &str = "stratum/suppression";

#[derive(Debug, Clone)]
pub struct Suppressed {
    pub finding: Finding,
    pub reason: String,
}

/// What the rules found after `// stratum-ignore` directives were applied, before any baseline.
#[derive(Debug, Default)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub suppressed: Vec<Suppressed>,
}

struct Directive<'g> {
    module: ModuleId,
    file: String,
    at: &'g Suppression,
    problems: Vec<String>,
}

impl Directive<'_> {
    fn is_valid(&self) -> bool {
        self.problems.is_empty()
    }

    fn names(&self) -> String {
        if self.at.slugs.is_empty() {
            return "<none>".to_owned();
        }
        self.at.slugs.join(", ")
    }
}

pub(crate) trait Reported: Sized {
    fn of(input: &EngineInput, v: Violation) -> Self;
    fn finding(&self) -> &Finding;
    fn into_finding(self) -> Finding;
}

impl Reported for Finding {
    fn of(input: &EngineInput, v: Violation) -> Self {
        input.finding(v)
    }

    fn finding(&self) -> &Finding {
        self
    }

    fn into_finding(self) -> Finding {
        self
    }
}

impl Reported for (Violation, Finding) {
    fn of(input: &EngineInput, v: Violation) -> Self {
        let finding = input.finding(v.clone());
        (v, finding)
    }

    fn finding(&self) -> &Finding {
        &self.1
    }

    fn into_finding(self) -> Finding {
        self.1
    }
}

pub(crate) fn apply<T: Reported>(
    input: &EngineInput,
    reported: Vec<T>,
) -> Result<(Vec<T>, Vec<Suppressed>), ConfigError> {
    let directives = directives(input);
    if directives.is_empty() {
        return Ok((reported, Vec::new()));
    }

    let mut at_line: HashMap<(&str, u32), Vec<usize>> = HashMap::new();
    for (i, d) in directives.iter().enumerate().filter(|(_, d)| d.is_valid()) {
        at_line
            .entry((d.file.as_str(), d.at.applies_to))
            .or_default()
            .push(i);
    }

    let mut used = vec![false; directives.len()];
    let mut kept = Vec::with_capacity(reported.len());
    let mut suppressed = Vec::new();
    for item in reported {
        let finding = item.finding();
        let hit = at_line
            .get(&(finding.file.as_str(), finding.location.line))
            .into_iter()
            .flatten()
            .copied()
            .find(|&i| directives[i].at.slugs.contains(&finding.rule));
        match hit {
            Some(i) => {
                used[i] = true;
                let reason = directives[i].at.reason.clone().unwrap_or_default();
                suppressed.push(Suppressed {
                    finding: item.into_finding(),
                    reason,
                });
            }
            None => kept.push(item),
        }
    }

    kept.extend(problems(input, &directives, &used)?);
    Ok((kept, suppressed))
}

fn directives(input: &EngineInput) -> Vec<Directive<'_>> {
    let mut out: Vec<Directive<'_>> = input
        .graph
        .modules
        .values()
        .flat_map(|m| {
            let file = input.relative(&m.path);
            m.suppressions.iter().map(move |at| Directive {
                module: m.id,
                file: file.clone(),
                at,
                problems: problems_of(input, at),
            })
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.at.location.line).cmp(&(&b.file, b.at.location.line)));
    out
}

fn problems_of(input: &EngineInput, at: &Suppression) -> Vec<String> {
    let mut problems = Vec::new();
    if at.slugs.is_empty() {
        problems.push("it names no rule".to_owned());
    }
    for slug in &at.slugs {
        if input.registry.by_slug(slug).is_none() {
            problems.push(format!("`{slug}` is not a rule"));
        }
    }
    if at.reason.is_none() {
        problems.push("it has no reason after `--`".to_owned());
    }
    problems
}

fn problems<T: Reported>(
    input: &EngineInput,
    directives: &[Directive<'_>],
    used: &[bool],
) -> Result<Vec<T>, ConfigError> {
    let Some(rule) = input.registry.by_slug(SLUG) else {
        return Ok(Vec::new());
    };
    let severities = EffectiveSeverities::new(&input.config, &input.project_root)?;
    if severities.off_everywhere(rule) {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (d, &used) in directives.iter().zip(used) {
        let (severity, message) = if !d.is_valid() {
            (
                Severity::Error,
                format!(
                    "Suppression `{}` suppresses nothing: {}.",
                    d.names(),
                    d.problems.join(" and ")
                ),
            )
        } else if !used {
            (
                Severity::Warning,
                format!(
                    "Suppression `{}` hides nothing on line {}; delete it.",
                    d.names(),
                    d.at.applies_to
                ),
            )
        } else {
            continue;
        };
        let cap = severities.for_slashed(&d.file).of(rule);
        if cap == Severity::Off {
            continue;
        }
        let Some(module) = input.graph.modules.get(&d.module) else {
            continue;
        };
        out.push(T::of(
            input,
            Violation {
                rule: SUPPRESSION,
                severity: severity.clamp(Severity::Info, cap),
                message,
                file: module.path.clone(),
                location: d.at.location,
                modules: vec![d.module],
                edge: None,
                fix: None,
            },
        ));
    }
    Ok(out)
}
