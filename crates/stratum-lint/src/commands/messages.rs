use std::fmt::Write as _;

use stratum_core::text::{Key, placeholders};
use stratum_rules::RuleRegistry;

use crate::cli::BriefFormat;

pub fn run(format: BriefFormat) -> miette::Result<i32> {
    let registry = RuleRegistry::with_builtins();
    let slugs: Vec<&str> = registry.rules().iter().map(|r| r.slug()).collect();
    match format {
        BriefFormat::Agent => print!("{}", text(&slugs)),
        BriefFormat::Json => {
            let s =
                serde_json::to_string_pretty(&json(&slugs)).map_err(|e| miette::miette!("{e}"))?;
            println!("{s}");
        }
    }
    Ok(0)
}

fn text(slugs: &[&str]) -> String {
    let mut out = String::from(
        "Translate in stratum.config.jsonc: \"messages\": { \"titles\": { \"<rule>\": \"...\" }, \"texts\": { \"<key>\": \"...\" } }; a text keeps the placeholders of its template\n\nrules (messages.titles):\n",
    );
    for slug in slugs {
        let _ = writeln!(out, "  {slug}");
    }
    out.push_str("\ntexts (messages.texts):\n");
    for &key in Key::ALL {
        let _ = writeln!(out, "  {}: {:?}", key.name(), key.template());
    }
    out
}

fn json(slugs: &[&str]) -> serde_json::Value {
    let texts: Vec<serde_json::Value> = Key::ALL
        .iter()
        .map(|key| {
            serde_json::json!({
                "key": key.name(),
                "template": key.template(),
                "placeholders": placeholders(key.template()),
            })
        })
        .collect();
    serde_json::json!({ "version": 1, "rules": slugs, "texts": texts })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn json_lists_every_key_with_its_placeholders_and_every_rule() {
        let value = json(&["stratum/purity"]);
        let texts = value["texts"].as_array().unwrap();
        assert_eq!(texts.len(), Key::ALL.len());
        let cycle = texts.iter().find(|t| t["key"] == "Cycle").unwrap();
        assert_eq!(
            cycle["placeholders"],
            serde_json::json!(["closing", "count", "name"])
        );
        assert_eq!(value["rules"], serde_json::json!(["stratum/purity"]));
    }

    #[test]
    fn text_names_every_key() {
        let out = text(&["stratum/purity"]);
        assert!(out.contains("  stratum/purity\n"));
        assert!(
            Key::ALL
                .iter()
                .all(|k| out.contains(&format!("  {}: ", k.name())))
        );
    }
}
