// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the REAL `migrate-bc-index` COORDINATOR (spawned binary), module
//! independent: what an operator and the S-25.09 recovery legs observe.
//!
//! Authority: ADR-054 v1.0 Decision 1.7-1.9, Decision 3.1 (exact stderr lines, corrections
//! C-1..C-3); BC-1.18.011 v1.21 Postcondition 15(b)(c)(e), EC-065..EC-068; story S-25.10
//! AC-005, AC-006..AC-008, AC-010 (the two exact lines through `migrate-bc-index`), AC-011.
//!
//! This file needs NOTHING from the not-yet-existing `shard_manager::intent_log` module: it
//! builds every log with the independent reference encoder, drives the real binary, and reads
//! results back with the independent reference parser, so it compiles today and fails by
//! ASSERTION (a genuine Red: the current coordinator speaks the OLD log format).
//!
//! The fixture is a crashed `migrate-bc-index` (txn COMMITTING, plan of three moves: two
//! ABSENT canonical targets and one PRESENT) with whatever intent-log bytes the test writes;
//! the forward-recovery arm reads that log before it moves anything.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-008 / T2 (end to end) | `..._EC065_blackbox_torn_tail_is_repaired_before_the_first_append_then_the_run_completes` |
//! | AC-007 / T4 | `..._EC066_blackbox_bit_flip_in_each_field_of_a_middle_record_exits_2_intent_log_corrupt_nothing_mutated` |
//! | AC-006 / A-1 | `..._EC066_blackbox_garbage_between_records_exits_2_never_a_silent_drop` |
//! | AC-007 / T15 | `..._EC068_blackbox_log_invariants_l1_to_l4_exit_2_log_invariant_violation_nothing_mutated` |
//! | AC-005 / T6 | `..._EC067_plan_builder_rejects_lf_cr_nul_non_utf8_overlong_paths_nothing_staged` |
//! | AC-005 / T6 (`failpoints`) | `..._EC067_rejection_after_generation_dir_takes_the_pre_commit_abort_path_then_prints` |
//! | AC-005 / T6 (`failpoints`) | `..._EC067_a_failed_abort_write_is_that_writes_own_io_never_the_rejection_line` |
//! | AC-008 / T2 (`failpoints`) | `..._EC065_blackbox_staging_resume_repairs_a_torn_tail_before_the_intent_batch_append` |
//! | AC-011 (oracle twin) | `..._15c_full_run_log_is_a_pure_record_sequence_and_every_done_copies_its_intent` |
//!
//! Every spawned child has stdin set to null and a per-command timeout.

#[path = "s2510_support/mod.rs"]
mod support;

use std::path::Path;

use support::{
    Cx, LOG_FILE, Rec, assert_no_failures, canonical_bc_index, changed, concat, corrupt_line,
    encode, parse_prefix, read_txn, run_migrate, setup_fresh_fixture, stderr_of,
    value_rejected_line,
};

/// In a failpoints build, serialize against the process-global fail registry.
#[cfg(feature = "failpoints")]
fn fail_point_scope() -> fail::FailScenario<'static> {
    fail::FailScenario::setup()
}

fn exact_line_failures(
    label: &str,
    out: &std::process::Output,
    code: i32,
    line: &str,
    failures: &mut Vec<String>,
) {
    let err = stderr_of(out);
    if out.status.code() != Some(code) || !out.stdout.is_empty() || err != format!("{line}\n") {
        failures.push(format!(
            "[{label}] expected exit {code}, empty stdout and stderr EXACTLY\n    {line:?}\n  got \
             exit {:?}, stdout {} bytes, stderr\n    {err:?}",
            out.status.code(),
            out.stdout.len()
        ));
    }
}

// ===========================================================================
// AC-008 / T2 end to end: a torn tail is repaired before the first append
// ===========================================================================

