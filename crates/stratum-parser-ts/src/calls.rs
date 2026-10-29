use std::collections::{BTreeMap, BTreeSet};

use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Declaration, ExportDefaultDeclarationKind,
    Expression, IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, Program,
    Statement, VariableDeclarator,
};
use oxc_span::Span;
use stratum_core::types::{CallArg, CallForm};

use crate::extractor::{CallPattern, RawCall, TrackedCalls};
use crate::source_span::SourceSpan;

/// Records the calls of one module that [`TrackedCalls`] asks for, in source order.
pub(crate) struct CallRecorder<'t> {
    tracked: &'t TrackedCalls,
    aliases: Vec<(String, String)>,
    namespaces: Vec<String>,
    owners: Vec<(Span, String)>,
    /// The names of `owners`, ascending.
    top_names: Vec<String>,
    local_refs: BTreeMap<String, BTreeSet<String>>,
    clients: Vec<Client>,
    members: Vec<MemberCall>,
    calls: Vec<RawCall>,
}

struct Client {
    variable: String,
    factory: String,
    call: Span,
    args: Vec<CallArg>,
    owner: Option<String>,
    module_level: bool,
}

struct MemberCall {
    object: String,
    method: String,
    span: Span,
    owner: Option<String>,
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

impl<'t> CallRecorder<'t> {
    pub(crate) fn new(tracked: &'t TrackedCalls, program: &Program<'_>) -> Self {
        let owners = if tracked.is_empty() {
            Vec::new()
        } else {
            program.body.iter().flat_map(top_level_names).collect()
        };
        let mut top_names: Vec<String> = owners.iter().map(|(_, n)| n.clone()).collect();
        top_names.sort();
        top_names.dedup();
        Self {
            tracked,
            aliases: Vec::new(),
            namespaces: Vec::new(),
            owners,
            top_names,
            local_refs: BTreeMap::new(),
            clients: Vec::new(),
            members: Vec::new(),
            calls: Vec::new(),
        }
    }

