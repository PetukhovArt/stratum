use std::collections::BTreeSet;

use serde_json::json;
use stratum_core::severity::Severity;

use super::Reporter;
use crate::report::{Finding, Report};

#[derive(Debug, Default)]
pub struct SarifReporter;

impl Reporter for SarifReporter {
    fn write(&self, report: &Report, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let all = || {
            let open = report.findings.iter().map(|f| (f, None));
            let hidden = report
                .suppressed
                .iter()
                .map(|s| (&s.finding, Some(s.reason.as_str())));
            open.chain(hidden)
        };
        let slugs: BTreeSet<&str> = all().map(|(f, _)| f.rule.as_str()).collect();
        let rules: Vec<_> = slugs
            .iter()
            .map(|slug| {
                json!({
                    "id": slug,
                    "name": slug,
                    "shortDescription": { "text": slug },
                    "defaultConfiguration": { "level": "warning" }
                })
            })
            .collect();
        let results: Vec<_> = all().map(|(f, reason)| result(f, reason)).collect();
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

fn result(f: &Finding, suppression_reason: Option<&str>) -> serde_json::Value {
    let mut result = json!({
        "ruleId": f.rule,
        "level": sarif_level(f.severity),
        "message": { "text": f.message },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": { "uri": f.file, "uriBaseId": "%SRCROOT%" },
                "region": { "startLine": f.location.line, "startColumn": f.location.column }
            }
        }]
    });
    if let Some(justification) = suppression_reason {
        result["suppressions"] = json!([{ "kind": "inSource", "justification": justification }]);
    }
    result
}

const fn sarif_level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
        Severity::Off => "none",
    }
}
