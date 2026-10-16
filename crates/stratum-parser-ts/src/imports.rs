use std::cell::RefCell;

use oxc_allocator::Allocator;
use oxc_ast::Visit;
use oxc_ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportNamedDeclaration, Expression,
    ImportDeclaration, ImportDeclarationSpecifier, ImportExpression,
};
use oxc_ast::visit::walk;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use stratum_core::edge::EdgeKind;
use stratum_core::types::{Bindings, ImportedName};

use crate::comments::suppressions;
use crate::exports::top_level_exports;
use crate::extractor::{CallPattern, RawCall, RawExports, RawImport, RawSuppression, TrackedCalls};
use crate::source_span::SourceSpan;

#[derive(Debug)]
pub struct Collected {
    /// Every static `import` / `export from`, dynamic `import()`, `require()` and
    /// `import.meta.glob()` edge in source order, type-only imports included.
    pub imports: Vec<RawImport>,
    /// Top-level ES exports only; `module.exports` / `exports.x` are not counted.
    pub exports: RawExports,
    pub calls: Vec<RawCall>,
    pub suppressions: Vec<RawSuppression>,
}

#[must_use]
pub fn collect(source: &str, source_type: SourceType, tracked: &TrackedCalls) -> Collected {
    ARENA.with_borrow_mut(|allocator| {
        let collected = {
            let ret = Parser::new(allocator, source, source_type).parse();
            let mut visitor = ImportVisitor::new(tracked);
            visitor.visit_program(&ret.program);
            Collected {
                exports: top_level_exports(&ret.program, &visitor.imports),
                imports: visitor.imports,
                calls: visitor.calls,
                suppressions: suppressions(source, &ret.trivias),
            }
        };
        if allocator.allocated_bytes() > ARENA_KEEP_BYTES {
            *allocator = Allocator::default();
        } else {
            allocator.reset();
        }
        collected
    })
}

const ARENA_KEEP_BYTES: usize = 4 << 20;

thread_local! {
    static ARENA: RefCell<Allocator> = RefCell::new(Allocator::default());
}

struct ImportVisitor<'t> {
    tracked: &'t TrackedCalls,
    aliases: Vec<(String, String)>,
    namespaces: Vec<String>,
    imports: Vec<RawImport>,
    calls: Vec<RawCall>,
}

enum Callee<'a> {
    Plain(&'a str),
    Member {
        object: Option<&'a str>,
        name: &'a str,
    },
}

impl<'a> Callee<'a> {
    fn of(callee: &'a Expression<'_>) -> Option<Self> {
        match callee {
            Expression::Identifier(id) => Some(Self::Plain(id.name.as_str())),
            Expression::StaticMemberExpression(member) => Some(Self::Member {
                object: match &member.object {
                    Expression::Identifier(id) => Some(id.name.as_str()),
                    _ => None,
                },
                name: member.property.name.as_str(),
            }),
            _ => None,
        }
    }
}

impl<'t> ImportVisitor<'t> {
    const fn new(tracked: &'t TrackedCalls) -> Self {
        Self {
            tracked,
            aliases: Vec::new(),
            namespaces: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
        }
    }

    fn record_local_names(&mut self, decl: &ImportDeclaration<'_>) {
        if self.tracked.is_empty() {
            return;
        }
        for spec in decl.specifiers.iter().flatten() {
            match spec {
                ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                    let imported = spec.imported.name();
                    if spec.local.name != imported && self.is_plain_entry(imported.as_str()) {
                        self.aliases
                            .push((spec.local.name.to_string(), imported.to_string()));
                    }
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                    self.namespaces.push(spec.local.name.to_string());
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {}
            }
        }
    }

    fn is_plain_entry(&self, name: &str) -> bool {
        self.tracked
            .entries()
            .iter()
            .any(|t| matches!(&t.pattern, CallPattern::Plain(n) if n == name))
    }

    fn matches(&self, pattern: &CallPattern, callee: &Callee<'_>) -> bool {
        match (pattern, callee) {
            (CallPattern::Plain(n), Callee::Plain(local)) => {
                local == n
                    || self
                        .aliases
                        .iter()
                        .find(|(alias, _)| alias == local)
                        .is_some_and(|(_, imported)| imported == n)
            }
            (
                CallPattern::Plain(n),
                Callee::Member {
                    object: Some(object),
                    name,
                },
            ) => name == n && self.namespaces.iter().any(|ns| ns == object),
            (
                CallPattern::Member { object, name },
                Callee::Member {
                    object: Some(o),
                    name: p,
                },
            ) => o == object && p == name,
            (CallPattern::AnyMember(n), Callee::Member { name, .. }) => name == n,
            _ => false,
        }
    }

    fn record_call(&mut self, call: &CallExpression<'_>) {
        if self.tracked.is_empty() {
            return;
        }
        let Some(callee) = Callee::of(&call.callee) else {
            return;
        };
        for tracked in self.tracked.entries() {
            if !self.matches(&tracked.pattern, &callee) {
                continue;
            }
            self.calls.push(RawCall {
                name: tracked.entry.clone(),
                span: SourceSpan::new(call.span.start, call.span.end),
            });
        }
    }

