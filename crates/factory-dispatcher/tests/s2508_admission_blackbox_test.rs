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
        if out.status.code() != Some(2) || !err.contains(&want) {
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
