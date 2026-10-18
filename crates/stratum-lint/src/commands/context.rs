use camino::{Utf8Path, Utf8PathBuf};
use stratum_engine::CONFIG_FILE;

use crate::{
    brief::{brief, relative_to, render_agent},
    cli::BriefFormat,
    commands::lint,
    cwd::absolutize,
};

pub fn run(
    file: &Utf8Path,
    root: Option<&Utf8Path>,
    format: BriefFormat,
    config_path: Option<&Utf8Path>,
) -> miette::Result<i32> {
    let file_abs = absolutize(file)?;
    let root = match root {
        Some(r) => absolutize(r)?,
        None => discover_root(&file_abs)?,
    };
    let rel = relative_to(&file_abs, &root)?;
    let input = lint::build(&root, &lint::load_config(&root, config_path)?)?;
    let brief = brief(&input, None, &file_abs, &rel)?;
    match format {
        BriefFormat::Agent => print!("{}", render_agent(&brief)),
        BriefFormat::Json => {
            let s = serde_json::to_string_pretty(&brief).map_err(|e| miette::miette!("{e}"))?;
            println!("{s}");
        }
    }
    Ok(0)
}

fn discover_root(file_abs: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let ancestors = || file_abs.ancestors().skip(1);
    if let Some(dir) = ancestors().find(|d| d.join(CONFIG_FILE).is_file()) {
        return Ok(dir.to_path_buf());
    }
    if let Some(dir) = ancestors().find(|d| d.join("package.json").is_file()) {
        return Ok(dir.to_path_buf());
    }
    absolutize(Utf8Path::new("."))
}
