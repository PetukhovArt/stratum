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
        tracked_calls: stratum_graph::TrackedCalls::default(),
        default_purity: Purity::new(2).unwrap(),
        structure: stratum_core::structure::Structure::default(),
    };
    let (graph, _) = GraphBuilder::new(cfg).build().unwrap();
    let snap = snapshot_of(&graph, &root);

    insta::assert_yaml_snapshot!("tiny_ts_graph", snap);
}
