use std::path::Path;

use camino::Utf8Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

use stratum_config::{Config, ConfigError};
use stratum_core::severity::Severity;

use crate::registry::DynRule;

pub(crate) struct EffectiveSeverities<'c> {
    config: &'c Config,
    globs: GlobSet,
    block_of_glob: Vec<usize>,
    root: String,
}

pub(crate) struct FileSeverities<'s> {
    of: &'s EffectiveSeverities<'s>,
    blocks: Vec<usize>,
}

impl<'c> EffectiveSeverities<'c> {
    pub(crate) fn new(config: &'c Config, project_root: &Utf8Path) -> Result<Self, ConfigError> {
        let mut builder = GlobSetBuilder::new();
        let mut block_of_glob = Vec::new();
        for (block, o) in config.overrides.iter().enumerate() {
            for pattern in &o.files {
                let glob = Glob::new(pattern).map_err(|e| ConfigError::BadGlob {
                    pattern: pattern.clone(),
                    source: e,
                })?;
                builder.add(glob);
                block_of_glob.push(block);
            }
        }
        let globs = builder.build().map_err(|e| ConfigError::BadGlob {
            pattern: config
                .overrides
                .iter()
                .flat_map(|o| o.files.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(","),
            source: e,
        })?;
        let root = project_root.as_str();
        let root = root.strip_prefix(r"\\?\").unwrap_or(root);
        Ok(Self {
            config,
            globs,
            block_of_glob,
            root: slashed(Path::new(root)).trim_end_matches('/').to_owned(),
        })
    }

    pub(crate) fn for_file(&self, file: &Path) -> FileSeverities<'_> {
        self.for_slashed(&slashed(file))
    }

    pub(crate) fn for_slashed(&self, file: &str) -> FileSeverities<'_> {
        self.for_relative(under(file, &self.root).unwrap_or(file))
    }

    pub(crate) fn for_project(&self) -> FileSeverities<'_> {
        self.for_relative("<project>")
    }

    fn for_relative(&self, rel: &str) -> FileSeverities<'_> {
        let mut blocks: Vec<usize> = self
            .globs
            .matches(rel)
            .into_iter()
            .map(|g| self.block_of_glob[g])
            .collect();
        blocks.sort_unstable();
        blocks.dedup();
        FileSeverities { of: self, blocks }
    }

    pub(crate) fn off_everywhere(&self, rule: &dyn DynRule) -> bool {
        let slug = rule.slug();
        self.base(rule) == Severity::Off
            && self
                .config
                .overrides
                .iter()
                .filter_map(|block| block.rules.get(slug))
                .all(|r| r.severity == Severity::Off)
    }

    fn base(&self, rule: &dyn DynRule) -> Severity {
        self.config
            .rules
            .get(rule.slug())
            .map_or_else(|| rule.default_severity(), |rc| rc.severity)
    }
}

impl FileSeverities<'_> {
    pub(crate) fn of(&self, rule: &dyn DynRule) -> Severity {
        let overrides = &self.of.config.overrides;
        self.blocks
            .iter()
            .rev()
            .find_map(|&b| overrides[b].rules.get(rule.slug()))
            .map_or_else(|| self.of.base(rule), |r| r.severity)
    }
}

pub(crate) fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn under<'p>(path: &'p str, dir: &str) -> Option<&'p str> {
    if dir.is_empty() {
        return None;
    }
    let rest = path.strip_prefix(dir)?;
    if rest.is_empty() {
        Some(rest)
    } else {
        rest.strip_prefix('/')
    }
}
