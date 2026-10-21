use std::cmp::Reverse;

use petgraph::algo::tarjan_scc;
use petgraph::visit::{EdgeFiltered, EdgeRef};
use rustc_hash::FxHashMap;

use stratum_core::ids::ModuleId;

use crate::graph::CompoundGraph;

mod feedback;

const MAX_SCORED_EDGES: usize = 64;
const MAX_CUTS: usize = 3;

/// A non-trivial strongly-connected component: a cycle of two or more modules,
/// or a singleton with an explicit self-loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    pub modules: Vec<ModuleId>,
}

impl Cycle {
    /// The members ordered by their `/`-separated path, then raw id: the same
    /// order whatever order the graph was built in. The first is the anchor.
    #[must_use]
    pub fn by_path(&self, g: &CompoundGraph) -> Vec<ModuleId> {
        let mut members = self.modules.clone();
        members.sort_by_cached_key(|m| {
            let path = g.modules.get(m).map_or_else(String::new, |module| {
                module.path.to_string_lossy().replace('\\', "/")
            });
            (path, m.raw())
        });
        members
    }
}

/// Find every cycle: SCCs of size ≥ 2, plus singletons with a self-loop.
///
/// Modules within each cycle are sorted by their raw id; the returned vec is
/// also sorted by the first module's raw id — deterministic across runs.
#[must_use]
pub fn find_cycles(g: &CompoundGraph) -> Vec<Cycle> {
    find_cycles_where(g, |_, _| true)
}

/// [`find_cycles`] over only the dependencies `keep(from, to)` accepts.
#[must_use]
pub fn find_cycles_where(
    g: &CompoundGraph,
    keep: impl Fn(ModuleId, ModuleId) -> bool,
) -> Vec<Cycle> {
    let kept = EdgeFiltered::from_fn(&g.deps, |e| keep(g.deps[e.source()], g.deps[e.target()]));
    let sccs = tarjan_scc(&kept);
    let mut cycles = Vec::new();
    for scc in sccs {
        if scc.len() >= 2 {
            let mut modules: Vec<ModuleId> = scc.iter().map(|n| g.deps[*n]).collect();
            modules.sort_by_key(|m| m.raw());
            cycles.push(Cycle { modules });
        } else if scc.len() == 1 {
            let n = scc[0];
            if g.deps.find_edge(n, n).is_some() && keep(g.deps[n], g.deps[n]) {
                cycles.push(Cycle {
                    modules: vec![g.deps[n]],
                });
            }
        }
    }
    cycles.sort_by_key(|c| c.modules.first().map_or(u32::MAX, |m| m.raw()));
    cycles
}

/// An import inside a cycle and how many of the cycle's modules leave it when
/// that one import is removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cut {
    pub from: ModuleId,
    pub to: ModuleId,
    pub freed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breaks {
    /// Up to three cuts, best first.
    pub cuts: Vec<Cut>,
    /// Imports a greedy feedback-arc ordering removes to make the cycle
    /// acyclic: an estimate of how many close it, not a minimum.
    pub closing: usize,
}

/// The up to three imports of `cycle` whose removal frees the most modules,
/// best first (ties by path), and the size of a greedy feedback-arc set. With
/// at most 64 imports inside the cycle all are scored; with more, the first 64
/// back edges of the greedy ordering sorted by (from path, to path). Same
/// result for any build order.
#[must_use]
pub fn break_candidates(g: &CompoundGraph, cycle: &Cycle) -> Breaks {
    break_candidates_where(g, cycle, |_, _| true)
}

/// [`break_candidates`] over only the dependencies `keep(from, to)` accepts: the
/// filter `cycle` was found with.
#[must_use]
pub fn break_candidates_where(
    g: &CompoundGraph,
    cycle: &Cycle,
    keep: impl Fn(ModuleId, ModuleId) -> bool,
) -> Breaks {
    let local = Local::of(g, cycle, &keep);
    let mut back = feedback::back_edges(&local);
    let closing = back.len();
    let pool: Vec<usize> = if local.edges.len() <= MAX_SCORED_EDGES {
        (0..local.edges.len()).collect()
    } else {
        back.truncate(MAX_SCORED_EDGES);
        back
    };
    let mut scratch = SccScratch::new(local.n);
    let mut scored: Vec<(usize, usize)> = pool
        .into_iter()
        .map(|e| (local.n - scratch.largest_cycle(&local, e), e))
        .collect();
    scored.sort_by_key(|&(freed, e)| (Reverse(freed), e));
    scored.truncate(MAX_CUTS);
    let cuts = scored
        .into_iter()
        .map(|(freed, e)| {
            let (u, v) = local.edges[e];
            Cut {
                from: local.members[u as usize],
                to: local.members[v as usize],
                freed,
            }
        })
        .collect();
    Breaks { cuts, closing }
}

