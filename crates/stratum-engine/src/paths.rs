use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};

const VERBATIM_PREFIX: &str = r"\\?\";

pub fn canonicalize(path: &Utf8Path) -> std::io::Result<Utf8PathBuf> {
    path.canonicalize_utf8()
        .map(|p| Utf8PathBuf::from(strip_verbatim(p.as_str())))
}

#[must_use]
pub fn strip_verbatim(path: &str) -> &str {
    path.strip_prefix(VERBATIM_PREFIX).unwrap_or(path)
}

#[must_use]
pub fn relative(path: &Path, root: &Utf8Path) -> String {
    let path = path.to_string_lossy();
    let path = strip_verbatim(&path).replace('\\', "/");
    let root = strip_verbatim(root.as_str()).replace('\\', "/");
    let root = root.trim_end_matches('/');
    match path.strip_prefix(root) {
        Some(rest) if rest.starts_with('/') => rest[1..].to_owned(),
        _ => path,
    }
}
