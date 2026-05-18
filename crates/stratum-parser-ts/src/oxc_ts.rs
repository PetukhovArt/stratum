use camino::Utf8Path;
use oxc_span::SourceType;

use crate::extractor::{ExtractError, ExtractedData, LanguageExtractor};
use crate::imports::collect_imports;

/// Extractor for TypeScript, JavaScript, JSX, TSX, plus `.mjs`/`.cjs`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OxcTsExtractor;

impl OxcTsExtractor {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn source_type_for(path: &Utf8Path) -> Option<SourceType> {
        let st = match path.extension()? {
            "ts" => SourceType::default()
                .with_typescript(true)
                .with_module(true),
            "tsx" => SourceType::default()
                .with_typescript(true)
                .with_jsx(true)
                .with_module(true),
            "js" | "mjs" => SourceType::default()
                .with_javascript(true)
                .with_module(true),
            "cjs" => SourceType::default()
                .with_javascript(true)
                .with_module(false),
            "jsx" => SourceType::default()
                .with_javascript(true)
                .with_jsx(true)
                .with_module(true),
            _ => return None,
        };
        Some(st)
    }
}

impl LanguageExtractor for OxcTsExtractor {
    fn handles(&self, path: &Utf8Path) -> bool {
        Self::source_type_for(path).is_some()
    }

    fn extract(&self, path: &Utf8Path, source: &str) -> Result<ExtractedData, ExtractError> {
        let source_type = Self::source_type_for(path)
            .ok_or_else(|| ExtractError::UnsupportedExtension(path.to_path_buf()))?;
        let imports = collect_imports(source, source_type);
        Ok(ExtractedData {
            source_path: path.to_path_buf(),
            imports,
            diagnostics: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::edge::EdgeKind;

    #[test]
    fn handles_typescript_extensions() {
        let ex = OxcTsExtractor::new();
        for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
            let path = Utf8Path::new("foo").with_extension(ext);
            assert!(ex.handles(&path), "should handle .{ext}");
        }
        assert!(!ex.handles(Utf8Path::new("foo.vue")));
        assert!(!ex.handles(Utf8Path::new("foo.rs")));
    }

    #[test]
    fn extracts_imports_from_tsx() {
        let ex = OxcTsExtractor::new();
        let source = r#"
            import React from "react";
            import type { Props } from "./types";
            export { Button } from "./button";
        "#;
        let data = ex.extract(Utf8Path::new("comp.tsx"), source).unwrap();
        assert_eq!(data.imports.len(), 3);

        let by_spec: Vec<_> = data.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(by_spec, ["react", "./types", "./button"]);

        let type_only_flags: Vec<bool> = data.imports.iter().map(|i| i.type_only).collect();
        assert_eq!(type_only_flags, [false, true, false]);

        for imp in &data.imports {
            assert_eq!(imp.kind, EdgeKind::Static);
        }
    }

    #[test]
    fn rejects_unknown_extension() {
        let ex = OxcTsExtractor::new();
        let err = ex.extract(Utf8Path::new("foo.rs"), "").unwrap_err();
        assert!(matches!(err, ExtractError::UnsupportedExtension(_)));
    }
}
