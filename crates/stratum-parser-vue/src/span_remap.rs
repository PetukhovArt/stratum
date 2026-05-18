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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_span_by_block_start() {
        let s = SourceSpan { start: 4, end: 8 };
        let out = shift(s, 100);
        assert_eq!(out.start, 104);
        assert_eq!(out.end, 108);
    }

    #[test]
    fn zero_shift_is_identity() {
        let s = SourceSpan { start: 7, end: 11 };
        assert_eq!(shift(s, 0), s);
    }
}
