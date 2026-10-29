use std::collections::{BTreeMap, BTreeSet};

use stratum_core::{
    ids::{ContainerId, ModuleId},
    types::{Container, ContainerKind},
};
use stratum_engine::{EngineInput, SystemMap};

use super::{
    Diagram, Element, Kind, Rel,
    text::{Ids, count},
};
use crate::{cli::C4Level, commands::features::ensure_known};

#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub level: C4Level,
    /// Restricts the diagram to the union of these features' modules; empty means every module.
    pub features: &'a [String],
    /// The container the component level opens, by the name the container level gives it.
    pub container: Option<&'a str>,
}

/// Past this many elements inside the boundary, the smallest give way to one "… N more".
const MAX_INNER: usize = 60;
/// Contract ids named in a relationship label; the rest are counted.
const TOP_CONTRACTS: usize = 3;
const ROOT_SEGMENT: &str = "(root)";

pub fn diagram(input: &EngineInput, request: &Request) -> miette::Result<Diagram> {
    let scope = scope(input, request.features)?;
    let systems = input.systems();
    let mut ids = Ids::default();
    let me = self_system(input, systems.as_ref(), &mut ids);
    match request.level {
        C4Level::System => {
            let systems = systems.ok_or_else(|| {
                miette::miette!(
                    "the system level needs `systems` in the config (see docs/configuration.md); \
                     `--level container` works without it"
                )
            })?;
            Ok(system_level(input, &scope, &systems, me, &mut ids))
        }
        C4Level::Container => Ok(container_level(
            input,
            &scope,
            systems.as_ref(),
            me,
            &mut ids,
        )),
        C4Level::Component => component_level(input, &scope, request.container, me, &mut ids),
    }
}

/// The modules a diagram may show: `None` for every module.
type Scope = Option<BTreeSet<ModuleId>>;

fn scope(input: &EngineInput, selected: &[String]) -> miette::Result<Scope> {
    if selected.is_empty() {
        return Ok(None);
    }
    let features = input.features().map_err(|e| miette::miette!("{e}"))?;
    ensure_known(&features, selected)?;
    Ok(Some(
        selected
            .iter()
            .filter_map(|id| features.get(id))
            .flat_map(|f| f.modules.iter().copied())
            .collect(),
    ))
}

fn in_scope(scope: &Scope, module: ModuleId) -> bool {
    scope.as_ref().is_none_or(|s| s.contains(&module))
}

fn self_system(input: &EngineInput, systems: Option<&SystemMap>, ids: &mut Ids) -> Element {
    let (id, name) = systems
        .and_then(|s| s.systems.iter().find(|s| s.is_self))
        .map_or_else(
            || {
                let project = input.project_root.file_name().unwrap_or("project");
                (project.to_owned(), project.to_owned())
            },
            |s| (s.id.clone(), s.name.clone()),
        );
    Element {
        id: ids.take("sys", &id),
        kind: Kind::System,
        name,
        technology: String::new(),
        description: String::new(),
    }
}

fn other_systems(
    systems: Option<&SystemMap>,
    ids: &mut Ids,
) -> (Vec<Element>, BTreeMap<String, String>) {
    let mut by_config_id = BTreeMap::new();
    let mut outer: Vec<Element> = systems
        .into_iter()
        .flat_map(|s| &s.systems)
        .filter(|s| !s.is_self)
        .map(|s| {
            let id = ids.take("sys", &s.id);
            by_config_id.insert(s.id.clone(), id.clone());
            let description = s.unavailable.as_ref().map_or_else(
                || format!("{} declared", count(s.contracts_total, "contract")),
                |why| format!("contracts unavailable: {why}"),
            );
            Element {
                id,
                kind: Kind::SystemExt,
                name: s.name.clone(),
                technology: String::new(),
                description,
            }
        })
        .collect();
    outer.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    (outer, by_config_id)
}

/// Per relationship, each called contract id with its number of call sites.
type Calls = BTreeMap<(String, String), BTreeMap<String, usize>>;

/// Calls into other systems from modules in scope, keyed by `key(module)` and the system id.
fn calls(
    systems: Option<&SystemMap>,
    scope: &Scope,
    system_ids: &BTreeMap<String, String>,
    mut key: impl FnMut(ModuleId) -> Option<String>,
) -> Calls {
    let mut calls = Calls::new();
    for channel in systems.into_iter().flat_map(|s| &s.channels) {
        let Some(to) = system_ids.get(&channel.to) else {
            continue;
        };
        for contract in &channel.contracts {
            for caller in contract
                .callers
                .iter()
                .filter(|c| in_scope(scope, c.module))
            {
                if let Some(from) = key(caller.module) {
                    *calls
                        .entry((from, to.clone()))
                        .or_default()
                        .entry(contract.id.clone())
                        .or_default() += 1;
                }
            }
        }
    }
    calls
}

