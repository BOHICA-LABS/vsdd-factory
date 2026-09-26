//! Pure module bridging ADR-052 §Decision 3's closed argument grammar (which
//! names the `backfill-append-logs` migration's four target FILES, never
//! stems) to BC-1.18.008's existing per-stem marker-regex oracle
//! ([`crate::shard_manager::mechanism_a_record_boundary_offsets`]).
//!
//! S-25.06 / BC-1.18.013. No disk I/O — every function here takes
//! already-read file content or a caller-supplied relative path and
//! performs pure computation only; the caller (`executor.rs`'s governed
//! migration envelope) owns all filesystem access.
//!
//! Architecture Compliance Rule 3 (S-25.06 story): this module does NOT
//! reimplement BC-1.18.008's marker regexes. It reuses
//! [`crate::shard_manager::mechanism_a_record_boundary_offsets`] unmodified
//! after resolving a target file's relative name to its BC-1.18.008
//! artifact stem.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// The ADR-052 §Decision 8 ratified cycle directory for this migration's
/// fixed four-file scope (BC-1.18.013 Postcondition 6). Never a different
/// cycle without a separate ADR-052 allowlist amendment (Postcondition 6 /
/// Invariant 5; S-25.06 AC-008).
pub const APPEND_LOG_CYCLE_DIR: &str = "v1.0-brownfield-backfill";

/// The exact four canonical target files, in the FIXED order BC-1.18.013
/// Postcondition 3a's once-only fingerprint recheck concatenates them in.
/// Relative to `.factory/cycles/<APPEND_LOG_CYCLE_DIR>/`. This is the
/// complete, closed set (BC-1.18.013 Postcondition 6 / Invariant 5) — never
/// a wildcard, never auto-discovered, never a different cycle's files.
pub const APPEND_LOG_TARGET_FILES_IN_ORDER: [&str; 4] = [
    "decision-log.md",
    "burst-log.md",
    "lessons.md",
    "session-checkpoints.md",
];

/// Failures raised while resolving a target file's marker stem or
/// record-boundary offsets. Distinct from
/// [`crate::executor::AppendLogMigrationError`] (the migration-lifecycle
/// error type) — this module's errors are pure-computation failures with
/// no process-exit-code semantics of their own; the governed-migration
/// envelope maps them onto the appropriate `AppendLogMigrationError`
/// variant (`ContentPreservationAbort` / `EmptyBoundaryOracleAbort`) at its
/// own call site.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppendLogMarkerError {
    /// `relative_path` is not one of [`APPEND_LOG_TARGET_FILES_IN_ORDER`]
    /// (BC-1.18.013 Postcondition 6 / Invariant 5 — never a wildcard,
    /// never auto-discovered).
    #[error(
        "append_log_markers: \"{relative_path}\" is not one of the four ratified \
         backfill-append-logs target files"
    )]
    UnknownTargetFile { relative_path: String },

    /// EC-007: `record_boundary_offsets` yielded zero boundaries for
    /// non-empty content — BC-1.18.008 Postcondition 6's fail-loud gate,
    /// surfaced by the governed migration as `CONTENT_PRESERVATION_ABORT`
    /// for the whole migration (S-25.06 AC-014).
    #[error(
        "append_log_markers: record_boundary_offsets yielded zero boundaries for non-empty \
         content in \"{relative_path}\" (EC-007)"
    )]
    EmptyBoundaryOracle { relative_path: String },
}

/// Maps one of the four fixed target files (by its relative filename under
/// [`APPEND_LOG_CYCLE_DIR`]) to its BC-1.18.008 artifact stem — the same
/// stem vocabulary
/// [`crate::shard_manager::mechanism_a_record_boundary_offsets`]'s own
/// match arms use (`"decision-log"`, `"burst-log"`, `"lessons"`,
/// `"session-checkpoints"`). Pure; no I/O. Rejects anything outside
/// [`APPEND_LOG_TARGET_FILES_IN_ORDER`] (BC-1.18.013 Postcondition 6).
pub fn artifact_stem_for_target_file(
    relative_path: &str,
) -> Result<&'static str, AppendLogMarkerError> {
    match relative_path {
        "decision-log.md" => Ok("decision-log"),
        "burst-log.md" => Ok("burst-log"),
        "lessons.md" => Ok("lessons"),
        "session-checkpoints.md" => Ok("session-checkpoints"),
        _ => Err(AppendLogMarkerError::UnknownTargetFile {
            relative_path: relative_path.to_string(),
        }),
    }
}

/// Computes `record_boundary_offsets` for one target file's content by
/// resolving `relative_path` to its stem via
/// [`artifact_stem_for_target_file`] and delegating to
/// [`crate::shard_manager::mechanism_a_record_boundary_offsets`]
/// (Architecture Compliance Rule 3 — reused unmodified, never
/// reimplemented). Surfaces EC-007 as
/// [`AppendLogMarkerError::EmptyBoundaryOracle`] when the oracle yields
/// zero boundaries for non-empty `content`.
pub fn record_boundary_offsets_for_target_file(
    relative_path: &str,
    content: &[u8],
) -> Result<Vec<usize>, AppendLogMarkerError> {
    let stem = artifact_stem_for_target_file(relative_path)?;
    let offsets = crate::shard_manager::mechanism_a_record_boundary_offsets(stem, content);
    if offsets.is_empty() && !content.is_empty() {
        return Err(AppendLogMarkerError::EmptyBoundaryOracle {
            relative_path: relative_path.to_string(),
        });
    }
    Ok(offsets)
}

/// Resolves a target file's relative name to its absolute canonical path
/// under `<repo_root>/.factory/cycles/<APPEND_LOG_CYCLE_DIR>/`. Pure path
/// arithmetic only — existence/readability is EC-005's concern, checked by
/// the governed-migration envelope before the flock is acquired
/// (S-25.06 AC-012), not by this function.
///
/// GREEN-BY-DESIGN (BC-5.38.002): zero branching, no I/O, no calls to
/// non-trivial helpers (only `Path::join`, a primitive path operation), 3
/// lines — see the stub commit report.
pub fn canonical_path_for_target_file(repo_root: &Path, relative_path: &str) -> PathBuf {
    repo_root
        .join(".factory/cycles")
        .join(APPEND_LOG_CYCLE_DIR)
        .join(relative_path)
}
