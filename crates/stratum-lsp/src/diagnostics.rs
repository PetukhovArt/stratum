use stratum_core::{severity::Severity, violation::Violation};
use stratum_rules::slug_for;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

#[must_use]
pub fn convert(v: &Violation) -> Diagnostic {
    let line = v.location.line.saturating_sub(1);
    let col = v.location.column.saturating_sub(1);
    Diagnostic {
        range: Range {
            start: Position { line, character: col },
            end: Position { line, character: col },
        },
        severity: Some(severity_to_lsp(v.severity)),
        code: Some(NumberOrString::String(
            slug_for(v.rule).unwrap_or("unknown").to_string(),
        )),
        source: Some("stratum-lint".into()),
        message: v.message.clone(),
        ..Default::default()
    }
}

fn severity_to_lsp(s: Severity) -> DiagnosticSeverity {
    match s {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Off => DiagnosticSeverity::HINT,
    }
}
