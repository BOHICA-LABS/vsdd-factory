//! S-25.08 — the ONE shared native admission core (ADR-052 v1.18 §Decision 5a
//! "Shared protected-path union and shared admission state"; BC-1.18.013
//! Precondition 6(b)/(c); BC-1.18.011 Precondition 6(c)/(d)).
//!
//! Both governed one-time migrations (`migrate-bc-index`, `backfill-append-logs`)
//! share ONE `exclusive.lock`, ONE `gate-state.json`, ONE `txn-*.json`
//! directory and ONE `reservations/` directory, and the protected path set is
//! the UNION `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/`.
//! Admission is therefore implemented ONCE, here. The dispatcher's
//! PreToolUse path (`main.rs` → `executor::bc_index_migration_admission`)
//! evaluates it exactly once per event, structurally ahead of
//! `shard_cap_precheck` and every registry plugin; the B2 in-process entry
//! point (`admit_or_block_bc_index_writer`) delegates to it too — there is no
//! second reserve/verify implementation.
//!
//! # Reserve-then-verify (ADR-052 §5a step 0; closes D5)
//!
//! 1. **W1** — create `reservations/<tool_use_id>.reservation`
//!    (`{created_at, tool_use_id}`, atomic temp+rename, no PID) FIRST;
//! 2. **W2** — only THEN read `gate-state.json` and scan `txn-*.json`;
//! 3. if verification fails, run the §5a step-3.5 reconciliation
//!    ([`reconcile_stale_admission_gate`]: Branch A gate repair, Branch B
//!    null-generation STAGING discard, Branch C terminal-record decision) and
//!    re-evaluate;
//! 4. if it still fails, remove the admitter's OWN reservation before
//!    returning the `E-MAINTENANCE-001` block.
//!
//! The admitter holds NO lock for ordering (the coordinator replaces
//! `gate-state.json` via `rename`, so a lock on the replaced inode would not
//! serialize against an admitter holding the old inode). Race-freedom is the
//! Dekker argument: W1 < W2 against the coordinator's C1 (durable DRAINING
//! flip) < C2 (read `reservations/`) — never both miss. The `exclusive.lock`
//! flock is taken ONLY inside reconciliation, as coordinator-vs-reconciler
//! mutual exclusion; a held lock (EWOULDBLOCK) means a live coordinator: no
//! action, block.
//!
//! # Purity
//!
//! The admit/block/branch DECISIONS are the pure functions in the parent
//! module ([`super::is_bc_index_admission_open`],
//! [`super::plan_stale_gate_reconciliation`] (Branch A/B/C selection, built on
//! [`super::decide_terminal_record_reconciliation`]),
//! [`super::reservation_is_stale`]); this module is the effectful shell around
//! them (reservation files, gate/txn reads, flock-gated reconciliation).
//!
//! # Test-only admission seam
//!
//! `VSDD_TEST_ADMISSION_SEAM_DIR` (see [`seam`]) lets the black-box suite
//! observe and interleave W1/W2. It is compiled ONLY in
//! `#[cfg(any(debug_assertions, feature = "test-support"))]` builds — the same
//! gate as `VSDD_FORCE_ENGINE_BUILD_FAILURE` in `main.rs` — and is a no-op
//! that does not even name the env var in shipped release builds.

use std::path::{Path, PathBuf};

use super::{
    BcIndexAdmissionGateState, BcIndexMigrationError, BcIndexMigrationTxnRecord,
    BcIndexMigrationTxnState, StaleGateReconciliationPlan, TerminalReconcileInputs,
    WriterReservation, is_bc_index_admission_open, migrate_err_to_io,
    plan_stale_gate_reconciliation, read_admission_gate_state, try_acquire_migration_lock,
    write_admission_gate_state,
};

/// The `migration_id` of the B2 BC-INDEX migration. A txn record that lacks the
/// field (written by pre-v1.11 code) is read as this value (BC-1.18.011
/// Precondition 6(e)).
pub const MIGRATION_ID_B2: &str = "migrate-bc-index";

/// The `migration_id` of the mechanism-A append-log backfill migration.
pub const MIGRATION_ID_APPEND_LOG: &str = "backfill-append-logs";

/// The `abort_reason` marker Branch B writes when it discards a
/// null-generation STAGING txn (BC-1.18.011 Precondition 6(d) "Branch B
/// marker"). Informational/audit only — no decision may consult it.
pub const ABORT_REASON_NULL_GENERATION: &str = "null_generation";

/// The path family a protected write targets. The `E-MAINTENANCE-001`
/// `<scope>` token is keyed on THIS (never on the live migration — a
/// gate-only block has no txn to key on): BC-1.18.013 Precondition 6(b).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedPathFamily {
    /// A path under `.factory/specs/behavioral-contracts/` (`BC-INDEX` scope).
    BcIndex,
    /// A path under `.factory/cycles/` (`.factory/cycles/` scope).
    Cycles,
}