/// BC-1.18.011 EC-065 / ADR-054 1.9 step 2: valid INTENTs + a torn tail (garbage, 512 NULs,
/// a partial start marker, a partial END line, invalid UTF-8) then forward recovery moves
/// all three targets and appends their DONEs. The coordinator durably TRUNCATES the tail
/// before its first append, so the final log is a pure sequence of six valid records:
/// the three original INTENT records BYTE-IDENTICAL, then the three DONEs; nothing is
/// sandwiched behind the garbage (A-1) and a single invalid UTF-8 byte does not make the
/// log unreadable (A-2).
#[test]
fn test_BC_1_18_011_EC065_blackbox_torn_tail_is_repaired_before_the_first_append_then_the_run_completes()
 {
    let end_line = "END_INTENT_LOG_RECORD\n".len();
    let tails: Vec<(&str, Vec<u8>)> = vec![
        ("garbage line", b"this is not a record\n".to_vec()),
        ("garbage, no LF", b"zzzz".to_vec()),
        ("512 NUL bytes", vec![0u8; 512]),
        ("partial start marker", b"INTENT_LOG_RECO".to_vec()),
        ("partial END line", {
            let e = encode(&Rec::sample("any", 9, "INTENT", 1));
            let mut t = e[..e.len() - end_line].to_vec();
            t.extend_from_slice(b"END_INTENT_LOG_");
            t
        }),
        ("invalid UTF-8 bytes", vec![0xff, 0xfe, 0x80, 0xc3, 0x28]),
    ];
    let mut failures = Vec::new();
    for (label, tail) in tails {
        let cx = Cx::new();
        let intents: Vec<Vec<u8>> = cx.intents().iter().map(encode).collect();
        let prefix = concat(&intents);
        let mut log = prefix.clone();
        log.extend_from_slice(&tail);
        cx.write_log(&log);

        let out = cx.run();
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] forward recovery over a log with a torn tail must complete (exit 0); \
                 got {:?}, stderr {:?}",
                out.status.code(),
                stderr_of(&out)
            ));
            continue;
        }
        let bytes = std::fs::read(cx.log_path()).unwrap();
        let (recs, valid) = parse_prefix(&bytes);
        if valid != bytes.len() {
            failures.push(format!(
                "[{label}] the final log must be a PURE sequence of valid records (the torn tail \
                 truncated before the first append, nothing sandwiched): valid prefix {valid} of \
                 {} bytes",
                bytes.len()
            ));
        }
        if bytes.len() < prefix.len() || bytes[..prefix.len()] != prefix[..] {
            failures.push(format!(
                "[{label}] the three INTENT records must be preserved byte-identical"
            ));
        }
        let dones: Vec<_> = recs.iter().filter(|p| p.rec.rtype == "DONE").collect();
        if recs.len() != 6 || dones.len() != 3 {
            failures.push(format!(
                "[{label}] expected exactly INTENT x3 then DONE x3 (6 records); got {} records, {} DONE",
                recs.len(),
                dones.len()
            ));
        }
        for m in cx.moves() {
            if std::fs::read(&m.canonical).ok().as_deref() != Some(m.staged.as_slice()) {
                failures.push(format!(
                    "[{label}] {} must hold the staged content",
                    m.canonical.display()
                ));
            }
        }
        if !cx.ms().join("completed.json").exists() {
            failures.push(format!("[{label}] completed.json must be written"));
        }
    }
    assert_no_failures("torn-tail repair, end to end", failures);
}

// ===========================================================================
// AC-007 / T4: bit flips in a MIDDLE record
// ===========================================================================

/// Flip the lowest bit of the first value byte of record line `line_idx` (0 = start marker,
/// 1..=9 = `txn_id`..`record_checksum`, 10 = END marker).
fn flip_line(record: &[u8], line_idx: usize) -> Vec<u8> {
    let mut starts = vec![0usize];
    for (i, b) in record.iter().enumerate() {
        if *b == b'\n' {
            starts.push(i + 1);
        }
    }
    let line_start = starts[line_idx];
    let value_start = if (1..=9).contains(&line_idx) {
        line_start
            + record[line_start..]
                .iter()
                .position(|&b| b == b'=')
                .unwrap()
            + 1
    } else {
        line_start
    };
    let mut out = record.to_vec();
    out[value_start] ^= 0x01;
    out
}

/// BC-1.18.011 EC-066 / ADR-054 1.7 (iv): a bit flip in EACH field of a MIDDLE record that
/// is followed by a later valid record is corruption in the middle -- the coordinator exits 2
/// with the exact `INTENT_LOG_CORRUPT` line (`kind` = `mid_log_corruption`, `<offset>` = the
/// reader's `first_bad_offset`, i.e. where the corrupt record starts), makes no further move
/// or append, and the whole project tree (log, txn, gate, canonical and staged files) is
/// byte-identical.
#[test]
fn test_BC_1_18_011_EC066_blackbox_bit_flip_in_each_field_of_a_middle_record_exits_2_intent_log_corrupt_nothing_mutated()
 {
    let names = [
        "start marker",
        "txn_id",
        "fencing_generation",
        "record_type",
        "target_canonical",
        "staging_path",
        "expected_post_hash",
        "expected_pre_state",
        "timestamp_utc",
        "record_checksum",
        "END marker",
    ];
    let mut failures = Vec::new();
    for (line_idx, name) in names.iter().enumerate() {
        let cx = Cx::new();
        let enc: Vec<Vec<u8>> = cx.intents().iter().map(encode).collect();
        let flipped = flip_line(&enc[1], line_idx);
        assert_ne!(flipped, enc[1], "fixture sanity: the flip changed a byte");
        cx.write_log(&concat(&[enc[0].clone(), flipped, enc[2].clone()]));

        let before = cx.snapshot();
        let out = cx.run();
        let after = cx.snapshot();
        exact_line_failures(
            &format!("flip in {name}"),
            &out,
            2,
            &corrupt_line(LOG_FILE, "mid_log_corruption", enc[0].len()),
            &mut failures,
        );
        if before != after {
            failures.push(format!(
                "[flip in {name}] nothing may be mutated; changed: {:?}",
                changed(&before, &after)
            ));
        }
    }
    assert_no_failures("mid-log bit flips", failures);
}

