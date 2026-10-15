use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibilityScope {
    Public,
    Private { root: PathBuf },
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_tag() {
        let v = VisibilityScope::Private {
            root: PathBuf::from("src/a"),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, r#"{"kind":"private","root":"src/a"}"#);
    }
}
