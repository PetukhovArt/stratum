use super::Declared;
use super::scan::{Scan, Token};

/// Each `fn` whose attributes include `#[attr]` or `#[attr(…)]`, `attr` written in full or as
/// its last path segment, is a contract named after the function.
pub(super) fn contracts(text: &str, attr: &str) -> Vec<Declared> {
    let short = attr.rsplit("::").next().unwrap_or(attr);
    let scan = Scan::new(text);
    let tokens = scan.tokens(&[':']);
    let mut found = Vec::new();
    let mut marked = false;
    let mut item_start = None;
    let mut i = 0;
    while let Some(token) = tokens.get(i) {
        if token.is("#") && tokens.get(i + 1).is_some_and(|t| t.is("[")) {
            let path = tokens.get(i + 2).map_or("", |t| t.text);
            marked |= path == attr || path == short;
            i = closing_bracket(&tokens, i + 1);
            item_start = None;
            continue;
        }
        let start = *item_start.get_or_insert(token.start);
        match token.text {
            "fn" => {
                if marked {
                    if let Some(declared) = function(&scan, &tokens[i..], start) {
                        found.push(declared);
                    }
                }
                marked = false;
                item_start = None;
            }
            "pub" | "async" | "unsafe" | "const" | "extern" | "(" | ")" | "crate" | "super"
            | "self" | "in" | "\"" => {}
            _ => {
                marked = false;
                item_start = None;
            }
        }
        i += 1;
    }
    found
}

fn closing_bracket(tokens: &[Token<'_>], open: usize) -> usize {
    let mut depth = 0usize;
    for (at, token) in tokens.iter().enumerate().skip(open) {
        match token.text {
            "[" => depth += 1,
            "]" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// `tokens` start at `fn`; the signature runs from `start` to the body's `{` or a `;`.
fn function(scan: &Scan, tokens: &[Token<'_>], start: usize) -> Option<Declared> {
    let name = tokens.get(1)?;
    let end = tokens.iter().find(|t| t.is("{") || t.is(";"))?;
    Some(Declared {
        id: name.text.to_owned(),
        signature: scan.squeezed(start, end.start),
        line: scan.line(tokens.first()?.start),
    })
}

#[cfg(test)]
mod tests {
    use super::contracts;

    #[test]
    fn functions_under_the_attribute_in_full_or_by_its_last_segment_with_their_signature() {
        let text = r#"use tauri::command;

/// Says hello.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("{name} {")
}

#[command(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) async fn save_file(
    path: String,
    data: Vec<u8>,
) -> Result<(), String> {
    Ok(())
}

#[derive(Debug)]
struct NotACommand;

fn helper() -> char { '{' }

#[test]
fn checks() {}
"#;
        let found: Vec<(String, String, u32)> = contracts(text, "tauri::command")
            .into_iter()
            .map(|c| (c.id, c.signature, c.line))
            .collect();
        assert_eq!(
            found,
            [
                (
                    "greet".into(),
                    "pub fn greet(name: &str) -> String".into(),
                    5
                ),
                (
                    "save_file".into(),
                    "pub(crate) async fn save_file( path: String, data: Vec<u8>, ) -> Result<(), String>"
                        .into(),
                    11
                ),
            ]
        );
    }
}
