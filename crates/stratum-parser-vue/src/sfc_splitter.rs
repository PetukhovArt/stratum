use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBlock {
    pub setup: bool,
    pub is_typescript: bool,
    /// Byte range covering the inner content (excludes the tag bytes themselves).
    pub content_range: Range<usize>,
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SplitError {
    #[error("Malformed SFC: <script> opened at byte {0} but no matching </script> found")]
    UnclosedScript(usize),
}

/// Find every `<script>` (and `<script setup>`) block. Returns up to two blocks
/// per SFC. Does not parse the rest of the SFC.
///
/// # Errors
/// Returns `SplitError::UnclosedScript` if any `<script>` opening tag has no
/// matching `</script>`.
pub fn split(source: &str) -> Result<Vec<ScriptBlock>, SplitError> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let Some(open_rel) = find_tag_start(&bytes[i..]) else {
            break;
        };
        let open_abs = i + open_rel;
        let tag_end_rel =
            find_tag_end(&bytes[open_abs..]).ok_or(SplitError::UnclosedScript(open_abs))?;
        let tag_end_abs = open_abs + tag_end_rel;
        let attrs = &source[open_abs + "<script".len()..tag_end_abs];
        let content_start = tag_end_abs + 1;
        let close_rel =
            find_close(&source[content_start..]).ok_or(SplitError::UnclosedScript(open_abs))?;
        let content_end = content_start + close_rel;
        out.push(ScriptBlock {
            setup: contains_attr(attrs, "setup"),
            is_typescript: attrs.contains("lang=\"ts\"") || attrs.contains("lang='ts'"),
            content_range: content_start..content_end,
            body: source[content_start..content_end].to_string(),
        });
        i = content_end + "</script>".len();
    }
    Ok(out)
}

fn find_tag_start(bytes: &[u8]) -> Option<usize> {
    let needle = b"<script";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if bytes[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            let next = bytes.get(i + needle.len()).copied();
            if matches!(next, Some(b' ' | b'\t' | b'\r' | b'\n' | b'>')) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_close(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let needle = b"</script";
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if bytes[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            let next = bytes.get(i + needle.len()).copied();
            if matches!(next, Some(b'>' | b' ' | b'\t')) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_tag_end(bytes: &[u8]) -> Option<usize> {
    let mut quote = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (quote, b) {
            (None, b'"' | b'\'') => quote = Some(b),
            (Some(q), _) if b == q => quote = None,
            (None, b'>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn contains_attr(attrs: &str, name: &str) -> bool {
    for tok in attrs.split_whitespace() {
        let head = tok.split('=').next().unwrap_or(tok);
        if head.eq_ignore_ascii_case(name) {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn plain_script_block() {
        let src = "<template></template>\n<script>\nimport x from 'a';\n</script>";
        let blocks = split(src).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].setup);
        assert!(!blocks[0].is_typescript);
        assert!(blocks[0].body.contains("import x"));
    }

    #[test]
    fn script_setup_block() {
        let src = "<script setup>\nimport x from 'a';\n</script>";
        let blocks = split(src).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].setup);
    }

    #[test]
    fn script_lang_ts_block() {
        let src = "<script lang=\"ts\">\nimport x from 'a';\n</script>";
        let blocks = split(src).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].is_typescript);
    }

    #[test]
    fn generic_attribute_with_angle_brackets_keeps_the_body() {
        let src = "<script setup lang=\"ts\" generic=\"T extends Record<string, unknown>\">\nimport x from 'a';\n</script>";
        let blocks = split(src).unwrap();
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].setup);
        assert!(blocks[0].is_typescript);
        assert_eq!(blocks[0].body.trim(), "import x from 'a';");
    }

    #[test]
    fn unclosed_script_errors() {
        let src = "<script>\nimport x from 'a';\n";
        let err = split(src).unwrap_err();
        assert!(matches!(err, SplitError::UnclosedScript(_)));
    }
}
