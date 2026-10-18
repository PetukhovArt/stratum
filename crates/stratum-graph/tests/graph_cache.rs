#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
use stratum_core::{ids::LayerId, purity::Purity, types::Layer};
use stratum_graph::{BuildConfig, CacheState, CacheStats, CompoundGraph, GraphBuilder};

struct Project {
    _dir: tempfile::TempDir,
    root: Utf8PathBuf,
    cache: Utf8PathBuf,
}

impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let base = Utf8PathBuf::from_path_buf(dir.path().canonicalize().unwrap()).unwrap();
        let project = Self {
            root: base.join("app"),
            cache: base.join("cache"),
            _dir: dir,
        };
        for (path, text) in files {
            project.write(path, text);
        }
        project
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn builder(&self, cache: bool) -> GraphBuilder {
        GraphBuilder::new(BuildConfig {
            project_root: self.root.clone(),
            layers: vec![Layer {
                id: LayerId::new(0),
                name: "src".into(),
                path: PathBuf::from("src"),
                depends_on: vec![],
                slices: false,
            }],
            structure: stratum_core::structure::Structure::default(),
            tracked_calls: stratum_graph::TrackedCalls::default(),
            default_purity: Purity::new(2).unwrap(),
        })
        .with_cache_dir(cache.then(|| self.cache.clone()), "test")
    }

    fn build(&self) -> (CompoundGraph, CacheStats) {
        self.builder(true).build().unwrap()
    }

    fn cold(&self) -> CompoundGraph {
        self.builder(false).build().unwrap().0
    }

    fn cache_file(&self) -> Utf8PathBuf {
        let entry = fs::read_dir(&self.cache)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.extension().is_some_and(|e| e == "graph"))
            .unwrap();
        Utf8PathBuf::from_path_buf(entry).unwrap()
    }
}

fn targets_of(graph: &CompoundGraph, from: &str) -> Vec<String> {
    let module = graph
        .modules
        .values()
        .find(|m| Utf8Path::from_path(&m.path).unwrap().ends_with(from))
        .unwrap();
    module
        .imports
        .iter()
        .map(|i| {
            i.target.map_or_else(
                || format!("{} -> none", i.specifier),
                |t| {
                    let to = graph.modules[&t].path.to_string_lossy().replace('\\', "/");
                    format!("{} -> {}", i.specifier, to.rsplit("/app/").next().unwrap())
                },
            )
        })
        .collect()
}

fn same(a: &CompoundGraph, b: &CompoundGraph) -> bool {
    let edges = |g: &CompoundGraph| {
        g.deps
            .raw_edges()
            .iter()
            .map(|e| (g.deps[e.source()], g.deps[e.target()], e.weight))
            .collect::<Vec<_>>()
    };
    a.modules == b.modules && edges(a) == edges(b)
}

#[test]
fn an_edit_inside_the_racy_window_is_not_served_from_the_cache() {
    let p = Project::new(&[
        ("src/a.ts", "import './b';\n"),
        ("src/b.ts", "export {};\n"),
        ("src/c.ts", "export {};\n"),
    ]);
    let a = p.root.join("src/a.ts");
    let written = fs::metadata(&a).unwrap().modified().unwrap();
    assert_eq!(p.build().1.state, CacheState::Cold);

    fs::write(&a, "import './c';\n").unwrap();
    fs::File::options()
        .write(true)
        .open(&a)
        .unwrap()
        .set_modified(written)
        .unwrap();

    let (graph, stats) = p.build();
    assert_eq!(stats.state, CacheState::Warm);
    assert_eq!(targets_of(&graph, "src/a.ts"), ["./c -> src/c.ts"]);
}

#[test]
fn a_corrupt_cache_file_gives_the_cold_run_graph() {
    let p = Project::new(&[
        (
            "src/a.ts",
            "import { b } from './b';\nexport const a = b;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
    ]);
    p.build();
    let file = p.cache_file();
    let bytes = fs::read(&file).unwrap();
    let specifier = bytes.windows(3).rposition(|w| w == b"./b").unwrap();
    let mut tampered = bytes.clone();
    tampered[specifier + 2] = b'z';
    let truncated = &bytes[..bytes.len() / 2];

    for corrupt in [tampered.as_slice(), truncated] {
        fs::write(&file, corrupt).unwrap();
        let (graph, stats) = p.build();
        assert_eq!(stats.state, CacheState::Cold);
        assert!(same(&graph, &p.cold()));
    }
}

#[test]
fn adding_x_ts_next_to_x_index_ts_re_resolves_dot_slash_x() {
    let p = Project::new(&[
        ("src/a.ts", "import './x';\n"),
        ("src/x/index.ts", "export {};\n"),
    ]);
    assert_eq!(
        targets_of(&p.build().0, "src/a.ts"),
        ["./x -> src/x/index.ts"]
    );

    p.write("src/x.ts", "export {};\n");

    let (graph, stats) = p.build();
    assert_eq!(stats.state, CacheState::Warm);
    assert_eq!(targets_of(&graph, "src/a.ts"), ["./x -> src/x.ts"]);
}

#[test]
fn a_tsconfig_paths_change_re_resolves_aliases() {
    let tsconfig = |to: &str| {
        format!(
            r#"{{ "compilerOptions": {{ "baseUrl": ".", "paths": {{ "@/*": ["{to}/*"] }} }} }}"#
        )
    };
    let p = Project::new(&[
        ("tsconfig.json", &tsconfig("src/one")),
        ("src/a.ts", "import '@/b';\n"),
        ("src/one/b.ts", "export {};\n"),
        ("src/two/b.ts", "export {};\n"),
    ]);
    assert_eq!(
        targets_of(&p.build().0, "src/a.ts"),
        ["@/b -> src/one/b.ts"]
    );

    p.write("tsconfig.json", &tsconfig("src/two"));

    let (graph, stats) = p.build();
    assert_eq!(stats.state, CacheState::Warm);
    assert_eq!(targets_of(&graph, "src/a.ts"), ["@/b -> src/two/b.ts"]);
}
