//! Instability (Robert C. Martin): `I = Ce / (Ca + Ce)`.
//!
//! Computed per module over the dependency DAG. `Ce` (efferent coupling) is the
//! number of *distinct* modules this module depends on; `Ca` (afferent
//! coupling) is the number of distinct modules that depend on it. Parallel
//! edges (the same target imported twice) collapse; self-loops are ignored.
//!
//! `I` ranges from `0.0` (maximally stable — many depend on it, it depends on
//! nothing) to `1.0` (maximally unstable). It is undefined for an isolated node
//! (no edges) and reported as `None`.

use petgraph::Direction;
use rustc_hash::FxHashSet;
use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

/// Afferent (`Ca`) and efferent (`Ce`) coupling for `module`, counted by
/// distinct neighbour modules. Returns `None` if the module is unknown.
#[must_use]
pub fn coupling(g: &CompoundGraph, module: ModuleId) -> Option<(u32, u32)> {
    let node = g.node_for(module)?;
    let distinct = |dir: Direction| -> u32 {
        let n = g
            .deps
            .neighbors_directed(node, dir)
            .map(|n| g.deps[n])
            .filter(|&m| m != module) // ignore self-loops
            .collect::<FxHashSet<_>>()
            .len();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let ce = distinct(Direction::Outgoing);
    let ca = distinct(Direction::Incoming);
    Some((ca, ce))
}

/// Instability `I = Ce / (Ca + Ce)` in `[0.0, 1.0]`.
///
/// Returns `None` if the module is unknown or has no edges (`Ca + Ce == 0`).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn instability(g: &CompoundGraph, module: ModuleId) -> Option<f64> {
    let (ca, ce) = coupling(g, module)?;
    let total = ca + ce;
    if total == 0 {
        return None;
    }
    Some(f64::from(ce) / f64::from(total))
}

/// An edge that violates the Stable Dependencies Principle: `from` depends on
/// `to` while `to` is *less* stable (higher instability) than `from`.
#[derive(Debug, Clone, PartialEq)]
pub struct SdpViolation {
    pub from: ModuleId,
    pub to: ModuleId,
    pub from_instability: f64,
    pub to_instability: f64,
}

