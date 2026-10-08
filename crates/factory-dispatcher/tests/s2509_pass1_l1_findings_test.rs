// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 LOCAL adversary pass-1 (cycle L1) red tests: F-S2509-L1-003 / 004 / 005 / 006 /
//! 007(b) / 009 / 010 / 014 (default features; the failpoint-gated half -- 007(a) and 012 --
//! lives in `s2509_pass1_l1_failpoints_test.rs`).
//!
//! Authority: ADR-052 v1.25 items 8, 11(c), 11(d), 11(f); BC-1.18.011 v1.21 Precondition 6(f)(ii)
//! (1a), 6(f)(iii); BC-1.18.013 v1.13 Postcondition 10 (incl. 10(c) and the "Coordinators"
//! clause); BC-3.08.001 v1.37 Events 12 and 13 ("truncated to 64 characters for any
//! data-derived substring").
//!
//! | Finding | Test |
//! |---------|------|
//! | F-003 | `..._F003_drain_timeout_gate_open_write_failure_...` |
//! | F-004 (i)(ii) | `..._F004_dangling_symlink_completed_json_...`, `..._F004_eloop_completed_json_...`, `..._F004_directory_...`, `..._F004_unreadable_eacces_...`, `..._F004_completed_json_that_is_not_a_record_...`, controls `..._F004_control_...`, `..._F004_valid_completed_json_control_...`, `..._F004_symlink_to_a_valid_record_...` |
//! | F-004 reader parity | `..._F004_reader_parity_dangling_symlink_...`, `..._F004_reader_parity_non_record_...`, `..._F004_reader_parity_valid_and_absent_controls` |
//! | F-004 (iii) | `..._F004_recover_receives_the_under_lock_read_...` (source gate + controls) |
//! | F-005 | `..._F005_admitter_own_release_failure_...`, `..._F005_main_rs_release_on_block_failure_...` |
//! | F-006 | `..._F006_gate_open_reset_failed_advisory_...`, `..._F006_terminal_txn_archive_failed_advisory_...`, `..._F006_staging_dir_remove_failed_advisory_...`, `..._F006_oversized_row_subshard_advisory_...` (canonical-move halt reasons moved to S-25.11 AC-015) |
//! | F-007(b) | `..._F007_intent_log_path_exact_string_variants_...` |
//! | F-009 | `..._F009_old_entry_point_names_absent_source_gate` (+ positive / negative controls) |
//! | F-010 | `..._F010_event_12_13_detail_data_derived_substrings_truncated_to_64_...` |
//! | F-014 | `..._F014_read_active_txn_record_does_not_strict_decode_terminal_...` |
//!
//! Every spawned child has stdin set to null (coordinator) or written-then-closed (hook
//! dispatch) and a per-command timeout.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use factory_dispatcher::shard_manager::migration_fs::StdFs;
use factory_dispatcher::shard_manager::{
    BcIndexAddressingError, BcIndexMigrationReadState, MigrationLockGuard,
    detect_migration_read_state, read_active_txn_record, try_acquire_migration_lock,
};
use serde_json::{Value, json};

const MIGRATION_ID: &str = "migrate-bc-index";

const SHARD_CONFIG: &str = "\
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

const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 1
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |
";

// ---------------------------------------------------------------------------
// Shared harness
// ---------------------------------------------------------------------------

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_factory-dispatcher"))
}

fn finish(mut child: Child, timeout: Duration) -> Output {
    let start = Instant::now();
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

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// lstat-based snapshot of a whole tree (symlinks recorded, never followed).
fn raw_tree(root: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            let md = std::fs::symlink_metadata(&p).unwrap();
            if md.file_type().is_symlink() {
                out.insert(
                    rel,
                    format!("symlink->{}", std::fs::read_link(&p).unwrap().display()),
                );
            } else if md.is_dir() {
                out.insert(format!("{rel}/"), "dir".to_string());
                walk(base, &p, out);
            } else {
                out.insert(
                    rel,
                    format!(
                        "file:{}",
                        String::from_utf8_lossy(&std::fs::read(&p).unwrap_or_default())
                    ),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// A real `migrate-bc-index` coordinator fixture: `.factory/shard-config.toml`, a flat
/// BC-INDEX and an (optionally populated) `.factory/migration-state/`.
struct Coord {
    dir: tempfile::TempDir,
    /// A nested (long) project root under `dir`, when the fixture needs a long path.
    sub: Option<PathBuf>,
}

impl Coord {
    fn new() -> Self {
        Self::make(None)
    }
    /// A fixture whose project root is `segments` nested directories of `seg_len`
    /// characters each, under the CANONICAL tempdir (so every spelling agrees).
    fn new_long(seg_len: usize, segments: usize) -> Self {
        Self::make(Some((seg_len, segments)))
    }
    fn make(long: Option<(usize, usize)>) -> Self {
        let dir = tempfile::tempdir().expect("project tempdir");
        let sub = long.map(|(seg_len, segments)| {
            let mut root = dir.path().canonicalize().unwrap();
            for i in 0..segments {
                let ch = char::from(b'a' + u8::try_from(i % 26).unwrap());
                root = root.join(ch.to_string().repeat(seg_len));
            }
            std::fs::create_dir_all(&root).unwrap();
            root
        });
        let c = Coord { dir, sub };
        std::fs::create_dir_all(c.root().join(".factory")).unwrap();
        std::fs::write(c.root().join(".factory/shard-config.toml"), SHARD_CONFIG).unwrap();
        let canon = c.canonical();
        std::fs::create_dir_all(canon.parent().unwrap()).unwrap();
        std::fs::write(&canon, ORIGINAL_CONTENT).unwrap();
        std::fs::create_dir_all(c.ms().join("reservations")).unwrap();
        std::fs::write(c.ms().join("exclusive.lock"), b"").unwrap();
        std::fs::write(c.ms().join("gate-state.json"), "\"OPEN\"").unwrap();
        c
    }
    fn root(&self) -> &Path {
        self.sub.as_deref().unwrap_or_else(|| self.dir.path())
    }
    fn ms(&self) -> PathBuf {
        self.root().join(".factory/migration-state")
    }
    fn canonical(&self) -> PathBuf {
        self.root()
            .join(".factory/specs/behavioral-contracts/BC-INDEX.md")
    }
    fn spawn(&self) -> Child {
        let mut cmd = Command::new(binary_path());
        cmd.arg("migrate-bc-index")
            .env("CLAUDE_PROJECT_DIR", self.root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.spawn().expect("spawn factory-dispatcher")
    }
    fn run(&self) -> Output {
        finish(self.spawn(), Duration::from_secs(60))
    }
    fn txn_files(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.ms())
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with("txn-"))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }
}

// ===========================================================================
// F-003 -- drain-timeout branch: a failed gate-OPEN write is surfaced, never claimed
// ===========================================================================

/// ADR-052 v1.24 item 11(d) "Rule for every abort-path cleanup": the first cleanup
/// failure "is returned as that write's `Io` (exit 2) with the ORIGINAL failure's code
/// token named in its `detail`"; the abort-path cleanup covers EVERY site that opens the
/// gate after a failure (the drain-timeout branch of `run_bc_index_migration_core` is one:
/// it writes gate OPEN after `DrainTimeoutAbort`). The `DrainTimeoutAbort` Display says
/// "gate returned to OPEN" -- which must therefore never be printed when that write FAILED
/// (item 8(b)'s principle: the printed claims must be true).
///
/// Black-box: a live reservation holds the drain open for the 30 s production timeout; the
/// test waits for the DRAINING gate write, then replaces `gate-state.json` by a directory
/// (root-safe; every later gate write fails) so the gate-OPEN write after the timeout
/// fails. Expected: exit 2, ONE stderr line that is the gate write's own `Io` text naming
/// `gate-state.json` and the original token `DRAIN_TIMEOUT_ABORT`, and NEVER the phrase
/// "returned to OPEN".
#[test]
fn test_BC_1_18_011_F003_drain_timeout_gate_open_write_failure_is_io_exit_2_naming_token_never_returned_to_open_blackbox()
 {
    let c = Coord::new();
    let reservation = json!({
        "created_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "tool_use_id": "toolu_f003",
    });
    std::fs::write(
        c.ms().join("reservations/toolu_f003.reservation"),
        serde_json::to_vec(&reservation).unwrap(),
    )
    .unwrap();
    let gate = c.ms().join("gate-state.json");
    let child = c.spawn();
    // Wait for the coordinator's DRAINING write, then sabotage the gate file.
    let start = Instant::now();
    loop {
        if std::fs::read_to_string(&gate)
            .map(|s| s.contains("DRAINING"))
            .unwrap_or(false)
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "setup: the coordinator never wrote the DRAINING gate"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::remove_file(&gate).unwrap();
    std::fs::create_dir(&gate).unwrap();
    let out = finish(child, Duration::from_secs(90));
    let err = stderr_of(&out);

    let mut failures = Vec::new();
    let io_prefix = "migrate-bc-index: BC-INDEX migration: I/O error at ";
    if out.status.code() != Some(2) || !out.stdout.is_empty() {
        failures.push(format!(
            "expected exit 2 with empty stdout; got {:?} / {} bytes",
            out.status.code(),
            out.stdout.len()
        ));
    }
    if err.contains("returned to OPEN") {
        failures.push(format!(
            "the gate-OPEN write FAILED, so the printed claim \"gate returned to OPEN\" is false \
             and must not appear; stderr {err:?}"
        ));
    }
    if err.lines().count() != 1 || !err.starts_with(io_prefix) || !err.contains("gate-state.json") {
        failures.push(format!(
            "expected ONE `{io_prefix}...gate-state.json...` line (the failed write's own Io); \
             stderr {err:?}"
        ));
    }
    if !err.contains("DRAIN_TIMEOUT_ABORT") {
        failures.push(format!(
            "item 11(d): the Io must name the ORIGINAL failure's code token DRAIN_TIMEOUT_ABORT \
             so the root cause stays visible; stderr {err:?}"
        ));
    }
    assert_no_failures(
        "test_BC_1_18_011_F003_drain_timeout_gate_open_write_failure_is_io_exit_2_naming_token_never_returned_to_open_blackbox",
        failures,
    );
}

// ===========================================================================
// F-004 -- completed.json existence is an lstat-class probe under the lock
// ===========================================================================

/// Control for F-004: the SAME fixture without any `completed.json` runs a fresh
/// migration to completion (exit 0, one COMPLETED txn, `completed.json` written). Without
/// this the "no fresh run" assertions below could pass because the fixture can never run.
#[test]
fn test_BC_1_18_011_F004_control_fixture_without_completed_json_runs_fresh_to_completion_blackbox()
{
    let c = Coord::new();
    let out = c.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "control: a clean fixture must migrate; stderr {:?}",
        stderr_of(&out)
    );
    assert!(
        c.ms().join("completed.json").is_file(),
        "control: completed.json must be written by the fresh run"
    );
    assert_eq!(c.txn_files().len(), 1, "control: one txn record expected");
}

/// Snapshot-compare helper for F-004: nothing in the project tree changed and no fresh
/// run left evidence (txn record, generation dir, CURRENT.json).
fn assert_no_fresh_run(
    label: &str,
    c: &Coord,
    before: &BTreeMap<String, String>,
    failures: &mut Vec<String>,
) {
    let after = raw_tree(c.root());
    if &after != before {
        let changed: Vec<&String> = before
            .keys()
            .chain(after.keys())
            .filter(|k| before.get(*k) != after.get(*k))
            .collect();
        failures.push(format!(
            "[{label}] the tree must be byte-identical (a fresh run started on a tree whose \
             completion marker could not be read as ABSENT); changed: {changed:?}"
        ));
    }
    if !c.txn_files().is_empty() || c.ms().join("CURRENT.json").exists() {
        failures.push(format!(
            "[{label}] a fresh run left a txn record / CURRENT.json"
        ));
    }
}

const COMPLETED_IO_PREFIX: &str = "migrate-bc-index: BC-INDEX migration: I/O error at ";

/// The one-line verdict ADR-052 v1.25 item 11(c) rulings 2(a)-(c) give an unusable
/// `completed.json`: exit 2, empty stdout, exactly ONE stderr line
/// `migrate-bc-index: BC-INDEX migration: I/O error at <path>: <os error>` naming
/// `completed.json`, and a tree byte-identical to the fixture (no fresh run, no repair,
/// no gate flip). Failures are appended to `failures`, tagged with `label`.
fn assert_completed_json_io_exit_2(
    label: &str,
    c: &Coord,
    before: &BTreeMap<String, String>,
    out: &Output,
    failures: &mut Vec<String>,
) {
    let err = stderr_of(out);
    if out.status.code() != Some(2) || !out.stdout.is_empty() {
        failures.push(format!(
            "[{label}] expected exit 2 with empty stdout; got {:?} / {} stdout bytes, stderr {err:?}",
            out.status.code(),
            out.stdout.len()
        ));
    }
    if err.lines().count() != 1 || !err.starts_with(COMPLETED_IO_PREFIX) {
        failures.push(format!(
            "[{label}] expected ONE `{COMPLETED_IO_PREFIX}<path>: <os error>` line; stderr {err:?}"
        ));
    } else {
        let rest = &err[COMPLETED_IO_PREFIX.len()..];
        let path_part = rest.split(": ").next().unwrap_or("");
        if !path_part.ends_with("completed.json") || rest.trim_end().ends_with(": ") {
            failures.push(format!(
                "[{label}] the Io line must name `.../completed.json` and carry an os error text; \
                 stderr {err:?}"
            ));
        }
    }
    assert_no_fresh_run(label, c, before, failures);
}

/// ADR-052 v1.25 item 11(c) ruling 1: "Absence is a fact about the DIRECTORY ENTRY, never
/// about the link target. The probe is `symlink_metadata` ... ONLY `NotFound` from that call
/// means absent." Ruling 2(a): a DANGLING SYMLINK (lstat sees an entry; the read returns
/// `NotFound` of the TARGET) is `Io { path: completed.json }`, exit 2, ONE stderr line --
/// "NOT absent (a fresh run), and NOT exit 0 `ALREADY_MIGRATED`" (an unreadable entry is not
/// a record). Identical to the ELOOP verdict below; no exit 0 is accepted.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_dangling_symlink_completed_json_is_io_exit_2_not_a_fresh_run_blackbox() {
    let c = Coord::new();
    std::os::unix::fs::symlink("no-such-target.json", c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let mut failures = Vec::new();
    assert_completed_json_io_exit_2("dangling completed.json", &c, &before, &out, &mut failures);
    let md = std::fs::symlink_metadata(c.ms().join("completed.json"));
    if !md.map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        failures.push("completed.json must remain the dangling symlink, not a written file".into());
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_dangling_symlink_completed_json_is_io_exit_2_not_a_fresh_run_blackbox",
        failures,
    );
}

/// ADR-052 v1.25 item 11(c) ruling 2(b): a non-`NotFound` lstat/read error (here ELOOP:
/// `completed.json` is a self-referential symlink; every uid) leaves existence UNKNOWN =>
/// the same `Io`, exit 2, ONE line, tree unchanged. (The pre-v1.25 `fs.exists(..)` mapped the
/// stat error to "absent" and started a fresh run.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_eloop_completed_json_is_io_exit_2_not_a_fresh_run_blackbox() {
    let c = Coord::new();
    std::os::unix::fs::symlink("completed.json", c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let mut failures = Vec::new();
    assert_completed_json_io_exit_2("ELOOP completed.json", &c, &before, &out, &mut failures);
    assert_no_failures(
        "test_BC_1_18_011_F004_eloop_completed_json_is_io_exit_2_not_a_fresh_run_blackbox",
        failures,
    );
}

/// A schema-valid `CompletedMigrationRecord` (ADR-052 v1.25 item 11(c) ruling 2(c):
/// `generation_id` string, `txn_id` string, `completed_at` string, `canonical_paths_count`
/// non-negative integer fitting `u64`; unknown extra keys are ignored).
fn valid_completed_record() -> Value {
    json!({
        "generation_id": "gen-0000",
        "txn_id": "act-0000",
        "completed_at": "2026-10-07T00:00:00Z",
        "canonical_paths_count": 4,
    })
}

/// Every non-record content ruling 2(c) names (not UTF-8, empty, truncated, unparseable,
/// not an object, an object that does not deserialize), as `(label, bytes)`.
fn non_record_completed_vectors() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), b"".to_vec()),
        (
            "truncated".into(),
            b"{\"generation_id\": \"gen-0000\", \"txn_id".to_vec(),
        ),
        ("unparseable".into(), b"this is not json".to_vec()),
        ("not UTF-8".into(), vec![0xff, 0xfe, b'{', b'}']),
        ("json array".into(), b"[]".to_vec()),
        ("json null".into(), b"null".to_vec()),
        ("json string".into(), b"\"completed\"".to_vec()),
        ("empty object".into(), b"{}".to_vec()),
    ];
    for key in [
        "generation_id",
        "txn_id",
        "completed_at",
        "canonical_paths_count",
    ] {
        let mut rec = valid_completed_record();
        rec.as_object_mut().unwrap().remove(key);
        v.push((
            format!("missing key {key}"),
            serde_json::to_vec(&rec).unwrap(),
        ));
    }
    let wrong: [(&str, Value); 12] = [
        ("generation_id", json!(7)),
        ("generation_id", Value::Null),
        ("txn_id", json!(["act"])),
        ("txn_id", Value::Null),
        ("completed_at", json!(20261007)),
        ("completed_at", Value::Null),
        ("canonical_paths_count", json!(-1)),
        ("canonical_paths_count", json!(1.5)),
        ("canonical_paths_count", json!("4")),
        ("canonical_paths_count", Value::Null),
        // Not representable as u64 (serde_json parses it as a float).
        ("canonical_paths_count", json!(1.8446744073709552e19)),
        ("canonical_paths_count", json!(true)),
    ];
    for (key, bad) in wrong {
        let mut rec = valid_completed_record();
        rec[key] = bad.clone();
        v.push((
            format!("wrong-typed {key} = {bad}"),
            serde_json::to_vec(&rec).unwrap(),
        ));
    }
    v
}

