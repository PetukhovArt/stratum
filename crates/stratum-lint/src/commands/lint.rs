use std::io::Write;

use camino::Utf8Path;
use stratum_core::{severity::Severity, violation::Violation};

use crate::{
    cli::Format,
    pipeline,
    reporters::{JsonReporter, Reporter, SarifReporter, TerminalReporter},
    zero_config,
};

pub fn run(root: &Utf8Path, format: Format, config_path: Option<&Utf8Path>) -> miette::Result<i32> {
    let config = if let Some(p) = config_path {
        stratum_config::parse_file(p).map_err(|e| miette::miette!("{e}"))?
    } else {
        let default = root.join("stratum.config.jsonc");
        if default.exists() {
            stratum_config::parse_file(&default).map_err(|e| miette::miette!("{e}"))?
        } else {
            zero_config::infer(root)
        }
    };
    let violations = pipeline::run(root, &config).map_err(|e| miette::miette!("{e}"))?;

    let mut stdout = std::io::stdout().lock();
    write_with_format(format, &violations, &mut stdout).map_err(|e| miette::miette!("{e}"))?;
    stdout.flush().ok();

    Ok(exit_code(&violations))
}

fn write_with_format(format: Format, vs: &[Violation], w: &mut dyn Write) -> std::io::Result<()> {
    match format {
        Format::Terminal => TerminalReporter.write(vs, w),
        Format::Json => JsonReporter.write(vs, w),
        Format::Sarif => SarifReporter.write(vs, w),
    }
}

fn exit_code(vs: &[Violation]) -> i32 {
    i32::from(vs.iter().any(|v| v.severity == Severity::Error))
}
