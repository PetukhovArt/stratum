use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, LayerConfig, ProjectConfig, RuleConfig};
use stratum_core::severity::Severity;
use walkdir::WalkDir;

const BUILTIN_RULES: &[(&str, Severity)] = &[
    ("stratum/no-cross-layer-import", Severity::Warning),
    ("stratum/no-circular-deps", Severity::Error),
    ("stratum/stage-purity", Severity::Warning),
    ("stratum/miller-limit", Severity::Info),
    ("stratum/depth-ratio", Severity::Info),
];

/// Synthesise a `Config` by inspecting `<root>/src/*` directories.
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
                });
            }
        }
    }
    layers.sort_by(|a, b| a.id.cmp(&b.id));

    let mut rules = BTreeMap::new();
    for (slug, sev) in BUILTIN_RULES {
        rules.insert(
            (*slug).into(),
            RuleConfig {
                severity: *sev,
                options: serde_json::Value::Null,
                script: None,
            },
        );
    }

    Config {
        version: 1,
        project: ProjectConfig {
            root: Utf8PathBuf::from("./src"),
            extends: None,
        },
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
        assert_eq!(names, vec!["app", "entities", "features", "legacy", "shared"]);
        assert_eq!(cfg.rules.len(), 5);
    }

    #[test]
    fn returns_empty_layers_if_no_src() {
        let cfg = infer(Utf8Path::new("non-existent-dir-9999"));
        assert!(cfg.layers.is_empty());
    }
}
