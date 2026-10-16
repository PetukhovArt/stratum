use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use oxc_resolver::{
    FileMetadata, FileSystem, FileSystemOs, FsCache, ResolveOptions, ResolverGeneric,
    TsconfigOptions, TsconfigReferences,
};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("Could not resolve {specifier:?} from {from}: {reason}")]
    Unresolved {
        specifier: String,
        from: Utf8PathBuf,
        reason: String,
    },
    #[error("Resolver failure: {0}")]
    Internal(String),
}

/// Files that differ from the disk, by absolute path: `Some` text replaces or adds the file,
/// `None` hides it. Paths not in the overlay read through to the disk.
#[derive(Debug, Default)]
pub struct Overlay {
    files: HashMap<PathBuf, Option<String>>,
    dirs: HashSet<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlaid<'a> {
    Disk,
    Absent,
    Text(&'a str),
}

impl Overlay {
    #[must_use]
    pub fn new(files: impl IntoIterator<Item = (PathBuf, Option<String>)>) -> Self {
        let files: HashMap<PathBuf, Option<String>> = files.into_iter().collect();
        let dirs = files
            .iter()
            .filter(|(_, text)| text.is_some())
            .flat_map(|(path, _)| path.ancestors().skip(1).map(Path::to_path_buf))
            .collect();
        Self { files, dirs }
    }

    #[must_use]
    pub fn get(&self, path: &Path) -> Overlaid<'_> {
        match self.files.get(path) {
            None => Overlaid::Disk,
            Some(None) => Overlaid::Absent,
            Some(Some(text)) => Overlaid::Text(text),
        }
    }

    pub fn present(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.files
            .iter()
            .filter_map(|(path, text)| Some((path.as_path(), text.as_deref()?)))
    }

    fn exists(&self, path: &Path) -> bool {
        match self.get(path) {
            Overlaid::Disk => path.exists(),
            Overlaid::Absent => false,
            Overlaid::Text(_) => true,
        }
    }
}

#[derive(Debug, Default)]
struct OverlayFs(Arc<Overlay>);

impl OverlayFs {
    fn metadata_with(
        &self,
        path: &Path,
        disk: impl FnOnce(&Path) -> io::Result<FileMetadata>,
    ) -> io::Result<FileMetadata> {
        match self.0.get(path) {
            Overlaid::Text(_) => Ok(FileMetadata::new(true, false, false)),
            Overlaid::Absent => Err(io::ErrorKind::NotFound.into()),
            Overlaid::Disk if self.0.dirs.contains(path) => {
                Ok(FileMetadata::new(false, true, false))
            }
            Overlaid::Disk => disk(path),
        }
    }
}

impl FileSystem for OverlayFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        match self.0.get(path) {
            Overlaid::Text(text) => Ok(text.to_owned()),
            Overlaid::Absent => Err(io::ErrorKind::NotFound.into()),
            Overlaid::Disk => FileSystemOs::read_to_string(path),
        }
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata_with(path, FileSystemOs::metadata)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata_with(path, FileSystemOs::symlink_metadata)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        FileSystemOs::read_link(path)
    }
}

/// Project-scoped path alias / module resolver.
#[derive(Debug)]
pub struct PathResolver {
    inner: ResolverGeneric<FsCache<OverlayFs>>,
}

impl PathResolver {
    /// Build a resolver rooted at `project_root`. If a `tsconfig.json` exists,
    /// its `paths` are honoured.
    #[must_use]
    pub fn new(project_root: &Utf8Path) -> Self {
        Self::with_overlay(project_root, Arc::default())
    }

