use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, metrics::depth_ratio};

use crate::ids::DEPTH_RATIO;
use crate::options::DepthRatioOptions;
use crate::rule::Rule;

#[derive(Debug, Default, Clone, Copy)]
pub struct DepthRatio;

impl Rule for DepthRatio {
    type Scope = ModuleId;
    type Options = DepthRatioOptions;

    fn id(&self) -> RuleId {
        DEPTH_RATIO
    }
    fn slug(&self) -> &'static str {
        "stratum/depth-ratio"
    }
    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &DepthRatioOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let Some(m) = graph.modules.get(&scope) else {
            return vec![];
        };
        let Some(ratio) = depth_ratio(graph, scope) else {
            return vec![];
        };
        if ratio >= options.min_ratio {
            return vec![];
        }
        vec![Violation {
            rule: self.id(),
            severity,
            message: format!(
                "Depth ratio {ratio:.2} (modules in its container / its {exports} exports) is below the configured minimum {min:.2}; module exposes too much relative to its container size",
                exports = m.exports.count,
                min = options.min_ratio
            ),
            file: m.path.clone(),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![scope],
            edge: None,
            suggestion: Some(
                "Consolidate functionality so each public symbol stands for more internal work, or split the container.".into(),
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
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Exports, Module},
        visibility::VisibilityScope,
    };

    fn module(id: u32, exports: u32) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(format!("src/m{id}.ts")),
            container: ContainerId::new(1),
            layer: LayerId::new(0),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
            exports: Exports {
                count: exports,
                star_reexports: 0,
            },
        }
    }

    #[test]
    fn flags_modules_whose_exports_outweigh_their_container() {
        let mut g = CompoundGraph::empty();
        for m in [module(0, 0), module(1, 1), module(2, 2), module(3, 0)] {
            let node = g.deps.add_node(m.id);
            g.node_index.insert(m.id, node);
            g.modules.insert(m.id, m);
        }
        let options = DepthRatioOptions { min_ratio: 3.0 };
        let flagged: Vec<u32> = (0..4)
            .filter(|&i| {
                !DepthRatio
                    .check(&g, ModuleId::new(i), &options, Severity::Info)
                    .is_empty()
            })
            .collect();
        assert_eq!(flagged, [2]);
    }
}
