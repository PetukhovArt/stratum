use camino::Utf8Path;

use crate::{
    c4::{Diagram, Request},
    cli::C4Format,
    commands::lint,
};

pub fn run(
    root: &Utf8Path,
    config: Option<&Utf8Path>,
    format: C4Format,
    request: &Request,
) -> miette::Result<()> {
    let input = lint::build(root, &lint::load_config(root, config)?)?;
    print!("{}", Diagram::of(&input, request)?.render(format));
    Ok(())
}
