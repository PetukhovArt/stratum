use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use stratum_core::{
    ids::LayerId,
    purity::Purity,
    severity::Severity,
    types::{Container, ContainerKind, Layer, Module},
    visibility::VisibilityScope,
};
use stratum_engine::{CONFIG_FILE, EngineInput, Finding, paths};
use stratum_graph::{CompoundGraph, direct_dependencies, direct_dependents};
use stratum_rules::territory::{PlannedSlice, Region, Site, Territories};

use crate::{cli::BriefFormat, commands::lint};

const SHOWN_PER_LIST: usize = 2;

#[derive(Debug, Serialize)]
struct Brief {
    version: u32,
    file: String,
    status: Status,
    layer: Option<String>,
    container: Option<ContainerBrief>,
    slice: Option<ContainerBrief>,
    purity: Option<u8>,
    visibility: Option<String>,
    may_import_layers: Vec<String>,
    max_import_purity: Option<u8>,
    private_zones: Vec<String>,
    may_not_import: Vec<Forbidden>,
    imported_by_layers: Vec<String>,
    imports: usize,
    violating_imports: Vec<ViolatingImport>,
    dependents: usize,
    violations: Vec<FileViolation>,
    layers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Module,
    NewFile,
    NotASourceModule,
    OutsideLayers,
}