/// ADR-054 1.7 (iv) / A-1: R1, then non-record garbage that ends at a line boundary, then a
/// valid R3 -- the reader never silently drops R3; the coordinator exits 2 mid-log at the
/// first byte of the garbage and mutates nothing.
#[test]
fn test_BC_1_18_011_EC066_blackbox_garbage_between_records_exits_2_never_a_silent_drop() {
    let mut failures = Vec::new();
    for (label, garbage) in [
        ("garbage line", b"garbage\n".to_vec()),
        ("invalid UTF-8 line", vec![0xff, 0xfe, b'\n']),
        ("NULs then LF", {
            let mut v = vec![0u8; 64];
            v.push(b'\n');
            v
        }),
    ] {
        let cx = Cx::new();
        let enc: Vec<Vec<u8>> = cx.intents().iter().map(encode).collect();
        cx.write_log(&concat(&[enc[0].clone(), garbage, enc[2].clone()]));
        let before = cx.snapshot();
        let out = cx.run();
        let after = cx.snapshot();
        exact_line_failures(
            label,
            &out,
            2,
            &corrupt_line(LOG_FILE, "mid_log_corruption", enc[0].len()),
            &mut failures,
        );
        if before != after {
            failures.push(format!(
                "[{label}] nothing may be mutated; changed: {:?}",
                changed(&before, &after)
            ));
        }
    }
    assert_no_failures("garbage between records", failures);
}

// ===========================================================================
// AC-007 / T15: L1-L4 through the binary
// ===========================================================================

/// BC-1.18.011 EC-068 / ADR-054 1.8 and 3.1: each log-level invariant violation over
/// otherwise VALID records is exit 2 `INTENT_LOG_CORRUPT` with `<kind>` =
/// `log_invariant_violation` and `<offset>` = the start offset of the FIRST violating
/// record in file order; nothing is mutated.
#[test]
fn test_BC_1_18_011_EC068_blackbox_log_invariants_l1_to_l4_exit_2_log_invariant_violation_nothing_mutated()
 {
    // Each vector builds its records from the per-run fixture's intents `i` (their paths embed
    // the run's temp directory) and names the index of the first violating record.
    type Build = Box<dyn Fn(&[Rec]) -> (Vec<Rec>, usize)>;
    let foreign = |r: &Rec| Rec {
        txn_id: "other-txn".into(),
        ..r.clone()
    };
    let vectors: Vec<(&str, Build)> = vec![
        (
            "L1 foreign txn_id",
            Box::new(move |i| (vec![i[0].clone(), foreign(&i[1]), i[2].clone()], 1)),
        ),
        (
            "L2 conflicting INTENTs (expected_post_hash)",
            Box::new(|i| {
                (
                    vec![
                        i[0].clone(),
                        i[1].clone(),
                        Rec {
                            post: support::sha256_hex(b"other"),
                            ..i[0].clone()
                        },
                    ],
                    2,
                )
            }),
        ),
        (
            "L3 DONE with no INTENT for its target",
            Box::new(|i| {
                (
                    vec![
                        i[0].clone(),
                        i[1].clone(),
                        i[2].done_of(1, "2026-10-08T12:00:09Z"),
                    ],
                    2,
                )
            }),
        ),
        (
            "L4 decreasing fencing_generation",
            Box::new(|i| {
                (
                    vec![
                        Rec {
                            fencing: 2,
                            ..i[0].clone()
                        },
                        i[1].clone(),
                        i[2].clone(),
                    ],
                    1,
                )
            }),
        ),
    ];
    let mut failures = Vec::new();
    for (label, build) in vectors {
        let cx = Cx::new();
        let (recs, at) = build(&cx.intents());
        let parts: Vec<Vec<u8>> = recs.iter().map(encode).collect();
        let offset: usize = parts[..at].iter().map(Vec::len).sum();
        cx.write_log(&concat(&parts));
        let before = cx.snapshot();
        let out = cx.run();
        let after = cx.snapshot();
        exact_line_failures(
            label,
            &out,
            2,
            &corrupt_line(LOG_FILE, "log_invariant_violation", offset),
            &mut failures,
        );
        if before != after {
            failures.push(format!(
                "[{label}] nothing may be mutated; changed: {:?}",
                changed(&before, &after)
            ));
        }
    }
    assert_no_failures("log invariants through the binary", failures);
}

