use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, LayerConfig, ProjectConfig};
use stratum_core::severity::Severity;
use stratum_core::structure::Structure;

const NEEDS_DECLARED_STRUCTURE: [&str; 5] = [
    "stratum/public-api",
    "stratum/sibling-isolation",
    "stratum/container-breadth",
    "stratum/zone-breadth",
    "stratum/missing-entry",
];

const TRUSTS_INFERRED_SLICES: [&str; 2] = ["stratum/public-api", "stratum/sibling-isolation"];

const MIN_SEGMENT_UNITS: usize = 3;
const MIN_SEGMENT_SHARE_PERCENT: usize = 30;
const MIN_SLICES_WITH_ENTRY: usize = 2;

struct Unit {
    name: String,
    has_entry: bool,
    children: Vec<String>,
}

#[must_use]
pub fn infer(root: &Utf8Path) -> Config {
    let mut structure = stratum_config::recommended_structure();
    let mut layers: Vec<LayerConfig> = sorted_dirs(&root.join("src"))
        .into_iter()
        .map(|name| LayerConfig {
            path: Utf8PathBuf::from(format!("src/{name}")),
            id: name,
            depends_on: vec![],
            slices: false,
        })
        .collect();
    let units: Vec<Vec<Unit>> = layers
        .iter()
        .map(|layer| read_units(&root.join(&layer.path), &structure))
        .collect();
    for (layer, units) in layers.iter_mut().zip(&units) {
        layer.slices = looks_sliced(units);
    }
    structure.segments = infer_segments(&units, &structure);

    let sliced = layers.iter().any(|l| l.slices);
    let mut rules = stratum_config::recommended_rules();
    for slug in NEEDS_DECLARED_STRUCTURE {
        if sliced && TRUSTS_INFERRED_SLICES.contains(&slug) {
            continue;
        }
        if let Some(rule) = rules.get_mut(slug) {
            rule.severity = Severity::Off;
        }
    }

    Config {
        version: 1,
        project: ProjectConfig::default(),
        structure,
        layers,
        rules,
        overrides: vec![],
        messages: stratum_core::text::Messages::default(),
    }
}

