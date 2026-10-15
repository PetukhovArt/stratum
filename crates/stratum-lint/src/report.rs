use std::time::Duration;

use serde::Serialize;
use stratum_core::severity::Severity;
pub use stratum_engine::{EdgeRef, Finding};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub baselined: usize,
    pub fixed: usize,
    pub scope_files: Option<usize>,
    pub zero_config: bool,
    #[serde(rename = "elapsed_ms", serialize_with = "as_millis")]
    pub elapsed: Duration,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn as_millis<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u128(d.as_millis())
}

impl Summary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.errors + self.warnings + self.infos
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub summary: Summary,
    pub findings: Vec<Finding>,
}

impl Report {
    #[must_use]
    pub fn new(findings: Vec<Finding>, mut summary: Summary) -> Self {
        summary.errors = count(&findings, Severity::Error);
        summary.warnings = count(&findings, Severity::Warning);
        summary.infos = count(&findings, Severity::Info);
        Self { summary, findings }
    }

    #[must_use]
    pub fn exit_code(&self) -> i32 {
        i32::from(self.summary.errors > 0)
    }
}

fn count(findings: &[Finding], severity: Severity) -> usize {
    findings.iter().filter(|f| f.severity == severity).count()
}
