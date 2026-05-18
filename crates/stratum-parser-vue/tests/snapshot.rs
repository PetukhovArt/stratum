#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use camino::Utf8PathBuf;

use stratum_parser_ts::LanguageExtractor;
use stratum_parser_vue::VueExtractor;

fn fixture_dir() -> PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny-vue/src")
        .canonicalize_utf8()
        .unwrap()
        .into_std_path_buf()
}

#[test]
fn extracts_imports_from_all_tiny_vue_fixtures() {
    let dir = fixture_dir();
    let ex = VueExtractor::new();

    let mut summaries: Vec<(String, usize)> = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vue") {
            continue;
        }
        let upath = camino::Utf8PathBuf::from_path_buf(path.clone()).unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let data = ex.extract(&upath, &src).unwrap();
        summaries.push((upath.file_name().unwrap().to_string(), data.imports.len()));
    }
    summaries.sort();

    insta::assert_yaml_snapshot!("tiny_vue_import_counts", summaries);
}
