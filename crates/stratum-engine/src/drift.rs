use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use stratum_config::Config;
use stratum_core::edge::EdgeKind;
use stratum_core::ids::ModuleId;
use stratum_core::severity::Severity;
use stratum_core::types::ContainerKind;
use stratum_graph::{CompoundGraph, find_cycles};
use stratum_rules::RuleRegistry;
use stratum_rules::territory::Territories;

use crate::paths::relative;
use crate::pipeline::{self, EngineError};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Drift {
    pub edges: Vec<EdgeChange>,
    pub layer_edges: Vec<LayerEdgeGrowth>,
    pub slice_edges: Vec<SliceEdge>,
    pub cycles: Vec<CycleChange>,
    pub stages: Vec<StageChange>,
    pub fan_in: Vec<FanInGrowth>,
}

impl Drift {
    /// An edge change counts only when it is added and breaks a rule: a rename alone
    /// removes and adds edges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.breaking_imports().is_empty()
            && self.layer_edges.is_empty()
            && self.slice_edges.is_empty()
            && self.cycles.is_empty()
            && self.stages.is_empty()
            && self.fan_in.is_empty()
    }

    /// Added edges with a non-empty `breaks`, the ones breaking an error rule first. An added
    /// edge that shares an end and its `breaks` with a removed one is a moved file, not listed.
    #[must_use]
    pub fn breaking_imports(&self) -> Vec<&EdgeChange> {
        let removed: HashSet<(&str, &[(String, Severity)])> = self
            .edges
            .iter()
            .filter(|e| e.change == Change::Removed && !e.breaks.is_empty())
            .flat_map(|e| {
                [
                    (e.from.as_str(), &e.breaks[..]),
                    (e.to.as_str(), &e.breaks[..]),
                ]
            })
            .collect();
        let moved = |e: &EdgeChange| {
            removed.contains(&(e.from.as_str(), &e.breaks[..]))
                || removed.contains(&(e.to.as_str(), &e.breaks[..]))
        };
        let mut added: Vec<&EdgeChange> = self
            .edges
            .iter()
            .filter(|e| e.change == Change::Added && !e.breaks.is_empty() && !moved(e))
            .collect();
        added.sort_by_key(|e| std::cmp::Reverse(e.breaks[0].1));
        added
    }
}

