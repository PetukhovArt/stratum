use std::fmt::Write as _;

use camino::Utf8Path;
use serde::Serialize;
use stratum_core::ids::ModuleId;
use stratum_engine::{EngineInput, Features};

use crate::{cli::FeaturesFormat, commands::lint};

/// `features` and `shared` with each module written by `module`; every list ascending.
#[derive(Debug, Serialize)]
pub struct Listing<T> {
    features: Vec<FeatureListing<T>>,
    shared: Vec<T>,
}

#[derive(Debug, Serialize)]
struct FeatureListing<T> {
    id: String,
    name: String,
    entry: Vec<T>,
    modules: Vec<T>,
}

pub fn listing<T: Ord>(features: &Features, module: impl Fn(ModuleId) -> T) -> Listing<T> {
    let sorted = |ids: &[ModuleId]| {
        let mut out: Vec<T> = ids.iter().map(|&m| module(m)).collect();
        out.sort();
        out
    };
    Listing {
        features: features
            .features()
            .iter()
            .map(|f| FeatureListing {
                id: f.id.clone(),
                name: f.name.clone(),
                entry: sorted(&f.entry),
                modules: sorted(&f.modules),
            })
            .collect(),
        shared: sorted(features.shared()),
    }
}

/// # Errors
/// The first id in `selected` that is not a feature, with the known ids in the message.
pub fn ensure_known(features: &Features, selected: &[String]) -> miette::Result<()> {
    let Some(unknown) = selected.iter().find(|id| features.get(id).is_none()) else {
        return Ok(());
    };
    let known: Vec<&str> = features.features().iter().map(|f| f.id.as_str()).collect();
    let known = if known.is_empty() {
        "none".to_owned()
    } else {
        known.join(", ")
    };
    Err(miette::miette!(
        "unknown feature {unknown:?}; known: {known}"
    ))
}

pub fn run(
    root: &Utf8Path,
    format: FeaturesFormat,
    config: Option<&Utf8Path>,
) -> miette::Result<i32> {
    let input = lint::build(root, &lint::load_config(root, config)?)?;
    let features = input.features().map_err(|e| miette::miette!("{e}"))?;
    match format {
        FeaturesFormat::Json => {
            let listing = listing(&features, |m| module_path(&input, m));
            let json =
                serde_json::to_string_pretty(&listing).map_err(|e| miette::miette!("{e}"))?;
            println!("{json}");
        }
        FeaturesFormat::Text => print!("{}", text(&input, &features)),
    }
    Ok(0)
}

fn module_path(input: &EngineInput, module: ModuleId) -> String {
    input
        .graph
        .modules
        .get(&module)
        .map_or_else(String::new, |m| input.relative(&m.path))
}

fn text(input: &EngineInput, features: &Features) -> String {
    let mut out = String::new();
    if features.features().is_empty() {
        out.push_str(
            "no features: declare `features` in the config, or a layer with `slices: true`\n",
        );
        return out;
    }
    let listing = listing(features, |m| module_path(input, m));
    for (feature, listed) in features.features().iter().zip(&listing.features) {
        let _ = writeln!(
            out,
            "{} ({}): {} modules, {} exclusive",
            feature.id,
            feature.name,
            feature.modules.len(),
            features.exclusive_count(feature)
        );
        for path in &listed.modules {
            let _ = writeln!(out, "  {path}");
        }
    }
    let mut shared: Vec<(String, Vec<&str>)> = features
        .shared()
        .iter()
        .map(|&m| (module_path(input, m), features.holders(m).collect()))
        .collect();
    shared.sort();
    let _ = writeln!(out, "shared: {} modules", shared.len());
    for (path, holders) in shared {
        let _ = writeln!(out, "  {path}: {}", holders.join(", "));
    }
    out
}