struct Local {
    n: usize,
    members: Vec<ModuleId>,
    edges: Vec<(u32, u32)>,
    off: Vec<usize>,
    tg: Vec<u32>,
}

impl Local {
    fn of(g: &CompoundGraph, cycle: &Cycle, keep: &dyn Fn(ModuleId, ModuleId) -> bool) -> Self {
        let members = cycle.by_path(g);
        let n = members.len();
        let local_of: FxHashMap<ModuleId, u32> = members
            .iter()
            .enumerate()
            .map(|(i, m)| (*m, u32::try_from(i).unwrap_or(u32::MAX)))
            .collect();
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for (u, m) in members.iter().enumerate() {
            let Some(node) = g.node_for(*m) else { continue };
            for next in g.deps.neighbors(node) {
                let to = g.deps[next];
                if let Some(&v) = local_of.get(&to).filter(|_| keep(*m, to)) {
                    edges.push((u32::try_from(u).unwrap_or(u32::MAX), v));
                }
            }
        }
        edges.sort_unstable();
        edges.dedup();
        let mut off = vec![0; n + 1];
        for &(u, _) in &edges {
            off[u as usize + 1] += 1;
        }
        for i in 0..n {
            off[i + 1] += off[i];
        }
        let tg = edges.iter().map(|&(_, v)| v).collect();
        Self {
            n,
            members,
            edges,
            off,
            tg,
        }
    }
}

const UNVISITED: u32 = u32::MAX;

struct SccScratch {
    index: Vec<u32>,
    low: Vec<u32>,
    on_stack: Vec<bool>,
    stack: Vec<u32>,
    calls: Vec<(u32, usize)>,
}

impl SccScratch {
    fn new(n: usize) -> Self {
        Self {
            index: vec![UNVISITED; n],
            low: vec![0; n],
            on_stack: vec![false; n],
            stack: Vec::new(),
            calls: Vec::new(),
        }
    }

    fn largest_cycle(&mut self, local: &Local, skip: usize) -> usize {
        self.index.fill(UNVISITED);
        self.on_stack.fill(false);
        let (mut next, mut best) = (0_u32, 0_usize);
        for root in 0..local.n {
            if self.index[root] != UNVISITED {
                continue;
            }
            self.enter(root, &mut next);
            self.calls
                .push((u32::try_from(root).unwrap_or(u32::MAX), local.off[root]));
            while let Some(&(v, pos)) = self.calls.last() {
                let vi = v as usize;
                if pos < local.off[vi + 1] {
                    if let Some(top) = self.calls.last_mut() {
                        top.1 += 1;
                    }
                    if pos == skip {
                        continue;
                    }
                    let w = local.tg[pos];
                    let wi = w as usize;
                    if self.index[wi] == UNVISITED {
                        self.enter(wi, &mut next);
                        self.calls.push((w, local.off[wi]));
                    } else if self.on_stack[wi] {
                        self.low[vi] = self.low[vi].min(self.index[wi]);
                    }
                    continue;
                }
                self.calls.pop();
                if self.low[vi] == self.index[vi] {
                    let mut size = 0;
                    while let Some(x) = self.stack.pop() {
                        self.on_stack[x as usize] = false;
                        size += 1;
                        if x == v {
                            break;
                        }
                    }
                    if size > 1 {
                        best = best.max(size);
                    }
                }
                if let Some(&(parent, _)) = self.calls.last() {
                    let p = parent as usize;
                    self.low[p] = self.low[p].min(self.low[vi]);
                }
            }
        }
        best
    }

