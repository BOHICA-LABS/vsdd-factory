// Shared fixtures for the S-25.10 red tests. This file is `#[path]`-included by each
// `s2510_*` test crate, so any item a given crate does not use is dead code there.
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.10 test support: an INDEPENDENT reference encoder / parser of the ADR-054
//! Decision 1 wire format (so black-box tests do not depend on the module under test),
//! the verbatim golden vector, and the COMMITTING-coordinator fixture.
//!
//! This module deliberately imports NOTHING from `factory_dispatcher`'s not-yet-existing
//! `shard_manager::intent_log` module: files that only need the reference oracle and the
//! real binary must keep compiling so that they fail by ASSERTION (a genuine Red), not by
//! a missing import. The module-API conversions live in `s2510_support/api.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

// ---------------------------------------------------------------------------
// ADR-054 Decision 1.4 GOLDEN RECORD (normative test vector), verbatim.
//
// Source: ADR-054 v1.0 Decision 1.4 "Golden record (normative test vector)". The INTENT
// bytes are the fenced block of that section; the DONE bytes are "the same bytes except
// `fencing_generation=2`, `record_type=DONE`, `timestamp_utc=2026-10-08T12:00:07Z` and
// `record_checksum=3382c172...`". Both checksums were INDEPENDENTLY recomputed with
// `sed -n 1,9p <record> | shasum -a 256` (ADR-054 Decision 1.5 recipe) and equal the ADR's.
// ---------------------------------------------------------------------------

pub const GOLDEN_TXN_ID: &str = "0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90";
pub const GOLDEN_TARGET: &str = "/proj/.factory/specs/behavioral-contracts/BC-INDEX.md";
pub const GOLDEN_STAGING: &str =
    "/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f/BC-INDEX.md";
pub const GOLDEN_POST_HASH: &str =
    "dd2ce8a1000fd7b33e333408ed7dc137d3c8358b5602dcc14bf9919ea00fe2bc"; // sha256("lean-body")
pub const GOLDEN_PRE_HASH: &str =
    "405dc7565658c749a2775f62875bc67d7892c97795a43438ea5936ef078b72a4"; // sha256("monolith")
pub const GOLDEN_INTENT_CHECKSUM: &str =
    "2767fd1de24bdaf614a8eb417d3d5db14b5ff165940a32579944944d6ce9e02b";
pub const GOLDEN_DONE_CHECKSUM: &str =
    "3382c172c4b08aef623fcc04469e6cdca6eea1281c6b90415ba18919f3aac5c1";

pub const GOLDEN_INTENT: &str = "\
INTENT_LOG_RECORD_V1
txn_id=0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90
fencing_generation=1
record_type=INTENT
target_canonical=/proj/.factory/specs/behavioral-contracts/BC-INDEX.md
staging_path=/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f/BC-INDEX.md
expected_post_hash=dd2ce8a1000fd7b33e333408ed7dc137d3c8358b5602dcc14bf9919ea00fe2bc
expected_pre_state=405dc7565658c749a2775f62875bc67d7892c97795a43438ea5936ef078b72a4
timestamp_utc=2026-10-08T12:00:00Z
record_checksum=2767fd1de24bdaf614a8eb417d3d5db14b5ff165940a32579944944d6ce9e02b
END_INTENT_LOG_RECORD
";

pub const GOLDEN_DONE: &str = "\
INTENT_LOG_RECORD_V1
txn_id=0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90
fencing_generation=2
record_type=DONE
target_canonical=/proj/.factory/specs/behavioral-contracts/BC-INDEX.md
staging_path=/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f/BC-INDEX.md
expected_post_hash=dd2ce8a1000fd7b33e333408ed7dc137d3c8358b5602dcc14bf9919ea00fe2bc
expected_pre_state=405dc7565658c749a2775f62875bc67d7892c97795a43438ea5936ef078b72a4
timestamp_utc=2026-10-08T12:00:07Z
record_checksum=3382c172c4b08aef623fcc04469e6cdca6eea1281c6b90415ba18919f3aac5c1
END_INTENT_LOG_RECORD
";

// ---------------------------------------------------------------------------
// Reference encoder (independent oracle)
// ---------------------------------------------------------------------------

/// A logical record, as text. `pre` is `"MISSING"` or 64 lowercase hex digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub txn_id: String,
    pub fencing: u64,
    pub rtype: String,
    pub target: String,
    pub staging: String,
    pub post: String,
    pub pre: String,
    pub ts: String,
}