// ===========================================================================
// AC-005 / T6: the plan builder validates every plan path
// ===========================================================================

/// The accepted stderr for a plan-path rejection: either plan field may be the first named
/// (both embed the hostile project root), with the closed-domain `reason`.
fn plan_rejection_lines(reason: &str) -> Vec<String> {
    ["staging_path", "canonical_path"]
        .iter()
        .map(|f| value_rejected_line(f, reason))
        .collect()
}

/// Sizes of the intent logs under `migration-state/` (the baseline a rejection must not grow).
#[cfg(feature = "failpoints")]
fn intent_log_sizes(root: &Path) -> std::collections::BTreeMap<String, u64> {
    let ms = root.join(".factory/migration-state");
    std::fs::read_dir(&ms)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.starts_with("intent-")
                        .then(|| (n, e.metadata().map(|m| m.len()).unwrap_or(0)))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// State invariants common to a plan rejection BEFORE and AFTER the generation directory
/// exists: nothing committed, nothing live, the gate not left blocking, no INTENT appended,
/// the canonical source untouched, no staged generation left behind.
fn plan_rejection_state_failures(
    label: &str,
    root: &Path,
    original: &[u8],
    logs_before: &std::collections::BTreeMap<String, u64>,
    failures: &mut Vec<String>,
) {
    let ms = root.join(".factory/migration-state");
    let names: Vec<String> = std::fs::read_dir(&ms)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    if names.iter().any(|n| n.starts_with("gen-")) {
        failures.push(format!(
            "[{label}] no staged generation may remain: {names:?}"
        ));
    }
    for forbidden in ["CURRENT.json", "completed.json"] {
        if names.iter().any(|n| n == forbidden) {
            failures.push(format!("[{label}] {forbidden} must not exist"));
        }
    }
    for n in names.iter().filter(|n| n.starts_with("intent-")) {
        let len = std::fs::metadata(ms.join(n)).map(|m| m.len()).unwrap_or(0);
        let before = logs_before.get(n).copied().unwrap_or(0);
        if len != before {
            failures.push(format!(
                "[{label}] no record of the rejected batch may be appended ({n} held {before} \
                 bytes before the run and holds {len} after)"
            ));
        }
    }
    for n in names
        .iter()
        .filter(|n| n.starts_with("txn-") && n.ends_with(".json"))
    {
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(ms.join(n)).unwrap()).unwrap();
        if v["state"] != serde_json::json!("ABORTED") {
            failures.push(format!(
                "[{label}] no live txn may remain (pre-commit abort path): {n} is {}",
                v["state"]
            ));
        }
    }
    if let Ok(gate) = std::fs::read(ms.join("gate-state.json"))
        && serde_json::from_slice::<serde_json::Value>(&gate).ok()
            != Some(serde_json::json!("OPEN"))
    {
        failures.push(format!(
            "[{label}] the writer gate must not be left blocking (OPEN or absent): {:?}",
            String::from_utf8_lossy(&gate)
        ));
    }
    if std::fs::read(canonical_bc_index(root)).ok().as_deref() != Some(original) {
        failures.push(format!(
            "[{label}] the canonical BC-INDEX.md must be byte-identical"
        ));
    }
    if root
        .join(".factory/specs/behavioral-contracts/shards")
        .exists()
    {
        failures.push(format!(
            "[{label}] no canonical shard may have been created"
        ));
    }
}

