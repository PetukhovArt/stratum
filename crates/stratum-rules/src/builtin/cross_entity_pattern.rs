use std::collections::{BTreeMap, BTreeSet};

use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::CROSS_ENTITY_PATTERN;
use crate::options::CrossEntityPatternOptions;
use crate::rule::Rule;
use crate::scope::ProjectScope;

#[derive(Debug, Default, Clone, Copy)]
pub struct CrossEntityPattern;

impl Rule for CrossEntityPattern {
    type Scope = ProjectScope;
    type Options = CrossEntityPatternOptions;

    fn id(&self) -> RuleId {
        CROSS_ENTITY_PATTERN
    }
    fn slug(&self) -> &'static str {
        "stratum/cross-entity-pattern"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &CrossEntityPatternOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        let mut entity_modules: BTreeMap<String, Vec<ModuleId>> = BTreeMap::new();
        for (mid, m) in &graph.modules {
            if let Some(name) = entity_name(&m.path.to_string_lossy()) {
                entity_modules.entry(name).or_default().push(*mid);
            }
        }

        let mut out = Vec::new();
        for (entity, members) in &entity_modules {
            let member_set: BTreeSet<ModuleId> = members.iter().copied().collect();
            let mut feature_importers: BTreeSet<String> = BTreeSet::new();
            for (importer_id, importer) in &graph.modules {
                if member_set.contains(importer_id) {
                    continue;
                }
                let Some(feat) = feature_name(&importer.path.to_string_lossy()) else {
                    continue;
                };
                let Some(node) = graph.node_for(*importer_id) else {
                    continue;
                };
                let imports_entity = graph
                    .deps
                    .edges_directed(node, petgraph::Direction::Outgoing)
                    .any(|e| member_set.contains(&graph.deps[e.target()]));
                if imports_entity {
                    feature_importers.insert(feat);
                }
            }
            let fanout = u32::try_from(feature_importers.len()).unwrap_or(u32::MAX);
            if fanout <= options.max_fanout {
                continue;
            }
            let representative = members
                .iter()
                .find(|mid| {
                    graph
                        .modules
                        .get(*mid)
                        .is_some_and(|m| m.path.to_string_lossy().ends_with("/index.ts"))
                })
                .or_else(|| members.first())
                .copied();
            let Some(rep_id) = representative else {
                continue;
            };
            let Some(rep) = graph.modules.get(&rep_id) else {
                continue;
            };
            out.push(Violation {
                rule: self.id(),
                severity,
                message: format!(
                    "Entity '{entity}' is imported by {fanout} features (limit {limit})",
                    limit = options.max_fanout
                ),
                file: rep.path.clone(),
                location: SourceLocation { line: 1, column: 1 },
                modules: vec![rep_id],
                edge: None,
                suggestion: Some(
                    "Narrow the entity's public surface or split it into smaller entities so each feature reaches only what it needs.".into(),
                ),
            });
        }
        out
    }
}

use petgraph::visit::EdgeRef;

fn entity_name(path: &str) -> Option<String> {
    let norm = path.replace('\\', "/");
    let needle = "/entities/";
    let i = norm.find(needle)?;
    let rest = &norm[i + needle.len()..];
    let end = rest.find('/').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn feature_name(path: &str) -> Option<String> {
    let norm = path.replace('\\', "/");
    let needle = "/features/";
    let i = norm.find(needle)?;
    let rest = &norm[i + needle.len()..];
    let end = rest.find('/').unwrap_or(rest.len());
    Some(rest[..end].to_string())
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
        purity::Purity,
        types::Module,
        visibility::VisibilityScope,
    };

    fn m(id: u32, path: &str) -> Module {
        Module {
            id: ModuleId::new(id),
            path: PathBuf::from(path),
            container: ContainerId::new(1),
            layer: LayerId::new(1),
            purity: Purity::new(2).unwrap(),
            visibility: VisibilityScope::Public,
        }
    }

    fn build(mods: Vec<Module>, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph {
            modules: IndexMap::new(),
            containers: IndexMap::new(),
            layers: IndexMap::new(),
            deps: DiGraph::new(),
            node_index: FxHashMap::default(),
        };
        for mm in mods {
            let node = g.deps.add_node(mm.id);
            g.node_index.insert(mm.id, node);
            g.modules.insert(mm.id, mm);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn fanout_under_threshold_no_violation() {
        let g = build(
            vec![
                m(0, "src/entities/user/index.ts"),
                m(1, "src/features/cart/view.ts"),
                m(2, "src/features/profile/view.ts"),
            ],
            &[(1, 0), (2, 0)],
        );
        let v = CrossEntityPattern.check(
            &g,
            ProjectScope,
            &CrossEntityPatternOptions { max_fanout: 5 },
            Severity::Warning,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn fanout_over_threshold_emits_violation() {
        let mut mods = vec![m(0, "src/entities/user/index.ts")];
        for i in 1..=6 {
            mods.push(m(i, &format!("src/features/f{i}/view.ts")));
        }
        let edges: Vec<(u32, u32)> = (1..=6).map(|i| (i, 0)).collect();
        let g = build(mods, &edges);
        let v = CrossEntityPattern.check(
            &g,
            ProjectScope,
            &CrossEntityPatternOptions { max_fanout: 5 },
            Severity::Warning,
        );
        assert_eq!(v.len(), 1);
    }
}
