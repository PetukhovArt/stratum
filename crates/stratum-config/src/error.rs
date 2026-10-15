use camino::Utf8PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSONC parse error in {path}: {message}")]
    Parse { path: Utf8PathBuf, message: String },
    #[error("Config validation error in {path}: {message}")]
    Validation { path: Utf8PathBuf, message: String },
    #[error("Config {path} extends unknown preset {name:?}; built-in presets: {presets}")]
    UnknownPreset {
        path: Utf8PathBuf,
        name: String,
        presets: String,
    },
    #[error("Cycle in extends chain: {chain}")]
    ExtendsCycle { chain: String },
    #[error("Rule {slug:?}: {message}")]
    Rule { slug: String, message: String },
    #[error("Invalid glob pattern {pattern:?}: {source}")]
    BadGlob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}
