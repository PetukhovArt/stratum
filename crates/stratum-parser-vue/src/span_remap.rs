use stratum_parser_ts::SourceSpan;

/// Translate a span produced against an in-script body to one in the outer SFC.
#[must_use]
pub fn shift(span: SourceSpan, block_start: usize) -> SourceSpan {
    let delta = u32::try_from(block_start).unwrap_or(u32::MAX);
    SourceSpan {
        start: span.start.saturating_add(delta),
        end: span.end.saturating_add(delta),
    }
}
