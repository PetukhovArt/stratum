use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use camino::Utf8Path;

pub fn changed_files(root: &Utf8Path, reference: &str) -> miette::Result<Vec<String>> {
    if reference == "HEAD" {
        return changed_since_head(root);
    }
    Ok(changes(root, reference)?
        .into_iter()
        .map(|change| change.path)
        .collect())
}

struct Change {
    path: String,
    absent_at_reference: bool,
}

/// Paths relative to `root` whose working-tree state differs from `reference`, untracked ones
/// included. Renames count as a deletion plus an addition.
fn changes(root: &Utf8Path, reference: &str) -> miette::Result<Vec<Change>> {
    let (verified, diff, untracked) = std::thread::scope(|s| {
        let verified = s.spawn(|| verify(root, reference));
        let diff = s.spawn(|| {
            run(
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
            )
        });
        let untracked = run(root, &["ls-files", "--others", "--exclude-standard", "-z"]);
        (joined(verified), joined(diff), untracked)
    });
    verified?;
    let mut changes: Vec<Change> = lines(&diff?)
        .chunks_exact(2)
        .map(|pair| Change {
            absent_at_reference: pair[0].starts_with('A'),
            path: pair[1].clone(),
        })
        .collect();
    changes.extend(lines(&untracked?).into_iter().map(|path| Change {
        path,
        absent_at_reference: true,
    }));
    Ok(changes)
}

/// Porcelain paths are relative to the work tree top, not to `root`:
/// <https://git-scm.com/docs/git-status#_porcelain_format_version_1>.
fn changed_since_head(root: &Utf8Path) -> miette::Result<Vec<String>> {
    let (head, status) = std::thread::scope(|s| {
        let head = s.spawn(|| {
            git(root)
                .args(["rev-parse", "--show-prefix", "--verify", "--quiet"])
                .arg(commit("HEAD"))
                .output()
                .map_err(spawn_error)
        });
        let status = run(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        );
        (joined(head), status)
    });
    let head = head?;
    if !head.status.success() {
        return verify(root, "HEAD").map(|()| Vec::new());
    }
    let prefix = String::from_utf8_lossy(&head.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    Ok(status_paths(&lines(&status?))
        .filter_map(|path| path.strip_prefix(prefix.as_str()).map(str::to_owned))
        .collect())
}

/// With `-z`, a rename or copy entry `XY to` is followed by a bare `from` entry (git-status docs).
fn status_paths(entries: &[String]) -> impl Iterator<Item = &str> {
    let mut source_next = false;
    entries.iter().filter_map(move |entry| {
        if std::mem::take(&mut source_next) {
            return None;
        }
        let (code, path) = (entry.get(..2)?, entry.get(3..)?);
        source_next = code.contains(['R', 'C']);
        Some(path)
    })
}

fn joined<T>(handle: std::thread::ScopedJoinHandle<'_, miette::Result<T>>) -> miette::Result<T> {
    handle
        .join()
        .unwrap_or_else(|_| Err(miette::miette!("git worker panicked")))
}

/// `None` when `rel` (relative to `root`) does not exist at `reference`.
pub fn show(root: &Utf8Path, reference: &str, rel: &str) -> miette::Result<Option<String>> {
    verify(root, reference)?;
    show_verified(root, reference, rel)
}

fn show_verified(root: &Utf8Path, reference: &str, rel: &str) -> miette::Result<Option<String>> {
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
    let (absent, present): (Vec<Change>, Vec<Change>) = changes(root, reference)?
        .into_iter()
        .filter(|change| keep(&change.path))
        .partition(|change| change.absent_at_reference);
    let present: Vec<String> = present.into_iter().map(|change| change.path).collect();
    let texts = blobs(root, reference, &present)?;
    Ok(present
        .into_iter()
        .zip(texts)
        .chain(absent.into_iter().map(|change| (change.path, None)))
        .collect())
}

/// `cat-file --batch` reads one object name per line
/// (<https://git-scm.com/docs/git-cat-file#_batch_output>), so a path with a line break
/// goes through `git cat-file` on its own.
fn blobs(root: &Utf8Path, reference: &str, rels: &[String]) -> miette::Result<Vec<Option<String>>> {
    let single_line = |rel: &String| !rel.contains(['\n', '\r']);
    let batchable: Vec<&String> = rels.iter().filter(|rel| single_line(rel)).collect();
    let mut batched = batch(root, reference, &batchable)?.into_iter();
    rels.iter()
        .map(|rel| {
            if single_line(rel) {
                Ok(batched.next().flatten())
            } else {
                show_verified(root, reference, rel)
            }
        })
        .collect()
}

fn batch(
    root: &Utf8Path,
    reference: &str,
    rels: &[&String],
) -> miette::Result<Vec<Option<String>>> {
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
        let read = read_batch(BufReader::new(stdout), rels.len());
        if read.is_err() {
            child.kill().ok();
        }
        read
    });
    let out = child.wait_with_output().map_err(spawn_error)?;
    let read = read.map_err(|e| miette::miette!("Could not read git cat-file output: {e}"))?;
    if !out.status.success() {
        return Err(miette::miette!(
            "git cat-file --batch failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(read)
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
