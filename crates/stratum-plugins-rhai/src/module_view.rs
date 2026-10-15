use petgraph::Direction;
use petgraph::visit::EdgeRef;
use rhai::{Array, Dynamic, Engine, Map};

use stratum_core::{
    edge::EdgeKind,
    ids::{LayerId, ModuleId},
    types::Module,
};
use stratum_graph::{CompoundGraph, direct_dependents};

#[derive(Debug, Clone)]
pub struct ModuleView {
    pub id: i64,
    pub path: String,
    pub layer: String,
    pub container: i64,
    pub stage: i64,
    pub dependents: Vec<i64>,
    pub imports: Array,
    pub metadata: Map,
}

fn layer_name(g: &CompoundGraph, layer: LayerId) -> String {
    g.layers
        .get(&layer)
        .map_or_else(String::new, |l| l.name.clone())
}

fn forward_slash(m: &Module) -> String {
    m.path.to_string_lossy().replace('\\', "/")
}

fn imports(g: &CompoundGraph, id: ModuleId) -> Array {
    let Some(node) = g.node_for(id) else {
        return Array::new();
    };
    g.deps
        .edges_directed(node, Direction::Outgoing)
        .filter_map(|edge| {
            let target = g.modules.get(&g.deps[edge.target()])?;
            let kind = match edge.weight() {
                EdgeKind::Static => "static",
                EdgeKind::Di => "di",
                EdgeKind::Runtime => "runtime",
            };
            let mut import = Map::new();
            import.insert("path".into(), forward_slash(target).into());
            import.insert("layer".into(), layer_name(g, target.layer).into());
            import.insert("kind".into(), kind.into());
            Some(Dynamic::from_map(import))
        })
        .collect()
}

#[must_use]
pub fn build_view(g: &CompoundGraph, id: ModuleId) -> Option<ModuleView> {
    let m = g.modules.get(&id)?;
    let dependents = direct_dependents(g, id)
        .into_iter()
        .map(|d| i64::from(d.raw()))
        .collect();
    Some(ModuleView {
        id: i64::from(id.raw()),
        path: forward_slash(m),
        layer: layer_name(g, m.layer),
        container: i64::from(m.container.raw()),
        stage: i64::from(m.stage.rank()),
        dependents,
        imports: imports(g, id),
        metadata: Map::new(),
    })
}

pub fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<ModuleView>("ModuleView")
        .register_get("id", |m: &mut ModuleView| m.id)
        .register_get("path", |m: &mut ModuleView| m.path.clone())
        .register_get("layer", |m: &mut ModuleView| m.layer.clone())
        .register_get("container", |m: &mut ModuleView| m.container)
        .register_get("stage", |m: &mut ModuleView| m.stage)
        .register_get("dependents", |m: &mut ModuleView| {
            m.dependents
                .iter()
                .copied()
                .map(rhai::Dynamic::from)
                .collect::<rhai::Array>()
        })
        .register_get("imports", |m: &mut ModuleView| m.imports.clone())
        .register_get("metadata", |m: &mut ModuleView| m.metadata.clone());
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::engine::build_engine;

    fn sample_view() -> ModuleView {
        ModuleView {
            id: 7,
            path: "src/legacy/foo.ts".to_string(),
            layer: "shared".to_string(),
            container: 2,
            stage: 3,
            dependents: vec![1, 2],
            imports: Array::new(),
            metadata: Map::new(),
        }
    }

    #[test]
    fn path_and_layer_readable_from_rhai() {
        let mut engine = build_engine();
        register(&mut engine);
        let mut scope = rhai::Scope::new();
        scope.push("m", sample_view());
        let s: String = engine.eval_with_scope(&mut scope, "m.path").unwrap();
        assert_eq!(s, "src/legacy/foo.ts");
        let layer: String = engine.eval_with_scope(&mut scope, "m.layer").unwrap();
        assert_eq!(layer, "shared");
    }
}
