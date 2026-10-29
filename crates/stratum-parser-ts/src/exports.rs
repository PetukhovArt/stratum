use std::collections::HashMap;

use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, ModuleDeclaration, Program,
};
use oxc_ast::syntax_directed_operations::BoundNames;
use oxc_span::Span;
use stratum_core::types::ExportOrigin;

use crate::extractor::{RawExportDecl, RawExports, RawImport};
use crate::source_span::SourceSpan;

pub(crate) fn top_level_exports(program: &Program<'_>, imports: &[RawImport]) -> RawExports {
    let mut exports = Exports::default();
    let import_of = |statement: Span| {
        imports
            .iter()
            .position(|i| i.statement.start == statement.start)
    };
    for stmt in &program.body {
        let Some(decl) = stmt.as_module_declaration() else {
            continue;
        };
        match decl {
            ModuleDeclaration::ExportNamedDeclaration(export) => {
                let at = span(export.span);
                if let Some(declaration) = &export.declaration {
                    let type_only = matches!(
                        declaration,
                        Declaration::TSTypeAliasDeclaration(_)
                            | Declaration::TSInterfaceDeclaration(_)
                    );
                    declared_names(declaration, &mut |name| {
                        exports.add(name, type_only, at, ExportOrigin::Local);
                    });
                }
                let reexport = export.source.as_ref().and_then(|_| import_of(export.span));
                for spec in &export.specifiers {
                    let local = spec.local.name().to_string();
                    let exported = spec.exported.name().to_string();
                    let origin = match reexport {
                        Some(import) => ExportOrigin::Reexport {
                            import,
                            imported: local,
                        },
                        None => local_origin(&exported, Some(local)),
                    };
                    exports.add(
                        exported,
                        export.export_kind.is_type() || spec.export_kind.is_type(),
                        span(spec.span),
                        origin,
                    );
                }
            }
            ModuleDeclaration::ExportDefaultDeclaration(export) => {
                let type_only = matches!(
                    export.declaration,
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
                );
                let binding = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.id.as_ref(),
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => c.id.as_ref(),
                    _ => None,
                }
                .map(|id| id.name.to_string())
                .or_else(|| identifier(export.declaration.as_expression()));
                exports.add(
                    "default".to_owned(),
                    type_only,
                    span(export.span),
                    local_origin("default", binding),
                );
            }
            ModuleDeclaration::TSExportAssignment(export) => exports.add(
                "default".to_owned(),
                false,
                span(export.span),
                local_origin("default", identifier(Some(&export.expression))),
            ),
            ModuleDeclaration::ExportAllDeclaration(all) => {
                let Some(import) = import_of(all.span) else {
                    continue;
                };
                match &all.exported {
                    Some(alias) => exports.add(
                        alias.name().to_string(),
                        all.export_kind.is_type(),
                        span(all.span),
                        ExportOrigin::Reexport {
                            import,
                            imported: "*".to_owned(),
                        },
                    ),
                    None => exports.raw.stars.push(import),
                }
            }
            ModuleDeclaration::ImportDeclaration(_)
            | ModuleDeclaration::TSNamespaceExportDeclaration(_) => {}
        }
    }
    exports.raw
}

fn local_origin(exported: &str, binding: Option<String>) -> ExportOrigin {
    match binding {
        Some(binding) if binding != exported => ExportOrigin::Alias { binding },
        _ => ExportOrigin::Local,
    }
}

