use stratum_core::{severity::Severity, violation::Violation};
use stratum_rules::slug_for;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

#[must_use]
pub fn convert(v: &Violation) -> Diagnostic {
    let line = v.location.line.saturating_sub(1);
    let col = v.location.column.saturating_sub(1);
    Diagnostic {
        range: Range {
            start: Position {
                line,
                character: col,
            },
            end: Position {
                line,
                character: col,
            },
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use stratum_core::ids::RuleId;
    use stratum_core::violation::SourceLocation;
    use stratum_rules::ids::NO_CIRCULAR_DEPS;

    use super::*;

    fn fixture(rule: RuleId, line: u32, col: u32) -> Violation {
        Violation {
            rule,
            severity: Severity::Error,
            message: "cycle".into(),
            file: PathBuf::from("x.ts"),
            location: SourceLocation { line, column: col },
            modules: Vec::new(),
            edge: None,
            suggestion: None,
        }
    }

    #[test]
    fn converts_zero_based_positions() {
        let d = convert(&fixture(NO_CIRCULAR_DEPS, 12, 5));
        assert_eq!(d.range.start.line, 11);
        assert_eq!(d.range.start.character, 4);
    }

    #[test]
    fn slug_carried_through_code_field() {
        let d = convert(&fixture(NO_CIRCULAR_DEPS, 1, 1));
        assert_eq!(
            d.code,
            Some(NumberOrString::String("stratum/no-circular-deps".into()))
        );
    }

    #[test]
    fn source_is_stratum_lint() {
        let d = convert(&fixture(NO_CIRCULAR_DEPS, 1, 1));
        assert_eq!(d.source.as_deref(), Some("stratum-lint"));
    }
}
