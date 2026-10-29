use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use miette::{IntoDiagnostic, WrapErr};
use serde::Serialize;
use stratum_core::ids::ModuleId;

use super::{Api, ApiEdge, EdgeEnd, VisualizerAssets, edge_imports, source_lines};
use crate::brief;

/// `window.__STRATUM_STATIC__` in the exported `index.html`: which optional files `data/` holds.
#[derive(Debug, Serialize)]
struct StaticFlags {
    source: bool,
    systems: bool,
    drift: bool,
}

/// What [`Api::write_site`] wrote, for the command's summary line.
#[derive(Debug, Default)]
pub struct SiteSummary {
    pub modules: usize,
    pub edges: usize,
    pub sources: usize,
}

impl Api {
    /// Writes the visualizer and every answer of its API as files under `out`, so a plain file
    /// server serves the same views. `out/data/` is replaced. Source text is written only with
    /// `with_source`; otherwise the output holds paths, names and signatures and no file contents.
    ///
    /// # Errors
    /// When `out` holds files but no `.stratum-export` of an earlier export, before writing.
    pub fn write_site(&self, out: &Utf8Path, with_source: bool) -> miette::Result<SiteSummary> {
        claim(out)?;
        let data = out.join("data");
        if data.exists() {
            std::fs::remove_dir_all(&data)
                .into_diagnostic()
                .wrap_err_with(|| format!("cannot replace {data}"))?;
        }
        std::fs::create_dir_all(data.join("context")).into_diagnostic()?;
        let write = |rel: &str, body: &str| -> miette::Result<()> {
            let path = data.join(rel);
            std::fs::write(&path, body)
                .into_diagnostic()
                .wrap_err_with(|| format!("cannot write {path}"))
        };
        write("snapshot.json", &self.snapshot)?;
        write("violations.json", &self.violations)?;
        write("features.json", &self.features)?;
        if let Some(systems) = &self.systems {
            write("systems.json", systems)?;
        }
        if let Some(drift) = &self.drift {
            write("drift.json", drift)?;
        }
        let edges = self.module_edges();
        write("edges.json", &to_json(&edges)?)?;
        write("members.json", &to_json(&self.container_members())?)?;
        let mut summary = SiteSummary {
            modules: self.input.graph.modules.len(),
            edges: edges.values().map(BTreeMap::len).sum(),
            sources: 0,
        };
        for (id, module) in &self.input.graph.modules {
            let rel = self.input.relative(&module.path);
            if let Ok(text) = brief::agent_brief(&self.input, &self.findings, &rel) {
                write(&format!("context/{}.txt", id.0), &text)?;
            }
        }
        if with_source {
            std::fs::create_dir_all(data.join("source")).into_diagnostic()?;
            let mut index = BTreeMap::new();
            for rel in self.source_paths()? {
                let Ok(file) = source_lines(&self.input, &rel) else {
                    continue;
                };
                let n = index.len();
                write(&format!("source/{n}.json"), &to_json(&file)?)?;
                index.insert(rel, n);
            }
            summary.sources = index.len();
            write("source/index.json", &to_json(&index)?)?;
        }
        let flags = StaticFlags {
            source: with_source,
            systems: self.systems.is_some(),
            drift: self.drift.is_some(),
        };
        write_assets(out, &flags)?;
        Ok(summary)
    }

    /// `/api/edge?from=&to=` of every module pair that an import joins, by `from` then `to`.
    fn module_edges(&self) -> BTreeMap<u32, BTreeMap<u32, ApiEdge>> {
        let pairs: BTreeSet<(ModuleId, ModuleId)> = self
            .input
            .graph
            .modules
            .iter()
            .flat_map(|(&from, m)| {
                m.imports
                    .iter()
                    .filter_map(move |i| Some((from, i.target?)))
            })
            .collect();
        let mut edges: BTreeMap<u32, BTreeMap<u32, ApiEdge>> = BTreeMap::new();
        for (from, to) in pairs {
            let edge = edge_imports(&self.input, EdgeEnd::Module(from), EdgeEnd::Module(to));
            edges.entry(from.0).or_default().insert(to.0, edge);
        }
        edges
    }

    /// The modules of each container and of the containers nested in it: the frontend joins the
    /// module edges between two of these sets into `/api/edge?from_container=&to_container=`.
    fn container_members(&self) -> BTreeMap<u32, Vec<u32>> {
        let graph = &self.input.graph;
        graph
            .containers
            .keys()
            .map(|&c| {
                let mut ids: Vec<u32> = EdgeEnd::Container(c)
                    .modules(graph)
                    .into_iter()
                    .map(|m| m.0)
                    .collect();
                ids.sort_unstable();
                (c.0, ids)
            })
            .collect()
    }

    /// Every path the Code panel can open: each module, and each declaration of a cross-system
    /// contract (`system-id:rel/path`).
    fn source_paths(&self) -> miette::Result<BTreeSet<String>> {
        let mut paths: BTreeSet<String> = self
            .input
            .graph
            .modules
            .values()
            .map(|m| self.input.relative(&m.path))
            .collect();
        let systems = self.input.systems().map_err(|e| miette::miette!("{e}"))?;
        let decls = systems
            .iter()
            .flat_map(|map| &map.channels)
            .flat_map(|channel| &channel.contracts)
            .filter_map(|contract| contract.decl.as_ref());
        paths.extend(decls.map(|decl| decl.path.clone()));
        Ok(paths)
    }
}

/// The file an export leaves in its folder, so a later export may replace that folder's contents.
const EXPORT_MARKER: &str = ".stratum-export";

fn claim(out: &Utf8Path) -> miette::Result<()> {
    let marker = out.join(EXPORT_MARKER);
    let foreign = match std::fs::read_dir(out) {
        Ok(mut entries) => entries.next().is_some() && !marker.exists(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            return Err(e)
                .into_diagnostic()
                .wrap_err_with(|| format!("cannot read {out}"));
        }
    };
    if foreign {
        miette::bail!(
            "{out} is not empty and holds no earlier export ({EXPORT_MARKER} is missing); pass an empty folder or the folder of an earlier export"
        );
    }
    std::fs::create_dir_all(out).into_diagnostic()?;
    std::fs::write(&marker, "")
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot write {marker}"))
}

fn to_json(value: &impl Serialize) -> miette::Result<String> {
    serde_json::to_string(value).into_diagnostic()
}

/// The embedded frontend without its source maps; `index.html` gets `window.__STRATUM_STATIC__`.
fn write_assets(out: &Utf8Path, flags: &StaticFlags) -> miette::Result<()> {
    for name in VisualizerAssets::iter() {
        if name.ends_with(".map") {
            continue;
        }
        let Some(asset) = VisualizerAssets::get(&name) else {
            continue;
        };
        let path = out.join(name.as_ref());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).into_diagnostic()?;
        }
        let body = if name == "index.html" {
            static_index(&asset.data, flags)?.into_bytes()
        } else {
            asset.data.into_owned()
        };
        std::fs::write(&path, body)
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot write {path}"))?;
    }
    Ok(())
}

fn static_index(html: &[u8], flags: &StaticFlags) -> miette::Result<String> {
    let html = std::str::from_utf8(html).into_diagnostic()?;
    let flags = format!(
        "<head>\n    <script>window.__STRATUM_STATIC__ = {}</script>",
        to_json(flags)?
    );
    if !html.contains("<head>") {
        miette::bail!("the embedded index.html has no <head>");
    }
    Ok(html.replacen("<head>", &flags, 1))
}
