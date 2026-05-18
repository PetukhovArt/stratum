use std::collections::BTreeMap;
use std::path::PathBuf;

use stratum_core::violation::Violation;

use super::Reporter;

#[derive(Debug, Default)]
pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn write(
        &self,
        violations: &[Violation],
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let mut by_file: BTreeMap<PathBuf, Vec<&Violation>> = BTreeMap::new();
        for v in violations {
            by_file.entry(v.file.clone()).or_default().push(v);
        }
        for (file, vs) in &by_file {
            writeln!(out, "{}", file.display())?;
            for v in vs {
                writeln!(
                    out,
                    "  {}  {}:{}  {}",
                    v.severity, v.location.line, v.location.column, v.message
                )?;
                if let Some(s) = &v.suggestion {
                    writeln!(out, "    suggestion: {s}")?;
                }
            }
        }
        writeln!(out, "\n{} violations", violations.len())?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::{
        ids::{ModuleId, RuleId},
        severity::Severity,
        violation::SourceLocation,
    };

    #[test]
    fn renders_empty() {
        let r = TerminalReporter;
        let mut buf = Vec::new();
        r.write(&[], &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "\n0 violations\n");
    }

    #[test]
    fn renders_one_violation_with_suggestion() {
        let v = Violation {
            rule: RuleId::new(1),
            severity: Severity::Error,
            message: "msg".into(),
            file: PathBuf::from("a.ts"),
            location: SourceLocation { line: 7, column: 3 },
            modules: vec![ModuleId::new(0)],
            edge: None,
            suggestion: Some("do X".into()),
        };
        let mut buf = Vec::new();
        TerminalReporter.write(&[v], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("a.ts"));
        assert!(s.contains("suggestion: do X"));
    }
}