/// ADR-052 v1.25 item 11(c) ruling 2(c) (+ ruling 3): a `completed.json` that READS but is
/// not a record (not UTF-8, empty, truncated, unparseable, not an object, or an object
/// missing / mistyping any of the four required keys) is
/// `Io { path: completed.json, source: InvalidData(..) }`, exit 2, ONE line, "never exit 0,
/// never a fresh run", nothing written. Presence-only semantics select the BRANCH, but the
/// marker must be a readable schema-shaped record before it may select the exit-0 branch.
#[test]
fn test_BC_1_18_011_F004_completed_json_that_is_not_a_record_is_io_exit_2_nothing_written_blackbox()
{
    let mut failures = Vec::new();
    for (label, bytes) in non_record_completed_vectors() {
        let c = Coord::new();
        std::fs::write(c.ms().join("completed.json"), &bytes).unwrap();
        let before = raw_tree(c.root());
        let out = c.run();
        assert_completed_json_io_exit_2(&label, &c, &before, &out, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_completed_json_that_is_not_a_record_is_io_exit_2_nothing_written_blackbox",
        failures,
    );
}

/// Ruling 2(b): "EISDIR when it is a directory" -- an entry that cannot be READ is `Io`
/// exit 2, not absent and not a record.
#[test]
fn test_BC_1_18_011_F004_directory_completed_json_is_io_exit_2_nothing_written_blackbox() {
    let c = Coord::new();
    std::fs::create_dir(c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let mut failures = Vec::new();
    assert_completed_json_io_exit_2("directory completed.json", &c, &before, &out, &mut failures);
    assert_no_failures(
        "test_BC_1_18_011_F004_directory_completed_json_is_io_exit_2_nothing_written_blackbox",
        failures,
    );
}

/// Ruling 2(b): EACCES on the read (lstat succeeds; the file is mode 000). Skipped when the
/// process can read a mode-000 file (uid 0 -- permission bits do not bind root). The EACCES
/// of an lstat itself is not reachable black-box (a parent without search permission would
/// fail the earlier `exclusive.lock` open instead); the `Fs`-seam twin lives in the
/// failpoints file.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_unreadable_eacces_completed_json_is_io_exit_2_nothing_written_blackbox() {
    use std::os::unix::fs::PermissionsExt;
    let c = Coord::new();
    let marker = c.ms().join("completed.json");
    std::fs::write(
        &marker,
        serde_json::to_vec(&valid_completed_record()).unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&marker).is_ok() {
        // Running as root: the mode bits do not bind; the scenario cannot be staged.
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    let before = raw_tree(c.root());
    let out = c.run();
    let mut failures = Vec::new();
    assert_completed_json_io_exit_2("EACCES completed.json", &c, &before, &out, &mut failures);
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_no_failures(
        "test_BC_1_18_011_F004_unreadable_eacces_completed_json_is_io_exit_2_nothing_written_blackbox",
        failures,
    );
}

/// Valid-record CONTROL (ruling 2(d) + the "clean steady state ... performs zero filesystem
/// mutation" row of ADR-052 §4e): a readable, schema-valid `completed.json` beside an OPEN
/// gate and no txn is `ALREADY_MIGRATED`: exit 0, ZERO writes, EMPTY stdout AND stderr
/// (the v1.23 exit-0 silence; no advisory applies). Unknown extra keys are ignored. Without
/// this the exit-2 vectors above could pass because the fixture can never short-circuit.
#[test]
fn test_BC_1_18_011_F004_valid_completed_json_control_is_exit_0_zero_writes_silent_blackbox() {
    let mut failures = Vec::new();
    let mut with_extra = valid_completed_record();
    with_extra["future_field"] = json!({"ignored": true});
    for (label, record) in [
        ("valid record", valid_completed_record()),
        ("valid record with an unknown extra key", with_extra),
    ] {
        let c = Coord::new();
        std::fs::write(
            c.ms().join("completed.json"),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();
        let before = raw_tree(c.root());
        let out = c.run();
        if out.status.code() != Some(0) || !out.stdout.is_empty() || !out.stderr.is_empty() {
            failures.push(format!(
                "[{label}] expected exit 0 with EMPTY stdout and stderr; got {:?}, stdout {} \
                 bytes, stderr {:?}",
                out.status.code(),
                out.stdout.len(),
                stderr_of(&out)
            ));
        }
        assert_no_fresh_run(label, &c, &before, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_valid_completed_json_control_is_exit_0_zero_writes_silent_blackbox",
        failures,
    );
}

/// Ruling 2(d): "a regular file, or a symlink whose target reads and validates" is present.
/// A symlink to a valid record is therefore `ALREADY_MIGRATED` exactly like the regular
/// file (exit 0, zero writes, silent) -- the lstat-first probe must not conflate "is a
/// symlink" with "unreadable".
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_symlink_to_a_valid_record_is_present_exit_0_zero_writes_silent_blackbox() {
    let c = Coord::new();
    std::fs::write(
        c.ms().join("real-completed.json"),
        serde_json::to_vec_pretty(&valid_completed_record()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink("real-completed.json", c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let mut failures = Vec::new();
    if out.status.code() != Some(0) || !out.stdout.is_empty() || !out.stderr.is_empty() {
        failures.push(format!(
            "expected exit 0 with EMPTY stdout and stderr; got {:?}, stderr {:?}",
            out.status.code(),
            stderr_of(&out)
        ));
    }
    assert_no_fresh_run("symlink to a valid record", &c, &before, &mut failures);
    assert_no_failures(
        "test_BC_1_18_011_F004_symlink_to_a_valid_record_is_present_exit_0_zero_writes_silent_blackbox",
        failures,
    );
}

/// Reader parity (ADR-052 v1.25 item 11(c) ruling 5, TD-VSDD-060 sibling):
/// `detect_migration_read_state` must use the SAME lstat-first classification and record-shape
/// validation as the coordinator (verdicts a-d; `BcIndexAddressingError::Io` for a-c). A
/// reader that reports `NotStarted` ("not migrated") beside a coordinator that says
/// "unknown" is the divergence the ruling closes: the dangling `completed.json` used to fall
/// through (`read_to_string` NotFound) to `CURRENT.json` / `NotStarted`.
fn parity_verdict(
    label: &str,
    r: Result<BcIndexMigrationReadState, BcIndexAddressingError>,
    failures: &mut Vec<String>,
) {
    match r {
        Err(BcIndexAddressingError::Io { path, .. }) if path.ends_with("completed.json") => {}
        other => failures.push(format!(
            "[{label}] expected Err(BcIndexAddressingError::Io {{ path: .../completed.json }}); \
             got {other:?}"
        )),
    }
}

#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_reader_parity_dangling_symlink_is_io_never_not_migrated() {
    let mut failures = Vec::new();

    // Dangling symlink, nothing else: must NOT be NotStarted.
    {
        let d = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("no-such-target.json", d.path().join("completed.json")).unwrap();
        parity_verdict(
            "dangling",
            detect_migration_read_state(d.path()),
            &mut failures,
        );
    }
    // Dangling symlink BESIDE a committing CURRENT.json: must not fall through to Committing.
    {
        let d = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("no-such-target.json", d.path().join("completed.json")).unwrap();
        std::fs::write(
            d.path().join("CURRENT.json"),
            br#"{"status":"committing","generation_id":"gen-1"}"#,
        )
        .unwrap();
        parity_verdict(
            "dangling beside committing CURRENT.json",
            detect_migration_read_state(d.path()),
            &mut failures,
        );
    }
    // ELOOP.
    {
        let d = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("completed.json", d.path().join("completed.json")).unwrap();
        parity_verdict(
            "ELOOP",
            detect_migration_read_state(d.path()),
            &mut failures,
        );
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_reader_parity_dangling_symlink_is_io_never_not_migrated",
        failures,
    );
}

/// Reader parity for ruling 2(c): the same non-record vectors the coordinator refuses are
/// `Io` for the reader (it used to answer `Completed` for any parseable JSON, e.g. `{}` or a
/// record with a missing / mistyped key), plus the directory case.
#[test]
fn test_BC_1_18_011_F004_reader_parity_non_record_completed_json_is_io_never_completed() {
    let mut failures = Vec::new();
    for (label, bytes) in non_record_completed_vectors() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("completed.json"), &bytes).unwrap();
        parity_verdict(&label, detect_migration_read_state(d.path()), &mut failures);
    }
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("completed.json")).unwrap();
    parity_verdict(
        "directory",
        detect_migration_read_state(d.path()),
        &mut failures,
    );
    assert_no_failures(
        "test_BC_1_18_011_F004_reader_parity_non_record_completed_json_is_io_never_completed",
        failures,
    );
}

/// Reader parity CONTROLS (ruling 2(d) and "NotFound only means absent"): a valid record
/// (regular file, or a symlink to one, unknown keys ignored) is `Completed`; a truly absent
/// entry with no `CURRENT.json` is `NotStarted`; absent beside a committing `CURRENT.json` is
/// `Committing`. These keep the Io vectors above honest: a reader that returned `Err` for
/// everything would pass them.
#[test]
fn test_BC_1_18_011_F004_reader_parity_valid_and_absent_controls() {
    let mut failures = Vec::new();
    let mut check = |label: &str,
                     r: Result<BcIndexMigrationReadState, BcIndexAddressingError>,
                     want: BcIndexMigrationReadState| match r {
        Ok(got) if got == want => {}
        other => failures.push(format!("[{label}] expected Ok({want:?}); got {other:?}")),
    };

    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("completed.json"),
        serde_json::to_vec(&valid_completed_record()).unwrap(),
    )
    .unwrap();
    check(
        "valid record",
        detect_migration_read_state(d.path()),
        BcIndexMigrationReadState::Completed,
    );

    let mut extra = valid_completed_record();
    extra["future_field"] = json!(1);
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("completed.json"),
        serde_json::to_vec(&extra).unwrap(),
    )
    .unwrap();
    check(
        "valid record with an unknown extra key",
        detect_migration_read_state(d.path()),
        BcIndexMigrationReadState::Completed,
    );

    #[cfg(unix)]
    {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("real.json"),
            serde_json::to_vec(&valid_completed_record()).unwrap(),
        )
        .unwrap();
        std::os::unix::fs::symlink("real.json", d.path().join("completed.json")).unwrap();
        check(
            "symlink to a valid record",
            detect_migration_read_state(d.path()),
            BcIndexMigrationReadState::Completed,
        );
    }

    let d = tempfile::tempdir().unwrap();
    check(
        "absent",
        detect_migration_read_state(d.path()),
        BcIndexMigrationReadState::NotStarted,
    );

    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("CURRENT.json"),
        br#"{"status":"committing","generation_id":"gen-1"}"#,
    )
    .unwrap();
    check(
        "absent beside committing CURRENT.json",
        detect_migration_read_state(d.path()),
        BcIndexMigrationReadState::Committing {
            generation_id: "gen-1".into(),
        },
    );
    assert_no_failures(
        "test_BC_1_18_011_F004_reader_parity_valid_and_absent_controls",
        failures,
    );
}

