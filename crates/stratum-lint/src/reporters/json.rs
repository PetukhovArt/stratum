use super::Reporter;
use crate::report::Report;

pub const VERSION: u32 = 2;

#[derive(Debug, Default)]
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "version": VERSION,
            "summary": report.summary,
            "violations": report.findings,
        }))
        .map_err(std::io::Error::other)?;
        out.write_all(json.as_bytes())?;
        writeln!(out)?;
        Ok(())
    }
}
