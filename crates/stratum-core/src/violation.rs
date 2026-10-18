use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    edge::Edge,
    ids::{ModuleId, RuleId},
    severity::Severity,
};

/// A pointer into a source file: 1-based line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

/// A single Rule infraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    pub rule: RuleId,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub location: SourceLocation,
    pub modules: Vec<ModuleId>,
    pub edge: Option<Edge>,
    /// Human-readable advice — never an auto-applied fix.
    pub fix: Option<String>,
}
