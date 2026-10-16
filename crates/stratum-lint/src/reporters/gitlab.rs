use serde_json::json;
use stratum_core::severity::Severity;
use stratum_engine::baseline::Fingerprint;

use super::Reporter;
use crate::report::{Finding, Report};

#[derive(Debug)]
pub struct GitlabReporter {
    prefix: String,
}

impl GitlabReporter {
    #[must_use]
    pub const fn new(prefix: String) -> Self {
        Self { prefix }
    }
}

impl Reporter for GitlabReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let fingerprints = line_free_fingerprints(&report.findings);
        let issues: Vec<_> = report
            .findings
            .iter()
            .zip(fingerprints)
            .map(|(f, fingerprint)| {
                json!({
                    "description": f.message,
                    "check_name": f.rule,
                    "fingerprint": fingerprint,
                    "severity": severity(f.severity),
                    "location": {
                        "path": format!("{}{}", self.prefix, f.file),
                        "lines": { "begin": f.location.line }
                    }
                })
            })
            .collect();
        let pretty = serde_json::to_string_pretty(&issues).map_err(std::io::Error::other)?;
        out.write_all(pretty.as_bytes())?;
        writeln!(out)?;
        Ok(())
    }
}

const fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "major",
        Severity::Warning => "minor",
        Severity::Info | Severity::Off => "info",
    }
}

fn line_free_fingerprints(findings: &[Finding]) -> Vec<String> {
    let keys: Vec<Fingerprint> = findings.iter().map(Fingerprint::of).collect();
    let mut by_position: Vec<usize> = (0..findings.len()).collect();
    by_position.sort_by_key(|&i| {
        (
            &keys[i],
            findings[i].location.line,
            findings[i].location.column,
        )
    });
    let mut out = vec![String::new(); findings.len()];
    let mut occurrence = 0;
    for (n, &i) in by_position.iter().enumerate() {
        let same_as_previous = n > 0 && keys[by_position[n - 1]] == keys[i];
        occurrence = if same_as_previous { occurrence + 1 } else { 0 };
        out[i] = format!("{:032x}", fnv1a_128(&keys[i], occurrence));
    }
    out
}

fn fnv1a_128(key: &Fingerprint, occurrence: usize) -> u128 {
    const OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
    let occurrence = occurrence.to_string();
    let fields = [
        key.rule.as_bytes(),
        key.file.as_bytes(),
        key.message.as_bytes(),
        occurrence.as_bytes(),
    ];
    let mut hash = OFFSET;
    for field in fields {
        for &byte in field.iter().chain(&[0]) {
            hash ^= u128::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }
    hash
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::violation::SourceLocation;

    fn finding(file: &str, line: u32, message: &str) -> Finding {
        Finding {
            rule: "stratum/purity".into(),
            severity: Severity::Error,
            message: message.into(),
            file: file.into(),
            location: SourceLocation { line, column: 1 },
            edge: None,
            suggestion: None,
            members: Vec::new(),
        }
    }

    #[test]
    fn a_line_shift_keeps_the_fingerprint() {
        let before = line_free_fingerprints(&[finding("src/a.ts", 4, "calls fetch")]);
        let after = line_free_fingerprints(&[finding("src/a.ts", 9, "calls fetch")]);
        assert_eq!(before, after);
    }

    #[test]
    fn identical_violations_in_one_file_get_distinct_fingerprints() {
        let prints = line_free_fingerprints(&[
            finding("src/a.ts", 7, "calls fetch"),
            finding("src/a.ts", 3, "calls fetch"),
        ]);
        assert_ne!(prints[0], prints[1]);
        let earliest_alone = line_free_fingerprints(&[finding("src/a.ts", 3, "calls fetch")]);
        assert_eq!(prints[1], earliest_alone[0]);
    }
}
