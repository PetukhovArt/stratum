mod proto;
mod rust_attr;
mod scan;

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use stratum_config::{CallMatcher, Config, ContractSource, SystemConfig};
use stratum_core::{
    ids::ModuleId,
    types::{Call, CallArg, CallForm},
};
use stratum_graph::{CompoundGraph, TrackedCalls};

use crate::features::{Features, Reach};
use crate::paths::canonicalize;

/// The systems of `config.systems`, and per system the contracts the `self` modules call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemMap {
    pub systems: Vec<System>,
    /// One per system the project calls, in `systems` order.
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct System {
    pub id: String,
    pub name: String,
    #[serde(rename = "self")]
    pub is_self: bool,
    /// Declared contracts, called or not.
    pub contracts_total: usize,
    /// Why no contract could be read: the `root` is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Channel {
    pub from: String,
    pub to: String,
    /// The called contracts, by id.
    pub contracts: Vec<CalledContract>,
    /// Calls matched by a `calls` entry that name no contract: a non-literal name, or a client
    /// that leaves its module.
    pub unattributed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalledContract {
    pub id: String,
    /// `None` when no declared contract has this id.
    pub signature: Option<String>,
    pub decl: Option<Decl>,
    /// By module, then line.
    pub callers: Vec<Caller>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decl {
    /// `system-id:path/relative/to/root`.
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Caller {
    pub module: ModuleId,
    pub line: u32,
    pub owner: Option<String>,
    /// The ids of the features that run the call, ascending.
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Declared {
    id: String,
    signature: String,
    line: u32,
}

impl SystemMap {
    /// `None` without `systems`. A system's `root` is relative to `project_root`.
    #[must_use]
    pub fn of(
        graph: &CompoundGraph,
        config: &Config,
        project_root: &Utf8Path,
        features: &Features,
    ) -> Option<Self> {
        let reach = Reach::of(features, graph);
        let me = config.systems.iter().find(|s| s.is_self)?;
        let mut systems = Vec::new();
        let mut channels = Vec::new();
        for system in &config.systems {
            let declared = declared(system, project_root);
            let (contracts, unavailable) = match declared {
                Ok(contracts) => (contracts, None),
                Err(reason) => (BTreeMap::new(), Some(reason)),
            };
            systems.push(System {
                id: system.id.clone(),
                name: system.display_name().to_owned(),
                is_self: system.is_self,
                contracts_total: contracts.len(),
                unavailable,
            });
            if system.is_self {
                continue;
            }
            let channel = channel(graph, &reach, &me.id, system, &contracts);
            if !channel.contracts.is_empty() || channel.unattributed > 0 {
                channels.push(channel);
            }
        }
        Some(Self { systems, channels })
    }
}

/// The directory of system `id`'s `root`, if it declares one.
#[must_use]
pub fn system_root(config: &Config, project_root: &Utf8Path, id: &str) -> Option<Utf8PathBuf> {
    let system = config.systems.iter().find(|s| s.id == id)?;
    Some(project_root.join(system.root.as_ref()?))
}

/// Adds the `calls` entries of every system to `tracked`.
pub fn track_calls(config: &Config, tracked: &mut TrackedCalls) {
    for matcher in config.systems.iter().flat_map(|s| &s.calls) {
        let _validated_at_load = match matcher {
            CallMatcher::Callee { callee, .. } => tracked.insert(callee),
            CallMatcher::Factory { factory, .. } => tracked.insert_factory(factory),
        };
    }
}

type Contracts = BTreeMap<String, (Declared, String)>;

/// The declared contracts by id, each with its file as `system-id:rel`.
fn declared(system: &SystemConfig, project_root: &Utf8Path) -> Result<Contracts, String> {
    let (Some(root), Some(source)) = (&system.root, &system.contracts) else {
        return Ok(BTreeMap::new());
    };
    let dir = canonicalize(&project_root.join(root))
        .ok()
        .filter(|d| d.is_dir())
        .ok_or_else(|| format!("root {root} is not a directory"))?;
    let extension = match source {
        ContractSource::Proto => "proto",
        ContractSource::RustAttr { .. } => "rs",
    };
    let mut contracts = BTreeMap::new();
    for (rel, text) in files(&dir, extension) {
        let found = match source {
            ContractSource::Proto => proto::contracts(&text),
            ContractSource::RustAttr { attr } => rust_attr::contracts(&text, attr),
        };
        for contract in found {
            let path = format!("{}:{rel}", system.id);
            contracts
                .entry(contract.id.clone())
                .or_insert((contract, path));
        }
    }
    Ok(contracts)
}

/// Every readable `*.extension` file under `dir`, by path relative to it with `/`, in path
/// order; hidden directories, `node_modules` and `target` are skipped.
fn files(dir: &Utf8Path, extension: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0
                || !(e.file_type().is_dir()
                    && (name.starts_with('.') || name == "node_modules" || name == "target"))
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|x| x == extension))
        .filter_map(|e| {
            let rel = e.path().strip_prefix(dir).ok()?;
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            Some((rel, std::fs::read_to_string(e.path()).ok()?))
        })
        .collect();
    found.sort();
    found
}

enum Named {
    Contract(String),
    Unattributed,
}

fn named(matcher: &CallMatcher, call: &Call) -> Option<Named> {
    let named = match (matcher, &call.form) {
        (CallMatcher::Callee { callee, name_arg }, CallForm::Direct) if &call.name == callee => {
            match call.args.get(*name_arg) {
                Some(CallArg::Str(name)) => Some(name.clone()),
                _ => None,
            }
        }
        (
            CallMatcher::Factory {
                factory,
                service_arg,
            },
            CallForm::Client { method },
        ) if &call.name == factory => match call.args.get(*service_arg) {
            Some(CallArg::Ident(service)) => Some(format!("{service}.{method}")),
            _ => None,
        },
        (CallMatcher::Factory { factory, .. }, CallForm::Unfollowed) if &call.name == factory => {
            None
        }
        _ => return None,
    };
    Some(named.map_or(Named::Unattributed, Named::Contract))
}

/// The declared id `name` stands for: itself, or for `Service.method` the declared
/// `Service.Method` whose method differs only in the case of its first letter.
fn resolve<'c>(contracts: &'c Contracts, name: &str) -> Option<&'c (Declared, String)> {
    if let Some(found) = contracts.get(name) {
        return Some(found);
    }
    let (service, method) = name.rsplit_once('.')?;
    let mut chars = method.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    [first.to_ascii_uppercase(), first.to_ascii_lowercase()]
        .into_iter()
        .find_map(|c| contracts.get(&format!("{service}.{c}{rest}")))
}

