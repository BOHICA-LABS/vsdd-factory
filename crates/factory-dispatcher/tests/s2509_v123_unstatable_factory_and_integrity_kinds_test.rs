// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 -- the S-25.09 share of the ADR-052 v1.23 rulings, real spawned binary.
//!
//! Authority: ADR-052 v1.23 section 5a "Factory-root lookup mapping", section
//! "Error Code Semantics" tiers, section Downstream v1.23 (S-25.09 list);
//! BC-1.18.013 v1.12 EC-041 / EC-042 / EC-043 / EC-045 and Postcondition 10(c);
//! BC-1.18.011 v1.20 parallel ECs; BC-3.08.001 v1.36 Events 11-13.
//!
//! | Delta | Test |
//! |-------|------|
//! | 1 coordinator `Io` (EC-043) | `..._EC043_coordinator_unstatable_factory_exit_2_io_not_root_not_found_blackbox` |
//! | 1 absent controls (EC-035) | `..._EC035_coordinator_absent_dangling_and_enotdir_stay_factory_root_not_found_blackbox` |
//! | 2 admission event (EC-041) | `..._EC041_unstatable_factory_admission_writes_one_admission_failed_io_blackbox` |
//! | 3 release advisory (EC-042, PC 10(c)) | `..._EC042_unstatable_factory_release_writes_one_reservation_release_failed_advisory_blackbox` |
//! | 4 kinds, Tier 0 | `..._EC032_tier0_failures_event_kind_blackbox` |
//! | 4 kinds, Tier 1 (EC-045) | `..._EC045_staging_without_generation_id_event_kind_txn_record_malformed_blackbox` |
//! | 5 sixth kind `txn_record_newer_schema`, Branch B version gate (ADR-052 v1.23 item 10(b)-(d), third binary-leg extension) | `..._EC049_branch_b_null_generation_version_gate_event_kind_..._blackbox` |
//! | 5 control: Branch B discard under an absent / 1 `schema_version` preserves unknown keys | `..._EC049_branch_b_discard_with_absent_or_1_schema_version_preserves_unknown_key_blackbox` |
//!
//! Already covered elsewhere (NOT duplicated here), all in
//! `s2508_admission_blackbox_test.rs`:
//! * verdict-level (exit 2 + `E-MAINTENANCE-002 (io)`, tree unchanged, no
//!   reservation) for ELOOP / EACCES admission, the input-guard precedence, and
//!   the release leg's exit-0 / reservation-left behaviour:
//!   `..._ADR052_v123_unstatable_factory_{eloop,eacces}_fails_closed_io_blackbox`,
//!   `..._input_guards_precede_stat_blackbox`,
//!   `..._release_leg_no_verdict_reservation_left_blackbox`;
//! * the absent closed set on the admission/release legs (verdicts only, no
//!   event assertion): `..._absent_factory_root_closed_set_..._blackbox`;
//! * EC-035 coordinator rows `no .factory` and `.factory is a regular file`:
//!   `test_BC_1_18_013_EC035_coordinator_factory_root_not_found_exit_2_nothing_created_blackbox`;
//! * event kinds for unparseable JSON (`txn_record_malformed`) and a numeric
//!   `migration_id` (`txn_migration_id_not_string`):
//!   `test_BC_1_18_013_EC038_admission_failed_event_per_cause_blackbox`;
//! * EC-045 CAUSE only (not the event `kind`):
//!   `..._tier1_branch_b_absent_or_ill_typed_generation_id_is_state_integrity`.
//!
//! `backfill-append-logs` has no CLI subcommand in this build (only
//! `migrate-bc-index` is routed by `main.rs`), so the coordinator rows exercise
//! `migrate-bc-index` only.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use factory_dispatcher::shard_manager::{MigrationLockGuard, try_acquire_migration_lock};

const CYCLES_PATH: &str = ".factory/cycles/c1/a.md";
const BC_PATH: &str = ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md";
const SECRET_ID: &str = "toolu_S2509SECRET";

// ---------------------------------------------------------------------------
// Harness (self-contained; mirrors tests/s2508_admission_blackbox_test.rs)
// ---------------------------------------------------------------------------

struct Project {
    dir: tempfile::TempDir,
    plugin_root: tempfile::TempDir,
}

impl Project {
    /// `bare`: nothing under the root. Otherwise `.factory/migration-state/
    /// {reservations/}` exists with gate OPEN.
    fn make(bare: bool) -> Self {
        let dir = tempfile::tempdir().expect("project tempdir");
        let plugin_root = tempfile::tempdir().expect("plugin_root tempdir");
        std::fs::write(
            plugin_root.path().join("hooks-registry.toml"),
            "schema_version = 2\n",
        )
        .expect("write empty registry");
        let p = Project { dir, plugin_root };
        if !bare {
            std::fs::create_dir_all(p.ms().join("reservations")).unwrap();
            std::fs::write(p.ms().join("exclusive.lock"), b"").unwrap();
            write_gate(&p.ms(), "OPEN");
        }
        p
    }
    fn new() -> Self {
        Self::make(false)
    }
    fn bare() -> Self {
        Self::make(true)
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
    fn hold_lock(&self) -> MigrationLockGuard {
        try_acquire_migration_lock(&self.ms().join("exclusive.lock"))
            .expect("lock open")
            .expect("lock must be free in fixture setup")
    }
}

fn write_gate(ms: &Path, state: &str) {
    std::fs::create_dir_all(ms).unwrap();
    std::fs::write(ms.join("gate-state.json"), format!("\"{state}\"")).unwrap();
}

fn write_raw_txn(p: &Project, bytes: &[u8]) {
    std::fs::write(p.ms().join("txn-act-s2509.json"), bytes).unwrap();
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
        "session_id": "sess-s2509",
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

/// Run the dispatcher with an explicit project dir AND a log dir OUTSIDE the
/// project (a chmod-000 root must not block the log, and the project tree is then
/// exactly what the dispatcher left). Stdin is written then closed.
fn run_at(p: &Project, proj_dir: &Path, log_dir: &Path, payload: &str) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", p.plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", proj_dir)
        .env("VSDD_LOG_DIR", log_dir)
        .env_remove("VSDD_TEST_ADMISSION_SEAM_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    finish(child, Duration::from_secs(30))
}

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// Every parsed `dispatcher-internal-*.jsonl` line under `log_dir`.
fn read_events(log_dir: &Path) -> Vec<(serde_json::Value, String)> {
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
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push((v, line.to_string()));
            }
        }
    }
    out
}

