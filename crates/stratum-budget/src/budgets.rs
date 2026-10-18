const HARNESS: &str = include_str!("../../../docs/harness.md");
const START: &str = "<!-- budgets:start -->";
const END: &str = "<!-- budgets:end -->";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub scope: String,
    pub p95_ms: u64,
    pub rss_mb: u64,
}

pub fn from_harness_doc() -> Result<Vec<Budget>, String> {
    parse(HARNESS)
}

pub fn parse(doc: &str) -> Result<Vec<Budget>, String> {
    let (_, rest) = doc
        .split_once(START)
        .ok_or("budgets:start marker missing")?;
    let (table, _) = rest.split_once(END).ok_or("budgets:end marker missing")?;
    let mut rows = table.lines().map(str::trim).filter(|l| l.starts_with('|'));
    rows.next().ok_or("budgets table has no header")?;
    rows.next().ok_or("budgets table has no separator")?;
    rows.map(row).collect()
}

fn row(line: &str) -> Result<Budget, String> {
    let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
    let [scope, _invocation, p95, rss] = cells[..] else {
        return Err(format!("budgets row needs 4 cells: {line}"));
    };
    Ok(Budget {
        scope: scope.to_owned(),
        p95_ms: amount(p95, "ms")?,
        rss_mb: amount(rss, "MB")?,
    })
}

fn amount(cell: &str, unit: &str) -> Result<u64, String> {
    cell.strip_suffix(unit)
        .map(|n| n.trim().replace(',', ""))
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| format!("expected a number in {unit}, got `{cell}`"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_doc_table_still_parses_into_the_four_scopes_the_check_measures() {
        let budgets = from_harness_doc().unwrap();
        let scopes: Vec<&str> = budgets.iter().map(|b| b.scope.as_str()).collect();
        assert_eq!(scopes, ["One file", "Changed", "Full", "Full, cold"]);
        assert!(budgets.iter().all(|b| b.p95_ms > 0 && b.rss_mb > 0));
        assert_eq!(budgets[3].p95_ms, 1500);
    }
}