impl Rec {
    pub fn golden_intent() -> Rec {
        Rec {
            txn_id: GOLDEN_TXN_ID.into(),
            fencing: 1,
            rtype: "INTENT".into(),
            target: GOLDEN_TARGET.into(),
            staging: GOLDEN_STAGING.into(),
            post: GOLDEN_POST_HASH.into(),
            pre: GOLDEN_PRE_HASH.into(),
            ts: "2026-10-08T12:00:00Z".into(),
        }
    }

    pub fn golden_done() -> Rec {
        Rec {
            fencing: 2,
            rtype: "DONE".into(),
            ts: "2026-10-08T12:00:07Z".into(),
            ..Rec::golden_intent()
        }
    }

    /// A valid record for logical target `i` under txn `txn` (distinct paths and hashes
    /// per `i`).
    pub fn sample(txn: &str, i: usize, rtype: &str, fencing: u64) -> Rec {
        Rec {
            txn_id: txn.into(),
            fencing,
            rtype: rtype.into(),
            target: format!("/proj/.factory/specs/behavioral-contracts/shards/T{i}.md"),
            staging: format!("/proj/.factory/migration-state/gen-1/shards/T{i}.md"),
            post: sha256_hex(format!("post-{i}").as_bytes()),
            pre: if i.is_multiple_of(2) {
                "MISSING".into()
            } else {
                sha256_hex(format!("pre-{i}").as_bytes())
            },
            ts: format!("2026-10-08T12:00:{:02}Z", i % 60),
        }
    }

    /// The same logical target as `self` but as a `DONE` at `fencing`.
    pub fn done_of(&self, fencing: u64, ts: &str) -> Rec {
        Rec {
            rtype: "DONE".into(),
            fencing,
            ts: ts.into(),
            ..self.clone()
        }
    }
}

/// The nine checksummed lines of `r` (each WITHOUT its LF).
pub fn nine_lines(r: &Rec) -> Vec<Vec<u8>> {
    [
        "INTENT_LOG_RECORD_V1".to_string(),
        format!("txn_id={}", r.txn_id),
        format!("fencing_generation={}", r.fencing),
        format!("record_type={}", r.rtype),
        format!("target_canonical={}", r.target),
        format!("staging_path={}", r.staging),
        format!("expected_post_hash={}", r.post),
        format!("expected_pre_state={}", r.pre),
        format!("timestamp_utc={}", r.ts),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect()
}

/// Assemble a record from `nine` arbitrary line byte-strings: the checksum is computed
/// over EXACTLY the bytes written (each line + LF), so a grammar-violating line still
/// carries a checksum that matches its own bytes (only the grammar can reject it).
pub fn assemble(nine: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for l in nine {
        out.extend_from_slice(l);
        out.push(b'\n');
    }
    let sum = sha256_hex(&out);
    out.extend_from_slice(format!("record_checksum={sum}\nEND_INTENT_LOG_RECORD\n").as_bytes());
    out
}

pub fn encode(r: &Rec) -> Vec<u8> {
    assemble(&nine_lines(r))
}

pub fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.iter().flatten().copied().collect()
}

/// Cumulative end offsets of consecutive encoded records.
pub fn ends(parts: &[Vec<u8>]) -> Vec<usize> {
    let mut acc = 0;
    parts
        .iter()
        .map(|p| {
            acc += p.len();
            acc
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Reference parser (independent oracle): longest valid prefix of strict records.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub rec: Rec,
    pub start: usize,
}

/// Parse the longest run of valid 11-line records from offset 0. Returns the records and
/// the length of the valid prefix.
pub fn parse_prefix(bytes: &[u8]) -> (Vec<Parsed>, usize) {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off < bytes.len() {
        match parse_one(&bytes[off..]) {
            Some((rec, used)) => {
                out.push(Parsed { rec, start: off });
                off += used;
            }
            None => break,
        }
    }
    (out, off)
}

fn parse_one(b: &[u8]) -> Option<(Rec, usize)> {
    let mut lines: Vec<&[u8]> = Vec::new();
    let mut pos = 0usize;
    for _ in 0..11 {
        let rel = b[pos..].iter().position(|&c| c == b'\n')?;
        lines.push(&b[pos..pos + rel]);
        pos += rel + 1;
    }
    let text: Vec<&str> = lines
        .iter()
        .map(|l| std::str::from_utf8(l).ok())
        .collect::<Option<Vec<_>>>()?;
    if text[0] != "INTENT_LOG_RECORD_V1" || text[10] != "END_INTENT_LOG_RECORD" {
        return None;
    }
    let keys = [
        "txn_id",
        "fencing_generation",
        "record_type",
        "target_canonical",
        "staging_path",
        "expected_post_hash",
        "expected_pre_state",
        "timestamp_utc",
        "record_checksum",
    ];
    let mut vals = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        let prefix = format!("{key}=");
        vals.push(text[i + 1].strip_prefix(prefix.as_str())?.to_string());
    }
    let nine_len: usize = lines[..9].iter().map(|l| l.len() + 1).sum();
    if sha256_hex(&b[..nine_len]) != vals[8] {
        return None;
    }
    Some((
        Rec {
            txn_id: vals[0].clone(),
            fencing: vals[1].parse().ok()?,
            rtype: vals[2].clone(),
            target: vals[3].clone(),
            staging: vals[4].clone(),
            post: vals[5].clone(),
            pre: vals[6].clone(),
            ts: vals[7].clone(),
        },
        pos,
    ))
}

// ---------------------------------------------------------------------------
// Process helpers (stdin always null, per-command timeout always enforced)
// ---------------------------------------------------------------------------

pub fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_factory-dispatcher"))
}

