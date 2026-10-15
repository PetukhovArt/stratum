use std::fs;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use rhai::{AST, Dynamic, Engine, Scope};
use stratum_core::ids::RuleId;
use stratum_rules::RuleDoc;

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
    #[error("Rhai script {path}: {message}")]
    Shape { path: Utf8PathBuf, message: String },
}

/// First `RuleId` assigned to user plugins. Built-ins occupy `1..=9`; the gap
/// keeps registries diff-friendly in snapshots.
pub const PLUGIN_ID_FLOOR: u32 = 1000;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DocSpec {
    summary: String,
    #[serde(default)]
    why: String,
    #[serde(default)]
    fix: String,
    #[serde(default)]
    options: Vec<String>,
}

/// Read and compile `path` into a [`RhaiRule`] tagged with `slug` + `id`.
///
/// `slug` must outlive the resulting rule (the `Rule` trait demands `&'static
/// str`). Callers typically pass a leaked string they own for the lifetime of
/// the registry.
///
/// # Errors
/// - [`LoadError::Io`] if the file cannot be read.
/// - [`LoadError::Compile`] if Rhai rejects the script.
/// - [`LoadError::Shape`] if the script defines no `check(m)` / `check(m, options)`,
///   or its optional `doc()` fails or returns a map other than
///   `#{ summary, why?, fix?, options? }`.
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
    let shape_error = |message: String| LoadError::Shape {
        path: path.to_path_buf(),
        message,
    };
    let takes_options = match arity(&ast, "check") {
        Some(1) => false,
        Some(2) => true,
        _ => {
            return Err(shape_error(
                "define fn check(m) or fn check(m, options)".into(),
            ));
        }
    };
    let doc = match arity(&ast, "doc") {
        Some(0) => Some(call_doc(&engine, &ast).map_err(shape_error)?),
        _ => None,
    };
    Ok(RhaiRule {
        id,
        slug,
        ast,
        engine: Arc::new(engine),
        takes_options,
        doc,
    })
}

fn arity(ast: &AST, name: &str) -> Option<usize> {
    ast.iter_functions()
        .find(|f| f.name == name)
        .map(|f| f.params.len())
}

fn call_doc(engine: &Engine, ast: &AST) -> Result<RuleDoc, String> {
    let value: Dynamic = engine
        .call_fn(&mut Scope::new(), ast, "doc", ())
        .map_err(|e| format!("doc(): {e}"))?;
    let spec: DocSpec = rhai::serde::from_dynamic(&value)
        .map_err(|e| format!("doc() must return #{{ summary, why, fix, options }}: {e}"))?;
    Ok(RuleDoc {
        summary: spec.summary,
        why: spec.why,
        fix: spec.fix,
        options: spec.options,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::io::Write;

    use super::*;

    fn load_source(source: &str) -> Result<RhaiRule, LoadError> {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "{source}").unwrap();
        let path = Utf8Path::from_path(tmp.path()).unwrap().to_path_buf();
        load("custom/test", RuleId::new(PLUGIN_ID_FLOOR), &path)
    }

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
        assert!(matches!(
            load_source("let x =;"),
            Err(LoadError::Compile { .. })
        ));
    }

    #[test]
    fn script_without_check_or_with_malformed_doc_fails_to_load() {
        assert!(matches!(
            load_source("fn chek(m) { [] }"),
            Err(LoadError::Shape { .. })
        ));
        assert!(matches!(
            load_source("fn check(m) { [] } fn doc() { #{ sumary: \"x\" } }"),
            Err(LoadError::Shape { .. })
        ));
    }

    #[test]
    fn doc_function_becomes_the_rule_doc() {
        let rule = load_source(
            r#"fn check(m, options) { [] }
               fn doc() { #{ summary: "No legacy", options: ["dir: folder (legacy)"] } }"#,
        )
        .unwrap();
        let doc = stratum_rules::Rule::doc(&rule).unwrap();
        assert_eq!(doc.summary, "No legacy");
        assert_eq!(doc.options, vec!["dir: folder (legacy)"]);
    }
}