    fn push(&mut self, specifier: Span, text: &str, statement: Span, import: Import) {
        self.imports.push(RawImport {
            specifier: text.to_owned(),
            span: SourceSpan::new(specifier.start, specifier.end),
            statement: SourceSpan::new(statement.start, statement.end),
            kind: import.kind,
            type_only: import.type_only,
            bindings: import.bindings,
            glob: import.glob,
        });
    }
}

struct Import {
    kind: EdgeKind,
    type_only: bool,
    bindings: Bindings,
    glob: bool,
}

impl Import {
    const fn static_(type_only: bool, bindings: Bindings) -> Self {
        Self {
            kind: EdgeKind::Static,
            type_only,
            bindings,
            glob: false,
        }
    }

    const fn runtime(glob: bool) -> Self {
        Self {
            kind: EdgeKind::Runtime,
            type_only: false,
            bindings: Bindings::All,
            glob,
        }
    }
}

fn import_bindings(decl: &ImportDeclaration<'_>) -> Bindings {
    let Some(specifiers) = &decl.specifiers else {
        return Bindings::None;
    };
    let decl_type = decl.import_kind.is_type();
    let mut names = Vec::with_capacity(specifiers.len());
    for spec in specifiers {
        names.push(match spec {
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => return Bindings::All,
            ImportDeclarationSpecifier::ImportSpecifier(s) => ImportedName {
                imported: s.imported.name().to_string(),
                type_only: decl_type || s.import_kind.is_type(),
            },
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => ImportedName {
                imported: "default".to_owned(),
                type_only: decl_type,
            },
        });
    }
    Bindings::Names(names)
}

impl<'a> Visit<'a> for ImportVisitor<'_> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        self.record_local_names(decl);
        let src = &decl.source;
        self.push(
            src.span,
            src.value.as_str(),
            decl.span,
            Import::static_(decl.import_kind.is_type(), import_bindings(decl)),
        );
        walk::walk_import_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(src) = decl.source.as_ref() {
            let type_only = decl.export_kind.is_type();
            let names = decl
                .specifiers
                .iter()
                .map(|s| ImportedName {
                    imported: s.local.name().to_string(),
                    type_only: type_only || s.export_kind.is_type(),
                })
                .collect();
            self.push(
                src.span,
                src.value.as_str(),
                decl.span,
                Import::static_(type_only, Bindings::Names(names)),
            );
        }
        walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let src = &decl.source;
        let bindings = if decl.exported.is_some() {
            Bindings::All
        } else {
            Bindings::Forward
        };
        self.push(
            src.span,
            src.value.as_str(),
            decl.span,
            Import::static_(decl.export_kind.is_type(), bindings),
        );
        walk::walk_export_all_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(lit) = &expr.source {
            self.push(
                lit.span,
                lit.value.as_str(),
                expr.span,
                Import::runtime(false),
            );
        }
        walk::walk_import_expression(self, expr);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.record_call(call);
        if let Some(Argument::StringLiteral(lit)) = call.arguments.first() {
            if is_require_callee(&call.callee) && call.arguments.len() == 1 {
                self.push(
                    lit.span,
                    lit.value.as_str(),
                    call.span,
                    Import::static_(false, Bindings::All),
                );
            } else if is_import_meta_glob(&call.callee) {
                self.push(
                    lit.span,
                    lit.value.as_str(),
                    call.span,
                    Import::runtime(true),
                );
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn is_require_callee(callee: &Expression<'_>) -> bool {
    matches!(callee, Expression::Identifier(id) if id.name.as_str() == "require")
}

fn is_import_meta_glob(callee: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = callee else {
        return false;
    };
    member.property.name == "glob"
        && matches!(&member.object, Expression::MetaProperty(meta)
            if meta.meta.name == "import" && meta.property.name == "meta")
}
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn collect_imports(source: &str, source_type: SourceType) -> Vec<RawImport> {
        collect(source, source_type, &TrackedCalls::default()).imports
    }

    fn call_names(source: &str, tracked: &[&str]) -> Vec<String> {
        let mut calls = TrackedCalls::default();
        for entry in tracked {
            calls.insert(entry).unwrap();
        }
        collect(source, ts_module(), &calls)
            .calls
            .into_iter()
            .map(|c| c.name)
            .collect()
    }

    #[test]
    fn ns_inject_on_a_namespace_import_matches_inject() {
        let names = call_names(
            "import * as vue from 'vue';\nvue.inject('k');\nother.inject('k');\n",
            &["inject"],
        );
        assert_eq!(names, ["inject"]);
    }

    #[test]
    fn local_alias_of_a_tracked_import_is_reported_under_the_imported_name() {
        let names = call_names(
            "import { inject as i, provide } from 'vue';\nconst a = i('k');\nprovide('k', a);\nobj.inject('k');\nother();",
            &["inject", "provide"],
        );
        assert_eq!(names, ["inject", "provide"]);
    }

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
