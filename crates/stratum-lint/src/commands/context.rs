use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use stratum_config::Config;
use stratum_core::{
    ids::{LayerId, ModuleId},
    severity::Severity,
    types::{Layer, Module},
    violation::Violation,
    visibility::VisibilityScope,
};
use stratum_graph::{CompoundGraph, assign_layer, direct_dependencies, direct_dependents};

use crate::{
    cli::BriefFormat,
    engine::{EngineInput, RuleEngine},
    pipeline, zero_config,
};

const DEFAULT_STAGE: u8 = 2;
const SHOWN_PER_LIST: usize = 3;

#[derive(Debug, Serialize)]
struct Brief {
    version: u32,
    file: String,
    status: Status,
    layer: Option<String>,
    container: Option<String>,
    stage: Option<u8>,
    visibility: Option<String>,
    may_import_layers: Vec<String>,
    max_import_stage: Option<u8>,
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
    let rel = file_abs.strip_prefix(&root).map_err(|_| {
        miette::miette!("{file_abs} is outside the project root {root}; pass --root")
    })?;
    let rel = Utf8PathBuf::from(rel.as_str().replace('\\', "/"));
    let config = load_config(&root, config_path)?;
    let input = pipeline::build(&root, &config).map_err(|e| miette::miette!("{e}"))?;
    let brief = brief(&input, &root, &file_abs, &rel);
    match format {
        BriefFormat::Agent => print!("{}", render_agent(&brief)),
        BriefFormat::Json => {
            let s = serde_json::to_string_pretty(&brief).map_err(|e| miette::miette!("{e}"))?;
            println!("{s}");
        }
    }
    Ok(0)
}

