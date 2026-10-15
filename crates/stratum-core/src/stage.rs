use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "u8", into = "u8")]
pub struct Stage(u8);

#[derive(Debug, thiserror::Error)]
#[error("Stage must be in 1..=4, got {0}")]
pub struct InvalidStage(pub u8);

impl Stage {
    pub const FILE: Self = Self(1);
    pub const FLAT: Self = Self(2);
    pub const SEGMENTED: Self = Self(3);
    pub const COMPOSED: Self = Self(4);

    /// # Errors
    /// Returns [`InvalidStage`] if `rank` is not in `1..=4`.
    pub const fn new(rank: u8) -> Result<Self, InvalidStage> {
        match rank {
            1..=4 => Ok(Self(rank)),
            other => Err(InvalidStage(other)),
        }
    }

    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Stage {
    type Error = InvalidStage;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Stage> for u8 {
    fn from(s: Stage) -> Self {
        s.0
    }
}
