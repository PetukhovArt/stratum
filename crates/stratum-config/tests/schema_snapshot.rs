#[test]
#[allow(clippy::unwrap_used)]
fn schema_is_stable() {
    let schema = stratum_config::schema().unwrap();
    insta::assert_yaml_snapshot!("schema_v1", schema);
}
