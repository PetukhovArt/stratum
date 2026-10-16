use camino::Utf8Path;
use stratum_engine::Baseline;

use crate::commands::lint;

pub fn run(
    root: &Utf8Path,
    config_path: Option<&Utf8Path>,
    out: Option<&Utf8Path>,
    prune: bool,
) -> miette::Result<()> {
    let loaded = lint::load_config(root, config_path)?;
    let findings = lint::findings(&lint::build(root, &loaded)?)?;
    let out = out.map_or_else(|| lint::default_baseline_path(root), Utf8Path::to_path_buf);

    let next = if prune {
        let previous = Baseline::load(&out).map_err(|e| miette::miette!("{e}"))?;
        let pruned = previous.pruned(&findings);
        println!(
            "Pruned {} fixed entries; {} remain in {out}",
            previous.len() - pruned.len(),
            pruned.len()
        );
        pruned
    } else {
        let accepted = Baseline::from_findings(&findings);
        println!("Baselined {} violations in {out}", accepted.len());
        accepted
    };
    next.save(&out).map_err(|e| miette::miette!("{e}"))
}

pub fn check(_root: &Utf8Path, _out: Option<&Utf8Path>, _reference: &str) -> miette::Result<i32> {
    Err(miette::miette!(
        "stratum-lint baseline --check is not implemented yet"
    ))
}
