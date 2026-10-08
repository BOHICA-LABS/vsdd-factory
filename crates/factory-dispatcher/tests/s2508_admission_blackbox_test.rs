// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 (T-1) RED-GATE black-box suite for the SHARED ADMISSION CORE wired
//! on the dispatcher's REAL PreToolUse/PostToolUse path.
//!
//! Every test in this file SPAWNS THE REAL `factory-dispatcher` BINARY
//! (`CARGO_BIN_EXE_factory-dispatcher`), writes a Claude-Code-shaped hook
//! envelope to its stdin (then closes stdin), and observes only process
//! outcomes and the on-disk `.factory/migration-state/` tree of a tempdir
//! project. No internal function is called (Claim boundary, ADR-052 v1.18 /
//! story S-25.08: a unit test cannot see a missing call site).
//!
//! # Traceability
//!
//! | Test | AC | BC clause | VP facet |
//! |------|----|-----------|----------|
//! | `test_BC_1_18_013_PC6b_gate_wired_on_production_path_blackbox` | AC-001 | BC-1.18.013 Pre 6(b), Inv 6(c), EC-013 | VP-143 D1 |
//! | `test_BC_1_18_013_PC6b_single_shared_core_evaluated_once_ahead_of_shard_cap_precheck` | AC-002 | BC-1.18.013 Pre 6(b) "Where", EC-013 | VP-143 D1 |
//! | `test_BC_1_18_013_PC6c_reservation_created_on_pretooluse_removed_on_posttooluse_blackbox` | AC-003 | BC-1.18.013 Pre 6(b)/6(c), EC-013 | VP-143 D1 / VP-133 f7 |
//! | `test_BC_1_18_013_EC014_bash_left_unprocessed_no_reservation_no_state_write_blackbox` | AC-004 | BC-1.18.013 Pre 6(b) "Which events / tools" + "Exemption", EC-014 | VP-143 D1 |
//! | `test_BC_1_18_013_PC6c_reserve_then_verify_all_interleavings` | AC-005 | BC-1.18.013 Pre 6(c), EC-017 | VP-143 D5 / VP-133 f7 |
//! | `test_BC_1_18_011_EC007_b2_reserve_then_verify_all_interleavings` | AC-005/AC-009 | BC-1.18.011 Pre 6(c), EC-007 | VP-133 f7 |
//! | `test_BC_1_18_013_EC018_later_stage_block_releases_reservation_blackbox` | AC-006 | BC-1.18.013 Pre 6(c) release-on-block, EC-018; BC-1.18.011 EC-008 | VP-133 f7 |
//! | `test_BC_1_18_011_PC6d_reconciliation_branches_ABC_wired_on_production_path_blackbox` | AC-007 | BC-1.18.011 Pre 6(d), EC-010; BC-1.18.013 Post 5a(c), Inv 6(c) | VP-133 f6 |
//! | `test_BC_1_18_011_PC6c_b2_delegates_to_shared_core_no_check_then_reserve_race` | AC-009 | BC-1.18.011 Pre 6(c), EC-007, EC-008 | VP-133 f7 |
//!
//! (AC-008 and the pure decision cores live in `s2508_pure_core_and_ttl_test.rs`;
//! AC-009's in-process B2 entry-point tests and the T-2 fixture updates live in
//! `bc_1_18_011_b2_migration_test.rs`.)
//!
//! # TEST-ONLY SEAM the implementer MUST provide (Red-Gate note, BC-5.38.001)
//!
//! Reserve-then-verify (D5) is an ORDERING property INSIDE one dispatcher
//! process (W1 = create reservation, then W2 = read gate/scan txn). A
//! black-box test can only observe/interleave it through an injectable seam.
//! BC-1.18.013 Precondition 6(c) already mandates injectable timing seams; this
//! file defines the admission-core seam it needs. No production code reads it
//! yet (so these tests are RED for the right reason: the seam and the core
//! behavior do not exist). The seam is active ONLY in
//! `#[cfg(any(debug_assertions, feature = "test-support"))]` builds (same gate
//! as `VSDD_FORCE_ENGINE_BUILD_FAILURE` in `main.rs`) and is keyed on ONE env
//! var:
//!
//! `VSDD_TEST_ADMISSION_SEAM_DIR=<dir>` -- when set, the shared admission core:
//!   1. appends one line per event to `<dir>/events.log`, in execution order,
//!      IMMEDIATELY AFTER the effect it names:
//!        - `W1_RESERVE:<tool_use_id>` after the reservation file is durably
//!          created (W1);
//!        - `W2_VERIFY` after it has read `gate-state.json` and scanned
//!          `txn-*.json` for the first time (W2);
//!        - `RECONCILE` each time the step-3.5 reconciliation procedure runs.
//!   2. after writing `W1_RESERVE:*`, if the file `<dir>/pause-after-w1`
//!      exists, polls (<=30 s, <=10 ms interval) until `<dir>/resume` exists
//!      before proceeding to W2 -- this lets a test place C1/C2 coordinator
//!      steps BETWEEN W1 and W2.
//!
//! # Fixture conventions
//!
//! * A "live coordinator" is modelled by HOLDING `exclusive.lock`
//!   (`try_acquire_migration_lock`) for the test's duration (T-2 principle: a
//!   fixture that models STAGING must hold the lock, otherwise Branch B
//!   legitimately self-heals it).
//! * Txn fixtures carry the v1.11 `migration_id` field
//!   (`"migrate-bc-index"` | `"backfill-append-logs"` | absent = B2).
//!
//! # Message format (BC-1.18.013 v1.6 Pre 6(b); error-taxonomy v1.35)
//!
//! `<scope> write blocked: migration window active (txn record in STAGING or
//! COMMITTING state); retry after migration completes or aborts`, where
//! `<scope>` is keyed on the WRITTEN PATH FAMILY (`BC-INDEX` under
//! `.factory/specs/behavioral-contracts/`, `.factory/cycles/` under
//! `.factory/cycles/`), never on the live migration. A STAGING + terminal-record
//! block appends ` (completion-record mismatch -- operator investigation
//! required)` (em dash U+2014); foreign-migration and EWOULDBLOCK blocks are the
//! plain message.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use factory_dispatcher::shard_manager::{MigrationLockGuard, try_acquire_migration_lock};

const SEAM_ENV: &str = "VSDD_TEST_ADMISSION_SEAM_DIR";
const MISMATCH_SUFFIX: &str =
    " (completion-record mismatch \u{2014} operator investigation required)";

/// Exact E-MAINTENANCE-001 message for a written-path family.
fn plain_msg(scope: &str) -> String {
    format!(
        "{scope} write blocked: migration window active (txn record in STAGING or COMMITTING \
         state); retry after migration completes or aborts"
    )
}

/// `<scope>` keyed on the written path family.
fn scope_of(rel: &str) -> &'static str {
    if rel.starts_with(".factory/cycles/") {
        ".factory/cycles/"
    } else {
        "BC-INDEX"
    }
}
const CYCLES_PATH: &str = ".factory/cycles/c1/burst-log.md";
const BC_PATH: &str = ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

struct Project {
    dir: tempfile::TempDir,
    plugin_root: tempfile::TempDir,
}

impl Project {
    /// Fresh project: empty registry (zero `[[hooks]]`, so ONLY native gates
    /// can act), `.factory/migration-state/{reservations/}` present, gate OPEN.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("project tempdir");
        let plugin_root = tempfile::tempdir().expect("plugin_root tempdir");
        std::fs::write(
            plugin_root.path().join("hooks-registry.toml"),
            "schema_version = 2\n",
        )
        .expect("write empty registry");
        let p = Project { dir, plugin_root };
        std::fs::create_dir_all(p.ms().join("reservations")).unwrap();
        std::fs::write(p.ms().join("exclusive.lock"), b"").unwrap();
        write_gate(&p.ms(), "OPEN");
        p
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

    fn abs(&self, rel: &str) -> PathBuf {
        let p = self.root().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        p
    }

    /// Model a LIVE coordinator: hold `exclusive.lock` until the guard drops.
    fn hold_lock(&self) -> MigrationLockGuard {
        try_acquire_migration_lock(&self.ms().join("exclusive.lock"))
            .expect("lock open")
            .expect("lock must be free in fixture setup")
    }

    fn reservation_count(&self) -> usize {
        std::fs::read_dir(self.ms().join("reservations"))
            .map(|it| it.flatten().count())
            .unwrap_or(0)
    }

    /// Byte-for-byte snapshot of the whole migration-state tree.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                let p = e.path();
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
                if p.is_dir() {
                    out.insert(format!("{rel}/"), Vec::new());
                    walk(base, &p, out);
                } else {
                    out.insert(rel, std::fs::read(&p).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.ms(), &self.ms(), &mut out);
        out
    }

    fn gate(&self) -> String {
        std::fs::read_to_string(self.ms().join("gate-state.json"))
            .unwrap_or_default()
            .trim()
            .trim_matches('"')
            .to_string()
    }

    fn txn_json(&self) -> serde_json::Value {
        let p = std::fs::read_dir(self.ms())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                n.starts_with("txn-") && n.ends_with(".json")
            })
            .expect("a txn-*.json must exist (retained)");
        serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
    }
}

fn write_gate(ms: &Path, state: &str) {
    std::fs::create_dir_all(ms).unwrap();
    let tmp = ms.join("gate-state.json.tmp");
    std::fs::write(&tmp, format!("\"{state}\"")).unwrap();
    std::fs::rename(tmp, ms.join("gate-state.json")).unwrap();
}

/// Write a txn record. `migration_id = None` models a pre-v1.11 record (read as
/// `migrate-bc-index`).
fn write_txn(ms: &Path, state: &str, generation_id: Option<&str>, migration_id: Option<&str>) {
    std::fs::create_dir_all(ms).unwrap();
    let mut v = serde_json::json!({
        "txn_id": "txn-s2508",
        "activation_id": "act-s2508",
        "fencing_generation": 1,
        "state": state,
        "generation_id": generation_id,
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": null,
        "pending_canonical_moves": [],
        "created_at": "2026-10-06T00:00:00Z",
        "updated_at": "2026-10-06T00:00:00Z",
    });
    if let Some(m) = migration_id {
        v["migration_id"] = serde_json::Value::String(m.to_string());
    }
    std::fs::write(
        ms.join("txn-act-s2508.json"),
        serde_json::to_vec_pretty(&v).unwrap(),
    )
    .unwrap();
}

