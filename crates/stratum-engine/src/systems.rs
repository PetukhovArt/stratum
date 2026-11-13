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
use crate::paths::{canonicalize, relative};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemMap {
    pub systems: Vec<System>,
    pub channels: Vec<Channel>,
    pub actors: Vec<Actor>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct System {
    pub id: String,
    pub name: String,
    #[serde(rename = "self")]
    pub is_self: bool,
    pub contracts_total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
    pub external: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Channel {
    pub from: String,
    pub to: String,
    pub contracts: Vec<CalledContract>,
    pub unattributed: usize,
    pub via: Vec<ViaModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViaModule {
    pub module: ModuleId,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Actor {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Link {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalledContract {
    pub id: String,
    pub signature: Option<String>,
    pub decl: Option<Decl>,
    pub callers: Vec<Caller>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decl {
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Caller {
    pub module: ModuleId,
    pub line: u32,
    pub owner: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Declared {
    id: String,
    signature: String,
    line: u32,
}

impl SystemMap {
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
                external: system.external,
                description: system.description.clone(),
            });
            if system.is_self {
                continue;
            }
            let mut channel = channel(graph, &reach, &me.id, system, &contracts);
            channel.via = via(graph, &reach, system, project_root);
            if !channel.contracts.is_empty() || channel.unattributed > 0 || !channel.via.is_empty()
            {
                channels.push(channel);
            }
        }
        let actors = config
            .actors
            .iter()
            .map(|a| Actor {
                id: a.id.clone(),
                name: a.display_name().to_owned(),
                description: a.description.clone(),
            })
            .collect();
        let links = config
            .actors
            .iter()
            .map(|a| (&a.id, &a.uses))
            .chain(config.systems.iter().map(|s| (&s.id, &s.uses)))
            .flat_map(|(from, uses)| {
                uses.iter().map(move |u| Link {
                    from: from.clone(),
                    to: u.to.clone(),
                    label: u.label.clone(),
                })
            })
            .collect();
        Some(Self {
            systems,
            channels,
            actors,
            links,
        })
    }
}

fn via(
    graph: &CompoundGraph,
    reach: &Reach,
    system: &SystemConfig,
    project_root: &Utf8Path,
) -> Vec<ViaModule> {
    let Ok(globs) = system.via_globs() else {
        return Vec::new();
    };
    if globs.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<ViaModule> = graph
        .modules
        .values()
        .filter(|m| {
            let path = relative(&m.path, project_root);
            globs.iter().any(|(_, g)| g.is_match(&path))
        })
        .map(|m| ViaModule {
            module: m.id,
            features: reach.features_running(m.id, None),
        })
        .collect();
    found.sort_by_key(|v| v.module);
    found
}

#[must_use]
pub fn system_root(config: &Config, project_root: &Utf8Path, id: &str) -> Option<Utf8PathBuf> {
    let system = config.systems.iter().find(|s| s.id == id)?;
    Some(project_root.join(system.root.as_ref()?))
}

pub fn track_calls(config: &Config, tracked: &mut TrackedCalls) {
    for matcher in config.systems.iter().flat_map(|s| &s.calls) {
        let _validated_at_load = match matcher {
            CallMatcher::Callee { callee, .. } => tracked.insert(callee),
            CallMatcher::Factory { factory, .. } => tracked.insert_factory(factory),
        };
    }
}

type Contracts = BTreeMap<String, (Declared, String)>;

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
        via: Vec::new(),
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
            external: false,
            description: None,
            via: Vec::new(),
            uses: Vec::new(),
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
    fn via_modules_make_a_channel_without_calls_and_actors_and_uses_come_in_config_order() {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-systems/web");
        let config_path = root.join("zones.config.jsonc");
        let config = crate::config::load(&root, Some(&config_path))
            .unwrap()
            .config;
        let (root, graph, ..) = crate::pipeline::graph(
            &root,
            &config,
            crate::pipeline::Source::Overlaid(Vec::new()),
        )
        .unwrap();
        let features = Features::of(&graph, &config, &root).unwrap();
        let map = SystemMap::of(&graph, &config, &root, &features).unwrap();

        let gateway = map.systems.iter().find(|s| s.id == "opc-gateway").unwrap();
        assert!(gateway.external);
        assert_eq!(gateway.description.as_deref(), Some("Plant signals"));
        let channels: Vec<(&str, usize, usize)> = map
            .channels
            .iter()
            .map(|c| (c.to.as_str(), c.contracts.len(), c.via.len()))
            .collect();
        assert_eq!(
            channels,
            [("host", 2, 0), ("api", 4, 0), ("opc-gateway", 0, 1)]
        );
        let via = &map.channels[2].via[0];
        assert_eq!(
            relative(&graph.modules[&via.module].path, &root),
            "src/shared/config/rpc.ts"
        );
        assert_eq!(via.features, ["checkout", "profile"]);
        assert_eq!(
            map.actors
                .iter()
                .map(|a| (a.id.as_str(), a.name.as_str(), a.description.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("operator", "Operator", Some("Places orders")),
                ("auditor", "auditor", None)
            ]
        );
        assert_eq!(
            map.links
                .iter()
                .map(|l| (l.from.as_str(), l.to.as_str(), l.label.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("operator", "ui", Some("places orders")),
                ("auditor", "billing", Some("reviews invoices")),
                ("ui", "api", Some("signs in")),
                ("api", "billing", Some("charges orders")),
                ("billing", "opc-gateway", Some("reads meter signals")),
            ]
        );
    }

    #[test]
    fn a_feature_runs_what_it_imports_by_name_even_through_a_barrel_or_a_factory_its_entries_and_top_level_code()
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
}
