---
document_type: behavioral-contract
level: L3
version: "1.18"
status: active
producer: product-owner
timestamp: 2026-09-05T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md
input-hash: "a7e82d0"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: draft
introduced: v1.0-brownfield-backfill
modified: []
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.011: Governed One-Time Migration for the B2 BC-INDEX Body Split (Content-Preservation, Independent Census, Crash-Atomicity, Rollback)

## Description

BC-1.18.010 specifies mechanism B2's END-STATE (per-subsystem shard files, zero-lookup first-level
addressing, manifest-based second-level sub-sharding) but does not itself specify the TRANSITION
from today's monolithic `BC-INDEX.md` body to that end-state — exactly the same gap BC-1.18.008
closes for mechanism A's four append-log backfills. Because `BC-INDEX.md`'s H1-per-BC-row is the
POLICY-7 title source-of-truth, a dropped or duplicated row during this split corrupts title
authority for that BC — this is a governance-integrity-critical migration, not a cosmetic one, and
is modeled directly on BC-1.18.008's structure: byte-for-byte content-preservation, an independent
census verifying every BC row lands in exactly one shard, staging+verify+atomic-replace
crash-atomicity, fail-loud rollback on verification failure, and idempotency against a partial
prior attempt. This BC additionally covers the SS-05/SS-06 second-level sub-split within the SAME
one-time operation, since both subsystems already exceed the provisional cap on their own section
size alone and require immediate sub-sharding at the same F4 activation moment.

## Preconditions

1. BC-1.18.010's end-state addressing scheme (per-subsystem shard files at
   `shards/BC-INDEX-SS-NN.md`, the top-level shard-manifest schema, and the SS-05/SS-06
   second-level manifest schema) is fully specified and available as the TARGET this migration
   produces.
2. BC-1.18.006's atomic-write primitive (`write_atomic`, the temp-file-then-rename staging/publish
   discipline implemented in `last_amended_migrate::atomic_write::write_atomic` and called in
   `crates/factory-dispatcher/src/shard_manager.rs`) is implemented and available for reuse for
   per-file writes. This BC's crash-atomicity envelope is provided by the NEW multi-file machinery
   introduced by ADR-052 §Decision 7: §Decision 7a (advisory flock on stable never-unlinked inode;
   durable txn record with STAGING/COMMITTING/COMPLETED/ABORTED lifecycle, enforcing writer
   exclusion independent of PID liveness), §Decision 7b (framed checksummed intent log with WAL
   boundary and matching-destination-hash crash recovery), §Decision 7c (CURRENT.json atomic pointer
   swap as the sole commit-point; completed.json as the permanent terminal record). The per-file
   write primitive is reused from BC-1.18.006; the multi-file atomicity infrastructure is new.
3. `BC-INDEX.md`'s live frontmatter `total_bcs` field is readable and is treated as an independent
   count-oracle against which the pre-split census (Postcondition 2) is cross-checked — not as the
   census itself (the census is a fresh enumeration of the body's actual `BC-X.YY.NNN` rows,
   `total_bcs` is a sanity bound the fresh enumeration must match).
4. The migration is independently gated on the F4 activation boundary. It has NO timing or
   ordering dependency on BC-1.18.008's mechanism-A backfill-split; the two migrations activate
   independently (each via its own armed-activation manifest per ADR-052 §Decision 4) and may
   run in any order. They share an F4 activation window by operational convenience, not by
   specification.

5. A durable transaction record at `.factory/migration-state/txn-<activation_uuid>.json`
   and a framed checksummed intent log at
   `.factory/migration-state/intent-<generation_uuid>.log` are maintained across the full
   migration lifecycle:
   - State STAGING: flock held; quiescence reached; staging generation built; intent log
     written with per-target expected hashes + pre-states; all fsync barriers applied;
     authorization gate check pending.
   - State COMMITTING (the pivot): CURRENT.json pointer swap executed atomically; from this
     point forward recovery is mandatory; authorization expiry does NOT abort.
   - State COMPLETED: all canonical path moves complete and hash-verified; completed.json
     written at stable path; PERMANENT.
   EC-003 resume logic reads the intent log + txn record to determine which canonical path
   moves succeeded (matching-destination-hash rule) and resumes from first uncompleted move.

