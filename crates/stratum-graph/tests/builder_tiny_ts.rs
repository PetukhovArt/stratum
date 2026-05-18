use std::path::PathBuf;

use camino::Utf8PathBuf;

use stratum_core::{ids::LayerId, stage::Stage, types::Layer, visibility::VisibilityScope};
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
        },
        Layer {
            id: LayerId::new(1),
            name: "features".into(),
            path: PathBuf::from("src/features"),
            depends_on: vec![LayerId::new(2), LayerId::new(3)],
        },
        Layer {
            id: LayerId::new(2),
            name: "entities".into(),
            path: PathBuf::from("src/entities"),
            depends_on: vec![LayerId::new(3)],
        },
        Layer {
            id: LayerId::new(3),
            name: "shared".into(),
            path: PathBuf::from("src/shared"),
            depends_on: vec![],
        },
    ];
    let cfg = BuildConfig {
        project_root: root.clone(),
        layers,
        default_stage: Stage::new(2).unwrap(),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(cfg).build().unwrap();
    let mut snap = snapshot_of(&graph);

    // Strip absolute, machine-specific prefix from module paths and normalize
    // Windows separators so the snapshot is portable.
    let prefix_with_ext = root.as_str().to_string();
    let prefix_no_ext = prefix_with_ext
        .strip_prefix(r"\\?\")
        .map(str::to_string)
        .unwrap_or_else(|| prefix_with_ext.clone());
    for m in &mut snap.modules {
        for p in [&prefix_with_ext, &prefix_no_ext] {
            if let Some(stripped) = m.path.strip_prefix(p) {
                m.path = stripped.trim_start_matches(['/', '\\']).to_string();
                break;
            }
        }
        m.path = m.path.replace('\\', "/");
    }

    insta::assert_yaml_snapshot!("tiny_ts_graph", snap);
}
