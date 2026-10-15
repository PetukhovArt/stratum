use std::fs;

use camino::Utf8Path;
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde_json::Value;

use crate::{Config, ConfigError, IgnoreSet, extends, overrides};

/// Parse a JSONC string into a [`Config`], resolving `project.extends`
/// (paths relative to `path`'s directory, or `stratum:*` presets).
///
/// # Errors
/// Returns [`ConfigError::Parse`] when the document is not valid JSONC,
/// [`ConfigError::Io`] when an extended file cannot be read,
/// [`ConfigError::UnknownPreset`] / [`ConfigError::ExtendsCycle`] for a bad `extends` chain, or
/// [`ConfigError::Validation`] when the merged document does not match the [`Config`] schema
/// or a layer depends on an undeclared layer;
/// [`ConfigError::BadGlob`] for an invalid `project.ignore` glob.
pub fn parse_str(path: &Utf8Path, text: &str) -> Result<Config, ConfigError> {
    let document = parse_jsonc(path, text)?;
    let mut merged = extends::resolve(path, document)?;
    if let Some(fields) = merged.as_object_mut() {
        fields.remove("$schema");
    }
    let config: Config = serde_json::from_value(merged).map_err(|e| ConfigError::Validation {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    check_layer_refs(path, &config)?;
    IgnoreSet::new(&config)?;
    overrides::validate(&config)?;
    Ok(config)
}

/// # Errors
/// Returns [`ConfigError::Io`] if the file cannot be read; otherwise see [`parse_str`].
pub fn parse_file(path: &Utf8Path) -> Result<Config, ConfigError> {
    parse_str(path, &read(path)?)
}

pub(crate) fn read_jsonc(path: &Utf8Path) -> Result<Value, ConfigError> {
    parse_jsonc(path, &read(path)?)
}

fn read(path: &Utf8Path) -> Result<String, ConfigError> {
    fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

fn parse_jsonc(path: &Utf8Path, text: &str) -> Result<Value, ConfigError> {
    parse_to_serde_value(text, &ParseOptions::default())
        .map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?
        .ok_or_else(|| ConfigError::Parse {
            path: path.to_path_buf(),
            message: "empty document".into(),
        })
}

fn check_layer_refs(path: &Utf8Path, config: &Config) -> Result<(), ConfigError> {
    for layer in &config.layers {
        for dep in &layer.depends_on {
            if !config.layers.iter().any(|l| &l.id == dep) {
                return Err(ConfigError::Validation {
                    path: path.to_path_buf(),
                    message: format!("layer {:?} depends on undeclared layer {dep:?}", layer.id),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn fixture(name: &str) -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn parses_example_jsonc_with_comments_and_trailing_commas() {
        let path = fixture("example.config.jsonc");
        let cfg = parse_file(&path).unwrap();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.layers.len(), 4);
        assert_eq!(cfg.layers[0].id, "app");
        assert_eq!(cfg.rules.len(), 5);
        assert_eq!(cfg.overrides.len(), 1);
        assert_eq!(cfg.overrides[0].files, vec!["src/legacy/**".to_string()]);
    }

    #[test]
    fn rejects_unknown_severity() {
        let path = Utf8PathBuf::from("inline");
        let bad = r#"{ "version": 1, "project": {}, "layers": [],
                       "rules": { "x": { "severity": "panic" } } }"#;
        let err = parse_str(&path, bad).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn rejects_missing_required_field() {
        let path = Utf8PathBuf::from("inline");
        let bad = r#"{ "layers": [] }"#;
        let err = parse_str(&path, bad).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn fsd_preset_overridden_paths_keep_fsd_dependencies_and_recommended_rules() {
        let path = Utf8PathBuf::from("inline");
        let text = r#"{ "$schema": "./stratum.schema.json", "version": 1,
            "project": { "extends": "stratum:fsd" },
            "layers": [{ "id": "features", "path": "src/renderer/features" }] }"#;
        let cfg = parse_str(&path, text).unwrap();
        let features = cfg.layers.iter().find(|l| l.id == "features").unwrap();
        assert_eq!(features.path, "src/renderer/features");
        assert_eq!(features.depends_on, vec!["entities", "shared"]);
        assert_eq!(cfg.layers.len(), 6);
        assert_eq!(cfg.rules, crate::recommended_rules());
    }

    #[test]
    fn extends_cycle_is_an_error_naming_the_chain() {
        let dir = tempfile::tempdir().unwrap();
        let dir = Utf8Path::from_path(dir.path()).unwrap();
        let a = dir.join("a.jsonc");
        fs::write(&a, r#"{ "project": { "extends": "./b.jsonc" } }"#).unwrap();
        fs::write(
            dir.join("b.jsonc"),
            r#"{ "project": { "extends": "./a.jsonc" } }"#,
        )
        .unwrap();
        let err = parse_file(&a).unwrap_err();
        assert!(
            matches!(&err, ConfigError::ExtendsCycle { chain } if chain.matches("a.jsonc").count() == 2),
            "{err}"
        );
    }

    #[test]
    fn unknown_preset_and_undeclared_layer_dependency_are_errors() {
        let path = Utf8PathBuf::from("inline");
        let preset = r#"{ "version": 1, "project": { "extends": "stratum:nope" } }"#;
        assert!(matches!(
            parse_str(&path, preset),
            Err(ConfigError::UnknownPreset { .. })
        ));
        let dep = r#"{ "version": 1, "project": {},
            "layers": [{ "id": "app", "path": "src/app", "depends_on": ["shraed"] }] }"#;
        let err = parse_str(&path, dep).unwrap_err();
        assert!(err.to_string().contains("shraed"), "{err}");
    }
}
