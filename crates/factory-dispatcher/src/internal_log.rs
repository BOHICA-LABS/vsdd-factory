//! Always-on dispatcher self-telemetry log (S-1.7).
//!
//! Every dispatcher lifecycle event and every `internal.*` event the
//! dispatcher emits lands in a daily-rotated JSONL file at
//! `<log_dir>/dispatcher-internal-YYYY-MM-DD.jsonl`. This path exists
//! independent of `observability-config.toml` (Q6 Option B) so the
//! dispatcher remains debuggable even when all configured sinks are
//! down or misconfigured.
//!
//! Contract:
//! - Writes are best-effort. All I/O errors are swallowed; `write` never
//!   panics, never propagates.
//! - Files are daily-rotated by the timestamp on the event itself (not
//!   "now") so tests can write events dated in the past/future without
//!   reaching for a mocked clock.
//! - Retention is 30 days by default; `prune_old` walks the log dir and
//!   unlinks matching files older than the threshold. Also best-effort.
//! - Appends rely on `OpenOptions::append`; atomicity at PIPE_BUF size is
//!   OS-guaranteed. Lines larger than PIPE_BUF (4096 on Linux, 512 on
//!   macOS) may interleave across concurrent writers — acceptable per
//!   the story spec, and v1.0 runs one dispatcher process per hook
//!   invocation anyway.
//!
//! Plugin-lifecycle events (`plugin.loaded`, `plugin.invoked`, …) are
//! defined here as constants but the callsites land in S-1.5; sink-error
//! events land in S-1.8. This story ships structural plumbing +
//! `dispatcher.started` + `internal.dispatcher_error` only.

use chrono::{DateTime, Duration, Local, TimeZone};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Filename prefix for every rotated log. Matched during retention
/// scans.
const FILENAME_PREFIX: &str = "dispatcher-internal-";
/// Filename suffix for every rotated log.
const FILENAME_SUFFIX: &str = ".jsonl";
/// Filename prefix for DLQ rotated logs.
const DLQ_FILENAME_PREFIX: &str = "dead-letter-";
/// Filename suffix for DLQ rotated logs.
const DLQ_FILENAME_SUFFIX: &str = ".jsonl";
/// Default retention window in days. Callers should pass this (or
/// override) to [`InternalLog::prune_old`] at dispatcher startup.
pub const DEFAULT_RETENTION_DAYS: u32 = 30;

/// Default DLQ retention window in days (BC-1.06.011 PC1).
///
/// Independent of `DEFAULT_RETENTION_DAYS` (30 days for dispatcher logs).
/// Tests MUST assert against this constant, not a literal `7`.
/// Stub — value declared; `prune_old_dlq` method not yet implemented.
pub const INTERNAL_DLQ_DEFAULT_RETENTION_DAYS: u32 = 7;

/// Event type constant for successful DLQ writes (BC-3.07.003 PC3).
pub const INTERNAL_SINK_DLQ_WRITE: &str = "internal.sink_dlq_write";

/// Event type constant for DLQ write failures (BC-3.07.004 PC4).
pub const INTERNAL_SINK_DLQ_FAILURE: &str = "internal.sink_dlq_failure";

/// `schema_version` embedded in every event. Bumped when the event
/// shape changes in a non-backwards-compatible way.
pub const INTERNAL_EVENT_SCHEMA_VERSION: u32 = 1;

// --- Event type names ------------------------------------------------
//
// These are `pub const` strings rather than an enum so callers can
// attach arbitrary `fields` without ceremony, and downstream log
// pipelines (jq / OpenTelemetry attribute mapping) can filter by a
// stable string tag. Keep in lock-step with the catalog in the story
// spec.

pub const DISPATCHER_STARTED: &str = "dispatcher.started";
pub const DISPATCHER_SHUTTING_DOWN: &str = "dispatcher.shutting_down";
pub const PLUGIN_LOADED: &str = "plugin.loaded";
pub const PLUGIN_LOAD_FAILED: &str = "plugin.load_failed";
pub const PLUGIN_INVOKED: &str = "plugin.invoked";
pub const PLUGIN_COMPLETED: &str = "plugin.completed";
pub const PLUGIN_TIMEOUT: &str = "plugin.timeout";
pub const PLUGIN_CRASHED: &str = "plugin.crashed";
/// S-25.01 / BC-3.08.001 Event 8: emitted for every INDETERMINATE outcome
/// (both fail-closed and fail-open). Carries all 8 mandatory fields.
pub const PLUGIN_INDETERMINATE: &str = "plugin.indeterminate";
/// ADR-048 v1.1 / BC-3.08.001 Event 9: emitted whenever the
/// `.factory/unvalidated-mutation.marker` is cleared. The `trace_id` field
/// on this event equals the marker's stored `trace_id`, linking back to the
/// originating `plugin.indeterminate` (Event 8) that wrote the marker.
///
/// Mandatory fields: `type`, `trace_id`, `session_id`, `plugin_name`,
/// `artifact_path`, `clear_mode` ∈ {REVALIDATED|TTL_EXPIRED|OPERATOR_OVERRIDE},
/// `actor_type` ∈ {validator|deadman|operator}, `reason` (null unless
/// `clear_mode == OPERATOR_OVERRIDE`).
pub const PLUGIN_MARKER_CLEARED: &str = "marker.cleared";
/// ADR-048 v1.4 / BC-3.08.001 Event 10: emitted whenever
/// `write_indeterminate_marker` returns `Ok(())` — the positive, durable
/// creation record for the `.factory/unvalidated-mutation.marker` file.
/// Emitted ONLY on a confirmed successful write (never before the write is
/// attempted, never on a write failure), so an unmatched `marker.written`
/// with no subsequent `marker.cleared` is proof-by-construction that a
/// marker was durably written and has since become absent through a path
/// other than the three already-audited clears (REVALIDATED, TTL_EXPIRED,
/// SUPERSEDED) — i.e. a human out-of-band `rm` (OPERATOR_OVERRIDE).
/// `reconcile_raw_delete` scans for this event type (S-25.01 adversary pass
/// 6 F-P6-001 MEDIUM resolution).
///
/// Mandatory fields: `type`, `trace_id`, `session_id`, `plugin_name`,
/// `artifact_path`, `cause` ∈ {fuel|epoch|output-too-large}, `expires_at`.
pub const PLUGIN_MARKER_WRITTEN: &str = "marker.written";
pub const INTERNAL_CAPABILITY_DENIED: &str = "internal.capability_denied";
pub const INTERNAL_HOST_FUNCTION_PANIC: &str = "internal.host_function_panic";
pub const INTERNAL_SINK_ERROR: &str = "internal.sink_error";
pub const INTERNAL_SINK_QUEUE_FULL: &str = "internal.sink_queue_full";
pub const INTERNAL_SINK_CIRCUIT_OPENED: &str = "internal.sink_circuit_opened";
pub const INTERNAL_SINK_CIRCUIT_CLOSED: &str = "internal.sink_circuit_closed";
pub const INTERNAL_DISPATCHER_ERROR: &str = "internal.dispatcher_error";
/// Emitted at `debug` level when Router::submit silently drops an event
/// that fails a sink's RoutingFilter (BC-3.04.003 postcondition 3).
/// One entry per (event, filtering-sink) pair.
pub const INTERNAL_EVENT_FILTERED: &str = "internal.event_filtered";

/// Event type for absent-but-allowlisted file reads (AC-002 S-19.03; AC-001
/// S-19.06 T-007). Emitted by `read_file::prepare` and `read_prefix::prepare`
/// when the path is within the allow-list but the file does not exist on disk.
/// Named constant replaces bare literals across read_file.rs, read_prefix.rs
/// (D21, F-WG-002, S-19.09).
pub const INTERNAL_FILE_NOT_FOUND: &str = "internal.file_not_found";