/// BC-1.18.011 EC-067 / ADR-054 1.9 step 1: plan-build validates every `staging_path` and
/// `canonical_path` with the Decision 1.2 rule (via `Path::to_str`, no lossy conversion). A
/// project root containing LF, CR, ESC, another C0 or DEL makes EVERY plan path hostile: the
/// coordinator exits 2 with the exact `INTENT_LOG_VALUE_REJECTED` line (`contains_control_
/// character`), appends nothing, leaves no staged generation, no live txn and no blocking
/// gate. (NUL and over-4096-byte paths cannot exist on a real filesystem: they are covered at
/// module level by `validate_path` in `s2510_intent_log_values_test.rs`; a non-UTF-8 root is
/// covered below on Linux, where the filesystem permits it.)
#[test]
fn test_BC_1_18_011_EC067_plan_builder_rejects_lf_cr_nul_non_utf8_overlong_paths_nothing_staged() {
    let mut failures = Vec::new();
    for (label, prefix) in [
        ("LF", "proj\nlf"),
        ("CR", "proj\rcr"),
        ("ESC", "proj\u{1b}esc"),
        ("C0 0x01", "proj\u{1}c0"),
        ("DEL", "proj\u{7f}del"),
    ] {
        let dir = match tempfile::Builder::new().prefix(prefix).tempdir() {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!(
                    "[{label}] the platform cannot create the hostile directory: {e}"
                ));
                continue;
            }
        };
        setup_fresh_fixture(dir.path());
        let out = run_migrate(dir.path());
        let err = stderr_of(&out);
        let ok_lines: Vec<String> = plan_rejection_lines("contains_control_character")
            .into_iter()
            .map(|l| format!("{l}\n"))
            .collect();
        if out.status.code() != Some(2) || !out.stdout.is_empty() || !ok_lines.contains(&err) {
            failures.push(format!(
                "[{label}] expected exit 2, empty stdout and stderr exactly one of {ok_lines:?}; got \
                 exit {:?}, stdout {} bytes, stderr {err:?}",
                out.status.code(),
                out.stdout.len()
            ));
        }
        plan_rejection_state_failures(
            label,
            dir.path(),
            support::ORIGINAL_CONTENT.as_bytes(),
            &std::collections::BTreeMap::new(),
            &mut failures,
        );
    }
    assert_no_failures("plan-build rejection (hostile project root)", failures);
}