fn contracts_label(called: &BTreeMap<String, usize>, unattributed: usize) -> String {
    let mut top: Vec<(&String, &usize)> = called.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let mut names: Vec<&str> = top
        .iter()
        .take(TOP_CONTRACTS)
        .map(|(id, _)| id.as_str())
        .collect();
    let rest = called.len().saturating_sub(TOP_CONTRACTS);
    let more = format!("+{rest} more");
    if rest > 0 {
        names.push(&more);
    }
    let mut label = count(called.len(), "contract");
    if !names.is_empty() {
        label = format!("{label}: {}", names.join(", "));
    }
    if unattributed > 0 {
        label = format!("{label}; {} not attributed", count(unattributed, "call"));
    }
    label
}

fn system_level(
    input: &EngineInput,
    scope: &Scope,
    systems: &SystemMap,
    mut me: Element,
    ids: &mut Ids,
) -> Diagram {
    let modules = input
        .graph
        .modules
        .keys()
        .filter(|&&m| in_scope(scope, m))
        .count();
    me.description = count(modules, "module");
    let (outer, system_ids) = other_systems(Some(systems), ids);
    let calls = calls(Some(systems), scope, &system_ids, |_| Some(me.id.clone()));
    let mut rels = Vec::new();
    for channel in &systems.channels {
        let Some(to) = system_ids.get(&channel.to) else {
            continue;
        };
        let called = calls
            .get(&(me.id.clone(), to.clone()))
            .cloned()
            .unwrap_or_default();
        let unattributed = if scope.is_none() {
            channel.unattributed
        } else {
            0
        };
        if called.is_empty() && unattributed == 0 {
            continue;
        }
        rels.push(Rel {
            from: me.id.clone(),
            to: to.clone(),
            label: contracts_label(&called, unattributed),
        });
    }
    rels.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Diagram {
        level: C4Level::System,
        title: format!("System context of {}", me.name),
        boundaries: Vec::new(),
        inner: vec![me],
        outer,
        rels,
    }
}

/// A layer, or a slice of a sliced layer: the container a module's [`outer_container`] gives.
///
/// [`outer_container`]: stratum_graph::CompoundGraph::outer_container
struct Unit<'g> {
    container: &'g Container,
    name: String,
    modules: Vec<ModuleId>,
}

fn units<'g>(input: &'g EngineInput, scope: &Scope) -> BTreeMap<ContainerId, Unit<'g>> {
    let graph = &input.graph;
    let mut units: BTreeMap<ContainerId, Unit<'g>> = BTreeMap::new();
    for &module in graph.modules.keys().filter(|&&m| in_scope(scope, m)) {
        let Some(container) = graph.outer_container(module) else {
            continue;
        };
        units
            .entry(container.id)
            .or_insert_with(|| Unit {
                container,
                name: unit_name(input, container),
                modules: Vec::new(),
            })
            .modules
            .push(module);
    }
    units
}

fn unit_name(input: &EngineInput, container: &Container) -> String {
    match (container.kind, input.graph.layers.get(&container.layer)) {
        (ContainerKind::Slice, Some(layer)) => format!("{}/{}", layer.name, container.name),
        _ => container.name.clone(),
    }
}

fn unit_element(input: &EngineInput, unit: &Unit, modules: usize, ids: &mut Ids) -> Element {
    let technology = match unit.container.kind {
        ContainerKind::Slice => "slice",
        _ => "layer",
    };
    Element {
        id: ids.take("ctr", &input.relative(&unit.container.path)),
        kind: Kind::Container,
        name: unit.name.clone(),
        technology: technology.to_owned(),
        description: count(modules, "module"),
    }
}

/// Distinct module pairs `from → to` with both ends in scope, by `key` of each end; pairs
/// within one key are left out.
fn imports(
    input: &EngineInput,
    scope: &Scope,
    key: impl Fn(ModuleId) -> Option<String>,
) -> BTreeMap<(String, String), usize> {
    let deps = &input.graph.deps;
    let pairs: BTreeSet<(ModuleId, ModuleId)> = deps
        .raw_edges()
        .iter()
        .map(|e| (deps[e.source()], deps[e.target()]))
        .filter(|&(from, to)| from != to && in_scope(scope, from) && in_scope(scope, to))
        .collect();
    let mut counts = BTreeMap::new();
    for (from, to) in pairs {
        match (key(from), key(to)) {
            (Some(from), Some(to)) if from != to => *counts.entry((from, to)).or_default() += 1,
            _ => {}
        }
    }
    counts
}

