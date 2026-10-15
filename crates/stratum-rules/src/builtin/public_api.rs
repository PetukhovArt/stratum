use petgraph::Direction;
use petgraph::visit::EdgeRef;

use stratum_core::{
    edge::Edge,
    ids::{ModuleId, RuleId},
    severity::Severity,
    types::{Container, ContainerKind},
    violation::{SourceLocation, Violation},
};
use stratum_graph::CompoundGraph;

use crate::ids::PUBLIC_API;
use crate::paths::project_relative;
use crate::rule::{EmptyOptions, Rule};

#[derive(Debug, Default, Clone, Copy)]
pub struct PublicApi;

impl Rule for PublicApi {
    type Scope = ModuleId;
    type Options = EmptyOptions;

    fn id(&self) -> RuleId {
        PUBLIC_API
    }
    fn slug(&self) -> &'static str {
        "stratum/public-api"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(
        &self,
        graph: &CompoundGraph,
        scope: ModuleId,
        _options: &EmptyOptions,
        severity: Severity,
    ) -> Vec<Violation> {
        edge_violations(graph, scope, self.id(), severity, |to| {
            let entered = entered_container(graph, scope, to)?;
            if entered.entry == Some(to) {
                return None;
            }
            let unit = project_relative(graph, entered.layer, &entered.path);
            let target = graph.modules.get(&to)?;
            let target = project_relative(graph, target.layer, &target.path);
            let entry = entered
                .entry
                .and_then(|e| graph.modules.get(&e))
                .map(|e| project_relative(graph, e.layer, &e.path));
            Some(match entry {
                Some(entry) => Finding {
                    message: format!("Import of `{target}` reaches inside `{unit}`"),
                    suggestion: format!(
                        "Import from `{unit}` through its entry `{entry}`, exporting the name there if it is missing."
                    ),
                },
                None => Finding {
                    message: format!(
                        "Import of `{target}` reaches inside `{unit}`, which has no entry file"
                    ),
                    suggestion: format!(
                        "Add an entry file to `{unit}` (a name from `structure.entries`, e.g. index.ts) that exports what outside code needs, and import through it."
                    ),
                },
            })
        })
    }
}

fn entered_container(graph: &CompoundGraph, from: ModuleId, to: ModuleId) -> Option<&Container> {
    graph
        .containers_of(to)
        .filter(|c| c.kind != ContainerKind::Layer)
        .take_while(|c| !graph.containers_of(from).any(|f| f.id == c.id))
        .last()
}

pub(crate) struct Finding {
    pub message: String,
    pub suggestion: String,
}

