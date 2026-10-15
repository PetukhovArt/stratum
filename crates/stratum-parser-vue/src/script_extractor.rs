use camino::Utf8Path;

use stratum_core::types::Exports;
use stratum_parser_ts::{
    ExtractError, ExtractedData, LanguageExtractor, RawImport, SourceType, collect, extract_purity,
};

use crate::setup_macros;
use crate::sfc_splitter::split;
use crate::span_remap::shift;

#[derive(Debug, Default, Clone, Copy)]
pub struct VueExtractor;

impl VueExtractor {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageExtractor for VueExtractor {
    fn handles(&self, path: &Utf8Path) -> bool {
        path.extension().is_some_and(|e| e == "vue")
    }

    fn extract(&self, path: &Utf8Path, source: &str) -> Result<ExtractedData, ExtractError> {
        let blocks = split(source).map_err(|e| ExtractError::Hard {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        let mut all_imports: Vec<RawImport> = Vec::new();
        let mut exports = Exports::default();
        for block in blocks {
            let st = if block.is_typescript {
                SourceType::default()
                    .with_typescript(true)
                    .with_module(true)
            } else {
                SourceType::default()
                    .with_javascript(true)
                    .with_module(true)
            };
            let collected = collect(&block.body, st);
            let macros = if block.setup {
                setup_macros::surface(&block.body, st)
            } else {
                0
            };
            exports.count = exports
                .count
                .saturating_add(collected.exports.count)
                .saturating_add(macros);
            exports.star_reexports += collected.exports.star_reexports;
            for mut imp in collected.imports {
                imp.span = shift(imp.span, block.content_range.start);
                all_imports.push(imp);
            }
        }
        all_imports.sort_by_key(|i| (i.specifier.clone(), i.span.start));
        all_imports.dedup_by(|a, b| a.specifier == b.specifier && a.span.start == b.span.start);
        Ok(ExtractedData {
            source_path: path.to_path_buf(),
            imports: all_imports,
            diagnostics: vec![],
            purity: extract_purity(source),
            exports,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn handles_vue_extension() {
        let ex = VueExtractor::new();
        assert!(ex.handles(Utf8Path::new("App.vue")));
        assert!(!ex.handles(Utf8Path::new("App.ts")));
    }

    #[test]
    fn extracts_imports_from_script_setup() {
        let src = "<template></template>\n<script setup lang=\"ts\">\nimport { Cart } from '@/features/cart';\nimport http from '../shared/api/http';\n</script>";
        let data = VueExtractor::new()
            .extract(Utf8Path::new("App.vue"), src)
            .unwrap();
        assert_eq!(data.imports.len(), 2);
        let specs: Vec<_> = data.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert!(specs.contains(&"@/features/cart"));
        assert!(specs.contains(&"../shared/api/http"));
    }

    #[test]
    fn merges_both_script_blocks() {
        let src = "<script>\nimport a from 'a';\n</script>\n<script setup>\nimport b from 'b';\n</script>";
        let data = VueExtractor::new()
            .extract(Utf8Path::new("Mixed.vue"), src)
            .unwrap();
        assert_eq!(data.imports.len(), 2);
    }

    #[test]
    fn exports_sum_plain_script_exports_and_setup_macros() {
        let src = "<script lang=\"ts\">\nexport default { inheritAttrs: false };\nexport type Size = 's' | 'm';\n</script>\n<script setup lang=\"ts\">\ndefineProps<{ size: Size; label: string }>();\n</script>";
        let data = VueExtractor::new()
            .extract(Utf8Path::new("Btn.vue"), src)
            .unwrap();
        assert_eq!(data.exports.count, 4);
    }

    #[test]
    fn no_script_returns_empty() {
        let src = "<template><div /></template>";
        let data = VueExtractor::new()
            .extract(Utf8Path::new("Empty.vue"), src)
            .unwrap();
        assert!(data.imports.is_empty());
    }

    #[test]
    fn unclosed_script_returns_hard_error() {
        let src = "<script>\nimport x from 'x';";
        let err = VueExtractor::new()
            .extract(Utf8Path::new("Broken.vue"), src)
            .unwrap_err();
        assert!(matches!(err, ExtractError::Hard { .. }));
    }

    #[test]
    fn span_maps_back_to_sfc_offset() {
        let prefix = "<template><div /></template>\n<script setup>\n";
        let src = format!("{prefix}import x from 'x';\n</script>");
        let data = VueExtractor::new()
            .extract(Utf8Path::new("Span.vue"), &src)
            .unwrap();
        assert_eq!(data.imports.len(), 1);
        let span = &data.imports[0].span;
        let slice = &src[span.start as usize..span.end as usize];
        assert_eq!(slice, "'x'");
    }
}