/// Scan `run_bc_index_migration_core`'s body for a HARD-CODED "absent" flowing into
/// `recover()`'s `completed` argument. Returns the offending evidence, if any.
///
/// Rationale (ADR-052 v1.24 item 11(c), last sentence): "absent => the recovery path, with
/// `recover(&planner_records, <the under-lock completed read>, None, ..)` -- the
/// hard-coded `completed = None` and its \"honest current read\" comment are removed."
/// Black-box this cannot be observed (a present marker short-circuits BEFORE `recover()`,
/// so the value `recover()` receives is `None` in every reachable state); the normative
/// text is therefore pinned at the source level, in the style of the EC033 sibling-sweep
/// gate.
fn hard_coded_completed_none_into_recover(core_body: &str) -> Option<String> {
    let code: Vec<&str> = core_body
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect();
    let code = code.join("\n");
    let call_at = code.find("= recover(")? + "= recover(".len();
    // Top-level split of the argument list of the `recover(` call.
    let mut depth = 1i32;
    let mut args: Vec<String> = vec![String::new()];
    for ch in code[call_at..].chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        args.last_mut().unwrap().push(ch);
    }
    let completed_arg = args.get(2)?.trim().to_string();
    if completed_arg == "None" {
        return Some("`recover(.., .., None, ..)`: literal None as `completed`".into());
    }
    // `completed_arg` is `<name>.as_ref()` / `<name>`: the binding must not be a bare None.
    let name = completed_arg
        .trim_end_matches(".as_ref()")
        .trim_end_matches(".as_deref()")
        .trim();
    for line in code.lines() {
        let s = line.trim();
        if let Some(rest) = s.strip_prefix("let ")
            && let Some((lhs, rhs)) = rest.split_once('=')
        {
            let lhs_name = lhs.split(':').next().unwrap_or("").trim();
            if lhs_name == name && rhs.trim() == "None;" {
                return Some(format!("`let {lhs_name}.. = None;` flows into recover()"));
            }
        }
    }
    None
}

fn core_body(src: &str) -> &str {
    let start = src
        .find("\nfn run_bc_index_migration_core(")
        .expect("run_bc_index_migration_core must exist");
    let rest = &src[start + 1..];
    let end = rest.find("\n}\n").expect("fn body must end") + 3;
    &rest[..end]
}

/// F-004(iii), POSITIVE control: a body shaped like the defect is detected.
#[test]
fn test_BC_1_18_011_F004_gate_positive_control_hard_coded_none_is_detected() {
    let defect = "fn core() {\n    let completed_under_lock: Option<CompletedMigrationRecord> = None;\n    let decision = recover(&planner_records, None, completed_under_lock.as_ref(), g, m);\n}\n";
    assert!(
        hard_coded_completed_none_into_recover(defect).is_some(),
        "the gate must flag a hard-coded `let .. = None;` flowing into recover()"
    );
    let literal = "fn core() {\n    let d = recover(&r, None, None, g, m);\n}\n";
    assert!(
        hard_coded_completed_none_into_recover(literal).is_some(),
        "the gate must flag a literal None third argument"
    );
}

/// F-004(iii), NEGATIVE control: a body that passes a real read is not flagged, and
/// comments are ignored.
#[test]
fn test_BC_1_18_011_F004_gate_negative_control_real_read_is_not_flagged() {
    let ok = "fn core() {\n    // let completed_under_lock = None; (retired)\n    let completed_under_lock = read_completed_under_lock(&dir)?;\n    let decision = recover(&planner_records, None, completed_under_lock.as_ref(), g, m);\n}\n";
    assert_eq!(hard_coded_completed_none_into_recover(ok), None);
}

/// F-004(iii): `recover()` must receive the under-lock read, not a hard-coded `None`
/// (ADR-052 v1.24 item 11(c): "the hard-coded `completed = None` and its \"honest current
/// read\" comment are removed"). RED today: `let completed_under_lock:
/// Option<CompletedMigrationRecord> = None;` is passed to `recover()`.
#[test]
fn test_BC_1_18_011_F004_recover_receives_the_under_lock_read_not_a_hard_coded_none_source_gate() {
    let src = include_str!("../src/shard_manager.rs");
    let body = core_body(src);
    assert!(
        body.contains("= recover("),
        "sanity: the core body must call recover()"
    );
    let evidence = hard_coded_completed_none_into_recover(body);
    assert!(
        evidence.is_none(),
        "run_bc_index_migration_core must pass the UNDER-LOCK `completed.json` read to \
         recover(), not a hard-coded None: {}",
        evidence.unwrap_or_default()
    );
}

// ===========================================================================
// F-005 -- release failures on the release-on-block paths are a Postcondition 10(c)
//          advisory, not tracing-only
// ===========================================================================

struct Adm {
    dir: tempfile::TempDir,
    plugin_root: tempfile::TempDir,
    logs: tempfile::TempDir,
}

const CYCLES_PATH: &str = ".factory/cycles/c1/burst-log.md";

/// A shard-config entry matching `CYCLES_PATH` but missing `shape`: `shard_cap_precheck`
/// returns a fail-loud Error AFTER admission created the reservation (a LATER stage of the
/// same dispatch blocks => `main.rs` release-on-block).
const CYCLES_MALFORMED_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"burst-log\"
artifact_path = \".factory/cycles/c1/burst-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";