fn write_terminal_record(ms: &Path, file_name: &str, txn_id: &str, generation_id: &str) {
    let v = serde_json::json!({
        "generation_id": generation_id,
        "txn_id": txn_id,
        "completed_at": "2026-10-06T00:00:00Z",
        "canonical_paths_count": 4,
    });
    std::fs::write(ms.join(file_name), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

fn envelope(
    event: &str,
    tool: &str,
    tool_use_id: Option<&str>,
    tool_input: serde_json::Value,
) -> String {
    let mut v = serde_json::json!({
        "hook_event_name": event,
        "tool_name": tool,
        "session_id": "sess-s2508",
        "tool_input": tool_input,
    });
    if let Some(id) = tool_use_id {
        v["tool_use_id"] = serde_json::Value::String(id.to_string());
    }
    v.to_string()
}

fn edit_input(abs_path: &Path) -> serde_json::Value {
    serde_json::json!({
        "file_path": abs_path.to_string_lossy(),
        "old_string": "a",
        "new_string": "b",
    })
}

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_factory-dispatcher"))
}

/// Spawn the real binary, write `payload`, CLOSE stdin (never left open).
fn spawn(project: &Project, payload: &str, seam_dir: Option<&Path>) -> Child {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", project.plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", project.root())
        .env("VSDD_LOG_DIR", project.root().join("logs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(d) = seam_dir {
        cmd.env(SEAM_ENV, d);
    } else {
        cmd.env_remove(SEAM_ENV);
    }
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    child
}

/// Wait for exit with a hard per-command timeout (kills + panics on expiry).
fn finish(mut child: Child, timeout: Duration) -> Output {
    let start = Instant::now();
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return child.wait_with_output().expect("wait_with_output");
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("factory-dispatcher did not exit within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn run(project: &Project, payload: &str) -> Output {
    finish(spawn(project, payload, None), Duration::from_secs(30))
}

fn run_with_seam(project: &Project, payload: &str, seam: &Path) -> Output {
    finish(spawn(project, payload, Some(seam)), Duration::from_secs(30))
}

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn events(seam: &Path) -> Vec<String> {
    std::fs::read_to_string(seam.join("events.log"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Poll until `events.log` contains a `W1_RESERVE:*` line. Returns `Err` as soon
/// as the child exits without ever producing it (seam/core absent => RED) or
/// after `deadline`.
fn wait_for_w1(child: &mut Child, seam: &Path, deadline: Duration) -> Result<(), String> {
    let start = Instant::now();
    loop {
        if events(seam).iter().any(|e| e.starts_with("W1_RESERVE:")) {
            return Ok(());
        }
        if let Some(st) = child.try_wait().expect("try_wait") {
            return Err(format!(
                "dispatcher exited ({st}) without recording W1_RESERVE in the admission seam \
                 (shared admission core / {SEAM_ENV} seam not implemented)"
            ));
        }
        if start.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timed out waiting for W1_RESERVE in the admission seam".to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// AC-001 -- D1: gate wired on the production path (black-box)
// ---------------------------------------------------------------------------

/// AC-001 (BC-1.18.013 Precondition 6(b), Invariant 6(c), EC-013; VP-143 D1).
#[test]
fn test_BC_1_18_013_PC6b_gate_wired_on_production_path_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    // (state, label): STAGING and COMMITTING both block; live coordinator holds
    // exclusive.lock so no self-heal is in play.
    for state in ["STAGING", "COMMITTING"] {
        for tool in ["Edit", "Write", "MultiEdit"] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), state, Some("gen-1"), Some("backfill-append-logs"));
            let _live = p.hold_lock();
            let target = p.abs(CYCLES_PATH);
            let payload = envelope("PreToolUse", tool, Some("T1"), edit_input(&target));
            let out = run(&p, &payload);
            let err = stderr_of(&out);
            if out.status.code() != Some(2) {
                failures.push(format!(
                    "{tool}/{state} under .factory/cycles/: expected exit 2, got {:?}",
                    out.status.code()
                ));
            }
            if !err.contains("E-MAINTENANCE-001") {
                failures.push(format!(
                    "{tool}/{state}: stderr lacks E-MAINTENANCE-001: {err}"
                ));
            }
            // Taxonomy v1.35 exact message, <scope> = `.factory/cycles/`.
            let want = plain_msg(".factory/cycles/");
            if !err.contains(&want) {
                failures.push(format!(
                    "{tool}/{state}: block message lacks `{want}` (error-taxonomy v1.35 \
                     E-MAINTENANCE-001 format); stderr: {err}"
                ));
            }
            if p.reservation("T1").exists() {
                failures.push(format!(
                    "{tool}/{state}: a blocked admission left T1.reservation"
                ));
            }
        }
    }

    // A path OUTSIDE the protected union is admitted even while a txn is live.
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "STAGING",
            Some("gen-1"),
            Some("backfill-append-logs"),
        );
        let _live = p.hold_lock();
        let target = p.abs(".factory/STATE.md");
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("T2"), edit_input(&target)),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                ".factory/STATE.md (outside union) must be admitted, got exit {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if p.reservation("T2").exists() {
            failures.push("outside-union path must not create a reservation".to_string());
        }
    }

    // Gate OPEN + no live txn => admitted.
    {
        let p = Project::new();
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("T3"), edit_input(&target)),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "gate OPEN + no txn under .factory/cycles/ must be admitted, got {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
    }

    // UNION + keying: a live txn of EITHER migration blocks BOTH path families;
    // the message is keyed on the WRITTEN PATH FAMILY (2 strings, 4 combinations).
    for (mig, label) in [
        (Some("migrate-bc-index"), "migrate-bc-index"),
        (None, "B2 (migration_id absent)"),
        (Some("backfill-append-logs"), "backfill-append-logs"),
    ] {
        for rel in [BC_PATH, CYCLES_PATH] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), "COMMITTING", Some("gen-1"), mig);
            let _live = p.hold_lock();
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TU"), edit_input(&target)),
            );
            let err = stderr_of(&out);
            let want = plain_msg(scope_of(rel));
            if out.status.code() != Some(2)
                || !err.contains("E-MAINTENANCE-001")
                || !err.contains(&want)
                || err.contains("completion-record mismatch")
            {
                failures.push(format!(
                    "keying: live {label} txn writing {rel} must block with exactly `{want}`; \
                     got exit {:?}, stderr: {err}",
                    out.status.code()
                ));
            }
        }
    }

    // Gate-only block (DRAINING, no txn, live coordinator holds the lock): same
    // path-family message.
    for rel in [BC_PATH, CYCLES_PATH] {
        let p = Project::new();
        write_gate(&p.ms(), "DRAINING");
        let _live = p.hold_lock();
        let target = p.abs(rel);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TG"), edit_input(&target)),
        );
        let err = stderr_of(&out);
        let want = plain_msg(scope_of(rel));
        if out.status.code() != Some(2)
            || !err.contains(&want)
            || err.contains("completion-record mismatch")
        {
            failures.push(format!(
                "gate-only (DRAINING, no txn, lock held) writing {rel} must block with exactly \
                 `{want}`; got exit {:?}, stderr: {err}",
                out.status.code()
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_PC6b_gate_wired_on_production_path_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-002 -- D1: ONE shared core, evaluated exactly once, ahead of shard_cap
// ---------------------------------------------------------------------------

const CYCLES_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"burst-log\"
artifact_path = \".factory/cycles/c1/burst-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100
shape = \"flat\"
";

/// AC-002 (BC-1.18.013 Precondition 6(b) "Where", EC-013; VP-143 D1).
#[test]
fn test_BC_1_18_013_PC6b_single_shared_core_evaluated_once_ahead_of_shard_cap_precheck() {
    let mut failures: Vec<String> = Vec::new();

    // Part 1: ONE evaluation. Gate LOCKED + no txn + lock FREE => Branch A runs
    // exactly once, reservation is created exactly once, then admitted.
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        let seam = tempfile::tempdir().unwrap();
        let target = p.abs(CYCLES_PATH);
        let out = run_with_seam(
            &p,
            &envelope("PreToolUse", "Edit", Some("T2"), edit_input(&target)),
            seam.path(),
        );
        let ev = events(seam.path());
        let w1 = ev.iter().filter(|e| e.starts_with("W1_RESERVE:")).count();
        let rec = ev.iter().filter(|e| e.as_str() == "RECONCILE").count();
        if w1 != 1 {
            failures.push(format!(
                "reservation must be written exactly ONCE per PreToolUse event (single \
                 evaluation), saw {w1} W1_RESERVE events: {ev:?}"
            ));
        }
        if rec != 1 {
            failures.push(format!(
                "reconciliation must run exactly ONCE (not re-run by a second evaluation), saw \
                 {rec} RECONCILE events: {ev:?}"
            ));
        }
        let w1_pos = ev.iter().position(|e| e.starts_with("W1_RESERVE:"));
        let w2_pos = ev.iter().position(|e| e.as_str() == "W2_VERIFY");
        match (w1_pos, w2_pos) {
            (Some(a), Some(b)) if a < b => {}
            _ => failures.push(format!("W1_RESERVE must precede W2_VERIFY: {ev:?}")),
        }
        if out.status.code() != Some(0) {
            failures.push(format!(
                "Branch A self-heal then admit expected exit 0, got {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if !p.reservation("T2").exists() {
            failures.push(
                "admitted PreToolUse must leave T2.reservation until PostToolUse".to_string(),
            );
        }
    }

    // Part 2: structurally AHEAD of shard_cap_precheck. A blocked protected-path
    // write that is also genuinely over-cap must NOT reach the destructive
    // seal-and-truncate (execute_roll).
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "COMMITTING",
            Some("gen-1"),
            Some("backfill-append-logs"),
        );
        let _live = p.hold_lock();
        std::fs::write(
            p.root().join(".factory/shard-config.toml"),
            CYCLES_SHARD_CONFIG,
        )
        .unwrap();
        let target = p.abs(CYCLES_PATH);
        std::fs::write(&target, "y".repeat(50)).unwrap();
        let before = std::fs::read(&target).unwrap();
        let seam = tempfile::tempdir().unwrap();
        let payload = envelope(
            "PreToolUse",
            "Write",
            Some("T2b"),
            serde_json::json!({
                "file_path": target.to_string_lossy(),
                "content": "x".repeat(5_000),
            }),
        );
        let out = run_with_seam(&p, &payload, seam.path());
        if out.status.code() != Some(2) {
            failures.push(format!(
                "blocked over-cap protected write must exit 2, got {:?}",
                out.status.code()
            ));
        }
        if std::fs::read(&target).unwrap() != before {
            failures
                .push("shard_cap_precheck's destructive truncate ran on a BLOCKED write".into());
        }
        let sealed = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .flatten()
            .count();
        if sealed != 1 {
            failures.push(format!(
                "a sealed shard was published for a blocked write ({sealed} files)"
            ));
        }
        let ev = events(seam.path());
        if !ev.iter().any(|e| e.starts_with("W1_RESERVE:")) {
            failures.push(format!(
                "the shared core must have been evaluated for the protected-path write \
                 (no W1_RESERVE event): {ev:?}"
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_PC6b_single_shared_core_evaluated_once_ahead_of_shard_cap_precheck",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-003 -- reservation lifecycle across Pre/Post (black-box)
// ---------------------------------------------------------------------------

/// AC-003 (BC-1.18.013 Precondition 6(b)/6(c) "Who creates a reservation", EC-013).
#[test]
fn test_BC_1_18_013_PC6c_reservation_created_on_pretooluse_removed_on_posttooluse_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let p = Project::new();
    let seam = tempfile::tempdir().unwrap();
    let target = p.abs(CYCLES_PATH);

    // PreToolUse T9 => reservation created (created_at ISO-8601 + tool_use_id).
    let out = run_with_seam(
        &p,
        &envelope("PreToolUse", "Edit", Some("T9"), edit_input(&target)),
        seam.path(),
    );
    if out.status.code() != Some(0) {
        failures.push(format!(
            "PreToolUse admit expected exit 0, got {:?}",
            out.status.code()
        ));
    }
    let res = p.reservation("T9");
    match std::fs::read(&res) {
        Err(e) => failures.push(format!("T9.reservation missing after PreToolUse: {e}")),
        Ok(bytes) => {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
            if v["tool_use_id"] != "T9" {
                failures.push(format!("reservation tool_use_id != T9: {v}"));
            }
            let created = v["created_at"].as_str().unwrap_or("");
            if chrono::DateTime::parse_from_rfc3339(created).is_err() {
                failures.push(format!(
                    "reservation created_at not ISO-8601/RFC3339: {created:?}"
                ));
            }
            if v.get("pid").is_some() || v.get("start_time").is_some() {
                failures.push("reservation must carry no PID/start-time".to_string());
            }
        }
    }
    // Reserve-then-verify is observable even for an admitted call.
    let ev = events(seam.path());
    if ev.first().map(String::as_str) != Some("W1_RESERVE:T9") {
        failures.push(format!(
            "the FIRST admission event must be W1_RESERVE:T9 (reservation before gate read): {ev:?}"
        ));
    }
    if std::fs::read_dir(p.ms().join("reservations"))
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().contains(".tmp"))
    {
        failures.push("a temp file leaked in reservations/ (atomic temp+rename expected)".into());
    }

    // PostToolUse for an UNKNOWN id is a no-op (T9 untouched).
    let out = run(
        &p,
        &envelope("PostToolUse", "Edit", Some("UNKNOWN"), edit_input(&target)),
    );
    if out.status.code() != Some(0) {
        failures.push(format!(
            "PostToolUse(unknown) must exit 0, got {:?}",
            out.status.code()
        ));
    }
    if !res.exists() {
        failures.push("PostToolUse for an unknown id must not remove T9.reservation".to_string());
    }

    // PostToolUse for T9 removes exactly that file (tool failure shape included).
    let mut post: serde_json::Value = serde_json::from_str(&envelope(
        "PostToolUse",
        "Edit",
        Some("T9"),
        edit_input(&target),
    ))
    .unwrap();
    post["tool_response"] = serde_json::json!({"is_error": true, "error": "tool failed"});
    // a second, unrelated reservation must survive
    std::fs::write(
        p.reservation("OTHER"),
        r#"{"created_at":"2026-10-06T00:00:00Z","tool_use_id":"OTHER"}"#,
    )
    .unwrap();
    let out = run(&p, &post.to_string());
    if out.status.code() != Some(0) {
        failures.push(format!(
            "PostToolUse(T9) must exit 0, got {:?}",
            out.status.code()
        ));
    }
    if res.exists() {
        failures
            .push("PostToolUse(T9) must remove T9.reservation (on tool success OR failure)".into());
    }
    if !p.reservation("OTHER").exists() {
        failures.push("PostToolUse(T9) must remove ONLY T9's reservation".to_string());
    }

    assert_no_failures(
        "test_BC_1_18_013_PC6c_reservation_created_on_pretooluse_removed_on_posttooluse_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-004 -- EC-014: Bash left unprocessed (regression guard + live-core control)
// ---------------------------------------------------------------------------

/// AC-004 (BC-1.18.013 Precondition 6(b) "Which events / tools" + "Exemption", EC-014).
///
/// Bash is ALREADY unprocessed in the merged code, so a Bash-only assertion
/// would be GREEN at the Red Gate (a pure regression lock). To keep this test
/// meaningful AND red-for-the-right-reason, it asserts the DIFFERENTIAL that
/// matters for the implementer who wires the shared core: in the SAME fixture,
/// an `Edit` control IS processed by the core (W1_RESERVE recorded) while the
/// `Bash` envelopes are NOT (no reservation, no event, migration-state
/// byte-identical).
#[test]
fn test_BC_1_18_013_EC014_bash_left_unprocessed_no_reservation_no_state_write_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let p = Project::new();
    write_gate(&p.ms(), "LOCKED");
    write_txn(
        &p.ms(),
        "COMMITTING",
        Some("gen-1"),
        Some("backfill-append-logs"),
    );
    let _live = p.hold_lock();
    let protected = p.abs(CYCLES_PATH);

    // Control: an Edit under the protected union IS evaluated by the core.
    let ctl_seam = tempfile::tempdir().unwrap();
    let _ = run_with_seam(
        &p,
        &envelope("PreToolUse", "Edit", Some("TC"), edit_input(&protected)),
        ctl_seam.path(),
    );
    if !events(ctl_seam.path())
        .iter()
        .any(|e| e.starts_with("W1_RESERVE:"))
    {
        failures.push(
            "control Edit was not evaluated by the shared admission core (no W1_RESERVE): the \
             core is not wired, so a Bash-unprocessed assertion would be vacuous"
                .to_string(),
        );
    }

    let before = p.snapshot();
    let coordinator_cmd = format!(
        "{}/target/release/factory-dispatcher backfill-append-logs",
        p.root().display()
    );
    let commands = [
        format!("echo x >> {}", protected.display()),
        coordinator_cmd.clone(),
        format!("{coordinator_cmd} --census"),
    ];
    for (i, cmd) in commands.iter().enumerate() {
        let id = format!("TB{i}");
        let seam = tempfile::tempdir().unwrap();
        let out = run_with_seam(
            &p,
            &envelope(
                "PreToolUse",
                "Bash",
                Some(&id),
                serde_json::json!({ "command": cmd }),
            ),
            seam.path(),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "Bash envelope #{i} must be left unprocessed (exit 0, no decision), got {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if p.reservation(&id).exists() {
            failures.push(format!(
                "Bash envelope #{i} created a reservation (must create none)"
            ));
        }
        if !events(seam.path()).is_empty() {
            failures.push(format!(
                "Bash envelope #{i} reached the admission core (events: {:?}); Bash is UNPROCESSED",
                events(seam.path())
            ));
        }
    }
    // The control Edit's own (blocked => removed) reservation must also be gone,
    // so the whole tree equals the pre-Bash snapshot.
    if p.snapshot() != before {
        failures.push(".factory/migration-state/ changed across Bash envelopes".to_string());
    }

    assert_no_failures(
        "test_BC_1_18_013_EC014_bash_left_unprocessed_no_reservation_no_state_write_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-005 -- D5: reserve-then-verify over every Dekker ordering
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    W1,
    W2,
    C1,
    C2,
}

/// Every interleaving of the chains W1<W2 and C1<C2 (C(4,2) = 6 orderings).
fn all_orderings() -> Vec<[Step; 4]> {
    use Step::*;
    vec![
        [W1, W2, C1, C2],
        [W1, C1, W2, C2],
        [W1, C1, C2, W2],
        [C1, W1, W2, C2],
        [C1, W1, C2, W2],
        [C1, C2, W1, W2],
    ]
}

struct OrderingResult {
    c2_saw_reservation: bool,
    exit_code: Option<i32>,
    reservation_after: bool,
    stderr: String,
}

/// Drive ONE ordering against the real binary. W-steps are the dispatcher's
/// own W1/W2 (paused via the seam); C-steps are the scripted coordinator.
fn drive_ordering(
    order: &[Step; 4],
    rel_path: &str,
    migration_id: Option<&str>,
) -> Result<OrderingResult, String> {
    let p = Project::new(); // gate OPEN, no txn, lock free
    let seam = tempfile::tempdir().unwrap();
    std::fs::write(seam.path().join("pause-after-w1"), b"").unwrap();
    let target = p.abs(rel_path);
    let payload = envelope("PreToolUse", "Edit", Some("TW"), edit_input(&target));

    let mut child: Option<Child> = None;
    let mut output: Option<Output> = None;
    let mut coordinator_lock: Option<MigrationLockGuard> = None;
    let mut c2_saw = false;

    for step in order {
        match step {
            Step::W1 => {
                let mut c = spawn(&p, &payload, Some(seam.path()));
                wait_for_w1(&mut c, seam.path(), Duration::from_secs(5))?;
                child = Some(c);
            }
            Step::W2 => {
                std::fs::write(seam.path().join("resume"), b"").unwrap();
                let c = child.take().ok_or("W2 before W1")?;
                output = Some(finish(c, Duration::from_secs(30)));
            }
            Step::C1 => {
                // coordinator: take the lock, durable DRAINING flip, then txn
                // STAGING(generation_id=null) (BC-1.18.013 Pre 6(c) steps 2,3,3a).
                coordinator_lock = Some(p.hold_lock());
                write_gate(&p.ms(), "DRAINING");
                write_txn(&p.ms(), "STAGING", None, migration_id);
            }
            Step::C2 => {
                c2_saw = p.reservation_count() > 0;
            }
        }
    }
    drop(coordinator_lock);
    let out = output.ok_or("no dispatcher output")?;
    Ok(OrderingResult {
        c2_saw_reservation: c2_saw,
        exit_code: out.status.code(),
        reservation_after: p.reservation("TW").exists(),
        stderr: stderr_of(&out),
    })
}

fn check_ordering(order: &[Step; 4], r: &OrderingResult) -> Vec<String> {
    use Step::*;
    let mut f = Vec::new();
    let pos = |s: Step| order.iter().position(|x| *x == s).unwrap();
    let w2_before_c1 = pos(W2) < pos(C1);
    // Dekker: never both miss.
    let blocked_and_released = r.exit_code == Some(2) && !r.reservation_after;
    if !(r.c2_saw_reservation || blocked_and_released) {
        f.push(format!(
            "{order:?}: BOTH MISSED (C2 saw no reservation and W2 did not block+release) -- \
             exit={:?}, reservation_after={}, stderr={}",
            r.exit_code, r.reservation_after, r.stderr
        ));
    }
    if w2_before_c1 {
        // W2 read OPEN => admitted, and C2 must see the (still-held) reservation.
        if r.exit_code != Some(0) || !r.c2_saw_reservation || !r.reservation_after {
            f.push(format!(
                "{order:?}: W1<W2<C1<C2 must admit (exit 0) and C2 must observe the reservation; \
                 got exit={:?}, c2_saw={}, reservation_after={}",
                r.exit_code, r.c2_saw_reservation, r.reservation_after
            ));
        }
    } else {
        // W2 happened after C1 => it reads DRAINING => blocked, own reservation removed.
        if r.exit_code != Some(2) || r.reservation_after || !r.stderr.contains("E-MAINTENANCE-001")
        {
            f.push(format!(
                "{order:?}: W2 after C1 must see DRAINING => E-MAINTENANCE-001 exit 2 with own \
                 reservation removed; got exit={:?}, reservation_after={}, stderr={}",
                r.exit_code, r.reservation_after, r.stderr
            ));
        }
    }
    f
}

fn run_all_orderings(test: &str, rel_path: &str, migration_id: Option<&str>) {
    let mut failures = Vec::new();
    for order in all_orderings() {
        match drive_ordering(&order, rel_path, migration_id) {
            Ok(r) => failures.extend(check_ordering(&order, &r)),
            Err(e) => failures.push(format!("{order:?}: {e}")),
        }
    }
    assert_no_failures(test, failures);
}

/// AC-005, mechanism-A shape (BC-1.18.013 Precondition 6(c) RESERVE-THEN-VERIFY, EC-017;
/// VP-143 D5; VP-133 facet 7).
#[test]
fn test_BC_1_18_013_PC6c_reserve_then_verify_all_interleavings() {
    run_all_orderings(
        "test_BC_1_18_013_PC6c_reserve_then_verify_all_interleavings",
        CYCLES_PATH,
        Some("backfill-append-logs"),
    );
}

/// AC-005 B2 shape (BC-1.18.011 Precondition 6(c), EC-007; VP-133 facet 7).
#[test]
fn test_BC_1_18_011_EC007_b2_reserve_then_verify_all_interleavings() {
    run_all_orderings(
        "test_BC_1_18_011_EC007_b2_reserve_then_verify_all_interleavings",
        BC_PATH,
        Some("migrate-bc-index"),
    );
}

// ---------------------------------------------------------------------------
// AC-006 -- release-on-block (black-box)
// ---------------------------------------------------------------------------

/// A shard config entry matching CYCLES_PATH but missing `shape` (EC-009) =>
/// `shard_cap_precheck` returns a fail-loud Error AFTER admission created the
/// reservation (a LATER stage of the same dispatch blocks).
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

/// AC-006 (BC-1.18.013 Precondition 6(c) "Release-on-block", EC-018; BC-1.18.011 EC-008;
/// VP-133 facet 7).
#[test]
fn test_BC_1_18_013_EC018_later_stage_block_releases_reservation_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    // T7: admission (gate OPEN, no txn) creates the reservation, THEN
    // shard_cap_precheck errors => dispatcher exit 2 and NO reservation remains.
    {
        let p = Project::new();
        std::fs::write(
            p.root().join(".factory/shard-config.toml"),
            CYCLES_MALFORMED_SHARD_CONFIG,
        )
        .unwrap();
        let seam = tempfile::tempdir().unwrap();
        let target = p.abs(CYCLES_PATH);
        std::fs::write(&target, "x".repeat(100)).unwrap();
        let out = run_with_seam(
            &p,
            &envelope("PreToolUse", "Edit", Some("T7"), edit_input(&target)),
            seam.path(),
        );
        if out.status.code() != Some(2) {
            failures.push(format!(
                "T7: a later-stage (shard_cap) Error must exit 2, got {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if !events(seam.path()).iter().any(|e| e == "W1_RESERVE:T7") {
            failures.push(format!(
                "T7: admission must have created the reservation FIRST (W1_RESERVE:T7) before the \
                 later stage blocked; events: {:?}",
                events(seam.path())
            ));
        }
        if p.reservation("T7").exists() {
            failures.push(
                "T7: dispatcher exited 2 but left reservations/T7.reservation (release-on-block \
                 violated)"
                    .to_string(),
            );
        }
    }

    // T8: the admitter's OWN verification fails (live txn) => none left either.
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "STAGING",
            Some("gen-1"),
            Some("backfill-append-logs"),
        );
        let _live = p.hold_lock();
        let seam = tempfile::tempdir().unwrap();
        let target = p.abs(CYCLES_PATH);
        let out = run_with_seam(
            &p,
            &envelope("PreToolUse", "Edit", Some("T8"), edit_input(&target)),
            seam.path(),
        );
        if out.status.code() != Some(2) {
            failures.push(format!(
                "T8: live txn must block (exit 2), got {:?}",
                out.status.code()
            ));
        }
        // reserve-then-verify: even the blocked admitter created, then removed.
        if !events(seam.path()).iter().any(|e| e == "W1_RESERVE:T8") {
            failures.push(format!(
                "T8: admitter must create its reservation BEFORE verifying (reserve-then-verify); \
                 events: {:?}",
                events(seam.path())
            ));
        }
        if p.reservation("T8").exists() {
            failures
                .push("T8: failed verification must remove the admitter's own reservation".into());
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC018_later_stage_block_releases_reservation_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-007 -- B2-1: reconciliation Branches A / B / C wired (black-box)
// ---------------------------------------------------------------------------

/// AC-007 (BC-1.18.011 Precondition 6(d), EC-010; BC-1.18.013 Postcondition 5a(c),
/// Invariant 6(c); VP-133 facet 6). Scenarios are evaluated independently and
/// failures aggregated so ONE run reports every unwired branch.
#[test]
fn test_BC_1_18_011_PC6d_reconciliation_branches_ABC_wired_on_production_path_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    for (rel, mig, label) in [
        (BC_PATH, Some("migrate-bc-index"), "B2/BC path"),
        (
            CYCLES_PATH,
            Some("backfill-append-logs"),
            "mechanism-A/cycles path",
        ),
    ] {
        // ---- Branch A: gate LOCKED / DRAINING, no active txn, lock free ----
        for gate in ["LOCKED", "DRAINING"] {
            let p = Project::new();
            write_gate(&p.ms(), gate);
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TA"), edit_input(&target)),
            );
            if out.status.code() != Some(0) || p.gate() != "OPEN" {
                failures.push(format!(
                    "[{label}] Branch A ({gate}, no txn, lock free): expected admit (exit 0) and \
                     gate -> OPEN; got exit {:?}, gate={}, stderr={}",
                    out.status.code(),
                    p.gate(),
                    stderr_of(&out)
                ));
            }
        }
        // Branch A also applies to a terminal (COMPLETED/ABORTED) txn.
        for st in ["COMPLETED", "ABORTED"] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), st, Some("gen-1"), mig);
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TA2"), edit_input(&target)),
            );
            if out.status.code() != Some(0) || p.gate() != "OPEN" {
                failures.push(format!(
                    "[{label}] Branch A (gate LOCKED + txn {st}): expected admit and gate -> OPEN; \
                     got exit {:?}, gate={}",
                    out.status.code(),
                    p.gate()
                ));
            }
        }

        // ---- Branch B: STAGING with generation_id = null, lock free ----
        {
            let p = Project::new();
            write_gate(&p.ms(), "DRAINING");
            write_txn(&p.ms(), "STAGING", None, mig);
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TB"), edit_input(&target)),
            );
            let txn = p.txn_json();
            let txn_path = p.ms().join("txn-act-s2508.json");
            let bytes_after_first = std::fs::read(&txn_path).unwrap_or_default();
            if out.status.code() != Some(0)
                || p.gate() != "OPEN"
                || txn["state"] != "ABORTED"
                || txn["abort_reason"] != "null_generation"
                || !txn["generation_id"].is_null()
                || !txn["source_sha256"].is_null()
                || txn["activation_id"] != "act-s2508"
                || bytes_after_first.is_empty()
            {
                failures.push(format!(
                    "[{label}] Branch B (STAGING, generation_id=null): expected admit, gate OPEN, \
                     SAME txn file retained with state=ABORTED, abort_reason=null_generation, \
                     generation_id/source_sha256 null; got exit {:?}, gate={}, txn={}",
                    out.status.code(),
                    p.gate(),
                    txn
                ));
            }
            // A second PreToolUse leaves the txn file byte-identical.
            let out2 = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TB2"), edit_input(&target)),
            );
            if out2.status.code() != Some(0)
                || std::fs::read(&txn_path).unwrap_or_default() != bytes_after_first
            {
                failures.push(format!(
                    "[{label}] second PreToolUse after Branch B must admit and leave the txn \
                     file byte-identical; exit {:?}",
                    out2.status.code()
                ));
            }
        }

        // ---- EWOULDBLOCK: a live coordinator holds the lock => no action + block ----
        for (gate, txn_state) in [
            ("LOCKED", None),
            ("DRAINING", Some(("STAGING", None::<&str>))),
        ] {
            let p = Project::new();
            write_gate(&p.ms(), gate);
            if let Some((s, g)) = txn_state {
                write_txn(&p.ms(), s, g, mig);
            }
            let _live = p.hold_lock();
            let before_gate = p.gate();
            let before_txn = std::fs::read_dir(p.ms())
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("txn-"))
                .map(|e| std::fs::read(e.path()).unwrap())
                .collect::<Vec<_>>();
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TL"), edit_input(&target)),
            );
            let after_txn = std::fs::read_dir(p.ms())
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("txn-"))
                .map(|e| std::fs::read(e.path()).unwrap())
                .collect::<Vec<_>>();
            if out.status.code() != Some(2)
                || !stderr_of(&out).contains(&plain_msg(scope_of(rel)))
                || stderr_of(&out).contains("completion-record mismatch")
                || p.gate() != before_gate
                || after_txn != before_txn
                || p.reservation("TL").exists()
            {
                failures.push(format!(
                    "[{label}] EWOULDBLOCK ({gate}): live coordinator => no action, \
                     E-MAINTENANCE-001 block, no reservation left; got exit {:?}, gate {} -> {}, \
                     txn_changed={}, reservation_left={}",
                    out.status.code(),
                    before_gate,
                    p.gate(),
                    after_txn != before_txn,
                    p.reservation("TL").exists()
                ));
            }
        }
    }

    // ---- Branch C (fail-closed seam in THIS story): zero-write guarantee ----
    // (own-migration terminal record, STAGING => always fail-closed, no verification)
    for (mig, terminal, label) in [
        (
            Some("migrate-bc-index"),
            "completed.json",
            "B2 STAGING + completed.json",
        ),
        (
            None,
            "completed.json",
            "legacy(absent migration_id) STAGING + completed.json",
        ),
        (
            Some("backfill-append-logs"),
            "completed-backfill-append-logs.json",
            "mechanism-A STAGING + completed-backfill-append-logs.json",
        ),
    ] {
        // generation_id null AND non-null: Branch B must NOT apply when the
        // migration's terminal record is present (Branch C governs).
        for gen_id in [None, Some("gen-1")] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), "STAGING", gen_id, mig);
            write_terminal_record(&p.ms(), terminal, "txn-s2508", "gen-1");
            let before = p.snapshot();
            // mechanism-A analogue writes under cycles/; B2 shapes under
            // behavioral-contracts/ (path-family keyed message).
            let rel = if mig == Some("backfill-append-logs") {
                CYCLES_PATH
            } else {
                BC_PATH
            };
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TC"), edit_input(&target)),
            );
            let err = stderr_of(&out);
            let want = format!("{}{MISMATCH_SUFFIX}", plain_msg(scope_of(rel)));
            if out.status.code() != Some(2) || !err.contains(&want) || p.snapshot() != before {
                failures.push(format!(
                    "[{label}, generation_id={gen_id:?}] Branch C: STAGING + terminal record is \
                     ALWAYS fail-closed: expected exit 2, exactly `{want}`, and \
                     migration-state byte-identical (no txn write, no gate write, no leftover \
                     reservation); got exit {:?}, snapshot_unchanged={}, stderr={err}",
                    out.status.code(),
                    p.snapshot() == before
                ));
            }
        }
    }

    // Branch C, COMMITTING + own terminal record that this story's FAIL-CLOSED
    // seam cannot verify: block, never finalize, never flip the gate.
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "COMMITTING",
            Some("gen-1"),
            Some("migrate-bc-index"),
        );
        write_terminal_record(&p.ms(), "completed.json", "act-s2508", "gen-1");
        let before = p.snapshot();
        let target = p.abs(BC_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TC2"), edit_input(&target)),
        );
        if out.status.code() != Some(2)
            || !stderr_of(&out).contains("E-MAINTENANCE-001")
            || p.snapshot() != before
        {
            failures.push(format!(
                "Branch C COMMITTING (unverifiable in this story): must fail closed (exit 2, \
                 E-MAINTENANCE-001), no txn/gate write; got exit {:?}, snapshot_unchanged={}",
                out.status.code(),
                p.snapshot() == before
            ));
        }
    }

    // Branch C foreign-migration guard: a live txn of the OTHER migration is
    // never finalized by this migration's terminal record; PLAIN E-MAINTENANCE-001
    // (no mismatch reason), zero-write.
    for (mig, terminal) in [
        ("backfill-append-logs", "completed.json"),
        ("migrate-bc-index", "completed-backfill-append-logs.json"),
    ] {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "COMMITTING", Some("gen-1"), Some(mig));
        write_terminal_record(&p.ms(), terminal, "act-s2508", "gen-1");
        let before = p.snapshot();
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TF"), edit_input(&target)),
        );
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err.contains(&plain_msg(".factory/cycles/"))
            || err.contains("completion-record mismatch")
            || err.contains("COMPLETION_RECORD_MISMATCH_ABORT")
            || p.snapshot() != before
        {
            failures.push(format!(
                "foreign live txn ({mig}) + {terminal}: must be a PLAIN E-MAINTENANCE-001 (no \
                 mismatch reason), never finalized, zero-write; got exit {:?}, \
                 snapshot_unchanged={}, stderr={err}",
                out.status.code(),
                p.snapshot() == before
            ));
        }
    }

    // NoOp cell (BC-1.18.011 v1.13 vector): own live txn (STAGING gen-1, and
    // COMMITTING), lock FREE (reconciler acquires it), terminal record ABSENT =>
    // the core decides NoOp (no txn/gate write); ordinary admission blocks with the
    // PLAIN path-family message, no mismatch suffix, migration-state byte-identical.
    for (state, mig) in [
        ("STAGING", Some("migrate-bc-index")),
        ("COMMITTING", Some("migrate-bc-index")),
        ("STAGING", None),
    ] {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), state, Some("gen-1"), mig);
        let before = p.snapshot();
        let target = p.abs(BC_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TN"), edit_input(&target)),
        );
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err.contains(&plain_msg("BC-INDEX"))
            || err.contains("completion-record mismatch")
            || p.snapshot() != before
        {
            failures.push(format!(
                "record-absent NoOp ({state}, {mig:?}): expected the PLAIN `BC-INDEX write \
                 blocked ...` block, no mismatch suffix, zero-write; got exit {:?}, \
                 snapshot_unchanged={}, stderr={err}",
                out.status.code(),
                p.snapshot() == before
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_011_PC6d_reconciliation_branches_ABC_wired_on_production_path_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// AC-009 -- B2 delegates to the SAME shared core (black-box facet)
// ---------------------------------------------------------------------------

/// AC-009 (BC-1.18.011 Precondition 6(c) reserve-then-verify + release-on-block,
/// EC-007, EC-008). B2 shape (path under `.factory/specs/behavioral-contracts/`,
/// txn migration_id = migrate-bc-index/absent): the reservation exists BEFORE the
/// gate is read (no check-then-reserve), and a failed verification removes it.
/// In-process B2 entry-point facets are in `bc_1_18_011_b2_migration_test.rs`.
#[test]
fn test_BC_1_18_011_PC6c_b2_delegates_to_shared_core_no_check_then_reserve_race() {
    let mut failures: Vec<String> = Vec::new();

    // (1) Reservation-FIRST proof: pause between W1 and W2 on a B2 path with the
    // gate OPEN; the reservation file must already exist while NO verify event
    // has been recorded yet.
    {
        let p = Project::new();
        let seam = tempfile::tempdir().unwrap();
        std::fs::write(seam.path().join("pause-after-w1"), b"").unwrap();
        let target = p.abs(BC_PATH);
        let mut child = spawn(
            &p,
            &envelope("PreToolUse", "Edit", Some("TB2"), edit_input(&target)),
            Some(seam.path()),
        );
        match wait_for_w1(&mut child, seam.path(), Duration::from_secs(5)) {
            Err(e) => failures.push(format!("B2 W1 pause: {e}")),
            Ok(()) => {
                if !p.reservation("TB2").exists() {
                    failures.push(
                        "B2: at the W1 point the reservation file must already exist (reserve \
                         FIRST)"
                            .to_string(),
                    );
                }
                if events(seam.path()).iter().any(|e| e == "W2_VERIFY") {
                    failures.push(
                        "B2: gate was read (W2_VERIFY) BEFORE the reservation resumed".into(),
                    );
                }
                std::fs::write(seam.path().join("resume"), b"").unwrap();
                let out = finish(child, Duration::from_secs(30));
                if out.status.code() != Some(0) {
                    failures.push(format!(
                        "B2: gate OPEN must admit, got {:?}",
                        out.status.code()
                    ));
                }
            }
        }
    }

    // (2) B2 failed verification (live B2 txn, live coordinator): own reservation
    // removed, E-MAINTENANCE-001, and the SAME core's reconcile-first behavior is
    // visible for a B2 path with a stale gate (Branch A through the B2 entry).
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "COMMITTING",
            Some("gen-1"),
            Some("migrate-bc-index"),
        );
        let _live = p.hold_lock();
        let seam = tempfile::tempdir().unwrap();
        let target = p.abs(BC_PATH);
        let out = run_with_seam(
            &p,
            &envelope("PreToolUse", "Edit", Some("TB3"), edit_input(&target)),
            seam.path(),
        );
        if out.status.code() != Some(2) || p.reservation("TB3").exists() {
            failures.push(format!(
                "B2 failed verification: expected exit 2 and no leftover reservation; got {:?}, \
                 reservation_left={}",
                out.status.code(),
                p.reservation("TB3").exists()
            ));
        }
        if !events(seam.path()).iter().any(|e| e == "W1_RESERVE:TB3") {
            failures.push(format!(
                "B2 must create its reservation before verifying (shared core); events: {:?}",
                events(seam.path())
            ));
        }
    }
    {
        let p = Project::new();
        write_gate(&p.ms(), "DRAINING"); // stale: no txn, lock free
        let seam = tempfile::tempdir().unwrap();
        let target = p.abs(BC_PATH);
        let out = run_with_seam(
            &p,
            &envelope("PreToolUse", "Edit", Some("TB4"), edit_input(&target)),
            seam.path(),
        );
        if out.status.code() != Some(0) || p.gate() != "OPEN" {
            failures.push(format!(
                "B2 path must run the shared core's Branch A reconciliation: expected exit 0 and \
                 gate OPEN; got exit {:?}, gate {}",
                out.status.code(),
                p.gate()
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_011_PC6c_b2_delegates_to_shared_core_no_check_then_reserve_race",
        failures,
    );
}

// ---------------------------------------------------------------------------
// BC-1.18.011 Precondition 6(d) row 3: Branch B applies WITHOUT any gate
// condition (absent / explicitly-OPEN gate-state.json)
// ---------------------------------------------------------------------------

/// Branch B must apply iff the live own-migration txn is STAGING with
/// `generation_id = null` (no terminal record, lock acquirable) -- regardless of
/// the gate. An absent `gate-state.json` reads as OPEN, so a gate-conditioned
/// Branch B leaves the null-generation STAGING txn undiscarded and EVERY
/// protected write blocked forever (VP-147 S6 no-permanent-self-lock).
#[test]
fn test_BC_1_18_011_PC6d_branch_b_applies_with_absent_or_open_gate_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    for (gate_variant, label) in [
        (None, "gate-state.json ABSENT"),
        (Some("OPEN"), "gate explicitly OPEN"),
    ] {
        for (rel, mig) in [
            (BC_PATH, Some("migrate-bc-index")),
            (BC_PATH, None),
            (CYCLES_PATH, Some("backfill-append-logs")),
        ] {
            let p = Project::new();
            match gate_variant {
                None => std::fs::remove_file(p.ms().join("gate-state.json")).unwrap(),
                Some(g) => write_gate(&p.ms(), g),
            }
            write_txn(&p.ms(), "STAGING", None, mig);
            let txn_path = p.ms().join("txn-act-s2508.json");
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TBO"), edit_input(&target)),
            );
            let txn = p.txn_json();
            let bytes_after_first = std::fs::read(&txn_path).unwrap_or_default();
            if out.status.code() != Some(0)
                || txn["state"] != "ABORTED"
                || txn["abort_reason"] != "null_generation"
                || !txn["generation_id"].is_null()
                || !txn["source_sha256"].is_null()
                || bytes_after_first.is_empty()
            {
                failures.push(format!(
                    "[{label}, {rel}, migration_id={mig:?}] Branch B must apply with no gate \
                     condition: expected admit (exit 0) and the SAME txn file rewritten in place \
                     to ABORTED + abort_reason=null_generation, generation_id/source_sha256 null; \
                     got exit {:?}, txn={txn}, stderr={}",
                    out.status.code(),
                    stderr_of(&out)
                ));
            }
            // Second PreToolUse: txn byte-identical.
            let out2 = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TBO2"), edit_input(&target)),
            );
            if out2.status.code() != Some(0)
                || std::fs::read(&txn_path).unwrap_or_default() != bytes_after_first
            {
                failures.push(format!(
                    "[{label}, {rel}] second PreToolUse must admit and leave the txn \
                     byte-identical; exit {:?}",
                    out2.status.code()
                ));
            }
        }

        // EWOULDBLOCK negative control: live coordinator holds the lock => no action.
        let p = Project::new();
        match gate_variant {
            None => std::fs::remove_file(p.ms().join("gate-state.json")).unwrap(),
            Some(g) => write_gate(&p.ms(), g),
        }
        write_txn(&p.ms(), "STAGING", None, Some("migrate-bc-index"));
        let _live = p.hold_lock();
        let txn_path = p.ms().join("txn-act-s2508.json");
        let before = std::fs::read(&txn_path).unwrap();
        let target = p.abs(BC_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TBL"), edit_input(&target)),
        );
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err.contains(&plain_msg("BC-INDEX"))
            || std::fs::read(&txn_path).unwrap() != before
            || p.reservation("TBL").exists()
        {
            failures.push(format!(
                "[{label}] EWOULDBLOCK control: live coordinator => no action, plain BC-INDEX \
                 block, txn untouched, no reservation; got exit {:?}, stderr={err}",
                out.status.code()
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_011_PC6d_branch_b_applies_with_absent_or_open_gate_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// BC-1.18.013 v1.7 EC-020 / BC-1.18.011 v1.15 EC-015 -- UNCONDITIONAL
// reservation namespace (first-activation race), real binary
// ---------------------------------------------------------------------------

impl Project {
    /// A repository where `.factory/` exists but `.factory/migration-state/`
    /// has NEVER existed (no migration ever run).
    fn new_bare() -> Self {
        let dir = tempfile::tempdir().expect("project tempdir");
        let plugin_root = tempfile::tempdir().expect("plugin_root tempdir");
        std::fs::write(
            plugin_root.path().join("hooks-registry.toml"),
            "schema_version = 2\n",
        )
        .expect("write empty registry");
        std::fs::create_dir_all(dir.path().join(".factory")).unwrap();
        Project { dir, plugin_root }
    }
}

/// Scripted coordinator over the on-disk namespace: create the directory (it
/// did not exist), take the lock, flip DRAINING, write txn STAGING(null).
fn coordinator_activate(p: &Project) -> MigrationLockGuard {
    std::fs::create_dir_all(p.ms().join("reservations")).unwrap();
    std::fs::write(p.ms().join("exclusive.lock"), b"").unwrap();
    let guard = p.hold_lock();
    write_gate(&p.ms(), "DRAINING");
    write_txn(&p.ms(), "STAGING", None, Some("migrate-bc-index"));
    guard
}

/// EC-020 / EC-015 (a): a writer admitted BEFORE migration-state/ existed is seen
/// by the coordinator's drain: the drain WAITS (timeout variant =>
/// DRAIN_TIMEOUT_ABORT; mid-poll PostToolUse release variant => drain proceeds).
#[test]
fn test_BC_1_18_013_EC020_pre_directory_writer_is_seen_by_coordinator_drain_blackbox() {
    use factory_dispatcher::shard_manager::{BcIndexMigrationError, drain_bc_index_writers};
    let mut failures: Vec<String> = Vec::new();

    for rel in [CYCLES_PATH, BC_PATH] {
        // --- variant (a): writer withheld PostToolUse => DRAIN_TIMEOUT_ABORT ---
        {
            let p = Project::new_bare();
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("T1"), edit_input(&target)),
            );
            if out.status.code() != Some(0) {
                failures.push(format!(
                    "[{rel}] pre-directory admit expected exit 0, got {:?}: {}",
                    out.status.code(),
                    stderr_of(&out)
                ));
            }
            if !p.reservation("T1").exists() {
                failures.push(format!(
                    "[{rel}] admitter must create migration-state/reservations/T1.reservation \
                     even though migration-state/ never existed (unconditional namespace)"
                ));
            }
            let _coord = coordinator_activate(&p);
            let r = drain_bc_index_writers(
                &p.ms().join("reservations"),
                Duration::from_millis(150),
                Duration::from_secs(3600),
            );
            if !matches!(r, Err(BcIndexMigrationError::DrainTimeoutAbort)) {
                failures.push(format!(
                    "[{rel}] coordinator drain must SEE the pre-directory writer and end in \
                     DRAIN_TIMEOUT_ABORT, got {r:?}"
                ));
            }
        }

        // --- variant (b): PostToolUse release mid-poll => drain proceeds ---
        {
            let p = Project::new_bare();
            let target = p.abs(rel);
            let _ = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("T1"), edit_input(&target)),
            );
            let _coord = coordinator_activate(&p);
            let post = envelope("PostToolUse", "Edit", Some("T1"), edit_input(&target));
            let reservations = p.ms().join("reservations");
            let had_reservation = p.reservation("T1").exists();
            let releaser = std::thread::scope(|sc| {
                let h = sc.spawn(|| {
                    std::thread::sleep(Duration::from_millis(100));
                    run(&p, &post)
                });
                let r = drain_bc_index_writers(
                    &reservations,
                    Duration::from_secs(10),
                    Duration::from_secs(3600),
                );
                (r, h.join().unwrap())
            });
            let (r, post_out) = releaser;
            if !had_reservation {
                failures.push(format!(
                    "[{rel}] mid-poll variant: no reservation existed for the coordinator to wait on"
                ));
            }
            if r.is_err() || post_out.status.code() != Some(0) {
                failures.push(format!(
                    "[{rel}] mid-poll PostToolUse release must let the drain reach quiescence; \
                     drain={r:?}, post exit={:?}",
                    post_out.status.code()
                ));
            }
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC020_pre_directory_writer_is_seen_by_coordinator_drain_blackbox",
        failures,
    );
}

/// EC-020 / EC-015 (b): concurrent FIRST-EVER admissions racing on directory
/// creation all succeed (idempotent create; no EEXIST error), each with its own
/// reservation.
#[test]
fn test_BC_1_18_013_EC020_concurrent_first_ever_admissions_all_succeed_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    for round in 0..3 {
        let p = Project::new_bare();
        let target = p.abs(CYCLES_PATH);
        let ids = ["C1", "C2", "C3", "C4"];
        let children: Vec<Child> = ids
            .iter()
            .map(|id| {
                spawn(
                    &p,
                    &envelope("PreToolUse", "Edit", Some(id), edit_input(&target)),
                    None,
                )
            })
            .collect();
        for (id, child) in ids.iter().zip(children) {
            let out = finish(child, Duration::from_secs(30));
            if out.status.code() != Some(0) {
                failures.push(format!(
                    "round {round}: concurrent first-ever admission {id} must succeed, got {:?}: {}",
                    out.status.code(),
                    stderr_of(&out)
                ));
            }
            if !p.reservation(id).exists() {
                failures.push(format!("round {round}: {id} has no reservation of its own"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC020_concurrent_first_ever_admissions_all_succeed_blackbox",
        failures,
    );
}

/// EC-020 / EC-015 (c): directory/reservation creation failure (unwritable
/// `.factory/`) fails the PreToolUse CLOSED (`HookResult::Error`, exit 2, the
/// writer-admission-check error) with no reservation left behind -- never an
/// untracked admit. Skipped (loudly) only when running as root, where chmod does
/// not restrict writes.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC020_unwritable_namespace_fails_closed_no_reservation_blackbox() {
    use std::os::unix::fs::PermissionsExt;

    let uid_out = Command::new("id").arg("-u").output().expect("id -u");
    if String::from_utf8_lossy(&uid_out.stdout).trim() == "0" {
        eprintln!(
            "SKIP: running as root; chmod 0555 does not restrict writes, so the EACCES path is unobservable"
        );
        return;
    }

    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    let p = Project::new_bare();
    let target = p.abs(CYCLES_PATH);
    let factory_dir = p.root().join(".factory");
    // .factory/ read+exec only: migration-state/ cannot be created under it.
    // (cycles/ was created by p.abs BEFORE the chmod, so it is not affected.)
    std::fs::set_permissions(&factory_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = Restore(factory_dir.clone());

    let out = run(
        &p,
        &envelope("PreToolUse", "Edit", Some("TX"), edit_input(&target)),
    );
    let err = stderr_of(&out);
    if out.status.code() != Some(2) || !err.contains("writer-admission check failed") {
        panic!(
            "unwritable namespace must FAIL CLOSED with the writer-admission-check error \
             (exit 2); got exit {:?}, stderr: {err}",
            out.status.code()
        );
    }
    assert!(
        !factory_dir.join("migration-state").exists(),
        "no migration-state/ can exist, hence no reservation left behind"
    );
}

// ===========================================================================
// ADR-052 v1.20 / BC-1.18.013 v1.8 / BC-1.18.011 v1.16 -- adversary pass-1
// red tests (F-001 .. F-009), real spawned binary
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RootEnv {
    Set,
    Unset,
    Empty,
}

/// Generic hook envelope: any event, optional `tool_name`, arbitrary
/// `tool_use_id` JSON value (None = field ABSENT), extra top-level fields.
fn json_env(
    event: &str,
    tool_name: Option<&str>,
    tool_input: serde_json::Value,
    tool_use_id: Option<serde_json::Value>,
    extra: serde_json::Value,
) -> String {
    let mut v = serde_json::json!({
        "hook_event_name": event,
        "session_id": "sess-s2508",
        "tool_input": tool_input,
    });
    if let Some(t) = tool_name {
        v["tool_name"] = serde_json::Value::String(t.to_string());
    }
    if let Some(id) = tool_use_id {
        v["tool_use_id"] = id;
    }
    if let Some(m) = extra.as_object() {
        for (k, val) in m {
            v[k] = val.clone();
        }
    }
    v.to_string()
}

fn spawn_env(project: &Project, payload: &str, seam: Option<&Path>, root: RootEnv) -> Child {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PROJECT_DIR", project.root())
        .env("VSDD_LOG_DIR", project.root().join("logs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match root {
        RootEnv::Set => {
            cmd.env("CLAUDE_PLUGIN_ROOT", project.plugin_root.path());
        }
        RootEnv::Unset => {
            cmd.env_remove("CLAUDE_PLUGIN_ROOT");
        }
        RootEnv::Empty => {
            cmd.env("CLAUDE_PLUGIN_ROOT", "");
        }
    }
    match seam {
        Some(d) => {
            cmd.env(SEAM_ENV, d);
        }
        None => {
            cmd.env_remove(SEAM_ENV);
        }
    }
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    child
}

fn run_env(project: &Project, payload: &str, seam: Option<&Path>, root: RootEnv) -> Output {
    finish(
        spawn_env(project, payload, seam, root),
        Duration::from_secs(30),
    )
}

fn migration_state_dirs_under(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let is_link = p
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            let n = e.file_name().to_string_lossy().to_string();
            if n == "migration-state" || n == "reservations" {
                out.push(p.clone());
            }
            if p.is_dir() && !is_link {
                walk(&p, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn count_w1(seam: &Path) -> usize {
    events(seam)
        .iter()
        .filter(|e| e.starts_with("W1_RESERVE:"))
        .count()
}

// ---------------------------------------------------------------------------
// F-001 / EC-021 -- PostToolUseFailure releases the reservation
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.8 EC-021 / ADR-052 v1.20 §5a F-001: a `PostToolUseFailure`
/// envelope `{tool_name, tool_input, tool_use_id, error, is_interrupt}` for an id
/// whose PreToolUse was admitted releases exactly that reservation, regardless of
/// `is_interrupt` and of `tool_name` (absent / differently shaped). An
/// unknown/invalid id is a silent zero-mutation no-op. This is the in-repo
/// PostToolUseFailure fixture the ADR requires (real shape, with `tool_use_id`).
#[test]
fn test_BC_1_18_013_EC021_posttoolusefailure_releases_reservation_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    let variants: [(Option<&str>, bool, &str); 5] = [
        (Some("Edit"), false, "tool failed"),
        (Some("Edit"), true, "interrupted"),
        (None, false, "no tool_name"),
        (None, true, "no tool_name + interrupt"),
        (Some("SomeOtherTool"), false, "differently-shaped tool_name"),
    ];
    for (tool, interrupt, label) in variants {
        let p = Project::new();
        let target = p.abs(CYCLES_PATH);
        let pre = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TF1"), edit_input(&target)),
        );
        if pre.status.code() != Some(0) || !p.reservation("TF1").exists() {
            failures.push(format!(
                "[{label}] precondition: PreToolUse must admit and reserve TF1 (exit {:?})",
                pre.status.code()
            ));
            continue;
        }
        let payload = json_env(
            "PostToolUseFailure",
            tool,
            edit_input(&target),
            Some(serde_json::json!("TF1")),
            serde_json::json!({ "error": "boom", "is_interrupt": interrupt }),
        );
        let out = run(&p, &payload);
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] PostToolUseFailure must exit 0, got {:?}",
                out.status.code()
            ));
        }
        if p.reservation("TF1").exists() || p.reservation_count() != 0 {
            failures.push(format!(
                "[{label}] PostToolUseFailure must remove reservations/TF1.reservation (directory \
                 empty after the pair); {} file(s) remain",
                p.reservation_count()
            ));
        }
    }

    // Unknown / invalid id => silent zero-mutation no-op; a planted sibling file
    // outside reservations/ (what `../x` would address) must survive.
    for id in ["NEVER-RESERVED", "../x", ".hidden", ""] {
        let p = Project::new();
        std::fs::write(
            p.reservation("OTHER"),
            r#"{"created_at":"2026-10-06T00:00:00Z","tool_use_id":"OTHER"}"#,
        )
        .unwrap();
        std::fs::write(p.ms().join("x.reservation"), b"outside").unwrap();
        let before = p.snapshot();
        let target = p.abs(CYCLES_PATH);
        let payload = json_env(
            "PostToolUseFailure",
            Some("Edit"),
            edit_input(&target),
            Some(serde_json::json!(id)),
            serde_json::json!({ "error": "boom", "is_interrupt": false }),
        );
        let out = run(&p, &payload);
        if out.status.code() != Some(0) || p.snapshot() != before {
            failures.push(format!(
                "PostToolUseFailure for unknown/invalid id {id:?} must be a zero-mutation exit-0 \
                 no-op; exit {:?}, tree_unchanged={}",
                out.status.code(),
                p.snapshot() == before
            ));
        }
    }

    // migration-state/ absent => zero-mutation no-op.
    {
        let p = Project::new_bare();
        let target = p.abs(CYCLES_PATH);
        let payload = json_env(
            "PostToolUseFailure",
            Some("Edit"),
            edit_input(&target),
            Some(serde_json::json!("TZ")),
            serde_json::json!({ "error": "boom", "is_interrupt": false }),
        );
        let out = run(&p, &payload);
        if out.status.code() != Some(0) || p.ms().exists() {
            failures.push(format!(
                "PostToolUseFailure with migration-state/ absent must be a no-op (exit {:?}, \
                 migration-state created={})",
                out.status.code(),
                p.ms().exists()
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC021_posttoolusefailure_releases_reservation_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-004 / EC-022 -- the gate is registry-independent (O1..O4)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Registry {
    Normal,
    Missing,
    Unparseable,
    SchemaMismatch,
}

fn set_registry(p: &Project, r: Registry) {
    let path = p.plugin_root.path().join("hooks-registry.toml");
    match r {
        Registry::Normal => std::fs::write(&path, "schema_version = 2\n").unwrap(),
        Registry::Missing => {
            let _ = std::fs::remove_file(&path);
        }
        Registry::Unparseable => std::fs::write(&path, "this is [[[ not toml\n").unwrap(),
        Registry::SchemaMismatch => std::fs::write(&path, "schema_version = 99\n").unwrap(),
    }
}

/// BC-1.18.013 v1.8 EC-022 / ADR-052 v1.20 §5a "Evaluation position" O1-O4. Gate
/// BEHAVIOR only (registry exit-code wording is being reconciled separately).
#[test]
fn test_BC_1_18_013_EC022_gate_is_registry_independent_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let modes: [(&str, RootEnv, Registry); 6] = [
        ("CLAUDE_PLUGIN_ROOT unset", RootEnv::Unset, Registry::Normal),
        ("CLAUDE_PLUGIN_ROOT empty", RootEnv::Empty, Registry::Normal),
        ("registry missing", RootEnv::Set, Registry::Missing),
        ("registry unparseable", RootEnv::Set, Registry::Unparseable),
        (
            "registry empty/no matching plugin",
            RootEnv::Set,
            Registry::Normal,
        ),
        (
            "registry schema mismatch",
            RootEnv::Set,
            Registry::SchemaMismatch,
        ),
    ];

    for (label, root, reg) in modes {
        // (A) live txn => blocked E-MAINTENANCE-001 exit 2 WITHOUT loading the
        // registry; core runs exactly once.
        {
            let p = Project::new();
            set_registry(&p, reg);
            write_gate(&p.ms(), "LOCKED");
            write_txn(
                &p.ms(),
                "STAGING",
                Some("gen-1"),
                Some("backfill-append-logs"),
            );
            let _live = p.hold_lock();
            let seam = tempfile::tempdir().unwrap();
            let target = p.abs(CYCLES_PATH);
            let out = run_env(
                &p,
                &envelope("PreToolUse", "Write", Some("TR"), edit_input(&target)),
                Some(seam.path()),
                root,
            );
            let err = stderr_of(&out);
            if out.status.code() != Some(2)
                || !err.contains(&plain_msg(".factory/cycles/"))
                || err.contains("E-REG")
            {
                failures.push(format!(
                    "[{label}] live txn must block E-MAINTENANCE-001 (exit 2) without loading the \
                     registry; got exit {:?}, stderr: {err}",
                    out.status.code()
                ));
            }
            if count_w1(seam.path()) != 1 {
                failures.push(format!(
                    "[{label}] core must run EXACTLY ONCE (one W1_RESERVE), events: {:?}",
                    events(seam.path())
                ));
            }
        }

        // (B) gate OPEN => admitted with the reservation standing (a registry that
        // fails CLOSED after the admitted verdict removes it: release-on-block).
        {
            let p = Project::new();
            set_registry(&p, reg);
            let seam = tempfile::tempdir().unwrap();
            let target = p.abs(CYCLES_PATH);
            let out = run_env(
                &p,
                &envelope("PreToolUse", "Edit", Some("TR2"), edit_input(&target)),
                Some(seam.path()),
                root,
            );
            let fail_closed_registry = reg == Registry::SchemaMismatch;
            let ok = if fail_closed_registry {
                out.status.code() == Some(2) && !p.reservation("TR2").exists()
            } else {
                out.status.code() == Some(0) && p.reservation("TR2").exists()
            };
            if !ok || count_w1(seam.path()) != 1 {
                failures.push(format!(
                    "[{label}] gate OPEN: expected {} ; got exit {:?}, reservation_present={}, \
                     W1 count={}",
                    if fail_closed_registry {
                        "registry fail-closed exit 2 with NO reservation left (release-on-block)"
                    } else {
                        "admit (exit 0) with the reservation standing"
                    },
                    out.status.code(),
                    p.reservation("TR2").exists(),
                    count_w1(seam.path())
                ));
            }
        }

        // (C) the matching Post / PostToolUseFailure releases under the same broken
        // registry (registry-independent release).
        if reg != Registry::SchemaMismatch {
            for post_event in ["PostToolUse", "PostToolUseFailure"] {
                let p = Project::new();
                let target = p.abs(CYCLES_PATH);
                let _ = run(
                    &p,
                    &envelope("PreToolUse", "Edit", Some("TR3"), edit_input(&target)),
                );
                set_registry(&p, reg);
                let payload = json_env(
                    post_event,
                    Some("Edit"),
                    edit_input(&target),
                    Some(serde_json::json!("TR3")),
                    serde_json::json!({}),
                );
                let out = run_env(&p, &payload, None, root);
                if out.status.code() != Some(0) || p.reservation("TR3").exists() {
                    failures.push(format!(
                        "[{label}] {post_event} must release TR3 under the same registry; exit \
                         {:?}, reservation_present={}",
                        out.status.code(),
                        p.reservation("TR3").exists()
                    ));
                }
            }
        }
    }

    // (g) unparseable stdin: existing parse-error exit unchanged; nothing
    // reservable, no classification, no mutation.
    {
        let p = Project::new();
        let before = p.snapshot();
        let seam = tempfile::tempdir().unwrap();
        let _ = run_with_seam(&p, "this is { not json", seam.path());
        if p.snapshot() != before || !events(seam.path()).is_empty() {
            failures.push(
                "unparseable stdin must create no reservation and run no classification".into(),
            );
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC022_gate_is_registry_independent_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-002 / EC-023 -- admission scope anchored to the session's factory_root
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.8 EC-023 / ADR-052 v1.20 "Admission scope anchoring".
#[test]
fn test_BC_1_18_013_EC023_out_of_root_factory_paths_are_out_of_scope_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    // (a)-(d): protected-LOOKING paths outside the session's factory_root, with a
    // live txn in the session's OWN tree.
    let other = tempfile::tempdir().unwrap();
    let p = Project::new();
    write_gate(&p.ms(), "LOCKED");
    write_txn(
        &p.ms(),
        "STAGING",
        Some("gen-1"),
        Some("backfill-append-logs"),
    );
    let _live = p.hold_lock();
    let cases: Vec<(String, PathBuf)> = vec![
        (
            "(a) another project's .factory/cycles".into(),
            other.path().join(".factory/cycles/c1/x.md"),
        ),
        (
            "(a) another project's .factory/specs/behavioral-contracts".into(),
            other
                .path()
                .join(".factory/specs/behavioral-contracts/x.md"),
        ),
        (
            "(b) nested project_root/sub/.factory/cycles".into(),
            p.root().join("sub/.factory/cycles/c1/x.md"),
        ),
        (
            "(d) look-alike x.factory/cycles".into(),
            p.root().join("x.factory/cycles/y.md"),
        ),
        (
            "(d) look-alike .factory-old/cycles".into(),
            p.root().join(".factory-old/cycles/z.md"),
        ),
    ];
    for (label, target) in cases {
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let before = p.snapshot();
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TO"), edit_input(&target)),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "{label}: out-of-root path must be ADMITTED even with a live txn in the session \
                 root; got exit {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if p.snapshot() != before || p.reservation("TO").exists() {
            failures.push(format!(
                "{label}: must create NO reservation / state in the session root"
            ));
        }
        let stray: Vec<PathBuf> = migration_state_dirs_under(other.path())
            .into_iter()
            .chain(migration_state_dirs_under(&p.root().join("sub")))
            .chain(migration_state_dirs_under(&p.root().join("x.factory")))
            .chain(migration_state_dirs_under(&p.root().join(".factory-old")))
            .collect();
        if !stray.is_empty() {
            failures.push(format!("{label}: spurious namespace created: {stray:?}"));
        }
    }

    // (e) `.factory` is a SYMLINK to a real directory: both spellings are gated
    // in ONE namespace (the real directory's).
    #[cfg(unix)]
    {
        let real = tempfile::tempdir().unwrap();
        let p = Project::new_bare();
        std::fs::create_dir_all(real.path().join("migration-state/reservations")).unwrap();
        std::fs::remove_dir_all(p.root().join(".factory")).unwrap();
        std::os::unix::fs::symlink(real.path(), p.root().join(".factory")).unwrap();
        let real_c = real.path().canonicalize().unwrap();
        let via_link = p.root().join(".factory/cycles/c1/x.md");
        let via_real = real_c.join("cycles/c1/x.md");
        for (spelling, target, id) in [
            ("via symlink", &via_link, "S1"),
            ("via real path", &via_real, "S2"),
        ] {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some(id), edit_input(target)),
            );
            let landed = real_c
                .join("migration-state/reservations")
                .join(format!("{id}.reservation"));
            if out.status.code() != Some(0) || !landed.exists() {
                failures.push(format!(
                    "(e) {spelling}: must be reserved in the REAL factory_root's migration-state; \
                     exit {:?}, reservation_present={}",
                    out.status.code(),
                    landed.exists()
                ));
            }
        }
        // and gated when a txn is live in the real directory
        write_gate(&real_c.join("migration-state"), "LOCKED");
        write_txn(
            &real_c.join("migration-state"),
            "COMMITTING",
            Some("gen-1"),
            Some("migrate-bc-index"),
        );
        std::fs::write(real_c.join("migration-state/exclusive.lock"), b"").unwrap();
        let _live = try_acquire_migration_lock(&real_c.join("migration-state/exclusive.lock"))
            .unwrap()
            .unwrap();
        for (spelling, target) in [("via symlink", &via_link), ("via real path", &via_real)] {
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("S3"), edit_input(target)),
            );
            if out.status.code() != Some(2)
                || !stderr_of(&out).contains(&plain_msg(".factory/cycles/"))
            {
                failures.push(format!(
                    "(e) {spelling}: live txn in the real root must block (exit 2); got {:?}",
                    out.status.code()
                ));
            }
        }
    }

    // (f) the project has NO `.factory` directory: out of scope, nothing created.
    {
        let p = Project::new_bare();
        std::fs::remove_dir_all(p.root().join(".factory")).unwrap();
        let target = p.root().join(".factory/cycles/c1/x.md");
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("TN"), edit_input(&target)),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "(f) no .factory: must be admitted, got exit {:?}",
                out.status.code()
            ));
        }
        if p.root().join(".factory").exists() {
            failures
                .push("(f) the gate must NEVER create `.factory` (nor anything under it)".into());
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC023_out_of_root_factory_paths_are_out_of_scope_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-003 / EC-024 -- target-path aliasing (black-box classification)
// ---------------------------------------------------------------------------

struct AliasCase {
    label: &'static str,
    file_path: Box<dyn Fn(&Project) -> String>,
    cwd_field: bool,
    expect: Option<&'static str>, // Some(scope) = blocked; None = admitted
}

/// BC-1.18.013 v1.8 EC-024 / ADR-052 v1.20 "Target path resolution": in-scope
/// aliases are BLOCKED with the path-family message while a txn is live;
/// out-of-scope inputs are admitted.
#[test]
fn test_BC_1_18_013_EC024_target_path_aliases_classified_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let scratch = tempfile::tempdir().unwrap();
    let scratch_real = scratch.path().canonicalize().unwrap();
    let sr1 = scratch_real.clone();
    let sr2 = scratch_real.clone();

    let cases: Vec<AliasCase> = vec![
        AliasCase {
            label: "(a) specs/./behavioral-contracts",
            file_path: Box::new(|p| {
                format!(
                    "{}/.factory/specs/./behavioral-contracts/x.md",
                    p.root().display()
                )
            }),
            cwd_field: false,
            expect: Some("BC-INDEX"),
        },
        AliasCase {
            label: "(b) specs/../specs/behavioral-contracts",
            file_path: Box::new(|p| {
                format!(
                    "{}/.factory/specs/../specs/behavioral-contracts/x.md",
                    p.root().display()
                )
            }),
            cwd_field: false,
            expect: Some("BC-INDEX"),
        },
        AliasCase {
            label: "(c) doubled slashes",
            file_path: Box::new(|p| format!("{}/.factory//cycles//c1//log.md", p.root().display())),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "(f) nonexistent tail",
            file_path: Box::new(|p| {
                format!("{}/.factory/cycles/newcycle/new.md", p.root().display())
            }),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "(g) .FACTORY/Cycles",
            file_path: Box::new(|p| format!("{}/.FACTORY/Cycles/x", p.root().display())),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "(g) Specs/Behavioral-Contracts",
            file_path: Box::new(|p| {
                format!(
                    "{}/.factory/Specs/Behavioral-Contracts/x.md",
                    p.root().display()
                )
            }),
            cwd_field: false,
            expect: Some("BC-INDEX"),
        },
        AliasCase {
            label: "(h) relative path joined to payload cwd",
            file_path: Box::new(|_| ".factory/cycles/c1/log.md".to_string()),
            cwd_field: true,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "~ is never expanded",
            file_path: Box::new(|_| "~/.factory/cycles/x".to_string()),
            cwd_field: true,
            expect: None,
        },
        AliasCase {
            label: "out of scope: STATE.md",
            file_path: Box::new(|p| format!("{}/.factory/STATE.md", p.root().display())),
            cwd_field: false,
            expect: None,
        },
        AliasCase {
            label: "out of scope: stories/x",
            file_path: Box::new(|p| format!("{}/.factory/stories/x", p.root().display())),
            cwd_field: false,
            expect: None,
        },
        AliasCase {
            label: "(d) symlink alias to .factory/cycles",
            file_path: Box::new(move |_| format!("{}/alias/x", sr1.display())),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "(e) link/../ resolves link FIRST (real form in scope)",
            file_path: Box::new(move |_| format!("{}/link/../x", sr2.display())),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
        AliasCase {
            label: "(e) union: lexical form in scope, real form outside",
            file_path: Box::new(|p| format!("{}/.factory/cycles/sub/../x", p.root().display())),
            cwd_field: false,
            expect: Some(".factory/cycles/"),
        },
    ];

    for c in cases {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
        let _live = p.hold_lock();
        std::fs::create_dir_all(p.root().join(".factory/cycles/c1")).unwrap();
        std::fs::create_dir_all(p.root().join(".factory/specs/behavioral-contracts")).unwrap();
        #[cfg(unix)]
        {
            // alias -> <factory_root>/cycles ; link -> <factory_root>/cycles/c1 ;
            // cycles/sub -> a directory elsewhere
            let _ = std::fs::remove_file(scratch_real.join("alias"));
            let _ = std::fs::remove_file(scratch_real.join("link"));
            let real_root = p.root().canonicalize().unwrap();
            std::os::unix::fs::symlink(
                real_root.join(".factory/cycles"),
                scratch_real.join("alias"),
            )
            .unwrap();
            std::os::unix::fs::symlink(
                real_root.join(".factory/cycles/c1"),
                scratch_real.join("link"),
            )
            .unwrap();
            let elsewhere = scratch_real.join(format!("elsewhere-{}", std::process::id()));
            std::fs::create_dir_all(elsewhere.join("deep")).unwrap();
            let sub = p.root().join(".factory/cycles/sub");
            let _ = std::fs::remove_file(&sub);
            std::os::unix::fs::symlink(elsewhere.join("deep"), &sub).unwrap();
        }
        let fp = (c.file_path)(&p);
        let extra = if c.cwd_field {
            serde_json::json!({ "cwd": p.root().to_string_lossy() })
        } else {
            serde_json::json!({})
        };
        let payload = json_env(
            "PreToolUse",
            Some("Edit"),
            serde_json::json!({ "file_path": fp, "old_string": "a", "new_string": "b" }),
            Some(serde_json::json!("TA1")),
            extra,
        );
        let out = run(&p, &payload);
        let err = stderr_of(&out);
        match c.expect {
            Some(scope) => {
                if out.status.code() != Some(2) || !err.contains(&plain_msg(scope)) {
                    failures.push(format!(
                        "{}: file_path `{fp}` must be classified in scope and BLOCKED with `{}`; \
                         got exit {:?}, stderr: {err}",
                        c.label,
                        plain_msg(scope),
                        out.status.code()
                    ));
                }
            }
            None => {
                if out.status.code() != Some(0) {
                    failures.push(format!(
                        "{}: file_path `{fp}` is OUT of scope and must be admitted; got exit {:?}: {err}",
                        c.label,
                        out.status.code()
                    ));
                }
            }
        }
    }

    // (i) on Unix `\` is an ordinary name byte (no separator rewrite): a file
    // literally named `.factory\cycles\x` is NOT under `<factory_root>/cycles`.
    #[cfg(unix)]
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
        let _live = p.hold_lock();
        let fp = format!("{}/.factory\\cycles\\c1\\x.md", p.root().display());
        let out = run(
            &p,
            &envelope(
                "PreToolUse",
                "Edit",
                Some("TB1"),
                edit_input(Path::new(&fp)),
            ),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "(i) backslash name on Unix must be out of scope (admitted); got exit {:?}",
                out.status.code()
            ));
        }
    }

    // missing / non-string / empty / NUL file_path => out of scope, not an error.
    for (label, tool_input) in [
        (
            "missing file_path",
            serde_json::json!({ "old_string": "a" }),
        ),
        (
            "non-string file_path",
            serde_json::json!({ "file_path": 7 }),
        ),
        ("empty file_path", serde_json::json!({ "file_path": "" })),
        (
            "NUL in file_path",
            serde_json::json!({ "file_path": ".factory/cycles/x\u{0}y" }),
        ),
    ] {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
        let _live = p.hold_lock();
        let before = p.snapshot();
        let out = run(
            &p,
            &json_env(
                "PreToolUse",
                Some("Edit"),
                tool_input,
                Some(serde_json::json!("TM")),
                serde_json::json!({}),
            ),
        );
        if out.status.code() != Some(0) || p.snapshot() != before {
            failures.push(format!(
                "{label}: must be out of scope (admitted, zero-mutation); got exit {:?}",
                out.status.code()
            ));
        }
    }

    // (j) unresolvable component (EACCES): classified on the lexical form alone,
    // still protected (fail-closed).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let uid = Command::new("id")
            .arg("-u")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
            struct Restore(PathBuf);
            impl Drop for Restore {
                fn drop(&mut self) {
                    let _ =
                        std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
                }
            }
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
            let _live = p.hold_lock();
            let locked = p.root().join(".factory/cycles/locked");
            std::fs::create_dir_all(locked.join("inner")).unwrap();
            let _r = Restore(locked.clone());
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            let fp = locked.join("inner/y.md");
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("TJ"), edit_input(&fp)),
            );
            if out.status.code() != Some(2)
                || !stderr_of(&out).contains(&plain_msg(".factory/cycles/"))
            {
                failures.push(format!(
                    "(j) unresolvable (EACCES) ancestor must fall back to the lexical form and stay \
                     protected; got exit {:?}",
                    out.status.code()
                ));
            }
        } else {
            eprintln!("SKIP (j): running as root; chmod 000 does not restrict lstat");
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC024_target_path_aliases_classified_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-006 / EC-027..EC-030 -- migration_id knownness, dispatcher path
// ---------------------------------------------------------------------------

fn write_txn_with_raw_migration_id(
    ms: &Path,
    state: &str,
    generation_id: Option<&str>,
    migration_id: serde_json::Value,
) {
    write_txn(ms, state, generation_id, None);
    let path = ms.join("txn-act-s2508.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["migration_id"] = migration_id;
    std::fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

/// BC-1.18.013 v1.8 EC-027 / EC-029 / EC-030; BC-1.18.011 v1.16 (F-006).
/// (EC-028's finalize-on-verifying-record path needs the S-25.06 effectful
/// verification seam to be fixturable; its pure decision is pinned in
/// `s2508_pure_core_and_ttl_test.rs`.)
#[test]
fn test_BC_1_18_013_EC027_029_030_migration_id_knownness_dispatcher_path_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    // EC-027: a live txn of the OTHER KNOWN migration with ITS OWN record absent
    // (only the mechanism-A record present): NoOp -> plain block, byte-identical.
    for state in ["STAGING", "COMMITTING"] {
        for rel in [BC_PATH, CYCLES_PATH] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), state, Some("gen-1"), Some("migrate-bc-index"));
            write_terminal_record(
                &p.ms(),
                "completed-backfill-append-logs.json",
                "act-s2508",
                "gen-1",
            );
            let before = p.snapshot();
            let target = p.abs(rel);
            let out = run(
                &p,
                &envelope("PreToolUse", "Edit", Some("T27"), edit_input(&target)),
            );
            let err = stderr_of(&out);
            if out.status.code() != Some(2)
                || !err.contains(&plain_msg(scope_of(rel)))
                || err.contains("completion-record mismatch")
                || p.snapshot() != before
            {
                failures.push(format!(
                    "EC-027 [{state}, {rel}]: B2 txn + only the mechanism-A record must be a PLAIN \
                     block (own record absent => NoOp), byte-identical; got exit {:?}",
                    out.status.code()
                ));
            }
        }
    }
    // EC-027 Branch B: other known migration, null-generation STAGING, own record
    // absent (B2's record present) => Branch B applies (txn ABORTED), admitted.
    {
        let p = Project::new();
        write_gate(&p.ms(), "DRAINING");
        write_txn(&p.ms(), "STAGING", None, Some("backfill-append-logs"));
        write_terminal_record(&p.ms(), "completed.json", "act-s2508", "gen-1");
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("T27b"), edit_input(&target)),
        );
        let txn = p.txn_json();
        if out.status.code() != Some(0)
            || txn["state"] != "ABORTED"
            || txn["abort_reason"] != "null_generation"
        {
            failures.push(format!(
                "EC-027 Branch B: mechanism-A null-generation STAGING with only B2's record must be \
                 aborted (own record absent); got exit {:?}, txn={txn}",
                out.status.code()
            ));
        }
    }

    // EC-029: unknown string migration_id => FOREIGN: plain block, never
    // aborted/finalized, byte-identical.
    let long_id = "x".repeat(1000);
    let ctl_id = "bad\u{1}id\nline\u{7}".to_string();
    for mig in ["future-migration", long_id.as_str(), ctl_id.as_str()] {
        for (state, gen_id) in [
            ("STAGING", None),
            ("STAGING", Some("gen-1")),
            ("COMMITTING", Some("gen-1")),
        ] {
            for with_record in [false, true] {
                let p = Project::new();
                write_gate(&p.ms(), "LOCKED");
                write_txn(&p.ms(), state, gen_id, Some(mig));
                if with_record {
                    write_terminal_record(&p.ms(), "completed.json", "act-s2508", "gen-1");
                }
                let before = p.snapshot();
                let target = p.abs(BC_PATH);
                let out = run(
                    &p,
                    &envelope("PreToolUse", "Edit", Some("T29"), edit_input(&target)),
                );
                let err = stderr_of(&out);
                if out.status.code() != Some(2)
                    || !err.contains(&plain_msg("BC-INDEX"))
                    || err.contains("completion-record mismatch")
                    || p.snapshot() != before
                {
                    failures.push(format!(
                        "EC-029 [migration_id len {}, {state}, gen={gen_id:?}, record={with_record}]: \
                         unknown id => plain block, NEVER aborted/finalized, byte-identical; got \
                         exit {:?}, tree_unchanged={}",
                        mig.len(),
                        out.status.code(),
                        p.snapshot() == before
                    ));
                }
            }
        }
    }

    // EC-030: a PRESENT non-string migration_id => E-MAINTENANCE-002
    // (state_integrity), fail-closed, no reservation, no txn/gate write.
    for raw in [
        serde_json::json!(42),
        serde_json::Value::Null,
        serde_json::json!([1]),
        serde_json::json!({"a": 1}),
        serde_json::json!(true),
    ] {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn_with_raw_migration_id(&p.ms(), "STAGING", Some("gen-1"), raw.clone());
        let before = p.snapshot();
        let target = p.abs(BC_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some("T30"), edit_input(&target)),
        );
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err.contains("E-MAINTENANCE-002: writer-admission check failed (state_integrity)")
            || p.snapshot() != before
        {
            failures.push(format!(
                "EC-030 [migration_id={raw}]: non-string id must fail closed with \
                 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`, zero-write; \
                 got exit {:?}, tree_unchanged={}, stderr={err}",
                out.status.code(),
                p.snapshot() == before
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC027_029_030_migration_id_knownness_dispatcher_path_blackbox",
        failures,
    );
}

/// REGRESSION PIN (green at Red Gate; implemented at 332c14e0 without a red
/// test): a terminal-record STAT ERROR must fail closed and must never be read
/// as "record ABSENT" (which would let Branch B discard a null-generation STAGING
/// txn whose record merely could not be stat'ed). A self-referential symlink
/// `completed.json` makes `try_exists` return `Err(ELOOP)`.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_PC6d_terminal_record_stat_error_fails_closed_never_branch_b_regression_pin() {
    let p = Project::new();
    write_gate(&p.ms(), "DRAINING");
    write_txn(&p.ms(), "STAGING", None, Some("migrate-bc-index"));
    std::os::unix::fs::symlink("completed.json", p.ms().join("completed.json")).unwrap();
    let txn_before = std::fs::read(p.ms().join("txn-act-s2508.json")).unwrap();
    let target = p.abs(BC_PATH);
    let out = run(
        &p,
        &envelope("PreToolUse", "Edit", Some("TS"), edit_input(&target)),
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "a stat error on the terminal record must fail closed (never admit)"
    );
    assert_eq!(
        std::fs::read(p.ms().join("txn-act-s2508.json")).unwrap(),
        txn_before,
        "the null-generation STAGING txn must NOT be discarded when the terminal record cannot be stat'ed"
    );
    assert!(!p.reservation("TS").exists(), "no reservation left behind");
}

