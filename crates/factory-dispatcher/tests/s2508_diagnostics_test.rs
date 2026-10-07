// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 adversary pass-2 tests that observe `tracing` DIAGNOSTICS in-process
//! (a capturing `tracing::Subscriber` installed with `with_default` around the
//! production code path -- the real binary cannot be observed this way):
//!
//! * F-S2508-L2-008 (BC-1.18.013 Precondition 6(c) rule (5)): the reservation
//!   timestamp-fallback warn `reason` tokens;
//! * F-S2508-L2-007 (BC-1.18.013 Pre 6(c) terminal-record decision cell / EC-029):
//!   the foreign `migration_id` diagnostic is truncated to 64 chars with control
//!   characters escaped;
//! * F-S2508-L2-010 (BC-1.18.013 Pre 6(b)(i) spelling (ii)): the as-given
//!   `CLAUDE_PROJECT_DIR` lexical alias is ignored when empty or not absolute.
//!
//! # Stub surface (BC-5.38.001)
//! `as_given_factory_root_spelling` is added to `shard_manager::admission` as a
//! `todo!()` stub: the merged `executor.rs` inlines the `CLAUDE_PROJECT_DIR`
//! alias derivation (filtering only the EMPTY case), so the "not absolute"
//! rule is not unit-observable without extracting it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use factory_dispatcher::shard_manager::{
    AdmissionOutcome, ProtectedPathFamily, admit_protected_write, as_given_factory_root_spelling,
    drain_bc_index_writers,
};

// ---------------------------------------------------------------------------
// Capturing subscriber
// ---------------------------------------------------------------------------

type Fields = Vec<(String, String)>;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<Fields>>>);

struct FieldVisitor(Fields);
impl tracing::field::Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push((field.name().to_string(), value.to_string()));
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .push((field.name().to_string(), format!("{value:?}")));
    }
}

impl tracing::Subscriber for Capture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut v = FieldVisitor(Vec::new());
        event.record(&mut v);
        v.0.push(("target".to_string(), event.metadata().target().to_string()));
        v.0.push(("level".to_string(), event.metadata().level().to_string()));
        self.0.lock().unwrap().push(v.0);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

fn field<'a>(f: &'a Fields, name: &str) -> Option<&'a str> {
    f.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

fn captured<R>(f: impl FnOnce() -> R) -> (R, Vec<Fields>) {
    let cap = Capture::default();
    let r = tracing::subscriber::with_default(cap.clone(), f);
    let events = cap.0.lock().unwrap().clone();
    (r, events)
}

// ---------------------------------------------------------------------------
// F-S2508-L2-008 -- reservation timestamp fallback warn `reason` tokens
// ---------------------------------------------------------------------------

fn write_reservation(dir: &Path, body: &str, mtime: filetime::FileTime) {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("T1.reservation");
    std::fs::write(&path, body).unwrap();
    filetime::set_file_mtime(&path, mtime).unwrap();
}

