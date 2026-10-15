use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rustc_hash::FxHashMap;
use stratum_core::{ids::ContainerId, visibility::VisibilityScope};

use crate::graph::{CompoundGraph, PrivateZone};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GraphIndex {
    pub(crate) subtree_modules: FxHashMap<ContainerId, usize>,
    pub(crate) dirs: FxHashMap<PathBuf, DirEntries>,
    pub(crate) zones: Vec<PrivateZone>,
    pub(crate) zone_containers: Vec<Option<ContainerId>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirEntries {
    pub files: Vec<String>,
    pub dirs: BTreeSet<String>,
}

impl GraphIndex {
    pub(crate) fn of(g: &CompoundGraph) -> Self {
        let mut subtree_modules: FxHashMap<ContainerId, usize> = FxHashMap::default();
        let mut dirs: FxHashMap<PathBuf, DirEntries> = FxHashMap::default();
        for m in g.modules.values() {
            let enclosing = g.containers_of(m.id).skip(1).map(|c| c.id);
            for c in std::iter::once(m.container).chain(enclosing) {
                *subtree_modules.entry(c).or_default() += 1;
            }
            let mut inner = m.path.as_path();
            while let (Some(outer), Some(name)) = (inner.parent(), inner.file_name()) {
                if outer.as_os_str().is_empty() {
                    break;
                }
                let name = name.to_string_lossy().into_owned();
                let entries = dirs.entry(outer.to_path_buf()).or_default();
                if inner == m.path {
                    entries.files.push(name);
                } else if !entries.dirs.insert(name) {
                    break;
                }
                inner = outer;
            }
        }
        let zones = private_zones(g);
        let zone_containers = zones
            .iter()
            .map(|z| g.container_at(&z.path).map(|c| c.id))
            .collect();
        Self {
            subtree_modules,
            dirs,
            zones,
            zone_containers,
        }
    }
}

fn private_zones(g: &CompoundGraph) -> Vec<PrivateZone> {
    let mut zones: BTreeMap<PathBuf, PrivateZone> = BTreeMap::new();
    for m in g.modules.values() {
        let VisibilityScope::Private { root } = &m.visibility else {
            continue;
        };
        let Some(zone) = m
            .path
            .strip_prefix(root)
            .ok()
            .and_then(|rest| rest.components().next())
        else {
            continue;
        };
        let path = root.join(zone);
        zones
            .entry(path.clone())
            .or_insert_with(|| PrivateZone {
                path,
                root: root.clone(),
                files: vec![],
            })
            .files
            .push(m.id);
    }
    zones.into_values().collect()
}
