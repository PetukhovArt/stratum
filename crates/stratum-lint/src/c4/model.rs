use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use stratum_core::{
    ids::ModuleId,
    types::{ContainerKind, Layer},
};
use stratum_engine::{Caller, EngineInput, SystemMap};

use super::{
    Diagram, Element, Kind, Rel,
    text::{Ids, count},
};
use crate::{cli::C4Level, commands::features::ensure_known};

#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub level: C4Level,
    pub features: &'a [String],
    pub container: Option<&'a str>,
}

const MAX_INNER: usize = 60;

pub fn diagram(input: &EngineInput, request: &Request) -> miette::Result<Diagram> {
    let scope = scope(input, request.features)?;
    let systems = input.systems().map_err(|e| miette::miette!("{e}"))?;
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

type Scope = Option<Selection>;

struct Selection {
    modules: BTreeSet<ModuleId>,
    ids: Vec<String>,
}

fn scope(input: &EngineInput, selected: &[String]) -> miette::Result<Scope> {
    if selected.is_empty() {
        return Ok(None);
    }
    let features = input.features().map_err(|e| miette::miette!("{e}"))?;
    ensure_known(&features, selected)?;
    Ok(Some(Selection {
        modules: selected
            .iter()
            .filter_map(|id| features.get(id))
            .flat_map(|f| f.modules.iter().copied())
            .collect(),
        ids: selected.to_vec(),
    }))
}

fn in_scope(scope: &Scope, module: ModuleId) -> bool {
    scope.as_ref().is_none_or(|s| s.modules.contains(&module))
}

fn runs(scope: &Scope, caller: &Caller) -> bool {
    scope
        .as_ref()
        .is_none_or(|s| caller.features.iter().any(|f| s.ids.contains(f)))
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
            let contracts = s.unavailable.as_ref().map_or_else(
                || format!("{} declared", count(s.contracts_total, "contract")),
                |why| format!("contracts unavailable: {why}"),
            );
            let description = match &s.description {
                None => contracts,
                Some(own) if s.contracts_total == 0 && s.unavailable.is_none() => own.clone(),
                Some(own) => format!("{own}, {contracts}"),
            };
            Element {
                id,
                kind: if s.external {
                    Kind::SystemExt
                } else {
                    Kind::System
                },
                name: s.name.clone(),
                technology: String::new(),
                description,
            }
        })
        .collect();
    outer.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    (outer, by_config_id)
}

