#![allow(clippy::unwrap_used)]
//! Regression gate against the curated `exemplar-stratum-app` fixture.
//!
//! Asserts a per-rule violation-count summary. The absolute file paths embedded
//! in violations would change between machines, so we don't compare the raw
//! JSON byte-for-byte; instead we compare the rule-count signature and let
//! `insta` track the per-rule histogram.

use std::collections::BTreeMap;
use std::path::PathBuf;

use camino::Utf8PathBuf;

use stratum_core::{ids::LayerId, purity::Purity, types::Layer, visibility::VisibilityScope};
use stratum_graph::{BuildConfig, GraphBuilder};

#[test]
fn exemplar_violation_histogram_is_stable() {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/exemplar-stratum-app")
        .canonicalize_utf8()
        .unwrap();
    let config = stratum_config::parse_file(&root.join("stratum.config.jsonc")).unwrap();

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
                .map(|d| {
                    let pos = config.layers.iter().position(|x| &x.id == d).unwrap_or(0);
                    LayerId::new(u32::try_from(pos).unwrap())
                })
                .collect(),
        })
        .collect();
    let cfg = BuildConfig {
        project_root: root.clone(),
        layers,
        default_purity: Purity::new(2).unwrap(),
        default_visibility: VisibilityScope::Public,
    };
    let graph = GraphBuilder::new(cfg).build().unwrap();
    let violations = stratum_rules::run_all(&graph, &config, &root).unwrap();

    let by_rule: BTreeMap<u32, usize> = violations.iter().fold(BTreeMap::new(), |mut acc, v| {
        *acc.entry(v.rule.raw()).or_insert(0) += 1;
        acc
    });

    assert!(
        by_rule
            .get(&stratum_rules::NO_CIRCULAR_DEPS.raw())
            .copied()
            .unwrap_or(0)
            >= 1,
        "expected the deliberate cycle to fire no-circular-deps; got {by_rule:?}"
    );
    assert!(
        by_rule
            .get(&stratum_rules::NO_CROSS_LAYER_IMPORT.raw())
            .copied()
            .unwrap_or(0)
            >= 1,
        "expected the deliberate upward import to fire no-cross-layer-import; got {by_rule:?}"
    );
    assert!(
        by_rule
            .get(&stratum_rules::PROMOTION_PRESSURE.raw())
            .copied()
            .unwrap_or(0)
            >= 1,
        "expected the shared util to fire promotion-pressure; got {by_rule:?}"
    );

    insta::assert_yaml_snapshot!("exemplar_violation_histogram", by_rule);
}
