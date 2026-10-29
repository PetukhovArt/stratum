use std::ffi::OsStr;
use std::path::Path;
use std::sync::OnceLock;

use globset::{Candidate, GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use rustc_hash::FxHashMap;
use stratum_core::ids::LayerId;
use stratum_core::types::Module;
use stratum_core::{
    ids::{ModuleId, RuleId},
    severity::Severity,
    violation::Violation,
};
use stratum_graph::CompoundGraph;

use crate::builtin::edge::{EdgeJudge, Finding, edge_violations};
use crate::docs::RuleDoc;
use crate::ids::IMPORT_POLICY;
use crate::paths::project_relative;
use crate::rule::Rule;

const DEFAULT_MESSAGE: &str = "Import denied by import-policy";
const MAX_WORKERS: usize = 8;

#[derive(Debug, Default, Clone, Copy)]
pub struct ImportPolicy;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(try_from = "Vec<PolicySpec>")]
pub struct ImportPolicyOptions {
    matcher: Matcher,
    messages: Vec<String>,
    fixes: Vec<Option<String>>,
    table: OnceLock<Option<Table>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicySpec {
    from: Vec<String>,
    #[serde(default)]
    except_from: Vec<String>,
    deny: Vec<String>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    slice: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Role {
    From,
    ExceptFrom,
    Deny,
    Allow,
}

const ROLES: usize = 4;

#[derive(Debug, Clone, Default)]
struct Matcher {
    policies: usize,
    all: GlobSet,
    roles: Vec<usize>,
    slices: GlobSet,
    slice_policy: Vec<usize>,
}

impl TryFrom<Vec<PolicySpec>> for ImportPolicyOptions {
    type Error = String;

    fn try_from(specs: Vec<PolicySpec>) -> Result<Self, String> {
        let mut all = GlobSetBuilder::new();
        let mut roles = Vec::new();
        let mut slices = GlobSetBuilder::new();
        let mut slice_policy = Vec::new();
        let mut messages = Vec::with_capacity(specs.len());
        let mut fixes = Vec::with_capacity(specs.len());
        for (i, spec) in specs.into_iter().enumerate() {
            for (role, patterns) in [
                (Role::From, &spec.from),
                (Role::ExceptFrom, &spec.except_from),
                (Role::Deny, &spec.deny),
                (Role::Allow, &spec.allow),
            ] {
                for p in patterns {
                    all.add(glob(p)?);
                    roles.push(i * ROLES + role as usize);
                }
            }
            if let Some(p) = &spec.slice {
                slices.add(glob(p)?);
                slice_policy.push(i);
            }
            messages.push(spec.message.unwrap_or_else(|| DEFAULT_MESSAGE.into()));
            fixes.push(spec.fix);
        }
        Ok(Self {
            matcher: Matcher {
                policies: messages.len(),
                all: all.build().map_err(|e| e.to_string())?,
                roles,
                slices: slices.build().map_err(|e| e.to_string())?,
                slice_policy,
            },
            messages,
            fixes,
            table: OnceLock::new(),
        })
    }
}

fn glob(pattern: &str) -> Result<globset::Glob, String> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|e| format!("invalid glob {pattern:?}: {e}"))
}

pub(crate) fn glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(glob(p)?);
    }
    builder.build().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Default)]
struct Rows {
    governs: Vec<bool>,
    denies: Vec<bool>,
    slice_roots: Vec<Option<usize>>,
    paths: Vec<String>,
}

#[derive(Default)]
struct Scratch {
    hits: Vec<usize>,
    matched: Vec<bool>,
}

impl Matcher {
    fn push_row(
        &self,
        path: String,
        slice_row: &[Option<usize>],
        scratch: &mut Scratch,
        rows: &mut Rows,
    ) {
        self.all
            .matches_candidate_into(&Candidate::new(&path), &mut scratch.hits);
        scratch.matched.clear();
        scratch.matched.resize(self.policies * ROLES, false);
        for &h in &scratch.hits {
            scratch.matched[self.roles[h]] = true;
        }
        for p in 0..self.policies {
            let role = |r: Role| scratch.matched[p * ROLES + r as usize];
            rows.governs
                .push(role(Role::From) && !role(Role::ExceptFrom));
            rows.denies.push(role(Role::Deny) && !role(Role::Allow));
        }
        rows.slice_roots.extend_from_slice(slice_row);
        rows.paths.push(path);
    }

    fn slice_row(&self, dir: &str) -> Vec<Option<usize>> {
        let mut row = vec![None; self.policies];
        if self.slice_policy.is_empty() || dir.is_empty() {
            return row;
        }
        let ends = dir
            .match_indices('/')
            .map(|(end, _)| end)
            .chain([dir.len()]);
        for end in ends {
            for s in self.slices.matches(&dir[..end]) {
                row[self.slice_policy[s]].get_or_insert(end);
            }
        }
        row
    }

