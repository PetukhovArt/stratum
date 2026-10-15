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
    pub suggestion: Option<String>,
    #[serde(skip)]
    pub members: Vec<String>,
}
