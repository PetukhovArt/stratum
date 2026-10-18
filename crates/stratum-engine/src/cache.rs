use std::sync::OnceLock;

use camino::{Utf8Path, Utf8PathBuf};

pub const CACHE_DIR_ENV: &str = "STRATUM_CACHE_DIR";

static DIR: OnceLock<Option<Utf8PathBuf>> = OnceLock::new();

/// Fixes the Graph Cache directory for this process; only the first call before any build counts.
/// `flag` is `--cache-dir`; without it, `STRATUM_CACHE_DIR`, else the OS user cache dir.
pub fn init(flag: Option<Utf8PathBuf>) {
    DIR.set(resolve(flag)).ok();
}

/// `None` when there is no flag, no env and no OS user cache dir: the cache is off.
#[must_use]
pub fn dir() -> Option<&'static Utf8Path> {
    DIR.get_or_init(|| resolve(None)).as_deref()
}

fn resolve(flag: Option<Utf8PathBuf>) -> Option<Utf8PathBuf> {
    flag.filter(|dir| !dir.as_str().is_empty())
        .or_else(|| {
            std::env::var(CACHE_DIR_ENV)
                .ok()
                .filter(|dir| !dir.is_empty())
                .map(Utf8PathBuf::from)
        })
        .or_else(|| Utf8PathBuf::from_path_buf(dirs::cache_dir()?.join("stratum")).ok())
}
