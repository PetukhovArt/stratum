#![allow(clippy::unwrap_used)]

use std::fs;

use camino::Utf8PathBuf;
use stratum_parser_ts::{LanguageExtractor, OxcTsExtractor};
use walkdir::WalkDir;

#[test]
fn snapshot_tiny_ts_extraction() {
    let fixture =
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny-ts");
    let extractor = OxcTsExtractor::new();
    let mut all = Vec::new();

    let mut entries: Vec<_> = WalkDir::new(fixture.as_std_path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();
    entries.sort_by_key(|e| e.path().to_path_buf());

    for entry in entries {
        let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf()).unwrap();
        if !extractor.handles(path.as_path()) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let data = extractor.extract(path.as_path(), &source).unwrap();

        let rel = path
            .strip_prefix(&fixture)
            .unwrap()
            .as_str()
            .replace('\\', "/");
        all.push((rel, data.imports));
    }

    insta::assert_yaml_snapshot!("tiny_ts_imports", all);
}
