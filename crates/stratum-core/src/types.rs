use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    ids::{ContainerId, LayerId, ModuleId},
    stage::Stage,
    visibility::VisibilityScope,
};

/// One source unit (file or SFC) appearing as a node in the Compound DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub path: PathBuf,
    pub container: ContainerId,
    pub layer: LayerId,
    pub stage: Stage,
    pub visibility: VisibilityScope,
}

/// A compound node grouping Modules of one Layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub name: String,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
}

/// A named group of Modules with its own visibility and dependency direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub path: PathBuf,
    pub depends_on: Vec<LayerId>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn module_construction_roundtrip() {
        let m = Module {
            id: ModuleId::new(1),
            path: PathBuf::from("src/features/cart/index.ts"),
            container: ContainerId::new(2),
            layer: LayerId::new(0),
            stage: Stage::new(3).unwrap(),
            visibility: VisibilityScope::Public,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Module = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn layer_dependencies_are_layer_ids() {
        let l = Layer {
            id: LayerId::new(0),
            name: "features".into(),
            path: PathBuf::from("src/features"),
            depends_on: vec![LayerId::new(1), LayerId::new(2)],
        };
        assert_eq!(l.depends_on.len(), 2);
    }
}
