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

    /// The items whose root-relative, `/`-separated path some glob matches, in input order.
    ///
    /// # Errors
    /// The first glob, in config order, that matches none of `files`.
    pub fn select<T>(&self, files: impl IntoIterator<Item = (T, String)>) -> Result<Vec<T>, &str> {
        let mut hit = vec![false; self.globs.len()];
        let mut selected = Vec::new();
        for (item, path) in files {
            let mut matched = false;
            for (i, (_, glob)) in self.globs.iter().enumerate() {
                if glob.is_match(&path) {
                    hit[i] = true;
                    matched = true;
                }
            }
            if matched {
                selected.push(item);
            }
        }
        match hit.iter().position(|h| !h) {
            Some(i) => Err(&self.globs[i].0),
            None => Ok(selected),
        }
    }
}
