use std::collections::BTreeSet;

/// Hands out element ids: `prefix_` and `raw` with every character outside `[A-Za-z0-9]` made
/// `_`, then `_2`, `_3`… for a sanitized id already given out.
#[derive(Debug, Default)]
pub struct Ids {
    given: BTreeSet<String>,
}

impl Ids {
    pub fn take(&mut self, prefix: &str, raw: &str) -> String {
        let base: String = format!("{prefix}_{raw}")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let mut id = base.clone();
        let mut n = 2;
        while !self.given.insert(id.clone()) {
            id = format!("{base}_{n}");
            n += 1;
        }
        id
    }
}

/// `label` inside `"…"` of any of the three formats: none of them escapes a quote reliably, so
/// `"` becomes `'`, a `\` becomes `/` so it cannot escape the closing quote, and each line
/// break becomes `newline`.
pub fn quoted(label: &str, newline: &str) -> String {
    let flat = label
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('"', "'")
        .replace('\\', "/");
    format!("\"{}\"", flat.replace('\n', newline))
}

/// `text` on one line outside quotes, each line break and each of `forbidden` made a space.
pub fn bare(text: &str, forbidden: &[char]) -> String {
    text.chars()
        .map(|c| {
            if c == '\n' || c == '\r' || forbidden.contains(&c) {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// `1 import`, `2 imports`.
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_keep_only_ascii_letters_digits_and_underscores_and_stay_unique() {
        let mut ids = Ids::default();
        let taken: Vec<String> = [
            ("ctr", "src/features/check-out"),
            ("ctr", "src/features/check_out"),
            ("ctr", "src/features/check.out"),
            ("cmp", "src/shared/(root)"),
            ("sys", "Web UI ✓"),
        ]
        .into_iter()
        .map(|(prefix, raw)| ids.take(prefix, raw))
        .collect();
        assert_eq!(
            taken,
            [
                "ctr_src_features_check_out",
                "ctr_src_features_check_out_2",
                "ctr_src_features_check_out_3",
                "cmp_src_shared__root_",
                "sys_Web_UI__",
            ]
        );
    }

    #[test]
    fn quoted_labels_hold_no_quote_backslash_or_raw_line_break() {
        assert_eq!(
            quoted("say \"hi\"\r\nto C:\\dir\nnow", "\\n"),
            r#""say 'hi'\nto C:/dir\nnow""#
        );
        assert_eq!(quoted("a\rb", " "), r#""a b""#);
    }

    #[test]
    fn a_bare_title_holds_no_line_break_and_none_of_the_forbidden_characters() {
        assert_eq!(bare("a; b #c\nd", &[';', '#']), "a  b  c d");
        assert_eq!(bare("a; b #c\r\nd", &[]), "a; b #c  d");
    }
}
