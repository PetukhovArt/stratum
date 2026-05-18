use camino::{Utf8Path, Utf8PathBuf};
use oxc_resolver::{ResolveOptions, Resolver, TsconfigOptions, TsconfigReferences};

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

/// Project-scoped path alias / module resolver.
#[derive(Debug)]
pub struct PathResolver {
    inner: Resolver,
    project_root: Utf8PathBuf,
}

impl PathResolver {
    /// Build a resolver rooted at `project_root`. If a `tsconfig.json` exists,
    /// its `paths` are honoured.
    #[must_use]
    pub fn new(project_root: &Utf8Path) -> Self {
        let tsconfig = project_root.join("tsconfig.json");
        let opts = ResolveOptions {
            extensions: vec![
                ".ts".into(),
                ".tsx".into(),
                ".js".into(),
                ".jsx".into(),
                ".mjs".into(),
                ".cjs".into(),
            ],
            condition_names: vec!["import".into(), "node".into(), "default".into()],
            main_files: vec!["index".into()],
            tsconfig: if tsconfig.exists() {
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
            inner: Resolver::new(opts),
            project_root: project_root.to_path_buf(),
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
        let from_dir = from.parent().unwrap_or_else(|| Utf8Path::new("."));
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

    #[must_use]
    pub fn project_root(&self) -> &Utf8Path {
        &self.project_root
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
