//! Violation text: a [`Key`] of the catalog plus named arguments, rendered with the built-in
//! templates or a project's [`Messages`]. The built-in rendering is the canonical text the
//! baseline and the GitLab fingerprint are keyed on; [`Messages`] are display only.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use crate::catalog::Key;

/// A project's own texts for the output of `lint`: the config block `messages`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Messages {
    /// Keyed by rule slug: printed as `<title>. ` before every message of the rule.
    #[serde(default)]
    pub titles: BTreeMap<String, String>,
    /// Keyed by catalog key name (`stratum-lint messages` lists them): overrides the built-in
    /// template, with the same `{placeholders}`.
    #[serde(default)]
    pub texts: BTreeMap<String, String>,
}

impl Messages {
    #[must_use]
    pub fn template(&self, key: Key) -> &str {
        self.texts
            .get(key.name())
            .map_or_else(|| key.template(), String::as_str)
    }

    #[must_use]
    pub fn title(&self, rule: &str) -> Option<&str> {
        self.titles.get(rule).map(String::as_str)
    }

    /// # Errors
    /// The first text whose key is not in the catalog or whose placeholders differ from the
    /// built-in template's, named in the message.
    pub fn check_texts(&self) -> Result<(), String> {
        for (name, text) in &self.texts {
            let Some(key) = Key::from_name(name) else {
                return Err(format!(
                    "messages.texts: unknown key {name:?}; `stratum-lint messages` lists the keys"
                ));
            };
            let (want, got) = (placeholders(key.template()), placeholders(text));
            if want != got {
                return Err(format!(
                    "messages.texts.{name}: placeholders {} differ from the built-in template's {}",
                    braced(&got),
                    braced(&want)
                ));
            }
        }
        Ok(())
    }
}

/// The `{name}` placeholders of a template.
#[must_use]
pub fn placeholders(template: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        out.insert(&after[..close]);
        rest = &after[close + 1..];
    }
    out
}

fn braced(names: &BTreeSet<&str>) -> String {
    if names.is_empty() {
        return "(none)".to_owned();
    }
    names
        .iter()
        .map(|n| format!("{{{n}}}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A catalog template with its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    key: Key,
    args: Vec<(&'static str, Arg)>,
}

/// The value of a `{name}` placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// Inserted as is whatever the messages: paths, names, numbers, user-written text.
    Plain(String),
    Text(Text),
    /// The items rendered and joined by the separator template.
    Join(Vec<Arg>, Key),
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Self::Plain(s)
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Self::Plain(s.to_owned())
    }
}

impl From<&String> for Arg {
    fn from(s: &String) -> Self {
        Self::Plain(s.clone())
    }
}

impl From<Text> for Arg {
    fn from(t: Text) -> Self {
        Self::Text(t)
    }
}

impl From<Key> for Arg {
    fn from(key: Key) -> Self {
        Self::Text(Text::new(key))
    }
}

macro_rules! plain_from_display {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(v: $t) -> Self {
                Self::Plain(v.to_string())
            }
        }
    )*};
}

plain_from_display!(u8, u32, usize);

impl Text {
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            args: Vec::new(),
        }
    }

    /// User-written text, never replaced by `messages.texts`.
    #[must_use]
    pub fn verbatim(text: impl Into<String>) -> Self {
        Self::new(Key::Verbatim).arg("text", text.into())
    }

    #[must_use]
    pub fn arg(mut self, name: &'static str, value: impl Into<Arg>) -> Self {
        self.args.push((name, value.into()));
        self
    }

    #[must_use]
    pub const fn key(&self) -> Key {
        self.key
    }

    /// The canonical text: what the baseline and fingerprints are keyed on.
    #[must_use]
    pub fn en(&self) -> String {
        self.render(&Messages::default())
    }

    /// The text with the templates of `messages`, built-in where it has none.
    #[must_use]
    pub fn render(&self, messages: &Messages) -> String {
        let template = messages.template(self.key);
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let value = after.find('}').and_then(|close| {
                let name = &after[..close];
                let arg = self.args.iter().find(|(n, _)| *n == name)?;
                Some((&arg.1, close))
            });
            if let Some((arg, close)) = value {
                arg.render_into(messages, &mut out);
                rest = &after[close + 1..];
            } else {
                out.push('{');
                rest = after;
            }
        }
        out.push_str(rest);
        out
    }

    /// Applies `f` to every plain argument, nested ones included.
    #[must_use]
    pub fn map_plain(self, f: &dyn Fn(&str) -> String) -> Self {
        Self {
            key: self.key,
            args: self
                .args
                .into_iter()
                .map(|(name, arg)| (name, arg.map_plain(f)))
                .collect(),
        }
    }
}

