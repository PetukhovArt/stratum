use std::fs;

use camino::Utf8Path;

use crate::{pipeline, zero_config};

pub fn run(root: &Utf8Path, out: &Utf8Path) -> miette::Result<()> {
    let config_path = root.join("stratum.config.jsonc");
    let config = if config_path.exists() {
        stratum_config::parse_file(&config_path).map_err(|e| miette::miette!("{e}"))?
    } else {
        zero_config::infer(root)
    };
    let input = pipeline::build(root, &config).map_err(|e| miette::miette!("{e}"))?;
    let snap = stratum_graph::snapshot_of(&input.graph);
    let json = serde_json::to_string_pretty(&snap).map_err(|e| miette::miette!("{e}"))?;
    fs::write(out, json).map_err(|e| miette::miette!("Could not write {out}: {e}"))?;
    println!("Wrote {out}");
    Ok(())
}
