use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, LayerConfig, ProjectConfig};
use walkdir::WalkDir;

const NEEDS_DECLARED_STRUCTURE: [&str; 3] = [
    "stratum/public-api",
    "stratum/sibling-isolation",
    "stratum/container-breadth",
];

#[must_use]
pub fn infer(root: &Utf8Path) -> Config {
    let src = root.join("src");
    let mut layers = Vec::new();
    if src.is_dir() {
        for entry in WalkDir::new(src.as_std_path())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                layers.push(LayerConfig {
                    id: name.clone(),
                    path: Utf8PathBuf::from(format!("src/{name}")),
                    depends_on: vec![],
                    slices: false,
                });
            }
        }
    }
    layers.sort_by(|a, b| a.id.cmp(&b.id));

    let mut rules = stratum_config::recommended_rules();
    for slug in NEEDS_DECLARED_STRUCTURE {
        if let Some(rule) = rules.get_mut(slug) {
            rule.severity = stratum_core::severity::Severity::Off;
        }
    }

    Config {
        version: 1,
        project: ProjectConfig::default(),
        structure: stratum_config::recommended_structure(),
        layers,
        rules,
        overrides: vec![],
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
        assert_eq!(cfg.rules.len(), stratum_config::recommended_rules().len());
    }

    #[test]
    fn returns_empty_layers_if_no_src() {
        let cfg = infer(Utf8Path::new("non-existent-dir-9999"));
        assert!(cfg.layers.is_empty());
    }
}
