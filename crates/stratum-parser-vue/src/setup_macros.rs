use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::Visit;
use oxc_ast::ast::{
    Argument, CallExpression, Declaration, Expression, Program, Statement, TSType, TSTypeName,
};
use oxc_ast::visit::walk;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Names a `<script setup>` block exposes through `defineProps`, `defineEmits`,
/// `defineExpose` and `defineModel`. A props or emits type that is neither a
/// type literal nor a local interface / object type alias counts as 1.
pub(crate) fn surface(body: &str, source_type: SourceType) -> u32 {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, body, source_type).parse();
    let mut visitor = MacroVisitor {
        local_types: local_type_sizes(&ret.program),
        count: 0,
    };
    visitor.visit_program(&ret.program);
    u32::try_from(visitor.count).unwrap_or(u32::MAX)
}

struct MacroVisitor {
    local_types: HashMap<String, usize>,
    count: usize,
}

impl MacroVisitor {
    fn declared_size(&self, call: &CallExpression<'_>) -> usize {
        let Some(ty) = call.type_parameters.as_ref().and_then(|t| t.params.first()) else {
            return runtime_size(call);
        };
        match ty {
            TSType::TSTypeLiteral(lit) => lit.members.len(),
            TSType::TSTypeReference(r) => match &r.type_name {
                TSTypeName::IdentifierReference(id) => {
                    self.local_types.get(id.name.as_str()).copied().unwrap_or(1)
                }
                TSTypeName::QualifiedName(_) => 1,
            },
            _ => 1,
        }
    }
}

fn runtime_size(call: &CallExpression<'_>) -> usize {
    match call.arguments.first() {
        Some(Argument::ObjectExpression(obj)) => obj.properties.len(),
        Some(Argument::ArrayExpression(arr)) => arr.elements.len(),
        Some(_) => 1,
        None => 0,
    }
}

impl<'a> Visit<'a> for MacroVisitor {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &call.callee {
            self.count += match callee.name.as_str() {
                "defineProps" | "defineEmits" => self.declared_size(call),
                "defineExpose" => runtime_size(call),
                "defineModel" => 1,
                _ => 0,
            };
        }
        walk::walk_call_expression(self, call);
    }
}

fn local_type_sizes(program: &Program<'_>) -> HashMap<String, usize> {
    let mut sizes = HashMap::new();
    for stmt in &program.body {
        let declaration = match stmt {
            Statement::ExportNamedDeclaration(named) => named.declaration.as_ref(),
            _ => stmt.as_declaration(),
        };
        match declaration {
            Some(Declaration::TSInterfaceDeclaration(d)) => {
                sizes.insert(d.id.name.to_string(), d.body.body.len());
            }
            Some(Declaration::TSTypeAliasDeclaration(d)) => {
                if let TSType::TSTypeLiteral(lit) = &d.type_annotation {
                    sizes.insert(d.id.name.to_string(), lit.members.len());
                }
            }
            _ => {}
        }
    }
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> SourceType {
        SourceType::default()
            .with_typescript(true)
            .with_module(true)
    }

    #[test]
    fn counts_members_of_every_macro() {
        let body = "
            interface Props { a: string; b?: number }
            const props = withDefaults(defineProps<Props>(), { b: 1 });
            const emit = defineEmits<{ (e: 'change', id: number): void; (e: 'close'): void }>();
            const model = defineModel<string>();
            defineExpose({ focus, reset, open });
        ";
        assert_eq!(surface(body, ts()), 2 + 2 + 1 + 3);
    }

    #[test]
    fn counts_runtime_declarations() {
        let body = "
            defineProps({ a: String, b: Number, c: Boolean });
            defineEmits(['save', 'cancel']);
        ";
        assert_eq!(surface(body, ts()), 5);
    }
}