/// An inner element with the number of modules it holds, before the cap.
struct Sized {
    element: Element,
    modules: usize,
}

/// Keeps the `MAX_INNER - 1` largest elements and folds the rest into one "… N more", whose
/// relationships are the sums of theirs. Ties go to the earlier name.
fn cap(
    mut sized: Vec<Sized>,
    imports: &mut BTreeMap<(String, String), usize>,
    calls: &mut Calls,
    ids: &mut Ids,
) -> Vec<Element> {
    sized.sort_by(|a, b| {
        b.modules
            .cmp(&a.modules)
            .then_with(|| a.element.name.cmp(&b.element.name))
    });
    if sized.len() > MAX_INNER {
        let rest = sized.split_off(MAX_INNER - 1);
        let more = Element {
            id: ids.take("more", "elements"),
            kind: rest[0].element.kind,
            name: format!("… {} more", rest.len()),
            technology: String::new(),
            description: count(rest.iter().map(|s| s.modules).sum(), "module"),
        };
        let folded: BTreeSet<&str> = rest.iter().map(|s| s.element.id.as_str()).collect();
        let fold = |id: String| {
            if folded.contains(id.as_str()) {
                more.id.clone()
            } else {
                id
            }
        };
        let mut kept_imports = BTreeMap::new();
        for ((from, to), n) in std::mem::take(imports) {
            let (from, to) = (fold(from), fold(to));
            if from != to {
                *kept_imports.entry((from, to)).or_default() += n;
            }
        }
        *imports = kept_imports;
        let mut kept_calls = Calls::new();
        for ((from, to), called) in std::mem::take(calls) {
            let entry = kept_calls.entry((fold(from), to)).or_default();
            for (contract, n) in called {
                *entry.entry(contract).or_default() += n;
            }
        }
        *calls = kept_calls;
        sized.push(Sized {
            element: more,
            modules: 0,
        });
    }
    let mut inner: Vec<Element> = sized.into_iter().map(|s| s.element).collect();
    inner.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    inner
}

fn rels(imports: BTreeMap<(String, String), usize>, calls: Calls) -> Vec<Rel> {
    let mut rels: Vec<Rel> = imports
        .into_iter()
        .map(|((from, to), n)| Rel {
            from,
            to,
            label: count(n, "import"),
        })
        .chain(calls.into_iter().map(|((from, to), called)| Rel {
            from,
            to,
            label: contracts_label(&called, 0),
        }))
        .collect();
    rels.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    rels
}

fn container_level(
    input: &EngineInput,
    scope: &Scope,
    systems: Option<&SystemMap>,
    me: Element,
    ids: &mut Ids,
) -> Diagram {
    let mut units: Vec<Unit> = units(input, scope).into_values().collect();
    units.sort_by_key(|u| input.relative(&u.container.path));
    let mut unit_ids: BTreeMap<ContainerId, String> = BTreeMap::new();
    let sized: Vec<Sized> = units
        .iter()
        .map(|unit| {
            let element = unit_element(input, unit, unit.modules.len(), ids);
            unit_ids.insert(unit.container.id, element.id.clone());
            Sized {
                element,
                modules: unit.modules.len(),
            }
        })
        .collect();
    let (outer, system_ids) = other_systems(systems, ids);
    let unit_of = |m: ModuleId| {
        let container = input.graph.outer_container(m)?;
        unit_ids.get(&container.id).cloned()
    };
    let mut imports = imports(input, scope, unit_of);
    let mut calls = calls(systems, scope, &system_ids, unit_of);
    let inner = cap(sized, &mut imports, &mut calls, ids);
    Diagram {
        level: C4Level::Container,
        title: format!("Containers of {}", me.name),
        boundaries: vec![me],
        inner,
        outer,
        rels: rels(imports, calls),
    }
}

