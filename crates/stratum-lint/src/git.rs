use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use tar::EntryType;

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

#[expect(dead_code, reason = "the diff command lands in the drift slice")]
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

/// Pathspecs that match nothing at `reference` are skipped; no pathspecs give an empty tree.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the diff command lands in the drift slice")
)]
pub fn tree_at(root: &Utf8Path, reference: &str, pathspecs: &[String]) -> miette::Result<TempTree> {
    verify(root, reference)?;
    let tree = TempTree::create()?;
    let mut specs: Vec<&str> = pathspecs.iter().map(String::as_str).collect();
    while !specs.is_empty() {
        let Some(unmatched) = archive(root, reference, &specs, &tree.0)? else {
            break;
        };
        let before = specs.len();
        specs.retain(|s| *s != unmatched);
        if specs.len() == before {
            return Err(miette::miette!(
                "git archive rejected pathspec {unmatched:?} that was not passed"
            ));
        }
    }
    Ok(tree)
}

#[derive(Debug)]
pub struct TempTree(Utf8PathBuf);

impl TempTree {
    fn create() -> miette::Result<Self> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "stratum-tree-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(name))
            .map_err(|p| miette::miette!("Temp dir {} is not UTF-8", p.display()))?;
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).map_err(|e| miette::miette!("Could not create {dir}: {e}"))?;
        Ok(Self(dir))
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the diff command lands in the drift slice")
    )]
    pub fn path(&self) -> &Utf8Path {
        &self.0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn archive(
    root: &Utf8Path,
    reference: &str,
    specs: &[&str],
    into: &Utf8Path,
) -> miette::Result<Option<String>> {
    let mut child = git(root)
        .args(["archive", "--format=tar", reference, "--"])
        .args(specs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| miette::miette!("git archive: no stdout"))?;
    let unpacked = unpack(&mut stdout, into);
    let _ = std::io::copy(&mut stdout, &mut std::io::sink());
    drop(stdout);
    let out = child.wait_with_output().map_err(spawn_error)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return unmatched_pathspec(&stderr)
            .map(Some)
            .ok_or_else(|| miette::miette!("git archive {reference} failed: {}", stderr.trim()));
    }
    unpacked
        .map(|()| None)
        .map_err(|e| miette::miette!("Could not extract {reference} into {into}: {e}"))
}

fn unpack(reader: impl Read, into: &Utf8Path) -> std::io::Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = entry?;
        if matches!(
            entry.header().entry_type(),
            EntryType::Regular | EntryType::Directory
        ) {
            entry.unpack_in(into)?;
        }
    }
    Ok(())
}

fn unmatched_pathspec(stderr: &str) -> Option<String> {
    let rest = stderr.split("pathspec '").nth(1)?;
    let (spec, tail) = rest.split_once('\'')?;
    tail.trim_start()
        .starts_with("did not match any files")
        .then(|| spec.to_owned())
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
    cmd.arg("-C").arg(root.as_str());
    cmd
}

#[allow(clippy::needless_pass_by_value)]
fn spawn_error(e: std::io::Error) -> miette::Report {
    miette::miette!("Could not run git: {e}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn repo_with_project_in_subdir() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let root = repo.join("web");
        std::fs::create_dir_all(root.join("src/shared")).unwrap();
        std::fs::write(root.join("src/shared/a.ts"), "export const a = 1;").unwrap();
        std::fs::write(root.join("tsconfig.json"), "{}\n").unwrap();
        std::fs::write(repo.join("outside.ts"), "export {};\n").unwrap();
        for args in [
            &["init", "-q"][..],
            &["add", "."],
            &[
                "-c",
                "user.name=stratum",
                "-c",
                "user.email=stratum@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-m",
                "fixture",
            ],
        ] {
            run(&repo, args).unwrap();
        }
        (dir, root)
    }

    #[test]
    fn tree_at_extracts_root_relative_files_outside_the_repo_and_skips_unmatched_pathspecs() {
        let (dir, root) = repo_with_project_in_subdir();
        std::fs::write(root.join("src/shared/a.ts"), "export const a = 2;").unwrap();
        let specs = ["src".to_owned(), "tsconfig.*.json".to_owned()];

        let tree = tree_at(&root, "HEAD", &specs).unwrap();
        let extracted = tree.path().to_path_buf();

        assert!(!extracted.starts_with(dir.path().to_str().unwrap()));
        assert_eq!(
            std::fs::read_to_string(extracted.join("src/shared/a.ts")).unwrap(),
            "export const a = 1;"
        );
        assert!(!extracted.join("tsconfig.json").exists());
        assert!(!extracted.join("outside.ts").exists());
        let status = run(&root, &["status", "--porcelain", "--untracked-files=all"]).unwrap();
        assert_eq!(
            String::from_utf8(status).unwrap(),
            " M web/src/shared/a.ts\n"
        );
        drop(tree);
        assert!(!extracted.exists());
    }
}