6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution via two
   independent mechanisms:
   (a) Advisory flock on `.factory/migration-state/exclusive.lock` (pre-created, never unlinked):
       the migration binary holds an exclusive flock for the full execution window; kernel
       releases automatically on process death; stale-owner detection is automatic.
   (b) Txn record at `.factory/migration-state/txn-<uuid>.json` with state STAGING or COMMITTING:
       ALL mutation tool calls (Edit/Write/MultiEdit/Bash) targeting BC-INDEX paths are blocked
       by the native admission gate in `executor.rs` (ADR-052 §Decision 5a) when a txn record
       exists in STAGING or COMMITTING state — regardless of whether the flock is currently held.
       This ensures ordinary writers remain blocked even during crash recovery when no process
       holds the flock. **Delivery cross-reference (per D-1236 Ruling 3):** the Edit/Write/MultiEdit
       legs of this admission gate ship with cluster-5 F4 TDD as native `executor.rs` logic. The
       Bash leg is delivered separately, by the ADR-052 §Decision 5c full-command classifier, as
       part of [D-1232-OBL-4] (devops-engineer's dispatcher-guard amendments), deployed at the
       cluster-5 F4 activation boundary — not by cluster-5 TDD itself. Both legs are in force by
       migration execution time; this precondition's "ALL ... Bash" scope holds as stated once
       both deliveries are activated. **Block message and `<scope>` keying (v1.13; normative
       definition and four-cell table in BC-1.18.013 Precondition 6(b) "`E-MAINTENANCE-001`
       `<scope>` keying rule"; this BC, BC-1.18.013 and error-taxonomy v1.35 state the same
       rule).** The `E-MAINTENANCE-001` message is keyed on the WRITTEN PATH FAMILY, not on the
       live txn's migration: for a write under `.factory/specs/behavioral-contracts/` the
       message is exactly `BC-INDEX write blocked: migration window active (txn record in STAGING
       or COMMITTING state); retry after migration completes or aborts` — whether the live txn
       is this migration's (`migrate-bc-index`) or `backfill-append-logs`'s; for a write under
       `.factory/cycles/` while a `migrate-bc-index` txn is live it is exactly
       `.factory/cycles/ write blocked: migration window active (txn record in STAGING or
       COMMITTING state); retry after migration completes or aborts`. (Each message is a single
       line; the line breaks above are editorial.) One write ⇒ one message; the migration's
       identity appears only in the `migration.admission_blocked` InternalLog event
       (field `migration_id`; Postcondition 10). **Verdict surface label (v1.18; normative text
       BC-1.18.013 Precondition 6(b) "Verdict surface label"; ADR-052 §Decision 5a "Admission
       verdict surface label (v1.21)"; closes F-013).** An admission verdict is reported to the
       operator as `blocking_plugins=migration-admission` (`block_reason` = the unchanged
       `E-MAINTENANCE-001`/`E-MAINTENANCE-002` text, exit 2); a shard-cap-gate verdict keeps
       `blocking_plugins=shard-cap-gate`. The gate name is a parameter of
       `shard_gate_verdict_outcomes` and the two outcome synthesizers — the closed enum
       `NativeGate { ShardCap, MigrationAdmission }`; `main.rs`'s admission leg passes
       `MigrationAdmission`, the `shard_cap_precheck` leg `ShardCap` (EC-034).
       **Events, tools and admission scope (v1.16; normative text in BC-1.18.013 Precondition
       6(b) "Which events / tools" sub-bullets (i)–(v), same rules; ADR-052 §Decision 5a
       "Admission scope anchoring" and "Target path resolution"; closes F-002/F-003).** Hook
       events: `PreToolUse` (admission + reservation creation) and `PostToolUse` **and
       `PostToolUseFailure`** (reservation release, (c)). Tools: `Edit`/`Write`/`MultiEdit`; the
       Rust gate does NOT process `Bash` (OBL-4, per the delivery cross-reference above).
       (i) **Anchor — single anchoring rule (v1.18; ADR-052 §Decision 5a "Single anchoring rule
       (v1.21)"; normative text BC-1.18.013 Precondition 6(b)(i); closes D-2)** — project root =
       `resolve_session_project_root(claude_project_dir, process_cwd)` (a PRESENT non-empty
       `CLAUDE_PROJECT_DIR` wins, canonicalized with an as-given fallback — never the cwd; absent
       or empty ⇒ the process cwd exactly; NO ancestor walk, NO `git rev-parse`); factory root =
       `resolve_factory_root(project_root)` (the real form of `<project_root>/.factory` via
       `resolve_target_path`; it MUST exist as a directory, else the gate is out of scope for the
       dispatch and NEVER creates `.factory`); `migration-state/` = `FactoryRoot::migration_state_dir()`
       = `<factory_root>/migration-state`, never derived from the written path. Admission, the
       release leg AND both coordinator binaries (`migrate-bc-index`, `backfill-append-logs`;
       Precondition 7) use this ONE function; a `.factory` symlink resolves to the real root so
       worktree and main sessions share ONE migration-state; migration-state is anchored on the REAL root regardless of which
       spelling of the root matched a target (v1.17). **Lexical root spellings (v1.17;
       ADR-052 §Decision 5a "Lexical root spellings"; normative text BC-1.18.013 6(b)(i)):**
       for the lexical comparison the session's own root has EXACTLY TWO spellings — (a) the
       lexical normalization of the canonical `factory_root`, (b) the lexical normalization of
       `<CLAUDE_PROJECT_DIR as given>/.factory` (raw env value; ignored if empty or
       non-absolute; no filesystem access; deduplicated) — and no other spelling.
       (ii) **Classification** — component-wise: `BcIndex`-family iff the
       target equals or descends from `<factory_root>/specs/behavioral-contracts`, `Cycles`-family
       iff from `<factory_root>/cycles` (for `T_lex`, `<factory_root>` is EITHER lexical spelling
       above); never a substring test. (iii) **Out-of-root `.factory`
       paths** (another project's, a nested project's `sub/.factory`, a scratch tree, a look-alike
       `x.factory/…`) are OUT OF SCOPE: admitted, NO reservation, NO directory creation, NO read
       of migration-state; accepted residual — a session in project A writing into project B's
       protected paths is not serialized by B's gate; B's §Decision 7c step-5 fingerprint recheck
       (`FINGERPRINT_MISMATCH_ABORT`) is the backstop. (iv) **Target path resolution** —
       `resolve_target_path` returns `(T_real, T_lex)`: `T_lex` collapses `//`, drops `.`,
       applies `..` lexically; `T_real` is the POSIX-correct left-to-right walk (symlinks
       spliced in at their position, ≤ 40 hops, the first `NotFound` ends resolution and the
       tail is appended lexically; any other error ⇒ `T_real` unavailable ⇒ classify on `T_lex`
       alone, fail-closed); a write is in scope iff EITHER the resolved OR the lexical form
       matches (the lexical form `T_lex` compared against the lexical normalization of EITHER
       spelling of the session's own root — canonical `factory_root` or
       `<CLAUDE_PROJECT_DIR as given>/.factory` — and no other spelling, v1.17); a relative `file_path` is joined to the payload `cwd` (absolute) else
       `project_root`, `~` never expanded; a missing/non-string/empty/NUL `file_path` is out of
       scope (not an error); `\` is a separator only on Windows; ALL component comparisons are
       ALWAYS case-insensitive with NO filesystem probe. (v) **Residuals** — hard links, bind
       mounts, TOCTOU symlink swap; backstopped by the §7c step-5 fingerprint recheck. The OBL-4
       Bash classifier MUST use the same `resolve_target_path`.
   (c) OPEN/DRAINING gate with writer reservations spanning PreToolUse→tool-completion ensures
       the migration coordinator waits for all in-flight admitted writers to complete before
       snapshotting source files (ADR-052 §Decision 5a). **(v1.11 — reserve-then-verify, release-
       on-block, TTL; ADR-052 v1.18 D5/D2; the merged B2 code deviates and is brought into
       alignment — the SPEC wins.)**
       - **Reserve-then-verify admission order (§Decision 5a step 0).** For every protected-path
         `Edit`/`Write`/`MultiEdit` the dispatcher's PreToolUse admission MUST create
         `.factory/migration-state/reservations/<tool_use_id>.reservation` (atomic temp+rename;
         `{"created_at": "<ISO-8601>", "tool_use_id": "<id>"}`) FIRST, and only THEN read
         `gate-state.json` and scan `txn-*.json` (and run the Precondition 6(d) reconciliation).
         On failed verification (gate ≠ OPEN or a live txn) it removes its own reservation before
         returning the `E-MAINTENANCE-001` block. The admitter takes NO lock; the earlier
         `LOCK_SH`-on-gate-file ordering is superseded (the coordinator's gate flip replaces the
         file by `rename`, so a lock on the replaced inode does not serialize against an admitter
         holding the old inode). Race-freedom is the Dekker ordering W1 (create reservation) <
         W2 (read gate) versus C1 (durable DRAINING flip) < C2 (read `reservations/`): never both
         miss.
       - **Unconditional reservation namespace — first-activation race closed (v1.15; S-25.08
         implementation finding; normative text in BC-1.18.013 Precondition 6(c), same rule).**
         The admitter MUST NOT condition admission, reservation or release on the pre-existence
         of `.factory/migration-state/` or its `reservations/` subdirectory. For every PreToolUse
         `Edit`/`Write`/`MultiEdit` inside the protected union with a valid `tool_use_id` the
         admitter first ensures `migration-state/reservations/` by an IDEMPOTENT recursive
         directory creation (already-exists, including a concurrent creation by another admitter
         or the coordinator, is success; no existence pre-check, no lock), THEN W1, THEN W2; an
         absent `gate-state.json` reads as `OPEN` and an absent txn set as "no live txn", so a
         never-migrated repository admits with the reservation standing until PostToolUse. The
         former "`migration-state/` absent ⇒ zero-cost no-op, no reservation" bypass is REMOVED;
         the no-op scope is exactly: not PreToolUse; tool not `Edit`/`Write`/`MultiEdit`; target
         outside the protected union or outside the session's `factory_root` (v1.16, (b)
         above); and the unchanged check-only degradation for a payload with an ABSENT or `null`
         `tool_use_id` (no directory, no reservation; backstopped by the §7c step-5
         fingerprint recheck). The release (PostToolUse / PostToolUseFailure) stays a
         best-effort no-op when the directory/file is absent. **Why:** under the bypass, a write admitted before the
         directory first existed created no reservation, so a coordinator that then created the
         directory, flipped DRAINING and polled saw quiescence while the write was in flight; the
         Dekker argument above (never both miss) now holds from the first protected write ever.
         **Failure:** inability to create the directory or reservation ⇒ admission error ⇒
         PreToolUse fails closed with `E-MAINTENANCE-002` (v1.16; message
         `E-MAINTENANCE-002: writer-admission check failed (<cause>)`, `<cause>` ∈
         {`invalid_tool_use_id`, `io`, `state_integrity`}, here `io`; `HookResult::Error`, exit 2
         at the PreToolUse hook surface; replaces the unnamed `BC-1.18.011: writer-admission
         check failed: {e}` string; the detail goes to the `migration.admission_failed` InternalLog
         event (Postcondition 10), never a raw id or record content), no reservation left
         behind, never an untracked admit.
         **`<cause>` classification (v1.16; total rule — normative text in BC-1.18.013
         Precondition 6(c) "`E-MAINTENANCE-002` `<cause>` classification", identical here for the
         B2 path; closes S-25.08 AC-018):** `invalid_tool_use_id` = a PRESENT `tool_use_id`
         that is a non-string / empty / grammar-violating value, decided from the payload alone
         before any filesystem access; `io` = an OS-level call made by the admission check
         failed other than ENOENT of the file itself (directory/reservation create, or
         open/read/readdir/stat of `gate-state.json`, a `txn-*.json` record or the terminal
         record — EACCES/EPERM/EROFS/ENOSPC/EIO/EISDIR/ELOOP/EMFILE/short read), content never
         examined after a failed call; `state_integrity` = every byte was read but the content
         is unusable (not UTF-8, empty/truncated/unparseable JSON, wrong JSON type/shape —
         `gate-state.json` is a bare JSON string `"OPEN"`/`"DRAINING"`/`"LOCKED"` —, unknown
         state value, a PRESENT non-string `migration_id`, more than one live txn). First
         failure wins in the order tool_use_id → reservation create (W1) → `gate-state.json` →
         `txn-*.json` (ascending filename) → terminal record. Reservation files are never read
         by admission (the drain's reservation GC is a binary surface, not `E-MAINTENANCE-002`);
         a terminal record whose read call fails ⇒ `io`, whose content does not verify ⇒ NOT
         `E-MAINTENANCE-002` (v1.17: `E-MAINTENANCE-001` WITH the completion-record-mismatch
         suffix, not finalized — unparseable/wrong-schema/count/id/hash mismatch is a Branch C
         verification failure; "plain" = no suffix, defined in BC-1.18.013 Precondition 6(b)).
         **Carrier (v1.18; normative text BC-1.18.013 Precondition 6(c); ADR-052 §Decision 5a
         "Admission state-integrity variant"; closes F-012).** `state_integrity` is
         `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` with `kind` ∈
         {`gate_record_malformed`, `txn_record_malformed`, `txn_migration_id_not_string`,
         `multiple_live_txns`, `reservation_serialization`} — it is NOT `BinaryIntegrityFailure`;
         its Display (`migration admission: state integrity failure (<kind-token>): <detail>`)
         MUST NOT contain `BINARY_INTEGRITY_FAILURE`; exit 2; `detail` is the already-sanitized
         path/parse message, never record content. The coordinator surfaces the same variant as
         exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`. `admission_failure_cause` returns the closed
         `AdmissionFailureCause { InvalidToolUseId, Io, StateIntegrity }` and is an exhaustive
         match (Invariant 5). Vectors: EC-026. **Cost (accepted):** one idempotent directory
         create plus one reservation create/rename/unlink (no fsync, no lock) per protected
         mutation in a factory project, the ≤ low-single-digit-ms increment ADR-052 v1.18
         already accepted; non-`.factory/` writes are outside the protected union and pay
         nothing. Rejected alternatives: bounded quiescence window (cannot prove completion of an
         arbitrarily long in-flight writer; adds a fixed activation delay) and accepting the
         fingerprint abort (known spurious liveness abort that a few-millisecond fix removes).
       - **Release-on-block.** If the dispatch's final aggregated outcome for the PreToolUse event
         is a block (exit 2) or error — including a block by a LATER stage in the same dispatcher
         process (`shard_cap_precheck`, any registry plugin) — the dispatcher removes the
         reservation its own admission created for that `tool_use_id` before exiting
         (harness-level denials the dispatcher cannot observe leak until PostToolUse/TTL).
       - **TTL.** `MAX_RESERVATION_TTL` default 3,600 s; PRODUCTION floor 1,800 s (a lower value in
         the production entry point is a configuration error — v1.16:
         `BcIndexMigrationError::ReservationTtlBelowFloor { configured_secs, floor_secs }`
         (`RESERVATION_TTL_BELOW_FLOOR`, exit 2) raised by `validate_production_reservation_ttl`,
         the FIRST statement of the crate-private `run_bc_index_migration_with_ttl` seam that the
         production entry delegates to (v1.18; EC-009(d)), BEFORE any gate or drain action,
         nothing mutated, never reusing `BinaryIntegrityFailure`;
         the TTL and the 30 s drain timeout
         remain injectable for tests — the floor binds the production entry point, not the test
         seam `drain_bc_index_writers`). The merged constant `DEFAULT_MAX_RESERVATION_TTL` = 120 s
         violates the floor and MUST be raised to 3,600 s. Staleness is judged by the
         reservation's `created_at` field, falling back to file mtime only when the field is
         absent, unparseable or untrusted (v1.16: pre-epoch, out-of-range, or a future stamp
         beyond the 300 s skew tolerance; both unusable ⇒ NOT stale + warn — timestamp rules
         below); reclamation is TTL-only — NO PID liveness.
       - **Release on `PostToolUse` OR `PostToolUseFailure` (v1.16; normative text in BC-1.18.013
         Precondition 6(c) "Release on PostToolUse OR PostToolUseFailure"; ADR-052 §Decision 5a;
         closes F-001).** The reservation created by the PreToolUse admission is removed on
         `PostToolUse` OR `PostToolUseFailure` for the same `tool_use_id` (Claude Code delivers a
         failed/interrupted/cancelled tool call as `PostToolUseFailure` and does NOT fire
         `PostToolUse`; the earlier "(success or failure of the tool)" wording named an event the
         harness never sends for failure and leaked the reservation until TTL). Keyed ONLY on
         `tool_use_id` — NO `tool_name` filter (an envelope with absent/differently shaped
         `tool_name` still releases; the unlink is idempotent). Gated by the pure predicate
         `is_tool_completion_event(event_name: &str) -> bool` (exactly `"PostToolUse"` or
         `"PostToolUseFailure"`) in `invoke.rs`; no new `EventType` variant. A missing file or an
         invalid-grammar id is a silent no-op; a non-ENOENT release error is non-fatal and is
         recorded as one `migration.admission_advisory` InternalLog event with `reason =
         reservation_release_failed` (the release leg returns it as data; `main.rs` writes it). Neither event arriving ⇒ the reservation leaks until
         `MAX_RESERVATION_TTL` (TTL backstop; manual deletion before that). The release runs at
         the registry-independent position (6(d) lead).
       - **Reservation timestamp rules (v1.16; normative text in BC-1.18.013 Precondition 6(c);
         ADR-052 §Decision 5a; closes F-008).** One pure predicate
         `reservation_is_stale(created_at: Option<u64>, mtime: Option<u64>, now: u64, ttl: u64)
         -> bool`, `RESERVATION_CLOCK_SKEW_TOLERANCE_SECS = 300`: (1) `created_at` parsed as RFC
         3339 (any offset, normalised to UTC epoch seconds, sub-second truncated); parse failure,
         pre-1970 (negative epoch) or outside `u64` ⇒ UNPARSEABLE ⇒ `None`, never clamped; (2)
         `created_at > now + 300` ⇒ UNTRUSTED ⇒ `None`; a stamp in `(now, now + 300]` is accepted
         and ages as 0; (3) `basis = created_at.or(mtime)`; both `None` (mtime unavailable,
         including pre-epoch) ⇒ age UNKNOWN ⇒ NOT stale (the merged `epoch_secs(pre-epoch) = 0 ⇒
         reclaimable` is REMOVED); (4) `age = now.saturating_sub(basis)`, stale iff `age > ttl`
         (a future mtime ages as 0); (5) every fallback/unknown-age outcome emits one
         coordinator-drain STDERR advisory line with token ∈ {`created_at_unparseable`,
         `created_at_pre_epoch`, `created_at_future`, `mtime_future`, `age_unknown`} (v1.18,
         corrected in the v1.18 follow-up per ADR-052 §Downstream item 33(c)): these are NOT
         `migration.admission_advisory` reasons — only the drain GC calls `reservation_is_stale`
         (admission never reads reservation files), so they have no dispatcher event. Direction:
         wrongfully reclaiming a live writer's reservation lets the coordinator snapshot mid-write
         (integrity hazard); wrongfully retaining one is a bounded visible stall
         (`DRAIN_TIMEOUT_ABORT`) — every ambiguity resolves toward retention; clamping is rejected.
       - **`tool_use_id` presence and validity (v1.16; normative text in BC-1.18.013 Precondition
         6(c); ADR-052 §Decision 5a; closes F-009 part 2).** Grammar `[A-Za-z0-9_.-]{1,128}`, not
         starting with `.`. ABSENT key or JSON `null` ⇒ check-only admission (no directory, no
         reservation). PRESENT but non-string / empty / grammar-violating ⇒
         `BcIndexMigrationError::InvalidToolUseId { len }` ⇒ the admission FAILS CLOSED:
         `E-MAINTENANCE-002` (`invalid_tool_use_id`), no reservation, nothing admitted — never
         silently downgraded to check-only; the error carries the byte length only, never the raw
         value; the release leg treats an invalid id as a silent no-op.
   (d) **PreToolUse stale-gate reconciliation is part of this contract and MUST be wired on the
       production admission path (v1.11; ADR-052 §Decision 5a step 3.5 Branches A/B/C; merged B2
       code defect: `reconcile_stale_admission_gate` is never called).** The single shared
       admission core — reached through exactly three migration-NEUTRAL entry points,
       `executor::migration_writer_admission` (reservation-returning; the one `main.rs` calls),
       `executor::migration_writer_admission_precheck` (verdict-only) and
       `executor::migration_reservation_release` (ADR-052 §Decision 5a "Entry-point naming
       (v1.21)"; closes F-014; NO per-migration delegates exist or may be added) — is evaluated
       at a REGISTRY-INDEPENDENT position (v1.16; normative text in
       BC-1.18.013 Precondition 6(b) "Where"; ADR-052 §Decision 5a "Evaluation position (v1.20)";
       closes F-004): in `main.rs::run` immediately after the stdin payload parse and
       `resolve_project_cwd()`, and the release leg likewise. Obligations (black-box testable):
       **O1** exactly once per PreToolUse event; **O2** BEFORE `Registry::load` /
       `resolve_registry_path()` / the Tier-1 degraded-registry branch, so it runs identically
       with `CLAUDE_PLUGIN_ROOT` unset or empty, with a missing, unparseable or
       schema-mismatched registry, and with an empty matched-plugin set; **O3** BEFORE
       `shard_cap_precheck` and every registry plugin tier; **O4** an unparseable stdin payload
       cannot be classified — the existing parse-error exit is UNCHANGED. **Why:** the
       registry-load failure arms are fail-open by contract (BC-1.08.001), acceptable for janitor
       legs but wrong for this writer-exclusion interlock — a degraded install must not let
       protected writers run unreserved while a coordinator drains and snapshots. A `Block`/
       `Error` verdict terminates the dispatch directly (exit 2 + reason) without loading the
       registry; `Admitted` stores the reservation handle and continues; the release-on-block
       funnel covers every later non-zero outcome (including registry fail-closed exits).
       **Architect Ruling 1 (re-anchored v1.17):** when the migration gate fires (a
       `Block`/`Error` verdict), `shard_cap_precheck` is never invoked — structurally skipped:
       admission returns before `shard_cap_precheck` is reachable (an early return in the
       dispatcher's `run`), so a live txn can never be mutated by a shard roll/seal/truncate
       of a protected canonical. The core runs, under the §5a flock-gated discipline
       (`flock(exclusive.lock, LOCK_EX|LOCK_NB)`; EWOULDBLOCK ⇒ live coordinator ⇒ no action,
       block): **Branch A** — gate ∈ {LOCKED, DRAINING} with no active txn (absent, COMPLETED,
       ABORTED) ⇒ gate → OPEN; **Branch B** — txn STAGING with `generation_id = null` (pre-
       generation crash) of a txn whose `migration_id ∈ K` (v1.16; K defined in the decision
       table below — a foreign `migration_id ∉ K` txn is NEVER discarded) ⇒ txn → ABORTED (`null_generation` disposition marker, retained — exact
       on-disk form in the **Branch B marker** paragraph below) and gate → OPEN, EXCEPT when the migration's terminal record is present (Branch C governs,
       Branch B does not apply — an unexplained terminal record beside a live STAGING txn is an
       integrity anomaly, never a discardable pre-generation crash); **Branch C** — a live txn
       (COMMITTING, or STAGING of any generation) with the live txn's own migration's terminal
       record, selected by `migration_id` (`completed.json` for `migrate-bc-index`/absent field;
       `completed-backfill-append-logs.json` for `backfill-append-logs`, BC-1.18.013
       Postcondition 5a) — the shared core dispatches on `migration_id` (ADR-052 §7e) — present
       ⇒ verify-then-finalize or fail-closed per Postcondition 9 (STAGING + terminal record is
       always fail-closed; no verification is attempted). The
       PreToolUse reconciler NEVER performs COMMITTING forward recovery (renames) — that remains
       exclusively the binary's job.
       **Branch B marker (v1.13; exact on-disk form).** The Branch B discard rewrites the txn
       record IN PLACE — the same `.factory/migration-state/txn-<uuid>.json` file, via ONE atomic
       write-temp + fsync + rename + dir-sync (§7d), so there is no observable intermediate state
       of "ABORTED without marker" or "marker without ABORTED" — setting `"state": "ABORTED"` and
       adding the top-level JSON string field `"abort_reason": "null_generation"`. All other
       fields are preserved unchanged; in particular `generation_id` and `source_sha256` remain
       `null` and `activation_id`/`migration_id` are untouched. The txn file is RETAINED at its
       original path: the reconciler MUST NOT delete, rename, move or archive it (retention until
       archival is governed solely by the ADR-052 §Decision 4e F4 GC policy). The gate flip to
       `OPEN` follows the txn rewrite (txn first, then gate — `gate=OPEN ⇒ no live txn`). Reader
       rules: (i) `abort_reason` is informational/audit only — no admission, reconciliation,
       recovery or migration-selection decision may consult it; every reader decides on `state`
       alone, and an `ABORTED` record (with or without `abort_reason`, with any value) is
       non-live exactly like any other `ABORTED` record (Branch A, the admission dual check and
       txn selection ignore it); (ii) the field is optional on read (`#[serde(default)]`;
       absent on pre-v1.13 records and on every other abort path, none of which this BC
       requires to write it) and unknown values MUST be tolerated, never rejected; (iii) a
       subsequent activation creates a NEW txn file with a fresh `activation_id`, never reusing
       the retained record; (iv) re-running the reconciler over the retained record is Branch A
       (idempotent; the record is not rewritten). Mechanism A's Branch B is identical.
       **Terminal-record reconciliation core decision table (v1.13; reworded v1.16 for F-006 —
       ADR-052 §Decision 7e "Definition of foreign in the shared admission core"; the pure core
       `decide_terminal_record_reconciliation`; ADR-052 §Decision 5a step 3.5 Branch C + Note
       (1)).** K = {`migrate-bc-index`, `backfill-append-logs`} (absent `migration_id` field ⇒
       `migrate-bc-index`). The pure core input `TerminalReconcileInputs.txn_is_own_migration`
       is RENAMED `txn_migration_known: bool` (true iff the live txn's `migration_id ∈ K`): the
       shared core serves BOTH migrations and has NO "evaluating migration". Decision order
       (first match wins):

       | # | Condition | Decision |
       |---|---|---|
       | 1 | `exclusive.lock` NOT acquired (EWOULDBLOCK, live coordinator) OR no live txn (state ∉ {STAGING, COMMITTING}) | `NoOp` (live coordinator: block; no live txn: gate-only repair is Branch A, outside this core) |
       | 2 | live txn's `migration_id ∉ K` (`txn_migration_known = false`) | `RefuseForeignMigration` (plain `E-MAINTENANCE-001`, no suffix; precedence over every record check) |
       | 3 | known migration (`migration_id ∈ K`), live txn, lock acquired, ITS OWN migration's terminal record ABSENT — **STAGING and COMMITTING are NOT distinguished** | **`NoOp`** (nothing for this core to reconcile; no txn write, no gate write) |
       | 4 | its own migration's terminal record present, txn COMMITTING, and every check passes (parses; `txn_id == activation_id`; `generation_id` equal; `canonical_paths_count == N`; every canonical `sha256 == expected_post_hash`) | `FinalizeThenOpenGate` |
       | 5 | its own migration's terminal record present and (txn STAGING — any `generation_id` — OR any verification failure, including COMMITTING with a different `activation_id`) | `FailClosedMismatch` (suffix message; Postcondition 9(d)) |

       **Selection by `migration_id`, not refusal (v1.16).** In the shared (dispatcher /
       §Decision 5c) core a live txn of EITHER known migration is decided against ITS OWN
       migration's terminal record, selected by `migration_id` together with the canonical-path
       set and N (`completed.json` for `migrate-bc-index`; `completed-backfill-append-logs.json`
       for `backfill-append-logs`, §Decision 7e); the OTHER known migration's record is NEVER
       consulted. Hence "foreign" never applies between the two known ids on the dispatcher
       path: a live `backfill-append-logs` txn beside a present B2 `completed.json` and NO
       `completed-backfill-append-logs.json` yields row 3 (`NoOp`) ⇒ Branch B only if STAGING with
       `generation_id = null`, otherwise the ordinary plain live-txn block (and a live
       `migrate-bc-index` txn symmetrically); neither is `RefuseForeignMigration`, neither is a
       mismatch, and neither is finalized on the strength of the other's record (the invariant
       "never finalize a txn on the strength of the other migration's record" is preserved by
       selection, not refusal). A foreign txn (`migration_id ∉ K`, written by a newer or alien
       build) is NEVER finalized and NEVER aborted — including Branch B's null-generation discard;
       no Branch A/B/C repair runs; the gate keeps blocking the union with the plain
       `E-MAINTENANCE-001`; the raw id (truncated to 64 chars, control characters escaped) goes
       only to the `migration.admission_blocked` InternalLog event (field `migration_id`,
       `branch = foreign_migration`; Postcondition 10). A `migration_id` that is not a JSON string is a
       MALFORMED record ⇒ admission error `E-MAINTENANCE-002` (`state_integrity`), not "foreign".
       The cross-migration refusal of Precondition 6(e) (binary exit 2 `LockContention`-class) is
       unchanged and is a property of the migration BINARIES' recovery path only; a
       formally-unreachable-for-two-known-ids `RefuseForeignMigration` is retained in the core so
       that totality over `txn_migration_known = false` makes forward-compatible ids fail closed.

       Row 3 consequence (caller contract): on `NoOp` from row 3 the admission core falls
       through, in order, to (a) Branch B — if and only if the txn is STAGING with
       `generation_id = null` (own terminal record absent is Branch B's precondition) —
       which discards and admits; otherwise (b) the ordinary admission decision (gate OPEN ∧ no
       live txn), which BLOCKS because a txn is live, with the PLAIN keyed `E-MAINTENANCE-001`
       message and NO mismatch suffix. COMMITTING with the terminal record absent is never
       finalized or recovered here (forward recovery is the binary's job, §4e rows 2–3); STAGING
       with `generation_id` set and the record absent is a legitimately running or
       binary-resumable activation. A reconciler that exists but is not reachable from the production admission path does NOT
       satisfy this precondition (black-box test through the real dispatcher entry required).
   (e) **Migration discriminator and ownership (v1.11; ADR-052 §Decision 7e).** The txn record
       carries `migration_id`; a record that lacks it (written by pre-v1.11 code) is read as
       `"migrate-bc-index"` (serde default). `migrate-bc-index` OWNS and is the only writer of
       `.factory/migration-state/completed.json` and `.factory/migration-state/CURRENT.json`
       (consumed unchanged by BC-1.18.010 §Reader Integration and `detect_migration_read_state`);
       `backfill-append-logs` (BC-1.18.013) never writes them — it uses
       `completed-backfill-append-logs.json` / `CURRENT-backfill-append-logs.json`. The terminal-
       record schema is `{generation_id, txn_id, completed_at, canonical_paths_count}`
       (`canonical_paths_count` = N for the migration; for B2 the number of canonical paths in
       its txn record/intent log). **Cross-migration recovery refusal:** the `migrate-bc-index`
       binary recovers, resumes, finalizes or aborts ONLY a live txn whose `migration_id` is
       `"migrate-bc-index"`; a live txn of `backfill-append-logs` ⇒ exit 2 (`LockContention`-
       class), NO mutation, `recover()` NOT run over the foreign record. This refusal is a property
       of the migration BINARIES' recovery path only, where "own" is the invoking subcommand
       (v1.16; ADR-052 §Decision 7e "Definition of foreign" (5)); on the dispatcher
       (shared-core) path "foreign" means ONLY `migration_id ∉ K` (6(d) decision table) and a
       live txn of either known migration is decided against its own record. The admission gate is
       migration-agnostic (a live txn of either migration blocks the union
       `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/`). One `exclusive.lock`, one
       `gate-state.json` (the physical name of the logical "gate-state" file), one
       `reservations/` directory and one `txn-*.json` directory are shared; at most ONE live txn
       exists across both migrations.

7. **Coordinator anchoring — `migrate-bc-index` resolves its factory root by the SAME single rule
   as admission (v1.18; ADR-052 §Decision 5a "Single anchoring rule (v1.21)"; normative text
   BC-1.18.013 Precondition 7; closes D-2).** `run_migrate_bc_index_cli` and
   `run_bc_index_migration` take the resolved PROJECT ROOT (the parameter formerly named `_cwd` —
   it was never a cwd), computed by `main.rs` as `resolve_session_project_root(
   std::env::var_os("CLAUDE_PROJECT_DIR"), <std::env::current_dir()>)` for BOTH subcommand routes,
   never the raw process cwd. The coordinator (a) obtains its factory root ONLY through
   `resolve_factory_root(project_root)`; (b) derives EVERY `.factory/…` path it reads or writes
   (`migration-state/` via `FactoryRoot::migration_state_dir()`, `specs/behavioral-contracts/BC-INDEX.md`,
   `…/shards`, `cycles/…`, the activation manifest) from that SAME resolved real factory root — the
   six `_cwd.join(".factory/…")` literals in `run_bc_index_migration` and its helpers are removed;
   (c) NEVER creates `.factory`, never falls back to another directory, and when
   `resolve_factory_root` yields `None` (no `.factory` directory) mutates NOTHING and exits 2
   `FACTORY_ROOT_NOT_FOUND` (`BcIndexMigrationError::FactoryRootNotFound { project_root,
   root_source }`; no `exclusive.lock`, no `migration-state/`, no gate write). The normative single stderr
   line (v1.18 follow-up; ADR-052 §Downstream item 33(e)) is `<subcommand>:
   FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root> (resolved from
   <source>)`, `<subcommand>` ∈ {`migrate-bc-index`, `backfill-append-logs`}, `<source>` ∈
   {`CLAUDE_PROJECT_DIR`, `process cwd`}. **Field name and optionality (v1.18 follow-up 2;
   normative text BC-1.18.013 Precondition 7):** the variant's source field is named
   `root_source: Option<ProjectRootSource>` — NOT `source`, because `thiserror` treats a field
   named `source` as the error's `std::error::Error::source()` and the variant would not compile;
   the stderr wording `(resolved from <source>)` is unchanged. **Rule (a), ruled:** the
   `migrate-bc-index` CLI route MUST always supply the source (it is known at the binary
   boundary) by calling `run_bc_index_migration_for_session(&SessionProjectRoot)`, so the CLI
   stderr line ALWAYS ends in `(resolved from <source>)` and the CLI can never print the
   suffix-less form; the path-only library entries `run_bc_index_migration(&Path)` /
   crate-private `run_bc_index_migration_with_ttl(&Path, ttl)` have no source to report and are
   PERMITTED (embedding / test seam, not reachable from either CLI route) with
   `root_source = None`, whose `Display` omits the suffix entirely (no ` (resolved from )`, no
   placeholder). **Consequence:** a reservation created
   by admission under a given `CLAUDE_PROJECT_DIR` lives in the SAME
   `migration-state/reservations/` directory the coordinator's first drain polls, whatever the
   process cwd — before v1.18 the coordinator passed the process cwd and the Dekker interlock of
   6(c) silently guarded nothing whenever `CLAUDE_PROJECT_DIR` ≠ the process cwd. The
   coordinator's failures and advisories are stderr lines, not `tracing` output (Postcondition 10).
   Vectors: EC-027..EC-030.

## Postconditions

1. **Content-preservation (structured per-BC-row equivalence).** Extract all BC-X.YY.NNN table
   rows from the staged shard files; sort them in canonical BC-ID order; compute SHA-256 of the
   sorted row content. Compare against `source_body_row_sha256` from the txn record — the SHA-256
   of the per-BC-row content from the ORIGINAL (pre-split) `BC-INDEX.md` body in canonical BC-ID
   sort order, captured at quiescence (ADR-052 §Decision 5a drain step 5(c)). Excluded from BOTH
   the staged-row extraction and the source hash: `§Summary`, `§Subsystem Shard Manifest`,
   cross-cutting invariants, and non-row separator lines — only the BC-X.YY.NNN row entries
   themselves are in scope, preserving the "modulo the newly-introduced §Subsystem Shard Manifest
   section" intent of the original content-preservation obligation. A whole-concat SHA-256 against
   `source_sha256` (the whole-file fingerprint) is UNSATISFIABLE: the staged lean body ADDS the
   `§Subsystem Shard Manifest` section (extra bytes absent from the original), and content is
   reordered — `source_sha256` is used ONLY by step 5 fingerprint recheck (Postcondition 3a),
   NOT here. This is BC-1.18.008 Postcondition 6(a)'s exact analogue, applied to a content
   partition (by subsystem) instead of a time partition (by seal sequence). PC1 is verified
   at ADR-052 §Decision 7c step 3b against the staged generation files BEFORE the CURRENT.json
   pointer swap; step 3c aborts cleanly (txn → ABORTED, gate → OPEN) on any verification failure.

2. **Independent-census integrity check — every BC row in EXACTLY one shard.** Before the split
   begins, capture an independent census: the complete set of `BC-X.YY.NNN` IDs present in the
   ORIGINAL (pre-split) `BC-INDEX.md` body (a fresh enumeration, not reused from any cached count),
   cross-checked against `BC-INDEX.md`'s own `total_bcs` frontmatter field (an independent
   count-oracle, e.g. 2,005 per BC-INDEX v5.52 at the time this BC was authored) as a sanity bound.
   After the split, verify: (a) every census ID appears in EXACTLY ONE resulting shard file
   (`shards/BC-INDEX-SS-NN.md`, or a sub-shard once second-level splitting applies) — never zero,
   never two; (b) the union of all shard files' row counts equals the pre-split census count
   exactly; (c) `BC-INDEX.md`'s own body, post-split, contains ZERO per-BC table rows (BC-1.18.010
   Invariant 3). This is BC-1.18.008 Postcondition 6(b)'s exact analogue (record-integrity),
   specialized to BC-INDEX's ID-keyed partition instead of decision-log's row-boundary partition,
   and is the "independent census" check BC-1.18.010 already specifies for the STEADY STATE — this
   migration BC specifies the ONE-TIME check that establishes that steady state correctly in the
   first place. PC2 (the independent census) is verified at ADR-052 §Decision 7c step 3b against
   the staged generation files BEFORE the CURRENT.json pointer swap; step 3c aborts cleanly
   (txn → ABORTED, gate → OPEN) on any census failure.

3. **Crash-atomicity: staging + verify + atomic replace, all-or-nothing.** Write all ten (or more)
   resulting shard files and the shard-manifest TOML to a staging location first; only after
   Postcondition 1 (content-preservation) and Postcondition 2 (independent census) both verify
   clean does the operation atomically replace `BC-INDEX.md`'s body and publish the shard-manifest
   at its canonical path, via the same temp-file-then-rename discipline BC-1.18.006 already
   establishes. This is BC-1.18.008 Postcondition 5's exact analogue. Each atomic file replacement
   (`rename(2)` call) MUST be followed by a platform-appropriate durability barrier before
   proceeding to the next replacement:
   - Linux (ext4/xfs): `fsync(file_fd)` + `fsync(parent_dir_fd)` — mandatory; ensures directory
     entry survives a system crash per Pillai et al. OSDI'14.
   - macOS/APFS: `fcntl(file_fd, F_FULLFSYNC)` — mandatory for power-loss durability (Apple
     `fsync(2)` does NOT flush the drive cache; `F_FULLFSYNC` is the documented durability lever);
     `fsync(parent_dir_fd)` — best-effort only; Apple docs do not guarantee APFS directory-fsync
     provides power-loss durability.
   This platform-branched durability guarantee is implemented in `sync_file_durable()` and
   `sync_dir_best_effort()` per ADR-052 §Decision 7d.

3a. **Pre-commit source-fingerprint recheck (TOCTOU guard).** This check is performed EXACTLY
    ONCE, before the CURRENT.json pointer swap (step 6 in ADR-052 §Decision 7c) — not between
    individual renames. The migration binary re-reads BC-INDEX.md's source content, computes
    SHA-256, and compares against the `source_sha256` field recorded in the txn record at
    quiescence. If they differ: ABORT. The txn record is set to state ABORTED. BC-INDEX.md's
    original body is left untouched (no renames have occurred at this point). The migration
    requires re-activation. This single-check design eliminates the v1.1 contradiction where
    a re-check after BC-INDEX.md's own rename would find a fingerprint mismatch and
    incorrectly trigger abort.

4. **Rollback on verification failure.** If EITHER the content-preservation check OR the
   independent-census check fails, the migration ABORTS: `BC-INDEX.md`'s original monolithic body
   is left completely untouched (fail-loud, not partial-and-silent) — no partial set of shard files
   is ever treated as authoritative, and no partial `§Subsystem Shard Manifest` is published. This
   is BC-1.18.008 Postcondition 6's "hard gate" analogue and its EC-004's exact analogue.

5. **Idempotency against a partially-completed prior attempt.** If a prior migration attempt left a
   valid partial shard-index/manifest state, re-running MUST either resume from the last
   verified-complete shard or detect the already-migrated state and skip re-splitting — never
   double-split. This is BC-1.18.008 Invariant 3's exact analogue.

6. **MUST cover the SS-05/SS-06 second-level sub-split within the SAME one-time migration
   operation, not a separate follow-on, using the `chunk_subsystem_rows_into_sub_shards` function
   contract specified by ADR-051 §Decision 18.** Both subsystems already exceed the provisional cap
   on their own section size alone (SS-05 ~88,695 bytes / 661 BCs; SS-06 ~85,407 bytes / 592 BCs,
   both measured 2026-09-05) and require immediate second-level sub-sharding at F4 activation,
   as part of the same one-time B2 migration operation (independently of mechanism A's activation
   schedule). The migration invokes
   `chunk_subsystem_rows_into_sub_shards(sorted_rows: &[(BcId, String)], preamble: &str,
   shard_cap_bytes: u64) -> Vec<SubShardChunk>` (ADR-051 §Decision 18 item 4) — a pure,
   canonical-BC-ID-sorted (via `extract_and_sort_bc_rows`, item 2), greedy-pack-until-cap,
   single left-to-right pass — to compute sub-shard boundaries for any over-cap subsystem section,
   reusing the SAME `shard_cap_bytes` value as first-level splitting (no separately-calibrated
   migration-time cap). This BC's content-preservation, independent-census, atomicity, and rollback
   obligations (Postconditions 1-5 above) apply IDENTICALLY at the sub-shard level for SS-05/SS-06 —
   i.e., the census for SS-05 verifies every `BC-5.YY.NNN` row lands in exactly one of
   `shards/BC-INDEX-SS-05.a.md`/`.b.md`/etc., with the SS-05-scoped total matching an independent
   pre-split count of `BC-5.*` rows specifically.

   **Migration-time edge-case rulings (ADR-051 §Decision 18; the migration MUST complete in every
   case below — none of these is a fail-loud abort condition):**
   - **Lone oversized row.** If a single BC row's own markdown line, by itself with only the
     preamble, exceeds `shard_cap_bytes`, the migration emits it as its own over-cap lone sub-shard
     — it MUST NOT split a table row's line across two files, and it MUST NOT fail-loud or abort the
     migration on this condition. A non-blocking advisory line is written to the coordinator's
     stderr (not `tracing`; ADR-052 §Decision 5a "Admission diagnostics channel" (5)) so the
     anomaly remains visible. This is a bounded case, not an unbounded hazard: BC-1.18.005 Postcondition 6's
     `MAX_SINGLE_RECORD_BYTES` margin exists precisely so that "current content + one more max-size
     record" never threatens the true fuel ceiling even when it nominally exceeds the provisional
     `shard_cap_bytes` figure.
   - **Exactly-at-cap boundary.** When appending a row would make `current_bytes + row_bytes`
     exactly equal to `shard_cap_bytes`, the row stays in the current chunk — `<=` inclusive,
     matching BC-1.18.005 Postcondition 3's `projected_size <= shard_cap_bytes -> Continue`
     convention verbatim (one inequality direction project-wide, not a second, subtly different
     rule).
   - **Sub-shard letter exhaustion.** If a subsystem's row set produces more than 26 chunks, the
     migration extends sub-shard naming with a base-26 two-letter scheme (`.a`..`.z`, then
     `.aa`..`.az`, `.ba`..., spreadsheet-column-naming style) — it MUST NOT fail-loud or refuse to
     sub-shard on letter exhaustion.
   - **Each row in exactly one sub-shard.** Already independently enforced by this BC's own
     Postcondition 2 census (an ID-set membership check over the staged bodies), which is
     structurally split-count-agnostic — feeding N sub-shard bodies in place of 1 whole-subsystem
     body requires no change to the census logic; the BC-ID-range addressing scheme in the
     sub-manifest schema is an addressing convenience only and is orthogonal to, and does not
     weaken, this independently-enforced correctness guarantee.

   **Verification.** Chunk-boundary determinism and correctness for this Postcondition — same input
   row set + preamble + `shard_cap_bytes` always yields identical chunk boundaries; every row
   appears in exactly one chunk; no chunk's preamble+rows exceeds `shard_cap_bytes` except the
   documented lone-row-overflow case above; consecutive chunks' BC-ID ranges are non-overlapping and
   jointly cover the full sorted sequence — is hosted by **VP-142** (proptest), cross-referenced from
   BC-1.18.010 Postcondition 4, since the property must hold identically for both this one-time
   migration and the steady-state rebuild path (ADR-051 §Decision 18 item 7).

7. **No new Cohort-B dependency.** Unlike BC-1.18.008 (which BC-7.08.001's fail-closed flip depends
   on, since `regression-gate`/`convergence-tracker` read the four mechanism-A artifacts), this
   migration has NO Cohort-B sequencing dependency: the F2 architecture-delta doc's §5
   migration-impact map confirms `regression-gate`/`convergence-tracker` do not read `BC-INDEX.md`.
   `BC-7.08.001`'s scope and gating conditions are UNCHANGED by this BC. Note: this postcondition
   governs B2/Cohort-B independence only. A/B2 scheduling independence (that mechanism A and B2
   activate independently at F4) is governed by Precondition 4 [as amended by ADR-052 §Decision 1].

8. **Relationships.** This BC depends on BC-1.18.010 (the end-state addressing scheme this
   migration produces) and BC-1.18.006 (reuses its atomic-write primitives) — the same "applies an
   existing primitive retroactively, once" relationship BC-1.18.008 has to BC-1.18.006, mirrored
   here for B2's own end-state BC.

9. **Verified `completed.json` + COMMITTING finalize; `COMPLETION_RECORD_MISMATCH_ABORT` replaces
   the unverified short-circuit (v1.11; ADR-052 §Decision 4e v1.18 D3 rows, §5a Branch C, §5c
   Branch 2 step 0.5; closes the B2 permanent self-lock and the cross-migration finalize hazard).**
   When the live txn's own migration's terminal record, selected by `migration_id`
   (`completed.json` for `migrate-bc-index`/absent field; `completed-backfill-append-logs.json`
   for `backfill-append-logs`, BC-1.18.013 Postcondition 5a), is present — the shared core
   dispatches on `migration_id` (ADR-052 §7e) — neither the `migrate-bc-index` binary nor the
   PreToolUse Branch C may treat its presence alone as authority to finalize a txn
   (the merged B2 code's unverified `completed.json` short-circuit — which would rewrite ANY
   live txn, including a `backfill-append-logs` txn, to COMPLETED and open the gate
   mid-migration — is REMOVED). The B2-specific clauses below (schema, N, verification) apply
   to `migrate-bc-index` txns; `backfill-append-logs` txns are verified per BC-1.18.013
   Postcondition 5a. Instead, under `flock(exclusive.lock, LOCK_EX|LOCK_NB)`
   (EWOULDBLOCK ⇒ live coordinator ⇒ no action, binary exits 0 with the §4e warning /
   PreToolUse blocks):
   (a) the live txn must have `migration_id = "migrate-bc-index"` (absent field counts as that);
       a live txn of the OTHER migration is NEVER finalized on the strength of B2's
       `completed.json` — in the `migrate-bc-index` BINARY by cross-migration refusal
       (Precondition 6(e): binary exit 2 `LockContention`-class, NOT
       `COMPLETION_RECORD_MISMATCH_ABORT`; binary recovery path only), and on the PreToolUse
       (shared-core) path by SELECTION (v1.16): a live `backfill-append-logs` txn is decided
       against ITS OWN record `completed-backfill-append-logs.json`, never `completed.json`, so
       with its own record absent the core returns row 3 `NoOp` and the PreToolUse outcome is the
       plain `E-MAINTENANCE-001` block, no mismatch reason (Precondition 6(d); "foreign" there
       means ONLY `migration_id ∉ K`; a non-string `migration_id` ⇒ `E-MAINTENANCE-002`
       `state_integrity`);
   (b) **VERIFY:** `completed.json` parses; `txn_id == activation_id`; `generation_id` equal to
       the txn's; `canonical_paths_count` equals B2's N for this txn; and for EVERY canonical
       path `sha256(path) == expected_post_hash` per the txn record/intent log; the txn must be
       COMMITTING;
   (c) on success: atomically rewrite the txn to COMPLETED (write-temp + fsync + rename +
       dir-sync, §7d), THEN (under gate `LOCK_EX`) flip `gate-state.json` → OPEN (order
       mandatory: `gate=OPEN ⇒ no live txn`); the binary then exits 0 `ALREADY_MIGRATED`; the
       PreToolUse Branch C performs the same action with no binary invocation and then admits
       (idempotent; a crash between the two writes leaves txn=COMPLETED + gate≠OPEN, repaired by
       Branch A);
   (d) on ANY verification failure (unparseable; id/generation mismatch; `canonical_paths_count` ≠
       N; any hash ≠ `expected_post_hash`; txn STAGING (STAGING + terminal record is always
       fail-closed; no verification is attempted); COMMITTING with a different
       `activation_id`): NO txn write, NO gate write, gate stays blocking; the binary exits 2
       `COMPLETION_RECORD_MISMATCH_ABORT`; the PreToolUse analogue is an `E-MAINTENANCE-001`
       block with the reason logged (one `migration.admission_blocked` InternalLog event with
       `branch = completion_record_mismatch` naming `migration_id`, `txn_id`, failing `check`). **Block message (v1.13):** the PreToolUse block message is the Precondition
       6(b) path-family-keyed `E-MAINTENANCE-001` format string followed by exactly one space
       and the suffix `(completion-record mismatch — operator investigation required)` (em dash
       U+2014). This applies identically to (i) a B2 (`migrate-bc-index`) txn in STAGING with
       `completed.json` present — the always-fail-closed case, where the PreToolUse block
       carries the suffix exactly as for a COMMITTING verification failure (the suffix is NOT
       reserved to COMMITTING); (ii) a B2 COMMITTING verification failure; and (iii) the
       mechanism-A analogues (BC-1.18.013 Postcondition 5a), whose `<scope>` token is
       determined by the written path family, not the migration. Examples:
       write under `.factory/specs/behavioral-contracts/`, B2 txn STAGING + `completed.json` ⇒
       `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING
       state); retry after migration completes or aborts (completion-record mismatch — operator
       investigation required)`; write under `.factory/cycles/`, mechanism-A txn STAGING +
       `completed-backfill-append-logs.json` ⇒ `.factory/cycles/ write blocked: migration
       window active (txn record in STAGING or COMMITTING state); retry after migration
       completes or aborts (completion-record mismatch — operator investigation required)`
       (single-line messages; line breaks editorial). The foreign-migration refusal in 9(a)
       and a live coordinator (EWOULDBLOCK) carry NO suffix.
       A byte-for-byte snapshot of `.factory/migration-state/` is unchanged by the attempt.
   `--census` stays read-only and never reconciles. Idempotent: a second run is a zero-mutation
   no-op.

10. **Admission diagnostics are dispatcher-internal-log events — exactly one per verdict or
    anomaly (v1.18; ADR-052 §Decision 5a "Admission diagnostics channel (v1.21)"; normative text
    and field lists BC-1.18.013 Postcondition 10; wire format BC-3.08.001 Events 11–13; closes
    D-1).** The dispatcher installs no `tracing` subscriber, so every `tracing::*!` diagnostic is
    discarded in production; every admission/reconciliation/release diagnostic this BC mandates is
    therefore a dispatcher-native `InternalEvent` written to `dispatcher-internal-YYYY-MM-DD.jsonl`
    by `main.rs` (the shared core returns `AdmissionDiagnostic` data; `main.rs` writes it before
    the early return on a verdict): every `E-MAINTENANCE-001` verdict ⇒ one
    `migration.admission_blocked` (`scope`, `family`, `branch`, `gate_state`, `migration_id`,
    `txn_id`, `check`, `reconciliation` — the closed six-token effectful domain `live_coordinator` /
    `nothing_to_reconcile` / `gate_reopened` / `null_generation_txn_aborted` /
    `foreign_migration_refused` / `completion_record_mismatch`, no `none`; `branch` is derived:
    `live_coordinator` ⇔ `live_coordinator`, `foreign_migration` ⇔ `foreign_migration_refused`,
    `completion_record_mismatch` ⇔ `completion_record_mismatch`, else `gate_only` if no live txn
    remains, else `live_txn`; a Branch C verification failure is ONE `_blocked` and NO `_advisory`;
    `check` is non-null ⇔ `branch = completion_record_mismatch` and is EXACTLY one token of the
    **closed nine-token domain** {`staging_with_terminal_record`, `terminal_record_unparseable`,
    `terminal_record_schema_mismatch`, `txn_id_mismatch`, `generation_id_mismatch`,
    `canonical_paths_count_mismatch`, `canonical_hash_mismatch`, `terminal_record_unverified`,
    `finalize_unwired`} (verification checks evaluated in that order, first failure wins; the
    last two are S-25.08-seam tokens, retired once S-25.06 delivers the verifier and finalize;
    a terminal-record or canonical-file read CALL failure is `E-MAINTENANCE-002 (io)`, not a
    check; v1.18 follow-up 2; normative text BC-1.18.013 Postcondition 10(a)); every `E-MAINTENANCE-002` verdict ⇒ one
    `migration.admission_failed` (`cause`, `kind`, `detail`); every non-verdict anomaly ⇒ one
    `migration.admission_advisory` (`reason` ∈ {`reservation_release_failed`,
    `branch_a_gate_reopened`, `branch_b_txn_aborted`, `branch_c_finalize_unwired`,
    `branch_c_finalized`}; optional context fields only from {`migration_id`, `txn_id`, `check`,
    `detail`, `tool_use_id_len`}; `branch_c_finalize_unwired` fires when
    `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` and the finalize effect
    is undelivered, IN ADDITION to the one `_blocked` — which then carries `reconciliation =
    completion_record_mismatch`, `branch = completion_record_mismatch`, `check =
    finalize_unwired` (item 33(g)) — retired, with that `check`, when S-25.06 delivers the finalize;
    `branch_c_finalized` is S-25.06-only, written on a successful verify-then-finalize that is
    admitted (no `_blocked`)). The five reservation-timestamp tokens are coordinator-drain stderr
    tokens, not advisory reasons. An admitted write with no anomaly, and an out-of-scope admit, write no event. No
    field carries a raw `tool_use_id` (at most its byte length) or record content; string fields
    derived from disk or payload data are sanitized (control characters escaped, truncated to 64
    characters, EC-024). The operator-visible verdict message is unchanged. The coordinator's own
    diagnostics (`migrate-bc-index: migration failed`, the drain's reservation-staleness
    advisories, the live-coordinator warning, the lone-oversized-row note) are single
    `E-…`/`<VARIANT>`-prefixed stderr lines. Vectors: EC-031, EC-032, EC-033.

## Invariants

1. **This BC's per-file write logic invokes BC-1.18.006's `write_atomic` primitive
   (`last_amended_migrate::atomic_write::write_atomic`, the temp-file-then-rename mechanism), not a
   reimplementation.** This BC differs from BC-1.18.006 in WHEN it runs (once, at F4 activation)
   and WHAT it operates on (BC-INDEX's existing body, partitioned by subsystem). However, this BC
   DOES introduce new multi-file crash-atomicity machinery via ADR-052 §Decision 7: §Decision 7a
   (advisory flock on stable pre-created never-unlinked inode; durable txn record separate from the
   flock), §Decision 7b (framed checksummed intent log with WAL boundary; matching-destination-hash
   crash recovery table), §Decision 7c (single CURRENT.json atomic pointer swap as the sole
   commit-point; completed.json as the permanent terminal record). BC-1.18.006's `write_atomic`
   provides the per-file write primitive; ADR-052 §Decision 7 provides the multi-file atomic
   publication envelope that `write_atomic` alone cannot provide.

2. **No BC row is ever counted twice or dropped.** The independent census (Postcondition 2) is the
   sole source of truth for "did every row survive the split" — a migration that passes
   content-preservation (Postcondition 1) but fails the census (e.g., a byte-identical
   concatenation that nonetheless duplicates one row and drops another via a compensating error)
   is STILL a failing migration; both checks are independently mandatory, neither substitutes for
   the other.

3. **The migration is never partially applied.** At every observable point in time — before the
   migration runs, during staging, and after it completes — `BC-INDEX.md`'s body is either the
   FULL original monolithic form or the FULL split end-state form; it is never observed in a state
   where some subsystems are split and others are not (this BC operates on all ten subsystems, plus
   SS-05/SS-06's second-level sub-split, as one atomic unit — Postcondition 3/6). The all-or-nothing
   guarantee is implemented via the CURRENT.json atomic pointer swap (Precondition 5 [as amended])
   and the intent log: before the CURRENT.json pointer swap, the state machine is either STAGING
   (staging generation in progress, intent log recording per-target expected hashes) or has no txn
   record (no migration in progress); after the CURRENT.json pointer swap transitions to COMMITTING,
   canonical readers see the migration as in-progress and access new-generation content via the
   OPEN-based protocol with ENOENT fallback (ADR-052 §Decision 7c): for each required file,
   open `gen-<uuid>/<file>`; on ENOENT, open the canonical path. A file present at `gen-<uuid>/`
   has not yet been moved to canonical (new content); ENOENT on `gen-<uuid>/<file>` means the file
   has already been renamed to canonical (new content at canonical) — open the canonical path
   instead. `rename(2)` atomicity makes this protocol race-free and ENOENT-safe: a required file
   is never absent from both `gen-<uuid>/` and canonical during the COMMITTING window. There is no turning back. completed.json (written after all canonical path moves complete and are hash-verified) is
   the permanent terminal record; forward recovery uses the intent log + matching-hash rule to
   resume from the first uncompleted canonical path move.
   The CURRENT.json atomic pointer swap is the sole commit-point for the multi-file atomic
   operation; composing N independent `write_atomic` calls without this commit-pointer does not
   satisfy this invariant.

4. **This BC's own execution does NOT gate BC-7.08.001's Cohort B flip.** Per Postcondition 7, no
   implementation may introduce an undocumented sequencing dependency between this migration and
   the Cohort B fail-closed flip; the two are independent one-time operations that happen to be
   scheduled at the same F4 activation boundary.

5. **`admission_failure_cause` is an EXHAUSTIVE `match` over `BcIndexMigrationError` — no `_`
   wildcard arm (v1.18; normative text BC-1.18.013 Invariant 7; ADR-052 §Decision 5a "Admission
   state-integrity variant"; closes F-012).** It returns the closed `AdmissionFailureCause {
   InvalidToolUseId, Io, StateIntegrity }` (`fn token(self) -> &'static str`):
   `InvalidToolUseId → InvalidToolUseId`, `Io → Io`, `AdmissionStateIntegrity → StateIntegrity`,
   and every coordinator-only variant is listed explicitly in one arm mapping to
   `StateIntegrity` (unreachable from admission; the fail-closed classification if ever
   surfaced). Adding a variant is a compile error until it is classified.

6. **ONE anchoring function, three call sites (v1.18; BC-1.18.013 Invariant 8; ADR-052 §Decision
   5a "Single anchoring rule").** Admission, the release leg and BOTH coordinator binaries obtain
   the project root only from `resolve_session_project_root` and the factory root only from
   `resolve_factory_root`; `migration-state/` is only ever `FactoryRoot::migration_state_dir()`;
   no `join(".factory/…")` literal exists in `shard_manager.rs` outside the resolver (Precondition
   7; EC-027).

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Content-preservation (Postcondition 1) passes but independent census (Postcondition 2) finds a duplicated row across two shard files | Migration ABORTS per Postcondition 4 — passing ONE check is not sufficient; both are independently mandatory (Invariant 2) |
| EC-002 | Migration crashes after writing 6 of 10 first-level shard files to staging | Postcondition 3's atomicity guarantee: `BC-INDEX.md`'s original body is untouched (staging was incomplete and never promoted); the partial staged output is discarded on the next attempt, which restarts cleanly |
| EC-003 | A prior migration attempt left a complete, verified shard set in staging but crashed before the atomic-replace step | Re-running MUST detect the verified-complete staged state (txn record state = STAGING) and resume toward the pointer swap. **§Decision 7c step 3b (H3 fix):** resume-from-STAGING MUST re-run the full census (ADR-052 §Decision 7c step 3b, referenced by §Decision 4e) before proceeding to the pointer swap — the census gate is not skippable on resume even when the staged generation was previously verified complete. The census re-run uses the existing staged generation files; it does NOT re-run the full split from scratch (Postcondition 5's idempotency). |
| EC-004 | SS-05's second-level sub-split (Postcondition 6) produces sub-shards `.a`/`.b`/`.c` whose combined row count does not match an independent pre-split count of `BC-5.*` rows | Migration ABORTS for the entire operation (not just SS-05) per Postcondition 4 — a sub-shard-level census failure is treated with the same severity as a top-level census failure, since a partial-success outcome (nine subsystems split correctly, SS-05 corrupted) would still violate Invariant 3's all-or-nothing guarantee |
| EC-005 | An implementer mistakenly makes this migration a precondition for BC-7.08.001's Cohort B flip | Scope violation of Postcondition 7/Invariant 4 — the F2 architecture-delta doc's migration-impact map already confirms zero dependency; this BC introduces none |
| EC-006 | The migration is re-run after already completing successfully (no partial state, fully migrated) | Idempotent no-op: the migration detects `BC-INDEX.md`'s body is already in the split end-state (zero per-BC rows remain in the body, per BC-1.18.010 Invariant 3) and exits without re-splitting or re-writing any shard file |
| EC-007 | (v1.11, reserve-then-verify, ADR-052 D5) Admitter (W1 create reservation, W2 read gate) versus coordinator (C1 durable DRAINING flip, C2 read `reservations/`) at every ordering of {W1, C1, C2, W2} | Either C2 lists the reservation (coordinator waits; `source_sha256`/`source_body_row_sha256` not computed while it exists) or W2 reads DRAINING (admission blocked `E-MAINTENANCE-001`, own reservation removed) — never both miss; an admitter that read `OPEN` from the pre-`rename` gate-file inode is still observed by the coordinator (Precondition 6(c)) |
| EC-008 | (v1.11, release-on-block) Admission has created `reservations/<tool_use_id>.reservation` and a later stage of the same dispatch (`shard_cap_precheck` or a registry plugin) blocks/errors, or the admitter's own verification fails | The dispatcher removes that reservation before exit; no reservation file remains for a blocked/errored event |
| EC-009 | (v1.11, TTL) (a) A reservation with `created_at` older than 3,600 s (test seam: injected smaller) is GC'd at drain step 1; (b) younger ⇒ never removed by the coordinator; (c) `created_at` absent/unparseable/untrusted ⇒ file mtime is used (v1.16: full timestamp rules and vectors in EC-020); (d) (v1.18, F-006) the production entry delegates to the crate-private `run_bc_index_migration_with_ttl(project_root, max_reservation_ttl)` passing `DEFAULT_MAX_RESERVATION_TTL`; that function's FIRST statement is `validate_production_reservation_ttl`; below-floor input (120 s, 1,799 s) versus 1,800 s / 3,600 s versus the compile-time default-constant check | (a) removed, drain proceeds; (b) drain waits then `DRAIN_TIMEOUT_ABORT` at 30 s; (c) mtime fallback judged by the same TTL; (d) configuration error (rejected): `RESERVATION_TTL_BELOW_FLOOR` (`BcIndexMigrationError::ReservationTtlBelowFloor { configured_secs, floor_secs: 1800 }`, exit 2) returned BEFORE any gate/drain action, nothing mutated (a byte-identical `migration-state/` snapshot, no `exclusive.lock` created), not `BINARY_INTEGRITY_FAILURE` (v1.16, F-009); 1,800 s and 3,600 s accepted; the test seam `drain_bc_index_writers` is not bound by the floor; `DEFAULT_MAX_RESERVATION_TTL >= MIN_PRODUCTION_RESERVATION_TTL` is enforced at COMPILE time by `const _: () = assert!(…)`; the seam is crate-private — NOT `pub`, NOT environment/argv-injectable; the production default constant is 3,600 s, NOT the merged 120 s; no PID-liveness anywhere |
| EC-010 | (v1.11, wiring) Gate LOCKED/DRAINING with no active txn (Branch A), or STAGING with `generation_id = null` (Branch B), and a PreToolUse `Edit`/`Write` under `.factory/specs/behavioral-contracts/` or `.factory/cycles/` arrives through the REAL dispatcher entry with `exclusive.lock` acquirable | The production admission path runs the §5a step-3.5 reconciliation (A: gate → OPEN; B: txn → ABORTED + gate → OPEN) and then admits; with `exclusive.lock` held by a live coordinator (EWOULDBLOCK) no action and `E-MAINTENANCE-001` block. A reconciler unit-tested but not called on the production path FAILS this EC |
| EC-011 | (v1.11, D3) `completed.json` present and valid, txn COMMITTING (matching `activation_id`/`generation_id`, `migration_id` = `migrate-bc-index` or absent), gate LOCKED, all canonical hashes at `expected_post_hash` (crash between `completed.json` fsync and the txn→COMPLETED rewrite) | (a) next PreToolUse (no binary re-invocation) runs Branch C: txn → COMPLETED THEN gate → OPEN, admitted; (b) a binary re-invocation instead finalizes identically and exits 0 `ALREADY_MIGRATED`; (c) a second run is zero-mutation; (d) crash between txn rewrite and gate flip ⇒ txn COMPLETED + gate≠OPEN ⇒ Branch A repairs next dispatch; (e) lock held by live coordinator ⇒ no action, blocked (Postcondition 9) |
| EC-012 | (v1.11, D3 mismatch) Same fixture but `completed.json.txn_id` ≠ `activation_id`, or `generation_id` ≠, or `canonical_paths_count` ≠ N, or any canonical hash ≠ `expected_post_hash`, or txn STAGING (always fail-closed; no verification is attempted), or COMMITTING with a different `activation_id` | NO txn write, NO gate write; PreToolUse blocks `E-MAINTENANCE-001` with the path-family-keyed message plus the ` (completion-record mismatch — operator investigation required)` suffix (reason also logged; v1.13 — including the B2 STAGING + `completed.json` case); binary exits 2 `COMPLETION_RECORD_MISMATCH_ABORT`; `.factory/migration-state/` byte-for-byte unchanged. (A live `backfill-append-logs` txn is NOT this EC — in the binary it is the cross-migration refusal, EC-013; on the dispatcher path it is decided against ITS OWN record (EC-022/EC-023); B2's `completed.json` NEVER finalizes it, Postcondition 9(a).) |
| EC-013 | (v1.11, D4; v1.16: **binary recovery path only**) The single live txn has `migration_id = "backfill-append-logs"` when the `migrate-bc-index` BINARY is invoked; or a txn record lacks `migration_id` | Other-known-migration live txn ⇒ cross-migration refusal (binary recovery path only; the dispatcher/shared-core path never refuses between the two known ids — it decides against the txn's own migration's record, EC-022/EC-023): exit 2 (`LockContention`-class), no mutation, `recover()` not run, txn untouched. Absent `migration_id` ⇒ treated as `"migrate-bc-index"` (legacy record stays valid and recoverable by this binary) |
| EC-014 | (v1.11, D4) `completed-backfill-append-logs.json`/`CURRENT-backfill-append-logs.json` exist (mechanism-A finished) while `completed.json`/`CURRENT.json` do not | `migrate-bc-index` is NOT `ALREADY_MIGRATED`; `detect_migration_read_state` does NOT report the BC-INDEX migration complete (readers keep legacy paths); B2 writes only `completed.json`/`CURRENT.json` and `backfill-append-logs` never does (Precondition 6(e)) |
| EC-015 | (v1.15, first-activation race; S-25.08 finding) A protected-path PreToolUse `Edit`/`Write`/`MultiEdit` (valid `tool_use_id`) is admitted in a repository where `.factory/migration-state/` does NOT yet exist; while it is in flight (PostToolUse not fired) `migrate-bc-index` activates (creates the directory, flips DRAINING, polls `reservations/`) | The admitter created `migration-state/reservations/` idempotently and its reservation BEFORE reading the gate (Precondition 6(c) "Unconditional reservation namespace"); the coordinator's C2 lists it and WAITS (`source_sha256`/`source_body_row_sha256` not computed while it exists) until PostToolUse removes it, else `DRAIN_TIMEOUT_ABORT` at 30 s with canonical files byte-identical; the §7c step-5 fingerprint abort is NOT the handling mechanism. Concurrent first-ever admissions all succeed (idempotent create). No `tool_use_id` ⇒ check-only, no directory/reservation. Creation failure ⇒ fail-closed `HookResult::Error` `E-MAINTENANCE-002` (`io`, v1.16), no reservation left behind |
| EC-016 | (v1.16, F-001) A protected-path `Edit`/`Write`/`MultiEdit` is admitted (reservation created), the tool call then FAILS, is interrupted or is cancelled, so the harness sends `PostToolUseFailure` (NOT `PostToolUse`) for the same `tool_use_id` — including (a) `is_interrupt: true`, (b) an envelope with NO `tool_name`, (c) a differently-shaped `tool_name` | The reservation is REMOVED (exactly that file; keyed only on `tool_use_id`, no `tool_name` filter); `is_tool_completion_event` is true for exactly `"PostToolUse"` and `"PostToolUseFailure"`; a `PostToolUseFailure` for an id with no reservation or an invalid-grammar id is a silent no-op; a non-ENOENT release error is a non-fatal warn; no new `EventType` variant; neither Post event ⇒ TTL backstop (documented residual) |
| EC-017 | (v1.16, F-004; O1–O4) Real dispatcher PreToolUse for a protected-path write with a live txn (or gate DRAINING/LOCKED) and: (a) `CLAUDE_PLUGIN_ROOT` unset/empty; (b) registry file missing; (c) registry unparseable; (d) registry schema-version mismatched; (e) empty matched-plugin set; separately (f) a registry that makes the dispatch fail CLOSED (non-zero) after an admitted verdict; (g) unparseable stdin | (a)–(e) The gate still enforces: `E-MAINTENANCE-001` exit 2 without loading the registry (gate OPEN/no txn: admitted, reservation standing, and the matching PostToolUse/PostToolUseFailure releases under the same broken registry); (f) the admission's reservation is removed before exit (release-on-block funnel); (g) the existing parse-error exit, nothing created; the core runs EXACTLY ONCE and BEFORE `shard_cap_precheck` |
| EC-018 | (v1.16, F-002) A protected-LOOKING path outside the session's `factory_root` (another project's, a nested project's `sub/.factory`, a scratch tree, a look-alike `x.factory/…`); (e) `<project>/.factory` is a SYMLINK to a real directory; (f) the project has NO `.factory` directory; (g) (v1.17, lexical root spellings) the project directory is reached through a SYMLINK spelling (macOS `/var/…` vs canonical `/private/var/…`; symlinked checkout) and `file_path` is spelled through the NON-canonical `CLAUDE_PROJECT_DIR` as given, with `T_real` UNAVAILABLE | Out-of-root (a)–(d): admitted, NO reservation, NO directory creation, NO migration-state read, even with a live txn in the session's own tree; (e) `factory_root` is the REAL directory — both spellings gated/reserved in that one migration-state; (f) out of scope, admitted, NO `.factory` (or anything under it) created; (g) IN SCOPE via the as-given lexical spelling even though `T_real` is unavailable: blocked `E-MAINTENANCE-001` under a live txn, else admitted with a reservation in the canonical `<factory_root_real>/migration-state/reservations/` |
| EC-019 | (v1.16, F-003) Path aliasing of a protected target: `.`/`..`/`//` segments; a symlink alias into the tree; `<tmp>/link/../…` (POSIX resolves `link` before `..`); a nonexistent tail; mixed-case family names (`.FACTORY/Cycles/…`); a relative `file_path` with payload `cwd`; a `\` in a name on Unix; an unresolvable component (EACCES/ELOOP/>40 hops); (v1.17, vector 1) the SAME protected target spelled via the canonical root form and via the as-given `CLAUDE_PROJECT_DIR/.factory` form; (v1.17, vector 2) a look-alike / other-project path sharing a symlink-ancestor spelling with NEITHER alias of the session's own root | In scope iff the resolved OR the lexical form matches (union, fail-closed): blocked `E-MAINTENANCE-001` (scope token of the path FAMILY) while a window is active, else admitted with a reservation; `link/..` classified by the REAL resolved path; unresolvable ⇒ classified on `T_lex` alone; `\` is an ordinary byte on Unix; case always folded, no filesystem probe; missing/non-string/empty/NUL `file_path` out of scope (not an error); `~` never expanded; (v1.17 vector 1) classified IDENTICALLY under both spellings (same verdict, scope token, block-or-reserve outcome, canonical migration-state); (v1.17 vector 2) OUT OF SCOPE, no over-match: admitted, NO reservation, NO directory creation, NO state read |
| EC-020 | (v1.16, F-008) Reservation timestamp cases (TTL 3,600 s seam): `created_at` = `now+299 s`; `now+301 s`; pre-1970; non-RFC-3339; year 9999; outside `u64`; non-UTC offset; and `created_at` unusable with mtime future / unavailable / pre-epoch | `now+299 s` ⇒ age 0, retained; (each `warn X` below = exactly one coordinator-drain STDERR advisory line with token X — NOT a dispatcher event; v1.18 follow-up, item 33(c)) `now+301 s` ⇒ untrusted ⇒ mtime (warn `created_at_future`); pre-1970 ⇒ unparseable, never clamped to 0 ⇒ mtime (warn `created_at_pre_epoch`); non-RFC-3339 / outside `u64` ⇒ mtime (warn `created_at_unparseable`); year 9999 ⇒ untrusted ⇒ mtime (warn `created_at_future`); non-UTC offset normalised to UTC; mtime future ⇒ age 0 retained (warn `mtime_future`); both unusable ⇒ age unknown ⇒ NOT stale (warn `age_unknown`); `reservation_is_stale(Option<u64>, Option<u64>, u64, u64) -> bool`, skew constant 300 |
| EC-021 | (v1.16, F-009 part 2) Protected-path PreToolUse whose `tool_use_id` is a number / empty string / `../x` / 129-char string / `.hidden` / boolean, versus ABSENT, JSON `null`, a valid 128-char id | The first six FAIL CLOSED `E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)` (exit 2), no reservation, no directory, raw id never logged (byte length only); ABSENT/`null` ⇒ check-only admit (no directory, no reservation); valid ⇒ reservation; release of an invalid id is a silent no-op |
| EC-022 | (v1.16, F-006) A live txn of the OTHER known migration — `backfill-append-logs`, STAGING with `generation_id` set, or COMMITTING — with ITS OWN terminal record (`completed-backfill-append-logs.json`) ABSENT, while B2's `completed.json` IS present and valid; dispatcher PreToolUse under either path family | `txn_migration_known = true`; row 3 `NoOp` (no txn/gate write; `completed.json` is NEVER consulted); STAGING `generation_id = null` ⇒ Branch B, else the ordinary admission blocks with the PLAIN `E-MAINTENANCE-001` (no suffix); NOT `RefuseForeignMigration`, not finalized, not a mismatch; migration-state byte-identical |
| EC-023 | (v1.16, F-006) Same live `backfill-append-logs` txn, COMMITTING, with ITS OWN `completed-backfill-append-logs.json` present and verifying (N = 4), with or without `completed.json` present | Branch C finalize: txn → COMPLETED THEN gate → OPEN, admitted — selected by `migration_id`, independent of B2's record; the mirror (live `migrate-bc-index` COMMITTING + its own verifying `completed.json`) finalizes identically |
| EC-024 | (v1.16, F-006) A live txn whose `migration_id` is a string NOT in K (e.g. `"future-migration"`, a 1,000-char id, an id with control characters), STAGING (null and non-null `generation_id`) or COMMITTING, terminal records present or absent | FOREIGN ⇒ `RefuseForeignMigration` (row 2): plain `E-MAINTENANCE-001`; NEVER finalized, NEVER aborted — Branch B's discard does NOT run; no repair; migration-state byte-identical; exactly one `migration.admission_blocked` event (`branch = foreign_migration`, `reconciliation = foreign_migration_refused`) is written, and the raw id appears only in its `migration_id` field, sanitized (≤ 64 chars, control chars escaped) (v1.18; Postcondition 10) |
| EC-025 | (v1.16, F-006/F-009) A live txn whose `migration_id` is a PRESENT non-string (number, JSON `null`, array, object, boolean) | Malformed record ⇒ `E-MAINTENANCE-002: writer-admission check failed (state_integrity)` (`HookResult::Error`, exit 2), fail-closed, NOT "foreign", no reservation left behind, no writes |
| EC-026 | (v1.16, S-25.08 AC-018 ambiguity; Precondition 6(d) "`<cause>` classification") An admission-time read of `gate-state.json` (or a `txn-*.json` record, or the Branch C terminal record) that FAILS (OS call error other than ENOENT) versus one that SUCCEEDS but yields unusable content (non-UTF-8, empty/truncated/unparseable, wrong JSON type/shape, unknown state value, non-string `migration_id`, >1 live txn) | OS read failure ⇒ `E-MAINTENANCE-002 (io)`; bytes read but unusable ⇒ `E-MAINTENANCE-002 (state_integrity)`; read failure wins when both would apply; terminal record: read failure ⇒ `io`, unverifiable content (unparseable, wrong schema, count/id/hash mismatch) ⇒ NOT `E-MAINTENANCE-002` (v1.17: `E-MAINTENANCE-001` WITH the completion-record-mismatch suffix, not finalized); reservation files are never read by admission; first-failure-wins (tool_use_id → W1 → gate-state → txn ascending → terminal record); fail-closed, no reservation left behind, no writes; **(v1.18, F-012)** the `state_integrity` carrier is `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }`, NOT `BinaryIntegrityFailure`, with `kind` ∈ {`gate_record_malformed`, `txn_record_malformed`, `txn_migration_id_not_string`, `multiple_live_txns`, `reservation_serialization`}; its Display `migration admission: state integrity failure (<kind-token>): <detail>` does NOT contain `BINARY_INTEGRITY_FAILURE`; each failing fixture writes exactly one `migration.admission_failed` event carrying `cause` and, for `state_integrity`, the matching `kind` |
| EC-027 | (v1.18, D-2) The process cwd of the `migrate-bc-index` invocation differs from `CLAUDE_PROJECT_DIR=<P>`, and admission under the same env left a reservation `T1` | The coordinator operates on — and creates `migration-state/` only under — `<P>/.factory`; its first drain observes `<P>/.factory/migration-state/reservations/T1.reservation`; NOTHING is created under the process cwd or any other directory; no `join(".factory/…")` literal outside the resolver in `shard_manager.rs` |
| EC-028 | (v1.18, D-2) `resolve_session_project_root(claude_project_dir, process_cwd)` over `Some(<abs existing dir>)`; `Some("")`; `None`; `Some(<abs nonexistent path>)`; `Some(<symlink to dir>)`; a `process_cwd` below a git work-tree root that has a `.factory` | The canonicalized path; `process_cwd`; `process_cwd`; the as-given path (NEVER the cwd); the resolved path; `process_cwd` exactly (no ancestor walk, no `git rev-parse`); the function is pure (testable without environment mutation) |
| EC-029 | (v1.18, D-2) The resolved project root has NO `.factory` directory (absent, or a regular file) when `migrate-bc-index` (or `backfill-append-logs`) runs | Exit 2 `FACTORY_ROOT_NOT_FOUND` (`FactoryRootNotFound { project_root, root_source }`, `root_source: Option<ProjectRootSource>`) with the normative stderr line `<subcommand>: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root> (resolved from <source>)`, `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`} — on the CLI the suffix is ALWAYS present (Precondition 7 rule (a)); only a path-only library entry (`run_bc_index_migration(&Path)` / crate-private `run_bc_index_migration_with_ttl(&Path, ttl)`) yields `root_source = None` and the suffix-less `Display`; nothing created or mutated (no `.factory`, no `migration-state/`, no `exclusive.lock`); the tree is byte-identical before and after |
| EC-030 | (v1.18, D-2) `<P>/.factory` is a symlink to a real factory directory, with admission and the coordinator each under `CLAUDE_PROJECT_DIR=<P>` | One shared REAL `migration-state/` namespace: admission's reservations and the coordinator's drain/gate/txn all live in the real directory; no second namespace under `<P>/.factory` |
| EC-031 | (v1.18, D-1) A protected-path write is blocked `E-MAINTENANCE-001` in each branch: gate-only; live txn; live coordinator; foreign migration; Branch C verification failure | Exactly ONE `migration.admission_blocked` event per blocked dispatch, written before the early return, with `scope`/`family` per the written path family, `branch` ∈ {`gate_only`, `live_txn`, `live_coordinator`, `foreign_migration`, `completion_record_mismatch`}, `gate_state`, `migration_id`/`txn_id` (sanitized; `null` for gate-only), `check` (non-null only for the mismatch branch), `reconciliation`; unchanged stderr message; no raw `tool_use_id` or record content; asserted against `dispatcher-internal-*.jsonl`, never a `tracing` capture |
| EC-032 | (v1.18, D-1) A protected-path write fails `E-MAINTENANCE-002` for each cause (`invalid_tool_use_id`, `io`, `state_integrity`) | Exactly ONE `migration.admission_failed` event per failed dispatch: `cause` = the token; `kind` = the `AdmissionStateIntegrityKind` token for `state_integrity`, else `null`; `detail` = path / `io::Error` kind+message / byte length; never the raw `tool_use_id` (byte length only) nor record content |
| EC-033 | (v1.18, D-1; reworded in the v1.18 follow-up per item 33(c)/(d)) Non-verdict anomalies: a non-ENOENT release error; Branch A gate re-open; Branch B null-generation discard; `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` with the finalize effect undelivered (S-25.08 seam); S-25.06's successful verify-then-finalize; and (separately) a reservation-timestamp fallback at drain GC | Exactly ONE `migration.admission_advisory` event per anomaly with `reason` = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired` / `branch_c_finalized` (the closed five-token domain); `branch_c_finalize_unwired` is written IN ADDITION to the one `migration.admission_blocked`; `branch_c_finalized` is admitted (no `_blocked`) and never emitted by S-25.08; optional context fields only from {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}, any other field forbidden; the dispatch outcome of the release/Branch A/Branch B cases is unchanged; the timestamp fallback writes NO dispatcher event — only a coordinator-drain stderr line with the `created_at_*` / `mtime_future` / `age_unknown` token |
| EC-034 | (v1.18, F-013) A protected-path write blocked/errored by the migration admission gate versus a write blocked by `shard_cap_precheck` | The admission verdict's stderr summary line carries `blocking_plugins=migration-admission` (`block_reason` = the unchanged `E-MAINTENANCE-001`/`E-MAINTENANCE-002` text, exit 2); the shard-cap block still carries `blocking_plugins=shard-cap-gate` |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `BC-INDEX.md` at 2,005 total BCs across 10 `### SS-NN` sections, `total_bcs: 2005` in frontmatter | Pre-split census enumerates exactly 2,005 unique `BC-X.YY.NNN` IDs, matching `total_bcs`; post-split, the union of all 10 (or more, with SS-05/SS-06 sub-shards) shard files' row counts is exactly 2,005; `BC-INDEX.md`'s body retains zero per-BC rows | happy-path |
| SS-05 (661 BCs, ~88,695 bytes) and SS-06 (592 BCs, ~85,407 bytes) both exceed the provisional cap | Both receive second-level sub-splits (e.g. SS-05 → `.a`/`.b`/`.c`) in the SAME migration operation; SS-05's sub-shard row counts sum to exactly 661, SS-06's to exactly 592 | happy-path |
| Content-preservation check finds a byte mismatch (one row's trailing whitespace altered during extraction) | Migration ABORTS; original `BC-INDEX.md` body untouched; fail-loud CONTENT_PRESERVATION_ABORT (process exit code) | error |
| Independent census finds a `BC-3.14.002` row present in BOTH `shards/BC-INDEX-SS-03.md` and (erroneously) `shards/BC-INDEX-SS-04.md` | Migration ABORTS per Postcondition 4/EC-001; fail-loud CENSUS_MISMATCH_ABORT (process exit code) naming the duplicated ID | error |
| Migration crashes mid-staging, restarted from scratch | Original `BC-INDEX.md` byte-identical to pre-crash state; restart produces the same split result as an uninterrupted run | error |
| Migration re-run after a prior successful completion | No-op: zero shard files rewritten, `BC-INDEX.md` body unchanged (EC-006) | edge-case |
| (v1.11, D5) Scripted admitter/coordinator at each of the 4 orderings of {W1, C1, C2, W2} (W1<W2, C1<C2) | Never "both miss": C2 sees the reservation, or W2 sees DRAINING and the admitter's reservation is removed (EC-007) | error |
| (v1.11) Admission creates `reservations/T7.reservation`, then `shard_cap_precheck`/a registry plugin blocks the same event | Dispatcher exit 2; `reservations/T7.reservation` absent (EC-008) | error |
| (v1.11) Reservation `created_at = now − 4000 s`, TTL 3,600 s (seam); another with no `created_at` and mtime `now − 4000 s`; another `created_at = now` | First two reclaimed at drain step 1; third blocks quiescence until PostToolUse or 30 s `DRAIN_TIMEOUT_ABORT`; production default constant asserted 3,600 s (not 120 s), floor 1,800 s (EC-009) | edge-case |
| (v1.11, wiring) Real spawned dispatcher, PreToolUse `Edit` under `.factory/specs/behavioral-contracts/`, gate `LOCKED`, no txn, lock acquirable; and gate `DRAINING`, txn STAGING `generation_id=null` | First: gate → OPEN then admitted; second: txn → ABORTED (`null_generation`), gate → OPEN, admitted (EC-010) | error |
| (v1.13, Branch B marker) Gate DRAINING, txn STAGING `generation_id=null`, `source_sha256=null`, no terminal record, lock acquirable; real-dispatcher PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` | Admitted; the SAME `txn-<uuid>.json` file still exists with `"state": "ABORTED"` and `"abort_reason": "null_generation"`, `generation_id` and `source_sha256` still `null`, `activation_id`/`migration_id` unchanged; gate `OPEN`; a second PreToolUse leaves the txn file byte-identical (Branch A no-op); a txn file lacking `abort_reason` and one with `abort_reason: "future-value"` both parse and are non-live (EC-010) | error |
| (v1.13, keying, B2 txn × cycles path) Gate LOCKED, txn `{migration_id: "migrate-bc-index", COMMITTING}`, no `completed.json`, lock held or terminal record absent; PreToolUse `Edit` of `.factory/cycles/c1/burst-log.md` | Exit 2; stderr contains exactly `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts`, no `completion-record mismatch`; the same txn with an `Edit` under `.factory/specs/behavioral-contracts/` yields `BC-INDEX write blocked: …` (same tail) (Precondition 6(b), BC-1.18.013 keying rule) | error |
| (v1.13, NoOp cell) Own-migration (`migrate-bc-index`) live txn — STAGING `generation_id=gen-1`, and separately COMMITTING — lock acquired, `completed.json` ABSENT; PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` | `decide_terminal_record_reconciliation` ⇒ `NoOp` in both cases (no txn write, no gate write); ordinary admission blocks with the PLAIN `BC-INDEX write blocked: …` message, no mismatch suffix; migration-state byte-identical (Precondition 6(d) table row 3) | error |
| (v1.13, STAGING + terminal record suffix) Gate LOCKED, B2 txn STAGING (`generation_id` null, and separately `gen-1`) with `completed.json` present; PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` | Exit 2; stderr = `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts (completion-record mismatch — operator investigation required)`; Branch B NOT applied (txn stays STAGING, `generation_id` unchanged); migration-state byte-identical (EC-012, Postcondition 9(d)) | error |
| (v1.11, D3) `completed.json` valid, txn COMMITTING matching, gate LOCKED, canonical files at `expected_post_hash`; then (a) PreToolUse Edit with no binary run, (b) separately `migrate-bc-index` re-run | (a) txn COMPLETED then gate OPEN then admitted; (b) exit 0 `ALREADY_MIGRATED`, same end state; second run zero-mutation (EC-011) | error |
| (v1.11, D3 mismatch) Same fixture but `completed.json.txn_id` ≠ `activation_id` (separate fixtures: one canonical hash differs; txn STAGING + terminal record (always fail-closed, no verification attempted); `canonical_paths_count` ≠ N) | Binary exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`; PreToolUse `E-MAINTENANCE-001` (path-family-keyed message + ` (completion-record mismatch — operator investigation required)` suffix, v1.13) with reason logged; txn still COMMITTING/STAGING, gate still non-OPEN; migration-state byte-identical (EC-012) | error |
| (v1.11, D4) `completed.json` valid, live txn `{migration_id: "backfill-append-logs", COMMITTING}`, `migrate-bc-index` invoked / PreToolUse Edit | binary: refusal exit 2 (LockContention-class; binary recovery path only, EC-013) / PreToolUse: plain `E-MAINTENANCE-001` block (v1.16: the live `backfill-append-logs` txn is decided against ITS OWN record `completed-backfill-append-logs.json`, ABSENT ⇒ row 3 `NoOp` ⇒ plain block, never against `completed.json`, EC-022); txn NOT finalized, gate NOT opened | error |
| (v1.11, D4) Txn record JSON without `migration_id`, state COMMITTING; `migrate-bc-index` invoked | Treated as `migrate-bc-index`; recovered normally (EC-013) | edge-case |
| (v1.11, D4) Only `completed-backfill-append-logs.json` + `CURRENT-backfill-append-logs.json` present; `migrate-bc-index` invoked and `detect_migration_read_state` called | Not `ALREADY_MIGRATED`; reader state not "complete" (EC-014) | edge-case |
| (v1.15, EC-015 — no pre-existing directory) Fresh project tempdir with `.factory/` but NO `.factory/migration-state/`; PreToolUse `Write` with `tool_use_id=T1` to `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md` via `migration_writer_admission_precheck`; separately the same payload with NO `tool_use_id` | `None` (admitted); `.factory/migration-state/reservations/T1.reservation` EXISTS (`created_at`, `tool_use_id=T1`, no PID), no `gate-state.json`/txn written by the admitter; PostToolUse for `T1` removes exactly that file. The no-`tool_use_id` payload: admitted check-only, NO directory and NO reservation created. (Replaces the pre-v1.15 expectation "no `migration-state/` ⇒ zero-cost no-op `None` with nothing created") | edge-case |
| (v1.15, EC-015 — drain observes pre-directory writer) Same fixture; after the PreToolUse admit of `T1` (PostToolUse withheld) the `migrate-bc-index` drain (TTL GC → flock → DRAINING → txn STAGING null → poll) runs with a short injected drain timeout; variant (b): PostToolUse for `T1` fires mid-poll | (a) poll sees `T1.reservation`; `source_sha256` stays `null`; exit 2 `DRAIN_TIMEOUT_ABORT`; gate → OPEN; canonical files byte-identical; (b) quiescence after release, snapshot includes the completed write — never a fingerprint abort | error |
| (v1.16, EC-016 — PostToolUseFailure release) Real dispatcher PreToolUse (gate OPEN, no txn) `Write` `tool_use_id=T10` under `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md` ⇒ `reservations/T10.reservation` exists; then a real-binary `PostToolUseFailure` envelope `{hook_event_name: "PostToolUseFailure", tool_name: "Write", tool_input: {...}, tool_use_id: "T10", error: "boom", is_interrupt: false}`; repeat with `is_interrupt: true`, `tool_name` omitted, `tool_name: "Weird"`; and a `PostToolUseFailure` for `T99` with no reservation | After each failure envelope `T10.reservation` is ABSENT; the T99 envelope is a silent no-op (exit 0); no `PostToolUse` sent; `is_tool_completion_event` true for `"PostToolUse"`/`"PostToolUseFailure"`, false for `"PreToolUse"`, `"Stop"`, `""`, `"posttooluse"` | happy-path |
| (v1.16, EC-017 — registry-independent) Fixtures: `CLAUDE_PLUGIN_ROOT` unset + no registry; registry unparseable TOML; registry `schema_version` mismatched; each with a live txn and a PreToolUse `Write` under `.factory/specs/behavioral-contracts/`; and each with gate OPEN/no txn, `Write` `tool_use_id=T11` then `PostToolUse` `T11`; plus a registry that fails the dispatch CLOSED after admission; plus an unparseable stdin payload | Block fixtures: exit 2 with the `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` string, registry never loaded; admit fixtures: `T11.reservation` created at Pre and REMOVED at Post under the same broken registry; fail-closed-after-admit: reservation absent at exit; unparseable stdin: pre-existing parse-error exit, nothing created; core ran once, before `Registry::load` and `shard_cap_precheck` (EC-017) | error |
| (v1.16, EC-018 — out-of-root / symlinked / no `.factory`) Project P (own `.factory/`, live txn) and second tempdir Q; PreToolUse `Write` to `Q/.factory/specs/behavioral-contracts/ss-01/x.md`, `P/sub/.factory/cycles/c1/a.md`, `P/x.factory/cycles/c1/a.md`; (e) `P/.factory` → symlink to `R/real-factory` (live txn), `Write` to both `P/.factory/…` and `R/real-factory/…`; (f) project with NO `.factory` | Out-of-root writes admitted, NO reservation, NO `migration-state/` created in Q/`P/sub`/`P/x.factory`, P's migration-state byte-identical; (e) both spellings blocked identically (one real `factory_root`); (f) admitted, tree byte-identical, no `.factory` created (EC-018) | edge-case |
| (v1.16, EC-019 — path aliasing) Live txn; PreToolUse `Edit` to `<P>/.factory/specs/./behavioral-contracts/x.md`, `<P>/.factory/specs/../specs/behavioral-contracts/x.md`, `<P>/.factory//cycles//c1//a.md`, nonexistent-tail `<P>/.factory/cycles/new/n.md`, relative `.factory/cycles/c1/a.md` with `cwd=<P>`, symlink alias `<P>/alias/c1/a.md` → `.factory/cycles`, `<P>/link/../.factory/cycles/c1/a.md`, `<P>/.FACTORY/Cycles/c1/a.md` (case-sensitive volume), unresolvable-component path, `<P>/.factory/cycles\c1\a.md` on Unix | All but the Unix-`\` path blocked `E-MAINTENANCE-001` (scope token by path FAMILY; `link/..` resolved POSIX-correctly with the lexical union; unresolvable ⇒ `T_lex` fail-closed); the Unix-`\` path is one ordinary component and is NOT classified `Cycles`; with gate OPEN/no txn each in-scope path admitted with a reservation (EC-019) | error |
| (v1.17, EC-018(g) — as-given spelling, `T_real` unavailable) `CLAUDE_PROJECT_DIR` = `<alias>/proj` where `<alias>` is a symlink to the real parent (canonical `<real>/proj`); the project's `.factory` ancestor chain is made unresolvable by a read-failing seam so `T_real` is unavailable; PreToolUse `Write` with `file_path=<alias>/proj/.factory/cycles/c1/a.md` and valid `tool_use_id`, (1) live txn, (2) gate OPEN/no txn | (1) blocked `E-MAINTENANCE-001` (scope `.factory/cycles/`); (2) admitted, reservation created in `<real>/proj/.factory/migration-state/reservations/` and NOT under `<alias>/…` (EC-018 g) | edge-case |
| (v1.17, EC-019 vector 1 — canonical vs as-given spelling) Same fixture; the same protected target once as `<real>/proj/.factory/specs/behavioral-contracts/x.md` and once as `<alias>/proj/.factory/specs/behavioral-contracts/x.md`, under (1) live txn, (2) gate OPEN | Identical outcome for both spellings: (1) both blocked with scope token `BC-INDEX`; (2) both admitted with a reservation in the one canonical migration-state (EC-019) | edge-case |
| (v1.17, EC-026 — terminal record present but unverifiable) Live B2 txn (STAGING `gen-1` and separately COMMITTING) with `completed.json` present as non-UTF-8 / zero-length / truncated / wrong-schema / `canonical_paths_count` = 3 / mismatching `txn_id` / hash mismatch; and separately mode `000` (read call fails); PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` | Content failures: `E-MAINTENANCE-001` keyed message WITH ` (completion-record mismatch — operator investigation required)`, not finalized, files byte-identical; read-call failure: `E-MAINTENANCE-002 (io)`; record ABSENT: the PLAIN suffix-less message (EC-026) | error |
| (v1.17, EC-019 vector 2 — look-alike / other project, no over-match) Same fixture, live txn; PreToolUse `Write` to `<alias>/other/.factory/cycles/c1/a.md` (sibling project through the same symlinked parent), `<alias>/proj-old/.factory/cycles/c1/a.md`, `<alias>/proj/sub/.factory/cycles/c1/a.md` | All admitted: NO reservation, NO directory creation, NO migration-state read; session's migration-state byte-identical (EC-019) | edge-case |
| (v1.16, EC-020 — timestamps) Reservations with TTL 3,600 s (seam), fixed `now`: `created_at`=`now+299 s`; `now+301 s` + mtime `now−4000 s`; `1969-12-31T23:59:59Z` + mtime `now−4000 s`; `"yesterday"` + mtime `now−4000 s`; `9999-12-31T23:59:59Z` + mtime `now−4000 s`; unusable + mtime `now+1000 s`; unusable + mtime unavailable; `now−4000 s` with a `+05:30` offset | In order (each `warn X` = exactly one coordinator-drain stderr advisory line with token X; no `migration.admission_advisory` event): retained; reclaimed (warn `created_at_future`); reclaimed (warn `created_at_pre_epoch`, not clamped); reclaimed (warn `created_at_unparseable`); reclaimed (warn `created_at_future`); retained (warn `mtime_future`); retained (warn `age_unknown`); reclaimed (UTC-normalised) (EC-020) | edge-case |
| (v1.16, EC-021 — invalid `tool_use_id`) PreToolUse `Write` to a protected path (gate OPEN, no txn) with `tool_use_id`: `12345`; `""`; `"../x"`; a 129-char string; `".hidden"`; `true`; versus ABSENT; `null`; a valid 128-char id | First six: exit 2 `E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)`, no reservation/directory, raw id never in output (length only); ABSENT/`null`: check-only admit, nothing created; valid: admitted + reservation (EC-021) | error |
| (v1.16, EC-022 — other known migration, own record absent) Live `backfill-append-logs` txn (STAGING gen-1; and COMMITTING), `completed-backfill-append-logs.json` ABSENT, B2 `completed.json` present and valid; PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` and `.factory/cycles/`; and STAGING `generation_id=null` | Row 3 `NoOp` for the first two (plain `E-MAINTENANCE-001`, path-family message, NO suffix, txn not finalized/aborted, migration-state byte-identical); the null-generation fixture ⇒ Branch B (txn → ABORTED + `abort_reason: "null_generation"`, gate → OPEN, admitted) (EC-022) | error |
| (v1.16, EC-023 — other known migration, own verifying record) Live `backfill-append-logs` txn COMMITTING with a valid, matching `completed-backfill-append-logs.json` (N = 4), gate LOCKED, with and without `completed.json` present; PreToolUse `Edit` | txn → COMPLETED THEN gate → OPEN, admitted, identical with/without B2's record; mirror fixture for `migrate-bc-index` + its own `completed.json` identical (EC-023) | error |
| (v1.16, EC-024 — unknown `migration_id`) Live txn `{migration_id: "future-migration"}` (STAGING null-gen, STAGING gen-1, COMMITTING; terminal records present and absent); a 1,000-char id with embedded control characters | Plain `E-MAINTENANCE-001`; txn NEVER aborted/finalized (no Branch B); migration-state byte-identical; exactly one `migration.admission_blocked` event (`branch = foreign_migration`) shows the id in `migration_id`, truncated to 64 chars with control chars escaped (EC-024) | error |
| (v1.16, EC-025 — non-string `migration_id`) Live txn with `migration_id: 7`, `null`, `["migrate-bc-index"]`, `true` | `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`, exit 2, no reservation left behind, no writes (EC-025) | error |
| (v1.16, EC-026 — unreadable vs malformed `gate-state.json`; protected-path PreToolUse `Write`, valid `tool_use_id`, no txn record unless stated) Fixtures: (a) `"OPEN"`; (b) `"LOCKED"`; (c) ABSENT; (d) mode `000` (open fails EACCES; use a read-failing seam when running as root); (e) `gate-state.json` is a DIRECTORY (EISDIR); (f) zero-length; (g) truncated `"OPE`; (h) invalid UTF-8 `0xFF 0xFE`; (i) unquoted `OPEN`; (j) `"open"`; (k) `"BOGUS"`; (l) `{"state":"OPEN"}`; (m) `null`; (n) `7`; (o) mode-`000` file whose bytes would also be malformed | (a) admitted + reservation; (b) plain `E-MAINTENANCE-001`; (c) admitted as `OPEN`; (d), (e), (o): exit 2 `E-MAINTENANCE-002: writer-admission check failed (io)` (content never examined, so (o) is `io`); (f)–(n): exit 2 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`; every failing fixture: `HookResult::Error`, no reservation left behind, gate-state/txn byte-identical; message carries only the cause token; the matrix repeats for a txn record (unreadable ⇒ `io`; zero-length/truncated/invalid UTF-8/array top level/unknown `state`/non-string `migration_id`/two live txns ⇒ `state_integrity`); gate-state failure precedes txn failure (first-failure-wins); terminal record: unreadable ⇒ `E-MAINTENANCE-002 (io)`, unparseable / wrong-schema ⇒ `E-MAINTENANCE-001` WITH the ` (completion-record mismatch — operator investigation required)` suffix (v1.17), not finalized, reason logged (EC-026) | error |
| (v1.16, EC-009(d) — production TTL floor; reworded v1.18 for the crate-private seam) In-crate test calling `run_bc_index_migration_with_ttl(project_root, ttl)` (the production `run_bc_index_migration` delegates to it passing `DEFAULT_MAX_RESERVATION_TTL`) with `ttl` = 120 s, 1,799 s, 1,800 s, 3,600 s; and the `drain_bc_index_writers` seam with TTL = 1 s | 120 s and 1,799 s: exit 2 `RESERVATION_TTL_BELOW_FLOOR` (`configured_secs`, `floor_secs=1800`) from `validate_production_reservation_ttl`, the FIRST statement of the seam, with a byte-identical `migration-state/` snapshot and NO `exclusive.lock` created; 1,800 s/3,600 s proceed; `drain_bc_index_writers` unaffected (EC-009) | error |
| (v1.18, EC-009(d) — compile-time floor on the default) Source/compile check: `const _: () = assert!(DEFAULT_MAX_RESERVATION_TTL >= MIN_PRODUCTION_RESERVATION_TTL)` is present; `run_bc_index_migration_with_ttl` is `pub(crate)` and no environment variable or argv token reaches it | A build with the default below the floor does not compile; the seam is not callable from outside the crate and no operator-controlled input can lower the production TTL (EC-009) | edge-case |
| (v1.18, EC-026 / F-012 — carrier and Display) Unit test constructing `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` for each of the five kinds and calling `format!("{e}")`, `process_exit_code(&e)` and `admission_failure_cause(&e).token()`; fixtures exercising the five admission-side raise sites (gate/txn record parse, txn not-an-object in `abort_null_generation_txn`, non-string `migration_id`, more than one live txn, reservation serialize) | Display = `migration admission: state integrity failure (<kind-token>): <detail>`; NO Display contains `BINARY_INTEGRITY_FAILURE`; `process_exit_code` = 2; `.token()` = `state_integrity`; no admission-side raise site produces `BinaryIntegrityFailure` (EC-026) | error |
| (v1.18, Invariant 5 — exhaustive cause match) Source assertion over `admission_failure_cause`: no `_ =>` arm; every coordinator-only variant listed explicitly in one arm mapping to `StateIntegrity` | `_ =>` is absent from the function body; an unclassified new variant fails to compile (Invariant 5) | edge-case |
| (v1.18, EC-027 — cwd ≠ `CLAUDE_PROJECT_DIR`, real binary) Project tempdir `<P>` with `.factory/`; a different cwd `<C>` with `CLAUDE_PROJECT_DIR=<P>`; dispatcher PreToolUse `Edit` `tool_use_id=T1` under `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md` creates a reservation; then `migrate-bc-index` is launched from `<C>` under the same env | The reservation is in `<P>/.factory/migration-state/reservations/T1.reservation`; the coordinator's first drain lists it; `migration-state/`, `exclusive.lock`, gate and txn are created only under `<P>/.factory`; NOTHING under `<C>`; `grep -n 'join(".factory' crates/factory-dispatcher/src/shard_manager.rs` returns only the resolver (EC-027) | error |
| (v1.18, EC-028 — `resolve_session_project_root` unit table) The six inputs of EC-028 | Canonicalized path; `process_cwd` (empty ≡ absent) ×2; as-given path (never the cwd); resolved symlink; `process_cwd` (no ancestor walk) (EC-028) | edge-case |
| (v1.18, EC-029 — no `.factory`) Project tempdir with NO `.factory` (and a variant with `.factory` a regular file); invoke `migrate-bc-index` under `CLAUDE_PROJECT_DIR=<P>` | Exit 2 `FACTORY_ROOT_NOT_FOUND`; stderr is exactly `migrate-bc-index: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <P> (resolved from CLAUDE_PROJECT_DIR)` — and, with `CLAUDE_PROJECT_DIR` unset/empty and the cwd `<P>` lacking `.factory`, `… (resolved from process cwd)`; the tree is byte-identical before/after (EC-029) | error |
| (v1.18, EC-030 — symlinked `.factory`) `<P>/.factory` → `R/real-factory`; admission reserves `T1`, then the coordinator runs under `CLAUDE_PROJECT_DIR=<P>` | `T1.reservation` is in `R/real-factory/migration-state/reservations/` and the coordinator drains that same directory; no separate namespace under `<P>/.factory` (EC-030) | edge-case |
| (v1.18, EC-031 — `migration.admission_blocked`, real dispatcher) One fixture per branch of EC-031 (gate-only DRAINING; live txn STAGING gen-1, terminal record absent; live coordinator holding `exclusive.lock`; foreign txn `migration_id = "future-migration\u001b…"` (1,000 chars); STAGING + terminal record present); PreToolUse `Write` under `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md`, `tool_use_id=toolu_ABC123`; then read `dispatcher-internal-YYYY-MM-DD.jsonl` | Exactly ONE `migration.admission_blocked` line per fixture (`scope="BC-INDEX"`, `family="bc_index"`, `branch` = `gate_only` / `live_txn` / `live_coordinator` / `foreign_migration` / `completion_record_mismatch`, `gate_state`, `migration_id`/`txn_id` null for gate-only, foreign id truncated to 64 chars with the control character escaped, `check` non-null only for the mismatch fixture, `reconciliation`); the stderr message is unchanged; `toolu_ABC123` and record content appear in NO field (EC-031) | error |
| (v1.18, EC-032 — `migration.admission_failed`, real dispatcher) Fixtures: `tool_use_id=12345`; `gate-state.json` a directory; `gate-state.json` = `"BOGUS"`; two live txn records; read the JSONL | Exactly ONE `migration.admission_failed` line each: `cause` = `invalid_tool_use_id` / `io` / `state_integrity` / `state_integrity`; `kind` = `null` / `null` / `gate_record_malformed` / `multiple_live_txns`; `detail` = byte length / path + `io::Error` kind / path + parse message / path set — never the raw id nor record content; stderr is the unchanged `E-MAINTENANCE-002: writer-admission check failed (<cause>)` (EC-032) | error |
| (v1.18, EC-033 — `migration.admission_advisory`; reworded in the follow-up) Fixtures: a non-ENOENT release error; stuck gate `LOCKED` with no live txn (Branch A); STAGING `generation_id=null` txn (Branch B); COMMITTING txn with a verifying terminal record while the finalize effect is undelivered; read the JSONL | Exactly ONE `migration.admission_advisory` line per anomaly with `reason` = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired`; the finalize-unwired fixture ALSO has its one `migration.admission_blocked` with `branch="completion_record_mismatch"`, `reconciliation="completion_record_mismatch"`, `check="finalize_unwired"`; no `reason` outside the five-token domain and no optional field outside {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}; no raw `tool_use_id` (EC-033) | edge-case |
| (v1.18 follow-up, EC-033 — timestamp tokens are stderr only) Drain GC over a reservation with unparseable `created_at` (and each other EC-020 timestamp case) | One coordinator stderr advisory line with the token; NO `migration.admission_*` line in `dispatcher-internal-*.jsonl` (EC-033) | edge-case |
| (v1.18 follow-up, EC-031 — `reconciliation` token and `branch` derivation, one vector per token) Unit table over the pure diagnostic builder with inputs (`reconciliation` token, live txn remains?): (`live_coordinator`, yes); (`nothing_to_reconcile`, yes) and (`nothing_to_reconcile`, no); (`gate_reopened`, no) and (`gate_reopened`, yes); (`null_generation_txn_aborted`, no) and (`null_generation_txn_aborted`, yes); (`foreign_migration_refused`, yes); (`completion_record_mismatch`, yes); plus real-dispatcher fixtures where reachable (live coordinator; live txn with own record absent; DRAINING gate no txn; foreign `migration_id`; STAGING + terminal record) | `branch` = `live_coordinator` / `live_txn` / `gate_only` / `gate_only` / `live_txn` / `gate_only` / `live_txn` / `foreign_migration` / `completion_record_mismatch` respectively (`live_coordinator` ⇔ `live_coordinator`; `foreign_migration` ⇔ `foreign_migration_refused`; `completion_record_mismatch` ⇔ `completion_record_mismatch`; else `gate_only` if no live txn remains, else `live_txn`); every real `migration.admission_blocked` line carries exactly one of the six tokens (`live_coordinator`, `nothing_to_reconcile`, `gate_reopened`, `null_generation_txn_aborted`, `foreign_migration_refused`, `completion_record_mismatch`); `none` and the decision-table names never appear; `check` non-null only for `completion_record_mismatch` (EC-031) | error |
| (v1.18 follow-up 2, EC-031 — closed `check` domain, one vector per token; same table as BC-1.18.013 EC-037 vector, `migrate-bc-index` txn with `completed.json`, N = B2's N) (1) STAGING + `completed.json`; (2) COMMITTING + unparseable / empty / non-UTF-8 `completed.json`; (3) valid JSON wrong shape; (4) `txn_id` ≠ `activation_id`; (5) `generation_id` ≠; (6) `canonical_paths_count` ≠ N; (7) a canonical file hash ≠ `expected_post_hash` (or removed); (8) S-25.08 build: COMMITTING + record present; (9) S-25.08 build: core decides `FinalizeThenOpenGate`; (10) record read CALL fails | Rows (1)–(9): ONE `migration.admission_blocked`, `branch=completion_record_mismatch`, `check` = `staging_with_terminal_record` / `terminal_record_unparseable` / `terminal_record_schema_mismatch` / `txn_id_mismatch` / `generation_id_mismatch` / `canonical_paths_count_mismatch` / `canonical_hash_mismatch` / `terminal_record_unverified` / `finalize_unwired` (+ `branch_c_finalize_unwired` advisory for (9)); first failing check in the fixed order wins; row (10) is `E-MAINTENANCE-002 (io)`, no `_blocked`; rows (8)/(9) retired by S-25.06; every non-null `check` ∈ the nine-token set (EC-031) | error |
| (v1.18 follow-up 2, EC-029 — `FactoryRootNotFound` optionality, rule (a)) Real `migrate-bc-index` binary, no `.factory`, with and without `CLAUDE_PROJECT_DIR`; library `run_bc_index_migration(<P>)`; `run_bc_index_migration_for_session(&SessionProjectRoot{..})` for both sources | CLI stderr ALWAYS ends ` (resolved from CLAUDE_PROJECT_DIR)` / ` (resolved from process cwd)`; path-only library entry: `root_source: None`, `to_string()` == `FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <P>` (no suffix, no trailing space); `_for_session`: `Some(ClaudeProjectDir)` / `Some(ProcessCwd)` with the suffix; field is `root_source` (BC-1.18.013 EC-035 vector) (EC-029) | error |
| (v1.18, EC-034 — verdict surface label, real dispatcher stderr) A protected-path `Write` blocked by a live txn; and a write that `shard_cap_precheck` blocks | First: stderr summary line has `blocking_plugins=migration-admission` and `block_reason` = the unchanged `E-MAINTENANCE-001` text, exit 2; second: `blocking_plugins=shard-cap-gate` (EC-034) | error |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-132 | Content-preservation invariant — BC-X.YY.NNN table rows extracted from all staged shard files and sorted in canonical BC-ID order produce a SHA-256 matching `source_body_row_sha256` from the txn record (the SHA-256 of the per-BC-row content from the original pre-split BC-INDEX.md body in canonical BC-ID sort order, excluding §Summary, §Subsystem Shard Manifest, cross-cutting invariants, and non-row separator lines); `source_sha256` (whole-file fingerprint) is used ONLY by step 5 fingerprint recheck, not by PC1 | proptest / golden-file round-trip against the live (or a synthetic fixture) `BC-INDEX.md` body |
| VP-133 | Independent-census integrity invariant — every `BC-X.YY.NNN` ID in the pre-split census appears in EXACTLY ONE post-split shard (or sub-shard) file; the union of all shard row counts equals the pre-split census count; `BC-INDEX.md`'s post-split body contains zero per-BC rows | integration test (full-corpus census comparison against synthetic fixtures with known BC-ID sets, including a duplicated-row negative-control fixture) |
| VP-133 | Atomicity-under-interruption invariant — a simulated crash at any staging step leaves `BC-INDEX.md`'s body either fully original or fully split, never a partial/corrupt intermediate state | fault-injection / integration test (simulated crash at each of N staging steps; assert post-recovery state is one of the two valid states) |
| VP-133 | Idempotency invariant — running the migration twice against an already-split `BC-INDEX.md`, or resuming from a verified-complete staged state, does not re-split, re-duplicate, or corrupt any shard; `completed.json` beside a live txn is the verified own-migration finalize (Postcondition 9), not an unverified no-op | integration test (double-invocation + resume-from-staged-checkpoint fixtures) |
| VP-133 | SS-05/SS-06 second-level sub-split coverage invariant — the same content-preservation/census/atomicity/rollback obligations hold at the sub-shard level for SS-05 and SS-06 specifically, verified against an independent `BC-5.*`/`BC-6.*`-scoped count | integration test (sub-shard-scoped census comparison for SS-05/SS-06 fixtures) |
| VP-133 | Production-path reconciliation wiring — a real dispatcher PreToolUse with the gate LOCKED/DRAINING reconciles Branch A/B and admits; a live coordinator ⇒ no action + `E-MAINTENANCE-001` block (Precondition 6(d), EC-010; defect B2-1) | integration test (real spawned dispatcher, black-box through the production PreToolUse entry) |
| VP-133 | Reserve-then-verify / release-on-block / TTL — never both-miss at every W1/W2/C1/C2 ordering; no reservation survives a blocked event; production TTL 3,600 s / floor 1,800 s by `created_at` (Precondition 6(c), EC-007/EC-008/EC-009; defects B2-3, B2-4) | integration test (scripted admitter/coordinator orderings; blocked-event reservation-absence; TTL seam + production-constant assertion) |
| VP-133 | Verified finalize, mismatch fail-closed, migration discriminator, per-migration namespace — `completed.json` + COMMITTING finalizes only after verification; any mismatch ⇒ `COMPLETION_RECORD_MISMATCH_ABORT` with zero mutation; `migration_id` + cross-migration refusal; per-migration terminal-record namespace (Postcondition 9, Precondition 6(e), EC-011..EC-014; defect B2-2) | integration test (finalize/mismatch/foreign-txn/legacy-record fixtures; migration-state byte-snapshot comparison) |
| VP-147 | B2 crash-recovery / admission-gate decision core (Kani h1..h6, the `obl1_kani_proofs` suite): recovery totality, recovery safety (old-or-new, never torn), txn state-machine inductive invariant, INV-GATE-TXN admission quiescence, pointer-swap crash atomicity, recovery idempotence | kani-proof (CI job `kani`, `--harness proof_obl1`, EXPECTED_PROOFS=7; 7/7 PROVED pre-v1.18; v1.18 extension — reserve-then-verify, TTL, Branches A/B/C, verified finalize, `migration_id` — owed). Anchors: Precondition 5, 6(b), 6(c), 6(d), 6(e); Postconditions 3, 4, 5, 9; Invariant 3; EC-002, EC-003, EC-006, EC-007..EC-014 |
| VP-134 | No-new-Cohort-B-dependency invariant — this BC's migration completion is never referenced as a precondition in `hooks-registry.toml`'s `failure_policy` deployment sequencing for `regression-gate`/`convergence-tracker` | static-check (config/PR-template audit confirming BC-7.08.001's gating conditions cite only BC-1.18.005/006/008, never this BC) |
| VP-142 | Chunk-boundary determinism and correctness (ADR-051 §Decision 18, hosted here as Postcondition 6's concrete algorithm) — for a fixed row set, preamble, and `shard_cap_bytes`, `chunk_subsystem_rows_into_sub_shards` always produces identical chunk boundaries on any invocation, any machine, any retry; every row appears in exactly one chunk; no chunk's preamble+rows exceeds `shard_cap_bytes` except the documented lone-row-overflow edge case; consecutive chunks' BC-ID ranges are non-overlapping and jointly cover the full sorted sequence. Cross-referenced from BC-1.18.010 Postcondition 4, since the property holds identically for this one-time migration and the future steady-state rebuild path | proptest (property: chunking twice over the same input yields identical output; every row in exactly one chunk; no chunk exceeds cap except the lone-row case; consecutive ranges non-overlapping and gap-free) |

VP IDs allocated by formal-verifier (S-25.02 F2 verification-property fix-burst; VP-INDEX v3.03):
**VP-132** (proptest; content-preservation byte-for-byte), **VP-133** (integration; independent-census
integrity + crash-atomicity + fail-loud rollback CENSUS_MISMATCH_ABORT (process exit code) + idempotency + SS-05/SS-06 sub-split
census — the four same-method safety obligations consolidated per the single-method-per-VP convention,
mirroring VP-124), and **VP-134** (static-check; no-new-Cohort-B-dependency). This is the B2 analogue
of VP-123/VP-124 (BC-1.18.008's content-preservation + atomicity/idempotency pair) but keyed to
BC-INDEX's ID-census model instead of decision-log's byte-count model. Traceability-reference
completion only (VP-side of the trace); no BC body/postcondition/version change.

**VP-142** (proptest; chunk-boundary determinism and correctness) allocated by formal-verifier per
ADR-051 §Decision 18's authoring instruction (architect design-proposal, human-approved 2026-09-22),
hosted on THIS BC's Postcondition 6 since Postcondition 6 is where the `chunk_subsystem_rows_into_sub_shards`
function contract is named as a migration-behavior obligation; cross-referenced (not re-hosted) from
BC-1.18.010 Postcondition 4.

**VP-133 v1.1 facets and VP-147 (v1.12).** VP-133 (integration) additionally carries three v1.11-clause
facets: production-path reconciliation wiring (Precondition 6(d), EC-010; B2-1), reserve-then-verify /
release-on-block / TTL (Precondition 6(c), EC-007/EC-008/EC-009; B2-3, B2-4), and verified finalize /
mismatch fail-closed / migration discriminator / per-migration namespace (Postcondition 9,
Precondition 6(e), EC-011..EC-014; B2-2); its Idempotency row is re-based so that `completed.json`
beside a live txn is the verified own-migration finalize, not an unverified no-op. **VP-147**
(kani-proof; B2 crash-recovery / admission-gate decision core, the `obl1_kani_proofs` suite) was
allocated by the architect under ADR-052 v1.18 (VP-INDEX v3.26); its v1.18 extension is owed.

## Related BCs

- BC-1.18.008 — this BC's structure is modeled directly on BC-1.18.008's governed one-time-migration pattern (content-preservation, census, atomicity, rollback, idempotency), substituting a content partition (by subsystem) for a time partition (by seal sequence) (related to)
- BC-1.18.010 — this BC governs the ONE-TIME transition to BC-1.18.010's end-state addressing scheme; BC-1.18.010 specifies the end-state, this BC specifies the transition (depends on)
- BC-1.18.006 — this BC reuses BC-1.18.006's atomic-write primitives (staging + verify + atomic replace) (depends on)
- BC-1.18.005 — the shard-cap formula that determines whether SS-05/SS-06 (and, empirically, any other subsystem) require second-level sub-sharding (related to)
- BC-7.08.001 — explicitly has NO dependency on this BC (Postcondition 7); cited here only to document the absence of a relationship an implementer might otherwise assume by analogy to BC-1.18.008 (related to)
- BC-3.08.001 — catalogues the three admission diagnostic events (Events 11–13: `migration.admission_blocked` / `migration.admission_failed` / `migration.admission_advisory`) this BC's Postcondition 10 mandates; wire-format/field-shape authority only (v1.18) (composes with)
- BC-1.18.013 — the mechanism-A sibling governed-migration BC sharing the admission core, the single anchoring rule, the diagnostics channel and the entry-point names (v1.18) (composes with)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — one-time migration entry point for the B2 body split, invoking BC-1.18.006's `write_atomic` primitive (`last_amended_migrate::atomic_write::write_atomic`) for per-file writes; multi-file crash-atomicity provided by ADR-052 §Decision 7
- `.factory/specs/behavioral-contracts/BC-INDEX.md` §Summary / `total_bcs` frontmatter field — the independent count-oracle this BC's census check (Postcondition 2) cross-checks against
- `.factory/specs/architecture/ARCH-INDEX.md` §Subsystem Registry — the `BC-S Prefix`→`SS-NN` mapping this BC's per-subsystem partition boundaries follow (same mapping BC-1.18.010 Postcondition 2 reuses)
- ADR-052 §Decision 4 — armed-activation manifest governing pre-mutation authorization (two-phase validation: pre-lock and under-exclusion)
- ADR-052 §Decision 5a — native admission gate in executor.rs: OPEN/DRAINING gate with writer reservations (PreToolUse-acquire/PostToolUse-release); txn record state check (STAGING/COMMITTING) blocks ordinary writers regardless of PID liveness
- ADR-052 §Decision 7a — advisory flock on stable pre-created never-unlinked inode (`.factory/migration-state/exclusive.lock`); durable txn record separate from lock file with `fencing_generation` for recovery-owner claim
- ADR-052 §Decision 7b — framed checksummed intent log with per-target expected post-hash + pre-state; WAL boundary after intent fsync; matching-destination-hash recovery decision table
- ADR-052 §Decision 7c — single atomic CURRENT.json pointer swap (the commit point); completed.json as permanent terminal record; reader protocol (completed.json → CURRENT.json → legacy)
- ADR-052 §Decision 8 — POLICY 22 exception declaration with accurate skipped-control inventory and enumerated allowed write targets
- ADR-052 §Decision 5a v1.20 sub-sections — "Evaluation position" (6(d) lead, O1–O4), "Admission scope anchoring" and "Target path resolution" (6(b) events/tools/scope), PostToolUse/PostToolUseFailure release, "Reservation timestamp rules", "`tool_use_id` presence and validity" (6(c)); ADR-052 §Decision 7e "Definition of foreign in the shared admission core" (6(d) decision table, 6(e), Postcondition 9); ADR-052 §Error Code Semantics v1.20 (`E-MAINTENANCE-002`, `RESERVATION_TTL_BELOW_FLOOR`)
- ADR-052 §Decision 5a v1.21 sub-sections — "Entry-point naming" (6(d) lead), "Single anchoring rule" (6(b)(i), Precondition 7, Invariant 6), "Admission diagnostics channel" (Postcondition 10), "Admission verdict surface label" (6(b), EC-034), "Admission state-integrity variant" (6(c), EC-026, Invariant 5), "EC-031 discharge — production TTL floor" (EC-009(d)); ADR-052 §Error Code Semantics v1.21 additions (`MIGRATION_STATE_INTEGRITY_FAILURE`, `FACTORY_ROOT_NOT_FOUND`)
- `crates/factory-dispatcher/src/shard_manager.rs` — `resolve_session_project_root` (pure single anchoring function), `resolve_factory_root`, `FactoryRoot::migration_state_dir()`, `run_bc_index_migration` / crate-private `run_bc_index_migration_with_ttl`; `crates/factory-dispatcher/src/executor.rs` — `migration_writer_admission`, `migration_writer_admission_precheck`, `migration_reservation_release`, `NativeGate`; `crates/factory-dispatcher/src/internal_log.rs` — `MIGRATION_ADMISSION_BLOCKED` / `MIGRATION_ADMISSION_FAILED` / `MIGRATION_ADMISSION_ADVISORY`

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

```
$ grep -oE "last_amended_migrate::atomic_write::write_atomic" crates/factory-dispatcher/src/shard_manager.rs | head -1
last_amended_migrate::atomic_write::write_atomic
```

Confirms BC-1.18.006's shipped `write_atomic` primitive (`last_amended_migrate::atomic_write::write_atomic`)
is the per-file write mechanism this BC's Invariant 1 states is invoked (not reimplemented) in
`crates/factory-dispatcher/src/shard_manager.rs`. Note: `write_indeterminate_marker` (a CAP-041
INDETERMINATE quarantine-marker writer in `crates/factory-dispatcher/src/indeterminate_marker.rs`)
is NOT an atomic-write staging primitive and is not cited here — removed per ADR-052 §BC Impact
v1.7 F7.

```
$ grep -oE "^total_bcs: [0-9]+" .factory/specs/behavioral-contracts/BC-INDEX.md | sed -E 's/[0-9]+/<N>/'
total_bcs: <N>
```

Confirms the live `total_bcs` frontmatter field exists and is numeric; the volatile value is
redacted to `<N>` (per POLICY 5 HEAD-reproducibility mandate — mirrors BC-1.18.010 v1.2's
structural-form fix for the identical drift class). Any future reader MUST re-execute the grep at
HEAD to obtain the CURRENT count. This BC's Postcondition 2's independent-count-oracle claim
depends only on the FIELD being present and numeric, not on any specific value (the count grows
with every new BC addition before F4 execution; the migration binary reads the live value at
activation time, not from this grounding evidence).

```
$ grep -oE "^\*\*CAP-04[123] " .factory/specs/domain-spec/capabilities.md
**CAP-041 
**CAP-042 
**CAP-043 
```

Confirms CAP-043's existence, grounding this BC's capability anchor.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-132, VP-133, VP-134 — allocated by formal-verifier (S-25.02 F2 verification-property fix-burst; VP-INDEX v3.03), analogous to VP-123/VP-124 (content-preservation + record-integrity; atomicity-under-interruption + idempotency) but keyed to BC-INDEX's ID-census model instead of decision-log's byte-count model, per the F2 architecture-delta doc §4a authorship input for this BC. VP-132 (proptest; content-preservation structured-row-equivalence), VP-133 (integration; independent-census integrity + crash-atomicity + fail-loud rollback CENSUS_MISMATCH_ABORT (process exit code) + idempotency + SS-05/SS-06 second-level sub-split census — four same-method obligations consolidated per the single-method-per-VP convention), VP-134 (static-check; no-new-Cohort-B-dependency). The six candidate properties enumerated in `## Verification Properties` above map to these three VPs: candidate 1 → VP-132; candidates 2/3/4/5 → VP-133; candidate 6 → VP-134.
- VP-147 — allocated by the architect under ADR-052 v1.18 (VP-INDEX v3.26) (kani-proof; B2 crash-recovery / admission-gate decision core, the `obl1_kani_proofs` suite). Anchors: Precondition 5, 6(b), 6(c), 6(d), 6(e); Postconditions 3, 4, 5, 9; Invariant 3; EC-002, EC-003, EC-006, EC-007..EC-014. VP-133 v1.1 additionally anchors the v1.11 facets (Precondition 6(d)/EC-010; Precondition 6(c)/EC-007/EC-008/EC-009; Postcondition 9/Precondition 6(e)/EC-011..EC-014).
- VP-142 — allocated by formal-verifier per ADR-051 §Decision 18's authoring instruction (S-25.02-b2-sharding cluster-5 spec-closure chain; human-approved 2026-09-22 design proposal). Hosted on THIS BC's Postcondition 6 (proptest; chunk-boundary determinism and correctness for `chunk_subsystem_rows_into_sub_shards`), cross-referenced from BC-1.18.010 Postcondition 4 since the property holds identically for the one-time migration and the steady-state rebuild path.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | Anchoring to CAP-043: "Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding" — because this BC describes the governed one-time migration that establishes mechanism B2's split end-state (BC-1.18.010) correctly and safely for `BC-INDEX.md`, which is exactly what CAP-043 defines per `capabilities.md` §CAP-043: "This capability has two mechanisms for two artifact shapes: mechanism A shards four append-only cycle logs... mechanism B shards `BC-INDEX.md`... via two sub-mechanisms: B1... and B2 splits the file's ten already-existing `### SS-NN` per-subsystem body sections into individually-addressable shard files." CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — no existing capability other than CAP-043 covers a governed one-time migration establishing B2's split end-state; CAP-041 (INDETERMINATE detection/quarantine) and CAP-042 (the `rotate_changelog`/`last_amended` write-path fix) are both distinguishable per capabilities.md's own CAP-043 entry, and neither covers a BC-INDEX body-structure migration. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with the sibling BC-1.18.005–010 precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` one-time B2 migration logic) |
| ADR | ADR-051 §Decision 10 (governed one-time migration for the B2 BC-INDEX body split, fix-burst addition F-S2502-F2-002); ADR-051 §Decision 7 (B2 end-state design this migration produces); ADR-051 §Decision 8 (shard-manifest schema this migration publishes); ADR-052 §Decision 4 (armed-activation manifest governing pre-mutation authorization); ADR-052 §Decision 5a (native admission gate: OPEN/DRAINING gate with writer reservations; txn state check blocks ordinary writers regardless of PID liveness); ADR-052 §Decision 7a (advisory flock on stable never-unlinked inode; durable txn record separate from lock file); ADR-052 §Decision 7b (framed checksummed intent log + WAL boundary + matching-destination-hash recovery decision table); ADR-052 §Decision 7c (single atomic CURRENT.json pointer swap + completed.json permanent terminal record + generation-first/canonical-fallback reader protocol); ADR-052 §Decision 8 (POLICY 22 exception declaration with enumerated allowed write targets) |
| Stories | S-25.02, S-25.06 (S-25.06 delivers B2-1..B2-4 conformance behavior under this BC; human-approved 2026-10-06) |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution fix-burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.18 | 2026-10-07 | product-owner | Application of ADR-052 v1.21 §Downstream items 23–29 (S-25.08 local adversary pass 2: D-1/D-2/F-006/F-012/F-013/F-014); shared-core sibling sweep with BC-1.18.013 v1.10. (23, D-2) Precondition 6(b)(i): the one-sided "migration-state is ALWAYS `<factory_root>/migration-state`" anchor sentence REPLACED by the single anchoring rule (`resolve_session_project_root` → `resolve_factory_root` → `FactoryRoot::migration_state_dir()`; ONE function used by admission, the release leg AND both coordinator binaries); NEW Precondition 7 (`migrate-bc-index` coordinator anchoring; no `.factory` ⇒ exit 2 `FACTORY_ROOT_NOT_FOUND`, `.factory` never created); NEW EC-027..EC-030 + four test vectors; NEW Invariant 6. (24, D-1) the seven live `tracing::warn!` diagnostic mandates (Preconditions 6(b)/(c)/(d), Postconditions 6 and 9(d); the eighth `tracing::` mention is the v1.15 changelog row) REPLACED by the `migration.admission_blocked` / `_failed` / `_advisory` InternalLog events for the admission/release/reconciliation legs and by stderr lines for the coordinator's own anomalies (the lone-oversized-row note); NEW Postcondition 10 (exactly one event per verdict or anomaly in `dispatcher-internal-YYYY-MM-DD.jsonl`; no raw `tool_use_id` or record content; sanitized/truncated fields per EC-024); NEW EC-031..EC-033 + vectors. (25) BC-3.08.001 v1.35 catalogues Events 11–13 (Related BCs). (26, F-006) EC-009(d): "the production entry point is configured below 1,800 s" replaced by the crate-private `run_bc_index_migration_with_ttl(project_root, max_reservation_ttl)` seam (first statement `validate_production_reservation_ttl`) + compile-time const-assertion vector; the seam is NOT public and NOT env/argv-injectable. (27, F-012) the `state_integrity` carrier is named — `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` (five kinds), NOT `BinaryIntegrityFailure`; Display-lacks-`BINARY_INTEGRITY_FAILURE` vector; NEW Invariant 5 (`admission_failure_cause` is an exhaustive match, no wildcard); EC-026 extended. (28, F-013) Precondition 6(b): an admission verdict is reported as `blocking_plugins=migration-admission` (`NativeGate::MigrationAdmission`; the shard-cap leg keeps `shard-cap-gate`); NEW EC-034 + vector. (29, F-014) Precondition 6(d) lead: single shared entry points `migration_writer_admission` / `migration_writer_admission_precheck` / `migration_reservation_release`, no per-migration delegates; EC-015 vector re-pointed. **Folded-in follow-up (ADR-052 v1.21 §Downstream item 33 (a)–(g), same version, uncommitted):** (g) undelivered finalize (`FinalizeThenOpenGate` with the S-25.08 seam) ⇒ the one `_blocked` carries `reconciliation=completion_record_mismatch`, `branch=completion_record_mismatch`, `check=finalize_unwired` plus the `_advisory` `branch_c_finalize_unwired` (replaces the earlier `branch=live_txn`); after S-25.06 delivers the finalize: no `_blocked`, `_advisory` `branch_c_finalized`, `finalize_unwired` retired (Postcondition 10, EC-033 vector); (a) Branch C verification failure = ONE `_blocked` (`branch=completion_record_mismatch`, `check`), no `_advisory`; (b) Postcondition 10 `reconciliation` = closed six-token effectful domain (`live_coordinator` / `nothing_to_reconcile` / `gate_reopened` / `null_generation_txn_aborted` / `foreign_migration_refused` / `completion_record_mismatch`; no `none`) with the total `branch` derivation and a vector per token (EC-031); (c) advisory `reason` domain = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired` / `branch_c_finalized`; the five timestamp tokens removed from the event and re-homed as coordinator-drain stderr tokens (EC-020, EC-033); `branch_c_finalize_unwired` in addition to the one `_blocked`, retired by S-25.06; `branch_c_finalized` S-25.06-only; (d) optional advisory context fields closed set {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}; (e) Precondition 7/EC-029: `FactoryRootNotFound { project_root, root_source }` and the normative stderr line with `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`}. **Folded-in follow-up 2 (S-25.08 implementation review @ f8726c34, same version, uncommitted):** field renamed `source` → `root_source` (`thiserror` reserves `source`); `root_source: Option<ProjectRootSource>` with rule (a) — CLI always prints the suffix, path-only library entries permitted with `None` and suffix-less `Display` (Precondition 7, EC-029 + vector); Postcondition 10 Branch C `check` is a CLOSED nine-token domain (normative: BC-1.18.013 Postcondition 10(a); EC-031 vector). Status UNCHANGED (active). |
| 1.17 | 2026-10-07 | product-owner | Application of ADR-052 v1.20 §Downstream items 21 and 22 plus the terminal-record suffix ruling; sibling sweep with BC-1.18.013 v1.9. (21) Precondition 6(b) anchor/classification/(iv): lexical comparison against EITHER spelling of the session's own root (canonical `factory_root`; `<CLAUDE_PROJECT_DIR as given>/.factory`), no other spelling; migration-state anchored on the real root; EC-018 vector (g); EC-019 vectors 1 (canonical vs as-given identical) and 2 (look-alike/other project, no over-match); TV rows each. (22) Architect Ruling 1 re-anchored in Precondition 6(b): "structurally skipped: admission returns before `shard_cap_precheck` is reachable" (no reference to `resolve_shard_gate_precedence`). (Ruling) Terminal record read but not verifying (unparseable/wrong-schema/count/id/hash) ⇒ `E-MAINTENANCE-001` WITH the mismatch suffix, not finalized (Precondition 6(d) classification text, EC-026 + TV, new TV row); "plain" = no suffix; read-call failure stays `io`. error-taxonomy v1.38. |
| 1.16 | 2026-10-07 | product-owner | Application of ADR-052 v1.20 §Downstream items 14–19 (S-25.08 local adversary pass 1: F-001/F-002/F-003/F-004/F-006/F-008/F-009); shared-core sibling sweep with BC-1.18.013 v1.8 (normative text lives in BC-1.18.013 Precondition 6(b)/(c); this BC carries the same rules for the B2 path). (14, F-004) Precondition 6(d) "single shared admission core" position: evaluation is registry-independent — obligations O1–O4 (exactly once; BEFORE `Registry::load` / `resolve_registry_path()` / the Tier-1 degraded branch, so it runs with `CLAUDE_PLUGIN_ROOT` unset, a missing/unparseable/schema-mismatched registry, an empty matched-plugin set; before `shard_cap_precheck` and every plugin tier; unparseable stdin keeps the existing exit; release leg registry-independent) with the reason (registry-load arms are fail-open, the interlock must not be). (15, F-002/F-003) Precondition 6(b): NEW "Events, tools and admission scope" paragraph — events `PreToolUse` + `PostToolUse` + `PostToolUseFailure`, tools `Edit`/`Write`/`MultiEdit` (`Bash` stays OBL-4), sub-bullets (i) Anchor (session `factory_root`; never creates `.factory`), (ii) component-wise Classification, (iii) out-of-root `.factory` paths out of scope + accepted residual + §7c step-5 backstop, (iv) Target path resolution (`resolve_target_path`), (v) residuals. (16, F-001/F-008/F-009) Precondition 6(c): release on `PostToolUse` OR `PostToolUseFailure` for the same `tool_use_id`, no `tool_name` filter, TTL backstop when neither arrives; NEW timestamp rules (RFC 3339; pre-epoch/out-of-range ⇒ unparseable ⇒ mtime; `created_at > now + 300 s` ⇒ untrusted ⇒ mtime; in-tolerance future ⇒ age 0; both unusable ⇒ NOT stale + warn; no clamping); NEW `tool_use_id` presence/validity (absent/`null` ⇒ check-only; present-but-invalid ⇒ fail closed `E-MAINTENANCE-002` `invalid_tool_use_id`; grammar `[A-Za-z0-9_.-]{1,128}`, no leading `.`); the writer-admission-check error result now named `E-MAINTENANCE-002`; production TTL floor violation named `RESERVATION_TTL_BELOW_FLOOR`. (17, F-006) Precondition 6(d) decision table row 2 now "live txn's `migration_id ∉ K`, K = {`migrate-bc-index`, `backfill-append-logs`} (absent field ⇒ `migrate-bc-index`) ⇒ `RefuseForeignMigration`"; core input `txn_is_own_migration` renamed `txn_migration_known`; a live txn of EITHER known migration is decided against ITS OWN migration's terminal record selected by `migration_id` (the other's record never consulted) so "foreign" never applies between the two known ids on the dispatcher path; Branch B never discards a foreign txn; Precondition 6(e) and Postcondition 9(a) cross-migration refusal marked binary-only (unchanged, `LockContention`-class exit 2); non-string `migration_id` ⇒ malformed ⇒ `E-MAINTENANCE-002` `state_integrity`. (18) NEW append-only EC-016..EC-025 and 11 canonical test vector rows (10 for the new ECs + the production TTL floor row anchored to EC-009(d)) (PostToolUseFailure release; broken/absent registry and unset `CLAUDE_PLUGIN_ROOT`; out-of-root / symlinked `.factory` / no-`.factory`; path aliasing; timestamp cases; invalid `tool_use_id`; other-known-migration live txn without/with its own record; unknown and non-string `migration_id`); EC-013 annotated "(binary recovery path only)"; EC-009 extended (timestamp rules; floor violation named `RESERVATION_TTL_BELOW_FLOOR`); EC-012/EC-015 aligned. (19, S-25.08 AC-018 ambiguity — folded into v1.16, uncommitted) Precondition 6(d) failure paragraph gains the total `E-MAINTENANCE-002` `<cause>` classification (normative text in BC-1.18.013 Precondition 6(c); identical here): `invalid_tool_use_id` = payload-only; `io` = an OS-level admission call failed other than ENOENT of the file itself (content never examined after a failed call); `state_integrity` = all bytes read but content unusable (non-UTF-8, empty/truncated/unparseable, wrong JSON type/shape — `gate-state.json` is a bare JSON string —, unknown state, PRESENT non-string `migration_id`, >1 live txn); first-failure-wins; reservation files never read by admission; terminal-record read failure ⇒ `io`, unverifiable content ⇒ plain block (not `E-MAINTENANCE-002`). NEW append-only EC-026 and one canonical test vector row (unreadable vs malformed `gate-state.json`, txn and terminal-record matrix). No clause renumbered/removed. **Stories affected by BC changes:** S-25.08 (ACs/tasks for F-001/F-002/F-003/F-004/F-006/F-008/F-009), S-25.06 (inherits the "foreign" wording and the `txn_migration_known` rename only; no new ACs), S-25.02 (anchor; no change) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change by PO. **VP citations changed in:** none by PO (VP-133, VP-143, VP-147 bodies amended by the architect in the same ADR-052 v1.20 burst; no VP ID added/removed in this BC). |
| 1.15 | 2026-10-07 | product-owner | S-25.08 implementation finding — first-activation reservation race (companion of BC-1.18.013 v1.7; same decision). The pre-existing guard "`.factory/migration-state/` absent ⇒ admit with no reservation" (in `bc_index_migration_admission` and its PostToolUse release counterpart) let a protected write admitted before the directory first existed evade the coordinator's drain. **Decision (option (a)):** NEW Precondition 6(c) bullet "Unconditional reservation namespace — first-activation race closed" — admitter ensures `migration-state/reservations/` by idempotent recursive create then W1/W2; absent gate ⇒ OPEN; directory-absent bypass REMOVED; fail-closed on creation failure; cost accepted; alternatives (b)/(c) rejected with rationale. New EC-015 + 2 canonical test vector rows. No clause renumbered/removed. **TEST-WRITER CHANGE (exact):** `crates/factory-dispatcher/tests/bc_1_18_011_b2_migration_test.rs::test_BC_1_18_011_PRECOND6_admission_precheck_returns_none_when_no_migration_state_dir` must be replaced by `..._admits_and_reserves_when_no_migration_state_dir` — same fixture plus a `tool_use_id`; still asserts verdict `None` (admitted) but now ALSO asserts `.factory/migration-state/reservations/<tool_use_id>.reservation` exists; a payload without `tool_use_id` asserts no directory/reservation created; add EC-015 vectors (drain observes pre-directory writer; concurrent first admissions). The `executor.rs` `!migration_state_dir.exists()` early returns in `bc_index_migration_admission` (admit path only — the PostToolUse release may keep its absent ⇒ no-op return) must be removed, with `create_dir_all(reservations)` ahead of W1 in `admit_protected_write`/`create_writer_reservation` (already idempotent there). **ADR-052 DELTA OWED (architect; exact text):** (1) §Decision 5a step 0, append: \"The admitter first ensures `.factory/migration-state/reservations/` exists by idempotent recursive directory creation (already-exists is success; no existence pre-check, no lock). Admission, reservation and release MUST NOT be conditioned on the pre-existence of `.factory/migration-state/`; an absent `gate-state.json` is `OPEN`, an absent txn set is 'no live txn'. This makes the Dekker argument hold from the first protected write ever (a write admitted before the directory existed would otherwise be invisible to the coordinator's C2 read). Failure to create the directory or reservation fails the PreToolUse closed.\" (2) §Decision 5a perf note (v1.18): state the directory-ensure + reservation cost applies to every protected Edit/Write/MultiEdit in a factory project, not only during migrations (still ≤ low-single-digit ms, accepted). (3) §Decision 7e Why (D4) sentence \"Neither migration has ever been activated in this repository (`.factory/migration-state/` does not exist)\" → \"(no txn record, terminal record or pointer exists; the empty `migration-state/reservations/` namespace may exist from admission)\" — directory existence is no longer a migration signal. (4) §Files to Change `executor.rs` row: note removal of the directory-exists early-return on the admit path. **Stories affected by BC changes:** S-25.08 (test-writer/implementer; no `bcs:` array change by PO) — story-writer propagates. **VP citations changed in:** none textually (VP-133 reserve-then-verify and VP-147 h4 quiescence facets gain the namespace-exists-from-first-write obligation — architect owns). |
| 1.14 | 2026-10-06 | product-owner | inputs: dropped downstream BC-INDEX to stop index-bump churn; orchestrator 2026-10-06. BC-INDEX records every BC's version, so each BC-INDEX bump re-staled this BC (content-dependency loop; BC-INDEX is downstream of the BC). Frontmatter `inputs:` only; prose cross-references to BC-INDEX.md retained. No Precondition/Postcondition/Invariant/EC/VP change; status UNCHANGED. **Stories affected by BC changes:** none. **VP citations changed in:** none. |
| 1.13 | 2026-10-06 | product-owner | Resolution of four spec ambiguities surfaced by the S-25.08 Red Gate (test-writer, commit 1b28017e). (1) Precondition 6(b): `E-MAINTENANCE-001` message keyed on the WRITTEN PATH FAMILY (`BC-INDEX` / `.factory/cycles/`), not the live txn's migration; exact single-line format string; cross-reference to the four-cell table in BC-1.18.013 Precondition 6(b) (same rule in error-taxonomy v1.35). (2) Precondition 6(d): Branch B on-disk marker specified — in-place atomic rewrite to `state=ABORTED` + top-level `abort_reason: "null_generation"`, `generation_id`/`source_sha256` stay null, txn file retained, informational-only reader rules. (3) Precondition 6(d): terminal-record reconciliation core decision table added; own migration + live txn + lock acquired + terminal record ABSENT ⇒ `NoOp` for STAGING and COMMITTING alike, then fall-through to Branch B (STAGING null-gen only) else ordinary admission (plain block). (4) Postcondition 9(d): PreToolUse block for B2 STAGING + terminal record (and any verification failure) carries the ` (completion-record mismatch — operator investigation required)` suffix after the keyed message; foreign-migration refusal and live coordinator carry none. EC-012 expected text and the D3-mismatch vector aligned; four Canonical Test Vector rows added. No clause renumbered/removed. **Stories affected by BC changes:** S-25.08 (Red Gate alignment; no `bcs:` array change by PO) — story-writer/test-writer propagate. **VP citations changed in:** none. |
| 1.12 | 2026-10-06 | product-owner | ADR-052 v1.18 follow-up deltas 9, 10, 11, 13 (architect review of v1.11; exact-text corrections). (9) EC-012 + its D3-mismatch vector: removed the live-`backfill-append-logs`-txn clause and the "(also …)" fixtures (foreign txn is covered by EC-013 only); Postcondition 9(a): foreign-txn refusal is binary exit 2 `LockContention`-class, NOT `COMPLETION_RECORD_MISMATCH_ABORT`, PreToolUse plain `E-MAINTENANCE-001` with no mismatch reason; foreign-txn D4 vector expected column corrected and EC-012 dropped from its anchors. (10) Precondition 6(d) Branch C and Postcondition 9 lead-in: terminal record is the live txn's own migration's, selected by `migration_id` (`completed.json` / `completed-backfill-append-logs.json`); shared core dispatches on `migration_id` (ADR-052 §7e); B2-specific clauses stay for `migrate-bc-index` txns. (11) STAGING wording in Precondition 6(d) and Postcondition 9(d): STAGING + terminal record is always fail-closed, no verification attempted (no behavior change). (13) Verification Properties: VP-133 rows added (production-path reconciliation wiring; reserve-then-verify/release-on-block/TTL; verified finalize/mismatch/discriminator/namespace), VP-133 Idempotency row re-based, VP-147 row added (kani-proof), VP Anchors paragraph updated. Traceability Stories row: S-25.06 added (human-approved 2026-10-06). No clause renumbered/removed. **Stories affected by BC changes:** S-25.02, S-25.06 — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs (Stories row only; no `bcs:` array change by PO). **VP citations changed in:** VP-133 (facets/anchors), VP-147 (new) — architect/story-writer propagate to VP-INDEX, S-25.02/S-25.06. |
| 1.11 | 2026-10-06 | product-owner | ADR-052 v1.18 formal-finding exception — shared-core sibling sweep (human-approved 2026-10-06; keeps B2 conformance fixes B2-1..B2-4 in S-25.06 scope; ADR-052 is the spec, merged B2 code deviates). **Precondition 6(c)** rewritten: reserve-then-verify admission order (reservation FIRST, then read gate/txn; own reservation removed on failed verification; no admitter lock; Dekker argument) + release-on-block (dispatcher removes its reservation when a later stage blocks/errors) + TTL (`MAX_RESERVATION_TTL` default 3,600 s, production floor 1,800 s, test seam injectable, `created_at` staleness with mtime fallback, no PID liveness; merged 120 s corrected). **NEW Precondition 6(d):** PreToolUse stale-gate reconciliation Branches A/B/C is part of the contract and MUST be wired on the production admission path (merged `reconcile_stale_admission_gate` is uncalled dead code). **NEW Precondition 6(e):** `migration_id` txn field (absent ⇒ `migrate-bc-index`), cross-migration recovery refusal, B2 owns `completed.json`/`CURRENT.json` and `backfill-append-logs` never writes them (ADR-052 §7e), shared lock/gate/reservations/txn dir, `gate-state.json` physical name. **NEW Postcondition 9:** verified `completed.json`+COMMITTING finalize (txn→COMPLETED then gate→OPEN) replaces the unverified short-circuit; `COMPLETION_RECORD_MISMATCH_ABORT` (binary exit 2) / `E-MAINTENANCE-001` block with reason logged (PreToolUse); must never finalize another migration's txn. **New edge cases (append-only):** EC-007..EC-014 + 10 canonical test vector rows. No existing clause renumbered/removed. **Stories affected by BC changes:** S-25.02 (anchor; B2 amendment note — merged cluster code deviates), S-25.06 (B2-1..B2-4 conformance fixes in scope) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change. **VP citations changed in:** none textually (VP-133 atomicity/idempotency facets and VP-124-class gate obligations are affected by reserve-then-verify — architect owns under ADR-052 v1.18). |
| 1.10 | 2026-09-23 | product-owner | Documentary cross-reference closing adversary finding F-C5-P1-004. Precondition 6(b) amended: added a "Delivery cross-reference (per D-1236 Ruling 3)" clause clarifying that the native admission gate's Edit/Write/MultiEdit legs ship with cluster-5 F4 TDD in `executor.rs`, while the Bash leg (full-command classification) is delivered separately by the ADR-052 §Decision 5c full-command classifier as part of [D-1232-OBL-4] (devops-engineer's dispatcher-guard amendments), deployed at the cluster-5 F4 activation boundary — not by cluster-5 TDD. This documents WHERE/WHEN each leg of the "ALL mutation tool calls (Edit/Write/MultiEdit/Bash)" admission-gate scope is actually enforced, resolving the apparent scope gap a fresh-context adversary flags when it cannot see D-1236. No change to any Postcondition, Invariant, Edge Case, Test Vector, or VP; the "ALL ... Bash" intent is preserved, only its phased delivery is now documented in-BC. input-hash recompute owed to state-manager. |
| 1.9 | 2026-09-22 | product-owner | ADR-051 §Decision 18 addendum encoding (spec-closure chain step 2 of 2: architect → product-owner; human-approved 2026-09-22 design proposal). Postcondition 6 amended: named the concrete `chunk_subsystem_rows_into_sub_shards(sorted_rows, preamble, shard_cap_bytes) -> Vec<SubShardChunk>` function contract (ADR-051 §Decision 18 item 4) the migration invokes for SS-05/SS-06's second-level sub-split, and added explicit migration-behavior edge-case rulings per §Decision 18's edge-case table: lone-oversized-row (emitted as an over-cap lone sub-shard with a non-blocking `tracing::warn!`, never fail-loud, bounded by BC-1.18.005's `MAX_SINGLE_RECORD_BYTES` margin), exactly-at-cap (`<=` inclusive, matching BC-1.18.005 Postcondition 3's convention), sub-shard letter exhaustion (base-26 `.aa`/`.ab`... naming beyond 26 chunks, never fail-loud), and each-row-in-exactly-one-sub-shard (already covered by this BC's own Postcondition 2 independent census — zero new verification code). Added **VP-142** (proptest; chunk-boundary determinism and correctness) to this BC's own Verification Properties table and VP Anchors as the hosting BC, cross-referenced from BC-1.18.010 Postcondition 4. No change to Postconditions 1-5, 7-8 or to this BC's existing content-preservation/census/atomicity/rollback machinery — this amendment supplies the previously-unspecified chunk-boundary mechanism Postcondition 6 assumed but did not name. input-hash recompute owed to state-manager. |
| 1.8 | 2026-09-13 | product-owner | ADR-052 v1.11 pass-8 reader-protocol mirror (MED-1). Invariant 3 COMMITTING-window accessibility description: replaced "generation-first/canonical-fallback protocol (ADR-052 §Decision 7c C-1 fix)" with the OPEN-based with ENOENT fallback form per the canonical reader protocol (ADR-052 §Decision 7c): for each required file, open `gen-<uuid>/<file>`; on ENOENT, open the canonical path. Replaced "a file absent from `gen-<uuid>/` has already been renamed to canonical" with "ENOENT on `gen-<uuid>/<file>` means the file has already been renamed to canonical — open the canonical path instead." Replaced "`rename(2)` atomicity ensures ENOENT is not possible for any new-generation file" with "`rename(2)` atomicity makes this protocol race-free and ENOENT-safe: a required file is never absent from both `gen-<uuid>/` and canonical during the COMMITTING window." Preserves the "never partially applied / There is no turning back" guarantee; grounds the COMMITTING-window accessibility claim in open-with-fallback, not exists-then-read. input-hash recompute owed to state-manager. |
| 1.7 | 2026-09-13 | product-owner | **ADR-052 v1.7 F1 PC1 structured-equivalence reconciliation (adversary pass-5 F-3 MED).** Rewrote Postcondition 1 from the "byte-for-byte concatenation" model to the structured per-BC-row-equivalence model per ADR-052 §Decision 7c step 3b / §Decision 5a drain step 5(c): PC1 now describes extracting BC-X.YY.NNN table rows from staged shard files, sorting in canonical BC-ID order, computing SHA-256, and comparing against `source_body_row_sha256` from the txn record. Explicitly calls out that `§Summary`, `§Subsystem Shard Manifest`, cross-cutting invariants, and non-row separator lines are excluded from both the staged-row extraction and the source hash — preserving the "modulo the manifest section" intent. Explicitly states that a whole-concat SHA-256 against `source_sha256` is UNSATISFIABLE (the staged lean body adds the §Subsystem Shard Manifest section, making the whole-concat SHA unequal to `source_sha256` by construction) and that `source_sha256` is used ONLY by step 5 fingerprint recheck (Postcondition 3a). Updated VP-132 description in Verification Properties table to the structured-row-equivalence formulation with the `source_body_row_sha256` / `source_sha256` scope distinction. Updated VP-132 label in VP Anchors from "(proptest; content-preservation byte-for-byte)" to "(proptest; content-preservation structured-row-equivalence)". input-hash recompute owed to state-manager (compute-input-hash --update). |
| 1.6 | 2026-09-13 | product-owner | ADR-052 v1.7 re-hardening (F2/F3/F6/F7). (F2) Replaced HookResult/E-SHD-005 terminology in Canonical Test Vectors rows 3–4 and VP-133/VP-Anchors prose with correct process exit code terminology — CONTENT_PRESERVATION_ABORT (exit 2) for content-preservation failure, CENSUS_MISMATCH_ABORT (exit 2) for census/ID-set failure; the migration binary is Bash-invoked and emits process exit codes, not HookResult values; E-SHD-005 is scoped to the steady-state native gate (BC-1.18.006/BC-1.18.010) only. (F3) Corrected all path-bearing COMPLETED.json occurrences in Precondition 5, Invariant 3, Architecture Anchors, and Traceability ADR row to lowercase completed.json per ADR-052 §Decision 7c step 8. (F6) Amended Precondition 2 and Invariant 1 to disclose ADR-052 §Decision 7 NEW multi-file crash-atomicity machinery: removed the false claim that the migration "does not invent new atomic-write machinery"; Precondition 2 now states BC-1.18.006's write_atomic handles per-file writes while §Decision 7a (advisory flock + durable txn record), §Decision 7b (framed intent log + WAL boundary + matching-destination-hash recovery), and §Decision 7c (CURRENT.json pointer swap + completed.json terminal record) provide the multi-file atomicity envelope; Invariant 1 retitled to reflect write_atomic invocation (not reimplementation) plus §Decision 7 machinery disclosure. (F7) SDK Grounding: removed write_indeterminate_marker (CAP-041 INDETERMINATE quarantine-marker writer; not an atomic-write staging primitive; incorrectly cited in prior versions); added grep confirming last_amended_migrate::atomic_write::write_atomic is called in crates/factory-dispatcher/src/shard_manager.rs; updated Architecture Anchors first bullet to cite write_atomic by fully-qualified name. |
| 1.5 | 2026-09-13 | product-owner | ADR-052 v1.5 re-hardening (3 amendments). (1) Invariant 3 — COMMITTING-window reader protocol corrected to generation-first/canonical-fallback (C-1 mirror): each file is accessible at `gen-<uuid>/` path (not yet moved to canonical) OR at its canonical path (already moved by step 7's progress); generation-first is correct for BOTH net-new shards AND in-place-overwrite targets such as BC-INDEX.md; citation updated from "canonical-first / generation-fallback (ADR-052 §Decision 7c reader protocol, C1 fix)" to "generation-first/canonical-fallback (ADR-052 §Decision 7c C-1 fix)". (2) EC-003 — corrected citation from "step 3b per ADR-052 §Decision 4e" to "ADR-052 §Decision 7c step 3b (referenced by §Decision 4e)" for precision (step 3b is defined in §7c; §4e references it for the resume-from-STAGING case); version pin "(v1.4)" removed from "H3 fix" label per H-4 POLICY 19 / TD-VSDD-091; stable form "§Decision 7c step 3b (H3 fix)" substituted. (3) Traceability ADR row — ADR-052 §Decision 4/5a/7a/7b/7c/8 added alongside existing ADR-051 citations, aligning Traceability with Architecture Anchors on governing ADRs (M-1). |
| 1.4 | 2026-09-13 | product-owner | ADR-052 v1.4 re-hardening (5 amendments). (1) Postconditions 1 and 2: added explicit cross-reference to ADR-052 §Decision 7c step 3b — PC1 (content-preservation) and PC2 (independent census) are verified against the staged generation BEFORE the CURRENT.json pointer swap; step 3c aborts cleanly (txn → ABORTED, gate → OPEN) on failure. (2) Postcondition 3a (Amendment 7): corrected pointer-swap step index from "step 5" to "step 6" (step 5 = fingerprint recheck; step 6 = CURRENT.json pointer swap = the commit point). (3) Invariant 3: added COMMITTING-window reader protocol (C1 fix): during COMMITTING, new-generation content is accessible via canonical-first / generation-fallback — each file is at its canonical path (if already moved by step 7) OR at gen-uuid/ (not yet moved); rename(2) atomicity ensures ENOENT is not possible for any file during the committing window (ADR-052 §Decision 7c reader protocol). (4) EC-003 resume logic: added H3 fix note — resume-from-STAGING MUST re-run the full census (step 3b) before proceeding to the pointer swap; the census gate is not skippable on resume (ADR-052 §Decision 4e). (5) SDK Grounding Evidence re-grounded: the literal `total_bcs: 2005` stdout value replaced with structural form `total_bcs: <N>` (per POLICY 5 HEAD-reproducibility mandate; mirrors BC-1.18.010 v1.2's structural-form fix for the identical drift class — the value is 2006 at HEAD and will continue to drift; any future reader MUST re-execute the grep at HEAD for the current count). No state-name corrections required: no PREPARED occurrences exist in the v1.3 body (v1.3 already replaced all such occurrences with STAGING/COMMITTING/COMPLETED/ABORTED). |
| 1.3 | 2026-09-13 | product-owner | ADR-052 v1.3 re-hardening (6 amendments). Amendment 4 — Precondition 5 replaced: PREPARED/COMMITTED/CLEANED phase-marker model replaced with STAGING/COMMITTING/COMPLETED txn-record state machine (`txn-<uuid>.json`) + framed checksummed intent log (`intent-<generation_uuid>.log`); EC-003 now reads intent log + txn record (matching-destination-hash rule) instead of `completed_renames`. Amendment 5 — Precondition 6 replaced: O_CREAT\|O_EXCL + alive-PID lock model replaced with dual independent mechanisms: (a) advisory flock on stable pre-created never-unlinked inode (kernel auto-releases on death); (b) txn record state STAGING/COMMITTING blocks ALL mutation tool calls via native admission gate regardless of PID liveness; (c) OPEN/DRAINING gate with writer reservations ensures quiescence before snapshot. Amendment 6 — Postcondition 3 corrected: removed "dir-fsync is mandatory, not best-effort" blanket assertion; replaced with platform-branched durability: Linux mandatory fsync+dir-fsync; macOS F_FULLFSYNC on file mandatory, APFS dir-fsync best-effort only (Apple docs do not guarantee power-loss durability for directory fsync); cites `sync_file_durable()`/`sync_dir_best_effort()` per ADR-052 §Decision 7d. Amendment 7 — Postcondition 3a updated: TOCTOU fingerprint check still EXACTLY ONCE; reference changed from "before the first rename" to "before the CURRENT.json pointer swap (step 5 in ADR-052 §Decision 7c)"; compared against `source_sha256` in txn record (not PREPARED marker); abort sets txn record to ABORTED. Amendment 8 — Invariant 3 updated: "COMMITTED marker is the sole commit-point" replaced with "CURRENT.json atomic pointer swap is the sole commit-point"; `completed_renames` tracking replaced with intent log + matching-hash rule for forward recovery; COMPLETED.json is the permanent terminal record. Amendment 9 — Architecture Anchors updated: §Decision 7 generic citation replaced with §Decision 7a (advisory flock + durable txn record + fencing_generation), §Decision 7b (framed intent log + WAL boundary + recovery decision table), §Decision 7c (CURRENT.json pointer swap + COMPLETED.json + reader protocol); §Decision 5a description updated to OPEN/DRAINING gate + txn state check. |
| 1.2 | 2026-09-13 | product-owner | ADR-052 v1.2 hardening (6 delta amendments over v1.1): Amendment 3 — Postcondition 7 scope-clarification reference updated from "ADR-052 v1.1" to "ADR-052 §Decision 1". Amendment 4 — Precondition 5 PREPARED/COMMITTED/CLEANED descriptions updated: PREPARED now includes `completed_renames` list initialized and "original content preserved in staging"; COMMITTED adds "per-target hashes verified" and "Written AFTER the last rename and dir-fsync"; CLEANED adds "terminal success state"; EC-003 sentence updated to "reads the `completed_renames` field to determine which renames succeeded and which must be retried; it does NOT re-run the full split from scratch." Amendment 5 — Precondition 6 corrected: lock file content added "(containing PID + activation_id + timestamp per ADR-052 §Decision 7a)"; "Ordinary governed writers (Edit/Write tool calls validated by the `validate-factory-path-staging` dispatcher guard)" replaced with "ALL mutation tool calls (Edit/Write/MultiEdit/Bash) targeting BC-INDEX paths are blocked by the native admission gate in `executor.rs` (ADR-052 §Decision 5a) when this lock file exists with an alive PID"; TOCTOU drain-protocol sentence added; `validate-factory-path-staging` guard sentence removed. Amendment 7 — Postcondition 3a rewritten: "EXACTLY ONCE, before the first rename" (not per-rename); compares against `source_sha256` field in PREPARED marker; "The migration requires re-activation" added; replaced closing sentence with v1.1 contradiction explanation. Amendment 8 — Invariant 3 updated: "single COMMITTED phase marker" replaced with "COMMITTED phase marker ... and the `completed_renames` tracking in PREPARED"; state machine description updated to "(staging in progress, with zero or more completed renames tracked)". Amendment 9 — Architecture Anchors updated: §Decision 4 description updated to "(two-phase validation: pre-lock and under-exclusion)"; new §Decision 5a anchor added; §Decision 7 description updated to include "per-target completion tracking + content-bearing lock file with crash-recovery ownership protocol"; §Decision 8 updated to "accurate skipped-control inventory and enumerated allowed write targets". No existing guarantee weakened. |
| 1.1 | 2026-09-12 | product-owner | ADR-052 v1.1 hardening (9 amendments): Amendment 1 — removed A/B2 simultaneous-activation coupling from Precondition 4 ("at the SAME moment BC-1.18.008 runs" language deleted; each migration independently gated via its own armed-activation manifest per ADR-052 §Decision 4). Amendment 2 — Postcondition 6 last sentence "at the SAME F4 activation moment mechanism A's own backfill (BC-1.18.008) runs" replaced with "at F4 activation, as part of the same one-time B2 migration operation (independently of mechanism A's activation schedule)" (SS-05/SS-06 B2-internal atomicity preserved; only A/B2 simultaneous-activation coupling removed). Amendment 3 — appended Postcondition 7 scope clarification ("governs B2/Cohort-B independence only; A/B2 scheduling independence governed by Precondition 4 as amended"). Amendment 4 — new Precondition 5: durable phase-marker file at `.factory/migration-state/migrate-bc-index-state.json` (PREPARED/COMMITTED/CLEANED transitions, EC-003 resume logic). Amendment 5 — new Precondition 6: writer-exclusion advisory lock at `.factory/migration-state/exclusive.lock`, governed writers fail E-MAINTENANCE-001 during migration window, lock released on COMMITTED or abort (ADR-052 §Decision 5 guard). Amendment 6 — appended mandatory dir-fsync mandate to Postcondition 3 (each rename(2) followed by fsync on parent directory; mandatory, not best-effort; POSIX rename(2) + Pillai et al. OSDI'14). Amendment 7 — new Postcondition 3a: TOCTOU pre-commit source-fingerprint recheck immediately before PREPARED→COMMITTED transition (SHA-256 recompute vs. census-time fingerprint; abort + rollback + delete PREPARED marker on mismatch). Amendment 8 — appended COMMITTED-marker commit-pointer specification to Invariant 3 (sole commit-point for the multi-file atomic operation; N independent write_atomic calls without it do not satisfy all-or-nothing). Amendment 9 — added ADR-052 §Decision 4/7/8 to Architecture Anchors. ADR-052 added to inputs. No existing guarantee weakened. |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, fix-burst addition per F-S2502-F2-002 HIGH, ADR-051 v1.1 Decision 10). Allocated as BC-1.18.011 — confirmed as the next free SS-01 slot against the live `ss-01/` directory (BC-1.18.001–010 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism B2's BC-INDEX body split: byte-for-byte content-preservation, independent-census integrity (every BC row in exactly one shard, cross-checked against `total_bcs`), staging+verify+atomic-replace crash-atomicity, fail-loud rollback on verification failure, idempotency against a partial prior attempt, and the SS-05/SS-06 second-level sub-split covered within the SAME one-time operation. Explicitly confirmed NO new Cohort-B sequencing dependency (unlike BC-1.18.008). Modeled directly on BC-1.18.008's structure per the F2 architecture-delta doc §4a authorship input. CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier per the established project convention. ADR-051 §D10/§D7/§D8 citations. |