impl Arg {
    fn render_into(&self, messages: &Messages, out: &mut String) {
        match self {
            Self::Plain(s) => out.push_str(s),
            Self::Text(t) => out.push_str(&t.render(messages)),
            Self::Join(items, separator) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(messages.template(*separator));
                    }
                    item.render_into(messages, out);
                }
            }
        }
    }

    fn map_plain(self, f: &dyn Fn(&str) -> String) -> Self {
        match self {
            Self::Plain(s) => Self::Plain(f(&s)),
            Self::Text(t) => Self::Text(t.map_plain(f)),
            Self::Join(items, separator) => Self::Join(
                items.into_iter().map(|a| a.map_plain(f)).collect(),
                separator,
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn texts(pairs: &[(&str, &str)]) -> Messages {
        Messages {
            titles: BTreeMap::new(),
            texts: pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    fn suppression_invalid() -> Text {
        Text::new(Key::SuppressionInvalid).arg("names", "x").arg(
            "problems",
            Arg::Join(
                vec![Key::ProblemNoRule.into(), Key::ProblemNoReason.into()],
                Key::ListAnd,
            ),
        )
    }

    #[test]
    fn key_names_round_trip() {
        for &key in Key::ALL {
            assert_eq!(Key::from_name(key.name()), Some(key));
        }
        assert_eq!(Key::from_name("NoSuchKey"), None);
    }

    #[test]
    fn renders_plain_nested_and_joined_arguments() {
        assert_eq!(
            suppression_invalid().en(),
            "Suppression `x` suppresses nothing: it names no rule and it has no reason after `--`."
        );
    }

    #[test]
    fn messages_replace_templates_at_every_depth_and_leave_the_rest_built_in() {
        let messages = texts(&[
            (
                "SuppressionInvalid",
                "Directive `{names}` disables nothing: {problems}",
            ),
            ("ListAnd", " plus "),
            ("ProblemNoRule", "no rule named"),
        ]);
        assert_eq!(
            suppression_invalid().render(&messages),
            "Directive `x` disables nothing: no rule named plus it has no reason after `--`"
        );
    }

    #[test]
    fn check_texts_names_an_unknown_key_and_a_placeholder_mismatch() {
        assert!(texts(&[("ListAnd", " plus ")]).check_texts().is_ok());
        let unknown = texts(&[("NoSuchKey", "x")]).check_texts().unwrap_err();
        assert!(unknown.contains("\"NoSuchKey\""), "{unknown}");
        let mismatch = texts(&[("UnusedExport", "`{nme}` is unused")])
            .check_texts()
            .unwrap_err();
        assert!(
            mismatch.contains("messages.texts.UnusedExport"),
            "{mismatch}"
        );
        assert!(
            mismatch.contains("{nme}") && mismatch.contains("{name}"),
            "{mismatch}"
        );
    }

    #[test]
    fn a_missing_argument_leaves_the_placeholder() {
        assert_eq!(Text::new(Key::Verbatim).en(), "{text}");
    }

    #[test]
    fn map_plain_reaches_nested_arguments() {
        let text = Text::new(Key::CycleFix)
            .arg(
                "cuts",
                Arg::Join(
                    vec![
                        Text::new(Key::CycleCut)
                            .arg("from", "/r/a.ts")
                            .arg("at", "")
                            .arg("to", "/r/b.ts")
                            .arg("freed", 2u32)
                            .arg("size", 2u32)
                            .into(),
                    ],
                    Key::ListComma,
                ),
            )
            .map_plain(&|s| s.replace("/r/", ""));
        assert!(text.en().contains("`a.ts -> b.ts`"));
    }
}