impl ProtectedPathFamily {
    /// Classify one write target against the protected-path union, anchored to
    /// the session's own `factory_root` (ADR-052 v1.20 §5a "Admission scope
    /// anchoring" / "Target path resolution").
    ///
    /// Classification is COMPONENT-WISE and CASE-INSENSITIVE: the target is
    /// `BcIndex`-family iff it equals or descends from
    /// `<factory_root>/specs/behavioral-contracts`, `Cycles`-family iff it equals
    /// or descends from `<factory_root>/cycles`; never a substring test. A write
    /// is in scope when EITHER the resolved comparison (`T_real` vs the real
    /// `factory_root`) OR the lexical comparison (`T_lex` vs the lexical
    /// `factory_root`) matches — the union is deliberately fail-closed. Any other
    /// `.factory/…` path (another project's, a nested project's, a look-alike) is
    /// OUT of scope (`None`).
    #[must_use]
    pub fn classify_target(
        target_real: Option<&Path>,
        target_lex: &Path,
        root: &FactoryRoot,
    ) -> Option<Self> {
        let by_real = match (target_real, root.real.as_deref()) {
            (Some(t), Some(r)) => Self::classify_under(t, r),
            _ => None,
        };
        by_real.or_else(|| {
            std::iter::once(&root.lex)
                .chain(root.lex_aliases.iter())
                .find_map(|lex_root| Self::classify_under(target_lex, lex_root))
        })
    }

    fn classify_under(target: &Path, factory_root: &Path) -> Option<Self> {
        let target: Vec<_> = target.components().collect();
        let root: Vec<_> = factory_root.components().collect();
        if target.len() < root.len()
            || !target
                .iter()
                .zip(root.iter())
                .all(|(t, r)| components_fold_eq(t.as_os_str(), r.as_os_str()))
        {
            return None;
        }
        let rest = &target[root.len()..];
        let name = |i: usize| rest.get(i).map(|c| c.as_os_str());
        let eq =
            |i: usize, want: &str| name(i).is_some_and(|n| components_fold_eq(n, want.as_ref()));
        if eq(0, "specs") && eq(1, "behavioral-contracts") {
            Some(Self::BcIndex)
        } else if eq(0, "cycles") {
            Some(Self::Cycles)
        } else {
            None
        }
    }

    /// The `<scope>` token of the `E-MAINTENANCE-001` message.
    #[must_use]
    pub fn scope(self) -> &'static str {
        match self {
            Self::BcIndex => "BC-INDEX",
            Self::Cycles => ".factory/cycles/",
        }
    }
}

/// Case-insensitive path-component equality (ADR-052 v1.20 "Target path
/// resolution" (e)): ASCII case-fold for ASCII, `str::to_lowercase` equality for
/// other UTF-8, bytewise ASCII-fold for non-UTF-8. ALWAYS folds — no filesystem
/// case-sensitivity probe (sensitivity is a per-volume property, a probe is
/// racy, and probe-guided exact compare would let `.FACTORY/Cycles/x` bypass the
/// gate on a case-insensitive volume).
fn components_fold_eq(a: &std::ffi::OsStr, b: &std::ffi::OsStr) -> bool {
    match (a.to_str(), b.to_str()) {
        (Some(a), Some(b)) => a.to_lowercase() == b.to_lowercase(),
        _ => a
            .as_encoded_bytes()
            .eq_ignore_ascii_case(b.as_encoded_bytes()),
    }
}

/// Maximum symlink hops [`resolve_target_path`] follows before declaring the
/// real form unavailable (POSIX `MAXSYMLINKS` is 40 on Linux, 32 on macOS).
const MAX_SYMLINK_HOPS: usize = 40;

/// ADR-052 v1.20 §5a "Target path resolution" (F-003): the shared function that
/// returns `(T_real, T_lex)` for one path.
///
/// * `T_lex` — lexical form: collapse `//`, drop `.`, apply `..` lexically (never
///   above the root); NO filesystem access. On Unix `\` is an ordinary name
///   byte; on Windows `Path::components` treats it as a separator.
/// * `T_real` — POSIX-correct resolved form: components are walked left to right
///   keeping a symlink-free `resolved` prefix; `..` pops `resolved` (after any
///   preceding symlink was resolved — `a/link/../b` resolves `link` FIRST); a
///   symlink is read and its target spliced in front of the remaining components
///   (an absolute target resets `resolved` to its root), bounded to
///   [`MAX_SYMLINK_HOPS`]; on the first `NotFound` that component and ALL remaining
///   ones are applied lexically (a nonexistent component cannot be a symlink —
///   this realpaths the deepest existing ancestor and appends the unresolved
///   tail). Any non-`NotFound` error (EACCES, ELOOP, ENOTDIR) or an exceeded hop
///   limit makes `T_real` unavailable (`None`): the caller then classifies on
///   `T_lex` alone, so a protected-looking path that cannot be resolved is
///   treated as protected (fail-closed).
#[must_use]
pub fn resolve_target_path(path: &Path) -> (Option<PathBuf>, PathBuf) {
    (resolve_real(path), lexical_normalize(path))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    let mut anchored = false;
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                out.push(component.as_os_str());
                anchored = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                // Pop a real name; never pop above the root / prefix; a relative
                // leading `..` is preserved.
                if out.file_name().is_some_and(|n| n != "..") {
                    out.pop();
                } else if !anchored {
                    out.push("..");
                }
            }
            Component::Normal(name) => out.push(name),
        }
    }
    out
}

