use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::finding::Finding;

pub const BASELINE_FILE: &str = "stratum.baseline.json";
const VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("Could not read baseline {path}: {source}")]
    Read {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid baseline {path}: {source}")]
    Invalid {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Unsupported baseline version {found} in {path} (expected {VERSION})")]
    Version { path: Utf8PathBuf, found: u32 },
    #[error("Could not write {path}: {source}")]
    Write {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fingerprint {
    pub rule: String,
    pub file: String,
    pub message: String,
}

impl Fingerprint {
    #[must_use]
    pub fn of(f: &Finding) -> Self {
        Self {
            rule: f.rule.clone(),
            file: f.file.clone(),
            message: without_numbers(&f.message),
        }
    }
}

fn without_numbers(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_digit() {
            out.push(c);
            continue;
        }
        out.push('#');
        while let Some(&next) = chars.peek() {
            let decimal_point =
                next == '.' && chars.clone().nth(1).is_some_and(|d| d.is_ascii_digit());
            if !next.is_ascii_digit() && !decimal_point {
                break;
            }
            chars.next();
        }
    }
    out
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    #[serde(flatten)]
    fingerprint: Fingerprint,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    count: usize,
}

const fn one() -> usize {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_one(n: &usize) -> bool {
    *n == 1
}

#[derive(Debug, Serialize, Deserialize)]
struct File {
    version: u32,
    violations: Vec<Entry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Baseline(BTreeMap<Fingerprint, usize>);

#[derive(Debug, Default)]
pub struct Matched {
    pub baselined: Vec<bool>,
    pub fixed: usize,
}

impl Baseline {
    #[must_use]
    pub fn from_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> Self {
        let mut map = BTreeMap::new();
        for f in findings {
            *map.entry(Fingerprint::of(f)).or_insert(0) += 1;
        }
        Self(map)
    }

    pub fn load(path: &Utf8Path) -> Result<Self, BaselineError> {
        let text = std::fs::read_to_string(path).map_err(|source| BaselineError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let file: File = serde_json::from_str(&text).map_err(|source| BaselineError::Invalid {
            path: path.to_path_buf(),
            source,
        })?;
        if file.version != VERSION {
            return Err(BaselineError::Version {
                path: path.to_path_buf(),
                found: file.version,
            });
        }
        let mut map = BTreeMap::new();
        for mut e in file.violations {
            e.fingerprint.message = without_numbers(&e.fingerprint.message);
            *map.entry(e.fingerprint).or_insert(0) += e.count;
        }
        Ok(Self(map))
    }

    pub fn load_if_exists(path: &Utf8Path) -> Result<Option<Self>, BaselineError> {
        if path.exists() {
            Self::load(path).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn save(&self, path: &Utf8Path) -> Result<(), BaselineError> {
        let file = File {
            version: VERSION,
            violations: self
                .0
                .iter()
                .map(|(fingerprint, &count)| Entry {
                    fingerprint: fingerprint.clone(),
                    count,
                })
                .collect(),
        };
        let mut json =
            serde_json::to_string_pretty(&file).map_err(|source| BaselineError::Invalid {
                path: path.to_path_buf(),
                source,
            })?;
        json.push('\n');
        std::fs::write(path, json).map_err(|source| BaselineError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each entry absorbs at most its `count` findings; `fixed` counts the unabsorbed rest.
    #[must_use]
    pub fn matches(&self, findings: &[Finding]) -> Matched {
        let mut budget = self.0.clone();
        let baselined = findings
            .iter()
            .map(|f| match budget.get_mut(&Fingerprint::of(f)) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    true
                }
                _ => false,
            })
            .collect();
        Matched {
            baselined,
            fixed: budget.values().sum(),
        }
    }

    /// Keeps only entries still produced by `findings`; never adds new ones.
    #[must_use]
    pub fn pruned(&self, findings: &[Finding]) -> Self {
        let current = Self::from_findings(findings);
        Self(
            self.0
                .iter()
                .filter_map(|(fp, &n)| {
                    let keep = n.min(current.0.get(fp).copied().unwrap_or(0));
                    (keep > 0).then(|| (fp.clone(), keep))
                })
                .collect(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::severity::Severity;
    use stratum_core::violation::SourceLocation;

    fn finding(file: &str, line: u32) -> Finding {
        Finding {
            rule: "stratum/no-circular-deps".into(),
            severity: Severity::Error,
            message: "cycle".into(),
            file: file.into(),
            location: SourceLocation { line, column: 1 },
            edge: None,
            suggestion: None,
            members: Vec::new(),
        }
    }

    #[test]
    fn a_second_identical_violation_is_new_and_line_shifts_stay_baselined() {
        let base = Baseline::from_findings(&[finding("a.ts", 3)]);
        let m = base.matches(&[finding("a.ts", 40), finding("a.ts", 41)]);
        assert_eq!(m.baselined, vec![true, false]);
        assert_eq!(m.fixed, 0);
    }

    #[test]
    fn a_changed_metric_in_the_message_stays_baselined() {
        let with_message = |message: &str| Finding {
            message: message.into(),
            ..finding("a.ts", 1)
        };
        let base =
            Baseline::from_findings(&[with_message("Depth ratio 0.33 below 3.0 (limit 5).")]);
        let m = base.matches(&[with_message("Depth ratio 0.25 below 3.0 (limit 5).")]);
        assert_eq!(m.baselined, vec![true]);
        assert_eq!(m.fixed, 0);
    }

    fn temp_baseline_path(dir: &tempfile::TempDir) -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(dir.path().join(BASELINE_FILE)).unwrap()
    }

    #[test]
    fn a_baseline_file_with_raw_numbers_still_matches_after_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        let entry = serde_json::json!({
            "version": 1,
            "violations": [{
                "rule": "stratum/no-circular-deps",
                "file": "a.ts",
                "message": "Depth ratio 0.33 below 3.0 (limit 5).",
            }],
        });
        std::fs::write(&path, entry.to_string()).unwrap();
        let m = Baseline::load(&path).unwrap().matches(&[Finding {
            message: "Depth ratio 0.25 below 3.0 (limit 5).".into(),
            ..finding("a.ts", 1)
        }]);
        assert_eq!(m.baselined, vec![true]);
        assert_eq!(m.fixed, 0);
    }

    #[test]
    fn save_then_load_keeps_single_and_repeated_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        let base =
            Baseline::from_findings(&[finding("a.ts", 1), finding("a.ts", 2), finding("b.ts", 1)]);
        base.save(&path).unwrap();
        assert_eq!(Baseline::load(&path).unwrap(), base);
    }

    #[test]
    fn prune_never_grows_the_baseline() {
        let base = Baseline::from_findings(&[finding("a.ts", 1), finding("b.ts", 1)]);
        let pruned = base.pruned(&[finding("a.ts", 1), finding("a.ts", 2), finding("c.ts", 1)]);
        assert_eq!(pruned, Baseline::from_findings(&[finding("a.ts", 1)]));
    }
}
