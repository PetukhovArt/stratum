use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::index::DirRole;
use stratum_core::{
    ids::{ContainerId, LayerId, ModuleId},
    stage::Stage,
    structure::Structure,
    types::{Container, ContainerKind, Layer},
    visibility::VisibilityScope,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub file: usize,
    pub layer: LayerId,
    pub container: ContainerId,
    pub visibility: VisibilityScope,
}

#[derive(Debug, Clone, Default)]
pub struct Placement {
    pub containers: IndexMap<ContainerId, Container>,
    pub modules: Vec<Placed>,
    pub dir_roles: BTreeMap<PathBuf, DirRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Key {
    Layer(usize),
    Path(Utf8PathBuf),
}

#[derive(Debug)]
enum RoleDraft {
    Container(Key),
    Segment,
    PrivateZone,
    Collection,
}

#[derive(Debug)]
struct Draft {
    kind: ContainerKind,
    layer: usize,
    parent: Option<Key>,
    entry: Option<usize>,
    is_file: bool,
    segment_dirs: BTreeSet<String>,
    has_sub_unit: bool,
}

impl Draft {
    const fn new(
        kind: ContainerKind,
        layer: usize,
        parent: Option<Key>,
        entry: Option<usize>,
    ) -> Self {
        Self {
            kind,
            layer,
            parent,
            entry,
            is_file: false,
            segment_dirs: BTreeSet::new(),
            has_sub_unit: false,
        }
    }

    fn stage(&self) -> Stage {
        if self.is_file {
            Stage::FILE
        } else if self.has_sub_unit {
            Stage::COMPOSED
        } else if !self.segment_dirs.is_empty() {
            Stage::SEGMENTED
        } else {
            Stage::FLAT
        }
    }
}

struct Entry {
    module: usize,
    layer: usize,
    rank: usize,
}

/// Places `files` (absolute, sorted) into layers and containers. A file outside every layer is
/// dropped; the rest become `modules` in input order, and a module's `ModuleId` is its index there.
#[must_use]
pub fn place(
    root: &Utf8Path,
    layers: &[Layer],
    structure: &Structure,
    files: &[Utf8PathBuf],
) -> Placement {
    let roots: Vec<Utf8PathBuf> = layers.iter().map(|l| layer_root(root, l)).collect();
    let root_components: Vec<Vec<Utf8Component>> =
        roots.iter().map(|r| r.components().collect()).collect();
    let assigned: Vec<(usize, usize)> = files
        .iter()
        .enumerate()
        .filter_map(|(file, path)| Some((file, layer_of(&root_components, path)?)))
        .collect();
    let mut walk = Walk {
        layers,
        structure,
        entries: entry_dirs(structure, files, &assigned),
        drafts: FxHashMap::default(),
        roles: FxHashMap::default(),
        roots,
    };
    for i in 0..layers.len() {
        let entry = walk.entry_in(&walk.roots[i], i);
        walk.drafts.insert(
            Key::Layer(i),
            Draft::new(ContainerKind::Layer, i, None, entry),
        );
    }
    let homes: Vec<(Key, VisibilityScope)> = assigned
        .iter()
        .enumerate()
        .map(|(module, &(file, layer))| walk.home_of(module, &files[file], layer))
        .collect();
    let (containers, ids, dir_roles) = walk.containers();

    let modules = assigned
        .iter()
        .zip(homes)
        .map(|(&(file, layer), (home, visibility))| Placed {
            file,
            layer: layers[layer].id,
            container: ids[&home],
            visibility,
        })
        .collect();

    Placement {
        containers,
        modules,
        dir_roles,
    }
}

struct Walk<'a> {
    layers: &'a [Layer],
    structure: &'a Structure,
    roots: Vec<Utf8PathBuf>,
    entries: FxHashMap<&'a Utf8Path, Entry>,
    drafts: FxHashMap<Key, Draft>,
    roles: FxHashMap<Utf8PathBuf, RoleDraft>,
}

