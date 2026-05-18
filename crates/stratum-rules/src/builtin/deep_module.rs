use petgraph::Direction;

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::DEEP_MODULE;
use crate::options::DeepModuleOptions;
use crate::rule::Rule;

/// Phase 5 proxy for "shallow module": flag any module whose outgoing-edge
/// surface (a stand-in for "public exports" until Phase 1's parser emits real
/// export counts) exceeds `options.max_exports`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeepModule;

impl Rule for DeepModule {
    type Scope = ModuleId;
    type Options = DeepModuleOptions;

    fn id(&self) -> RuleId {
        DEEP_MODULE
    }
    fn slug(&self) -> &'static str {
        "stratum/deep-module"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &DeepModuleOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let Some(node) = graph.node_for(scope) else {
            return vec![];
        };
        let outgoing = graph
            .deps
            .edges_directed(node, Direction::Outgoing)
            .count();
        let surface = u32::try_from(outgoing).unwrap_or(u32::MAX);
        if surface <= options.max_exports {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Module surface ({surface}) exceeds the deep-module limit ({limit}); split it or hide some exports",
                limit = options.max_exports
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            suggestion: Some(
                "Group related exports behind a single facade or split the module along the natural fault lines.".into(),
            ),
        }]
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use petgraph::graph::DiGraph;
    use rustc_hash::FxHashMap;
    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        stage::Stage,
        types::Module,
        visibility::VisibilityScope,
    };

    fn module(id: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(1),
            layer: LayerId::new(1),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        }
    }

    fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for i in 0..n {
            let m = module(i);
            let node = g.deps.add_node(m.id);
            g.node_index.insert(m.id, node);
            g.modules.insert(m.id, m);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn surface_under_limit_no_violation() {
        let g = build(5, &[(0, 1), (0, 2)]);
        let v = DeepModule.check(
            &g,
            ModuleId::new(0),
            &DeepModuleOptions { max_exports: 12 },
            Severity::Info,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn surface_over_limit_emits_violation() {
        let g = build(
            6,
            &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        );
        let v = DeepModule.check(
            &g,
            ModuleId::new(0),
            &DeepModuleOptions { max_exports: 3 },
            Severity::Info,
        );
        assert_eq!(v.len(), 1);
    }
}
