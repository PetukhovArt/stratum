use camino::Utf8Path;

pub fn run(_prev: &Utf8Path, _now: &Utf8Path) -> miette::Result<()> {
    Err(miette::miette!(
        "stratum-lint diff is not implemented in Phase 4 — see a later phase"
    ))
}
