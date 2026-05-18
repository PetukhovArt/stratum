use std::fs;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_core::ids::RuleId;

use crate::engine::build_engine;
use crate::module_view;
use crate::rule::RhaiRule;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("Could not read Rhai script {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Rhai compile error in {path}: {message}")]
    Compile { path: Utf8PathBuf, message: String },
}

/// First `RuleId` assigned to user plugins. Built-ins occupy `1..=9`; the gap
/// keeps registries diff-friendly in snapshots.
pub const PLUGIN_ID_FLOOR: u32 = 1000;

/// Read and compile `path` into a [`RhaiRule`] tagged with `slug` + `id`.
///
/// `slug` must outlive the resulting rule (the `Rule` trait demands `&'static
/// str`). Callers typically pass a leaked string they own for the lifetime of
/// the registry.
///
/// # Errors
/// - [`LoadError::Io`] if the file cannot be read.
/// - [`LoadError::Compile`] if Rhai rejects the script.
pub fn load(slug: &'static str, id: RuleId, path: &Utf8Path) -> Result<RhaiRule, LoadError> {
    let source = fs::read_to_string(path.as_std_path()).map_err(|e| LoadError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut engine = build_engine();
    module_view::register(&mut engine);
    let ast = engine.compile(&source).map_err(|e| LoadError::Compile {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    Ok(RhaiRule {
        id,
        slug,
        ast,
        engine: Arc::new(engine),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn rejects_missing_script() {
        let err = load(
            "custom/missing",
            RuleId::new(PLUGIN_ID_FLOOR),
            Utf8Path::new("does-not-exist.rhai"),
        );
        assert!(matches!(err, Err(LoadError::Io { .. })));
    }

    #[test]
    fn rejects_uncompilable_script() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = Utf8Path::from_path(tmp.path()).unwrap().to_path_buf();
        let mut f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        writeln!(f, "let x =;").unwrap();
        let err = load("custom/syntax", RuleId::new(PLUGIN_ID_FLOOR), &path);
        assert!(matches!(err, Err(LoadError::Compile { .. })));
    }
}
