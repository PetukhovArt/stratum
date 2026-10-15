use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use stratum_core::severity::Severity;

use super::Reporter;
use crate::report::{Finding, Report, Summary};

const EXAMPLES_PER_RULE: usize = 3;
const MESSAGE_CHARS: usize = 240;

#[derive(Debug)]
pub struct AgentReporter {
    pub max: usize,
}

impl AgentReporter {
    pub const DEFAULT_MAX: usize = 20;
}

impl Reporter for AgentReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(out, "{}", summary_line(&report.summary))?;

        let mut groups: BTreeMap<(&str, Severity), Vec<&Finding>> = BTreeMap::new();
        for f in &report.findings {
            groups.entry((&f.rule, f.severity)).or_default().push(f);
        }
        let mut groups: Vec<_> = groups.into_iter().collect();
        groups.sort_by(|((a_rule, a_sev), a), ((b_rule, b_sev), b)| {
            (b_sev, b.len(), a_rule).cmp(&(a_sev, a.len(), b_rule))
        });

        let mut budget = self.max;
        let mut hinted = HashSet::new();
        let mut hidden = Vec::new();
        for ((rule, severity), hits) in groups {
            if budget == 0 {
                hidden.push(format!("{rule} ({})", hits.len()));
                continue;
            }
            let hint = if hinted.insert(rule) {
                format!(" - stratum-lint explain {rule}")
            } else {
                String::new()
            };
            writeln!(out, "{severity} {rule} ({}){hint}", hits.len())?;
            let shown = hits.len().min(EXAMPLES_PER_RULE).min(budget);
            budget -= shown;
            for f in &hits[..shown] {
                writeln!(out, "  {}", example(f))?;
            }
            if hits.len() > shown {
                writeln!(out, "  ... {} more", hits.len() - shown)?;
            }
            if let Some(fix) = common_suggestion(&hits) {
                writeln!(out, "  fix: {fix}")?;
            }
        }
        if !hidden.is_empty() {
            writeln!(
                out,
                "+{} more rules over --max {}: {}",
                hidden.len(),
                self.max,
                hidden.join(", ")
            )?;
        }
        Ok(())
    }
}

fn summary_line(s: &Summary) -> String {
    let mut line = if s.total() == 0 {
        String::from("stratum: ok, no new violations")
    } else {
        let counts: Vec<String> = [
            (s.errors, "error", "errors"),
            (s.warnings, "warning", "warnings"),
            (s.infos, "info", "info"),
        ]
        .into_iter()
        .filter(|(n, ..)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if n == 1 { one } else { many }))
        .collect();
        format!("stratum: {}", counts.join(", "))
    };
    if s.baselined > 0 {
        let _ = write!(line, ", {} baselined", s.baselined);
    }
    if s.fixed > 0 {
        let _ = write!(
            line,
            ", {} fixed - run \"stratum-lint baseline --prune\"",
            s.fixed
        );
    }
    if let Some(n) = s.scope_files {
        let _ = write!(line, ", scope {n} file{}", if n == 1 { "" } else { "s" });
    }
    let _ = write!(line, ", {:.2}s", s.elapsed.as_secs_f64());
    if s.zero_config {
        line.push_str(
            " | zero-config: layers inferred from src/*, run \"stratum-lint init\" to pin them",
        );
    }
    line
}

fn example(f: &Finding) -> String {
    let target = f
        .edge
        .as_ref()
        .filter(|e| e.to != f.file)
        .map(|e| format!(" -> {}", e.to))
        .unwrap_or_default();
    format!(
        "{}:{}{target}  {}",
        f.file,
        f.location.line,
        clip(&f.message)
    )
}

fn clip(message: &str) -> String {
    match message.char_indices().nth(MESSAGE_CHARS) {
        None => message.to_owned(),
        Some((cut, _)) => format!(
            "{}... (+{} chars, full text in --format json)",
            &message[..cut],
            message[cut..].chars().count()
        ),
    }
}

fn common_suggestion<'a>(hits: &[&'a Finding]) -> Option<&'a str> {
    let first = hits.first()?.suggestion.as_deref()?;
    hits.iter()
        .all(|f| f.suggestion.as_deref() == Some(first))
        .then_some(first)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::report::{EdgeRef, Location};
    use stratum_core::edge::EdgeKind;

    fn finding(rule: &str, severity: Severity, file: &str) -> Finding {
        Finding {
            rule: rule.into(),
            severity,
            message: format!("{rule} in {file}"),
            file: file.into(),
            location: Location { line: 4, column: 1 },
            edge: Some(EdgeRef {
                from: file.into(),
                to: "src/app/main.ts".into(),
                kind: EdgeKind::Static,
            }),
            suggestion: Some("move it".into()),
        }
    }

    fn render(findings: Vec<Finding>, summary: Summary, max: usize) -> String {
        let mut buf = Vec::new();
        AgentReporter { max }
            .write(&Report::new(findings, summary), &mut buf)
            .unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn clean_run_is_one_line() {
        let summary = Summary {
            baselined: 3,
            scope_files: Some(2),
            zero_config: true,
            elapsed: Duration::from_millis(420),
            ..Summary::default()
        };
        insta::assert_snapshot!(render(Vec::new(), summary, 20));
    }

    #[test]
    fn groups_cap_and_hint() {
        let mut findings: Vec<Finding> = (0..5)
            .map(|i| {
                finding(
                    "stratum/stage-purity",
                    Severity::Error,
                    &format!("src/shared/m{i}.ts"),
                )
            })
            .collect();
        findings.push(finding(
            "stratum/no-cross-layer-import",
            Severity::Warning,
            "src/shared/x.ts",
        ));
        findings.push(finding(
            "stratum/depth-ratio",
            Severity::Info,
            "src/features/a.ts",
        ));
        let summary = Summary {
            fixed: 2,
            elapsed: Duration::from_millis(1234),
            ..Summary::default()
        };
        insta::assert_snapshot!(render(findings, summary, 4));
    }
}