impl Adm {
    fn new() -> Self {
        let a = Adm {
            dir: tempfile::tempdir().unwrap(),
            plugin_root: tempfile::tempdir().unwrap(),
            logs: tempfile::tempdir().unwrap(),
        };
        std::fs::write(
            a.plugin_root.path().join("hooks-registry.toml"),
            "schema_version = 2\n",
        )
        .unwrap();
        std::fs::create_dir_all(a.ms().join("reservations")).unwrap();
        std::fs::write(a.ms().join("exclusive.lock"), b"").unwrap();
        std::fs::write(a.ms().join("gate-state.json"), "\"OPEN\"").unwrap();
        a
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn ms(&self) -> PathBuf {
        self.root().join(".factory/migration-state")
    }
    fn reservation(&self, id: &str) -> PathBuf {
        self.ms()
            .join("reservations")
            .join(format!("{id}.reservation"))
    }
    fn hold_lock(&self) -> MigrationLockGuard {
        try_acquire_migration_lock(&self.ms().join("exclusive.lock"))
            .expect("lock open")
            .expect("lock must be free in fixture setup")
    }
}

fn edit_envelope(abs_path: &Path, tool_use_id: &str) -> String {
    json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "session_id": "sess-f005",
        "tool_use_id": tool_use_id,
        "tool_input": {
            "file_path": abs_path.to_string_lossy(),
            "old_string": "a",
            "new_string": "b",
        },
    })
    .to_string()
}

/// Spawn the dispatcher PAUSED after the reservation write (W1) via the admission test
/// seam (active in debug builds), swap the reservation file for a DIRECTORY (so the later
/// `remove_file` fails with a non-ENOENT error on every uid), resume, and return the output.
fn run_with_unremovable_reservation(a: &Adm, payload: &str, tool_use_id: &str) -> Output {
    let seam = tempfile::tempdir().unwrap();
    std::fs::write(seam.path().join("pause-after-w1"), b"").unwrap();
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", a.plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", a.root())
        .env("VSDD_LOG_DIR", a.logs.path())
        .env("VSDD_TEST_ADMISSION_SEAM_DIR", seam.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(payload.as_bytes()).unwrap();
    drop(stdin);
    let want = format!("W1_RESERVE:{tool_use_id}");
    let start = Instant::now();
    loop {
        let events = std::fs::read_to_string(seam.path().join("events.log")).unwrap_or_default();
        if events.lines().any(|l| l == want) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "setup: dispatcher never wrote {want}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let res = a.reservation(tool_use_id);
    std::fs::remove_file(&res).unwrap();
    std::fs::create_dir(&res).unwrap();
    std::fs::write(res.join("pin"), b"x").unwrap();
    std::fs::write(seam.path().join("resume"), b"").unwrap();
    finish(child, Duration::from_secs(30))
}

/// Every parsed `dispatcher-internal-*.jsonl` line under `log_dir`.
fn read_events(log_dir: &Path) -> Vec<(Value, String)> {
    fn find(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                find(&path, out);
            } else if path.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("dispatcher-internal-") && n.ends_with(".jsonl")
            }) {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    find(log_dir, &mut files);
    files.sort();
    let mut out = Vec::new();
    for f in files {
        for line in std::fs::read_to_string(f).unwrap_or_default().lines() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                out.push((v, line.to_string()));
            }
        }
    }
    out
}

fn of_type<'a>(all: &'a [(Value, String)], ty: &str) -> Vec<&'a (Value, String)> {
    all.iter().filter(|(v, _)| v["type"] == ty).collect()
}

fn check_release_failed_advisory(
    label: &str,
    out: &Output,
    logs: &Path,
    tool_use_id: &str,
    expect_blocked_events: usize,
    failures: &mut Vec<String>,
) {
    if out.status.code() != Some(2) {
        failures.push(format!(
            "[{label}] the dispatch must still block (exit 2); got {:?}, stderr {:?}",
            out.status.code(),
            stderr_of(out)
        ));
    }
    let events = read_events(logs);
    let advisories = of_type(&events, "migration.admission_advisory");
    let release_failed: Vec<_> = advisories
        .iter()
        .filter(|(v, _)| v["reason"] == "reservation_release_failed")
        .collect();
    if release_failed.len() != 1 || advisories.len() != 1 {
        failures.push(format!(
            "[{label}] BC-1.18.013 Postcondition 10(c): a non-ENOENT release error on a \
             release-on-block path produces EXACTLY ONE migration.admission_advisory \
             reason=reservation_release_failed; got {} advisory event(s), {} of them \
             reservation_release_failed (event types seen: {:?})",
            advisories.len(),
            release_failed.len(),
            events
                .iter()
                .filter_map(|(v, _)| v["type"].as_str())
                .collect::<Vec<_>>()
        ));
    }
    let blocked = of_type(&events, "migration.admission_blocked").len();
    if blocked != expect_blocked_events {
        failures.push(format!(
            "[{label}] expected exactly {expect_blocked_events} migration.admission_blocked \
             event(s), got {blocked}"
        ));
    }
    // Hygiene (BC-3.08.001 Event 13): no field carries the raw tool_use_id.
    for (_, line) in &advisories {
        if line.contains(tool_use_id) {
            failures.push(format!(
                "[{label}] the advisory must not carry the raw tool_use_id: {line}"
            ));
        }
    }
}

/// F-005 (a): the admitter's OWN release-on-block (`remove_own_reservation`) in the shared
/// core. Gate LOCKED + a foreign live txn + a held lock => `E-MAINTENANCE-001` block AFTER
/// the reservation was created; the reservation cannot be unlinked (a directory sits at its
/// path). Required: the block verdict is unchanged AND exactly one `reservation_release_failed`
/// advisory (beside exactly one `migration.admission_blocked`).
#[test]
fn test_BC_1_18_013_F005_admitter_own_release_failure_writes_one_reservation_release_failed_advisory_blackbox()
 {
    let a = Adm::new();
    std::fs::write(a.ms().join("gate-state.json"), "\"LOCKED\"").unwrap();
    std::fs::write(
        a.ms().join("txn-act-f005.json"),
        serde_json::to_vec_pretty(&json!({
            "txn_id": "act-f005", "activation_id": "act-f005", "fencing_generation": 1,
            "state": "STAGING", "generation_id": "gen-1", "source_sha256": null,
            "source_body_row_sha256": null, "intent_log_path": null,
            "pending_canonical_moves": [],
            "created_at": "2026-10-06T00:00:00Z", "updated_at": "2026-10-06T00:00:00Z",
            "migration_id": "backfill-append-logs",
        }))
        .unwrap(),
    )
    .unwrap();
    let _live = a.hold_lock();
    let target = a.root().join(CYCLES_PATH);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let id = "toolu_F005ownSECRET";
    let out = run_with_unremovable_reservation(&a, &edit_envelope(&target, id), id);
    let mut failures = Vec::new();
    if !stderr_of(&out).contains("E-MAINTENANCE-001") {
        failures.push(format!(
            "setup: the dispatch must be an E-MAINTENANCE-001 block; stderr {:?}",
            stderr_of(&out)
        ));
    }
    check_release_failed_advisory(
        "admitter own release",
        &out,
        a.logs.path(),
        id,
        1,
        &mut failures,
    );
    assert_no_failures(
        "test_BC_1_18_013_F005_admitter_own_release_failure_writes_one_reservation_release_failed_advisory_blackbox",
        failures,
    );
}

/// F-005 (b): the `main.rs` release-on-block (`if code == 2 { release_reservation_file(..) }`)
/// when a LATER stage (`shard_cap_precheck`) blocks an ADMITTED dispatch. Same fault. Required:
/// exit 2 unchanged, exactly one `reservation_release_failed` advisory, and NO
/// `migration.admission_blocked` (admission admitted).
#[test]
fn test_BC_1_18_013_F005_main_rs_release_on_block_failure_writes_one_reservation_release_failed_advisory_blackbox()
 {
    let a = Adm::new();
    std::fs::write(
        a.root().join(".factory/shard-config.toml"),
        CYCLES_MALFORMED_SHARD_CONFIG,
    )
    .unwrap();
    let target = a.root().join(CYCLES_PATH);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "x".repeat(100)).unwrap();
    let id = "toolu_F005mainSECRET";
    let out = run_with_unremovable_reservation(&a, &edit_envelope(&target, id), id);
    let mut failures = Vec::new();
    check_release_failed_advisory("main.rs release", &out, a.logs.path(), id, 0, &mut failures);
    assert_no_failures(
        "test_BC_1_18_013_F005_main_rs_release_on_block_failure_writes_one_reservation_release_failed_advisory_blackbox",
        failures,
    );
}

// ===========================================================================
// F-006 -- coordinator anomalies are stderr lines, not tracing-only
// ===========================================================================

// BC-1.18.013 v1.13 Postcondition 10, "Coordinators" clause (verbatim): "the two coordinator
// binaries are CLI processes whose stderr is the operator surface -- each failure or advisory
// they previously sent only to `tracing` (`migrate-bc-index: migration failed`, the drain's
// reservation-staleness advisories, the live-coordinator warning of Postcondition 5) is
// written to stderr as one `E-...`/`<VARIANT>`-prefixed line; a `tracing::error!` there is
// only an additional developer trace."
//
// ADR-052 v1.25 item 11(f) rules the clause an OPEN set and fixes the shape and the closed
// token domain of NON-FATAL advisories:
//
//     <subcommand>: <TOKEN> (advisory): <path>: <os error>; <one clause saying what is true now>
//
// stdout stays empty, one line per condition, exit status UNCHANGED (normally 0; "an exit-0
// run prints nothing EXCEPT advisory lines"). The domain for this build is
// GATE_OPEN_RESET_FAILED, TERMINAL_TXN_ARCHIVE_FAILED, STAGING_DIR_REMOVE_FAILED and, only
// if reachable from a coordinator process, OVERSIZED_ROW_SUBSHARD. Every test below pins
// (1) the exact token and line shape, (2) that the clause is TRUE of the on-disk state, and
// (3) that the run's verdict and exit are identical to a no-fault CONTROL run.
//
// Canonical-move HALT REASONS (rename / dir-sync / parent-create / FailClosed / post-rename
// verification / DONE-append failures in `execute_canonical_path_moves`) are NOT an S-25.09
// deliverable (ADR-052 v1.25 item 11(f)(4)): they are carried by S-25.11's
// `CANONICAL_MOVE_HALTED (exit 2): move to <target> halted: <reason>; ...` (AC-014/AC-015,
// which also owns their pass-1 black-box vectors). The former
// `..._F006_canonical_move_halt_reasons_are_stderr_lines_blackbox` test was REMOVED from this
// story for that reason; do not re-add halt-reason vectors here.

/// A COMMITTING record with a string `generation_id`, the matching generation dir and
/// `intent_log_path`, and the given `pending_canonical_moves`; gate LOCKED.
fn committing_record(pending: Value) -> Value {
    json!({
        "schema_version": 1,
        "txn_id": "act-1",
        "migration_id": MIGRATION_ID,
        "activation_id": "act-1",
        "fencing_generation": 1,
        "state": "COMMITTING",
        "generation_id": "gen-1",
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": ".factory/migration-state/intent-gen-1.log",
        "pending_canonical_moves": pending,
        "created_at": "2026-10-07T00:00:00Z",
        "updated_at": "2026-10-07T00:00:00Z",
    })
}

