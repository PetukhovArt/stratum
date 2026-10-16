use serde::Serialize;
use stratum_core::{edge::EdgeKind, severity::Severity, violation::SourceLocation};

#[derive(Debug, Clone, Serialize)]
pub struct EdgeRef {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub location: SourceLocation,
    pub edge: Option<EdgeRef>,
    /// Never empty: the violation's own text, else the rule doc's fix, else a pointer to `explain`.
    pub fix: String,
    #[serde(skip)]
    pub members: Vec<String>,
}