    fn row_of(
        &self,
        graph: &CompoundGraph,
        module: &Module,
        scratch: &mut Scratch,
        rows: &mut Rows,
    ) {
        let (dir, name) = split_relative(graph, module);
        let slice_row = self.slice_row(&dir);
        self.push_row(joined(&dir, &name), &slice_row, scratch, rows);
    }

    fn first_denying(&self, rows: &Rows, from: usize, to: usize) -> Option<usize> {
        let (f, t) = (from * self.policies, to * self.policies);
        (0..self.policies).find(|&p| {
            rows.governs[f + p]
                && rows.denies[t + p]
                && !rows.slice_roots[f + p].is_some_and(|len| {
                    rows.slice_roots[t + p] == Some(len)
                        && rows.paths[from][..len] == rows.paths[to][..len]
                })
        })
    }
}

type DirRow = (String, Vec<Option<usize>>);

#[derive(Debug, Clone)]
struct Table {
    graph: usize,
    modules: usize,
    rows: Rows,
}

impl Table {
    fn of(graph: &CompoundGraph, matcher: &Matcher) -> Option<Self> {
        let modules: Vec<&Module> = graph.modules.values().collect();
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .clamp(1, MAX_WORKERS);
        let chunk = modules.len().div_ceil(workers).max(1);
        let rows_of = |part: &[&Module]| {
            let mut rows = Rows::default();
            let mut scratch = Scratch::default();
            let mut dirs: FxHashMap<(LayerId, &OsStr), DirRow> = FxHashMap::default();
            for m in part {
                let parent = m.path.parent().map_or(OsStr::new(""), Path::as_os_str);
                let name = m.path.file_name().map(OsStr::to_string_lossy);
                let (Some(name), false) = (name, parent.is_empty()) else {
                    matcher.row_of(graph, m, &mut scratch, &mut rows);
                    continue;
                };
                let (dir, slice_row) = dirs.entry((m.layer, parent)).or_insert_with(|| {
                    let dir = project_relative(graph, m.layer, Path::new(parent));
                    let slice_row = matcher.slice_row(&dir);
                    (dir, slice_row)
                });
                matcher.push_row(joined(dir, &name), slice_row, &mut scratch, &mut rows);
            }
            rows
        };
        let parts: Vec<Rows> = std::thread::scope(|s| {
            let handles: Vec<_> = modules
                .chunks(chunk)
                .map(|part| s.spawn(move || rows_of(part)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().ok())
                .collect::<Vec<_>>()
        })
        .into_iter()
        .collect::<Option<_>>()?;
        let mut rows = Rows::default();
        for part in parts {
            rows.governs.extend(part.governs);
            rows.denies.extend(part.denies);
            rows.slice_roots.extend(part.slice_roots);
            rows.paths.extend(part.paths);
        }
        Some(Self {
            graph: std::ptr::from_ref(graph).addr(),
            modules: graph.modules.len(),
            rows,
        })
    }

    fn built_for(&self, graph: &CompoundGraph) -> bool {
        self.graph == std::ptr::from_ref(graph).addr() && self.modules == graph.modules.len()
    }
}

impl Rule for ImportPolicy {
    type Scope = ModuleId;
    type Options = ImportPolicyOptions;

    fn id(&self) -> RuleId {
        IMPORT_POLICY
    }
    fn slug(&self) -> &'static str {
        "stratum/import-policy"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn doc(&self) -> Option<RuleDoc> {
        Some(RuleDoc {
            summary: "Denies imports between path globs declared in config, without scripting."
                .into(),
            why: "Layers say which layer may import which; import-policy covers the finer boundaries: slices that must not reach into each other, private folders reachable only through an index, zones a team has closed.".into(),
            fix: "Import through the allowed entry point (often the slice's index), move the shared code down into a layer both sides may import, or change the policy if the boundary moved on purpose.".into(),
            options: vec![
                "options: a list of policies; an import is reported by the first policy that denies it".into(),
                "from: globs of importing files, relative to the project root (required)".into(),
                "except_from: importing files the policy skips ([])".into(),
                "deny: globs of imported files (required)".into(),
                "allow: imported files exempt from deny, e.g. a slice's index.ts ([])".into(),
                "slice: glob of slice directories, e.g. src/features/*; imports inside one slice are allowed (none)".into(),
                format!("message: text of the violation; the imported path is appended (\"{DEFAULT_MESSAGE}\")"),
                "fix: what to use instead, module and export; replaces the rule's generic fix in the finding (none)".into(),
            ],
        })
    }

