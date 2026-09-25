//! Durable unvalidated-mutation marker: atomic write, idempotent delete, predicate.
//!
//! This module owns the `.factory/unvalidated-mutation.marker` file lifecycle
//! for the INDETERMINATE outcome class (S-25.01 Layer-1, ADR-047):
//!
//! - [`write_indeterminate_marker`]: atomically writes the marker via temp+rename.
//! - [`delete_marker_if_pass`]: idempotently deletes the marker on PASS re-validation.
//! - [`should_write_marker`]: pure predicate — `true` iff outcome=Indeterminate AND policy=FailClosed.
//!
//! # PostToolUse-only invariant (BC-1.18.001 invariant 4)
//!
//! The marker write is PostToolUse only. Callers are responsible for enforcing this
//! constraint — `write_indeterminate_marker` itself does not check event type.
//! EC-002: PreToolUse INDETERMINATE events produce advisory events but no marker.
//!
//! # Single-marker policy (BC-1.18.001 invariant 3)
//!
//! A second INDETERMINATE event overwrites the existing marker (last-writer-wins).
//! The atomic temp+rename write achieves this without a read-modify-write cycle.
//!
//! # BC-5.38.001 Red Gate discipline
//!
//! All non-trivial function bodies use `todo!()`. Implementer fills in real logic.

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};

use crate::executor::DispatchOutcome;
use crate::host::HostContext;
use crate::internal_log::{InternalEvent, PLUGIN_MARKER_CLEARED, PLUGIN_MARKER_WRITTEN};
use crate::registry::FailurePolicy;

/// TTL for the `.factory/unvalidated-mutation.marker` deadman timer (ADR-048 §Decision 2).
///
/// 86 400 seconds = 24 hours. The marker's `expires_at` field is written once at
/// creation time as `timestamp + UNVALIDATED_MUTATION_MARKER_TTL_SECONDS`. The dispatcher
/// native crash-path check (`block_if_marker_check`) and the gate WASM plugin
/// (`evaluate_gate`) both treat an expired marker as absent (fail-open on stale
/// quarantine). No renewal mechanism; the quarantine must be explicitly resolved or
/// will auto-expire after 24 h.
///
/// BC-1.18.003 PC4: expired marker → Allow + auto-delete (idempotent, gate plugin).
pub const UNVALIDATED_MUTATION_MARKER_TTL_SECONDS: u64 = 86_400;

/// The six required TOML fields for `.factory/unvalidated-mutation.marker`.
///
/// BC-1.18.001 postcondition 4: all fields MUST be present.
/// `artifact_path` MUST be empty string (not omitted) when no artifact context.
/// `cause` MUST be one of `"fuel"`, `"epoch"`, `"output-too-large"` (BC-3.08.001 Event 8).
/// `expires_at` is new in ADR-048 §Decision 2; legacy markers without it are treated
/// conservatively (non-expired = block) by both native and WASM checks.
#[derive(Debug, Clone)]
pub struct MarkerFields {
    /// RFC 3339 timestamp of the INDETERMINATE event (e.g. "2026-08-30T12:00:00Z").
    pub timestamp: String,
    /// Registry name of the plugin that produced the INDETERMINATE outcome.
    pub plugin_name: String,
    /// Path of the artifact being validated when the INDETERMINATE occurred.
    /// MUST be empty string (not omitted) when no artifact context is available.
    /// BC-1.18.001 postcondition 4 + BC-3.08.001 Event 8: field is mandatory, empty string allowed.
    pub artifact_path: String,
    /// Cause of the INDETERMINATE outcome.
    /// MUST be exactly one of: `"fuel"`, `"epoch"`, `"output-too-large"`.
    /// Matches the `IndeterminateCause` enum string representation in the event wire format.
    pub cause: String,
    /// Dispatcher trace ID (`dispatcher_trace_id`) for the invocation.
    pub trace_id: String,
    /// RFC 3339 expiry timestamp = `timestamp` + `UNVALIDATED_MUTATION_MARKER_TTL_SECONDS`.
    /// ADR-048 §Decision 2: 24-hour deadman. The gate plugin auto-deletes the marker when
    /// this timestamp is reached. The native crash-path check (`block_if_marker_check`) treats
    /// an expired marker as absent (Allow). Missing/unparseable on legacy markers → non-expired.
    pub expires_at: String,
}

/// Atomically write the unvalidated-mutation marker file via write-to-temp + rename.
///
/// Algorithm (BC-1.18.001 postcondition 4 + invariant 3):
/// 1. Compute temp path: `<marker_path>.tmp` in the same directory as `marker_path`.
/// 2. Serialize the six required TOML fields to the temp file (O_CREAT | O_WRONLY | O_TRUNC).
/// 3. Atomically rename the temp file to `marker_path`.
///
/// Single-marker policy: if a marker already exists, the rename overwrites it (last-writer-wins).
/// Serialization uses the `toml` crate (Library table mandate; AC-005).
/// All control characters including `\n` are correctly escaped (MEDIUM-4 fix).
///
/// # Errors
///
/// Returns `io::Error` if the temp-file write or rename fails (EC-008).
/// The caller emits the `plugin.indeterminate` event regardless of whether this write succeeds.
pub fn write_indeterminate_marker(fields: &MarkerFields, marker_path: &Path) -> io::Result<()> {
    // Compute the temp path in the same directory (atomic rename invariant).
    // L-1 fix (S-25.01 adversary): use a unique suffix so concurrent writers from
    // same-tier plugins do not collide on a shared `.tmp` name (ENOENT race on rename).
    // Uniqueness: plugin_name + process_id + monotonic nonce → last-writer-wins still holds
    // because each writer renames ITS OWN temp file to the shared final path; the final
    // rename is still atomic (BC-1.18.001 INV3 preserved).
    static WRITE_NONCE: AtomicU64 = AtomicU64::new(0);
    let nonce = WRITE_NONCE.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    // Sanitize plugin_name for filename safety: keep alphanumeric, hyphen, underscore.
    let safe_plugin: String = fields
        .plugin_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file_name = marker_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unvalidated-mutation.marker");
    let tmp_name = format!("{file_name}.{safe_plugin}.{pid}.{nonce}.tmp");
    let tmp_path = marker_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(&tmp_name);

    // Serialize 6 mandatory TOML fields via the `toml` crate (Library table mandate).
    // toml::to_string correctly escapes all control characters including \n, \r, \t, etc.
    // Field insertion order preserved via BTreeMap (deterministic output).
    let mut table = toml::Table::new();
    table.insert(
        "timestamp".to_string(),
        toml::Value::String(fields.timestamp.clone()),
    );
    table.insert(
        "plugin_name".to_string(),
        toml::Value::String(fields.plugin_name.clone()),
    );
    table.insert(
        "artifact_path".to_string(),
        toml::Value::String(fields.artifact_path.clone()),
    );
    table.insert(
        "cause".to_string(),
        toml::Value::String(fields.cause.clone()),
    );
    table.insert(
        "trace_id".to_string(),
        toml::Value::String(fields.trace_id.clone()),
    );
    // ADR-048 §Decision 2: 24-hour deadman TTL field (BC-1.18.001 postcondition 4 + 5).
    table.insert(
        "expires_at".to_string(),
        toml::Value::String(fields.expires_at.clone()),
    );

    let content = toml::to_string(&table)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Write to temp file (O_CREAT | O_WRONLY | O_TRUNC semantics via fs::write).
    std::fs::write(&tmp_path, content.as_bytes())?;

    // Atomic rename temp → final path (single-marker policy: overwrites existing).
    // F-P3-004: a failed rename must not orphan the uniquely-named temp file it
    // just wrote — best-effort cleanup before propagating the rename error. The
    // temp path is unique per writer (plugin_name + pid + nonce), so removal here
    // can never race a DIFFERENT writer's own temp file.
    //
    // `rename_with_retry` (not a bare `std::fs::rename`) — TD-VSDD-060
    // sibling-site sweep from PR #842's Windows-CI transient-rename-denial
    // fix; see `last_amended_migrate::atomic_write::rename_with_retry`'s own
    // doc comment for the verified root cause.
    if let Err(e) = last_amended_migrate::atomic_write::rename_with_retry(&tmp_path, marker_path) {
        if let Err(cleanup_err) = std::fs::remove_file(&tmp_path) {
            tracing::warn!(
                tmp_path = %tmp_path.display(),
                rename_error = %e,
                cleanup_error = %cleanup_err,
                "write_indeterminate_marker: rename failed AND best-effort temp-file \
                 cleanup also failed; orphaned .tmp file may remain (F-P3-004)"
            );
        }
        return Err(e);
    }

    Ok(())
}

/// Read all six TOML fields from the unvalidated-mutation marker file.
///
/// Returns:
/// - `Ok(Some(fields))` if the file exists and all six fields are present and parseable.
/// - `Ok(None)` if the file does not exist (NotFound → no marker).
/// - `Ok(None)` if the file exists but cannot be parsed or any field is missing
///   (corrupt/legacy marker → treat as absent; conservative for the caller).
///
/// Used by the dispatcher's PASS-clear path to obtain the marker's `trace_id`,
/// `plugin_name`, and `artifact_path` before deleting the marker, so that
/// `marker.cleared` (BC-3.08.001 Event 9 / ADR-048 v1.1) can be emitted with
/// the correct provenance fields linking back to the originating `plugin.indeterminate`.
pub fn read_all_marker_fields(marker_path: &Path) -> io::Result<Option<MarkerFields>> {
    match std::fs::read_to_string(marker_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(content) => {
            let table: toml::Table = match toml::from_str(&content) {
                Ok(t) => t,
                Err(_) => return Ok(None),
            };
            let get_str =
                |key: &str| -> Option<String> { table.get(key)?.as_str().map(str::to_string) };
            // Five strictly-required fields must be present; missing/non-string fields → None.
            // `expires_at` is optional for legacy pre-ADR-048 markers (treated as non-expired
            // when absent) — default empty string in that case.
            let (
                Some(timestamp),
                Some(plugin_name),
                Some(artifact_path),
                Some(cause),
                Some(trace_id),
            ) = (
                get_str("timestamp"),
                get_str("plugin_name"),
                get_str("artifact_path"),
                get_str("cause"),
                get_str("trace_id"),
            )
            else {
                return Ok(None);
            };
            Ok(Some(MarkerFields {
                timestamp,
                plugin_name,
                artifact_path,
                cause,
                trace_id,
                expires_at: get_str("expires_at").unwrap_or_default(),
            }))
        }
    }
}

/// Read the `plugin_name` field from the unvalidated-mutation marker file.
///
/// Returns:
/// - `Ok(Some(name))` if the file exists and the `plugin_name` field is parseable.
/// - `Ok(None)` if the file does not exist (NotFound → no marker).
/// - `Ok(None)` if the file exists but cannot be parsed (corrupt marker → treat conservatively).
///
/// Used by `execute_tier` and `spawn_async_plugin` to enforce BC-1.18.003 INV2:
/// only the named plugin's PASS clears the marker (scoped clear).
/// The caller compares the returned name with the passing plugin's name before
/// calling `delete_marker_if_pass`.
pub fn read_marker_plugin_name(marker_path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(marker_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        // I/O errors other than NotFound: propagate them — caller decides.
        Err(e) => Err(e),
        Ok(content) => {
            // Parse via toml crate; treat parse errors as "unknown" plugin name (conservative).
            let table: toml::Table = match toml::from_str(&content) {
                Ok(t) => t,
                Err(_) => return Ok(None),
            };
            let name = table
                .get("plugin_name")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            Ok(name)
        }
    }
}