fn resolve_real(path: &Path) -> Option<PathBuf> {
    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::path::Component;

    enum Step {
        Parent,
        Name(OsString),
    }
    fn steps(path: &Path, queue: &mut VecDeque<Step>, front: bool) -> Option<PathBuf> {
        // Returns the root/prefix of `path` when it is absolute.
        let mut root = PathBuf::new();
        let mut collected: Vec<Step> = Vec::new();
        let mut has_root = false;
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    root.push(component.as_os_str());
                    has_root = true;
                }
                Component::CurDir => {}
                Component::ParentDir => collected.push(Step::Parent),
                Component::Normal(n) => collected.push(Step::Name(n.to_os_string())),
            }
        }
        if front {
            for step in collected.into_iter().rev() {
                queue.push_front(step);
            }
        } else {
            queue.extend(collected);
        }
        has_root.then_some(root)
    }

    let mut queue: VecDeque<Step> = VecDeque::new();
    let mut resolved = steps(path, &mut queue, false).unwrap_or_default();
    let mut floor = resolved.clone();
    let mut hops = 0usize;
    let mut in_tail = false;

    while let Some(step) = queue.pop_front() {
        match step {
            Step::Parent => {
                if resolved != floor {
                    resolved.pop();
                }
            }
            Step::Name(name) => {
                let candidate = resolved.join(&name);
                if in_tail {
                    resolved = candidate;
                    continue;
                }
                match std::fs::symlink_metadata(&candidate) {
                    Ok(meta) if meta.file_type().is_symlink() => {
                        hops += 1;
                        if hops > MAX_SYMLINK_HOPS {
                            return None;
                        }
                        let target = std::fs::read_link(&candidate).ok()?;
                        if let Some(new_root) = steps(&target, &mut queue, true) {
                            resolved = new_root.clone();
                            floor = new_root;
                        }
                    }
                    Ok(_) => resolved = candidate,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        in_tail = true;
                        resolved = candidate;
                    }
                    Err(_) => return None,
                }
            }
        }
    }
    Some(resolved)
}

/// The session's own factory root: where its governed-migration namespace
/// (`<factory_root>/migration-state`) lives (ADR-052 v1.20 §5a "Admission scope
/// anchoring"). `real` is the symlink-free form (`None` when it could not be
/// resolved), `lex` the lexical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryRoot {
    pub real: Option<PathBuf>,
    pub lex: PathBuf,
    /// Additional LEXICAL spellings of the same root (e.g. the un-canonicalized
    /// `CLAUDE_PROJECT_DIR` spelling the harness uses in its `file_path`s, which
    /// differs from the canonical form wherever a path component is a symlink,
    /// such as macOS `/var` -> `/private/var`). Part of the fail-closed union:
    /// a target whose lexical form matches ANY spelling is in scope.
    pub lex_aliases: Vec<PathBuf>,
}

impl FactoryRoot {
    /// `<factory_root>/migration-state`, anchored on the REAL form when available
    /// so a worktree whose `.factory` is a symlink to a shared checkout and the
    /// main session share ONE namespace.
    #[must_use]
    pub fn migration_state_dir(&self) -> PathBuf {
        self.real
            .as_deref()
            .unwrap_or(&self.lex)
            .join("migration-state")
    }
}

impl FactoryRoot {
    /// Add another lexical spelling of the factory root (lexically normalised).
    pub fn add_lexical_spelling(&mut self, factory_root_spelling: &Path) {
        let lex = lexical_normalize(factory_root_spelling);
        if lex != self.lex && !self.lex_aliases.contains(&lex) {
            self.lex_aliases.push(lex);
        }
    }
}

/// Resolve the session's `factory_root` = `resolve_target_path(project_root/.factory)`.
/// `None` when that does not exist as a directory — the gate is then OUT OF SCOPE
/// for the dispatch (no migration can be in flight without it) and NEVER creates
/// `.factory` itself.
#[must_use]
pub fn resolve_factory_root(project_root: &Path) -> Option<FactoryRoot> {
    let dot_factory = project_root.join(".factory");
    if !std::fs::metadata(&dot_factory).is_ok_and(|m| m.is_dir()) {
        return None;
    }
    let (real, lex) = resolve_target_path(&dot_factory);
    Some(FactoryRoot {
        real,
        lex,
        lex_aliases: Vec::new(),
    })
}

