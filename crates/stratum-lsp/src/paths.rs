use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::Url;

#[must_use]
pub fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    if let Ok(p) = uri.to_file_path() {
        return Some(p);
    }
    let decoded = uri.as_str().replace("%3A", ":").replace("%3a", ":");
    Url::parse(&decoded).ok()?.to_file_path().ok()
}

#[must_use]
pub fn path_to_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(strip_verbatim(path)).ok()
}

#[must_use]
pub fn path_key(path: &Path) -> String {
    let s = strip_verbatim(path).to_string_lossy().replace('\\', "/");
    if cfg!(windows) { s.to_lowercase() } else { s }
}

fn strip_verbatim(path: &Path) -> PathBuf {
    PathBuf::from(stratum_engine::paths::strip_verbatim(
        &path.to_string_lossy(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn decodes_percent_encoded_drive_colon() {
        let uri = Url::parse("file:///c%3A/Users/x/project").unwrap();
        let path = uri_to_path(&uri).unwrap();
        assert_eq!(path_key(&path), "c:/users/x/project");
    }

    #[cfg(windows)]
    #[test]
    fn canonical_and_uri_paths_share_a_key() {
        let canonical = Path::new(r"\\?\C:\Users\X\src\a.ts");
        let from_uri = uri_to_path(&Url::parse("file:///c%3A/Users/X/src/a.ts").unwrap()).unwrap();
        assert_eq!(path_key(canonical), path_key(&from_uri));
    }
}
