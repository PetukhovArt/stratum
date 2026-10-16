use camino::{Utf8Path, Utf8PathBuf};
use stratum_engine::{BASELINE_FILE, Baseline};

use crate::commands::lint;
use crate::git;

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

const SHOWN_GROWTHS: usize = 20;

pub fn check(root: &Utf8Path, out: Option<&Utf8Path>, reference: &str) -> miette::Result<i32> {
    let (head_path, rel) = match out {
        Some(out) => (out.to_path_buf(), slash_path_under(root, out)?),
        None => (lint::default_baseline_path(root), BASELINE_FILE.to_owned()),
    };
    let head = Baseline::load(&head_path).map_err(|e| miette::miette!("{e}"))?;
    let Some(text) = git::show(root, reference, &rel)? else {
        println!(
            "Baseline {rel} is introduced: absent at {reference}, {} accepted violations",
            head.len()
        );
        return Ok(0);
    };
    let origin = Utf8PathBuf::from(format!("{reference}:{rel}"));
    let base = Baseline::parse(&text, &origin).map_err(|e| miette::miette!("{e}"))?;

    let grown = head.grown_since(&base);
    if grown.is_empty() {
        println!("Baseline {rel} did not grow since {reference}");
        return Ok(0);
    }
    let added: usize = grown.iter().map(|g| g.head - g.base).sum();
    println!("Baseline {rel} grew by {added} since {reference}:");
    for g in grown.iter().take(SHOWN_GROWTHS) {
        let fp = &g.fingerprint;
        println!(
            "+{} {} {} {}",
            g.head - g.base,
            fp.rule,
            fp.file,
            fp.message
        );
    }
    if grown.len() > SHOWN_GROWTHS {
        println!("... {} more", grown.len() - SHOWN_GROWTHS);
    }
    Ok(1)
}

fn slash_path_under(root: &Utf8Path, path: &Utf8Path) -> miette::Result<String> {
    let absolute = |p: &Utf8Path| {
        std::path::absolute(p).map_err(|e| miette::miette!("Could not resolve {p}: {e}"))
    };
    let (root_abs, path_abs) = (absolute(root)?, absolute(path)?);
    let rel = path_abs.strip_prefix(&root_abs).map_err(|_| {
        miette::miette!("Baseline {path} is outside ROOT {root}; git cannot show it at a ref")
    })?;
    rel.components()
        .map(|c| {
            c.as_os_str()
                .to_str()
                .ok_or_else(|| miette::miette!("Baseline path {path} is not UTF-8"))
        })
        .collect::<miette::Result<Vec<_>>>()
        .map(|parts| parts.join("/"))
}
