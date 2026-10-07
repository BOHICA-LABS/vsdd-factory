//! Error taxonomy for `last-amended-migrate`.
//!
//! Named error variants for every fallible operation this crate performs, per
//! CLAUDE.md's "No `unwrap()`/`expect()` in critical code paths" convention.
//! Pure data declarations — the BC-5.38.001 `todo!()` obligation governs
//! function *bodies*, not type/variant declarations, so these are written out
//! in full at stub stage (mirrors every sibling crate's `error.rs`/`lib.rs`
//! error-enum convention, e.g. `factory-lock`, `validate-dispatch-advance`).

use std::path::PathBuf;
use thiserror::Error;

/// Errors surfaced by `last-amended-migrate`'s migration, rotation, and
/// registry-update operations (BC-10.13.001).
#[derive(Debug, Error)]
pub enum MigrateError {
    /// A filesystem read/write failed.
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `path`'s frontmatter could not be isolated/parsed.
    #[error("frontmatter parse error in {path}: {reason}")]
    FrontmatterParse { path: PathBuf, reason: String },

    /// `path`'s `last_amended` field cannot be located at all in frontmatter
    /// (BC-10.13.001 v1.1 EC-008 — the ONLY remaining `NotEligible` outcome
    /// as of the v1.1 full-recovery-split amendment; an inline `[Prior:
    /// ...]` chain is no longer this variant's trigger — see
    /// `crate::eligibility::Eligibility::PriorChainSplit`, which is now an
    /// ELIGIBLE, not an error, classification. A corrupted/unparseable
    /// frontmatter delimiter is instead reported via `FrontmatterParse`.)
    #[error(
        "{path} is not eligible for migration: last_amended field could not \
        be located in frontmatter (BC-10.13.001 v1.1 EC-008 — the only \
        remaining NotEligible outcome; a malformed-frontmatter condition, \
        not a chain-length or chain-presence condition)"
    )]
    NotEligible { path: PathBuf },

    /// A value this tool was about to write failed strict YAML `safe_load`
    /// validation (BC-10.13.001 Invariant 4 / PC3).
    #[error("YAML produced by this tool failed strict safe_load validation: {reason}")]
    InvalidYamlProduced { reason: String },

    /// Writing to `plugins/vsdd-factory/config/artifact-path-registry.yaml`
    /// failed (S-15.03 AC-006).
    #[error("artifact-path-registry.yaml write error: {reason}")]
    RegistryWrite { reason: String },

    /// A CLI-supplied `--path`/`--registry` argument failed the path
    /// allowlist check (S-15.03 SEC-002, CWE-73) — rejected before any file
    /// I/O against its content occurs.
    #[error("{path} is not an allowed target for this operation: {reason}")]
    PathNotAllowed { path: PathBuf, reason: String },

    /// A caller-supplied path parameter failed a structural validation check
    /// (e.g. CWE-22 path-traversal guard on `archive_path` in
    /// `rotate_changelog_at`). No file I/O has occurred when this variant is
    /// returned.
    #[error("{path}: invalid path argument — {reason}")]
    InvalidPath { path: PathBuf, reason: String },

    /// PC7's full-recovery split found an inline `[Prior: ...]` chain on
    /// `STATE.md`, but the caller did not pass the explicit opt-in
    /// (`--discard-state-chain` at the CLI boundary /
    /// `MigrationOptions::discard_state_chain` in the library API) required
    /// to authorize dropping the chained entries (S-15.03 pr-reviewer B2-R;
    /// BC-10.13.001 EC-006). Unlike the other 4 governed files, `STATE.md`
    /// has no `changelog:` field to relocate the entries into (PC1/ADR-049
    /// Decision 4), and PC6 forbids writing them into the frozen
    /// `STATE-amendment-history.md` sidecar — so proceeding would
    /// permanently discard the chained text with no real destination. The
    /// tool refuses by default rather than silently dropping it; no file is
    /// mutated when this variant is returned.
    #[error(
        "{path} has {entries} chained historical entry/entries in last_amended \
        that would be PERMANENTLY DISCARDED by a PC7 full-recovery split — \
        STATE.md has no changelog: field to relocate them into (ADR-049 \
        Decision 4), and this tool never writes to the frozen \
        STATE-amendment-history.md sidecar (BC-10.13.001 PC6). Re-run with \
        --discard-state-chain (CLI) or MigrationOptions::discard_state_chain \
        = true (library) to explicitly acknowledge the loss and proceed, or \
        first manually copy the chained entries' substantive text into \
        STATE.md's own '## Decisions Log' section before splitting."
    )]
    StateChainDiscardNotAuthorized { path: PathBuf, entries: usize },
}
