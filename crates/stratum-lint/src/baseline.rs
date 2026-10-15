use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use crate::report::Finding;

pub const FILE_NAME: &str = "stratum.baseline.json";
const VERSION: u32 = 1;

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
            message: f.message.clone(),
        }
    }
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

    pub fn load(path: &Utf8Path) -> miette::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| miette::miette!("Could not read baseline {path}: {e}"))?;
        let file: File = serde_json::from_str(&text)
            .map_err(|e| miette::miette!("Invalid baseline {path}: {e}"))?;
        if file.version != VERSION {
            return Err(miette::miette!(
                "Unsupported baseline version {} in {path} (expected {VERSION})",
                file.version
            ));
        }
        let mut map = BTreeMap::new();
        for e in file.violations {
            *map.entry(e.fingerprint).or_insert(0) += e.count;
        }
        Ok(Self(map))
    }

    pub fn save(&self, path: &Utf8Path) -> miette::Result<()> {
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
        let mut json = serde_json::to_string_pretty(&file).map_err(|e| miette::miette!("{e}"))?;
        json.push('\n');
        std::fs::write(path, json).map_err(|e| miette::miette!("Could not write {path}: {e}"))
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
    use crate::report::Location;
    use stratum_core::severity::Severity;

    fn finding(file: &str, line: u32) -> Finding {
        Finding {
            rule: "stratum/no-circular-deps".into(),
            severity: Severity::Error,
            message: "cycle".into(),
            file: file.into(),
            location: Location { line, column: 1 },
            edge: None,
            suggestion: None,
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
    fn prune_never_grows_the_baseline() {
        let base = Baseline::from_findings(&[finding("a.ts", 1), finding("b.ts", 1)]);
        let pruned = base.pruned(&[finding("a.ts", 1), finding("a.ts", 2), finding("c.ts", 1)]);
        assert_eq!(pruned, Baseline::from_findings(&[finding("a.ts", 1)]));
    }
}
