//! `stratum/co-change`, the diff-mode rule: run on the changed files, never on the graph alone.

use camino::Utf8Path;
use stratum_config::{Config, ConfigError};
use stratum_core::{
    severity::Severity,
    violation::{SourceLocation, Violation},
};
use stratum_rules::{
    CO_CHANGE, CO_CHANGE_SLUG, CoChangeMiss, CoChangeOptions, EffectiveSeverities,
};

use crate::finding::Finding;
use crate::pipeline::EngineInput;

const DIRECTIVE: &str = "stratum-ignore";

/// The policies `changed` (relative to `root`, with `/`) breaks, those a changed `when` file
/// acknowledges left out.
#[must_use]
pub fn misses(root: &Utf8Path, config: &Config, changed: &[String]) -> Vec<CoChangeMiss> {
    let Some(options) = config
        .rules
        .get(CO_CHANGE_SLUG)
        .filter(|rule| crate::side::enabled(config, CO_CHANGE_SLUG) && !rule.options.is_null())
        .and_then(|rule| serde_json::from_value::<CoChangeOptions>(rule.options.clone()).ok())
    else {
        return Vec::new();
    };
    options
        .misses(changed)
        .into_iter()
        .filter(|miss| !miss.when.iter().any(|file| acknowledges(root, file)))
        .collect()
}

/// The findings of `stratum/co-change` for a lint run limited to `changed`, each at line 1 of
/// the first changed `when` file, capped by that file's severity.
pub(crate) fn findings(
    input: &EngineInput,
    changed: &[String],
) -> Result<Vec<Finding>, ConfigError> {
    let Some(rule) = input.registry.by_slug(CO_CHANGE_SLUG) else {
        return Ok(Vec::new());
    };
    let severities = EffectiveSeverities::new(&input.config, &input.project_root)?;
    let mut out = Vec::new();
    for miss in misses(&input.project_root, &input.config, changed) {
        let file = &miss.when[0];
        let cap = severities.for_slashed(file).of(rule);
        if cap == Severity::Off {
            continue;
        }
        let path = file
            .split('/')
            .fold(input.project_root.clone(), |p, segment| p.join(segment));
        let modules = input
            .graph
            .modules
            .values()
            .filter(|m| m.path == path.as_std_path())
            .map(|m| m.id)
            .collect();
        out.push(input.finding(Violation {
            rule: CO_CHANGE,
            severity: Severity::Error.clamp(Severity::Info, cap),
            message: message(&miss),
            file: path.into_std_path_buf(),
            location: SourceLocation { line: 1, column: 1 },
            modules,
            edge: None,
            fix: Some(miss.fix.clone().unwrap_or_else(|| fix(&miss))),
        }));
    }
    Ok(out)
}

#[must_use]
pub fn message(miss: &CoChangeMiss) -> String {
    let more = match miss.when.len() {
        1 => String::new(),
        n => format!(" and {} more", n - 1),
    };
    format!(
        "{}: `{}`{more} changed, but no file matching {} did",
        miss.message,
        miss.when[0],
        globs(&miss.require)
    )
}

#[must_use]
pub fn fix(miss: &CoChangeMiss) -> String {
    format!(
        "Change a file matching {} in the same change, or acknowledge it: `stratum-ignore {CO_CHANGE_SLUG} -- <reason>` in a comment of `{}`",
        globs(&miss.require),
        miss.when[0]
    )
}

fn globs(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|p| format!("`{p}`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

/// `file` holds a `stratum-ignore` directive naming the rule, with a reason, in any comment
/// syntax: the text is scanned line by line, so `//`, `#`, `--` and `<!-- -->` all work.
fn acknowledges(root: &Utf8Path, file: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join(file)) else {
        return false;
    };
    text.lines().any(acknowledging_line)
}

fn acknowledging_line(line: &str) -> bool {
    let Some((_, rest)) = line.split_once(DIRECTIVE) else {
        return false;
    };
    if !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let Some((slugs, reason)) = rest.split_once("--") else {
        return false;
    };
    let reason = reason
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("-->")
        .trim();
    !reason.is_empty() && slugs.split(',').any(|s| s.trim() == CO_CHANGE_SLUG)
}

#[cfg(test)]
mod tests {
    use super::acknowledging_line;

    #[test]
    fn a_directive_naming_the_rule_with_a_reason_acknowledges_in_any_comment_syntax() {
        for line in [
            "// stratum-ignore stratum/co-change -- docs follow in !42",
            "-- stratum-ignore stratum/public-api, stratum/co-change -- migration only",
            "<!-- stratum-ignore stratum/co-change -- typo fix -->",
            "# stratum-ignore stratum/co-change -- comment only",
        ] {
            assert!(acknowledging_line(line), "{line}");
        }
        for line in [
            "// stratum-ignore stratum/co-change",
            "// stratum-ignore stratum/co-change -- */",
            "// stratum-ignore stratum/public-api -- other rule",
            "// stratum-ignored stratum/co-change -- not the directive",
        ] {
            assert!(!acknowledging_line(line), "{line}");
        }
    }
}
