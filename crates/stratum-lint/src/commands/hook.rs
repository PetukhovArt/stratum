use std::io::Read;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use stratum_core::severity::Severity;

use crate::{
    cli::{Format, LintArgs},
    commands::lint,
    report::Report,
};

const EDIT_TOOLS: [&str; 2] = ["Edit", "Write"];
const SOURCE_EXTENSIONS: [&str; 6] = ["ts", "tsx", "mts", "cts", "js", "vue"];
const POST_TOOL_USE_MAX: usize = 10;

#[derive(Debug, Deserialize)]
struct HookEvent {
    #[serde(rename = "hook_event_name")]
    name: String,
    cwd: Option<Utf8PathBuf>,
    #[serde(default)]
    stop_hook_active: bool,
    tool_name: Option<String>,
    tool_input: Option<ToolInput>,
}

#[derive(Debug, Deserialize)]
struct ToolInput {
    file_path: Option<Utf8PathBuf>,
}

/// Returns the stdout payload: `Some` only when the stop must be blocked
/// or an edited file has new error or warning violations.
/// <https://code.claude.com/docs/en/hooks#stop-decision-control>
/// <https://code.claude.com/docs/en/hooks#posttooluse-decision-control>
pub fn run(max: Option<usize>) -> miette::Result<Option<String>> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| miette::miette!("Could not read hook input: {e}"))?;
    let event: HookEvent =
        serde_json::from_str(&input).map_err(|e| miette::miette!("Invalid hook input: {e}"))?;
    match event.name.as_str() {
        "Stop" | "SubagentStop" if !event.stop_hook_active => stop(event, max),
        "PostToolUse" => post_tool_use(event, max),
        _ => Ok(None),
    }
}

fn stop(event: HookEvent, max: Option<usize>) -> miette::Result<Option<String>> {
    let args = agent_args(max, Vec::new(), Some("HEAD".into()));
    let report = lint::gate(&cwd(event.cwd)?, &args)?;
    if report.exit_code() == 0 {
        return Ok(None);
    }
    let reason = render(&args, &report)?;
    let decision = serde_json::json!({ "decision": "block", "reason": reason });
    Ok(Some(decision.to_string()))
}

fn post_tool_use(event: HookEvent, max: Option<usize>) -> miette::Result<Option<String>> {
    if !event
        .tool_name
        .as_deref()
        .is_some_and(|t| EDIT_TOOLS.contains(&t))
    {
        return Ok(None);
    }
    let Some(file) = event.tool_input.and_then(|t| t.file_path) else {
        return Ok(None);
    };
    let root = cwd(event.cwd)?;
    let Some(file) = source_under(&root, &file) else {
        return Ok(None);
    };
    let args = agent_args(Some(max.unwrap_or(POST_TOOL_USE_MAX)), vec![file], None);
    let report = lint::gate(&root, &args)?;
    let findings: Vec<_> = report
        .findings
        .into_iter()
        .filter(|f| f.severity != Severity::Info)
        .collect();
    if findings.is_empty() {
        return Ok(None);
    }
    let context = render(&args, &Report::new(findings, report.summary))?;
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": context,
        }
    });
    Ok(Some(output.to_string()))
}

fn source_under(root: &Utf8Path, file: &Utf8Path) -> Option<Utf8PathBuf> {
    if !file
        .extension()
        .is_some_and(|e| SOURCE_EXTENSIONS.contains(&e))
    {
        return None;
    }
    let root = root.canonicalize_utf8().ok()?;
    let file = root.join(file).canonicalize_utf8().ok()?;
    file.starts_with(&root).then_some(file)
}

fn cwd(cwd: Option<Utf8PathBuf>) -> miette::Result<Utf8PathBuf> {
    cwd.ok_or_else(|| miette::miette!("Hook input has no cwd"))
}

fn agent_args(max: Option<usize>, files: Vec<Utf8PathBuf>, changed: Option<String>) -> LintArgs {
    LintArgs {
        format: Format::Agent,
        config: None,
        watch: false,
        max,
        files,
        changed,
        baseline: None,
        no_baseline: false,
    }
}

fn render(args: &LintArgs, report: &Report) -> miette::Result<String> {
    let mut text = Vec::new();
    lint::render(args, report, &mut text).map_err(|e| miette::miette!("{e}"))?;
    Ok(String::from_utf8_lossy(&text).trim_end().to_owned())
}
