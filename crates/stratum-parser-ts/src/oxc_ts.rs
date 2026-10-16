use camino::Utf8Path;
use oxc_span::SourceType;

use crate::annotations::extract_purity;
use crate::extractor::{ExtractError, ExtractedData, LanguageExtractor, TrackedCalls};
use crate::imports::collect;

/// Extractor for TypeScript, JavaScript, JSX, TSX, plus `.mts`/`.cts`/`.mjs`/`.cjs`.
#[derive(Debug, Default, Clone)]
pub struct OxcTsExtractor {
    tracked_calls: TrackedCalls,
}

impl OxcTsExtractor {
    #[must_use]
    pub const fn new(tracked_calls: TrackedCalls) -> Self {
        Self { tracked_calls }
    }

    fn source_type_for(path: &Utf8Path) -> Option<SourceType> {
        let st = match path.extension()? {
            "ts" | "mts" | "cts" => SourceType::default()
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
        let collected = collect(source, source_type, &self.tracked_calls);
        let purity = extract_purity(source);
        Ok(ExtractedData {
            imports: collected.imports,
            purity,
            exports: collected.exports,
            calls: collected.calls,
            suppressions: collected.suppressions,
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
        let ex = OxcTsExtractor::default();
        for ext in ["ts", "mts", "cts", "tsx", "js", "jsx", "mjs", "cjs"] {
            let path = Utf8Path::new("foo").with_extension(ext);
            assert!(ex.handles(&path), "should handle .{ext}");
        }
        assert!(!ex.handles(Utf8Path::new("foo.vue")));
        assert!(!ex.handles(Utf8Path::new("foo.rs")));
    }

    #[test]
    fn extracts_imports_from_tsx() {
        let ex = OxcTsExtractor::default();
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
        let ex = OxcTsExtractor::default();
        let err = ex.extract(Utf8Path::new("foo.rs"), "").unwrap_err();
        assert!(matches!(err, ExtractError::UnsupportedExtension(_)));
    }
}
