use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use camino::Utf8Path;

pub fn changed_files(root: &Utf8Path, reference: &str) -> miette::Result<Vec<String>> {
    verify(root, reference)?;
    let mut files = lines(&run(
        root,
        &["diff", "--name-only", "--relative", "-z", reference, "--"],
    )?);
    files.extend(lines(&run(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?));
    Ok(files)
}

/// `None` when `rel` (relative to `root`) does not exist at `reference`.
pub fn show(root: &Utf8Path, reference: &str, rel: &str) -> miette::Result<Option<String>> {
    verify(root, reference)?;
    let object = format!("{reference}:./{rel}");
    let exists = git(root)
        .args(["cat-file", "-e", &object])
        .output()
        .map_err(spawn_error)?
        .status
        .success();
    if !exists {
        return Ok(None);
    }
    String::from_utf8(run(root, &["cat-file", "blob", &object])?)
        .map(Some)
        .map_err(|e| miette::miette!("{object} is not UTF-8: {e}"))
}

pub fn short_sha(root: &Utf8Path, reference: &str) -> miette::Result<String> {
    verify(root, reference)?;
    let out = run(root, &["rev-parse", "--short", &commit(reference)])?;
    Ok(String::from_utf8_lossy(&out).trim().to_owned())
}

/// `root`'s path inside its git work tree, with a trailing `/` unless empty; `None` outside a repo.
pub fn prefix(root: &Utf8Path) -> Option<String> {
    let out = git(root)
        .args(["rev-parse", "--show-prefix"])
        .output()
        .ok()?;
    out.status.success().then(|| {
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches(['\r', '\n'])
            .to_owned()
    })
}

/// Files under `root` whose working-tree state differs from `reference`, untracked ones
/// included, with their text at `reference` (`None`: absent there). Paths are relative to
/// `root`; `keep` filters them before any content is read.
pub fn files_at(
    root: &Utf8Path,
    reference: &str,
    keep: impl Fn(&str) -> bool,
) -> miette::Result<Vec<(String, Option<String>)>> {
    verify(root, reference)?;
    let status = lines(&run(
        root,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "--relative",
            "-z",
            reference,
            "--",
        ],
    )?);
    let mut absent = Vec::new();
    let mut present = Vec::new();
    for pair in status.chunks_exact(2) {
        let (code, path) = (&pair[0], &pair[1]);
        if keep(path) {
            if code.starts_with('A') {
                absent.push(path.clone());
            } else {
                present.push(path.clone());
            }
        }
    }
    let untracked = lines(&run(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?);
    absent.extend(untracked.into_iter().filter(|p| keep(p)));
    let texts = blobs(root, reference, &present)?;
    Ok(present
        .into_iter()
        .zip(texts)
        .chain(absent.into_iter().map(|p| (p, None)))
        .collect())
}

fn blobs(root: &Utf8Path, reference: &str, rels: &[String]) -> miette::Result<Vec<Option<String>>> {
    if rels.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = git(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(miette::miette!("git cat-file: no pipes"));
    };
    let read = std::thread::scope(|s| {
        s.spawn(move || {
            for rel in rels {
                if writeln!(stdin, "{reference}:./{rel}").is_err() {
                    break;
                }
            }
        });
        read_batch(BufReader::new(stdout), rels.len())
    });
    let out = child.wait_with_output().map_err(spawn_error)?;
    if !out.status.success() {
        return Err(miette::miette!(
            "git cat-file --batch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    read.map_err(|e| miette::miette!("Could not read git cat-file output: {e}"))
}

fn read_batch(mut out: impl BufRead, count: usize) -> std::io::Result<Vec<Option<String>>> {
    let mut texts = Vec::with_capacity(count);
    let mut header = String::new();
    for _ in 0..count {
        header.clear();
        out.read_line(&mut header)?;
        let header = header.trim_end();
        let size = if header.ends_with(" missing") {
            None
        } else {
            header
                .rsplit(' ')
                .next()
                .and_then(|s| s.parse::<usize>().ok())
        };
        let Some(size) = size else {
            texts.push(None);
            continue;
        };
        let mut bytes = vec![0; size + 1];
        out.read_exact(&mut bytes)?;
        bytes.pop();
        texts.push(Some(String::from_utf8_lossy(&bytes).into_owned()));
    }
    Ok(texts)
}

fn verify(root: &Utf8Path, reference: &str) -> miette::Result<()> {
    let out = git(root)
        .args(["rev-parse", "--verify", "--quiet", &commit(reference)])
        .output()
        .map_err(spawn_error)?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.trim().is_empty() {
        return Err(miette::miette!(
            help = format!("git fetch --depth=1 origin {reference}"),
            "Unknown git ref {reference:?} in {root}"
        ));
    }
    Err(miette::miette!("git in {root} failed: {}", stderr.trim()))
}

fn commit(reference: &str) -> String {
    format!("{reference}^{{commit}}")
}

fn run(root: &Utf8Path, args: &[&str]) -> miette::Result<Vec<u8>> {
    let out = git(root).args(args).output().map_err(spawn_error)?;
    if !out.status.success() {
        return Err(miette::miette!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

fn lines(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn git(root: &Utf8Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--no-optional-locks").arg("-C").arg(root.as_str());
    cmd
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_error(e: std::io::Error) -> miette::Report {
    miette::miette!("Could not run git: {e}")
}
