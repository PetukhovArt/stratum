use camino::Utf8PathBuf;

#[test]
#[allow(clippy::unwrap_used)]
fn committed_schema_matches_config_types() {
    let path = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stratum.schema.json");
    let generated = stratum_config::schema().unwrap();
    if std::env::var_os("STRATUM_UPDATE_SCHEMA").is_some() {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&generated).unwrap() + "\n",
        )
        .unwrap();
    }
    let committed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_default())
            .unwrap_or_default();
    assert!(
        committed == generated,
        "{path} is stale: rerun with STRATUM_UPDATE_SCHEMA=1"
    );
}