/// `breaks` holds the rules the edge breaks on the side where it exists (the head for an
/// added edge, the base for a removed one), worst severity first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EdgeChange {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub change: Change,
    pub breaks: Vec<(String, Severity)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayerEdgeGrowth {
    pub from: String,
    pub to: String,
    pub base: usize,
    pub head: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SliceEdge {
    pub from: String,
    pub to: String,
    pub example: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CycleChange {
    pub members: Vec<String>,
    pub grown_from: Option<usize>,
    /// Head cycle through the first added edge inside `members`, closed: `[from, to, .., from]`.
    /// Empty when no added edge is inside.
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StageChange {
    pub container: String,
    pub kind: ContainerKind,
    pub base: Option<u8>,
    pub head: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FanInGrowth {
    pub slice: String,
    pub base: usize,
    pub head: usize,
    pub added: Vec<String>,
}

/// Architecture changes from a base to the tree at `root`, both built with `config`.
/// The base is `root` with every file in `base` replaced by its base text, `None` for a file
/// absent at the base; paths are relative to `root` with `/`. Every list is sorted.
pub fn drift(
    root: &Utf8Path,
    base: Vec<(String, Option<String>)>,
    config: &Config,
) -> Result<Drift, EngineError> {
    let (base, head) = std::thread::scope(|s| {
        let base = s.spawn(|| Side::build(root, config, base));
        let head = Side::build(root, config, Vec::new());
        (base.join(), head)
    });
    let base = base.unwrap_or_else(|p| std::panic::resume_unwind(p))?;
    let head = head?;
    let registry = pipeline::registry(&head.root, config)?;
    let edges = edges(&base, &head, config, &registry)?;
    Ok(Drift {
        cycles: cycles(&base, &head, &edges),
        edges,
        layer_edges: layer_edges(&base, &head, config),
        slice_edges: slice_edges(&base, &head),
        stages: stages(&base, &head),
        fan_in: fan_in(&base, &head),
    })
}

struct Side {
    graph: CompoundGraph,
    root: Utf8PathBuf,
    paths: HashMap<ModuleId, String>,
    slices: HashMap<ModuleId, String>,
}

impl Side {
    fn build(
        root: &Utf8Path,
        config: &Config,
        overlay: Vec<(String, Option<String>)>,
    ) -> Result<Self, EngineError> {
        let (root, graph, _) = pipeline::graph(root, config, overlay)?;
        let paths = graph
            .modules
            .values()
            .map(|m| (m.id, relative(&m.path, &root)))
            .collect();
        let slices = graph
            .modules
            .keys()
            .filter_map(|&m| {
                let c = graph.outer_container(m)?;
                (c.kind == ContainerKind::Slice).then(|| (m, relative(&c.path, &root)))
            })
            .collect();
        Ok(Self {
            graph,
            root,
            paths,
            slices,
        })
    }

    fn edges(&self) -> impl Iterator<Item = (ModuleId, ModuleId)> + '_ {
        self.kinded_edges().map(|(a, b, _)| (a, b))
    }

    fn kinded_edges(&self) -> impl Iterator<Item = (ModuleId, ModuleId, EdgeKind)> + '_ {
        let deps = &self.graph.deps;
        deps.raw_edges()
            .iter()
            .map(move |e| (deps[e.source()], deps[e.target()], e.weight))
    }

    fn edge_keys(&self) -> BTreeMap<(&str, &str, u8), (ModuleId, ModuleId, EdgeKind)> {
        self.kinded_edges()
            .map(|(a, b, kind)| {
                let key = (self.paths[&a].as_str(), self.paths[&b].as_str(), kind as u8);
                (key, (a, b, kind))
            })
            .collect()
    }

    fn layer_of(&self, m: ModuleId) -> Option<&str> {
        let layer = self.graph.modules.get(&m)?.layer;
        self.graph.layers.get(&layer).map(|l| l.name.as_str())
    }

    fn layer_edge_counts(&self) -> BTreeMap<(&str, &str), usize> {
        let mut counts = BTreeMap::new();
        for (a, b) in self.edges() {
            if let (Some(from), Some(to)) = (self.layer_of(a), self.layer_of(b)) {
                if from != to {
                    *counts.entry((from, to)).or_default() += 1;
                }
            }
        }
        counts
    }

    fn slice_pairs(&self) -> BTreeMap<(&str, &str), String> {
        let mut pairs: BTreeMap<(&str, &str), String> = BTreeMap::new();
        for (a, b) in self.edges() {
            let (Some(from), Some(to)) = (self.slices.get(&a), self.slices.get(&b)) else {
                continue;
            };
            if from == to {
                continue;
            }
            let example = format!("{} -> {}", self.paths[&a], self.paths[&b]);
            pairs
                .entry((from, to))
                .and_modify(|first| {
                    if example < *first {
                        first.clone_from(&example);
                    }
                })
                .or_insert(example);
        }
        pairs
    }

    fn cycles(&self) -> Vec<BTreeSet<&str>> {
        find_cycles(&self.graph)
            .into_iter()
            .map(|c| c.modules.iter().map(|m| self.paths[m].as_str()).collect())
            .collect()
    }

    fn containers(&self) -> BTreeMap<String, (ContainerKind, u8)> {
        self.graph
            .containers
            .values()
            .map(|c| (relative(&c.path, &self.root), (c.kind, c.stage.rank())))
            .collect()
    }

    fn slice_importers(&self) -> BTreeMap<String, BTreeSet<String>> {
        self.graph
            .containers
            .values()
            .filter(|c| c.kind == ContainerKind::Slice)
            .map(|c| {
                let importers = self
                    .graph
                    .importing_slices(c.id)
                    .iter()
                    .filter_map(|i| self.graph.containers.get(i))
                    .map(|i| relative(&i.path, &self.root))
                    .collect();
                (relative(&c.path, &self.root), importers)
            })
            .collect()
    }
}

fn edges(
    base: &Side,
    head: &Side,
    config: &Config,
    registry: &RuleRegistry,
) -> Result<Vec<EdgeChange>, EngineError> {
    let before = base.edge_keys();
    let after = head.edge_keys();
    let mut changes = Vec::new();
    for (side, mine, theirs, change) in [
        (head, &after, &before, Change::Added),
        (base, &before, &after, Change::Removed),
    ] {
        let only: Vec<_> = mine
            .iter()
            .filter(|(key, _)| !theirs.contains_key(*key))
            .collect();
        if only.is_empty() {
            continue;
        }
        let territories = Territories::new(&side.graph, config, &side.root, registry)?;
        changes.extend(only.into_iter().map(|(&(from, to, _), &(a, b, kind))| {
            let mut breaks: Vec<(String, Severity)> = territories
                .verdict(a, b)
                .into_iter()
                .map(|breach| {
                    let slug = registry.by_id(breach.rule).map_or("unknown", |r| r.slug());
                    (slug.to_owned(), breach.severity)
                })
                .collect();
            breaks.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
            EdgeChange {
                from: from.to_owned(),
                to: to.to_owned(),
                kind,
                change,
                breaks,
            }
        }));
    }
    changes.sort_by(|x, y| {
        let key = |e: &EdgeChange| (e.from.clone(), e.to.clone(), e.kind as u8, e.change as u8);
        key(x).cmp(&key(y))
    });
    Ok(changes)
}

fn layer_edges(base: &Side, head: &Side, config: &Config) -> Vec<LayerEdgeGrowth> {
    let before = base.layer_edge_counts();
    let declared = |from: &str, to: &str| {
        config
            .layers
            .iter()
            .any(|l| l.id == from && l.depends_on.iter().any(|d| d == to))
    };
    head.layer_edge_counts()
        .into_iter()
        .filter(|((from, to), _)| !declared(from, to))
        .filter_map(|((from, to), head)| {
            let base = before.get(&(from, to)).copied().unwrap_or(0);
            (head > base).then(|| LayerEdgeGrowth {
                from: from.to_owned(),
                to: to.to_owned(),
                base,
                head,
            })
        })
        .collect()
}

fn slice_edges(base: &Side, head: &Side) -> Vec<SliceEdge> {
    let before = base.slice_pairs();
    head.slice_pairs()
        .into_iter()
        .filter(|(pair, _)| !before.contains_key(pair))
        .map(|((from, to), example)| SliceEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            example,
        })
        .collect()
}

fn cycles(base: &Side, head: &Side, edges: &[EdgeChange]) -> Vec<CycleChange> {
    let before = base.cycles();
    let mut imports: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    let mut changes: Vec<CycleChange> = head
        .cycles()
        .into_iter()
        .filter_map(|members| {
            let overlapping = before
                .iter()
                .filter(|b| !b.is_disjoint(&members))
                .map(BTreeSet::len)
                .max();
            let grown_from = match overlapping {
                None => None,
                Some(size) if members.len() > size => Some(size),
                Some(_) => return None,
            };
            if imports.is_empty() {
                for (a, b) in head.edges() {
                    imports
                        .entry(head.paths[&a].as_str())
                        .or_default()
                        .insert(head.paths[&b].as_str());
                }
            }
            Some(CycleChange {
                path: cycle_path(&members, &imports, edges),
                members: members.into_iter().map(str::to_owned).collect(),
                grown_from,
            })
        })
        .collect();
    changes.sort_by(|a, b| a.members.cmp(&b.members));
    changes
}

fn cycle_path(
    members: &BTreeSet<&str>,
    imports: &HashMap<&str, BTreeSet<&str>>,
    edges: &[EdgeChange],
) -> Vec<String> {
    let Some(added) = edges.iter().find(|e| {
        e.change == Change::Added
            && members.contains(e.from.as_str())
            && members.contains(e.to.as_str())
    }) else {
        return Vec::new();
    };
    let (from, to) = (added.from.as_str(), added.to.as_str());
    let mut reached_from: HashMap<&str, &str> = HashMap::new();
    let mut queue = VecDeque::from([to]);
    while let Some(at) = queue.pop_front() {
        if at == from {
            let mut back = vec![from];
            while back[back.len() - 1] != to {
                back.push(reached_from[back[back.len() - 1]]);
            }
            back.push(from);
            return back.into_iter().rev().map(str::to_owned).collect();
        }
        for &next in imports.get(at).into_iter().flatten() {
            if members.contains(next) && next != to && !reached_from.contains_key(next) {
                reached_from.insert(next, at);
                queue.push_back(next);
            }
        }
    }
    Vec::new()
}

fn stages(base: &Side, head: &Side) -> Vec<StageChange> {
    let before = base.containers();
    head.containers()
        .into_iter()
        .filter_map(|(container, (kind, head))| {
            let base = before.get(&container).map(|&(_, stage)| stage);
            (base != Some(head)).then_some(StageChange {
                container,
                kind,
                base,
                head,
            })
        })
        .collect()
}

fn fan_in(base: &Side, head: &Side) -> Vec<FanInGrowth> {
    let before = base.slice_importers();
    let none = BTreeSet::new();
    head.slice_importers()
        .into_iter()
        .filter_map(|(slice, importers)| {
            let was = before.get(&slice).unwrap_or(&none);
            (importers.len() > was.len()).then(|| FanInGrowth {
                base: was.len(),
                head: importers.len(),
                added: importers.difference(was).cloned().collect(),
                slice,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct Trees {
        _dir: tempfile::TempDir,
        head: Utf8PathBuf,
        config: Config,
        base: BTreeMap<String, Option<String>>,
    }

    fn exemplar_twice() -> Trees {
        let fixture = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/exemplar-stratum-app");
        let dir = tempfile::tempdir().unwrap();
        let head = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        for entry in walkdir::WalkDir::new(&fixture) {
            let entry = entry.unwrap();
            let rel = entry.path().strip_prefix(&fixture).unwrap();
            let to = head.as_std_path().join(rel);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&to).unwrap();
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
        let config = stratum_config::parse_file(&head.join("stratum.config.jsonc")).unwrap();
        Trees {
            _dir: dir,
            head,
            config,
            base: BTreeMap::new(),
        }
    }

    impl Trees {
        fn keep_base(&mut self, rel: &str) {
            let old = std::fs::read_to_string(self.head.join(rel)).ok();
            self.base.entry(rel.to_owned()).or_insert(old);
        }

        fn write(&mut self, rel: &str, text: &str) {
            self.keep_base(rel);
            let path = self.head.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }

        fn append(&mut self, rel: &str, text: &str) {
            let old = std::fs::read_to_string(self.head.join(rel)).unwrap();
            self.write(rel, &format!("{old}{text}"));
        }

        fn delete(&mut self, rel: &str) {
            self.keep_base(rel);
            std::fs::remove_file(self.head.join(rel)).unwrap();
        }

        fn drift(&self) -> Drift {
            let base = self.base.clone().into_iter().collect();
            drift(&self.head, base, &self.config).unwrap()
        }
    }

    #[test]
    fn unchanged_tree_has_no_drift() {
        assert_eq!(exemplar_twice().drift(), Drift::default());
    }

    #[test]
    fn new_cross_slice_import_is_reported_with_its_example() {
        let mut trees = exemplar_twice();
        trees.append(
            "src/features/checkout/index.ts",
            "import \"@/features/cart\";\n",
        );

        let drift = trees.drift();

        assert_eq!(
            drift.slice_edges,
            [SliceEdge {
                from: "src/features/checkout".into(),
                to: "src/features/cart".into(),
                example: "src/features/checkout/index.ts -> src/features/cart/index.ts".into(),
            }]
        );
        assert_eq!(drift.fan_in[0].added, ["src/features/checkout"]);
    }

    #[test]
    fn new_cycle_is_reported_and_the_old_one_is_not() {
        let mut trees = exemplar_twice();
        trees.append("src/shared/utils.ts", "import \"./api/http\";\n");

        assert_eq!(
            trees.drift().cycles,
            [CycleChange {
                members: vec![
                    "src/entities/user/model.ts".into(),
                    "src/shared/api/http.ts".into(),
                    "src/shared/utils.ts".into(),
                ],
                grown_from: None,
                path: vec![
                    "src/shared/utils.ts".into(),
                    "src/shared/api/http.ts".into(),
                    "src/entities/user/model.ts".into(),
                    "src/shared/utils.ts".into(),
                ],
            }]
        );
    }

    #[test]
    fn added_import_of_slice_internals_breaks_public_api_at_the_head() {
        let mut trees = exemplar_twice();
        trees.append(
            "src/features/checkout/index.ts",
            "import \"@/entities/user/model\";\n",
        );

        assert_eq!(
            trees.drift().edges,
            [EdgeChange {
                from: "src/features/checkout/index.ts".into(),
                to: "src/entities/user/model.ts".into(),
                kind: EdgeKind::Static,
                change: Change::Added,
                breaks: vec![("stratum/public-api".into(), Severity::Error)],
            }]
        );
    }

    #[test]
    fn removed_import_carries_the_rules_it_broke_at_the_base() {
        let mut trees = exemplar_twice();
        trees.write("src/shared/api/http.ts", "export const http = 1;\n");

        let drift = trees.drift();

        let removed = &drift.edges[0];
        assert_eq!(
            (removed.from.as_str(), removed.to.as_str(), removed.change),
            (
                "src/shared/api/http.ts",
                "src/entities/user/model.ts",
                Change::Removed
            ),
            "{:?}",
            drift.edges
        );
        assert!(
            removed
                .breaks
                .contains(&("stratum/public-api".into(), Severity::Error)),
            "{:?}",
            removed.breaks
        );
        assert!(drift.is_empty(), "a removed import alone is no drift");
    }

    #[test]
    fn new_sub_unit_with_an_entry_is_a_stage_change() {
        let mut trees = exemplar_twice();
        trees.write(
            "src/features/profile/settings/index.ts",
            "export const s = 1;\n",
        );

        let stages = trees.drift().stages;

        assert!(
            stages
                .iter()
                .any(|s| s.container == "src/features/profile/settings"
                    && s.kind == ContainerKind::SubUnit
                    && s.base.is_none()),
            "{stages:?}"
        );
    }

    #[test]
    fn a_file_deleted_in_head_still_imports_at_the_base() {
        let mut trees = exemplar_twice();
        trees.delete("src/features/profile/index.ts");
        trees.append(
            "src/features/cart/index.ts",
            "import \"@/features/checkout\";\n",
        );

        let drift = trees.drift();

        assert_eq!(drift.fan_in, [], "checkout had profile as its importer");
        assert_eq!(drift.slice_edges.len(), 1, "{:?}", drift.slice_edges);
    }
}
