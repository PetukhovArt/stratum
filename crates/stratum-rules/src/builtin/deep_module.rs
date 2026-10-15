use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::DEEP_MODULE;
use crate::options::DeepModuleOptions;
use crate::rule::Rule;

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
        let exports = m.exports;
        if exports.count <= options.max_exports {
            return vec![];
        }
        let stars = match exports.star_reexports {
            0 => String::new(),
            n => {
                format!(", {n} of them `export *` re-exports counted as 1 each that may hide more")
            }
        };
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Module exports {count} names{stars} (limit {limit}); split it or hide some exports",
                count = exports.count,
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

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        stage::Stage,
        types::{Exports, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, exports: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("m{id}.ts")),
            container: ContainerId::new(1),
            layer: LayerId::new(1),
            stage: Stage::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: Exports {
                count: exports,
                star_reexports: 0,
            },
        }
    }

    #[test]
    fn counts_exports_not_imports() {
        let mut g = CompoundGraph::empty();
        for m in [module(0, 0), module(1, 5), module(2, 0), module(3, 0)] {
            let node = g.deps.add_node(m.id);
            g.node_index.insert(m.id, node);
            g.modules.insert(m.id, m);
        }
        for to in 1..=3 {
            let from = g.node_index[&ModuleId::new(0)];
            let to = g.node_index[&ModuleId::new(to)];
            g.deps.add_edge(from, to, EdgeKind::Static);
        }
        let options = DeepModuleOptions { max_exports: 2 };
        let flagged: Vec<u32> = (0..4)
            .filter(|&i| {
                !DeepModule
                    .check(&g, ModuleId::new(i), &options, Severity::Info)
                    .is_empty()
            })
            .collect();
        assert_eq!(flagged, [1]);
    }
}