pub fn finish(mut child: Child, secs: u64) -> Output {
    let start = Instant::now();
    let timeout = Duration::from_secs(secs);
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return child.wait_with_output().expect("wait_with_output");
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not exit within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Run `factory-dispatcher migrate-bc-index` against `root` (stdin null, 60 s timeout).
pub fn run_migrate(root: &Path) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.arg("migrate-bc-index")
        .env("CLAUDE_PROJECT_DIR", root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    finish(cmd.spawn().expect("spawn factory-dispatcher"), 60)
}

pub fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// Whole-tree snapshot: relative path -> `None` (directory) / `Some(bytes)`.
pub fn snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                out.insert(format!("{rel}/"), None);
                walk(base, &p, out);
            } else {
                out.insert(rel, Some(std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

pub fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// ADR-054 Decision 3.1 normative stderr lines (`migrate-bc-index: ` + Display)
// ---------------------------------------------------------------------------

pub fn corrupt_line(log_file: &str, kind: &str, offset: usize) -> String {
    format!(
        "migrate-bc-index: INTENT_LOG_CORRUPT (exit 2): intent log {log_file} is corrupt \
         ({kind}) at byte offset {offset}; no further move or append was made and no \
         completion was recorded; operator investigation required"
    )
}

pub fn value_rejected_line(field: &str, reason: &str) -> String {
    format!(
        "migrate-bc-index: INTENT_LOG_VALUE_REJECTED (exit 2): intent-log field {field} \
         rejected: {reason}; no record of this batch was appended"
    )
}

// ---------------------------------------------------------------------------
// COMMITTING-coordinator fixture
//
// A crashed `migrate-bc-index` whose txn is COMMITTING over a fixed plan of THREE moves:
//   T0 shards/BC-INDEX-SS-01.md  canonical ABSENT  (INTENT pre-state MISSING)
//   T1 shards/BC-INDEX-SS-02.md  canonical ABSENT  (INTENT pre-state MISSING)
//   T2 BC-INDEX.md               canonical PRESENT (INTENT pre-state = sha256(old body))
// The test then writes whatever intent log bytes it wants and runs the real binary: the
// forward-recovery arm reads that log first (this story: `read_log`), so a corrupt log
// must stop it before anything moves.
// ---------------------------------------------------------------------------

pub const TXN: &str = "txn-act-1";
/// The txn's `generation_id`; the staged generation directory is `gen-<GEN>` and the intent
/// log `intent-<GEN>.log` (ADR-052 Decision 7b/7c).
pub const GEN: &str = "g1";
pub const LOG_FILE: &str = "intent-g1.log";
pub const OLD_BC_INDEX: &[u8] = b"monolith\n";

pub struct Cx {
    pub dir: tempfile::TempDir,
}

pub struct Move {
    pub staging: PathBuf,
    pub canonical: PathBuf,
    pub staged: Vec<u8>,
    pub pre: Option<Vec<u8>>,
}

impl Cx {
    pub fn new() -> Cx {
        let cx = Cx {
            dir: tempfile::tempdir().expect("project tempdir"),
        };
        std::fs::create_dir_all(cx.ms().join("reservations")).unwrap();
        std::fs::write(cx.ms().join("exclusive.lock"), b"").unwrap();
        // A crashed coordinator leaves the gate DRAINING/LOCKED.
        std::fs::write(cx.ms().join("gate-state.json"), "\"LOCKED\"").unwrap();
        for m in cx.moves() {
            std::fs::create_dir_all(m.staging.parent().unwrap()).unwrap();
            std::fs::create_dir_all(m.canonical.parent().unwrap()).unwrap();
            std::fs::write(&m.staging, &m.staged).unwrap();
            if let Some(pre) = &m.pre {
                std::fs::write(&m.canonical, pre).unwrap();
            }
        }
        std::fs::write(
            cx.ms().join("txn-act-1.json"),
            serde_json::to_vec_pretty(&cx.txn_json()).unwrap(),
        )
        .unwrap();
        cx
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn ms(&self) -> PathBuf {
        self.root().join(".factory/migration-state")
    }

    pub fn log_path(&self) -> PathBuf {
        self.ms().join(LOG_FILE)
    }

    pub fn moves(&self) -> Vec<Move> {
        let gen_dir = self.ms().join(format!("gen-{GEN}"));
        let canon = self.root().join(".factory/specs/behavioral-contracts");
        vec![
            Move {
                staging: gen_dir.join("shards/BC-INDEX-SS-01.md"),
                canonical: canon.join("shards/BC-INDEX-SS-01.md"),
                staged: b"alpha-shard\n".to_vec(),
                pre: None,
            },
            Move {
                staging: gen_dir.join("shards/BC-INDEX-SS-02.md"),
                canonical: canon.join("shards/BC-INDEX-SS-02.md"),
                staged: b"beta-shard\n".to_vec(),
                pre: None,
            },
            Move {
                staging: gen_dir.join("BC-INDEX.md"),
                canonical: canon.join("BC-INDEX.md"),
                staged: b"lean-body\n".to_vec(),
                pre: Some(OLD_BC_INDEX.to_vec()),
            },
        ]
    }

    pub fn txn_json(&self) -> Value {
        let pending: Vec<Value> = self
            .moves()
            .iter()
            .map(|m| {
                json!({
                    "staging_path": m.staging.to_string_lossy(),
                    "canonical_path": m.canonical.to_string_lossy(),
                })
            })
            .collect();
        json!({
            "schema_version": 1,
            "txn_id": TXN,
            "migration_id": "migrate-bc-index",
            "activation_id": "act-1",
            "fencing_generation": 1,
            "state": "COMMITTING",
            "generation_id": GEN,
            "source_sha256": null,
            "source_body_row_sha256": null,
            "intent_log_path": format!(".factory/migration-state/{LOG_FILE}"),
            "pending_canonical_moves": pending,
            "created_at": "2026-10-07T00:00:00Z",
            "updated_at": "2026-10-07T00:00:00Z",
        })
    }

    /// The three well-formed INTENT records for the plan (fencing 1, live txn id).
    pub fn intents(&self) -> Vec<Rec> {
        self.moves()
            .iter()
            .enumerate()
            .map(|(i, m)| Rec {
                txn_id: TXN.into(),
                fencing: 1,
                rtype: "INTENT".into(),
                target: m.canonical.to_string_lossy().into_owned(),
                staging: m.staging.to_string_lossy().into_owned(),
                post: sha256_hex(&m.staged),
                pre: m
                    .pre
                    .as_ref()
                    .map_or_else(|| "MISSING".to_string(), |b| sha256_hex(b)),
                ts: format!("2026-10-08T12:00:0{i}Z"),
            })
            .collect()
    }

    pub fn write_log(&self, bytes: &[u8]) {
        std::fs::write(self.log_path(), bytes).unwrap();
    }

    pub fn run(&self) -> Output {
        run_migrate(self.root())
    }

    pub fn snapshot(&self) -> BTreeMap<String, Option<Vec<u8>>> {
        snapshot(self.root())
    }
}

/// Names of the entries of the snapshot that differ between `before` and `after`.
pub fn changed(
    before: &BTreeMap<String, Option<Vec<u8>>>,
    after: &BTreeMap<String, Option<Vec<u8>>>,
) -> Vec<String> {
    before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

// ---------------------------------------------------------------------------
// Fresh-run fixture (the one the S-25.09 txn_id tests use): a two-subsystem BC-INDEX.md with
// its shard config. A full `migrate-bc-index` run over it has FOUR moves: the SS-01 shard,
// the SS-02 shard and the top-level shard manifest (all canonical targets ABSENT at INTENT
// time) and the lean BC-INDEX.md body (canonical target PRESENT at INTENT time).
// ---------------------------------------------------------------------------

pub const SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100000
shape = \"flat\"
";

pub const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 2
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |
| SS-02 Shard Manager | BC-2 | 1 | ss-02/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |

### SS-02 — Shard Manager (BC-2) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-2.01.001](ss-02/BC-2.01.001.md) | Shard manager splits oversized index | draft | CAP-TBD | S-25.02 |
";

pub fn setup_fresh_fixture(root: &Path) {
    let factory_dir = root.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), SHARD_CONFIG).unwrap();
    let canonical = root.join(".factory/specs/behavioral-contracts/BC-INDEX.md");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(&canonical, ORIGINAL_CONTENT).unwrap();
}

pub fn canonical_bc_index(root: &Path) -> PathBuf {
    root.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

/// The txn record (the single `txn-*.json`) as raw JSON.
pub fn read_txn(ms: &Path) -> Value {
    let mut found: Vec<PathBuf> = std::fs::read_dir(ms)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            n.starts_with("txn-") && n.ends_with(".json")
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one txn record, got {found:?}"
    );
    serde_json::from_slice(&std::fs::read(found.pop().unwrap()).unwrap()).unwrap()
}

// ---------------------------------------------------------------------------
// Source-scan helpers (comments and string contents blanked, `#[cfg(test)]` items removed).
// ---------------------------------------------------------------------------

/// Blank comments and the CONTENTS of string / raw-string / char literals; keep everything
/// else (so braces inside them cannot confuse a brace matcher).
pub fn blank_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let prev_ident = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == 'r' && !prev_ident && matches!(chars.get(i + 1), Some('"') | Some('#')) {
            // raw string r#"..."#
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                j += 1;
                loop {
                    if j >= chars.len() {
                        break;
                    }
                    if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                out.push_str("\"\"");
                i = j;
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '"' {
            out.push('"');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push('"');
            i += 1;
        } else if c == '\'' {
            // char literal ('x' or an escape) vs lifetime ('a)
            if chars.get(i + 1) == Some(&'\\') {
                let mut j = i + 2;
                while j < chars.len() && chars[j] != '\'' && j < i + 12 {
                    j += 1;
                }
                out.push_str("''");
                i = j + 1;
            } else if chars.get(i + 2) == Some(&'\'') {
                out.push_str("''");
                i += 3;
            } else {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Remove every `#[cfg(test)]` item (the attribute, any following attributes, and the item's
/// `{ ... }` block or `;`) from already-blanked text.
pub fn strip_cfg_test_items(blanked: &str) -> String {
    let mut out = String::new();
    let mut rest = blanked;
    while let Some(pos) = rest.find("#[cfg(test)]") {
        out.push_str(&rest[..pos]);
        let mut after = &rest[pos + "#[cfg(test)]".len()..];
        // skip following attributes
        loop {
            let t = after.trim_start();
            if t.starts_with("#[") {
                let mut depth = 0i32;
                let mut end = 0;
                for (i, c) in t.char_indices() {
                    match c {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                end = i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                after = &t[end..];
            } else {
                after = t;
                break;
            }
        }
        // the item: up to its first `;` or balanced `{ }` block, whichever comes first
        let semi = after.find(';');
        let brace = after.find('{');
        let end = match (semi, brace) {
            (Some(s), Some(b)) if s < b => s + 1,
            (Some(s), None) => s + 1,
            (_, Some(b)) => {
                let mut depth = 0i32;
                let mut e = after.len();
                for (i, c) in after[b..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                e = b + i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                e
            }
            (None, None) => after.len(),
        };
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// `src/<rel>` of this crate as BLANKED production source (comments/strings neutralised,
/// `#[cfg(test)]` items removed).
pub fn production_source(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    strip_cfg_test_items(&blank_comments_and_strings(&raw))
}

/// The brace-matched body (including the braces) of the first `{ ... }` after `header` in
/// already-blanked text.
pub fn fn_body(blanked: &str, header: &str) -> String {
    let start = blanked
        .find(header)
        .unwrap_or_else(|| panic!("`{header}` not found in the production source"));
    let open = start + blanked[start..].find('{').expect("a body after the header");
    let mut depth = 0usize;
    for (i, c) in blanked[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return blanked[open..open + i + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces after `{header}`");
}