/// Event type for async plugin drain-timer expiry (BC-3.08.001 v1.21 Event 5;
/// AC-002 S-19.05). Emitted by `emit_plugin_abandoned` when the async drain
/// timer fires with the plugin still in-flight. Named constant replaces bare
/// literals in emit_event.rs (D21, F-WG-002, S-19.09).
pub const PLUGIN_ABANDONED: &str = "plugin.abandoned";

/// One line in `dispatcher-internal-YYYY-MM-DD.jsonl`.
///
/// The top-level fields form the stable shape every log line carries;
/// event-specific extras go inside `fields`, flattened via
/// `#[serde(flatten)]` so the final JSON shape is flat (not `{ fields:
/// {...} }`).
#[derive(Debug, Clone, Serialize)]
pub struct InternalEvent {
    /// Event type — one of the `pub const` names above, e.g.
    /// `"dispatcher.started"`.
    #[serde(rename = "type")]
    pub type_: String,
    /// ISO-8601 timestamp with offset, e.g. `"2026-04-24T20:13:45-0500"`.
    pub ts: String,
    /// Epoch seconds corresponding to `ts`.
    pub ts_epoch: i64,
    /// Event-schema version; bump when the shape changes.
    pub schema_version: u32,
    /// Dispatcher's per-invocation trace id (v4 UUID), when known.
    /// Wire format uses `trace_id` per BC-3.08.001 v1.7 Invariant 5 + DI-017.
    /// The Rust field is kept as `dispatcher_trace_id` to minimise call-site churn;
    /// only the JSON key name changes.
    #[serde(rename = "trace_id", skip_serializing_if = "Option::is_none")]
    pub dispatcher_trace_id: Option<String>,
    /// Claude Code session id, when the event carries payload context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Plugin name for plugin-scoped events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    /// Plugin version for plugin-scoped events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
    /// Event-specific extras. Flattened into the top-level JSON object.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl InternalEvent {
    /// Construct a new event with the current local time. Optional
    /// fields default to `None` and `fields` default to empty; both are
    /// chainable via the builder-ish setters below.
    pub fn now(type_: impl Into<String>) -> Self {
        let now = Local::now();
        Self::with_ts(type_, now)
    }

    /// Construct an event at an explicit time. Used by tests that need
    /// to pin a particular date-of-rotation without reaching for a
    /// clock mock.
    pub fn with_ts<Tz: TimeZone>(type_: impl Into<String>, ts: DateTime<Tz>) -> Self
    where
        Tz::Offset: std::fmt::Display,
    {
        // `%z` emits `-0500`; matches the example in the story spec.
        let ts_str = ts.format("%Y-%m-%dT%H:%M:%S%z").to_string();
        let ts_epoch = ts.timestamp();
        Self {
            type_: type_.into(),
            ts: ts_str,
            ts_epoch,
            schema_version: INTERNAL_EVENT_SCHEMA_VERSION,
            dispatcher_trace_id: None,
            session_id: None,
            plugin_name: None,
            plugin_version: None,
            fields: Map::new(),
        }
    }

    /// Builder-style setter for the trace id.
    #[must_use]
    pub fn with_trace_id(mut self, id: impl Into<String>) -> Self {
        self.dispatcher_trace_id = Some(id.into());
        self
    }

    /// Builder-style setter for the Claude Code session id.
    #[must_use]
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Builder-style setter for the plugin name.
    #[must_use]
    pub fn with_plugin_name(mut self, name: impl Into<String>) -> Self {
        self.plugin_name = Some(name.into());
        self
    }

    /// Builder-style setter for the plugin version.
    #[must_use]
    pub fn with_plugin_version(mut self, version: impl Into<String>) -> Self {
        self.plugin_version = Some(version.into());
        self
    }

    /// Attach one extra field. Later calls overwrite earlier ones on
    /// key collision — the last-writer-wins semantics match the
    /// `serde_json::Map` contract.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Derive the date stamp used in the rotated filename. Uses the
    /// first 10 chars of `ts` (`YYYY-MM-DD`) so rotation matches the
    /// event's timestamp exactly — no "now" lookup inside `write`.
    fn date_stamp(&self) -> &str {
        // `ts` format is `YYYY-MM-DDThh:mm:ss±zzzz`; take the first 10
        // bytes. Safe for ASCII-only timestamps produced by `with_ts`.
        &self.ts[..10.min(self.ts.len())]
    }
}

/// Maximum number of unique error hashes held in `seen_errors` before
/// new insertions are paused (ADR-024 Decision 3 cap).
const SEEN_ERRORS_CAP: usize = 1024;

/// Best-effort JSONL writer for dispatcher self-telemetry.
///
/// Cheap to construct and `Clone`/share across threads. Each write
/// reopens the file (negligible vs dispatcher latency) and the
/// `seen_errors` set is held in an `Arc<Mutex<_>>` so clones share
/// the same dedup state within a process lifetime.
#[derive(Debug, Clone)]
pub struct InternalLog {
    log_dir: PathBuf,
    /// Per-session dedup set for `internal.dispatcher_error` events.
    /// Only those events are deduplicated; all others are written unconditionally.
    /// Shared via `Arc` so every `clone()` of `InternalLog` participates in the
    /// same dedup window (one process invocation = one dispatcher session).
    seen_errors: std::sync::Arc<Mutex<HashSet<u64>>>,
    /// One-shot flag so the mount-gate suppression (issue #206) warns exactly
    /// once per dispatcher session instead of once per event. Shared across
    /// clones like `seen_errors`.
    gate_warned: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Whether the #206 mount gate applies. `true` for resolved (level B–G)
    /// log dirs; `false` only when the operator explicitly chose the location
    /// via `VSDD_LOG_DIR` (level A) — an explicit override must not itself be
    /// overridden, even when it points at a `.factory/logs` path (the bats
    /// harness does exactly that with scratch fixtures). `FACTORY_ROOT`
    /// (level B) is deliberately NOT exempt: it resolves to
    /// `$FACTORY_ROOT/logs`, the racing shape the gate holds back.
    mount_gated: bool,
}

