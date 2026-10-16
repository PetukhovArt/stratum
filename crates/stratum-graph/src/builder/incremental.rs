use camino::{Utf8Path, Utf8PathBuf};
use petgraph::graph::DiGraph;
use rustc_hash::FxHashMap;
use stratum_core::edge::EdgeKind;
use stratum_core::ids::ModuleId;

use super::{BuildError, GraphBuilder};
use crate::graph::CompoundGraph;

impl GraphBuilder {
    /// # Errors
    /// [`BuildError::Parser`] if the extractor rejects `source`; `graph` is untouched then.
    pub fn update_module(
        &self,
        graph: &mut CompoundGraph,
        id: ModuleId,
        source: &str,
    ) -> Result<bool, BuildError> {
        let (Some(module), Some(&node)) = (graph.modules.get(&id), graph.node_index.get(&id))
        else {
            return Ok(false);
        };
        let Some(path) = Utf8Path::from_path(&module.path) else {
            return Ok(false);
        };
        let path_to_id: FxHashMap<Utf8PathBuf, ModuleId> = graph
            .modules
            .values()
            .filter_map(|m| Some((Utf8PathBuf::from_path_buf(m.path.clone()).ok()?, m.id)))
            .collect();
        let facts = self.facts_of(path, source, &path_to_id)?;
        let (exports, edges, calls) = (facts.exports, facts.edges, facts.calls);
        let purity = facts.purity.unwrap_or(self.config.default_purity);

        let old: Vec<(ModuleId, EdgeKind)> = graph
            .deps
            .raw_edges()
            .iter()
            .filter(|e| e.source() == node)
            .map(|e| (graph.deps[e.target()], e.weight))
            .collect();
        if old == edges
            && module.purity == purity
            && module.exports == exports
            && module.calls == calls
        {
            return Ok(false);
        }

        let mut deps = DiGraph::with_capacity(
            graph.deps.node_count(),
            graph.deps.edge_count() - old.len() + edges.len(),
        );
        for n in graph.deps.node_indices() {
            deps.add_node(graph.deps[n]);
        }
        let mut pending = Some(edges);
        let mut add_pending = |deps: &mut DiGraph<ModuleId, EdgeKind>| {
            for (to, kind) in pending.take().into_iter().flatten() {
                if let Some(&to_node) = graph.node_index.get(&to) {
                    deps.add_edge(node, to_node, kind);
                }
            }
        };
        for e in graph.deps.raw_edges() {
            if e.source() == node {
                continue;
            }
            if e.source() > node {
                add_pending(&mut deps);
            }
            deps.add_edge(e.source(), e.target(), e.weight);
        }
        add_pending(&mut deps);

        graph.replace_deps(deps);
        if let Some(m) = graph.modules.get_mut(&id) {
            m.purity = purity;
            m.exports = exports;
            m.calls = calls;
        }
        Ok(true)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use stratum_core::{ids::LayerId, purity::Purity, types::Layer};

    use super::*;
    use crate::BuildConfig;
    use crate::index::GraphIndex;

    #[test]
    fn an_edit_that_only_adds_a_tracked_call_updates_module_calls() {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-ts")
            .canonicalize_utf8()
            .unwrap();
        let builder = GraphBuilder::new(BuildConfig {
            project_root: root,
            layers: vec![Layer {
                id: LayerId::new(0),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
                slices: false,
            }],
            structure: stratum_core::structure::Structure::default(),
            tracked_calls: ["inject".to_owned()].into(),
            default_purity: Purity::new(2).unwrap(),
        });
        let mut graph = builder.build().unwrap();
        let id = graph
            .modules
            .values()
            .find(|m| m.path.ends_with("format.ts"))
            .unwrap()
            .id;
        assert!(graph.modules[&id].calls.is_empty());
        let index_before = graph.index().clone();

        let source = "export const formatId = (id: string) => `[${id}]`;\nconst x = inject('k');\n";
        assert!(builder.update_module(&mut graph, id, source).unwrap());
        assert_eq!(index_before, GraphIndex::of(&graph));

        let calls = &graph.modules[&id].calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(
            (calls[0].name.as_str(), calls[0].location.line),
            ("inject", 2)
        );
    }
}
