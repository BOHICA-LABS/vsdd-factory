// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 T-14 (D-1252-OWED) -- the BC-1.18.013 EC-035 optionality vectors and
//! the EC-037 closed-`check`-domain vectors for the tokens THIS build emits.
//!
//! Scope split (BC-1.18.013 v1.11 Postcondition 10(a), ADR-052 v1.22 section 5a):
//! this build emits exactly three of the nine `check` tokens --
//! `staging_with_terminal_record` (1), `terminal_record_unverified` (8),
//! `finalize_unwired` (9). Tokens (2)-(7) belong to S-25.06's verifier and are
//! deliberately NOT asserted here. The nine-token domain is not exposed by the
//! crate as a type or constant (the emitting sites are string literals), so the
//! set-membership test below carries the BC's nine tokens as the SPEC and checks
//! every token this build emits against them.
//!
//! Every EC-037 vector drives the REAL `factory-dispatcher` binary (a PreToolUse
//! `Edit` envelope on stdin, stdin then closed, per-command timeout) and reads the
//! `dispatcher-internal-*.jsonl` file. The EC-035 library vectors call the public
//! production entry points; the crate-private `run_bc_index_migration_with_ttl`
//! vector lives in `src/shard_manager/ttl_seam_tests.rs` (in-crate by design).
//!
//! Already covered elsewhere (NOT duplicated here):
//! * EC-035 (A), the CLI route with `CLAUDE_PROJECT_DIR` set / empty / unset: the
//!   exact one-line stderr incl. the ALWAYS-present suffix --
//!   `s2508_admission_blackbox_test.rs::test_BC_1_18_013_EC035_coordinator_factory_root_not_found_exit_2_nothing_created_blackbox`.
//! * EC-037 row (9) `finalize_unwired`: unreachable black-box in this build (the
//!   verification seam reports every check false); asserted at unit level by
//!   `s2508_v121_units_test.rs::test_BC_1_18_013_EC039_undelivered_finalize_diagnostics_mapping`.
//!   Here it is only fed into the set-membership assertion.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use factory_dispatcher::shard_manager::{
    BcIndexMigrationError, ProjectRootSource, SessionProjectRoot, run_bc_index_migration,
    run_bc_index_migration_for_session,
};

const MISMATCH_SUFFIX: &str =
    " (completion-record mismatch \u{2014} operator investigation required)";
const TOOL_USE_ID: &str = "toolu_T14SECRET";
const CYCLES_PATH: &str = ".factory/cycles/c1/burst-log.md";
const BC_PATH: &str = ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md";

/// BC-1.18.013 v1.11 Postcondition 10(a) "Closed `check` value domain" (the spec).
const NINE_TOKENS: [&str; 9] = [
    "staging_with_terminal_record",
    "terminal_record_unparseable",
    "terminal_record_schema_mismatch",
    "txn_id_mismatch",
    "generation_id_mismatch",
    "canonical_paths_count_mismatch",
    "canonical_hash_mismatch",
    "terminal_record_unverified",
    "finalize_unwired",
];

// ---------------------------------------------------------------------------
// Harness (self-contained, mirrors tests/s2508_admission_blackbox_test.rs)
// ---------------------------------------------------------------------------

struct Project {
    dir: tempfile::TempDir,
    plugin_root: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("project tempdir");
        let plugin_root = tempfile::tempdir().expect("plugin_root tempdir");
        std::fs::write(
            plugin_root.path().join("hooks-registry.toml"),
            "schema_version = 2\n",
        )
        .unwrap();
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
    fn abs(&self, rel: &str) -> PathBuf {
        let p = self.root().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        p
    }
    fn reservation_count(&self) -> usize {
        std::fs::read_dir(self.ms().join("reservations"))
            .map(|it| it.flatten().count())
            .unwrap_or(0)
    }
}

fn write_gate(ms: &Path, state: &str) {
    std::fs::create_dir_all(ms).unwrap();
    std::fs::write(ms.join("gate-state.json"), format!("\"{state}\"")).unwrap();
}