/// ADR-052 v1.20 §5a "`tool_use_id` presence and validity": an ABSENT key or JSON
/// `null` degrades to check-only admission (`Ok(None)`); a PRESENT value that is
/// not a string, is empty, or violates the grammar is
/// [`BcIndexMigrationError::InvalidToolUseId`] (the admission FAILS CLOSED —
/// never silently downgraded to check-only). The error carries the byte length
/// only, never the raw value.
///
/// # Errors
/// `InvalidToolUseId` for a present-but-invalid id.
pub fn classify_tool_use_id(
    value: Option<&serde_json::Value>,
) -> Result<Option<&str>, BcIndexMigrationError> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(id)) if is_valid_tool_use_id(id) => Ok(Some(id.as_str())),
        Some(serde_json::Value::String(id)) => {
            Err(BcIndexMigrationError::InvalidToolUseId { len: id.len() })
        }
        Some(other) => Err(BcIndexMigrationError::InvalidToolUseId {
            len: other.to_string().len(),
        }),
    }
}

/// The exact `E-MAINTENANCE-001` message for a written path family
/// (error-taxonomy v1.35; BC-1.18.013 Precondition 6(b)). With
/// `completion_record_mismatch` (a PreToolUse Branch C verification failure)
/// the message carries the em-dash (U+2014) mismatch suffix; the foreign
/// migration refusal and the live-coordinator (EWOULDBLOCK) block never do.
#[must_use]
pub fn e_maintenance_block_message(
    family: ProtectedPathFamily,
    completion_record_mismatch: bool,
) -> String {
    let base = format!(
        "{} write blocked: migration window active (txn record in STAGING or COMMITTING state); \
         retry after migration completes or aborts",
        family.scope()
    );
    if completion_record_mismatch {
        format!("{base} (completion-record mismatch \u{2014} operator investigation required)")
    } else {
        base
    }
}

/// Is `id` safe to use as a reservation file stem? The harness identifier is
/// interpolated into a file name, so anything that could escape
/// `reservations/` (separators, `..`, NUL, a leading dot) or is unreasonably
/// long is rejected.
#[must_use]
pub fn is_valid_tool_use_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// The outcome of one evaluation of the shared admission core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome {
    /// Admitted. `reservation` is the file this admission created (held until
    /// the matching PostToolUse, or until the dispatcher releases it because a
    /// LATER stage of the same dispatch blocked); `None` when the event carried
    /// no usable `tool_use_id` (check-only admission, nothing to track).
    Admitted { reservation: Option<PathBuf> },
    /// Blocked with `E-MAINTENANCE-001`. The admitter's OWN reservation has
    /// already been removed (release-on-block). `message` is the exact
    /// path-family-keyed message.
    Blocked { message: String },
}

/// What the §5a step-3.5 reconciliation found / did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleGateReconciliation {
    /// `exclusive.lock` is held (EWOULDBLOCK): a live coordinator. No action.
    LiveCoordinator,
    /// Nothing for the reconciler to repair (gate OPEN with no live txn, or a
    /// live txn that is a legitimately running / binary-resumable activation).
    NothingToReconcile,
    /// Branch A: gate ∈ {LOCKED, DRAINING} with no active txn -> gate OPEN.
    GateReopened,
    /// Branch B: STAGING with `generation_id = null` -> txn ABORTED
    /// (`abort_reason: "null_generation"`, retained), then gate OPEN.
    NullGenerationTxnAborted,
    /// Branch C: the live txn belongs to ANOTHER migration (or an unknown
    /// `migration_id`); never finalized. Plain `E-MAINTENANCE-001`.
    ForeignMigrationRefused,
    /// Branch C: the live txn's own terminal record is present but the txn is
    /// not provably finished (STAGING + record is always fail-closed; the
    /// effectful verification is a fail-closed seam in S-25.08). No txn write,
    /// no gate write; `E-MAINTENANCE-001` with the mismatch suffix.
    CompletionRecordMismatch,
}

/// Evaluate the shared admission core for ONE protected-path
/// `Edit`/`Write`/`MultiEdit` PreToolUse event.
///
/// `migration_state_dir` is `<project>/.factory/migration-state`.
/// `tool_use_id` is the harness identifier shared by the PreToolUse /
/// PostToolUse pair; `None` degrades to a check-only admission (no reservation
/// is created or tracked — a deliberate non-blocking degradation backstopped by
/// the §7c step-5 fingerprint recheck, never a fabricated key).
///
/// On `Blocked` and on `Err` the admitter's own reservation has already been
/// removed. On `Admitted` it stands until PostToolUse.
///
/// # Errors
/// An unreadable/malformed gate or txn record, more than one live txn, an
/// invalid `tool_use_id`, or an I/O failure creating the reservation.
pub fn admit_protected_write(
    migration_state_dir: &Path,
    tool_use_id: Option<&str>,
    family: ProtectedPathFamily,
) -> Result<AdmissionOutcome, BcIndexMigrationError> {
    // W1 — reserve FIRST.
    let reservation = match tool_use_id {
        Some(id) => {
            let path = create_writer_reservation(migration_state_dir, id)?;
            seam::record(&format!("W1_RESERVE:{id}"));
            seam::pause_after_w1();
            Some(path)
        }
        None => None,
    };

    // W2 and everything after — verify.
    match verify_admission(migration_state_dir, family) {
        Ok(None) => Ok(AdmissionOutcome::Admitted { reservation }),
        Ok(Some(message)) => {
            remove_own_reservation(reservation.as_deref());
            Ok(AdmissionOutcome::Blocked { message })
        }
        Err(e) => {
            remove_own_reservation(reservation.as_deref());
            Err(e)
        }
    }
}

