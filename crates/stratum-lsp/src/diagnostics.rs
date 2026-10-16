use std::collections::HashMap;
use std::path::{Path, PathBuf};

use stratum_core::severity::Severity;
use stratum_engine::Finding;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range,
};

use crate::paths::{path_key, path_to_uri};

pub const SOURCE: &str = "stratum";
const MAX_RELATED: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct FileDiagnostics {
    pub path: PathBuf,
    pub items: Vec<Diagnostic>,
}

pub type DiagnosticsByFile = HashMap<String, FileDiagnostics>;

#[must_use]
pub fn group(root: &Path, findings: &[Finding]) -> DiagnosticsByFile {
    let mut out = DiagnosticsByFile::new();
    for f in findings {
        let file = root.join(&f.file);
        let base = Diagnostic {
            range: point(f.location.line, f.location.column),
            severity: Some(severity_to_lsp(f.severity)),
            code: Some(NumberOrString::String(f.rule.clone())),
            source: Some(SOURCE.into()),
            message: format!("{}\n{}", f.message, f.fix),
            ..Default::default()
        };
        if let Some(edge) = &f.edge {
            let related = related(&root.join(&edge.to), "imported module").map(|r| vec![r]);
            push(
                &mut out,
                file,
                Diagnostic {
                    related_information: related,
                    ..base
                },
            );
        } else if f.members.len() > 1 {
            let members: Vec<PathBuf> = f.members.iter().map(|m| root.join(m)).collect();
            for (i, member) in members.iter().enumerate() {
                let others = members
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .filter_map(|(_, p)| related(p, "also involved"))
                    .take(MAX_RELATED)
                    .collect();
                let range = if path_key(member) == path_key(&file) {
                    base.range
                } else {
                    point(1, 1)
                };
                push(
                    &mut out,
                    member.clone(),
                    Diagnostic {
                        range,
                        related_information: Some(others),
                        ..base.clone()
                    },
                );
            }
        } else {
            push(&mut out, file, base);
        }
    }
    out
}

fn push(out: &mut DiagnosticsByFile, path: PathBuf, d: Diagnostic) {
    out.entry(path_key(&path))
        .or_insert_with(|| FileDiagnostics {
            path,
            items: Vec::new(),
        })
        .items
        .push(d);
}

fn related(path: &Path, message: &str) -> Option<DiagnosticRelatedInformation> {
    Some(DiagnosticRelatedInformation {
        location: Location {
            uri: path_to_uri(path)?,
            range: point(1, 1),
        },
        message: message.into(),
    })
}

fn point(line: u32, column: u32) -> Range {
    let p = Position {
        line: line.saturating_sub(1),
        character: column.saturating_sub(1),
    };
    Range { start: p, end: p }
}

fn severity_to_lsp(s: Severity) -> DiagnosticSeverity {
    match s {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Off => DiagnosticSeverity::HINT,
    }
}
