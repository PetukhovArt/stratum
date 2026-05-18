use stratum_core::stage::Stage;

/// Scan the leading comments of `source` for a `// @stratum-stage N` directive.
/// Returns `None` if absent or if `N` is out of the valid `1..=4` range.
#[must_use]
pub fn extract_stage(source: &str) -> Option<Stage> {
    for line in source.lines().take(20) {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") && !trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("//") {
            let rest = rest.trim_start();
            if let Some(rank_str) = rest.strip_prefix("@stratum-stage ") {
                if let Ok(rank) = rank_str.trim().parse::<u8>() {
                    if let Ok(s) = Stage::new(rank) {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn finds_directive_on_first_line() {
        assert_eq!(
            extract_stage("// @stratum-stage 3\nexport const x = 1;"),
            Some(Stage::new(3).unwrap())
        );
    }

    #[test]
    fn finds_directive_after_other_comments() {
        let src = "// Header.\n// @stratum-stage 1\nexport const x = 1;";
        assert_eq!(extract_stage(src), Some(Stage::new(1).unwrap()));
    }

    #[test]
    fn ignores_invalid_rank() {
        assert_eq!(extract_stage("// @stratum-stage 9\n"), None);
    }

    #[test]
    fn stops_scanning_after_first_non_comment_line() {
        let src = "export const x = 1;\n// @stratum-stage 2\n";
        assert_eq!(extract_stage(src), None);
    }
}