fn channel(
    graph: &CompoundGraph,
    reach: &Reach,
    from: &str,
    system: &SystemConfig,
    contracts: &Contracts,
) -> Channel {
    let mut called: BTreeMap<String, CalledContract> = BTreeMap::new();
    let mut unattributed = 0;
    for module in graph.modules.values() {
        for call in &module.calls {
            let Some(named) = system.calls.iter().find_map(|m| named(m, call)) else {
                continue;
            };
            let Named::Contract(name) = named else {
                unattributed += 1;
                continue;
            };
            let declared = resolve(contracts, &name);
            let id = declared.map_or(name, |(d, _)| d.id.clone());
            called
                .entry(id.clone())
                .or_insert_with(|| CalledContract {
                    id,
                    signature: declared.map(|(d, _)| d.signature.clone()),
                    decl: declared.map(|(d, path)| Decl {
                        path: path.clone(),
                        line: d.line,
                    }),
                    callers: Vec::new(),
                })
                .callers
                .push(Caller {
                    module: module.id,
                    line: call.location.line,
                    owner: call.owner.clone(),
                    features: reach.features_running(module.id, call.owner.as_deref()),
                });
        }
    }
    let mut contracts: Vec<CalledContract> = called.into_values().collect();
    for contract in &mut contracts {
        contract.callers.sort_by_key(|c| (c.module, c.line));
    }
    Channel {
        from: from.to_owned(),
        to: system.id.clone(),
        contracts,
        unattributed,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use stratum_core::{
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Exports, Module},
        violation::SourceLocation,
        visibility::VisibilityScope,
    };

    use super::*;

    fn call(name: &str, line: u32, args: Vec<CallArg>, form: CallForm) -> Call {
        Call {
            name: name.into(),
            location: SourceLocation { line, column: 1 },
            args,
            owner: Some(format!("f{line}")),
            form,
        }
    }

    fn graph(calls: Vec<Call>) -> CompoundGraph {
        let mut graph = CompoundGraph::empty();
        let id = ModuleId::new(7);
        graph.modules.insert(
            id,
            Module {
                id,
                path: PathBuf::from("src/api.ts"),
                container: ContainerId::new(0),
                layer: LayerId::new(0),
                purity: Purity::DEFAULT,
                visibility: VisibilityScope::Public,
                exports: Exports::default(),
                calls,
                imports: vec![],
                suppressions: vec![],
                local_refs: vec![],
            },
        );
        graph
    }

    fn system(calls: Vec<CallMatcher>) -> SystemConfig {
        SystemConfig {
            id: "api".into(),
            name: None,
            is_self: false,
            root: None,
            contracts: None,
            calls,
        }
    }

    fn declared(id: &str, line: u32) -> (String, (Declared, String)) {
        let contract = Declared {
            id: id.into(),
            signature: format!("sig {id}"),
            line,
        };
        (id.into(), (contract, "api:a.proto".into()))
    }

    fn str(s: &str) -> CallArg {
        CallArg::Str(s.into())
    }

    fn ident(s: &str) -> CallArg {
        CallArg::Ident(s.into())
    }

    fn client(method: &str) -> CallForm {
        CallForm::Client {
            method: method.into(),
        }
    }

    #[test]
    fn calls_name_declared_contracts_case_insensitively_on_the_first_method_letter() {
        let graph = graph(vec![
            call("invoke", 1, vec![ident("x"), str("save")], CallForm::Direct),
            call("invoke", 2, vec![str("x"), ident("name")], CallForm::Direct),
            call(
                "invoke",
                3,
                vec![str("x"), str("missing")],
                CallForm::Direct,
            ),
            call("createClient", 4, vec![ident("Users")], client("getUser")),
            call("createClient", 5, vec![ident("Users")], client("GetUser")),
            call("createClient", 6, vec![str("Users")], client("getUser")),
            call(
                "createClient",
                7,
                vec![ident("Users")],
                CallForm::Unfollowed,
            ),
            call("createClient", 8, vec![ident("Users")], CallForm::Direct),
            call("other", 9, vec![str("save")], CallForm::Direct),
        ]);
        let system = system(vec![
            CallMatcher::Callee {
                callee: "invoke".into(),
                name_arg: 1,
            },
            CallMatcher::Factory {
                factory: "createClient".into(),
                service_arg: 0,
            },
        ]);
        let contracts = [declared("save", 3), declared("Users.GetUser", 12)]
            .into_iter()
            .collect();

        let features = Features::of(
            &graph,
            &stratum_config::parse_str(Utf8Path::new("inline"), r#"{ "version": 1 }"#).unwrap(),
            Utf8Path::new("/"),
        )
        .unwrap();
        let reach = Reach::of(&features, &graph);

        let channel = channel(&graph, &reach, "ui", &system, &contracts);

        let summary: Vec<(&str, Option<u32>, Vec<u32>)> = channel
            .contracts
            .iter()
            .map(|c| {
                (
                    c.id.as_str(),
                    c.decl.as_ref().map(|d| d.line),
                    c.callers.iter().map(|c| c.line).collect(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("Users.GetUser", Some(12), vec![4, 5]),
                ("missing", None, vec![3]),
                ("save", Some(3), vec![1]),
            ]
        );
        assert_eq!(channel.unattributed, 3);
        assert_eq!(channel.contracts[0].callers[0].owner.as_deref(), Some("f4"));
    }

    /// Per feature, the contracts it runs a call of.
    fn contracts_per_feature(fixture: &str) -> BTreeMap<String, Vec<String>> {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(fixture);
        let config = crate::config::load(&root, None).unwrap().config;
        let (root, graph, ..) = crate::pipeline::graph(
            &root,
            &config,
            crate::pipeline::Source::Overlaid(Vec::new()),
        )
        .unwrap();
        let features = Features::of(&graph, &config, &root).unwrap();
        let map = SystemMap::of(&graph, &config, &root, &features).unwrap();
        let mut per_feature: BTreeMap<String, Vec<String>> = features
            .features()
            .iter()
            .map(|f| (f.id.clone(), Vec::new()))
            .collect();
        for contract in map.channels.iter().flat_map(|c| &c.contracts) {
            for feature in contract.callers.iter().flat_map(|c| &c.features) {
                let ids = per_feature.entry(feature.clone()).or_default();
                if !ids.contains(&contract.id) {
                    ids.push(contract.id.clone());
                }
            }
        }
        per_feature
    }

    #[test]
    fn a_feature_runs_what_it_imports_by_name_even_through_a_barrel_its_entries_and_top_level_code()
    {
        let per_feature = contracts_per_feature("../../tests/fixtures/tiny-systems/web");
        let expected = |ids: &[&str]| ids.iter().map(|&id| id.to_owned()).collect::<Vec<_>>();
        assert_eq!(
            per_feature,
            BTreeMap::from([
                (
                    "checkout".to_owned(),
                    expected(&[
                        "greet",
                        "save_file",
                        "OrderService.PlaceOrder",
                        "UserService.GetUser"
                    ])
                ),
                ("files".to_owned(), expected(&["greet", "save_file"])),
                (
                    "profile".to_owned(),
                    expected(&[
                        "greet",
                        "UserService.GetUser",
                        "UserService.WatchUsers",
                        "UserService.forgetUser"
                    ])
                ),
            ])
        );
    }

    #[test]
    fn a_feature_importing_what_a_factory_returns_runs_the_calls_inside_the_factory_and_only_it() {
        let per_feature = contracts_per_feature("../../tests/fixtures/tiny-systems/web");
        let runs = |feature: &str| {
            per_feature
                .get(feature)
                .is_some_and(|ids| ids.iter().any(|id| id == "OrderService.PlaceOrder"))
        };
        assert!(runs("checkout"), "imports ordersApi: {per_feature:?}");
        assert!(
            !runs("profile"),
            "imports only orderStatuses: {per_feature:?}"
        );
    }
}
