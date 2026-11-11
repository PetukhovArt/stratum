use std::collections::BTreeMap;

use globset::GlobSet;
use serde::Deserialize;
use stratum_core::{ids::RuleId, severity::Severity, violation::Violation};
use stratum_graph::CompoundGraph;

use crate::builtin::import_policy::{glob_set, slice_dir};
use crate::ids::CO_CHANGE;
use crate::rule::Rule;
use crate::scope::ProjectScope;

pub const CO_CHANGE_SLUG: &str = "stratum/co-change";
pub const DEFAULT_MESSAGE: &str = "Files that change together changed alone";

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
    slice: Option<String>,
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
    slice: Option<GlobSet>,
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
                    slice: spec.slice.map(|p| glob_set(&[p])).transpose()?,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
}

impl CoChangeOptions {
    /// `changed`: paths relative to the project root with `/`.
    #[must_use]
    pub fn misses(&self, changed: &[String]) -> Vec<CoChangeMiss> {
        let mut changed: Vec<&str> = changed.iter().map(String::as_str).collect();
        changed.sort_unstable();
        changed.dedup();
        self.policies
            .iter()
            .flat_map(|p| p.misses(&changed))
            .collect()
    }
}

impl CoChangePolicy {
    fn misses(&self, changed: &[&str]) -> Vec<CoChangeMiss> {
        let mut by_slice: BTreeMap<Option<&str>, Vec<String>> = BTreeMap::new();
        for file in changed.iter().filter(|f| self.when.is_match(f)) {
            let slice = self.slice.as_ref().and_then(|s| slice_dir(s, file));
            by_slice.entry(slice).or_default().push((*file).to_owned());
        }
        by_slice
            .into_iter()
            .filter(|(slice, _)| {
                !changed.iter().any(|f| {
                    self.require.is_match(f)
                        && slice.is_none_or(|dir| {
                            f.strip_prefix(dir)
                                .is_some_and(|rest| rest.starts_with('/'))
                        })
                })
            })
            .map(|(slice, when)| CoChangeMiss {
                message: self.message.clone(),
                fix: self.fix.clone(),
                when,
                require: self.require_patterns.clone(),
                slice: slice.map(str::to_owned),
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

    #[test]
    fn with_a_slice_each_slice_directory_needs_its_own_require_file() {
        let options: CoChangeOptions = serde_json::from_value(serde_json::json!([{
            "slice": "src/domain/*",
            "when": ["src/**/yaml-dto.ts"],
            "require": ["src/domain/*/migrations/**", "src/legacy/migrations/**"],
        }]))
        .unwrap();
        let changed =
            |files: &[&str]| -> Vec<String> { files.iter().map(|f| (*f).to_owned()).collect() };
        let missed = options.misses(&changed(&[
            "src/domain/connections/yaml-dto.ts",
            "src/domain/scenes/yaml-dto.ts",
            "src/domain/scenes/migrations/x.ts",
        ]));
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0].slice.as_deref(), Some("src/domain/connections"));
        assert_eq!(missed[0].when, ["src/domain/connections/yaml-dto.ts"]);
        assert_eq!(
            options.misses(&changed(&[
                "src/domain/connections/yaml-dto.ts",
                "src/domain/connections/migrations/1.ts",
            ])),
            []
        );

        let outside = options.misses(&changed(&["src/legacy/yaml-dto.ts"]));
        assert_eq!(outside.len(), 1);
        assert_eq!(outside[0].slice, None);
        assert_eq!(
            options.misses(&changed(&[
                "src/legacy/yaml-dto.ts",
                "src/domain/scenes/migrations/x.ts",
            ])),
            [],
            "outside every slice directory, any changed require file satisfies the policy"
        );
    }
}