fn write_txn(ms: &Path, state: &str, generation_id: Option<&str>, migration_id: &str) {
    let v = serde_json::json!({
        "txn_id": "txn-t14",
        "activation_id": "act-t14",
        "fencing_generation": 1,
        "state": state,
        "generation_id": generation_id,
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": null,
        "pending_canonical_moves": [],
        "created_at": "2026-10-06T00:00:00Z",
        "updated_at": "2026-10-06T00:00:00Z",
        "migration_id": migration_id,
    });
    std::fs::write(
        ms.join("txn-act-t14.json"),
        serde_json::to_vec_pretty(&v).unwrap(),
    )
    .unwrap();
}

/// (migration_id, own terminal-record file name, protected path in that family)
const FAMILIES: [(&str, &str, &str); 2] = [
    ("migrate-bc-index", "completed.json", BC_PATH),
    (
        "backfill-append-logs",
        "completed-backfill-append-logs.json",
        CYCLES_PATH,
    ),
];

fn scope_of(rel: &str) -> &'static str {
    if rel.starts_with(".factory/cycles/") {
        ".factory/cycles/"
    } else {
        "BC-INDEX"
    }
}

fn plain_msg(scope: &str) -> String {
    format!(
        "{scope} write blocked: migration window active (txn record in STAGING or COMMITTING \
         state); retry after migration completes or aborts"
    )
}

/// Run the real binary once with a PreToolUse `Edit` of `target`.
fn dispatch_edit(p: &Project, target: &Path) -> Output {
    let payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "session_id": "sess-t14",
        "tool_use_id": TOOL_USE_ID,
        "tool_input": {
            "file_path": target.to_string_lossy(),
            "old_string": "a",
            "new_string": "b",
        },
    })
    .to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_factory-dispatcher"));
    cmd.env("CLAUDE_PLUGIN_ROOT", p.plugin_root.path());
    cmd.env("CLAUDE_PROJECT_DIR", p.root());
    cmd.env("VSDD_LOG_DIR", p.root().join("logs"));
    cmd.env_remove("VSDD_TEST_ADMISSION_SEAM_DIR");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(payload.as_bytes()).unwrap();
    drop(stdin);
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("factory-dispatcher did not exit within 30s");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn read_events(p: &Project) -> Vec<serde_json::Value> {
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
    find(&p.root().join("logs"), &mut files);
    files.sort();
    let mut out = Vec::new();
    for f in files {
        for line in std::fs::read_to_string(f).unwrap_or_default().lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push(v);
            }
        }
    }
    out
}

fn of_type<'a>(all: &'a [serde_json::Value], ty: &str) -> Vec<&'a serde_json::Value> {
    all.iter().filter(|v| v["type"] == ty).collect()
}

/// Byte snapshot of a tree; directories recorded, unreadable files tolerated.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
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
                out.insert(rel, std::fs::read(&path).unwrap_or_default());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// The `.factory` subtree of the project (the logs dir is excluded: the run
/// legitimately appends the dispatcher-internal log there).
fn factory_snapshot(p: &Project) -> BTreeMap<String, Vec<u8>> {
    tree_snapshot(&p.root().join(".factory"))
}

fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// One Branch C `completion_record_mismatch` verdict, fully asserted.
fn assert_branch_c_verdict(
    label: &str,
    p: &Project,
    out: &Output,
    rel: &str,
    want_check: &str,
    before: &BTreeMap<String, Vec<u8>>,
    failures: &mut Vec<String>,
) {
    let all = read_events(p);
    let blocked = of_type(&all, "migration.admission_blocked");
    let advisory = of_type(&all, "migration.admission_advisory");
    let failed = of_type(&all, "migration.admission_failed");
    let err = stderr_of(out);
    let want_msg = format!(
        "E-MAINTENANCE-001: {}{MISMATCH_SUFFIX}",
        plain_msg(scope_of(rel))
    );
    if out.status.code() != Some(2) || !err.contains(&want_msg) {
        failures.push(format!(
            "[{label}] expected exit 2 with the mismatch-suffixed E-MAINTENANCE-001; got {:?}: {err}",
            out.status.code()
        ));
    }
    if !failed.is_empty() {
        failures.push(format!(
            "[{label}] no _failed for a Branch C verdict: {failed:?}"
        ));
    }
    if !advisory.is_empty() {
        failures.push(format!(
            "[{label}] a Branch C verification failure is ONE event, no _advisory: {advisory:?}"
        ));
    }
    if blocked.len() != 1 {
        failures.push(format!(
            "[{label}] exactly ONE _blocked expected, found {}",
            blocked.len()
        ));
        return;
    }
    let v = blocked[0];
    if v["branch"] != "completion_record_mismatch"
        || v["reconciliation"] != "completion_record_mismatch"
        || v["check"] != want_check
    {
        failures.push(format!(
            "[{label}] _blocked must be {{branch=reconciliation=completion_record_mismatch, \
             check={want_check}}}; got {v}"
        ));
    }
    if v.to_string().contains(TOOL_USE_ID) {
        failures.push(format!(
            "[{label}] raw tool_use_id leaked into the event: {v}"
        ));
    }
    let after = factory_snapshot(p);
    if &after != before {
        failures.push(format!(
            "[{label}] no txn/gate/record write: the .factory tree must be byte-identical; \
             before keys {:?} after keys {:?}",
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        ));
    }
    if p.reservation_count() != 0 {
        failures.push(format!("[{label}] no reservation may be left behind"));
    }
}

