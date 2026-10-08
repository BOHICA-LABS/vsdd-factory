# ADR-052 v1.23 Architect Calls: Independent Validation (Exit Codes, Unknown-Field Rejection, Interim Fail-Closed)

**Date:** 2026-10-07
**Project:** drbothen/vsdd-factory (self-referential)
**Researcher:** vsdd-factory:research-agent (adversarial validation mandate: evidence against each call sought as hard as evidence for)
**Scope:** Three architect rulings in ADR-052 §Error Code Semantics, "Migration binaries — recovery and finalize legs", items 8-11, and their implementation on `feature/S-25.09` @ `46169f0d`
**Artifacts read:**
- `.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md` (§Error Code Semantics; items 1-11; v1.21 additions table)
- `.factory/specs/prd-supplements/error-taxonomy.md` (rows `FOREIGN_MIGRATION_REFUSED`, `MIGRATION_LOCK_CONTENTION`, `COMPLETION_RECORD_MISMATCH_ABORT`, v1.40 changelog)
- `.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager.rs`, `shard_manager/admission.rs`, `main.rs`

---

## Summary of Verdicts

| # | Call | Verdict | Required change (short form) |
|---|------|---------|------------------------------|
| 1 | `FOREIGN_MIGRATION_REFUSED` (exit 2) + `MIGRATION_LOCK_CONTENTION` (exit 1) | **CONFIRMED-WITH-CHANGES** | Keep both codes and both exits. Fix the ADR's own definition of exit 1 (item 7(e) and the §Error Code Semantics Note still say exit 1 means "re-activation required", which lock contention is not). Exit 75 is a documented alternative, not a requirement. |
| 2 | Reject unknown top-level keys on a live known txn record at a rewriting arm | **CONFIRMED-WITH-CHANGES** | Rejection is the right default. Close three gaps: (a) unknown keys inside `pending_canonical_moves[]` elements are still silently dropped on rewrite; (b) the txn record has no `schema_version`, so "newer schema" can only be guessed from extra keys; (c) the operator sees `txn_record_malformed` (meaning corruption) when the cause is "written by a newer build". Also state explicitly why the null-generation raw discard keeps unknown keys. |
| 3a | Interim exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` when the own migration's live txn sits beside `completed.json` | **CONFIRMED-WITH-CHANGES** | Fail-closed is correct. But this build reaches that state on its own normal path, because `finish_committing_migration` ignores errors from the COMPLETED txn write (`let _ = write_txn_record(..)`) and still exits 0. Make that write fail loudly. Write down the operator recovery procedure, or make S-25.06 AC-031 a release prerequisite. |
| 3b | Flock held by another process + `completed.json` present ⇒ exit 0 `AlreadyMigrated` | **REJECTED** | Return `MIGRATION_LOCK_CONTENTION` (exit 1) on this path too. Separately, re-check `completed.json` after the lock is acquired: today it is read before the lock (TOCTOU), and a lost race can start a fresh run on a tree that is already migrated. |

---

## Codebase Grounding (anchors are function names, per TD-VSDD-091)

### Exit-code mapping
- `BcIndexMigrationError::process_exit_code`: `ExpiryAbort { .. } | MigrationLockContention => 1`, `_ => 2`. A wildcard arm, so every new variant defaults to exit 2.
- `migration_process_exit_code`: `Ok(_) => 0` for both `AlreadyMigrated` and `Completed`.
- `main.rs` sends `argv[1] == "migrate-bc-index"` to `run_migrate_bc_index_cli` **before** the hook-envelope stdin read and calls `std::process::exit` with that code. The in-code comment says: "the migration subcommand is a distinct invocation mode, never a hook dispatch". So the Claude Code hook meaning of exit 2 does **not** apply to the migration subcommand. Its exit status reaches the agent only as a Bash-tool result. The hook path's exit 2 (release-on-block in `main`) is a separate invocation mode.
- The only other non-zero exits this binary can produce are Rust's panic status (101) and shell-level 126/127. Within the migration subcommand, exit 1 therefore means exactly {`EXPIRY_ABORT`, `MIGRATION_LOCK_CONTENTION`}, and the stderr code token tells them apart.

### Lock and precedence (`run_bc_index_migration_core`)
1. `completed_present = fs.exists(migration_state_dir/"completed.json")` is evaluated **before** the lock.
2. `try_acquire_migration_lock` uses `std::fs::File::try_lock`, which is `flock(LOCK_EX|LOCK_NB)` on Unix and `LockFileEx` on Windows, stable since Rust 1.89.0. The workspace pins `rust-toolchain.toml` `channel = "1.95.0"`, so this is supported. `TryLockError::WouldBlock` ⇒ `Ok(None)`.
3. If `completed_present` and the lock is **not** acquired ⇒ `return Ok(BcIndexMigrationOutcome::AlreadyMigrated)` (exit 0, zero reads of txn/gate state).
4. If `completed_present` and the lock is acquired ⇒ `completed_json_interim_short_circuit`. That function runs the Tier 0 loader, then `select_live_txn`. An own live record ⇒ `CompletionRecordMismatchInterim` (exit 2). A foreign live record ⇒ `ForeignMigrationRefused` (exit 2). No live record ⇒ the gate is written OPEN only if it is not already OPEN.
5. If not `completed_present` ⇒ `lock_guard.ok_or(MigrationLockContention)?` (exit 1). Then: Tier 0 loader → one-live-txn → foreign refusal → `recover(&planner_records, None, None, ..)`, with **`completed` hard-coded to `None`** and justified by the comment "`completed.json`'s presence was already checked ... `None` is the honest current read". That pre-lock read is the TOCTOU described under Call 3b.

### Strict decode and rewrite
- `decode_txn_record_strict`: checks for missing required keys. On `file.is_live()` it rejects any top-level key other than `migration_id` and `TXN_RECORD_REQUIRED_KEYS` (`txn_record_malformed`, key names sanitized to 64 chars). Then `serde_json::from_value` into `BcIndexMigrationTxnRecord`.
- `BcIndexMigrationTxnRecord` and `PendingCanonicalMove` derive `Deserialize` **without** `#[serde(deny_unknown_fields)]` and **without** a `schema_version` field. Other on-disk records in the same module carry `schema_version: u32` (= 1); the txn record does not.
- `write_txn_record` re-serializes the typed struct and re-inserts only a present `migration_id` (via `existing_migration_id`). Nothing else from the old file survives.
- `admission::abort_null_generation_txn` (shared by admission Branch B and the coordinator) rewrites the **raw object**: "`state = ABORTED` plus `abort_reason = "null_generation"`, every other field preserved". That is preserve-and-pass-through semantics on one arm.