pub(crate) fn edge_violations(
    graph: &CompoundGraph,
    scope: ModuleId,
    rule: RuleId,
    severity: Severity,
    mut judge: impl FnMut(ModuleId) -> Option<Finding>,
) -> Vec<Violation> {
    let (Some(from), Some(node)) = (graph.modules.get(&scope), graph.node_for(scope)) else {
        return vec![];
    };
    let mut out: Vec<Violation> = graph
        .deps
        .edges_directed(node, Direction::Outgoing)
        .filter_map(|edge| {
            let to = graph.deps[edge.target()];
            let finding = judge(to)?;
            Some(Violation {
                rule,
                severity,
                message: finding.message,
                file: from.path.clone(),
                location: SourceLocation { line: 1, column: 1 },
                modules: vec![scope, to],
                edge: Some(Edge {
                    from: scope,
                    to,
                    kind: *edge.weight(),
                }),
                suggestion: Some(finding.suggestion),
            })
        })
        .collect();
    out.sort_by_key(|v| v.modules[1].raw());
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) mod fixture {
    use std::path::PathBuf;

    use stratum_core::{
        edge::EdgeKind,
        ids::{ContainerId, LayerId, ModuleId},
        purity::Purity,
        stage::Stage,
        types::{Container, ContainerKind, Exports, Layer, Module},
        visibility::VisibilityScope,
    };
    use stratum_graph::CompoundGraph;

    pub const ROOT: &str = "/repo";

    fn abs(rel: &str) -> PathBuf {
        PathBuf::from(format!("{ROOT}/{rel}"))
    }

    #[derive(Default)]
    pub struct Fixture {
        pub g: CompoundGraph,
    }

    impl Fixture {
        pub fn layer(&mut self, raw: u32, rel: &str) -> ContainerId {
            let id = LayerId::new(raw);
            self.g.layers.insert(
                id,
                Layer {
                    id,
                    name: rel.rsplit('/').next().unwrap().into(),
                    path: PathBuf::from(rel),
                    depends_on: vec![],
                    slices: true,
                },
            );
            self.insert(rel, ContainerKind::Layer, id, None)
        }

        pub fn unit(&mut self, rel: &str, kind: ContainerKind, parent: ContainerId) -> ContainerId {
            let layer = self.g.containers[&parent].layer;
            self.insert(rel, kind, layer, Some(parent))
        }

        fn insert(
            &mut self,
            rel: &str,
            kind: ContainerKind,
            layer: LayerId,
            parent: Option<ContainerId>,
        ) -> ContainerId {
            let id = ContainerId::new(u32::try_from(self.g.containers.len()).unwrap());
            self.g.containers.insert(
                id,
                Container {
                    id,
                    name: rel.rsplit('/').next().unwrap().into(),
                    path: abs(rel),
                    kind,
                    layer,
                    parent,
                    stage: Stage::FLAT,
                    entry: None,
                    segment_dirs: vec![],
                },
            );
            id
        }

        pub fn module(&mut self, rel: &str, private: Option<&str>) -> ModuleId {
            let path = abs(rel);
            let container = self.g.container_at(&path).unwrap();
            let (container, layer) = (container.id, container.layer);
            let id = ModuleId::new(u32::try_from(self.g.modules.len()).unwrap());
            let node = self.g.deps.add_node(id);
            self.g.node_index.insert(id, node);
            self.g.modules.insert(
                id,
                Module {
                    id,
                    path,
                    container,
                    layer,
                    purity: Purity::new(2).unwrap(),
                    visibility: private.map_or(VisibilityScope::Public, |root| {
                        VisibilityScope::Private { root: abs(root) }
                    }),
                    exports: Exports::default(),
                    calls: vec![],
                },
            );
            id
        }

        pub fn entry(&mut self, container: ContainerId, rel: &str) -> ModuleId {
            let m = self.module(rel, None);
            self.g.containers[&container].entry = Some(m);
            m
        }

        pub fn edge(&mut self, from: ModuleId, to: ModuleId) {
            let (a, b) = (self.g.node_index[&from], self.g.node_index[&to]);
            self.g.deps.add_edge(a, b, EdgeKind::Static);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::fixture::Fixture;
    use super::*;
    use stratum_core::ids::ContainerId;

    fn check(f: &Fixture, from: ModuleId) -> Vec<Violation> {
        PublicApi.check(&f.g, from, &EmptyOptions, Severity::Error)
    }

    fn two_slices() -> (Fixture, ModuleId, ModuleId) {
        let mut f = Fixture::default();
        let layer = f.layer(0, "src/features");
        let cart = f.unit("src/features/cart", ContainerKind::Slice, layer);
        let user = f.unit("src/features/user", ContainerKind::Slice, layer);
        f.entry(cart, "src/features/cart/index.ts");
        f.entry(user, "src/features/user/index.ts");
        let importer = f.module("src/features/cart/ui/cart.vue", None);
        let deep = f.module("src/features/user/model/store.ts", None);
        (f, importer, deep)
    }

    #[test]
    fn deep_import_into_a_slice_names_its_entry() {
        let (mut f, importer, deep) = two_slices();
        f.edge(importer, deep);
        let v = check(&f, importer);
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Import of `src/features/user/model/store.ts` reaches inside `src/features/user`"
        );
        assert!(
            v[0].suggestion
                .as_deref()
                .unwrap()
                .contains("`src/features/user/index.ts`")
        );
    }

    #[test]
    fn import_of_the_entry_is_allowed() {
        let (mut f, importer, _) = two_slices();
        let entry = f.g.containers[&ContainerId::new(2)].entry.unwrap();
        f.edge(importer, entry);
        assert!(check(&f, importer).is_empty());
    }

    #[test]
    fn import_inside_the_same_slice_is_allowed() {
        let (mut f, importer, _) = two_slices();
        let neighbour = f.module("src/features/cart/model/cart.ts", None);
        f.edge(importer, neighbour);
        assert!(check(&f, importer).is_empty());
    }

    #[test]
    fn outermost_entered_container_decides_not_the_innermost() {
        let (mut f, importer, _) = two_slices();
        let user = ContainerId::new(2);
        let profile = f.unit("src/features/user/profile", ContainerKind::SubUnit, user);
        let profile_entry = f.entry(profile, "src/features/user/profile/index.ts");
        f.edge(importer, profile_entry);
        let v = check(&f, importer);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.ends_with("reaches inside `src/features/user`"));
    }

    #[test]
    fn parent_reaching_past_a_sub_unit_entry_is_flagged() {
        let (mut f, _, _) = two_slices();
        let user = ContainerId::new(2);
        let profile = f.unit("src/features/user/profile", ContainerKind::SubUnit, user);
        f.entry(profile, "src/features/user/profile/index.ts");
        let composer = f.module("src/features/user/ui/page.vue", None);
        let deep = f.module("src/features/user/profile/ui/card.vue", None);
        f.edge(composer, deep);
        let v = check(&f, composer);
        assert_eq!(v.len(), 1);
        assert!(
            v[0].message
                .ends_with("reaches inside `src/features/user/profile`")
        );
    }

    #[test]
    fn container_without_entry_says_so() {
        let mut f = Fixture::default();
        let layer = f.layer(0, "src/features");
        f.unit("src/features/cart", ContainerKind::Slice, layer);
        f.unit("src/features/user", ContainerKind::Slice, layer);
        let importer = f.module("src/features/cart/a.ts", None);
        let deep = f.module("src/features/user/b.ts", None);
        f.edge(importer, deep);
        let v = check(&f, importer);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.ends_with("which has no entry file"));
        assert!(
            v[0].suggestion
                .as_deref()
                .unwrap()
                .contains("structure.entries")
        );
    }

    #[test]
    fn layer_root_files_are_not_guarded() {
        let mut f = Fixture::default();
        f.layer(0, "src/shared");
        let features = f.layer(1, "src/features");
        f.unit("src/features/cart", ContainerKind::Slice, features);
        let importer = f.module("src/features/cart/a.ts", None);
        let util = f.module("src/shared/lib/format.ts", None);
        f.edge(importer, util);
        assert!(check(&f, importer).is_empty());
    }
}