fn of_type<'a>(
    all: &'a [(serde_json::Value, String)],
    ty: &str,
) -> Vec<&'a (serde_json::Value, String)> {
    all.iter().filter(|(v, _)| v["type"] == ty).collect()
}

fn migration_event_types(all: &[(serde_json::Value, String)]) -> Vec<String> {
    all.iter()
        .filter_map(|(v, _)| v["type"].as_str())
        .filter(|t| t.starts_with("migration."))
        .map(str::to_string)
        .collect()
}

/// lstat-based snapshot of a whole tree (symlinks recorded, never followed;
/// unreadable entries tolerated).
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

fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

#[cfg(unix)]
mod fault {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    pub struct RestoreMode {
        path: PathBuf,
        mode: u32,
    }
    impl Drop for RestoreMode {
        fn drop(&mut self) {
            let _ =
                std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(self.mode));
        }
    }
    pub fn chmod000(path: &Path, restore_to: u32) -> RestoreMode {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
        RestoreMode {
            path: path.to_path_buf(),
            mode: restore_to,
        }
    }
    pub fn is_root() -> bool {
        let uid = Command::new("id")
            .arg("-u")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        String::from_utf8_lossy(&uid.stdout).trim() == "0"
    }
}

/// How to make `<root>/.factory` unstatable.
#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
enum Fault {
    /// `.factory` -> itself (ELOOP; every uid, root-safe)
    Eloop,
    /// project root `chmod 000` (EACCES; skipped as root)
    Eacces,
}

#[cfg(unix)]
impl Fault {
    fn skip_as_root(self) -> bool {
        matches!(self, Fault::Eacces) && fault::is_root()
    }
}

// ===========================================================================
// Delta 1 -- coordinator (BC-1.18.013 EC-043, EC-035; ADR-052 v1.23 5a)
// ===========================================================================