// ---------------------------------------------------------------------------
// F-009 / EC-026 -- tool_use_id validity
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.8 EC-026 / ADR-052 v1.20 "tool_use_id validity".
#[test]
fn test_BC_1_18_013_EC026_invalid_tool_use_id_fails_closed_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let long129 = "a".repeat(129);
    let invalid: Vec<(&str, serde_json::Value, Option<String>)> = vec![
        ("number", serde_json::json!(7), None),
        ("empty string", serde_json::json!(""), None),
        ("../x", serde_json::json!("../x"), Some("../x".to_string())),
        (
            "129 chars",
            serde_json::json!(long129),
            Some(long129.clone()),
        ),
        (
            ".hidden",
            serde_json::json!(".hidden"),
            Some(".hidden".to_string()),
        ),
        ("boolean", serde_json::json!(true), None),
        ("array", serde_json::json!(["a"]), None),
        ("object", serde_json::json!({"a": 1}), None),
    ];
    for (label, id, raw) in invalid {
        let p = Project::new_bare();
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &json_env(
                "PreToolUse",
                Some("Edit"),
                edit_input(&target),
                Some(id),
                serde_json::json!({}),
            ),
        );
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err
                .contains("E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)")
        {
            failures.push(format!(
                "[{label}] must FAIL CLOSED with `E-MAINTENANCE-002: writer-admission check failed \
                 (invalid_tool_use_id)` (exit 2); got exit {:?}, stderr: {err}",
                out.status.code()
            ));
        }
        if p.ms().exists() {
            failures.push(format!("[{label}] must create NO directory / reservation"));
        }
        if let Some(raw) = raw
            && err.contains(&raw)
        {
            failures.push(format!(
                "[{label}] the raw id must never appear in the output (byte length only)"
            ));
        }
    }

    // absent / null => check-only admission: no directory, no reservation.
    for (label, id) in [("absent", None), ("null", Some(serde_json::Value::Null))] {
        let p = Project::new_bare();
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &json_env(
                "PreToolUse",
                Some("Edit"),
                edit_input(&target),
                id,
                serde_json::json!({}),
            ),
        );
        if out.status.code() != Some(0) || p.ms().exists() {
            failures.push(format!(
                "[{label}] tool_use_id must degrade to CHECK-ONLY admission (exit 0, no directory); \
                 got exit {:?}, migration-state created={}",
                out.status.code(),
                p.ms().exists()
            ));
        }
    }

    // valid ids: exactly 128 chars, and `-` `_` `.` after the first char.
    let ok128 = format!("toolu_{}", "A1".repeat(61));
    assert_eq!(ok128.len(), 128);
    for (label, id) in [
        ("128 chars", ok128),
        ("dash/underscore/dot", "toolu_a-b.c_d".to_string()),
    ] {
        let p = Project::new_bare();
        let target = p.abs(CYCLES_PATH);
        let out = run(
            &p,
            &envelope("PreToolUse", "Edit", Some(&id), edit_input(&target)),
        );
        if out.status.code() != Some(0) || !p.reservation(&id).exists() {
            failures.push(format!(
                "[valid {label}] must be accepted and reserved; got exit {:?}, reservation_present={}",
                out.status.code(),
                p.reservation(&id).exists()
            ));
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC026_invalid_tool_use_id_fails_closed_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// EC-032 -- `E-MAINTENANCE-002 <cause>` classification fixture matrix
// (BC-1.18.013 v1.8 EC-032 + canonical test vector row; Precondition 6(c))
// ---------------------------------------------------------------------------

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Restores a path's mode on drop (so tempdir cleanup always works).
#[cfg(unix)]
struct RestoreMode {
    path: PathBuf,
    mode: u32,
}
#[cfg(unix)]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(self.mode));
    }
}

