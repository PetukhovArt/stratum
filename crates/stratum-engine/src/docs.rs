use std::collections::BTreeMap;
use std::path::PathBuf;

use camino::Utf8Path;
use stratum_config::Config;
use stratum_graph::{CompoundGraph, NavDoc};
use stratum_rules::ModuleDocsOptions;

const SLUG: &str = "stratum/module-docs";

/// The navigation docs of the folders in `graph.dir_roles`. Empty while `stratum/module-docs`
/// has no `nav_file`. Each `requires` entry is a path relative to `root` naming a folder or one
/// of its docs; either way it is stored as the folder.
pub(crate) fn read(
    graph: &CompoundGraph,
    root: &Utf8Path,
    config: &Config,
) -> BTreeMap<PathBuf, NavDoc> {
    let options = config
        .rules
        .get(SLUG)
        .filter(|rule| !rule.options.is_null())
        .and_then(|rule| serde_json::from_value::<ModuleDocsOptions>(rule.options.clone()).ok())
        .unwrap_or_default();
    let Some(nav_file) = options.nav_file else {
        return BTreeMap::new();
    };
    graph
        .dir_roles
        .keys()
        .filter_map(|dir| {
            let text = std::fs::read_to_string(dir.join(&nav_file)).ok()?;
            let requires = requires_of(&text, &options.requires_key)
                .iter()
                .map(|entry| folder_of(root, entry, &nav_file))
                .collect();
            Some((dir.clone(), NavDoc { requires }))
        })
        .collect()
}

fn folder_of(root: &Utf8Path, entry: &str, nav_file: &str) -> PathBuf {
    let path = root.join(entry.trim_end_matches('/')).into_std_path_buf();
    let is_doc = path.file_name().is_some_and(|name| name == nav_file)
        || path.extension().is_some_and(|ext| ext == "md");
    match path.parent() {
        Some(folder) if is_doc => folder.to_path_buf(),
        _ => path,
    }
}

fn requires_of(text: &str, key: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Vec::new();
    }
    let front: Vec<&str> = lines.take_while(|l| l.trim_end() != "---").collect();
    let prefix = format!("{key}:");
    let Some(at) = front.iter().position(|l| l.starts_with(&prefix)) else {
        return Vec::new();
    };
    let inline = front[at][prefix.len()..].trim();
    let values: Vec<&str> = if inline.is_empty() {
        front[at + 1..]
            .iter()
            .map_while(|l| l.trim_start().strip_prefix('-'))
            .collect()
    } else {
        inline
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .collect()
    };
    values
        .into_iter()
        .map(|v| v.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|v| !v.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_inline_and_scalar_requires_are_all_read() {
        let block = "---\ntitle: x\nrequires:\n  - src/a\n  - \"src/b/_index.md\"\n---\nbody\n- not a require\n";
        let inline = "---\nrequires: [src/a, 'src/b/_index.md']\n---\n";
        let scalar = "---\r\nrequires: src/a\r\n---\r\n";
        let expect = |text, want: &[&str]| assert_eq!(requires_of(text, "requires"), want);
        expect(block, &["src/a", "src/b/_index.md"]);
        expect(inline, &["src/a", "src/b/_index.md"]);
        expect(scalar, &["src/a"]);
        expect("requires: [src/a]\n", &[]);
        assert_eq!(requires_of(inline, "needs"), Vec::<String>::new());
    }

    #[test]
    fn a_requires_entry_naming_a_doc_means_its_folder() {
        let root = Utf8Path::new("/p");
        let folder = |entry| folder_of(root, entry, "_index.md");
        assert_eq!(folder("src/b/_index.md"), PathBuf::from("/p/src/b"));
        assert_eq!(folder("docs/glossary.md"), PathBuf::from("/p/docs"));
        assert_eq!(folder("src/b/"), PathBuf::from("/p/src/b"));
    }
}