### Finalize path
- `finish_committing_migration`: `execute_canonical_path_moves` → `write_completed_record` (writes `completed.json`) → `txn.state = Completed` → **`let _ = write_txn_record(fs, migration_state_dir, txn);`** (error discarded) → best-effort gate OPEN (warn on error) → `Ok(Completed)` ⇒ exit 0.

---

## Call 1: `FOREIGN_MIGRATION_REFUSED` (exit 2) and `MIGRATION_LOCK_CONTENTION` (exit 1)

### External findings (verified; URLs below)

| Source | Lock-contention exit behavior |
|---|---|
| `sysexits.h` `EX_TEMPFAIL` = 75 | "temporary failure ... user is invited to retry". The documented example is a mailer connection failure. FreeBSD `sysexits(3)` marks the interface **deprecated / discouraged / not portable**. glibc still ships the header and its manual recommends following an existing convention, noting 0/1 is the commonest. |
| util-linux `flock(1)` | `-n` / `-w` failure exits **1** by default. `-E/--conflict-exit-code N` overrides it. Other flock errors use sysexits codes; lock conflict is expressly the exception. |
| git `index.lock` | Fatal `die()` ⇒ **128**. That is git's generic fatal status, not a retry code. |
| apt / apt-get | **100**, the generic error code. dpkg: **2**, the generic fatal code. Neither has a lock-specific code. |
| cargo package-cache / build-dir lock | **Blocks and waits** ("Blocking waiting for file lock"). No contention exit. |
| Terraform state lock | **1** (generic error). `-lock-timeout` retries; the default `0s` fails immediately. |
| pacman, Homebrew, rpm, npm | No documented lock-specific numeric exit. |
| Claude Code hooks | Exit 2 = blocking error on blocking-capable events; exit 1 = non-blocking error. This applies **only** to hook commands. |

**Synthesis.** No cross-tool convention says "lock contention = 75". The closest primitive, `flock(1)`, uses **1**. Terraform uses **1**. Package managers use their generic error code. 75 is defensible only as an application-defined contract with its callers.

### Adversarial analysis

