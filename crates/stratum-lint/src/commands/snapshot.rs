use std::fs;

use camino::Utf8Path;

use crate::commands::lint;

pub fn run(root: &Utf8Path, out: &Utf8Path) -> miette::Result<()> {
    let input = lint::build(root, &lint::load_config(root, None)?)?;
    let snap = stratum_graph::snapshot_of(&input.graph);
    let json = serde_json::to_string_pretty(&snap).map_err(|e| miette::miette!("{e}"))?;
    fs::write(out, json).map_err(|e| miette::miette!("Could not write {out}: {e}"))?;
    println!("Wrote {out}");
    Ok(())
}
