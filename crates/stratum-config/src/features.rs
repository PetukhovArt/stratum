use std::collections::BTreeMap;

use camino::Utf8Path;
use globset::{Glob, GlobMatcher};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Config, ConfigError};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeatureConfig {
    /// Shown instead of the id; defaults to the id.
    #[serde(default)]
    pub name: Option<String>,
    /// Globs relative to the project root of the modules the feature starts from; each must
    /// match a module.
    pub entry: Vec<String>,
}

impl FeatureConfig {
    /// Each `entry` glob as written, with its matcher.
    ///
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry that is not a valid glob.
    pub fn entry_globs(&self) -> Result<Vec<(&str, GlobMatcher)>, ConfigError> {
        self.entry
            .iter()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(|g| (pattern.as_str(), g.compile_matcher()))
                    .map_err(|source| ConfigError::BadGlob {
                        pattern: pattern.clone(),
                        source,
                    })
            })
            .collect()
    }
}

fn is_feature_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub(crate) fn validate(path: &Utf8Path, config: &Config) -> Result<(), ConfigError> {
    let invalid = |message: String| ConfigError::Validation {
        path: path.to_path_buf(),
        message,
    };
    for (id, feature) in &config.features {
        if !is_feature_id(id) {
            return Err(invalid(format!(
                "feature id {id:?} must match ^[\\w.-]+$: ASCII letters, digits, `_`, `.`, `-`"
            )));
        }
        if feature.entry.is_empty() {
            return Err(invalid(format!(
                "feature {id:?} needs at least one `entry` glob"
            )));
        }
        feature.entry_globs()?;
    }
    Ok(())
}

pub(crate) fn schema(generator: &mut SchemaGenerator) -> Schema {
    let mut schema = generator
        .subschema_for::<BTreeMap<String, FeatureConfig>>()
        .into_object();
    schema
        .extensions
        .insert("propertyNames".into(), json!({ "pattern": r"^[\w.-]+$" }));
    Schema::Object(schema)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use camino::Utf8PathBuf;

    use crate::{ConfigError, parse_str};

    fn parse(features: &str) -> Result<crate::Config, ConfigError> {
        let text = format!(r#"{{ "version": 1, "features": {features} }}"#);
        parse_str(&Utf8PathBuf::from("inline"), &text)
    }

    #[test]
    fn features_parse_with_an_optional_name() {
        let config = parse(
            r#"{ "checkout": { "name": "Checkout", "entry": ["src/pages/checkout/**"] },
                 "search.v2": { "entry": ["src/features/search/index.ts"] } }"#,
        )
        .unwrap();
        assert_eq!(
            config.features["checkout"].name.as_deref(),
            Some("Checkout")
        );
        assert_eq!(config.features["search.v2"].name, None);
    }

    #[test]
    fn a_bad_id_an_empty_entry_a_bad_glob_and_an_unknown_key_are_errors() {
        for (features, expected) in [
            (r#"{ "check out": { "entry": ["a"] } }"#, "check out"),
            (r#"{ "checkout": { "entry": [] } }"#, "entry"),
            (r#"{ "checkout": { "entry": ["src/[a"] } }"#, "src/[a"),
            (
                r#"{ "checkout": { "entry": ["a"], "entrys": [] } }"#,
                "entrys",
            ),
        ] {
            let err = parse(features).unwrap_err();
            assert!(err.to_string().contains(expected), "{features}: {err}");
        }
    }
}
