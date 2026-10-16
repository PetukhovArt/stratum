use std::collections::{BTreeMap, BTreeSet, HashMap};

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::Config;
use stratum_core::ids::ModuleId;
use stratum_core::types::ContainerKind;
use stratum_graph::{CompoundGraph, find_cycles};

use crate::paths::relative;
use crate::pipeline::{self, EngineError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drift {
    pub layer_edges: Vec<LayerEdgeGrowth>,
    pub slice_edges: Vec<SliceEdge>,
    pub cycles: Vec<CycleChange>,
    pub stages: Vec<StageChange>,
    pub fan_in: Vec<FanInGrowth>,
}

impl Drift {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layer_edges.is_empty()
            && self.slice_edges.is_empty()
            && self.cycles.is_empty()
            && self.stages.is_empty()
            && self.fan_in.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerEdgeGrowth {
    pub from: String,
    pub to: String,
    pub base: usize,
    pub head: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceEdge {
    pub from: String,
    pub to: String,
    pub example: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleChange {
    pub members: Vec<String>,
    pub grown_from: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageChange {
    pub container: String,
    pub kind: ContainerKind,
    pub base: Option<u8>,
    pub head: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    Ok(Drift {
        layer_edges: layer_edges(&base, &head, config),
        slice_edges: slice_edges(&base, &head),
        cycles: cycles(&base, &head),
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
        let deps = &self.graph.deps;
        deps.raw_edges()
            .iter()
            .map(move |e| (deps[e.source()], deps[e.target()]))
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

fn cycles(base: &Side, head: &Side) -> Vec<CycleChange> {
    let before = base.cycles();
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
            Some(CycleChange {
                members: members.into_iter().map(str::to_owned).collect(),
                grown_from,
            })
        })
        .collect();
    changes.sort_by(|a, b| a.members.cmp(&b.members));
    changes
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
            }]
        );
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