/// Same claim for a NON-UTF-8 project root (`OsStr::to_str() == None`, never converted
/// lossily): reason `not_utf8`. Linux only -- macOS filesystems reject non-UTF-8 names.
#[cfg(target_os = "linux")]
#[test]
fn test_BC_1_18_011_EC067_plan_builder_rejects_a_non_utf8_project_root_nothing_staged() {
    use std::os::unix::ffi::OsStrExt;
    let prefix = std::ffi::OsStr::from_bytes(b"proj\xff\xfe");
    let dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .expect("linux permits non-UTF-8 names");
    setup_fresh_fixture(dir.path());
    let out = run_migrate(dir.path());
    let err = stderr_of(&out);
    let ok_lines: Vec<String> = plan_rejection_lines("not_utf8")
        .into_iter()
        .map(|l| format!("{l}\n"))
        .collect();
    let mut failures = Vec::new();
    if out.status.code() != Some(2) || !out.stdout.is_empty() || !ok_lines.contains(&err) {
        failures.push(format!(
            "expected exit 2 and stderr exactly one of {ok_lines:?}; got exit {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    plan_rejection_state_failures(
        "non-UTF-8 root",
        dir.path(),
        support::ORIGINAL_CONTENT.as_bytes(),
        &std::collections::BTreeMap::new(),
        &mut failures,
    );
    assert_no_failures("plan-build rejection (non-UTF-8 root)", failures);
}

// ---------------------------------------------------------------------------
// crash-child harness (failpoints builds only), shape of
// `bc_1_18_011_b2_migration_crash_injection_test.rs`
// ---------------------------------------------------------------------------

#[cfg(feature = "failpoints")]
mod crash {
    use super::*;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ENV_BOUNDARY: &str = "S2510_CRASH_BOUNDARY";
    const ENV_OCCURRENCE: &str = "S2510_CRASH_OCCURRENCE";
    const ENV_CWD: &str = "S2510_CRASH_CWD";

    /// Child entrypoint: with the env vars set it aborts (no unwinding) at the Nth reach of the
    /// named `migration_fs::*` failpoint while running the REAL fresh migration. A plain
    /// `cargo test` run (no env) is a no-op.
    #[test]
    fn test_S2510_crash_child_entrypoint() {
        let Ok(boundary) = std::env::var(ENV_BOUNDARY) else {
            return;
        };
        let occurrence: usize = std::env::var(ENV_OCCURRENCE).unwrap().parse().unwrap();
        let cwd = PathBuf::from(std::env::var(ENV_CWD).unwrap());
        let counter = AtomicUsize::new(0);
        fail::cfg_callback(boundary, move || {
            if counter.fetch_add(1, Ordering::SeqCst) + 1 == occurrence {
                std::process::abort();
            }
        })
        .unwrap();
        let _ = factory_dispatcher::shard_manager::run_bc_index_migration(&cwd);
        std::process::exit(97); // boundary never reached the requested number of times
    }

    /// Spawn the child; returns true iff it died of SIGABRT at the boundary.
    pub fn crash_at(boundary: &str, occurrence: usize, cwd: &Path) -> bool {
        let exe = std::env::current_exe().unwrap();
        let mut cmd = Command::new(exe);
        cmd.arg("--exact")
            .arg("crash::test_S2510_crash_child_entrypoint")
            .arg("--test-threads=1")
            .arg("--nocapture")
            .env(ENV_BOUNDARY, boundary)
            .env(ENV_OCCURRENCE, occurrence.to_string())
            .env(ENV_CWD, cwd.as_os_str())
            .env("RUST_BACKTRACE", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let out = support::finish(cmd.spawn().unwrap(), 60);
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal() == Some(6)
        }
        #[cfg(not(unix))]
        {
            let _ = out;
            false
        }
    }

    /// A fresh migration crashed at the 7th `write_temp` of the run: the staged generation is
    /// complete and the INTENT batch is durable, the txn record is still STAGING with its
    /// `generation_id`, and the plan has not been persisted (doc table of the OBL-1 crash
    /// suite: occurrence 7 = the txn write carrying `pending_canonical_moves`).
    pub fn crashed_staging_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        setup_fresh_fixture(dir.path());
        assert!(
            crash_at("migration_fs::write_temp", 7, dir.path()),
            "fixture: the child must abort at the 7th write_temp"
        );
        let txn = read_txn(&dir.path().join(".factory/migration-state"));
        assert_eq!(
            txn["state"],
            serde_json::json!("STAGING"),
            "fixture: crashed while STAGING"
        );
        assert!(
            txn["generation_id"].is_string(),
            "fixture: a generation exists"
        );
        dir
    }

    pub fn gen_dir(root: &Path) -> PathBuf {
        let txn = read_txn(&root.join(".factory/migration-state"));
        root.join(".factory/migration-state")
            .join(format!("gen-{}", txn["generation_id"].as_str().unwrap()))
    }
}

/// BC-1.18.011 EC-067 (post-generation half) / ADR-054 1.9 step 1 and 3.1 correction C-2: a
/// plan path that is rejected ONCE THE GENERATION DIRECTORY EXISTS (here: a STAGING resume
/// whose staged shard directory holds a file named with an LF, which the resume recompute
/// turns into a plan entry) takes the pre-commit abort path -- txn ABORTED, THEN gate OPEN,
/// both COMPLETE before the line is printed -- and only then prints the exact
/// `INTENT_LOG_VALUE_REJECTED` line (exit 2). Nothing is committed.
#[cfg(feature = "failpoints")]
#[test]
fn test_BC_1_18_011_EC067_rejection_after_generation_dir_takes_the_pre_commit_abort_path_then_prints()
 {
    let _fp = fail_point_scope();
    let dir = crash::crashed_staging_root();
    let gen_dir = crash::gen_dir(dir.path());
    std::fs::write(
        gen_dir.join("shards").join("evil\nname.md"),
        "no bc rows here\n",
    )
    .unwrap();

    let logs_before = intent_log_sizes(dir.path());
    let out = run_migrate(dir.path());
    let err = stderr_of(&out);
    let ok_lines: Vec<String> = plan_rejection_lines("contains_control_character")
        .into_iter()
        .map(|l| format!("{l}\n"))
        .collect();
    let mut failures = Vec::new();
    if out.status.code() != Some(2) || !out.stdout.is_empty() || !ok_lines.contains(&err) {
        failures.push(format!(
            "expected exit 2, empty stdout and stderr exactly one of {ok_lines:?}; got exit {:?}, \
             stdout {} bytes, stderr {err:?}",
            out.status.code(),
            out.stdout.len()
        ));
    }
    let txn = read_txn(&dir.path().join(".factory/migration-state"));
    if txn["state"] != serde_json::json!("ABORTED") {
        failures.push(format!(
            "the txn must be ABORTED before the line is printed, it is {}",
            txn["state"]
        ));
    }
    plan_rejection_state_failures(
        "post-generation",
        dir.path(),
        support::ORIGINAL_CONTENT.as_bytes(),
        &logs_before,
        &mut failures,
    );
    match std::fs::read_to_string(dir.path().join(".factory/migration-state/gate-state.json")) {
        Ok(g) if g.trim() == "\"OPEN\"" => {}
        other => failures.push(format!(
            "the gate must be written OPEN (after the ABORTED txn write, before the line); got {other:?}"
        )),
    }
    assert_no_failures("post-generation rejection", failures);
}

/// ADR-054 3.1 correction C-2 / ADR-052 item 8 rule (b): the `INTENT_LOG_VALUE_REJECTED` line
/// is printed only AFTER the pre-commit abort's writes (txn ABORTED, then gate OPEN) have
/// COMPLETED; if one of them fails, that write's own `Io` error (exit 2) is the result and
/// the rejection line is NEVER produced.
///
/// Sweep: for k = 0, 1, 2, ... the first k `write_temp` calls of the recovery process succeed
/// and every later one fails (`k*off->return(permission_denied)`), over a fresh crashed STAGING
/// root whose staged shard directory holds a file named with an LF. Whatever k, the outcome
/// must be (A) an `Io` error, or (B) the rejection with the txn ABORTED and the gate OPEN --
/// never the rejection with a still-live txn or a blocking gate. And some k MUST reach (B),
/// otherwise the plan path was never rejected at all (the test would be vacuous).
#[cfg(feature = "failpoints")]
#[test]
fn test_BC_1_18_011_EC067_a_failed_abort_write_is_that_writes_own_io_never_the_rejection_line() {
    use factory_dispatcher::shard_manager::{BcIndexMigrationError, run_bc_index_migration};
    let _fp = fail_point_scope();
    let mut failures = Vec::new();
    let mut reached_rejection_at = None;
    for k in 0..12usize {
        let dir = crash::crashed_staging_root();
        let gen_dir = crash::gen_dir(dir.path());
        std::fs::write(
            gen_dir.join("shards").join("evil\nname.md"),
            "no bc rows here\n",
        )
        .unwrap();
        fail::cfg(
            "migration_fs::write_temp",
            &format!("{k}*off->return(permission_denied)"),
        )
        .unwrap();
        let result = run_bc_index_migration(dir.path());
        fail::cfg("migration_fs::write_temp", "off").unwrap();

        let ms = dir.path().join(".factory/migration-state");
        match &result {
            Err(BcIndexMigrationError::Io { .. }) => {}
            Err(e) if e.to_string().contains("INTENT_LOG_VALUE_REJECTED") => {
                reached_rejection_at.get_or_insert(k);
                let txn_state = read_txn(&ms)["state"].clone();
                let gate = std::fs::read_to_string(ms.join("gate-state.json")).unwrap_or_default();
                if txn_state != serde_json::json!("ABORTED") || gate.trim() != "\"OPEN\"" {
                    failures.push(format!(
                        "k={k}: the rejection line was produced while the abort had not completed \
                         (txn state {txn_state}, gate {gate:?}): the line must follow the abort writes"
                    ));
                }
            }
            other => failures.push(format!(
                "k={k}: expected an Io error or the rejection (neither happened: the run \
                 completed or failed differently), got {other:?}"
            )),
        }
        if reached_rejection_at.is_some() {
            break;
        }
    }
    if reached_rejection_at.is_none() {
        failures.push(
            "no k produced the INTENT_LOG_VALUE_REJECTED outcome: a plan path containing LF was \
             never rejected after the generation directory existed"
                .into(),
        );
    }
    assert_no_failures("failed abort write vs rejection line", failures);
}

/// ADR-054 1.9 step 2 on the STAGING-resume path: the INTENT batch is appended by the same
/// writer, so a torn tail left after a crash is truncated BEFORE the batch is appended. The
/// final log is a pure sequence of valid records whose first records are the originally
/// durable INTENTs, byte-identical, and the run completes.
#[cfg(feature = "failpoints")]
#[test]
fn test_BC_1_18_011_EC065_blackbox_staging_resume_repairs_a_torn_tail_before_the_intent_batch_append()
 {
    let _fp = fail_point_scope();
    let dir = crash::crashed_staging_root();
    let ms = dir.path().join(".factory/migration-state");
    let txn = read_txn(&ms);
    let log = ms.join(format!(
        "intent-{}.log",
        txn["generation_id"].as_str().unwrap()
    ));
    let original = std::fs::read(&log).expect("the crash left the durable INTENT batch");
    let (recs_before, valid_before) = parse_prefix(&original);
    assert_eq!(
        recs_before.len(),
        4,
        "fixture: four INTENT records are durable at the crash"
    );
    assert_eq!(
        valid_before,
        original.len(),
        "fixture: the durable batch is a clean sequence"
    );

    let mut torn = original.clone();
    torn.extend_from_slice(b"INTENT_LOG_RECORD_V1\ntxn_id=");
    std::fs::write(&log, &torn).unwrap();

    let out = run_migrate(dir.path());
    let mut failures = Vec::new();
    if out.status.code() != Some(0) {
        failures.push(format!(
            "STAGING resume over a torn tail must complete (exit 0); got {:?}, stderr {:?}",
            out.status.code(),
            stderr_of(&out)
        ));
    }
    let bytes = std::fs::read(&log).unwrap();
    let (recs, valid) = parse_prefix(&bytes);
    if valid != bytes.len() {
        failures.push(format!(
            "the final log must be a PURE record sequence: valid prefix {valid} of {}",
            bytes.len()
        ));
    }
    if bytes.len() < original.len() || bytes[..original.len()] != original[..] {
        failures
            .push("the originally durable INTENT records must be preserved byte-identical".into());
    }
    if recs.iter().filter(|p| p.rec.rtype == "DONE").count() != 4 {
        failures.push(format!(
            "expected four DONE records, got {} record(s) in all",
            recs.len()
        ));
    }
    assert_no_failures("STAGING-resume tail repair", failures);
}

// ===========================================================================
// AC-011 oracle twin: the coordinator's own log is a pure sequence, DONE copies INTENT
// ===========================================================================

/// AC-011 (module-independent twin of `s2510_converted_coordinator_log_test.rs`): after a full
/// successful `migrate-bc-index` run the log is a PURE sequence of strict records (the
/// reference parser consumes every byte), four INTENT then four DONE, and every `DONE` copies
/// `target_canonical`, `staging_path`, `expected_post_hash` and `expected_pre_state` from its
/// INTENT and carries the live txn's id and fencing generation. The fixture covers canonical
/// targets ABSENT at INTENT time (`MISSING`) and PRESENT (a 64-hex pre-state).
#[test]
fn test_BC_1_18_011_15c_full_run_log_is_a_pure_record_sequence_and_every_done_copies_its_intent() {
    let dir = tempfile::tempdir().unwrap();
    setup_fresh_fixture(dir.path());
    let out = run_migrate(dir.path());
    assert_eq!(
        out.status.code(),
        Some(0),
        "a fresh run completes; stderr {:?}",
        stderr_of(&out)
    );

    let ms = dir.path().join(".factory/migration-state");
    let txn = read_txn(&ms);
    let activation = txn["activation_id"].as_str().unwrap().to_string();
    let fencing = txn["fencing_generation"].as_u64().unwrap();
    let log = ms.join(format!(
        "intent-{}.log",
        txn["generation_id"].as_str().unwrap()
    ));
    let bytes = std::fs::read(&log).unwrap();
    let (recs, valid) = parse_prefix(&bytes);

    let mut failures = Vec::new();
    if valid != bytes.len() {
        failures.push(format!(
            "the log must be a pure sequence of strict ADR-054 records: valid prefix {valid} of {} bytes",
            bytes.len()
        ));
    }
    let intents: Vec<_> = recs.iter().filter(|p| p.rec.rtype == "INTENT").collect();
    let dones: Vec<_> = recs.iter().filter(|p| p.rec.rtype == "DONE").collect();
    if intents.len() != 4 || dones.len() != 4 {
        failures.push(format!(
            "expected 4 INTENT and 4 DONE, got {} and {}",
            intents.len(),
            dones.len()
        ));
    }
    if !intents.iter().any(|p| p.rec.pre == "MISSING") {
        failures.push("an ABSENT canonical target must record expected_pre_state=MISSING".into());
    }
    let orig_hash = support::sha256_hex(support::ORIGINAL_CONTENT.as_bytes());
    if !intents.iter().any(|p| p.rec.pre == orig_hash) {
        failures.push(
            "the PRESENT BC-INDEX.md must record sha256(original body) as expected_pre_state"
                .into(),
        );
    }
    for d in &dones {
        let Some(i) = intents.iter().rev().find(|i| i.rec.target == d.rec.target) else {
            failures.push(format!("DONE for {} has no INTENT", d.rec.target));
            continue;
        };
        if d.rec.staging != i.rec.staging || d.rec.post != i.rec.post || d.rec.pre != i.rec.pre {
            failures.push(format!(
                "DONE for {} must COPY staging_path, expected_post_hash and expected_pre_state \
                 from its INTENT (DONE pre={}, INTENT pre={})",
                d.rec.target, d.rec.pre, i.rec.pre
            ));
        }
        if d.rec.txn_id != activation || d.rec.fencing != fencing {
            failures.push(format!(
                "DONE for {} must carry the live txn id {activation} and fencing {fencing}; got {} / {}",
                d.rec.target, d.rec.txn_id, d.rec.fencing
            ));
        }
    }
    assert_no_failures("coordinator's own log", failures);
}
