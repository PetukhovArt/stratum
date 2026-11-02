use std::borrow::Cow;

use serde::Serialize;
use stratum_core::{severity::Severity, text::Messages};

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
    message: Cow<'a, str>,
    fix: Cow<'a, str>,
}

impl<'a> Row<'a> {
    fn new(f: &'a Finding, messages: &Messages) -> Self {
        Self {
            v: CONTRACT_VERSION,
            tool: "stratum",
            rule: &f.rule,
            severity: f.severity,
            file: &f.file,
            line: f.location.line,
            message: f.message_in(messages),
            fix: f.fix_in(messages),
        }
    }
}

impl Reporter for JsonlReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        for finding in &report.findings {
            serde_json::to_writer(&mut *out, &Row::new(finding, &report.messages))?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }
}
