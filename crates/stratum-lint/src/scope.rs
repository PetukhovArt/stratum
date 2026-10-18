use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};

use stratum_engine::{Finding, paths};

use crate::{cwd::absolutize, git};

#[derive(Debug, Clone, Default)]
pub struct Scope(BTreeSet<String>);

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
        if let Some(reference) = changed {
            set.extend(git::changed_files(root_abs, reference)?);
        }
        Ok(Some(Self(set)))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn covers(&self, file: &str) -> bool {
        self.0.contains(file)
    }

    #[must_use]
    pub fn touches(&self, f: &Finding) -> bool {
        self.0.contains(&f.file)
            || f.edge
                .as_ref()
                .is_some_and(|e| self.0.contains(&e.from) || self.0.contains(&e.to))
            || f.members.iter().any(|m| self.0.contains(m))
    }
}
