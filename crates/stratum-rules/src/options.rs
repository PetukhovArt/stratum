use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MillerLimitOptions {
    #[serde(default = "default_max_children")]
    pub max_children: u32,
}

const fn default_max_children() -> u32 {
    7
}

impl Default for MillerLimitOptions {
    fn default() -> Self {
        Self {
            max_children: default_max_children(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DepthRatioOptions {
    #[serde(default = "default_min_ratio")]
    pub min_ratio: f64,
}

const fn default_min_ratio() -> f64 {
    3.0
}

impl Default for DepthRatioOptions {
    fn default() -> Self {
        Self {
            min_ratio: default_min_ratio(),
        }
    }
}
