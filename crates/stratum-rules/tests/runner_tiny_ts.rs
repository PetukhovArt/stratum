#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use camino::Utf8PathBuf;

use stratum_core::{ids::LayerId, stage::Stage, types::Layer, visibility::VisibilityScope};
use stratum_graph::{BuildConfig, GraphBuilder};

#[test]
fn runner_emits_expected_violations() {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-ts-violations")
        .canonicalize_utf8()
        .unwrap();
    let config_path = root.join("stratum.config.jsonc");
    let config = stratum_config::parse_file(&config_path).unwrap();

    let layers: Vec<Layer> = config
        .layers
        .iter()
        .enumerate()
        .map(|(i, l)| Layer {
            id: LayerId::new(u32::try_from(i).unwrap()),
            name: l.id.clone(),
            path: PathBuf::from(l.path.to_string()),
            depends_on: l
                .depends_on
                .iter()
                .map(|dep_id| {
                    let pos = config
                        .layers
                        .iter()
                        .position(|x| &x.id == dep_id)
                        .unwrap_or(0);
                    LayerId::new(u32::try_from(pos).unwrap())
                })
                .collect(),
        })
        .collect();

    let cfg = BuildConfig {
        project_root: root.clone(),
        layers,
        default_stage: Stage::new(2).unwrap(),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(cfg).build().unwrap();
    let violations = stratum_rules::run_all(&graph, &config, &root).unwrap();

    let by_rule: std::collections::BTreeMap<u32, usize> =
        violations.iter().fold(Default::default(), |mut acc, v| {
            *acc.entry(v.rule.raw()).or_insert(0) += 1;
            acc
        });
    assert_eq!(
        by_rule.get(&stratum_rules::NO_CIRCULAR_DEPS.raw()).copied(),
        Some(1),
        "expected exactly one no-circular-deps violation; got {by_rule:?}"
    );
    assert_eq!(
        by_rule
            .get(&stratum_rules::NO_CROSS_LAYER_IMPORT.raw())
            .copied(),
        Some(1),
        "expected exactly one no-cross-layer-import violation; got {by_rule:?}"
    );

    let xlayer = violations
        .iter()
        .find(|v| v.rule == stratum_rules::NO_CROSS_LAYER_IMPORT)
        .unwrap();
    assert_eq!(
        xlayer.severity,
        stratum_core::severity::Severity::Warning,
        "override under src/shared/api/** must demote no-cross-layer-import to warning"
    );

    insta::assert_yaml_snapshot!("tiny_ts_violations_summary", by_rule);
}
