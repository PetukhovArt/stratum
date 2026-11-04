use camino::Utf8Path;

use crate::commands::{lint, visualize::Api};

pub fn run(
    root: &Utf8Path,
    config: Option<&Utf8Path>,
    out: &Utf8Path,
    with_source: bool,
    base: Option<&str>,
) -> miette::Result<()> {
    let input = lint::build(root, &lint::load_config(root, config)?)?;
    let api = Api::new(input, base, &[])?;
    let summary = api.write_site(out, with_source)?;
    let source = if with_source {
        format!(", source of {} files", summary.sources)
    } else {
        ", no source".to_owned()
    };
    eprintln!(
        "stratum-visualizer exported to {out}: {} modules, {} edges{source}",
        summary.modules, summary.edges
    );
    Ok(())
}