    fn edge_judge(&self) -> Option<EdgeJudge<ImportPolicyOptions>> {
        Some(EdgeJudge(judge))
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        options: &ImportPolicyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        if options.messages.is_empty() {
            return Vec::new();
        }
        let table = options
            .table
            .get_or_init(|| Table::of(graph, &options.matcher))
            .as_ref()
            .filter(|t| t.built_for(graph));
        let mut seen = Vec::new();
        edge_violations(graph, scope, self.id(), severity, |to| {
            if seen.contains(&to) {
                return None;
            }
            seen.push(to);
            let Some(table) = table else {
                return judge(graph, scope, to, options);
            };
            let (from, to) = (index_of(graph, scope)?, index_of(graph, to)?);
            let policy = options.matcher.first_denying(&table.rows, from, to)?;
            Some(denied(options, policy, &table.rows.paths[to]))
        })
    }
}

pub(crate) fn judge(
    graph: &CompoundGraph,
    from: ModuleId,
    to: ModuleId,
    options: &ImportPolicyOptions,
) -> Option<Finding> {
    let mut rows = Rows::default();
    let mut scratch = Scratch::default();
    for id in [from, to] {
        let m = graph.modules.get(&id)?;
        options.matcher.row_of(graph, m, &mut scratch, &mut rows);
    }
    let policy = options.matcher.first_denying(&rows, 0, 1)?;
    Some(denied(options, policy, &rows.paths[1]))
}

fn split_relative(graph: &CompoundGraph, module: &Module) -> (String, String) {
    let path = project_relative(graph, module.layer, &module.path);
    match path.rsplit_once('/') {
        Some((dir, name)) => (dir.to_owned(), name.to_owned()),
        None => (String::new(), path),
    }
}

fn joined(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        return name.to_owned();
    }
    let mut path = String::with_capacity(dir.len() + 1 + name.len());
    path.push_str(dir);
    path.push('/');
    path.push_str(name);
    path
}

fn index_of(graph: &CompoundGraph, id: ModuleId) -> Option<usize> {
    let dense = usize::try_from(id.raw()).ok()?;
    match graph.modules.get_index(dense) {
        Some((&at, _)) if at == id => Some(dense),
        _ => graph.modules.get_index_of(&id),
    }
}

fn denied(options: &ImportPolicyOptions, policy: usize, to: &str) -> Finding {
    Finding {
        message: format!("{} ({to})", options.messages[policy]),
        fix: options.fixes[policy].clone(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId},
        purity::Purity,
        types::{Layer, Module},
        visibility::VisibilityScope,
    };

    fn graph(paths: &[&str], edges: &[(u32, u32)]) -> CompoundGraph {
        let mut g = CompoundGraph::empty();
        g.layers.insert(
            LayerId::new(0),
            Layer {
                id: LayerId::new(0),
                name: "web".into(),
                path: PathBuf::from("src/web"),
                depends_on: vec![],
                slices: false,
            },
        );
        for (i, path) in paths.iter().enumerate() {
            let id = ModuleId::new(u32::try_from(i).unwrap());
            let node = g.deps.add_node(id);
            g.node_index.insert(id, node);
            g.modules.insert(
                id,
                Module {
                    id,
                    path: PathBuf::from(format!("C:/repo/{path}")),
                    container: ContainerId::new(0),
                    layer: LayerId::new(0),
                    purity: Purity::new(2).unwrap(),
                    visibility: VisibilityScope::Public,
                    exports: stratum_core::types::Exports::default(),
                    calls: vec![],
                    imports: vec![],
                    suppressions: vec![],
                    local_refs: vec![],
                },
            );
        }
        for (a, b) in edges {
            let (na, nb) = (
                g.node_index[&ModuleId::new(*a)],
                g.node_index[&ModuleId::new(*b)],
            );
            g.deps.add_edge(na, nb, EdgeKind::Static);
        }
        g
    }

    #[test]
    fn cross_slice_import_is_denied_while_same_slice_and_allowed_entry_pass() {
        let g = graph(
            &[
                "src/web/features/auth/ui/Login.vue",
                "src/web/features/auth/model/session.ts",
                "src/web/features/cart/model/cart.ts",
                "src/web/features/cart/index.ts",
            ],
            &[(0, 1), (0, 2), (0, 3)],
        );
        let options: ImportPolicyOptions = serde_json::from_value(serde_json::json!([{
            "from": ["src/web/features/**"],
            "deny": ["src/web/features/**"],
            "allow": ["src/web/features/*/index.ts"],
            "slice": "src/web/features/*",
            "message": "A feature does not import another feature",
            "fix": "Import the feature's index.ts",
        }]))
        .unwrap();
        let v = ImportPolicy.check(&g, ModuleId::new(0), &options, Severity::Error);
        assert_eq!(v.len(), 1, "{v:?}");
        assert_eq!(
            v[0].message,
            "A feature does not import another feature (src/web/features/cart/model/cart.ts)"
        );
        assert_eq!(v[0].fix.as_deref(), Some("Import the feature's index.ts"));
    }

    #[test]
    fn unknown_policy_key_and_bad_glob_are_rejected() {
        let typo = serde_json::from_value::<ImportPolicyOptions>(serde_json::json!([
            { "from": ["a/**"], "deny": ["b/**"], "excpet": ["c"] }
        ]));
        assert!(typo.unwrap_err().to_string().contains("excpet"));
        let glob = serde_json::from_value::<ImportPolicyOptions>(serde_json::json!([
            { "from": ["a/{**"], "deny": ["b/**"] }
        ]));
        assert!(glob.is_err());
    }
}
