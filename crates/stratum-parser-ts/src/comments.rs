use oxc_ast::Trivias;

use crate::extractor::RawSuppression;
use crate::source_span::SourceSpan;

const DIRECTIVE: &str = "stratum-ignore";

pub(crate) fn suppressions(source: &str, trivias: &Trivias) -> Vec<RawSuppression> {
    trivias
        .comments()
        .filter_map(|comment| {
            let (slugs, reason) = directive(comment.span.source_text(source))?;
            Some(RawSuppression {
                span: SourceSpan::new(comment.real_span_start(), comment.real_span_end()),
                slugs,
                reason,
            })
        })
        .collect()
}

fn directive(text: &str) -> Option<(Vec<String>, Option<String>)> {
    let rest = text
        .trim_start_matches(|c: char| c == '*' || c.is_whitespace())
        .strip_prefix(DIRECTIVE)?;
    if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    let (slugs, reason) = rest
        .split_once("--")
        .map_or((rest, None), |(s, r)| (s, Some(r)));
    let slugs = slugs
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let reason = reason
        .map(|r| r.trim_matches(|c: char| c == '*' || c.is_whitespace()))
        .filter(|r| !r.is_empty())
        .map(str::to_owned);
    Some((slugs, reason))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use oxc_span::SourceType;

    use crate::extractor::TrackedCalls;
    use crate::imports::collect;

    fn directives(source: &str) -> Vec<(String, Vec<String>, Option<String>)> {
        let ts = SourceType::default()
            .with_typescript(true)
            .with_module(true);
        collect(source, ts, &TrackedCalls::default())
            .suppressions
            .into_iter()
            .map(|s| {
                let text = source[s.span.start as usize..s.span.end as usize].to_owned();
                (text, s.slugs, s.reason)
            })
            .collect()
    }

    #[test]
    fn only_a_comment_starting_with_the_directive_word_is_a_directive() {
        let found = directives(
            "// stratum-ignore stratum/public-api, custom/x -- legacy bridge\nimport a from './a';\n/* stratum-ignore stratum/purity */\n// stratum-ignored nothing\n// see stratum-ignore docs\n",
        );
        assert_eq!(
            found,
            [
                (
                    "// stratum-ignore stratum/public-api, custom/x -- legacy bridge".to_owned(),
                    vec!["stratum/public-api".to_owned(), "custom/x".to_owned()],
                    Some("legacy bridge".to_owned()),
                ),
                (
                    "/* stratum-ignore stratum/purity */".to_owned(),
                    vec!["stratum/purity".to_owned()],
                    None,
                ),
            ]
        );
    }
}
