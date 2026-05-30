//! Slices: the first-level containers directly under a layer root.
//!
//! In FSD terms these are the `auth` / `cart` folders under `features/`, the
//! entity folders under `entities/`, and so on. A slice is the unit that
//! cross-slice / public-API / sibling-visibility rules reason about, so the
//! graph needs to name it as a first-class entity rather than leave it implicit
//! in the container tree.
//!
//! A layer root is itself a container whose `id == ContainerId::new(layer.raw())`
//! and whose `parent` is `None` (see `builder::GraphBuilder::build`). A slice is
//! therefore any container whose `parent` is exactly that layer-root container.

use stratum_core::ids::{ContainerId, LayerId, ModuleId};

use crate::graph::CompoundGraph;

/// A first-level container directly under a layer root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slice {
    /// The container that *is* the slice.
    pub container: ContainerId,
    /// The layer this slice lives in.
    pub layer: LayerId,
    /// The slice's folder name (e.g. `auth`).
    pub name: String,
}

/// The container id of `layer`'s root container.
fn layer_root(layer: LayerId) -> ContainerId {
    ContainerId::new(layer.raw())
}

/// All slices in `g`: containers whose parent is their layer's root container.
///
/// Deterministic — sorted by container id.
#[must_use]
pub fn slices(g: &CompoundGraph) -> Vec<Slice> {
    let mut out: Vec<Slice> = g
        .containers
        .values()
        .filter(|c| c.parent == Some(layer_root(c.layer)))
        .map(|c| Slice {
            container: c.id,
            layer: c.layer,
            name: c.name.clone(),
        })
        .collect();
    out.sort_by_key(|s| s.container.raw());
    out
}

