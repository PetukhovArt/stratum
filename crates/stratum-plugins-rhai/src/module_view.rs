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
    pub purity: i64,
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
    let written = g.modules.get(&id).map_or(&[][..], |m| m.imports.as_slice());
    let mut taken = vec![false; written.len()];
    g.deps
        .edges_directed(node, Direction::Outgoing)
        .filter_map(|edge| {
            let to = g.deps[edge.target()];
            let target = g.modules.get(&to)?;
            let kind = match edge.weight() {
                EdgeKind::Static => "static",
                EdgeKind::Di => "di",
                EdgeKind::Runtime => "runtime",
            };
            let mut import = Map::new();
            import.insert("path".into(), forward_slash(target).into());
            import.insert("layer".into(), layer_name(g, target.layer).into());
            import.insert("kind".into(), kind.into());
            let source = (0..written.len()).rev().find(|&i| {
                !taken[i] && written[i].target == Some(to) && written[i].kind == *edge.weight()
            });
            if let Some(i) = source {
                taken[i] = true;
                let w = &written[i];
                import.insert("specifier".into(), w.specifier.clone().into());
                import.insert("line".into(), i64::from(w.location.line).into());
                import.insert("type_only".into(), w.type_only.into());
            }
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
        purity: i64::from(m.purity.rank()),
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
        .register_get("purity", |m: &mut ModuleView| m.purity)
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