    #[must_use]
    pub fn with_overlay(project_root: &Utf8Path, overlay: Arc<Overlay>) -> Self {
        let tsconfig = project_root.join("tsconfig.json");
        let opts = ResolveOptions {
            extensions: vec![
                ".ts".into(),
                ".tsx".into(),
                ".mts".into(),
                ".cts".into(),
                ".js".into(),
                ".jsx".into(),
                ".mjs".into(),
                ".cjs".into(),
            ],
            extension_alias: vec![
                (".mjs".into(), vec![".mts".into(), ".mjs".into()]),
                (".cjs".into(), vec![".cts".into(), ".cjs".into()]),
            ],
            condition_names: vec!["import".into(), "node".into(), "default".into()],
            main_files: vec!["index".into()],
            tsconfig: if overlay.exists(tsconfig.as_std_path()) {
                Some(TsconfigOptions {
                    config_file: tsconfig.into_std_path_buf(),
                    references: TsconfigReferences::Auto,
                })
            } else {
                None
            },
            ..ResolveOptions::default()
        };
        Self {
            inner: ResolverGeneric::new_with_cache(
                Arc::new(FsCache::new(OverlayFs(overlay))),
                opts,
            ),
        }
    }

    /// Resolve `specifier` as referenced from the file at `from`.
    /// Returns an absolute UTF-8 path or an error.
    ///
    /// # Errors
    /// Returns `ResolveError::Unresolved` when the specifier cannot be resolved
    /// (missing file, alias not configured, external package not in `node_modules`).
    /// Returns `ResolveError::Internal` when the resolver returns a path that is
    /// not valid UTF-8.
    pub fn resolve(&self, from: &Utf8Path, specifier: &str) -> Result<Utf8PathBuf, ResolveError> {
        let from_dir = from.parent().unwrap_or(Utf8Path::new("."));
        match self.inner.resolve(from_dir.as_std_path(), specifier) {
            Ok(res) => {
                let path = res.path();
                Utf8PathBuf::from_path_buf(path.to_path_buf())
                    .map_err(|p| ResolveError::Internal(format!("non-UTF-8 path: {}", p.display())))
            }
            Err(e) => Err(ResolveError::Unresolved {
                specifier: specifier.to_string(),
                from: from.to_path_buf(),
                reason: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn fixture_root() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-ts")
            .canonicalize_utf8()
            .unwrap()
    }

    fn ends_with_portable(got: &Utf8Path, suffix: &str) -> bool {
        got.as_str().replace('\\', "/").ends_with(suffix)
    }

    #[test]
    fn resolves_at_alias() {
        let root = fixture_root();
        let r = PathResolver::new(&root);
        let from = root.join("src/app/main.ts");
        let got = r.resolve(&from, "@/features/cart").unwrap();
        assert!(
            ends_with_portable(&got, "src/features/cart/index.ts"),
            "got: {got}"
        );
    }

    #[test]
    fn resolves_tilde_shared_alias() {
        let root = fixture_root();
        let r = PathResolver::new(&root);
        let from = root.join("src/app/main.ts");
        let got = r.resolve(&from, "~shared/api/http").unwrap();
        assert!(
            ends_with_portable(&got, "src/shared/api/http.ts"),
            "got: {got}"
        );
    }

    #[test]
    fn resolves_relative_import() {
        let root = fixture_root();
        let r = PathResolver::new(&root);
        let from = root.join("src/features/cart/index.ts");
        let got = r.resolve(&from, "./cart").unwrap();
        assert!(
            ends_with_portable(&got, "src/features/cart/cart.tsx"),
            "got: {got}"
        );
    }

    #[test]
    fn esm_and_cjs_specifiers_resolve_to_mts_and_cts_sources() {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/tiny-mts")
            .canonicalize_utf8()
            .unwrap();
        let r = PathResolver::new(&root);
        let from = root.join("src/a.mts");
        let mts = r.resolve(&from, "./b.mjs").unwrap();
        let cts = r.resolve(&from, "./c.cjs").unwrap();
        assert!(ends_with_portable(&mts, "src/b.mts"), "got: {mts}");
        assert!(ends_with_portable(&cts, "src/c.cts"), "got: {cts}");
    }

    #[test]
    fn unresolved_external_is_error() {
        let root = fixture_root();
        let r = PathResolver::new(&root);
        let from = root.join("src/app/main.ts");
        let err = r.resolve(&from, "react").unwrap_err();
        let specifier = match &err {
            ResolveError::Unresolved { specifier, .. } => specifier.as_str(),
            ResolveError::Internal(_) => "",
        };
        assert_eq!(
            specifier, "react",
            "expected Unresolved variant, got {err:?}"
        );
    }
}
