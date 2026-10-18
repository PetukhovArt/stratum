use camino::{Utf8Path, Utf8PathBuf};
use stratum_engine::paths;

/// `path` joined to the current directory when relative, with its longest existing prefix
/// canonicalized; the missing rest is appended as is, so the result may not exist.
pub fn absolutize(path: &Utf8Path) -> miette::Result<Utf8PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|e| miette::miette!("{e}"))?;
        Utf8PathBuf::from_path_buf(cwd)
            .map_err(|p| miette::miette!("non-UTF-8 cwd {}", p.display()))?
            .join(path)
    };
    let mut existing = joined.as_path();
    let mut missing = Vec::new();
    loop {
        if let Ok(mut out) = paths::canonicalize(existing) {
            for part in missing.iter().rev() {
                out.push(part);
            }
            return Ok(out);
        }
        let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
            return Ok(joined);
        };
        missing.push(name);
        existing = parent;
    }
}
