use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::Result;
use crate::budgets::{self, Budget};
use crate::generate::{self, MODULES};
use crate::peak::{self, Run};

const TOUCHED_FILES: usize = 20;
const CACHE_TRUST_WINDOW: Duration = Duration::from_millis(2100);

struct Scenario<'a> {
    scope: &'static str,
    args: Vec<&'a str>,
    cold: bool,
}

struct Measured {
    p50: u128,
    p95: u128,
    peak_mb: u64,
}

pub fn check(project: Option<PathBuf>, bin: Option<PathBuf>, runs: usize) -> Result<bool> {
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }
    let budgets = budgets::from_harness_doc()?;
    let bin = match bin {
        Some(bin) => bin,
        None => build_release_lint()?,
    };
    let scratch = tempfile::tempdir()?;
    let generated = project.is_none();
    let project = if let Some(project) = project {
        project
    } else {
        let dir = scratch.path().join("budget-app");
        generate::generate(&dir, MODULES)?;
        dir
    };

    let mut sources = Vec::new();
    collect_ts(&project, &mut sources)?;
    sources.sort();
    if sources.is_empty() {
        return Err(format!("no .ts files under {}", project.display()).into());
    }
    let file = relative(&project, &sources[sources.len() / 2]);
    if generated {
        let step = (sources.len() / TOUCHED_FILES).max(1);
        for path in sources.iter().step_by(step).take(TOUCHED_FILES) {
            let mut text = fs::read_to_string(path)?;
            text.push_str("// touched\n");
            fs::write(path, text)?;
        }
        std::thread::sleep(CACHE_TRUST_WINDOW);
    }

    let warm = scratch.path().join("warm");
    expect_lint_ran(&peak::run_draining_stdout(&mut lint(
        &bin,
        &project,
        &warm,
        &[],
    ))?)?;

    let scenarios = [
        Scenario {
            scope: "One file",
            args: vec!["--files", &file],
            cold: false,
        },
        Scenario {
            scope: "Changed",
            args: vec!["--changed"],
            cold: false,
        },
        Scenario {
            scope: "Full",
            args: vec![],
            cold: false,
        },
        Scenario {
            scope: "Full, cold",
            args: vec![],
            cold: true,
        },
    ];
    println!(
        "project {} ({} .ts files), {runs} runs each\n",
        project.display(),
        sources.len()
    );
    println!(
        "{:<11} {:>8} {:>8} {:>8} {:>9} {:>9}  status",
        "scope", "p50 ms", "p95 ms", "budget", "peak MB", "budget"
    );
    let mut passed = true;
    for scenario in &scenarios {
        let budget = budgets
            .iter()
            .find(|b| b.scope == scenario.scope)
            .ok_or_else(|| format!("no `{}` row in the budgets table", scenario.scope))?;
        let measured = measure(scenario, &bin, &project, &warm, scratch.path(), runs)?;
        passed &= report(scenario.scope, &measured, budget);
    }
    Ok(passed)
}

fn report(scope: &str, m: &Measured, budget: &Budget) -> bool {
    let within = m.p95 <= u128::from(budget.p95_ms) && m.peak_mb <= budget.rss_mb;
    println!(
        "{scope:<11} {:>8} {:>8} {:>8} {:>9} {:>9}  {}",
        m.p50,
        m.p95,
        budget.p95_ms,
        m.peak_mb,
        budget.rss_mb,
        if within { "ok" } else { "OVER BUDGET" }
    );
    within
}

fn measure(
    scenario: &Scenario,
    bin: &Path,
    project: &Path,
    warm: &Path,
    scratch: &Path,
    runs: usize,
) -> Result<Measured> {
    let mut walls = Vec::new();
    let mut peak_bytes = 0;
    for _ in 0..runs {
        let cold_dir;
        let cache = if scenario.cold {
            cold_dir = tempfile::tempdir_in(scratch)?;
            cold_dir.path()
        } else {
            warm
        };
        let run = peak::run_draining_stdout(&mut lint(bin, project, cache, &scenario.args))?;
        expect_lint_ran(&run)?;
        walls.push(run.wall);
        peak_bytes = peak_bytes.max(run.peak_bytes);
    }
    walls.sort();
    let at = |pct: usize| walls[(runs * pct).div_ceil(100) - 1].as_millis();
    Ok(Measured {
        p50: at(50),
        p95: at(95),
        peak_mb: peak_bytes >> 20,
    })
}

fn expect_lint_ran(run: &Run) -> Result<()> {
    if matches!(run.code, 0 | 1) {
        Ok(())
    } else {
        Err(format!(
            "stratum-lint exited with {}: the measurement is void",
            run.code
        )
        .into())
    }
}

fn lint(bin: &Path, project: &Path, cache: &Path, scope: &[&str]) -> Command {
    let mut cmd = Command::new(bin);
    cmd.current_dir(project)
        .args([".", "--no-baseline", "--format", "jsonl", "--cache-dir"])
        .arg(cache)
        .args(scope);
    cmd
}

fn build_release_lint() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or("the executable has no directory")?;
    if dir.file_name() != Some("release".as_ref()) {
        return Err(
            "run with `cargo run --release -p stratum-budget -- check`, or pass --bin".into(),
        );
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "--release", "-p", "stratum-lint"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;
    if !status.success() {
        return Err("building release stratum-lint failed".into());
    }
    Ok(dir.join(format!("stratum-lint{}", std::env::consts::EXE_SUFFIX)))
}

fn collect_ts(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_ts(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "ts") {
            out.push(path);
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}