**Against exit 1:**
1. **The ADR contradicts itself.** Item 7(e) says: "Exit 1 is reserved for conditions re-activation resolves; exit 2 for conditions that need an operator." The Note at the end of §Error Code Semantics says: "the `exit 1` for `EXPIRY_ABORT` signals 'no harm done, but re-activation required'". Lock contention is not resolved by re-activation; it is resolved by waiting. The v1.23 item-9 sibling ruling placed contention at exit 1 without updating either definition. This is a real spec defect: whoever reads the exit-1 definition gets the wrong follow-up action.
2. Exit 1 now covers two conditions with different follow-ups (re-activate vs. wait and retry). A purely exit-code-driven caller cannot tell them apart. Exit 75 would make contention machine-actionable.

**For exit 1 (and against 75 or 2):**
1. Exit 2 is wrong for contention. In this repo, exit 2 means "fail-closed, needs an operator" (item 7(e)). Contention needs no operator, nothing changed, and the precedence puts the flock **before** the loader, so the condition carries no integrity verdict.
2. The binary-mode exit code has no hook-blocking meaning (`main.rs` comment, quoted above). The concern that exit 2 has hook meaning in Claude Code is therefore not a reason either way. It only reinforces that exit 2 should stay reserved for real fail-closed outcomes.
3. Exit 1 matches the closest precedents (`flock(1)` default, Terraform). 75 comes from an interface FreeBSD itself discourages.
4. A blind retry on the shared exit 1 is harmless in both cases. After `EXPIRY_ABORT` the txn is ABORTED and the gate is OPEN, so a re-run takes `AbortedTerminal` → fresh run. That is the "re-activation" in a build with no manifest reader (`ManifestStatus::StillValid` is fixed today). After contention, a re-run is the intended action. The stderr token (`EXPIRY_ABORT` vs `MIGRATION_LOCK_CONTENTION`) tells them apart for the actual consumer, an agent reading Bash-tool stderr.
5. The taxonomy says S-25.06's `AppendLogMigrationError::LockContention` already uses exit 1 (that variant is not on this branch, so this was not verified in code). Moving to 75 would force a coordinated change across both binaries for no safety gain.

**Foreign refusal as its own code: justified.** Each existing code is wrong here:
- `E-MAINTENANCE-001` is a `HookResult`, not a process exit code.
- `BINARY_INTEGRITY_FAILURE` is reserved for the digest/TOCTOU check (v1.21 F-012). Reusing it repeats the mislabel item 9 corrects.
- `MIGRATION_STATE_INTEGRITY_FAILURE` asserts corruption. A well-formed record owned by another migration is not corrupt.
- `COMPLETION_RECORD_MISMATCH_ABORT` is reserved for the own migration's terminal record.

Exit 2 is correct, and the reason is the precedence. Both coordinators share `exclusive.lock` and hold it for their whole run. If the invoking binary **acquired** the lock and still sees a live foreign record, the foreign coordinator is not running: it crashed. Only the owning subcommand can recover it, which is an operator action, and re-running this subcommand cannot help. That matches git's refusal on unknown extensions and the 2PC "leave in-doubt for its owner" practice (Call 3 sources).

### Verdict: **CONFIRMED-WITH-CHANGES**

Required changes (architect, ADR-052; product-owner, error-taxonomy mirror):
1. Rewrite item 7(e) and the closing Note to define exit 1 by class rather than by one trigger. Suggested text: "Exit 1 = non-error termination: nothing harmful was done and re-invoking is safe; the stderr code token names the follow-up (`EXPIRY_ABORT` ⇒ re-activation required; `MIGRATION_LOCK_CONTENTION` ⇒ retry after the holding coordinator exits). Exit 2 = fail-closed, operator action required."
2. Optional hardening for the implementer: replace the `_ => 2` wildcard in `process_exit_code` with an exhaustive match, as `admission_failure_cause` already does. A future variant then has to be assigned an exit code explicitly instead of silently getting 2.
3. Alternative, offered but not recommended: exit 75 for contention, applied to both binaries at once. Choose it only if a non-agent automated caller that branches on exit codes is planned.

---

## Call 2: Reject unknown top-level keys on a live known txn record

### External findings (verified)

