use std::collections::HashMap;

use stratum_config::ConfigError;
use stratum_core::{
    ids::ModuleId,
    severity::Severity,
    types::Suppression,
    violation::{SourceLocation, Violation},
};
use stratum_rules::{CO_CHANGE_SLUG, EffectiveSeverities, SUPPRESSION, SuppressionOptions};

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
    let mut reported = reported;
    reported.extend(forbidden(input)?);
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
        } else if !used && !d.at.slugs.iter().any(|s| s == CO_CHANGE_SLUG) {
            // A `stratum/co-change` acknowledgement is read only on a diff, never per line.
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

/// One violation per comment, in a module, that starts with a directive of the rule's `forbid`.
/// Read from disk, so an unsaved buffer in the LSP shows the saved file's directives.
fn forbidden<T: Reported>(input: &EngineInput) -> Result<Vec<T>, ConfigError> {
    let forbid = input
        .config
        .rules
        .get(SLUG)
        .filter(|rule| !rule.options.is_null())
        .and_then(|rule| serde_json::from_value::<SuppressionOptions>(rule.options.clone()).ok())
        .map(|o| o.forbid)
        .unwrap_or_default();
    if forbid.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rule) = input.registry.by_slug(SLUG) else {
        return Ok(Vec::new());
    };
    let severities = EffectiveSeverities::new(&input.config, &input.project_root)?;
    if severities.off_everywhere(rule) {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for m in input.graph.modules.values() {
        let cap = severities.for_slashed(&input.relative(&m.path)).of(rule);
        if cap == Severity::Off {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&m.path) else {
            continue;
        };
        for (at, written, listed) in foreign_directives(&text, &forbid) {
            out.push(T::of(
                input,
                Violation {
                    rule: SUPPRESSION,
                    severity: Severity::Error.clamp(Severity::Info, cap),
                    message: format!(
                        "Directive `{written}` is forbidden: `stratum/suppression` `forbid` lists `{listed}`."
                    ),
                    file: m.path.clone(),
                    location: at,
                    modules: vec![m.id],
                    edge: None,
                    fix: Some(
                        "Fix what the directive hides and delete it. To keep existing ones, accept them into the baseline (`stratum-lint baseline`); for a folder, turn the rule off in an `overrides` block.".to_owned(),
                    ),
                },
            ));
        }
    }
    Ok(out)
}

/// `(location, directive as written, entry of forbid)` per comment of `text` whose body starts
/// with an entry of `forbid`, followed by the end of the body or a character that is not a letter,
/// digit or `_`: `eslint-disable` matches `eslint-disable-next-line` but not `eslint-disabled`.
/// Comments are found per line: `//`, `/*`, `<!--`, and a block comment line starting with `*`.
fn foreign_directives<'f>(
    text: &str,
    forbid: &'f [String],
) -> Vec<(SourceLocation, String, &'f str)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        for (column, body) in comment_bodies(line) {
            let Some(listed) = forbid.iter().find(|d| {
                body.strip_prefix(d.as_str()).is_some_and(|rest| {
                    !rest
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_')
                })
            }) else {
                continue;
            };
            let written: String = body
                .chars()
                .take_while(|c| !c.is_whitespace() && !matches!(c, '*' | ':' | ','))
                .collect();
            out.push((
                SourceLocation {
                    line: u32::try_from(i + 1).unwrap_or(u32::MAX),
                    column: u32::try_from(column + 1).unwrap_or(u32::MAX),
                },
                written,
                listed.as_str(),
            ));
        }
    }
    out
}

/// `(byte column of the marker, text after it, trimmed)` for every comment start on `line`.
fn comment_bodies(line: &str) -> Vec<(usize, &str)> {
    fn body(rest: &str) -> &str {
        rest.trim_start_matches(|c: char| c == '*' || c.is_whitespace())
    }
    let mut out = Vec::new();
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    if trimmed.starts_with('*') && !trimmed.starts_with("*/") {
        out.push((indent, body(&trimmed[1..])));
    }
    for marker in ["//", "/*", "<!--"] {
        let mut from = 0;
        while let Some(found) = line[from..].find(marker) {
            let at = from + found;
            from = at + marker.len();
            let url = marker == "//" && line[..at].ends_with(':');
            let doubled = marker == "//" && at > 0 && line[..at].ends_with('/');
            if !url && !doubled {
                out.push((at, body(&line[from..])));
            }
        }
    }
    out.sort_by_key(|(at, _)| *at);
    out.dedup_by_key(|(at, _)| *at);
    out
}

#[cfg(test)]
mod tests {
    use super::foreign_directives;

    #[test]
    fn a_comment_starting_with_a_forbidden_directive_is_found_with_its_longer_forms() {
        let forbid = ["eslint-disable".to_owned(), "@ts-ignore".to_owned()];
        let text = "// eslint-disable-next-line no-console\nconst a = 1; // @ts-ignore\n/* eslint-disable */\n/**\n * @ts-ignore\n */\n// eslint-disabled is fine\nconst url = 'https://eslint-disable';\n<!-- eslint-disable -->\n// see @ts-ignore docs\n";
        let found: Vec<(u32, String)> = foreign_directives(text, &forbid)
            .into_iter()
            .map(|(at, written, _)| (at.line, written))
            .collect();
        assert_eq!(
            found,
            [
                (1, "eslint-disable-next-line".to_owned()),
                (2, "@ts-ignore".to_owned()),
                (3, "eslint-disable".to_owned()),
                (5, "@ts-ignore".to_owned()),
                (9, "eslint-disable".to_owned()),
            ]
        );
    }
}
