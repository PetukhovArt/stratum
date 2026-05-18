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

    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
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
