use camino::Utf8Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::{Config, ConfigError};

pub const DEFAULT_IGNORE: [&str; 7] = [
    "**/node_modules",
    "**/dist",
    "**/out",
    "**/build",
    "**/coverage",
    "**/.git",
    "**/*.d.ts",
];

/// [`DEFAULT_IGNORE`] followed by `project.ignore`; for each path the last matching glob
/// decides, and a `!glob` un-ignores.
#[derive(Debug, Clone)]
pub struct IgnoreSet {
    globs: GlobSet,
    negated: Vec<bool>,
}

impl IgnoreSet {
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry of `project.ignore` that is not a valid glob.
    pub fn new(config: &Config) -> Result<Self, ConfigError> {
        let mut builder = GlobSetBuilder::new();
        let mut negated = Vec::new();
        let user = config.project.ignore.iter().map(String::as_str);
        for pattern in DEFAULT_IGNORE.into_iter().chain(user) {
            let (negate, glob) = pattern
                .strip_prefix('!')
                .map_or((false, pattern), |rest| (true, rest));
            builder.add(Glob::new(glob).map_err(|source| ConfigError::BadGlob {
                pattern: pattern.to_owned(),
                source,
            })?);
            negated.push(negate);
        }
        let globs = builder.build().map_err(|source| ConfigError::BadGlob {
            pattern: config.project.ignore.join(","),
            source,
        })?;
        Ok(Self { globs, negated })
    }

    /// `rel` is relative to the project root; it is ignored when it or any of its
    /// ancestor directories is.
    #[must_use]
    pub fn is_ignored(&self, rel: &Utf8Path) -> bool {
        let mut prefix = String::new();
        rel.components().any(|component| {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component.as_str());
            self.globs
                .matches(&prefix)
                .last()
                .is_some_and(|&i| !self.negated[i])
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn set(ignore: &[&str]) -> IgnoreSet {
        let text = serde_json::json!({
            "version": 1,
            "project": { "ignore": ignore },
        });
        IgnoreSet::new(&serde_json::from_value(text).unwrap()).unwrap()
    }

    #[test]
    fn later_negation_unignores_a_default_but_not_inside_an_ignored_directory() {
        let s = set(&["src/public/**", "!**/dist", "!src/public/vendor/keep.js"]);
        assert!(s.is_ignored(Utf8Path::new("src/types/env.d.ts")));
        assert!(s.is_ignored(Utf8Path::new("src/node_modules/a/index.ts")));
        assert!(!s.is_ignored(Utf8Path::new("src/dist/a.ts")));
        assert!(s.is_ignored(Utf8Path::new("src/public/vendor/keep.js")));
        assert!(!s.is_ignored(Utf8Path::new("src/publicity/a.ts")));
    }
}
