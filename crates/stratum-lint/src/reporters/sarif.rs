use std::collections::BTreeSet;

use serde_json::json;
use stratum_core::violation::Violation;

use super::Reporter;

#[derive(Debug, Default)]
pub struct SarifReporter;

impl Reporter for SarifReporter {
    fn write(
        &self,
        violations: &[Violation],
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let rule_ids: BTreeSet<u32> = violations.iter().map(|v| v.rule.raw()).collect();
        let rules: Vec<_> = rule_ids
            .iter()
            .map(|id| {
                let slug = stratum_rules::slug_for(stratum_core::ids::RuleId::new(*id))
                    .unwrap_or("unknown");
                json!({
                    "id": slug,
                    "name": slug,
                    "shortDescription": { "text": slug },
                    "defaultConfiguration": { "level": "warning" }
                })
            })
            .collect();
        let results: Vec<_> = violations
            .iter()
            .map(|v| {
                json!({
                    "ruleId": stratum_rules::slug_for(v.rule).unwrap_or("unknown"),
                    "level": sarif_level(v.severity),
                    "message": { "text": v.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": v.file.to_string_lossy() },
                            "region": { "startLine": v.location.line, "startColumn": v.location.column }
                        }
                    }]
                })
            })
            .collect();
        let doc = json!({
            "version": "2.1.0",
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "stratum-lint",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/PetukhovArt/stratum",
                        "rules": rules,
                    }
                },
                "results": results
            }]
        });
        let pretty = serde_json::to_string_pretty(&doc).map_err(std::io::Error::other)?;
        out.write_all(pretty.as_bytes())?;
        writeln!(out)?;
        Ok(())
    }
}

fn sarif_level(s: stratum_core::severity::Severity) -> &'static str {
    use stratum_core::severity::Severity;
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
        Severity::Off => "none",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use stratum_core::{
        ids::{ModuleId, RuleId},
        severity::Severity,
        violation::SourceLocation,
    };

    #[test]
    fn emits_sarif_envelope() {
        let v = Violation {
            rule: RuleId::new(1),
            severity: Severity::Error,
            message: "x".into(),
            file: PathBuf::from("a.ts"),
            location: SourceLocation { line: 1, column: 1 },
            modules: vec![ModuleId::new(0)],
            edge: None,
            suggestion: None,
        };
        let mut buf = Vec::new();
        SarifReporter.write(&[v], &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"version\": \"2.1.0\""));
        assert!(s.contains("\"ruleId\": \"stratum/no-cross-layer-import\""));
        assert!(s.contains("\"level\": \"error\""));
    }
}