impl InternalLog {
    /// Build a writer rooted at `log_dir`. The directory is NOT created
    /// eagerly; `write` will `mkdir -p` on first use — and, by default, only
    /// once the `.factory` parent is a mounted worktree (see
    /// [`crate::log_dir::factory_mount_ready`], issue #206). Callers whose
    /// `log_dir` came from an explicit operator override should disable the
    /// gate via [`InternalLog::with_mount_gate`].
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            seen_errors: std::sync::Arc::new(Mutex::new(HashSet::new())),
            gate_warned: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            mount_gated: true,
        }
    }

    /// Set whether the #206 mount gate applies. Pass `false` only when the
    /// log dir came from an explicit `VSDD_LOG_DIR` (level A) override — see
    /// `crate::log_dir::mount_gate_exempt`.
    #[must_use]
    pub fn with_mount_gate(mut self, gated: bool) -> Self {
        self.mount_gated = gated;
        self
    }

    /// Best-effort append. Never panics, never propagates errors. On
    /// failure, emits one diagnostic line to stderr and returns.
    pub fn write(&self, event: &InternalEvent) {
        if let Err(e) = self.write_inner(event) {
            // Keep the fallback deliberately short — if even stderr is
            // broken we have bigger problems, and we do not want the
            // dispatcher to hang on a blocked tty.
            eprintln!(
                "factory-dispatcher: internal_log write failed ({}): {}",
                event.type_, e
            );
        }
    }

    fn write_inner(&self, event: &InternalEvent) -> std::io::Result<()> {
        // ADR-024 Decision 3: deduplicate `internal.dispatcher_error` events.
        // Other event types are written unconditionally.
        if event.type_ == INTERNAL_DISPATCHER_ERROR {
            let hash = dedup_hash_for(event);
            // Mutex::lock() failure → skip dedup and write (non-panicking contract).
            match self.seen_errors.lock() {
                Ok(mut set) => {
                    if set.contains(&hash) {
                        // Already seen — skip this write.
                        return Ok(());
                    }
                    // Cap at SEEN_ERRORS_CAP: stop inserting once full, but still write.
                    if set.len() < SEEN_ERRORS_CAP {
                        set.insert(hash);
                    }
                }
                Err(_) => {
                    // Poisoned mutex — write anyway to avoid silent loss.
                }
            }
        }

        // Issue #206: never create `.factory/logs` ahead of the worktree
        // mount. The dispatcher fires on every tool use, so an unconditional
        // mkdir here continuously recreated a plain `.factory/` during
        // `/factory-health` bootstrap — blocking the later `git worktree add`
        // (`fatal: '.factory' already exists`) and feeding the #203/#205
        // bootstrap-failure cluster. Suppression is a skip, not an error —
        // events during the bootstrap window are still observable via
        // `VSDD_SINK_FILE`. Only an explicit VSDD_LOG_DIR override is exempt
        // (`mount_gated == false`): the operator chose that exact path.
        if self.mount_gated && !crate::log_dir::factory_mount_ready(&self.log_dir) {
            if !self
                .gate_warned
                .swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                eprintln!(
                    "factory-dispatcher: internal_log suppressed — parent of {} is not a \
                     mounted .factory worktree yet (no .git entry); run /factory-health to \
                     mount it, or set VSDD_LOG_DIR to log elsewhere",
                    self.log_dir.display()
                );
            }
            return Ok(());
        }

        fs::create_dir_all(&self.log_dir)?;

        let filename = format!(
            "{FILENAME_PREFIX}{date}{FILENAME_SUFFIX}",
            date = event.date_stamp()
        );
        let path = self.log_dir.join(filename);

        // Serialize first so a JSON error (should be impossible given
        // the typed shape, but defense-in-depth) does not leave a
        // half-open file descriptor.
        let mut line = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');

        let mut f = OpenOptions::new().append(true).create(true).open(&path)?;
        // Single write_all so the stdlib retries short writes; full
        // atomicity up to PIPE_BUF is OS-guaranteed.
        f.write_all(line.as_bytes())?;
        Ok(())
    }

    /// Delete rotated logs whose mtime is older than `max_age_days`.
    /// Best-effort: individual failures are skipped, directory-missing
    /// is not an error.
    pub fn prune_old(&self, max_age_days: u32) {
        if let Err(e) = self.prune_old_inner(max_age_days) {
            eprintln!("factory-dispatcher: internal_log prune failed: {e}");
        }
    }

    fn prune_old_inner(&self, max_age_days: u32) -> std::io::Result<()> {
        scan_files_with_prefix(
            &self.log_dir,
            FILENAME_PREFIX,
            FILENAME_SUFFIX,
            max_age_days,
        )
        .map(|_| ())
    }

    /// Expose the log directory — integration tests use this to read
    /// files back; dispatcher main may log it for ops.
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    /// Delete DLQ files older than `max_age_days` from `<log_dir>/dlq/`.
    ///
    /// Only files matching `dead-letter-*.jsonl` are removed (BC-1.06.011 PC2).
    /// Independent of [`Self::prune_old`] — uses `max_age_days` directly
    /// (caller passes `INTERNAL_DLQ_DEFAULT_RETENTION_DAYS` or a config override).
    pub fn prune_old_dlq(&self, max_age_days: u32) {
        let dlq_dir = self.log_dir.join("dlq");
        if let Err(e) = scan_files_with_prefix(
            &dlq_dir,
            DLQ_FILENAME_PREFIX,
            DLQ_FILENAME_SUFFIX,
            max_age_days,
        ) {
            eprintln!("factory-dispatcher: internal_log prune_old_dlq failed: {e}");
        }
    }
}

/// Compute the dedup hash for an `internal.dispatcher_error` event.
///
/// **Hash key = `event.type_ + ":" + char-boundary-safe-truncation(message_string_value, N≈4096)`**
/// (ADR-024 Decision 3, amended by adversary pass-2 findings C2-CRIT-2/C2-HIGH-1).
/// Uses `std::hash::DefaultHasher` which is non-cryptographic but sufficient
/// for in-process dedup.
///
/// **Implementation notes (pass-1 M-5 + pass-2 C2-CRIT-2/C2-HIGH-1 fixes):**
///
/// 1. **String value, not JSON repr.** We extract the raw string value via
///    `Value::as_str()` rather than `Value::to_string()`.  `to_string()` produces
///    the JSON-serialized form (e.g. `"\"hello\""` with surrounding quotes), so
///    two distinct messages that share a 254-char prefix would have identical first
///    256 bytes in their JSON repr even though their raw values differ — a false
///    dedup.  Using `as_str()` hashes the actual string content.
///
/// 2. **Bounded raw value at N=4096 bytes with char-safe ceiling.**
///    (ADR-024 Decision 3, amended pass-2.)  Dispatcher error messages are short
///    in practice, but unbounded hashing costs O(message length) per invocation.
///    To bound cost and prevent pathological slowdowns on runaway callers that
///    append context to the message, we truncate to at most 4096 bytes of the
///    raw string value.  The truncation MUST be char-safe: find the largest byte
///    index ≤ N that is a valid UTF-8 char boundary (`str::floor_char_boundary`
///    or equivalent scan).  A naive byte slice at N panics when byte N falls
///    inside a multi-byte codepoint.  Accepted tradeoff: two messages that share
///    the same first N bytes but differ in their tails are treated as duplicates.
///
/// 3. **Non-panicking contract.** No index-based string slicing without boundary
///    checks; no `unwrap()`.  `as_str()` returns `None` for non-string JSON
///    values; we fall back to the empty string.  `DefaultHasher` and `HashSet`
///    operations cannot panic.
fn dedup_hash_for(event: &InternalEvent) -> u64 {
    let mut h = DefaultHasher::new();
    event.type_.hash(&mut h);
    ":".hash(&mut h);
    // Extract the raw string value of the `message` field.
    // - For Value::String(s): as_str() yields s directly (no JSON quotes).
    // - For other Value variants (unlikely for `message`): fall back to "".
    let msg_str = event
        .fields
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Bounded char-safe truncation at N=4096 (ADR-024 Decision 3, amended pass-2).
    // `floor_char_boundary` is stable since Rust 1.80; toolchain is 1.95.0.
    // This bounds hashing cost to O(4 KiB) and is guaranteed to land on a
    // valid UTF-8 char boundary (never panics for any string content).
    const N: usize = 4096;
    let safe_n = msg_str.floor_char_boundary(N.min(msg_str.len()));
    msg_str[..safe_n].hash(&mut h);
    h.finish()
}