fn component_level(
    input: &EngineInput,
    scope: &Scope,
    wanted: Option<&str>,
    me: Element,
    ids: &mut Ids,
) -> miette::Result<Diagram> {
    let all = units(input, &None);
    let mut known: Vec<&str> = all.values().map(|u| u.name.as_str()).collect();
    known.sort_unstable();
    let known = known.join(", ");
    let wanted = wanted.ok_or_else(|| {
        miette::miette!("the component level needs `--container NAME`; containers: {known}")
    })?;
    let unit = all
        .values()
        .find(|u| u.name == wanted)
        .ok_or_else(|| miette::miette!("unknown container {wanted:?}; known: {known}"))?;
    let base = input.relative(&unit.container.path);
    let segment_of = |m: ModuleId| -> Option<String> {
        if input.graph.outer_container(m)?.id != unit.container.id {
            return None;
        }
        let path = input.relative(&input.graph.modules.get(&m)?.path);
        let rest = path.strip_prefix(&base)?.trim_start_matches('/');
        Some(
            rest.split_once('/')
                .map_or(ROOT_SEGMENT, |(first, _)| first)
                .to_owned(),
        )
    };
    let mut segments: BTreeMap<String, usize> = BTreeMap::new();
    for &m in unit.modules.iter().filter(|&&m| in_scope(scope, m)) {
        if let Some(segment) = segment_of(m) {
            *segments.entry(segment).or_default() += 1;
        }
    }
    let boundary = unit_element(input, unit, segments.values().sum(), ids);
    let mut segment_ids: BTreeMap<String, String> = BTreeMap::new();
    let sized: Vec<Sized> = segments
        .into_iter()
        .map(|(segment, modules)| {
            let id = ids.take("cmp", &format!("{base}/{segment}"));
            segment_ids.insert(segment.clone(), id.clone());
            let technology = if segment == ROOT_SEGMENT {
                "files"
            } else {
                "folder"
            };
            Sized {
                element: Element {
                    id,
                    kind: Kind::Component,
                    name: segment,
                    technology: technology.to_owned(),
                    description: count(modules, "module"),
                },
                modules,
            }
        })
        .collect();
    let mut imports = imports(input, scope, |m| segment_ids.get(&segment_of(m)?).cloned());
    let mut calls = Calls::new();
    let inner = cap(sized, &mut imports, &mut calls, ids);
    Ok(Diagram {
        level: C4Level::Component,
        title: format!("Components of {}", unit.name),
        boundaries: vec![me, boundary],
        inner,
        outer: Vec::new(),
        rels: rels(imports, calls),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(id: &str, modules: usize) -> Sized {
        Sized {
            element: Element {
                id: id.to_owned(),
                kind: Kind::Container,
                name: id.to_owned(),
                technology: String::new(),
                description: String::new(),
            },
            modules,
        }
    }

    #[test]
    fn past_the_cap_the_smallest_fold_into_one_element_that_takes_over_their_relationships() {
        let sized: Vec<Sized> = (0..=MAX_INNER)
            .map(|i| element(&format!("e{i:02}"), 100 - i))
            .collect();
        let last = format!("e{MAX_INNER:02}");
        let second_last = format!("e{:02}", MAX_INNER - 1);
        let mut imports: BTreeMap<(String, String), usize> = [
            (("e00".to_owned(), last.clone()), 2),
            (("e00".to_owned(), second_last.clone()), 3),
            ((last.clone(), second_last.clone()), 7),
        ]
        .into_iter()
        .collect();
        let mut calls: Calls = [(
            (last.clone(), "sys_api".to_owned()),
            [("Users.Get".to_owned(), 1)].into_iter().collect(),
        )]
        .into_iter()
        .collect();

        let inner = cap(sized, &mut imports, &mut calls, &mut Ids::default());

        assert_eq!(inner.len(), MAX_INNER);
        let more = inner
            .iter()
            .find(|e| e.name == "… 2 more")
            .map(|e| e.id.clone());
        assert_eq!(more.as_deref(), Some("more_elements"));
        assert!(!inner.iter().any(|e| e.id == last || e.id == second_last));
        assert_eq!(
            imports.into_iter().collect::<Vec<_>>(),
            [(("e00".to_owned(), "more_elements".to_owned()), 5)]
        );
        assert_eq!(
            calls.keys().collect::<Vec<_>>(),
            [&("more_elements".to_owned(), "sys_api".to_owned())]
        );
    }

    #[test]
    fn a_contract_label_names_the_most_called_three_and_counts_the_rest() {
        let called: BTreeMap<String, usize> = [("a", 1), ("b", 5), ("c", 1), ("d", 2), ("e", 1)]
            .into_iter()
            .map(|(id, n)| (id.to_owned(), n))
            .collect();
        assert_eq!(
            contracts_label(&called, 2),
            "5 contracts: b, d, a, +2 more; 2 calls not attributed"
        );
        assert_eq!(
            contracts_label(&BTreeMap::new(), 1),
            "0 contracts; 1 call not attributed"
        );
    }
}
