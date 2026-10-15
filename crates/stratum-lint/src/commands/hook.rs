use std::io::Read;

use camino::Utf8PathBuf;
use serde::Deserialize;

use crate::{
    cli::{Format, LintArgs},
    commands::lint,
};

#[derive(Debug, Deserialize)]
struct HookEvent {
    #[serde(rename = "hook_event_name")]
    name: String,
    cwd: Option<Utf8PathBuf>,
    #[serde(default)]
    stop_hook_active: bool,
}

/// Returns the stdout payload: `Some` only when the stop must be blocked.
/// <https://code.claude.com/docs/en/hooks#stop-decision-control>
pub fn run() -> miette::Result<Option<String>> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| miette::miette!("Could not read hook input: {e}"))?;
    let event: HookEvent =
        serde_json::from_str(&input).map_err(|e| miette::miette!("Invalid hook input: {e}"))?;
    if !matches!(event.name.as_str(), "Stop" | "SubagentStop") || event.stop_hook_active {
        return Ok(None);
    }
    let args = LintArgs {
        format: Format::Agent,
        config: None,
        watch: false,
        max: None,
        files: Vec::new(),
        changed: Some("HEAD".into()),
        baseline: None,
        no_baseline: false,
    };
    let cwd = event
        .cwd
        .ok_or_else(|| miette::miette!("Hook input has no cwd"))?;
    let report = lint::gate(&cwd, &args)?;
    if report.exit_code() == 0 {
        return Ok(None);
    }
    let mut text = Vec::new();
    lint::render(&args, &report, &mut text).map_err(|e| miette::miette!("{e}"))?;
    let reason = String::from_utf8_lossy(&text).trim_end().to_owned();
    let decision = serde_json::json!({ "decision": "block", "reason": reason });
    Ok(Some(decision.to_string()))
}