/// The slice a module belongs to.
///
/// Walks up the module's container chain to the first-level container under the
/// layer root. Returns `None` when the module sits directly in the layer root
/// (no enclosing slice) or when the module / a container is unknown.
///
/// Assumes the container parent chain is acyclic — the [`crate::builder`] always
/// produces a forest (each folder's parent is a previously-allocated ancestor),
/// so the upward walk terminates.
#[must_use]
pub fn slice_of(g: &CompoundGraph, module: ModuleId) -> Option<ContainerId> {
    let m = g.modules.get(&module)?;
    let root = layer_root(m.layer);
    let mut current = m.container;
    if current == root {
        return None;
    }
    loop {
        let c = g.containers.get(&current)?;
        match c.parent {
            Some(parent) if parent == root => return Some(current),
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use stratum_core::{
        purity::Purity,
        types::{Container, Module},
        visibility::VisibilityScope,
    };

    /// Build a graph mirroring the builder's container layout:
    /// layer root `L` (id == `layer.raw()`, parent `None`), slices directly under
    /// it, optionally a nested folder under a slice.
    fn graph() -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        // layer 0 root container (id 0)
        g.layers.insert(
            LayerId::new(0),
            stratum_core::types::Layer {
                id: LayerId::new(0),
                name: "features".into(),
                path: PathBuf::from("src/features"),
                depends_on: vec![],
            },
        );
        let root = Container {
            id: ContainerId::new(0),
            name: "features".into(),
            layer: LayerId::new(0),
            parent: None,
        };
        // two slices under the root
        let auth = Container {
            id: ContainerId::new(10),
            name: "auth".into(),
            layer: LayerId::new(0),
            parent: Some(ContainerId::new(0)),
        };
        let cart = Container {
            id: ContainerId::new(11),
            name: "cart".into(),
            layer: LayerId::new(0),
            parent: Some(ContainerId::new(0)),
        };
        // a nested folder under auth (NOT a slice)
        let auth_model = Container {
            id: ContainerId::new(12),
            name: "model".into(),
            layer: LayerId::new(0),
            parent: Some(ContainerId::new(10)),
        };
        for c in [root, auth, cart, auth_model] {
            g.containers.insert(c.id, c);
        }
        g
    }

    fn module(id: u32, container: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        }
    }

    #[test]
    fn slices_are_first_level_containers_only() {
        let g = graph();
        let found = slices(&g);
        let ids: Vec<u32> = found.iter().map(|s| s.container.raw()).collect();
        // auth (10) and cart (11) are slices; root (0) and nested model (12) are not.
        assert_eq!(ids, vec![10, 11]);
        assert_eq!(found[0].name, "auth");
        assert_eq!(found[1].name, "cart");
        assert!(found.iter().all(|s| s.layer == LayerId::new(0)));
    }

    /// Two layers with NON-zero ids. The layer-root container id equals the
    /// layer's raw id (builder invariant), so a hardcoded `ContainerId::new(0)`
    /// would fail this — it makes `layer_root` load-bearing.
    #[test]
    fn slices_discriminate_by_layer() {
        let mut g = CompoundGraph::empty();
        for (lid, name, path) in [
            (0u32, "features", "src/features"),
            (2u32, "entities", "src/entities"),
        ] {
            g.layers.insert(
                LayerId::new(lid),
                stratum_core::types::Layer {
                    id: LayerId::new(lid),
                    name: name.into(),
                    path: PathBuf::from(path),
                    depends_on: vec![],
                },
            );
            // layer-root container: id == layer.raw(), parent None
            g.containers.insert(
                ContainerId::new(lid),
                Container {
                    id: ContainerId::new(lid),
                    name: name.into(),
                    layer: LayerId::new(lid),
                    parent: None,
                },
            );
        }
        // A slice named "user" exists in BOTH layers — only `layer` discriminates.
        let feat_user = Container {
            id: ContainerId::new(10),
            name: "user".into(),
            layer: LayerId::new(0),
            parent: Some(ContainerId::new(0)),
        };
        let ent_user = Container {
            id: ContainerId::new(11),
            name: "user".into(),
            layer: LayerId::new(2),
            parent: Some(ContainerId::new(2)),
        };
        // nested folder under the entities/user slice
        let ent_user_model = Container {
            id: ContainerId::new(12),
            name: "model".into(),
            layer: LayerId::new(2),
            parent: Some(ContainerId::new(11)),
        };
        for c in [feat_user, ent_user, ent_user_model] {
            g.containers.insert(c.id, c);
        }

        let found = slices(&g);
        assert_eq!(found.len(), 2);
        // both named "user" but in different layers
        assert_eq!(found[0].container, ContainerId::new(10));
        assert_eq!(found[0].layer, LayerId::new(0));
        assert_eq!(found[1].container, ContainerId::new(11));
        assert_eq!(found[1].layer, LayerId::new(2));

        // a module nested in entities/user/model resolves to the entities slice,
        // proving slice_of uses layer_root(2) == ContainerId::new(2), not 0.
        let m = module(7, 12);
        let m = Module {
            layer: LayerId::new(2),
            ..m
        };
        g.modules.insert(m.id, m);
        assert_eq!(slice_of(&g, ModuleId::new(7)), Some(ContainerId::new(11)));
    }

    #[test]
    fn slice_of_module_in_slice_root() {
        let mut g = graph();
        let m = module(1, 10); // directly in auth
        g.modules.insert(m.id, m);
        assert_eq!(slice_of(&g, ModuleId::new(1)), Some(ContainerId::new(10)));
    }

    #[test]
    fn slice_of_module_in_nested_folder_resolves_to_slice() {
        let mut g = graph();
        let m = module(2, 12); // in auth/model
        g.modules.insert(m.id, m);
        assert_eq!(slice_of(&g, ModuleId::new(2)), Some(ContainerId::new(10)));
    }

    #[test]
    fn module_directly_in_layer_root_has_no_slice() {
        let mut g = graph();
        let m = module(3, 0); // directly in layer root
        g.modules.insert(m.id, m);
        assert_eq!(slice_of(&g, ModuleId::new(3)), None);
    }

    #[test]
    fn unknown_module_has_no_slice() {
        let g = graph();
        assert_eq!(slice_of(&g, ModuleId::new(99)), None);
    }
}
