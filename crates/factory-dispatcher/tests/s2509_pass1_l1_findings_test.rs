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
//! Authority: ADR-052 v1.24 items 8, 11(c), 11(d); BC-1.18.011 v1.21 Precondition 6(f)(ii)
//! (1a), 6(f)(iii); BC-1.18.013 v1.13 Postcondition 10 (incl. 10(c) and the "Coordinators"
//! clause); BC-3.08.001 v1.37 Events 12 and 13 ("truncated to 64 characters for any
//! data-derived substring").
//!
//! | Finding | Test |
//! |---------|------|
//! | F-003 | `..._F003_drain_timeout_gate_open_write_failure_...` |
//! | F-004 (i)(ii) | `..._F004_dangling_symlink_completed_json_...`, `..._F004_eloop_completed_json_...`, control `..._F004_control_...` |
//! | F-004 (iii) | `..._F004_recover_receives_the_under_lock_read_...` (source gate + controls) |
//! | F-005 | `..._F005_admitter_own_release_failure_...`, `..._F005_main_rs_release_on_block_failure_...` |
//! | F-006 | `..._F006_finish_committing_gate_open_failure_...`, `..._F006_archive_terminal_record_failure_...`, `..._F006_canonical_move_halt_reasons_...` |
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
    MigrationLockGuard, read_active_txn_record, try_acquire_migration_lock,
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
}

