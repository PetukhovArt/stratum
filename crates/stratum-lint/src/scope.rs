use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};

use stratum_engine::{Finding, paths};

use crate::{cwd::absolutize, git};

#[derive(Debug, Clone, Default)]
pub struct Scope {
    files: BTreeSet<String>,
    /// The `--changed` paths alone, without `--files`; `None` without `--changed`.
    changed: Option<Vec<String>>,
}

impl Scope {
    /// `None` when neither `files` nor `changed` restricts the report.
    pub fn resolve(
        root_abs: &Utf8Path,
        files: &[Utf8PathBuf],
        changed: Option<&str>,
    ) -> miette::Result<Option<Self>> {
        if files.is_empty() && changed.is_none() {
            return Ok(None);
        }
        let mut set: BTreeSet<String> = files
            .iter()
            .map(|f| {
                [absolutize(f)?, absolutize(&root_abs.join(f))?]
                    .into_iter()
                    .find(|abs| abs.exists())
                    .map(|abs| paths::relative(abs.as_std_path(), root_abs))
                    .ok_or_else(|| {
                        miette::miette!("--files: {f} not found in the current directory or ROOT")
                    })
            })
            .collect::<miette::Result<_>>()?;
        let changed = changed
            .map(|reference| git::changed_files(root_abs, reference))
            .transpose()?;
        set.extend(changed.iter().flatten().cloned());
        Ok(Some(Self {
            files: set,
            changed,
        }))
    }

    /// The files `--changed` found, for the diff-mode rules; `None` without `--changed`.
    #[must_use]
    pub fn changed(&self) -> Option<&[String]> {
        self.changed.as_deref()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(String::as_str)
    }

    #[must_use]
    pub fn touches(&self, f: &Finding) -> bool {
        self.files.contains(&f.file)
            || f.edge
                .as_ref()
                .is_some_and(|e| self.files.contains(&e.from) || self.files.contains(&e.to))
            || f.members.iter().any(|m| self.files.contains(m))
    }
}
