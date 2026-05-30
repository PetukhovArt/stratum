//! Filesystem validation of a parsed [`Config`].
//!
//! Parsing ([`crate::parse`]) is pure string→struct and never touches disk, so
//! a config that references a `layer.path` which does not exist on disk parses
//! fine but produces an empty/garbage graph downstream. This module closes that
//! gap with an explicit, filesystem-aware check.

use camino::Utf8Path;

use crate::{Config, ConfigError};

/// Check that every `layer.path` resolves to an existing directory under
/// `project_root`.
///
/// `layer.path` is interpreted relative to `project_root` — the absolute project
/// directory the graph builder walks (see `stratum-graph`'s `GraphBuilder`,
/// which strips `layer.path` off paths relative to that same root).
///
/// # Errors
/// Returns [`ConfigError::LayerPathNotFound`] for the first layer whose resolved
/// path is missing or is not a directory.
pub fn validate_layer_paths(config: &Config, project_root: &Utf8Path) -> Result<(), ConfigError> {
    for layer in &config.layers {
        let resolved = project_root.join(&layer.path);
        if !resolved.is_dir() {
            return Err(ConfigError::LayerPathNotFound {
                layer: layer.id.clone(),
                resolved,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{LayerConfig, ProjectConfig};
    use camino::Utf8PathBuf;
    use std::collections::BTreeMap;

    fn manifest_dir() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn config_with_layer_paths(paths: &[&str]) -> Config {
        Config {
            version: 1,
            project: ProjectConfig {
                root: Utf8PathBuf::from("."),
                extends: None,
            },
            layers: paths
                .iter()
                .enumerate()
                .map(|(i, p)| LayerConfig {
                    id: format!("layer{i}"),
                    path: Utf8PathBuf::from(*p),
                    depends_on: vec![],
                })
                .collect(),
            rules: BTreeMap::new(),
            overrides: vec![],
        }
    }

    #[test]
    fn existing_directories_pass() {
        // `src` exists in this crate.
        let cfg = config_with_layer_paths(&["src"]);
        assert!(validate_layer_paths(&cfg, &manifest_dir()).is_ok());
    }

    #[test]
    fn missing_directory_is_rejected_with_layer_name() {
        let cfg = config_with_layer_paths(&["src", "does-not-exist-xyz"]);
        let err = validate_layer_paths(&cfg, &manifest_dir()).unwrap_err();
        let ConfigError::LayerPathNotFound { layer, resolved } = &err else {
            unreachable!("expected LayerPathNotFound, got {err:?}");
        };
        // the second layer is the offending one, not the first (which exists)
        assert_eq!(layer, "layer1");
        assert!(resolved.as_str().ends_with("does-not-exist-xyz"));
        // the rendered message names the offending path
        let msg = ConfigError::LayerPathNotFound {
            layer: "layer1".into(),
            resolved: manifest_dir().join("does-not-exist-xyz"),
        }
        .to_string();
        assert!(msg.contains("layer1"));
        assert!(msg.contains("does-not-exist-xyz"));
    }

    #[test]
    fn a_file_path_is_rejected_because_it_is_not_a_directory() {
        // `Cargo.toml` exists but is a file, not a directory.
        let cfg = config_with_layer_paths(&["Cargo.toml"]);
        assert!(validate_layer_paths(&cfg, &manifest_dir()).is_err());
    }
}