/// BC-1.18.013 Precondition 6(c) rule (5) / ADR-052 v1.20 F-008: each fallback or
/// unknown-age outcome of the production drain GC emits a `tracing::warn!` whose
/// `reason` is exactly one of the five tokens.
#[test]
fn test_BC_1_18_013_PC6c_timestamp_fallback_warn_reason_tokens_captured() {
    let sys_now = std::time::SystemTime::now();
    let old = filetime::FileTime::from_system_time(sys_now - Duration::from_secs(4000));
    let fresh = filetime::FileTime::from_system_time(sys_now);
    let future = filetime::FileTime::from_system_time(sys_now + Duration::from_secs(5000));
    let pre_epoch = filetime::FileTime::from_unix_time(-1000, 0);
    let rfc = |d: i64| {
        (chrono::Utc::now() + chrono::Duration::seconds(d))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    };

    // (label, body, mtime, tokens that MUST be present, tokens that MUST be absent)
    type Vector = (
        &'static str,
        String,
        filetime::FileTime,
        Vec<&'static str>,
        Vec<&'static str>,
    );
    let vectors: Vec<Vector> = vec![
        (
            "created_at unparseable text",
            r#"{"created_at":"yesterday","tool_use_id":"T1"}"#.to_string(),
            old,
            vec!["created_at_unparseable"],
            vec!["created_at_pre_epoch", "created_at_future", "age_unknown"],
        ),
        (
            "created_at pre-epoch",
            r#"{"created_at":"1969-12-31T23:59:59Z","tool_use_id":"T1"}"#.to_string(),
            old,
            vec!["created_at_pre_epoch"],
            vec!["created_at_unparseable", "created_at_future", "age_unknown"],
        ),
        (
            "created_at beyond now+300",
            format!(r#"{{"created_at":"{}","tool_use_id":"T1"}}"#, rfc(330)),
            old,
            vec!["created_at_future"],
            vec![
                "created_at_unparseable",
                "created_at_pre_epoch",
                "age_unknown",
            ],
        ),
        (
            "created_at unusable + mtime in the future",
            r#"{"created_at":"nope","tool_use_id":"T1"}"#.to_string(),
            future,
            vec!["created_at_unparseable", "mtime_future"],
            vec!["age_unknown"],
        ),
        (
            "created_at unusable + mtime unavailable (pre-epoch)",
            r#"{"created_at":"nope","tool_use_id":"T1"}"#.to_string(),
            pre_epoch,
            vec!["created_at_unparseable", "age_unknown"],
            vec!["mtime_future"],
        ),
        (
            "created_at usable (within skew) => NO fallback warn",
            format!(r#"{{"created_at":"{}","tool_use_id":"T1"}}"#, rfc(-10)),
            fresh,
            vec![],
            vec![
                "created_at_unparseable",
                "created_at_pre_epoch",
                "created_at_future",
                "mtime_future",
                "age_unknown",
            ],
        ),
    ];

    let mut failures = Vec::new();
    for (label, body, mtime, must, must_not) in vectors {
        let dir = tempfile::tempdir().unwrap();
        let res_dir = dir.path().join("reservations");
        write_reservation(&res_dir, &body, mtime);
        let (_r, events) = captured(|| {
            let _ = drain_bc_index_writers(
                &res_dir,
                Duration::from_millis(60),
                Duration::from_secs(3600),
            );
        });
        let reasons: Vec<String> = events
            .iter()
            .filter(|e| field(e, "level") == Some("WARN"))
            .filter_map(|e| field(e, "reason").map(str::to_string))
            .collect();
        for token in &must {
            if !reasons.iter().any(|r| r == token) {
                failures.push(format!(
                    "[{label}] missing warn reason `{token}`; captured reasons: {reasons:?}"
                ));
            }
        }
        for token in &must_not {
            if reasons.iter().any(|r| r == token) {
                failures.push(format!(
                    "[{label}] unexpected warn reason `{token}`; captured reasons: {reasons:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L2-007 -- foreign migration_id diagnostic truncation + escaping
// ---------------------------------------------------------------------------

fn write_foreign_txn(ms: &Path, migration_id: &str) {
    std::fs::create_dir_all(ms.join("reservations")).unwrap();
    std::fs::write(ms.join("gate-state.json"), "\"LOCKED\"").unwrap();
    std::fs::write(ms.join("exclusive.lock"), b"").unwrap();
    let v = serde_json::json!({
        "txn_id": "txn-diag",
        "activation_id": "act-diag",
        "fencing_generation": 1,
        "state": "STAGING",
        "generation_id": "gen-1",
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": null,
        "pending_canonical_moves": [],
        "created_at": "2026-10-06T00:00:00Z",
        "updated_at": "2026-10-06T00:00:00Z",
        "migration_id": migration_id,
    });
    std::fs::write(
        ms.join("txn-act-diag.json"),
        serde_json::to_vec(&v).unwrap(),
    )
    .unwrap();
}

/// BC-1.18.013 Pre 6(c) terminal-record decision cell / EC-029: the raw foreign
/// `migration_id` appears ONLY in the `tracing::warn!` diagnostic, truncated to 64
/// characters with control characters escaped (never a raw newline / ESC / NUL).
#[test]
fn test_BC_1_18_013_EC029_foreign_migration_id_diagnostic_truncated_and_escaped() {
    let long_id = "x".repeat(1000);
    let ctl_id = "a\nb\u{1b}[31mc\0d".to_string();
    let long_ctl_id = format!("\u{1b}\n\0{}", "y".repeat(500));
    let mut failures = Vec::new();

    for (label, id) in [
        ("1,000-char id", long_id.clone()),
        ("id with \\n, ESC, NUL", ctl_id.clone()),
        ("long id with leading control chars", long_ctl_id.clone()),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let ms = dir.path().join("migration-state");
        write_foreign_txn(&ms, &id);
        let (outcome, events) =
            captured(|| admit_protected_write(&ms, Some("T1"), ProtectedPathFamily::Cycles));
        match outcome {
            Ok(AdmissionOutcome::Blocked { .. }) => {}
            other => failures.push(format!(
                "[{label}] foreign live txn must Block, got {other:?}"
            )),
        }
        // every captured event's `migration_id` field
        let ids: Vec<&str> = events
            .iter()
            .filter_map(|e| field(e, "migration_id"))
            .collect();
        if ids.is_empty() {
            failures.push(format!(
                "[{label}] no diagnostic carried a `migration_id` field"
            ));
        }
        for logged in ids {
            if logged.chars().count() > 64 {
                failures.push(format!(
                    "[{label}] logged migration_id is {} chars (> 64): {:?}",
                    logged.chars().count(),
                    &logged[..logged.len().min(80)]
                ));
            }
            if logged.chars().any(|c| c.is_control()) {
                failures.push(format!(
                    "[{label}] logged migration_id carries RAW control characters: {logged:?}"
                ));
            }
            if id.chars().any(|c| c.is_control()) && !logged.contains('\\') {
                failures.push(format!("[{label}] control characters must be ESCAPED (no backslash escape seen): {logged:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// F-S2508-L2-010 -- as-given CLAUDE_PROJECT_DIR spelling: ignored when empty or
// not absolute
// ---------------------------------------------------------------------------

/// BC-1.18.013 Pre 6(b)(i) spelling (ii): `<CLAUDE_PROJECT_DIR as given>/.factory`
/// (raw value, lexical normalization only) is a lexical alias ONLY when the raw
/// value is non-empty AND an absolute path.
#[test]
fn test_BC_1_18_013_PC6b_as_given_spelling_ignored_when_empty_or_not_absolute() {
    use std::ffi::OsStr;
    // ignored
    for raw in [
        "", "proj", "./proj", "../proj", "proj/sub", "~", "~/proj", ".",
    ] {
        assert_eq!(
            as_given_factory_root_spelling(OsStr::new(raw)),
            None,
            "raw CLAUDE_PROJECT_DIR {raw:?} (empty / not absolute) must add NO lexical alias"
        );
    }
    // accepted: absolute, lexically normalised, `/.factory` appended
    assert_eq!(
        as_given_factory_root_spelling(OsStr::new("/abs/proj")),
        Some(PathBuf::from("/abs/proj/.factory"))
    );
    assert_eq!(
        as_given_factory_root_spelling(OsStr::new("/abs//proj/./sub/../p2/")),
        Some(PathBuf::from("/abs/proj/p2/.factory")),
        "lexical normalization only (collapse //, ., ..), no symlink resolution"
    );
}
