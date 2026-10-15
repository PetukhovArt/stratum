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
        let (exports, edges) = self.edges_of(path, source, &path_to_id)?;
        let stage = stratum_parser_ts::extract_stage(source).unwrap_or(self.config.default_stage);

        let old: Vec<(ModuleId, EdgeKind)> = graph
            .deps
            .raw_edges()
            .iter()
            .filter(|e| e.source() == node)
            .map(|e| (graph.deps[e.target()], e.weight))
            .collect();
        if old == edges && module.stage == stage && module.exports == exports {
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

        graph.deps = deps;
        if let Some(m) = graph.modules.get_mut(&id) {
            m.stage = stage;
            m.exports = exports;
        }
        Ok(true)
    }
}
