use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, metrics::miller_fan_in};

use crate::ids::MILLER_LIMIT;
use crate::options::MillerLimitOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct MillerLimit;

impl Rule for MillerLimit {
    type Scope = ModuleId;
    type Options = MillerLimitOptions;

    fn id(&self) -> RuleId {
        MILLER_LIMIT
    }
    fn slug(&self) -> &'static str {
        "stratum/miller-limit"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &MillerLimitOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let fan_in = miller_fan_in(graph, scope);
        if u32::try_from(fan_in).unwrap_or(u32::MAX) <= options.max_children {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Imported by {fan_in} modules from other containers whose layers depend on this one (limit {limit}); this module is a change hotspot",
                limit = options.max_children
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            fix: Some(
                "Split the module so each consumer reaches only the surface it needs, or move the implementation closer to its consumers.".into(),
            ),
        }]
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn module(id: u32, container: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("src/m{id}.ts")),
            container: ContainerId::new(container),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: stratum_core::types::Exports::default(),
            calls: vec![],
            imports: vec![],
            suppressions: vec![],
            local_refs: vec![],
        }
    }

    fn build(mods: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for m in mods {
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
    fn under_limit_no_violation() {
        let target = module(0, 1);
        let mut mods = vec![target];
        for i in 1..=3u32 {
            mods.push(module(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=3).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let viols = MillerLimit.check(
            &g,
            ModuleId::new(0),
            &MillerLimitOptions { max_children: 7 },
            Severity::Warning,
        );
        assert!(viols.is_empty());
    }

    #[test]
    fn over_limit_emits_violation() {
        let target = module(0, 1);
        let mut mods = vec![target];
        for i in 1..=8u32 {
            mods.push(module(i, 2));
        }
        let edges: Vec<(u32, u32)> = (1..=8).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let viols = MillerLimit.check(
            &g,
            ModuleId::new(0),
            &MillerLimitOptions { max_children: 7 },
            Severity::Warning,
        );
        assert_eq!(viols.len(), 1);
        assert_eq!(viols[0].rule, MILLER_LIMIT);
    }
}
