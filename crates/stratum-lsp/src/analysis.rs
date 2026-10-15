use std::path::PathBuf;

use camino::Utf8Path;
use stratum_config::ConfigError;
use stratum_lint::engine::RuleEngine;
use stratum_lint::pipeline::{self, PipelineError};
use stratum_lint::zero_config;

use crate::diagnostics::{self, DiagnosticsByFile};

pub const CONFIG_FILE: &str = "stratum.config.jsonc";

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
}

#[derive(Debug)]
pub struct Analysis {
    pub inputs: Vec<PathBuf>,
    pub outcome: Result<DiagnosticsByFile, AnalysisError>,
}

#[must_use]
pub fn analyze(root: &Utf8Path) -> Analysis {
    let config_path = root.join(CONFIG_FILE);
    let mut inputs = vec![config_path.clone().into_std_path_buf()];
    let config = if config_path.is_file() {
        match stratum_config::parse_file(&config_path) {
            Ok(c) => c,
            Err(e) => {
                return Analysis {
                    inputs,
                    outcome: Err(e.into()),
                };
            }
        }
    } else {
        zero_config::infer(root)
    };
    inputs.extend(
        config
            .rules
            .values()
            .filter_map(|r| r.script.as_ref())
            .map(|s| root.join(s).into_std_path_buf()),
    );
    let outcome = pipeline::build(root, &config)
        .map(|input| diagnostics::group(&input, &RuleEngine::run(&input)))
        .map_err(AnalysisError::from);
    Analysis { inputs, outcome }
}
