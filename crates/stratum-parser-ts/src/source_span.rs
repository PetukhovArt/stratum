use serde::{Deserialize, Serialize};

/// Byte-offset range inside a source file. End-exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: u32,
    pub end: u32,
}

impl SourceSpan {
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }
}

/// Convert a byte offset into a 1-based (line, column).
/// `source` must be the same content the byte offset was produced against.
#[must_use]
pub fn offset_to_line_col(source: &str, offset: u32) -> (u32, u32) {
    let mut line: u32 = 1;
    let mut col: u32 = 1;
    let offset_usize = offset as usize;
    for (i, ch) in source.char_indices() {
        if i >= offset_usize {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[derive(Debug)]
pub struct LineIndex<'s> {
    source: &'s str,
    line_starts: Vec<usize>,
}

impl<'s> LineIndex<'s> {
    #[must_use]
    pub fn new(source: &'s str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            source,
            line_starts,
        }
    }

    #[must_use]
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let offset = offset as usize;
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line];
        let col = self.source[start..]
            .char_indices()
            .take_while(|(i, ch)| start + i < offset && *ch != '\n')
            .count();
        (
            u32::try_from(line + 1).unwrap_or(u32::MAX),
            u32::try_from(col + 1).unwrap_or(u32::MAX),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn offset_at_start_is_1_1() {
        assert_eq!(offset_to_line_col("foo", 0), (1, 1));
    }

    #[test]
    fn offset_after_newline_advances_line() {
        let src = "a\nbc\nd";
        assert_eq!(offset_to_line_col(src, 0), (1, 1));
        assert_eq!(offset_to_line_col(src, 1), (1, 2));
        assert_eq!(offset_to_line_col(src, 2), (2, 1));
        assert_eq!(offset_to_line_col(src, 3), (2, 2));
        assert_eq!(offset_to_line_col(src, 4), (2, 3));
        assert_eq!(offset_to_line_col(src, 5), (3, 1));
    }

    #[test]
    fn empty_source_returns_1_1() {
        assert_eq!(offset_to_line_col("", 0), (1, 1));
        assert_eq!(offset_to_line_col("", 99), (1, 1));
    }
}
