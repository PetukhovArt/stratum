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
    #[error("Layer {layer:?}: path does not exist or is not a directory: {resolved}")]
    LayerPathNotFound {
        layer: String,
        resolved: Utf8PathBuf,
    },
    #[error("Invalid glob pattern {pattern:?}: {source}")]
    BadGlob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}
