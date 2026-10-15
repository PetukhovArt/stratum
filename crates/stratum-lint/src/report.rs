use std::path::Path;
use std::time::Duration;

use camino::Utf8Path;
use serde::Serialize;
use stratum_core::severity::Severity;

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeRef {
    pub from: String,
    pub to: String,
    pub kind: stratum_core::edge::EdgeKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub location: Location,
    pub edge: Option<EdgeRef>,
    pub suggestion: Option<String>,
    #[serde(skip)]
    pub members: Vec<String>,
}

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

/// `path` relative to `root` with forward slashes; `path` itself (slashes normalised) when outside `root`.
#[must_use]
pub fn relative(path: &Path, root: &Utf8Path) -> String {
    let path = path.to_string_lossy();
    let path = strip_verbatim(&path).replace('\\', "/");
    let root = strip_verbatim(root.as_str()).replace('\\', "/");
    let root = root.trim_end_matches('/');
    match path.strip_prefix(root) {
        Some(rest) if rest.starts_with('/') => rest[1..].to_owned(),
        _ => path,
    }
}

fn strip_verbatim(s: &str) -> &str {
    s.strip_prefix(r"\\?\").unwrap_or(s)
}