type Calls = BTreeMap<(String, String), BTreeMap<String, usize>>;

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
            for caller in contract.callers.iter().filter(|c| runs(scope, c)) {
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

fn contracts_rel(
    from: String,
    to: String,
    called: &BTreeMap<String, usize>,
    unattributed: usize,
) -> Rel {
    let mut label = count(called.len(), "contract");
    if unattributed > 0 {
        label = format!("{label}, {} not attributed", count(unattributed, "call"));
    }
    Rel {
        from,
        to,
        label,
        weight: called.len() + unattributed,
    }
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
    if let Some(own) = systems
        .systems
        .iter()
        .find(|s| s.is_self)
        .and_then(|s| s.description.as_ref())
    {
        me.description = format!("{own}, {}", me.description);
    }
    let actors: Vec<Element> = systems
        .actors
        .iter()
        .map(|a| Element {
            id: ids.take("act", &a.id),
            kind: Kind::Person,
            name: a.name.clone(),
            technology: String::new(),
            description: a.description.clone().unwrap_or_default(),
        })
        .collect();
    let (others, mut element_ids) = other_systems(Some(systems), ids);
    let calls = calls(Some(systems), scope, &element_ids, |_| Some(me.id.clone()));
    let mut rels = Vec::new();
    for channel in &systems.channels {
        let Some(to) = element_ids.get(&channel.to) else {
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
        let via = channel
            .via
            .iter()
            .filter(|v| {
                scope
                    .as_ref()
                    .is_none_or(|s| v.features.iter().any(|f| s.ids.contains(f)))
            })
            .count();
        let mut rel = match (called.is_empty() && unattributed == 0, via) {
            (true, 0) => continue,
            (true, _) => Rel {
                from: me.id.clone(),
                to: to.clone(),
                label: String::new(),
                weight: 0,
            },
            (false, _) => contracts_rel(me.id.clone(), to.clone(), &called, unattributed),
        };
        if via > 0 {
            let through = format!("via {}", count(via, "module"));
            rel.label = if rel.label.is_empty() {
                through
            } else {
                format!("{}, {through}", rel.label)
            };
            rel.weight += via;
        }
        rels.push(rel);
    }
    if let Some(me_config) = systems.systems.iter().find(|s| s.is_self) {
        element_ids.insert(me_config.id.clone(), me.id.clone());
    }
    for (actor, element) in systems.actors.iter().zip(&actors) {
        element_ids.insert(actor.id.clone(), element.id.clone());
    }
    add_links(systems, &element_ids, &mut rels);
    rels.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Diagram {
        level: C4Level::System,
        title: format!("System context of {}", me.name),
        boundaries: Vec::new(),
        inner: vec![me],
        outer: actors.into_iter().chain(others).collect(),
        rels,
    }
}

fn add_links(systems: &SystemMap, element_ids: &BTreeMap<String, String>, rels: &mut Vec<Rel>) {
    for link in &systems.links {
        let (Some(from), Some(to)) = (element_ids.get(&link.from), element_ids.get(&link.to))
        else {
            continue;
        };
        let label = link.label.clone().unwrap_or_default();
        match rels.iter_mut().find(|r| &r.from == from && &r.to == to) {
            Some(_) if label.is_empty() => {}
            Some(rel) if rel.label.is_empty() => rel.label = label,
            Some(rel) => rel.label = format!("{label}, {}", rel.label),
            None => rels.push(Rel {
                from: from.clone(),
                to: to.clone(),
                label,
                weight: 1,
            }),
        }
    }
}

struct TopLayer<'g> {
    layer: &'g Layer,
    modules: Vec<(ModuleId, String)>,
}

fn top_layers<'g>(input: &'g EngineInput, scope: &Scope) -> Vec<TopLayer<'g>> {
    let layers: Vec<&Layer> = input.graph.layers.values().collect();
    let tops: Vec<&Layer> = layers
        .iter()
        .copied()
        .filter(|l| {
            !layers
                .iter()
                .any(|o| o.path != l.path && l.path.starts_with(&o.path))
        })
        .collect();
    let top_of = |path: &Path| tops.iter().position(|t| path.starts_with(&t.path));
    let mut depends: BTreeSet<(usize, usize)> = BTreeSet::new();
    for layer in &layers {
        let Some(from) = top_of(&layer.path) else {
            continue;
        };
        for dep in layer
            .depends_on
            .iter()
            .filter_map(|d| input.graph.layers.get(d))
        {
            match top_of(&dep.path) {
                Some(to) if to != from => {
                    depends.insert((from, to));
                }
                _ => {}
            }
        }
    }
    let mut left: Vec<usize> = (0..tops.len()).collect();
    let mut order = Vec::with_capacity(tops.len());
    while !left.is_empty() {
        let next = left
            .iter()
            .position(|&j| !left.iter().any(|&i| depends.contains(&(i, j))))
            .unwrap_or(0);
        order.push(left.remove(next));
    }
    let mut modules: Vec<Vec<(ModuleId, String)>> = vec![Vec::new(); tops.len()];
    for (&id, module) in input
        .graph
        .modules
        .iter()
        .filter(|(m, _)| in_scope(scope, **m))
    {
        let path = input.relative(&module.path);
        if let Some(top) = top_of(Path::new(&path)) {
            modules[top].push((id, path));
        }
    }
    order
        .into_iter()
        .map(|i| TopLayer {
            layer: tops[i],
            modules: std::mem::take(&mut modules[i]),
        })
        .collect()
}

fn layer_description(input: &EngineInput, top: &TopLayer) -> String {
    let mut slices = BTreeSet::new();
    let mut nested = BTreeSet::new();
    for container in top
        .modules
        .iter()
        .filter_map(|&(m, _)| input.graph.outer_container(m))
    {
        if container.kind == ContainerKind::Slice {
            slices.insert(container.id);
        } else if Path::new(&input.relative(&container.path)) != top.layer.path {
            nested.insert(container.id);
        }
    }
    let modules = count(top.modules.len(), "module");
    match (slices.len(), nested.len()) {
        (0, 0) => modules,
        (slices, 0) => format!("{modules}, {}", count(slices, "slice")),
        (slices, nested) => format!("{modules}, {}", count(slices + nested, "part")),
    }
}

fn layer_element(input: &EngineInput, top: &TopLayer, ids: &mut Ids) -> Element {
    Element {
        id: ids.take("ctr", &slashed(&top.layer.path)),
        kind: Kind::Container,
        name: top.layer.name.clone(),
        technology: "layer".to_owned(),
        description: layer_description(input, top),
    }
}

fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

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

struct Sized {
    element: Element,
    modules: usize,
}

