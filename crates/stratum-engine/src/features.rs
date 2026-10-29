use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use stratum_config::{Config, ConfigError};
use stratum_core::{edge::EdgeKind, ids::ModuleId, types::Import};
use stratum_graph::{CompoundGraph, crossing_names, forwarding_imports};

use crate::paths::relative;

/// Every feature of a project and the modules two or more of them hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Features {
    features: Vec<Feature>,
    shared: Vec<ModuleId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    pub id: String,
    pub name: String,
    /// Ascending.
    pub entry: Vec<ModuleId>,
    /// The entry modules and every module they reach by static imports; ascending.
    pub modules: Vec<ModuleId>,
}

impl Features {
    /// The `features` of `config`, in id order, else one feature per folder or file directly in
    /// the first sliced layer, by name. An entry glob is matched against module paths relative
    /// to `root`.
    ///
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry glob that is not valid.
    pub fn of(
        graph: &CompoundGraph,
        config: &Config,
        root: &Utf8Path,
    ) -> Result<Self, ConfigError> {
        let seeds = if config.features.is_empty() {
            slice_seeds(graph, root)
        } else {
            configured_seeds(graph, config, root)?
        };
        let features: Vec<Feature> = seeds
            .into_iter()
            .map(|(id, name, entry)| Feature {
                modules: static_closure(graph, &entry),
                entry: entry.into_iter().collect(),
                id,
                name,
            })
            .collect();
        let mut holders: BTreeMap<ModuleId, usize> = BTreeMap::new();
        for module in features.iter().flat_map(|f| &f.modules) {
            *holders.entry(*module).or_default() += 1;
        }
        let shared = holders
            .into_iter()
            .filter(|&(_, n)| n >= 2)
            .map(|(m, _)| m)
            .collect();
        Ok(Self { features, shared })
    }

    #[must_use]
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    /// Ascending.
    #[must_use]
    pub fn shared(&self) -> &[ModuleId] {
        &self.shared
    }

    #[must_use]
    pub fn is_shared(&self, module: ModuleId) -> bool {
        self.shared.binary_search(&module).is_ok()
    }

    /// The ids of the features holding `module`, in feature order.
    pub fn holders(&self, module: ModuleId) -> impl Iterator<Item = &str> {
        self.features
            .iter()
            .filter(move |f| f.modules.binary_search(&module).is_ok())
            .map(|f| f.id.as_str())
    }

    /// How many modules of `feature` no other feature holds.
    #[must_use]
    pub fn exclusive_count(&self, feature: &Feature) -> usize {
        feature
            .modules
            .iter()
            .filter(|&&m| !self.is_shared(m))
            .count()
    }
}

/// The code of a project each feature can run, per top-level declaration.
pub(crate) struct Reach<'f> {
    features: Vec<(&'f Feature, Runs)>,
}

/// What one feature runs: every declaration of `whole`, the `named` declarations, and the
/// top-level statements of its modules.
#[derive(Default)]
struct Runs {
    whole: BTreeSet<ModuleId>,
    named: BTreeMap<ModuleId, BTreeSet<String>>,
}

impl<'f> Reach<'f> {
    /// A feature runs its entry modules whole, and each declaration of another module that one
    /// of its modules imports by a name resolving to it; a namespace import or a `require` of a
    /// module runs it whole. Type-only names and `import()` run nothing. A re-export runs what it forwards only when its module runs
    /// whole, so a name imported through a barrel runs only its own declaration. A declaration
    /// that a run declaration calls is not followed.
    pub(crate) fn of(features: &'f Features, graph: &CompoundGraph) -> Self {
        let features = features
            .features
            .iter()
            .map(|f| (f, Runs::of(f, graph)))
            .collect();
        Self { features }
    }

    /// The ids, ascending, of the features that run the code of `module` inside the top-level
    /// declaration `owner`, or outside every declaration for `None`.
    pub(crate) fn features_running(&self, module: ModuleId, owner: Option<&str>) -> Vec<String> {
        let mut ids: Vec<String> = self
            .features
            .iter()
            .filter(|(f, runs)| match owner {
                None => f.modules.binary_search(&module).is_ok(),
                Some(owner) => {
                    runs.whole.contains(&module)
                        || runs.named.get(&module).is_some_and(|n| n.contains(owner))
                }
            })
            .map(|(f, _)| f.id.clone())
            .collect();
        ids.sort();
        ids
    }
}