| Practice | Rule for unknown data that will be rewritten |
|---|---|
| git `repositoryformatversion` / `extensions.*` | Git **MUST NOT proceed** if any `extensions.*` key is unknown (format 1). An older git must not operate on, and so must not rewrite, a newer repository. |
| SQLite header bytes 18/19 | Write version too high ⇒ **read-only**. Read version too high ⇒ cannot read or write. Understanding enough to read is explicitly separated from being safe to rewrite. |
| PNG (W3C PNG 3rd ed.) | An unknown **critical** chunk is fatal. An editor **shall not copy an unknown unsafe-to-copy chunk if it modifies critical data**. Ignoring data while reading does not license carrying it into a modified file. |
| RFC 5280 / RFC 7515 `crit` | Unrecognized **critical** extensions MUST be rejected. Unknown non-critical ones may be ignored. Tolerance requires a criticality signal. |
| Protocol Buffers (proto3, 3.5.0+) | Unknown fields are preserved in binary round-trips. JSON conversion or field-by-field rebuilds **lose** them. Caveat: no guarantee that a preserved field stays semantically current after an older program changes related known fields (`oneof` example). |
| Kubernetes | A typed older client's GET-modify-PUT can drop newer fields (api-conventions, api_changes). The server default is `fieldValidation=Warn`, which still **drops** the field. Structural CRDs prune unknown fields unless they set `x-kubernetes-preserve-unknown-fields`. |
| PostgreSQL `pg_control` | Refuses to start (FATAL "database files are incompatible with server") on a `PG_CONTROL_VERSION` / `CATALOG_VERSION_NO` mismatch, "to abort before possible damage". |
| serde 1.0.229 / serde_json 1.0.151 (crates.io, verified 2026-10-07; lockfile pins 1.0.228 / 1.0.149) | Unknown fields are ignored by default. `deny_unknown_fields` errors instead. `deny_unknown_fields` is **not supported** together with `flatten`. The documented capture pattern is `#[serde(flatten)] extra: Map<String, Value>`. |
| RFC 9413 (Thomson and Schinazi) | Postel-style tolerance belongs where the format **explicitly specifies** what may be ignored or preserved. Silently tolerating unexpected input entrenches errors. |

### Adversarial analysis

**The case for preserve-and-pass-through (`#[serde(flatten)] extra`):**
- Downgrade-friendly: an older binary keeps working, and nothing is silently dropped.
- Protobuf chose it.

**Why it loses here:** the recovery arms that rewrite the record (`ResumeFromStaging`, `ForwardRecovery`, `CleanAbortExpiredStaging`) do not just carry the record along. They **advance its state machine** (STAGING → COMMITTING → COMPLETED/ABORTED) and act on the filesystem. A newer build's field (for example, a further set of pending moves, or a cleanup obligation) may change what a correct state transition is. An older build that preserves the field while advancing state produces a record whose bytes are intact and whose meaning is false. That is exactly the PNG "unsafe-to-copy while modifying critical data" hazard and the protobuf staleness caveat. The record has no criticality marker (no `crit` list, no per-key ignorable flag). Without one, every precedent that rewrites state (git, SQLite, PNG, X.509, PostgreSQL) defaults to **refuse**. Drop-silently (serde's default) is the Kubernetes failure mode and is worst of all. For the downgrade case (old binary meets new record), refusal with nothing mutated is what git, SQLite and PostgreSQL do.

