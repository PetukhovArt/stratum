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
    /// Human-readable advice — never an auto-applied fix (PRD decision #10).
    pub suggestion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::EdgeKind;

    #[test]
    fn violation_round_trips() {
        let v = Violation {
            rule: RuleId::new(7),
            severity: Severity::Error,
            message: "Cross-layer import from app to shared".into(),
            file: PathBuf::from("src/app/main.ts"),
            location: SourceLocation { line: 12, column: 1 },
            modules: vec![ModuleId::new(1), ModuleId::new(2)],
            edge: Some(Edge {
                from: ModuleId::new(1),
                to: ModuleId::new(2),
                kind: EdgeKind::Static,
            }),
            suggestion: Some("Move the helper into entities/ if it is generic.".into()),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: Violation = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn no_fix_field_in_serialized_form() {
        let v = Violation {
            rule: RuleId::new(1),
            severity: Severity::Warning,
            message: "x".into(),
            file: PathBuf::from("x"),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![],
            edge: None,
            suggestion: None,
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("\"fix\""), "fix.edits is deliberately absent (PRD decision #10)");
    }
}