impl Walk<'_> {
    fn entry_in(&self, dir: &Utf8Path, layer: usize) -> Option<usize> {
        self.entries
            .get(dir)
            .filter(|e| e.layer == layer)
            .map(|e| e.module)
    }

    fn home_of(&mut self, module: usize, path: &Utf8Path, layer: usize) -> (Key, VisibilityScope) {
        let names: Vec<&str> = path
            .strip_prefix(&self.roots[layer])
            .map(|inner| inner.iter().collect())
            .unwrap_or_default();
        let dirs = names.split_last().map_or(&[][..], |(_, dirs)| dirs);
        let sliced = self.layers[layer].slices;
        let mut home = Key::Layer(layer);
        let mut home_kind = ContainerKind::Layer;
        let mut loose = false;
        let mut private_root: Option<Utf8PathBuf> = None;
        let mut dir = self.roots[layer].clone();
        for (depth, name) in dirs.iter().enumerate() {
            let parent_dir = dir.clone();
            dir.push(name);
            if self.structure.is_private(name) {
                self.roles
                    .entry(dir.clone())
                    .or_insert(RoleDraft::PrivateZone);
                private_root = Some(parent_dir);
                loose = true;
                continue;
            }
            if self.structure.is_segment(name) {
                self.roles.entry(dir.clone()).or_insert(RoleDraft::Segment);
                if !loose {
                    self.mark(&home, |d| {
                        d.segment_dirs.insert((*name).to_owned());
                    });
                }
                loose = true;
                continue;
            }
            let entry = self.entry_in(&dir, layer);
            let kind = if depth == 0 && sliced {
                ContainerKind::Slice
            } else if entry.is_none() {
                self.roles
                    .entry(dir.clone())
                    .or_insert(RoleDraft::Collection);
                loose = true;
                continue;
            } else if loose || home_kind == ContainerKind::Layer {
                ContainerKind::Nested
            } else {
                ContainerKind::SubUnit
            };
            if kind == ContainerKind::SubUnit {
                self.mark(&home, |d| d.has_sub_unit = true);
            }
            let key = Key::Path(dir.clone());
            self.roles
                .entry(dir.clone())
                .or_insert_with(|| RoleDraft::Container(key.clone()));
            self.drafts
                .entry(key.clone())
                .or_insert_with(|| Draft::new(kind, layer, Some(home.clone()), entry));
            home = key;
            home_kind = kind;
            loose = false;
        }
        let layer_entry = names.len() == 1 && self.structure.entry_rank(names[0]).is_some();
        if sliced && names.len() == 1 && !layer_entry {
            let key = Key::Path(path.to_path_buf());
            let mut slice = Draft::new(ContainerKind::Slice, layer, Some(home), Some(module));
            slice.is_file = true;
            self.drafts.insert(key.clone(), slice);
            home = key;
        }
        let visibility =
            private_root.map_or(VisibilityScope::Public, |root| VisibilityScope::Private {
                root: PathBuf::from(root.into_string()),
            });
        (home, visibility)
    }

    fn mark(&mut self, key: &Key, set: impl FnOnce(&mut Draft)) {
        if let Some(d) = self.drafts.get_mut(key) {
            set(d);
        }
    }

    fn containers(
        self,
    ) -> (
        IndexMap<ContainerId, Container>,
        FxHashMap<Key, ContainerId>,
        BTreeMap<PathBuf, DirRole>,
    ) {
        let layers = self.layers;
        let mut next = layers.iter().map(|l| l.id.raw() + 1).max().unwrap_or(0);
        let mut drafts: Vec<(Key, Draft)> = self.drafts.into_iter().collect();
        drafts.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        let ids: FxHashMap<Key, ContainerId> = drafts
            .iter()
            .map(|(key, _)| {
                let id = match key {
                    Key::Layer(i) => ContainerId::new(layers[*i].id.raw()),
                    Key::Path(_) => {
                        next += 1;
                        ContainerId::new(next - 1)
                    }
                };
                (key.clone(), id)
            })
            .collect();
        let containers = drafts
            .into_iter()
            .map(|(key, d)| {
                let id = ids[&key];
                let (name, path) = match key {
                    Key::Layer(i) => (layers[i].name.clone(), self.roots[i].clone()),
                    Key::Path(p) => (p.file_name().unwrap_or_default().to_owned(), p),
                };
                let container = Container {
                    id,
                    name,
                    path: PathBuf::from(path.into_string()),
                    kind: d.kind,
                    layer: layers[d.layer].id,
                    parent: d.parent.as_ref().map(|p| ids[p]),
                    stage: d.stage(),
                    entry: d.entry.map(module_id),
                    segment_dirs: d.segment_dirs.into_iter().collect(),
                };
                (id, container)
            })
            .collect();
        let mut dir_roles: BTreeMap<PathBuf, DirRole> = self
            .roles
            .into_iter()
            .map(|(dir, role)| {
                let role = match role {
                    RoleDraft::Container(key) => DirRole::Container(ids[&key]),
                    RoleDraft::Segment => DirRole::Segment {
                        entry: self.entries.get(dir.as_path()).map(|e| module_id(e.module)),
                    },
                    RoleDraft::PrivateZone => DirRole::PrivateZone,
                    RoleDraft::Collection => DirRole::Collection,
                };
                (PathBuf::from(dir.into_string()), role)
            })
            .collect();
        for root in &self.roots {
            dir_roles.insert(PathBuf::from(root.as_str()), DirRole::LayerRoot);
        }
        (containers, ids, dir_roles)
    }
}