**Gaps found in the call as written and implemented (against it):**
1. **Nested unknown keys are still silently dropped (code vs. ADR).** Item 10 says `pending_canonical_moves` must have "every element strictly decoded the same way". `decode_txn_record_strict` rejects unknown keys only at the **top level**. `PendingCanonicalMove` has no `deny_unknown_fields`, so `serde_json::from_value` ignores extra keys inside elements, and `write_txn_record` writes the record back without them. This is the same fabricated-state hazard the ruling exists to close, one level down. Under TD-VSDD-060 (sibling sweep) it belongs to the same fix. Fix: add `#[serde(deny_unknown_fields)]` to `PendingCanonicalMove` (no `flatten` involved, so serde supports it), or check element keys explicitly, and add a red test with an unknown key inside an element.
2. **No version signal.** The record has no `schema_version`, unlike sibling on-disk records in the same module (`schema_version: 1`). "Newer schema" is inferred from extra keys, which misses a newer build that changes the meaning of an existing key without adding one. Every precedent above (git, SQLite, PostgreSQL) uses an explicit version as the main gate and treats unknown members as the secondary gate. ADR item 11 states no activation has ever run in this repository, so adding a required `schema_version: 1` costs nothing for legacy data now and will not be free later. Recommend an architect ruling: a required `schema_version`, where an unknown or greater version is refused with nothing mutated, and unknown keys at a known version remain malformed.
3. **The operator is told the wrong cause.** An unknown key on a known live record surfaces as `MIGRATION_STATE_INTEGRITY_FAILURE (txn_record_malformed)`, meaning corruption. The likely real cause after a rollback is "written by a newer build". The follow-ups differ: investigate corruption vs. run the newer build. The detail string names the keys, but the kind token conflates the two. Recommend a distinct `AdmissionStateIntegrityKind`, e.g. `txn_record_newer_schema` / `txn_record_unknown_key`, or a normative message suffix such as "possibly written by a newer build; recover with that build". The exit stays 2 either way.
4. **The asymmetry with the null-generation discard is unstated.** `abort_null_generation_txn` rewrites the raw object, keeps every other field, and sets ABORTED. That is preserve semantics on a state-advancing arm, which is the pattern rejected above. It is probably safe: by definition, under this schema, nothing was staged before a generation existed. But that reasoning assumes this schema's semantics, which is the very thing a newer record may break. The ADR should either justify the asymmetry explicitly (item 5 / item 10), or apply the unknown-key check before the discard on both surfaces at once (D-2 parity: admission Branch B and the coordinator share the primitive).

**Scope check, in favor of the call:** exempting terminal records and `migration_id` is correct. Terminal records are never rewritten; the archive is a rename that keeps the bytes. `migration_id` is Tier 0 and `write_txn_record` keeps it verbatim. Admission does not need the check, because a live known record gets a plain block with no fields read.

### Verdict: **CONFIRMED-WITH-CHANGES**

Rejection is the right default for state the tool will advance and rewrite. Preserve-and-pass-through is right only once a criticality / ignorable-field contract exists, and none does. Required changes:
- (a) Implementer: strict nested decode of `pending_canonical_moves[]`, plus a red test.
- (b) Architect: rule on a required `schema_version` for the txn record.
- (c) Architect + product-owner: a distinct operator-visible cause for "unknown key / newer schema".
- (d) Architect: justify, or align, the null-generation raw discard.

---

## Call 3a: Interim fail-closed exit 2 when the own live txn sits beside `completed.json`

### External findings (verified)
- **ARIES (Mohan et al., 1992):** redo is guarded by `pageLSN < recordLSN`, so it acts only on a condition it can verify inside one protocol. A transaction with a commit record but no end record is finished at restart **because the commit record is in the same log, LSN-bound to that transaction**. Undo is not pageLSN-guarded, so not every action is guarded.
- **RocksDB 2PC:** recovery binds `Commit(xid)` to `Prepare(xid)` **by XID**. A prepared transaction without a matching resolution stays prepared. It is never presumed.
- **2PC in-doubt practice:** PostgreSQL `PREPARE TRANSACTION` persists until an explicit `COMMIT/ROLLBACK PREPARED`. Oracle `DBA_2PC_PENDING` guidance warns that forcing the wrong outcome creates inconsistency. XA names the resulting risks `XA_HEURHAZ` and `XA_HEURMIX`. Presumed-commit and presumed-abort are designed protocol rules, not permission to guess.
- **Integrity tooling:** `e2fsck -p` repairs only what it deems safe and leaves the rest to an administrator. `btrfs check` does not modify by default. `zpool import -F` and Terraform `state push -force` require an explicit override.
- Source synthesis: no protocol was found that treats an **unbound** completion marker beside a live txn as enough evidence to finalize.

### Adversarial analysis

**For the call:** `completed.json` (`CompletedMigrationRecord { generation_id, txn_id, canonical_paths_count, .. }`) is bound to the txn only through fields that a verifier must check. This build has no verifier; ADR item 11 deliberately reserves it for S-25.06 AC-031 so a single shared verifier exists (D-2). Finalizing without checking the binding is a heuristic commit, in the XA heuristic-hazard class. Refusal mirrors the S-25.08 admission Branch C seam (`FailClosedMismatch`), so both surfaces agree. The call is correct.

