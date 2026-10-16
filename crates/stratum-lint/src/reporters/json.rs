use serde::Serialize;
use stratum_core::{severity::Severity, violation::SourceLocation};
use stratum_engine::{EdgeRef, Finding};

use super::Reporter;
use crate::report::Report;

pub const VERSION: u32 = 2;

#[derive(Debug, Default)]
pub struct JsonReporter;

/// Version 2 is frozen: `fix` stays `suggestion` on the wire.
#[derive(Serialize)]
struct WireFinding<'a> {
    rule: &'a str,
    severity: Severity,
    message: &'a str,
    file: &'a str,
    location: SourceLocation,
    edge: Option<&'a EdgeRef>,
    suggestion: &'a str,
}

impl<'a> From<&'a Finding> for WireFinding<'a> {
    fn from(f: &'a Finding) -> Self {
        Self {
            rule: &f.rule,
            severity: f.severity,
            message: &f.message,
            file: &f.file,
            location: f.location,
            edge: f.edge.as_ref(),
            suggestion: &f.fix,
        }
    }
}

impl Reporter for JsonReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let violations: Vec<WireFinding<'_>> = report.findings.iter().map(Into::into).collect();
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "version": VERSION,
            "summary": report.summary,
            "violations": violations,
        }))
        .map_err(std::io::Error::other)?;
        out.write_all(json.as_bytes())?;
        writeln!(out)?;
        Ok(())
    }
}