/// Walk `dir` and delete files that match `prefix` + `suffix` (both must be present)
/// and whose mtime is older than `max_age_days`.
///
/// Returns the number of files deleted. Missing `dir` is silently treated as
/// "nothing to prune" (Ok(0)). Per-file errors (race with concurrent deletion,
/// metadata unavailable) are skipped rather than aborting the sweep.
fn scan_files_with_prefix(
    dir: &Path,
    prefix: &str,
    suffix: &str,
    max_age_days: u32,
) -> std::io::Result<usize> {
    let read_dir = match fs::read_dir(dir) {
        Ok(d) => d,
        // Missing dir on a fresh install is expected; nothing to prune.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };

    let cutoff = Local::now() - Duration::days(max_age_days as i64);
    let cutoff_epoch = cutoff.timestamp();
    let mut deleted = 0usize;

    for entry in read_dir.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) || !name.ends_with(suffix) {
            continue;
        }

        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let Ok(since_epoch) = modified.duration_since(std::time::UNIX_EPOCH) else {
            continue;
        };
        let file_epoch = since_epoch.as_secs() as i64;

        if file_epoch < cutoff_epoch {
            let _ = fs::remove_file(&path);
            deleted += 1;
        }
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::fs as stdfs;
    use std::io::BufRead;

    fn read_lines(path: &Path) -> Vec<String> {
        let f = stdfs::File::open(path).unwrap();
        std::io::BufReader::new(f)
            .lines()
            .map(|l| l.unwrap())
            .collect()
    }

    #[test]
    fn writes_jsonl_events_with_expected_shape() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 4, 24, 12, 0, 0).unwrap();

        for i in 0..10 {
            let event = InternalEvent::with_ts(DISPATCHER_STARTED, ts)
                .with_trace_id(format!("trace-{i}"))
                .with_field("iteration", i as i64);
            log.write(&event);
        }

        let expected = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-04-24{FILENAME_SUFFIX}"));
        assert!(expected.exists(), "expected log file at {expected:?}");

        let lines = read_lines(&expected);
        assert_eq!(lines.len(), 10);

        for (i, line) in lines.iter().enumerate() {
            let parsed: Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["type"], DISPATCHER_STARTED);
            assert_eq!(parsed["schema_version"], INTERNAL_EVENT_SCHEMA_VERSION);
            // BC-3.08.001 v1.7 Invariant 5: wire format uses "trace_id", not "dispatcher_trace_id".
            assert_eq!(parsed["trace_id"], format!("trace-{i}"));
            assert!(
                parsed["dispatcher_trace_id"].is_null(),
                "dispatcher_trace_id must not appear in wire output"
            );
            assert_eq!(parsed["iteration"], i as i64);
            assert!(parsed["ts"].as_str().unwrap().starts_with("2026-04-24"));
            assert!(parsed["ts_epoch"].is_i64());
        }
    }

    #[test]
    fn auto_creates_missing_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        // Two levels deeper than anything that exists.
        let nested = dir.path().join("a").join("b").join("c");
        let log = InternalLog::new(nested.clone());
        let ts = Local.with_ymd_and_hms(2026, 1, 15, 9, 30, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        let expected = nested.join(format!("{FILENAME_PREFIX}2026-01-15{FILENAME_SUFFIX}"));
        assert!(expected.exists());
        let lines = read_lines(&expected);
        assert_eq!(lines.len(), 1);
    }

    /// Issue #206: when the log dir is `.factory/logs` and `.factory` does not
    /// exist, `write` must NOT create it — a plain `.factory/` planted here
    /// forces the later `git worktree add .factory` to mount nested.
    #[test]
    fn gate_skips_write_when_factory_absent() {
        let dir = tempfile::tempdir().unwrap();
        let factory = dir.path().join(".factory");
        let log = InternalLog::new(factory.join("logs"));
        let ts = Local.with_ymd_and_hms(2026, 1, 15, 9, 30, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        assert!(
            !factory.exists(),
            ".factory must not be created by the internal log ahead of the mount"
        );
    }

    /// Issue #206/#203: `.factory` existing as a PLAIN directory (the
    /// onboard-before-health conflict state) must not gain a `logs/` child.
    #[test]
    fn gate_skips_write_into_plain_factory_dir() {
        let dir = tempfile::tempdir().unwrap();
        let factory = dir.path().join(".factory");
        stdfs::create_dir_all(&factory).unwrap();
        let log = InternalLog::new(factory.join("logs"));
        let ts = Local.with_ymd_and_hms(2026, 1, 15, 9, 30, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        assert!(
            !factory.join("logs").exists(),
            "plain .factory dir must not gain logs/ — that is the bootstrap race"
        );
    }

    /// Issue #206: an explicit override (`with_mount_gate(false)`, reached
    /// only via VSDD_LOG_DIR — level A) writes even into a plain `.factory`
    /// with no `.git` — the operator chose the location; the gate must not
    /// override the override. FACTORY_ROOT does NOT reach this path (#738).
    #[test]
    fn gate_bypassed_for_explicit_override() {
        let dir = tempfile::tempdir().unwrap();
        let factory = dir.path().join(".factory");
        stdfs::create_dir_all(&factory).unwrap();
        let log = InternalLog::new(factory.join("logs")).with_mount_gate(false);
        let ts = Local.with_ymd_and_hms(2026, 1, 15, 9, 30, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        let expected = factory
            .join("logs")
            .join(format!("{FILENAME_PREFIX}2026-01-15{FILENAME_SUFFIX}"));
        assert!(
            expected.exists(),
            "explicit override must write regardless of mount state"
        );
    }

    /// Issue #206 control: a mounted `.factory` (a `.git` FILE, the
    /// `git worktree add` shape) writes exactly as before.
    #[test]
    fn gate_allows_write_into_mounted_factory() {
        let dir = tempfile::tempdir().unwrap();
        let factory = dir.path().join(".factory");
        stdfs::create_dir_all(&factory).unwrap();
        stdfs::write(factory.join(".git"), "gitdir: ../.git/worktrees/.factory\n").unwrap();
        let log = InternalLog::new(factory.join("logs"));
        let ts = Local.with_ymd_and_hms(2026, 1, 15, 9, 30, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        let expected = factory
            .join("logs")
            .join(format!("{FILENAME_PREFIX}2026-01-15{FILENAME_SUFFIX}"));
        assert!(expected.exists(), "mounted .factory must write normally");
        assert_eq!(read_lines(&expected).len(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn silently_swallows_errors_on_read_only_dir() {
        // Create a dir, chmod 0o555 (read+execute, no write), verify
        // `write` does not panic.
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let ro = dir.path().join("ro");
        stdfs::create_dir_all(&ro).unwrap();
        let mut perms = stdfs::metadata(&ro).unwrap().permissions();
        perms.set_mode(0o555);
        stdfs::set_permissions(&ro, perms.clone()).unwrap();

        let log = InternalLog::new(ro.clone());
        let ts = Local.with_ymd_and_hms(2026, 4, 24, 12, 0, 0).unwrap();

        // Must not panic; returns ().
        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, ts));

        // Restore perms so tempdir cleanup works.
        perms.set_mode(0o755);
        stdfs::set_permissions(&ro, perms).unwrap();
    }

    #[test]
    fn daily_rotation_writes_separate_files_per_date() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let day1 = Local.with_ymd_and_hms(2026, 4, 23, 23, 59, 0).unwrap();
        let day2 = Local.with_ymd_and_hms(2026, 4, 24, 0, 1, 0).unwrap();

        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, day1));
        log.write(&InternalEvent::with_ts(DISPATCHER_STARTED, day2));
        log.write(&InternalEvent::with_ts(PLUGIN_INVOKED, day2));

        let f1 = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-04-23{FILENAME_SUFFIX}"));
        let f2 = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-04-24{FILENAME_SUFFIX}"));
        assert!(f1.exists(), "expected {f1:?}");
        assert!(f2.exists(), "expected {f2:?}");
        assert_eq!(read_lines(&f1).len(), 1);
        assert_eq!(read_lines(&f2).len(), 2);
    }

    #[test]
    fn prune_removes_files_older_than_max_age() {
        use filetime::{FileTime, set_file_mtime};

        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());

        let now = std::time::SystemTime::now();
        let now_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;

        // (age_in_days, expected_to_survive_30_day_prune)
        // Avoid the exact 30-day boundary — the test grabs `now_epoch`
        // before calling `prune_old`, but `prune_old_inner` re-evaluates
        // `Local::now()` later, so any sub-second drift between the two
        // crosses a file at exactly the boundary in or out. Windows
        // happened to flake this in CI run 24935133658; Unix never had,
        // but the same race exists. Use 29 / 31 to bracket the boundary
        // without sitting on it.
        let fixtures: &[(i64, bool)] =
            &[(1, true), (10, true), (29, true), (31, false), (60, false)];

        let day_secs: i64 = 86_400;
        let mut paths: Vec<(PathBuf, bool)> = Vec::new();
        for (i, (age, survives)) in fixtures.iter().enumerate() {
            let name = format!(
                "{FILENAME_PREFIX}2026-01-{:02}{FILENAME_SUFFIX}",
                i.saturating_add(1)
            );
            let p = dir.path().join(&name);
            stdfs::write(&p, b"{}\n").unwrap();
            let mtime = FileTime::from_unix_time(now_epoch - age * day_secs, 0);
            set_file_mtime(&p, mtime).unwrap();
            paths.push((p, *survives));
        }

        // Also drop an unrelated file to assert the matcher does not
        // sweep non-internal-log files even if they're ancient.
        let other = dir.path().join("unrelated-2020-01-01.jsonl");
        stdfs::write(&other, b"keep me").unwrap();
        set_file_mtime(
            &other,
            FileTime::from_unix_time(now_epoch - 365 * day_secs, 0),
        )
        .unwrap();

        log.prune_old(30);

        for (p, survives) in &paths {
            assert_eq!(
                p.exists(),
                *survives,
                "file {p:?} expected survives={survives}"
            );
        }
        assert!(other.exists(), "pruner must not touch non-matching names");
    }

    #[test]
    fn prune_is_no_op_when_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("never-created");
        let log = InternalLog::new(missing);
        // Must not panic.
        log.prune_old(30);
    }

    // ── AC-009: prune_old_dlq covers DLQ files (BC-1.06.011) ─────────────────

    /// AC-009 — Traces to: BC-1.06.011 PC1 + PC2.
    ///
    /// `prune_old_dlq` must:
    ///   1. Use `INTERNAL_DLQ_DEFAULT_RETENTION_DAYS` (= 7) as its default, NOT 30.
    ///   2. Remove DLQ files matching `dead-letter-*.jsonl` older than the threshold.
    ///   3. Leave DLQ files newer than the threshold intact.
    ///   4. Leave dispatcher-internal log files untouched (independent prune scope).
    ///
    /// RED gate: `prune_old_dlq` is `unimplemented!()`.
    #[test]
    fn test_BC_1_06_011_prune_old_extends_to_dlq_files() {
        use filetime::{FileTime, set_file_mtime};
        use std::fs as stdfs;

        let dir = tempfile::tempdir().unwrap();
        let dlq_dir = dir.path().join("dlq");
        stdfs::create_dir_all(&dlq_dir).unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());

        let now = std::time::SystemTime::now();
        let now_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let day_secs: i64 = 86_400;

        // Create DLQ files at 3 ages relative to INTERNAL_DLQ_DEFAULT_RETENTION_DAYS (7).
        // Ages chosen with margin away from the 7-day boundary to avoid race conditions.
        // (age_days, expected_to_survive_7_day_prune)
        let dlq_fixtures: &[(&str, i64, bool)] = &[
            ("dead-letter-my-sink-2026-01-01.jsonl", 1, true), // 1 day old → survives
            ("dead-letter-my-sink-2026-01-02.jsonl", 6, true), // 6 days old → survives
            ("dead-letter-my-sink-2026-01-03.jsonl", 9, false), // 9 days old → pruned
            ("dead-letter-my-sink-2026-01-04.jsonl", 30, false), // 30 days old → pruned
        ];

        let mut paths: Vec<(std::path::PathBuf, bool)> = Vec::new();
        for (name, age, survives) in dlq_fixtures {
            let p = dlq_dir.join(name);
            stdfs::write(&p, b"{}\n").unwrap();
            let mtime = FileTime::from_unix_time(now_epoch - age * day_secs, 0);
            set_file_mtime(&p, mtime).unwrap();
            paths.push((p, *survives));
        }

        // A dispatcher-internal log file in the parent dir must NOT be touched.
        let internal_log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2020-01-01{FILENAME_SUFFIX}"));
        stdfs::write(&internal_log_file, b"{}\n").unwrap();
        set_file_mtime(
            &internal_log_file,
            FileTime::from_unix_time(now_epoch - 365 * day_secs, 0),
        )
        .unwrap();

        // Use the constant — tests MUST NOT use a literal '7'.
        log.prune_old_dlq(INTERNAL_DLQ_DEFAULT_RETENTION_DAYS);

        for (p, survives) in &paths {
            assert_eq!(
                p.exists(),
                *survives,
                "BC-1.06.011: DLQ file {:?} expected survives={}",
                p,
                survives
            );
        }

        // Dispatcher log must be untouched by prune_old_dlq.
        assert!(
            internal_log_file.exists(),
            "BC-1.06.011: dispatcher internal log must NOT be pruned by prune_old_dlq"
        );
    }

    /// BC-1.06.011 PC1: DLQ retention is independent — `prune_old_dlq` uses
    /// `INTERNAL_DLQ_DEFAULT_RETENTION_DAYS` (7), NOT `DEFAULT_RETENTION_DAYS` (30).
    ///
    /// Assert the constant values are distinct to catch a regression where
    /// someone aliases them.
    ///
    /// RED gate: this test PASSES in RED state (pure constant assertion).
    /// Intentionally kept as a guard: the constant MUST be 7, never 30.
    #[test]
    fn test_BC_1_06_011_dlq_retention_constant_is_7_independent_of_dispatcher_retention() {
        assert_eq!(
            INTERNAL_DLQ_DEFAULT_RETENTION_DAYS, 7,
            "BC-1.06.011: INTERNAL_DLQ_DEFAULT_RETENTION_DAYS must be 7"
        );
        assert_ne!(
            INTERNAL_DLQ_DEFAULT_RETENTION_DAYS, DEFAULT_RETENTION_DAYS,
            "BC-1.06.011: DLQ retention must be independent of dispatcher log retention (30)"
        );
    }

    #[test]
    fn event_fields_flatten_to_top_level() {
        let ts = Local.with_ymd_and_hms(2026, 4, 24, 12, 0, 0).unwrap();
        let e = InternalEvent::with_ts(PLUGIN_LOADED, ts)
            .with_plugin_name("capture-commit-activity")
            .with_plugin_version("0.1.0")
            .with_field("wasm_path", "plugins/capture.wasm");
        let s = serde_json::to_string(&e).unwrap();
        let parsed: Value = serde_json::from_str(&s).unwrap();
        // No nested `fields` object: extras are flattened.
        assert!(parsed.get("fields").is_none());
        assert_eq!(parsed["wasm_path"], "plugins/capture.wasm");
        assert_eq!(parsed["plugin_name"], "capture-commit-activity");
        assert_eq!(parsed["plugin_version"], "0.1.0");
    }

    #[test]
    fn skips_serializing_none_optional_fields() {
        let ts = Local.with_ymd_and_hms(2026, 4, 24, 12, 0, 0).unwrap();
        let e = InternalEvent::with_ts(DISPATCHER_STARTED, ts);
        let s = serde_json::to_string(&e).unwrap();
        let parsed: Value = serde_json::from_str(&s).unwrap();
        assert!(parsed.get("dispatcher_trace_id").is_none());
        assert!(parsed.get("session_id").is_none());
        assert!(parsed.get("plugin_name").is_none());
        assert!(parsed.get("plugin_version").is_none());
    }

    // -----------------------------------------------------------------------
    // Red Gate test — issue #130 / ADR-024 Decision 3
    //
    // Writing the same `internal.dispatcher_error` message N times through
    // the public API must produce exactly ONE line in the log file (dedup).
    //
    // CURRENTLY FAILS: `InternalLog` has no `seen_errors` field and no
    // dedup logic.  Writing 5× produces 5 lines.
    //
    // The test uses a fixed timestamp so all writes land in the same JSONL
    // file and the line-count assertion is deterministic.
    //
    // ADR-024 Decision 3 spec (amended v1.2):
    //   - Dedup key = hash(type_ + ":" + char-boundary-safe
    //     bounded_prefix(message string value via Value::as_str(), N=4096)).
    //   - Cap at 1024 entries.
    //   - Non-dispatcher_error events are written unconditionally.
    //   - Mutex::lock() failure → log anyway (non-panicking contract).
    // -----------------------------------------------------------------------
    #[test]
    fn test_BC_2_06_001_internal_log_dedup_same_message() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());

        // Pin a date so all events land in the same file.
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 10, 0, 0).unwrap();
        let msg = "$CLAUDE_PLUGIN_ROOT is not set or empty — hook registry and resolver registry paths unresolvable";

        // Write the SAME dispatcher_error message 5 times.
        for _ in 0..5 {
            let event =
                InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg);
            log.write(&event);
        }

        // Write a non-dispatcher_error event once — it must NOT be deduped.
        let other_event = InternalEvent::with_ts(DISPATCHER_STARTED, ts).with_field("pid", 42_i64);
        log.write(&other_event);

        let expected_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        assert!(
            expected_file.exists(),
            "log file must be created: {expected_file:?}"
        );

        let lines = read_lines(&expected_file);

        // The 5 identical dispatcher_error events must be deduped to 1.
        // The 1 dispatcher.started event must pass through unconditionally.
        // Total expected: 2 lines.
        assert_eq!(
            lines.len(),
            2,
            "ADR-024 Decision 3: 5 identical dispatcher_error writes must be deduped to 1 line; \
             got {} lines (dedup not yet implemented)",
            lines.len()
        );

        // Verify the surviving lines are the right types.
        let parsed_0: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        let parsed_1: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();

        assert_eq!(
            parsed_0["type"], INTERNAL_DISPATCHER_ERROR,
            "first surviving line must be the dispatcher_error"
        );
        assert_eq!(
            parsed_1["type"], DISPATCHER_STARTED,
            "second line must be the non-deduped dispatcher.started event"
        );
    }

    // -----------------------------------------------------------------------
    // RED GATE: adversary pass-1 finding M-5 — dedup_hash_for char-boundary panic
    //
    // ADR-024 Decision 3 spec says "first 256 bytes (byte-level, not char-level)".
    // The current implementation does `s[..s.len().min(256)]` which slices at
    // byte 256. If byte 256 lands inside a multi-byte UTF-8 codepoint (e.g.
    // a 2-byte accented character whose first byte is at index 255), Rust will
    // PANIC at the slice boundary.
    //
    // Contract: `write` must NEVER panic regardless of the `message` field content.
    // This test verifies the non-panicking contract on a multi-byte UTF-8 path
    // long enough that byte 256 falls mid-codepoint.
    //
    // Construction: We need a string whose JSON serialization is >256 bytes
    // and has a multi-byte codepoint straddling the 256-byte boundary.
    //
    // JSON value of a string `msg` serializes to `"<msg>"` (with surrounding
    // quotes added by `v.to_string()` via serde_json::Value::String).
    // So `v.to_string().len()` = msg.len() + 2 (for the quotes), minus any
    // escape sequences.
    //
    // We use a path prefix of 254 ASCII chars, then a 2-byte UTF-8 codepoint
    // (e.g. 'é' = 0xC3 0xA9).  After JSON serialization:
    //   `"<254 ASCII chars><é>"` → len = 1 + 254 + 2 + 1 = 258 bytes.
    //   Byte 256 (0-indexed) = second byte of 'é' (0xA9), inside the codepoint.
    //   Slicing `s[..256]` splits the codepoint → PANIC.
    //
    // CURRENTLY PANICS — that is the RED. The fix must use
    // `s.floor_char_boundary(256)` (Rust 1.65+ nightly; available in stable via
    // manual scan) or `s.char_indices().take_while(|(i,_)| *i < 256)` to find
    // the largest valid boundary ≤ 256 bytes.
    // -----------------------------------------------------------------------

    /// M-5 adversary finding: `write` must not panic when `message` JSON value
    /// has a multi-byte UTF-8 codepoint straddling byte index 256.
    ///
    /// RED: currently panics with `byte index 256 is not a char boundary`.
    #[test]
    fn test_BC_2_06_001_dedup_no_panic_on_multibyte_utf8_at_256_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 10, 0, 0).unwrap();

        // Build a message such that the JSON-serialized value has a multi-byte
        // UTF-8 codepoint spanning byte 256.
        //
        // `serde_json::Value::String(s).to_string()` produces `"<s>"` (with
        // surrounding double-quotes, 2 extra bytes).  We need the boundary at
        // byte 256 of that serialized form.
        //
        // Target: place 'é' (2 bytes: 0xC3 0xA9) at positions 255-256 of the
        // serialized string. Serialized prefix = `"` + 254 ASCII chars = 255
        // bytes. Then 'é' starts at byte 255, meaning byte 256 is the second
        // byte (0xA9) — inside the codepoint.
        //
        // So msg = 254 ASCII chars + 'é' + enough filler to exceed 256.
        let prefix = "A".repeat(254);
        let msg = format!("{prefix}é/tmp/some/path/that/keeps/going");

        // Sanity-check: the JSON serialized value straddles byte 256.
        let json_val = serde_json::Value::String(msg.clone()).to_string();
        assert!(
            json_val.len() > 256,
            "test setup: JSON value must be >256 bytes, got {}",
            json_val.len()
        );
        // Byte 256 should not be a char boundary (it's mid-codepoint).
        // We assert the string is NOT valid UTF-8 if sliced at 256 bytes.
        // Using `is_char_boundary` to verify test construction.
        assert!(
            !json_val.is_char_boundary(256),
            "test setup: byte 256 must NOT be a char boundary (required for RED test)"
        );

        // This call MUST NOT panic (non-panicking write contract).
        // The dedup hash computation calls `s[..s.len().min(256)]` which panics
        // when byte 256 is not a char boundary.
        let event =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg);
        log.write(&event); // RED: panics here under current implementation

        // If we reach here, the non-panicking contract was upheld.
        // The event should have been written (first occurrence, no prior hash).
        let log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        assert!(
            log_file.exists(),
            "log file must exist after successful write"
        );
    }

    // -----------------------------------------------------------------------
    // REGRESSION GUARD: adversary pass-2 finding C2-MED-2 — dedup doc-block
    // recap must reflect the amended ADR-024 Decision 3 contract.
    //
    // The pass-1 doc-block in dedup_hash_for() cited "first 256 bytes of
    // message JSON value". ADR-024 Decision 3 was amended in pass-2 to use
    // "bounded raw Value::as_str() at 4096-byte char-safe ceiling". The
    // implementation was already updated (dedup_hash_for uses as_str() +
    // full hash). The implementer must complete the pass-2 amendment by
    // adding the 4096-byte char-safe truncation bound.
    //
    // This constant captures the intended N for test assertions so tests
    // do not embed a magic literal. When the implementer adds truncation,
    // the tests here remain in sync.
    // -----------------------------------------------------------------------

    /// Dedup bound N from ADR-024 Decision 3 (amended pass-2).
    /// The implementer must add truncation at this boundary in `dedup_hash_for`.
    /// Tests reference this constant — do NOT substitute a magic literal.
    const DEDUP_HASH_N: usize = 4096;

    // -----------------------------------------------------------------------
    // REGRESSION GUARD (C2-MED-2 / C2-HIGH-1): two DISTINCT messages that
    // differ at an EARLY byte (well within N) must BOTH be logged.
    //
    // This passes today with both the full-hash implementation AND the
    // bounded-N implementation (early difference → distinct hashes under
    // both strategies).  It is a regression guard: the implementer must not
    // accidentally collapse distinct messages into the same N-prefix bucket.
    //
    // Contract: two dispatcher_error events with messages that differ at
    // byte 20 → both are written (2 lines in the log file).
    // -----------------------------------------------------------------------

    /// C2-MED-2 / C2-HIGH-1 regression guard: messages differing at an early
    /// byte (byte 20, << N=4096) are not deduplicated.
    ///
    /// Expected: PASSES against current HEAD and after bounded-N implementation.
    #[test]
    fn test_BC_2_06_001_dedup_distinct_messages_early_diff_both_logged() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 14, 0, 0).unwrap();

        // Two messages: 20 identical ASCII chars, then differ at byte 20.
        // Both are short (< N) so both full-hash and bounded-N produce
        // distinct hashes.
        let shared_20 = "A".repeat(20);
        let msg_a = format!("{shared_20}ALPHA_suffix_one");
        let msg_b = format!("{shared_20}BETA__suffix_two");
        // Verify test setup: byte 20 differs.
        assert_ne!(
            msg_a.as_bytes()[20],
            msg_b.as_bytes()[20],
            "test setup: byte 20 must differ between msg_a and msg_b"
        );

        let event_a =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_a);
        let event_b =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_b);
        log.write(&event_a);
        log.write(&event_b);

        let log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        let lines = read_lines(&log_file);
        assert_eq!(
            lines.len(),
            2,
            "C2-MED-2 / C2-HIGH-1: two distinct dispatcher_error events differing at byte 20 \
             must both be logged (got {} lines)",
            lines.len()
        );
    }

    // -----------------------------------------------------------------------
    // REGRESSION GUARD (C2-HIGH-1): oversized message (>> N=4096) containing
    // multibyte UTF-8 near and beyond byte N, written twice → dedup to 1 line,
    // no panic.
    //
    // Current behavior (full-hash): passes — the two identical writes produce
    // the same hash and the second is skipped.  No panic since as_str() +
    // full hash never slices at a byte boundary.
    //
    // After implementer adds bounded truncation at N=4096: must also pass —
    // the truncation is char-safe so the multibyte codepoint near N does not
    // cause a panic, and the two identical truncated prefixes produce the
    // same hash (dedup to 1).
    //
    // If bounded truncation is added naively (byte slice at N with no char
    // safety), this test becomes RED (panic).  That is the point: this test
    // guards against the naive panic regression.
    // -----------------------------------------------------------------------

    /// C2-HIGH-1 regression guard: oversized message with multibyte UTF-8 near
    /// byte N written twice → dedup to 1 line, MUST NOT panic.
    ///
    /// Expected: PASSES against current HEAD and after correct bounded-N
    /// implementation.  Becomes RED if the implementer introduces a naive
    /// byte-slice at N that splits a multibyte codepoint.
    #[test]
    fn test_BC_2_06_001_dedup_oversized_multibyte_near_N_no_panic_deduped() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 15, 0, 0).unwrap();

        // Build a message that:
        //   1. Has a multibyte UTF-8 codepoint ('é' = 0xC3 0xA9, 2 bytes)
        //      placed such that its FIRST byte lands at index DEDUP_HASH_N-1
        //      (i.e., byte DEDUP_HASH_N is the second byte, inside the codepoint).
        //   2. Continues for ~1 MB past the multibyte codepoint (oversized).
        //
        // Placement: N-1 ASCII chars, then 'é', then ~1MB of 'Z' chars.
        // Raw string bytes: byte 0..(N-1) = ASCII, byte (N-1) = 0xC3,
        // byte N = 0xA9 (inside 'é' codepoint).
        //
        // A naive `msg[..N]` byte-slice would split 'é' and panic.
        // A char-safe implementation finds the last char boundary <= N and
        // truncates there, landing at byte N-1 (just before 'é').
        let oversized_suffix = "Z".repeat(1_000_000);
        let msg = format!(
            "{}{}{}",
            "A".repeat(DEDUP_HASH_N - 1),
            'é',
            oversized_suffix
        );

        // Verify the multibyte codepoint straddles the N boundary.
        assert_eq!(
            msg.as_bytes()[DEDUP_HASH_N - 1],
            0xC3,
            "test setup: byte N-1 must be first byte of 'é' (0xC3)"
        );
        assert_eq!(
            msg.as_bytes()[DEDUP_HASH_N],
            0xA9,
            "test setup: byte N must be second byte of 'é' (0xA9)"
        );
        assert!(
            !msg.is_char_boundary(DEDUP_HASH_N),
            "test setup: byte N must NOT be a char boundary"
        );

        // Write the same oversized message twice.
        for _ in 0..2 {
            let event = InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts)
                .with_field("message", msg.clone());
            log.write(&event); // Must not panic under any implementation.
        }

        let log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        let lines = read_lines(&log_file);
        assert_eq!(
            lines.len(),
            1,
            "C2-HIGH-1: two identical oversized dispatcher_error writes must dedup to 1 line \
             (got {} lines)",
            lines.len()
        );
    }

    // -----------------------------------------------------------------------
    // RED GATE (C2-MED-2 + C2-HIGH-1): two messages IDENTICAL for the first
    // N+100 bytes but differing only AFTER byte N → dedup to 1 line.
    //
    // This documents the accepted bounded-prefix tradeoff: once bounded
    // truncation is in place, two messages that share the same N-byte prefix
    // but differ in their tails are treated as duplicates.
    //
    // ADR-024 Decision 3 (amended): hash key = type + ":" +
    // char-boundary-safe-truncation(message string value, N≈4096).
    //
    // Current behavior (full-hash, no truncation): the two messages have
    // DISTINCT hashes (they differ after byte N), so BOTH are logged → 2 lines.
    // This test FAILS (RED) against the current full-hash implementation.
    //
    // After the implementer adds bounded truncation at N=DEDUP_HASH_N:
    // both messages produce the SAME truncated prefix → same hash → dedup
    // to 1 line. The test turns GREEN.
    //
    // This is the primary RED gate driving the pass-2 bounded-truncation work.
    // -----------------------------------------------------------------------

    /// C2-MED-2 / C2-HIGH-1: two messages identical for first N bytes, differing
    /// only after byte N → deduplicated to 1 line (bounded-prefix tradeoff).
    ///
    /// RED: currently 2 lines (full-hash distinguishes the messages).
    /// GREEN after: bounded truncation at N=DEDUP_HASH_N collapses them.
    #[test]
    fn test_BC_2_06_001_dedup_messages_differing_only_after_N_deduped_to_1() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 16, 0, 0).unwrap();

        // Construct two messages:
        //   msg_a: DEDUP_HASH_N ASCII chars + "TAIL_ALPHA_UNIQUE"
        //   msg_b: DEDUP_HASH_N ASCII chars + "TAIL_BETA_DIFFERENT"
        //
        // Both share the same N-char prefix; they differ only in the tail
        // that lies beyond byte N.  Under bounded-N truncation both hash to
        // the same N-byte prefix → dedup to 1 line.
        let shared_prefix = "C".repeat(DEDUP_HASH_N);
        let msg_a = format!("{shared_prefix}TAIL_ALPHA_UNIQUE");
        let msg_b = format!("{shared_prefix}TAIL_BETA_DIFFERENT");

        // Sanity: messages share exact N-byte prefix and differ after.
        assert_eq!(
            &msg_a.as_bytes()[..DEDUP_HASH_N],
            &msg_b.as_bytes()[..DEDUP_HASH_N],
            "test setup: first N bytes must be identical"
        );
        assert_ne!(msg_a, msg_b, "test setup: full messages must be distinct");

        let event_a =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_a);
        let event_b =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_b);
        log.write(&event_a);
        log.write(&event_b);

        let log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        let lines = read_lines(&log_file);
        assert_eq!(
            lines.len(),
            1,
            "C2-MED-2 / C2-HIGH-1 (bounded-prefix tradeoff): two dispatcher_error messages \
             that share the first {} bytes but differ only in the tail must be deduplicated \
             to 1 line under bounded-prefix hashing (got {} lines — full-hash implementation \
             is RED here; add char-safe truncation at N={} to turn GREEN)",
            DEDUP_HASH_N,
            lines.len(),
            DEDUP_HASH_N,
        );
    }

    // -----------------------------------------------------------------------
    // RED GATE: adversary pass-1 finding M-5 — false dedup on distinct
    // messages sharing identical first-256-byte JSON prefix.
    //
    // HISTORICAL NOTE: this test was written for pass-1 when the contract was
    // "first 256 bytes of JSON repr".  Under the AMENDED pass-2 contract
    // (N=4096, raw string value), it still passes as a regression guard:
    // messages differing after byte 256 (but before byte 4096) must BOTH be
    // logged under both the old and new contract.
    //
    // Contract (unchanged): two distinct `internal.dispatcher_error` events
    // whose message fields differ (even if only after byte 256) MUST both be
    // logged.
    //
    // CURRENTLY PASSES: the current full-hash implementation logs both.
    // After bounded-N (N=4096) truncation: still passes (difference is at
    // byte 256, well within the 4096 window).
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // T-009 (S-19.09 AC-006): value-pin for INTERNAL_FILE_NOT_FOUND
    //
    // Locks the string literal "internal.file_not_found" to the named
    // constant so a typo or rename in the production source is caught
    // immediately by cargo test rather than surfacing as a silent runtime
    // mismatch in log consumers.
    //
    // Same class as S-19.08 T-001 const-pin precedent.
    // -----------------------------------------------------------------------

    /// T-009 — value-pin: INTERNAL_FILE_NOT_FOUND == "internal.file_not_found".
    #[test]
    fn t_009_internal_file_not_found_value_pin() {
        assert_eq!(INTERNAL_FILE_NOT_FOUND, "internal.file_not_found");
    }

    // -----------------------------------------------------------------------
    // T-010 (S-19.09 AC-007): value-pin for PLUGIN_ABANDONED
    //
    // Locks the string literal "plugin.abandoned" to the named constant.
    // Same rationale as T-009 above.
    // -----------------------------------------------------------------------

    /// T-010 — value-pin: PLUGIN_ABANDONED == "plugin.abandoned".
    #[test]
    fn t_010_plugin_abandoned_value_pin() {
        assert_eq!(PLUGIN_ABANDONED, "plugin.abandoned");
    }

    // -----------------------------------------------------------------------
    // Adjudicated value-pins for PLUGIN_COMPLETED and PLUGIN_TIMEOUT
    //
    // S-19.09 refactors four literals in emit_event.rs as a group
    // (INTERNAL_FILE_NOT_FOUND, PLUGIN_ABANDONED, PLUGIN_COMPLETED,
    // PLUGIN_TIMEOUT — the "v1.1 four-literal extension").  PLUGIN_COMPLETED
    // and PLUGIN_TIMEOUT were pre-existing constants, so AC-006/AC-007 did not
    // mandate test IDs for them.  However, adding value-pins here is
    // consistent with the S-19.08 T-001 precedent and protects all four
    // callsite constants with the same regression class.  These tests are
    // additive and trivially satisfy the production-grade default.
    // -----------------------------------------------------------------------

    /// Adjudicated value-pin: PLUGIN_COMPLETED == "plugin.completed".
    #[test]
    fn plugin_completed_value_pin() {
        assert_eq!(PLUGIN_COMPLETED, "plugin.completed");
    }

    /// Adjudicated value-pin: PLUGIN_TIMEOUT == "plugin.timeout".
    #[test]
    fn plugin_timeout_value_pin() {
        assert_eq!(PLUGIN_TIMEOUT, "plugin.timeout");
    }

    // -----------------------------------------------------------------------

    /// M-5 adversary finding: two distinct errors sharing a 256-byte JSON
    /// prefix must BOTH be logged (no false dedup).
    ///
    /// Regression guard: PASSES against current HEAD and after bounded-N fix.
    #[test]
    fn test_BC_2_06_001_dedup_no_false_dedup_for_messages_differing_after_byte_256() {
        let dir = tempfile::tempdir().unwrap();
        let log = InternalLog::new(dir.path().to_path_buf());
        let ts = Local.with_ymd_and_hms(2026, 6, 9, 11, 0, 0).unwrap();

        // Construct two distinct messages whose JSON-serialized values share the
        // same first 256 bytes but differ after byte 256.
        //
        // JSON serialization of a plain ASCII string of length N is `"<N chars>"`,
        // so bytes 0..=N+1 (N+2 total). We want the shared prefix to be exactly
        // 256 bytes long.
        //
        // shared_prefix_chars: 254 ASCII chars (bytes 1..254 inside `"..."` quotes).
        // Full shared JSON prefix = `"` + 254 chars = 255 bytes (byte 0..254).
        // At byte 255 and 256 we place distinct characters so the msgs differ.
        //
        // msg_a: 254 shared chars + "X" + suffix_a
        // msg_b: 254 shared chars + "Y" + suffix_b
        // Both have identical bytes 0..255 in JSON form, differ at byte 255+.
        // After .min(256) truncation: bytes 0..256 are shared for both.
        //
        // Actually we need to ensure the first 256 bytes of `v.to_string()` are
        // identical. `v.to_string()` for a String value = `"<escaped_string>"`.
        // So: byte 0 = `"`, bytes 1..254 = shared prefix, byte 255 = first
        // differing char, byte 256+ = rest.
        //
        // Therefore: shared_prefix = 254 ASCII chars, then distinct char at index 254.
        // Both messages: same 254 chars, then different suffix.
        // JSON of both: `"<254 chars><different suffix>"`.
        // Bytes 0..255 = `"` + 254 chars = 255 bytes shared.
        // Byte 255 = first byte of different suffix char.
        // Byte 256 = second byte (or another char).
        //
        // For the hash to collide: we need bytes 0..256 identical.
        // So shared content must be 255 bytes: `"` + 254 chars.
        // Make suffix start at byte 255, ensure bytes 255-256 are ASCII and same.
        // Then differ at byte 257+. → shared_prefix = 255 ASCII chars works:
        // JSON = `"<255 chars><msg_a_tail>"` vs `"<255 chars><msg_b_tail>"`.
        // First 256 bytes = `"` + 255 chars = 256 bytes — IDENTICAL for both.
        // Byte 256 = start of msg_a_tail vs msg_b_tail — DIFFERENT.
        //
        // This is the false-dedup scenario: hash uses s[..256] = same for both.

        let shared = "B".repeat(255);
        let msg_a = format!("{shared}TAIL_ALPHA_UNIQUE");
        let msg_b = format!("{shared}TAIL_BETA_DIFFERENT");

        // Sanity-check setup: first 256 bytes of JSON value must be identical.
        let json_a = serde_json::Value::String(msg_a.clone()).to_string();
        let json_b = serde_json::Value::String(msg_b.clone()).to_string();
        assert_eq!(
            &json_a.as_bytes()[..256],
            &json_b.as_bytes()[..256],
            "test setup: first 256 bytes must be identical for both JSON values"
        );
        assert_ne!(
            json_a, json_b,
            "test setup: messages must be distinct overall"
        );

        // Write both distinct errors.
        let event_a =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_a);
        let event_b =
            InternalEvent::with_ts(INTERNAL_DISPATCHER_ERROR, ts).with_field("message", msg_b);
        log.write(&event_a);
        log.write(&event_b);

        // Both events are distinct — BOTH must be logged.
        let log_file = dir
            .path()
            .join(format!("{FILENAME_PREFIX}2026-06-09{FILENAME_SUFFIX}"));
        let lines = read_lines(&log_file);
        assert_eq!(
            lines.len(),
            2,
            "M-5 (false dedup): two distinct dispatcher_error events that differ after \
             byte 256 must BOTH be logged; got {} lines (second was falsely suppressed)",
            lines.len()
        );
    }
}