/// W2: read gate + scan txns; reconcile if not admissible; re-evaluate.
/// `Ok(None)` = admit; `Ok(Some(message))` = block with that exact message.
fn verify_admission(
    migration_state_dir: &Path,
    family: ProtectedPathFamily,
) -> Result<Option<String>, BcIndexMigrationError> {
    let first = AdmissionSnapshot::read(migration_state_dir)?;
    seam::record("W2_VERIFY");
    if first.is_admissible()? {
        return Ok(None);
    }

    let reconciliation = reconcile_stale_admission_gate(migration_state_dir)?;
    let second = AdmissionSnapshot::read(migration_state_dir)?;
    if second.is_admissible()? {
        return Ok(None);
    }

    let live = second.live_txn()?;
    tracing::warn!(
        target: "bc_1_18_011_migration",
        migration_id = live.map(|t| t.migration_id()).unwrap_or("none"),
        txn_id = live.map(|t| t.record.txn_id.as_str()).unwrap_or("none"),
        gate_state = ?second.gate,
        reconciliation = ?reconciliation,
        scope = family.scope(),
        "E-MAINTENANCE-001: protected-path write blocked by the shared admission core"
    );
    let mismatch = reconciliation == StaleGateReconciliation::CompletionRecordMismatch;
    Ok(Some(e_maintenance_block_message(family, mismatch)))
}

