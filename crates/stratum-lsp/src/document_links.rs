use tower_lsp::lsp_types::{Diagnostic, DocumentLink, NumberOrString, Url};

#[must_use]
pub fn links_from(diags: &[Diagnostic]) -> Vec<DocumentLink> {
    diags
        .iter()
        .filter_map(|d| {
            let Some(NumberOrString::String(slug)) = &d.code else {
                return None;
            };
            let target = Url::parse(&format!("https://stratum.dev/docs/rules/{slug}")).ok()?;
            Some(DocumentLink {
                range: d.range,
                target: Some(target),
                tooltip: Some(format!("Stratum: {slug}")),
                data: None,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use tower_lsp::lsp_types::{Position, Range};

    use super::*;

    fn make(slug: &str) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            code: Some(NumberOrString::String(slug.into())),
            ..Default::default()
        }
    }

    #[test]
    fn maps_slug_to_docs_url() {
        let links = links_from(&[make("stratum/no-circular-deps")]);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].target.as_ref().unwrap().as_str(),
            "https://stratum.dev/docs/rules/stratum/no-circular-deps"
        );
    }

    #[test]
    fn drops_diagnostics_without_string_code() {
        let mut d = make("ignored");
        d.code = None;
        assert!(links_from(&[d]).is_empty());
    }
}
