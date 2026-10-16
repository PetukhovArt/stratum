use serde::Serialize;
use stratum_core::severity::Severity;

use super::Reporter;
use crate::report::{Finding, Report};

const CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Default)]
pub struct JsonlReporter;

/// Finding contract v1: field order is part of the published contract.
#[derive(Serialize)]
struct Row<'a> {
    v: u32,
    tool: &'static str,
    rule: &'a str,
    severity: Severity,
    file: &'a str,
    line: u32,
    message: &'a str,
    fix: &'a str,
}

impl<'a> From<&'a Finding> for Row<'a> {
    fn from(f: &'a Finding) -> Self {
        Self {
            v: CONTRACT_VERSION,
            tool: "stratum",
            rule: &f.rule,
            severity: f.severity,
            file: &f.file,
            line: f.location.line,
            message: &f.message,
            fix: &f.fix,
        }
    }
}

impl Reporter for JsonlReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        for finding in &report.findings {
            serde_json::to_writer(&mut *out, &Row::from(finding))?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }
}