fn cap(
    sized: Vec<Sized>,
    imports: &mut BTreeMap<(String, String), usize>,
    calls: &mut Calls,
    ids: &mut Ids,
) -> Vec<Element> {
    let mut sized: Vec<(usize, Sized)> = sized.into_iter().enumerate().collect();
    sized.sort_by(|(_, a), (_, b)| {
        b.modules
            .cmp(&a.modules)
            .then_with(|| a.element.name.cmp(&b.element.name))
    });
    if sized.len() > MAX_INNER {
        let rest = sized.split_off(MAX_INNER - 1);
        let more = Element {
            id: ids.take("more", "elements"),
            kind: rest[0].1.element.kind,
            name: format!("… {} more", rest.len()),
            technology: String::new(),
            description: count(rest.iter().map(|(_, s)| s.modules).sum(), "module"),
        };
        let folded: BTreeSet<&str> = rest.iter().map(|(_, s)| s.element.id.as_str()).collect();
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
        sized.push((
            usize::MAX,
            Sized {
                element: more,
                modules: 0,
            },
        ));
    }
    sized.sort_by_key(|&(position, _)| position);
    sized.into_iter().map(|(_, s)| s.element).collect()
}

fn rels(imports: BTreeMap<(String, String), usize>, calls: Calls) -> Vec<Rel> {
    let mut rels: Vec<Rel> = imports
        .into_iter()
        .map(|((from, to), n)| Rel {
            from,
            to,
            label: count(n, "import"),
            weight: n,
        })
        .chain(
            calls
                .into_iter()
                .map(|((from, to), called)| contracts_rel(from, to, &called, 0)),
        )
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
    let mut layer_ids: BTreeMap<ModuleId, String> = BTreeMap::new();
    let sized: Vec<Sized> = top_layers(input, scope)
        .iter()
        .filter(|top| !top.modules.is_empty())
        .map(|top| {
            let element = layer_element(input, top, ids);
            for &(m, _) in &top.modules {
                layer_ids.insert(m, element.id.clone());
            }
            Sized {
                element,
                modules: top.modules.len(),
            }
        })
        .collect();
    let (outer, system_ids) = other_systems(systems, ids);
    let layer_of = |m: ModuleId| layer_ids.get(&m).cloned();
    let mut imports = imports(input, scope, layer_of);
    let mut calls = calls(systems, scope, &system_ids, layer_of);
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
    let tops = top_layers(input, scope);
    let mut known: Vec<&str> = tops.iter().map(|t| t.layer.name.as_str()).collect();
    known.sort_unstable();
    let known = known.join(", ");
    let wanted = wanted.ok_or_else(|| {
        miette::miette!("the component level needs `--container NAME`; containers: {known}")
    })?;
    let top = tops
        .iter()
        .find(|t| t.layer.name == wanted)
        .ok_or_else(|| miette::miette!("unknown container {wanted:?}; known: {known}"))?;
    let base = slashed(&top.layer.path);
    let mut children: BTreeMap<&str, (usize, bool)> = BTreeMap::new();
    let mut child_of: BTreeMap<ModuleId, &str> = BTreeMap::new();
    for (m, path) in &top.modules {
        let Some(rest) = path
            .strip_prefix(base.as_str())
            .and_then(|r| r.strip_prefix('/'))
        else {
            continue;
        };
        let (child, is_file) = rest
            .split_once('/')
            .map_or((rest, true), |(c, _)| (c, false));
        let entry = children.entry(child).or_insert((0, is_file));
        entry.0 += 1;
        child_of.insert(*m, child);
    }
    let boundary = layer_element(input, top, ids);
    let mut child_ids: BTreeMap<&str, String> = BTreeMap::new();
    let sized: Vec<Sized> = children
        .into_iter()
        .map(|(child, (modules, is_file))| {
            let path = format!("{base}/{child}");
            let id = ids.take("cmp", &path);
            child_ids.insert(child, id.clone());
            let technology = if is_file {
                "file"
            } else if input
                .graph
                .layers
                .values()
                .any(|l| slashed(&l.path) == path)
            {
                "layer"
            } else if top.layer.slices {
                "slice"
            } else {
                "folder"
            };
            Sized {
                element: Element {
                    id,
                    kind: Kind::Component,
                    name: child.to_owned(),
                    technology: technology.to_owned(),
                    description: count(modules, "module"),
                },
                modules,
            }
        })
        .collect();
    let mut imports = imports(input, scope, |m| child_ids.get(child_of.get(&m)?).cloned());
    let mut calls = Calls::new();
    let inner = cap(sized, &mut imports, &mut calls, ids);
    Ok(Diagram {
        level: C4Level::Component,
        title: format!("Components of {}", top.layer.name),
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
}
