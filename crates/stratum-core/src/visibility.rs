use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ids::{ContainerId, LayerId};

/// The set of Modules allowed to import a given Module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibilityScope {
    /// Visible only inside the same Container.
    Container { id: ContainerId },
    /// Visible to any Module inside the named Layer.
    Layer { id: LayerId },
    /// Public across all Layers.
    Public,
    /// Restricted to Modules whose path matches a `_shared`-style sibling rule.
    Shared,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn variants_distinct_under_eq() {
        let a = VisibilityScope::Container {
            id: ContainerId::new(1),
        };
        let b = VisibilityScope::Container {
            id: ContainerId::new(2),
        };
        let c = VisibilityScope::Public;
        let d = VisibilityScope::Shared;
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(c, d);
    }

    #[test]
    fn serializes_with_tag() {
        let v = VisibilityScope::Layer {
            id: LayerId::new(3),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, r#"{"kind":"layer","id":3}"#);
    }
}
