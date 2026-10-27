use std::collections::VecDeque;
use std::path::PathBuf;

use globset::GlobSet;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use stratum_core::{
    edge::{Edge, EdgeKind},
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::Import,
    violation::{SourceLocation, Violation},
};
use stratum_graph::{CompoundGraph, SideRole};

use crate::builtin::import_policy::glob_set;
use crate::builtin::no_circular_deps::type_only;
use crate::ids::REACHABILITY_POLICY;
use crate::paths::project_relative;
use crate::rule::Rule;
use crate::scope::ProjectScope;

const DEFAULT_MESSAGE: &str = "Denied module reachable by reachability-policy";

#[derive(Debug, Default, Clone, Copy)]
pub struct ReachabilityPolicy;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(try_from = "Vec<PolicySpec>")]
pub struct ReachabilityPolicyOptions {
    policies: Vec<Policy>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicySpec {
    from: Vec<String>,
    deny: Vec<String>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    ignore_dynamic: bool,
    #[serde(default)]
    ignore_type_only: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Debug, Clone)]
struct Policy {
    from: GlobSet,
    deny: GlobSet,
    allow: GlobSet,
    ignore_dynamic: bool,
    ignore_type_only: bool,
    message: String,
    fix: Option<String>,
}

impl TryFrom<Vec<PolicySpec>> for ReachabilityPolicyOptions {
    type Error = String;

    fn try_from(specs: Vec<PolicySpec>) -> Result<Self, String> {
        let policies = specs
            .into_iter()
            .map(|spec| {
                Ok(Policy {
                    from: glob_set(&spec.from)?,
                    deny: glob_set(&spec.deny)?,
                    allow: glob_set(&spec.allow)?,
                    ignore_dynamic: spec.ignore_dynamic,
                    ignore_type_only: spec.ignore_type_only,
                    message: spec.message.unwrap_or_else(|| DEFAULT_MESSAGE.into()),
                    fix: spec.fix,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { policies })
    }
}

impl Policy {
    fn follows(&self, import: &Import) -> bool {
        !(self.ignore_dynamic && import.kind == EdgeKind::Runtime
            || self.ignore_type_only && type_only(import))
    }
}

/// Where a search starts: a module, or an entry point outside every layer.
struct Start<'g> {
    module: Option<ModuleId>,
    path: &'g std::path::Path,
    relative: String,
    imports: &'g [Import],
}

impl Rule for ReachabilityPolicy {
    type Scope = ProjectScope;
    type Options = ReachabilityPolicyOptions;

    fn id(&self) -> RuleId {
        REACHABILITY_POLICY
    }
    fn slug(&self) -> &'static str {
        "stratum/reachability-policy"
    }
    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        _scope: ProjectScope,
        options: &ReachabilityPolicyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        if options.policies.is_empty() {
            return Vec::new();
        }
        let paths: FxHashMap<ModuleId, String> = graph
            .modules
            .values()
            .map(|m| (m.id, project_relative(graph, m.layer, &m.path)))
            .collect();
        let modules = graph.modules.values().map(|m| Start {
            module: Some(m.id),
            path: &m.path,
            relative: paths[&m.id].clone(),
            imports: &m.imports,
        });
        let sides = graph
            .side_files
            .iter()
            .filter(|s| s.role == SideRole::EntryPoint)
            .map(|s| Start {
                module: None,
                path: &s.path,
                relative: s.relative.clone(),
                imports: &s.imports,
            });
        let starts: Vec<Start<'_>> = modules.chain(sides).collect();
        let mut out = Vec::new();
        for policy in &options.policies {
            for start in starts.iter().filter(|s| policy.from.is_match(&s.relative)) {
                if let Some(v) = self.search(graph, &paths, policy, start, severity) {
                    out.push(v);
                }
            }
        }
        out
    }
}

impl ReachabilityPolicy {
    /// Breadth-first from `start`: the first denied module found has the shortest chain.
    fn search(
        self,
        graph: &CompoundGraph,
        paths: &FxHashMap<ModuleId, String>,
        policy: &Policy,
        start: &Start<'_>,
        severity: Severity,
    ) -> Option<Violation> {
        let denied = |id: ModuleId| {
            let path = &paths[&id];
            policy.deny.is_match(path) && !policy.allow.is_match(path)
        };
        let mut parent: FxHashMap<ModuleId, Option<ModuleId>> = FxHashMap::default();
        let mut queue = VecDeque::new();
        for import in start.imports.iter().filter(|i| policy.follows(i)) {
            if let Some(t) = import.target
                && Some(t) != start.module
                && !parent.contains_key(&t)
            {
                parent.insert(t, None);
                queue.push_back(t);
            }
        }
        let mut nearest = None;
        let mut more = 0usize;
        while let Some(id) = queue.pop_front() {
            if denied(id) {
                if nearest.is_none() {
                    nearest = Some(id);
                } else {
                    more += 1;
                }
            }
            let Some(m) = graph.modules.get(&id) else {
                continue;
            };
            for import in m.imports.iter().filter(|i| policy.follows(i)) {
                if let Some(t) = import.target
                    && Some(t) != start.module
                    && !parent.contains_key(&t)
                {
                    parent.insert(t, Some(id));
                    queue.push_back(t);
                }
            }
        }
        let nearest = nearest?;
        let mut chain = vec![nearest];
        while let Some(Some(up)) = parent.get(chain.last()?) {
            chain.push(*up);
        }
        chain.reverse();
        let first = chain[0];
        let first_import = start
            .imports
            .iter()
            .find(|i| i.target == Some(first) && policy.follows(i))?;
        let shown: Vec<&str> = std::iter::once(start.relative.as_str())
            .chain(chain.iter().map(|id| paths[id].as_str()))
            .collect();
        let tail = if more == 0 {
            String::new()
        } else {
            format!(" (and {more} more denied modules)")
        };
        let mut modules: Vec<ModuleId> = start.module.into_iter().collect();
        modules.extend(&chain);
        Some(Violation {
            rule: self.id(),
            severity,
            message: format!(
                "{}: `{}` reaches `{}` through {}{tail}",
                policy.message,
                start.relative,
                paths[&nearest],
                shown.join(" -> ")
            ),
            file: PathBuf::from(start.path),
            location: SourceLocation {
                line: first_import.location.line,
                column: first_import.location.column,
            },
            modules,
            edge: start.module.map(|from| Edge {
                from,
                to: first,
                kind: first_import.kind,
            }),
            fix: policy.fix.clone(),
        })
    }
}
