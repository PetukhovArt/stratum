use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stratum purity rank from 1 (most pure) to 4 (impure side-effectful code).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
    Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "u8", into = "u8")]
pub struct Stage(u8);

#[derive(Debug, thiserror::Error)]
#[error("Stage must be in 1..=4, got {0}")]
pub struct InvalidStage(pub u8);

impl Stage {
    /// # Errors
    /// Returns [`InvalidStage`] if `rank` is not in `1..=4`.
    pub const fn new(rank: u8) -> Result<Self, InvalidStage> {
        match rank {
            1..=4 => Ok(Self(rank)),
            other => Err(InvalidStage(other)),
        }
    }

    #[must_use]
    pub const fn rank(self) -> u8 { self.0 }

    /// A module at stage `self` may depend on modules at stage `other`
    /// iff `other <= self` — purer code cannot reach impurer code.
    #[must_use]
    pub const fn may_depend_on(self, other: Stage) -> bool {
        other.0 <= self.0
    }
}

impl TryFrom<u8> for Stage {
    type Error = InvalidStage;
    fn try_from(value: u8) -> Result<Self, Self::Error> { Self::new(value) }
}

impl From<Stage> for u8 {
    fn from(s: Stage) -> Self { s.0 }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range() {
        assert!(Stage::new(0).is_err());
        assert!(Stage::new(5).is_err());
        assert!(Stage::new(1).is_ok());
        assert!(Stage::new(4).is_ok());
    }

    #[test]
    fn purer_cannot_depend_on_impurer() {
        let s1 = Stage::new(1).unwrap();
        let s4 = Stage::new(4).unwrap();
        assert!(!s1.may_depend_on(s4));
        assert!(s4.may_depend_on(s1));
        assert!(s4.may_depend_on(s4));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Stage::new(3).unwrap();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, "3");
        let back: Stage = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);

        let bad: Result<Stage, _> = serde_json::from_str("7");
        assert!(bad.is_err());
    }
}