#[cfg(unix)]
fn chmod000(path: &Path, restore_to: u32) -> RestoreMode {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    RestoreMode {
        path: path.to_path_buf(),
        mode: restore_to,
    }
}

#[cfg(unix)]
fn is_root() -> bool {
    let uid = Command::new("id")
        .arg("-u")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    String::from_utf8_lossy(&uid.stdout).trim() == "0"
}

/// Byte snapshot that tolerates unreadable (mode-000) entries.
#[cfg(unix)]
fn lenient_snapshot(p: &Project) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let path = e.path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(base, &path, out);
            } else {
                out.insert(
                    rel,
                    std::fs::read(&path).unwrap_or_else(|_| b"<unreadable>".to_vec()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(&p.ms(), &p.ms(), &mut out);
    out
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// admitted (exit 0) and a reservation stands
    Admit,
    /// plain `E-MAINTENANCE-001` block (prefix match: the path-family message),
    /// NEVER `E-MAINTENANCE-002`
    PlainBlock,
    /// exit 2 `E-MAINTENANCE-001` keyed message followed by the EXACT
    /// ` (completion-record mismatch -- operator investigation required)` suffix
    /// (em dash U+2014); txn not finalized; tree byte-identical (v1.9 ruling)
    MismatchBlock,
    /// exit 2 `E-MAINTENANCE-002: writer-admission check failed (<cause>)`
    Cause(&'static str),
}

#[cfg(unix)]
struct Fixture {
    _modes: Vec<RestoreMode>,
    _lock: Option<MigrationLockGuard>,
}

#[cfg(unix)]
struct Row {
    label: &'static str,
    rel: &'static str,
    needs_non_root: bool,
    expect: Expect,
    setup: Box<dyn Fn(&Project) -> Fixture>,
}

#[cfg(unix)]
fn fx() -> Fixture {
    Fixture {
        _modes: Vec::new(),
        _lock: None,
    }
}

#[cfg(unix)]
fn gate_row(label: &'static str, bytes: &'static [u8], expect: Expect) -> Row {
    Row {
        label,
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect,
        setup: Box::new(move |p| {
            std::fs::write(p.ms().join("gate-state.json"), bytes).unwrap();
            fx()
        }),
    }
}

#[cfg(unix)]
fn assert_row(row: &Row, failures: &mut Vec<String>) {
    if row.needs_non_root && is_root() {
        eprintln!(
            "SKIP [{}]: running as root; chmod 000 does not restrict reads",
            row.label
        );
        return;
    }
    let p = Project::new();
    let _fixture = (row.setup)(&p);
    let before = lenient_snapshot(&p);
    let target = p.abs(row.rel);
    let out = run(
        &p,
        &envelope("PreToolUse", "Write", Some("T32"), edit_input(&target)),
    );
    let err = stderr_of(&out);
    let reason = err
        .split("block_reason=\"")
        .nth(1)
        .unwrap_or(&err)
        .to_string();
    let ok = match row.expect {
        Expect::Admit => out.status.code() == Some(0) && p.reservation("T32").exists(),
        Expect::PlainBlock => {
            out.status.code() == Some(2)
                && err.contains(&plain_msg(scope_of(row.rel)))
                && !err.contains("completion-record mismatch")
                && !err.contains("E-MAINTENANCE-002")
                && !p.reservation("T32").exists()
                && lenient_snapshot(&p) == before
        }
        Expect::MismatchBlock => {
            out.status.code() == Some(2)
                && err.contains(&format!(
                    "{}{MISMATCH_SUFFIX}",
                    plain_msg(scope_of(row.rel))
                ))
                && !err.contains("E-MAINTENANCE-002")
                && !p.reservation("T32").exists()
                && lenient_snapshot(&p) == before
        }
        Expect::Cause(cause) => {
            let want = format!("E-MAINTENANCE-002: writer-admission check failed ({cause})");
            out.status.code() == Some(2)
                && err.contains(&want)
                // message carries ONLY the cause token (no path / errno / serde position)
                && !reason.contains("gate-state")
                && !reason.contains("txn-")
                && !reason.contains("os error")
                && !reason.contains("expected")
                && !p.reservation("T32").exists()
                && lenient_snapshot(&p) == before
        }
    };
    if !ok {
        failures.push(format!(
            "[{}] expected {:?}; got exit {:?}, reservation_left={}, tree_unchanged={}, \
             stderr: {err}",
            row.label,
            row.expect,
            out.status.code(),
            p.reservation("T32").exists(),
            lenient_snapshot(&p) == before
        ));
    }
}

#[cfg(unix)]
fn write_raw_txn(p: &Project, name: &str, bytes: &[u8]) {
    std::fs::write(p.ms().join(name), bytes).unwrap();
}

/// Gate OPEN + a raw `txn-*.json` fixture.
#[cfg(unix)]
fn txn_row(label: &'static str, bytes: &'static [u8], expect: Expect) -> Row {
    Row {
        label,
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect,
        setup: Box::new(move |p| {
            write_raw_txn(p, "txn-act-s2508.json", bytes);
            fx()
        }),
    }
}

/// BC-1.18.013 v1.8 EC-032 (canonical TV row): gate-state rows (a)-(o), the
/// txn-record rows, first-failure-wins precedence, and the terminal-record rows.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC032_gate_state_fixture_matrix_cause_classification_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();

    // ---- gate-state.json rows (a)-(o) ----
    rows.push(gate_row(
        "(a) gate-state \"OPEN\"",
        b"\"OPEN\"",
        Expect::Admit,
    ));
    rows.push(Row {
        label: "(b) gate-state \"LOCKED\" (live coordinator holds the lock)",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::PlainBlock,
        setup: Box::new(|p| {
            std::fs::write(p.ms().join("gate-state.json"), b"\"LOCKED\"").unwrap();
            Fixture {
                _modes: Vec::new(),
                _lock: Some(p.hold_lock()),
            }
        }),
    });
    rows.push(Row {
        label: "(c) gate-state ABSENT (ENOENT is not an error => OPEN)",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::Admit,
        setup: Box::new(|p| {
            std::fs::remove_file(p.ms().join("gate-state.json")).unwrap();
            fx()
        }),
    });
    rows.push(Row {
        label: "(d) gate-state mode 000 (open fails EACCES)",
        rel: CYCLES_PATH,
        needs_non_root: true,
        expect: Expect::Cause("io"),
        setup: Box::new(|p| {
            let path = p.ms().join("gate-state.json");
            Fixture {
                _modes: vec![chmod000(&path, 0o644)],
                _lock: None,
            }
        }),
    });
    rows.push(Row {
        label: "(e) gate-state is a DIRECTORY (read fails EISDIR)",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::Cause("io"),
        setup: Box::new(|p| {
            let path = p.ms().join("gate-state.json");
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
            fx()
        }),
    });
    rows.push(gate_row(
        "(f) gate-state zero-length",
        b"",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(g) gate-state truncated \"OPE",
        b"\"OPE",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(h) gate-state invalid UTF-8 0xFF 0xFE",
        &[0xFF, 0xFE],
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(i) gate-state unquoted OPEN",
        b"OPEN",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(j) gate-state \"open\" (wrong casing)",
        b"\"open\"",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(k) gate-state \"BOGUS\"",
        b"\"BOGUS\"",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(l) gate-state {\"state\":\"OPEN\"}",
        b"{\"state\":\"OPEN\"}",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(m) gate-state null",
        b"null",
        Expect::Cause("state_integrity"),
    ));
    rows.push(gate_row(
        "(n) gate-state 7",
        b"7",
        Expect::Cause("state_integrity"),
    ));
    rows.push(Row {
        label: "(o) gate-state mode 000 whose bytes would ALSO be malformed => io (content never examined)",
        rel: CYCLES_PATH,
        needs_non_root: true,
        expect: Expect::Cause("io"),
        setup: Box::new(|p| {
            let path = p.ms().join("gate-state.json");
            std::fs::write(&path, [0xFF, 0xFE, b'x']).unwrap();
            Fixture {
                _modes: vec![chmod000(&path, 0o644)],
                _lock: None,
            }
        }),
    });

    // ---- txn-record rows (gate OPEN) ----
    rows.push(Row {
        label: "txn record unreadable (mode 000) => io",
        rel: CYCLES_PATH,
        needs_non_root: true,
        expect: Expect::Cause("io"),
        setup: Box::new(|p| {
            write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
            let path = p.ms().join("txn-act-s2508.json");
            Fixture {
                _modes: vec![chmod000(&path, 0o644)],
                _lock: None,
            }
        }),
    });
    rows.push(txn_row(
        "txn record zero-length",
        b"",
        Expect::Cause("state_integrity"),
    ));
    rows.push(txn_row(
        "txn record truncated JSON",
        b"{\"txn_id\": \"x\", \"state\"",
        Expect::Cause("state_integrity"),
    ));
    rows.push(txn_row(
        "txn record invalid UTF-8",
        &[0xFF, 0xFE, 0xFD],
        Expect::Cause("state_integrity"),
    ));
    rows.push(txn_row(
        "txn record JSON array top level",
        b"[1,2,3]",
        Expect::Cause("state_integrity"),
    ));
    rows.push(Row {
        label: "txn record `state` unknown value",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::Cause("state_integrity"),
        setup: Box::new(|p| {
            write_txn(&p.ms(), "BOGUS", Some("gen-1"), Some("migrate-bc-index"));
            fx()
        }),
    });
    rows.push(Row {
        label: "txn record non-string migration_id (EC-030) => state_integrity",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::Cause("state_integrity"),
        setup: Box::new(|p| {
            write_txn_with_raw_migration_id(
                &p.ms(),
                "STAGING",
                Some("gen-1"),
                serde_json::json!(42),
            );
            fx()
        }),
    });
    rows.push(Row {
        label: "two live txn records => state_integrity",
        rel: CYCLES_PATH,
        needs_non_root: false,
        expect: Expect::Cause("state_integrity"),
        setup: Box::new(|p| {
            write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
            let one = std::fs::read(p.ms().join("txn-act-s2508.json")).unwrap();
            write_raw_txn(p, "txn-act-second.json", &one);
            fx()
        }),
    });

    // ---- first-failure-wins: gate-state is evaluated BEFORE the txn records ----
    rows.push(Row {
        label: "precedence: gate `7` (state_integrity) + txn unreadable (io) => state_integrity",
        rel: CYCLES_PATH,
        needs_non_root: true,
        expect: Expect::Cause("state_integrity"),
        setup: Box::new(|p| {
            std::fs::write(p.ms().join("gate-state.json"), b"7").unwrap();
            write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
            let path = p.ms().join("txn-act-s2508.json");
            Fixture {
                _modes: vec![chmod000(&path, 0o644)],
                _lock: None,
            }
        }),
    });
    rows.push(Row {
        label: "precedence: gate unreadable (io) + txn zero-length (state_integrity) => io",
        rel: CYCLES_PATH,
        needs_non_root: true,
        expect: Expect::Cause("io"),
        setup: Box::new(|p| {
            write_raw_txn(p, "txn-act-s2508.json", b"");
            let path = p.ms().join("gate-state.json");
            Fixture {
                _modes: vec![chmod000(&path, 0o644)],
                _lock: None,
            }
        }),
    });

    // ---- terminal record (Branch C): live txn of its OWN migration, lock FREE so
    // the reconciler reaches the record. BC-1.18.013 v1.9 EC-032 TV row (a)-(h),
    // STAGING gen-1 and separately COMMITTING, both migrations / path families. ----
    type RecordBytes = Box<dyn Fn() -> Vec<u8>>;
    let mk = |txn_id: &'static str, gen_id: &'static str, count: u64| -> RecordBytes {
        Box::new(move || {
            serde_json::to_vec(&serde_json::json!({
                "generation_id": gen_id,
                "txn_id": txn_id,
                "completed_at": "2026-10-06T00:00:00Z",
                "canonical_paths_count": count,
            }))
            .unwrap()
        })
    };
    let record_cases: Vec<(&'static str, RecordBytes)> = vec![
        ("(a) non-UTF-8 bytes", Box::new(|| vec![0xFF, 0xFE, 0xFD])),
        ("(b) zero-length", Box::new(Vec::new)),
        (
            "(c) truncated JSON",
            Box::new(|| b"{\"generation_id\": \"gen-1\", \"tx".to_vec()),
        ),
        ("(d) wrong schema []", Box::new(|| b"[]".to_vec())),
        (
            "(d) wrong schema {\"foo\":1}",
            Box::new(|| b"{\"foo\":1}".to_vec()),
        ),
        ("(e) canonical_paths_count = 3", mk("act-s2508", "gen-1", 3)),
        ("(f) mismatching txn_id", mk("some-other-txn", "gen-1", 4)),
        // (g) hash mismatch: every field matches, the canonical files do not
        // carry `expected_post_hash` (no canonical files exist in the fixture).
        (
            "(g) all fields match, canonical hash mismatch",
            mk("act-s2508", "gen-1", 4),
        ),
        ("not JSON text", Box::new(|| b"this is not json".to_vec())),
    ];
    let combos: [(&'static str, &'static str, &'static str, &'static str); 2] = [
        ("migrate-bc-index", "completed.json", BC_PATH, "B2"),
        (
            "backfill-append-logs",
            "completed-backfill-append-logs.json",
            CYCLES_PATH,
            "mechanism-A",
        ),
    ];
    for (mig, record_file, rel, mig_label) in combos {
        for state in ["STAGING", "COMMITTING"] {
            for (case_label, bytes) in &record_cases {
                let label: &'static str = Box::leak(
                    format!(
                        "[{mig_label} {state}] terminal record {case_label} => mismatch suffix"
                    )
                    .into_boxed_str(),
                );
                let content = bytes();
                rows.push(Row {
                    label,
                    rel,
                    needs_non_root: false,
                    expect: Expect::MismatchBlock,
                    setup: Box::new(move |p| {
                        write_gate(&p.ms(), "LOCKED");
                        write_txn(&p.ms(), state, Some("gen-1"), Some(mig));
                        std::fs::write(p.ms().join(record_file), &content).unwrap();
                        fx()
                    }),
                });
            }
            // (h) mode 000 => the read call fails => E-MAINTENANCE-002 (io)
            let label: &'static str = Box::leak(
                format!(
                    "[{mig_label} {state}] terminal record (h) mode 000 => E-MAINTENANCE-002 (io)"
                )
                .into_boxed_str(),
            );
            rows.push(Row {
                label,
                rel,
                needs_non_root: true,
                expect: Expect::Cause("io"),
                setup: Box::new(move |p| {
                    write_gate(&p.ms(), "LOCKED");
                    write_txn(&p.ms(), state, Some("gen-1"), Some(mig));
                    write_terminal_record(&p.ms(), record_file, "act-s2508", "gen-1");
                    let path = p.ms().join(record_file);
                    Fixture {
                        _modes: vec![chmod000(&path, 0o644)],
                        _lock: None,
                    }
                }),
            });
            // record ABSENT => PLAIN suffix-less block
            let label: &'static str = Box::leak(
                format!("[{mig_label} {state}] terminal record ABSENT => plain (no suffix)")
                    .into_boxed_str(),
            );
            rows.push(Row {
                label,
                rel,
                needs_non_root: false,
                expect: Expect::PlainBlock,
                setup: Box::new(move |p| {
                    write_gate(&p.ms(), "LOCKED");
                    write_txn(&p.ms(), state, Some("gen-1"), Some(mig));
                    fx()
                }),
            });
        }
    }

    for row in &rows {
        assert_row(row, &mut failures);
    }

    assert!(
        failures.is_empty(),
        "EC-032: {} of {} fixture row(s) failed:\n  - {}",
        failures.len(),
        rows.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// ADR-052 v1.20 §Downstream item 21 -- lexical root spellings
// (BC-1.18.013 v1.9 EC-023(g), EC-024(k)/(l); BC-1.18.011 v1.17 EC-018(g), EC-019)
// ---------------------------------------------------------------------------

/// Run the real binary with `CLAUDE_PROJECT_DIR` set to `proj_dir` EXACTLY AS GIVEN
/// (possibly a symlink spelling of the real project directory).
fn run_as_given(p: &Project, proj_dir: &Path, payload: &str) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", p.plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", proj_dir)
        .env("VSDD_LOG_DIR", p.root().join("logs"))
        .env_remove(SEAM_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    finish(child, Duration::from_secs(30))
}

/// `scratch/alias -> <project real dir>`; returns `(scratch, alias_path)`.
#[cfg(unix)]
fn alias_project(p: &Project) -> (tempfile::TempDir, PathBuf) {
    let scratch = tempfile::tempdir().unwrap();
    let scratch_real = scratch.path().canonicalize().unwrap();
    let alias = scratch_real.join("alias");
    std::os::unix::fs::symlink(p.root().canonicalize().unwrap(), &alias).unwrap();
    (scratch, alias)
}

#[cfg(unix)]
struct ModeGuard(PathBuf, u32);
#[cfg(unix)]
impl Drop for ModeGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(self.1));
    }
}