fn sorted_dirs(dir: &Utf8Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn is_candidate(name: &str, structure: &Structure) -> bool {
    !name.starts_with('.') && name != "node_modules" && !structure.is_private(name)
}

fn read_units(layer_dir: &Utf8Path, structure: &Structure) -> Vec<Unit> {
    sorted_dirs(layer_dir)
        .into_iter()
        .filter(|name| is_candidate(name, structure))
        .map(|name| {
            let dir = layer_dir.join(&name);
            Unit {
                has_entry: structure.entries.iter().any(|e| dir.join(e).is_file()),
                children: sorted_dirs(&dir)
                    .into_iter()
                    .filter(|child| is_candidate(child, structure))
                    .collect(),
                name,
            }
        })
        .collect()
}

fn looks_sliced(units: &[Unit]) -> bool {
    let with_entry = units.iter().filter(|u| u.has_entry).count();
    with_entry >= MIN_SLICES_WITH_ENTRY && with_entry * 2 >= units.len()
}

fn infer_segments(layers: &[Vec<Unit>], structure: &Structure) -> Vec<String> {
    let mut segments = BTreeSet::new();
    for units in layers {
        let unit_names: BTreeSet<&str> = units.iter().map(|u| u.name.as_str()).collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for child in units.iter().flat_map(|u| &u.children) {
            *counts.entry(child.as_str()).or_default() += 1;
        }
        segments.extend(
            counts
                .into_iter()
                .filter(|&(name, count)| {
                    count >= MIN_SEGMENT_UNITS
                        && count * 100 >= units.len() * MIN_SEGMENT_SHARE_PERCENT
                        && !unit_names.contains(name)
                        && structure.entry_rank(name).is_none()
                })
                .map(|(name, _)| name.to_owned()),
        );
    }
    segments.into_iter().collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn tree(paths: &[&str]) -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        for path in paths {
            let path = root.join(path);
            if path.extension().is_some() {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, "").unwrap();
            } else {
                fs::create_dir_all(&path).unwrap();
            }
        }
        (dir, root)
    }

    fn layer<'a>(cfg: &'a Config, id: &str) -> &'a LayerConfig {
        cfg.layers.iter().find(|l| l.id == id).unwrap()
    }

    #[test]
    fn infers_layers_from_tiny_ts_fixture() {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-ts")
            .canonicalize_utf8()
            .unwrap();
        let cfg = infer(&root);
        let names: Vec<&str> = cfg.layers.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(
            names,
            vec!["app", "entities", "features", "legacy", "shared"]
        );
        assert!(cfg.layers.iter().all(|l| !l.slices));
        assert!(cfg.structure.segments.is_empty());
        assert_eq!(cfg.rules.len(), stratum_config::recommended_rules().len());
    }

    #[test]
    fn returns_empty_layers_if_no_src() {
        let cfg = infer(Utf8Path::new("non-existent-dir-9999"));
        assert!(cfg.layers.is_empty());
    }

    #[test]
    fn a_folder_name_under_three_units_and_30_percent_of_a_layer_is_a_segment() {
        let (_dir, root) = tree(&[
            "src/features/a/ui",
            "src/features/a/model",
            "src/features/b/ui",
            "src/features/b/model",
            "src/features/c/ui",
            "src/features/c/lib",
            "src/features/d/lib",
            "src/features/e/.cache",
            "src/features/f/.cache",
            "src/features/g/.cache",
        ]);
        let cfg = infer(&root);
        assert_eq!(cfg.structure.segments, vec!["ui"]);
    }

    #[test]
    fn a_name_under_less_than_30_percent_of_the_units_is_no_segment() {
        let (_dir, root) = tree(&[
            "src/features/a/ui",
            "src/features/b/ui",
            "src/features/c/ui",
            "src/features/d",
            "src/features/e",
            "src/features/f",
            "src/features/g",
            "src/features/h",
            "src/features/i",
            "src/features/j",
            "src/features/k",
        ]);
        let cfg = infer(&root);
        assert!(cfg.structure.segments.is_empty());
    }

    #[test]
    fn a_private_zone_or_a_unit_name_of_the_layer_is_no_segment() {
        let (_dir, root) = tree(&[
            "src/features/a/_shared",
            "src/features/a/b",
            "src/features/b/_shared",
            "src/features/c/_shared",
            "src/features/c/b",
            "src/features/d/b",
        ]);
        let cfg = infer(&root);
        assert!(cfg.structure.segments.is_empty());
    }

    #[test]
    fn segments_from_several_layers_are_merged_and_sorted() {
        let (_dir, root) = tree(&[
            "src/features/a/ui",
            "src/features/b/ui",
            "src/features/c/ui",
            "src/services/a/model",
            "src/services/b/model",
            "src/services/c/model",
        ]);
        let cfg = infer(&root);
        assert_eq!(cfg.structure.segments, vec!["model", "ui"]);
    }

    #[test]
    fn a_layer_whose_folders_mostly_hold_an_entry_is_sliced() {
        let (_dir, root) = tree(&[
            "src/features/cart/index.ts",
            "src/features/auth/index.tsx",
            "src/features/legacy/old.ts",
            "src/features/_shared/index.ts",
            "src/features/index.ts",
            "src/shared/ui/index.ts",
            "src/shared/lib/a.ts",
            "src/shared/config/a.ts",
            "src/app/one/index.ts",
        ]);
        let cfg = infer(&root);
        assert!(layer(&cfg, "features").slices);
        assert!(!layer(&cfg, "shared").slices);
        assert!(!layer(&cfg, "app").slices);
    }

    #[test]
    fn inferred_slices_keep_public_api_and_sibling_isolation_on() {
        let (_dir, root) = tree(&["src/features/a/index.ts", "src/features/b/index.ts"]);
        let cfg = infer(&root);
        let recommended = stratum_config::recommended_rules();
        for slug in TRUSTS_INFERRED_SLICES {
            assert_eq!(cfg.rules[slug].severity, recommended[slug].severity);
        }
        assert_eq!(cfg.rules["stratum/missing-entry"].severity, Severity::Off);
    }

    #[test]
    fn without_inferred_slices_structure_rules_stay_off() {
        let (_dir, root) = tree(&["src/features/a/index.ts", "src/features/b/a.ts"]);
        let cfg = infer(&root);
        for slug in NEEDS_DECLARED_STRUCTURE {
            assert_eq!(cfg.rules[slug].severity, Severity::Off);
        }
    }
}
