use std::borrow::Cow;

use serde::Serialize;
use stratum_core::{
    edge::EdgeKind,
    severity::Severity,
    text::{Messages, Text},
    violation::SourceLocation,
};

#[derive(Debug, Clone, Serialize)]
pub struct EdgeRef {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub location: SourceLocation,
    pub edge: Option<EdgeRef>,
    /// Never empty: the violation's own text, else the rule doc's fix, else a pointer to `explain`.
    pub fix: String,
    #[serde(skip)]
    pub members: Vec<String>,
    /// `message` as a catalog key and arguments, when it comes from the catalog.
    #[serde(skip)]
    pub message_text: Option<Text>,
    /// `fix` as a catalog key and arguments, when it comes from the catalog.
    #[serde(skip)]
    pub fix_text: Option<Text>,
}

impl Finding {
    /// The message to print: the project's text for it, after the rule's title when
    /// `messages` has one. `message` itself stays English: the baseline and fingerprints are
    /// keyed on it.
    #[must_use]
    pub fn message_in(&self, messages: &Messages) -> Cow<'_, str> {
        let message = shown(&self.message, self.message_text.as_ref(), messages);
        match messages.title(&self.rule) {
            Some(title) => Cow::Owned(format!("{title}. {message}")),
            None => message,
        }
    }

    #[must_use]
    pub fn fix_in(&self, messages: &Messages) -> Cow<'_, str> {
        shown(&self.fix, self.fix_text.as_ref(), messages)
    }
}

fn shown<'f>(canonical: &'f str, text: Option<&Text>, messages: &Messages) -> Cow<'f, str> {
    match text {
        Some(text) if !messages.texts.is_empty() => Cow::Owned(text.render(messages)),
        _ => Cow::Borrowed(canonical),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_core::text::Key;

    fn cycle() -> Finding {
        let text = Text::new(Key::Cycle)
            .arg("name", "a.ts")
            .arg("count", 2u32)
            .arg("closing", 1u32);
        Finding {
            rule: "stratum/no-circular-deps".into(),
            severity: Severity::Error,
            message: text.en(),
            file: "a.ts".into(),
            location: SourceLocation { line: 1, column: 1 },
            edge: None,
            fix: "fix".into(),
            members: Vec::new(),
            message_text: Some(text),
            fix_text: None,
        }
    }

    fn messages() -> Messages {
        Messages {
            titles: [(
                "stratum/no-circular-deps".to_owned(),
                "Цикл импортов".to_owned(),
            )]
            .into(),
            texts: [(
                "Cycle".to_owned(),
                "Файлы через {name} импортируют друг друга ({count}, ~{closing})".to_owned(),
            )]
            .into(),
        }
    }

    #[test]
    fn messages_print_the_title_and_the_text_and_no_messages_print_the_english_message() {
        let f = cycle();
        assert_eq!(f.message_in(&Messages::default()), f.message);
        assert_eq!(
            f.message_in(&messages()),
            "Цикл импортов. Файлы через a.ts импортируют друг друга (2, ~1)"
        );
        assert!(f.message.starts_with("Import cycle of 2 modules"));
    }

    #[test]
    fn a_title_alone_goes_before_the_english_message() {
        let f = Finding {
            message_text: None,
            ..cycle()
        };
        assert_eq!(
            f.message_in(&messages()),
            format!("Цикл импортов. {}", f.message)
        );
    }
}
