use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CrossEntityPatternOptions {
    #[serde(default = "default_max_fanout")]
    pub max_fanout: u32,
}

const fn default_max_fanout() -> u32 {
    5
}

impl Default for CrossEntityPatternOptions {
    fn default() -> Self {
        Self {
            max_fanout: default_max_fanout(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DeepModuleOptions {
    #[serde(default = "default_max_exports")]
    pub max_exports: u32,
}

const fn default_max_exports() -> u32 {
    12
}

impl Default for DeepModuleOptions {
    fn default() -> Self {
        Self {
            max_exports: default_max_exports(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PromotionPressureOptions {
    #[serde(default = "default_threshold")]
    pub threshold: u32,
}

const fn default_threshold() -> u32 {
    5
}

impl Default for PromotionPressureOptions {
    fn default() -> Self {
        Self {
            threshold: default_threshold(),
        }
    }
}

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
