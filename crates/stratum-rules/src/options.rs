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
#[allow(clippy::struct_field_names)]
pub struct ContainerBreadthOptions {
    #[serde(default = "default_max_kinds")]
    pub max_kinds: u32,
    #[serde(default = "default_max_items")]
    pub max_items: u32,
    #[serde(default = "default_max_root_files")]
    pub max_root_files: u32,
}

const fn default_max_kinds() -> u32 {
    6
}

const fn default_max_items() -> u32 {
    20
}

const fn default_max_root_files() -> u32 {
    4
}

impl Default for ContainerBreadthOptions {
    fn default() -> Self {
        Self {
            max_kinds: default_max_kinds(),
            max_items: default_max_items(),
            max_root_files: default_max_root_files(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneBreadthOptions {
    #[serde(default = "default_max_kinds")]
    pub max_kinds: u32,
}

impl Default for ZoneBreadthOptions {
    fn default() -> Self {
        Self {
            max_kinds: default_max_kinds(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoCircularDepsOptions {
    #[serde(default)]
    pub ignore_type_only: bool,
    #[serde(default)]
    pub ignore_dynamic: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoBarrelOptions {
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposerScopeOptions {
    #[serde(default, deserialize_with = "glob_or_none")]
    pub scope_file: Option<String>,
}

fn glob_or_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let glob = Option::<String>::deserialize(d)?;
    if let Some(pattern) = &glob {
        globset::Glob::new(pattern).map_err(serde::de::Error::custom)?;
    }
    Ok(glob)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FolderCase {
    Kebab,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamingOptions {
    #[serde(default)]
    pub folder_case: Option<FolderCase>,
    #[serde(default)]
    pub reserved: Vec<String>,
    #[serde(default)]
    pub duplicate_slices: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDocsOptions {
    #[serde(default)]
    pub nav_file: Option<String>,
    #[serde(default = "default_requires_key")]
    pub requires_key: String,
}

fn default_requires_key() -> String {
    "requires".into()
}

impl Default for ModuleDocsOptions {
    fn default() -> Self {
        Self {
            nav_file: None,
            requires_key: default_requires_key(),
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
    20
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
    20
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
    1.0
}

impl Default for DepthRatioOptions {
    fn default() -> Self {
        Self {
            min_ratio: default_min_ratio(),
        }
    }
}