/// ADR-052 §Decision 5a step 3.5 — flock-gated stale-gate reconciliation,
/// Branches A/B/C. Wired on the production admission path by
/// [`admit_protected_write`] (never reachable only from tests).
///
/// Runs under `flock(exclusive.lock, LOCK_EX|LOCK_NB)`; EWOULDBLOCK means a
/// live coordinator holds it: no action. Gate / txn state is RE-READ under the
/// lock and acted on as re-read. Which branch fires is decided by the pure
/// [`plan_stale_gate_reconciliation`]; this function performs its effects.
///
/// * **Branch A** — gate ∈ {LOCKED, DRAINING}, no active txn (absent,
///   COMPLETED, ABORTED) -> gate OPEN.
/// * **Branch B** — STAGING with `generation_id = null` (pre-generation
///   crash), this migration's terminal record absent -> the SAME txn file is
///   rewritten in place (one atomic write-temp + fsync + rename + dir-sync) to
///   `state = ABORTED` plus `abort_reason = "null_generation"`, every other
///   field preserved, file retained; THEN gate OPEN (`gate=OPEN ⇒ no live
///   txn`).
/// * **Branch C** — a live txn plus the live txn's own migration's terminal
///   record (selected by `migration_id`): decided by the pure
///   [`decide_terminal_record_reconciliation`]. STAGING + record is ALWAYS
///   fail-closed. The effectful verify-then-finalize behind
///   `FinalizeThenOpenGate` is S-25.06's deliverable; here the verification
///   inputs are reported UNVERIFIED, so a Branch C that cannot verify blocks
///   and never finalizes or flips the gate.
///
/// The reconciler NEVER performs COMMITTING forward recovery (renames) — that
/// is exclusively the migration binary's job.
///
/// # Errors
/// An unreadable/malformed gate or txn record, more than one live txn, or an
/// I/O failure performing a repair write.
pub fn reconcile_stale_admission_gate(
    migration_state_dir: &Path,
) -> Result<StaleGateReconciliation, BcIndexMigrationError> {
    seam::record("RECONCILE");

    let lock_path = migration_state_dir.join("exclusive.lock");
    if !lock_path.exists() {
        std::fs::write(&lock_path, b"").map_err(|source| BcIndexMigrationError::Io {
            path: lock_path.clone(),
            source,
        })?;
    }
    let Some(_coordinator_exclusion) = try_acquire_migration_lock(&lock_path)? else {
        return Ok(StaleGateReconciliation::LiveCoordinator);
    };

    let snapshot = AdmissionSnapshot::read(migration_state_dir)?;
    let live = snapshot.live_txn()?;

    // `None` for no live txn AND for a live txn whose `migration_id` this
    // build does not know (foreign).
    let terminal_record_name = live.and_then(|t| terminal_record_file_name(t.migration_id()));
    let inputs = TerminalReconcileInputs {
        lock_acquired: true,
        // The terminal record is actually READ (own migration's record, selected
        // by `migration_id`): ENOENT => ABSENT; any other read failure => `Io`
        // (E-MAINTENANCE-002 `io`, fail closed — never mistaken for ABSENT, which
        // would let Branch B discard a txn whose record merely could not be
        // read). A record that reads is PRESENT; whether it VERIFIES is the
        // fail-closed seam below (S-25.06 delivers the real verification).
        record_present: match terminal_record_name {
            Some(name) => {
                let record_path = migration_state_dir.join(name);
                match std::fs::read(&record_path) {
                    Ok(_bytes) => true,
                    Err(source) if source.kind() == std::io::ErrorKind::NotFound => false,
                    Err(source) => {
                        return Err(BcIndexMigrationError::Io {
                            path: record_path,
                            source,
                        });
                    }
                }
            }
            None => false,
        },
        txn_state: live.map(|t| t.record.state),
        // The effectful verification (parse the record; txn_id / generation_id
        // / canonical_paths_count equality; every canonical sha256) is
        // S-25.06's deliverable (AC-021/022/023/031). Until it lands the seam
        // reports every check UNVERIFIED, so the pure core can only answer
        // NoOp (record absent), RefuseForeignMigration, or FailClosedMismatch
        // — never FinalizeThenOpenGate.
        record_parses: false,
        txn_id_eq: false,
        generation_id_eq: false,
        count_eq_n: false,
        hashes_eq: [false; 4],
        txn_migration_known: terminal_record_name.is_some(),
    };

    let live_generation_id_is_null = live.is_some_and(|t| t.record.generation_id.is_none());

    // The branch selection is the pure planner (VP-147); this shell only
    // performs the effects it names. `plan` names a txn-bearing branch
    // (B / C) only when `inputs.txn_state` is live, i.e. only when `live` is
    // `Some` (proven by `proof_obl1_h6_terminal_reconcile_idempotence`); the
    // `None` arms below are unreachable and fail closed.
    match (
        plan_stale_gate_reconciliation(snapshot.gate, &inputs, live_generation_id_is_null),
        live,
    ) {
        (StaleGateReconciliationPlan::LiveCoordinator, _) => {
            Ok(StaleGateReconciliation::LiveCoordinator)
        }
        (StaleGateReconciliationPlan::NothingToReconcile, _)
        | (StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate, None) => {
            Ok(StaleGateReconciliation::NothingToReconcile)
        }
        (StaleGateReconciliationPlan::ReopenGate, _) => {
            // Branch A.
            write_admission_gate_state(migration_state_dir, BcIndexAdmissionGateState::Open)?;
            Ok(StaleGateReconciliation::GateReopened)
        }
        (StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate, Some(live)) => {
            // Branch B (terminal record ABSENT; STAGING with generation_id =
            // null; any gate state).
            abort_null_generation_txn(migration_state_dir, live)?;
            Ok(StaleGateReconciliation::NullGenerationTxnAborted)
        }
        (StaleGateReconciliationPlan::RefuseForeignMigration, _) => {
            Ok(StaleGateReconciliation::ForeignMigrationRefused)
        }
        (StaleGateReconciliationPlan::FailClosedMismatch, Some(live)) => {
            tracing::warn!(
                target: "bc_1_18_011_migration",
                migration_id = live.migration_id(),
                txn_id = %live.record.txn_id,
                txn_state = ?live.record.state,
                failing_check = "terminal record present; verification is fail-closed in this \
                                 build (STAGING + record always; COMMITTING unverified)",
                "Branch C: completion-record mismatch -- no txn write, no gate write"
            );
            Ok(StaleGateReconciliation::CompletionRecordMismatch)
        }
        (StaleGateReconciliationPlan::FinalizeThenOpenGate, Some(live)) => {
            // Unreachable while the verification seam reports every check
            // unverified; fail closed (never finalize) rather than panic if a
            // future change makes it reachable before the effect is delivered.
            tracing::error!(
                target: "bc_1_18_011_migration",
                migration_id = live.migration_id(),
                txn_id = %live.record.txn_id,
                "Branch C: FinalizeThenOpenGate decided but the finalize effect is not wired; \
                 failing closed"
            );
            Ok(StaleGateReconciliation::CompletionRecordMismatch)
        }
        (
            StaleGateReconciliationPlan::FailClosedMismatch
            | StaleGateReconciliationPlan::FinalizeThenOpenGate,
            None,
        ) => Ok(StaleGateReconciliation::CompletionRecordMismatch),
    }
}

