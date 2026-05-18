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

#[cfg(test)]
mod tests {
    use super::*;

    fn markup_value(h: Hover) -> String {
        match h.contents {
            HoverContents::Markup(c) => c.value,
            _ => String::new(),
        }
    }

    #[test]
    fn renders_markdown_with_docs_link() {
        let value = markup_value(hover_for(
            "stratum/no-circular-deps",
            "cycle detected",
            None,
        ));
        assert!(value.contains("`stratum/no-circular-deps`"));
        assert!(value.contains("cycle detected"));
        assert!(value.contains("https://stratum.dev/docs/rules/stratum/no-circular-deps"));
    }

    #[test]
    fn includes_suggestion_when_present() {
        let value = markup_value(hover_for(
            "stratum/visibility-scope",
            "out of scope",
            Some("widen"),
        ));
        assert!(value.contains("_Suggestion:_ widen"));
    }
}
