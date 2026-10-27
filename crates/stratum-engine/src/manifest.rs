//! `package.json` at the project root, for `stratum/unused-dependencies`.

use std::collections::BTreeMap;

use camino::Utf8Path;
use serde_json::Value;
use stratum_core::violation::SourceLocation;
use stratum_graph::{Declared, Manifest};

const FIELDS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
];

/// `None` when ROOT has no readable, valid `package.json`.
pub(crate) fn read(root: &Utf8Path) -> Option<Manifest> {
    let path = root.join("package.json");
    let text = std::fs::read_to_string(path.as_std_path()).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let mut declared = Vec::new();
    for field in FIELDS {
        let Some(packages) = json.get(field).and_then(Value::as_object) else {
            continue;
        };
        for package in packages.keys() {
            declared.push(Declared {
                field: field.to_owned(),
                package: package.clone(),
                location: line_of(&text, field, package),
            });
        }
    }
    declared.sort_by_key(|d| d.location.line);
    let scripts = json
        .get("scripts")
        .and_then(Value::as_object)
        .map(|scripts| {
            scripts
                .values()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut bins = BTreeMap::new();
    for d in &declared {
        if let Some(names) = bins_of(root, &d.package) {
            bins.insert(d.package.clone(), names);
        }
    }
    Some(Manifest {
        path: path.into_std_path_buf(),
        name: json.get("name").and_then(Value::as_str).map(str::to_owned),
        declared,
        scripts,
        bins,
    })
}

/// The line of `"package"` as a key after the line of `"field"`; line 1 when not found.
fn line_of(text: &str, field: &str, package: &str) -> SourceLocation {
    let field_key = format!("\"{field}\"");
    let package_key = format!("\"{package}\"");
    let mut in_field = false;
    for (i, line) in text.lines().enumerate() {
        if line.contains(&field_key) {
            in_field = true;
        }
        if in_field
            && let Some(column) = line.find(&package_key)
            && line[column + package_key.len()..]
                .trim_start()
                .starts_with(':')
        {
            return SourceLocation {
                line: u32::try_from(i + 1).unwrap_or(u32::MAX),
                column: u32::try_from(column + 1).unwrap_or(u32::MAX),
            };
        }
    }
    SourceLocation { line: 1, column: 1 }
}

/// The binaries an installed package declares in its `bin`; `None` when it is not installed.
fn bins_of(root: &Utf8Path, package: &str) -> Option<Vec<String>> {
    let path = package
        .split('/')
        .fold(root.join("node_modules"), |p, segment| p.join(segment))
        .join("package.json");
    let json: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let own = package.rsplit('/').next().unwrap_or(package).to_owned();
    Some(match json.get("bin") {
        Some(Value::String(_)) => vec![own],
        Some(Value::Object(bins)) => bins.keys().cloned().collect(),
        _ => Vec::new(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_package_is_found_on_its_own_line_under_its_field() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(
            root.join("package.json"),
            "{\n  \"name\": \"app\",\n  \"scripts\": { \"lint\": \"eslint .\" },\n  \"dependencies\": {\n    \"solid-js\": \"1\"\n  },\n  \"devDependencies\": {\n    \"eslint\": \"9\",\n    \"solid-js\": \"1\"\n  }\n}\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("node_modules/eslint")).unwrap();
        std::fs::write(
            root.join("node_modules/eslint/package.json"),
            r#"{ "bin": { "eslint": "./bin/eslint.js" } }"#,
        )
        .unwrap();
        let m = read(root).unwrap();
        let found: Vec<(&str, &str, u32)> = m
            .declared
            .iter()
            .map(|d| (d.field.as_str(), d.package.as_str(), d.location.line))
            .collect();
        assert_eq!(
            found,
            [
                ("dependencies", "solid-js", 5),
                ("devDependencies", "eslint", 8),
                ("devDependencies", "solid-js", 9),
            ]
        );
        assert_eq!(m.name.as_deref(), Some("app"));
        assert_eq!(m.scripts, ["eslint ."]);
        assert_eq!(m.bins["eslint"], ["eslint"]);
        assert!(!m.bins.contains_key("solid-js"));
    }
}
