use camino::{Utf8Path, Utf8PathBuf};
use petgraph::graph::DiGraph;
use rustc_hash::FxHashMap;
use stratum_core::edge::EdgeKind;
use stratum_core::ids::ModuleId;

use super::{BuildError, GraphBuilder, edges};
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
        let purity = facts.purity.unwrap_or(self.config.default_purity);
        let edges: Vec<(ModuleId, EdgeKind)> = edges(&facts.imports).collect();

        let old: Vec<(ModuleId, EdgeKind)> = graph
            .deps
            .raw_edges()
            .iter()
            .filter(|e| e.source() == node)
            .map(|e| (graph.deps[e.target()], e.weight))
            .collect();
        let edges_changed = old != edges;
        if !edges_changed
            && module.purity == purity
            && module.exports == facts.exports
            && module.calls == facts.calls
            && module.imports == facts.imports
            && module.suppressions == facts.suppressions
            && module.local_refs == facts.local_refs
        {
            return Ok(false);
        }

        if edges_changed {
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
        }

        if let Some(m) = graph.modules.get_mut(&id) {
            m.purity = purity;
            m.exports = facts.exports;
            m.calls = facts.calls;
            m.imports = facts.imports;
            m.suppressions = facts.suppressions;
            m.local_refs = facts.local_refs;
        }
        Ok(true)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use stratum_core::{ids::LayerId, purity::Purity, types::Layer};
    use stratum_parser_ts::TrackedCalls;

    use super::*;
    use crate::BuildConfig;
    use crate::index::GraphIndex;

    fn tiny_ts(tracked_calls: TrackedCalls) -> GraphBuilder {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-ts")
            .canonicalize_utf8()
            .unwrap();
        GraphBuilder::new(BuildConfig {
            project_root: root,
            layers: vec![Layer {
                id: LayerId::new(0),
                name: "shared".into(),
                path: PathBuf::from("src/shared"),
                depends_on: vec![],
                slices: false,
            }],
            structure: stratum_core::structure::Structure::default(),
            tracked_calls,
            default_purity: Purity::new(2).unwrap(),
        })
    }

    fn module_ending(graph: &CompoundGraph, suffix: &str) -> ModuleId {
        graph
            .modules
            .values()
            .find(|m| m.path.ends_with(suffix))
            .unwrap()
            .id
    }

    #[test]
    fn an_edit_that_only_adds_a_tracked_call_updates_module_calls() {
        let mut tracked = TrackedCalls::default();
        tracked.insert("inject").unwrap();
        let builder = tiny_ts(tracked);
        let (mut graph, _) = builder.build().unwrap();
        let id = module_ending(&graph, "format.ts");
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

    #[test]
    fn a_multi_line_import_is_located_at_its_import_keyword() {
        let builder = tiny_ts(TrackedCalls::default());
        let (mut graph, _) = builder.build().unwrap();
        let from = module_ending(&graph, "format.ts");
        let to = module_ending(&graph, "http.ts");

        let source = "// header\n\nimport {\n  http,\n} from '../api/http';\nexport const formatId = http;\n";
        assert!(builder.update_module(&mut graph, from, source).unwrap());

        let at = graph.import_location(from, to).unwrap();
        assert_eq!((at.line, at.column), (3, 1));
    }
}