#[cfg(unix)]
fn running_as_root() -> bool {
    let uid = Command::new("id")
        .arg("-u")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    String::from_utf8_lossy(&uid.stdout).trim() == "0"
}

/// EC-023(g) / BC-1.18.011 EC-018(g): the project directory is reached through a
/// SYMLINK spelling, `CLAUDE_PROJECT_DIR` is the non-canonical spelling and the
/// target `file_path` is spelled through it, with `T_real` UNAVAILABLE (an
/// unresolvable ancestor, EACCES). In scope via the as-given lexical spelling:
/// blocked under a live txn; with the gate OPEN, admitted with the reservation in
/// the CANONICAL `<factory_root_real>/migration-state/reservations/`.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC023g_as_given_spelling_in_scope_when_t_real_unavailable_blackbox() {
    use std::os::unix::fs::PermissionsExt;
    if running_as_root() {
        eprintln!("SKIP: running as root; chmod 000 does not make T_real unavailable");
        return;
    }
    let mut failures: Vec<String> = Vec::new();

    for (family_rel, scope, label) in [
        (".factory/cycles", ".factory/cycles/", "cycles"),
        (
            ".factory/specs/behavioral-contracts",
            "BC-INDEX",
            "behavioral-contracts",
        ),
    ] {
        // --- live txn => blocked ---
        {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), "STAGING", Some("gen-1"), Some("migrate-bc-index"));
            let _live = p.hold_lock();
            let (_scratch, alias) = alias_project(&p);
            let locked = p.root().join(family_rel).join("locked");
            std::fs::create_dir_all(locked.join("inner")).unwrap();
            let _g = ModeGuard(locked.clone(), 0o755);
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            let fp = alias.join(family_rel).join("locked/inner/y.md");
            let out = run_as_given(
                &p,
                &alias,
                &envelope("PreToolUse", "Edit", Some("Tg1"), edit_input(&fp)),
            );
            let err = stderr_of(&out);
            if out.status.code() != Some(2)
                || !err.contains(&plain_msg(scope))
                || err.contains("completion-record mismatch")
            {
                failures.push(format!(
                    "[{label}] as-given symlink spelling with T_real unavailable must be IN SCOPE \
                     and blocked with the plain `{scope}` message; got exit {:?}, stderr: {err}",
                    out.status.code()
                ));
            }
        }
        // --- gate OPEN => admitted, reservation in the CANONICAL migration-state ---
        {
            let p = Project::new();
            let (_scratch, alias) = alias_project(&p);
            let locked = p.root().join(family_rel).join("locked");
            std::fs::create_dir_all(locked.join("inner")).unwrap();
            let _g = ModeGuard(locked.clone(), 0o755);
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            let fp = alias.join(family_rel).join("locked/inner/y.md");
            let out = run_as_given(
                &p,
                &alias,
                &envelope("PreToolUse", "Edit", Some("Tg2"), edit_input(&fp)),
            );
            let canonical = p
                .root()
                .canonicalize()
                .unwrap()
                .join(".factory/migration-state/reservations/Tg2.reservation");
            if out.status.code() != Some(0) || !canonical.exists() {
                failures.push(format!(
                    "[{label}] gate OPEN: must be admitted with the reservation in the CANONICAL \
                     {}; got exit {:?}, present={}",
                    canonical.display(),
                    out.status.code(),
                    canonical.exists()
                ));
            }
        }
    }

    assert_no_failures(
        "test_BC_1_18_013_EC023g_as_given_spelling_in_scope_when_t_real_unavailable_blackbox",
        failures,
    );
}