fn run_coordinator(proj_dir: &Path, cwd: &Path) -> Output {
    let mut cmd = Command::new(binary_path());
    cmd.arg("migrate-bc-index")
        .current_dir(cwd)
        .env("CLAUDE_PROJECT_DIR", proj_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    finish(cmd.spawn().unwrap(), Duration::from_secs(30))
}

/// BC-1.18.013 v1.12 EC-043 (+ canonical vector): `migrate-bc-index` under a
/// project root whose `.factory` `stat` is UNSTATABLE (ELOOP via self-symlink;
/// EACCES via `chmod 000` root, skipped as root) exits 2 with the EXISTING
/// `BcIndexMigrationError::Io { path: <root>/.factory, source }` Display --
/// `migrate-bc-index: BC-INDEX migration: I/O error at <root>/.factory: <os error>`
/// -- NOT `FACTORY_ROOT_NOT_FOUND`; exactly ONE stderr line, empty stdout; the
/// tree is byte-identical (no `.factory`, `migration-state/`, `exclusive.lock`).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC043_coordinator_unstatable_factory_exit_2_io_not_root_not_found_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    for fault in [Fault::Eloop, Fault::Eacces] {
        if fault.skip_as_root() {
            eprintln!("SKIP [EACCES]: running as root");
            continue;
        }
        let p = Project::bare();
        let other = tempfile::tempdir().unwrap();
        let root = p.root().canonicalize().unwrap();
        let _restore = match fault {
            Fault::Eloop => {
                std::os::unix::fs::symlink(".factory", root.join(".factory")).unwrap();
                None
            }
            Fault::Eacces => Some(fault::chmod000(&root, 0o755)),
        };
        // The exact OS error the coordinator's own `stat` hits.
        let os_err = std::fs::metadata(root.join(".factory")).unwrap_err();
        let want = format!(
            "migrate-bc-index: BC-INDEX migration: I/O error at {}: {os_err}",
            root.join(".factory").display()
        );
        // The root may be unreadable for the snapshot: snapshot the sibling cwd and
        // (after restoring) the root.
        let other_before = raw_tree(other.path());
        let out = run_coordinator(&root, other.path());
        drop(_restore);
        let err = stderr_of(&out);
        let lines: Vec<&str> = err.lines().collect();
        if out.status.code() != Some(2)
            || lines.len() != 1
            || lines[0] != want
            || !out.stdout.is_empty()
        {
            failures.push(format!(
                "[{fault:?}] expected exit 2, exactly ONE stderr line `{want}`, empty stdout; got \
                 exit {:?}, stderr lines {lines:?}, stdout {} bytes",
                out.status.code(),
                out.stdout.len()
            ));
        }
        if err.contains("FACTORY_ROOT_NOT_FOUND") {
            failures.push(format!(
                "[{fault:?}] an unstatable `.factory` must NOT be reported as FACTORY_ROOT_NOT_FOUND: {err}"
            ));
        }
        // Nothing created anywhere.
        let after = raw_tree(&root);
        let expect_after: BTreeMap<String, String> = match fault {
            Fault::Eloop => {
                BTreeMap::from([(".factory".to_string(), "symlink->.factory".to_string())])
            }
            Fault::Eacces => BTreeMap::new(),
        };
        if after != expect_after || raw_tree(other.path()) != other_before {
            failures.push(format!(
                "[{fault:?}] the coordinator must create/mutate NOTHING (no .factory, \
                 migration-state/, exclusive.lock); root tree after: {after:?}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC043_coordinator_unstatable_factory_exit_2_io_not_root_not_found_blackbox",
        failures,
    );
}

/// BC-1.18.013 v1.12 EC-035 (reworded) / EC-043 control rows. The ABSENT closed set
/// for the coordinator now also names a DANGLING `.factory` symlink (ENOENT) and a
/// project root that is itself a regular file (ENOTDIR): each is still exit 2
/// `FACTORY_ROOT_NOT_FOUND` with the EC-035 one-line text, nothing created. (The
/// existing EC-035 test covers only `no .factory` and `.factory is a regular file`.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC035_coordinator_absent_dangling_and_enotdir_stay_factory_root_not_found_blackbox()
 {
    let mut failures: Vec<String> = Vec::new();
    type Setup = fn(&Path) -> PathBuf;
    let rows: [(&str, Setup); 2] = [
        ("`.factory` is a DANGLING symlink (ENOENT)", |root| {
            std::os::unix::fs::symlink(root.join("no-such-target"), root.join(".factory")).unwrap();
            root.to_path_buf()
        }),
        (
            "the project root itself is a regular file (ENOTDIR)",
            |root| {
                let f = root.join("afile");
                std::fs::write(&f, b"x").unwrap();
                f
            },
        ),
    ];
    for (label, setup) in rows {
        let p = Project::bare();
        let other = tempfile::tempdir().unwrap();
        let root = p.root().canonicalize().unwrap();
        let proj = setup(&root);
        let resolved = proj.canonicalize().unwrap_or_else(|_| proj.clone());
        let before = (raw_tree(&root), raw_tree(other.path()));
        let out = run_coordinator(&proj, other.path());
        let err = stderr_of(&out);
        let want = format!(
            "migrate-bc-index: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root {} \
             (resolved from CLAUDE_PROJECT_DIR)",
            resolved.display()
        );
        let lines: Vec<&str> = err.lines().collect();
        if out.status.code() != Some(2)
            || lines.len() != 1
            || lines[0] != want
            || !out.stdout.is_empty()
        {
            failures.push(format!(
                "[{label}] expected exit 2, ONE stderr line `{want}`, empty stdout; got exit {:?}, \
                 stderr {lines:?}",
                out.status.code()
            ));
        }
        if (raw_tree(&root), raw_tree(other.path())) != before {
            failures.push(format!("[{label}] nothing may be created or mutated"));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC035_coordinator_absent_dangling_and_enotdir_stay_factory_root_not_found_blackbox",
        failures,
    );
}

// ===========================================================================
// Delta 2 -- admission event (EC-041)
// ===========================================================================

/// BC-1.18.013 v1.12 EC-041 + canonical vector; BC-3.08.001 Event 12; ADR-052 v1.23
/// 5a mapping. PreToolUse `Write` (valid `file_path`, valid `tool_use_id`) while the
/// `.factory` `stat` is UNSTATABLE (ELOOP, EACCES), for BOTH a `.factory` path and a
/// non-`.factory` path (`README.md`: the root's identity is unknown, so no path is
/// classified): exit 2 `E-MAINTENANCE-002 (io)`, `blocking_plugins=migration-admission`,
/// EXACTLY ONE `migration.admission_failed` (`cause=io`, `kind` PRESENT and null,
/// `detail` carries the sanitized `.factory` path, the `ErrorKind` and the OS message,
/// never the `tool_use_id`), ZERO `_blocked`, ZERO `_advisory`, no reservation,
/// nothing created. Controls: the ABSENT variants (regular-file `.factory`, dangling
/// symlink, root-is-a-file ENOTDIR, no `.factory`) are admitted (exit 0) with NO
/// migration event and a byte-identical tree.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC041_unstatable_factory_admission_writes_one_admission_failed_io_blackbox() {
    let mut failures: Vec<String> = Vec::new();

    for fault in [Fault::Eloop, Fault::Eacces] {
        if fault.skip_as_root() {
            eprintln!("SKIP [EACCES]: running as root");
            continue;
        }
        for rel in [CYCLES_PATH, "README.md"] {
            let p = if matches!(fault, Fault::Eacces) {
                Project::new() // a real `.factory/migration-state` exists under the root
            } else {
                Project::bare()
            };
            let logs = tempfile::tempdir().unwrap();
            let root = p.root().to_path_buf();
            let target = root.join(rel); // not created: the parent may be unreachable
            let restore = if matches!(fault, Fault::Eloop) {
                std::os::unix::fs::symlink(".factory", root.join(".factory")).unwrap();
                None
            } else {
                Some(fault::chmod000(&root, 0o755))
            };
            let os_err = std::fs::metadata(root.join(".factory")).unwrap_err();
            let kind_dbg = format!("{:?}", os_err.kind());
            let out = run_at(
                &p,
                &root,
                logs.path(),
                &envelope("PreToolUse", "Write", Some(SECRET_ID), edit_input(&target)),
            );
            drop(restore);
            let label = format!("{fault:?} / {rel}");
            let err = stderr_of(&out);
            if out.status.code() != Some(2)
                || !err.contains("E-MAINTENANCE-002: writer-admission check failed (io)")
                || err.contains("E-MAINTENANCE-001")
                || err.contains("FACTORY_ROOT_NOT_FOUND")
            {
                failures.push(format!(
                    "[{label}] expected exit 2 + E-MAINTENANCE-002 (io); got {:?}: {err}",
                    out.status.code()
                ));
            }
            if !err.contains("blocking_plugins=migration-admission") {
                failures.push(format!(
                    "[{label}] stderr summary must name `blocking_plugins=migration-admission`: {err}"
                ));
            }
            let all = read_events(logs.path());
            let failed = of_type(&all, "migration.admission_failed");
            if failed.len() != 1 {
                failures.push(format!(
                    "[{label}] exactly ONE migration.admission_failed expected, found {} \
                     (migration events: {:?})",
                    failed.len(),
                    migration_event_types(&all)
                ));
            } else {
                let (v, line) = failed[0];
                if v["cause"] != "io" {
                    failures.push(format!("[{label}] cause must be `io`: {v}"));
                }
                if !v.get("kind").is_some_and(serde_json::Value::is_null) {
                    failures.push(format!("[{label}] kind must be PRESENT and null: {v}"));
                }
                let detail = v["detail"].as_str().unwrap_or("");
                let root_leaf = root.file_name().unwrap().to_string_lossy().to_string();
                let msg_core = os_err.to_string();
                if !detail.contains(".factory")
                    || !detail.contains(&root_leaf)
                    || !detail.contains(&kind_dbg)
                    || !detail.contains(&msg_core)
                {
                    failures.push(format!(
                        "[{label}] detail must carry the sanitized `.factory` path (under \
                         `{root_leaf}`), the ErrorKind `{kind_dbg}` and the OS message \
                         `{msg_core}`; got {detail:?}"
                    ));
                }
                if line.contains(SECRET_ID) {
                    failures.push(format!("[{label}] raw tool_use_id leaked: {line}"));
                }
                if v.get("plugin_name").is_some() {
                    failures.push(format!("[{label}] plugin_name MUST NOT appear"));
                }
            }
            for ty in [
                "migration.admission_blocked",
                "migration.admission_advisory",
            ] {
                if !of_type(&all, ty).is_empty() {
                    failures.push(format!(
                        "[{label}] no `{ty}` for an unstatable-root io failure"
                    ));
                }
            }
            if matches!(fault, Fault::Eacces) {
                // Only checkable with the root mode restored (done above).
                if p.reservation(SECRET_ID).exists() {
                    failures.push(format!("[{label}] a reservation was left"));
                }
            }
            let tree = raw_tree(&root);
            let allowed: BTreeMap<String, String> = match fault {
                Fault::Eloop => {
                    BTreeMap::from([(".factory".to_string(), "symlink->.factory".to_string())])
                }
                Fault::Eacces => tree.clone(),
            };
            if tree != allowed || (matches!(fault, Fault::Eloop) && tree.len() != 1) {
                failures.push(format!("[{label}] the project tree changed: {tree:?}"));
            }
        }
    }

    // Controls: ABSENT => admitted, no migration event, tree unchanged.
    type Setup = fn(&Path) -> PathBuf;
    let controls: [(&str, Setup); 4] = [
        ("`.factory` regular file", |root| {
            std::fs::write(root.join(".factory"), b"not a directory").unwrap();
            root.to_path_buf()
        }),
        ("`.factory` dangling symlink", |root| {
            std::os::unix::fs::symlink(root.join("nope"), root.join(".factory")).unwrap();
            root.to_path_buf()
        }),
        ("project root is a regular file (ENOTDIR)", |root| {
            std::fs::write(root.join("afile"), b"x").unwrap();
            root.join("afile")
        }),
        ("no `.factory`", |root| root.to_path_buf()),
    ];
    for (label, setup) in controls {
        for rel in [CYCLES_PATH, "README.md"] {
            let p = Project::bare();
            let logs = tempfile::tempdir().unwrap();
            let proj = setup(p.root());
            let before = raw_tree(p.root());
            let out = run_at(
                &p,
                &proj,
                logs.path(),
                &envelope(
                    "PreToolUse",
                    "Write",
                    Some(SECRET_ID),
                    edit_input(&proj.join(rel)),
                ),
            );
            let all = read_events(logs.path());
            if out.status.code() != Some(0) || !migration_event_types(&all).is_empty() {
                failures.push(format!(
                    "[control {label} / {rel}] must be admitted with NO migration event; got exit \
                     {:?}, events {:?}, stderr {}",
                    out.status.code(),
                    migration_event_types(&all),
                    stderr_of(&out)
                ));
            }
            if raw_tree(p.root()) != before {
                failures.push(format!("[control {label} / {rel}] the tree changed"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC041_unstatable_factory_admission_writes_one_admission_failed_io_blackbox",
        failures,
    );
}

// ===========================================================================
// Delta 3 -- release advisory (EC-042, Postcondition 10(c))
// ===========================================================================

/// BC-1.18.013 v1.12 EC-042 + Postcondition 10(c) + canonical vector; BC-3.08.001
/// Event 13 `reason` domain (`reservation_release_failed`). A reservation is
/// admitted FIRST, then the `.factory` `stat` is made unstatable (ELOOP: real dir
/// renamed aside + self-link; EACCES: project root `chmod 000`); a real
/// `PostToolUse` and a real `PostToolUseFailure` envelope each yield exit 0 (never
/// a verdict), EXACTLY ONE `migration.admission_advisory`
/// (`reason=reservation_release_failed`, `detail` = `.factory` path + ErrorKind +
/// message, no raw tool_use_id), no `_blocked` / `_failed`, nothing created or
/// deleted, and the reservation left UNCHANGED for the drain-start TTL GC.
/// Controls (ABSENT variants, no reservation): exit 0, NO migration event.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC042_unstatable_factory_release_writes_one_reservation_release_failed_advisory_blackbox()
 {
    let mut failures: Vec<String> = Vec::new();
    for fault in [Fault::Eloop, Fault::Eacces] {
        if fault.skip_as_root() {
            eprintln!("SKIP [EACCES]: running as root");
            continue;
        }
        for ev in ["PostToolUse", "PostToolUseFailure"] {
            let label = format!("{fault:?} / {ev}");
            let p = Project::new();
            let pre_logs = tempfile::tempdir().unwrap();
            let logs = tempfile::tempdir().unwrap();
            let root = p.root().to_path_buf();
            let target = p.abs(CYCLES_PATH);
            let pre = run_at(
                &p,
                &root,
                pre_logs.path(),
                &envelope("PreToolUse", "Write", Some(SECRET_ID), edit_input(&target)),
            );
            if pre.status.code() != Some(0) || !p.reservation(SECRET_ID).exists() {
                failures.push(format!("[{label}] precondition: admit + reserve failed"));
                continue;
            }
            let res_path = match fault {
                Fault::Eloop => root.join(format!(
                    ".factory-real/migration-state/reservations/{SECRET_ID}.reservation"
                )),
                Fault::Eacces => p.reservation(SECRET_ID),
            };
            let res_before = std::fs::read(p.reservation(SECRET_ID)).unwrap();
            let restore;
            let tree_before;
            if matches!(fault, Fault::Eloop) {
                std::fs::rename(root.join(".factory"), root.join(".factory-real")).unwrap();
                std::os::unix::fs::symlink(".factory", root.join(".factory")).unwrap();
                restore = None;
                tree_before = raw_tree(&root);
            } else {
                tree_before = raw_tree(&root); // readable now; the fault is injected next
                restore = Some(fault::chmod000(&root, 0o755));
            }
            let os_err = std::fs::metadata(root.join(".factory")).unwrap_err();
            let kind_dbg = format!("{:?}", os_err.kind());
            let out = run_at(
                &p,
                &root,
                logs.path(),
                &envelope(ev, "Write", Some(SECRET_ID), edit_input(&target)),
            );
            drop(restore);
            let err = stderr_of(&out);
            if out.status.code() != Some(0) || err.contains("E-MAINTENANCE") {
                failures.push(format!(
                    "[{label}] release must yield NO verdict (exit 0, no E-MAINTENANCE-*); got \
                     {:?}: {err}",
                    out.status.code()
                ));
            }
            let all = read_events(logs.path());
            let adv = of_type(&all, "migration.admission_advisory");
            if adv.len() != 1 {
                failures.push(format!(
                    "[{label}] exactly ONE migration.admission_advisory expected, found {} \
                     (migration events: {:?})",
                    adv.len(),
                    migration_event_types(&all)
                ));
            } else {
                let (v, line) = adv[0];
                if v["reason"] != "reservation_release_failed" {
                    failures.push(format!(
                        "[{label}] reason must be `reservation_release_failed`: {v}"
                    ));
                }
                let detail = v["detail"].as_str().unwrap_or("");
                if !detail.contains(".factory")
                    || !detail.contains(&kind_dbg)
                    || !detail.contains(&os_err.to_string())
                {
                    failures.push(format!(
                        "[{label}] detail must carry the `.factory` path, ErrorKind `{kind_dbg}` \
                         and message `{os_err}`; got {detail:?}"
                    ));
                }
                if line.contains(SECRET_ID) {
                    failures.push(format!("[{label}] raw tool_use_id leaked: {line}"));
                }
                if v.get("plugin_name").is_some() {
                    failures.push(format!("[{label}] plugin_name MUST NOT appear"));
                }
            }
            for ty in ["migration.admission_blocked", "migration.admission_failed"] {
                if !of_type(&all, ty).is_empty() {
                    failures.push(format!("[{label}] release must not write `{ty}`"));
                }
            }
            match std::fs::read(&res_path) {
                Ok(b) if b == res_before => {}
                other => failures.push(format!(
                    "[{label}] reservation must be LEFT UNCHANGED for the TTL GC; got {other:?}"
                )),
            }
            if raw_tree(&root) != tree_before {
                failures.push(format!(
                    "[{label}] nothing may be created or deleted by the release"
                ));
            }
        }
    }

    // Controls: ABSENT => silent no-op, exit 0, NO event.
    type Setup = fn(&Path) -> PathBuf;
    let controls: [(&str, Setup); 4] = [
        ("`.factory` regular file", |root| {
            std::fs::write(root.join(".factory"), b"x").unwrap();
            root.to_path_buf()
        }),
        ("`.factory` dangling symlink", |root| {
            std::os::unix::fs::symlink(root.join("nope"), root.join(".factory")).unwrap();
            root.to_path_buf()
        }),
        ("project root is a regular file (ENOTDIR)", |root| {
            std::fs::write(root.join("afile"), b"x").unwrap();
            root.join("afile")
        }),
        ("no `.factory`", |root| root.to_path_buf()),
    ];
    for (label, setup) in controls {
        for ev in ["PostToolUse", "PostToolUseFailure"] {
            let p = Project::bare();
            let logs = tempfile::tempdir().unwrap();
            let proj = setup(p.root());
            let before = raw_tree(p.root());
            let out = run_at(
                &p,
                &proj,
                logs.path(),
                &envelope(
                    ev,
                    "Write",
                    Some(SECRET_ID),
                    edit_input(&proj.join(CYCLES_PATH)),
                ),
            );
            let all = read_events(logs.path());
            if out.status.code() != Some(0)
                || stderr_of(&out).contains("E-MAINTENANCE")
                || !migration_event_types(&all).is_empty()
            {
                failures.push(format!(
                    "[control {label} / {ev}] ABSENT release must be a silent no-op with NO \
                     event; got exit {:?}, events {:?}",
                    out.status.code(),
                    migration_event_types(&all)
                ));
            }
            if raw_tree(p.root()) != before {
                failures.push(format!("[control {label} / {ev}] the tree changed"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC042_unstatable_factory_release_writes_one_reservation_release_failed_advisory_blackbox",
        failures,
    );
}

// ===========================================================================
// Delta 4 -- AdmissionStateIntegrity kinds on the event
// ===========================================================================

/// One state-integrity scenario: raw `txn-*.json` bytes (+ gate) => the
/// `migration.admission_failed` event has `cause=state_integrity` and the given
/// `kind`; exit 2 `E-MAINTENANCE-002 (state_integrity)`; exactly one event;
/// no `_blocked`; no reservation; no raw record content in the event; the txn and
/// gate bytes UNCHANGED.
#[cfg(unix)]
fn assert_integrity_kind(
    label: &str,
    gate: &str,
    rel: &str,
    txn: &[u8],
    lock_held: bool,
    want_kind: &str,
    failures: &mut Vec<String>,
) {
    let p = Project::new();
    write_gate(&p.ms(), gate);
    write_raw_txn(&p, txn);
    let _guard = lock_held.then(|| p.hold_lock());
    let logs = tempfile::tempdir().unwrap();
    let before = raw_tree(&p.ms());
    let out = run_at(
        &p,
        p.root(),
        logs.path(),
        &envelope(
            "PreToolUse",
            "Write",
            Some(SECRET_ID),
            edit_input(&p.abs(rel)),
        ),
    );
    let err = stderr_of(&out);
    if out.status.code() != Some(2)
        || !err.contains("E-MAINTENANCE-002: writer-admission check failed (state_integrity)")
    {
        failures.push(format!(
            "[{label}] expected exit 2 + E-MAINTENANCE-002 (state_integrity); got {:?}: {err}",
            out.status.code()
        ));
    }
    let all = read_events(logs.path());
    let failed = of_type(&all, "migration.admission_failed");
    if failed.len() != 1 {
        failures.push(format!(
            "[{label}] exactly ONE migration.admission_failed expected, found {} (events {:?})",
            failed.len(),
            migration_event_types(&all)
        ));
    } else {
        let (v, line) = failed[0];
        if v["cause"] != "state_integrity" || v["kind"] != want_kind {
            failures.push(format!(
                "[{label}] expected cause=state_integrity kind={want_kind}; got {v}"
            ));
        }
        if line.contains(SECRET_ID) || line.contains("SECRETCONTENT") {
            failures.push(format!(
                "[{label}] raw tool_use_id / record content leaked: {line}"
            ));
        }
    }
    if !of_type(&all, "migration.admission_blocked").is_empty() {
        failures.push(format!("[{label}] no _blocked for an E-MAINTENANCE-002"));
    }
    if p.reservation(SECRET_ID).exists() {
        failures.push(format!("[{label}] a reservation was left"));
    }
    if raw_tree(&p.ms()) != before {
        failures.push(format!(
            "[{label}] migration-state tree changed (txn/gate must be UNCHANGED; NOT rewritten \
             to ABORTED)"
        ));
    }
}

/// BC-1.18.013 v1.12 Precondition 6(c) rule 3 Tier 0 + EC-032 (v1.10 carrier list)
/// + BC-3.08.001 Event 12 `kind`. Tier 0 is applied to EVERY `txn-*.json` at read
/// time, regardless of gate state or whether any branch consumes the record:
///   * not a JSON object (`[]`, `"STAGING"`, `42`, `null`, `true`), a missing
///     `state`, a non-string `state`, or an unknown `state` (incl. wrong case)
///     => `kind = txn_record_malformed`;
///   * a PRESENT non-string `migration_id` (`42`, `null`, `true`, array, object),
///     on a live OR a non-live record => `kind = txn_migration_id_not_string`.
/// Both under both path families and the gate states OPEN / LOCKED.
/// (Unparseable JSON and numeric `migration_id` on a STAGING record are also pinned
/// by `..._EC038_admission_failed_event_per_cause_blackbox`.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC032_tier0_failures_event_kind_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let malformed: Vec<(&str, &[u8])> = vec![
        ("array record", b"[]"),
        ("string record", b"\"STAGING\""),
        ("number record", b"42"),
        ("null record", b"null"),
        ("boolean record", b"true"),
        ("empty object (state missing)", b"{}"),
        (
            "state missing, migration_id present",
            b"{\"migration_id\":\"migrate-bc-index\",\"x\":\"SECRETCONTENT\"}",
        ),
        ("state not a string (7)", b"{\"state\":7}"),
        ("state null", b"{\"state\":null}"),
        ("state unknown BOGUS", b"{\"state\":\"BOGUS\"}"),
        ("state wrong case", b"{\"state\":\"staging\"}"),
        (
            "state unknown, full-looking record",
            b"{\"txn_id\":\"t\",\"state\":\"PENDING\",\"generation_id\":null}",
        ),
    ];
    let not_string: Vec<(&str, &[u8])> = vec![
        (
            "STAGING migration_id 42",
            b"{\"state\":\"STAGING\",\"migration_id\":42}",
        ),
        (
            "STAGING migration_id null",
            b"{\"state\":\"STAGING\",\"migration_id\":null}",
        ),
        (
            "COMMITTING migration_id true",
            b"{\"state\":\"COMMITTING\",\"migration_id\":true}",
        ),
        (
            "COMPLETED (non-live) migration_id array",
            b"{\"state\":\"COMPLETED\",\"migration_id\":[\"x\"]}",
        ),
        (
            "ABORTED (non-live) migration_id object",
            b"{\"state\":\"ABORTED\",\"migration_id\":{\"a\":1}}",
        ),
    ];
    for (rel, fam) in [(BC_PATH, "bc"), (CYCLES_PATH, "cycles")] {
        for gate in ["OPEN", "LOCKED"] {
            for (what, bytes) in &malformed {
                assert_integrity_kind(
                    &format!("{fam}/{gate}: {what}"),
                    gate,
                    rel,
                    bytes,
                    false,
                    "txn_record_malformed",
                    &mut failures,
                );
            }
            for (what, bytes) in &not_string {
                assert_integrity_kind(
                    &format!("{fam}/{gate}: {what}"),
                    gate,
                    rel,
                    bytes,
                    false,
                    "txn_migration_id_not_string",
                    &mut failures,
                );
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC032_tier0_failures_event_kind_blackbox",
        failures,
    );
}

/// BC-1.18.013 v1.12 EC-045 + canonical vector: a KNOWN STAGING record WITHOUT a
/// usable `generation_id` -- the minimal `{"state":"STAGING","migration_id":...}`
/// (and `migration_id` absent => `migrate-bc-index`), a full Decision-7a record with
/// the key REMOVED, and `generation_id` of type number / array / boolean / object --
/// under the exact Branch B conditions (gate DRAINING, `exclusive.lock` FREE, own
/// terminal record ABSENT), both path families: exit 2
/// `E-MAINTENANCE-002 (state_integrity)`, ONE `migration.admission_failed` with
/// `cause=state_integrity` and `kind=txn_record_malformed`, txn bytes UNCHANGED (NOT
/// rewritten to ABORTED, no `abort_reason`), gate UNCHANGED, no reservation.
/// Control: the same minimal record with `exclusive.lock` HELD by a live coordinator
/// => Branch B does not execute, no field is read => plain `E-MAINTENANCE-001`
/// (`branch=live_coordinator`) and NO `migration.admission_failed`.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC045_staging_without_generation_id_event_kind_txn_record_malformed_blackbox() {
    let mut failures: Vec<String> = Vec::new();
    let fams: [(Option<&str>, &str); 3] = [
        (Some("migrate-bc-index"), BC_PATH),
        (Some("backfill-append-logs"), CYCLES_PATH),
        (None, BC_PATH),
    ];
    let full = |mig: Option<&str>, gen_id: Option<serde_json::Value>| -> Vec<u8> {
        let mut v = serde_json::json!({
            "txn_id": "txn-s2509",
            "activation_id": "act-s2509",
            "fencing_generation": 1,
            "state": "STAGING",
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
            v["migration_id"] = serde_json::json!(m);
        }
        serde_json::to_vec(&v).unwrap()
    };
    let minimal = |mig: Option<&str>, gen_id: Option<serde_json::Value>| -> Vec<u8> {
        let mut v = serde_json::json!({"state": "STAGING"});
        if let Some(m) = mig {
            v["migration_id"] = serde_json::json!(m);
        }
        if let Some(g) = gen_id {
            v["generation_id"] = g;
        }
        serde_json::to_vec(&v).unwrap()
    };
    for (mig, rel) in fams {
        for gate in ["DRAINING", "LOCKED", "OPEN"] {
            let mut cases: Vec<(String, Vec<u8>)> = vec![
                ("minimal, generation_id ABSENT".into(), minimal(mig, None)),
                ("full, generation_id KEY REMOVED".into(), full(mig, None)),
            ];
            for (what, val) in [
                ("number 7", serde_json::json!(7)),
                ("array", serde_json::json!(["x"])),
                ("boolean", serde_json::json!(true)),
                ("object", serde_json::json!({"id": "x"})),
            ] {
                cases.push((
                    format!("minimal, generation_id {what}"),
                    minimal(mig, Some(val.clone())),
                ));
                cases.push((format!("full, generation_id {what}"), full(mig, Some(val))));
            }
            for (what, bytes) in cases {
                assert_integrity_kind(
                    &format!("{mig:?} {rel} gate {gate}: {what}"),
                    gate,
                    rel,
                    &bytes,
                    false,
                    "txn_record_malformed",
                    &mut failures,
                );
            }
        }
    }

    // Control: live coordinator => plain E-MAINTENANCE-001, no field read, no _failed.
    for (mig, rel) in fams {
        let p = Project::new();
        write_gate(&p.ms(), "DRAINING");
        write_raw_txn(&p, &minimal(mig, None));
        let _guard = p.hold_lock();
        let logs = tempfile::tempdir().unwrap();
        let before = raw_tree(&p.ms());
        let out = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope(
                "PreToolUse",
                "Write",
                Some(SECRET_ID),
                edit_input(&p.abs(rel)),
            ),
        );
        let all = read_events(logs.path());
        let blocked = of_type(&all, "migration.admission_blocked");
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || !err.contains("E-MAINTENANCE-001")
            || err.contains("E-MAINTENANCE-002")
            || !of_type(&all, "migration.admission_failed").is_empty()
            || blocked.len() != 1
            || blocked[0].0["branch"] != "live_coordinator"
            || raw_tree(&p.ms()) != before
        {
            failures.push(format!(
                "[control live coordinator {mig:?} {rel}] expected plain E-MAINTENANCE-001 \
                 (branch=live_coordinator), no _failed, tree unchanged; got exit {:?}, events \
                 {:?}: {err}",
                out.status.code(),
                migration_event_types(&all)
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC045_staging_without_generation_id_event_kind_txn_record_malformed_blackbox",
        failures,
    );
}

// ===========================================================================
// Delta 5 -- the sixth kind `txn_record_newer_schema` and the Branch B version
// gate (ADR-052 v1.23 item 10(b)-(d); BC-1.18.013 v1.12 Branch B discard row).
// ===========================================================================

/// The Branch B pre-generation record: `state: STAGING`, `generation_id: null`, as
/// the minimal hand-built shape or the full Decision-7a shape, for the given
/// `migration_id` discriminator (`None` = key absent, read as `migrate-bc-index`).
fn branch_b_record(minimal: bool, mig: Option<&str>) -> serde_json::Value {
    let mut v = if minimal {
        serde_json::json!({"state": "STAGING", "generation_id": null})
    } else {
        serde_json::json!({
            "txn_id": "txn-s2509",
            "activation_id": "act-s2509",
            "fencing_generation": 1,
            "state": "STAGING",
            "generation_id": null,
            "source_sha256": null,
            "source_body_row_sha256": null,
            "intent_log_path": null,
            "pending_canonical_moves": [],
            "created_at": "2026-10-06T00:00:00Z",
            "updated_at": "2026-10-06T00:00:00Z",
        })
    };
    if let Some(m) = mig {
        v["migration_id"] = serde_json::json!(m);
    }
    v
}

/// ADR-052 v1.23 item 10(d) + 10(c), admission surface (BC-1.18.013 v1.12 Branch B
/// discard row, "version gate FIRST, then `generation_id` tri-state"): under the exact
/// Branch B conditions (gate DRAINING / LOCKED / OPEN, `exclusive.lock` FREE, own terminal
/// record ABSENT) a known STAGING record with `generation_id: null` whose `schema_version`
/// is
///  * a JSON integer >= 2 (`2`, `4294967296`), alone or beside a `generation_id` that would
///    itself be malformed (absent / a number) => exit 2 `E-MAINTENANCE-002
///    (state_integrity)`, EXACTLY ONE `migration.admission_failed` with
///    `cause=state_integrity` and `kind=txn_record_newer_schema` (the SIXTH token of the
///    closed BC-3.08.001 Event 12 `kind` domain), no `_blocked`, no reservation, the txn
///    and gate bytes UNCHANGED (NOT the discard);
///  * PRESENT and any other value (`0`, `"1"`, `1.5`, `null`, `true`) => `kind =
///    txn_record_malformed`, same shape, nothing mutated.
/// Both path families, the three `migration_id` shapes, minimal and full records.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC049_branch_b_null_generation_version_gate_event_kind_newer_schema_and_malformed_blackbox()
 {
    let mut failures: Vec<String> = Vec::new();
    let fams: [(Option<&str>, &str); 3] = [
        (Some("migrate-bc-index"), BC_PATH),
        (Some("backfill-append-logs"), CYCLES_PATH),
        (None, BC_PATH),
    ];
    for (mig, rel) in fams {
        for gate in ["DRAINING", "LOCKED", "OPEN"] {
            for minimal in [true, false] {
                let shape = if minimal { "minimal" } else { "full" };
                let mut cases: Vec<(String, serde_json::Value, &str)> = Vec::new();
                for (what, v) in [
                    ("2", serde_json::json!(2)),
                    ("4294967296", serde_json::json!(4_294_967_296u64)),
                ] {
                    let mut r = branch_b_record(minimal, mig);
                    r["schema_version"] = v;
                    cases.push((
                        format!("{shape}, schema_version {what}"),
                        r,
                        "txn_record_newer_schema",
                    ));
                }
                // newer wins over a malformed generation_id
                let mut r = branch_b_record(minimal, mig);
                r["schema_version"] = serde_json::json!(2);
                r.as_object_mut().unwrap().remove("generation_id");
                cases.push((
                    format!("{shape}, schema_version 2 + generation_id ABSENT"),
                    r,
                    "txn_record_newer_schema",
                ));
                let mut r = branch_b_record(minimal, mig);
                r["schema_version"] = serde_json::json!(2);
                r["generation_id"] = serde_json::json!(7);
                cases.push((
                    format!("{shape}, schema_version 2 + generation_id 7"),
                    r,
                    "txn_record_newer_schema",
                ));
                for (what, v) in [
                    ("0", serde_json::json!(0)),
                    ("string \"1\"", serde_json::json!("1")),
                    ("float 1.5", serde_json::json!(1.5)),
                    ("null", serde_json::Value::Null),
                    ("bool", serde_json::json!(true)),
                ] {
                    let mut r = branch_b_record(minimal, mig);
                    r["schema_version"] = v;
                    cases.push((
                        format!("{shape}, schema_version {what}"),
                        r,
                        "txn_record_malformed",
                    ));
                }
                for (what, rec, want_kind) in cases {
                    assert_integrity_kind(
                        &format!("{mig:?} {rel} gate {gate}: {what}"),
                        gate,
                        rel,
                        &serde_json::to_vec(&rec).unwrap(),
                        false,
                        want_kind,
                        &mut failures,
                    );
                }
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC049_branch_b_null_generation_version_gate_event_kind_newer_schema_and_malformed_blackbox",
        failures,
    );
}

/// ADR-052 v1.23 item 10(d) control, admission surface: `schema_version` ABSENT or `1`,
/// with or without an unknown extra top-level key, under the Branch B conditions => the
/// shared `abort_null_generation_txn` primitive discards: the request is ADMITTED (exit
/// 0), the txn record is rewritten in place to ABORTED + `abort_reason:
/// "null_generation"` with EVERY other key (the unknown key, `migration_id`,
/// `schema_version` as found) PRESERVED verbatim, the gate is OPEN, and no
/// `migration.admission_failed` / `_blocked` event is written. (The unknown-key rejection
/// exists only for rewrites that keep the record live.)
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC049_branch_b_discard_with_absent_or_1_schema_version_preserves_unknown_key_blackbox()
 {
    let mut failures: Vec<String> = Vec::new();
    let unknown = serde_json::json!({"nested": [1, "two"]});
    let mut cases: Vec<(String, serde_json::Value)> = Vec::new();
    for minimal in [true, false] {
        let shape = if minimal { "minimal" } else { "full" };
        let mut r = branch_b_record(minimal, Some("migrate-bc-index"));
        r["schema_v2_field"] = unknown.clone();
        cases.push((format!("{shape}, schema_version ABSENT + unknown key"), r));
        let mut r = branch_b_record(minimal, Some("migrate-bc-index"));
        r["schema_version"] = serde_json::json!(1);
        r["schema_v2_field"] = unknown.clone();
        cases.push((format!("{shape}, schema_version 1 + unknown key"), r));
        let mut r = branch_b_record(minimal, Some("migrate-bc-index"));
        r["schema_version"] = serde_json::json!(1);
        cases.push((format!("{shape}, schema_version 1, no unknown key"), r));
    }
    for (label, rec) in cases {
        let p = Project::new();
        write_gate(&p.ms(), "DRAINING");
        write_raw_txn(&p, &serde_json::to_vec(&rec).unwrap());
        let logs = tempfile::tempdir().unwrap();
        let out = run_at(
            &p,
            p.root(),
            logs.path(),
            &envelope(
                "PreToolUse",
                "Write",
                Some(SECRET_ID),
                edit_input(&p.abs(BC_PATH)),
            ),
        );
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] Branch B must discard and ADMIT (exit 0); got {:?}: {}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        let mut want = rec.clone();
        want["state"] = serde_json::json!("ABORTED");
        want["abort_reason"] = serde_json::json!("null_generation");
        let got: serde_json::Value = serde_json::from_slice(
            &std::fs::read(p.ms().join("txn-act-s2509.json")).unwrap_or_default(),
        )
        .unwrap_or(serde_json::Value::Null);
        if got != want {
            failures.push(format!(
                "[{label}] the discard must preserve every key as found; want {want}, got {got}"
            ));
        }
        let gate: serde_json::Value = serde_json::from_slice(
            &std::fs::read(p.ms().join("gate-state.json")).unwrap_or_default(),
        )
        .unwrap_or(serde_json::Value::Null);
        if gate != serde_json::json!("OPEN") {
            failures.push(format!("[{label}] the gate must be OPEN after the discard"));
        }
        let all = read_events(logs.path());
        for ty in ["migration.admission_failed", "migration.admission_blocked"] {
            if !of_type(&all, ty).is_empty() {
                failures.push(format!(
                    "[{label}] a successful discard must not write `{ty}`"
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC049_branch_b_discard_with_absent_or_1_schema_version_preserves_unknown_key_blackbox",
        failures,
    );
}
