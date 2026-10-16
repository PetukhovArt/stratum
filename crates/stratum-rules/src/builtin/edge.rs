use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::{
    edge::Edge,
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

pub(crate) struct Finding {
    pub message: String,
    pub suggestion: Option<String>,
}

pub(crate) type Judge<O> = fn(&CompoundGraph, ModuleId, ModuleId, &O) -> Option<Finding>;

/// The judge of an edge rule over options of type `O`; only built-in rules have one.
#[derive(Debug)]
pub struct EdgeJudge<O>(pub(crate) Judge<O>);

type Breaks = dyn Fn(&CompoundGraph, ModuleId, ModuleId) -> bool + Send + Sync;

pub struct BoundJudge(Box<Breaks>);

impl BoundJudge {
    pub(crate) fn new<O: Send + Sync + 'static>(judge: Judge<O>, options: O) -> Self {
        Self(Box::new(move |graph, from, to| {
            judge(graph, from, to, &options).is_some()
        }))
    }

    pub(crate) fn breaks(&self, graph: &CompoundGraph, from: ModuleId, to: ModuleId) -> bool {
        (self.0)(graph, from, to)
    }
}

impl std::fmt::Debug for BoundJudge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundJudge")
    }
}

pub(crate) fn edge_violations(
    graph: &CompoundGraph,
    scope: ModuleId,
    rule: RuleId,
    severity: Severity,
    mut judge: impl FnMut(ModuleId) -> Option<Finding>,
) -> Vec<Violation> {
    let (Some(from), Some(node)) = (graph.modules.get(&scope), graph.node_for(scope)) else {
        return vec![];
    };
    let mut out: Vec<Violation> = graph
        .deps
        .edges_directed(node, Direction::Outgoing)
        .filter_map(|edge| {
            let to = graph.deps[edge.target()];
            let finding = judge(to)?;
            Some(Violation {
                rule,
                severity,
                message: finding.message,
                file: from.path.clone(),
                location: SourceLocation { line: 1, column: 1 },
                modules: vec![scope, to],
                edge: Some(Edge {
                    from: scope,
                    to,
                    kind: *edge.weight(),
                }),
                suggestion: finding.suggestion,
            })
        })
        .collect();
    out.sort_by_key(|v| v.modules[1].raw());
    out
}
