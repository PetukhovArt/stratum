use camino::Utf8Path;

use crate::{baseline::Baseline, commands::lint, pipeline};

pub fn run(
    root: &Utf8Path,
    config_path: Option<&Utf8Path>,
    out: Option<&Utf8Path>,
    prune: bool,
) -> miette::Result<()> {
    let (config, _) = lint::load_config(root, config_path)?;
    let input = pipeline::build(root, &config).map_err(|e| miette::miette!("{e}"))?;
    let findings = lint::findings(&input)?;
    let out = out.map_or_else(|| lint::default_baseline_path(root), Utf8Path::to_path_buf);

    let next = if prune {
        let previous = Baseline::load(&out)?;
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
    next.save(&out)
}
