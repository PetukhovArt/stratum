use std::fs;

use camino::Utf8Path;
use jsonc_parser::{ParseOptions, parse_to_serde_value};

use crate::{Config, ConfigError};

/// Parse a JSONC string into a [`Config`].
///
/// # Errors
/// Returns [`ConfigError::Parse`] when the document is not valid JSONC or
/// [`ConfigError::Validation`] when it does not match the [`Config`] schema.
pub fn parse_str(path: &Utf8Path, text: &str) -> Result<Config, ConfigError> {
    let value = parse_to_serde_value(text, &ParseOptions::default())
        .map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?
        .ok_or_else(|| ConfigError::Parse {
            path: path.to_path_buf(),
            message: "empty document".into(),
        })?;
    serde_json::from_value(value).map_err(|e| ConfigError::Validation {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Read `path` from disk and parse it as JSONC.
///
/// # Errors
/// Returns [`ConfigError::Io`] if the file cannot be read; otherwise see [`parse_str`].
pub fn parse_file(path: &Utf8Path) -> Result<Config, ConfigError> {
    let text = fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    parse_str(path, &text)
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
        let bad = r#"{ "version": 1, "project": { "root": "." }, "layers": [],
                       "rules": { "x": { "severity": "panic" } } }"#;
        let err = parse_str(&path, bad).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    #[test]
    fn rejects_missing_required_field() {
        let path = Utf8PathBuf::from("inline");
        let bad = r#"{ "version": 1, "layers": [] }"#;
        let err = parse_str(&path, bad).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }
}