impl Runs {
    fn of(feature: &Feature, graph: &CompoundGraph) -> Self {
        let mut runs = Self::default();
        let mut whole: Vec<ModuleId> = Vec::new();
        for &entry in &feature.entry {
            if runs.whole.insert(entry) {
                whole.push(entry);
            }
        }
        for module in feature.modules.iter().filter_map(|m| graph.modules.get(m)) {
            let forwarding = forwarding_imports(module);
            let uses = module
                .imports
                .iter()
                .enumerate()
                .filter(|(index, _)| !forwarding.contains(index))
                .map(|(_, import)| import);
            runs.take(graph, module.id, uses, &mut whole);
        }
        while let Some(id) = whole.pop() {
            let Some(module) = graph.modules.get(&id) else {
                continue;
            };
            let forwarded = forwarding_imports(module)
                .into_iter()
                .filter_map(|index| module.imports.get(index));
            runs.take(graph, id, forwarded, &mut whole);
        }
        runs
    }

    /// Takes the names the static `imports` of module `from` resolve to in other modules,
    /// queueing each module that newly runs whole on `whole`.
    fn take<'g>(
        &mut self,
        graph: &CompoundGraph,
        from: ModuleId,
        imports: impl Iterator<Item = &'g Import>,
        whole: &mut Vec<ModuleId>,
    ) {
        let names = imports
            .filter(|i| i.kind == EdgeKind::Static)
            .flat_map(|i| crossing_names(graph, i))
            .filter(|n| !n.type_only)
            .filter_map(|n| n.decl)
            .filter(|d| d.module != from);
        for decl in names {
            match decl.binding {
                None => {
                    if self.whole.insert(decl.module) {
                        whole.push(decl.module);
                    }
                }
                Some(binding) => {
                    self.named.entry(decl.module).or_default().insert(binding);
                }
            }
        }
    }
}

type Seed = (String, String, BTreeSet<ModuleId>);

fn configured_seeds(
    graph: &CompoundGraph,
    config: &Config,
    root: &Utf8Path,
) -> Result<Vec<Seed>, ConfigError> {
    let paths: Vec<(ModuleId, String)> = graph
        .modules
        .values()
        .map(|m| (m.id, relative(&m.path, root)))
        .collect();
    config
        .features
        .iter()
        .map(|(id, feature)| {
            let globs = feature.entry_globs()?;
            let entry = paths
                .iter()
                .filter(|(_, path)| globs.iter().any(|(_, g)| g.is_match(path)))
                .map(|(m, _)| *m)
                .collect();
            let name = feature.name.clone().unwrap_or_else(|| id.clone());
            Ok((id.clone(), name, entry))
        })
        .collect()
}

/// One seed per entry of the first sliced layer's folder, a folder another layer claims included.
fn slice_seeds(graph: &CompoundGraph, root: &Utf8Path) -> Vec<Seed> {
    let Some(layer) = graph
        .layers
        .values()
        .filter(|l| l.slices)
        .min_by_key(|l| l.id)
    else {
        return Vec::new();
    };
    let folder = format!("{}/", relative(&layer.path, root));
    let mut seeds: BTreeMap<String, BTreeSet<ModuleId>> = BTreeMap::new();
    for module in graph.modules.values() {
        let path = relative(&module.path, root);
        if let Some(rest) = path.strip_prefix(&folder)
            && let Some(name) = rest.split('/').next()
        {
            seeds.entry(name.to_owned()).or_default().insert(module.id);
        }
    }
    seeds
        .into_iter()
        .map(|(name, entry)| (name.clone(), name, entry))
        .collect()
}

fn static_closure(graph: &CompoundGraph, entry: &BTreeSet<ModuleId>) -> Vec<ModuleId> {
    let mut reached = entry.clone();
    let mut queue: Vec<ModuleId> = entry.iter().copied().collect();
    while let Some(id) = queue.pop() {
        let Some(module) = graph.modules.get(&id) else {
            continue;
        };
        let targets = module
            .imports
            .iter()
            .filter(|i| i.kind == EdgeKind::Static)
            .filter_map(|i| i.target);
        for target in targets {
            if reached.insert(target) {
                queue.push(target);
            }
        }
    }
    reached.into_iter().collect()
}

