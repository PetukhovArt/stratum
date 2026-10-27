use globset::{Glob, GlobMatcher};

use crate::{Config, ConfigError};

#[derive(Debug, Clone)]
pub struct EntryPoints {
    globs: Vec<(String, GlobMatcher)>,
}

impl EntryPoints {
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry of `project.entry_points` that is not a valid glob.
    pub fn new(config: &Config) -> Result<Self, ConfigError> {
        let globs = config
            .project
            .entry_points
            .iter()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(|g| (pattern.clone(), g.compile_matcher()))
                    .map_err(|source| ConfigError::BadGlob {
                        pattern: pattern.clone(),
                        source,
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { globs })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.globs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    /// The `i`-th glob as written in the config.
    #[must_use]
    pub fn glob(&self, i: usize) -> &str {
        &self.globs[i].0
    }

    /// Whether some glob matches the root-relative, `/`-separated `path`; sets `hit[i]` for
    /// every glob `i` that does. `hit` has [`Self::len`] slots.
    pub fn mark(&self, path: &str, hit: &mut [bool]) -> bool {
        let mut matched = false;
        for (i, (_, glob)) in self.globs.iter().enumerate() {
            if glob.is_match(path) {
                hit[i] = true;
                matched = true;
            }
        }
        matched
    }

    /// The directories, relative to the root, below which a file some glob matches can lie: each
    /// glob's leading segments up to the first one with a glob character (`""` is the root).
    #[must_use]
    pub fn base_dirs(&self) -> Vec<String> {
        let mut dirs: Vec<String> = self
            .globs
            .iter()
            .map(|(pattern, _)| literal_dir(pattern))
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }
}

/// `pattern`'s leading segments before the first one holding `*`, `?`, `[` or `{`, without the
/// last segment when every segment is literal.
#[must_use]
pub fn literal_dir(pattern: &str) -> String {
    let segments: Vec<&str> = pattern.split('/').collect();
    let literal = segments
        .iter()
        .take_while(|s| !s.contains(['*', '?', '[', '{']))
        .count()
        .min(segments.len().saturating_sub(1));
    segments[..literal].join("/")
}

#[cfg(test)]
mod tests {
    use super::literal_dir;

    #[test]
    fn literal_dir_stops_at_the_first_glob_segment() {
        assert_eq!(literal_dir("src/main.tsx"), "src");
        assert_eq!(literal_dir("main.ts"), "");
        assert_eq!(literal_dir("src/**/*.stories.ts"), "src");
        assert_eq!(literal_dir("*.config.ts"), "");
        assert_eq!(literal_dir("apps/web/src/*.ts"), "apps/web/src");
    }
}