// ---------------------------------------------------------------------------
// EC-037 row (1): STAGING + own terminal record => `staging_with_terminal_record`
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.11 EC-037 row (1) / Postcondition 10(a) item (1): STAGING with
/// gen-1 and the txn's OWN terminal record present (STAGING + record is ALWAYS
/// fail-closed) => the one `_blocked` carries EXACTLY `staging_with_terminal_record`
/// (the S-25.08 test only asserts "a string"). Both migration families.
#[test]
fn test_BC_1_18_013_EC037_row1_staging_with_terminal_record_exact_token_blackbox() {
    let mut failures = Vec::new();
    for (mig, record, rel) in FAMILIES {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "STAGING", Some("gen-1"), mig);
        std::fs::write(p.ms().join(record), br#"{"txn_id":"act-t14"}"#).unwrap();
        let target = p.abs(rel);
        let before = factory_snapshot(&p);
        let out = dispatch_edit(&p, &target);
        assert_branch_c_verdict(
            &format!("row1 {mig}"),
            &p,
            &out,
            rel,
            "staging_with_terminal_record",
            &before,
            &mut failures,
        );
    }
    assert_no_failures(
        "test_BC_1_18_013_EC037_row1_staging_with_terminal_record_exact_token_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// EC-037 row (8): COMMITTING + own record present, ANY content
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.11 EC-037 row (8): COMMITTING + own terminal record present with
/// ANY content (non-UTF-8, empty, truncated, wrong shape, well-formed) =>
/// `terminal_record_unverified` EXACTLY (never the S-25.06 content tokens (2)-(5)
/// -- this build does not examine content), the mismatch suffix, no advisory, no
/// txn/gate write. Both migration families.
#[test]
fn test_BC_1_18_013_EC037_row8_terminal_record_unverified_for_any_record_content_blackbox() {
    let mut failures = Vec::new();
    let contents: [(&str, &[u8]); 6] = [
        ("non-UTF-8 bytes", b"\xff\xfe"),
        ("empty file", b""),
        ("truncated JSON", br#"{"txn_id":"#),
        ("valid JSON, wrong shape", br#"{"txn_id":7}"#),
        (
            "valid JSON, txn_id != activation_id",
            br#"{"generation_id":"gen-1","txn_id":"other","completed_at":"2026-10-06T00:00:00Z","canonical_paths_count":4}"#,
        ),
        (
            "fully well-formed matching record",
            br#"{"generation_id":"gen-1","txn_id":"act-t14","completed_at":"2026-10-06T00:00:00Z","canonical_paths_count":4}"#,
        ),
    ];
    for (mig, record, rel) in FAMILIES {
        for (variant, bytes) in contents {
            let p = Project::new();
            write_gate(&p.ms(), "LOCKED");
            write_txn(&p.ms(), "COMMITTING", Some("gen-1"), mig);
            std::fs::write(p.ms().join(record), bytes).unwrap();
            let target = p.abs(rel);
            let before = factory_snapshot(&p);
            let out = dispatch_edit(&p, &target);
            assert_branch_c_verdict(
                &format!("row8 {mig} / {variant}"),
                &p,
                &out,
                rel,
                "terminal_record_unverified",
                &before,
                &mut failures,
            );
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC037_row8_terminal_record_unverified_for_any_record_content_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// EC-037 set membership across the emitted tokens
// ---------------------------------------------------------------------------

/// BC-1.18.013 v1.11 EC-037 "Set-membership assertion": across all rows every
/// non-null `check` is one of the nine tokens. Collected from the real binary
/// (rows (1) and (8)) and from the pure builder for the undelivered-finalize
/// seam (row (9), `finalize_unwired`); every one of the three tokens this build
/// emits is also required to appear (so the test cannot pass on an empty set), and
/// a non-Branch-C block carries `check = null`.
#[test]
fn test_BC_1_18_013_EC037_every_emitted_check_token_is_in_the_closed_nine_token_domain() {
    use factory_dispatcher::shard_manager::{
        BcIndexAdmissionGateState, ProtectedPathFamily, diagnostics_for_undelivered_finalize,
    };
    let mut emitted: Vec<String> = Vec::new();
    let mut failures = Vec::new();

    let mut run_case = |state: &str, content: &[u8], failures: &mut Vec<String>| {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), state, Some("gen-1"), "migrate-bc-index");
        std::fs::write(p.ms().join("completed.json"), content).unwrap();
        let _ = dispatch_edit(&p, &p.abs(BC_PATH));
        let all = read_events(&p);
        let blocked = of_type(&all, "migration.admission_blocked");
        if blocked.len() != 1 {
            failures.push(format!(
                "{state}: expected one _blocked, got {}",
                blocked.len()
            ));
            return;
        }
        match blocked[0]["check"].as_str() {
            Some(t) => emitted.push(t.to_string()),
            None => failures.push(format!(
                "{state}: Branch C block must carry a check: {}",
                blocked[0]
            )),
        }
    };
    run_case("STAGING", b"{}", &mut failures);
    run_case("COMMITTING", b"{}", &mut failures);
    run_case("COMMITTING", b"\xff\xfe", &mut failures);

    // row (9): the pure undelivered-finalize builder (unreachable black-box here)
    let (diags, _msg) = diagnostics_for_undelivered_finalize(
        ProtectedPathFamily::Cycles,
        BcIndexAdmissionGateState::Locked,
        "backfill-append-logs",
        "txn-t14",
    );
    for d in &diags {
        let f: BTreeMap<_, _> = d.fields().into_iter().collect();
        if let Some(c) = f.get("check") {
            emitted.push(c.as_str().unwrap_or("<non-string>").to_string());
        }
    }

    // non-Branch-C block: check is null
    {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "STAGING", Some("gen-1"), "migrate-bc-index");
        let _ = dispatch_edit(&p, &p.abs(BC_PATH));
        let all = read_events(&p);
        let blocked = of_type(&all, "migration.admission_blocked");
        if blocked.len() != 1
            || !blocked[0]["check"].is_null()
            || blocked[0]["branch"] != "live_txn"
        {
            failures.push(format!("live_txn block must carry check=null: {blocked:?}"));
        }
    }

    for t in &emitted {
        if !NINE_TOKENS.contains(&t.as_str()) {
            failures.push(format!(
                "emitted check {t:?} is not in the closed nine-token domain"
            ));
        }
    }
    for must in [
        "staging_with_terminal_record",
        "terminal_record_unverified",
        "finalize_unwired",
    ] {
        if !emitted.iter().any(|t| t == must) {
            failures.push(format!(
                "this build must emit `{must}`; emitted: {emitted:?}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC037_every_emitted_check_token_is_in_the_closed_nine_token_domain",
        failures,
    );
}

// ---------------------------------------------------------------------------
// EC-037 row (10) / ADR-052 v1.22 "Read-failure mapping": io, never a check token
// ---------------------------------------------------------------------------

fn assert_io_failure(
    label: &str,
    p: &Project,
    out: &Output,
    before: &BTreeMap<String, Vec<u8>>,
    failures: &mut Vec<String>,
) {
    let all = read_events(p);
    let failed = of_type(&all, "migration.admission_failed");
    let err = stderr_of(out);
    if out.status.code() != Some(2)
        || !err.contains("E-MAINTENANCE-002: writer-admission check failed (io)")
        || err.contains("E-MAINTENANCE-001")
    {
        failures.push(format!(
            "[{label}] expected exit 2 with `E-MAINTENANCE-002: writer-admission check failed (io)` \
             and no E-MAINTENANCE-001; got {:?}: {err}",
            out.status.code()
        ));
    }
    if failed.len() != 1 {
        failures.push(format!(
            "[{label}] exactly ONE _failed expected, found {}",
            failed.len()
        ));
    } else {
        let v = failed[0];
        if v["cause"] != "io" || !v.get("kind").is_some_and(serde_json::Value::is_null) {
            failures.push(format!(
                "[{label}] _failed must be cause=io, kind=null: {v}"
            ));
        }
        if v.to_string().contains(TOOL_USE_ID) {
            failures.push(format!("[{label}] raw tool_use_id leaked: {v}"));
        }
    }
    for ty in [
        "migration.admission_blocked",
        "migration.admission_advisory",
    ] {
        if !of_type(&all, ty).is_empty() {
            failures.push(format!("[{label}] NO {ty} for a failed read CALL"));
        }
    }
    if &factory_snapshot(p) != before {
        failures.push(format!(
            "[{label}] no txn/gate write, txn not finalized and not discarded: tree must be byte-identical"
        ));
    }
    if p.reservation_count() != 0 {
        failures.push(format!("[{label}] no reservation may be left behind"));
    }
}

/// BC-1.18.013 v1.11 EC-037 row (10) + EC-032 + ADR-052 v1.22 "Read-failure
/// mapping": the terminal-record path is a DIRECTORY (EISDIR on the read CALL) =>
/// `E-MAINTENANCE-002 (io)`, one `_failed`, NO `_blocked`, NO `_advisory`, no
/// write -- never a `check` token. Covers COMMITTING and the STAGING
/// null-generation shape (the Branch B candidate): an unreadable record must never
/// be classified ABSENT, and the txn must not be discarded. Both families.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC037_row10_terminal_record_path_is_directory_maps_to_io_blackbox() {
    let mut failures = Vec::new();
    for (mig, record, rel) in FAMILIES {
        for (state, gen_id) in [("COMMITTING", Some("gen-1")), ("STAGING", None)] {
            let p = Project::new();
            write_gate(
                &p.ms(),
                if gen_id.is_some() {
                    "LOCKED"
                } else {
                    "DRAINING"
                },
            );
            write_txn(&p.ms(), state, gen_id, mig);
            std::fs::create_dir(p.ms().join(record)).unwrap();
            let target = p.abs(rel);
            let before = factory_snapshot(&p);
            let out = dispatch_edit(&p, &target);
            assert_io_failure(
                &format!("row10 dir {mig} / {state}"),
                &p,
                &out,
                &before,
                &mut failures,
            );
        }
    }
    assert_no_failures(
        "test_BC_1_18_013_EC037_row10_terminal_record_path_is_directory_maps_to_io_blackbox",
        failures,
    );
}

#[cfg(unix)]
struct RestoreMode(PathBuf);
#[cfg(unix)]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o600));
    }
}

#[cfg(unix)]
fn running_as_root() -> bool {
    let o = Command::new("id")
        .arg("-u")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout).trim() == "0"
}

/// BC-1.18.013 v1.11 EC-037 row (10), `chmod 000` variant (EACCES on the read
/// CALL of an existing record): `E-MAINTENANCE-002 (io)`, no `_blocked`. Skipped
/// when running as root (root bypasses mode bits; the vector cannot be built).
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC037_row10_terminal_record_chmod_000_maps_to_io_blackbox() {
    use std::os::unix::fs::PermissionsExt;
    if running_as_root() {
        eprintln!("SKIP: running as root; chmod 000 does not deny reads");
        return;
    }
    let mut failures = Vec::new();
    for (mig, record, rel) in FAMILIES {
        let p = Project::new();
        write_gate(&p.ms(), "LOCKED");
        write_txn(&p.ms(), "COMMITTING", Some("gen-1"), mig);
        let rec = p.ms().join(record);
        std::fs::write(&rec, br#"{"txn_id":"act-t14"}"#).unwrap();
        std::fs::set_permissions(&rec, std::fs::Permissions::from_mode(0o000)).unwrap();
        let _restore = RestoreMode(rec.clone());
        let target = p.abs(rel);
        let before = factory_snapshot(&p);
        let out = dispatch_edit(&p, &target);
        assert_io_failure(
            &format!("row10 chmod000 {mig}"),
            &p,
            &out,
            &before,
            &mut failures,
        );
    }
    assert_no_failures(
        "test_BC_1_18_013_EC037_row10_terminal_record_chmod_000_maps_to_io_blackbox",
        failures,
    );
}

// ---------------------------------------------------------------------------
// EC-035 optionality vectors (B), (C), (D) -- library entries
// ---------------------------------------------------------------------------

fn root_without_factory(variant_file: bool) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    if variant_file {
        std::fs::write(d.path().join(".factory"), b"i am a file").unwrap();
    }
    d
}

/// BC-1.18.013 v1.11 EC-035 vector (B): the path-only public entry
/// `run_bc_index_migration(&P)` returns `FactoryRootNotFound { root_source: None, .. }`
/// whose `to_string()` is EXACTLY `FACTORY_ROOT_NOT_FOUND: no .factory directory
/// under project root <P>` (no trailing space, no `(resolved from`), and (D) the
/// `{ project_root, root_source }` pattern compiles; tree byte-identical.
#[test]
fn test_BC_1_18_013_EC035_path_only_library_entry_root_source_none_suffixless_display() {
    for variant_file in [false, true] {
        let d = root_without_factory(variant_file);
        let before = tree_snapshot(d.path());
        let r = run_bc_index_migration(d.path());
        match r {
            Err(
                e @ BcIndexMigrationError::FactoryRootNotFound {
                    project_root: _,
                    root_source: _,
                },
            ) => {
                let BcIndexMigrationError::FactoryRootNotFound {
                    project_root,
                    root_source,
                } = &e
                else {
                    unreachable!()
                };
                assert_eq!(project_root, d.path(), "project_root echoes the given path");
                assert_eq!(*root_source, None, "path-only entry => root_source None");
                let shown = e.to_string();
                assert_eq!(
                    shown,
                    format!(
                        "FACTORY_ROOT_NOT_FOUND: no .factory directory under project root {}",
                        d.path().display()
                    ),
                    "suffix-less Display, exact (variant_file={variant_file})"
                );
                assert!(!shown.ends_with(' ') && !shown.contains("(resolved from"));
            }
            other => {
                panic!("expected FactoryRootNotFound (variant_file={variant_file}), got {other:?}")
            }
        }
        assert_eq!(
            tree_snapshot(d.path()),
            before,
            "nothing created or mutated (variant_file={variant_file})"
        );
    }
}

/// BC-1.18.013 v1.11 EC-035 vector (C): `run_bc_index_migration_for_session` threads
/// the whole `SessionProjectRoot` => `root_source: Some(ClaudeProjectDir)` /
/// `Some(ProcessCwd)` and a `to_string()` ending ` (resolved from CLAUDE_PROJECT_DIR)`
/// / ` (resolved from process cwd)`; tree byte-identical.
#[test]
fn test_BC_1_18_013_EC035_for_session_entry_threads_root_source_with_suffix() {
    for (source, label) in [
        (ProjectRootSource::ClaudeProjectDir, "CLAUDE_PROJECT_DIR"),
        (ProjectRootSource::ProcessCwd, "process cwd"),
    ] {
        let d = root_without_factory(false);
        let before = tree_snapshot(d.path());
        let root = SessionProjectRoot {
            path: d.path().to_path_buf(),
            source,
        };
        match run_bc_index_migration_for_session(&root) {
            Err(e @ BcIndexMigrationError::FactoryRootNotFound { .. }) => {
                let BcIndexMigrationError::FactoryRootNotFound {
                    project_root,
                    root_source,
                } = &e
                else {
                    unreachable!()
                };
                assert_eq!(project_root, d.path());
                assert_eq!(*root_source, Some(source));
                assert_eq!(
                    e.to_string(),
                    format!(
                        "FACTORY_ROOT_NOT_FOUND: no .factory directory under project root {} \
                         (resolved from {label})",
                        d.path().display()
                    )
                );
            }
            other => panic!("expected FactoryRootNotFound ({label}), got {other:?}"),
        }
        assert_eq!(
            tree_snapshot(d.path()),
            before,
            "tree byte-identical ({label})"
        );
    }
}
