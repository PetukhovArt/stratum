use camino::{Utf8Path, Utf8PathBuf};
use stratum_config::{Config, ConfigError};

pub use crate::zero_config::infer;

pub const CONFIG_FILE: &str = "stratum.config.jsonc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    File(Utf8PathBuf),
    Inferred,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub source: ConfigSource,
}

impl LoadedConfig {
    #[must_use]
    pub fn is_inferred(&self) -> bool {
        self.source == ConfigSource::Inferred
    }
}

pub fn load(root: &Utf8Path, explicit: Option<&Utf8Path>) -> Result<LoadedConfig, ConfigError> {
    let path = explicit
        .map(Utf8Path::to_path_buf)
        .or_else(|| Some(root.join(CONFIG_FILE)).filter(|p| p.is_file()));
    Ok(match path {
        Some(path) => LoadedConfig {
            config: stratum_config::parse_file(&path)?,
            source: ConfigSource::File(path),
        },
        None => LoadedConfig {
            config: infer(root),
            source: ConfigSource::Inferred,
        },
    })
}

#[must_use]
pub fn watched_files(path: &Utf8Path) -> Vec<Utf8PathBuf> {
    stratum_config::config_files(path).unwrap_or_else(|_| vec![path.to_path_buf()])
}
