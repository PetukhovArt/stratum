use std::fmt::Write;

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

#[must_use]
pub fn hover_for(slug: &str, message: &str, suggestion: Option<&str>) -> Hover {
    let mut md = format!("**`{slug}`**\n\n{message}\n");
    if let Some(s) = suggestion {
        let _ = writeln!(md, "\n_Suggestion:_ {s}");
    }
    let _ = writeln!(md, "\n[docs](https://stratum.dev/docs/rules/{slug})");
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md,
        }),
        range: None,
    }
}
