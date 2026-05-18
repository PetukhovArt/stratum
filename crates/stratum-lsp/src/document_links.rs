use tower_lsp::lsp_types::{Diagnostic, DocumentLink, NumberOrString, Url};

#[must_use]
pub fn links_from(diags: &[Diagnostic]) -> Vec<DocumentLink> {
    diags
        .iter()
        .filter_map(|d| {
            let Some(NumberOrString::String(slug)) = &d.code else {
                return None;
            };
            let target =
                Url::parse(&format!("https://stratum.dev/docs/rules/{slug}")).ok()?;
            Some(DocumentLink {
                range: d.range,
                target: Some(target),
                tooltip: Some(format!("Stratum: {slug}")),
                data: None,
            })
        })
        .collect()
}