/// EC-024(k) / BC-1.18.011 EC-019 vector 1: the SAME protected target spelled via
/// the canonical root form and via the as-given `CLAUDE_PROJECT_DIR/.factory` form
/// is classified IDENTICALLY (same verdict, scope token, block-or-reserve outcome,
/// same canonical migration-state).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC024k_canonical_and_as_given_spellings_classified_identically_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    for (rel, scope) in [
        (".factory/cycles/c1/x.md", ".factory/cycles/"),
        (".factory/specs/behavioral-contracts/ss-01/x.md", "BC-INDEX"),
    ] {
        // live txn => both spellings blocked with the SAME message
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(
            &p.ms(),
            "COMMITTING",
            Some("gen-1"),
            Some("backfill-append-logs"),
        );
        let _live = p.hold_lock();
        let (_scratch, alias) = alias_project(&p);
        let canonical_root = p.root().canonicalize().unwrap();
        let mut stderrs = Vec::new();
        for (spelling, base) in [
            ("as-given", alias.clone()),
            ("canonical", canonical_root.clone()),
        ] {
            let fp = base.join(rel);
            std::fs::create_dir_all(fp.parent().unwrap()).unwrap();
            let out = run_as_given(
                &p,
                &alias,
                &envelope("PreToolUse", "Edit", Some("Tk1"), edit_input(&fp)),
            );
            let err = stderr_of(&out);
            if out.status.code() != Some(2)
                || !err.contains(&plain_msg(scope))
                || err.contains("completion-record mismatch")
            {
                failures.push(format!(
                    "{rel} [{spelling}] live txn must block with the plain `{scope}` message; \
                     got exit {:?}, stderr: {err}",
                    out.status.code()
                ));
            }
            stderrs.push(
                err.split("block_reason=\"")
                    .nth(1)
                    .unwrap_or("")
                    .to_string(),
            );
        }
        if stderrs[0] != stderrs[1] {
            failures.push(format!(
                "{rel}: block reason differs between spellings: {:?} vs {:?}",
                stderrs[0], stderrs[1]
            ));
        }

        // gate OPEN => both spellings reserve in the SAME canonical migration-state
        let p = Project::new();
        let (_scratch, alias) = alias_project(&p);
        let canonical_root = p.root().canonicalize().unwrap();
        for (spelling, base, id) in [
            ("as-given", alias.clone(), "Tk2"),
            ("canonical", canonical_root.clone(), "Tk3"),
        ] {
            let fp = base.join(rel);
            std::fs::create_dir_all(fp.parent().unwrap()).unwrap();
            let out = run_as_given(
                &p,
                &alias,
                &envelope("PreToolUse", "Edit", Some(id), edit_input(&fp)),
            );
            let landed = canonical_root
                .join(".factory/migration-state/reservations")
                .join(format!("{id}.reservation"));
            if out.status.code() != Some(0) || !landed.exists() {
                failures.push(format!(
                    "{rel} [{spelling}] gate OPEN: must reserve in the canonical migration-state; \
                     exit {:?}, present={}",
                    out.status.code(),
                    landed.exists()
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC024k_canonical_and_as_given_spellings_classified_identically_blackbox",
        failures,
    );
}

/// EC-024(l) / BC-1.18.011 EC-019 vector 2: a look-alike / other-project path that
/// shares a symlink-ancestor spelling with NEITHER alias of the session's own root
/// is OUT OF SCOPE (no over-match): admitted, NO reservation, NO directory, NO
/// migration-state read -- with a live txn in the session's own tree, and with
/// `T_real` both available and unavailable.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC024l_lookalike_and_sibling_paths_out_of_scope_no_over_match_blackbox() {
    use std::os::unix::fs::PermissionsExt;
    let mut failures: Vec<String> = Vec::new();
    let p = Project::new();
    write_gate(&p.ms(), "LOCKED");
    write_txn(
        &p.ms(),
        "STAGING",
        Some("gen-1"),
        Some("backfill-append-logs"),
    );
    let _live = p.hold_lock();
    let (scratch, alias) = alias_project(&p);
    let scratch_real = scratch.path().canonicalize().unwrap();

    // `<as-given-parent>/other/.factory/cycles/...` (sibling project through the
    // same symlinked parent), `<as-given-root>-old/...`, `<alias>/.factory-old/...`,
    // `<alias>/x.factory/...`.
    let siblings: Vec<(&str, PathBuf)> = vec![
        (
            "sibling project via the same parent",
            scratch_real.join("other/.factory/cycles/c/x.md"),
        ),
        (
            "<as-given-root>-old",
            scratch_real.join("alias-old/.factory/cycles/c/x.md"),
        ),
        (
            "<alias>/.factory-old",
            alias.join(".factory-old/cycles/c/x.md"),
        ),
        ("<alias>/x.factory", alias.join("x.factory/cycles/c/x.md")),
    ];
    let unresolvable_root = running_as_root();
    for (label, target) in &siblings {
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        for unresolvable in [false, true] {
            if unresolvable && unresolvable_root {
                continue;
            }
            // make T_real unavailable by locking the deepest existing ancestor's parent
            let guard = if unresolvable {
                let lock_dir = target.parent().unwrap().parent().unwrap().to_path_buf();
                std::fs::set_permissions(&lock_dir, std::fs::Permissions::from_mode(0o000))
                    .unwrap();
                Some(ModeGuard(lock_dir, 0o755))
            } else {
                None
            };
            let before = p.snapshot();
            let out = run_as_given(
                &p,
                &alias,
                &envelope("PreToolUse", "Edit", Some("Tl1"), edit_input(target)),
            );
            drop(guard);
            if out.status.code() != Some(0)
                || p.snapshot() != before
                || p.reservation("Tl1").exists()
            {
                failures.push(format!(
                    "{label} (T_real unavailable={unresolvable}): must be OUT of scope (admitted, \
                     no reservation, zero-mutation) even with a live txn; got exit {:?}: {}",
                    out.status.code(),
                    stderr_of(&out)
                ));
            }
            let stray = migration_state_dirs_under(&scratch_real);
            // the only migration-state under scratch is the session's own via the alias symlink,
            // which the walk does not follow
            if !stray.is_empty() {
                failures.push(format!("{label}: spurious namespace created: {stray:?}"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC024l_lookalike_and_sibling_paths_out_of_scope_no_over_match_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// ADR-052 v1.20 §Downstream item 22 -- replacement for the deleted
// `resolve_shard_gate_precedence` tests: real-binary structural early return
// ---------------------------------------------------------------------------

const BC_INDEX_ROLL_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100
shape = \"flat\"
";

/// BC-1.18.011 Architect Ruling 1 re-anchored (ADR-052 v1.20 item 22; BC-1.18.013
/// Precondition 6(b) O3): with a live txn, a protected `Write` to a path that
/// `shard_cap_precheck` WOULD roll (over-cap canonical + matching `[[shard]]`)
/// exits 2 `E-MAINTENANCE-001`, leaves the canonical byte-identical and publishes no
/// sealed shard (no seal/truncate). Covers both path families and both txn states.
#[test]
fn test_BC_1_18_011_PC6b_live_txn_blocks_protected_write_before_shard_cap_roll_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let cases: [(&str, &str, &str); 2] = [
        (CYCLES_PATH, CYCLES_SHARD_CONFIG, ".factory/cycles/"),
        (
            ".factory/specs/behavioral-contracts/BC-INDEX.md",
            BC_INDEX_ROLL_SHARD_CONFIG,
            "BC-INDEX",
        ),
    ];
    for (rel, config, scope) in cases {
        for state in ["STAGING", "COMMITTING"] {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), state, Some("gen-1"), Some("migrate-bc-index"));
            let _live = p.hold_lock();
            std::fs::write(p.root().join(".factory/shard-config.toml"), config).unwrap();
            let target = p.abs(rel);
            std::fs::write(&target, "y".repeat(50)).unwrap();
            let before = std::fs::read(&target).unwrap();
            let dir_before: Vec<String> = std::fs::read_dir(target.parent().unwrap())
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            let payload = envelope(
                "PreToolUse",
                "Write",
                Some("Tr1"),
                serde_json::json!({
                    "file_path": target.to_string_lossy(),
                    "content": "x".repeat(5_000),
                }),
            );
            let out = run(&p, &payload);
            let err = stderr_of(&out);
            let mut dir_after: Vec<String> = std::fs::read_dir(target.parent().unwrap())
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            let mut dir_before_sorted = dir_before.clone();
            dir_before_sorted.sort();
            dir_after.sort();
            if out.status.code() != Some(2)
                || !err.contains(&format!("E-MAINTENANCE-001: {}", plain_msg(scope)))
                || std::fs::read(&target).unwrap() != before
                || dir_after != dir_before_sorted
                || p.reservation("Tr1").exists()
            {
                failures.push(format!(
                    "[{rel}, {state}] live txn must block the over-cap protected write with \
                     `E-MAINTENANCE-001: {}` BEFORE shard_cap_precheck: canonical byte-identical, \
                     no sealed shard, no reservation; got exit {:?}, canonical_unchanged={}, \
                     dir {dir_before_sorted:?} -> {dir_after:?}, stderr: {err}",
                    plain_msg(scope),
                    out.status.code(),
                    std::fs::read(&target).unwrap() == before
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_PC6b_live_txn_blocks_protected_write_before_shard_cap_roll_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L2-003 -- positive control: with the gate OPEN and no txn, the SAME
// fixtures DO roll (seal + truncate), so the blocked-case byte-identity above is
// meaningful (it is not a fixture that never rolls).
// ---------------------------------------------------------------------------

/// Same fixtures as `..._live_txn_blocks_protected_write_before_shard_cap_roll_blackbox`
/// and AC-002 part 2 (cap 100, 50-byte canonical, 5,000-byte Write), gate OPEN, no
/// txn: `shard_cap_precheck` MUST roll -- the canonical is truncated away from its
/// original bytes and a sealed shard file is published next to it.
#[test]
fn test_BC_1_18_011_PC6b_positive_control_gate_open_same_fixture_rolls_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let cases: [(&str, &str); 2] = [
        (CYCLES_PATH, CYCLES_SHARD_CONFIG),
        (
            ".factory/specs/behavioral-contracts/BC-INDEX.md",
            BC_INDEX_ROLL_SHARD_CONFIG,
        ),
    ];
    for (rel, config) in cases {
        let p = Project::new(); // gate OPEN, no txn
        std::fs::write(p.root().join(".factory/shard-config.toml"), config).unwrap();
        let target = p.abs(rel);
        std::fs::write(&target, "y".repeat(50)).unwrap();
        let before = std::fs::read(&target).unwrap();
        let mut dir_before: Vec<String> = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        dir_before.sort();
        let payload = envelope(
            "PreToolUse",
            "Write",
            Some("Tp1"),
            serde_json::json!({
                "file_path": target.to_string_lossy(),
                "content": "x".repeat(5_000),
            }),
        );
        let out = run(&p, &payload);
        let after = std::fs::read(&target).unwrap_or_default();
        let mut dir_after: Vec<String> = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        dir_after.sort();
        let truncated = after != before;
        let new_shard = dir_after.len() > dir_before.len();
        if !truncated || !new_shard {
            failures.push(format!(
                "[{rel}] POSITIVE CONTROL: with the gate OPEN the over-cap Write must make \
                 shard_cap_precheck roll (canonical changed AND a sealed shard published); got \
                 exit {:?}, canonical_changed={truncated}, new_shard_file={new_shard}, \
                 dir {dir_before:?} -> {dir_after:?}, stderr: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_PC6b_positive_control_gate_open_same_fixture_rolls_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L2-010 black-box spot-check -- relative CLAUDE_PROJECT_DIR
// ---------------------------------------------------------------------------

/// BC-1.18.013 Pre 6(b)(i) spelling (ii): a RELATIVE `CLAUDE_PROJECT_DIR` must not
/// break or widen the gate. The child runs with its cwd = the project's parent and
/// `CLAUDE_PROJECT_DIR=proj`: a protected write inside `proj` is still gated (live
/// txn => blocked) and a protected-looking path in a SIBLING project is out of scope.
/// (The "no relative alias is added" rule itself is unit-tested through
/// `as_given_factory_root_spelling`.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_PC6b_relative_claude_project_dir_spot_check_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let parent = tempfile::tempdir().unwrap();
    let parent_real = parent.path().canonicalize().unwrap();
    let proj = parent_real.join("proj");
    let sibling = parent_real.join("sibling");
    for d in [&proj, &sibling] {
        std::fs::create_dir_all(d.join(".factory/cycles/c1")).unwrap();
    }
    let plugin_root = tempfile::tempdir().unwrap();
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .unwrap();
    let ms = proj.join(".factory/migration-state");
    std::fs::create_dir_all(ms.join("reservations")).unwrap();
    std::fs::write(ms.join("exclusive.lock"), b"").unwrap();
    write_gate(&ms, "LOCKED");
    write_txn(&ms, "STAGING", Some("gen-1"), Some("backfill-append-logs"));
    let _live = try_acquire_migration_lock(&ms.join("exclusive.lock"))
        .unwrap()
        .unwrap();

    let run_rel = |target: &Path, id: &str| -> Output {
        let mut cmd = Command::new(binary_path());
        cmd.current_dir(&parent_real)
            .env("CLAUDE_PLUGIN_ROOT", plugin_root.path())
            .env("CLAUDE_PROJECT_DIR", "proj")
            .env("VSDD_LOG_DIR", parent_real.join("logs"))
            .env_remove(SEAM_ENV)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn");
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(envelope("PreToolUse", "Edit", Some(id), edit_input(target)).as_bytes())
            .unwrap();
        drop(stdin);
        finish(child, Duration::from_secs(30))
    };

    let inside = run_rel(&proj.join(".factory/cycles/c1/x.md"), "Tr1");
    if inside.status.code() != Some(2)
        || !stderr_of(&inside).contains(&plain_msg(".factory/cycles/"))
    {
        failures.push(format!(
            "relative CLAUDE_PROJECT_DIR: a protected write inside the project must stay gated; \
             got exit {:?}",
            inside.status.code()
        ));
    }
    let outside = run_rel(&sibling.join(".factory/cycles/c1/x.md"), "Tr2");
    if outside.status.code() != Some(0) {
        failures.push(format!(
            "relative CLAUDE_PROJECT_DIR: a sibling project's protected-looking path must be out \
             of scope (admitted); got exit {:?}",
            outside.status.code()
        ));
    }
    if !migration_state_dirs_under(&sibling).is_empty() {
        failures.push(
            "relative CLAUDE_PROJECT_DIR: a namespace was created in the sibling project".into(),
        );
    }
    assert_no_failures(
        "test_BC_1_18_013_PC6b_relative_claude_project_dir_spot_check_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// Pass-3 F-S2508-L3-001 -- minimal-shape (interpretability) of a foreign txn
// ---------------------------------------------------------------------------

/// Row over a raw `txn-*.json` body with an explicit gate state and path family.
#[cfg(unix)]
fn raw_txn_row(
    label: &'static str,
    gate: &'static str,
    rel: &'static str,
    bytes: &'static [u8],
    expect: Expect,
) -> Row {
    Row {
        label,
        rel,
        needs_non_root: false,
        expect,
        setup: Box::new(move |p| {
            write_gate(&p.ms(), gate);
            write_raw_txn(p, "txn-act-s2508.json", bytes);
            fx()
        }),
    }
}

/// F-S2508-L3-001. BC-1.18.013 v1.11 Precondition 6(c) rule 3: "a txn record must
/// be a JSON object whose `state` is a known txn state and whose PRESENT
/// `migration_id` is a JSON string"; Precondition 6 (foreign): "A live txn whose
/// `migration_id` is a string NOT in K ... is FOREIGN: `RefuseForeignMigration`
/// (plain `E-MAINTENANCE-001`, no suffix, precedence over every record check) ...
/// a record this build cannot interpret is not this build's to discard" (EC-029).
/// ADR-052 §7e "Definition of foreign": `migration_id ∉ K`. So a foreign record
/// that carries ONLY `state` + `migration_id` (a newer build's schema) is a plain
/// refusal, never `E-MAINTENANCE-002 (state_integrity)`; a NON-live foreign record
/// is not live, so admission ignores it (dual check "gate OPEN AND no live txn").
/// Both path families.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC029_minimal_shape_foreign_txn_is_foreign_not_state_integrity_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();

    for rel in [BC_PATH, CYCLES_PATH] {
        // (a) live foreign, minimal shape => plain E-MAINTENANCE-001, byte-identical.
        for (state_label, bytes) in [
            (
                "STAGING",
                &br#"{"state":"STAGING","migration_id":"future-migration"}"#[..],
            ),
            (
                "COMMITTING",
                &br#"{"state":"COMMITTING","migration_id":"future-migration"}"#[..],
            ),
        ] {
            for gate in ["LOCKED", "OPEN"] {
                let label: &'static str = Box::leak(
                    format!(
                        "(a) [{rel}] gate {gate}: minimal live {state_label} foreign record => \
                         plain E-MAINTENANCE-001 (not -002)"
                    )
                    .into_boxed_str(),
                );
                rows.push(raw_txn_row(label, gate, rel, bytes, Expect::PlainBlock));
            }
        }
        // (b)/(c) minimal NON-live foreign records are not live => admitted (gate OPEN).
        for (state_label, bytes) in [
            (
                "COMPLETED",
                &br#"{"state":"COMPLETED","migration_id":"future-migration"}"#[..],
            ),
            (
                "ABORTED",
                &br#"{"state":"ABORTED","migration_id":"future-migration"}"#[..],
            ),
        ] {
            let label: &'static str = Box::leak(
                format!(
                    "(b/c) [{rel}] gate OPEN: minimal NON-live {state_label} foreign record => \
                     admitted (not live, not -002)"
                )
                .into_boxed_str(),
            );
            rows.push(raw_txn_row(label, "OPEN", rel, bytes, Expect::Admit));
        }
        // Controls (spec-backed shape violations stay state_integrity, with a foreign id):
        // missing `state`, an UNKNOWN `state`, and a non-object body.
        for (what, bytes) in [
            (
                "missing state",
                &br#"{"migration_id":"future-migration"}"#[..],
            ),
            (
                "unknown state",
                &br#"{"state":"WEIRD","migration_id":"future-migration"}"#[..],
            ),
            ("non-object body (array)", &br#"["STAGING"]"#[..]),
        ] {
            let label: &'static str = Box::leak(
                format!("(control) [{rel}] foreign-id record with {what} => state_integrity")
                    .into_boxed_str(),
            );
            rows.push(raw_txn_row(
                label,
                "LOCKED",
                rel,
                bytes,
                Expect::Cause("state_integrity"),
            ));
        }
    }
    for row in &rows {
        assert_row(row, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_013_EC029_minimal_shape_foreign_txn_is_foreign_not_state_integrity_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// Pass-3 F-S2508-L3-002 -- non-string tool_name must still release
// ---------------------------------------------------------------------------

/// F-S2508-L3-002. BC-1.18.013 v1.11 EC-021: a `PostToolUseFailure` for an admitted
/// `tool_use_id` REMOVES the reservation "keyed only on `tool_use_id`, no
/// `tool_name` filter", "including ... (b) an envelope with NO `tool_name`, (c) an
/// envelope with a differently-shaped `tool_name`"; ADR-052 §5a F-001. A `null`,
/// numeric, boolean, array or object `tool_name` is differently shaped, so the
/// envelope must still release (and exit 0).
#[test]
fn test_BC_1_18_013_EC021c_posttoolusefailure_non_string_tool_name_still_releases_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let shapes: [(&str, Option<serde_json::Value>); 6] = [
        ("tool_name null", Some(serde_json::Value::Null)),
        ("tool_name number", Some(serde_json::json!(7))),
        ("tool_name bool", Some(serde_json::json!(true))),
        ("tool_name array", Some(serde_json::json!(["Edit"]))),
        ("tool_name object", Some(serde_json::json!({"n": "Edit"}))),
        ("tool_name absent", None),
    ];
    for (label, tool_name) in shapes {
        let p = Project::new();
        let target = p.abs(CYCLES_PATH);
        let pre = run(
            &p,
            &envelope("PreToolUse", "Write", Some("TN1"), edit_input(&target)),
        );
        if pre.status.code() != Some(0) || !p.reservation("TN1").exists() {
            failures.push(format!(
                "[{label}] precondition: PreToolUse Write must admit and reserve TN1 (exit {:?})",
                pre.status.code()
            ));
            continue;
        }
        let mut v = serde_json::json!({
            "hook_event_name": "PostToolUseFailure",
            "session_id": "sess-s2508",
            "tool_input": edit_input(&target),
            "tool_use_id": "TN1",
            "error": "boom",
            "is_interrupt": false,
        });
        if let Some(t) = tool_name {
            v["tool_name"] = t;
        }
        let out = run(&p, &v.to_string());
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] PostToolUseFailure must exit 0, got {:?}",
                out.status.code()
            ));
        }
        if p.reservation("TN1").exists() {
            failures.push(format!(
                "[{label}] PostToolUseFailure must REMOVE reservations/TN1.reservation \
                 (EC-021(c): keyed only on tool_use_id, no tool_name filter); it is still there. \
                 stderr: {}",
                stderr_of(&out)
            ));
        }
    }

    // PreToolUse with a non-string tool_name cannot be classified as Edit/Write/
    // MultiEdit (ADR-052 §5a evaluation position O4: "an unparseable payload cannot
    // be classified"): it must not be treated as a protected write -- exit 0, no
    // reservation, no migration-state mutation.
    for (label, tool_name) in [
        ("PreToolUse tool_name null", serde_json::Value::Null),
        ("PreToolUse tool_name object", serde_json::json!({"n": 1})),
    ] {
        let p = Project::new();
        let target = p.abs(CYCLES_PATH);
        let before = p.snapshot();
        let v = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": tool_name,
            "session_id": "sess-s2508",
            "tool_input": edit_input(&target),
            "tool_use_id": "TN2",
        });
        let out = run(&p, &v.to_string());
        if out.status.code() != Some(0) || p.reservation("TN2").exists() || p.snapshot() != before {
            failures.push(format!(
                "[{label}] must be an unclassifiable no-op (exit 0, no reservation, tree \
                 unchanged); got exit {:?}",
                out.status.code()
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC021c_posttoolusefailure_non_string_tool_name_still_releases_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// Pass-3 F-S2508-L3-007 -- EISDIR (a directory at the path) is io, root-safe
// ---------------------------------------------------------------------------

/// F-S2508-L3-007. BC-1.18.013 v1.11 Precondition 6(c) rule 2 (`io`): a failed
/// read CALL of a txn record or a terminal record is `E-MAINTENANCE-002 (io)`.
/// A DIRECTORY at the file's path makes `read` fail with EISDIR for EVERY uid
/// (unlike chmod 000, which root bypasses), so these rows run under root too.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC032_eisdir_directory_at_record_path_is_io_root_safe_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();

    for (mig, record_file, rel) in [
        ("migrate-bc-index", "completed.json", BC_PATH),
        (
            "backfill-append-logs",
            "completed-backfill-append-logs.json",
            CYCLES_PATH,
        ),
    ] {
        for state in ["STAGING", "COMMITTING"] {
            let label: &'static str = Box::leak(
                format!("[{mig} {state}] terminal record path is a DIRECTORY => -002 (io)")
                    .into_boxed_str(),
            );
            rows.push(Row {
                label,
                rel,
                needs_non_root: false,
                expect: Expect::Cause("io"),
                setup: Box::new(move |p| {
                    write_gate(&p.ms(), "LOCKED");
                    write_txn(&p.ms(), state, Some("gen-1"), Some(mig));
                    std::fs::create_dir(p.ms().join(record_file)).unwrap();
                    fx()
                }),
            });
        }
    }
    for rel in [BC_PATH, CYCLES_PATH] {
        for gate in ["OPEN", "LOCKED"] {
            let label: &'static str = Box::leak(
                format!("[{rel}] gate {gate}: a txn-*.json path that is a DIRECTORY => -002 (io)")
                    .into_boxed_str(),
            );
            rows.push(Row {
                label,
                rel,
                needs_non_root: false,
                expect: Expect::Cause("io"),
                setup: Box::new(move |p| {
                    write_gate(&p.ms(), gate);
                    std::fs::create_dir(p.ms().join("txn-dir.json")).unwrap();
                    fx()
                }),
            });
        }
    }
    for row in &rows {
        assert_row(row, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_013_EC032_eisdir_directory_at_record_path_is_io_root_safe_blackbox",
        failures,
    );
}

/// BC-1.18.013 v1.11 EC-021 / ADR-052 §5a F-001: release is keyed only on
/// `tool_use_id` for BOTH completion events (`is_tool_completion_event` is true for
/// `PostToolUse` and `PostToolUseFailure`), so a `PostToolUse` whose `tool_name`
/// is null / numeric / boolean / an array / an object must still release.
#[test]
fn test_BC_1_18_013_EC021c_posttooluse_non_string_tool_name_still_releases_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let shapes: [(&str, serde_json::Value); 5] = [
        ("tool_name null", serde_json::Value::Null),
        ("tool_name number", serde_json::json!(7)),
        ("tool_name bool", serde_json::json!(true)),
        ("tool_name array", serde_json::json!(["Write"])),
        ("tool_name object", serde_json::json!({"n": "Write"})),
    ];
    for (label, tool_name) in shapes {
        let p = Project::new();
        let target = p.abs(CYCLES_PATH);
        let pre = run(
            &p,
            &envelope("PreToolUse", "Write", Some("TP1"), edit_input(&target)),
        );
        if pre.status.code() != Some(0) || !p.reservation("TP1").exists() {
            failures.push(format!(
                "[{label}] precondition: PreToolUse Write must admit and reserve TP1 (exit {:?})",
                pre.status.code()
            ));
            continue;
        }
        let v = serde_json::json!({
            "hook_event_name": "PostToolUse",
            "tool_name": tool_name,
            "session_id": "sess-s2508",
            "tool_input": edit_input(&target),
            "tool_response": {"ok": true},
            "tool_use_id": "TP1",
        });
        let out = run(&p, &v.to_string());
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] PostToolUse must exit 0, got {:?}",
                out.status.code()
            ));
        }
        if p.reservation("TP1").exists() {
            failures.push(format!(
                "[{label}] PostToolUse must REMOVE reservations/TP1.reservation (keyed only on \
                 tool_use_id); it is still there. stderr: {}",
                stderr_of(&out)
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC021c_posttooluse_non_string_tool_name_still_releases_blackbox",
        failures,
    );
}

// ===========================================================================
// ADR-052 v1.23 -- adversary pass-3 rulings, real spawned binary
//
//  * F-S2508-L3-009: section 5a "Factory-root lookup mapping" (the `.factory`
//    stat is classified Found / Absent / Unstatable; admission leg + release leg).
//    The coordinator `Io` mapping is S-25.09 and is NOT exercised here.
//  * F-S2508-L3-001: section "Error Code Semantics" -> "Txn-record
//    interpretation -- tiers" (Tier 0 shape at read; Tier 1 fields read lazily
//    by the consuming branch only).
//
// The BC mirror (BC-1.18.013 v1.12) is being written in parallel; the ADR is
// authoritative, so these tests cite ADR sections. Only BC ECs that already
// exist (EC-029, EC-035) are cited.
// ===========================================================================

#[cfg(unix)]
fn bare_project() -> Project {
    let dir = tempfile::tempdir().expect("project tempdir");
    let plugin_root = tempfile::tempdir().expect("plugin_root tempdir");
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .expect("write empty registry");
    Project { dir, plugin_root }
}

/// Run the real binary with an explicit `CLAUDE_PROJECT_DIR` AND a log directory
/// OUTSIDE the project (so a chmod-000 project root never blocks the log and the
/// project tree is exactly what the dispatcher left).
#[cfg(unix)]
fn run_at(p: &Project, proj_dir: &Path, log_dir: &Path, payload: &str) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", p.plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", proj_dir)
        .env("VSDD_LOG_DIR", log_dir)
        .env_remove(SEAM_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    finish(child, Duration::from_secs(30))
}

/// lstat-based snapshot of a whole tree (symlinks are recorded, never followed).
#[cfg(unix)]
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

#[cfg(unix)]
fn factory_target(proj: &Path) -> PathBuf {
    proj.join(".factory/cycles/c1/burst-log.md")
}

/// Block reason text of the stderr summary line (what the operator sees).
#[cfg(unix)]
fn reason_of(err: &str) -> String {
    err.split("block_reason=\"")
        .nth(1)
        .unwrap_or(err)
        .to_string()
}

/// F-S2508-L3-009, ADR-052 v1.23 section 5a "Factory-root lookup mapping" (b)
/// (BC-1.18.013 EC-035 for the regular-file row). `<project_root>/.factory` ABSENT
/// is the CLOSED set {stat succeeds on a non-directory (regular file); ENOENT
/// (dangling symlink); ENOTDIR (a path-prefix component, incl. the project root
/// itself, is a regular file)}. On the PreToolUse admission leg the gate is OUT
/// OF SCOPE: admitted (exit 0), no reservation, NOTHING created. The completion
/// legs are a silent no-op. Root-safe (no chmod).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_absent_factory_root_closed_set_is_out_of_scope_admitted_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    type Setup = fn(&Path) -> PathBuf;
    let rows: [(&str, Setup); 4] = [
        ("`.factory` is a REGULAR FILE (EC-035)", |root| {
            std::fs::write(root.join(".factory"), b"not a directory").unwrap();
            root.to_path_buf()
        }),
        ("`.factory` is a DANGLING symlink (ENOENT)", |root| {
            std::os::unix::fs::symlink(root.join("no-such-target"), root.join(".factory")).unwrap();
            root.to_path_buf()
        }),
        (
            "a path-prefix component of the project root is a regular file (ENOTDIR)",
            |root| {
                std::fs::write(root.join("afile"), b"x").unwrap();
                root.join("afile").join("proj")
            },
        ),
        (
            "the project root itself is a regular file (ENOTDIR)",
            |root| {
                std::fs::write(root.join("afile"), b"x").unwrap();
                root.join("afile")
            },
        ),
    ];
    for (label, setup) in rows {
        for tool in ["Edit", "Write", "MultiEdit"] {
            let p = bare_project();
            let logs = tempfile::tempdir().unwrap();
            let proj = setup(p.root());
            let before = raw_tree(p.root());
            let target = factory_target(&proj);
            let pre = run_at(
                &p,
                &proj,
                logs.path(),
                &envelope("PreToolUse", tool, Some("TA1"), edit_input(&target)),
            );
            let err = stderr_of(&pre);
            if pre.status.code() != Some(0) || err.contains("E-MAINTENANCE") {
                failures.push(format!(
                    "[{label}] {tool}: Absent must be admitted (exit 0, no E-MAINTENANCE-*); got \
                     exit {:?}, stderr: {err}",
                    pre.status.code()
                ));
            }
            if !migration_state_dirs_under(p.root()).is_empty() {
                failures.push(format!(
                    "[{label}] {tool}: Absent must create NOTHING; found {:?}",
                    migration_state_dirs_under(p.root())
                ));
            }
            // Completion legs: silent no-op, exit 0, nothing created.
            for ev in ["PostToolUse", "PostToolUseFailure"] {
                let post = run_at(
                    &p,
                    &proj,
                    logs.path(),
                    &envelope(ev, tool, Some("TA1"), edit_input(&target)),
                );
                if post.status.code() != Some(0) || stderr_of(&post).contains("E-MAINTENANCE") {
                    failures.push(format!(
                        "[{label}] {tool}/{ev}: Absent release must be a silent no-op (exit 0); \
                         got {:?}: {}",
                        post.status.code(),
                        stderr_of(&post)
                    ));
                }
            }
            if raw_tree(p.root()) != before {
                failures.push(format!(
                    "[{label}] {tool}: the project tree changed (Absent must create/mutate nothing)"
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_absent_factory_root_closed_set_is_out_of_scope_admitted_blackbox",
        failures,
    );
}

/// Shared assertions for an UNSTATABLE `.factory` on the admission leg.
#[cfg(unix)]
fn assert_unstatable_admission_fails_closed(
    label: &str,
    tool: &str,
    out: &Output,
    failures: &mut Vec<String>,
) {
    let err = stderr_of(out);
    let reason = reason_of(&err);
    let want = "E-MAINTENANCE-002: writer-admission check failed (io)";
    if out.status.code() != Some(2) || !err.contains(want) {
        failures.push(format!(
            "[{label}] {tool}: expected exit 2 + `{want}`; got exit {:?}, stderr: {err}",
            out.status.code()
        ));
    }
    // Not the absent verdicts, not a block, and the message carries ONLY the
    // cause token (no path / errno text).
    for bad in [
        "FACTORY_ROOT_NOT_FOUND",
        "E-MAINTENANCE-001",
        "os error",
        "Permission denied",
        "Too many levels",
        ".factory",
    ] {
        if reason.contains(bad) {
            failures.push(format!(
                "[{label}] {tool}: block reason must not contain `{bad}`: {reason}"
            ));
        }
    }
}

/// F-S2508-L3-009, ADR-052 v1.23 section 5a "Factory-root lookup mapping" (c) +
/// section "Error Code Semantics" "Factory-root stat failure". A self-referential
/// symlink `.factory` makes the one `stat` fail with ELOOP (for EVERY uid, so this
/// row runs under root): existence is UNKNOWN => the admission leg FAILS CLOSED,
/// `E-MAINTENANCE-002 (io)`, exit 2, no reservation, tree unchanged. NOT admitted
/// (the pre-v1.23 `is_ok_and(is_dir)` collapse admitted it).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_unstatable_factory_eloop_fails_closed_io_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    for tool in ["Edit", "Write", "MultiEdit"] {
        let p = bare_project();
        let logs = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(".factory", p.root().join(".factory")).unwrap();
        let before = raw_tree(p.root());
        let target = factory_target(p.root());
        let out = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope("PreToolUse", tool, Some("TL1"), edit_input(&target)),
        );
        assert_unstatable_admission_fails_closed("ELOOP", tool, &out, &mut failures);
        if raw_tree(p.root()) != before {
            failures.push(format!("[ELOOP] {tool}: the project tree changed"));
        }
        if !migration_state_dirs_under(p.root()).is_empty() {
            failures.push(format!(
                "[ELOOP] {tool}: a migration-state/reservations dir was created"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_unstatable_factory_eloop_fails_closed_io_blackbox",
        failures,
    );
}

/// F-S2508-L3-009, same ruling, EACCES instance: the PROJECT ROOT (parent of
/// `.factory`) is `chmod 000`, so `stat(<root>/.factory)` fails EACCES. Skipped as
/// root (root bypasses the permission check).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_unstatable_factory_eacces_fails_closed_io_blackbox() {
    if is_root() {
        eprintln!("SKIP: running as root; chmod 000 does not make the stat fail");
        return;
    }
    let mut failures: Vec<String> = Vec::new();
    for tool in ["Edit", "Write", "MultiEdit"] {
        let p = Project::new(); // a REAL `.factory/migration-state` exists under the root
        let logs = tempfile::tempdir().unwrap();
        let target = factory_target(p.root());
        let before = raw_tree(p.root());
        let out;
        {
            let _restore = chmod000(p.root(), 0o755);
            out = run_at(
                &p,
                p.root(),
                logs.path(),
                &envelope("PreToolUse", tool, Some("TE1"), edit_input(&target)),
            );
        }
        assert_unstatable_admission_fails_closed("EACCES", tool, &out, &mut failures);
        if p.reservation("TE1").exists() {
            failures.push(format!("[EACCES] {tool}: a reservation was left"));
        }
        if raw_tree(p.root()) != before {
            failures.push(format!(
                "[EACCES] {tool}: the project tree changed (fail-closed must write nothing)"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_unstatable_factory_eacces_fails_closed_io_blackbox",
        failures,
    );
}

/// F-S2508-L3-009, ADR-052 v1.23 section 5a: the `.factory` stat is evaluated
/// "after the input guards" and only for Edit/Write/MultiEdit with a valid
/// `file_path`. With an UNSTATABLE `.factory`, a Bash dispatch, a Read, and an
/// Edit with an absent / empty / non-string `file_path` are NOT gated (exit 0):
/// the blast radius is exactly the gated tools ("`Bash` is not gated by this leg,
/// so the operator can repair the mount/permission").
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_unstatable_factory_input_guards_precede_stat_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let p = bare_project();
    let logs = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(".factory", p.root().join(".factory")).unwrap();
    let before = raw_tree(p.root());
    let target = factory_target(p.root());
    let cases: [(&str, String); 5] = [
        (
            "Bash",
            envelope(
                "PreToolUse",
                "Bash",
                Some("TG1"),
                serde_json::json!({"command": "true"}),
            ),
        ),
        (
            "Read",
            envelope(
                "PreToolUse",
                "Read",
                Some("TG2"),
                serde_json::json!({"file_path": target.to_string_lossy()}),
            ),
        ),
        (
            "Edit with NO file_path",
            envelope("PreToolUse", "Edit", Some("TG3"), serde_json::json!({})),
        ),
        (
            "Edit with EMPTY file_path",
            envelope(
                "PreToolUse",
                "Edit",
                Some("TG4"),
                serde_json::json!({"file_path": ""}),
            ),
        ),
        (
            "Edit with non-string file_path",
            envelope(
                "PreToolUse",
                "Edit",
                Some("TG5"),
                serde_json::json!({"file_path": 7}),
            ),
        ),
    ];
    for (label, payload) in cases {
        let out = run_at(&p, p.root(), logs.path(), &payload);
        let err = stderr_of(&out);
        if out.status.code() != Some(0) || err.contains("E-MAINTENANCE") {
            failures.push(format!(
                "[{label}] must not be gated by the `.factory` stat; got exit {:?}: {err}",
                out.status.code()
            ));
        }
    }
    if raw_tree(p.root()) != before {
        failures.push("the project tree changed".to_string());
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_unstatable_factory_input_guards_precede_stat_blackbox",
        failures,
    );
}

/// F-S2508-L3-009, ADR-052 v1.23 section 5a mapping table, release row: with an
/// UNSTATABLE `.factory` a PostToolUse / PostToolUseFailure yields NO verdict
/// (exit 0, no E-MAINTENANCE-*; "release is never a verdict"), creates and
/// deletes nothing; the reservation admitted BEFORE the fault was injected is
/// left in place for the drain-start TTL GC. Two fault kinds (ELOOP root-safe via
/// `.factory` -> self-link with the real directory renamed aside; EACCES via
/// chmod 000 root, skipped as root). A no-fault control proves the release leg
/// otherwise removes the reservation.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_unstatable_factory_release_leg_no_verdict_reservation_left_blackbox()
 {
    let mut failures: Vec<String> = Vec::new();

    // Control: no fault => PostToolUse removes the reservation.
    {
        let p = Project::new();
        let logs = tempfile::tempdir().unwrap();
        let target = p.abs(CYCLES_PATH);
        let pre = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope("PreToolUse", "Write", Some("TR0"), edit_input(&target)),
        );
        if pre.status.code() != Some(0) || !p.reservation("TR0").exists() {
            failures.push("control: PreToolUse must admit and reserve TR0".to_string());
        }
        let post = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope("PostToolUse", "Write", Some("TR0"), edit_input(&target)),
        );
        if post.status.code() != Some(0) || p.reservation("TR0").exists() {
            failures
                .push("control: PostToolUse (no fault) must remove the reservation".to_string());
        }
    }

    for kind in ["ELOOP", "EACCES"] {
        if kind == "EACCES" && is_root() {
            eprintln!("SKIP [EACCES]: running as root");
            continue;
        }
        for ev in ["PostToolUse", "PostToolUseFailure"] {
            let p = Project::new();
            let logs = tempfile::tempdir().unwrap();
            let target = p.abs(CYCLES_PATH);
            let pre = run_at(
                &p,
                p.root(),
                logs.path(),
                &envelope("PreToolUse", "Write", Some("TR1"), edit_input(&target)),
            );
            if pre.status.code() != Some(0) || !p.reservation("TR1").exists() {
                failures.push(format!("[{kind}/{ev}] precondition: admit + reserve TR1"));
                continue;
            }
            let res_path = if kind == "ELOOP" {
                p.root()
                    .join(".factory-real/migration-state/reservations/TR1.reservation")
            } else {
                p.reservation("TR1")
            };
            let res_bytes_before;
            let out;
            let before;
            if kind == "ELOOP" {
                // Inject the fault AFTER the reservation exists.
                std::fs::rename(p.root().join(".factory"), p.root().join(".factory-real")).unwrap();
                std::os::unix::fs::symlink(".factory", p.root().join(".factory")).unwrap();
                res_bytes_before = std::fs::read(&res_path).unwrap();
                before = raw_tree(p.root());
                out = run_at(
                    &p,
                    p.root(),
                    logs.path(),
                    &envelope(ev, "Write", Some("TR1"), edit_input(&target)),
                );
            } else {
                res_bytes_before = std::fs::read(&res_path).unwrap();
                before = raw_tree(p.root());
                let _restore = chmod000(p.root(), 0o755);
                out = run_at(
                    &p,
                    p.root(),
                    logs.path(),
                    &envelope(ev, "Write", Some("TR1"), edit_input(&target)),
                );
            }
            let err = stderr_of(&out);
            if out.status.code() != Some(0) || err.contains("E-MAINTENANCE") {
                failures.push(format!(
                    "[{kind}/{ev}] release with an unstatable `.factory` must yield NO verdict \
                     (exit 0, no E-MAINTENANCE-*); got {:?}: {err}",
                    out.status.code()
                ));
            }
            match std::fs::read(&res_path) {
                Ok(b) if b == res_bytes_before => {}
                other => failures.push(format!(
                    "[{kind}/{ev}] the reservation must be LEFT UNCHANGED for the TTL GC; got {other:?}"
                )),
            }
            if raw_tree(p.root()) != before {
                failures.push(format!(
                    "[{kind}/{ev}] nothing may be created or deleted by the release no-op"
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_unstatable_factory_release_leg_no_verdict_reservation_left_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L3-001 -- ADR-052 v1.23 "Txn-record interpretation -- tiers"
// ---------------------------------------------------------------------------

/// (migration_id as written, protected path family, own terminal-record name).
/// `None` = the pre-v1.11 B2 record (migration_id ABSENT).
#[cfg(unix)]
const TIER_FAMILIES: [(Option<&str>, &str, &str); 3] = [
    (Some("migrate-bc-index"), BC_PATH, "completed.json"),
    (
        Some("backfill-append-logs"),
        CYCLES_PATH,
        "completed-backfill-append-logs.json",
    ),
    (None, BC_PATH, "completed.json"),
];

#[cfg(unix)]
const TIER_GATES: [&str; 3] = ["DRAINING", "LOCKED", "OPEN"];

/// A MINIMAL (Tier 0 only) record: `state` plus, optionally, `migration_id`, plus
/// the given extra keys. NO other Decision-7a field.
#[cfg(unix)]
fn minimal_txn(state: &str, mig: Option<&str>, extra: &[(&str, serde_json::Value)]) -> Vec<u8> {
    let mut v = serde_json::json!({ "state": state });
    if let Some(m) = mig {
        v["migration_id"] = serde_json::Value::String(m.to_string());
    }
    for (k, val) in extra {
        v[*k] = val.clone();
    }
    serde_json::to_vec(&v).unwrap()
}

/// A FULL Decision-7a record whose `generation_id` is the given value, or whose
/// `generation_id` KEY IS REMOVED when `gen_id` is `None` (serde's `Option`
/// default would read a removed key as `None`, i.e. as the null generation).
#[cfg(unix)]
fn full_txn(state: &str, mig: Option<&str>, gen_id: Option<serde_json::Value>) -> Vec<u8> {
    let mut v = serde_json::json!({
        "txn_id": "txn-s2508",
        "activation_id": "act-s2508",
        "fencing_generation": 1,
        "state": state,
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": null,
        "pending_canonical_moves": [],
        "created_at": "2026-10-06T00:00:00Z",
        "updated_at": "2026-10-06T00:00:00Z",
    });
    if let Some(g) = gen_id {
        v["generation_id"] = g;
    }
    if let Some(m) = mig {
        v["migration_id"] = serde_json::Value::String(m.to_string());
    }
    serde_json::to_vec(&v).unwrap()
}

/// Gate + the given raw txn files (+ the family's terminal record when
/// `terminal`); the coordinator lock is NOT held (acquirable).
#[cfg(unix)]
fn tier_row(
    label: String,
    gate: &'static str,
    rel: &'static str,
    files: Vec<(&'static str, Vec<u8>)>,
    terminal: Option<&'static str>,
    expect: Expect,
) -> Row {
    Row {
        label: Box::leak(label.into_boxed_str()),
        rel,
        needs_non_root: false,
        expect,
        setup: Box::new(move |p| {
            write_gate(&p.ms(), gate);
            for (name, bytes) in &files {
                write_raw_txn(p, name, bytes);
            }
            if let Some(t) = terminal {
                write_terminal_record(&p.ms(), t, "txn-s2508", "gen-1");
            }
            fx()
        }),
    }
}

#[cfg(unix)]
fn run_rows(test: &str, rows: &[Row]) {
    let mut failures: Vec<String> = Vec::new();
    for row in rows {
        assert_row(row, &mut failures);
    }
    assert_no_failures(test, failures);
}

/// F-S2508-L3-001, ADR-052 v1.23 Tier 1 table, Branch B row: STAGING, lock
/// acquirable, own terminal record ABSENT => `generation_id` is read; an ABSENT
/// key is NOT null and any non-null/non-string type is unusable => `E-MAINTENANCE-002
/// (state_integrity)` with NO txn or gate write (tree byte-identical). Covers the
/// minimal record (Tier 0 shape only) AND a full Decision-7a record with the key
/// removed (serde's `Option` default would read it as null and DISCARD).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_branch_b_absent_or_ill_typed_generation_id_is_state_integrity()
 {
    let mut rows: Vec<Row> = Vec::new();
    for (mig, rel, _t) in TIER_FAMILIES {
        for gate in TIER_GATES {
            let mut cases: Vec<(String, Vec<u8>)> = vec![
                (
                    "minimal STAGING, generation_id ABSENT".to_string(),
                    minimal_txn("STAGING", mig, &[]),
                ),
                (
                    "FULL STAGING with the generation_id KEY REMOVED".to_string(),
                    full_txn("STAGING", mig, None),
                ),
            ];
            for (what, val) in [
                ("number 7", serde_json::json!(7)),
                ("boolean true", serde_json::json!(true)),
                ("array", serde_json::json!(["gen-1"])),
                ("object", serde_json::json!({"id": "gen-1"})),
            ] {
                cases.push((
                    format!("minimal STAGING, generation_id ill-typed ({what})"),
                    minimal_txn("STAGING", mig, &[("generation_id", val.clone())]),
                ));
                cases.push((
                    format!("FULL STAGING, generation_id ill-typed ({what})"),
                    full_txn("STAGING", mig, Some(val)),
                ));
            }
            for (what, bytes) in cases {
                rows.push(tier_row(
                    format!(
                        "[{mig:?} {rel} gate {gate}] {what} => state_integrity, tree unchanged"
                    ),
                    gate,
                    rel,
                    vec![("txn-act-s2508.json", bytes)],
                    None,
                    Expect::Cause("state_integrity"),
                ));
            }
        }
    }
    run_rows(
        "test_BC_1_18_013_ADR052_v123_tier1_branch_b_absent_or_ill_typed_generation_id_is_state_integrity",
        &rows,
    );
}

/// F-S2508-L3-001, ADR-052 v1.23 Tier 1 table, Branch B row: key PRESENT with JSON
/// `null` => the null generation => DISCARD. The discard "rewrites the raw object
/// and needs no other field": a MINIMAL record (state + migration_id + null
/// generation_id) is discarded to `state=ABORTED` + `abort_reason=null_generation`
/// with every other key preserved and NONE added; gate OPEN; admitted; non-live
/// sibling records are neither modified nor deleted. (The full-record discard is
/// pinned by `test_BC_1_18_011_PC6d_reconciliation_branches_ABC_wired_on_production_path_blackbox`
/// and `test_BC_1_18_011_PC6d_branch_b_applies_with_absent_or_open_gate_blackbox`.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_branch_b_null_generation_minimal_record_discards_raw_object()
{
    let mut failures: Vec<String> = Vec::new();
    for (mig, rel, _t) in TIER_FAMILIES {
        for gate in TIER_GATES {
            for noise in [false, true] {
                let label = format!("[{mig:?} {rel} gate {gate} noise={noise}]");
                let p = Project::new();
                write_gate(&p.ms(), gate);
                let orig = minimal_txn(
                    "STAGING",
                    mig,
                    &[("generation_id", serde_json::Value::Null)],
                );
                write_raw_txn(&p, "txn-b-live.json", &orig);
                let noise_files: Vec<(&str, Vec<u8>)> = if noise {
                    vec![
                        (
                            "txn-a-done.json",
                            minimal_txn("COMPLETED", Some("future-migration"), &[]),
                        ),
                        ("txn-c-abort.json", minimal_txn("ABORTED", None, &[])),
                    ]
                } else {
                    vec![]
                };
                for (n, b) in &noise_files {
                    write_raw_txn(&p, n, b);
                }
                let target = p.abs(rel);
                let out = run(
                    &p,
                    &envelope("PreToolUse", "Write", Some("TD1"), edit_input(&target)),
                );
                if out.status.code() != Some(0) || !p.reservation("TD1").exists() {
                    failures.push(format!(
                        "{label}: Branch B must admit with a reservation; got exit {:?}: {}",
                        out.status.code(),
                        stderr_of(&out)
                    ));
                }
                if p.gate() != "OPEN" {
                    failures.push(format!("{label}: gate must be OPEN, got {}", p.gate()));
                }
                let after: serde_json::Value = match std::fs::read(p.ms().join("txn-b-live.json")) {
                    Ok(b) => serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
                    Err(e) => {
                        failures.push(format!("{label}: the txn file must be retained: {e}"));
                        continue;
                    }
                };
                let mut want: serde_json::Value = serde_json::from_slice(&orig).unwrap();
                want["state"] = serde_json::json!("ABORTED");
                want["abort_reason"] = serde_json::json!("null_generation");
                if after != want {
                    failures.push(format!(
                        "{label}: discarded record must equal the raw record + state=ABORTED + \
                         abort_reason (no key added or lost); want {want}, got {after}"
                    ));
                }
                for (n, b) in &noise_files {
                    if std::fs::read(p.ms().join(n)).ok().as_deref() != Some(b.as_slice()) {
                        failures.push(format!("{label}: non-live record {n} was modified/deleted"));
                    }
                }
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_ADR052_v123_tier1_branch_b_null_generation_minimal_record_discards_raw_object",
        failures,
    );
}

/// F-S2508-L3-001, ADR-052 v1.23 Tier 1 table, "Live block" row + Branch B row
/// ("PRESENT string => not Branch B => live block"): a shape-valid live known
/// record that no consuming branch decides on blocks with PLAIN
/// `E-MAINTENANCE-001`. Rows: STAGING with a string `generation_id`; COMMITTING
/// WITHOUT a terminal record whatever `generation_id` is (absent / null / string /
/// ill-typed: the live block reads no field). Tree byte-identical, no reservation.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_live_block_is_plain_and_reads_no_field() {
    let mut rows: Vec<Row> = Vec::new();
    for (mig, rel, _t) in TIER_FAMILIES {
        for gate in TIER_GATES {
            rows.push(tier_row(
                format!(
                    "[{mig:?} {rel} gate {gate}] minimal STAGING, generation_id string => plain block"
                ),
                gate,
                rel,
                vec![(
                    "txn-act-s2508.json",
                    minimal_txn(
                        "STAGING",
                        mig,
                        &[("generation_id", serde_json::json!("gen-1"))],
                    ),
                )],
                None,
                Expect::PlainBlock,
            ));
            let committing: [(&str, Vec<(&str, serde_json::Value)>); 4] = [
                ("generation_id ABSENT", vec![]),
                (
                    "generation_id null",
                    vec![("generation_id", serde_json::Value::Null)],
                ),
                (
                    "generation_id string",
                    vec![("generation_id", serde_json::json!("gen-1"))],
                ),
                (
                    "generation_id ill-typed (7)",
                    vec![("generation_id", serde_json::json!(7))],
                ),
            ];
            for (what, extra) in committing {
                rows.push(tier_row(
                    format!(
                        "[{mig:?} {rel} gate {gate}] minimal COMMITTING, no terminal record, \
                         {what} => plain block (never -002)"
                    ),
                    gate,
                    rel,
                    vec![("txn-act-s2508.json", minimal_txn("COMMITTING", mig, &extra))],
                    None,
                    Expect::PlainBlock,
                ));
            }
        }
    }
    run_rows(
        "test_BC_1_18_013_ADR052_v123_tier1_live_block_is_plain_and_reads_no_field",
        &rows,
    );
}

/// F-S2508-L3-001, ADR-052 v1.23 Tier 1 table, Branch C row (S-25.08 build: "none
/// ... every check reports unverified => `FailClosedMismatch`"; "`staging_with_
/// terminal_record` reads none"): a live known record WITH its own terminal record
/// is the keyed block + the exact completion-record-mismatch suffix, whatever its
/// Tier 1 fields are (a minimal STAGING with generation_id absent/null/ill-typed
/// must NOT reach Branch B, which needs the record ABSENT, and must NOT be
/// -002 because nothing is read). Tree byte-identical.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_branch_c_seam_reads_no_field_mismatch_block() {
    let mut rows: Vec<Row> = Vec::new();
    for (mig, rel, term) in TIER_FAMILIES {
        for gate in ["LOCKED", "DRAINING"] {
            for state in ["STAGING", "COMMITTING"] {
                let gens: [(&str, Vec<(&str, serde_json::Value)>); 4] = [
                    ("generation_id ABSENT", vec![]),
                    (
                        "generation_id null",
                        vec![("generation_id", serde_json::Value::Null)],
                    ),
                    (
                        "generation_id string",
                        vec![("generation_id", serde_json::json!("gen-1"))],
                    ),
                    (
                        "generation_id ill-typed (7)",
                        vec![("generation_id", serde_json::json!(7))],
                    ),
                ];
                for (what, extra) in gens {
                    rows.push(tier_row(
                        format!(
                            "[{mig:?} {rel} gate {gate}] minimal {state} + own terminal record, \
                             {what} => mismatch block (no field read)"
                        ),
                        gate,
                        rel,
                        vec![("txn-act-s2508.json", minimal_txn(state, mig, &extra))],
                        Some(term),
                        Expect::MismatchBlock,
                    ));
                }
            }
        }
    }
    run_rows(
        "test_BC_1_18_013_ADR052_v123_tier1_branch_c_seam_reads_no_field_mismatch_block",
        &rows,
    );
}

/// F-S2508-L3-001, ADR-052 v1.23 Tier 1 table, "Foreign refusal" row: a live
/// record whose `migration_id` is not known is refused PLAIN with no field beyond
/// Tier 0 read, "precedence over every other record check" -- even with poisoned
/// Tier 1 fields and even when a B2 terminal record exists.
/// (Extends the minimal-shape pins of
/// `test_BC_1_18_013_EC029_minimal_shape_foreign_txn_is_foreign_not_state_integrity_blackbox`.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_foreign_live_with_poisoned_fields_is_plain_refusal() {
    let mut rows: Vec<Row> = Vec::new();
    for rel in [BC_PATH, CYCLES_PATH] {
        for gate in TIER_GATES {
            for state in ["STAGING", "COMMITTING"] {
                let poisons: [(&str, Vec<(&str, serde_json::Value)>); 5] = [
                    ("generation_id ABSENT", vec![]),
                    (
                        "generation_id null",
                        vec![("generation_id", serde_json::Value::Null)],
                    ),
                    (
                        "generation_id 7",
                        vec![("generation_id", serde_json::json!(7))],
                    ),
                    (
                        "generation_id array",
                        vec![("generation_id", serde_json::json!([1]))],
                    ),
                    (
                        "pending_canonical_moves = 'x'",
                        vec![("pending_canonical_moves", serde_json::json!("x"))],
                    ),
                ];
                for (what, extra) in poisons {
                    for with_b2_terminal in [false, true] {
                        rows.push(tier_row(
                            format!(
                                "[{rel} gate {gate}] FOREIGN live {state}, {what}, b2_terminal=\
                                 {with_b2_terminal} => plain refusal"
                            ),
                            gate,
                            rel,
                            vec![(
                                "txn-act-s2508.json",
                                minimal_txn(state, Some("future-migration"), &extra),
                            )],
                            with_b2_terminal.then_some("completed.json"),
                            Expect::PlainBlock,
                        ));
                    }
                }
            }
        }
    }
    run_rows(
        "test_BC_1_18_013_ADR052_v123_tier1_foreign_live_with_poisoned_fields_is_plain_refusal",
        &rows,
    );
}

/// F-S2508-L3-001, ADR-052 v1.23: "Non-live records with a valid Tier 0 shape are
/// admitted in all cases ... the 'more than one live txn' `state_integrity` counts
/// live records only, foreign included." One live + N non-live (COMPLETED /
/// ABORTED, minimal, known / foreign / B2, any filename order) is ONE live txn:
/// the live record's own verdict (plain block / mismatch block), never "more than
/// one live txn". Plus the control: two live records (one foreign) ARE
/// `state_integrity` (full records, so only the live-count rule can fail them).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_ADR052_v123_tier1_one_live_plus_non_live_counts_as_one_live() {
    let mut rows: Vec<Row> = Vec::new();
    for (mig, rel, term) in TIER_FAMILIES {
        for gate in ["LOCKED", "DRAINING"] {
            // the live record sorts FIRST, MIDDLE, LAST among the filenames
            let orders: [(&str, [&'static str; 4]); 3] = [
                (
                    "live first",
                    ["txn-a-live.json", "txn-b.json", "txn-c.json", "txn-d.json"],
                ),
                (
                    "live middle",
                    ["txn-b.json", "txn-c-live.json", "txn-d.json", "txn-e.json"],
                ),
                (
                    "live last",
                    ["txn-b.json", "txn-c.json", "txn-d.json", "txn-z-live.json"],
                ),
            ];
            for (pos, names) in orders {
                let live_idx = names.iter().position(|n| n.contains("live")).unwrap();
                let others: Vec<&'static str> = names
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != live_idx)
                    .map(|(_, n)| *n)
                    .collect();
                for (what, live_bytes, terminal, expect) in [
                    (
                        "STAGING gen string, no terminal record",
                        minimal_txn(
                            "STAGING",
                            mig,
                            &[("generation_id", serde_json::json!("gen-1"))],
                        ),
                        None,
                        Expect::PlainBlock,
                    ),
                    (
                        "COMMITTING, no terminal record",
                        minimal_txn("COMMITTING", mig, &[]),
                        None,
                        Expect::PlainBlock,
                    ),
                    (
                        "COMMITTING + own terminal record",
                        minimal_txn("COMMITTING", mig, &[]),
                        Some(term),
                        Expect::MismatchBlock,
                    ),
                ] {
                    let mut files: Vec<(&'static str, Vec<u8>)> = vec![
                        (
                            others[0],
                            minimal_txn("COMPLETED", Some("future-migration"), &[]),
                        ),
                        (others[1], minimal_txn("ABORTED", None, &[])),
                        (
                            others[2],
                            minimal_txn("COMPLETED", Some("migrate-bc-index"), &[]),
                        ),
                    ];
                    files.push((names[live_idx], live_bytes));
                    rows.push(tier_row(
                        format!(
                            "[{mig:?} {rel} gate {gate}] {pos}: ONE live ({what}) + 3 non-live => \
                             that live record's verdict (not 'more than one live txn')"
                        ),
                        gate,
                        rel,
                        files,
                        terminal,
                        expect,
                    ));
                }
            }
        }
        // Control: >1 live (full records; one foreign) => state_integrity.
        rows.push(tier_row(
            format!("[{mig:?} {rel}] control: two LIVE records (one foreign) => state_integrity"),
            "LOCKED",
            rel,
            vec![
                (
                    "txn-a.json",
                    full_txn("STAGING", mig, Some(serde_json::json!("gen-1"))),
                ),
                (
                    "txn-b.json",
                    full_txn(
                        "COMMITTING",
                        Some("future-migration"),
                        Some(serde_json::json!("gen-2")),
                    ),
                ),
            ],
            None,
            Expect::Cause("state_integrity"),
        ));
    }
    run_rows(
        "test_BC_1_18_013_ADR052_v123_tier1_one_live_plus_non_live_counts_as_one_live",
        &rows,
    );
}

/// BC-1.18.013 v1.12 EC-045 CONTROL (S-25.08 v1.6 AC-029 gap), ADR-052 v1.23
/// "Txn-record interpretation -- tiers", Branch B row: the `generation_id` rule
/// applies ONLY when the migration is known, the txn is STAGING, the lock is
/// ACQUIRED and the own terminal record is ABSENT. With `exclusive.lock` HELD by
/// a live coordinator Branch B does not execute and NO field is read, so a
/// minimal (or full-minus-key) known STAGING record with an ABSENT or ill-typed
/// `generation_id` gives the PLAIN `E-MAINTENANCE-001` live-coordinator block --
/// never `E-MAINTENANCE-002 (state_integrity)` -- with the txn, gate and the
/// whole migration-state tree byte-identical and no reservation left. Covers both
/// migration families, the absent-`migration_id` family, and every gate state.
/// The free-lock twin is
/// `test_BC_1_18_013_ADR052_v123_tier1_branch_b_absent_or_ill_typed_generation_id_is_state_integrity`.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC045_lock_held_minimal_staging_without_generation_id_is_plain_live_coordinator_block()
 {
    let mut rows: Vec<Row> = Vec::new();
    for (mig, rel, _t) in TIER_FAMILIES {
        for gate in TIER_GATES {
            let mut cases: Vec<(String, Vec<u8>)> = vec![
                (
                    "minimal STAGING, generation_id ABSENT".to_string(),
                    minimal_txn("STAGING", mig, &[]),
                ),
                (
                    "FULL STAGING with the generation_id KEY REMOVED".to_string(),
                    full_txn("STAGING", mig, None),
                ),
            ];
            for (what, val) in [
                ("number 7", serde_json::json!(7)),
                ("boolean true", serde_json::json!(true)),
                ("array", serde_json::json!(["x"])),
                ("object", serde_json::json!({"id": "gen-1"})),
            ] {
                cases.push((
                    format!("minimal STAGING, generation_id ill-typed ({what})"),
                    minimal_txn("STAGING", mig, &[("generation_id", val.clone())]),
                ));
                cases.push((
                    format!("FULL STAGING, generation_id ill-typed ({what})"),
                    full_txn("STAGING", mig, Some(val)),
                ));
            }
            for (what, bytes) in cases {
                let label = format!(
                    "[{mig:?} {rel} gate {gate}, exclusive.lock HELD] {what} => plain \
                     E-MAINTENANCE-001 live-coordinator block, no field read, tree unchanged"
                );
                rows.push(Row {
                    label: Box::leak(label.into_boxed_str()),
                    rel,
                    needs_non_root: false,
                    expect: Expect::PlainBlock,
                    setup: Box::new(move |p| {
                        write_gate(&p.ms(), gate);
                        write_raw_txn(p, "txn-act-s2508.json", &bytes);
                        Fixture {
                            _modes: Vec::new(),
                            _lock: Some(p.hold_lock()),
                        }
                    }),
                });
            }
        }
    }
    run_rows(
        "test_BC_1_18_013_EC045_lock_held_minimal_staging_without_generation_id_is_plain_live_coordinator_block",
        &rows,
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L4-001 -- release leg, unstatable `migration-state` (black-box half)
// ---------------------------------------------------------------------------

/// F-S2508-L4-001 (externally visible half only; the warn-vs-silent distinction
/// is not observable in S-25.08 -- see `bc_1_18_013_release_stat_error_test.rs`
/// for the error-path classification pin against the factored helper). A
/// self-referential `migration-state` symlink makes the release's stat fail
/// ELOOP (every uid, root-safe). BC-1.18.013 EC-021 / AC-011: release is
/// best-effort, NEVER a verdict => exit 0, no `E-MAINTENANCE`, nothing deleted
/// or created. Green today; guards the fix against turning the stat error into a
/// verdict or a destructive action.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_001_release_with_eloop_migration_state_exits_0_deletes_nothing_blackbox() {
    for ev in ["PostToolUse", "PostToolUseFailure"] {
        let p = bare_project();
        let logs = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(p.root().join(".factory")).unwrap();
        std::os::unix::fs::symlink("migration-state", p.root().join(".factory/migration-state"))
            .unwrap();
        // A decoy reservation elsewhere in the tree that a wrongly-targeted
        // release must never touch.
        let decoy = p.root().join(".factory/decoy/reservations");
        std::fs::create_dir_all(&decoy).unwrap();
        std::fs::write(decoy.join("TE1.reservation"), b"{}").unwrap();
        let before = raw_tree(p.root());
        let target = factory_target(p.root());
        let out = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope(ev, "Edit", Some("TE1"), edit_input(&target)),
        );
        assert_eq!(
            out.status.code(),
            Some(0),
            "[{ev}] release is best-effort: a stat error must never be a verdict; stderr: {}",
            stderr_of(&out)
        );
        assert!(
            !stderr_of(&out).contains("E-MAINTENANCE"),
            "[{ev}] no E-MAINTENANCE code on the release leg: {}",
            stderr_of(&out)
        );
        assert_eq!(
            raw_tree(p.root()),
            before,
            "[{ev}] the release must delete/create nothing"
        );
    }
}

// ---------------------------------------------------------------------------
// F-S2508-L4-002 -- ADR-052 v1.24 "E-MAINTENANCE-002 <cause> -- total decision rule"
// (regression pin of the ruled order: input guards -> .factory stat -> target
// classification -> tool_use_id check, the last only for an in-scope protected
// write). The code already complies; these are green on arrival.
// ---------------------------------------------------------------------------

/// (1) unstatable `.factory` (ELOOP) + invalid id on a protected path: the
/// `.factory` stat precedes the tool_use_id check => `io`, NOT `invalid_tool_use_id`.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_002_unstatable_factory_beats_invalid_tool_use_id_io_blackbox() {
    let p = bare_project();
    let logs = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(".factory", p.root().join(".factory")).unwrap();
    let before = raw_tree(p.root());
    let target = factory_target(p.root());
    let out = run_at(
        &p,
        p.root(),
        logs.path(),
        &envelope("PreToolUse", "Write", Some("../x"), edit_input(&target)),
    );
    let err = stderr_of(&out);
    assert_eq!(out.status.code(), Some(2), "stderr: {err}");
    assert!(
        err.contains("E-MAINTENANCE-002: writer-admission check failed (io)"),
        "unstatable .factory must win over the id check => (io); stderr: {err}"
    );
    assert!(
        !err.contains("invalid_tool_use_id"),
        "the id check must not run before the .factory stat; stderr: {err}"
    );
    assert_eq!(raw_tree(p.root()), before, "tree must be unchanged");
}

/// (2) statable `.factory` + OUT-of-scope target + invalid id: the id is never
/// examined => admitted, exit 0, no reservation, nothing created.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_002_out_of_scope_target_ignores_invalid_tool_use_id_blackbox() {
    let p = Project::new_bare();
    let target = p.abs("src/lib.rs");
    let before = raw_tree(p.root());
    let logs = tempfile::tempdir().unwrap();
    let out = run_at(
        &p,
        p.root(),
        logs.path(),
        &envelope("PreToolUse", "Write", Some("../x"), edit_input(&target)),
    );
    let err = stderr_of(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "an out-of-scope write must be admitted regardless of the id; stderr: {err}"
    );
    assert!(!err.contains("E-MAINTENANCE"), "stderr: {err}");
    assert!(!p.ms().exists(), "no migration-state / reservation created");
    assert_eq!(raw_tree(p.root()), before, "tree must be unchanged");
}

/// (3) statable `.factory` + IN-scope protected target + invalid id =>
/// `E-MAINTENANCE-002 (invalid_tool_use_id)`, nothing created.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_002_in_scope_target_invalid_tool_use_id_fails_closed_blackbox() {
    let p = Project::new_bare();
    let target = p.abs(BC_PATH);
    let before = raw_tree(p.root());
    let logs = tempfile::tempdir().unwrap();
    let out = run_at(
        &p,
        p.root(),
        logs.path(),
        &envelope("PreToolUse", "Write", Some("../x"), edit_input(&target)),
    );
    let err = stderr_of(&out);
    assert_eq!(out.status.code(), Some(2), "stderr: {err}");
    assert!(
        err.contains("E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)"),
        "stderr: {err}"
    );
    assert!(!p.ms().exists(), "no migration-state created");
    assert_eq!(raw_tree(p.root()), before, "tree must be unchanged");
}
