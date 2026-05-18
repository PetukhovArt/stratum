use oxc_allocator::Allocator;
use oxc_ast::Visit;
use oxc_ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportNamedDeclaration, Expression,
    ImportDeclaration, ImportExpression,
};
use oxc_ast::visit::walk;
use oxc_parser::Parser;
use oxc_span::SourceType;
use stratum_core::edge::EdgeKind;

use crate::extractor::RawImport;
use crate::source_span::SourceSpan;

/// Parse `source` and collect every static `import` / `export from` and
/// dynamic `import()` edge.
///
/// The returned vec preserves source order and includes type-only imports
/// (callers may filter on `RawImport::type_only`).
#[must_use]
pub fn collect_imports(source: &str, source_type: SourceType) -> Vec<RawImport> {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let mut visitor = ImportVisitor::default();
    visitor.visit_program(&ret.program);
    visitor.imports
}

#[derive(Default)]
struct ImportVisitor {
    imports: Vec<RawImport>,
}

impl ImportVisitor {
    fn push(&mut self, specifier: &str, start: u32, end: u32, kind: EdgeKind, type_only: bool) {
        self.imports.push(RawImport {
            specifier: specifier.to_owned(),
            span: SourceSpan::new(start, end),
            kind,
            type_only,
        });
    }
}

impl<'a> Visit<'a> for ImportVisitor {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let src = &decl.source;
        self.push(
            src.value.as_str(),
            src.span.start,
            src.span.end,
            EdgeKind::Static,
            decl.import_kind.is_type(),
        );
        walk::walk_import_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(src) = decl.source.as_ref() {
            self.push(
                src.value.as_str(),
                src.span.start,
                src.span.end,
                EdgeKind::Static,
                decl.export_kind.is_type(),
            );
        }
        walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let src = &decl.source;
        self.push(
            src.value.as_str(),
            src.span.start,
            src.span.end,
            EdgeKind::Static,
            decl.export_kind.is_type(),
        );
        walk::walk_export_all_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(lit) = &expr.source {
            self.push(
                lit.value.as_str(),
                lit.span.start,
                lit.span.end,
                EdgeKind::Runtime,
                false,
            );
        }
        walk::walk_import_expression(self, expr);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_require_callee(&call.callee) && call.arguments.len() == 1 {
            if let Some(Argument::StringLiteral(lit)) = call.arguments.first() {
                self.push(
                    lit.value.as_str(),
                    lit.span.start,
                    lit.span.end,
                    EdgeKind::Static,
                    false,
                );
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn is_require_callee(callee: &Expression<'_>) -> bool {
    matches!(callee, Expression::Identifier(id) if id.name.as_str() == "require")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ts_module() -> SourceType {
        SourceType::default()
            .with_typescript(true)
            .with_module(true)
    }

    fn cjs_script() -> SourceType {
        SourceType::default()
            .with_javascript(true)
            .with_module(false)
    }

    #[test]
    fn collects_static_import() {
        let imports = collect_imports(r#"import { foo } from "./bar";"#, ts_module());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Static);
        assert!(!imports[0].type_only);
    }

    #[test]
    fn collects_type_only_import() {
        let imports = collect_imports(r#"import type { T } from "./types";"#, ts_module());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./types");
        assert_eq!(imports[0].kind, EdgeKind::Static);
        assert!(imports[0].type_only);
    }

    #[test]
    fn collects_named_reexport() {
        let imports = collect_imports(r#"export { foo } from "./bar";"#, ts_module());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Static);
        assert!(!imports[0].type_only);
    }

    #[test]
    fn collects_star_reexport() {
        let imports = collect_imports(r#"export * from "./bar";"#, ts_module());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Static);
    }

    #[test]
    fn ignores_local_export() {
        let imports = collect_imports("export const x = 1;", ts_module());
        assert!(imports.is_empty());
    }

    #[test]
    fn collects_dynamic_import() {
        let imports = collect_imports(r#"const x = await import("./bar");"#, ts_module());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Runtime);
        assert!(!imports[0].type_only);
    }

    #[test]
    fn ignores_dynamic_import_with_non_literal_specifier() {
        let imports = collect_imports("const x = await import(maybeLater);", ts_module());
        assert!(imports.is_empty());
    }

    #[test]
    fn collects_dynamic_import_inside_function() {
        let imports = collect_imports(
            r#"async function load() { return import("./bar"); }"#,
            ts_module(),
        );
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Runtime);
    }

    #[test]
    fn collects_require_in_cjs() {
        let imports = collect_imports(r#"const x = require("./bar");"#, cjs_script());
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].specifier, "./bar");
        assert_eq!(imports[0].kind, EdgeKind::Static);
        assert!(!imports[0].type_only);
    }

    #[test]
    fn require_with_dynamic_arg_is_ignored() {
        let imports = collect_imports("const x = require(name);", cjs_script());
        assert!(imports.is_empty());
    }

    #[test]
    fn span_points_at_specifier_string_literal() {
        let source = r#"import { foo } from "./bar";"#;
        let imports = collect_imports(source, ts_module());
        assert_eq!(imports.len(), 1);
        let span = imports[0].span;
        assert_eq!(
            &source[span.start as usize..span.end as usize],
            r#""./bar""#
        );
    }
}
