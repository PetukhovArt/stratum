use globset::GlobSet;
use serde::Deserialize;
use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::builtin::import_policy::glob_set;
use crate::ids::CO_CHANGE;
use crate::rule::Rule;
use crate::scope::ProjectScope;

pub const CO_CHANGE_SLUG: &str = "stratum/co-change";
const DEFAULT_MESSAGE: &str = "Files that change together changed alone";

/// A diff-mode rule: the engine runs it on the changed files of `lint --changed` and
/// `diff --base`. On the graph alone it reports nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct CoChange;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(try_from = "Vec<CoChangeSpec>")]
pub struct CoChangeOptions {
    policies: Vec<CoChangePolicy>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoChangeSpec {
    when: Vec<String>,
    require: Vec<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Debug, Clone)]
struct CoChangePolicy {
    when: GlobSet,
    require: GlobSet,
    require_patterns: Vec<String>,
    message: String,
    fix: Option<String>,
}

impl TryFrom<Vec<CoChangeSpec>> for CoChangeOptions {
    type Error = String;

    fn try_from(specs: Vec<CoChangeSpec>) -> Result<Self, String> {
        let policies = specs
            .into_iter()
            .map(|spec| {
                if spec.require.is_empty() {
                    return Err("a co-change policy needs at least one `require` glob".to_owned());
                }
                Ok(CoChangePolicy {
                    when: glob_set(&spec.when)?,
                    require: glob_set(&spec.require)?,
                    require_patterns: spec.require,
                    message: spec.message.unwrap_or_else(|| DEFAULT_MESSAGE.into()),
                    fix: spec.fix,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { policies })
    }
}

/// A policy the change breaks: files matching `when` changed, none matching `require` did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CoChangeMiss {
    pub message: String,
    pub fix: Option<String>,
    /// The changed files matching `when`, sorted.
    pub when: Vec<String>,
    /// The `require` globs as written.
    pub require: Vec<String>,
}

impl CoChangeOptions {
    /// `changed`: paths relative to the project root with `/`.
    #[must_use]
    pub fn misses(&self, changed: &[String]) -> Vec<CoChangeMiss> {
        self.policies
            .iter()
            .filter(|p| !changed.iter().any(|f| p.require.is_match(f)))
            .filter_map(|p| {
                let mut when: Vec<String> = changed
                    .iter()
                    .filter(|f| p.when.is_match(f.as_str()))
                    .cloned()
                    .collect();
                if when.is_empty() {
                    return None;
                }
                when.sort();
                when.dedup();
                Some(CoChangeMiss {
                    message: p.message.clone(),
                    fix: p.fix.clone(),
                    when,
                    require: p.require_patterns.clone(),
                })
            })
            .collect()
    }
}

impl Rule for CoChange {
    type Scope = ProjectScope;
    type Options = CoChangeOptions;

    fn id(&self) -> RuleId {
        CO_CHANGE
    }
    fn slug(&self) -> &'static str {
        CO_CHANGE_SLUG
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(
        &self,
        _graph: &CompoundGraph,
        _scope: ProjectScope,
        _options: &CoChangeOptions,
        _severity: Severity,
    ) -> Vec<Violation> {
        vec![]
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_policy_is_missed_when_a_when_file_changed_and_no_require_file_did() {
        let options: CoChangeOptions = serde_json::from_value(serde_json::json!([
            { "when": ["src/api/**"], "require": ["docs/api/**", "CHANGELOG.md"] }
        ]))
        .unwrap();
        let changed =
            |files: &[&str]| -> Vec<String> { files.iter().map(|f| (*f).to_owned()).collect() };
        assert_eq!(options.misses(&changed(&["src/app.ts"])), []);
        assert_eq!(
            options.misses(&changed(&["src/api/b.ts", "src/api/a.ts", "CHANGELOG.md"])),
            []
        );
        let missed = options.misses(&changed(&["src/api/b.ts", "src/api/a.ts"]));
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0].when, ["src/api/a.ts", "src/api/b.ts"]);
        assert!(
            serde_json::from_value::<CoChangeOptions>(serde_json::json!([
                { "when": ["a"], "require": [] }
            ]))
            .is_err()
        );
    }
}