    fn enter(&mut self, v: usize, next: &mut u32) {
        self.index[v] = *next;
        self.low[v] = *next;
        *next += 1;
        self.stack.push(u32::try_from(v).unwrap_or(u32::MAX));
        self.on_stack[v] = true;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use stratum_core::edge::EdgeKind;

    fn graph_with_edges(n: u32, edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        for i in 0..n {
            let id = ModuleId::new(i);
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
        }
        for (a, b) in edges {
            let fa = g.node_index[&ModuleId::new(*a)];
            let fb = g.node_index[&ModuleId::new(*b)];
            g.deps.add_edge(fa, fb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn dag_has_no_cycles() {
        let g = graph_with_edges(3, &[(0, 1), (1, 2)]);
        assert!(find_cycles(&g).is_empty());
    }

    #[test]
    fn finds_two_node_cycle() {
        let g = graph_with_edges(2, &[(0, 1), (1, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules, vec![ModuleId::new(0), ModuleId::new(1)]);
    }

    #[test]
    fn finds_three_node_cycle() {
        let g = graph_with_edges(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules.len(), 3);
    }

    #[test]
    fn finds_self_loop() {
        let g = graph_with_edges(1, &[(0, 0)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules, vec![ModuleId::new(0)]);
    }

    #[test]
    fn multiple_disjoint_cycles() {
        let g = graph_with_edges(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let cycles = find_cycles(&g);
        assert_eq!(cycles.len(), 2);
        assert_eq!(cycles[0].modules[0], ModuleId::new(0));
        assert_eq!(cycles[1].modules[0], ModuleId::new(2));
    }

    #[test]
    fn a_dropped_dependency_breaks_its_cycle_and_self_loop() {
        let g = graph_with_edges(3, &[(0, 1), (1, 0), (2, 2)]);
        let id = ModuleId::new;
        assert!(find_cycles_where(&g, |from, to| from != id(1) && to != id(2)).is_empty());
        assert_eq!(find_cycles_where(&g, |from, _| from != id(2)).len(), 1);
    }

    #[test]
    fn candidates_skip_a_dropped_dependency() {
        let g = pathed_graph(&[0, 1, 2], &[(0, 1), (1, 2), (2, 0), (1, 0)]);
        let keep = |from: ModuleId, to: ModuleId| (from.raw(), to.raw()) != (1, 0);
        let freed = |breaks: &Breaks| breaks.cuts.iter().map(|c| c.freed).collect::<Vec<_>>();

        let all = break_candidates(&g, &find_cycles(&g).remove(0));
        assert_eq!(freed(&all), [3, 1, 1]);
        let kept = break_candidates_where(&g, &find_cycles_where(&g, keep).remove(0), keep);
        assert_eq!(freed(&kept), [3, 3, 3]);
        assert_eq!(kept.closing, 1);
    }

    fn pathed_graph(order: &[u32], edges: &[(u32, u32)]) -> CompoundGraph {
        use std::path::PathBuf;
        use stratum_core::{
            ids::{ContainerId, LayerId},
            purity::Purity,
            types::{Exports, Module},
            visibility::VisibilityScope,
        };

        let mut g = CompoundGraph::empty();
        let id_of = |logical: u32| {
            let at = order.iter().position(|&l| l == logical).unwrap();
            ModuleId::new(u32::try_from(at).unwrap())
        };
        for &logical in order {
            let id = id_of(logical);
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(format!("src/m{logical:03}.ts")),
                    container: ContainerId::new(0),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: Exports::default(),
                    calls: vec![],
                    imports: vec![],
                    suppressions: vec![],
                },
            );
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
        }
        let mut edges = edges.to_vec();
        edges.sort_by_key(|&(a, b)| std::cmp::Reverse(id_of(a).raw() * 1000 + id_of(b).raw()));
        for (a, b) in edges {
            g.deps.add_edge(
                g.node_index[&id_of(a)],
                g.node_index[&id_of(b)],
                EdgeKind::Static,
            );
        }
        g
    }

    fn candidates(g: &CompoundGraph) -> (Vec<(String, String, usize)>, usize) {
        let path = |m: ModuleId| g.modules[&m].path.display().to_string();
        let breaks = break_candidates(g, &find_cycles(g).remove(0));
        let cuts = breaks
            .cuts
            .iter()
            .map(|c| (path(c.from), path(c.to), c.freed))
            .collect();
        (cuts, breaks.closing)
    }

    #[test]
    fn candidates_are_identical_across_node_orders() {
        let n = 30_u32;
        let ring: Vec<(u32, u32)> = (0..n).map(|i| (i, (i + 1) % n)).collect();
        let dense: Vec<(u32, u32)> = ring
            .iter()
            .copied()
            .chain((0..n).map(|i| (i, (i + 7) % n)))
            .chain((0..n).map(|i| (i, (i + 11) % n)))
            .collect();
        assert!(dense.len() > MAX_SCORED_EDGES);

        for edges in [&ring[..], &dense[..]] {
            let forward: Vec<u32> = (0..n).collect();
            let reversed: Vec<u32> = (0..n).rev().collect();
            let shuffled: Vec<u32> = (0..n).map(|i| (i * 7) % n).collect();
            let expected = candidates(&pathed_graph(&forward, edges));
            assert_eq!(expected.0.len(), MAX_CUTS);
            assert_eq!(candidates(&pathed_graph(&reversed, edges)), expected);
            assert_eq!(candidates(&pathed_graph(&shuffled, edges)), expected);
        }
    }

    #[test]
    fn a_cut_frees_only_the_modules_that_leave_every_cycle() {
        let g = pathed_graph(&[0, 1, 2, 3], &[(0, 1), (1, 2), (2, 1), (2, 3), (3, 0)]);
        let freed: Vec<(String, usize)> = candidates(&g)
            .0
            .into_iter()
            .map(|(from, to, freed)| (format!("{from}>{to}"), freed))
            .collect();
        assert_eq!(
            freed,
            [
                ("src/m001.ts>src/m002.ts".to_owned(), 4),
                ("src/m000.ts>src/m001.ts".to_owned(), 2),
                ("src/m002.ts>src/m003.ts".to_owned(), 2),
            ]
            .into_iter()
            .collect::<Vec<_>>()
        );
    }
}
