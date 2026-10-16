#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use camino::{Utf8Path, Utf8PathBuf};
use stratum_core::{
    ids::{ContainerId, ModuleId, RuleId},
    severity::Severity,
    types::ContainerKind,
};
use stratum_engine::{EngineInput, pipeline};
use stratum_rules::territory::{Region, Site, Territories, Territory};
use stratum_rules::{
    NO_CROSS_LAYER_IMPORT, PUBLIC_API, PURITY, SIBLING_ISOLATION, VISIBILITY_SCOPE,
};

type Breaches = BTreeSet<(RuleId, Severity)>;

#[derive(Debug, Default)]
struct Agreement {
    edges: usize,
    pairs: usize,
    disagreements: Vec<String>,
}

fn build(root: &Utf8Path, config: Option<&Utf8Path>) -> EngineInput {
    let loaded = stratum_engine::config::load(root, config).unwrap();
    pipeline::build(root, &loaded.config).unwrap()
}

fn edge_findings(input: &EngineInput) -> HashMap<(ModuleId, ModuleId), Breaches> {
    let edge_rules: BTreeSet<RuleId> = input
        .registry
        .rules()
        .iter()
        .filter(|r| r.edge_judge(&serde_json::Value::Null).is_some())
        .map(|r| r.id())
        .collect();
    let mut by_edge: HashMap<(ModuleId, ModuleId), Breaches> = HashMap::new();
    for v in input.rule_violations().unwrap() {
        if let (Some(e), true) = (v.edge, edge_rules.contains(&v.rule)) {
            by_edge
                .entry((e.from, e.to))
                .or_default()
                .insert((v.rule, v.severity));
        }
    }
    by_edge
}

fn regions_naming(
    input: &EngineInput,
    zone_of: &HashMap<ModuleId, usize>,
    from: ModuleId,
    territory: &Territory,
    to: ModuleId,
) -> BTreeSet<RuleId> {
    let graph = &input.graph;
    let target = &graph.modules[&to];
    let mut named = BTreeSet::new();
    if !territory.may_import_layers.contains(&target.layer) {
        named.insert(NO_CROSS_LAYER_IMPORT);
    }
    if target.purity.rank() > territory.max_import_purity {
        named.insert(PURITY);
    }
    if zone_of
        .get(&to)
        .is_some_and(|z| !territory.visible_zones.iter().any(|v| v.index() == *z))
    {
        named.insert(VISIBILITY_SCOPE);
    }
    let holders: BTreeSet<ContainerId> = graph.containers_of(to).map(|c| c.id).collect();
    for f in &territory.may_not {
        let inside = match &f.region {
            Region::Siblings { containers, .. } => containers.iter().any(|c| holders.contains(c)),
            Region::Internals => graph
                .containers_of(to)
                .filter(|c| c.kind != ContainerKind::Layer)
                .take_while(|c| !graph.containers_of(from).any(|h| h.id == c.id))
                .last()
                .is_some_and(|c| c.entry != Some(to)),
        };
        if inside {
            named.insert(f.rule);
        }
    }
    named
}

fn agreement(input: &EngineInput) -> Agreement {
    let graph = &input.graph;
    let territories =
        Territories::new(graph, &input.config, &input.project_root, &input.registry).unwrap();
    let findings = edge_findings(input);
    let mut out = Agreement::default();

    let edges: BTreeSet<(ModuleId, ModuleId)> = graph
        .deps
        .raw_edges()
        .iter()
        .map(|e| (graph.deps[e.source()], graph.deps[e.target()]))
        .collect();
    for &(from, to) in &edges {
        out.edges += 1;
        let verdict: Breaches = territories
            .verdict(from, to)
            .into_iter()
            .map(|b| (b.rule, b.severity))
            .collect();
        let emitted = findings.get(&(from, to)).cloned().unwrap_or_default();
        if verdict != emitted {
            out.disagreements.push(format!(
                "edge {from:?}->{to:?}: verdict {verdict:?}, run_all {emitted:?}"
            ));
        }
    }

    let zone_of: HashMap<ModuleId, usize> = graph
        .private_zones()
        .iter()
        .enumerate()
        .flat_map(|(i, z)| z.files.iter().map(move |&m| (m, i)))
        .collect();
    let mapped = [
        NO_CROSS_LAYER_IMPORT,
        PURITY,
        VISIBILITY_SCOPE,
        SIBLING_ISOLATION,
        PUBLIC_API,
    ];
    for &from in graph.modules.keys() {
        let territory = territories.of(Site::Module(from));
        for &to in graph.modules.keys() {
            out.pairs += 1;
            let named: BTreeSet<RuleId> = territories
                .verdict(from, to)
                .into_iter()
                .map(|b| b.rule)
                .filter(|r| mapped.contains(r))
                .collect();
            let inside = regions_naming(input, &zone_of, from, &territory, to);
            if named != inside {
                out.disagreements.push(format!(
                    "pair {from:?}->{to:?}: verdict {named:?}, regions {inside:?}"
                ));
            }
        }
    }
    out
}

#[test]
fn territory_agrees_with_run_all_on_every_fixture() {
    let fixtures = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut seen = BTreeMap::new();
    for dir in fixtures.read_dir_utf8().unwrap() {
        let root = dir.unwrap().path().to_path_buf();
        let a = agreement(&build(&root, None));
        assert!(a.disagreements.is_empty(), "{root}: {:#?}", a.disagreements);
        seen.insert(root.file_name().unwrap().to_owned(), (a.edges, a.pairs));
    }
    assert!(seen.values().any(|&(edges, _)| edges > 0), "{seen:?}");
}
