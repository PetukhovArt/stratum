use std::fs;
use std::io::Write;
use std::ops::Range;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use stratum_core::ids::ModuleId;
use stratum_parser_ts::TrackedCalls;
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

use super::Parsed;

const MAGIC: &[u8; 8] = b"STRGRPH3";
const HEAD_WORDS: usize = 5;
const HEAD: usize = MAGIC.len() + HEAD_WORDS * 8;
const RACY_NS: u64 = 2_000_000_000;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// How a graph build used the Graph Cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CacheStats {
    pub state: CacheState,
    /// Modules parsed from their text in this build, not taken from the cache.
    pub reparsed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheState {
    /// A cache file written by this binary with these tracked calls was read.
    Warm,
    /// The cache file is absent, corrupt, or written by another binary or tracked calls.
    Cold,
    /// No cache directory, or the build reads overlay text.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Stat {
    pub size: u64,
    pub mtime_ns: u64,
}

impl Stat {
    pub(super) const ZERO: Self = Self {
        size: 0,
        mtime_ns: 0,
    };

    pub(super) fn of(meta: &fs::Metadata) -> Self {
        Self {
            size: meta.len(),
            mtime_ns: meta.modified().map_or(0, nanos),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Index<P> {
    env: u64,
    written_ns: u64,
    slots: Vec<Slot<P>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Slot<P> {
    path: P,
    stat: Stat,
    hash: u64,
    len: u64,
}

#[derive(Debug)]
pub(super) struct Written<'a> {
    pub path: &'a str,
    pub stat: Stat,
    pub hash: u64,
    pub blob: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Cached<'a> {
    pub hash: u64,
    pub blob: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Prior<'a> {
    Fresh(Cached<'a>),
    Verify(Cached<'a>),
    Miss,
}

#[derive(Debug)]
pub(super) struct Cache {
    file: Utf8PathBuf,
    head: [u64; 3],
}

#[derive(Debug)]
pub(super) struct Loaded {
    pub env: u64,
    written_ns: u64,
    bytes: Vec<u8>,
    slots: FxHashMap<String, (Stat, u64, Range<usize>)>,
}

impl Cache {
    pub(super) fn new(
        dir: &Utf8Path,
        root: &Utf8Path,
        identity: &str,
        tracked: &TrackedCalls,
    ) -> Self {
        let mut key = Xxh3::new();
        key.update(root.as_str().as_bytes());
        key.update(&[0]);
        key.update(identity.as_bytes());
        let exe = std::env::current_exe()
            .and_then(fs::metadata)
            .map_or(Stat::ZERO, |m| Stat::of(&m));
        let tracked = serde_json::to_vec(tracked).map_or(0, |bytes| xxh3_64(&bytes));
        Self {
            file: dir.join(format!("stratum-{:016x}.graph", key.digest())),
            head: [exe.size, exe.mtime_ns, tracked],
        }
    }

    pub(super) fn load(&self) -> Option<Loaded> {
        let bytes = fs::read(&self.file).ok()?;
        let (head, rest) = bytes.split_at_checked(HEAD)?;
        let (magic, words) = head.split_at(MAGIC.len());
        let words: Vec<u64> = words
            .chunks_exact(8)
            .map(|w| u64::from_le_bytes(w.try_into().unwrap_or_default()))
            .collect();
        if magic != MAGIC || words[..3] != self.head || words[3] != xxh3_64(rest) {
            return None;
        }
        let index_len = usize::try_from(words[4]).ok()?;
        let index: Index<String> = postcard::from_bytes(rest.get(..index_len)?).ok()?;
        let mut at = HEAD.checked_add(index_len)?;
        let mut slots = FxHashMap::default();
        for slot in index.slots {
            let end = at.checked_add(usize::try_from(slot.len).ok()?)?;
            slots.insert(slot.path, (slot.stat, slot.hash, at..end));
            at = end;
        }
        (at == bytes.len()).then_some(Loaded {
            env: index.env,
            written_ns: index.written_ns,
            bytes,
            slots,
        })
    }

    pub(super) fn encode(&self, env: u64, modules: &[Written<'_>]) -> Option<Vec<u8>> {
        let mut rest = postcard::to_stdvec(&Index {
            env,
            written_ns: nanos(SystemTime::now()),
            slots: modules
                .iter()
                .map(|m| Slot {
                    path: m.path,
                    stat: m.stat,
                    hash: m.hash,
                    len: u64::try_from(m.blob.len()).unwrap_or(u64::MAX),
                })
                .collect(),
        })
        .ok()?;
        let index_len = u64::try_from(rest.len()).ok()?;
        for m in modules {
            rest.extend_from_slice(m.blob);
        }
        let mut bytes = Vec::with_capacity(HEAD + rest.len());
        bytes.extend_from_slice(MAGIC);
        for word in self.head.into_iter().chain([xxh3_64(&rest), index_len]) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(&rest);
        Some(bytes)
    }

    pub(super) fn write(&self, bytes: &[u8]) {
        let (Some(dir), Some(name)) = (self.file.parent(), self.file.file_name()) else {
            return;
        };
        if fs::create_dir_all(dir).is_err() {
            return;
        }
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let written = fs::File::create(&tmp).and_then(|mut f| f.write_all(bytes));
        if written.is_err() || fs::rename(&tmp, &self.file).is_err() {
            fs::remove_file(&tmp).ok();
        }
        prune(dir, name);
    }
}

impl Loaded {
    pub(super) fn prior(&self, path: &str, now: Stat) -> Prior<'_> {
        let Some((stat, hash, range)) = self.slots.get(path) else {
            return Prior::Miss;
        };
        let Some(blob) = self.bytes.get(range.clone()) else {
            return Prior::Miss;
        };
        let cached = Cached { hash: *hash, blob };
        let racy = stat.mtime_ns.saturating_add(RACY_NS) >= self.written_ns;
        if *stat == now && !racy {
            Prior::Fresh(cached)
        } else {
            Prior::Verify(cached)
        }
    }
}

pub(super) fn encode_facts(parsed: &Parsed, targets: &[Vec<ModuleId>]) -> Option<Vec<u8>> {
    postcard::to_stdvec(&(parsed, targets)).ok()
}

pub(super) fn decode_facts(blob: &[u8]) -> Option<(Parsed, Vec<Vec<ModuleId>>)> {
    postcard::from_bytes(blob).ok()
}

pub(super) fn env_fingerprint<'a>(
    root: &Utf8Path,
    modules: &[&Utf8Path],
    walked: impl Iterator<Item = &'a Utf8Path>,
) -> u64 {
    let mut h = Xxh3::new();
    for m in modules {
        h.update(m.as_str().as_bytes());
        h.update(&[0]);
    }
    for path in walked.filter(|p| shapes_resolution(root, p)) {
        h.update(path.as_str().as_bytes());
        h.update(&[1]);
        h.update(&xxh3_64(&fs::read(path).unwrap_or_default()).to_le_bytes());
    }
    h.digest()
}

pub(super) fn shapes_resolution(root: &Utf8Path, path: &Utf8Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let json = path.extension() == Some("json");
    let config = name == "package.json"
        || (json && (name.starts_with("tsconfig") || name.starts_with("jsconfig")));
    let root_level =
        path.parent() == Some(root) && (json || name == "pnpm-lock.yaml" || name == "yarn.lock");
    config || root_level
}

pub(super) fn hash(text: &str) -> u64 {
    xxh3_64(text.as_bytes())
}

fn prune(dir: &Utf8Path, own: &str) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = Utf8Path::from_path(&path).filter(|p| p.extension() == Some("graph"))
        else {
            continue;
        };
        if name.file_name() == Some(own)
            || !name.file_name().is_some_and(|n| n.starts_with("stratum-"))
        {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > MAX_AGE));
        if stale {
            fs::remove_file(&path).ok();
        }
    }
}

fn nanos(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}
