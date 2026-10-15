use std::collections::HashSet;

use oxc_ast::ast::{Declaration, ModuleDeclaration, Program};
use oxc_ast::syntax_directed_operations::BoundNames;
use stratum_core::types::Exports;

pub(crate) fn count_exports(program: &Program<'_>) -> Exports {
    let mut names: HashSet<String> = HashSet::new();
    let mut star_reexports: u32 = 0;
    for stmt in &program.body {
        let Some(decl) = stmt.as_module_declaration() else {
            continue;
        };
        match decl {
            ModuleDeclaration::ExportNamedDeclaration(export) => {
                if let Some(declaration) = &export.declaration {
                    declared_names(declaration, &mut names);
                }
                for spec in &export.specifiers {
                    names.insert(spec.exported.name().to_string());
                }
            }
            ModuleDeclaration::ExportDefaultDeclaration(_)
            | ModuleDeclaration::TSExportAssignment(_) => {
                names.insert("default".to_owned());
            }
            ModuleDeclaration::ExportAllDeclaration(all) => match &all.exported {
                Some(alias) => {
                    names.insert(alias.name().to_string());
                }
                None => star_reexports += 1,
            },
            ModuleDeclaration::ImportDeclaration(_)
            | ModuleDeclaration::TSNamespaceExportDeclaration(_) => {}
        }
    }
    let distinct = u32::try_from(names.len()).unwrap_or(u32::MAX);
    Exports {
        count: distinct.saturating_add(star_reexports),
        star_reexports,
    }
}

fn declared_names(declaration: &Declaration<'_>, names: &mut HashSet<String>) {
    match declaration {
        Declaration::VariableDeclaration(_)
        | Declaration::FunctionDeclaration(_)
        | Declaration::ClassDeclaration(_) => {
            declaration.bound_names(&mut |id| {
                names.insert(id.name.to_string());
            });
        }
        Declaration::TSTypeAliasDeclaration(d) => {
            names.insert(d.id.name.to_string());
        }
        Declaration::TSInterfaceDeclaration(d) => {
            names.insert(d.id.name.to_string());
        }
        Declaration::TSEnumDeclaration(d) => {
            names.insert(d.id.name.to_string());
        }
        Declaration::TSImportEqualsDeclaration(d) => {
            names.insert(d.id.name.to_string());
        }
        Declaration::TSModuleDeclaration(d) => {
            if !d.kind.is_global() {
                names.insert(d.id.name().to_string());
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use oxc_span::SourceType;

    use crate::imports::collect;

    fn ts(source: &str) -> stratum_core::types::Exports {
        collect(
            source,
            SourceType::default()
                .with_typescript(true)
                .with_module(true),
            &std::collections::HashSet::new(),
        )
        .exports
    }

    #[test]
    fn overloads_and_declaration_merging_count_once() {
        let e = ts("export function f(a: string): void;
             export function f(a: number): void;
             export function f(a: unknown) {}
             export interface Box {}
             export const Box = 1;");
        assert_eq!(e.count, 2);
    }

    #[test]
    fn namespace_members_are_not_module_exports() {
        let e = ts("export namespace N { export const a = 1; export const b = 2; }");
        assert_eq!(e.count, 1);
    }

    #[test]
    fn star_reexport_counts_once_and_is_flagged() {
        let e = ts(r#"export * from "./a";
             export * as ns from "./b";
             export { x, y as z } from "./c";
             export const [p, { q }] = pair;
             export default 1;"#);
        assert_eq!(e.count, 7);
        assert_eq!(e.star_reexports, 1);
    }
}