fn module_id(index: usize) -> ModuleId {
    ModuleId::new(u32::try_from(index).unwrap_or(u32::MAX))
}

fn layer_root(root: &Utf8Path, layer: &Layer) -> Utf8PathBuf {
    let mut out = root.to_path_buf();
    let rel = Utf8Path::new(layer.path.to_str().unwrap_or_default());
    for c in rel.components() {
        if let Utf8Component::Normal(name) = c {
            out.push(name);
        }
    }
    out
}

fn layer_of(roots: &[Vec<Utf8Component>], path: &Utf8Path) -> Option<usize> {
    let path: Vec<Utf8Component> = path.components().collect();
    let mut best: Option<(usize, usize)> = None;
    for (i, r) in roots.iter().enumerate() {
        if path.starts_with(r) {
            let depth = r.len();
            if best.is_none_or(|(d, _)| depth > d) {
                best = Some((depth, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

fn entry_dirs<'f>(
    structure: &Structure,
    files: &'f [Utf8PathBuf],
    assigned: &[(usize, usize)],
) -> FxHashMap<&'f Utf8Path, Entry> {
    let mut out: FxHashMap<&Utf8Path, Entry> = FxHashMap::default();
    for (module, &(file, layer)) in assigned.iter().enumerate() {
        let path = &files[file];
        let (Some(dir), Some(rank)) = (
            path.parent(),
            path.file_name().and_then(|n| structure.entry_rank(n)),
        ) else {
            continue;
        };
        if out.get(dir).is_none_or(|e| rank < e.rank) {
            out.insert(
                dir,
                Entry {
                    module,
                    layer,
                    rank,
                },
            );
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const ROOT: &str = "/p";

    fn abs(rel: &str) -> Utf8PathBuf {
        Utf8Path::new(ROOT).join(rel)
    }

    struct Fixture {
        placement: Placement,
        files: Vec<Utf8PathBuf>,
    }

    impl Fixture {
        fn new() -> Self {
            let layer = |id: u32, path: &str, slices: bool| Layer {
                id: LayerId::new(id),
                name: path.rsplit('/').next().unwrap().into(),
                path: PathBuf::from(path),
                depends_on: vec![],
                slices,
            };
            let layers = [
                layer(0, "src/entities", true),
                layer(1, "src/shared", false),
            ];
            let structure = Structure {
                segments: vec!["model".into(), "ui".into()],
                private: vec!["_shared".into()],
                ..Structure::default()
            };
            let mut files: Vec<Utf8PathBuf> = [
                "src/entities/index.ts",
                "src/entities/order.ts",
                "src/entities/flat/a.ts",
                "src/entities/user/index.ts",
                "src/entities/user/model/store.ts",
                "src/entities/user/_shared/z.ts",
                "src/entities/user/_shared/inner/index.ts",
                "src/entities/user/_shared/deep/_shared/q.ts",
                "src/entities/cart/index.ts",
                "src/entities/cart/model/s.ts",
                "src/entities/cart/model/deep/index.ts",
                "src/entities/cart/parts/index.ts",
                "src/entities/cart/parts/a.ts",
                "src/entities/cart/helpers/x/index.ts",
                "src/entities/_shared/k.ts",
                "src/shared/kit/index.ts",
                "src/shared/kit/inner/index.ts",
                "src/shared/util.ts",
                "outside/x.ts",
            ]
            .into_iter()
            .map(abs)
            .collect();
            files.sort();
            let placement = place(Utf8Path::new(ROOT), &layers, &structure, &files);
            Self { placement, files }
        }

        fn at(&self, rel: &str) -> &Container {
            let path = PathBuf::from(abs(rel).into_string());
            self.placement
                .containers
                .values()
                .find(|c| c.path == path)
                .unwrap()
        }

        fn module(&self, rel: &str) -> &Placed {
            self.placement
                .modules
                .iter()
                .find(|m| self.files[m.file] == abs(rel))
                .unwrap()
        }

        fn home(&self, rel: &str) -> &Container {
            &self.placement.containers[&self.module(rel).container]
        }

        fn stage(&self, rel: &str) -> u8 {
            self.at(rel).stage.rank()
        }
    }

    #[test]
    fn kinds_follow_slices_entries_segments_and_private_zones() {
        use ContainerKind::{Layer, Nested, Slice, SubUnit};
        let f = Fixture::new();
        assert_eq!(f.at("src/entities/order.ts").kind, Slice);
        assert_eq!(f.at("src/entities/flat").kind, Slice);
        assert_eq!(f.at("src/entities/cart/parts").kind, SubUnit);
        assert_eq!(f.at("src/entities/cart/helpers/x").kind, Nested);
        assert_eq!(f.at("src/entities/cart/model/deep").kind, Nested);
        assert_eq!(f.at("src/entities/user/_shared/inner").kind, Nested);
        assert_eq!(f.at("src/shared/kit").kind, Nested);
        assert_eq!(f.at("src/shared/kit/inner").kind, SubUnit);
        assert_eq!(f.home("src/entities/_shared/k.ts").kind, Layer);
        assert_eq!(f.home("src/entities/cart/model/s.ts").name, "cart");
        assert_eq!(f.home("src/entities/order.ts").name, "order.ts");
        assert!(
            f.placement
                .containers
                .values()
                .all(|c| !c.path.ends_with("model") && !c.path.ends_with("_shared"))
        );
        assert_eq!(f.placement.modules.len(), f.files.len() - 1);

        let chain: Vec<&str> = std::iter::successors(Some(f.at("src/entities/cart/parts")), |c| {
            c.parent.map(|id| &f.placement.containers[&id])
        })
        .map(|c| c.name.as_str())
        .collect();
        assert_eq!(chain, ["parts", "cart", "entities"]);

        let entry = f.at("src/entities/user").entry.unwrap();
        let entry_module = &f.placement.modules[usize::try_from(entry.raw()).unwrap()];
        assert_eq!(
            f.files[entry_module.file],
            abs("src/entities/user/index.ts")
        );
        assert_eq!(f.at("src/entities/flat").entry, None);
    }

    #[test]
    fn entry_file_in_a_sliced_layer_root_is_the_layer_entry_not_a_slice() {
        let f = Fixture::new();
        let home = f.home("src/entities/index.ts");
        assert_eq!(home.kind, ContainerKind::Layer);
        let entry = f.at("src/entities").entry.unwrap();
        let entry_module = &f.placement.modules[usize::try_from(entry.raw()).unwrap()];
        assert_eq!(f.files[entry_module.file], abs("src/entities/index.ts"));
        assert!(
            f.placement
                .containers
                .values()
                .all(|c| c.path.as_path() != abs("src/entities/index.ts").as_std_path())
        );
    }

    #[test]
    fn innermost_private_zone_sets_the_visibility_root() {
        let f = Fixture::new();
        let root_of = |rel: &str| match &f.module(rel).visibility {
            VisibilityScope::Private { root } => Some(root.clone()),
            VisibilityScope::Public => None,
        };
        let dir = |rel: &str| Some(PathBuf::from(abs(rel).into_string()));
        assert_eq!(
            root_of("src/entities/user/_shared/z.ts"),
            dir("src/entities/user")
        );
        assert_eq!(
            root_of("src/entities/user/_shared/inner/index.ts"),
            dir("src/entities/user")
        );
        assert_eq!(
            root_of("src/entities/user/_shared/deep/_shared/q.ts"),
            dir("src/entities/user/_shared/deep")
        );
        assert_eq!(root_of("src/entities/_shared/k.ts"), dir("src/entities"));
        assert_eq!(root_of("src/entities/user/index.ts"), None);
    }

    #[test]
    fn every_folder_inside_a_layer_gets_one_role_from_structure_and_entries() {
        let f = Fixture::new();
        let role = |rel: &str| f.placement.dir_roles[&PathBuf::from(abs(rel).into_string())];
        let container = |rel: &str| DirRole::Container(f.at(rel).id);
        assert_eq!(role("src/entities"), DirRole::LayerRoot);
        assert_eq!(role("src/shared"), DirRole::LayerRoot);
        assert_eq!(role("src/entities/user"), container("src/entities/user"));
        assert_eq!(
            role("src/entities/cart/parts"),
            container("src/entities/cart/parts")
        );
        assert_eq!(
            role("src/entities/user/model"),
            DirRole::Segment { entry: None }
        );
        assert_eq!(role("src/entities/user/_shared"), DirRole::PrivateZone);
        assert_eq!(
            role("src/entities/user/_shared/deep/_shared"),
            DirRole::PrivateZone
        );
        assert_eq!(role("src/entities/user/_shared/deep"), DirRole::Collection);
        assert_eq!(role("src/entities/cart/helpers"), DirRole::Collection);
        assert_eq!(
            role("src/entities/cart/model/deep"),
            container("src/entities/cart/model/deep")
        );
    }

    #[test]
    fn stage_follows_the_direct_children() {
        let f = Fixture::new();
        assert_eq!(f.stage("src/entities/order.ts"), 1);
        assert_eq!(f.stage("src/entities/flat"), 2);
        assert_eq!(f.stage("src/entities/user"), 3);
        assert_eq!(f.stage("src/entities/cart"), 4);
        assert_eq!(f.stage("src/shared/kit"), 4);
        assert_eq!(f.stage("src/shared"), 2);
    }

    #[test]
    fn segment_dirs_are_the_direct_segment_children() {
        let f = Fixture::new();
        assert_eq!(f.at("src/entities/cart").segment_dirs, ["model"]);
        assert_eq!(f.at("src/entities/user").segment_dirs, ["model"]);
        assert!(f.at("src/entities/cart/parts").segment_dirs.is_empty());
        assert!(f.at("src/entities").segment_dirs.is_empty());
    }
}
