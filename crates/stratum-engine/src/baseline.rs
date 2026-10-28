use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::finding::Finding;

pub const BASELINE_FILE: &str = "stratum.baseline.json";
const VERSION: u32 = 2;
const BUILT_IN_PREFIX: &str = "stratum/";

#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("Could not read baseline {path}: {source}")]
    Read {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid baseline {path}: {source}")]
    Invalid {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Unsupported baseline version {found} in {path} (expected 1 or {VERSION})")]
    Version { path: Utf8PathBuf, found: u32 },
    #[error("Invalid baseline {path}: violation {index} {line:?}: {reason}")]
    Entry {
        path: Utf8PathBuf,
        index: usize,
        line: String,
        reason: &'static str,
    },
    #[error("Could not write {path}: {source}")]
    Write {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The identity of a finding across line shifts: rule, anchor file and message with numbers masked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fingerprint {
    pub rule: String,
    pub file: String,
    pub message: String,
}

impl Fingerprint {
    #[must_use]
    pub fn of(f: &Finding) -> Self {
        Self {
            rule: f.rule.clone(),
            file: f.file.clone(),
            message: without_numbers(&f.message),
        }
    }
}

fn without_numbers(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_digit() {
            out.push(c);
            continue;
        }
        out.push('#');
        while let Some(&next) = chars.peek() {
            let decimal_point =
                next == '.' && chars.clone().nth(1).is_some_and(|d| d.is_ascii_digit());
            if !next.is_ascii_digit() && !decimal_point {
                break;
            }
            chars.next();
        }
    }
    out
}

/// FNV-1a 32 of a text, written as 8 lowercase hex digits; stable across releases and platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(u32);

impl Digest {
    #[must_use]
    pub fn of(text: &str) -> Self {
        const OFFSET: u32 = 0x811c_9dc5;
        const PRIME: u32 = 0x0100_0193;
        Self(text.bytes().fold(OFFSET, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(PRIME)
        }))
    }

    fn parse(token: &str) -> Option<Self> {
        let hex = token.len() == 8
            && token
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if !hex {
            return None;
        }
        u32::from_str_radix(token, 16).ok().map(Self)
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

/// What a baseline entry matches: a [`Fingerprint`] with the masked message reduced to its [`Digest`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub rule: String,
    pub file: String,
    pub message: Digest,
}

impl Key {
    #[must_use]
    pub fn of(f: &Finding) -> Self {
        Self::masked(f.rule.clone(), f.file.clone(), &without_numbers(&f.message))
    }

    fn masked(rule: String, file: String, masked_message: &str) -> Self {
        Self {
            rule,
            file,
            message: Digest::of(masked_message),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Header {
    version: u32,
}

#[derive(Debug, Deserialize)]
struct FileV1 {
    violations: Vec<EntryV1>,
}

#[derive(Debug, Deserialize)]
struct EntryV1 {
    rule: String,
    file: String,
    message: String,
    #[serde(default = "one")]
    count: usize,
    #[serde(default)]
    touches: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
struct FileV2 {
    violations: Vec<String>,
}

const fn one() -> usize {
    1
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Baseline(BTreeMap<Key, Accepted>);

/// `touches`: digests of the edge endpoints and members besides the anchor file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Accepted {
    count: usize,
    touches: BTreeSet<Digest>,
}

impl Accepted {
    fn add(&mut self, count: usize, touches: impl IntoIterator<Item = Digest>) {
        self.count += count;
        self.touches.extend(touches);
    }
}

fn touched_besides_anchor(f: &Finding) -> impl Iterator<Item = Digest> + '_ {
    f.edge
        .iter()
        .flat_map(|e| [&e.from, &e.to])
        .chain(&f.members)
        .filter(|file| **file != f.file)
        .map(|file| Digest::of(file))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Growth {
    pub key: Key,
    pub base: usize,
    pub head: usize,
}

#[derive(Debug, Default)]
pub struct Matched {
    pub new: Vec<Finding>,
    pub baselined: Vec<Finding>,
    fixed: Vec<Fixed>,
}

#[derive(Debug)]
struct Fixed {
    file: String,
    touches: BTreeSet<Digest>,
    count: usize,
}

impl Matched {
    #[must_use]
    pub fn fixed(&self) -> usize {
        self.fixed.iter().map(|f| f.count).sum()
    }

    /// Fixed entries whose anchor file, edge endpoints or members are among `files`.
    #[must_use]
    pub fn fixed_among<'a>(&self, files: impl IntoIterator<Item = &'a str>) -> usize {
        let files: BTreeSet<&str> = files.into_iter().collect();
        let digests: BTreeSet<Digest> = files.iter().map(|file| Digest::of(file)).collect();
        self.fixed
            .iter()
            .filter(|f| files.contains(f.file.as_str()) || !f.touches.is_disjoint(&digests))
            .map(|f| f.count)
            .sum()
    }
}

impl Baseline {
    #[must_use]
    pub fn from_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> Self {
        let mut map = BTreeMap::<_, Accepted>::new();
        for f in findings {
            map.entry(Key::of(f))
                .or_default()
                .add(1, touched_besides_anchor(f));
        }
        Self(map)
    }

    pub fn load(path: &Utf8Path) -> Result<Self, BaselineError> {
        let text = std::fs::read_to_string(path).map_err(|source| BaselineError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text, path)
    }

    /// Reads version 1 (one object per violation) and version 2 (one string per violation).
    pub fn parse(text: &str, origin: &Utf8Path) -> Result<Self, BaselineError> {
        let invalid = |source| BaselineError::Invalid {
            path: origin.to_path_buf(),
            source,
        };
        let header: Header = serde_json::from_str(text).map_err(invalid)?;
        let mut map = BTreeMap::<_, Accepted>::new();
        match header.version {
            1 => {
                let file: FileV1 = serde_json::from_str(text).map_err(invalid)?;
                for e in file.violations {
                    let key = Key::masked(e.rule, e.file, &without_numbers(&e.message));
                    let touches = e.touches.iter().map(|file| Digest::of(file));
                    map.entry(key).or_default().add(e.count, touches);
                }
            }
            VERSION => {
                let file: FileV2 = serde_json::from_str(text).map_err(invalid)?;
                for (index, line) in file.violations.into_iter().enumerate() {
                    match decode_line(&line) {
                        Ok((key, accepted)) => map
                            .entry(key)
                            .or_default()
                            .add(accepted.count, accepted.touches),
                        Err(reason) => {
                            return Err(BaselineError::Entry {
                                path: origin.to_path_buf(),
                                index,
                                line,
                                reason,
                            });
                        }
                    }
                }
            }
            found => {
                return Err(BaselineError::Version {
                    path: origin.to_path_buf(),
                    found,
                });
            }
        }
        Ok(Self(map))
    }

    pub fn load_if_exists(path: &Utf8Path) -> Result<Option<Self>, BaselineError> {
        if path.exists() {
            Self::load(path).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Always writes version 2: one JSON string per line, in key order.
    pub fn save(&self, path: &Utf8Path) -> Result<(), BaselineError> {
        let lines = self
            .0
            .iter()
            .map(|(key, accepted)| serde_json::to_string(&encode_line(key, accepted)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| BaselineError::Invalid {
                path: path.to_path_buf(),
                source,
            })?;
        let mut text = format!("{{\"version\":{VERSION},\"violations\":[\n");
        if !lines.is_empty() {
            text.push_str(&lines.join(",\n"));
            text.push('\n');
        }
        text.push_str("]}\n");
        std::fs::write(path, text).map_err(|source| BaselineError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().map(|a| a.count).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn count(&self, key: &Key) -> usize {
        self.0.get(key).map_or(0, |a| a.count)
    }

    /// Each entry absorbs at most its `count` findings; `fixed` counts the unabsorbed rest.
    #[must_use]
    pub fn apply(&self, findings: Vec<Finding>) -> Matched {
        let mut budget: BTreeMap<&Key, usize> =
            self.0.iter().map(|(key, a)| (key, a.count)).collect();
        let mut matched = Matched::default();
        for f in findings {
            match budget.get_mut(&Key::of(&f)) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    matched.baselined.push(f);
                }
                _ => matched.new.push(f),
            }
        }
        matched.fixed = budget
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(key, count)| Fixed {
                file: key.file.clone(),
                touches: self
                    .0
                    .get(key)
                    .map(|a| a.touches.clone())
                    .unwrap_or_default(),
                count,
            })
            .collect();
        matched
    }

    #[must_use]
    pub fn grown_since(&self, base: &Self) -> Vec<Growth> {
        self.0
            .iter()
            .filter_map(|(key, accepted)| {
                let (head, base) = (accepted.count, base.count(key));
                (head > base).then(|| Growth {
                    key: key.clone(),
                    base,
                    head,
                })
            })
            .collect()
    }

    /// Keeps only entries still produced by `findings`; never adds new ones.
    #[must_use]
    pub fn pruned(&self, findings: &[Finding]) -> Self {
        let current = Self::from_findings(findings);
        Self(
            self.0
                .iter()
                .filter_map(|(key, accepted)| {
                    let count = accepted.count.min(current.count(key));
                    (count > 0).then(|| {
                        let touches = accepted.touches.clone();
                        (key.clone(), Accepted { count, touches })
                    })
                })
                .collect(),
        )
    }
}

/// `<rule> <file> <message digest>[ x<count>][ <touch digest>...]`; `count` is omitted when 1.
fn encode_line(key: &Key, accepted: &Accepted) -> String {
    use fmt::Write;
    let mut line = format!(
        "{} {} {}",
        escape(&short_rule(&key.rule)),
        escape(&key.file),
        key.message
    );
    if accepted.count != 1 {
        let _ = write!(line, " x{}", accepted.count);
    }
    for touch in &accepted.touches {
        let _ = write!(line, " {touch}");
    }
    line
}

fn decode_line(line: &str) -> Result<(Key, Accepted), &'static str> {
    let mut tokens = line.split(' ');
    let rule = tokens.next().filter(|t| !t.is_empty()).ok_or("no rule")?;
    let file = tokens.next().filter(|t| !t.is_empty()).ok_or("no file")?;
    let message = tokens
        .next()
        .and_then(Digest::parse)
        .ok_or("no 8-hex-digit message digest after the file")?;
    let mut accepted = Accepted {
        count: 1,
        touches: BTreeSet::new(),
    };
    let mut tokens = tokens.peekable();
    if let Some(count) = tokens.next_if(|t| t.starts_with('x')) {
        accepted.count = count
            .strip_prefix('x')
            .and_then(|n| n.parse().ok())
            .ok_or("count is not x<number>")?;
    }
    for token in tokens {
        let touch = Digest::parse(token).ok_or("a touched file is not an 8-hex-digit digest")?;
        accepted.touches.insert(touch);
    }
    let key = Key {
        rule: long_rule(&unescape(rule)?),
        file: unescape(file)?,
        message,
    };
    Ok((key, accepted))
}

/// `stratum/<name>` is written `<name>`; a slug that would read back as built-in gets a leading `/`.
fn short_rule(rule: &str) -> String {
    match rule.strip_prefix(BUILT_IN_PREFIX) {
        Some(name) if !name.contains('/') => name.to_owned(),
        _ if !rule.contains('/') || rule.starts_with('/') => format!("/{rule}"),
        _ => rule.to_owned(),
    }
}

fn long_rule(token: &str) -> String {
    if let Some(literal) = token.strip_prefix('/') {
        literal.to_owned()
    } else if token.contains('/') {
        token.to_owned()
    } else {
        format!("{BUILT_IN_PREFIX}{token}")
    }
}

/// A space separates fields, so spaces (and `%` itself) in a rule or file are percent-encoded.
fn escape(field: &str) -> String {
    field.replace('%', "%25").replace(' ', "%20")
}

fn unescape(token: &str) -> Result<String, &'static str> {
    const BAD: &str = "a % is not followed by 20 or 25";
    let mut out = String::with_capacity(token.len());
    let mut rest = token;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        out.push(match rest.get(at + 1..at + 3) {
            Some("20") => ' ',
            Some("25") => '%',
            _ => return Err(BAD),
        });
        rest = &rest[at + 3..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use stratum_core::severity::Severity;
    use stratum_core::violation::SourceLocation;

    fn finding(file: &str, line: u32) -> Finding {
        Finding {
            rule: "stratum/no-circular-deps".into(),
            severity: Severity::Error,
            message: "cycle".into(),
            file: file.into(),
            location: SourceLocation { line, column: 1 },
            edge: None,
            fix: "fix".into(),
            members: Vec::new(),
        }
    }

    fn with(rule: &str, file: &str, message: &str, members: &[&str]) -> Finding {
        Finding {
            rule: rule.into(),
            message: message.into(),
            members: members.iter().map(|&m| m.to_owned()).collect(),
            ..finding(file, 1)
        }
    }

    /// Covers every v2 field: a repeat, touches, custom slugs (bare, rooted, nested) and a space.
    fn varied_findings() -> Vec<Finding> {
        vec![
            finding("a.ts", 1),
            finding("a.ts", 2),
            with(
                "stratum/no-circular-deps",
                "b.ts",
                "Cycle of 3",
                &["b.ts", "c.ts", "d.ts"],
            ),
            with(
                "stratum/depth-ratio",
                "a.ts",
                "Depth ratio 0.33 below 3.0",
                &[],
            ),
            with("custom/no-lodash", "src/my dir/100%.ts", "Lodash", &[]),
            with("bare", "e.ts", "bare slug", &[]),
            with("/rooted", "e.ts", "rooted slug", &[]),
            with("stratum/a/b", "e.ts", "nested slug", &[]),
        ]
    }

    /// The version 1 file 0.4.0 wrote for `findings`: masked messages, `count` omitted when 1.
    fn v1_text(findings: &[Finding]) -> String {
        let mut entries = BTreeMap::<Fingerprint, (usize, BTreeSet<String>)>::new();
        for f in findings {
            let entry = entries.entry(Fingerprint::of(f)).or_default();
            entry.0 += 1;
            entry
                .1
                .extend(f.members.iter().filter(|m| **m != f.file).cloned());
        }
        let violations: Vec<_> = entries
            .into_iter()
            .map(|(fp, (count, touches))| {
                let mut v = serde_json::to_value(&fp).unwrap();
                if count != 1 {
                    v["count"] = count.into();
                }
                if !touches.is_empty() {
                    v["touches"] = serde_json::to_value(touches).unwrap();
                }
                v
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "version": 1, "violations": violations }))
            .unwrap()
    }

    fn origin() -> &'static Utf8Path {
        Utf8Path::new(BASELINE_FILE)
    }

    type At = Vec<(String, u32)>;

    fn outcome(m: &Matched) -> (At, At, [usize; 3]) {
        let at = |fs: &[Finding]| {
            fs.iter()
                .map(|f| (f.file.clone(), f.location.line))
                .collect()
        };
        let fixed = [m.fixed(), m.fixed_among(["c.ts"]), m.fixed_among(["a.ts"])];
        (at(&m.new), at(&m.baselined), fixed)
    }

    fn temp_baseline_path(dir: &tempfile::TempDir) -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(dir.path().join(BASELINE_FILE)).unwrap()
    }

    #[test]
    fn a_second_identical_violation_is_new_and_line_shifts_stay_baselined() {
        let base = Baseline::from_findings(&[finding("a.ts", 3)]);
        let m = base.apply(vec![finding("a.ts", 40), finding("a.ts", 41)]);
        assert_eq!(
            (m.baselined[0].location.line, m.new[0].location.line),
            (40, 41)
        );
        assert_eq!(m.fixed(), 0);
    }

    #[test]
    fn a_changed_metric_in_the_message_stays_baselined() {
        let with_message = |message: &str| Finding {
            message: message.into(),
            ..finding("a.ts", 1)
        };
        let base =
            Baseline::from_findings(&[with_message("Depth ratio 0.33 below 3.0 (limit 5).")]);
        let m = base.apply(vec![with_message("Depth ratio 0.25 below 3.0 (limit 5).")]);
        assert_eq!((m.baselined.len(), m.new.len()), (1, 0));
        assert_eq!(m.fixed(), 0);
    }

    #[test]
    fn a_v1_file_with_raw_numbers_still_matches_after_load() {
        let entry = serde_json::json!({
            "version": 1,
            "violations": [{
                "rule": "stratum/no-circular-deps",
                "file": "a.ts",
                "message": "Depth ratio 0.33 below 3.0 (limit 5).",
            }],
        });
        let m = Baseline::parse(&entry.to_string(), origin())
            .unwrap()
            .apply(vec![Finding {
                message: "Depth ratio 0.25 below 3.0 (limit 5).".into(),
                ..finding("a.ts", 1)
            }]);
        assert_eq!((m.baselined.len(), m.new.len()), (1, 0));
        assert_eq!(m.fixed(), 0);
    }

    #[test]
    fn a_v1_file_sums_counts_of_identical_entries_and_keeps_touches() {
        let text = serde_json::json!({
            "version": 1,
            "violations": [
                { "rule": "stratum/no-circular-deps", "file": "b.ts", "message": "cycle", "count": 2, "touches": ["c.ts"] },
                { "rule": "stratum/no-circular-deps", "file": "b.ts", "message": "cycle" },
            ],
        });
        let base = Baseline::parse(&text.to_string(), origin()).unwrap();
        assert_eq!(base.len(), 3);
        let m = base.apply(vec![finding("b.ts", 1)]);
        assert_eq!(
            (m.baselined.len(), m.fixed(), m.fixed_among(["c.ts"])),
            (1, 2, 2)
        );
        assert_eq!(m.fixed_among(["z.ts"]), 0);
    }

    #[test]
    fn save_writes_v2_one_entry_per_line_with_short_built_in_slugs() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        Baseline::from_findings(&varied_findings())
            .save(&path)
            .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let has = |line: &str| lines.iter().any(|l| l.trim_end_matches(',') == line);
        let (cycle, cycle_of) = (Digest::of("cycle"), Digest::of("Cycle of #"));
        let (c, d) = (Digest::of("c.ts"), Digest::of("d.ts"));
        let (first, second) = (c.min(d), c.max(d));

        assert_eq!(lines[0], "{\"version\":2,\"violations\":[");
        assert!(
            has(&format!("\"no-circular-deps a.ts {cycle} x2\"")),
            "{text}"
        );
        assert!(
            has(&format!(
                "\"no-circular-deps b.ts {cycle_of} {first} {second}\""
            )),
            "{text}"
        );
        assert!(
            text.contains("\"custom/no-lodash src/my%20dir/100%25.ts "),
            "{text}"
        );
        assert!(
            text.contains("\"/bare e.ts ") && text.contains("\"//rooted e.ts "),
            "{text}"
        );
        assert!(text.contains("\"stratum/a/b e.ts "), "{text}");
        assert_eq!(lines.last(), Some(&"]}"));
        assert_eq!(lines.len(), 2 + 7);
    }

    #[test]
    fn save_then_load_round_trips_every_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        let base = Baseline::from_findings(&varied_findings());
        base.save(&path).unwrap();
        assert_eq!(Baseline::load(&path).unwrap(), base);
    }

    #[test]
    fn an_empty_baseline_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        Baseline::default().save(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\"version\":2,\"violations\":[\n]}\n"
        );
        assert!(Baseline::load(&path).unwrap().is_empty());
    }

    #[test]
    fn v1_and_v2_of_the_same_violations_match_identically() {
        let accepted = varied_findings();
        let dir = tempfile::tempdir().unwrap();
        let path = temp_baseline_path(&dir);
        let v1 = Baseline::parse(&v1_text(&accepted), origin()).unwrap();
        v1.save(&path).unwrap();
        let v2 = Baseline::load(&path).unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v2, Baseline::from_findings(&accepted));

        let run = || {
            vec![
                finding("a.ts", 9),
                finding("a.ts", 10),
                finding("a.ts", 11),
                with(
                    "stratum/depth-ratio",
                    "a.ts",
                    "Depth ratio 0.5 below 3.0",
                    &[],
                ),
                with("custom/no-lodash", "src/my dir/100%.ts", "Lodash", &[]),
                with("stratum/no-circular-deps", "x.ts", "cycle", &[]),
            ]
        };
        let (m1, m2) = (v1.apply(run()), v2.apply(run()));
        assert_eq!(outcome(&m1), outcome(&m2));
        assert_eq!((m1.new.len(), m1.baselined.len()), (2, 4));
        assert_eq!(outcome(&m1).2, [4, 1, 0]);
    }

    #[test]
    fn a_v1_base_and_a_v2_head_compare_by_the_same_key() {
        let accepted = varied_findings();
        let v1 = Baseline::parse(&v1_text(&accepted), origin()).unwrap();
        assert!(
            Baseline::from_findings(&accepted)
                .grown_since(&v1)
                .is_empty()
        );

        let mut grown = accepted.clone();
        grown.push(finding("a.ts", 3));
        let growth = Baseline::from_findings(&grown).grown_since(&v1);
        assert_eq!(growth.len(), 1);
        assert_eq!((growth[0].base, growth[0].head), (2, 3));
    }

    #[test]
    fn a_v2_entry_with_a_bad_digest_count_or_escape_is_rejected() {
        for line in [
            "no-circular-deps a.ts nothex00",
            "no-circular-deps a.ts",
            "no-circular-deps a.ts 0000000a xtwo",
            "no-circular-deps a.ts 0000000a 123",
            "no-circular-deps a%2.ts 0000000a",
        ] {
            let text = serde_json::json!({ "version": 2, "violations": [line] }).to_string();
            let err = Baseline::parse(&text, origin()).unwrap_err();
            assert!(
                matches!(err, BaselineError::Entry { index: 0, .. }),
                "{line}: {err}"
            );
        }
    }

    #[test]
    fn an_unknown_version_is_rejected() {
        let err = Baseline::parse("{\"version\":3,\"violations\":[]}", origin()).unwrap_err();
        assert!(
            matches!(err, BaselineError::Version { found: 3, .. }),
            "{err}"
        );
    }

    #[test]
    fn prune_never_grows_the_baseline() {
        let base = Baseline::from_findings(&[finding("a.ts", 1), finding("b.ts", 1)]);
        let pruned = base.pruned(&[finding("a.ts", 1), finding("a.ts", 2), finding("c.ts", 1)]);
        assert_eq!(pruned, Baseline::from_findings(&[finding("a.ts", 1)]));
    }

    #[test]
    fn prune_of_a_v1_baseline_lowers_counts_and_keeps_touches() {
        let accepted = varied_findings();
        let v1 = Baseline::parse(&v1_text(&accepted), origin()).unwrap();
        let still = [accepted[0].clone(), accepted[2].clone()];
        let pruned = v1.pruned(&still);
        assert_eq!(pruned, Baseline::from_findings(&still));
        assert_eq!(pruned.len(), 2);
    }
}
