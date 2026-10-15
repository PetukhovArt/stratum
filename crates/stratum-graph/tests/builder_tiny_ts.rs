use std::path::PathBuf;

use camino::Utf8PathBuf;

use stratum_core::{ids::LayerId, purity::Purity, types::Layer};
use stratum_graph::{BuildConfig, GraphBuilder, snapshot_of};

#[test]
#[allow(clippy::unwrap_used)]
fn tiny_ts_graph_snapshot() {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts")
        .canonicalize_utf8()
        .unwrap();
    let layers = vec![
        Layer {
            id: LayerId::new(0),
            name: "app".into(),
            path: PathBuf::from("src/app"),
            depends_on: vec![LayerId::new(1), LayerId::new(2), LayerId::new(3)],
            slices: false,
        },
        Layer {
            id: LayerId::new(1),
            name: "features".into(),
            path: PathBuf::from("src/features"),
            depends_on: vec![LayerId::new(2), LayerId::new(3)],
            slices: false,
        },
        Layer {
            id: LayerId::new(2),
            name: "entities".into(),
            path: PathBuf::from("src/entities"),
            depends_on: vec![LayerId::new(3)],
            slices: false,
        },
        Layer {
            id: LayerId::new(3),
            name: "shared".into(),
            path: PathBuf::from("src/shared"),
            depends_on: vec![],
            slices: false,
        },
    ];
    let cfg = BuildConfig {
        project_root: root.clone(),
        layers,
        default_purity: Purity::new(2).unwrap(),
        structure: stratum_core::structure::Structure::default(),
        tracked_calls: std::collections::HashSet::new(),
    };
    let graph = GraphBuilder::new(cfg).build().unwrap();
    let mut snap = snapshot_of(&graph);

    // Strip absolute, machine-specific prefix from module paths and normalize
    // Windows separators so the snapshot is portable.
    let prefix_with_ext = root.as_str().to_string();
    let prefix_no_ext = prefix_with_ext
        .strip_prefix(r"\\?\")
        .map_or_else(|| prefix_with_ext.clone(), str::to_string);
    let portable = |path: &str| {
        [&prefix_with_ext, &prefix_no_ext]
            .into_iter()
            .find_map(|p| path.strip_prefix(p.as_str()))
            .map_or(path, |s| s.trim_start_matches(['/', '\\']))
            .replace('\\', "/")
    };
    for m in &mut snap.modules {
        m.path = portable(&m.path);
    }
    for c in &mut snap.containers {
        c.path = portable(&c.path);
    }

    insta::assert_yaml_snapshot!("tiny_ts_graph", snap);
}