fn committing_fixture(c: &Coord, pending: Value) {
    std::fs::write(
        c.ms().join("txn-act-1.json"),
        serde_json::to_vec_pretty(&committing_record(pending)).unwrap(),
    )
    .unwrap();
    std::fs::create_dir_all(c.ms().join("gen-gen-1")).unwrap();
    std::fs::write(c.ms().join("gate-state.json"), "\"LOCKED\"").unwrap();
}

/// The part of a run's outcome an advisory must NOT change: exit status, the
/// `canonical_paths_count` of `completed.json`, and how many live-looking `txn-*.json`
/// records are in each state.
#[derive(Debug, PartialEq, Eq)]
struct Verdict {
    exit: Option<i32>,
    completed_paths: Option<u64>,
    txn_states: Vec<String>,
}

fn verdict_of(c: &Coord, out: &Output) -> Verdict {
    let completed_paths = std::fs::read(c.ms().join("completed.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["canonical_paths_count"].as_u64());
    let mut txn_states: Vec<String> = c
        .txn_files()
        .iter()
        .filter(|n| n.ends_with(".json"))
        .filter_map(|n| std::fs::read(c.ms().join(n)).ok())
        .filter_map(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|v| v["state"].as_str().unwrap_or("?").to_string())
        .collect();
    txn_states.sort();
    Verdict {
        exit: out.status.code(),
        completed_paths,
        txn_states,
    }
}

/// Validate every stderr line of a run as an advisory/known line and pin ONE advisory line
/// for `token`: `migrate-bc-index: <TOKEN> (advisory): <path>: <os error>; <clause>`, where
/// `<path>` ends with `path_suffix`, `<os error>` is non-empty and `<clause>` contains every
/// `clause_needles` entry and none of `clause_forbidden`. Returns the failures.
fn check_advisory_line(
    label: &str,
    err: &str,
    token: &str,
    path_suffix: &str,
    clause_needles: &[&str],
    clause_forbidden: &[&str],
) -> Vec<String> {
    let mut failures = Vec::new();
    let head = format!("migrate-bc-index: {token} (advisory): ");
    let lines: Vec<&str> = err.lines().filter(|l| l.contains(token)).collect();
    if lines.len() != 1 {
        failures.push(format!(
            "[{label}] expected exactly ONE `{head}...` stderr line; found {} in stderr {err:?}",
            lines.len()
        ));
        return failures;
    }
    let line = lines[0];
    let Some(rest) = line.strip_prefix(&head) else {
        failures.push(format!(
            "[{label}] the advisory line must start with `{head}`; got {line:?}"
        ));
        return failures;
    };
    let Some((path, after_path)) = rest.split_once(": ") else {
        failures.push(format!(
            "[{label}] shape is `<path>: <os error>; <clause>`; no `: ` after the path in {line:?}"
        ));
        return failures;
    };
    if !path.ends_with(path_suffix) {
        failures.push(format!(
            "[{label}] <path> must end with {path_suffix:?}; got {path:?} in {line:?}"
        ));
    }
    let Some((os_error, clause)) = after_path.split_once("; ") else {
        failures.push(format!(
            "[{label}] shape is `<path>: <os error>; <clause>`; no `; ` clause separator in {line:?}"
        ));
        return failures;
    };
    if os_error.trim().is_empty() {
        failures.push(format!(
            "[{label}] <os error> must be non-empty in {line:?}"
        ));
    }
    for needle in clause_needles {
        if !clause.contains(needle) {
            failures.push(format!(
                "[{label}] the clause must say {needle:?}; clause {clause:?}"
            ));
        }
    }
    for bad in clause_forbidden {
        if clause.contains(bad) {
            failures.push(format!(
                "[{label}] the clause must not claim {bad:?} (it would be false); clause {clause:?}"
            ));
        }
    }
    failures
}

/// "an exit-0 run prints nothing EXCEPT advisory lines" (v1.25 item 11(f)(3)): stdout empty
/// and every stderr line is an `(advisory)` line.
fn check_exit_0_only_advisories(label: &str, out: &Output) -> Vec<String> {
    let mut failures = Vec::new();
    let err = stderr_of(out);
    if !out.stdout.is_empty() {
        failures.push(format!(
            "[{label}] stdout must stay empty; got {} bytes",
            out.stdout.len()
        ));
    }
    for l in err.lines().filter(|l| !l.contains("(advisory)")) {
        failures.push(format!(
            "[{label}] an exit-0 run may print only `(advisory)` lines; got {l:?}"
        ));
    }
    failures
}

/// F-006 (a): `finish_committing_migration`'s best-effort gate-OPEN write FAILS after the txn
/// reached COMPLETED. ADR-052 v1.25 item 11(f)(2): token `GATE_OPEN_RESET_FAILED`; the exit
/// stays 0 and the clause is "the migration completed; the gate stays <state> until the next
/// admission check or `migrate-bc-index` run reconciles it", which "must be true and must not
/// claim OPEN".
///
/// The gate file is replaced by a directory (root-safe: every gate write fails). CONTROL: the
/// identical fixture without the sabotage completes silently with the gate OPEN; the sabotaged
/// run's verdict (exit, `completed.json`, txn states) must equal the control's.
#[test]
fn test_BC_1_18_013_F006_gate_open_reset_failed_advisory_exit_0_verdict_unchanged_blackbox() {
    // CONTROL.
    let control = Coord::new();
    committing_fixture(&control, json!([]));
    let control_out = control.run();
    let control_verdict = verdict_of(&control, &control_out);

    let c = Coord::new();
    committing_fixture(&c, json!([]));
    let gate = c.ms().join("gate-state.json");
    std::fs::remove_file(&gate).unwrap();
    std::fs::create_dir(&gate).unwrap();
    let out = c.run();
    let err = stderr_of(&out);

    let mut failures = Vec::new();
    if control_out.status.code() != Some(0)
        || !control_out.stdout.is_empty()
        || !control_out.stderr.is_empty()
        || std::fs::read_to_string(control.ms().join("gate-state.json"))
            .map(|s| !s.contains("OPEN"))
            .unwrap_or(true)
    {
        failures.push(format!(
            "control: the unsabotaged fixture must complete silently (exit 0, empty stdout and \
             stderr) with the gate OPEN; got {:?}, stderr {:?}",
            control_out.status.code(),
            stderr_of(&control_out)
        ));
    }
    if out.status.code() != Some(0) {
        failures.push(format!(
            "the advisory must not change the exit (0); got {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    failures.extend(check_exit_0_only_advisories("GATE_OPEN_RESET_FAILED", &out));
    failures.extend(check_advisory_line(
        "GATE_OPEN_RESET_FAILED",
        &err,
        "GATE_OPEN_RESET_FAILED",
        "gate-state.json",
        &["gate stays", "reconcile"],
        &["stays OPEN", "returned to OPEN", "is OPEN", "is now OPEN"],
    ));
    // The clause is TRUE: completed, txn COMPLETED, the gate is still not OPEN.
    let v = verdict_of(&c, &out);
    if v != control_verdict {
        failures.push(format!(
            "the advisory must leave the verdict unchanged; control {control_verdict:?}, got {v:?}"
        ));
    }
    if !c.ms().join("gate-state.json").is_dir() {
        failures.push("setup: the gate must still be the sabotaged directory".into());
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_gate_open_reset_failed_advisory_exit_0_verdict_unchanged_blackbox",
        failures,
    );
}

/// F-006 (b): `archive_terminal_txn_record`'s best-effort rename FAILS with a non-ENOENT
/// error. ADR-052 v1.25 item 11(f)(2): token `TERMINAL_TXN_ARCHIVE_FAILED`, same shape, clause
/// "the stale terminal record stays in place and is skipped as non-live"; exit stays 0.
///
/// A stale ABORTED record is archived before the fresh run; the archive name is occupied by a
/// non-empty directory so the rename fails (root-safe). CONTROL: without the blocker the
/// record is archived silently and the fresh run completes; verdicts agree on exit and
/// `completed.json`.
#[test]
fn test_BC_1_18_013_F006_terminal_txn_archive_failed_advisory_exit_0_verdict_unchanged_blackbox() {
    let stale = |c: &Coord| {
        std::fs::write(
            c.ms().join("txn-act-old.json"),
            serde_json::to_vec_pretty(&json!({"state": "ABORTED", "migration_id": MIGRATION_ID}))
                .unwrap(),
        )
        .unwrap();
    };
    // CONTROL.
    let control = Coord::new();
    stale(&control);
    let control_out = control.run();
    let control_verdict = verdict_of(&control, &control_out);

    let c = Coord::new();
    stale(&c);
    let blocker = c.ms().join("txn-act-old.json.archived");
    std::fs::create_dir(&blocker).unwrap();
    std::fs::write(blocker.join("pin"), b"x").unwrap();
    let out = c.run();
    let err = stderr_of(&out);

    let mut failures = Vec::new();
    if control_out.status.code() != Some(0)
        || !control_out.stdout.is_empty()
        || !control_out.stderr.is_empty()
        || !control.ms().join("txn-act-old.json.archived").is_file()
    {
        failures.push(format!(
            "control: the unblocked fixture must archive the stale record and complete silently \
             (exit 0, empty stdout and stderr); got {:?}, stderr {:?}",
            control_out.status.code(),
            stderr_of(&control_out)
        ));
    }
    if out.status.code() != Some(0) {
        failures.push(format!(
            "the advisory must not change the exit (0); got {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    failures.extend(check_exit_0_only_advisories(
        "TERMINAL_TXN_ARCHIVE_FAILED",
        &out,
    ));
    failures.extend(check_advisory_line(
        "TERMINAL_TXN_ARCHIVE_FAILED",
        &err,
        "TERMINAL_TXN_ARCHIVE_FAILED",
        "txn-act-old.json",
        &[
            "stale terminal record stays in place",
            "skipped as non-live",
        ],
        &[],
    ));
    // The clause is TRUE: the stale record is still in place; the run still completed.
    if !c.ms().join("txn-act-old.json").is_file() || !blocker.join("pin").is_file() {
        failures.push("the stale terminal record must stay in place beside the blocker".into());
    }
    let v = verdict_of(&c, &out);
    if v.exit != control_verdict.exit || v.completed_paths != control_verdict.completed_paths {
        failures.push(format!(
            "the advisory must leave the verdict unchanged; control {control_verdict:?}, got {v:?}"
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_terminal_txn_archive_failed_advisory_exit_0_verdict_unchanged_blackbox",
        failures,
    );
}

/// Make `dir/<file>` impossible to remove, independent of uid where the platform allows.
/// macOS/BSD: the user-immutable flag (`chflags uchg`), which binds root too. Other unix: an
/// unwritable parent directory (mode 0500), which does NOT bind root -- `None` is returned
/// then so the caller skips. Restores everything on drop so the tempdir can be cleaned up.
struct Unremovable {
    dir: PathBuf,
    file: PathBuf,
    flagged: bool,
}

impl Unremovable {
    #[cfg(unix)]
    fn pin(dir: &Path) -> Option<Unremovable> {
        use std::os::unix::fs::PermissionsExt;
        let file = dir.join("pinned.bin");
        std::fs::write(&file, b"pinned").unwrap();
        if cfg!(target_os = "macos") {
            let st = Command::new("chflags")
                .arg("uchg")
                .arg(&file)
                .stdin(Stdio::null())
                .status()
                .expect("chflags");
            assert!(st.success(), "setup: chflags uchg failed");
            return Some(Unremovable {
                dir: dir.to_path_buf(),
                file,
                flagged: true,
            });
        }
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        let probe = dir.join("probe");
        if std::fs::write(&probe, b"x").is_ok() {
            // Permission bits do not bind this uid (root): the scenario cannot be staged.
            let _ = std::fs::remove_file(&probe);
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
            return None;
        }
        Some(Unremovable {
            dir: dir.to_path_buf(),
            file,
            flagged: false,
        })
    }
}

impl Drop for Unremovable {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if self.flagged {
                let _ = Command::new("chflags")
                    .arg("nouchg")
                    .arg(&self.file)
                    .stdin(Stdio::null())
                    .status();
            } else {
                let _ = std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o755));
            }
        }
    }
}

/// A STAGING record (generation `gen-1`, its `gen-gen-1` directory present) whose staged
/// generation is empty, so `resume_from_staging` rejects it and the coordinator discards the
/// incomplete generation through `discard_incomplete_staging`.
fn staging_fixture(c: &Coord) {
    let mut rec = committing_record(json!([]));
    rec["state"] = json!("STAGING");
    std::fs::write(
        c.ms().join("txn-act-1.json"),
        serde_json::to_vec_pretty(&rec).unwrap(),
    )
    .unwrap();
    std::fs::create_dir_all(c.ms().join("gen-gen-1/shards")).unwrap();
    std::fs::write(c.ms().join("gate-state.json"), "\"LOCKED\"").unwrap();
}

/// F-006 (d): `discard_incomplete_staging`'s generation-directory removal FAILS. ADR-052
/// v1.25 item 11(f)(2): token `STAGING_DIR_REMOVE_FAILED`, same shape, clause "the orphaned
/// generation directory is inert; the txn is ABORTED". The run's own failure (the resume
/// rejection, exit and line) is the verdict and must be IDENTICAL to the no-fault CONTROL; the
/// advisory is one additional line. The removal failure is injected with an unremovable file
/// inside the generation directory (uid-independent on macOS via `chflags uchg`; elsewhere
/// the test skips under uid 0).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_F006_staging_dir_remove_failed_advisory_verdict_unchanged_blackbox() {
    // CONTROL: no pin -> the generation directory is removed, the txn is ABORTED, no advisory.
    let control = Coord::new();
    staging_fixture(&control);
    let control_out = control.run();
    let control_err = stderr_of(&control_out);
    let control_verdict = verdict_of(&control, &control_out);

    let c = Coord::new();
    staging_fixture(&c);
    let gen_dir = c.ms().join("gen-gen-1");
    let Some(_pin) = Unremovable::pin(&gen_dir.join("shards")) else {
        return;
    };
    let out = c.run();
    let err = stderr_of(&out);

    let mut failures = Vec::new();
    if control_out.status.code() == Some(0)
        || control.ms().join("gen-gen-1").exists()
        || control_verdict.txn_states != ["ABORTED"]
        || control_err.contains("(advisory)")
    {
        failures.push(format!(
            "control: the unpinned fixture must be rejected (non-zero exit), remove the \
             generation directory, ABORT the txn and print no advisory; got exit {:?}, \
             gen-gen-1 exists={}, states {:?}, stderr {control_err:?}",
            control_out.status.code(),
            control.ms().join("gen-gen-1").exists(),
            control_verdict.txn_states
        ));
    }
    if out.status.code() != control_out.status.code() {
        failures.push(format!(
            "the advisory must not change the exit; control {:?}, got {:?}, stderr {err:?}",
            control_out.status.code(),
            out.status.code()
        ));
    }
    let non_advisory = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| !l.contains("(advisory)"))
            .map(str::to_string)
            .collect()
    };
    if non_advisory(&err) != non_advisory(&control_err) {
        failures.push(format!(
            "apart from the advisory line the stderr must equal the control's (the run's own \
             failure line is unchanged); control {control_err:?}, got {err:?}"
        ));
    }
    failures.extend(check_advisory_line(
        "STAGING_DIR_REMOVE_FAILED",
        &err,
        "STAGING_DIR_REMOVE_FAILED",
        "gen-gen-1",
        &["orphaned generation directory is inert", "ABORTED"],
        &[],
    ));
    // The clause is TRUE: the directory is still there, inert, and the txn is ABORTED.
    let v = verdict_of(&c, &out);
    if !gen_dir.is_dir() || v.txn_states != ["ABORTED"] {
        failures.push(format!(
            "the clause must be true: gen-gen-1 must remain (exists={}) and the txn must be \
             ABORTED; states {:?}",
            gen_dir.is_dir(),
            v.txn_states
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_staging_dir_remove_failed_advisory_verdict_unchanged_blackbox",
        failures,
    );
}

/// F-006 (e): `OVERSIZED_ROW_SUBSHARD`. Call-graph audit (ADR-052 v1.25 item 11(f)(2) makes
/// the token conditional on reachability from a coordinator process): REACHABLE.
/// `run_bc_index_migration_core` (fresh-run path) -> `chunk_subsystem_rows_into_sub_shards`
/// -> `close_sub_shard_chunk`, whose `tracing::warn!` fires for a lone row whose sub-shard
/// body exceeds `shard_cap_bytes`; a grep finds exactly one production call
/// site of `chunk_subsystem_rows_into_sub_shards`. The migration still COMPLETES (the over-cap sub-shard is emitted as its own file,
/// ADR-051 Decision 18), so this is a degraded success: exit 0 + an advisory line. The line
/// names the offending row (`BC-1.01.001`, the sub-shard's range) so the operator can find
/// it; the `<os error>` slot of the item-11(f) shape does not apply (no OS call failed), so
/// only the token, the `(advisory)` marker, the row id and the exit-0 contract are pinned.
#[test]
fn test_BC_1_18_013_F006_oversized_row_subshard_advisory_exit_0_verdict_unchanged_blackbox() {
    let big_title = "x".repeat(2_000);
    let content = ORIGINAL_CONTENT.replace("Registry rejects unknown schema version", &big_title);
    // CONTROL: the default cap -> no sub-split, silent success.
    let control = Coord::new();
    std::fs::write(control.canonical(), &content).unwrap();
    let control_out = control.run();

    let c = Coord::new();
    std::fs::write(c.canonical(), &content).unwrap();
    // A cap smaller than the single oversized row.
    let small_cap = SHARD_CONFIG.replace("shard_cap_bytes = 100000", "shard_cap_bytes = 1500");
    std::fs::write(c.root().join(".factory/shard-config.toml"), small_cap).unwrap();
    let out = c.run();
    let err = stderr_of(&out);
    let v = verdict_of(&c, &out);

    let mut failures = Vec::new();
    if control_out.status.code() != Some(0) || !control_out.stderr.is_empty() {
        failures.push(format!(
            "control: the default-cap fixture must complete silently; got {:?}, stderr {:?}",
            control_out.status.code(),
            stderr_of(&control_out)
        ));
    }
    if out.status.code() != Some(0) || v.completed_paths.is_none() {
        failures.push(format!(
            "setup/contract: an over-cap lone row is a degraded SUCCESS (exit 0, completed.json \
             written); got {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    failures.extend(check_exit_0_only_advisories("OVERSIZED_ROW_SUBSHARD", &out));
    let head = "migrate-bc-index: OVERSIZED_ROW_SUBSHARD (advisory): ";
    let lines: Vec<&str> = err
        .lines()
        .filter(|l| l.contains("OVERSIZED_ROW_SUBSHARD"))
        .collect();
    if lines.len() != 1 || !lines[0].starts_with(head) || !lines[0].contains("BC-1.01.001") {
        failures.push(format!(
            "expected exactly ONE `{head}...BC-1.01.001...` line; stderr {err:?}"
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_oversized_row_subshard_advisory_exit_0_verdict_unchanged_blackbox",
        failures,
    );
}

// ===========================================================================
// F-007(b) -- the intent_log_path equality is EXACT (no normalization)
// ===========================================================================

fn pair_record(state: &str, intent_log_path: &str) -> Value {
    let mut v = committing_record(json!([]));
    v["state"] = json!(state);
    v["intent_log_path"] = json!(intent_log_path);
    v
}

/// BC-1.18.011 v1.21 Precondition 6(f)(iii): `intent_log_path` "MUST be a JSON string EQUAL
/// to `.factory/migration-state/intent-<generation_id>.log`; a `null`, absent, non-string or
/// mismatching value => `txn_record_malformed` ... nothing mutated". Equality is EXACT: an
/// ABSOLUTE spelling of the same file, a `./`-prefixed spelling and a trailing-space
/// spelling all denote (or nearly denote) the generation's log but are not EQUAL, so each
/// is rejected for both STAGING (string generation) and COMMITTING; the existing
/// `..._not_equal_to_the_generation_log_path_...` test deliberately omitted them.
#[test]
fn test_BC_1_18_011_F007_intent_log_path_exact_string_variants_are_txn_record_malformed_blackbox() {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        let variants: Vec<(&str, Option<String>)> = vec![
            ("absolute path", None),
            (
                "./-prefixed path",
                Some("./.factory/migration-state/intent-gen-1.log".to_string()),
            ),
            (
                "trailing-space path",
                Some(".factory/migration-state/intent-gen-1.log ".to_string()),
            ),
        ];
        for (vlabel, path) in variants {
            let c = Coord::new();
            let path = path.unwrap_or_else(|| {
                format!(
                    "{}/.factory/migration-state/intent-gen-1.log",
                    c.root().display()
                )
            });
            std::fs::write(
                c.ms().join("txn-act-1.json"),
                serde_json::to_vec_pretty(&pair_record(state, &path)).unwrap(),
            )
            .unwrap();
            std::fs::create_dir_all(c.ms().join("gen-gen-1")).unwrap();
            std::fs::write(c.ms().join("gate-state.json"), "\"LOCKED\"").unwrap();
            let txn_before = std::fs::read(c.ms().join("txn-act-1.json")).unwrap();
            let before = raw_tree(c.root());
            let out = c.run();
            let err = stderr_of(&out);
            let label = format!("{state}, intent_log_path = {vlabel} ({path:?})");
            let prefix = "migrate-bc-index: migration admission: state integrity failure \
                          (txn_record_malformed): ";
            if out.status.code() != Some(2)
                || !out.stdout.is_empty()
                || err.lines().count() != 1
                || !err.starts_with(prefix)
                || !err.contains("intent_log_path")
            {
                failures.push(format!(
                    "[{label}] expected exit 2 and ONE `{prefix}...intent_log_path...` line \
                     (txn_record_malformed); got exit {:?}, stderr {err:?}",
                    out.status.code()
                ));
            }
            if raw_tree(c.root()) != before
                || std::fs::read(c.ms().join("txn-act-1.json")).unwrap() != txn_before
            {
                failures.push(format!("[{label}] nothing may be mutated"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_F007_intent_log_path_exact_string_variants_are_txn_record_malformed_blackbox",
        failures,
    );
}

// ===========================================================================
// F-009 -- AC-006 / T-13: the old per-migration entry-point names are gone
// ===========================================================================

/// The names AC-006 forbids anywhere under `crates/`, assembled from fragments so this file
/// itself never contains them.
fn forbidden_entry_point_names() -> Vec<String> {
    let bc = ["bc_index_", "migration_"].concat();
    let ap = ["append_log_", "backfill_"].concat();
    vec![
        format!("{bc}admission"), // also covers `..._admission_precheck`
        format!("{bc}reservation_release"),
        format!("{ap}admission_precheck"),
        format!("{ap}reservation_release"),
    ]
}

/// Every occurrence of a forbidden name in `text` as `(line_number, line)`.
fn find_forbidden(text: &str, names: &[String]) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| names.iter().any(|n| l.contains(n.as_str())))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// Recursively scan `dir` for text files containing a forbidden name, skipping build
/// output (`target/`).
fn scan_tree(dir: &Path, names: &[String], hits: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let ft = e.file_type().unwrap();
        if ft.is_dir() {
            if name == "target" || name == ".git" {
                continue;
            }
            scan_tree(&p, names, hits);
        } else if ft.is_file()
            && let Ok(text) = std::fs::read_to_string(&p)
        {
            for (n, l) in find_forbidden(&text, names) {
                hits.push(format!("{}:{n}: {l}", p.display()));
            }
        }
    }
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .to_path_buf()
}

/// F-009 positive control: a file containing a forbidden name is detected by `scan_tree`.
#[test]
fn test_BC_1_18_013_F009_gate_positive_control_a_file_with_an_old_name_is_detected() {
    let names = forbidden_entry_point_names();
    for name in &names {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("nested")).unwrap();
        std::fs::write(
            d.path().join("nested/stale.rs"),
            format!("// call {name}(payload, cwd);\n"),
        )
        .unwrap();
        let mut hits = Vec::new();
        scan_tree(d.path(), &names, &mut hits);
        assert_eq!(hits.len(), 1, "the scanner must detect `{name}`: {hits:?}");
    }
}

/// F-009 negative control: neutral names and ignored `target/` output are not flagged.
#[test]
fn test_BC_1_18_013_F009_gate_negative_control_neutral_names_and_target_dir_are_not_flagged() {
    let names = forbidden_entry_point_names();
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ok.rs"),
        "fn migration_writer_admission() {}\nfn migration_writer_admission_precheck() {}\nfn migration_reservation_release() {}\n",
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join("target")).unwrap();
    std::fs::write(d.path().join("target/built.rs"), format!("{}\n", names[0])).unwrap();
    let mut hits = Vec::new();
    scan_tree(d.path(), &names, &mut hits);
    assert!(
        hits.is_empty(),
        "neutral names must not be flagged: {hits:?}"
    );
}

/// F-009 / AC-006 / T-13: the `grep -rn` gate of AC-006 over `crates/` (the four old
/// per-migration admission / precheck / reservation-release entry-point names, see
/// `forbidden_entry_point_names`) returns nothing (ADR-052 §Decision 5a "Entry-point
/// naming"; BC-1.18.013 Precondition 6(b) "Where", EC-020; BC-1.18.011 EC-015).
#[test]
fn test_BC_1_18_013_F009_old_entry_point_names_absent_source_gate() {
    let names = forbidden_entry_point_names();
    let mut hits = Vec::new();
    scan_tree(&crates_dir(), &names, &mut hits);
    assert!(
        hits.is_empty(),
        "AC-006: the per-migration entry-point names must not appear anywhere under crates/; \
         {} occurrence(s):\n  {}",
        hits.len(),
        hits.join("\n  ")
    );
}

// ===========================================================================
// F-010 -- Event 12/13 `detail`: data-derived substrings truncated to 64 characters
// ===========================================================================

// BC-3.08.001 v1.37 Event 12 Field semantics (verbatim): "`detail` ... sanitized (control
// characters escaped, truncated to 64 characters for any data-derived substring)"; Invariant
// 7: "string fields derived from disk/payload data are sanitized (control characters escaped,
// truncated to 64 characters)"; BC-1.18.013 Postcondition 10 Hygiene: "every string field
// derived from on-disk or payload data ... is sanitized -- control characters escaped and
// truncated to 64 characters (EC-029)". The cap is PER DATA-DERIVED SUBSTRING (a `detail`
// such as `<path>: <ErrorKind>: <message>` may legitimately exceed 64 characters overall; the
// path substring and the message substring may not). `sanitize_diagnostic` includes the
// trailing `…` marker inside its cap.

fn long_project() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp
        .path()
        .canonicalize()
        .unwrap()
        .join("p".repeat(30))
        .join("q".repeat(30));
    std::fs::create_dir_all(&root).unwrap();
    assert!(root.display().to_string().chars().count() > 64);
    (tmp, root)
}

fn run_hook(root: &Path, plugin_root: &Path, logs: &Path, payload: &str) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", plugin_root)
        .env("CLAUDE_PROJECT_DIR", root)
        .env("VSDD_LOG_DIR", logs)
        .env_remove("VSDD_TEST_ADMISSION_SEAM_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(payload.as_bytes()).unwrap();
    drop(stdin);
    finish(child, Duration::from_secs(30))
}

/// Assert the `<path>`-led `detail` respects the 64-character per-substring cap.
fn check_path_substring_cap(
    label: &str,
    detail: &str,
    full_path: &str,
    failures: &mut Vec<String>,
) {
    let first = detail.split(": ").next().unwrap_or("");
    let n = first.chars().count();
    if n > 64 {
        failures.push(format!(
            "[{label}] the data-derived path substring of `detail` is {n} characters (> 64): \
             {detail:?}"
        ));
    }
    if detail.contains(full_path) {
        failures.push(format!(
            "[{label}] the full {}-character path must not appear in `detail`: {detail:?}",
            full_path.chars().count()
        ));
    }
    let kept = first.trim_end_matches('…');
    if !full_path.starts_with(kept) || kept.is_empty() {
        failures.push(format!(
            "[{label}] the truncated path must be a PREFIX of the real path (marker `…` \
             allowed); got {first:?}"
        ));
    }
}

#[cfg(unix)]
#[test]
fn test_BC_3_08_001_F010_event_12_13_detail_data_derived_substrings_truncated_to_64_chars_blackbox()
{
    let mut failures = Vec::new();
    let plugin_root = tempfile::tempdir().unwrap();
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .unwrap();

    // (1) Event 12 `cause=io`: ELOOP `.factory` => detail `<root>/.factory: <kind>: <msg>`.
    {
        let (_keep, root) = long_project();
        std::os::unix::fs::symlink(".factory", root.join(".factory")).unwrap();
        let logs = tempfile::tempdir().unwrap();
        let target = root.join(CYCLES_PATH);
        let out = run_hook(
            &root,
            plugin_root.path(),
            logs.path(),
            &edit_envelope(&target, "toolu_f010a"),
        );
        let events = read_events(logs.path());
        let failed = of_type(&events, "migration.admission_failed");
        if out.status.code() != Some(2) || failed.len() != 1 || failed[0].0["cause"] != "io" {
            failures.push(format!(
                "[event 12 io] setup: expected exit 2 and one cause=io admission_failed; got \
                 exit {:?}, {} failed event(s)",
                out.status.code(),
                failed.len()
            ));
        } else {
            let detail = failed[0].0["detail"].as_str().unwrap_or("").to_string();
            check_path_substring_cap(
                "event 12 io",
                &detail,
                &root.join(".factory").display().to_string(),
                &mut failures,
            );
        }
    }

    // (2) Event 12 `cause=state_integrity`: an unparseable txn record under a long path.
    {
        let (_keep, root) = long_project();
        let ms = root.join(".factory/migration-state");
        std::fs::create_dir_all(ms.join("reservations")).unwrap();
        std::fs::write(ms.join("exclusive.lock"), b"").unwrap();
        std::fs::write(ms.join("gate-state.json"), "\"OPEN\"").unwrap();
        std::fs::write(ms.join("txn-act-bad.json"), b"{ not json").unwrap();
        let logs = tempfile::tempdir().unwrap();
        let target = root.join(CYCLES_PATH);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let out = run_hook(
            &root,
            plugin_root.path(),
            logs.path(),
            &edit_envelope(&target, "toolu_f010b"),
        );
        let events = read_events(logs.path());
        let failed = of_type(&events, "migration.admission_failed");
        if out.status.code() != Some(2)
            || failed.len() != 1
            || failed[0].0["cause"] != "state_integrity"
        {
            failures.push(format!(
                "[event 12 state_integrity] setup: expected exit 2 and one cause=state_integrity \
                 admission_failed; got exit {:?}, {} failed event(s), stderr {:?}",
                out.status.code(),
                failed.len(),
                stderr_of(&out)
            ));
        } else {
            let detail = failed[0].0["detail"].as_str().unwrap_or("").to_string();
            check_path_substring_cap(
                "event 12 state_integrity",
                &detail,
                &ms.join("txn-act-bad.json").display().to_string(),
                &mut failures,
            );
        }
    }

    // (3) Event 13 `reservation_release_failed`: PostToolUse with an unstatable `.factory`.
    {
        let (_keep, root) = long_project();
        std::os::unix::fs::symlink(".factory", root.join(".factory")).unwrap();
        let logs = tempfile::tempdir().unwrap();
        let payload = json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "session_id": "sess-f010",
            "tool_use_id": "toolu_f010c",
            "tool_input": {},
        })
        .to_string();
        let _ = run_hook(&root, plugin_root.path(), logs.path(), &payload);
        let events = read_events(logs.path());
        let adv = of_type(&events, "migration.admission_advisory");
        if adv.len() != 1 || adv[0].0["reason"] != "reservation_release_failed" {
            failures.push(format!(
                "[event 13] setup: expected one reservation_release_failed advisory; got {} \
                 advisory event(s)",
                adv.len()
            ));
        } else {
            let detail = adv[0].0["detail"].as_str().unwrap_or("").to_string();
            check_path_substring_cap(
                "event 13",
                &detail,
                &root.join(".factory").display().to_string(),
                &mut failures,
            );
        }
    }
    assert_no_failures(
        "test_BC_3_08_001_F010_event_12_13_detail_data_derived_substrings_truncated_to_64_chars_blackbox",
        failures,
    );
}

// ===========================================================================
// F-014 -- the pub read_active_txn_record must not strict-decode a terminal record
// ===========================================================================

/// ADR-052 v1.24 item 3 / BC-1.18.011 v1.21 Precondition 6(f)(ii): "terminal (COMPLETED/
/// ABORTED) and foreign records are NEVER rejected for a missing Tier 1 field and never
/// modified". `read_active_txn_record` (pub) with NO live record falls back to the first
/// terminal record and currently strict-decodes it, so a COMPLETED / ABORTED record that
/// lacks Tier-1 fields (a minimal hand-built or foreign-schema terminal record) is rejected.
#[test]
fn test_BC_1_18_011_F014_read_active_txn_record_does_not_strict_decode_terminal_records() {
    let mut failures = Vec::new();
    for state in ["COMPLETED", "ABORTED"] {
        for (vlabel, rec) in [
            (
                "minimal {state, migration_id}",
                json!({"state": state, "migration_id": MIGRATION_ID}),
            ),
            (
                "missing generation_id / intent_log_path / pending moves",
                json!({
                    "state": state, "txn_id": "t", "activation_id": "a",
                    "fencing_generation": 1,
                    "created_at": "2026-10-07T00:00:00Z", "updated_at": "2026-10-07T00:00:00Z",
                }),
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join("txn-act-term.json"),
                serde_json::to_vec_pretty(&rec).unwrap(),
            )
            .unwrap();
            match read_active_txn_record(&StdFs, dir.path()) {
                Ok(_) => {}
                Err(e) => failures.push(format!(
                    "[{state}, {vlabel}] a terminal record must never be rejected for missing \
                     Tier-1 fields; got Err({e})"
                )),
            }
        }
    }
    // CONTROL: the LIVE record is still strict-decoded (the fix must not loosen it).
    {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("txn-act-live.json"),
            serde_json::to_vec_pretty(&json!({"state": "STAGING", "migration_id": MIGRATION_ID}))
                .unwrap(),
        )
        .unwrap();
        if read_active_txn_record(&StdFs, dir.path()).is_ok() {
            failures.push(
                "control: a live STAGING record missing its Tier-1 keys must still be rejected \
                 (strict-presence decode)"
                    .into(),
            );
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_F014_read_active_txn_record_does_not_strict_decode_terminal_records",
        failures,
    );
}

// ===========================================================================
// ADR-052 v1.25 items 11(f)(2) / 11(g): the per-surface truncation caps
// ===========================================================================
//
// The 64-character per-substring cap applies to InternalLog Events 11-13 `detail` ONLY.
// The operator stderr line and the error Display use 256 per substring; coordinator
// advisory `<subject>` and `<cause>` are capped at 256 EACH (`…` is the 256th character)
// and the fixed clause is never truncated; the admission entry points write NOTHING
// to stderr for a coordinator-advisory condition (11(g)3).

/// `sanitize_diagnostic` for a path with no control characters: whole when it fits,
/// else the first `max - 1` characters plus `…` (total `max`).
fn capped(full: &str, max: usize) -> String {
    if full.chars().count() <= max {
        full.to_string()
    } else {
        let mut s: String = full.chars().take(max - 1).collect();
        s.push('…');
        s
    }
}

/// (a) AdmissionStateIntegrity under a LONG project path: the coordinator's stderr Display
/// shows the txn-record path in FULL (<= 256), while the hook's Event 12 `detail` shows the
/// 64-character truncation of the SAME path. Today both are truncated to 64 (the cap is
/// applied inside the error's own `detail`), so the stderr half is RED.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v125_11g_state_integrity_stderr_full_path_event_12_detail_capped_64_blackbox()
 {
    let c = Coord::new_long(40, 2);
    let txn = c.ms().join("txn-act-bad.json");
    std::fs::write(&txn, b"{ not json").unwrap();
    let full = txn.display().to_string();
    let n = full.chars().count();
    assert!(
        n > 64 && n <= 256,
        "fixture path must exceed the event cap and fit the stderr cap: {n}"
    );
    let mut failures = Vec::new();

    // Operator surface: coordinator stderr (256 per substring => the full path).
    let out = c.run();
    let err = stderr_of(&out);
    if out.status.code() != Some(2)
        || !out.stdout.is_empty()
        || !err.contains("state integrity failure")
    {
        failures.push(format!(
            "setup: expected exit 2, empty stdout and a `state integrity failure` line; got \
             exit {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    if !err.contains(&full) {
        failures.push(format!(
            "stderr Display must carry the txn path in FULL ({n} chars, cap 256): {full:?}; \
             got {err:?}"
        ));
    }

    // Event surface: the hook's Event 12 `detail` (64 per substring).
    let plugin_root = tempfile::tempdir().unwrap();
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .unwrap();
    let logs = tempfile::tempdir().unwrap();
    let target = c.root().join(CYCLES_PATH);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let hook = run_hook(
        c.root(),
        plugin_root.path(),
        logs.path(),
        &edit_envelope(&target, "toolu_v125a"),
    );
    let events = read_events(logs.path());
    let failed = of_type(&events, "migration.admission_failed");
    if hook.status.code() != Some(2)
        || failed.len() != 1
        || failed[0].0["cause"] != "state_integrity"
    {
        failures.push(format!(
            "event setup: expected exit 2 and one cause=state_integrity event; got exit {:?}, \
             {} event(s)",
            hook.status.code(),
            failed.len()
        ));
    } else {
        let detail = failed[0].0["detail"].as_str().unwrap_or("");
        let want = capped(&full, 64);
        if !detail.starts_with(&format!("{want}: ")) || detail.contains(&full) {
            failures.push(format!(
                "Event 12 detail must start with the 64-char-capped path {want:?} and not carry \
                 the full path; got {detail:?}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v125_11g_state_integrity_stderr_full_path_event_12_detail_capped_64_blackbox",
        failures,
    );
}

/// (b) A coordinator advisory whose interpolated `<subject>` exceeds 256 characters: the
/// subject is capped at 256 with `…` as the 256th character, and the fixed clause is intact
/// (the same cap applies to `<cause>`; a >256-character OS message cannot be provoked, so the
/// subject, a >256-character gate path under a long project root, carries the vector).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v125_11f2_advisory_subject_capped_at_256_with_marker_clause_intact_blackbox()
 {
    let c = Coord::new_long(120, 3);
    committing_fixture(&c, json!([]));
    let gate = c.ms().join("gate-state.json");
    std::fs::remove_file(&gate).unwrap();
    std::fs::create_dir(&gate).unwrap();
    let full = gate.display().to_string();
    assert!(
        full.chars().count() > 256,
        "fixture gate path must exceed 256"
    );
    let out = c.run();
    let err = stderr_of(&out);
    let mut failures = Vec::new();
    let head = "migrate-bc-index: GATE_OPEN_RESET_FAILED (advisory): ";
    let want_subject = capped(&full, 256);
    let lines: Vec<&str> = err
        .lines()
        .filter(|l| l.contains("GATE_OPEN_RESET_FAILED"))
        .collect();
    if out.status.code() != Some(0) || lines.len() != 1 {
        failures.push(format!(
            "setup: expected exit 0 and ONE advisory line; got {:?}, stderr {err:?}",
            out.status.code()
        ));
    } else if let Some(rest) = lines[0].strip_prefix(head) {
        let Some(after) = rest.strip_prefix(&format!("{want_subject}: ")) else {
            failures.push(format!(
                "<subject> must be the 256-char cap of the gate path ({want_subject:?}, ending \
                 in `…`); got {:?}",
                rest.split(": ").next().unwrap_or("")
            ));
            return assert_no_failures("advisory_subject_cap", failures);
        };
        let Some((cause, clause)) = after.split_once("; ") else {
            failures.push(format!("no `; <clause>` after the cause in {:?}", lines[0]));
            return assert_no_failures("advisory_subject_cap", failures);
        };
        if cause.trim().is_empty() || cause.chars().count() > 256 {
            failures.push(format!(
                "<cause> must be non-empty and <= 256 chars: {cause:?}"
            ));
        }
        if !clause.starts_with("the migration completed; the gate stays ")
            && !clause.starts_with("the migration completed")
            || !clause.ends_with("reconciles it")
            || clause.contains('…')
        {
            failures.push(format!(
                "the fixed clause must be intact (never truncated): {clause:?}"
            ));
        }
    } else {
        failures.push(format!(
            "advisory line must start with {head:?}: {:?}",
            lines[0]
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v125_11f2_advisory_subject_capped_at_256_with_marker_clause_intact_blackbox",
        failures,
    );
}

/// (c) ADR-052 v1.25 item 11(g)3: the admission entry points (`migration_writer_admission`,
/// `migration_writer_release`, `read_active_txn_record`, driven through the real binary on
/// PreToolUse / PostToolUse) write NOTHING to stderr for the states that trip each
/// coordinator advisory: a terminal txn record whose archive rename cannot succeed, a leftover
/// staging directory, an unwritable gate file, and an over-cap-row-free clean tree. An
/// admitted run prints nothing; a fail-closed run prints only its single `E-MAINTENANCE-002`
/// line. No coordinator advisory token or `(advisory)` marker may ever appear.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v125_11g3_admission_entry_points_write_no_coordinator_advisory_to_stderr_blackbox()
 {
    use std::os::unix::fs::PermissionsExt as _;
    const TOKENS: [&str; 4] = [
        "GATE_OPEN_RESET_FAILED",
        "TERMINAL_TXN_ARCHIVE_FAILED",
        "STAGING_DIR_REMOVE_FAILED",
        "OVERSIZED_ROW_SUBSHARD",
    ];
    let plugin_root = tempfile::tempdir().unwrap();
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .unwrap();
    let mut failures = Vec::new();
    let is_root = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false);

    type Setup = fn(&Coord);
    let cases: Vec<(&str, Setup)> = vec![
        ("terminal COMPLETED txn record (archive candidate)", |c| {
            let mut v = committing_record(json!([]));
            v["state"] = json!("COMPLETED");
            std::fs::write(
                c.ms().join("txn-act-1.json"),
                serde_json::to_vec_pretty(&v).unwrap(),
            )
            .unwrap();
        }),
        (
            "terminal txn record in a read-only migration-state dir",
            |c| {
                let mut v = committing_record(json!([]));
                v["state"] = json!("COMPLETED");
                std::fs::write(
                    c.ms().join("txn-act-1.json"),
                    serde_json::to_vec_pretty(&v).unwrap(),
                )
                .unwrap();
                std::fs::set_permissions(c.ms(), std::fs::Permissions::from_mode(0o555)).unwrap();
            },
        ),
        ("leftover staging directory, no txn record", |c| {
            std::fs::create_dir_all(c.ms().join("gen-gen-orphan")).unwrap();
        }),
        (
            "gate-state.json is a directory (gate write would fail)",
            |c| {
                let gate = c.ms().join("gate-state.json");
                std::fs::remove_file(&gate).unwrap();
                std::fs::create_dir(&gate).unwrap();
            },
        ),
    ];
    for (label, setup) in cases {
        if label.contains("read-only") && is_root {
            eprintln!("SKIP [{label}]: running as root");
            continue;
        }
        for event in ["PreToolUse", "PostToolUse"] {
            let c = Coord::new();
            setup(&c);
            let logs = tempfile::tempdir().unwrap();
            let target = c.root().join(CYCLES_PATH);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            let payload = json!({
                "hook_event_name": event,
                "tool_name": "Edit",
                "session_id": "sess-v125g3",
                "tool_use_id": "toolu_v125g3",
                "tool_input": {
                    "file_path": target.to_string_lossy(),
                    "old_string": "a",
                    "new_string": "b",
                },
            })
            .to_string();
            let out = run_hook(c.root(), plugin_root.path(), logs.path(), &payload);
            // Restore so the tempdir can be removed.
            let _ = std::fs::set_permissions(c.ms(), std::fs::Permissions::from_mode(0o755));
            // The dispatcher's own always-on `factory-dispatcher trace=...` summary
            // (two lines on PreToolUse) is not an admission-entry-point write.
            let raw = stderr_of(&out);
            let err: String = raw
                .lines()
                .filter(|l| {
                    !l.starts_with("factory-dispatcher trace=") && !l.starts_with("  plugins_run=")
                })
                .collect::<Vec<_>>()
                .join("\n");
            let l = format!("{label} / {event}");
            if err.contains("(advisory)") || TOKENS.iter().any(|t| err.contains(t)) {
                failures.push(format!(
                    "[{l}] no coordinator advisory may reach stderr: {err:?}"
                ));
            }
            match out.status.code() {
                Some(0) if !err.is_empty() => {
                    failures.push(format!("[{l}] an admitted run must print nothing: {err:?}"));
                }
                // A fail-closed run's single `E-MAINTENANCE-002` line rides on the
                // dispatcher summary's `block_reason`; nothing else is written.
                Some(2) if !err.is_empty() || !raw.contains("E-MAINTENANCE-002") => {
                    failures.push(format!(
                        "[{l}] a fail-closed run writes only the E-MAINTENANCE-002 verdict \
                         (no extra stderr lines): {raw:?}"
                    ));
                }
                _ => {}
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v125_11g3_admission_entry_points_write_no_coordinator_advisory_to_stderr_blackbox",
        failures,
    );
}
