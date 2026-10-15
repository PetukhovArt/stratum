#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RuleDoc {
    pub summary: String,
    pub why: String,
    pub fix: String,
    /// One `name: meaning (default)` line per option; empty when the rule takes none.
    pub options: Vec<String>,
}

#[must_use]
pub fn builtin(_slug: &str) -> Option<RuleDoc> {
    None
}
