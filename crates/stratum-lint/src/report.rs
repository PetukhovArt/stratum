use std::time::Duration;

use serde::Serialize;
use stratum_core::severity::Severity;
pub use stratum_engine::{EdgeRef, Finding, Suppressed};

const EXIT_ERRORS: i32 = 1;
const EXIT_STALE_BASELINE: i32 = 3;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub baselined: usize,
    pub fixed: usize,
    pub suppressed: usize,
    pub scope_files: Option<usize>,
    pub zero_config: bool,
    #[serde(rename = "elapsed_ms", serialize_with = "as_millis")]
    pub elapsed: Duration,
    #[serde(skip)]
    pub fixed_in_scope: usize,
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
    pub suppressed: Vec<Suppressed>,
}

impl Report {
    #[must_use]
    pub fn new(findings: Vec<Finding>, suppressed: Vec<Suppressed>, mut summary: Summary) -> Self {
        summary.errors = count(&findings, Severity::Error);
        summary.warnings = count(&findings, Severity::Warning);
        summary.infos = count(&findings, Severity::Info);
        summary.suppressed = suppressed.len();
        Self {
            summary,
            findings,
            suppressed,
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        if self.summary.errors > 0 {
            EXIT_ERRORS
        } else if self.summary.fixed_in_scope > 0 {
            EXIT_STALE_BASELINE
        } else {
            0
        }
    }
}

fn count(findings: &[Finding], severity: Severity) -> usize {
    findings.iter().filter(|f| f.severity == severity).count()
}