#[derive(Debug, Serialize)]
struct ContainerBrief {
    path: String,
    kind: ContainerKind,
    stage: u8,
    entry: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Forbidden {
    SiblingSlices { parent: String },
    SiblingSubUnits { parent: String },
    ContainerInternals,
}

#[derive(Debug, Serialize)]
struct ViolatingImport {
    target: String,
    rule: String,
    severity: Severity,
}

#[derive(Debug, Serialize)]
struct FileViolation {
    rule: String,
    severity: Severity,
    message: String,
    on_import: bool,
}

pub fn run(
    file: &Utf8Path,
    root: Option<&Utf8Path>,
    format: BriefFormat,
    config_path: Option<&Utf8Path>,
) -> miette::Result<i32> {
    let file_abs = absolutize(file)?;
    let root = match root {
        Some(r) => absolutize(r)?,
        None => discover_root(&file_abs)?,
    };
    let rel = relative_to(&file_abs, &root)?;
    let input = lint::build(&root, &lint::load_config(&root, config_path)?)?;
    let brief = brief(&input, None, &file_abs, &rel)?;
    match format {
        BriefFormat::Agent => print!("{}", render_agent(&brief)),
        BriefFormat::Json => {
            let s = serde_json::to_string_pretty(&brief).map_err(|e| miette::miette!("{e}"))?;
            println!("{s}");
        }
    }
    Ok(0)
}

/// `context REL --format agent` for a root-relative `rel`, over `findings` the caller computed once.
pub fn agent_brief(input: &EngineInput, findings: &[Finding], rel: &str) -> miette::Result<String> {
    let file_abs = absolutize(&input.project_root.join(rel))?;
    let rel = relative_to(&file_abs, &input.project_root)?;
    Ok(render_agent(&brief(
        input,
        Some(findings),
        &file_abs,
        &rel,
    )?))
}

fn relative_to(file_abs: &Utf8Path, root: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let rel = file_abs.strip_prefix(root).map_err(|_| {
        miette::miette!("{file_abs} is outside the project root {root}; pass --root")
    })?;
    Ok(Utf8PathBuf::from(rel.as_str().replace('\\', "/")))
}

fn brief(
    input: &EngineInput,
    findings: Option<&[Finding]>,
    file_abs: &Utf8Path,
    rel: &Utf8Path,
) -> miette::Result<Brief> {
    let graph = &input.graph;
    let layers: Vec<Layer> = graph.layers.values().cloned().collect();
    let file = Path::new(file_abs.as_str());
    let module = graph.modules.values().find(|m| file == m.path);
    let container = module
        .and_then(|m| graph.containers.get(&m.container))
        .or_else(|| graph.container_at(file));
    let layer_id = module.map(|m| m.layer).or(container.map(|c| c.layer));
    let status = match (module, layer_id) {
        (Some(_), _) => Status::Module,
        (None, None) => Status::OutsideLayers,
        (None, Some(_)) if file_abs.exists() => Status::NotASourceModule,
        (None, Some(_)) => Status::NewFile,
    };
    let mut brief = Brief {
        version: 2,
        file: rel.to_string(),
        status,
        layer: None,
        container: None,
        slice: None,
        purity: None,
        visibility: None,
        may_import_layers: vec![],
        max_import_purity: None,
        private_zones: vec![],
        may_not_import: vec![],
        imported_by_layers: vec![],
        imports: 0,
        violating_imports: vec![],
        dependents: 0,
        violations: vec![],
        layers: layers
            .iter()
            .map(|l| format!("{}={}", l.name, l.path.display()))
            .collect(),
    };
    let Some(layer_id) = layer_id else {
        return Ok(brief);
    };
    let territories = Territories::new(graph, &input.config, &input.project_root, &input.registry)
        .map_err(|e| miette::miette!("{e}"))?;
    let territory = territories.of(module.map_or(Site::Planned(file), |m| Site::Module(m.id)));
    brief.layer = Some(layer_name(graph, layer_id));
    let planned = match module {
        Some(_) => None,
        None => territories.planned_slice(file),
    };
    fill_containers(&mut brief, input, container, planned);
    brief.private_zones = territory
        .visible_zones
        .iter()
        .map(|z| input.relative(&graph.private_zones()[z.index()].path))
        .collect();
    brief.purity = Some(module.map_or(Purity::DEFAULT, |m| m.purity).rank());
    brief.max_import_purity = Some(territory.max_import_purity);
    brief.visibility = Some(module.map_or_else(
        || visibility_name(input, &visibility_at(input, file_abs)),
        |m| visibility_name(input, &m.visibility),
    ));
    brief.may_import_layers = territory
        .may_import_layers
        .iter()
        .map(|&id| layer_name(graph, id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    brief.imported_by_layers = territory
        .imported_by_layers
        .iter()
        .map(|&id| layer_name(graph, id))
        .collect();
    brief.may_not_import = territory
        .may_not
        .iter()
        .map(|f| match &f.region {
            Region::Siblings { parent, kind, .. } => {
                let parent = graph
                    .containers
                    .get(parent)
                    .map(|p| input.relative(&p.path))
                    .unwrap_or_default();
                if *kind == ContainerKind::SubUnit {
                    Forbidden::SiblingSubUnits { parent }
                } else {
                    Forbidden::SiblingSlices { parent }
                }
            }
            Region::Internals => Forbidden::ContainerInternals,
        })
        .collect();
    if let Some(m) = module {
        fill_from_graph(&mut brief, input, findings, m)?;
    }
    Ok(brief)
}

fn container_brief(input: &EngineInput, c: &Container) -> ContainerBrief {
    ContainerBrief {
        path: input.relative(&c.path),
        kind: c.kind,
        stage: c.stage.rank(),
        entry: c
            .entry
            .and_then(|id| input.graph.modules.get(&id))
            .map(|m| input.relative(&m.path)),
    }
}

fn fill_containers(
    brief: &mut Brief,
    input: &EngineInput,
    innermost: Option<&Container>,
    planned: Option<PlannedSlice>,
) {
    let graph = &input.graph;
    let chain: Vec<&Container> = std::iter::successors(innermost, |c| {
        c.parent.and_then(|p| graph.containers.get(&p))
    })
    .collect();
    brief.slice = chain
        .iter()
        .skip(1)
        .find(|c| c.kind == ContainerKind::Slice)
        .map(|c| container_brief(input, c));
    brief.container = planned
        .map(|s| ContainerBrief {
            path: input.relative(&s.path),
            kind: ContainerKind::Slice,
            stage: s.stage.rank(),
            entry: None,
        })
        .or_else(|| chain.first().map(|c| container_brief(input, c)));
}

fn fill_from_graph(
    brief: &mut Brief,
    input: &EngineInput,
    findings: Option<&[Finding]>,
    m: &Module,
) -> miette::Result<()> {
    let graph = &input.graph;
    brief.imports = direct_dependencies(graph, m.id).len();
    brief.dependents = direct_dependents(graph, m.id).len();
    let here = input.relative(&m.path);
    let mut seen_imports = BTreeSet::new();
    let computed;
    let findings = if let Some(f) = findings {
        f
    } else {
        computed = lint::findings(input)?;
        &computed
    };
    for f in findings {
        let from_here = f.edge.as_ref().is_some_and(|e| e.from == here);
        let touches = f.file == here || (f.edge.is_none() && f.members.contains(&here));
        if !(from_here || touches) {
            continue;
        }
        if let Some(edge) = f.edge.as_ref().filter(|_| from_here) {
            if seen_imports.insert((edge.to.clone(), f.rule.clone())) {
                brief.violating_imports.push(ViolatingImport {
                    target: edge.to.clone(),
                    rule: f.rule.clone(),
                    severity: f.severity,
                });
            }
        }
        brief.violations.push(FileViolation {
            rule: f.rule.clone(),
            severity: f.severity,
            message: f.message.clone(),
            on_import: from_here,
        });
    }
    brief.violations.sort_by(|a, b| b.severity.cmp(&a.severity));
    brief
        .violating_imports
        .sort_by(|a, b| b.severity.cmp(&a.severity));
    Ok(())
}

fn render_agent(b: &Brief) -> String {
    let mut out = String::new();
    let Some(layer) = &b.layer else {
        let _ = writeln!(
            out,
            "{}: outside every layer, Stratum does not check it. Layers: {}",
            b.file,
            b.layers.join(", ")
        );
        return out;
    };
    let note = match b.status {
        Status::NewFile => " (new file)",
        Status::NotASourceModule => " (not a source module)",
        Status::Module | Status::OutsideLayers => "",
    };
    let _ = writeln!(
        out,
        "{}{note}: layer {layer}, purity {}, visibility {}",
        b.file,
        b.purity.unwrap_or(Purity::DEFAULT.rank()),
        b.visibility.as_deref().unwrap_or("public"),
    );
    if let Some(c) = &b.container {
        let _ = write!(out, "container {}", describe(c));
        if let Some(s) = &b.slice {
            let _ = write!(out, ", in slice {}", describe(s));
        }
        if b.may_not_import
            .iter()
            .any(|f| matches!(f, Forbidden::ContainerInternals))
        {
            out.push_str(": import other containers only via their entry");
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "may import: layers {}; modules at purity <= {}",
        b.may_import_layers.join(", "),
        b.max_import_purity.unwrap_or(Purity::DEFAULT.rank())
    );
    if !b.private_zones.is_empty() {
        let _ = writeln!(
            out,
            "private zones visible here: {}",
            capped(&b.private_zones)
        );
    }
    if !b.may_not_import.is_empty() {
        let parts: Vec<String> = b.may_not_import.iter().map(forbidden_text).collect();
        let _ = writeln!(out, "may not import: {}", parts.join("; "));
    }
    let _ = write!(
        out,
        "imported by: layers {}",
        b.imported_by_layers.join(", ")
    );
    if b.status != Status::Module {
        out.push('\n');
        return out;
    }
    let _ = writeln!(out, "; {} dependent modules", b.dependents);
    let _ = write!(out, "imports: {} modules", b.imports);
    write_count_header(&mut out, b.violating_imports.len(), "violate");
    for i in b.violating_imports.iter().take(SHOWN_PER_LIST) {
        let _ = writeln!(out, "  -> {}  {} ({})", i.target, i.rule, i.severity);
    }
    let others: Vec<&FileViolation> = b.violations.iter().filter(|v| !v.on_import).collect();
    let _ = write!(out, "violations: {}", b.violations.len());
    if !b.violations.is_empty() {
        let _ = write!(out, " ({})", severity_counts(&b.violations));
    }
    if others.is_empty() {
        let _ = writeln!(out);
    } else {
        let _ = write!(out, "; not about imports");
        write_count_header(&mut out, others.len(), "");
    }
    for v in others.iter().take(SHOWN_PER_LIST) {
        let _ = writeln!(out, "  {} {}: {}", v.severity, v.rule, v.message);
    }
    out
}

fn describe(c: &ContainerBrief) -> String {
    let kind = match c.kind {
        ContainerKind::Layer => "layer",
        ContainerKind::Slice => "slice",
        ContainerKind::SubUnit => "sub-unit",
        ContainerKind::Nested => "nested",
    };
    let entry = c.entry.as_deref().map_or_else(
        || "no entry".to_string(),
        |e| {
            let name = e
                .strip_prefix(c.path.as_str())
                .and_then(|r| r.strip_prefix('/'))
                .unwrap_or_else(|| e.rsplit('/').next().unwrap_or(e));
            format!("entry {name}")
        },
    );
    format!("{} ({kind}, stage {}, {entry})", c.path, c.stage)
}

fn forbidden_text(f: &Forbidden) -> String {
    match f {
        Forbidden::SiblingSlices { parent } => format!("other slices of {parent}/, entry included"),
        Forbidden::SiblingSubUnits { parent } => {
            format!("other sub-units of {parent}/, entry included")
        }
        Forbidden::ContainerInternals => "files behind another container's entry".into(),
    }
}

fn capped(items: &[String]) -> String {
    let shown = items[..items.len().min(SHOWN_PER_LIST)].join(", ");
    match items.len().saturating_sub(SHOWN_PER_LIST) {
        0 => shown,
        more => format!("{shown} (+{more} more)"),
    }
}

fn write_count_header(out: &mut String, n: usize, verb: &str) {
    let sep = if verb.is_empty() { " " } else { ", " };
    let verb = if verb.is_empty() {
        String::new()
    } else {
        format!(" {verb}")
    };
    match n {
        0 => {
            let _ = writeln!(out, "{sep}none{verb}");
        }
        n if n > SHOWN_PER_LIST => {
            let _ = writeln!(out, "{sep}{n}{verb} (first {SHOWN_PER_LIST} shown):");
        }
        n => {
            let _ = writeln!(out, "{sep}{n}{verb}:");
        }
    }
}

fn severity_counts(vs: &[FileViolation]) -> String {
    [Severity::Error, Severity::Warning, Severity::Info]
        .iter()
        .filter_map(|s| {
            let n = vs.iter().filter(|v| v.severity == *s).count();
            (n > 0).then(|| format!("{n} {s}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn layer_name(graph: &CompoundGraph, id: LayerId) -> String {
    graph
        .layers
        .get(&id)
        .map_or_else(|| format!("layer#{}", id.raw()), |l| l.name.clone())
}

fn visibility_name(input: &EngineInput, v: &VisibilityScope) -> String {
    match v {
        VisibilityScope::Public => "public".into(),
        VisibilityScope::Private { root } => format!("private to {}", input.relative(root)),
    }
}

fn visibility_at(input: &EngineInput, file_abs: &Utf8Path) -> VisibilityScope {
    file_abs
        .ancestors()
        .skip(1)
        .take_while(|d| *d != input.project_root)
        .find(|d| {
            d.file_name()
                .is_some_and(|n| input.config.structure.is_private(n))
        })
        .and_then(Utf8Path::parent)
        .map_or(VisibilityScope::Public, |root| VisibilityScope::Private {
            root: root.as_std_path().to_path_buf(),
        })
}

fn absolutize(path: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|e| miette::miette!("{e}"))?;
        Utf8PathBuf::from_path_buf(cwd)
            .map_err(|p| miette::miette!("non-UTF-8 cwd {}", p.display()))?
            .join(path)
    };
    let mut existing = joined.as_path();
    let mut missing = Vec::new();
    loop {
        if let Ok(mut out) = paths::canonicalize(existing) {
            for part in missing.iter().rev() {
                out.push(part);
            }
            return Ok(out);
        }
        let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
            return Ok(joined);
        };
        missing.push(name);
        existing = parent;
    }
}

fn discover_root(file_abs: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let ancestors = || file_abs.ancestors().skip(1);
    if let Some(dir) = ancestors().find(|d| d.join(CONFIG_FILE).is_file()) {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = ancestors().find(|d| d.join("package.json").is_file()) {
        return Ok(dir.to_path_buf());
    }
    absolutize(Utf8Path::new("."))
}
