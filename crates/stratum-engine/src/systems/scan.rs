use std::iter::Peekable;
use std::str::Chars;

/// A source text with comments and string contents blanked out, keeping every byte offset and
/// line, cut into identifier tokens and single punctuation characters.
pub(super) struct Scan {
    clean: String,
    line_starts: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Token<'s> {
    pub text: &'s str,
    pub start: usize,
}

impl Token<'_> {
    pub(super) fn is(&self, text: &str) -> bool {
        self.text == text
    }

    pub(super) fn end(&self) -> usize {
        self.start + self.text.len()
    }
}

impl Scan {
    pub(super) fn new(text: &str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            clean: blank_comments_and_strings(text),
            line_starts,
        }
    }

    /// `path` are the characters besides `[A-Za-z0-9_]` that an identifier token may hold, such
    /// as `.` for `pkg.Type` or `:` for `a::b`.
    pub(super) fn tokens(&self, path: &[char]) -> Vec<Token<'_>> {
        let is_word = |c: char| c.is_alphanumeric() || c == '_' || path.contains(&c);
        let mut tokens = Vec::new();
        let mut chars = self.clean.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c.is_whitespace() {
                continue;
            }
            let mut end = start + c.len_utf8();
            if is_word(c) {
                while let Some(&(i, next)) = chars.peek() {
                    if !is_word(next) {
                        break;
                    }
                    end = i + next.len_utf8();
                    chars.next();
                }
            }
            tokens.push(Token {
                text: &self.clean[start..end],
                start,
            });
        }
        tokens
    }

    /// 1-based.
    pub(super) fn line(&self, at: usize) -> u32 {
        let line = self.line_starts.partition_point(|&start| start <= at);
        u32::try_from(line).unwrap_or(u32::MAX)
    }

    /// The text between two offsets with each run of whitespace made one space.
    pub(super) fn squeezed(&self, from: usize, to: usize) -> String {
        self.clean
            .get(from..to)
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// `//` and `/* */` comments become spaces and the inside of `"…"` strings becomes spaces;
/// newlines stay, so offsets and lines match the original text.
fn blank_comments_and_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let blank = |out: &mut String, c: char| {
        if c == '\n' {
            out.push('\n');
        } else {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    };
    let mut prev: Option<char> = None;
    while let Some(c) = chars.next() {
        let starts_raw =
            c == 'r' && prev.is_none_or(|p| p == 'b' || !(p.is_alphanumeric() || p == '_'));
        if starts_raw && let Some(hashes) = raw_string_hashes(&chars) {
            out.push(c);
            out.extend(chars.by_ref().take(hashes + 1));
            while let Some(next) = chars.next() {
                if next == '"' && closes_raw(&chars, hashes) {
                    out.push('"');
                    out.extend(chars.by_ref().take(hashes));
                    break;
                }
                blank(&mut out, next);
            }
            prev = Some('"');
            continue;
        }
        prev = Some(c);
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                blank(&mut out, c);
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    blank(&mut out, next);
                    chars.next();
                }
            }
            ('/', Some('*')) => {
                blank(&mut out, c);
                let mut last = ' ';
                for next in chars.by_ref() {
                    blank(&mut out, next);
                    if last == '*' && next == '/' {
                        break;
                    }
                    last = next;
                }
            }
            ('"', _) => {
                out.push('"');
                let mut escaped = false;
                for next in chars.by_ref() {
                    if next == '"' && !escaped {
                        out.push('"');
                        break;
                    }
                    escaped = next == '\\' && !escaped;
                    blank(&mut out, next);
                }
            }
            ('\'', Some(&next)) => {
                out.push(c);
                let mut ahead = chars.clone();
                ahead.next();
                let literal = matches!((next, ahead.next()), ('\\', _) | (_, Some('\'')));
                if literal {
                    if next == '\\' {
                        for escape in chars.by_ref().take(2) {
                            blank(&mut out, escape);
                        }
                    }
                    for next in chars.by_ref().take(8) {
                        if next == '\'' {
                            out.push('\'');
                            break;
                        }
                        blank(&mut out, next);
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// After the `r` of a raw string: the number of `#` before its opening `"`.
fn raw_string_hashes(chars: &Peekable<Chars<'_>>) -> Option<usize> {
    let mut hashes = 0;
    for c in chars.clone() {
        match c {
            '#' => hashes += 1,
            '"' => return Some(hashes),
            _ => return None,
        }
    }
    None
}

fn closes_raw(chars: &Peekable<Chars<'_>>, hashes: usize) -> bool {
    chars.clone().take(hashes).filter(|&c| c == '#').count() == hashes
}

#[cfg(test)]
mod tests {
    use super::Scan;

    #[test]
    fn a_raw_string_ending_in_a_backslash_closes_at_its_quote() {
        let scan = Scan::new("a(r\"\\\\?\\\").b(r#\"x\"# c);\nfn d");
        let words: Vec<&str> = scan
            .tokens(&[])
            .into_iter()
            .map(|t| t.text)
            .filter(|w| w.chars().all(char::is_alphanumeric))
            .collect();
        assert_eq!(words, ["a", "r", "b", "r", "c", "fn", "d"]);
    }

    #[test]
    fn comments_and_string_contents_are_blank_and_lines_are_kept() {
        let scan = Scan::new("a // b {\n/* c\n } */ d \"e { \\\" }\" f");
        let words: Vec<(&str, u32)> = scan
            .tokens(&[])
            .into_iter()
            .map(|t| (t.text, scan.line(t.start)))
            .collect();
        assert_eq!(words, [("a", 1), ("d", 3), ("\"", 3), ("\"", 3), ("f", 3)]);
    }
}