/// Idempotently delete the unvalidated-mutation marker file with artifact-scope enforcement.
///
/// # Artifact-scoped clear (M-1 fix, BC-1.18.003 PC1 + INV2)
///
/// A marker records the artifact path under validation when the INDETERMINATE event
/// occurred. Clearing must be scoped: if plugin P went INDETERMINATE while validating
/// artifact A, a subsequent PASS by P on artifact B must NOT discharge the quarantine —
/// the original mutation (artifact A) is still unvalidated.
///
/// Clear predicate:
/// - If the marker's `artifact_path` field is EMPTY — this was a non-artifact-scoped
///   validator; the marker is cleared unconditionally on a PASS (vacuously satisfied).
/// - If the marker's `artifact_path` == `current_artifact_path` (normalized absolute path
///   comparison via `std::path::Path`) — same artifact; clear the marker.
/// - Otherwise — different artifact; preserve the marker (return `Ok(())` without deleting).
///
/// # Idempotency (AC-013; BC-1.18.003 PC2)
///
/// If `marker_path` does not exist (`io::ErrorKind::NotFound`), returns `Ok(())` (no-op).
/// All other `io::Error` variants are propagated as `Err`.
///
/// # Conservative parse-error posture
///
/// If the marker file exists but cannot be parsed as TOML, this function returns `Ok(())`
/// WITHOUT deleting the file — quarantine is preserved. The normal call path already checks
/// `read_marker_plugin_name` before calling this, so a corrupt marker prevents the call.
/// Returns:
/// - `Ok(true)`  — the marker file was actually removed by this call.
/// - `Ok(false)` — no-op: marker absent, artifact-path mismatch, or corrupt TOML.
/// - `Err(_)`    — I/O error other than `NotFound` during read or delete.
///
/// Callers use the `bool` return to decide whether to emit `marker.cleared`
/// (BC-3.08.001 Event 9 / ADR-048 v1.1): only emit when `Ok(true)`.
pub fn delete_marker_if_pass(marker_path: &Path, current_artifact_path: &str) -> io::Result<bool> {
    // Read the marker to obtain its artifact_path for the scoped-clear predicate.
    let marker_artifact: String = match std::fs::read_to_string(marker_path) {
        // NotFound: no marker exists — idempotent no-op (AC-013).
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        // Other I/O errors: propagate.
        Err(e) => return Err(e),
        Ok(content) => {
            match toml::from_str::<toml::Table>(&content) {
                Ok(table) => table
                    .get("artifact_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                // Corrupt marker: conservative posture — preserve quarantine, do NOT delete.
                Err(_) => return Ok(false),
            }
        }
    };

    // M-1 clear predicate: delete IFF marker's artifact_path is EMPTY (non-artifact
    // validator, vacuously satisfied) OR marker's artifact_path == current_artifact_path
    // (exact equality of normalized absolute paths via Path comparison).
    let should_clear = marker_artifact.is_empty()
        || Path::new(&marker_artifact) == Path::new(current_artifact_path);

    if !should_clear {
        // Artifact mismatch: this marker belongs to a different artifact; quarantine persists.
        // BC-1.18.003 INV2: marker{plugin=p, artifact=A} MUST NOT clear on p PASSing artifact B.
        return Ok(false);
    }

    // Delete the marker. NotFound means a concurrent process already cleared it — treat as
    // no-op (Ok(false)), since THIS call did not perform the removal.
    match std::fs::remove_file(marker_path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Pure predicate: returns `true` iff `outcome` is `Indeterminate` AND `policy` is `FailClosed`.
///
/// This is the load-bearing gate before `write_indeterminate_marker` — fail-open plugins
/// MUST NOT write the marker or trigger the gate (BC-1.18.004 postcondition 1–3).
///
/// - `should_write_marker(Indeterminate, FailClosed) == true`
/// - `should_write_marker(Indeterminate, FailOpen) == false`
/// - `should_write_marker(Pass, FailClosed) == false`
/// - `should_write_marker(Fail, FailClosed) == false`
///
/// `FailurePolicy::default() == FailOpen` (S-21.10 canonical; ADR-039 §Decision 1).
/// VP-106 proof harness covers this predicate.
///
/// # BC-5.38.001
///
/// Non-trivial body — branching on enum variants. Uses `todo!()`.
pub fn should_write_marker(outcome: &DispatchOutcome, policy: FailurePolicy) -> bool {
    // True iff outcome=Indeterminate AND policy=FailClosed (AC-015; BC-1.18.004 PC1).
    // All other combinations return false:
    //   - Indeterminate + FailOpen  → advisory event only (BC-1.18.004 PC2)
    //   - Pass/Fail + FailClosed    → no INDETERMINATE event (write guard)
    //   - Pass/Fail + FailOpen      → no event at all
    // VP-106 proof harness covers this predicate.
    matches!(outcome, DispatchOutcome::Indeterminate { .. }) && policy == FailurePolicy::FailClosed
}

// ---------------------------------------------------------------------------
// ADR-048 §Decision 1 — native crash-path gate check
// ---------------------------------------------------------------------------

/// Check whether a non-expired `.factory/unvalidated-mutation.marker` exists and return
/// `true` (block) iff one does, `false` (allow) otherwise.
///
/// This is the NATIVE (non-WASM) crash-path guard for `on_error = "block_if_marker"`
/// (ADR-048 §Decision 1, BC-1.18.002 v1.5). It is called from the dispatcher's
/// `execute_tiers` loop when a plugin with `on_error = "block_if_marker"` crashes or
/// times out — before the plugin's WASM sandbox gets a chance to run.
///
/// # Decision logic
///
/// - Marker absent (NotFound) → `false` (Allow).
/// - Marker present, non-expired → `true` (Block).
/// - Marker present, expired (`expires_at <= now`) → `false` (Allow).
/// - Marker present, missing/unparseable `expires_at` (legacy pre-ADR-048) → `true` (Block,
///   conservative).
/// - Marker unreadable due to I/O error other than NotFound → `true` (Block,
///   fail CLOSED — PR #842 item 3 fix). This is a security/quarantine gate:
///   when the marker's state cannot be determined, silently defaulting to
///   Allow would let a transient filesystem fault (permission hiccup,
///   corrupted mount, etc.) disable the whole quarantine mechanism. Only a
///   genuine, unambiguous `NotFound` is treated as "no marker" — every other
///   error is treated as "indeterminate, so block" (CWE-636 corrected: the
///   PRIOR "fail-open on infra fault" framing balanced availability over
///   safety for a fail-CLOSED gate, which is backwards).
///
/// # Parameters
///
/// - `factory_root`: project root; the marker path is `<factory_root>/.factory/unvalidated-mutation.marker`.
/// - `now`: injectable clock (use `chrono::Utc::now()` in production; fixed value in tests).
pub fn block_if_marker_check(factory_root: &Path, now: DateTime<Utc>) -> bool {
    let marker_path = factory_root
        .join(".factory")
        .join("unvalidated-mutation.marker");
    let content = match std::fs::read_to_string(&marker_path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return false,
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %marker_path.display(),
                "block_if_marker: marker read I/O error — blocking (fail CLOSED; gate state \
                 indeterminate, PR #842 item 3 fix)"
            );
            return true;
        }
    };
    // TTL check (ADR-048 §Decision 2): expired marker → allow (treat as absent).
    match parse_expires_at(&content) {
        Some(exp) if exp <= now => {
            tracing::debug!(
                expires_at = %exp,
                now = %now,
                "block_if_marker: marker TTL elapsed — allowing (expired deadman)"
            );
            false
        }
        // Non-expired expiry, missing expiry (legacy marker), or unparseable expiry → block.
        _ => {
            tracing::info!(
                path = %marker_path.display(),
                "block_if_marker: non-expired marker present — blocking (ADR-048 §Decision 1)"
            );
            true
        }
    }
}

/// Parse the RFC 3339 `expires_at` field from TOML marker content.
///
/// Returns `Some(DateTime<Utc>)` on successful parse; `None` if the field is absent,
/// non-string, or fails RFC 3339 parsing. Callers treat `None` conservatively (block).
fn parse_expires_at(content: &str) -> Option<DateTime<Utc>> {
    let table: toml::Table = toml::from_str(content).ok()?;
    let s = table.get("expires_at")?.as_str()?;
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// ---------------------------------------------------------------------------
// ADR-048 §Decision 4 v1.2 — dispatcher-native marker.cleared emission
// (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH + F-P2-003 MED resolution)
// ---------------------------------------------------------------------------

/// Emit `marker.cleared` (BC-3.08.001 Event 9 / ADR-048 §Decision 4).
///
/// Single dispatcher-native emission point for ALL THREE `clear_mode` values
/// (`REVALIDATED`, `TTL_EXPIRED`, `OPERATOR_OVERRIDE`) per ADR-048 §D4 v1.2 —
/// relocated here (from a private `executor.rs` function of the same name)
/// so `check_and_clear_expired_marker` and `reconcile_raw_delete` share the
/// same emission logic as the REVALIDATED path's callsites in `executor.rs`.
///
/// The `trace_id` on this event is set to the MARKER'S stored `trace_id` (not
/// the current dispatch's trace), so the event links back to the originating
/// `plugin.indeterminate` (Event 8) that wrote the marker. This function
/// constructs the `InternalEvent` directly and routes it through
/// `HostContext::emit_internal` — it NEVER crosses the WASM `emit_event` host
/// ABI boundary, so the RESERVED_FIELDS enrichment that would otherwise
/// silently overwrite `trace_id`/`plugin_name` with the CALLING plugin's own
/// dispatch identity never applies (ADR-048 §D4 v1.2 Emission-Point
/// Correction). `emit_internal` is the same dual-sink primitive (durable
/// `InternalLog` write when `ctx.internal_log` is wired, AND the `ctx.events`
/// queue) every other dispatcher-native BC-3.08.001 event already uses — a
/// raw `InternalLog::write` call would silently diverge from that
/// established pattern (ADR-048 §D4 v1.3 Emission-Mechanism Precision
/// Correction; S-25.01 LOCAL adversary pass 3 F-P3-001).
///
/// `session_id` is read from `ctx.session_id` (identical value to what was
/// previously threaded explicitly as a separate parameter).
///
/// Best-effort: `emit_internal`'s own `InternalLog::write` never panics or
/// propagates errors (see `InternalLog::write`).
pub(crate) fn emit_marker_cleared(
    ctx: &HostContext,
    marker_fields: &MarkerFields,
    clear_mode: &str,
    actor_type: &str,
    reason: Option<&str>,
) {
    let ev = InternalEvent::now(PLUGIN_MARKER_CLEARED);
    // BC-3.08.001 wire format: mandatory `timestamp` field distinct from `ts` (DI-017).
    // `with_field("timestamp", ...)` adds the BC-required `timestamp` alias for `ts`.
    let ts = ev.ts.clone();
    let ev = ev
        // Use marker's trace_id (not current dispatch's) to link back to plugin.indeterminate.
        .with_trace_id(&marker_fields.trace_id)
        .with_session_id(&ctx.session_id)
        .with_plugin_name(&marker_fields.plugin_name)
        .with_field("timestamp", ts.as_str())
        .with_field(
            "artifact_path",
            serde_json::Value::String(marker_fields.artifact_path.clone()),
        )
        .with_field(
            "clear_mode",
            serde_json::Value::String(clear_mode.to_string()),
        )
        .with_field(
            "actor_type",
            serde_json::Value::String(actor_type.to_string()),
        )
        .with_field(
            "reason",
            reason.map_or(serde_json::Value::Null, |r| {
                serde_json::Value::String(r.to_string())
            }),
        );
    ctx.emit_internal(ev);
}

/// Emit `marker.written` (BC-3.08.001 Event 10 / ADR-048 §Decision 4 v1.4).
///
/// Structurally parallel to [`emit_marker_cleared`] but with no
/// `clear_mode`/`actor_type`/`reason` axis — creation has no mode/actor
/// axis; the "actor" for a write is always "the plugin that produced the
/// INDETERMINATE outcome," already fully captured by `plugin_name`.
///
/// # Emission point (implementer directive — F-P6-001)
///
/// Called from `executor.rs`'s two `write_indeterminate_marker` call sites
/// ONLY immediately after that call returns `Ok(())` — never before the
/// write is attempted, never on `Err`. This is the positive-creation-record
/// half of `reconcile_raw_delete`'s reconciliation premise (ADR-048 §D4
/// v1.4): an unmatched `marker.written` record is definitional proof that a
/// marker was durably written and has since become absent through some
/// path OTHER than the three already-audited clears (REVALIDATED,
/// TTL_EXPIRED, SUPERSEDED) — the only remaining explanation is a human
/// out-of-band `rm` (OPERATOR_OVERRIDE). Prior to this event's existence,
/// `reconcile_raw_delete` inferred "a marker was written" from an unmatched
/// fail-closed `plugin.indeterminate`, which is FALSE for a PreToolUse
/// fail-closed INDETERMINATE (never attempts a write, BC-1.18.001 INV4) and
/// for a PostToolUse marker-write I/O failure (EC-007) — both fabricated a
/// false `marker.cleared(OPERATOR_OVERRIDE)` audit record.
///
/// Routes through `HostContext::emit_internal` (dual-sink: durable
/// `InternalLog` write + `ctx.events` queue), never a raw `InternalLog::write`
/// and never the WASM `emit_event` host ABI — the same discipline every
/// other dispatcher-native BC-3.08.001 event in this module already follows
/// (ADR-048 §D4 v1.3 Emission-Mechanism Precision Correction).
pub(crate) fn emit_marker_written(ctx: &HostContext, fields: &MarkerFields) {
    let ev = InternalEvent::now(PLUGIN_MARKER_WRITTEN)
        // The marker's own trace_id — same dispatch as the concurrent plugin.indeterminate.
        .with_trace_id(&fields.trace_id)
        .with_session_id(&ctx.session_id)
        .with_plugin_name(&fields.plugin_name)
        .with_field(
            "artifact_path",
            serde_json::Value::String(fields.artifact_path.clone()),
        )
        .with_field("cause", serde_json::Value::String(fields.cause.clone()))
        .with_field(
            "expires_at",
            serde_json::Value::String(fields.expires_at.clone()),
        );
    ctx.emit_internal(ev);
}

/// Detect a cross-pair marker overwrite and emit `marker.cleared(SUPERSEDED)`
/// for the superseded pair (BC-1.18.001 INV3 single-marker last-writer-wins;
/// ADR-048 §D4 v1.3 F-P3-002).
///
/// Called by `write_indeterminate_marker`'s caller (`executor.rs`) with
/// `existing` = the marker's fields read via [`read_all_marker_fields`]
/// BEFORE the temp+rename overwrite is attempted, and `new_fields` = the
/// fields about to be written — but the call itself is made ONLY immediately
/// after `write_indeterminate_marker` returns `Ok(())`, alongside
/// [`emit_marker_written`] (ADR-048 §D4 v1.5 / F-P9-001, symmetric to the
/// v1.4 marker.written fix: SUPERSEDED must never be emitted before the
/// write is attempted, and never on `Err` — doing so would falsely record
/// the old marker as overwritten while it is still on disk untouched).
/// No-ops when:
/// - `existing` is `None` (no marker to supersede), or
/// - the existing marker's `(plugin_name, artifact_path)` pair is IDENTICAL
///   to `new_fields`'s (continuous quarantine of the same target — not a
///   cross-pair supersession; already fully covered by the marker's own
///   `trace_id` update at overwrite).
///
/// Without this event, [`reconcile_raw_delete`] has no record that the
/// superseded pair was ever cleared, and a LATER raw-delete of the
/// (now-different) marker would falsely emit `marker.cleared(OPERATOR_OVERRIDE)`
/// for the superseded pair — attributing a human action that never happened
/// (NIST AU-3/AU-10 event-content/non-repudiation defect).
pub(crate) fn emit_superseded_if_cross_pair(
    ctx: &HostContext,
    existing: Option<&MarkerFields>,
    new_fields: &MarkerFields,
) {
    let Some(old) = existing else {
        return;
    };
    if old.plugin_name == new_fields.plugin_name && old.artifact_path == new_fields.artifact_path {
        // Same pair re-INDETERMINATEs before being cleared — not a supersession.
        return;
    }
    emit_marker_cleared(
        ctx,
        old,
        "SUPERSEDED",
        "system",
        Some(
            "SUPERSEDED: marker overwritten by a new plugin.indeterminate event for a \
             different (plugin_name, artifact_path) pair before being cleared; \
             last-writer-wins (BC-1.18.001 INV3)",
        ),
    );
}

/// Single source of truth for the ADR-048 §D4 v1.5 emission-point discipline
/// tied to a `write_indeterminate_marker` call (TD-VSDD-060 sibling-duplication
/// fix; F-P12-001 pre-req).
///
/// Both `executor.rs` call sites (`execute_tier` and `spawn_async_plugin`) call
/// `write_indeterminate_marker`, then immediately hand this function the exact
/// `io::Result<()>` it returned (never inspected or branched on beforehand) so
/// that THIS function — not the callsite — is the one place that decides
/// whether the tied audit events fire:
///
/// - `Ok(())` — the write has actually happened, so:
///   1. [`emit_superseded_if_cross_pair`] first (F-P9-001: close the
///      superseded pair's record before the new pair's creation record), then
///   2. [`emit_marker_written`] (F-P6-001: positive creation record).
/// - `Err(_)` — the write did NOT happen: emit NEITHER audit event, only a
///   `tracing::warn!` (the `plugin.indeterminate` event was already emitted by
///   the caller before the write was attempted; best-effort write failure does
///   not fail the dispatch).
///
/// # Why the `Result` is a parameter, not re-derived
///
/// Passing `write_result` in (rather than having this function call
/// `write_indeterminate_marker` itself) is what makes the `Err(_)` no-emit
/// path directly unit-testable: a test can synthesize a failing `io::Result`
/// without needing to force a real filesystem failure.
///
/// `marker_path` is used only for the failure-path `tracing::warn!` (matches
/// the pre-refactor inline log fields at both call sites); `fields.plugin_name`
/// supplies the warn log's `plugin` field, since both call sites already set
/// it from the same `entry.name` used to populate `fields`.
pub(crate) fn emit_write_tied_audit_events(
    ctx: &HostContext,
    write_result: io::Result<()>,
    marker_path: &Path,
    existing: Option<&MarkerFields>,
    fields: &MarkerFields,
) {
    match write_result {
        Ok(()) => {
            // F-P9-001: the overwrite has actually happened now, so the
            // old (pre-existing) pair really was superseded — emit its
            // SUPERSEDED closure record before the new pair's creation
            // record (marker.written), preserving the audit trail's
            // close-then-open ordering.
            emit_superseded_if_cross_pair(ctx, existing, fields);
            // ADR-048 §D4 v1.4 / F-P6-001: emit marker.written ONLY
            // immediately after a confirmed successful write — never
            // before the write, never on Err — so reconcile_raw_delete's
            // unmatched-marker.written inference is sound by construction.
            emit_marker_written(ctx, fields);
        }
        Err(e) => {
            tracing::warn!(
                plugin = %fields.plugin_name,
                marker_path = %marker_path.display(),
                error = %e,
                "marker write failed on INDETERMINATE; dispatch continues"
            );
        }
    }
}

/// Dispatcher-native pre-check: detect and clear an expired
/// `.factory/unvalidated-mutation.marker`, emitting the audited
/// `marker.cleared(TTL_EXPIRED)` event (BC-3.08.001 Event 9 / ADR-048 §D4 v1.2).
///
/// Called from `executor.rs`'s tier-execution loop BEFORE every Arm 1/Arm 2
/// (`on_error = "block_if_marker"`) plugin invocation on the NORMAL (non-crash)
/// path. This is a DISTINCT code path from the crash-path native check
/// (`block_if_marker_check`), which never auto-deletes or emits — a crash
/// means this pre-check either did not run or was interrupted before
/// completing (BC-1.18.003 §EC-014; VP-108 Postcondition 4).
///
/// # Why dispatcher-native (not the WASM gate plugin)
///
/// The WASM `emit_event` host ABI's RESERVED_FIELDS enrichment unconditionally
/// overwrites plugin-supplied `trace_id`/`plugin_name` with the CALLING
/// plugin's own dispatch identity — a WASM plugin can never emit an event
/// carrying the marker's OWN (foreign) `trace_id`/`plugin_name`, which the
/// Event 9 wire contract requires (ADR-048 §D4 v1.2 Emission-Point
/// Correction; S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH). By the time
/// `evaluate_gate` (hook-plugins crate) subsequently runs, the marker is
/// guaranteed already absent-or-non-expired, so it performs no `expires_at`
/// parsing of its own — it is a pure presence check.
///
/// # Decision logic
///
/// - Marker absent, unreadable, or corrupt (unparseable TOML) — `Ok(None)`,
///   no clear (fail-safe).
/// - Marker present, `expires_at` absent or unparseable (legacy pre-ADR-048
///   marker) — treated as non-expired, conservative — `Ok(None)`.
/// - Marker present, `expires_at > now` — `Ok(None)`, no clear.
/// - Marker present, `expires_at <= now` — delete the marker file
///   (idempotent; a `NotFound` race with a concurrent clear is treated as
///   "this call did not clear it" — `Ok(None)`), emit
///   `marker.cleared(TTL_EXPIRED, deadman, reason=null)` with the MARKER's
///   own `trace_id`/`plugin_name`, and return `Ok(Some(fields))`.
///
/// # Errors
///
/// Propagates non-`NotFound` I/O errors from the marker read or delete.
/// Callers MUST treat an `Err` as "no clear occurred" (fail-safe) and
/// continue the dispatch rather than failing it — a durable pre-check must
/// never itself become a new self-lock source.
pub fn check_and_clear_expired_marker(
    factory_root: &Path,
    now: DateTime<Utc>,
    ctx: &HostContext,
) -> io::Result<Option<MarkerFields>> {
    let marker_path = factory_root
        .join(".factory")
        .join("unvalidated-mutation.marker");

    let fields = match read_all_marker_fields(&marker_path)? {
        Some(f) => f,
        None => return Ok(None),
    };

    let expired = chrono::DateTime::parse_from_rfc3339(&fields.expires_at)
        .map(|dt| dt.with_timezone(&Utc) <= now)
        // Absent/unparseable expires_at (legacy marker) -> conservative non-expired.
        .unwrap_or(false);

    if !expired {
        return Ok(None);
    }

    match std::fs::remove_file(&marker_path) {
        Ok(()) => {}
        // Concurrent clear (e.g. a racing REVALIDATED delete) already removed
        // it -- this call did not perform the clear; no emission from here.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    }

    emit_marker_cleared(ctx, &fields, "TTL_EXPIRED", "deadman", None);

    Ok(Some(fields))
}

/// Bound on the number of trailing bytes read from today's dispatcher-internal
/// log when [`reconcile_raw_delete`] scans for an unmatched `marker.written`
/// event (ADR-048 §D4 v1.4). ADR-048 §D4 v1.2: this scan runs before every Arm 1/Arm 2 dispatch on
/// the marker-absent path, so it MUST be bounded — a fixed-size tail read, not
/// a full-file scan (explicit production-grade constraint, not an optional
/// optimization; an unbounded scan on every Agent/git-commit dispatch would
/// reintroduce the large-artifact resource cost class S-25.01 exists to
/// eliminate).
const RECONCILE_SCAN_BYTE_CAP: u64 = 256 * 1024;

/// Dispatcher-native, best-effort reconciliation for the T3 (human out-of-band
/// `rm`) marker clear path (ADR-048 §Decision 4 v1.2 RAW_DELETE_DETECTED).
///
/// The out-of-band `rm` is never mediated by the dispatcher, so no real-time
/// `marker.cleared(OPERATOR_OVERRIDE)` can be emitted at the moment of
/// deletion. This function is called from the SAME native pre-check that
/// runs before every Arm 1/Arm 2 dispatch, in the branch where the marker is
/// confirmed absent. It performs a bounded scan of TODAY's
/// `dispatcher-internal-<date>.jsonl` for a `marker.written` (BC-3.08.001
/// Event 10) record with no subsequent `marker.cleared` for the same
/// `(plugin_name, artifact_path)` pair, and retroactively emits
/// `marker.cleared(OPERATOR_OVERRIDE)` — with a non-null `reason` per
/// BC-1.18.003 §PC3 / ADR-048 §D4's event field contract — for each such
/// unmatched pair found.
///
/// # Reconciliation-premise correction (ADR-048 §D4 v1.4 — F-P6-001)
///
/// The scan target is `marker.written`, NOT `plugin.indeterminate`. An
/// unmatched fail-closed `plugin.indeterminate` does NOT prove a marker was
/// ever written: a PreToolUse fail-closed INDETERMINATE never attempts a
/// write at all (BC-1.18.001 INV4), and a PostToolUse marker-write I/O
/// failure (EC-007) leaves the identical no-marker-ever-existed footprint.
/// Both would fabricate a false `marker.cleared(OPERATOR_OVERRIDE)` audit
/// record under the pre-v1.4 premise. `marker.written` is emitted (via
/// [`emit_marker_written`]) ONLY after a confirmed successful marker write,
/// so an unmatched `marker.written` is proof-by-construction that a marker
/// was durably written — making the reconciliation sound for every case, no
/// `failure_policy` filter required (a fail-open dispatch never reaches the
/// marker-write branch, so it never emits `marker.written` either).
///
/// # Bounded and best-effort (ADR-048 §D4 v1.2 production-grade requirement)
///
/// - Scans ONLY today's log file (never prior days).
/// - Reads at most the last [`RECONCILE_SCAN_BYTE_CAP`] bytes (tail read).
/// - Any failure to locate/read/parse the log is swallowed — this
///   reconciliation step never gates the dispatch decision (BC-3.08.001
///   Invariant 3) and never returns an error to the caller for a
///   missing/unavailable log.
///
/// # Idempotency
///
/// Once a pair is reconciled (its `marker.cleared` written), the next call
/// sees that emitted event within the scan window and does not re-reconcile
/// it — steady state converges to zero unmatched pairs.
///
/// # Defensive re-check
///
/// Re-verifies the marker is still absent before scanning; a marker found
/// present (a race with the caller's own absent-branch decision) short-
/// circuits to `Ok(())` with no scan performed.
pub fn reconcile_raw_delete(factory_root: &Path, ctx: &HostContext) -> io::Result<()> {
    let marker_path = factory_root
        .join(".factory")
        .join("unvalidated-mutation.marker");
    if marker_path.exists() {
        return Ok(());
    }

    // ADR-048 §D4 v1.3: the scan target is `dispatcher-internal-{date}.jsonl`,
    // obtained via `ctx.internal_log` — the only log every default production
    // dispatcher run writes unconditionally — NOT a literal `events-*.jsonl`
    // (which is not durably produced absent an opt-in `VSDD_SINK_FILE` or the
    // not-yet-main.rs-wired S-4.07 Router/FileSink). `None` short-circuits to
    // a no-op, exactly like `emit_internal`'s own `if let Some(log) = ...`
    // guard — a dispatch with no wired InternalLog has nothing to scan.
    let Some(log) = ctx.internal_log.as_ref() else {
        return Ok(());
    };

    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_path = log
        .log_dir()
        .join(format!("dispatcher-internal-{date}.jsonl"));

    let content = match read_tail(&log_path, RECONCILE_SCAN_BYTE_CAP) {
        Ok(c) => c,
        // No log yet today, or unavailable for any reason: best-effort no-op.
        Err(_) => return Ok(()),
    };

    // Track the most recent unmatched marker.written per (plugin_name,
    // artifact_path) (ADR-048 §D4 v1.4 / F-P6-001). A subsequent
    // marker.cleared for the same pair (any clear_mode) marks it reconciled.
    // Last-write-wins per pair: an older marker.written superseded by a
    // newer one for the same pair was itself superseded (single-marker
    // policy), not raw-deleted.
    let mut unmatched: std::collections::HashMap<(String, String), MarkerFields> =
        std::collections::HashMap::new();

    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            // A leading partial line from the tail read, or a corrupt line --
            // skip; best-effort.
            continue;
        };
        let Some(type_) = v.get("type").and_then(|t| t.as_str()) else {
            continue;
        };
        let plugin_name = v
            .get("plugin_name")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        let artifact_path = v
            .get("artifact_path")
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();
        let key = (plugin_name.clone(), artifact_path.clone());

        match type_ {
            // ADR-048 §D4 v1.4 / F-P6-001: was "plugin.indeterminate" +
            // failure_policy=="fail-closed" filter. marker.written is
            // emitted iff a marker was ACTUALLY, durably written, so the
            // failure_policy filter is now structurally redundant (removed).
            "marker.written" => {
                let trace_id = v
                    .get("trace_id")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let cause = v
                    .get("cause")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let timestamp = v
                    .get("ts")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let expires_at = v
                    .get("expires_at")
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .to_string();
                unmatched.insert(
                    key,
                    MarkerFields {
                        timestamp,
                        plugin_name,
                        artifact_path,
                        cause,
                        trace_id,
                        expires_at,
                    },
                );
            }
            "marker.cleared" => {
                unmatched.remove(&key);
            }
            _ => {}
        }
    }

    for fields in unmatched.into_values() {
        emit_marker_cleared(
            ctx,
            &fields,
            "OPERATOR_OVERRIDE",
            "operator",
            Some(
                "RAW_DELETE_DETECTED: marker absent without prior marker.cleared event; \
                 inferred operator out-of-band clear",
            ),
        );
    }

    Ok(())
}

/// Read at most the trailing `cap` bytes of the file at `path`. Drops a
/// possibly-truncated leading partial line when the read did not start at
/// byte 0 (a tail read may begin mid-write of the byte immediately before
/// the first fully-retained record).
fn read_tail(path: &Path, cap: u64) -> io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(cap);
    if start > 0 {
        f.seek(SeekFrom::Start(start))?;
    }
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    if start > 0 {
        return Ok(match text.find('\n') {
            Some(idx) => text[idx + 1..].to_string(),
            None => String::new(),
        });
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::internal_log::InternalLog;
    use std::sync::Arc;

    /// EC-008 (BC-1.18.003 INV2 / VP-106): same plugin, DIFFERENT non-empty artifact →
    /// `delete_marker_if_pass` MUST NOT clear the marker (quarantine persists).
    ///
    /// Negative assertion: marker with artifact_path="/abs/A.md" is NOT cleared when
    /// current_artifact_path="/abs/B.md". Positive control: the same marker IS cleared
    /// when current_artifact_path="/abs/A.md" (confirming the predicate is artifact-scoped,
    /// not always-keep).
    #[test]
    fn test_BC_1_18_003_EC_008_different_artifact_preserves_marker() {
        // EC-008 / BC-1.18.003 INV2 / VP-106
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        // Write marker for plugin "p" recording artifact "/abs/A.md".
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: "/abs/A.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-ec-008".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed for a writable path");
        assert!(
            marker_path.exists(),
            "pre-condition: marker must exist after write"
        );

        // Phase 1 — PASS on artifact "/abs/B.md" (DIFFERENT): marker MUST NOT be cleared.
        // BC-1.18.003 INV2: marker{artifact=/abs/A.md} MUST persist when current artifact ≠ marker artifact.
        delete_marker_if_pass(&marker_path, "/abs/B.md")
            .expect("delete_marker_if_pass must not return IO error");
        assert!(
            marker_path.exists(),
            "EC-008 / BC-1.18.003 INV2: delete_marker_if_pass(\"/abs/B.md\") MUST NOT clear \
             marker{{artifact_path=\"/abs/A.md\"}} — different artifact, quarantine persists."
        );

        // Phase 2 — positive control: PASS on artifact "/abs/A.md" (SAME): marker MUST be cleared.
        delete_marker_if_pass(&marker_path, "/abs/A.md")
            .expect("delete_marker_if_pass must not return IO error");
        assert!(
            !marker_path.exists(),
            "EC-008 (positive control) / BC-1.18.003 INV2: delete_marker_if_pass(\"/abs/A.md\") \
             MUST clear marker{{artifact_path=\"/abs/A.md\"}} — same artifact, quarantine lifted."
        );
    }

    /// EC-009 (BC-1.18.003 INV2 / VP-106): empty marker `artifact_path` → marker IS cleared
    /// unconditionally for ANY `current_artifact_path` (vacuous/name-only fallback).
    ///
    /// A non-artifact-scoped validator (e.g. a plugin that inspects process state rather
    /// than a specific file) writes an empty `artifact_path`. Its PASS MUST clear the
    /// marker regardless of what artifact the current dispatch is associated with.
    #[test]
    fn test_BC_1_18_003_EC_009_empty_marker_artifact_path_clears_unconditionally() {
        // EC-009 / BC-1.18.003 INV2 / VP-106
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        // Write marker with empty artifact_path (non-artifact-scoped validator).
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: "".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-ec-009".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed for a writable path");
        assert!(
            marker_path.exists(),
            "pre-condition: marker must exist after write"
        );

        // PASS with a non-empty current_artifact_path — the empty marker path is vacuously
        // satisfied; marker MUST be cleared regardless.
        delete_marker_if_pass(&marker_path, "/abs/anything.md")
            .expect("delete_marker_if_pass must not return IO error");
        assert!(
            !marker_path.exists(),
            "EC-009 / BC-1.18.003 INV2: delete_marker_if_pass(\"/abs/anything.md\") MUST clear \
             marker{{artifact_path=\"\"}} — empty artifact_path is the non-artifact-scoped \
             fallback; cleared unconditionally on any PASS."
        );
    }

    /// MEDIUM-4 (RED): The marker TOML serializer MUST correctly round-trip values
    /// containing a double-quote character and a newline. The `toml` crate is the
    /// mandated serialization library (Library table, S-25.01). AC-005.
    ///
    /// Currently FAILS (RED) because `toml_quote` only escapes `\\` and `"` but NOT
    /// `\n` (or other control characters). A newline inside the `artifact_path` value
    /// produces a multi-line TOML basic string that is syntactically INVALID TOML —
    /// `toml::from_str` fails to parse it back, or the round-trip produces a different
    /// string than the original.
    ///
    /// The fix is to use `toml::ser::to_string` (or equivalent) for correct escaping
    /// of all control characters per the TOML specification.
    #[test]
    fn test_BC_1_18_001_marker_toml_round_trips_quote_and_newline() {
        // AC-005 (Library requirement: `toml` crate; hand-rolled toml_quote insufficient)
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        // artifact_path contains both a double-quote and a newline — the two characters
        // that the hand-rolled toml_quote fails to serialize correctly (\n is not escaped).
        let artifact_with_special = "/path/with/\"quote\"\nand newline".to_string();
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "validate-factory-path-staging".to_string(),
            artifact_path: artifact_with_special.clone(),
            cause: "fuel".to_string(),
            trace_id: "trace-special-chars-001".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };

        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed for a writable path");

        let content = std::fs::read_to_string(&marker_path)
            .expect("marker file must be readable after write");

        // Step 1: the content MUST parse as valid TOML.
        // RED: toml_quote does not escape \n, producing invalid multi-line TOML.
        let parsed: toml::Table = toml::from_str(&content).unwrap_or_else(|e| {
            panic!(
                "AC-005: marker content MUST be valid TOML (per Library table, `toml` crate \
                 is mandatory). hand-rolled toml_quote does not escape \\n → invalid TOML. \
                 Parse error: {e}\nMarker content:\n{content}"
            )
        });

        // Step 2: the artifact_path field MUST round-trip to the original value.
        let rt_artifact = parsed
            .get("artifact_path")
            .and_then(|v| v.as_str())
            .expect("artifact_path key must be present and a string after round-trip");

        assert_eq!(
            rt_artifact, artifact_with_special,
            "AC-005: artifact_path containing double-quote and newline MUST round-trip \
             correctly via the `toml` crate serialization (\\n must be escaped as \\\\n)"
        );
    }

    // ── ADR-048 §Decision 1/2: block_if_marker_check + expires_at round-trip ──

    /// BC-1.18.002 PC5/PC6: absent marker → block_if_marker_check returns false (Allow).
    ///
    /// Precondition: no marker file at `factory_root/.factory/unvalidated-mutation.marker`.
    /// Postcondition: returns false (NotFound I/O error → fail-open per ADR-048 §D1 decision logic).
    #[test]
    fn test_BC_1_18_002_block_if_marker_check_absent_allows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        // No marker file written — NotFound path must return false.
        let now = Utc::now();
        assert!(
            !block_if_marker_check(dir.path(), now),
            "BC-1.18.002 PC5: absent marker MUST return false (Allow)"
        );
    }

    /// BC-1.18.002 PC5: marker present with future expires_at → block_if_marker_check returns true.
    ///
    /// Non-expired marker (expires_at >> now) MUST block per ADR-048 §Decision 1.
    #[test]
    fn test_BC_1_18_002_block_if_marker_check_future_expires_at_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-future".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed");
        let now = Utc::now();
        assert!(
            block_if_marker_check(dir.path(), now),
            "BC-1.18.002 PC5: marker with future expires_at MUST return true (Block)"
        );
    }

    /// BC-1.18.002 PC6: marker present with past expires_at → block_if_marker_check returns false.
    ///
    /// Expired marker (expires_at <= now) MUST allow per ADR-048 §Decision 2 TTL logic.
    #[test]
    fn test_BC_1_18_002_block_if_marker_check_past_expires_at_allows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2020-01-01T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-past".to_string(),
            expires_at: "2020-01-02T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed");
        let now = Utc::now();
        assert!(
            !block_if_marker_check(dir.path(), now),
            "BC-1.18.002 PC6: marker with past expires_at MUST return false (Allow, TTL elapsed)"
        );
    }

    /// BC-1.18.002: legacy marker (missing expires_at) → conservative block (true).
    ///
    /// Pre-ADR-048 markers have no expires_at field. The native check treats missing
    /// expires_at as non-expired (conservative) per the ADR-048 §D1 spec.
    #[test]
    fn test_BC_1_18_002_block_if_marker_check_missing_expires_at_blocks_legacy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        // 5-field legacy marker — no expires_at field.
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"legacy-plugin\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-legacy\"\n",
        )
        .expect("write legacy marker");
        let now = Utc::now();
        assert!(
            block_if_marker_check(dir.path(), now),
            "BC-1.18.002: legacy marker (missing expires_at) MUST return true (conservative block)"
        );
    }

    /// BC-1.18.002 I/O error path (PR #842 item 3 fix): a non-`NotFound` I/O
    /// error on the marker read (here, `IsADirectory`) MUST fail CLOSED
    /// (`true` / block) — the gate's state could not be determined, so the
    /// quarantine gate must not be silently disabled by a filesystem hiccup.
    ///
    /// Prior to this fix, `block_if_marker_check` returned `false` (Allow)
    /// on ANY read error other than `NotFound`, meaning a transient I/O
    /// fault silently disabled the whole quarantine mechanism — the opposite
    /// of the fail-safe posture a security gate requires.
    #[cfg(unix)]
    #[test]
    fn test_BC_1_18_002_block_if_marker_check_io_error_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        // Place a DIRECTORY at the marker path — read_to_string returns IsADirectory (not NotFound).
        std::fs::create_dir_all(&marker_path).expect("create dir-as-marker-path");
        let now = Utc::now();
        assert!(
            block_if_marker_check(dir.path(), now),
            "PR #842 item 3: a non-NotFound I/O error on the marker read MUST return true \
             (fail CLOSED / Block) — the gate's state is indeterminate, so it must not be \
             silently disabled by a filesystem hiccup (CWE-636 corrected)."
        );
    }

    /// BC-1.18.001 v1.1 PC4: write_indeterminate_marker persists all 6 fields correctly;
    /// the round-tripped delta from timestamp to expires_at equals UNVALIDATED_MUTATION_MARKER_TTL_SECONDS.
    ///
    /// Exercises VP-105 (marker write fidelity) and ADR-048 §Decision 2 (24-hour deadman TTL).
    #[test]
    fn test_BC_1_18_001_write_marker_stamps_expires_at_with_86400s_delta() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let ts_str = "2026-08-31T12:00:00Z";
        let ts_dt = chrono::DateTime::parse_from_rfc3339(ts_str)
            .expect("parse test timestamp")
            .with_timezone(&Utc);
        let expires_dt =
            ts_dt + chrono::Duration::seconds(UNVALIDATED_MUTATION_MARKER_TTL_SECONDS as i64);
        let fields = MarkerFields {
            timestamp: ts_str.to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-delta-test".to_string(),
            expires_at: expires_dt.to_rfc3339(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker must succeed");
        let content = std::fs::read_to_string(&marker_path).expect("read back marker content");
        let table: toml::Table =
            toml::from_str(&content).expect("round-tripped marker must be valid TOML");
        let rt_ts = table
            .get("timestamp")
            .and_then(|v| v.as_str())
            .expect("timestamp field must be present");
        let rt_exp = table
            .get("expires_at")
            .and_then(|v| v.as_str())
            .expect("expires_at field MUST be present (BC-1.18.001 v1.1 PC4 — 6th required field)");
        let rt_ts_dt = chrono::DateTime::parse_from_rfc3339(rt_ts)
            .expect("round-tripped timestamp must parse as RFC 3339")
            .with_timezone(&Utc);
        let rt_exp_dt = chrono::DateTime::parse_from_rfc3339(rt_exp)
            .expect("round-tripped expires_at must parse as RFC 3339")
            .with_timezone(&Utc);
        let delta_secs = (rt_exp_dt - rt_ts_dt).num_seconds();
        assert_eq!(
            delta_secs, UNVALIDATED_MUTATION_MARKER_TTL_SECONDS as i64,
            "BC-1.18.001 v1.1 PC4: expires_at − timestamp MUST equal \
             UNVALIDATED_MUTATION_MARKER_TTL_SECONDS ({UNVALIDATED_MUTATION_MARKER_TTL_SECONDS}s) \
             — got {delta_secs}s"
        );
    }

    // ── ADR-048 §D4 v1.3: check_and_clear_expired_marker + reconcile_raw_delete +
    // emit_marker_cleared — dispatcher-native via HostContext::emit_internal ──
    // (S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM + F-P3-002 LOW resolution)

    /// Build a `HostContext` wired to a real, file-backed `InternalLog` rooted at
    /// `dir/logs`, so `emit_internal`'s `InternalLog` write and
    /// `reconcile_raw_delete`'s scan both operate on the SAME durable
    /// `dispatcher-internal-{date}.jsonl` file a real dispatch would use. Log dir
    /// deliberately NOT under `.factory` so the #206 mount gate does not suppress
    /// the write (matches the executor.rs test pattern).
    fn test_ctx(dir: &Path, session_id: &str) -> HostContext {
        let mut ctx = HostContext::new("test-plugin", "0.0.0", session_id, "test-trace-vp108");
        ctx.internal_log = Some(Arc::new(InternalLog::new(dir.join("logs"))));
        ctx
    }

    /// Read back the single JSONL line written under `log_dir` and parse it as JSON.
    /// Panics if zero or more than one file/line is present — every test using this
    /// helper writes exactly one InternalLog-backed event before calling it.
    fn read_only_log_event(log_dir: &Path) -> serde_json::Value {
        let files: Vec<_> = std::fs::read_dir(log_dir)
            .expect("log dir must exist after a write")
            .map(|e| e.expect("dir entry").path())
            .collect();
        assert_eq!(
            files.len(),
            1,
            "expected exactly one log file, got {files:?}"
        );
        let content = std::fs::read_to_string(&files[0]).expect("read log file");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "expected exactly one log line, got {lines:?}"
        );
        serde_json::from_str(lines[0]).expect("log line must be valid JSON")
    }

    /// VP-108 PC2 / VP-106 PC-F (v1.2 — emission-mechanism corrected, ADR-048 §D4
    /// v1.3 F-P3-001): expired marker → `check_and_clear_expired_marker` deletes it
    /// and emits exactly one `marker.cleared(TTL_EXPIRED)` event carrying the
    /// MARKER's own trace_id/plugin_name (not the caller's session_id-scoped
    /// identity) and `reason = null`. Asserts the DUAL-WRITE: the event lands in
    /// BOTH the durable `InternalLog` file AND the `ctx.drain_events()` queue —
    /// proof that the call goes through `HostContext::emit_internal`, not a raw
    /// `InternalLog::write`.
    #[test]
    fn test_ADR_048_D4_check_and_clear_expired_marker_ttl_expired_deletes_and_emits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2020-01-01T00:00:00Z".to_string(),
            plugin_name: "regression-gate".to_string(),
            artifact_path: "/tmp/.factory/STATE.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "deadbeef-0106-0001-0001-000000000001".to_string(),
            expires_at: "2020-01-02T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");

        let ctx = test_ctx(dir.path(), "sess-ttl");
        let now = chrono::DateTime::parse_from_rfc3339("2020-01-03T00:00:00Z")
            .expect("parse now")
            .with_timezone(&Utc);

        let cleared = check_and_clear_expired_marker(dir.path(), now, &ctx)
            .expect("check_and_clear_expired_marker must not error");

        assert!(
            matches!(&cleared, Some(f) if f.trace_id == fields.trace_id),
            "VP-106 PC-F: expired marker MUST yield Some(fields) with the marker's own trace_id"
        );
        assert!(
            !marker_path.exists(),
            "VP-106 PC-F: check_and_clear_expired_marker MUST auto-delete the expired marker"
        );

        // Durable-log sink assertion.
        let event = read_only_log_event(&dir.path().join("logs"));
        assert_eq!(
            event["type"], "marker.cleared",
            "VP-108 PC2: type must be marker.cleared"
        );
        assert_eq!(
            event["clear_mode"], "TTL_EXPIRED",
            "VP-108 PC2: clear_mode must be TTL_EXPIRED"
        );
        assert_eq!(
            event["actor_type"], "deadman",
            "VP-108 PC2: actor_type must be deadman"
        );
        assert_eq!(
            event["trace_id"], fields.trace_id,
            "VP-108 PC2: trace_id MUST be the MARKER's own trace_id, not the gate plugin's dispatch trace_id"
        );
        assert_eq!(event["plugin_name"], fields.plugin_name);
        assert_eq!(event["artifact_path"], fields.artifact_path);
        assert!(
            event["reason"].is_null(),
            "VP-108 PC2: reason must be null/absent for TTL_EXPIRED"
        );
        // F-P10-002 / BC-3.08.001 §Event 9 Wire format + Mandatory fields: `marker.cleared`
        // declares a distinct top-level `timestamp` field (ISO-8601 UTC — time of the
        // clear, not the original INDETERMINATE event), separate from the common `ts`
        // field. No prior test asserted this on any marker.cleared clear_mode, which let
        // the real `emit_marker_cleared` implementation ship without it (F-P10-002).
        let timestamp_str = event
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !timestamp_str.is_empty(),
            "F-P10-002 / BC-3.08.001 Event 9: emit_marker_cleared(TTL_EXPIRED) must emit a \
             non-empty distinct 'timestamp' field; field is absent or empty"
        );

        // Drained-events-queue sink assertion (ADR-048 §D4 v1.3 F-P3-001):
        // `emit_internal` pushes onto `ctx.events` in addition to the durable
        // InternalLog write — a raw `InternalLog::write` would never reach here.
        let drained = ctx.drain_events();
        assert_eq!(
            drained.len(),
            1,
            "VP-108 PC2: exactly one marker.cleared event must reach ctx.events (drain_events)"
        );
        let dev = &drained[0];
        assert_eq!(dev.type_, "marker.cleared");
        assert_eq!(
            dev.fields.get("clear_mode").and_then(|v| v.as_str()),
            Some("TTL_EXPIRED")
        );
        assert_eq!(
            dev.dispatcher_trace_id.as_deref(),
            Some(fields.trace_id.as_str()),
            "VP-108 PC2: drained event trace_id must be the marker's own trace_id"
        );
        assert!(
            dev.fields
                .get("timestamp")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty()),
            "F-P10-002 / BC-3.08.001 Event 9: drained ctx.events copy of \
             marker.cleared(TTL_EXPIRED) must also carry a non-empty 'timestamp' field"
        );
    }

    /// VP-106 INV5: non-expired marker → `check_and_clear_expired_marker` returns
    /// `None`, does NOT delete the marker, and emits NO event (neither sink).
    #[test]
    fn test_ADR_048_D4_check_and_clear_expired_marker_non_expired_returns_none_no_emit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-non-expired".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");

        let ctx = test_ctx(dir.path(), "sess-active");
        let now = Utc::now();

        let cleared = check_and_clear_expired_marker(dir.path(), now, &ctx)
            .expect("check_and_clear_expired_marker must not error");

        assert!(
            cleared.is_none(),
            "VP-106 INV5: non-expired marker MUST yield None"
        );
        assert!(
            marker_path.exists(),
            "VP-106 INV5: non-expired marker MUST NOT be deleted"
        );
        assert!(
            !dir.path().join("logs").exists(),
            "VP-108: no marker.cleared event (and no log dir at all) when nothing was cleared"
        );
        assert!(
            ctx.drain_events().is_empty(),
            "VP-108: no marker.cleared event on the drained-events queue either"
        );
    }

    /// VP-106: absent marker → `check_and_clear_expired_marker` returns `None`, no emit.
    #[test]
    fn test_ADR_048_D4_check_and_clear_expired_marker_absent_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");
        let ctx = test_ctx(dir.path(), "sess-absent");

        let cleared = check_and_clear_expired_marker(dir.path(), Utc::now(), &ctx)
            .expect("check_and_clear_expired_marker must not error on absent marker");

        assert!(cleared.is_none(), "absent marker MUST yield None");
        assert!(
            !dir.path().join("logs").exists(),
            "no event MUST be emitted when there is nothing to clear"
        );
        assert!(ctx.drain_events().is_empty());
    }

    /// VP-106 PC-G: legacy marker (no expires_at field) → treated as non-expired
    /// (conservative); `check_and_clear_expired_marker` returns `None`, no delete, no emit.
    #[test]
    fn test_ADR_048_D4_check_and_clear_expired_marker_legacy_missing_expires_at_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        // Legacy 5-field marker — no expires_at.
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"legacy-plugin\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-legacy-check\"\n",
        )
        .expect("write legacy marker");

        let ctx = test_ctx(dir.path(), "sess-legacy");

        let cleared = check_and_clear_expired_marker(dir.path(), Utc::now(), &ctx)
            .expect("check_and_clear_expired_marker must not error");

        assert!(
            cleared.is_none(),
            "VP-106 PC-G: legacy marker (absent expires_at) MUST yield None — conservative"
        );
        assert!(
            marker_path.exists(),
            "VP-106 PC-G: legacy marker MUST NOT be auto-deleted"
        );
        assert!(
            !dir.path().join("logs").exists(),
            "no marker.cleared event MUST be emitted for a legacy marker"
        );
        assert!(ctx.drain_events().is_empty());
    }

    /// VP-108 PC3 (BC-1.18.003 §PC3 + ADR-048 §D4 v1.4 RAW_DELETE_DETECTED):
    /// an unmatched `marker.written` (BC-3.08.001 Event 10) with the marker
    /// now absent → `reconcile_raw_delete` retroactively emits exactly one
    /// `marker.cleared(OPERATOR_OVERRIDE)` with the marker.written event's own
    /// trace_id/plugin_name/artifact_path and a NON-null `reason`
    /// (BC-1.18.003 §PC3 / ADR-048 §D4 event field contract — `reason` is
    /// mandatory, not null, for OPERATOR_OVERRIDE). Asserts the dual-write.
    ///
    /// Scan MATCH TYPE (v1.3 correction, ADR-048 §D4 v1.4 / F-P6-001) is
    /// `marker.written` — NOT a raw `plugin.indeterminate` line. The fixture
    /// seeds `marker.written` via [`emit_marker_written`], the SAME function
    /// production code calls after a successful marker write, so this
    /// fixture is only reachable in the state a real successful write
    /// produces (contrast the F-P6-001 negative-control test below, whose
    /// fixture seeds `plugin.indeterminate` WITHOUT `marker.written` and
    /// asserts NO reconciliation occurs).
    #[test]
    fn test_ADR_048_D4_reconcile_raw_delete_unmatched_event_emits_operator_override() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");
        // No marker file — this is the marker-absent branch reconcile_raw_delete runs in.

        let ctx = test_ctx(dir.path(), "sess-reconcile");
        // Seed an unmatched marker.written record via the SAME emit_marker_written
        // function production code calls after a successful write — lands in
        // dispatcher-internal-{today}.jsonl, which reconcile_raw_delete scans.
        let written_fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "regression-gate".to_string(),
            artifact_path: "/tmp/.factory/STATE.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-raw-delete-1".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        emit_marker_written(&ctx, &written_fields);

        reconcile_raw_delete(dir.path(), &ctx).expect("reconcile_raw_delete must not error");

        // Durable-log sink assertion — the scan target itself
        // (`dispatcher-internal-{date}.jsonl`, ADR-048 §D4 v1.3) also receives the
        // retroactive emission, appended to the very file that was scanned
        // (which already contains the seeded marker.written line).
        let log_dir = dir.path().join("logs");
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_file = log_dir.join(format!("dispatcher-internal-{date}.jsonl"));
        let content = std::fs::read_to_string(&log_file).expect("read log");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "VP-108 PC3: marker.written + exactly one marker.cleared line — got {lines:?}"
        );
        let event: serde_json::Value =
            serde_json::from_str(lines[1]).expect("second line must be valid JSON");
        assert_eq!(event["type"], "marker.cleared");
        assert_eq!(event["clear_mode"], "OPERATOR_OVERRIDE");
        assert_eq!(event["actor_type"], "operator");
        assert_eq!(
            event["trace_id"], "trace-raw-delete-1",
            "VP-108 PC3: trace_id MUST come from the unmatched marker.written event"
        );
        assert_eq!(event["plugin_name"], "regression-gate");
        assert_eq!(event["artifact_path"], "/tmp/.factory/STATE.md");
        assert!(
            event["reason"]
                .as_str()
                .map(|r| !r.is_empty())
                .unwrap_or(false),
            "BC-1.18.003 §PC3 / ADR-048 §D4: reason MUST be non-null and non-empty for \
             OPERATOR_OVERRIDE — got {:?}",
            event["reason"]
        );
        // F-P10-002 / BC-3.08.001 §Event 9 Wire format + Mandatory fields: `marker.cleared`
        // declares a distinct top-level `timestamp` field, separate from the common `ts`
        // field. No prior test asserted this on the OPERATOR_OVERRIDE clear_mode, which
        // let the real `emit_marker_cleared` implementation ship without it (F-P10-002).
        let timestamp_str = event
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !timestamp_str.is_empty(),
            "F-P10-002 / BC-3.08.001 Event 9: emit_marker_cleared(OPERATOR_OVERRIDE) must \
             emit a non-empty distinct 'timestamp' field; field is absent or empty"
        );

        // Drained-events-queue sink assertion. drain_events() so far contains
        // [marker.written, marker.cleared] (the seed call also pushed onto
        // the queue) — filter to the clear event under test.
        let drained = ctx.drain_events();
        let clear_events: Vec<_> = drained
            .iter()
            .filter(|e| e.type_ == "marker.cleared")
            .collect();
        assert_eq!(
            clear_events.len(),
            1,
            "VP-108 PC3: exactly one marker.cleared event must reach ctx.events (drain_events)"
        );
        assert_eq!(
            clear_events[0]
                .fields
                .get("clear_mode")
                .and_then(|v| v.as_str()),
            Some("OPERATOR_OVERRIDE")
        );
        assert!(
            clear_events[0]
                .fields
                .get("reason")
                .and_then(|v| v.as_str())
                .is_some_and(|r| !r.is_empty()),
            "VP-108 PC3: drained event reason MUST also be non-null for OPERATOR_OVERRIDE"
        );
        assert!(
            clear_events[0]
                .fields
                .get("timestamp")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty()),
            "F-P10-002 / BC-3.08.001 Event 9: drained ctx.events copy of \
             marker.cleared(OPERATOR_OVERRIDE) must also carry a non-empty 'timestamp' field"
        );
    }

    /// VP-108 PC3 (negative): a `marker.written` already followed by its own
    /// `marker.cleared` for the same (plugin_name, artifact_path) is already
    /// reconciled — `reconcile_raw_delete` MUST NOT emit a duplicate.
    #[test]
    fn test_ADR_048_D4_reconcile_raw_delete_already_matched_emits_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");

        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).expect("create log dir");
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_file = log_dir.join(format!("dispatcher-internal-{date}.jsonl"));
        let written_line = serde_json::json!({
            "type": "marker.written",
            "trace_id": "trace-matched-1",
            "plugin_name": "regression-gate",
            "artifact_path": "/tmp/.factory/STATE.md",
            "cause": "fuel",
            "expires_at": "2099-01-01T00:00:00Z",
            "ts": "2026-08-31T00:00:00+0000",
        });
        let cleared_line = serde_json::json!({
            "type": "marker.cleared",
            "trace_id": "trace-matched-1",
            "plugin_name": "regression-gate",
            "artifact_path": "/tmp/.factory/STATE.md",
            "clear_mode": "REVALIDATED",
            "actor_type": "validator",
            "ts": "2026-08-31T00:05:00+0000",
        });
        std::fs::write(&log_file, format!("{written_line}\n{cleared_line}\n")).expect("seed log");

        let ctx = test_ctx(dir.path(), "sess-reconcile");
        reconcile_raw_delete(dir.path(), &ctx).expect("reconcile_raw_delete must not error");

        let content = std::fs::read_to_string(&log_file).expect("read log");
        assert_eq!(
            content.lines().count(),
            2,
            "already-reconciled pair MUST NOT produce a new marker.cleared emission"
        );
        assert!(ctx.drain_events().is_empty());
    }

    /// `reconcile_raw_delete` defensive re-check: marker still present → no-op,
    /// regardless of log content (never emits while the marker itself is active).
    #[test]
    fn test_ADR_048_D4_reconcile_raw_delete_marker_present_short_circuits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-present".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");

        let ctx = test_ctx(dir.path(), "sess-present");

        reconcile_raw_delete(dir.path(), &ctx).expect("reconcile_raw_delete must not error");

        assert!(
            !dir.path().join("logs").exists(),
            "reconcile_raw_delete MUST NOT scan or emit while the marker is present"
        );
        assert!(ctx.drain_events().is_empty());
    }

    /// ADR-048 §D4 v1.3 bounded-scan requirement: an unmatched `marker.written`
    /// that falls OUTSIDE the trailing `RECONCILE_SCAN_BYTE_CAP` window (because it
    /// is followed by more than the cap's worth of filler bytes) MUST NOT be
    /// reconciled — the scan is a bounded tail read, not a full-file scan.
    #[test]
    fn test_ADR_048_D4_reconcile_raw_delete_respects_byte_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");

        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).expect("create log dir");
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_file = log_dir.join(format!("dispatcher-internal-{date}.jsonl"));

        let indeterminate_line = serde_json::json!({
            "type": "marker.written",
            "trace_id": "trace-out-of-window",
            "plugin_name": "regression-gate",
            "artifact_path": "/tmp/.factory/STATE.md",
            "cause": "fuel",
            "expires_at": "2099-01-01T00:00:00Z",
            "ts": "2026-08-31T00:00:00+0000",
        })
        .to_string();

        // Filler well past the byte cap, written AFTER the target line so the
        // trailing tail-read window no longer includes it.
        let filler_line = serde_json::json!({
            "type": "plugin.completed",
            "plugin_name": "irrelevant-plugin",
            "ts": "2026-08-31T00:00:01+0000",
        })
        .to_string();
        let mut content = format!("{indeterminate_line}\n");
        while (content.len() as u64) < RECONCILE_SCAN_BYTE_CAP + 4096 {
            content.push_str(&filler_line);
            content.push('\n');
        }
        std::fs::write(&log_file, &content).expect("seed large log");

        let ctx = test_ctx(dir.path(), "sess-bounded");
        reconcile_raw_delete(dir.path(), &ctx).expect("reconcile_raw_delete must not error");

        let after = std::fs::read_to_string(&log_file).expect("read log");
        assert!(
            !after.contains("OPERATOR_OVERRIDE"),
            "bounded scan MUST NOT reconcile an unmatched event that falls outside the \
             trailing RECONCILE_SCAN_BYTE_CAP window"
        );
        assert!(ctx.drain_events().is_empty());
    }

    /// `reconcile_raw_delete` no-op when today's log file does not exist at all
    /// (fresh session, nothing indeterminate has happened yet today).
    #[test]
    fn test_ADR_048_D4_reconcile_raw_delete_no_log_file_is_noop() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");
        // log_dir itself does not exist yet.
        let ctx = test_ctx(dir.path(), "sess-no-log");

        let result = reconcile_raw_delete(dir.path(), &ctx);
        assert!(
            result.is_ok(),
            "reconcile_raw_delete MUST be a best-effort no-op when no log exists yet, got {result:?}"
        );
    }

    /// ADR-048 §D4 v1.3: `reconcile_raw_delete` MUST be a no-op — never panic,
    /// never attempt a scan — when `ctx.internal_log` is `None` (no wired durable
    /// log to scan). This is the `ctx.internal_log.as_ref().map(...)` `None`
    /// short-circuit the implementer directive requires.
    #[test]
    fn test_ADR_048_D4_v1_3_reconcile_raw_delete_noop_when_internal_log_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".factory")).expect("create .factory subdir");
        // ctx with NO internal_log wired — HostContext::new() default.
        let ctx = HostContext::new("test-plugin", "0.0.0", "sess-no-ctx-log", "trace-no-log");
        assert!(
            ctx.internal_log.is_none(),
            "precondition: HostContext::new leaves internal_log unset"
        );

        let result = reconcile_raw_delete(dir.path(), &ctx);
        assert!(
            result.is_ok(),
            "VP-108: reconcile_raw_delete MUST no-op (not error) when ctx.internal_log is None, \
             got {result:?}"
        );
        assert!(
            ctx.drain_events().is_empty(),
            "VP-108: no marker.cleared event when there is no durable log to scan"
        );
    }

    // ── ADR-048 §D4 v1.3 F-P3-002: SUPERSEDED clear_mode ──

    /// VP-108 PC5 (NEW — BC-1.18.001 INV3 cross-pair overwrite; ADR-048 §D4 v1.3;
    /// S-25.01 pass-3 F-P3-002 LOW): a DIFFERENT `(plugin_name, artifact_path)`
    /// pair about to overwrite an existing marker → `emit_superseded_if_cross_pair`
    /// emits exactly one `marker.cleared(SUPERSEDED, system)` carrying the OLD
    /// pair's own fields (not the new pair's), with a non-null `reason`.
    #[test]
    fn test_ADR_048_D4_v1_3_superseded_clear_emits_for_cross_pair_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let pair_a = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "plugin-a".to_string(),
            artifact_path: "/tmp/A.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "deadbeef-0108-0005-0005-00000000000a".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&pair_a, &marker_path).expect("write marker A");

        let ctx = test_ctx(dir.path(), "sess-superseded");
        // Implementer directive: the write_indeterminate_marker CALLER (executor.rs)
        // reads the pre-existing marker, detects the (plugin_name, artifact_path)
        // mismatch against the NEW pair about to be written, and emits SUPERSEDED
        // for the OLD pair before overwriting. This test exercises that exact
        // pattern via emit_superseded_if_cross_pair.
        let existing = read_all_marker_fields(&marker_path).unwrap();
        let pair_b = MarkerFields {
            timestamp: "2026-08-31T01:00:00Z".to_string(),
            plugin_name: "plugin-b".to_string(),
            artifact_path: "/tmp/B.md".to_string(),
            cause: "epoch".to_string(),
            trace_id: "deadbeef-0108-0005-0005-00000000000b".to_string(),
            expires_at: "2099-01-01T01:00:00Z".to_string(),
        };
        emit_superseded_if_cross_pair(&ctx, existing.as_ref(), &pair_b);
        write_indeterminate_marker(&pair_b, &marker_path).expect("write marker B (overwrite)");

        let events = ctx.drain_events();
        assert_eq!(
            events.len(),
            1,
            "VP-108 PC5: exactly one marker.cleared(SUPERSEDED) event"
        );
        let ev = &events[0];
        assert_eq!(ev.type_, "marker.cleared");
        assert_eq!(
            ev.fields.get("clear_mode").and_then(|v| v.as_str()),
            Some("SUPERSEDED")
        );
        assert_eq!(
            ev.fields.get("actor_type").and_then(|v| v.as_str()),
            Some("system")
        );
        assert_eq!(
            ev.dispatcher_trace_id.as_deref(),
            Some(pair_a.trace_id.as_str()),
            "VP-108 PC5: trace_id must be the SUPERSEDED (old) pair's own trace_id, not the new pair's"
        );
        assert_eq!(ev.plugin_name.as_deref(), Some("plugin-a"));
        assert_eq!(
            ev.fields.get("artifact_path").and_then(|v| v.as_str()),
            Some("/tmp/A.md")
        );
        assert!(
            ev.fields
                .get("reason")
                .and_then(|v| v.as_str())
                .is_some_and(|r| !r.is_empty()),
            "VP-108 PC5: reason MUST be non-null for SUPERSEDED"
        );
        // F-P10-002 / BC-3.08.001 §Event 9 Wire format + Mandatory fields: `marker.cleared`
        // declares a distinct top-level `timestamp` field, separate from the common `ts`
        // field. No prior test asserted this on the SUPERSEDED clear_mode, which let the
        // real `emit_marker_cleared` implementation ship without it (F-P10-002).
        assert!(
            ev.fields
                .get("timestamp")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty()),
            "F-P10-002 / BC-3.08.001 Event 9: emit_marker_cleared(SUPERSEDED) must emit a \
             non-empty distinct 'timestamp' field; field is absent or empty"
        );

        // Positive control: the marker on disk is now pair B's.
        let after = read_all_marker_fields(&marker_path).unwrap().unwrap();
        assert_eq!(after.plugin_name, "plugin-b");
    }

    /// VP-108 PC5 (negative): the SAME `(plugin_name, artifact_path)` pair
    /// re-INDETERMINATEs before being cleared — this is continuous quarantine of
    /// the same target, NOT a cross-pair supersession. `emit_superseded_if_cross_pair`
    /// MUST NOT emit `marker.cleared(SUPERSEDED)`.
    #[test]
    fn test_ADR_048_D4_v1_3_superseded_clear_not_emitted_for_same_pair_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let first = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "plugin-a".to_string(),
            artifact_path: "/tmp/A.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-same-pair-1".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&first, &marker_path).expect("write first marker");

        let ctx = test_ctx(dir.path(), "sess-same-pair");
        let existing = read_all_marker_fields(&marker_path).unwrap();
        let second = MarkerFields {
            timestamp: "2026-08-31T02:00:00Z".to_string(),
            plugin_name: "plugin-a".to_string(),
            artifact_path: "/tmp/A.md".to_string(),
            cause: "epoch".to_string(),
            trace_id: "trace-same-pair-2".to_string(),
            expires_at: "2099-01-01T02:00:00Z".to_string(),
        };
        emit_superseded_if_cross_pair(&ctx, existing.as_ref(), &second);
        write_indeterminate_marker(&second, &marker_path).expect("write second marker");

        assert!(
            ctx.drain_events().is_empty(),
            "VP-108 PC5 (negative): same (plugin_name, artifact_path) pair MUST NOT emit \
             marker.cleared(SUPERSEDED) — continuous quarantine of the same target"
        );
    }

    /// `emit_superseded_if_cross_pair` no-op when there is no pre-existing marker
    /// (`existing = None`) — nothing to supersede on a fresh INDETERMINATE write.
    #[test]
    fn test_ADR_048_D4_v1_3_superseded_clear_not_emitted_when_no_existing_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = test_ctx(dir.path(), "sess-no-existing");
        let new_fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "plugin-fresh".to_string(),
            artifact_path: "/tmp/fresh.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-fresh".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        emit_superseded_if_cross_pair(&ctx, None, &new_fields);
        assert!(
            ctx.drain_events().is_empty(),
            "no marker.cleared(SUPERSEDED) when there was no pre-existing marker to supersede"
        );
    }

    // ── VP-108 PC8 / F-P9-001 / F-P12-001: emit_write_tied_audit_events no-emit-on-Err ──

    /// VP-108 Postcondition 8 (F-P12-001 gap closure): the F-P9-001 negative-control
    /// regression test. A cross-pair marker overwrite whose write FAILS must emit
    /// NEITHER `marker.cleared(SUPERSEDED)` for the superseded pair NOR
    /// `marker.written` for the new pair — fabricating either audit record on a
    /// write that never durably happened would be a false NIST AU-3/AU-10 audit
    /// entry (BC-1.18.001 §PC4).
    ///
    /// Calls [`emit_write_tied_audit_events`] DIRECTLY (not through a real
    /// filesystem failure) with a synthesized `Err(io::Error)` as `write_result`,
    /// and `existing = Some(&pair_a)` for a DIFFERENT `(plugin_name, artifact_path)`
    /// pair than `fields` (pair B) — proving the cross-pair SUPERSEDED path WOULD
    /// fire on `Ok(())` (see the sibling positive control,
    /// `test_ADR_048_D4_v1_3_superseded_clear_emits_for_cross_pair_overwrite`, which
    /// already covers that `Ok` path and is intentionally not duplicated here), and
    /// that the absence of both events on `Err(_)` is caused by the write failure,
    /// not by pair_a/fields already being the same pair.
    #[test]
    fn test_ADR_048_D4_PC8_no_emit_on_cross_pair_write_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let ctx = test_ctx(dir.path(), "sess-pc8-write-failure");

        let pair_a = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "plugin-a".to_string(),
            artifact_path: "/tmp/A.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-pc8-pair-a".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        let pair_b = MarkerFields {
            timestamp: "2026-08-31T01:00:00Z".to_string(),
            plugin_name: "plugin-b".to_string(),
            artifact_path: "/tmp/B.md".to_string(),
            cause: "epoch".to_string(),
            trace_id: "trace-pc8-pair-b".to_string(),
            expires_at: "2099-01-01T01:00:00Z".to_string(),
        };

        // Synthesize a failing write result without touching the filesystem —
        // the whole point of threading `write_result` in as a parameter
        // (see emit_write_tied_audit_events's doc comment) is that this path
        // is directly unit-testable this way.
        let write_result: io::Result<()> =
            Err(io::Error::other("forced write failure (VP-108 PC8 test)"));

        emit_write_tied_audit_events(&ctx, write_result, &marker_path, Some(&pair_a), &pair_b);

        assert!(
            ctx.drain_events().is_empty(),
            "VP-108 PC8 / F-P9-001: a cross-pair overwrite whose write FAILS must emit \
             NEITHER marker.cleared(SUPERSEDED) for the superseded pair NOR marker.written \
             for the new pair — no fabricated audit record on write failure"
        );
    }

    // ── F-P3-004: orphan .tmp file on rename failure ──

    /// F-P3-004 (direct unit test of the cleanup branch): simulate a rename
    /// failure by racing a directory into existence at the marker's final path
    /// AFTER the temp file is written but represented here via a controlled
    /// two-step marker_path (a directory) so `fs::rename(tmp, marker_path)`
    /// fails with `IsADirectory`/`ENOTEMPTY`-class errors while the temp file
    /// write itself succeeded — the exact orphan scenario F-P3-004 fixes.
    #[cfg(unix)]
    #[test]
    fn test_F_P3_004_orphan_tmp_removed_when_rename_target_is_a_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        // Pre-create a NON-EMPTY directory at the marker's final path: on POSIX,
        // `rename` onto a non-empty directory fails (ENOTEMPTY), so the temp
        // file write (which targets a sibling path in the SAME parent as
        // marker_path — the tempdir root) succeeds, but the rename fails.
        std::fs::create_dir_all(&marker_path).expect("create dir at marker_path");
        std::fs::write(marker_path.join("occupant.txt"), b"non-empty")
            .expect("create dir occupant");

        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "validate-factory-path".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-orphan-tmp-2".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };

        let result = write_indeterminate_marker(&fields, &marker_path);
        assert!(
            result.is_err(),
            "precondition: renaming onto a non-empty directory must fail"
        );

        // F-P3-004: no `.tmp` file left behind in the tempdir root (the temp
        // file's parent) after the failed rename.
        let stray_tmp_files: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read tempdir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp"))
            .collect();
        assert!(
            stray_tmp_files.is_empty(),
            "F-P3-004: rename failure MUST NOT orphan the uniquely-named temp file — \
             found {stray_tmp_files:?}"
        );
    }

    // ── ADR-048 §D4 v1.4 F-P6-001: marker.written positive creation record ──

    /// VP-108 PC6 (BC-1.18.001 PC4 v1.4; BC-3.08.001 Event 10; ADR-048 §D4 v1.4;
    /// S-25.01 adversary pass 6 F-P6-001 MEDIUM): a successful marker write
    /// emits `marker.written`; a failed marker write emits nothing.
    #[test]
    fn test_marker_written_emitted_on_successful_write_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "validate-factory-path-staging".to_string(),
            artifact_path: "/tmp/.factory/STATE.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "deadbeef-0108-0006-0006-000000000006".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };

        // Success path: write succeeds -> emit_marker_written IS called by the caller.
        let ctx = test_ctx(dir.path(), "session-vp108-pc6-ok");
        write_indeterminate_marker(&fields, &marker_path)
            .expect("VP-108 PC6: write must succeed for this fixture");
        emit_marker_written(&ctx, &fields);

        let events = ctx.drain_events();
        assert_eq!(
            events.len(),
            1,
            "VP-108 PC6: exactly one marker.written event on success"
        );
        let ev = &events[0];
        assert_eq!(ev.type_, "marker.written");
        assert_eq!(
            ev.dispatcher_trace_id.as_deref(),
            Some(fields.trace_id.as_str())
        );
        assert_eq!(
            ev.plugin_name.as_deref(),
            Some("validate-factory-path-staging")
        );
        assert_eq!(
            ev.fields.get("artifact_path").and_then(|v| v.as_str()),
            Some("/tmp/.factory/STATE.md")
        );
        assert_eq!(
            ev.fields.get("cause").and_then(|v| v.as_str()),
            Some("fuel")
        );
        assert_eq!(
            ev.fields.get("expires_at").and_then(|v| v.as_str()),
            Some("2099-01-01T00:00:00Z")
        );

        // Failure path: write fails (unwritable directory) -> caller MUST NOT call
        // emit_marker_written. This assertion documents the CALLER contract (the
        // marker-write callsite in executor.rs), not a claim that
        // write_indeterminate_marker itself emits anything -- it never does, per
        // its own function contract.
        let unwritable_marker_path = dir.path().join("does-not-exist").join("marker");
        let ctx2 = test_ctx(dir.path(), "session-vp108-pc6-err");
        let write_result = write_indeterminate_marker(&fields, &unwritable_marker_path);
        assert!(
            write_result.is_err(),
            "VP-108 PC6 fixture: write must fail (parent dir absent)"
        );
        // No emit_marker_written call on this path -- implementer directive (ADR-048 §D4 v1.4).
        let events2 = ctx2.drain_events();
        assert_eq!(
            events2.len(),
            0,
            "VP-108 PC6: a failed marker write MUST NOT produce a marker.written event"
        );
    }

    /// VP-108 PC7 (negative control; BC-1.18.001 INV4; ADR-048 §D4 v1.4;
    /// S-25.01 adversary pass 6 F-P6-001 MEDIUM direct regression test): a
    /// PreToolUse fail-closed `plugin.indeterminate` (which never triggers a
    /// marker write, so never emits `marker.written`) does NOT cause
    /// `reconcile_raw_delete` to fabricate `marker.cleared(OPERATOR_OVERRIDE)`.
    /// This is the exact shape `validate-factory-path-staging` (PreToolUse
    /// `^Bash$`, fail-closed, Cohort A-immediate) produces on fuel exhaustion.
    #[test]
    fn test_pretooluse_indeterminate_without_marker_written_does_not_fabricate_operator_override() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = test_ctx(dir.path(), "session-vp108-pc7");
        // Seed ONLY a raw plugin.indeterminate (fail-closed) record -- deliberately
        // NOT calling emit_marker_written, simulating a PreToolUse fail-closed
        // INDETERMINATE that never attempted a marker write (BC-1.18.001 INV4/PC4).
        let ev = InternalEvent::now("plugin.indeterminate")
            .with_trace_id("deadbeef-0108-0007-0007-000000000007")
            .with_session_id("session-vp108-pc7")
            .with_plugin_name("validate-factory-path-staging")
            .with_field("artifact_path", "")
            .with_field("cause", "fuel")
            .with_field("failure_policy", "fail-closed");
        ctx.emit_internal(ev);
        // No marker exists -- correctly so, since none was ever written (not "raw-deleted").
        assert!(!dir.path().join("unvalidated-mutation.marker").exists());

        reconcile_raw_delete(dir.path(), &ctx)
            .expect("VP-108 PC7: reconcile_raw_delete must not error");

        let events = ctx.drain_events();
        let fabricated_override = events.iter().any(|e| {
            e.type_ == "marker.cleared"
                && e.fields.get("clear_mode").and_then(|v| v.as_str()) == Some("OPERATOR_OVERRIDE")
                && e.plugin_name.as_deref() == Some("validate-factory-path-staging")
        });
        assert!(
            !fabricated_override,
            "VP-108 PC7 (F-P6-001 regression): a PreToolUse fail-closed plugin.indeterminate with \
             NO marker.written MUST NOT produce a fabricated marker.cleared(OPERATOR_OVERRIDE) — \
             no marker was ever written, so there is nothing to have been raw-deleted"
        );
    }
}
