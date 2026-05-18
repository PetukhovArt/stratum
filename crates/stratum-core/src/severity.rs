use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
    Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Off,
    Info,
    Warning,
    Error,
}

impl Severity {
    pub const fn fails_build(self) -> bool {
        matches!(self, Self::Error)
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Off => "off",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_off_lt_info_lt_warning_lt_error() {
        assert!(Severity::Off < Severity::Info);
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn only_error_fails_build() {
        assert!(!Severity::Off.fails_build());
        assert!(!Severity::Info.fails_build());
        assert!(!Severity::Warning.fails_build());
        assert!(Severity::Error.fails_build());
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Severity::Warning).unwrap(), r#""warning""#);
        let s: Severity = serde_json::from_str(r#""error""#).unwrap();
        assert_eq!(s, Severity::Error);
    }
}
