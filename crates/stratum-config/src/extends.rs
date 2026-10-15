use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

use crate::{ConfigError, parse::read_jsonc, presets};

pub fn resolve(path: &Utf8Path, document: Value) -> Result<Value, ConfigError> {
    let mut chain = vec![chain_key(path)];
    resolve_chain(path, document, &mut chain)
}

fn resolve_chain(
    path: &Utf8Path,
    child: Value,
    chain: &mut Vec<String>,
) -> Result<Value, ConfigError> {
    let Some(extends) = child
        .pointer("/project/extends")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(child);
    };
    let (parent_path, parent) = if extends.starts_with(presets::PREFIX) {
        let parent = presets::preset(&extends).ok_or_else(|| ConfigError::UnknownPreset {
            path: path.to_path_buf(),
            name: extends.clone(),
            presets: presets::NAMES.join(", "),
        })?;
        (Utf8PathBuf::from(&extends), parent)
    } else {
        let parent_path = path.parent().unwrap_or(Utf8Path::new("")).join(&extends);
        let parent = read_jsonc(&parent_path)?;
        (parent_path, parent)
    };
    let key = chain_key(&parent_path);
    let cycle = chain.contains(&key);
    chain.push(key);
    if cycle {
        return Err(ConfigError::ExtendsCycle {
            chain: chain.join(" -> "),
        });
    }
    let parent = resolve_chain(&parent_path, parent, chain)?;
    chain.pop();
    Ok(merge(parent, child))
}

/// The config file at `path` followed by every file it pulls in through `extends`,
/// nearest parent first; built-in presets are not files and are left out.
///
/// # Errors
/// [`ConfigError::Io`] / [`ConfigError::Parse`] for a file in the chain that cannot be read,
/// [`ConfigError::ExtendsCycle`] when the chain loops.
pub fn config_files(path: &Utf8Path) -> Result<Vec<Utf8PathBuf>, ConfigError> {
    let mut files = vec![path.to_path_buf()];
    loop {
        let current = &files[files.len() - 1];
        let document = read_jsonc(current)?;
        let Some(extends) = document.pointer("/project/extends").and_then(Value::as_str) else {
            return Ok(files);
        };
        if extends.starts_with(presets::PREFIX) {
            return Ok(files);
        }
        let next = current.parent().unwrap_or(Utf8Path::new("")).join(extends);
        let key = chain_key(&next);
        let cycle = files.iter().any(|f| chain_key(f) == key);
        files.push(next);
        if cycle {
            return Err(ConfigError::ExtendsCycle {
                chain: files
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> "),
            });
        }
    }
}

fn chain_key(path: &Utf8Path) -> String {
    path.canonicalize_utf8()
        .map_or_else(|_| path.to_string(), |p| p.to_string())
}

fn merge(parent: Value, child: Value) -> Value {
    let (Value::Object(mut base), Value::Object(over)) = (parent, child.clone()) else {
        return child;
    };
    for (key, value) in over {
        let inherited = base.remove(&key);
        let merged = match key.as_str() {
            "layers" => merge_layers(inherited, value),
            "rules" => merge_rules(inherited, value),
            "overrides" => append(inherited, value),
            "project" => merge_fields(inherited, value),
            _ => value,
        };
        base.insert(key, merged);
    }
    Value::Object(base)
}

fn merge_fields(parent: Option<Value>, child: Value) -> Value {
    match (parent, child) {
        (Some(Value::Object(mut base)), Value::Object(over)) => {
            base.extend(over);
            Value::Object(base)
        }
        (_, child) => child,
    }
}

fn merge_rules(parent: Option<Value>, child: Value) -> Value {
    match (parent, child) {
        (Some(Value::Object(mut base)), Value::Object(over)) => {
            for (slug, rule) in over {
                let merged = merge_fields(base.remove(&slug), rule);
                base.insert(slug, merged);
            }
            Value::Object(base)
        }
        (_, child) => child,
    }
}

fn merge_layers(parent: Option<Value>, child: Value) -> Value {
    match (parent, child) {
        (Some(Value::Array(mut base)), Value::Array(over)) => {
            for layer in over {
                let id = layer.get("id").cloned();
                match base
                    .iter_mut()
                    .find(|l| id.is_some() && l.get("id") == id.as_ref())
                {
                    Some(slot) => *slot = merge_fields(Some(slot.take()), layer),
                    None => base.push(layer),
                }
            }
            Value::Array(base)
        }
        (_, child) => child,
    }
}

fn append(parent: Option<Value>, child: Value) -> Value {
    match (parent, child) {
        (Some(Value::Array(mut base)), Value::Array(over)) => {
            base.extend(over);
            Value::Array(base)
        }
        (_, child) => child,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn child_overrides_layer_fields_by_id_and_keeps_the_rest() {
        let parent = json!({ "layers": [
            { "id": "app", "path": "src/app", "depends_on": ["shared"] },
            { "id": "shared", "path": "src/shared" },
        ]});
        let child = json!({ "layers": [
            { "id": "app", "path": "src/renderer/app" },
            { "id": "core", "path": "src/core" },
        ]});
        assert_eq!(
            merge(parent, child)["layers"],
            json!([
                { "id": "app", "path": "src/renderer/app", "depends_on": ["shared"] },
                { "id": "shared", "path": "src/shared" },
                { "id": "core", "path": "src/core" },
            ])
        );
    }

    #[test]
    fn config_files_lists_the_file_chain_without_presets() {
        let dir = tempfile::tempdir().unwrap();
        let dir = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::create_dir(dir.join("base")).unwrap();
        let child = dir.join("stratum.config.jsonc");
        std::fs::write(
            &child,
            r#"{ "project": { "extends": "./base/team.jsonc" } }"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("base/team.jsonc"),
            r#"{ "project": { "extends": "stratum:fsd" } }"#,
        )
        .unwrap();
        assert_eq!(
            config_files(&child).unwrap(),
            vec![child.clone(), dir.join("base/team.jsonc")]
        );
    }

    #[test]
    fn child_rule_severity_keeps_parent_options_and_overrides_append() {
        let parent = json!({
            "rules": { "r": { "severity": "error", "options": { "n": 1 } } },
            "overrides": [{ "files": ["a/**"] }],
        });
        let child = json!({
            "rules": { "r": { "severity": "off" } },
            "overrides": [{ "files": ["b/**"] }],
        });
        let merged = merge(parent, child);
        assert_eq!(
            merged["rules"]["r"],
            json!({ "severity": "off", "options": { "n": 1 } })
        );
        assert_eq!(
            merged["overrides"],
            json!([{ "files": ["a/**"] }, { "files": ["b/**"] }])
        );
    }
}