impl Coord {
    fn new() -> Self {
        let c = Coord {
            dir: tempfile::tempdir().expect("project tempdir"),
        };
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
        self.dir.path()
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

/// `true` when some stderr line starts with the coordinator's one-line prefix
/// (`migrate-bc-index: `) and contains every needle.
fn stderr_line_with(err: &str, needles: &[&str]) -> bool {
    err.lines()
        .any(|l| l.starts_with("migrate-bc-index: ") && needles.iter().all(|n| l.contains(n)))
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

/// ADR-052 v1.24 item 11(c): "Acquired => THEN read `completed.json` (existence and content
/// ...) UNDER the lock; any pre-lock existence probe is a hint only and MUST NOT select the
/// branch ... absent => the recovery path". A `completed.json` that is a DANGLING SYMLINK is
/// not absent: an lstat sees an entry, `Path::exists()` (which follows the link) reports
/// `false`, and the current `fs.exists(..)` therefore starts a FRESH migration on a tree
/// with a completion marker. Fail-closed requirement: no fresh run, tree byte-identical.
/// (The ADR does not say which verdict -- the marker's presence short-circuits to the
/// gate-reconciled exit 0 `ALREADY_MIGRATED`, or an unreadable marker is `Io` exit 2; both
/// are accepted, a fresh run is not.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_dangling_symlink_completed_json_does_not_start_a_fresh_run_blackbox() {
    let c = Coord::new();
    std::os::unix::fs::symlink("no-such-target.json", c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let err = stderr_of(&out);
    let mut failures = Vec::new();
    assert_no_fresh_run("dangling completed.json", &c, &before, &mut failures);
    match out.status.code() {
        Some(0) => {}
        Some(2) if err.contains("completed.json") && err.starts_with("migrate-bc-index: ") => {}
        other => failures.push(format!(
            "expected exit 0 (marker present, gate reconciled) or exit 2 Io naming completed.json; \
             got {other:?}, stderr {err:?}"
        )),
    }
    if c.ms().join("completed.json").is_file() {
        failures.push("completed.json must remain a dangling symlink, not a written file".into());
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_dangling_symlink_completed_json_does_not_start_a_fresh_run_blackbox",
        failures,
    );
}

/// ADR-052 v1.24 item 11(c) + the stat-failure principle of BC-1.18.011 Precondition 7 /
/// ADR-052 v1.22 ("a non-ENOENT stat/read CALL failure leaves existence UNKNOWN => `Io`,
/// exit 2"): `completed.json` is a self-referential symlink (ELOOP, every uid). The current
/// `fs.exists(..)` maps the stat error to `false` ("absent") and starts a fresh run.
/// Required: exit 2, ONE stderr line that is the `Io` text naming `completed.json`, tree
/// unchanged.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_F004_eloop_completed_json_is_io_exit_2_not_a_fresh_run_blackbox() {
    let c = Coord::new();
    std::os::unix::fs::symlink("completed.json", c.ms().join("completed.json")).unwrap();
    let before = raw_tree(c.root());
    let out = c.run();
    let err = stderr_of(&out);
    let mut failures = Vec::new();
    assert_no_fresh_run("ELOOP completed.json", &c, &before, &mut failures);
    let io_prefix = "migrate-bc-index: BC-INDEX migration: I/O error at ";
    if out.status.code() != Some(2)
        || !out.stdout.is_empty()
        || err.lines().count() != 1
        || !err.starts_with(io_prefix)
        || !err.contains("completed.json")
    {
        failures.push(format!(
            "expected exit 2 and ONE `{io_prefix}...completed.json...` line; got exit {:?}, \
             stderr {err:?}",
            out.status.code()
        ));
    }
    assert_no_failures(
        "test_BC_1_18_011_F004_eloop_completed_json_is_io_exit_2_not_a_fresh_run_blackbox",
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
// The parenthetical lists EXAMPLES of tracing-only diagnostics; the operative words are
// "EACH failure or advisory they previously sent only to tracing". Nothing in the clause
// exempts a best-effort/non-fatal anomaly. The exact `E-...`/`<VARIANT>` token for the
// three anomalies below is not specified, so the tests pin only what the clause fixes: a
// stderr line in the coordinator's one-line `migrate-bc-index: ` form that names the
// anomaly's subject (path / step). Reported to the dispatcher as an ambiguity.

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

/// F-006 (a): `finish_committing_migration`'s best-effort gate-OPEN failure after the txn
/// reached COMPLETED. The migration still completes (exit 0, ADR-052 item 11(d): "the
/// gate-OPEN write after a SUCCESSFUL txn COMPLETED write stays best-effort"), but the
/// failure is an advisory the Coordinators clause sends to stderr.
#[test]
fn test_BC_1_18_013_F006_finish_committing_gate_open_failure_is_a_stderr_line_blackbox() {
    let c = Coord::new();
    committing_fixture(&c, json!([]));
    let gate = c.ms().join("gate-state.json");
    std::fs::remove_file(&gate).unwrap();
    std::fs::create_dir(&gate).unwrap();
    let out = c.run();
    let err = stderr_of(&out);
    let mut failures = Vec::new();
    if out.status.code() != Some(0) {
        failures.push(format!(
            "setup/contract: the best-effort gate reset must not change the Completed outcome \
             (exit 0); got {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    if !c.ms().join("completed.json").is_file() {
        failures.push("setup: the forward recovery must have written completed.json".into());
    }
    if !stderr_line_with(&err, &["gate-state.json"]) {
        failures.push(format!(
            "Postcondition 10 Coordinators clause: the failed gate-OPEN write must be written \
             to stderr as one `migrate-bc-index: `-prefixed line naming gate-state.json; \
             stderr {err:?}"
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_finish_committing_gate_open_failure_is_a_stderr_line_blackbox",
        failures,
    );
}

/// F-006 (b): `archive_terminal_txn_record`'s best-effort archive failure. A stale ABORTED
/// record is archived before the fresh run; the archive name is occupied by a non-empty
/// directory so the rename fails with a non-ENOENT error. The fresh run completes (exit 0)
/// and the failure must reach stderr.
#[test]
fn test_BC_1_18_013_F006_archive_terminal_record_failure_is_a_stderr_line_blackbox() {
    let c = Coord::new();
    std::fs::write(
        c.ms().join("txn-act-old.json"),
        serde_json::to_vec_pretty(&json!({"state": "ABORTED", "migration_id": MIGRATION_ID}))
            .unwrap(),
    )
    .unwrap();
    let blocker = c.ms().join("txn-act-old.json.archived");
    std::fs::create_dir(&blocker).unwrap();
    std::fs::write(blocker.join("pin"), b"x").unwrap();
    let out = c.run();
    let err = stderr_of(&out);
    let mut failures = Vec::new();
    if out.status.code() != Some(0) || !c.ms().join("completed.json").is_file() {
        failures.push(format!(
            "setup/contract: the best-effort archive failure must not change the outcome (fresh \
             run completes, exit 0); got {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    if !stderr_line_with(&err, &["txn-act-old.json"]) {
        failures.push(format!(
            "Postcondition 10 Coordinators clause: the failed archive of the stale terminal \
             record must be written to stderr as one `migrate-bc-index: `-prefixed line naming \
             txn-act-old.json; stderr {err:?}"
        ));
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_archive_terminal_record_failure_is_a_stderr_line_blackbox",
        failures,
    );
}

/// F-006 (c): `execute_canonical_path_moves` halt reasons. The final error only says
/// "canonical path moves halted after N/M"; the REASON (which step failed, on which path) is
/// tracing-only. Each halt reason must be a stderr line naming the halted move's path.
#[test]
fn test_BC_1_18_013_F006_canonical_move_halt_reasons_are_stderr_lines_blackbox() {
    let mut failures = Vec::new();

    // Halt 1: the rename fails (staging file missing => ENOENT).
    {
        let c = Coord::new();
        let staging = c.ms().join("gen-gen-1/missing-staged.md");
        let canonical = c
            .root()
            .join(".factory/specs/behavioral-contracts/shards/halt1.md");
        committing_fixture(
            &c,
            json!([{
                "staging_path": staging.to_string_lossy(),
                "canonical_path": canonical.to_string_lossy(),
            }]),
        );
        let out = c.run();
        let err = stderr_of(&out);
        if out.status.code() != Some(2) {
            failures.push(format!(
                "[rename failure] expected the halted-moves exit 2; got {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        if !stderr_line_with(&err, &["missing-staged.md", "No such file"]) {
            failures.push(format!(
                "[rename failure] the halt reason (the failed rename of missing-staged.md and \
                 its OS error) must be a `migrate-bc-index: `-prefixed stderr line; stderr {err:?}"
            ));
        }
    }

    // Halt 2: the canonical parent directory cannot be created (a regular file occupies
    // the parent path).
    {
        let c = Coord::new();
        let staging = c.ms().join("gen-gen-1/staged.md");
        std::fs::create_dir_all(c.ms().join("gen-gen-1")).unwrap();
        std::fs::write(&staging, b"staged").unwrap();
        let blocker = c.root().join(".factory/specs/behavioral-contracts/shards");
        std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
        std::fs::write(&blocker, b"i am a file, not a directory").unwrap();
        let canonical = blocker.join("halt2.md");
        committing_fixture(
            &c,
            json!([{
                "staging_path": staging.to_string_lossy(),
                "canonical_path": canonical.to_string_lossy(),
            }]),
        );
        let out = c.run();
        let err = stderr_of(&out);
        if out.status.code() != Some(2) {
            failures.push(format!(
                "[parent dir failure] expected the halted-moves exit 2; got {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        if !stderr_line_with(&err, &["shards"]) {
            failures.push(format!(
                "[parent dir failure] the halt reason (canonical parent dir `shards` cannot be \
                 created) must be a `migrate-bc-index: `-prefixed stderr line naming the \
                 parent; stderr {err:?}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_F006_canonical_move_halt_reasons_are_stderr_lines_blackbox",
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