/// All SDP-violating edges, deterministically ordered by `(from, to)`.
///
/// An edge `from → to` violates SDP when `I(to) > I(from)` — dependencies should
/// point toward *more* stable code, not less. Edges where either endpoint has an
/// undefined instability are skipped (cannot have edges and be undefined, but the
/// guard keeps the function total).
#[must_use]
pub fn sdp_violations(g: &CompoundGraph) -> Vec<SdpViolation> {
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    let mut out: Vec<SdpViolation> = Vec::new();
    for node in g.deps.node_indices() {
        let from = g.deps[node];
        for to_node in g.deps.neighbors_directed(node, Direction::Outgoing) {
            let to = g.deps[to_node];
            if to == from || !seen.insert((from.raw(), to.raw())) {
                continue;
            }
            let (Some(fi), Some(ti)) = (instability(g, from), instability(g, to)) else {
                continue;
            };
            if ti > fi {
                out.push(SdpViolation {
                    from,
                    to,
                    from_instability: fi,
                    to_instability: ti,
                });
            }
        }
    }
    out.sort_by_key(|v| (v.from.raw(), v.to.raw()));
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
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

    fn build(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for i in 0..n {
            let id = ModuleId::new(i);
            let m = Module {
                id,
                path: PathBuf::from(format!("src/m{i}.ts")),
                container: ContainerId::new(0),
                layer: LayerId::new(0),
                purity: Purity::new(2).unwrap(),
                visibility: VisibilityScope::Public,
            };
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(id, m);
        }
        for (a, b) in edges {
            let na = g.node_index[&ModuleId::new(*a)];
            let nb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    // Chain 0 -> 1 -> 2. Hand computed:
    //   0: Ce=1 (->1), Ca=0           => I = 1/1   = 1.0
    //   1: Ce=1 (->2), Ca=1 (0->)     => I = 1/2   = 0.5
    //   2: Ce=0,       Ca=1 (1->)     => I = 0/1   = 0.0
    #[test]
    fn instability_matches_hand_count_on_chain() {
        let g = build(3, &[(0, 1), (1, 2)]);
        assert_eq!(coupling(&g, ModuleId::new(0)), Some((0, 1)));
        assert_eq!(coupling(&g, ModuleId::new(1)), Some((1, 1)));
        assert_eq!(coupling(&g, ModuleId::new(2)), Some((1, 0)));
        assert_eq!(instability(&g, ModuleId::new(0)), Some(1.0));
        assert_eq!(instability(&g, ModuleId::new(1)), Some(0.5));
        assert_eq!(instability(&g, ModuleId::new(2)), Some(0.0));
    }

    #[test]
    fn parallel_edges_collapse_to_distinct_neighbours() {
        // 0 imports 1 twice; still Ce(0) = 1.
        let g = build(2, &[(0, 1), (0, 1)]);
        assert_eq!(coupling(&g, ModuleId::new(0)), Some((0, 1)));
    }

    #[test]
    fn self_loop_contributes_to_neither_coupling() {
        // 0 -> 0 (self) and 0 -> 1. The self-loop must not inflate Ca or Ce.
        let g = build(2, &[(0, 0), (0, 1)]);
        assert_eq!(coupling(&g, ModuleId::new(0)), Some((0, 1)));
    }

    #[test]
    fn multiple_sdp_violations_are_sorted() {
        // Two independent stable-hub→unstable-leaf stars: hub 0→leaf 3, hub 4→leaf 7.
        let g = build(
            8,
            &[
                (1, 0),
                (2, 0),
                (3, 0),
                (0, 3),
                (5, 4),
                (6, 4),
                (7, 4),
                (4, 7),
            ],
        );
        let v = sdp_violations(&g);
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].from, v[0].to), (ModuleId::new(0), ModuleId::new(3)));
        assert_eq!((v[1].from, v[1].to), (ModuleId::new(4), ModuleId::new(7)));
    }

    #[test]
    fn isolated_node_has_undefined_instability() {
        let g = build(1, &[]);
        assert_eq!(instability(&g, ModuleId::new(0)), None);
    }

    #[test]
    fn unknown_module_is_none() {
        let g = build(1, &[]);
        assert_eq!(instability(&g, ModuleId::new(42)), None);
        assert_eq!(coupling(&g, ModuleId::new(42)), None);
    }

    // Chain 0->1->2 has no SDP violations (each depends on a more stable node).
    #[test]
    fn well_ordered_chain_has_no_sdp_violations() {
        let g = build(3, &[(0, 1), (1, 2)]);
        assert!(sdp_violations(&g).is_empty());
    }

    // Add 2 -> 0: node 2 (I=0 before; recompute) now depends on node 0.
    // Recompute with edges {0->1, 1->2, 2->0}:
    //   0: Ce=1(->1), Ca=1(2->) => I=0.5
    //   1: Ce=1(->2), Ca=1(0->) => I=0.5
    //   2: Ce=1(->0), Ca=1(1->) => I=0.5
    // All equal => still no SDP violation (need strictly greater).
    #[test]
    fn equal_instability_is_not_a_violation() {
        let g = build(3, &[(0, 1), (1, 2), (2, 0)]);
        assert!(sdp_violations(&g).is_empty());
    }

    // Star: 1,2,3 all depend on 0 (hub). 0 also depends on 3 (a leaf-ish node).
    //   0: Ce=1(->3), Ca=3(1,2,3->) => I = 1/4 = 0.25
    //   3: Ce=1(->0), Ca=1(0->)     => I = 1/2 = 0.5
    // Edge 0->3: I(3)=0.5 > I(0)=0.25 => SDP violation (stable hub depends on
    // a less stable node).
    #[test]
    fn detects_stable_depending_on_unstable() {
        let g = build(4, &[(1, 0), (2, 0), (3, 0), (0, 3)]);
        let v = sdp_violations(&g);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].from, ModuleId::new(0));
        assert_eq!(v[0].to, ModuleId::new(3));
        assert_eq!(v[0].from_instability, 0.25);
        assert_eq!(v[0].to_instability, 0.5);
    }
}