/// Branch B effect: txn ABORTED + `abort_reason`, THEN gate OPEN.
fn abort_null_generation_txn(
    migration_state_dir: &Path,
    live: &TxnFile,
) -> Result<(), BcIndexMigrationError> {
    let mut raw = live.raw.clone();
    let Some(object) = raw.as_object_mut() else {
        return Err(BcIndexMigrationError::BinaryIntegrityFailure {
            message: format!("txn record {} is not a JSON object", live.path.display()),
        });
    };
    object.insert("state".to_string(), serde_json::json!("ABORTED"));
    object.insert(
        "abort_reason".to_string(),
        serde_json::json!(ABORT_REASON_NULL_GENERATION),
    );
    let json = serde_json::to_string_pretty(&raw).map_err(|e| {
        BcIndexMigrationError::BinaryIntegrityFailure {
            message: format!("failed to serialize aborted txn record: {e}"),
        }
    })?;
    // ONE atomic write-temp + fsync + rename + dir-sync (§7d): no observable
    // "ABORTED without marker" / "marker without ABORTED" intermediate state.
    last_amended_migrate::atomic_write::write_atomic_strict_durable(&live.path, &json).map_err(
        |e| BcIndexMigrationError::Io {
            path: live.path.clone(),
            source: migrate_err_to_io(e),
        },
    )?;
    // Txn first, then gate: `gate=OPEN ⇒ no live txn` is never violated.
    write_admission_gate_state(migration_state_dir, BcIndexAdmissionGateState::Open)
}

/// The per-migration terminal record file (ADR-052 §Decision 7e). `None` for a
/// `migration_id` this build does not know — such a txn is foreign.
fn terminal_record_file_name(migration_id: &str) -> Option<&'static str> {
    match migration_id {
        MIGRATION_ID_B2 => Some("completed.json"),
        MIGRATION_ID_APPEND_LOG => Some("completed-backfill-append-logs.json"),
        _ => None,
    }
}

/// One `txn-*.json` file: the raw JSON (so a rewrite preserves every field,
/// including ones this build does not model) and its typed view.
struct TxnFile {
    path: PathBuf,
    raw: serde_json::Value,
    record: BcIndexMigrationTxnRecord,
}

impl TxnFile {
    /// The record's `migration_id`; absent reads as [`MIGRATION_ID_B2`].
    fn migration_id(&self) -> &str {
        self.raw
            .get("migration_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(MIGRATION_ID_B2)
    }

    fn is_live(&self) -> bool {
        matches!(
            self.record.state,
            BcIndexMigrationTxnState::Staging | BcIndexMigrationTxnState::Committing
        )
    }
}

/// A consistent read of the shared admission state (gate + every txn record).
struct AdmissionSnapshot {
    gate: BcIndexAdmissionGateState,
    txns: Vec<TxnFile>,
}

impl AdmissionSnapshot {
    fn read(migration_state_dir: &Path) -> Result<Self, BcIndexMigrationError> {
        let gate = read_admission_gate_state(migration_state_dir)?;
        let txns = read_txn_files(migration_state_dir)?;
        Ok(Self { gate, txns })
    }

    /// The single live (STAGING/COMMITTING) txn, if any. At most one may exist
    /// across both migrations; more is a genuine integrity violation surfaced
    /// fail-loud rather than silently picking one.
    fn live_txn(&self) -> Result<Option<&TxnFile>, BcIndexMigrationError> {
        let mut live = self.txns.iter().filter(|t| t.is_live());
        let first = live.next();
        if live.next().is_some() {
            let count = self.txns.iter().filter(|t| t.is_live()).count();
            return Err(BcIndexMigrationError::BinaryIntegrityFailure {
                message: format!(
                    "found {count} coexisting LIVE (STAGING/COMMITTING) txn records -- the \
                     writer-exclusion invariant requires at most one active migration in flight \
                     at a time across both governed migrations"
                ),
            });
        }
        Ok(first)
    }

    /// Dual check: gate OPEN AND no live txn (ADR-052 §5a step 4/5).
    fn is_admissible(&self) -> Result<bool, BcIndexMigrationError> {
        let live = self.live_txn()?;
        Ok(is_bc_index_admission_open(
            self.gate,
            live.map(|t| &t.record),
        ))
    }
}

/// Parse every `txn-*.json` in `migration_state_dir` (sorted, deterministic).
fn read_txn_files(migration_state_dir: &Path) -> Result<Vec<TxnFile>, BcIndexMigrationError> {
    let entries = match std::fs::read_dir(migration_state_dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(BcIndexMigrationError::Io {
                path: migration_state_dir.to_path_buf(),
                source,
            });
        }
    };
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| BcIndexMigrationError::Io {
            path: migration_state_dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("txn-") && name.ends_with(".json") {
            paths.push(entry.path());
        }
    }
    paths.sort();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        // Raw bytes, then decode: an OS read failure is `Io`; invalid UTF-8 or
        // malformed JSON is `state_integrity` (BC-1.18.013 v1.9 EC-032).
        let bytes = std::fs::read(&path).map_err(|source| BcIndexMigrationError::Io {
            path: path.clone(),
            source,
        })?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            BcIndexMigrationError::BinaryIntegrityFailure {
                message: format!("malformed txn record at {}: {e}", path.display()),
            }
        })?;
        let record: BcIndexMigrationTxnRecord =
            serde_json::from_value(raw.clone()).map_err(|e| {
                BcIndexMigrationError::BinaryIntegrityFailure {
                    message: format!("malformed txn record at {}: {e}", path.display()),
                }
            })?;
        // A PRESENT non-string `migration_id` (including JSON `null`) is a MALFORMED
        // record — `E-MAINTENANCE-002` `state_integrity` — never "foreign"
        // (BC-1.18.013 v1.8 EC-030). An ABSENT key reads as B2 (serde default).
        if raw
            .get("migration_id")
            .is_some_and(|v| !matches!(v, serde_json::Value::String(_)))
        {
            return Err(BcIndexMigrationError::BinaryIntegrityFailure {
                message: format!(
                    "malformed txn record at {}: migration_id is not a JSON string",
                    path.display()
                ),
            });
        }
        files.push(TxnFile { path, raw, record });
    }
    Ok(files)
}

