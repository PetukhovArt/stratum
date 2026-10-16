use std::collections::BTreeMap;

use stratum_core::severity::Severity;

use super::{AgentReporter, Reporter};
use crate::report::{Finding, Report};

const ANNOTATIONS_PER_SEVERITY: usize = 10;

#[derive(Debug)]
pub struct GithubReporter {
    prefix: String,
}

impl GithubReporter {
    #[must_use]
    pub const fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

impl Reporter for GithubReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (severity, command) in [(Severity::Error, "error"), (Severity::Warning, "warning")] {
            for f in round_robin(&report.findings, severity, ANNOTATIONS_PER_SEVERITY) {
                writeln!(
                    out,
                    "::{command} file={},line={},col={},title={}::{}",
                    escape_property(&format!("{}{}", self.prefix, f.file)),
                    f.location.line,
                    f.location.column,
                    escape_property(&f.rule),
                    escape_data(&f.message),
                )?;
            }
        }
        AgentReporter {
            max: AgentReporter::DEFAULT_MAX,
        }
        .write(report, out)
    }
}

fn round_robin(findings: &[Finding], severity: Severity, cap: usize) -> Vec<&Finding> {
    let mut by_rule: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for f in findings.iter().filter(|f| f.severity == severity) {
        by_rule.entry(&f.rule).or_default().push(f);
    }
    let mut rules: Vec<Vec<&Finding>> = by_rule.into_values().collect();
    rules.sort_by_key(|hits| std::cmp::Reverse(hits.len()));
    for hits in &mut rules {
        hits.sort_by(|a, b| (&a.file, a.location.line).cmp(&(&b.file, b.location.line)));
    }
    let mut picked = Vec::new();
    for round in 0.. {
        let before = picked.len();
        for hits in &rules {
            if picked.len() == cap {
                return picked;
            }
            if let Some(f) = hits.get(round) {
                picked.push(*f);
            }
        }
        if picked.len() == before {
            break;
        }
    }
    picked
}

fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::report::Summary;
    use stratum_core::violation::SourceLocation;

    fn finding(rule: &str, severity: Severity, file: &str, line: u32, message: &str) -> Finding {
        Finding {
            rule: rule.into(),
            severity,
            message: message.into(),
            file: file.into(),
            location: SourceLocation { line, column: 2 },
            edge: None,
            fix: None,
            members: Vec::new(),
        }
    }

    fn annotations(findings: Vec<Finding>, prefix: &str) -> Vec<String> {
        let summary = Summary {
            elapsed: Duration::ZERO,
            ..Summary::default()
        };
        let mut buf = Vec::new();
        GithubReporter::new(prefix.into())
            .write(&Report::new(findings, summary), &mut buf)
            .unwrap();
        String::from_utf8(buf)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("::"))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn errors_past_the_cap_are_spread_round_robin_across_rules() {
        let mut findings: Vec<Finding> = (1..=12)
            .map(|i| finding("stratum/purity", Severity::Error, "src/a.ts", i, "m"))
            .collect();
        findings.extend(
            (1..=3).map(|i| finding("stratum/public-api", Severity::Error, "src/b.ts", i, "m")),
        );
        findings.push(finding(
            "stratum/depth-ratio",
            Severity::Info,
            "src/c.ts",
            1,
            "m",
        ));
        let lines = annotations(findings, "");
        let titles: Vec<&str> = lines
            .iter()
            .map(|l| {
                l.split("title=")
                    .nth(1)
                    .unwrap()
                    .split("::")
                    .next()
                    .unwrap()
            })
            .collect();
        assert_eq!(titles.len(), 10);
        assert_eq!(
            titles
                .iter()
                .filter(|t| **t == "stratum/public-api")
                .count(),
            3
        );
        assert_eq!(titles[..2], ["stratum/purity", "stratum/public-api"]);
    }

    #[test]
    fn commas_colons_and_newlines_are_escaped() {
        let lines = annotations(
            vec![finding(
                "stratum/purity",
                Severity::Warning,
                "src/a,b:c.ts",
                3,
                "50% of a, b: c\r\nnext",
            )],
            "web/",
        );
        assert_eq!(
            lines,
            [
                "::warning file=web/src/a%2Cb%3Ac.ts,line=3,col=2,title=stratum/purity::50%25 of a, b: c%0D%0Anext"
            ]
        );
    }
}