fn brief(input: &EngineInput, root: &Utf8Path, file_abs: &Utf8Path, rel: &Utf8Path) -> Brief {
    let graph = &input.graph;
    let layers: Vec<Layer> = graph.layers.values().cloned().collect();
    let module = graph
        .modules
        .values()
        .find(|m| Path::new(file_abs.as_str()) == m.path);
    let layer_id = module
        .map(|m| m.layer)
        .or_else(|| assign_layer(&layers, rel));
    let status = match (module, layer_id) {
        (Some(_), _) => Status::Module,
        (None, None) => Status::OutsideLayers,
        (None, Some(_)) if file_abs.exists() => Status::NotASourceModule,
        (None, Some(_)) => Status::NewFile,
    };
    let mut brief = Brief {
        version: 1,
        file: rel.to_string(),
        status,
        layer: None,
        container: None,
        stage: None,
        visibility: None,
        may_import_layers: vec![],
        max_import_stage: None,
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
        return brief;
    };
    let stage = module.map_or(DEFAULT_STAGE, |m| m.stage.rank());
    brief.layer = Some(layer_name(graph, layer_id));
    brief.container = Some(
        module
            .and_then(|m| graph.containers.get(&m.container))
            .map_or_else(|| layer_name(graph, layer_id), |c| c.name.clone()),
    );
    brief.stage = Some(stage);
    brief.max_import_stage = Some(stage);
    brief.visibility = Some(module.map_or_else(
        || "public".to_string(),
        |m| visibility_name(graph, &m.visibility),
    ));
    brief.may_import_layers = std::iter::once(layer_id)
        .chain(
            graph
                .layers
                .get(&layer_id)
                .map(|l| l.depends_on.clone())
                .unwrap_or_default(),
        )
        .map(|id| layer_name(graph, id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    brief.imported_by_layers = layers
        .iter()
        .filter(|l| l.id == layer_id || l.depends_on.contains(&layer_id))
        .map(|l| l.name.clone())
        .collect();
    if let Some(m) = module {
        fill_from_graph(&mut brief, input, root, m);
    }
    brief
}

fn fill_from_graph(brief: &mut Brief, input: &EngineInput, root: &Utf8Path, m: &Module) {
    let graph = &input.graph;
    brief.imports = direct_dependencies(graph, m.id).len();
    brief.dependents = direct_dependents(graph, m.id).len();
    let slug = |v: &Violation| {
        input.registry.by_id(v.rule).map_or_else(
            || format!("rule#{}", v.rule.raw()),
            |r| r.slug().to_string(),
        )
    };
    let mut seen_imports = BTreeSet::new();
    for v in RuleEngine::run(input) {
        let from_here = v.edge.as_ref().is_some_and(|e| e.from == m.id);
        let touches = v.file == m.path || (v.edge.is_none() && v.modules.contains(&m.id));
        if !(from_here || touches) {
            continue;
        }
        if let Some(edge) = v.edge.as_ref().filter(|_| from_here) {
            let target = module_rel(graph, root, edge.to);
            if seen_imports.insert((target.clone(), v.rule.raw())) {
                brief.violating_imports.push(ViolatingImport {
                    target,
                    rule: slug(&v),
                    severity: v.severity,
                });
            }
        }
        brief.violations.push(FileViolation {
            rule: slug(&v),
            severity: v.severity,
            message: v.message,
            on_import: from_here,
        });
    }
    brief.violations.sort_by(|a, b| b.severity.cmp(&a.severity));
    brief
        .violating_imports
        .sort_by(|a, b| b.severity.cmp(&a.severity));
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
        "{}{note}: layer {layer}, container {}, stage {}, visibility {}",
        b.file,
        b.container.as_deref().unwrap_or("-"),
        b.stage.unwrap_or(DEFAULT_STAGE),
        b.visibility.as_deref().unwrap_or("public"),
    );
    let _ = writeln!(
        out,
        "may import: layers {}; modules at stage <= {}",
        b.may_import_layers.join(", "),
        b.max_import_stage.unwrap_or(DEFAULT_STAGE)
    );
    let _ = writeln!(
        out,
        "imported by: layers {}",
        b.imported_by_layers.join(", ")
    );
    if b.status != Status::Module {
        return out;
    }
    let _ = write!(out, "imports: {} modules", b.imports);
    write_count_header(&mut out, b.violating_imports.len(), "violate");
    for i in b.violating_imports.iter().take(SHOWN_PER_LIST) {
        let _ = writeln!(out, "  -> {}  {} ({})", i.target, i.rule, i.severity);
    }
    let _ = writeln!(out, "dependents: {} modules", b.dependents);
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

fn visibility_name(graph: &CompoundGraph, v: &VisibilityScope) -> String {
    match v {
        VisibilityScope::Public => "public".into(),
        VisibilityScope::Shared => "shared".into(),
        VisibilityScope::Layer { id } => format!("layer {}", layer_name(graph, *id)),
        VisibilityScope::Container { id } => format!(
            "container {}",
            graph
                .containers
                .get(id)
                .map_or_else(|| format!("#{}", id.raw()), |c| c.name.clone())
        ),
    }
}

fn module_rel(graph: &CompoundGraph, root: &Utf8Path, id: ModuleId) -> String {
    graph.modules.get(&id).map_or_else(
        || format!("module#{}", id.raw()),
        |m| {
            let p = m.path.to_string_lossy();
            p.strip_prefix(root.as_str())
                .unwrap_or(&p)
                .trim_start_matches(['/', '\\'])
                .replace('\\', "/")
        },
    )
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
        if let Ok(canon) = existing.canonicalize_utf8() {
            let mut out = strip_extended_prefix(&canon);
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

fn strip_extended_prefix(p: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(p.as_str().strip_prefix(r"\\?\").unwrap_or(p.as_str()))
}

fn discover_root(file_abs: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let ancestors = || file_abs.ancestors().skip(1);
    if let Some(dir) = ancestors().find(|d| d.join("stratum.config.jsonc").is_file()) {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = ancestors().find(|d| d.join("package.json").is_file()) {
        return Ok(dir.to_path_buf());
    }
    absolutize(Utf8Path::new("."))
}

fn load_config(root: &Utf8Path, config_path: Option<&Utf8Path>) -> miette::Result<Config> {
    let default = root.join("stratum.config.jsonc");
    let path = match config_path {
        Some(p) => Some(p.to_path_buf()),
        None => default.exists().then_some(default),
    };
    match path {
        Some(p) => stratum_config::parse_file(&p).map_err(|e| miette::miette!("{e}")),
        None => Ok(zero_config::infer(root)),
    }
}
