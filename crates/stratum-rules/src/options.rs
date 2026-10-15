use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceFanInOptions {
    #[serde(default = "default_max_fan_in")]
    pub max_fan_in: u32,
}

const fn default_max_fan_in() -> u32 {
    5
}

impl Default for SliceFanInOptions {
    fn default() -> Self {
        Self {
            max_fan_in: default_max_fan_in(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerBreadthOptions {
    #[serde(default = "default_max_kinds")]
    pub max_kinds: u32,
}

const fn default_max_kinds() -> u32 {
    6
}

impl Default for ContainerBreadthOptions {
    fn default() -> Self {
        Self {
            max_kinds: default_max_kinds(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageMixingOptions {
    #[serde(default)]
    pub segments: Vec<String>,
}
