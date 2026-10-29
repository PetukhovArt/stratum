use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Config, ConfigError};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SystemConfig {
    /// `^[\w.-]+$`; source paths of the system are addressed as `id:rel/path`.
    pub id: String,
    /// Shown instead of the id; defaults to the id.
    #[serde(default)]
    pub name: Option<String>,
    /// The system this project is: its modules are the callers. Exactly one system has it.
    #[serde(default, rename = "self")]
    pub is_self: bool,
    /// Directory relative to the project root where `contracts` are read from.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub root: Option<Utf8PathBuf>,
    #[serde(default)]
    pub contracts: Option<ContractSource>,
    /// How a call in this project names a contract of this system.
    #[serde(default)]
    pub calls: Vec<CallMatcher>,
}

impl SystemConfig {
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ContractSource {
    /// Each `rpc M(…) returns (…)` of a `service S` in a `*.proto` file is the contract `S.M`.
    Proto,
    /// Each `fn` of a `*.rs` file under `#[attr]` or `#[attr(…)]` is a contract named after the
    /// function; `attr` is a path such as `tauri::command`, and its last segment matches too.
    RustAttr { attr: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum CallMatcher {
    /// `callee(…)` whose argument at `name_arg` (from 0) is a string literal naming the
    /// contract. `callee` is `name`, `obj.name` or `*.name`, as in `stratum/restricted-calls`.
    Callee { callee: String, name_arg: usize },
    /// `const c = factory(…)` whose argument at `service_arg` (from 0) is an identifier naming
    /// the service, then `c.method(…)` in the same module: the contract `Service.method`.
    Factory { factory: String, service_arg: usize },
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

fn is_callee(entry: &str) -> bool {
    match entry.split_once('.') {
        None => is_identifier(entry),
        Some((object, name)) => (object == "*" || is_identifier(object)) && is_identifier(name),
    }
}

fn is_system_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub(crate) fn validate(path: &Utf8Path, config: &Config) -> Result<(), ConfigError> {
    let invalid = |message: String| ConfigError::Validation {
        path: path.to_path_buf(),
        message,
    };
    if config.systems.is_empty() {
        return Ok(());
    }
    let selves = config.systems.iter().filter(|s| s.is_self).count();
    if selves != 1 {
        return Err(invalid(format!(
            "`systems` needs exactly one system with `\"self\": true`, found {selves}"
        )));
    }
    let mut ids = BTreeSet::new();
    for system in &config.systems {
        let id = &system.id;
        if !is_system_id(id) {
            return Err(invalid(format!(
                "system id {id:?} must match ^[\\w.-]+$: ASCII letters, digits, `_`, `.`, `-`"
            )));
        }
        if !ids.insert(id.as_str()) {
            return Err(invalid(format!("system id {id:?} is declared twice")));
        }
        if system.is_self
            && (system.root.is_some() || system.contracts.is_some() || !system.calls.is_empty())
        {
            return Err(invalid(format!(
                "system {id:?} is `self`: its contracts are the project's own, so it takes no `root`, `contracts` or `calls`"
            )));
        }
        if system.contracts.is_some() != system.root.is_some() {
            return Err(invalid(format!(
                "system {id:?} needs both `root` and `contracts`, or neither"
            )));
        }
        if let Some(ContractSource::RustAttr { attr }) = &system.contracts {
            if !attr.split("::").all(is_identifier) {
                return Err(invalid(format!(
                    "system {id:?}: `attr` {attr:?} must be a path such as `tauri::command`"
                )));
            }
        }
        for matcher in &system.calls {
            let (entry, valid) = match matcher {
                CallMatcher::Callee { callee, .. } => (callee, is_callee(callee)),
                CallMatcher::Factory { factory, .. } => (factory, is_identifier(factory)),
            };
            if !valid {
                return Err(invalid(format!(
                    "system {id:?}: call entry {entry:?} must be `name`, `obj.name` or `*.name` (a factory: `name`)"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use camino::Utf8PathBuf;

    use super::{CallMatcher, ContractSource};
    use crate::{ConfigError, parse_str};

    fn parse(systems: &str) -> Result<crate::Config, ConfigError> {
        let text = format!(r#"{{ "version": 1, "systems": {systems} }}"#);
        parse_str(&Utf8PathBuf::from("inline"), &text)
    }

    #[test]
    fn systems_parse_with_their_contract_source_and_call_matchers() {
        let config = parse(
            r#"[{ "id": "ui", "self": true },
                { "id": "host", "name": "Desktop host", "root": "../host/src",
                  "contracts": { "kind": "rust-attr", "attr": "tauri::command" },
                  "calls": [{ "callee": "invoke", "name_arg": 0 }] },
                { "id": "api", "root": "../proto", "contracts": { "kind": "proto" },
                  "calls": [{ "factory": "createClient", "service_arg": 0 }] }]"#,
        )
        .unwrap();
        let [ui, host, api] = config.systems.as_slice() else {
            unreachable!("three systems");
        };
        assert!(ui.is_self);
        assert_eq!(host.display_name(), "Desktop host");
        assert_eq!(
            host.contracts,
            Some(ContractSource::RustAttr {
                attr: "tauri::command".into()
            })
        );
        assert_eq!(
            api.calls,
            [CallMatcher::Factory {
                factory: "createClient".into(),
                service_arg: 0
            }]
        );
    }

    #[test]
    fn a_bad_systems_list_names_what_is_wrong() {
        let ui = r#"{ "id": "ui", "self": true }"#;
        let api = r#"{ "id": "api", "root": "p", "contracts": { "kind": "proto" } }"#;
        for (systems, expected) in [
            (vec![api], "exactly one"),
            (
                vec![
                    r#"{ "id": "a", "self": true }"#,
                    r#"{ "id": "b", "self": true }"#,
                ],
                "found 2",
            ),
            (vec![ui, api, api], "declared twice"),
            (vec![r#"{ "id": "u:i", "self": true }"#], "u:i"),
            (
                vec![r#"{ "id": "ui", "self": true, "root": "x" }"#],
                "is `self`",
            ),
            (
                vec![ui, r#"{ "id": "api", "root": "p" }"#],
                "both `root` and `contracts`",
            ),
            (
                vec![
                    ui,
                    r#"{ "id": "h", "calls": [{ "callee": "a.b.c", "name_arg": 0 }] }"#,
                ],
                "a.b.c",
            ),
            (
                vec![ui, r#"{ "id": "h", "calls": [{ "callee": "x" }] }"#],
                "untagged",
            ),
            (
                vec![
                    ui,
                    r#"{ "id": "h", "root": "p", "contracts": { "kind": "openapi" } }"#,
                ],
                "openapi",
            ),
        ] {
            let systems = format!("[{}]", systems.join(", "));
            let err = parse(&systems).unwrap_err();
            assert!(err.to_string().contains(expected), "{systems}: {err}");
        }
    }
}