**Against the call (real issues):**
1. **This build reaches the blocked state on its own normal path, and reports success while doing so.** `finish_committing_migration` writes `completed.json` and then runs `let _ = write_txn_record(fs, migration_state_dir, txn);`. If the COMPLETED txn write fails (ENOSPC, EIO, EACCES on rename), the error is discarded, the function returns `Ok(Completed)`, and the process exits 0. A crash between the two writes leaves the same state. The next invocation then hits the interim block (exit 2), and admission's fail-closed Branch C blocks every BC-INDEX writer (`E-MAINTENANCE-001` with the mismatch suffix) until an operator steps in. The interim block is right; the upstream success claim is a TD-VSDD-059 paper-fix, the same pattern item 8(b) already removed from the `EXPIRY_ABORT` path. Fix: propagate the COMPLETED-write error (`Io`, exit 2) so the run never claims success with a live txn left behind. This does not remove the crash window (`completed.json` is the commit point and the txn update is bookkeeping after it), so the operator path in item 3 below is still needed.
2. **"Blocks no real recovery" is true today but does not hold up over time.** ADR item 11 justifies the interim by saying no activation has run in this repository. Once a release ships `migrate-bc-index` to operator caches before S-25.06 AC-031 merges, any crash or failed write in the finalize window produces a hard block. Production-grade options: (i) make S-25.06 AC-031 a **release prerequisite** for any release that ships `migrate-bc-index`; or (ii) put a manual verify-then-finalize procedure in the ADR/runbook (compare canonical `sha256` against intent-log DONE `expected_post_hash` and `completed.json` `txn_id`/`generation_id` against the txn, then hand-advance txn → COMPLETED, then gate → OPEN). The line "operator investigation required" currently has nothing behind it.
3. **A cheaper verifier was available.** The external precedent (RocksDB XID, ARIES same-log binding) treats **identity binding** (`txn_id` / `generation_id` equality) as the minimum proof, and full content hashing as extra. The binary could finalize safely on identity binding alone. The ADR chose one shared verifier over two diverging ones; that is a legitimate D-2 trade-off, not a defect. It is recorded here as an option for the human, not a requirement.

### Verdict: **CONFIRMED-WITH-CHANGES**

Fail-closed is the production-grade choice, and finalizing best-effort would be a heuristic commit. Required changes:
- (a) Implementer: replace `let _ = write_txn_record(..)` in `finish_committing_migration` with error propagation. Sibling sweep: the same `let _ = discard_incomplete_staging(..)` pattern remains in the `ResumeFromStaging` failure arms. It returns the original error rather than claiming success, so it is lower severity, but the same audit should cover it.
- (b) Architect / devops: either gate release of `migrate-bc-index` on S-25.06 AC-031, or document the manual verify-then-finalize procedure that the interim line refers to.

---

## Call 3b: Flock held by another process + `completed.json` ⇒ exit 0 `AlreadyMigrated`

### External findings (verified)
- `flock(1)` reports contention (exit 1 by default). Mapping it to 0 needs an explicit `-E 0`, which is a scheduling policy for skippable cron ticks, not evidence that work completed.
- systemd timers firing while their unit is active leave the unit running. They do not certify completion.
- Git `update-ref` takes the lock and **then** verifies the expected old value. The authoritative check happens under the lock; a check made before the lock is only a hint.
- No source treats advisory-lock contention as meaning "already done".

### Adversarial analysis

**For exit 0:** the ADR's item 11(c) says exactly this ("EWOULDBLOCK ⇒ skip, exit 0"), so the implementer followed the spec. Not writing is correct: the gate of a live coordinator must not be touched (the §7e D4 hazard). Also, `completed.json` is written only after every canonical move is done (the guard in `finish_committing_migration`), so its presence strongly suggests the migration's data work is finished.

