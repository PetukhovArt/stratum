use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobMatcher};
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
    /// Outside the product: external infrastructure such as a database or a device. Not with
    /// `self`.
    #[serde(default)]
    pub external: bool,
    #[serde(default)]
    pub description: Option<String>,
    /// Globs relative to the project root of the modules that talk to this system without
    /// contract calls, such as a connection driver. Not with `self`.
    #[serde(default)]
    pub via: Vec<String>,
    /// Declared relations from this system to other systems.
    #[serde(default)]
    pub uses: Vec<UseConfig>,
}

impl SystemConfig {
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Each `via` glob as written, with its matcher.
    ///
    /// # Errors
    /// [`ConfigError::BadGlob`] for an entry that is not a valid glob.
    pub fn via_globs(&self) -> Result<Vec<(&str, GlobMatcher)>, ConfigError> {
        self.via
            .iter()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(|g| (pattern.as_str(), g.compile_matcher()))
                    .map_err(|source| ConfigError::BadGlob {
                        pattern: pattern.clone(),
                        source,
                    })
            })
            .collect()
    }
}

/// A role of a person who uses the systems.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorConfig {
    /// `^[\w.-]+$`, unique across `actors` and `systems`.
    pub id: String,
    /// Shown instead of the id; defaults to the id.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// The systems this role uses.
    #[serde(default)]
    pub uses: Vec<UseConfig>,
}

impl ActorConfig {
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UseConfig {
    /// The id of a system in `systems`.
    pub to: String,
    /// What the relation does, such as `reads signals`.
    #[serde(default)]
    pub label: Option<String>,
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
        if config.actors.is_empty() {
            return Ok(());
        }
        return Err(invalid(
            "`actors` need `systems` to use: declare the systems first".to_owned(),
        ));
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
        if system.is_self && system.external {
            return Err(invalid(format!(
                "system {id:?} is `self`, so it cannot be `external`"
            )));
        }
        if system.is_self && !system.via.is_empty() {
            return Err(invalid(format!(
                "system {id:?} is `self`: `via` names the project's modules that talk to another system, so it goes on that system"
            )));
        }
        system.via_globs()?;
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
    for actor in &config.actors {
        let id = &actor.id;
        if !is_system_id(id) {
            return Err(invalid(format!(
                "actor id {id:?} must match ^[\\w.-]+$: ASCII letters, digits, `_`, `.`, `-`"
            )));
        }
        if !ids.insert(id.as_str()) {
            return Err(invalid(format!(
                "actor id {id:?} is declared twice, or is also a system id"
            )));
        }
    }
    let systems: BTreeSet<&str> = config.systems.iter().map(|s| s.id.as_str()).collect();
    let uses = config
        .actors
        .iter()
        .map(|a| ("actor", &a.id, &a.uses))
        .chain(config.systems.iter().map(|s| ("system", &s.id, &s.uses)));
    for (kind, id, uses) in uses {
        if let Some(unknown) = uses.iter().find(|u| !systems.contains(u.to.as_str())) {
            let to = &unknown.to;
            return Err(invalid(format!(
                "{kind} {id:?} uses {to:?}, which is not a system id; systems: {}",
                systems.iter().copied().collect::<Vec<_>>().join(", ")
            )));
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

    fn parse_with_actors(systems: &str, actors: &str) -> Result<crate::Config, ConfigError> {
        let text = format!(r#"{{ "version": 1, "systems": {systems}, "actors": {actors} }}"#);
        parse_str(&Utf8PathBuf::from("inline"), &text)
    }

    #[test]
    fn actors_and_external_systems_parse_with_their_relations() {
        let config = parse_with_actors(
            r#"[{ "id": "ui", "self": true, "uses": [{ "to": "billing" }] },
                { "id": "billing", "description": "Invoices",
                  "uses": [{ "to": "opc-gateway", "label": "reads signals" }] },
                { "id": "opc-gateway", "external": true, "via": ["src/shared/opc/**"] }]"#,
            r#"[{ "id": "operator", "name": "Operator",
                  "uses": [{ "to": "ui", "label": "watches the plant" }] }]"#,
        )
        .unwrap();
        let [operator] = config.actors.as_slice() else {
            unreachable!("one actor");
        };
        assert_eq!(operator.display_name(), "Operator");
        assert_eq!(operator.uses[0].label.as_deref(), Some("watches the plant"));
        let [ui, billing, gateway] = config.systems.as_slice() else {
            unreachable!("three systems");
        };
        assert_eq!(ui.uses[0].to, "billing");
        assert_eq!(ui.uses[0].label, None);
        assert_eq!(billing.description.as_deref(), Some("Invoices"));
        assert!(!billing.external);
        assert!(gateway.external);
        assert_eq!(gateway.via, ["src/shared/opc/**"]);
    }

    #[test]
    fn a_bad_actor_or_relation_names_what_is_wrong() {
        let ui = r#"{ "id": "ui", "self": true }"#;
        for (systems, actors, expected) in [
            (
                vec![r#"{ "id": "ui", "self": true, "external": true }"#],
                "[]",
                "cannot be `external`",
            ),
            (
                vec![r#"{ "id": "ui", "self": true, "via": ["src/**"] }"#],
                "[]",
                "`via`",
            ),
            (
                vec![ui, r#"{ "id": "db", "via": ["src/[a"] }"#],
                "[]",
                "src/[a",
            ),
            (
                vec![ui, r#"{ "id": "db", "uses": [{ "to": "nope" }] }"#],
                "[]",
                r#"system "db" uses "nope""#,
            ),
            (
                vec![ui],
                r#"[{ "id": "op", "uses": [{ "to": "ui", "lable": "x" }] }]"#,
                "lable",
            ),
            (
                vec![ui],
                r#"[{ "id": "op", "uses": [{ "to": "op" }] }]"#,
                r#"actor "op" uses "op""#,
            ),
            (vec![ui], r#"[{ "id": "ui" }]"#, "also a system id"),
            (
                vec![ui],
                r#"[{ "id": "op" }, { "id": "op" }]"#,
                "declared twice",
            ),
            (vec![ui], r#"[{ "id": "o p" }]"#, "o p"),
            (vec![], r#"[{ "id": "op" }]"#, "`actors` need `systems`"),
        ] {
            let systems = format!("[{}]", systems.join(", "));
            let err = parse_with_actors(&systems, actors).unwrap_err();
            assert!(
                err.to_string().contains(expected),
                "{systems} {actors}: {err}"
            );
        }
    }
}
