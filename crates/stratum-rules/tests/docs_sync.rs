#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use stratum_rules::RuleRegistry;

fn doc_file(relative: &str) -> String {
    let path = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path).unwrap()
}

fn rows(markdown: &str) -> BTreeMap<String, Vec<String>> {
    markdown
        .lines()
        .filter(|line| line.starts_with("| `stratum/"))
        .map(|line| {
            let mut cells = line.trim_matches('|').split('|').map(str::trim);
            let slug = cells.next().unwrap().trim_matches('`').to_owned();
            (slug, cells.map(str::to_owned).collect())
        })
        .collect()
}

fn preset_severities(preset: &str) -> BTreeMap<String, String> {
    let text = format!(r#"{{ "version": 1, "project": {{ "extends": "{preset}" }} }}"#);
    let config = stratum_config::parse_str(Utf8Path::new("inline"), &text).unwrap();
    config
        .rules
        .into_iter()
        .map(|(slug, rule)| (slug, rule.severity.to_string()))
        .collect()
}

#[test]
fn rule_tables_in_the_docs_match_the_registered_rules_and_their_explain_docs() {
    let registry = RuleRegistry::with_builtins();
    let registered: BTreeSet<String> = registry
        .rules()
        .iter()
        .map(|r| r.slug().to_owned())
        .collect();
    let recommended = preset_severities("stratum:recommended");
    let fsd = preset_severities("stratum:fsd");

    let readme = rows(&doc_file("README.md"));
    assert_eq!(
        readme.keys().cloned().collect::<BTreeSet<_>>(),
        registered,
        "README.md rule table"
    );

    let table = rows(&doc_file("docs/custom-rules.md"));
    assert_eq!(
        table.keys().cloned().collect::<BTreeSet<_>>(),
        registered,
        "docs/custom-rules.md severity table"
    );
    for rule in registry.rules() {
        let slug = rule.slug();
        let cells = &table[slug];
        let severity =
            |preset: &BTreeMap<String, String>| preset.get(slug).cloned().unwrap_or_default();
        assert_eq!(
            cells[0],
            rule.default_severity().to_string(),
            "{slug}: default severity"
        );
        assert_eq!(
            cells[1],
            severity(&recommended),
            "{slug}: recommended severity"
        );
        assert_eq!(cells[2], severity(&fsd), "{slug}: fsd severity");

        let options = rule.doc().unwrap().options;
        let cell = &cells[3];
        if options.is_empty() {
            assert_eq!(cell, "none", "{slug}: options");
        } else if options[0].starts_with("options:") {
            assert!(cell.starts_with("list of policies"), "{slug}: options");
        } else {
            for line in &options {
                let (name, _) = line.split_once(':').unwrap();
                let default = line.rsplit_once('(').unwrap().1.trim_end_matches(')');
                let expected = format!("`{name}` ({default})");
                assert!(
                    cell.contains(&expected),
                    "{slug}: options cell {cell:?} lacks {expected}"
                );
            }
        }
    }
}
