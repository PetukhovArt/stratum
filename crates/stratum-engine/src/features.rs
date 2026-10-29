use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use stratum_config::{Config, ConfigError};
use stratum_core::{edge::EdgeKind, ids::ModuleId, types::ContainerKind};
use stratum_graph::CompoundGraph;

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
    /// The `features` of `config`, in id order, else one feature per slice of the first sliced
    /// layer, in graph order. An entry glob is matched against module paths relative to `root`.
    ///
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry glob that is not valid.
    pub fn of(
        graph: &CompoundGraph,
        config: &Config,
        root: &Utf8Path,
    ) -> Result<Self, ConfigError> {
        let seeds = if config.features.is_empty() {
            slice_seeds(graph)
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

fn slice_seeds(graph: &CompoundGraph) -> Vec<Seed> {
    let Some(layer) = graph
        .layers
        .values()
        .filter(|l| l.slices)
        .map(|l| l.id)
        .min()
    else {
        return Vec::new();
    };
    graph
        .containers
        .values()
        .filter(|c| c.kind == ContainerKind::Slice && c.layer == layer)
        .map(|slice| {
            let entry = graph
                .modules
                .keys()
                .filter(|&&m| graph.outer_container(m).is_some_and(|c| c.id == slice.id))
                .copied()
                .collect();
            (slice.name.clone(), slice.name.clone(), entry)
        })
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

    const FILES: [(&str, &str); 10] = [
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
    fn without_features_each_slice_of_the_first_sliced_layer_is_one_and_shared_is_in_two() {
        let (features, lines) = features_of("{}").unwrap();
        assert_eq!(
            lines,
            [
                "a (a): src/lib/b.ts src/lib/c.ts src/lib/req.ts src/lib/t.ts src/pages/a/index.ts",
                "b (b): src/lib/b.ts src/lib/c.ts src/pages/b/index.ts",
                "c.ts (c.ts): src/pages/c.ts",
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
