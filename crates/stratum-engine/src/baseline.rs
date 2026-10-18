use std::collections::{BTreeMap, BTreeSet};

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
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    touches: BTreeSet<String>,
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
pub struct Baseline(BTreeMap<Fingerprint, Accepted>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Accepted {
    count: usize,
    touches: BTreeSet<String>,
}

impl Accepted {
    fn add(&mut self, count: usize, touches: impl IntoIterator<Item = String>) {
        self.count += count;
        self.touches.extend(touches);
    }
}

fn touched_besides_anchor(f: &Finding) -> impl Iterator<Item = String> + '_ {
    f.edge
        .iter()
        .flat_map(|e| [&e.from, &e.to])
        .chain(&f.members)
        .filter(|file| **file != f.file)
        .cloned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Growth {
    pub fingerprint: Fingerprint,
    pub base: usize,
    pub head: usize,
}

#[derive(Debug, Default)]
pub struct Matched {
    pub new: Vec<Finding>,
    pub baselined: Vec<Finding>,
    fixed: Vec<(Vec<String>, usize)>,
}

impl Matched {
    #[must_use]
    pub fn fixed(&self) -> usize {
        self.fixed.iter().map(|(_, n)| n).sum()
    }

    /// Fixed entries whose anchor file, edge endpoints or members `in_scope` accepts.
    #[must_use]
    pub fn fixed_where(&self, in_scope: impl Fn(&str) -> bool) -> usize {
        self.fixed
            .iter()
            .filter(|(files, _)| files.iter().any(|file| in_scope(file)))
            .map(|(_, n)| n)
            .sum()
    }
}

impl Baseline {
    #[must_use]
    pub fn from_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> Self {
        let mut map = BTreeMap::<_, Accepted>::new();
        for f in findings {
            map.entry(Fingerprint::of(f))
                .or_default()
                .add(1, touched_besides_anchor(f));
        }
        Self(map)
    }

    pub fn load(path: &Utf8Path) -> Result<Self, BaselineError> {
        let text = std::fs::read_to_string(path).map_err(|source| BaselineError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text, path)
    }

    pub fn parse(text: &str, origin: &Utf8Path) -> Result<Self, BaselineError> {
        let file: File = serde_json::from_str(text).map_err(|source| BaselineError::Invalid {
            path: origin.to_path_buf(),
            source,
        })?;
        if file.version != VERSION {
            return Err(BaselineError::Version {
                path: origin.to_path_buf(),
                found: file.version,
            });
        }
        let mut map = BTreeMap::<_, Accepted>::new();
        for mut e in file.violations {
            e.fingerprint.message = without_numbers(&e.fingerprint.message);
            map.entry(e.fingerprint)
                .or_default()
                .add(e.count, e.touches);
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
                .map(|(fingerprint, accepted)| Entry {
                    fingerprint: fingerprint.clone(),
                    count: accepted.count,
                    touches: accepted.touches.clone(),
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
        self.0.values().map(|a| a.count).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn count(&self, fingerprint: &Fingerprint) -> usize {
        self.0.get(fingerprint).map_or(0, |a| a.count)
    }

    /// Each entry absorbs at most its `count` findings; `fixed` counts the unabsorbed rest.
    #[must_use]
    pub fn apply(&self, findings: Vec<Finding>) -> Matched {
        let mut budget: BTreeMap<&Fingerprint, usize> =
            self.0.iter().map(|(fp, a)| (fp, a.count)).collect();
        let mut matched = Matched::default();
        for f in findings {
            match budget.get_mut(&Fingerprint::of(&f)) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    matched.baselined.push(f);
                }
                _ => matched.new.push(f),
            }
        }
        matched.fixed = budget
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(fp, n)| {
                let touches = self.0.get(fp).map(|a| &a.touches).into_iter().flatten();
                let files = std::iter::once(&fp.file).chain(touches).cloned().collect();
                (files, n)
            })
            .collect();
        matched
    }

    #[must_use]
    pub fn grown_since(&self, base: &Self) -> Vec<Growth> {
        self.0
            .iter()
            .filter_map(|(fingerprint, accepted)| {
                let (head, base) = (accepted.count, base.count(fingerprint));
                (head > base).then(|| Growth {
                    fingerprint: fingerprint.clone(),
                    base,
                    head,
                })
            })
            .collect()
    }

    /// Keeps only entries still produced by `findings`; never adds new ones.
    #[must_use]
    pub fn pruned(&self, findings: &[Finding]) -> Self {
        let current = Self::from_findings(findings);
        Self(
            self.0
                .iter()
                .filter_map(|(fp, accepted)| {
                    let count = accepted.count.min(current.count(fp));
                    (count > 0).then(|| {
                        let touches = accepted.touches.clone();
                        (fp.clone(), Accepted { count, touches })
                    })
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
            fix: "fix".into(),
            members: Vec::new(),
        }
    }

    #[test]
    fn a_second_identical_violation_is_new_and_line_shifts_stay_baselined() {
        let base = Baseline::from_findings(&[finding("a.ts", 3)]);
        let m = base.apply(vec![finding("a.ts", 40), finding("a.ts", 41)]);
        assert_eq!(
            (m.baselined[0].location.line, m.new[0].location.line),
            (40, 41)
        );
        assert_eq!(m.fixed(), 0);
    }

    #[test]
    fn a_changed_metric_in_the_message_stays_baselined() {
        let with_message = |message: &str| Finding {
            message: message.into(),
            ..finding("a.ts", 1)
        };
        let base =
            Baseline::from_findings(&[with_message("Depth ratio 0.33 below 3.0 (limit 5).")]);
        let m = base.apply(vec![with_message("Depth ratio 0.25 below 3.0 (limit 5).")]);
        assert_eq!((m.baselined.len(), m.new.len()), (1, 0));
        assert_eq!(m.fixed(), 0);
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
        let m = Baseline::load(&path).unwrap().apply(vec![Finding {
            message: "Depth ratio 0.25 below 3.0 (limit 5).".into(),
            ..finding("a.ts", 1)
        }]);
        assert_eq!((m.baselined.len(), m.new.len()), (1, 0));
        assert_eq!(m.fixed(), 0);
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