pub(crate) fn unmatched_entry(
    graph: &CompoundGraph,
    config: &Config,
    root: &Utf8Path,
) -> Result<Option<(String, String)>, ConfigError> {
    let paths: Vec<String> = graph
        .modules
        .values()
        .map(|m| relative(&m.path, root))
        .collect();
    for (id, feature) in &config.features {
        for (glob, matcher) in feature.entry_globs()? {
            if !paths.iter().any(|p| matcher.is_match(p)) {
                return Ok(Some((id.clone(), glob.to_owned())));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use camino::Utf8PathBuf;

    use super::*;
    use crate::pipeline::{EngineError, Source, graph};

    const FILES: [(&str, &str); 12] = [
        (
            "src/pages/a/index.ts",
            r#"import type { T } from "../../lib/t";
import { b } from "../../lib/b";
export const a = () => import("../../lib/lazy");
export const r = require("../../lib/req");
"#,
        ),
        (
            "src/pages/b/index.ts",
            r#"import { c } from "../../lib/c"; export const b = c;"#,
        ),
        ("src/pages/c.ts", "export const c = 1;"),
        (
            "src/pages/ed/index.ts",
            r#"import { core } from "./core/core"; export const ed = core;"#,
        ),
        ("src/pages/ed/core/core.ts", "export const core = 1;"),
        ("src/widgets/w/index.ts", "export const w = 1;"),
        (
            "src/lib/b.ts",
            r#"import { c } from "./c"; export const b = c;"#,
        ),
        (
            "src/lib/c.ts",
            r#"import { b } from "./b"; export const c = 1;"#,
        ),
        ("src/lib/t.ts", "export type T = number;"),
        ("src/lib/lazy.ts", "export const lazy = 1;"),
        ("src/lib/req.ts", "export const req = 1;"),
        ("src/lib/unused.ts", "export const unused = 1;"),
    ];

    struct Project {
        _dir: tempfile::TempDir,
        root: Utf8PathBuf,
    }

    fn project() -> Project {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        for (rel, text) in FILES {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Project { _dir: dir, root }
    }

    fn config(features: &str) -> Config {
        let text = format!(
            r#"{{ "version": 1, "features": {features}, "layers": [
                {{ "id": "lib", "path": "src/lib" }},
                {{ "id": "pages", "path": "src/pages", "slices": true }},
                {{ "id": "ed-core", "path": "src/pages/ed/core" }},
                {{ "id": "ed", "path": "src/pages/ed", "depends_on": ["ed-core"] }},
                {{ "id": "widgets", "path": "src/widgets", "slices": true }}
            ] }}"#
        );
        stratum_config::parse_str(&Utf8PathBuf::from("inline"), &text).unwrap()
    }

    fn features_of(features: &str) -> Result<(Features, Vec<String>), EngineError> {
        let project = project();
        let config = config(features);
        let (root, graph, ..) = graph(&project.root, &config, Source::Overlaid(Vec::new()))?;
        let features = Features::of(&graph, &config, &root).unwrap();
        let path = |m: &ModuleId| relative(&graph.modules[m].path, &root);
        let mut lines: Vec<String> = features
            .features()
            .iter()
            .map(|f| {
                let mut paths: Vec<String> = f.modules.iter().map(path).collect();
                paths.sort();
                format!("{} ({}): {}", f.id, f.name, paths.join(" "))
            })
            .collect();
        let mut shared: Vec<String> = features.shared().iter().map(path).collect();
        shared.sort();
        lines.push(format!("shared: {}", shared.join(" ")));
        Ok((features, lines))
    }

    #[test]
    fn a_feature_follows_static_type_only_and_require_imports_through_a_cycle_but_not_import_calls()
    {
        let (_, lines) =
            features_of(r#"{ "a": { "name": "A", "entry": ["src/pages/a/**"] } }"#).unwrap();
        assert_eq!(
            lines,
            [
                "a (A): src/lib/b.ts src/lib/c.ts src/lib/req.ts src/lib/t.ts src/pages/a/index.ts",
                "shared: ",
            ]
        );
    }

    #[test]
    fn without_features_each_entry_of_the_first_sliced_layer_is_one_and_shared_is_in_two() {
        let (features, lines) = features_of("{}").unwrap();
        assert_eq!(
            lines,
            [
                "a (a): src/lib/b.ts src/lib/c.ts src/lib/req.ts src/lib/t.ts src/pages/a/index.ts",
                "b (b): src/lib/b.ts src/lib/c.ts src/pages/b/index.ts",
                "c.ts (c.ts): src/pages/c.ts",
                "ed (ed): src/pages/ed/core/core.ts src/pages/ed/index.ts",
                "shared: src/lib/b.ts src/lib/c.ts",
            ]
        );
        let a = features.get("a").unwrap();
        assert_eq!(features.exclusive_count(a), 3);
        let b_ts = features.shared()[0];
        assert_eq!(features.holders(b_ts).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn an_entry_glob_matching_no_module_fails_the_build() {
        let err =
            features_of(r#"{ "a": { "entry": ["src/pages/a/**", "src/pagse/**"] } }"#).unwrap_err();
        assert!(
            matches!(&err, EngineError::FeatureEntryUnmatched { feature, glob }
                if feature == "a" && glob == "src/pagse/**"),
            "{err}"
        );
    }
}