**Against exit 0, which decides it:**
1. **It contradicts the ADR's own `ALREADY_MIGRATED` predicate.** The row is: "own terminal record exists AND (clean steady state OR the §4e verify-then-finalize reconciliation succeeded)". Under contention the binary reads neither txn nor gate state, so neither disjunct is established. Exit 0 claims something that was not checked.
2. **The same on-disk state gives different verdicts depending on timing.** Take state S = `completed.json` + an own live COMMITTING txn (reachable via Call 3a issue 1). Uncontended, S ⇒ exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` (item 11(b)). If another process happens to hold the lock, the same S ⇒ exit 0. Likewise, a live foreign `backfill-append-logs` record ⇒ exit 2 `FOREIGN_MIGRATION_REFUSED` when uncontended, but exit 0 while that coordinator holds the lock. Fail-closed verdicts must not depend on timing. Item 9 itself says "contention beside a foreign record reports contention", and this path breaks that rule.
3. **It is inconsistent with the stated precedence.** `run_bc_index_migration_core` says "flock -> Tier 0 loader -> ... The flock is acquired FIRST on every path, including the `completed.json` short-circuit". Contention without `completed.json` is `MIGRATION_LOCK_CONTENTION`; with `completed.json` it is success. The only difference is a file read **without** the lock.
4. **TOCTOU (a separate defect on the same lines).** `completed_present` is sampled before `try_acquire_migration_lock`. Race: the check finds `completed.json` absent; a running coordinator then writes `completed.json`, marks its txn COMPLETED and releases the lock; this process then acquires the lock and takes the non-completed branch. `recover(.., completed = None, ..)` sees only a COMPLETED terminal record ⇒ `NoActiveTransaction` ⇒ **fresh run**: the gate goes to DRAINING then LOCKED and the already-sharded `BC-INDEX.md` is re-read for a new split. That contradicts `recover()`'s own contract that `completed.json` is "checked FIRST, unconditionally". The window is narrow but real, and the fix is trivial.

### Verdict: **REJECTED**

Required changes (architect: amend item 11(c) and the taxonomy `ALREADY_MIGRATED` / `MIGRATION_LOCK_CONTENTION` rows; implementer: code):
1. When the lock is not acquired, return `MigrationLockContention` (exit 1, "nothing was changed; retry after it exits") **whether or not** `completed.json` exists. The "skip, zero writes" behavior is kept; only the exit claim changes. `ALREADY_MIGRATED` exit 0 is then emitted only after the state was read under the lock.
2. Treat the pre-lock `completed.json` check as a hint only. After acquiring the lock, re-read `completed.json` and branch on that read: present ⇒ `completed_json_interim_short_circuit`; absent ⇒ recovery path. Pass the under-lock read to `recover()` instead of a hard-coded `None`. Add a red test that creates `completed.json` + a COMPLETED txn between the existence probe and lock acquisition (via the `Fs` seam) and asserts no fresh run.
3. Caller compatibility: idempotent re-invokers ("callers that re-invoke after completion must not treat this as a failure") still get exit 0 in the steady state. They see exit 1 only during real concurrent execution, which is the correct moment to retry.

---

## Top Sources

| Topic | URL |
|---|---|
| flock(1) default exit 1, `-E` | https://manpages.debian.org/trixie/util-linux/flock.1.en.html |
| sysexits deprecated (FreeBSD) | https://man.freebsd.org/cgi/man.cgi?sysexits |
| glibc exit status guidance | https://sourceware.org/glibc/manual/latest/html_node/Exit-Status.html |
| Terraform state locking / `-lock-timeout` | https://developer.hashicorp.com/terraform/language/state/locking |
| git lockfile API / `die()` = 128 | https://git-scm.com/docs/api-lockfile ; https://git.github.io/htmldocs/technical/api-error-handling.html |
| cargo flock (blocks) | https://github.com/rust-lang/cargo/blob/master/src/cargo/util/flock.rs |
| apt-get exit 100 / dpkg exit 2 | https://manpages.debian.org/trixie/apt/apt-get.8.en.html ; https://manpages.debian.org/trixie/dpkg/dpkg.1.en.html |
| Claude Code hook exit codes | https://code.claude.com/docs/en/hooks |
| Rust `File::try_lock` (1.89.0, flock / LockFileEx) | https://doc.rust-lang.org/stable/std/fs/struct.File.html |
| git repository-version / extensions MUST NOT proceed | https://github.com/git/git/blob/master/Documentation/technical/repository-version.txt |
| SQLite header read/write version | https://www.sqlite.org/fileformat.html |
| PNG critical / safe-to-copy | https://www.w3.org/TR/png-3/ |
| RFC 5280 critical extensions | https://www.ietf.org/rfc/rfc5280.html |
| RFC 7515 `crit` | https://datatracker.ietf.org/doc/html/rfc7515 |
| proto3 unknown fields | https://protobuf.dev/programming-guides/proto3/ |
| Kubernetes API conventions / field validation / CRD pruning | https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md ; https://kubernetes.io/docs/reference/using-api/api-concepts/ ; https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/ |
| serde `deny_unknown_fields`, `flatten` | https://serde.rs/container-attrs.html ; https://serde.rs/attr-flatten.html |
| RFC 9413 | https://www.rfc-editor.org/rfc/rfc9413.html |
| PostgreSQL pg_control version check | https://github.com/postgres/postgres/blob/master/src/backend/access/transam/xlog.c |
| ARIES paper | https://cs.stanford.edu/people/chrismre/cs345/rl/aries.pdf |
| RocksDB 2PC (XID binding) | https://github.com/facebook/rocksdb/wiki/Two-Phase-Commit-Implementation |
| PostgreSQL PREPARE TRANSACTION | https://www.postgresql.org/docs/current/sql-prepare-transaction.html |
| Oracle in-doubt / XA heuristics | https://docs.oracle.com/cd/B28359_01/server.111/b28310/ds_txnman004.htm ; https://docs.oracle.com/en/database/oracle/oracle-database/21/arpls/DBMS_XA.html |
| git update-ref verify-under-lock | https://github.com/git/git/blob/master/Documentation/git-update-ref.adoc |
| Terraform state push lineage/serial refusal | https://developer.hashicorp.com/terraform/cli/commands/state/push |
| crates.io serde / serde_json (versions verified) | https://crates.io/api/v1/crates/serde ; https://crates.io/api/v1/crates/serde_json |

**Version verification (2026-10-07):**
- serde **1.0.229** and serde_json **1.0.151** are the latest on crates.io (fetched from the registry API). Workspace `Cargo.lock` pins serde 1.0.228 and serde_json 1.0.149. The `deny_unknown_fields` / `flatten` semantics cited are unchanged across these versions per the serde docs.
- Rust `File::try_lock` was stabilized in 1.89.0. The workspace toolchain is 1.95.0.

**Not verified / inconclusive:**
- The exit code of `AppendLogMigrationError::LockContention`. It is cited by the ADR and taxonomy as exit 1, but it lives on S-25.06's branch, not on `feature/S-25.09`.
- pacman, Homebrew, rpm and npm lock-contention exit numbers (no documented lock-specific code).
- Whether a fresh run started by the Call 3b TOCTOU race would fail safely: likely a census/frontmatter failure on the sharded `BC-INDEX.md`, but this was not traced to its end. The finding stands regardless, because the run should never start.

---

## Research Methods

| Tool | Queries | Purpose |
|------|---------|---------|
| **Perplexity perplexity_research (PRIMARY)** | 3 | (1) Lock-contention exit codes across sysexits / flock / git / apt / dpkg / cargo / Terraform / pacman / brew / Claude Code hooks; (2) unknown-field handling for rewritten on-disk state (git extensions, SQLite, PNG, X.509, JWS, protobuf, Kubernetes, serde, RFC 9413, PostgreSQL); (3) crash-recovery fail-closed practice (ARIES, 2PC in-doubt, RocksDB, SQLite, git, fsck / ZFS / Terraform) and flock contention / TOCTOU |
| Perplexity perplexity_reason | 0 | — |
| Perplexity perplexity_search | 0 | — |
| Perplexity perplexity_ask | 1 | Rust `File::try_lock` stabilization version and OS primitive; serde / serde_json latest versions (then cross-checked against the registry) |
| Context7 | 0 | Not needed: the serde attribute semantics were cited from serde.rs via research call (2); no other library API question |
| Tavily | 0 | Not available in this session's toolset; cross-validation done via WebFetch of the crates.io registry API and codebase reads |
| WebFetch | 2 | crates.io API for `serde` and `serde_json` (version verification) |
| WebSearch | 0 | — |
| Codebase reads (Read / Grep) | ~20 | ADR-052 §Error Code Semantics items 1-11 and v1.21 table; error-taxonomy rows; `shard_manager.rs` (`process_exit_code`, `BcIndexMigrationError`, `run_bc_index_migration_core`, `completed_json_interim_short_circuit`, `try_acquire_migration_lock`, `recover`, `decode_txn_record_strict`, `write_txn_record`, `finish_committing_migration`, `BcIndexMigrationTxnRecord`, `PendingCanonicalMove`); `admission.rs` (`abort_null_generation_txn` doc); `main.rs` argv routing; `Cargo.lock`; `rust-toolchain.toml` |
| Training data | 2 areas | General framing of ARIES end-record handling and of the PNG / X.509 criticality analogy (each then confirmed by the cited research sources) |

**Total MCP tool calls:** 4
**Training data reliance:** low. Every external claim is tied to a cited URL from the research calls or to the crates.io registry. Every codebase claim is anchored to a function read directly on `feature/S-25.09`.
