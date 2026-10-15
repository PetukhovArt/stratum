use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stratum purity rank from 1 (most pure) to 4 (impure side-effectful code).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "u8", into = "u8")]
pub struct Purity(u8);

#[derive(Debug, thiserror::Error)]
#[error("Purity must be in 1..=4, got {0}")]
pub struct InvalidPurity(pub u8);

impl Purity {
    /// # Errors
    /// Returns [`InvalidPurity`] if `rank` is not in `1..=4`.
    pub const fn new(rank: u8) -> Result<Self, InvalidPurity> {
        match rank {
            1..=4 => Ok(Self(rank)),
            other => Err(InvalidPurity(other)),
        }
    }

    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0
    }

    /// A module at purity `self` may depend on modules at purity `other`
    /// iff `other <= self` — purer code cannot reach impurer code.
    #[must_use]
    pub const fn may_depend_on(self, other: Purity) -> bool {
        other.0 <= self.0
    }
}

impl TryFrom<u8> for Purity {
    type Error = InvalidPurity;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Purity> for u8 {
    fn from(s: Purity) -> Self {
        s.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range() {
        assert!(Purity::new(0).is_err());
        assert!(Purity::new(5).is_err());
        assert!(Purity::new(1).is_ok());
        assert!(Purity::new(4).is_ok());
    }

    #[test]
    fn purer_cannot_depend_on_impurer() {
        let s1 = Purity::new(1).unwrap();
        let s4 = Purity::new(4).unwrap();
        assert!(!s1.may_depend_on(s4));
        assert!(s4.may_depend_on(s1));
        assert!(s4.may_depend_on(s4));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Purity::new(3).unwrap();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, "3");
        let back: Purity = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);

        let bad: Result<Purity, _> = serde_json::from_str("7");
        assert!(bad.is_err());
    }
}