    pub(crate) fn import(&mut self, decl: &ImportDeclaration<'_>) {
        if self.tracked.is_empty() {
            return;
        }
        for spec in decl.specifiers.iter().flatten() {
            match spec {
                ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                    let imported = spec.imported.name();
                    if spec.local.name != imported && self.is_plain_name(imported.as_str()) {
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

    pub(crate) fn declarator(&mut self, decl: &VariableDeclarator<'_>) {
        if self.tracked.factories().is_empty() {
            return;
        }
        let BindingPatternKind::BindingIdentifier(variable) = &decl.id.kind else {
            return;
        };
        let call = match &decl.init {
            Some(Expression::CallExpression(call)) => call,
            Some(Expression::AwaitExpression(wait)) => match &wait.argument {
                Expression::CallExpression(call) => call,
                _ => return,
            },
            _ => return,
        };
        let Some(factory) = self.factory_of(call) else {
            return;
        };
        let owner = self.owner(call.span);
        let module_level =
            owner.is_none() || self.owners.iter().any(|(span, _)| *span == decl.span);
        self.clients.push(Client {
            variable: variable.name.to_string(),
            factory,
            call: call.span,
            args: args(call),
            owner,
            module_level,
        });
    }

    pub(crate) fn call(&mut self, call: &CallExpression<'_>) {
        if self.tracked.is_empty() {
            return;
        }
        let Some(callee) = Callee::of(&call.callee) else {
            return;
        };
        for tracked in self.tracked.entries() {
            if self.matches(&tracked.pattern, &callee) {
                self.calls.push(RawCall {
                    name: tracked.entry.clone(),
                    span: SourceSpan::new(call.span.start, call.span.end),
                    args: args(call),
                    owner: self.owner(call.span),
                    form: CallForm::Direct,
                });
            }
        }
        if self.tracked.factories().is_empty() {
            return;
        }
        if let Some(factory) = self.factory_of(call) {
            if !self.clients.iter().any(|c| c.call == call.span) {
                self.calls.push(RawCall {
                    name: factory,
                    span: SourceSpan::new(call.span.start, call.span.end),
                    args: args(call),
                    owner: self.owner(call.span),
                    form: CallForm::Unfollowed,
                });
            }
        } else if let Callee::Member {
            object: Some(object),
            name,
        } = callee
        {
            self.members.push(MemberCall {
                object: object.to_owned(),
                method: name.to_owned(),
                span: call.span,
                owner: self.owner(call.span),
            });
        }
    }

    /// Records `reference` when it names another top-level declaration of the module than the
    /// one it stands in. Scopes are not resolved: a local that shadows a top-level name counts.
    pub(crate) fn reference(&mut self, reference: &IdentifierReference<'_>) {
        let name = reference.name.as_str();
        if self
            .top_names
            .binary_search_by(|n| n.as_str().cmp(name))
            .is_err()
        {
            return;
        }
        let Some(owner) = self.owner(reference.span) else {
            return;
        };
        if owner != name {
            self.local_refs
                .entry(owner)
                .or_default()
                .insert(name.to_owned());
        }
    }

    /// Per top-level declaration with any, ascending by name: the other top-level declarations
    /// of the module it references, ascending.
    pub(crate) fn local_refs(&self) -> Vec<(String, Vec<String>)> {
        self.local_refs
            .iter()
            .map(|(owner, refs)| (owner.clone(), refs.iter().cloned().collect()))
            .collect()
    }

    /// The recorded calls in source order. A member call is matched to the client of its
    /// variable name declared inside the same top-level declaration, the nearest preceding one
    /// first; without one, to the module-level client of that name, wherever it stands.
    pub(crate) fn finish(mut self) -> Vec<RawCall> {
        let mut used = vec![false; self.clients.len()];
        for member in self.members {
            let Some(at) = client_of(&self.clients, &member) else {
                continue;
            };
            used[at] = true;
            let client = &self.clients[at];
            self.calls.push(RawCall {
                name: client.factory.clone(),
                span: SourceSpan::new(member.span.start, member.span.end),
                args: client.args.clone(),
                owner: member.owner,
                form: CallForm::Client {
                    method: member.method,
                },
            });
        }
        for (client, used) in self.clients.into_iter().zip(used) {
            if !used {
                self.calls.push(RawCall {
                    name: client.factory,
                    span: SourceSpan::new(client.call.start, client.call.end),
                    args: client.args,
                    owner: client.owner,
                    form: CallForm::Unfollowed,
                });
            }
        }
        self.calls.sort_by_key(|c| c.span.start);
        self.calls
    }

    fn owner(&self, span: Span) -> Option<String> {
        let after = self.owners.partition_point(|(s, _)| s.start <= span.start);
        let (owner, name) = self.owners.get(after.checked_sub(1)?)?;
        (span.end <= owner.end).then(|| name.clone())
    }

    fn is_plain_name(&self, name: &str) -> bool {
        self.tracked
            .entries()
            .iter()
            .any(|t| matches!(&t.pattern, CallPattern::Plain(n) if n == name))
            || self.tracked.factories().iter().any(|f| f == name)
    }

    fn factory_of(&self, call: &CallExpression<'_>) -> Option<String> {
        let callee = Callee::of(&call.callee)?;
        self.tracked
            .factories()
            .iter()
            .find(|f| self.matches(&CallPattern::Plain((*f).clone()), &callee))
            .cloned()
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
}

fn client_of(clients: &[Client], member: &MemberCall) -> Option<usize> {
    let named = || {
        clients
            .iter()
            .enumerate()
            .filter(|(_, c)| c.variable == member.object)
    };
    let local = || named().filter(|(_, c)| !c.module_level && c.owner == member.owner);
    local()
        .rfind(|(_, c)| c.call.start <= member.span.start)
        .or_else(|| local().next())
        .or_else(|| named().find(|(_, c)| c.module_level))
        .map(|(at, _)| at)
}

fn args(call: &CallExpression<'_>) -> Vec<CallArg> {
    call.arguments
        .iter()
        .map(|arg| match arg {
            Argument::StringLiteral(s) => CallArg::Str(s.value.to_string()),
            Argument::TemplateLiteral(t) if t.expressions.is_empty() => t
                .quasis
                .first()
                .and_then(|q| q.value.cooked.as_ref())
                .map_or(CallArg::Other, |s| CallArg::Str(s.to_string())),
            Argument::Identifier(id) => CallArg::Ident(id.name.to_string()),
            _ => CallArg::Other,
        })
        .collect()
}

fn top_level_names(statement: &Statement<'_>) -> Vec<(Span, String)> {
    match statement {
        Statement::ExportNamedDeclaration(export) => export
            .declaration
            .as_ref()
            .map(declaration_names)
            .unwrap_or_default(),
        Statement::ExportDefaultDeclaration(export) => {
            let name = match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    f.id.as_ref().map(|id| id.name.to_string())
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    c.id.as_ref().map(|id| id.name.to_string())
                }
                _ => None,
            };
            vec![(export.span, name.unwrap_or_else(|| "default".to_owned()))]
        }
        _ => statement
            .as_declaration()
            .map(declaration_names)
            .unwrap_or_default(),
    }
}

fn declaration_names(decl: &Declaration<'_>) -> Vec<(Span, String)> {
    match decl {
        Declaration::VariableDeclaration(vars) => vars
            .declarations
            .iter()
            .filter_map(|d| match &d.id.kind {
                BindingPatternKind::BindingIdentifier(id) => Some((d.span, id.name.to_string())),
                _ => None,
            })
            .collect(),
        Declaration::FunctionDeclaration(f) => {
            f.id.iter()
                .map(|id| (f.span, id.name.to_string()))
                .collect()
        }
        Declaration::ClassDeclaration(c) => {
            c.id.iter()
                .map(|id| (c.span, id.name.to_string()))
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use oxc_span::SourceType;
    use stratum_core::types::{CallArg, CallForm};

    use crate::{TrackedCalls, collect};

    type Found = (String, Vec<CallArg>, Option<String>, CallForm);

    fn calls(source: &str) -> Vec<Found> {
        let mut tracked = TrackedCalls::default();
        tracked.insert("invoke").unwrap();
        tracked.insert_factory("createClient").unwrap();
        let ts = SourceType::default()
            .with_typescript(true)
            .with_module(true);
        collect(source, ts, &tracked)
            .calls
            .into_iter()
            .map(|c| (c.name, c.args, c.owner, c.form))
            .collect()
    }

    fn s(v: &str) -> CallArg {
        CallArg::Str(v.into())
    }

    fn id(v: &str) -> CallArg {
        CallArg::Ident(v.into())
    }

    #[test]
    fn a_direct_call_keeps_its_literal_and_identifier_args_and_its_top_level_owner() {
        let found = calls(
            "export async function load(x) { return invoke('a', x, 1); }
export default () => invoke(`b`);
invoke(name);
",
        );
        assert_eq!(
            found,
            [
                (
                    "invoke".into(),
                    vec![s("a"), id("x"), CallArg::Other],
                    Some("load".into()),
                    CallForm::Direct
                ),
                (
                    "invoke".into(),
                    vec![s("b")],
                    Some("default".into()),
                    CallForm::Direct
                ),
                ("invoke".into(), vec![id("name")], None, CallForm::Direct),
            ]
        );
    }

    #[test]
    fn a_member_call_on_a_factory_variable_is_a_client_call_with_the_factory_args() {
        let found = calls(
            "export const getUser = (id) => users.getUser({ id });
const users = createClient(UserService, transport);
other.getUser();
",
        );
        assert_eq!(
            found,
            [(
                "createClient".into(),
                vec![id("UserService"), id("transport")],
                Some("getUser".into()),
                CallForm::Client {
                    method: "getUser".into()
                }
            )]
        );
    }

    #[test]
    fn a_member_call_uses_the_client_of_its_own_declaration_before_a_module_level_one_of_the_same_name()
     {
        let found = calls(
            "const client = createClient(CartService);
function a() { const client = createClient(UserService); client.getUser(); }
function b() { const client = createClient(OrderService); client.placeOrder(); }
function c() { client.addItem(); }
",
        );
        let client = |arg: &str, owner: &str, method: &str| {
            (
                "createClient".into(),
                vec![id(arg)],
                Some(owner.into()),
                CallForm::Client {
                    method: method.into(),
                },
            )
        };
        assert_eq!(
            found,
            [
                client("UserService", "a", "getUser"),
                client("OrderService", "b", "placeOrder"),
                client("CartService", "c", "addItem"),
            ]
        );
    }

    fn local_refs(source: &str) -> Vec<(String, Vec<String>)> {
        let mut tracked = TrackedCalls::default();
        tracked.insert("invoke").unwrap();
        let ts = SourceType::default()
            .with_typescript(true)
            .with_module(true);
        collect(source, ts, &tracked).local_refs
    }

    #[test]
    fn a_declaration_records_the_other_top_level_declarations_it_references_but_not_types_or_imports()
     {
        let found = local_refs(
            "import { makeTransport } from './transport';
type Options = { base: string };
function helper() { return invoke('a'); }
export const createApi = (t: Options) => {
  const client = helper();
  return { get: () => client.get(), again: createApi, helper };
};
export const api = createApi(makeTransport({ base: 'x' }) as Options);
export const unrelated = 1;
export default function main() { return api; }
",
        );
        let owned = |o: &str, r: &[&str]| (o.to_owned(), r.iter().map(|&s| s.to_owned()).collect());
        assert_eq!(
            found,
            [
                owned("api", &["createApi"]),
                owned("createApi", &["helper"]),
                owned("main", &["api"]),
            ]
        );
    }

    #[test]
    fn no_local_refs_are_recorded_when_no_call_is_tracked() {
        let ts = SourceType::default()
            .with_typescript(true)
            .with_module(true);
        let source = "const a = 1; export const b = a;";
        assert!(
            collect(source, ts, &TrackedCalls::default())
                .local_refs
                .is_empty()
        );
    }

    #[test]
    fn a_factory_call_no_member_call_of_the_module_uses_is_unfollowed() {
        let found = calls(
            "export const shared = createClient(OrderService);
export function make() { return createClient(CartService); }
",
        );
        assert_eq!(
            found,
            [
                (
                    "createClient".into(),
                    vec![id("OrderService")],
                    Some("shared".into()),
                    CallForm::Unfollowed
                ),
                (
                    "createClient".into(),
                    vec![id("CartService")],
                    Some("make".into()),
                    CallForm::Unfollowed
                ),
            ]
        );
    }
}
