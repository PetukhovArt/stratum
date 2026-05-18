use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ids::ModuleId;

/// Classification of a typed connection between two Modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Static,
    Di,
    Runtime,
}

/// A typed directed dependency between two Modules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub from: ModuleId,
    pub to: ModuleId,
    pub kind: EdgeKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kinds_serialize_lowercase() {
        assert_eq!(serde_json::to_string(&EdgeKind::Static).unwrap(), r#""static""#);
        assert_eq!(serde_json::to_string(&EdgeKind::Di).unwrap(), r#""di""#);
        assert_eq!(serde_json::to_string(&EdgeKind::Runtime).unwrap(), r#""runtime""#);
    }

    #[test]
    fn edge_is_directional_under_eq() {
        let a = Edge { from: ModuleId::new(1), to: ModuleId::new(2), kind: EdgeKind::Static };
        let b = Edge { from: ModuleId::new(2), to: ModuleId::new(1), kind: EdgeKind::Static };
        assert_ne!(a, b);
    }
}
