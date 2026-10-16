use std::collections::HashMap;
use std::path::{Path, PathBuf};

use camino::Utf8Path;

use stratum_config::{Config, ConfigError};
use stratum_core::{
    ids::{ContainerId, LayerId, ModuleId, RuleId},
    severity::Severity,
    stage::Stage,
    types::{Container, ContainerKind, Module},
};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::BoundJudge;
use crate::builtin::no_cross_layer_import::may_import;
use crate::builtin::sibling_isolation::is_isolated;
use crate::ids::{NO_CROSS_LAYER_IMPORT, PUBLIC_API, PURITY, SIBLING_ISOLATION, VISIBILITY_SCOPE};
use crate::registry::{DynRule, RuleRegistry};
use crate::severity::{EffectiveSeverities, slashed, under};

const PLANNED_PURITY: u8 = 2;
const IMPUREST: u8 = 4;

/// Index of a zone in [`CompoundGraph::private_zones`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZoneId(u32);

impl ZoneId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    fn of(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Site<'p> {
    Module(ModuleId),
    /// An absolute path no module has yet.
    Planned(&'p Path),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Territory {
    pub may_import_layers: Vec<LayerId>,
    pub imported_by_layers: Vec<LayerId>,
    pub max_import_purity: u8,
    pub visible_zones: Vec<ZoneId>,
    pub may_not: Vec<Forbidden>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Forbidden {
    pub rule: RuleId,
    pub region: Region,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Region {
    /// Every module under one of `containers`: the isolated children of `parent` other than
    /// the site's own `kind` container.
    Siblings {
        parent: ContainerId,
        kind: ContainerKind,
        containers: Vec<ContainerId>,
    },
    /// Every module inside a container that does not hold the site, that container's entry aside.
    Internals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Breach {
    pub rule: RuleId,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedSlice {
    pub path: PathBuf,
    pub stage: Stage,
}

pub struct Territories<'a> {
    graph: &'a CompoundGraph,
    config: &'a Config,
    severities: EffectiveSeverities<'a>,
    judges: Vec<(&'a dyn DynRule, BoundJudge)>,
    registry: &'a RuleRegistry,
    isolated_children: HashMap<ContainerId, Vec<ContainerId>>,
    zone_roots: Vec<String>,
}

impl std::fmt::Debug for Territories<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Territories")
            .field("judges", &self.judges.len())
            .finish_non_exhaustive()
    }
}

impl<'a> Territories<'a> {
    /// # Errors
    /// [`ConfigError::BadGlob`] for an `overrides[].files` entry that is not a valid glob.
    pub fn new(
        graph: &'a CompoundGraph,
        config: &'a Config,
        project_root: &Utf8Path,
        registry: &'a RuleRegistry,
    ) -> Result<Self, ConfigError> {
        let judges = registry
            .rules()
            .iter()
            .filter_map(|r| {
                let options = config
                    .rules
                    .get(r.slug())
                    .map_or(&serde_json::Value::Null, |rc| &rc.options);
                Some((r.as_ref(), r.edge_judge(options)?))
            })
            .collect();
        let mut isolated_children: HashMap<ContainerId, Vec<ContainerId>> = HashMap::new();
        for c in graph.containers.values() {
            if let (true, Some(parent)) = (is_isolated(c.kind), c.parent) {
                isolated_children.entry(parent).or_default().push(c.id);
            }
        }
        Ok(Self {
            graph,
            config,
            severities: EffectiveSeverities::new(config, project_root)?,
            judges,
            registry,
            isolated_children,
            zone_roots: graph
                .private_zones()
                .iter()
                .map(|z| slashed(&z.root))
                .collect(),
        })
    }

    #[must_use]
    pub fn of(&self, site: Site<'_>) -> Territory {
        let graph = self.graph;
        let (path, module): (&Path, Option<&Module>) = match site {
            Site::Module(id) => graph
                .modules
                .get(&id)
                .map_or((Path::new(""), None), |m| (m.path.as_path(), Some(m))),
            Site::Planned(path) => (path, None),
        };
        let file = slashed(path);
        let severities = self.severities.for_slashed(&file);
        let on = |id: RuleId| {
            self.registry
                .by_id(id)
                .is_some_and(|r| severities.of(r) != Severity::Off)
        };
        let innermost = module.map_or_else(
            || graph.container_at(path),
            |m| graph.containers.get(&m.container),
        );
        let layer = module
            .map(|m| m.layer)
            .or(innermost.map(|c| c.layer))
            .and_then(|l| graph.layers.get(&l));

        let cross_layer = on(NO_CROSS_LAYER_IMPORT);
        let may_import_layers = graph
            .layers
            .values()
            .filter(|l| !cross_layer || layer.is_none_or(|from| may_import(from, l.id)))
            .map(|l| l.id)
            .collect();
        let imported_by_layers = layer.map_or_else(Vec::new, |to| {
            graph
                .layers
                .values()
                .filter(|l| may_import(l, to.id))
                .map(|l| l.id)
                .collect()
        });
        let max_import_purity = if on(PURITY) {
            module.map_or(PLANNED_PURITY, |m| m.purity.rank())
        } else {
            IMPUREST
        };
        let visible_zones = if on(VISIBILITY_SCOPE) {
            self.zone_roots
                .iter()
                .enumerate()
                .filter(|(_, root)| under(&file, root).is_some())
                .map(|(i, _)| ZoneId::of(i))
                .collect()
        } else {
            (0..graph.private_zones().len()).map(ZoneId::of).collect()
        };

        let mut may_not = Vec::new();
        if on(SIBLING_ISOLATION) {
            let planned = innermost
                .filter(|_| module.is_none())
                .filter(|c| self.planned_slice_in(c, path).is_some());
            may_not.extend(self.siblings(innermost, planned));
        }
        if on(PUBLIC_API) {
            may_not.push(Forbidden {
                rule: PUBLIC_API,
                region: Region::Internals,
            });
        }

        Territory {
            may_import_layers,
            imported_by_layers,
            max_import_purity,
            visible_zones,
            may_not,
        }
    }

    fn siblings(
        &self,
        innermost: Option<&'a Container>,
        planned_in: Option<&'a Container>,
    ) -> impl Iterator<Item = Forbidden> {
        let units = std::iter::successors(innermost, |c| {
            c.parent.and_then(|p| self.graph.containers.get(&p))
        })
        .filter(|c| is_isolated(c.kind))
        .filter_map(|c| Some((c.parent?, c.kind, Some(c.id))));
        planned_in
            .map(|layer| (layer.id, ContainerKind::Slice, None))
            .into_iter()
            .chain(units)
            .filter_map(|(parent, kind, own)| {
                let containers: Vec<ContainerId> = self
                    .isolated_children
                    .get(&parent)
                    .into_iter()
                    .flatten()
                    .copied()
                    .filter(|&c| Some(c) != own)
                    .collect();
                (!containers.is_empty()).then_some(Forbidden {
                    rule: SIBLING_ISOLATION,
                    region: Region::Siblings {
                        parent,
                        kind,
                        containers,
                    },
                })
            })
    }

    /// Each breach carries the rule's effective severity for the file of `from`.
    #[must_use]
    pub fn verdict(&self, from: ModuleId, to: ModuleId) -> Vec<Breach> {
        let Some(m) = self.graph.modules.get(&from) else {
            return vec![];
        };
        let severities = self.severities.for_file(&m.path);
        self.judges
            .iter()
            .filter_map(|(rule, judge)| {
                let severity = severities.of(*rule);
                (severity != Severity::Off && judge.breaks(self.graph, from, to)).then(|| Breach {
                    rule: rule.id(),
                    severity,
                })
            })
            .collect()
    }

    /// `Some` for a path directly in a sliced layer that is not one of its entries.
    #[must_use]
    pub fn planned_slice(&self, path: &Path) -> Option<PlannedSlice> {
        self.planned_slice_in(self.graph.container_at(path)?, path)
    }

    fn planned_slice_in(&self, container: &Container, path: &Path) -> Option<PlannedSlice> {
        let sliced = self
            .graph
            .layers
            .get(&container.layer)
            .is_some_and(|l| l.slices);
        if container.kind != ContainerKind::Layer || !sliced {
            return None;
        }
        let below = path.strip_prefix(&container.path).ok()?;
        let first = below.components().next()?;
        let is_file_slice = below.components().count() == 1;
        let entries = &self.config.structure.entries;
        if is_file_slice && entries.iter().any(|e| first.as_os_str() == e.as_str()) {
            return None;
        }
        Some(PlannedSlice {
            path: container.path.join(first),
            stage: if is_file_slice {
                Stage::FILE
            } else {
                Stage::FLAT
            },
        })
    }
}
