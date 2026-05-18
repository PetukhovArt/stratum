use camino::Utf8PathBuf;

#[test]
#[allow(clippy::unwrap_used)]
fn example_config_snapshot() {
    let path =
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example.config.jsonc");
    let cfg = stratum_config::parse_file(&path).unwrap();
    insta::assert_yaml_snapshot!("example_config", cfg);
}
