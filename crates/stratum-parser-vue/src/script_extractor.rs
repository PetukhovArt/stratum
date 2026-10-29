use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;

use stratum_core::types::ExportOrigin;
use stratum_parser_ts::{
    ExtractError, ExtractedData, LanguageExtractor, RawCall, RawExports, RawImport, RawSuppression,
    SourceType, TrackedCalls, collect, extract_purity,
};

use crate::setup_macros;
use crate::sfc_splitter::split;
use crate::span_remap::shift;

#[derive(Debug, Default, Clone)]
pub struct VueExtractor {
    tracked_calls: TrackedCalls,
}

impl VueExtractor {
    #[must_use]
    pub const fn new(tracked_calls: TrackedCalls) -> Self {
        Self { tracked_calls }
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
        let mut exports = RawExports::default();
        let mut calls: Vec<RawCall> = Vec::new();
        let mut local_refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut suppressions: Vec<RawSuppression> = Vec::new();
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
            let collected = collect(&block.body, st, &self.tracked_calls);
            if block.setup {
                exports.unimportable = exports
                    .unimportable
                    .saturating_add(setup_macros::surface(&block.body, st));
            }
            let offset = block.content_range.start;
            let first_import = all_imports.len();
            exports.unimportable = exports
                .unimportable
                .saturating_add(collected.exports.unimportable);
            exports
                .stars
                .extend(collected.exports.stars.iter().map(|i| first_import + i));
            for mut decl in collected.exports.decls {
                decl.span = shift(decl.span, offset);
                if let ExportOrigin::Reexport { import, .. } = &mut decl.origin {
                    *import += first_import;
                }
                exports.decls.push(decl);
            }
            for mut imp in collected.imports {
                imp.span = shift(imp.span, offset);
                imp.statement = shift(imp.statement, offset);
                all_imports.push(imp);
            }
            for mut call in collected.calls {
                call.span = shift(call.span, offset);
                calls.push(call);
            }
            for (owner, refs) in collected.local_refs {
                local_refs.entry(owner).or_default().extend(refs);
            }
            for mut suppression in collected.suppressions {
                suppression.span = shift(suppression.span, offset);
                suppressions.push(suppression);
            }
        }
        let all_imports = sorted_by_specifier(all_imports, &mut exports);
        Ok(ExtractedData {
            imports: all_imports,
            purity: extract_purity(source),
            exports,
            calls,
            local_refs: local_refs
                .into_iter()
                .map(|(owner, refs)| (owner, refs.into_iter().collect()))
                .collect(),
            suppressions,
        })
    }
}

fn sorted_by_specifier(imports: Vec<RawImport>, exports: &mut RawExports) -> Vec<RawImport> {
    let mut order: Vec<usize> = (0..imports.len()).collect();
    order.sort_by(|&a, &b| {
        (&imports[a].specifier, imports[a].span.start)
            .cmp(&(&imports[b].specifier, imports[b].span.start))
    });
    let mut new_index = vec![0; imports.len()];
    for (new, &old) in order.iter().enumerate() {
        new_index[old] = new;
    }
    for star in &mut exports.stars {
        *star = new_index[*star];
    }
    for decl in &mut exports.decls {
        if let ExportOrigin::Reexport { import, .. } = &mut decl.origin {
            *import = new_index[*import];
        }
    }
    let mut slots: Vec<Option<RawImport>> = imports.into_iter().map(Some).collect();
    order.iter().filter_map(|&old| slots[old].take()).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn handles_vue_extension() {
        let ex = VueExtractor::default();
        assert!(ex.handles(Utf8Path::new("App.vue")));
        assert!(!ex.handles(Utf8Path::new("App.ts")));
    }

    #[test]
    fn extracts_imports_from_script_setup() {
        let src = "<template></template>\n<script setup lang=\"ts\">\nimport { Cart } from '@/features/cart';\nimport http from '../shared/api/http';\n</script>";
        let data = VueExtractor::default()
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
        let data = VueExtractor::default()
            .extract(Utf8Path::new("Mixed.vue"), src)
            .unwrap();
        assert_eq!(data.imports.len(), 2);
    }

    #[test]
    fn exports_sum_plain_script_exports_and_setup_macros() {
        let src = "<script lang=\"ts\">\nexport default { inheritAttrs: false };\nexport type Size = 's' | 'm';\n</script>\n<script setup lang=\"ts\">\ndefineProps<{ size: Size; label: string }>();\n</script>";
        let data = VueExtractor::default()
            .extract(Utf8Path::new("Btn.vue"), src)
            .unwrap();
        assert_eq!(data.exports.decls.len(), 2);
        assert_eq!(data.exports.unimportable, 2);
    }

    #[test]
    fn tracked_calls_in_both_script_blocks_map_to_sfc_offsets() {
        let src = "<script>\nprovide('a', 1);\n</script>\n<script setup>\nimport { inject as i } from 'vue';\ni('a');\n</script>";
        let mut tracked = TrackedCalls::default();
        tracked.insert("inject").unwrap();
        tracked.insert("provide").unwrap();
        let data = VueExtractor::new(tracked)
            .extract(Utf8Path::new("Di.vue"), src)
            .unwrap();
        let found: Vec<_> = data
            .calls
            .iter()
            .map(|c| {
                (
                    c.name.as_str(),
                    &src[c.span.start as usize..c.span.end as usize],
                )
            })
            .collect();
        assert_eq!(
            found,
            [("provide", "provide('a', 1)"), ("inject", "i('a')")]
        );
    }

    #[test]
    fn local_refs_of_both_script_blocks_are_kept() {
        let src = "<script>\nconst base = 1;\nexport const api = base;\n</script>\n<script setup>\nconst load = () => 2;\nconst run = () => load();\n</script>";
        let mut tracked = TrackedCalls::default();
        tracked.insert("inject").unwrap();
        let data = VueExtractor::new(tracked)
            .extract(Utf8Path::new("Api.vue"), src)
            .unwrap();
        assert_eq!(
            data.local_refs,
            [
                ("api".to_owned(), vec!["base".to_owned()]),
                ("run".to_owned(), vec!["load".to_owned()]),
            ]
        );
    }

    #[test]
    fn no_script_returns_empty() {
        let src = "<template><div /></template>";
        let data = VueExtractor::default()
            .extract(Utf8Path::new("Empty.vue"), src)
            .unwrap();
        assert!(data.imports.is_empty());
    }

    #[test]
    fn unclosed_script_returns_hard_error() {
        let src = "<script>\nimport x from 'x';";
        let err = VueExtractor::default()
            .extract(Utf8Path::new("Broken.vue"), src)
            .unwrap_err();
        assert!(matches!(err, ExtractError::Hard { .. }));
    }

    #[test]
    fn span_maps_back_to_sfc_offset() {
        let prefix = "<template><div /></template>\n<script setup>\n";
        let src = format!("{prefix}import x from 'x';\n</script>");
        let data = VueExtractor::default()
            .extract(Utf8Path::new("Span.vue"), &src)
            .unwrap();
        assert_eq!(data.imports.len(), 1);
        let span = &data.imports[0].span;
        let slice = &src[span.start as usize..span.end as usize];
        assert_eq!(slice, "'x'");
    }
}