/// W1: durably create this admission's reservation (atomic temp+rename; body
/// `{created_at, tool_use_id}`, no PID/start-time).
fn create_writer_reservation(
    migration_state_dir: &Path,
    tool_use_id: &str,
) -> Result<PathBuf, BcIndexMigrationError> {
    if !is_valid_tool_use_id(tool_use_id) {
        return Err(BcIndexMigrationError::InvalidToolUseId {
            len: tool_use_id.len(),
        });
    }
    let reservations_dir = migration_state_dir.join("reservations");
    std::fs::create_dir_all(&reservations_dir).map_err(|source| BcIndexMigrationError::Io {
        path: reservations_dir.clone(),
        source,
    })?;
    let reservation = WriterReservation {
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        tool_use_id: tool_use_id.to_string(),
    };
    let json = serde_json::to_string_pretty(&reservation).map_err(|e| {
        BcIndexMigrationError::BinaryIntegrityFailure {
            message: format!("failed to serialize writer reservation: {e}"),
        }
    })?;
    let path = reservations_dir.join(format!("{tool_use_id}.reservation"));
    last_amended_migrate::atomic_write::write_atomic(&path, &json).map_err(|e| {
        BcIndexMigrationError::Io {
            path: path.clone(),
            source: migrate_err_to_io(e),
        }
    })?;
    Ok(path)
}

/// Release-on-block for the admitter's OWN reservation. A failure to remove it
/// is non-fatal: the TTL GC ([`super::reservation_is_stale`]) reclaims it.
fn remove_own_reservation(reservation: Option<&Path>) {
    release_reservation_file(reservation);
}

/// Remove a reservation file (idempotent; a missing file is a no-op). Shared by
/// the admitter's own release-on-block and the dispatcher's release when a
/// LATER stage of the same dispatch blocks (`main.rs`).
pub fn release_reservation_file(reservation: Option<&Path>) {
    let Some(path) = reservation else {
        return;
    };
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            tracing::warn!(
                target: "bc_1_18_011_migration",
                path = %path.display(),
                error = %source,
                "failed to remove a writer reservation (non-fatal); drain's TTL GC will \
                 reclaim it"
            );
        }
    }
}

/// Test-only admission seam (see the module docs). Active ONLY in
/// `debug_assertions` / `feature = "test-support"` builds.
#[cfg(any(debug_assertions, feature = "test-support"))]
mod seam {
    use std::io::Write as _;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    const ENV_ADMISSION_SEAM_DIR: &str = "VSDD_TEST_ADMISSION_SEAM_DIR";
    const PAUSE_MAX: Duration = Duration::from_secs(30);
    const PAUSE_POLL: Duration = Duration::from_millis(10);

    fn dir() -> Option<PathBuf> {
        std::env::var_os(ENV_ADMISSION_SEAM_DIR)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }

    /// Append one event line to `<dir>/events.log`, immediately after the
    /// effect it names. Best-effort: a seam write failure must never change
    /// admission behavior.
    pub(super) fn record(event: &str) {
        let Some(dir) = dir() else {
            return;
        };
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("events.log"))
        {
            let _ = writeln!(file, "{event}");
        }
    }

    /// If `<dir>/pause-after-w1` exists, poll (<= 30 s, 10 ms interval) until
    /// `<dir>/resume` exists, so a test can place coordinator steps BETWEEN W1
    /// and W2.
    pub(super) fn pause_after_w1() {
        let Some(dir) = dir() else {
            return;
        };
        if !dir.join("pause-after-w1").exists() {
            return;
        }
        let start = Instant::now();
        while !dir.join("resume").exists() && start.elapsed() < PAUSE_MAX {
            std::thread::sleep(PAUSE_POLL);
        }
    }
}

/// Release builds: the seam does not exist (the env var name is not compiled
/// in at all).
#[cfg(not(any(debug_assertions, feature = "test-support")))]
mod seam {
    #[inline]
    pub(super) fn record(_event: &str) {}

    #[inline]
    pub(super) fn pause_after_w1() {}
}