fn identifier(expression: Option<&Expression<'_>>) -> Option<String> {
    match expression? {
        Expression::Identifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

const fn span(s: Span) -> SourceSpan {
    SourceSpan::new(s.start, s.end)
}

#[derive(Default)]
struct Exports {
    raw: RawExports,
    by_name: HashMap<String, usize>,
}

impl Exports {
    fn add(&mut self, name: String, type_only: bool, span: SourceSpan, origin: ExportOrigin) {
        if let Some(&first) = self.by_name.get(&name) {
            self.raw.decls[first].type_only &= type_only;
            return;
        }
        self.by_name.insert(name.clone(), self.raw.decls.len());
        self.raw.decls.push(RawExportDecl {
            name,
            type_only,
            span,
            origin,
        });
    }
}

fn declared_names(declaration: &Declaration<'_>, add: &mut impl FnMut(String)) {
    match declaration {
        Declaration::VariableDeclaration(_)
        | Declaration::FunctionDeclaration(_)
        | Declaration::ClassDeclaration(_) => {
            declaration.bound_names(&mut |id| add(id.name.to_string()));
        }
        Declaration::TSTypeAliasDeclaration(d) => add(d.id.name.to_string()),
        Declaration::TSInterfaceDeclaration(d) => add(d.id.name.to_string()),
        Declaration::TSEnumDeclaration(d) => add(d.id.name.to_string()),
        Declaration::TSImportEqualsDeclaration(d) => add(d.id.name.to_string()),
        Declaration::TSModuleDeclaration(d) => {
            if !d.kind.is_global() {
                add(d.id.name().to_string());
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use oxc_span::SourceType;
    use stratum_core::types::ExportOrigin;

    use crate::extractor::{RawExports, TrackedCalls};
    use crate::imports::collect;

    fn ts(source: &str) -> RawExports {
        collect(
            source,
            SourceType::default()
                .with_typescript(true)
                .with_module(true),
            &TrackedCalls::default(),
        )
        .exports
    }

    fn count(e: &RawExports) -> usize {
        e.decls.len() + e.stars.len()
    }

    #[test]
    fn overloads_and_declaration_merging_count_once() {
        let e = ts("export function f(a: string): void;
             export function f(a: number): void;
             export function f(a: unknown) {}
             export interface Box {}
             export const Box = 1;");
        assert_eq!(count(&e), 2);
        assert!(!e.decls[1].type_only);
    }

    #[test]
    fn namespace_members_are_not_module_exports() {
        let e = ts("export namespace N { export const a = 1; export const b = 2; }");
        assert_eq!(count(&e), 1);
    }

    #[test]
    fn star_reexport_counts_once_and_is_flagged() {
        let e = ts(r#"export * from "./a";
             export * as ns from "./b";
             export { x, y as z } from "./c";
             export const [p, { q }] = pair;
             export default 1;"#);
        assert_eq!(count(&e), 7);
        assert_eq!(e.stars, [0]);
    }

    #[test]
    fn reexports_name_the_import_they_come_through() {
        let e = ts(r#"import { a } from "./a";
             export { y as z } from "./c";
             export * as ns from "./b";
             export { a };"#);
        let origins: Vec<_> = e
            .decls
            .iter()
            .map(|d| (d.name.as_str(), d.origin.clone()))
            .collect();
        let reexport = |import, imported: &str| ExportOrigin::Reexport {
            import,
            imported: imported.into(),
        };
        assert_eq!(
            origins,
            [
                ("z", reexport(1, "y")),
                ("ns", reexport(2, "*")),
                ("a", ExportOrigin::Local),
            ]
        );
    }

    #[test]
    fn a_binding_exported_under_another_name_is_an_alias_of_it() {
        let origin = |source: &str| ts(source).decls[0].origin.clone();
        let alias = |binding: &str| ExportOrigin::Alias {
            binding: binding.into(),
        };
        assert_eq!(origin("export default function load() {}"), alias("load"));
        assert_eq!(origin("export default class Store {}"), alias("Store"));
        assert_eq!(origin("const run = 1; export default run;"), alias("run"));
        assert_eq!(origin("const run = 1; export = run;"), alias("run"));
        assert_eq!(origin("const a = 1; export { a as b };"), alias("a"));
        assert_eq!(origin("export default () => 1;"), ExportOrigin::Local);
        assert_eq!(origin("const a = 1; export { a };"), ExportOrigin::Local);
    }
}
