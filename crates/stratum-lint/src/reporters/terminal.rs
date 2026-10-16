use std::collections::BTreeMap;

use super::Reporter;
use crate::report::{Finding, Report};

#[derive(Debug, Default)]
pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut by_file: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
        for f in &report.findings {
            by_file.entry(&f.file).or_default().push(f);
        }
        for (file, fs) in &by_file {
            writeln!(out, "{file}")?;
            for f in fs {
                writeln!(
                    out,
                    "  {}  {}:{}  {}  {}",
                    f.severity, f.location.line, f.location.column, f.rule, f.message
                )?;
                if let Some(s) = &f.fix {
                    writeln!(out, "    fix: {s}")?;
                }
            }
        }
        let s = &report.summary;
        write!(out, "\n{} violations", report.findings.len())?;
        if s.baselined > 0 {
            write!(out, ", {} baselined", s.baselined)?;
        }
        if s.fixed > 0 {
            write!(
                out,
                ", {} fixed (run \"stratum-lint baseline --prune\")",
                s.fixed
            )?;
        }
        writeln!(out)?;
        Ok(())
    }
}
