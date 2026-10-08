---
document_type: behavioral-contract
level: L3
version: "1.13"
status: draft
producer: product-owner
timestamp: 2026-09-25T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md
  - .factory/specs/architecture/decisions/ADR-054-governed-migration-intent-log-format-fixed-move-plan-and-crash-recovery.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
input-hash: "6524e4d"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: draft
last_amended: "2026-10-08 (v1.13) — ADR-054 v1.0 §Downstream \"BC wording owed\" + ADR-052 v1.24 (human-authorized spec amendment 2026-10-08; story S-25.10 owns the shared intent_log module, S-25.06 consumes it): Precondition 5 re-worded to the shared module and the ADR-054 §Decision 1.7 reader (torn tail absent, mid-log corruption fails closed; fixed `canonical_move_plan` of four, N = 4 = `len(plan)`, never rewritten per move); Branch C hash source and `intent_log_path` bullets (`DONE == INTENT == sha256(file)`); the \"NOT a `pending_canonical_moves` field\" bullets replaced by `canonical_move_plan`; Postcondition 5a(b) AC-031 verifier steps (shared module; mid-log corruption ⇒ `canonical_hash_mismatch`; B-3 differing plan ⇒ `canonical_hash_mismatch` on verifiers, empty/ill-typed ⇒ `txn_record_malformed`); EC-050/EC-051/EC-052 extended (no new EC number: EC numbering unchanged). Prior: 2026-10-07 (v1.12) — ADR-052 v1.23 §Downstream (S-25.08 local adversary pass 3: F-S2508-L3-009 absent-vs-unstatable `.factory` classification; F-S2508-L3-001 two-tier txn-record interpretation); EC-035 reworded, EC-041..EC-048 added; sibling BC-1.18.011 v1.20. Same-version extension for the ADR-052 v1.23 binary-leg ruling (\"Migration binaries — recovery and finalize legs\", \"Branch C hash source\"): Postcondition 5a(b)/failure paragraph and EC-047 gain the binary leg; hash source corrected to the intent-log DONE record via `intent_log_path`; EC-049..EC-052 added. Status unchanged (draft). SECOND same-version extension (ADR-052 v1.23 items 8-11, \"second binary-leg extension\" Downstream block): Precondition 5 and the Postcondition 5a foreign-migration guard, EC-016 and its vector use `FOREIGN_MIGRATION_REFUSED` (exit 2, exact `backfill-append-logs: refused: …` line) in place of the retired \"LockContention-class\" label, and cite `AppendLogMigrationError::LockContention` (exit 1, `MIGRATION_LOCK_CONTENTION`) for flock contention; EC-047(a)-(c) and EC-049 cite \"migrate-bc-index §4e reconciliation (S-25.06 AC-031; fail-closed until then)\". No new EC row (EC numbering unchanged: EC-001..EC-052). THIRD same-version extension (ADR-052 v1.23 \"third binary-leg extension\": items 7(e), 9, 10(a)-(d), 11(c)-(e)): contention exit 1 on every path and the under-lock terminal-record read (Postcondition 5a(a)/(a2), Precondition 5/6 contention clause, EC-016, EC-004), the exit-1 class and exhaustive exit-code rule, the Branch B/C version gate (Precondition 6(c) rule 9) with the sixth `state_integrity` kind `txn_record_newer_schema` (BC-3.08.001 v1.37); EC-053..EC-057 added (EC numbering now EC-001..EC-057)."
introduced: v1.0-brownfield-backfill
modified: ["2026-10-07 (v1.12)", "2026-10-08 (v1.13)"]
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.013: Governed One-Time Migration for the Mechanism-A Backfill-Split of the Four Append-Log Files (Activation via the ADR-052 Sanctioned Execution Path)

## Description

BC-1.18.008 specifies the mechanism-A backfill-split ALGORITHM (record-boundary-safe
partitioning, the Backfill Recovery Manifest, per-file content-preservation and crash-atomicity)
but — exactly as BC-1.18.010 specified mechanism B2's end-state without specifying the
TRANSITION to it — does not itself specify how that algorithm is actually INVOKED against the
four live, currently-oversized append-log files (`decision-log.md`, `burst-log.md`,
`lessons.md`, `session-checkpoints.md`) without an agent performing a raw shell write that
POL-3/TD-FACTORY-HOOK-BYPASS-001 correctly blocks. BC-1.18.011 closed this exact gap for
mechanism B2 (the BC-INDEX body split) by specifying a governed one-time migration invoked via
ADR-052's sanctioned execution path (`Bash` tool, one-time interactive human approval, armed
activation manifest, native admission gate, multi-file crash-atomicity). This BC is that same
governed-migration wrapper for mechanism A: it specifies the `backfill-append-logs` subcommand's
activation contract, reusing BC-1.18.008's already-implemented split algorithm as the per-file
mechanism ADR-052 §Decision 7's multi-file atomic envelope wraps. It additionally closes the
Layer-2 loop opened by BC-1.18.008 alone: BC-1.18.008's backfill shrinks the four files ONCE,
but without ShardRegistry enrollment they grow unbounded again afterward — Postcondition 8
below specifies that closure. This BC directly discharges S-25.06's Spec-First Gate (S-7.01).

## Preconditions

1. BC-1.18.008's split algorithm (`run_mechanism_a_backfill_split`, the Record-Boundary Marker
   Table, the Backfill Recovery Manifest, and the per-file idempotency/crash-atomicity/rollback
   guarantees of BC-1.18.008 Postconditions 1-6 and Invariants 1-5) is fully specified and
   implemented, and is the SOLE per-file split mechanism this BC's governed migration invokes —
   this BC does not reimplement or alter any part of BC-1.18.008's algorithm (ADR-051 §Scope
   Note; Architecture Compliance Rule 3 of the S-25.06 story).

2. ADR-052 is ACCEPTED and human-ratified (POLICY 22, D-1232, 2026-09-20). The activation step is
   executed via the `Bash` tool with one-time interactive human approval at F4 activation — no
   standing `.claude/settings.json` allowlist entry — invoking the migration binary at its
   absolute trusted path: `{project-root}/target/release/factory-dispatcher backfill-append-logs`
   or `... backfill-append-logs --census` (ADR-052 §Decision 1, §Decision 3, §Decision 9). This
   resolves the S-25.06 Architecture Compliance Rule 7 contradiction (invocation is a `Bash`
   execution of a native binary under a closed argument grammar, NOT an Edit/Write tool call;
   ADR-052 §Decision 9 withdraws and replaces the story's original Rule 7 text). The binary lives
   in `crates/factory-dispatcher/` (ADR-052 §Decision 2), co-located with `shard_manager.rs`.

3. **Closed argument grammar — no path arguments, no auto-discovery (ADR-052 §Decision 3).**
   The accepted invocation forms are EXACTLY `backfill-append-logs` and
   `backfill-append-logs --census`. This SUPERSEDES the S-25.06 story's provisional AC-001(b)
   ("accepts... a cycle directory and auto-discovers the four canonical files within it"): the
   ratified design accepts no cycle-directory argument at all. The four target files are the
   FIXED set named in ADR-052 §Decision 8's ratified allowed-write-targets allowlist for the
   `v1.0-brownfield-backfill` cycle (see Postcondition 6). Story-writer must update S-25.06
   AC-001 to match this ratified grammar in the same burst this BC is registered.

4. An armed-activation manifest is written by state-manager to
   `.factory/activation/backfill-append-logs-YYYY-MM-DD.json` at the human-directed F4 activation
   step, per ADR-052 §Decision 4a's manifest schema (`activation_id`, `migration_id:
   "backfill-append-logs"`, `repo_root_sha`, `approved_by: "human-F4-interactive"`,
   `expires_after_hours: 24`; `approved_arch_index_sha`/`expected_total_bcs` are B2-only fields
   and are `null` for this migration). Pre-lock checks (§Decision 4b) and under-exclusion
   validation (§Decision 4c) apply as specified, EXCEPT item 4 of §Decision 4c (the three-way
   ARCH-INDEX parity check, ADR-052 §Decision 10) — that check is B2-specific (ARCH-INDEX governs
   the BC-S-prefix→SS-NN mapping BC-1.18.010's per-subsystem partition uses) and does NOT apply
   to this migration; `ARCH_INDEX_PARITY_ABORT` is unreachable for `backfill-append-logs`.

5. A durable transaction record at `.factory/migration-state/txn-<activation_uuid>.json` and an
   intent log at `.factory/migration-state/intent-<generation_uuid>.log` in the HARDENED wire format
   of ADR-054 §Decision 1 (v1.13; formerly "a framed checksummed intent log" per ADR-052 §Decision 7b,
   now a pointer) are maintained across the full migration lifecycle (STAGING → COMMITTING →
   COMPLETED, or ABORTED), per ADR-052 §Decision 7a and ADR-054 exactly as BC-1.18.011 Precondition 5
   specifies for B2. **Shared module (v1.13; ADR-054 §Decision 4; story S-25.10 introduces it, S-25.06
   consumes it):** this migration reads and writes the intent log through the SAME module as
   `migrate-bc-index` (`shard_manager/intent_log.rs`: record encode/decode, byte-exact checksum
   (ADR-054 §Decision 1.4), byte-level line-anchored reader (§Decision 1.7) with log invariants L1-L4
   (§Decision 1.8), writer with tail repair, value validation and strict barriers (§Decision 1.9), and
   the pure total `decide_recovery`); S-25.06's own copies (`write_append_log_intent_record`,
   `parse_append_log_intent_log_block`, `read_append_log_intent_log`,
   `append_log_intent_log_checksum_input`, `AppendLogIntentLogRecoveryDecision`,
   `decide_append_log_recovery`) are DELETED, not wrapped (S-25.06 rebases after S-25.10). A torn
   trailing record is ABSENT (the reader returns the valid prefix and the coordinator durably truncates
   the tail before its first append); corruption in the MIDDLE of the log fails closed
   (`INTENT_LOG_CORRUPT` on the coordinator; `canonical_hash_mismatch` on the verifier/admission
   surfaces). The txn field `canonical_move_plan` (ADR-054 §Decision 2; renamed from
   `pending_canonical_moves`) holds the four-entry fixed plan — N is FIXED at 4 and equals
   `len(canonical_move_plan)` for this migration — is persisted before the pointer swap and is never
   rewritten per move; completion is recorded only by txn-bound, INTENT-confirmed `DONE` records
   (B-1/B-2/B-3: BC-1.18.011 Postconditions 12-14, applying here through the shared per-move
   procedure of ADR-054 §Decision 3 and `finish_append_log_migration`). **`generation_id` /
   `intent_log_path` pairing (v1.13; mirrors BC-1.18.011 Precondition 6(f)(iii) and EC-069; ADR-052
   v1.24 "Branch C hash source" ruling (i)):** `intent_log_path` is written WITH `generation_id` and
   never derived; at the `backfill-append-logs` COORDINATOR's arm entry a live record that is
   COMMITTING, or STAGING with a string `generation_id`, whose `intent_log_path` is null, absent,
   non-string or not equal to `.factory/migration-state/intent-<generation_id>.log` is
   `txn_record_malformed` (exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`, nothing mutated); STAGING resume
   never fills in a missing value (it may only rewrite the identical one); the only valid `null` pairing
   is `generation_id: null` with `intent_log_path: null`. The
   `source_body_row_sha256` txn-record field is B2-only and is `null` for this migration (ADR-052
   §Decision 7a); `source_sha256` (the whole-corpus fingerprint over all four target files' bytes
   at quiescence) is the field this migration uses for the step-5 fingerprint recheck.

   **Migration discriminator and per-migration terminal-record namespace (v1.4; ADR-052 §Decision
   7e; closes D4).** The txn record carries `migration_id: "backfill-append-logs"` (ADR-052
   §Decision 7a txn-record schema; a record that lacks the field is read as `"migrate-bc-index"`,
   serde default — so BC-1.18.011's pre-existing records stay valid). Both migrations share ONE
   `exclusive.lock`, ONE `gate-state.json`, ONE `reservations/` directory and ONE `txn-*.json`
   directory (at most ONE live txn across both migrations), but the TERMINAL RECORD and POINTER
   are PER-MIGRATION: this migration writes and reads ONLY
   `.factory/migration-state/completed-backfill-append-logs.json` and
   `.factory/migration-state/CURRENT-backfill-append-logs.json`. It never reads, writes, or is
   satisfied by `completed.json` / `CURRENT.json`, which are `migrate-bc-index`'s (BC-1.18.011).
   The terminal-record schema is `{generation_id, txn_id, completed_at, canonical_paths_count}`
   (`canonical_paths_count`, not the draft name `file_count`; N = 4 here). **Cross-migration
   recovery refusal:** the `backfill-append-logs` binary recovers, resumes, finalizes or aborts
   ONLY a live txn whose `migration_id` equals `"backfill-append-logs"`; if the single live txn
   belongs to `migrate-bc-index` it exits 2 `FOREIGN_MIGRATION_REFUSED` (v1.12 same-version
   extension; ADR-052 v1.23 item 9 — the former "`LockContention`-class" label is RETIRED),
   performs NO mutation and MUST NOT run `recover()` over the foreign record (and symmetrically, per
   BC-1.18.011 Precondition 6(e), which carries the `migrate-bc-index` twin and the full
   precedence flock → Tier-0 loader → one-live-txn → foreign refusal → `recover()`). The
   `backfill-append-logs` binary mirrors `BcIndexMigrationError::ForeignMigrationRefused` as
   `AppendLogMigrationError::ForeignMigrationRefused { live_migration_id }`, with EXACT stderr
   `backfill-append-logs: refused: a live migration transaction owned by migration_id "<id>" is in progress (FOREIGN_MIGRATION_REFUSED, exit 2); this subcommand never recovers, finalizes or aborts another migration's record; nothing was changed`
   (`<id>` truncated to 64 chars, control characters escaped). Flock contention on this binary is
   NOT foreign: `flock(exclusive.lock, LOCK_EX|LOCK_NB)` EWOULDBLOCK is
   `AppendLogMigrationError::LockContention` (already built by S-25.06), exit **1** (nothing
   changed, retry later), aligned to taxonomy code `MIGRATION_LOCK_CONTENTION`, and is checked
   before the loader (contention beside a foreign record reports contention). **Contention on
   EVERY path (v1.12 third binary-leg extension; ADR-052 v1.23 item 11(c), mirrored from
   BC-1.18.011 Precondition 6(e)/6(f)(ii) and Postcondition 9(e)(a0)):** the flock is attempted
   BEFORE any state the coordinator will act on is read, so "not acquired" is
   `MIGRATION_LOCK_CONTENTION`, exit **1**, whether or not `completed-backfill-append-logs.json`
   (or `completed.json`) exists and whether or not a txn record is live, and NEVER exit 0
   (`ALREADY_MIGRATED` is emitted only for a state read UNDER the lock). `completed*.json` is read
   UNDER the lock, any pre-lock existence probe is a hint only and MUST NOT select the branch, and
   the under-lock read is what `recover()` receives (TOCTOU: EC-054). **Exit-code classes (ADR-052
   item 7(e), as amended):** exit 0 = the requested end state holds and was read under the lock;
   exit 1 = *no harm done, safe to re-run; the stderr code token says what to do next* — a class
   covering `EXPIRY_ABORT` (re-activate, then re-run) and `MIGRATION_LOCK_CONTENTION` (re-run after
   the holder exits); exit 2 = fail closed, an operator must act; `AppendLogMigrationError::
   process_exit_code` is an EXHAUSTIVE match with no wildcard arm (S-25.06, when it lands); the
   "exit 1 = re-activation" wording is retired. This
   cross-migration refusal is a property of the migration BINARIES' recovery path only, where
   "own" is the invoking subcommand (v1.8; ADR-052 §Decision 7e "Definition of foreign" (5)); it
   is the only place "own vs the other known migration" is a refusal. The admission gate itself
   is migration-agnostic: any live txn of either migration blocks the union, and on the
   dispatcher (shared-core) path "foreign" has a different, narrower meaning — `migration_id ∉ K`
   (Precondition 6(d) cell below; Postcondition 5a).

6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution via the SAME
   two independent mechanisms BC-1.18.011 Precondition 6 specifies: (a) the advisory flock on
   `.factory/migration-state/exclusive.lock`; (b) the native admission gate in `executor.rs`
   (ADR-052 §Decision 5a), which blocks ALL mutation tool calls (Edit/Write/MultiEdit/Bash)
   targeting `.factory/cycles/` paths — not only `.factory/specs/behavioral-contracts/` — while a
   txn record exists in STAGING or COMMITTING state, regardless of PID liveness; (c) the
   OPEN/DRAINING gate with writer reservations. Per the same delivery cross-reference as
   BC-1.18.011 Precondition 6(b) (D-1236 Ruling 3): the Edit/Write/MultiEdit legs ship with
   cluster-5 F4 TDD in `executor.rs`; the Bash leg ships separately as part of [D-1232-OBL-4].

   **6(b) Wiring obligation — the gate MUST be live on the dispatcher's REAL admission path
   (v1.3; closes S-25.06 formal-verification defect D1).** A gate function that exists and is
   exercised only by unit tests does NOT satisfy this precondition. Specifically:
   - **Where (v1.4: single shared admission core; v1.8: registry-independent evaluation position,
     obligations O1–O4; ADR-052 §Decision 5a "Evaluation position (v1.20)"; closes F-004).** The
     gate is ONE shared admission core (ADR-052 §Decision 5a "Shared protected-path union and
     shared admission state"; implemented in `shard_manager.rs`) served by exactly THREE
     migration-NEUTRAL executor entry points (v1.10; ADR-052 §Decision 5a "Entry-point naming
     (v1.21)"; closes F-014): `executor::migration_writer_admission(payload, cwd) ->
     MigrationAdmission` (the reservation-returning form `main.rs` calls),
     `executor::migration_writer_admission_precheck(payload, cwd) -> Option<HookResult>` (the
     verdict-only form over the same core) and `executor::migration_reservation_release(payload,
     cwd)` (the release leg). They exist exactly once and serve BOTH governed migrations over ONE
     protected-path union; NO per-migration delegates exist or may be added (a second name for the
     same function with no distinct behavior is false surface and invites a second
     implementation). The native admission (PreToolUse) and the reservation release
     (PostToolUse / PostToolUseFailure, 6(c)) are REGISTRY-INDEPENDENT:
     `crates/factory-dispatcher/src/main.rs` (`run`) MUST execute them immediately after the
     stdin payload parse and `resolve_project_cwd()` and BEFORE the `CLAUDE_PLUGIN_ROOT` tiering,
     `resolve_registry_path()` and `Registry::load`. Obligations (all testable black-box through
     the real dispatcher entry; EC-013, EC-022):
     **O1** — the core is evaluated EXACTLY ONCE per PreToolUse event (a second evaluation would
     re-create the reservation and re-run reconciliation);
     **O2** — it is evaluated BEFORE `Registry::load` / `resolve_registry_path()` / the Tier-1
     degraded-registry branch, so it runs identically with `CLAUDE_PLUGIN_ROOT` unset or empty,
     with a missing, unparseable or schema-mismatched registry, and with an empty matched-plugin
     set; the release leg (PostToolUse / PostToolUseFailure) is registry-independent in the same
     way;
     **O3** — it is evaluated BEFORE `shard_cap_precheck` (whose fired branch performs a
     destructive seal-and-truncate and must never run for a blocked protected-path write) and
     before every registry plugin tier;
     **O4** — an unparseable stdin payload cannot be classified (no protected path is known, so
     nothing is reservable): the existing parse-error exit is UNCHANGED.
     Verdict handling at this position: a `Block`/`Error` verdict terminates the dispatch
     directly through the same exit mapping the empty-tier short-circuit already uses (exit 2 +
     reason) WITHOUT loading the registry; an `Admitted` outcome stores the reservation handle
     and the dispatch continues into the registry stages unchanged. The release-on-block funnel
     (6(c)) covers EVERY later non-zero outcome, including registry fail-closed exits (e.g.
     schema-version mismatch, async+block conflict), shard-cap blocks and plugin blocks; a
     registry fail-open exit 0 leaves the reservation for the matching Post event, which is
     itself registry-independent. **Why:** the registry-load failure arms are fail-open by
     contract (BC-1.08.001: file-not-found / parse error ⇒ exit 0; `resolve_registry_path()?`
     propagates), which is acceptable for janitor legs but WRONG for this gate — it is the
     writer-exclusion interlock of a governed migration, so a missing or unparseable registry on
     a degraded install would silently let protected writers run unreserved while a coordinator
     drains and snapshots (a mid-write snapshot; safety would then rest only on the §7c step-5
     abort). The v1.4–v1.7 position (the position the verdict-only precheck already held in the
     merged code) is AFTER `Registry::load` and so inherits its fail-open arms; it is
     superseded. `main.rs` invokes `migration_writer_admission` (the reservation-returning form)
     at the registry-independent position; the observable obligation is the black-box test
     through the real dispatcher entry (EC-013, EC-022). Because both migrations share ONE
     gate-state.json file, ONE txn-record directory and ONE reservation directory under
     `.factory/migration-state/`, the protected path set enforced by the admission gate is the
     UNION `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/` (ADR-052 §Decision 5a):
     a STAGING/COMMITTING txn of EITHER migration blocks mutations under BOTH path families.
   - **Which events / tools (v1.8: events extended; admission scope anchored).** Hook events:
     `PreToolUse` (admission + reservation creation) and `PostToolUse` **and `PostToolUseFailure`**
     (reservation release; see 6(c)). Tools: `Edit`, `Write`,
     `MultiEdit` ONLY (v1.4). The Rust admission gate does NOT process `Bash`: the write-effect
     `Bash` leg (the ADR-052 §Decision 5c conservative classifier, including the coordinator
     exemption) is the separately-tracked [D-1232-OBL-4] deliverable (devops-engineer, F4
     activation) and is NOT part of S-25.06's tested surface. Until OBL-4 ships, a write-effect
     `Bash` command targeting a protected path is neither reserved nor blocked by the Rust gate;
     the backstops are the existing `^Bash$` PreToolUse guards (POL-3) and the §Decision 7c
     step-5 pre-commit fingerprint recheck (`FINGERPRINT_MISMATCH_ABORT`). **S-25.06 owes the
     regression requirement/test that the Rust gate leaves `Bash` UNPROCESSED** — for any
     `Bash` envelope (including the coordinator's own closed-grammar invocation) the gate creates
     NO reservation and returns no decision (`None`) — so the coordinator can never self-deadlock
     through it (EC-014); any future wiring of a `Bash` leg into the Rust gate must carry the
     coordinator exemption. Every other tool name, and every
     target path inside the session's `factory_root` but outside the protected union (e.g.
     `.factory/STATE.md`, `.factory/stories/`), is NOT affected by this gate. The scope of
     "protected" is fixed by the following sub-bullets (v1.8; ADR-052 §Decision 5a "Admission
     scope anchoring" and "Target path resolution"; closes F-002 and F-003):
     - **(i) Anchor.** The gate guards exactly ONE factory root per dispatch — the session's own.
       **Single anchoring rule (v1.10; ADR-052 §Decision 5a "Single anchoring rule — admission AND
       both coordinator binaries (v1.21)"; closes D-2).** `project_root` =
       `resolve_session_project_root(claude_project_dir: Option<&OsStr>, process_cwd: &Path) ->
       PathBuf`, the single pure `pub` function in the dispatcher library (`shard_manager`): a
       PRESENT non-empty `CLAUDE_PROJECT_DIR` wins, canonicalized (a canonicalize failure falls
       back to the as-given path — never to the cwd); an absent or empty value falls back to
       `process_cwd` exactly (NO ancestor walk for a `.factory`, NO `git rev-parse`);
       `main.rs::resolve_project_cwd()` is reduced to a call to it. `factory_root` =
       `resolve_factory_root(project_root)` (the existing function: the real form of
       `project_root/.factory` via `resolve_target_path`, sub-bullet (iv); v1.12: returns
       `Result<Option<FactoryRoot>, BcIndexMigrationError>` — `Ok(None)` ONLY when `.factory` is ABSENT
       per sub-bullet (iii), `Err(Io)` when the `stat` is unstatable) — the ONLY way any call site obtains a factory root.
       `factory_root` MUST exist as a directory; if it is ABSENT (the closed set of sub-bullet (iii)
       "Absent vs unstatable `.factory`" — never a non-ENOENT `stat` failure, which is
       `E-MAINTENANCE-002` `io`), the gate is
       OUT OF SCOPE for the dispatch (no migration can be in flight without it) and the gate
       NEVER creates `.factory` itself — the idempotent `create_dir_all` of 6(c) is applied only
       to `<factory_root>/migration-state/reservations`, after that check. The migration-state
       directory is ALWAYS `FactoryRoot::migration_state_dir()` = `<factory_root>/migration-state`,
       and admission, the release leg AND both coordinator binaries (`migrate-bc-index` and
       `backfill-append-logs`; Precondition 7) obtain it through this ONE function — never
       derived from the written path and never from a different anchor (v1.9 asserted that the coordinator
       binary used the same anchor; that was one-sided: the coordinator passed the
       PROCESS cwd, so whenever `CLAUDE_PROJECT_DIR` ≠ the process cwd the writer reservations
       landed in one directory while the drain and gate operated on another and the Dekker
       interlock silently guarded nothing). It is anchored on the REAL root
       regardless of which spelling of the root matched a target (v1.9; ADR-052 §Decision 5a
       "Lexical root spellings"). **Lexical root spellings (v1.9).** For the lexical comparison
       the session's own root has EXACTLY TWO lexical spellings: (a) the lexical normalization
       of the canonical `factory_root` (`factory_root_lex`), and (b) the lexical normalization
       of `<CLAUDE_PROJECT_DIR as given>/.factory` — the RAW environment value before
       `canonicalize`, ignored when empty or not an absolute path, with NO filesystem access and
       NO symlink resolution (`.`/`..`/`//` collapsed lexically), deduplicated against (a).
       Both derive from the same `CLAUDE_PROJECT_DIR` value, so each names the session's own
       project by construction; NO other spelling is ever added (none derived from the target
       `file_path`, the payload `cwd`, or an ancestor/`$HOME`). Worktrees need no special case: each
       session anchors on its own `project_root`; a worktree whose `.factory` is a symlink (or
       bind mount) to the shared factory-artifacts checkout resolves to the SAME real
       `factory_root` and so shares ONE migration-state; a worktree with its own separate
       `.factory` checkout is a separate directory with its own namespace.
     - **(ii) Classification — component-wise, never a substring.** A target is
       `BcIndex`-family iff it equals or descends from `<factory_root>/specs/behavioral-contracts`
       and `Cycles`-family iff it equals or descends from `<factory_root>/cycles` — where, for
       the lexical form `T_lex`, `<factory_root>` is EITHER lexical spelling of the session's own
       root named in (i) (v1.9) — compared
       component-by-component (case rule in (iv)). The v1.3–v1.7 substring tests
       `contains(".factory/specs/behavioral-contracts/")` / `contains(".factory/cycles/")` are
       REMOVED (they matched any `.factory/…` path anywhere on disk).
     - **(iii) Out-of-root `.factory` paths are OUT OF SCOPE.** A `.factory/…` path that is not
       under the session's `factory_root` — another project's `.factory`, a nested project's
       `project_root/sub/.factory`, a scratch tree, a look-alike such as `x.factory/cycles/…` —
       is admitted unconditionally: NO reservation, NO directory creation, NO read of any
       migration-state, and NO `migration.admission_*` event (a `tracing::debug!` developer breadcrumb
       is permitted but is not an obligation — Postcondition 10). Rationale: the interlock serializes a session
       against ITS OWN project's governed migration (the coordinator operates on
       `<project>/.factory/migration-state`); holding state for a foreign tree would mean writing
       outside the session's project, creating `migration-state/` in trees with no governed
       migration, and gating by a migration that tree's sessions — not this one — are bound to.
       **Accepted residual:** a session in project A writing into project B's protected paths is
       not serialized by B's gate; B's §Decision 7c step-5 pre-commit fingerprint recheck
       (`FINGERPRINT_MISMATCH_ABORT`) is the backstop.
       **Absent vs unstatable `.factory` (v1.12; ADR-052 §Decision 5a "Factory-root lookup mapping
       (v1.23 ruling)"; closes F-S2508-L3-009).** `factory_root` is OUT OF SCOPE only when
       `<project_root>/.factory` is ABSENT, where ABSENT is the CLOSED set: the single
       symlink-following `stat` (`std::fs::metadata`) of `<project_root>/.factory` (a) succeeds on a
       non-directory (a regular file, FIFO, socket, device, or a symlink resolving to one), (b) fails
       with `ENOENT` (this includes a DANGLING `.factory` symlink), or (c) fails with `ENOTDIR` (a
       component of the path prefix — e.g. the project root itself — is not a directory, so
       `.factory` cannot exist). A `stat` failure of ANY other kind (EACCES, EPERM, EIO, ESTALE,
       ELOOP, ENAMETOOLONG, EMFILE, ETIMEDOUT, …) leaves the existence of the directory UNKNOWN and
       FAILS CLOSED: `E-MAINTENANCE-002` `<cause>` = `io`, exactly one `migration.admission_failed`
       (`detail` = sanitized path + `ErrorKind` + message), no `migration.admission_blocked`, no
       reservation, nothing created, the write is NOT admitted. The lookup MUST NOT collapse an OS
       failure into "absent" (the former `is_ok_and(|m| m.is_dir())` collapse is removed):
       `resolve_factory_root(project_root)` returns `Result<Option<FactoryRoot>,
       BcIndexMigrationError>` — `Ok(Some(_))` Found, `Ok(None)` Absent, `Err(BcIndexMigrationError::Io
       { path: <project_root>/.factory, source })` Unstatable. A `.factory` that is a regular file is
       ABSENT (EC-035). Blast radius (decided): an unstatable `.factory` blocks every
       `Edit`/`Write`/`MultiEdit` dispatch of the session, because the root's identity is unknown and a
       path therefore cannot be classified as protected or not; this is bounded and loud (`Bash` is
       not gated by this leg, so the operator can repair the mount or permission and the next
       dispatch recovers). Mapping by leg, identical classification, leg-appropriate verdict:

       | Leg | Absent | Unstatable (`Err`) |
       |---|---|---|
       | PreToolUse admission (valid `file_path`, after the input guards) | OUT OF SCOPE: admitted, no reservation, nothing created (the `tracing::debug!` breadcrumb stays a non-obligation) | `E-MAINTENANCE-002` (`io`), fail closed, one `migration.admission_failed` (Postcondition 10(b)) |
       | PostToolUse / PostToolUseFailure release | silent no-op | NO verdict (release is never a verdict): one `migration.admission_advisory` `reservation_release_failed` (Postcondition 10(c)); nothing created or deleted; a leaked reservation is reclaimed by the drain-start TTL GC |
       | Coordinators (`migrate-bc-index`, `backfill-append-logs`; Precondition 7) | exit 2 `FACTORY_ROOT_NOT_FOUND` (unchanged) | exit 2 with the existing `BcIndexMigrationError::Io { path, source }`, NOT `FACTORY_ROOT_NOT_FOUND`; nothing created or mutated |

       One function, one classification (the D-2 single anchoring rule): the three legs cannot
       disagree about what "no `.factory`" means. Vectors: EC-035, EC-041..EC-043.
     - **(iv) Target path resolution (shared function `resolve_target_path` in
       `shard_manager.rs`, returning the pair `(T_real, T_lex)`; the OBL-4 Bash write-effect
       classifier MUST use the same function for every extracted write-target).** (a) *Input
       guards:* a missing, non-string, empty or NUL-containing `file_path` is out of scope (not
       an error); a relative `file_path` (defensive) is joined onto the payload's `cwd` when that
       is an absolute path, else onto `project_root`; `~` is never expanded. (b) *Separators:* on
       Windows targets `\` is a separator and `Path::components` handles drive/UNC prefixes; on
       Unix `\` is an ordinary name byte (the unconditional `replace('\\','/')` is removed). (c)
       *Lexical form `T_lex`:* collapse `//`, drop `.`, apply `..` lexically; no filesystem
       access. (d) *Real form `T_real` (POSIX-correct — plain lexical normalization BEFORE
       resolution is wrong because `a/link/../b` resolves `link` first):* walk components left to
       right keeping a symlink-free `resolved`: `.` and empty components are skipped; `..` pops
       `resolved` (never above the root); a normal component is `lstat`ed — a symlink is read and
       its target spliced in front of the remaining components (an absolute target resets
       `resolved` to the root), bounded to 40 hops; an existing non-symlink is pushed; on the
       first `NotFound` that component and ALL remaining components are applied lexically without
       further `lstat` (the deepest existing ancestor is realpath'd and the unresolved tail
       appended). Any non-`NotFound` error (EACCES, ELOOP, ENOTDIR, hop limit exceeded) makes
       `T_real` unavailable and classification proceeds on `T_lex` alone — a protected-looking
       path that cannot be resolved is treated as protected (fail-closed). (e) *Either-form
       match:* a write is in scope iff it matches under EITHER the resolved comparison
       (`T_real` vs `factory_root_real`) OR the lexical comparison (`T_lex` vs the lexical
       normalization of EITHER spelling of the session's own root — (a) canonical
       `factory_root_lex`, (b) `<CLAUDE_PROJECT_DIR as given>/.factory` — and no other
       spelling; v1.9, per (i)); the union is deliberately fail-closed (its only cost is a spurious
       reservation, or a spurious block while a window is active, for a path that merely looks
       protected through a symlink pointing out of the tree). (f) *Case:* ALL path-component
       comparisons — the `factory_root` prefix AND the `specs`/`behavioral-contracts`/`cycles`
       family names — are ALWAYS case-insensitive (ASCII case-fold for ASCII, `str::to_lowercase`
       equality for other UTF-8 components, ASCII-fold bytewise for non-UTF-8); there is NO
       filesystem case-sensitivity probe (sensitivity is a per-volume property, a probe is racy,
       and on a case-insensitive volume a probe-guided exact compare would let
       `.FACTORY/Cycles/x` bypass the gate); always-fold only over-matches on a case-sensitive
       volume (a spurious reservation, released at the Post event, or a spurious block during an
       active window). Unicode NFC/NFD folding is NOT applied (family names are ASCII; the
       `factory_root` prefix is compared realpath-form to realpath-form).
     - **(v) Residuals (accepted; same-user local consistency interlock, not an authorization
       boundary):** hard links into a protected file from outside the tree, bind mounts not
       visible to `lstat`, and a symlink swapped between admission and the tool's actual write
       (TOCTOU); all backstopped by the §Decision 7c step-5 fingerprint recheck.
   - **Decision.** Admit iff `gate_state = OPEN` AND no txn record in state STAGING or
     COMMITTING exists (dual check, ADR-052 §Decision 5a step 4/5; PID liveness and flock
     ownership are irrelevant to the decision). Otherwise block with `E-MAINTENANCE-001`, exit 2
     at the PreToolUse hook-block surface, with the message defined by the **`<scope>` keying
     rule (v1.6)** immediately below. The step-0 reservation is created first; the ADR-052
     §Decision 5a step-3.5 reconciliation (plus Postcondition 5a(c) below) then runs and may
     flip the gate to OPEN, after which the decision is re-evaluated (reserve-then-verify,
     6(c)). **Verdict surface label (v1.10; ADR-052 §Decision 5a "Admission verdict surface
     label (v1.21)"; closes F-013).** An admission verdict (`E-MAINTENANCE-001` block or
     `E-MAINTENANCE-002` error) is reported to the operator as `blocking_plugins=migration-admission`
     on the dispatcher's stderr summary line (constant `MIGRATION_ADMISSION_GATE_NAME`; `block_reason`
     = the unchanged `E-MAINTENANCE-001`/`E-MAINTENANCE-002` text; exit 2); a shard-cap-gate verdict
     keeps `blocking_plugins=shard-cap-gate`. Shape: `shard_gate_verdict_outcomes` and the two
     outcome synthesizers (`shard_gate_block_outcome`, `shard_gate_error_outcome`) take the gate
     name as a parameter — the closed enum `NativeGate { ShardCap, MigrationAdmission }` with
     `fn plugin_name(self) -> &'static str` (no free-form strings); `main.rs`'s admission leg
     passes `MigrationAdmission`, the `shard_cap_precheck` leg keeps `ShardCap`. The message text
     and the exit code are unchanged (EC-040).
   - **`E-MAINTENANCE-001` `<scope>` keying rule and exact message (v1.6; resolves the
     path-vs-migration keying ambiguity; this BC, BC-1.18.011 Precondition 6(b)/(d) and
     error-taxonomy v1.35 state the SAME rule).** `<scope>` is keyed on the WRITTEN PATH
     FAMILY, never on the migration that owns the live txn. Rationale: the admission core is one
     shared core over a shared protected-path union, and it also blocks when NO txn exists
     (gate `DRAINING`/`LOCKED` only, e.g. the drain window before step 3a, or a live
     coordinator mid-recovery), where no `migration_id` is available to key on; a path-keyed
     scope is a total function of the dispatch, a migration-keyed scope is not. The live txn's
     `migration_id` (when one exists) travels only in the structured `migration.admission_blocked`
     InternalLog event (field `migration_id`; Postcondition 10), never in the message. The scope token is exactly:
     a target path under `.factory/specs/behavioral-contracts/` ⇒ `BC-INDEX`; a target path
     under `.factory/cycles/` ⇒ `.factory/cycles/`. The block message is exactly the format
     string
     `<scope> write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts`
     with `<scope>` substituted and nothing else substituted; the fixed parenthetical is
     emitted verbatim even when the block is gate-only with no txn (it names the two txn states
     that govern the window, not a claim that a txn file exists). All four combinations:

     | Written path family | Live migration (txn `migration_id`) | Exact block message |
     |---|---|---|
     | `.factory/specs/behavioral-contracts/` | `migrate-bc-index` (or absent field) | `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` |
     | `.factory/specs/behavioral-contracts/` | `backfill-append-logs` | `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` |
     | `.factory/cycles/` | `backfill-append-logs` | `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` |
     | `.factory/cycles/` | `migrate-bc-index` (or absent field) | `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` |

     A write to `.factory/cycles/` while a B2 txn is live (or to `behavioral-contracts/` while a
     mechanism-A txn is live) therefore produces exactly ONE message — the row above for its
     path family — and never a second or merged message. **Mismatch suffix (v1.6):** when the
     block is the PreToolUse Branch C verification-failure analogue (Postcondition 5a, EC-010;
     STAGING + terminal record, or COMMITTING with a failed verification — for EITHER
     migration, and for EITHER path family), the message is the format string above followed by
     exactly one space and the suffix
     `(completion-record mismatch — operator investigation required)` (em dash U+2014); the
     `<scope>` token is still the path-family token from the table, and no other text is added.
     A foreign-migration live txn (Postcondition 5a foreign-migration guard) and a live
     coordinator (EWOULDBLOCK) produce the PLAIN message with no suffix. **Definition of
     "plain" (v1.9; normative for every use of the word in this BC, BC-1.18.011 and
     error-taxonomy):** "plain `E-MAINTENANCE-001`" = the keyed format string WITHOUT the
     mismatch suffix. The suffix-less cases are exactly: gate-only block (no txn); live txn with
     its terminal record ABSENT (`NoOp`); foreign-migration refusal; live coordinator
     (EWOULDBLOCK). **Terminal record present but unverifiable (v1.9 ruling):** a terminal
     record that was READ successfully but does not verify — unparseable / non-UTF-8 /
     empty-truncated, schema-mismatched or wrong-shaped, `canonical_paths_count ≠ 4`,
     mismatching `txn_id`/`activation_id`/`generation_id`/hash, or STAGING + terminal record —
     is a Branch C verification failure and carries the mismatch suffix (an operator MUST be
     told a corrupt or mismatching completion record exists); it is still NOT
     `E-MAINTENANCE-002`, and the txn is not finalized. A terminal-record read CALL that fails
     (non-ENOENT OS error) remains `E-MAINTENANCE-002 (io)`; ENOENT is "absent" (plain).
   - **Exemption (no self-deadlock) — re-scoped in v1.4.** The migration binary's own
     closed-grammar invocation (`backfill-append-logs` / `backfill-append-logs --census`,
     Precondition 3, ADR-052 §Decision 5c Branches 1–4) is the coordinator, not a writer: it
     creates NO writer reservation and is not blocked by the admission gate. (Were it to
     reserve, the coordinator's own drain — 6(c) — would wait on its own reservation and always
     end in `DRAIN_TIMEOUT_ABORT`.) The exemption is keyed EXCLUSIVELY on the §Decision 5c
     classifier's Branch 1–4 verdict (exact command-string match, `realpath()`-resolved absolute
     binary path, metacharacter rejection, executable-digest verification) — never on a
     substring/prefix match; a compound command fails classification and is treated as a writer.
     It is IMPLEMENTED in the OBL-4 classifier, not in the Rust gate: because the Rust gate does
     not process `Bash` (Tools bullet above), the coordinator can never reach the Rust
     reservation path, and S-25.06's obligation is the regression test of EC-014. The
     coordinator's own filesystem writes are made by the binary through `std::fs`, not through
     hook-mediated tool calls, so they are never subject to the gate.

   (c) **OPEN/DRAINING gate with writer reservations — precise protocol (v1.3; closes
   S-25.06 formal-verification defect D2; restates ADR-052 §Decision 5a normatively for this
   BC).** Gate states persist at `.factory/migration-state/gate-state.json` (`OPEN` | `DRAINING` |
   `LOCKED`, atomic replace-via-rename; the dispatcher is a per-event binary, so all gate and
   reservation state is durable on disk, never in-process).
   - **Who creates a reservation, and when — RESERVE-THEN-VERIFY (v1.4; ADR-052 §Decision 5a
     step 0; closes D5; supersedes the v1.3 "atomically under `LOCK_SH` on the gate-state.json
     file" wording, which was unsound).** The dispatcher's PreToolUse admission (6(b)) for every
     protected-path `Edit`/`Write`/`MultiEdit` MUST create
     `.factory/migration-state/reservations/<tool_use_id>.reservation` FIRST (atomic temp+rename;
     content `{"created_at": "<ISO-8601>", "tool_use_id": "<id>"}`, no PID, no start-time), where
     `tool_use_id` is the harness identifier shared by the PreToolUse and its completion event
     (PostToolUse, or PostToolUseFailure for a failed call) of the same tool call, and ONLY THEN read `gate-state.json`, scan `txn-*.json` and run the §Decision 5a
     step-3.5 reconciliation. If verification fails (gate ≠ OPEN, or a live txn), the admitter
     MUST remove its own just-created reservation before returning the `E-MAINTENANCE-001` block.
     The admitter holds NO lock (parallel admissions never block each other). **Why it is
     race-free (Dekker ordering; one local filesystem, create/rename/readdir mutually
     coherent):** the coordinator performs C1 = durable `gate_state=DRAINING` flip then C2 = read
     `reservations/`; the admitter performs W1 = create reservation then W2 = read gate. If W2
     sees `OPEN`, then W1 < W2 < C1 < C2 so the coordinator's C2 sees the reservation and waits;
     if C2 sees an empty directory then W1 follows C2, so W2 follows C1 and sees DRAINING and
     backs off — never both miss. (`flock` on the gate file is NOT relied on for admitter
     ordering: the coordinator's gate flip replaces the file via `rename`, and a lock on the
     replaced inode does not serialize against an admitter holding the old inode. The
     `LOCK_SH`/`LOCK_EX` on `gate-state.json` are retained only as coordinator-vs-reconciler
     mutual exclusion between writers of the gate FILE, all of whom take `exclusive.lock`
     first.) No reservation survives a blocked admission: `active_writer_count` = number of
     files in `reservations/`; quiescence = directory empty.
   - **Release on `PostToolUse` OR `PostToolUseFailure` (v1.8; ADR-052 §Decision 5a release
     events; closes F-001; replaces "(success or failure of the tool)").** The reservation
     `reservations/<tool_use_id>.reservation` created by the PreToolUse admission is removed on
     `PostToolUse` (the tool call succeeded) OR `PostToolUseFailure` (the tool call failed, was
     interrupted or was cancelled) for the SAME `tool_use_id`. Claude Code delivers a failed tool
     call as `PostToolUseFailure` and does NOT fire `PostToolUse` for it; the former wording named
     an event the harness never sends for the failure leg and leaked every failed protected
     write's reservation until the TTL. The `PostToolUseFailure` stdin envelope carries the common
     fields plus top-level `tool_name`, `tool_input`, `tool_use_id` (the same value as the paired
     PreToolUse), `error`, `is_interrupt`; the dispatcher's `HookPayload` captures `tool_use_id`
     in `payload.extra` on every event. The release is keyed ONLY on `tool_use_id` and applies NO
     `tool_name` filter (a failure envelope whose `tool_name` is absent or differently shaped must
     still release; the unlink is idempotent and a non-protected tool's id has no reservation
     file, costing one `unlink` returning ENOENT). It is gated by the single pure predicate
     `is_tool_completion_event(event_name: &str) -> bool` (true iff the already-aliased
     `payload.event_name` is exactly `"PostToolUse"` or `"PostToolUseFailure"`) in
     `crates/factory-dispatcher/src/invoke.rs`; NO new `EventType` variant is added
     (`EventType::from_event_str("PostToolUseFailure")` stays `Other`; the release MUST NOT be
     conditioned on `EventType::PostToolUse` alone). A missing reservation file at release (writer
     crashed between Pre and Post, never created, or already released) is a no-op (normal); a
     `tool_use_id` that fails the grammar below can never have created a reservation and is a
     silent no-op on release; a release error other than ENOENT is non-fatal (never a verdict)
     and is recorded as one `migration.admission_advisory` InternalLog event with `reason =
     reservation_release_failed` (the release leg returns the diagnostic as data; `main.rs`
     writes it — Postcondition 10). The release runs at the registry-independent position (6(b) "Where"). **If
     NEITHER Post event arrives** (harness crash, session killed, hook timeout, a user denial of
     the permission prompt, a block by a hook process other than this dispatcher), the reservation
     leaks until `MAX_RESERVATION_TTL` elapses and drain step 1 GC reclaims it by `created_at`
     (TTL-only, no PID liveness); before that the sanctioned operator remediation (manual deletion
     of a confirmed-stale `*.reservation`) applies — the designed, sole residual of this leg.
     S-25.08 MUST add a real-binary `PostToolUseFailure` fixture (no in-repo fixture currently
     carries a `PostToolUseFailure` envelope with `tool_use_id`); if a captured real payload ever
     lacks the id, the TTL backstop is the only recovery and the ruling must be re-opened.
   - **Reservation timestamp rules (v1.8; ADR-052 §Decision 5a "Reservation timestamp rules";
     closes F-008).** Staleness is computed by the ONE pure predicate
     `reservation_is_stale(created_at: Option<u64>, mtime: Option<u64>, now: u64, ttl: u64) ->
     bool` (epoch seconds; `mtime` is an `Option` because it can be unavailable), with constant
     `RESERVATION_CLOCK_SKEW_TOLERANCE_SECS = 300` (5 min). Rules, in order:
     (1) `created_at` is parsed as RFC 3339 (any UTC offset, normalised to UTC epoch seconds,
     sub-second truncated); a parse failure, a value before 1970-01-01T00:00:00Z (negative epoch)
     or a value outside `u64` is UNPARSEABLE ⇒ `None` — NEVER clamped (clamping a pre-epoch stamp
     to 0 would make a live writer's reservation maximally old and reclaimable);
     (2) `created_at > now + 300` (a FUTURE stamp beyond the skew tolerance) is UNTRUSTED ⇒
     `None`; a stamp in `(now, now + 300]` is accepted as ordinary skew and ages as 0;
     (3) `basis = created_at.or(mtime)`; if both are `None` (mtime unavailable, including a
     pre-epoch mtime) the age is UNKNOWN and the reservation is NOT stale (the merged
     `epoch_secs(pre-epoch) = 0 ⇒ reclaimable` behaviour is REMOVED);
     (4) `age = now.saturating_sub(basis)`; stale iff `age > ttl`; a future `mtime` therefore also
     ages as 0;
     (5) every fallback or unknown-age outcome emits one coordinator-drain STDERR advisory line
     whose token ∈ {`created_at_unparseable`, `created_at_pre_epoch`, `created_at_future`,
     `mtime_future`, `age_unknown`} (v1.10, corrected in the v1.10 follow-up; ADR-052 §Decision 5a "Reservation
     timestamp rules" (5) and "Admission diagnostics channel" (5), §Downstream item 33(c)). These
     five are NOT `migration.admission_advisory` reasons: only the drain GC calls
     `reservation_is_stale` (admission never reads reservation files), so they have no dispatcher
     event.
     **Direction rationale:** wrongfully RECLAIMING a live writer's reservation lets the
     coordinator snapshot mid-write (an integrity hazard); wrongfully RETAINING one yields a
     bounded, visible stall (`DRAIN_TIMEOUT_ABORT`, gate returned to OPEN) and the manual-deletion
     remediation — so every ambiguous timestamp resolves toward retention; clamping a future stamp
     to `now` per evaluation (age 0 forever) and clamping a pre-epoch stamp to 0 are both
     rejected. A forged far-future `created_at` falls back to the file mtime, which the host
     filesystem sets at creation; forging both requires write access to `migration-state/`, which
     already implies the ability to delete the txn record or rewrite `gate-state.json`.
   - **`E-MAINTENANCE-002` `<cause>` classification — total decision rule (v1.8; ADR-052
     §Error Code Semantics v1.20; closes the S-25.08 AC-018 ambiguity).** Every way the
     writer-admission check can fail to complete maps to EXACTLY ONE `<cause>`; the three
     causes partition the failure space by WHAT failed, evaluated in this order and stopping at
     the FIRST failure (no aggregation; evaluation order: the `tool_use_id` check, then
     `reservations/` creation and the reservation create (W1), then `gate-state.json`, then the
     `txn-*.json` records in ascending filename order (W2), then — only on the Branch C
     verification path — the terminal record):
     1. `invalid_tool_use_id` — payload-only. A PRESENT `tool_use_id` that is not a string, is
        empty, or violates `[A-Za-z0-9_.-]{1,128}` (no leading `.`). Decided from the payload
        alone, BEFORE any filesystem access.
     2. `io` — an OS-level filesystem call made by the admission check returned an error:
        the `stat` of `<project_root>/.factory` (v1.12), directory create, reservation
        create/rename/write, or `open`/`read`/`readdir`/`stat` of
        the migration-state directory, `gate-state.json`, a `txn-*.json` record, the terminal
        record or — on the Branch C verification path — a canonical file (v1.11; ADR-052 v1.22
        "Read-failure mapping"), failing with anything other than `ENOENT` on the FILE ITSELF (EACCES, EPERM,
        EROFS, ENOSPC, EIO, EISDIR — e.g. the path is a directory —, ELOOP, EMFILE, a short or
        interrupted read, ...). **v1.12 (ADR-052 v1.23 "Factory-root stat failure"; closes
        F-S2508-L3-009):** the FIRST OS call of the check is the `stat` of
        `<project_root>/.factory`; a failure other than the closed ABSENT set (`ENOENT` including a
        dangling symlink, `ENOTDIR`, or success on a non-directory; 6(b)(iii)) is `io` on the
        admission leg — same event shape as every other `io` failure, no tenth cause, no new code.
        The content is NOT examined once a call has failed: an
        unreadable file is `io` even if its bytes would also have been malformed. `ENOENT` of
        `gate-state.json`, of the txn set or of a terminal record is NOT an error (absent
        semantics: `OPEN` / no live txn / no terminal record); `ENOENT` of a canonical file
        during Branch C verification is NOT `io` either — it is the `canonical_hash_mismatch`
        check (Postcondition 10(a) domain item (7)). A failed terminal-record or canonical-file
        read CALL is never a Branch C `check` token and never `E-MAINTENANCE-001`: exactly one
        `migration.admission_failed` (`cause=io`), no `migration.admission_blocked`, no
        `_advisory`, no txn/gate write.
     3. `state_integrity` — every byte was read successfully but the content cannot be
        accepted: not valid UTF-8; empty (zero-length), truncated or otherwise unparseable JSON;
        wrong JSON type or shape for the file (`gate-state.json` must be exactly one JSON
        string — a bare `"OPEN"`/`"DRAINING"`/`"LOCKED"`, per `BcIndexAdmissionGateState`; an
        object, number, `null`, unquoted token or any other string value, including wrong
        casing, is malformed; a txn record must be a JSON object whose `state` is a known txn
        state and whose PRESENT `migration_id` is a JSON string — an ABSENT `migration_id`
        defaults to `migrate-bc-index` and is NOT this cause); or a cross-file inconsistency
        (more than one live txn record). "Unreadable" in ADR-052 §Error Code Semantics'
        wording of this cause means "readable as bytes but not interpretable as the record";
        a failed read CALL is `io` (rule 2), which the same ADR row lists as "cannot ... read
        gate/txn".
        **Carrier (v1.10; ADR-052 §Decision 5a "Admission state-integrity variant (v1.21)";
        closes F-012).** `state_integrity` is the dedicated variant
        `BcIndexMigrationError::AdmissionStateIntegrity { kind: AdmissionStateIntegrityKind,
        detail: String }` — it is NOT `BinaryIntegrityFailure` (whose Display is the digest/TOCTOU
        code `BINARY_INTEGRITY_FAILURE`). `kind` is the closed set {`gate_record_malformed`,
        `txn_record_malformed`, `txn_migration_id_not_string`, `multiple_live_txns`,
        `reservation_serialization`, `txn_record_newer_schema`} (SIX tokens; v1.12 third binary-leg
        extension, additive token — BC-3.08.001 v1.37 Event 12): `txn_record_newer_schema` = a txn
        record whose `schema_version` is an integer ≥ 2 (written by a newer build; the follow-up is to
        recover it with that build, not to investigate corruption); `gate_record_malformed` = `gate-state.json` content unusable;
        `txn_record_malformed` = a txn record that is unparseable, not a JSON object (including in
        `abort_null_generation_txn`) or has an unknown `state`; `txn_migration_id_not_string` = a
        PRESENT non-string `migration_id`; `multiple_live_txns` = more than one live txn;
        `reservation_serialization` = the reservation record could not be serialized. Display:
        `migration admission: state integrity failure (<kind-token>): <detail>` — it MUST NOT contain
        `BINARY_INTEGRITY_FAILURE`; `process_exit_code` = 2; `detail` is the already-sanitized
        path/parse message, never record content. The shared loaders are also used by the
        coordinator, where the same variant surfaces as exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`
        (no coordinator behavior change besides the message token). The classification function
        `admission_failure_cause` returns the closed `AdmissionFailureCause { InvalidToolUseId, Io,
        StateIntegrity }` (`fn token(self) -> &'static str`) and is an EXHAUSTIVE match (Invariant 7).
        **Txn-record interpretation — two tiers (v1.12; ADR-052 §Error Code Semantics "Txn-record
        interpretation — tiers (v1.23 ruling)"; closes F-S2508-L3-001).** The shape rule of rule 3
        ("a txn record must be a JSON object whose `state` is a known txn state and whose PRESENT
        `migration_id` is a JSON string") is **Tier 0** and is the ONLY record interpretation applied
        at read time, to EVERY `txn-*.json` (ascending filename order, first failure wins), including
        a record that no executing branch consumes. Admission MUST NOT deserialize a `txn-*.json`
        into the full `BcIndexMigrationTxnRecord` schema at read time (whether a record is
        interpretable must not depend on a schema version the reader may not share — that would turn
        a foreign, newer-build record into `state_integrity` instead of the foreign refusal that
        EC-029 gives precedence over every record check). The Tier 0 read yields the raw JSON object,
        the parsed `state`, the effective `migration_id` (absent ⇒ `migrate-bc-index`) and the path,
        and nothing else. Every other field is **Tier 1**: read LAZILY, from the raw object, by the
        branch that consumes it, ONLY when that branch executes (Branch B: `schema_version` FIRST
        (rule 9), then `generation_id`; Branch C verification: `schema_version` FIRST (rule 9), then
        `activation_id`, `generation_id`, `intent_log_path` — v1.12 extension; the hash source is the intent log, see item (6)). Normatively:
        (1) a PRESENT `generation_id` that is JSON `null` is the null generation; a string is not;
        an ABSENT `generation_id` key, or any other JSON type, is `E-MAINTENANCE-002`
        `state_integrity` (kind `txn_record_malformed`, `BcIndexMigrationError::
        AdmissionStateIntegrity`) with NO txn and NO gate write — **an absent key is NEVER read as
        null** (serde's `Option` default must not turn a hand-edited or truncated-but-parseable
        record into a Branch B discard, a mutation on an unproven precondition); (2) this
        `generation_id` check applies ONLY under the exact Branch B execution conditions (live known
        txn, STAGING, `exclusive.lock` acquired, own terminal record ABSENT — the planner's
        `NoOp`/STAGING arm), resolved as a tri-state (null / non-null / unusable) by the shell BEFORE
        the unchanged planner `plan_stale_gate_reconciliation` (VP-147 is unchanged); (3) a
        shape-valid live record that no consuming branch reaches blocks with PLAIN
        `E-MAINTENANCE-001` (the diagnostic `txn_id` is the field if a string, else the literal
        `unknown`; informational, never a verdict input); (4) a FOREIGN record (`migration_id ∉ K`)
        is refused plain with no field beyond rule 3 read (foreign precedence); (5) Branch A reads no
        field; (6) on the Branch C verification path an absent or ill-typed Tier 1 field is
        `state_integrity` evaluated AT that check's position in the fixed first-failure-wins order
        (an earlier check's mismatch wins; `staging_with_terminal_record` reads none). **Hash source
        (v1.12 extension; ADR-052 §Error Code Semantics "Branch C hash source"):** the
        `canonical_hash_mismatch` check reads the txn's Tier 1 `intent_log_path` (absent, null or
        non-string at that check ⇒ `state_integrity`, `txn_record_malformed`) and, per canonical
        path, the intent log's DONE record read with the shared ADR-054 §Decision 1.7 reader (v1.13;
        a torn tail is ABSENT; a checksum-failed record that is followed by a later valid record is
        mid-log corruption, below); `expected_post_hash` is a field of the intent-log record,
        NOT of `canonical_move_plan` (v1.13 rename; the plan is `{staging_path, canonical_path}` only).
        **Hash source, v1.13 (ADR-054 §Decision 3 B-2):** the DONE's `expected_post_hash` is REQUIRED EQUAL
        to the latest INTENT's `expected_post_hash` for that path AND to the file's `sha256`
        (`DONE == INTENT == sha256(file)`). A
        missing intent log, a canonical path with no DONE record, a DONE `expected_post_hash` that
        differs from the INTENT's, or one that the file's `sha256` does not equal, mid-log corruption
        or a §Decision 1.8 invariant (L1-L4) violation of the log (ADR-054 §Decision 1.7 surface-mapping
        table), and a `canonical_move_plan` whose set of `canonical_path` values differs from the
        distinct `target_canonical` set of the INTENT records (ADR-054 §Decision 3 B-3(b)), are all
        `canonical_hash_mismatch` (the unverifiable-hash class, NOT `state_integrity`); a plan that is
        absent, not an array or EMPTY is `state_integrity` `txn_record_malformed` at this check's
        position (Tier 1); an intent-log read failure other than `ENOENT` is `io` on
        the admission leg and `BcIndexMigrationError::Io` (exit 2) on the binary leg. **Binary leg
        (v1.12 extension; ADR-052 §Error Code Semantics "Migration binaries — recovery and finalize
        legs" item 6):** `backfill-append-logs` (verifier and finalize) and the `migrate-bc-index`
        §4e reconciliation apply the SAME nine-token order and the SAME Tier 1 reads; a malformed
        Tier 1 txn field at its check is `MIGRATION_STATE_INTEGRITY_FAILURE` (exit 2), a
        well-formed disagreeing field is `COMPLETION_RECORD_MISMATCH_ABORT` (exit 2), an earlier
        check wins over a later field's malformation, and neither finalizes nor flips (nothing
        mutated); (7)
        non-live records (COMPLETED / ABORTED), foreign or known, with a valid Tier 0 shape are
        ADMITTED — either gate OPEN with no live txn, or Branch A reopens a stuck gate — and are
        never modified or deleted; (8) the "more than one live txn" `state_integrity` check counts
        LIVE records only (STAGING / COMMITTING), foreign included; (9) **Version gate (v1.12 third
        binary-leg extension; ADR-052 item 10(b)/(c)/(d)) — `schema_version` is the FIRST Tier 1
        field read at EVERY Tier 1 consumer, before any other field, nested element or unknown key is
        examined, and before the Branch B `generation_id` tri-state.** At Branch B (the
        null-generation discard, via the shared `abort_null_generation_txn`) and at the Branch C
        verification (and the binary-leg rewriting arms of BC-1.18.011 Precondition 6(f)): (i) key
        PRESENT and a JSON integer ≥ 2 (any magnitude, e.g. `4294967296`) ⇒ `E-MAINTENANCE-002`
        `state_integrity`, kind `txn_record_newer_schema`
        (`BcIndexMigrationError::AdmissionStateIntegrity`), exit 2, exactly ONE
        `migration.admission_failed` (`cause=state_integrity`, `kind=txn_record_newer_schema`), NO
        `migration.admission_blocked`, no reservation left behind, NO txn and NO gate write — the txn
        bytes are UNCHANGED and the record is NOT discarded; (ii) PRESENT and integer `1` ⇒ proceed;
        (iii) PRESENT and anything else (`0`, negative, float, string, `null`, array, object) ⇒ kind
        `txn_record_malformed`; (iv) ABSENT ⇒ "not consumed, proceed" at Branch B and at the lazy
        Branch C reads (a minimal hand-built `{state, migration_id, generation_id}` record stays
        admissible; on the BINARY rewriting arms an absent `schema_version` is `txn_record_malformed`,
        BC-1.18.011 Precondition 6(f)(iii)). The newer-schema record wins over every other defect on
        the same record, and the gate does not alter Tier 0, the foreign refusal (still refused as
        foreign before ANY field is read), the one-live-txn check or non-live record handling. The
        discard otherwise PRESERVES unknown top-level keys (`schema_version` absent or `1` + an
        unknown extra key ⇒ the discard succeeds and the key is kept in the ABORTED record). The
        normative Display of kind `txn_record_newer_schema` is the one in BC-1.18.011 Precondition
        6(f)(iii) ("written by a newer build; recover it with that build"); it never contains
        `txn_record_malformed`, `corrupt` or `BINARY_INTEGRITY_FAILURE`; the `<cause>` domain
        {`invalid_tool_use_id`, `io`, `state_integrity`} is UNCHANGED. Vectors: EC-044..EC-048,
        EC-055, EC-056.
     **Scope boundaries (the rule is total over every file the check touches).** (a)
     *Reservation files*: the admission check only CREATES them (failure ⇒ `io`) and never
     reads one; reading/parsing a reservation file is the binary drain path (step-1 GC; EC-019
     / EC-025 timestamp rules), a process-exit-code surface, and is NOT an `E-MAINTENANCE-002`
     case. (b) *Terminal record* (`completed.json` / `completed-backfill-append-logs.json`),
     read only to decide Branch C: a failed read call ⇒ `io`; ENOENT ⇒ absent; bytes read but
     unparseable / schema-mismatched / `canonical_paths_count ≠ 4` / mismatching
     `txn_id`/`activation_id`/`generation_id`/hashes ⇒ the record simply DOES NOT VERIFY (v1.9:
     `E-MAINTENANCE-001` block WITH the ` (completion-record mismatch — operator investigation
     required)` suffix, txn not finalized — EC-010/EC-032; "plain" = no suffix, defined in
     6(b)); it is NOT
     `E-MAINTENANCE-002`, because an unverifiable terminal record is an expected recovery
     input, not a failure of the admission check itself. (c) *Gate state and txn record*: both
     `io` and `state_integrity` are fail-closed `E-MAINTENANCE-002` — no reservation left
     behind, no txn/gate write, no Branch A/B/C repair attempted. The single-line message
     carries only the cause token; the path, `io::Error` kind/`errno` or serde error position
     go to the `migration.admission_failed` InternalLog event (field `detail`; Postcondition 10),
     never record content. An `io`/`state_integrity` failure is never
     reclassified as "foreign" or as a suffix-less (plain) `E-MAINTENANCE-001` block. Vectors: EC-032.
   - **`tool_use_id` presence and validity (v1.8; ADR-052 §Decision 5a; closes F-009 part 2).**
     Grammar: `[A-Za-z0-9_.-]{1,128}`, not starting with `.` (the harness form is
     `toolu_<alphanumerics>`). An ABSENT key or JSON `null` ⇒ check-only admission (the unchanged
     degradation: no directory, no reservation, backstopped by the §Decision 7c step-5 recheck). A
     key that is PRESENT but is not a string, is empty, or violates the grammar ⇒
     `BcIndexMigrationError::InvalidToolUseId { len }`: the admission FAILS CLOSED with
     `E-MAINTENANCE-002` (`invalid_tool_use_id`), NO reservation created, nothing admitted — never
     silently downgraded to check-only (a harness id this core cannot key would otherwise turn
     every protected write into an untracked admit with no signal). The error carries the byte
     length only, never the raw value (log-injection / path-traversal hygiene). The release leg
     treats an invalid id as a silent no-op.
   - **Unconditional reservation namespace — first-activation race closed (v1.7; S-25.08
     implementation finding; ADR-052 §Decision 5a step 0 as clarified by the ADR delta in this
     BC's v1.7 changelog row).** The admitter MUST NOT condition admission, reservation or
     release on the PRE-EXISTENCE of `.factory/migration-state/` (or of its `reservations/`
     subdirectory). Normatively: (i) for every PreToolUse `Edit`/`Write`/`MultiEdit` whose target
     is inside the protected union of the session's `factory_root` (6(b) sub-bullets (i)–(iv))
     and which carries a valid `tool_use_id` (v1.8: grammar and fail-closed rule in the
     "`tool_use_id` presence and validity" bullet), the admitter first
     ensures `.factory/migration-state/reservations/` exists by an IDEMPOTENT recursive
     directory creation (an already-existing directory, including one created concurrently by
     another admitter or by the coordinator, is success, never an error — no existence
     pre-check, no lock), THEN performs W1 (create the reservation), THEN W2 (read gate/txn);
     (ii) an absent `gate-state.json` is read as `OPEN` and an absent/empty txn set as "no live
     txn", so on a repository that has never run a migration the verification admits and the
     reservation STANDS until PostToolUse; (iii) the former "migration-state directory absent ⇒
     zero-cost no-op, no reservation" bypass is REMOVED — the no-op scope is exactly: not
     PreToolUse; tool not `Edit`/`Write`/`MultiEdit`; target outside the protected union (e.g.
     `.factory/STATE.md`) or outside the session's `factory_root` (v1.8, 6(b)(iii)); and the
     existing degradation for a payload with an ABSENT or `null` `tool_use_id` (check-only, no
     reservation, no directory creation — backstopped by the §Decision 7c step-5 fingerprint
     recheck, unchanged); (iv) the coordinator's own directory creation is
     likewise idempotent, and the release (PostToolUse / PostToolUseFailure) remains a
     best-effort no-op when the directory or the file is absent. **Why (closes the first-activation window):** under the
     former bypass a protected write admitted before the directory first existed created NO
     reservation, so a coordinator that then created the directory, flipped DRAINING and polled
     `reservations/` observed quiescence while that write was still in flight, and the snapshot
     could race it (safety held only via the step-5 fingerprint abort — a spurious liveness
     abort). With (i)–(iii) the Dekker argument above holds from the very first protected write
     ever, because the reservation namespace exists before W1 on every admission path: every
     admitted writer is either listed by C2 or observes DRAINING at W2. **Failure semantics:**
     if the directory or reservation cannot be created (e.g. EACCES/EROFS/ENOSPC) the admission
     returns an error and the dispatcher fails the PreToolUse closed with `E-MAINTENANCE-002`
     (v1.8; message `E-MAINTENANCE-002: writer-admission check failed (<cause>)`, here
     `<cause>` = `io`; `<cause>` ∈ {`invalid_tool_use_id`, `io`, `state_integrity`};
     `HookResult::Error`, exit 2 at the PreToolUse hook surface; it replaces the unnamed
     `BC-1.18.011: writer-admission check failed: {e}` string; the underlying detail — path,
     `io::Error`, byte length, never a raw id or record content — goes to the
     `migration.admission_failed` InternalLog event, Postcondition 10)
     with no reservation left behind — never an
     untracked admit. The protected path is itself under `.factory/`, so a `.factory/` tree too
     unwritable to hold `migration-state/` could not accept the protected write either; no new
     operator-visible failure class is introduced. **Cost (accepted, not an MVP deferral):**
     one idempotent directory-creation call (a no-op metadata hit once the directory exists)
     plus one reservation create+rename+unlink, no fsync and no admitter lock, on every
     protected `Edit`/`Write`/`MultiEdit` in a factory project irrespective of whether any
     migration ever runs — the same ≤ low-single-digit-ms increment that ADR-052 v1.18 already
     accepted against ADR-020 Class A; repositories with no `.factory/` protected paths pay
     nothing (their writes fall outside the protected union). The bounded-quiescence-window
     alternative is rejected (a wall-clock wait cannot prove an arbitrarily long in-flight
     writer finished, and adds a fixed activation delay); accepting the fingerprint abort as
     designed behavior is rejected (it leaves a known spurious-abort path that a
     few-millisecond fix removes).
   - **Release-on-block, including dispatcher-side release (v1.4; ADR-052 §Decision 5a
     "Release-on-block"; D2).** Reservation creation is followed, within the SAME dispatcher
     process, by `shard_cap_precheck` and the registry plugin tiers, any of which may Block/Error
     — in which case the tool never runs and PostToolUse is not guaranteed to fire. Therefore: if
     the dispatch's final aggregated outcome for a PreToolUse event is a block (exit 2) or an
     error, the dispatcher MUST remove the reservation its own admission created for that
     `tool_use_id` before exiting, so a later-stage block leaves no reservation file behind.
     (Harness-level denials the dispatcher cannot observe — user denying the permission prompt,
     or a block by a hook process other than this dispatcher — still leak until PostToolUse or
     the TTL; that residual is what the TTL and the operator remediation cover.)
   - **How DRAINING waits.** The coordinator (`run_backfill_append_logs`) MUST execute, in this
     order, BEFORE any source snapshot, STAGING work or staging generation: (1) stale-reservation
     TTL GC; (2) acquire the advisory flock on `exclusive.lock`; (3) flip `gate_state =
     DRAINING` under `LOCK_EX` on gate-state.json (from this instant PreToolUse admissions are
     refused with `E-MAINTENANCE-001`, so no NEW reservation can appear); (3a) write the initial
     txn record `state=STAGING`, `generation_id=null`, `source_sha256=null` (strictly AFTER the
     DRAINING flip, so `gate=OPEN` with an active txn is unreachable); (4) poll `reservations/`
     until empty; (5) only after quiescence, compute `source_sha256` over the four files,
     flip `gate_state = LOCKED`, and fill the txn record. The txn record MUST NOT contain a
     non-null `source_sha256` computed before quiescence. A writer admitted before the DRAINING
     flip therefore always completes (PostToolUse removes its reservation) before the snapshot.
   - **Timeout.** The quiescence poll has a 30 s limit (default; the limit and the
     TTL below MUST be injectable parameters of the drain function so tests do not wait wall-clock
     time). On timeout the coordinator MUST: set the txn record `state = ABORTED`, flip
     `gate_state = OPEN` (under `LOCK_EX`), release `exclusive.lock`, leave all four canonical
     files byte-identical and create no staging generation, and exit 2 with
     `DRAIN_TIMEOUT_ABORT` (error-taxonomy MIG category). Every other abort/rollback path
     (census, content-preservation, fingerprint, expiry) likewise flips the gate to OPEN before
     returning (ADR-052 §Decision 5a abort gate-reset obligation).
   - **Stale reservations from dead writers.** A writer that crashes between PreToolUse and
     PostToolUse leaves its reservation behind. Liveness is NOT judged by PID (the creating
     per-event PreToolUse process is dead by design while the tool still runs; PID-liveness GC
     would reclaim a live writer's reservation and let the coordinator snapshot mid-write). The
     ONLY reclamation rule is TTL: at the start of EVERY drain (step 1), any reservation whose
     `now − created_at > MAX_RESERVATION_TTL` (default 3,600 s; the production entry delegates to
     the crate-private `…_with_ttl` seam passing `DEFAULT_MAX_RESERVATION_TTL` (EC-031) and MUST
     NOT be configured below the 1,800 s floor — a lower value is a configuration error, v1.8:
     `BcIndexMigrationError::ReservationTtlBelowFloor { configured_secs, floor_secs }`
     (`RESERVATION_TTL_BELOW_FLOOR`, exit 2) raised by `validate_production_reservation_ttl`, the
     FIRST statement of that seam (v1.10), BEFORE any gate or drain action, nothing mutated, and not reusing `BinaryIntegrityFailure`;
     the TTL and drain timeout stay injectable parameters of the drain function for tests, the
     floor binds the production entry point, not the test seam — `drain_bc_index_writers` is not
     bound) is removed. **Staleness is judged by
     the reservation file's `created_at` field (v1.4), falling back to the file mtime ONLY when
     the field is absent, unparseable or untrusted (v1.8: pre-epoch, out-of-range, or a future
     stamp beyond the 300 s skew tolerance — see "Reservation timestamp rules"; both unusable ⇒
     NOT stale + warn)**; never by PID/liveness (v1.9 H1, unchanged); a younger reservation is never removed by the coordinator and so
     blocks quiescence until its PostToolUse fires or the 30 s timeout elapses. A crashed
     session's abandoned reservation therefore yields `DRAIN_TIMEOUT_ABORT` on activations
     attempted before its TTL elapses; the sanctioned operator remediation is manual deletion of
     the confirmed-stale `*.reservation` file (ADR-052 §Decision 5a "Operator remediation").
   - **Gate self-healing.** After any crash, a stuck `LOCKED`/`DRAINING` gate with no active
     txn (or a STAGING txn with `generation_id=null` and no live coordinator) is returned to
     `OPEN` by the next PreToolUse via the flock-gated reconciliation of ADR-052 §Decision 5a
     step 3.5; `E-MAINTENANCE-001` never persists across a crashed abort. **Branch B on-disk
     marker (v1.6; normative text in BC-1.18.011 Precondition 6(d)):** the Branch B discard — whose
     execution order is now **version gate first (Precondition 6(c) rule 9), then the `generation_id`
     tri-state**: a `schema_version` ≥ 2 on that record is `txn_record_newer_schema` and NOT the
     discard (txn bytes unchanged), while `schema_version` absent or `1` + an unknown extra key still
     discards and preserves the key — rewrites the txn record IN PLACE (same `txn-*.json` file, one atomic write-temp + fsync +
     rename + dir-sync) to `"state": "ABORTED"` plus the top-level string field
     `"abort_reason": "null_generation"`; `generation_id` and `source_sha256` stay `null`; the
     file is RETAINED (never deleted, renamed or archived by the reconciler). Mechanism A's
     Branch B behaves identically (the shared core is migration-agnostic for Branch B).
     **Terminal-record reconciliation decision cell (v1.6; reworded v1.8 for F-006; table in
     BC-1.18.011 Precondition 6(d)):** a live txn whose `migration_id ∈ K` (K =
     {`migrate-bc-index`, `backfill-append-logs`}; absent field ⇒ `migrate-bc-index`) +
     `exclusive.lock` acquired + THAT txn's own migration's terminal record (selected by
     `migration_id`) ABSENT ⇒ `NoOp` for BOTH STAGING and COMMITTING; the dispatch falls through
     to Branch B (only if STAGING with `generation_id = null`) and otherwise to the ordinary
     admission decision, which blocks because a txn is live (plain message, no suffix). The
     pure core has NO "evaluating migration" (it serves both): its input
     `TerminalReconcileInputs.txn_is_own_migration` is RENAMED `txn_migration_known: bool`
     (true iff `migration_id ∈ K`), and a live txn of EITHER known migration (e.g. a live
     `migrate-bc-index` txn beside a present `completed-backfill-append-logs.json`) is
     decided against ITS OWN migration's terminal record; the OTHER known migration's record is
     NEVER consulted (so "foreign" never applies between the two known ids on the dispatcher
     path). A live txn whose `migration_id ∉ K` (an id written by a newer or alien build) is
     FOREIGN: `RefuseForeignMigration` (plain `E-MAINTENANCE-001`, no suffix, precedence over
     every record check), NEVER finalized and NEVER aborted — including Branch B's
     null-generation discard (a record this build cannot interpret is not this build's to
     discard); no Branch A/B/C repair runs; the gate keeps blocking the union; the raw id
     (sanitized: truncated to 64 chars, control characters escaped) goes only to the
     `migration.admission_blocked` InternalLog event (field `migration_id`, `branch =
     foreign_migration`; Postcondition 10). A `migration_id` that is not a JSON string is a MALFORMED record ⇒ admission
     error `E-MAINTENANCE-002` (`state_integrity`), not "foreign".

7. **Coordinator anchoring — `backfill-append-logs` resolves its factory root by the SAME single
   rule as admission (v1.10; ADR-052 §Decision 5a "Single anchoring rule (v1.21)"; closes D-2).**
   The `backfill-append-logs` coordinator and its CLI entry take the resolved PROJECT ROOT as
   their parameter — computed by `main.rs` as `resolve_session_project_root(
   std::env::var_os("CLAUDE_PROJECT_DIR"), <std::env::current_dir()>)`, the function of
   Precondition 6(b)(i), never the raw process cwd — and: (a) obtain the factory root ONLY through
   `resolve_factory_root(project_root)`; (b) derive EVERY `.factory/…` path they read or write
   (`migration-state/` via `FactoryRoot::migration_state_dir()`, the four canonical append-log
   paths, the activation manifest, the BC-INDEX/shard paths, `shard-config.toml`) from that SAME
   resolved real factory root — no `join(".factory/…")` literal remains outside the resolver; (c)
   NEVER create `.factory`, never fall back to another directory, and when `resolve_factory_root`
   yields `Ok(None)` (`.factory` ABSENT under the resolved project root, per Precondition
   6(b)(iii): a `stat` success on a non-directory, `ENOENT` including a dangling symlink, or
   `ENOTDIR`) mutate NOTHING and
   exit 2 `FACTORY_ROOT_NOT_FOUND` (`BcIndexMigrationError::FactoryRootNotFound { project_root,
   root_source }`; migration-binary exit 2, blocked; no `exclusive.lock`, no `migration-state/`, no
   gate write). The normative single stderr line (v1.10-rev; ADR-052 §Downstream item 33(e)) is
   `<subcommand>: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root
   <project_root> (resolved from <source>)`, with `<subcommand>` ∈ {`migrate-bc-index`,
   `backfill-append-logs`} and `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`} (the origin
   `resolve_session_project_root` used: a present non-empty `CLAUDE_PROJECT_DIR`, else the
   process cwd).
   **Field name (v1.10-rev2):** the variant's source field is `root_source`, NOT `source` —
   `thiserror` treats a field named `source` as the error's `std::error::Error::source()`
   (it would have to implement `Error` and the variant would not compile); the stderr wording
   `(resolved from <source>)` is unchanged. **Optionality — rule (a), normative (v1.10-rev2):**
   `root_source: Option<ProjectRootSource>`. (i) BOTH coordinator CLI routes (`migrate-bc-index`
   and `backfill-append-logs`) MUST thread the whole `SessionProjectRoot` (path AND source, as
   `resolve_session_project_root` returned it — never re-read from the environment) to their
   coordinator entry (`run_bc_index_migration_for_session(&SessionProjectRoot)` for B2; the
   mechanism-A coordinator takes the same `&SessionProjectRoot`), so `root_source = Some(_)` on
   every CLI-reachable `FactoryRootNotFound` and the CLI stderr line ALWAYS carries the
   ` (resolved from <source>)` suffix — a suffix-less `FACTORY_ROOT_NOT_FOUND` line on either
   binary is a conformance failure. (ii) The path-only library entries
   (`run_bc_index_migration(&Path)` and the crate-private `run_bc_index_migration_with_ttl(&Path,
   ttl)`, plus any path-only mechanism-A entry) have no source to report; they are PERMITTED
   (embedding / test seams; neither is invoked by a CLI route) and set `root_source = None`,
   whose `Display` omits the suffix entirely (no ` (resolved from )`, no placeholder). Rationale
   (production-grade lens): the operator-visible line is exact and total at the only surface an
   operator sees (the binary); removing the path-only entries (option b) would break the
   existing public signature and every in-crate test caller to buy nothing, while the
   `Option` is unreachable-as-`None` from the CLI by construction (i).
   **Consequence:** a reservation created by admission under a given `CLAUDE_PROJECT_DIR` lives
   in the SAME `migration-state/reservations/` directory the coordinator's first drain polls,
   whatever the process cwd (the Dekker interlock of 6(c) depends on it). The coordinator's
   failures and advisories are stderr lines, not `tracing` output (Postcondition 10). S-25.06's
   `backfill-append-logs` CLI does not exist in code yet and is built on this rule from its first
   line; S-25.09 delivers the function and the B2 re-anchoring (D-2; ADR §5a (d); this story-anchor
   moved from S-25.08 at the S-25.08/S-25.09 split, D-1252(f)).
   **Unstatable `.factory` on the coordinator (v1.12; ADR-052 §Decision 5a "Factory-root lookup
   mapping (v1.23 ruling)"; closes F-S2508-L3-009).** A `stat` failure of `<project_root>/.factory`
   other than `ENOENT` / `ENOTDIR` / not-a-directory (EACCES, EPERM, EIO, ESTALE, ELOOP, …) makes
   `resolve_factory_root` return `Err(BcIndexMigrationError::Io { path: <project_root>/.factory,
   source })`; the coordinator exits 2 with that EXISTING `Io` variant (its existing Display and exit
   mapping), NOT `FACTORY_ROOT_NOT_FOUND`, because `FACTORY_ROOT_NOT_FOUND` asserts a fact (no
   directory) that an `EACCES`/`EIO` `stat` did not establish. The error is raised before any lock
   or write: nothing is created or mutated (no `.factory`, no `migration-state/`, no
   `exclusive.lock`; the tree is byte-identical before and after). S-25.09 owns this coordinator
   `Io` mapping (`Absent` ⇒ `FactoryRootNotFound`, `Err` ⇒ propagate `Io`). Vectors:
   EC-033..EC-036, EC-043.

## Postconditions

1. **Per-file content-preservation, delegated entirely to BC-1.18.008.** For EACH of the four
   target files independently, content-preservation is BC-1.18.008 Postcondition 6(a)'s existing
   byte-for-byte record-boundary-safe partition check (concatenation of all resulting shards plus
   the final current file reproduces the original monolithic file byte-for-byte) — this BC
   introduces NO new per-file content-preservation mechanism; it is BC-1.18.011's structured
   per-BC-row equivalence check that has no analogue here, because mechanism A's partition unit
   (a time-ordered record sequence within ONE file) is not a cross-file ID space the way
   BC-INDEX's `BC-X.YY.NNN` rows are. This check is verified at ADR-052 §Decision 7c step 3b
   against each of the four staged generations, BEFORE the CURRENT-backfill-append-logs.json pointer swap.

2. **Per-file independent census, delegated entirely to BC-1.18.008.** For EACH of the four
   target files independently, the independent census is BC-1.18.008 Postcondition 6(b)'s
   existing record-integrity check (every structural record present in the original file appears
   in EXACTLY ONE resulting shard). There is no cross-file ID oracle (unlike B2's `total_bcs`);
   each file's own record set is closed and self-contained. This check is likewise verified at
   ADR-052 §Decision 7c step 3b, per file, before the pointer swap.

3. **All-or-nothing across the FOUR INDEPENDENT FILES, not sub-partitions of one file.** Unlike
   BC-1.18.011 (whose atomic unit is ten-or-more shards of ONE logical body), this migration's
   atomic unit is four SEPARATE, independently-splittable files sharing ONE txn record, ONE
   generation, and ONE CURRENT-backfill-append-logs.json pointer swap. If Postcondition 1 or 2 fails for ANY ONE of
   the four files, the ENTIRE migration aborts (step 3c): none of the four canonical files is
   touched, not merely the failing one. Each atomic file replacement (`rename(2)`) for each of
   the four files, once past the pointer swap, is followed by the platform-appropriate durability
   barrier (Linux: `fsync(file_fd)` + `fsync(parent_dir_fd)`; macOS/APFS: `fcntl(F_FULLFSYNC)` on
   the file, best-effort `fsync(dir_fd)`) per ADR-052 §Decision 7d, exactly as BC-1.18.011
   Postcondition 3 specifies for B2's shard files.

3a. **Pre-commit source-fingerprint recheck (TOCTOU guard), once over all four files.** Performed
    EXACTLY ONCE, immediately before the CURRENT-backfill-append-logs.json pointer swap (ADR-052 §Decision 7c step 5,
    following step 4's authorization gate and preceding step 6's pointer swap) — not once per
    file and not between individual renames. The migration binary re-reads all four target
    files' current content, computes a single SHA-256 over their concatenated bytes (a
    deterministic, fixed ordering: `decision-log.md`, `burst-log.md`, `lessons.md`,
    `session-checkpoints.md`), and compares against `source_sha256` in the txn record. Any
    divergence: ABORT (`FINGERPRINT_MISMATCH_ABORT`, exit 2); all four canonical files are left
    untouched; re-activation required.

4. **Rollback on verification failure is whole-migration, not per-file.** If EITHER
   Postcondition 1 OR Postcondition 2 fails for any of the four files, ALL FOUR files' original
   content is left completely untouched (fail-loud, not partial-and-silent): no subset of the
   four files is ever migrated while the others are left pending. This generalizes BC-1.18.008's
   own single-file "hard gate" (Postcondition 6/EC-004) to the four-file governed-migration unit,
   the same way BC-1.18.011 Postcondition 4 generalizes it to the ten-subsystem unit.

5. **Idempotency has two layers: the migration binary's own top-level sentinel, and
   BC-1.18.008's per-file manifest.** The governed migration's OWN idempotency signal is
   `completed-backfill-append-logs.json`'s presence (`ALREADY_MIGRATED`, exit 0, per ADR-052 §Decision 4e/7c step 8) —
   this is checked BEFORE any lock is acquired and is authoritative for "has this governed
   migration already run." (v1.3: "BEFORE any lock is acquired" means before any MIGRATION lock —
   the `exclusive.lock` flock attempt that Postcondition 5a's reconciliation performs, non-blocking,
   is the sole lock operation permitted on this path, and it comes FIRST: the sentinel is read UNDER
   that lock (v1.12 third binary-leg extension, 5a(a)/(a2); not acquired ⇒ exit 1
   `MIGRATION_LOCK_CONTENTION`, never exit 0); and `completed-backfill-append-logs.json`'s authority is
   subject to Postcondition 5a's txn-record reconciliation, which runs BEFORE the
   `ALREADY_MIGRATED` early-return.) Independently, and unchanged, each of the four files' OWN
   `mechanism_a_backfill_already_migrated`/Backfill Recovery Manifest presence check
   (BC-1.18.008 Invariant 3) remains the per-file idempotency basis the migration binary consults
   WHILE performing each file's split — e.g., on resume-from-STAGING (EC-003 below), the binary
   MUST re-run BC-1.18.008's own per-file idempotency check for each of the four files rather
   than assuming none have a pre-existing manifest. These two layers do not conflict: `completed-backfill-append-logs.json`
   answers "did the governed migration finish," BC-1.18.008's manifest answers "has THIS file's
   split already been performed" — the latter can, in principle, already be true for a file that
   was rolled by BC-1.18.006 before this migration ever ran (BC-1.18.008 Postcondition 3's
   Composability clause), and this migration's per-file invocation of
   `run_mechanism_a_backfill_split` handles that case exactly as BC-1.18.008 already specifies.

5a. **Terminal-record reconciliation precedes `ALREADY_MIGRATED` (v1.3; closes S-25.06
    formal-verification defect D3 — permanent self-lock crash window).** Step 8 of ADR-052
    §Decision 7c writes and fsyncs `completed-backfill-append-logs.json` and THEN rewrites the txn record to
    COMPLETED; a crash between the two leaves `completed-backfill-append-logs.json` durable with the txn record still
    COMMITTING (and `gate_state = LOCKED`), which the admission gate (6(b)) treats as an active
    migration window. The idempotency path MUST therefore NOT return on `completed-backfill-append-logs.json`
    presence alone. On every non-`--census` invocation in which `completed-backfill-append-logs.json` exists, the
    binary MUST, in this order:
    (a) Attempt a non-blocking `flock(exclusive.lock, LOCK_EX|LOCK_NB)` FIRST — before
        `completed-backfill-append-logs.json` (or any txn/gate state) is read for the purpose of
        acting on it; any pre-lock existence probe is a hint only. On `EWOULDBLOCK` (a live
        coordinator holds the lock; v1.12 third binary-leg extension, ADR-052 item 11(c) — this
        REPLACES the earlier "warning line, exit 0 `ALREADY_MIGRATED`", which is REJECTED because
        under contention neither the txn records nor the gate were read, so exit 0 would claim
        something unchecked, and the same on-disk state would yield different fail-closed verdicts
        depending on timing): perform NO reconciliation and NO read or mutation of txn/gate state,
        exit **1** `MIGRATION_LOCK_CONTENTION` on EVERY path including `completed-backfill-append-logs.json`
        present, NEVER exit 0 (exit 1 = "no harm done, safe to re-run"; EC-053, EC-054).
    (a2) On acquisition, `completed-backfill-append-logs.json` is read UNDER the lock and THAT read
        is what the recovery decision receives (not a hard-coded `None`); if it and a COMPLETED
        terminal txn appeared between a pre-lock probe and the lock acquisition, no fresh run
        starts, the gate is untouched and the binary exits 0 `ALREADY_MIGRATED` (a state read
        under the lock; EC-054). `ALREADY_MIGRATED` (exit 0) is emitted only for a state read
        under `exclusive.lock`.
    (b) On acquisition, select the txn record by the ADR-052 §Decision 4e txn-selection rule. If
        no txn record is in state STAGING or COMMITTING (clean steady state, COMPLETED, or
        ABORTED), skip to (d). If a STAGING txn exists alongside `completed-backfill-append-logs.json`,
        fail closed per the failure paragraph below (STAGING + terminal record is always
        fail-closed; no verification is attempted). If a COMMITTING txn exists, VERIFY before trusting `completed-backfill-append-logs.json`:
        `completed-backfill-append-logs.json` parses; its `txn_id` equals the txn record's `activation_id` and its
        `generation_id` equals the txn record's `generation_id`; its `canonical_paths_count` is 4
        (== `len(canonical_move_plan)`, v1.13);
        and for each of the four canonical files `sha256(file)` equals the `expected_post_hash`
        of that path's DONE record in the intent log located via the txn's Tier 1
        `intent_log_path`, which DONE record must itself equal the latest INTENT's `expected_post_hash`
        (v1.12, hash source v1.13: `DONE == INTENT == sha256(file)`; NOT a `canonical_move_plan` field —
        content-verified — the claim in `completed-backfill-append-logs.json` is
        checked, not assumed). On success: atomically rewrite the txn record to `state =
        COMPLETED` (write-temp + fsync + rename + dir-sync, per ADR-052 §Decision 7d). This
        finalization is idempotent and a crash during it leaves the same recoverable state
        (re-running repeats (a)–(d)).
        **AC-031 verifier steps, v1.13 (S-25.06 AC-031 consumes the S-25.10 shared module; ADR-054
        §Decision 3 B-2/B-3, §Decision 4 `verify_plan_completion`; the SAME function serves the
        `backfill-append-logs` finalize, the `migrate-bc-index` §4e verify-then-finalize and Branch C):**
        (1) read the intent log through the shared `intent_log` module (ADR-054 §Decision 1.7 reader
        and §Decision 1.8 invariants; a torn tail is absent; NO per-migration reader); (2) a log with
        mid-log corruption (bytes after the valid prefix followed by a valid record) or an L1-L4
        violation is `canonical_hash_mismatch` on the verifier and admission surfaces
        (⇒ `COMPLETION_RECORD_MISMATCH_ABORT` on the binary), never silently dropped and never a
        `state_integrity`; (3) the plan check: `canonical_move_plan` absent/ill-typed/empty ⇒
        `state_integrity` `txn_record_malformed` at this check's position, and a plan whose
        `canonical_path` set (or any pair's `staging_path`) differs from the INTENT target set ⇒
        `canonical_hash_mismatch` (B-3: a differing plan on the VERIFIERS is the mismatch class; the
        COORDINATOR, about to act on the record, reports `txn_record_malformed`); (4) per plan target,
        a `DONE` exists, satisfies L3, and `DONE.expected_post_hash == INTENT.expected_post_hash ==
        sha256(file)`; (5) only then finalize the txn, THEN open the gate.
    (c) The admission gate's own stale-state reconciliation (ADR-052 §Decision 5a step 3.5) MUST
        recognize the same state — `completed-backfill-append-logs.json` present, matching COMMITTING txn,
        `exclusive.lock` acquirable — and apply the identical verify-then-finalize action under
        its flock-gated discipline BEFORE evaluating admission, so the self-lock cannot persist
        waiting for an operator to re-run the binary (and so a blocked-writer session is never
        required to run a mutation to heal a mutation block).
    (d) Only AFTER the txn record is non-active: if `gate_state ≠ OPEN`, flip it to `OPEN`
        (ADR-052 §Decision 5c Branch 2 steps 1–4; order is mandatory — txn finalize, THEN gate
        open — so the dual-check invariant `gate=OPEN ⇒ no active txn` is never violated), release
        the locks, and exit 0 `ALREADY_MIGRATED`.
    If verification in (b) fails (any mismatch, unparseable `completed-backfill-append-logs.json`, a STAGING txn
    alongside `completed-backfill-append-logs.json` (STAGING + terminal record is always
    fail-closed; no verification is attempted), or a COMMITTING txn whose `activation_id` differs from
    `completed-backfill-append-logs.json.txn_id`): finalize NOTHING, flip NOTHING, leave the gate blocking, and exit 2
    `COMPLETION_RECORD_MISMATCH_ABORT` (fail-closed integrity anomaly; EC-010). **Binary leg for
    a malformed txn field (v1.12 extension):** if instead a Tier 1 txn field the verification
    consumes (`activation_id`, `generation_id`, `intent_log_path`) is ABSENT or ill-typed at its
    check, the binary exits 2 `MIGRATION_STATE_INTEGRITY_FAILURE` (`AdmissionStateIntegrity`,
    `txn_record_malformed`) — NOT `COMPLETION_RECORD_MISMATCH_ABORT`, which means every field read
    was well-formed but disagreeing — finalizing nothing and flipping nothing; the checks run in
    the fixed first-failure-wins order, so an earlier check's mismatch (`COMPLETION_RECORD_MISMATCH_ABORT`)
    wins over a later field's malformation; a missing intent log, a path without a DONE record, a DONE
    whose hash differs from the INTENT's, mid-log corruption (ADR-054 §Decision 1.7) or a plan that
    differs from the INTENT set (v1.13) is
    `canonical_hash_mismatch` (⇒ `COMPLETION_RECORD_MISMATCH_ABORT`), a non-ENOENT intent-log read
    is `Io` exit 2 (EC-047, EC-049..EC-052). **PreToolUse
    analogue (v1.4):** when the same verification fails on the §Decision 5a step-3.5 Branch C
    path (no binary invocation), the PreToolUse outcome is an `E-MAINTENANCE-001` block with the
    mismatch reason logged (one `migration.admission_blocked` InternalLog event with `branch =
    completion_record_mismatch`, naming `migration_id`, `txn_id` and the failing `check`; block message = the Precondition 6(b) `<scope>`-keyed format string
    followed by one space and the suffix `(completion-record mismatch — operator
    investigation required)`, identically for STAGING + terminal record and for a COMMITTING
    verification failure, for either migration's live txn and either path family — see the v1.6
    keying rule and message table) — NOT the binary exit code — and the same
    zero-write guarantee holds (no txn write, no gate write; a byte-for-byte snapshot of
    `.factory/migration-state/` is unchanged by the attempt). **Foreign-migration guard (v1.4;
    reworded v1.8 for F-006, ADR-052 §Decision 7e "Definition of foreign"):** the BINARY's
    reconciliation (this paragraph, (a)–(d)) concerns ONLY a live txn with `migration_id =
    "backfill-append-logs"` and ONLY this migration's own terminal record
    (`completed-backfill-append-logs.json`, Precondition 5); the binary MUST NEVER finalize a txn
    whose `migration_id` is `"migrate-bc-index"` on the strength of this record (nor the
    reverse), and a live txn of the OTHER known migration is refused by the binary per
    Precondition 5 / EC-016 (binary recovery path only) with exit 2 `FOREIGN_MIGRATION_REFUSED`
    (the `backfill-append-logs: refused: …` line of Precondition 5; NOT
    `COMPLETION_RECORD_MISMATCH_ABORT`). On the DISPATCHER / shared-core path
    (the step-3.5 Branch C of (c) and §Decision 5c Branch 2) the same invariant is preserved by
    SELECTION, not refusal: the core selects the terminal record, canonical-path set and N by the
    live txn's `migration_id` and never consults the other known migration's record; "foreign"
    there means ONLY `migration_id ∉ K` (K = {`migrate-bc-index`, `backfill-append-logs`}; absent
    field ⇒ `migrate-bc-index`) ⇒ `RefuseForeignMigration` (plain `E-MAINTENANCE-001`, no
    suffix, never finalized, never aborted — Precondition 6(c) "Gate self-healing" decision
    cell); a non-string `migration_id` is a malformed record ⇒ `E-MAINTENANCE-002`
    (`state_integrity`). In the clean
    steady state (no active txn, gate already OPEN) this path performs zero filesystem mutation
    (EC-004). `--census` remains read-only and never reconciles.

6. **Scope: exactly the four canonical `v1.0-brownfield-backfill` files, per ADR-052 §Decision 8's
   ratified allowlist — never a wildcard, never a different cycle, without a separate ADR-052
   allowlist amendment.** The migration operates, in ONE invocation, on exactly:
   `.factory/cycles/v1.0-brownfield-backfill/decision-log.md`,
   `.factory/cycles/v1.0-brownfield-backfill/burst-log.md`,
   `.factory/cycles/v1.0-brownfield-backfill/lessons.md`,
   `.factory/cycles/v1.0-brownfield-backfill/session-checkpoints.md`. `validate_write_target()`
   verifies the resolved canonical path matches EXACTLY one of these four; the wildcard/prefix
   form (`.factory/cycles/**`) is explicitly NOT an accepted pattern (ADR-052 §Decision 8). Other
   cycles' oversized append-log files (e.g. `v1.0-feature-engine-discipline-pass-1/`, which the
   S-25.06 story's own Context section documents as ALSO oversized) are OUT OF SCOPE for this
   BC's activation and require a SEPARATE ADR-052 allowlist amendment (architect-owned) plus a
   separate armed-activation manifest before an equivalent one-time backfill can run for them.
   S-25.06's own Task T-10 is scoped identically (only the `v1.0-brownfield-backfill/` files).

7. **Unlike BC-1.18.011 (B2), this migration's completion DOES gate BC-7.08.001's Cohort-B
   fail-closed flip.** BC-1.18.008's own Related BCs section already establishes: "the Cohort B
   fail-closed flip is gated on THIS BC completing (the existing oversized files must be split
   before flipping fail-closed, or the flip would immediately re-trigger the exact INDETERMINATE
   loop Layer 2 exists to eliminate)." This governed migration is how that completion is actually
   achieved against the live files; `BC-7.08.001`'s gating condition should read "BC-1.18.008's
   algorithm, as ACTUALLY EXECUTED via this BC's governed migration" — an implementer or
   architect closing out BC-7.08.001's own gate check MUST verify `completed-backfill-append-logs.json` exists for
   `backfill-append-logs`, not merely that BC-1.18.008's algorithm exists in the codebase.

8. **ShardRegistry enrollment (Layer 2 closure) is a SEPARATE, ordinary Edit/Write step — NOT
   part of this migration binary's ADR-052 §Decision 8 write-target allowlist.** ADR-052
   §Decision 8's allowed-write-targets list does not include `.factory/shard-config.toml`; the
   migration binary is not authorized to, and does not, write it. Enrollment is performed by an
   ordinary agent `Edit`/`Write` tool call (the sanctioned, un-exceptional path — no ADR-052
   apparatus is needed because `.factory/shard-config.toml` is an ordinary spec/config artifact,
   not a currently-oversized append-log file) adding one `[[shard]]` entry per file, using
   BC-1.18.005's already-specified schema (`artifact_stem`, `artifact_path` set to the file's
   full path under `v1.0-brownfield-backfill/` per BC-1.18.005 Postcondition 1's empty-path
   guard, `shard_cap_bytes`, `shape = "flat"`, and the four `cap_formula_inputs` fields). Once
   these four entries exist, BC-1.18.005's size-trigger and BC-1.18.006's roll fire automatically
   for these files with NO further code or config change — this Postcondition specifies WHAT
   entries are added and WHEN (as part of the same F4 activation burst, after this migration's
   `completed-backfill-append-logs.json` is written, so the enrolled cap check does not race an in-flight backfill),
   not a new rotation mechanism. **Precondition on the enrollment write itself:**
   `.factory/shard-config.toml` MUST be a registered `artifact_type` in
   `plugins/vsdd-factory/config/artifact-path-registry.yaml` before this write is attempted, or
   the `validate-artifact-path` PreToolUse gate rejects it — this is the SAME pre-existing,
   already-identified gap ADR-053 §Decision documents for STORY-INDEX's own `shard-config.toml`
   enrollment (`.factory/shard-config.toml` does not exist in the committed tree and carries no
   registry entry as of ADR-053's authoring). This registry addition is devops-engineer/architect
   scope (routed, not product-owner's to perform) and is a precondition of Postcondition 8's
   enrollment write landing successfully — it does NOT block this BC's own Spec-First Gate
   status, since Postconditions 1-7 (the backfill-split migration itself) are independently
   dispatch-ready.

9. **Relationships.** This BC depends on BC-1.18.008 (the per-file split algorithm it invokes
   unmodified), BC-1.18.006 (BC-1.18.008's own reused atomic-write/seal primitives), and
   BC-1.18.005/BC-1.18.006 (the ShardRegistry schema and rotation mechanism Postcondition 8
   enrolls these four files into) — the same "applies an existing primitive via a governed
   one-time wrapper" relationship BC-1.18.011 has to BC-1.18.010/BC-1.18.006, mirrored here for
   mechanism A's own governed-migration BC.

10. **Admission diagnostics are dispatcher-internal-log events — exactly one per verdict or
    anomaly (v1.10; ADR-052 §Decision 5a "Admission diagnostics channel (v1.21)"; closes D-1;
    wire format catalogued in BC-3.08.001 Events 11–13).** The dispatcher binary installs NO
    `tracing` subscriber, so every `tracing::*!` diagnostic is discarded in production; installing
    one is explicitly NOT the fix. Every admission, reconciliation and release diagnostic this BC
    mandates is therefore a dispatcher-native `InternalEvent` written to
    `dispatcher-internal-YYYY-MM-DD.jsonl` (via `InternalLog::write`, NOT
    `HostContext::emit_internal` — no `HostContext` exists at the registry-independent admission
    position; the one deliberate deviation from BC-3.08.001's dual-sink pattern). **Purity:** the
    shared core and `executor::migration_writer_admission*` stay free of the log and return the
    diagnostics as DATA (`MigrationAdmission.diagnostics: Vec<AdmissionDiagnostic>`;
    `AdmissionOutcome::{Admitted,Blocked}` carry the same; the release leg returns its own list);
    `main.rs` writes them immediately after each call and BEFORE the early return on a verdict, so
    a blocked or failed dispatch is logged too. **Mapping (each distinct verdict or anomaly
    produces EXACTLY ONE event; an admitted write with no anomaly, and an out-of-scope admit,
    produce none):**
    (a) every `E-MAINTENANCE-001` verdict ⇒ one `migration.admission_blocked` — fields `scope`
    (`BC-INDEX` | `.factory/cycles/`), `family` (`bc_index` | `cycles`), `branch` ∈
    {`gate_only`, `live_txn`, `foreign_migration`, `live_coordinator`,
    `completion_record_mismatch`}, `gate_state`, `migration_id` / `txn_id` (sanitized; `null` when
    no live txn), `check` (non-null ⇔ `branch = completion_record_mismatch`, else `null`; when non-null EXACTLY
    one token of the **closed nine-token `check` domain** below), `reconciliation` (closed six-token domain of the EFFECTFUL reconciliation
    outcome that preceded the block: `live_coordinator` | `nothing_to_reconcile` |
    `gate_reopened` | `null_generation_txn_aborted` | `foreign_migration_refused` |
    `completion_record_mismatch`; there is NO `none` — reconciliation always runs before a block;
    v1.10-rev, ADR-052 §Downstream item 33(b)). **`branch` derivation (total):** `branch =
    live_coordinator` ⇔ `reconciliation = live_coordinator`; `branch = foreign_migration` ⇔
    `reconciliation = foreign_migration_refused`; `branch = completion_record_mismatch` ⇔
    `reconciliation = completion_record_mismatch`; otherwise `branch = gate_only` if no live txn
    remains, else `branch = live_txn`. A Branch C verification failure is ONE `_blocked`
    (`branch = completion_record_mismatch`, `check` = the failing check) and NO `_advisory`
    (item 33(a)).
    **Closed `check` value domain (v1.10-rev2; exactly these nine snake_case tokens, each a
    compile-time constant of an exhaustive `enum` with `fn token(self) -> &'static str` — never
    derived from on-disk content, never `null` when `branch = completion_record_mismatch`; any
    other value is a conformance failure).** *Verification checks* — evaluated in this fixed
    order, FIRST failure wins, over the live txn's OWN migration's terminal record (4 canonical
    paths for `backfill-append-logs`, B2's N for `migrate-bc-index`): (1)
    `staging_with_terminal_record` — txn STAGING and its own terminal record present (always
    fail-closed, no verification attempted; emitted by S-25.09 and S-25.06); (2)
    `terminal_record_unparseable` — bytes read but not valid UTF-8 / empty / truncated / not
    valid JSON; (3) `terminal_record_schema_mismatch` — valid JSON of the wrong shape (a required
    field missing or wrong-typed, unsupported `schema_version`); (4) `txn_id_mismatch` — record
    `txn_id` ≠ the txn record's `activation_id`; (5) `generation_id_mismatch` — record
    `generation_id` ≠ the txn record's; (6) `canonical_paths_count_mismatch` —
    `canonical_paths_count` ≠ the migration's N; (7) `canonical_hash_mismatch` — for the first
    canonical path (in canonical path order) whose `sha256` ≠ the `expected_post_hash` of its DONE
    record in the intent log (located via the txn's Tier 1 `intent_log_path`; v1.12) or which is
    ENOENT, or whose intent log is missing or has no DONE record for it. *S-25.09 seam tokens* (emitted ONLY while the verify-then-finalize
    effect is undelivered; BOTH retired, never emitted, once S-25.06 delivers the verifier and
    finalize): (8) `terminal_record_unverified` — txn COMMITTING with its own terminal record
    present but the build cannot verify it (the pure core's fail-closed arm); (9)
    `finalize_unwired` — the pure core decided `FinalizeThenOpenGate` but the finalize effect is
    undelivered (also yields the `branch_c_finalize_unwired` advisory). The domain is exactly
    {`staging_with_terminal_record`, `terminal_record_unparseable`,
    `terminal_record_schema_mismatch`, `txn_id_mismatch`, `generation_id_mismatch`,
    `canonical_paths_count_mismatch`, `canonical_hash_mismatch`, `terminal_record_unverified`,
    `finalize_unwired`} — nine tokens (each ≤ 64 chars; the event-level sanitization of `check`
    is retained as defense in depth but is a no-op on these constants). A terminal-record or canonical-file read CALL that fails with a
    non-ENOENT OS error is NOT a Branch C check: it is `E-MAINTENANCE-002 (io)` (no
    `migration.admission_blocked`; one `migration.admission_failed`).
    (b) every `E-MAINTENANCE-002` verdict ⇒ one `migration.admission_failed` — fields `cause` ∈
    {`invalid_tool_use_id`, `io`, `state_integrity`}, `kind` (the `AdmissionStateIntegrityKind`
    token when `cause = state_integrity`, else `null`), `detail` (path, `io::Error` kind and
    message, or byte length);
    (c) every non-verdict anomaly ⇒ one `migration.admission_advisory` — field `reason` ∈
    {`reservation_release_failed`, `branch_a_gate_reopened`, `branch_b_txn_aborted`,
    `branch_c_finalize_unwired`, `branch_c_finalized`} (v1.10-rev; item 33(c)), plus optional
    context fields drawn ONLY from the closed set {`migration_id`, `txn_id`, `check`, `detail`,
    `tool_use_id_len`} (each sanitized as below; `tool_use_id_len` = byte length; ANY other field
    is forbidden; item 33(d)). Mapping: Branch A gate re-open ⇒ `branch_a_gate_reopened`; Branch B
    null-generation discard ⇒ `branch_b_txn_aborted`; a non-ENOENT release error ⇒
    `reservation_release_failed`, and (v1.12; ADR-052 v1.23 "Factory-root lookup mapping") ALSO
    when `resolve_factory_root` reports the `.factory` `stat` as UNSTATABLE on the release leg
    (PostToolUse / PostToolUseFailure): `detail` carries the sanitized path, `ErrorKind` and
    message; it is NEVER a verdict, nothing is created or deleted, and the reservation (if one
    exists) is reclaimed by the drain-start TTL GC (an ABSENT `.factory` on the release leg is a
    silent no-op with NO event); `branch_c_finalize_unwired` fires when
    `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` and the finalize effect
    is undelivered (the S-25.09 seam) — written IN ADDITION to the one `migration.admission_blocked`,
    which in this case carries `reconciliation = completion_record_mismatch`, `branch =
    completion_record_mismatch` and `check = finalize_unwired` (item 33(g); `finalize_unwired` is
    therefore a member of the `check` value domain), and retired — together with the
    `finalize_unwired` check — when S-25.06 delivers the finalize (after which a verified
    finalize is admitted: no `_blocked`, `_advisory` `branch_c_finalized`); `branch_c_finalized` is written by S-25.06's
    successful verify-then-finalize (the write is then admitted, so NO `_blocked`) and neither S-25.08
    nor S-25.09 emits it. The five reservation-timestamp tokens (`created_at_unparseable`,
    `created_at_pre_epoch`, `created_at_future`, `mtime_future`, `age_unknown`) are NOT advisory
    reasons: they are coordinator-drain stderr tokens (6(c) "Reservation timestamp rules" (5)).
    **Hygiene (all three types):** no field carries a raw `tool_use_id` (at most its byte length)
    or record content; every string field derived from on-disk or payload data (`migration_id`,
    `txn_id`, `check`, `detail`) is sanitized — control characters escaped and truncated to 64
    characters (EC-029). The operator-visible verdict message is UNCHANGED (single line, cause
    token only); the events are the queryable detail. **Coordinators:** the two coordinator
    binaries are CLI processes whose stderr is the operator surface — each failure or advisory they
    previously sent only to `tracing` (`migrate-bc-index: migration failed`, the drain's
    reservation-staleness advisories, the live-coordinator warning of Postcondition 5) is
    written to stderr as one `E-…`/`<VARIANT>`-prefixed line; a `tracing::error!` there is only an
    additional developer trace. Vectors: EC-037, EC-038, EC-039.

## Invariants

1. **This BC's per-file split logic invokes BC-1.18.008's `run_mechanism_a_backfill_split`
   (which itself invokes BC-1.18.006's atomic-write/seal primitives), never a reimplementation.**
   This BC differs from BC-1.18.008 in WHEN and HOW it is invoked (once, at F4 activation, via
   the `Bash`-tool-invoked `backfill-append-logs` subcommand under ADR-052's sanctioned execution
   path) and in WHAT ATOMICITY ENVELOPE wraps the four independent per-file invocations: ADR-052
   §Decision 7 supplies the NEW multi-file crash-atomicity machinery (advisory flock; durable txn
   record; framed intent log; single CURRENT-backfill-append-logs.json pointer swap across all four files;
   `completed-backfill-append-logs.json` as the permanent terminal record) that BC-1.18.008 alone — a single-file
   algorithm — does not and need not provide.

2. **No record, in any of the four files, is ever counted twice or dropped.** BC-1.18.008's own
   per-file record-integrity check (Postcondition 6(b)) is the sole source of truth for "did
   every record in THIS file survive its split"; this BC's Postcondition 4 additionally
   guarantees that a per-file failure aborts ALL FOUR files' migration, never only the failing
   one — neither check substitutes for the other.

3. **The migration is never partially applied across the four files.** At every observable point
   in time, either ALL FOUR canonical files are still in their pre-migration monolithic form, or
   ALL FOUR are in their post-migration split form (current file + sealed shards); it is never
   observed with two files migrated and two not. The all-or-nothing guarantee is implemented via
   the single CURRENT-backfill-append-logs.json atomic pointer swap (Precondition 5) and the intent log, exactly as
   BC-1.18.011 Invariant 3 specifies for the ten-subsystem case, generalized here to four
   independent files sharing one txn record instead of one file's ten-way internal partition.
   During the COMMITTING window, readers use the SAME open-with-ENOENT-fallback protocol
   (ADR-052 §Decision 7c step 2a) per file. `completed-backfill-append-logs.json` is the permanent terminal record;
   forward recovery uses the intent log + matching-destination-hash rule to resume from the
   first uncompleted canonical path move among the four.

4. **This BC's completion DOES gate BC-7.08.001's Cohort-B fail-closed flip** (Postcondition 7)
   — the INVERSE of BC-1.18.011 Invariant 4, which states B2's migration explicitly does NOT
   gate that flip. An implementer or architect verifying BC-7.08.001's readiness MUST check
   `completed-backfill-append-logs.json` for `backfill-append-logs`, not merely BC-1.18.008's code presence.

5. **Exactly the four ratified `v1.0-brownfield-backfill` files — never a wildcard, never
   auto-discovered.** No implementation may add a cycle-directory argument, a glob, or an
   auto-discovery mode to `backfill-append-logs`'s argument grammar without a superseding
   ADR-052 amendment (ADR-052 §Decision 3's closed-grammar guarantee; Precondition 3/
   Postcondition 6 above).

6. **No permanent self-lock; the gate is live; terminal state is coherent (v1.3).** (a) For
   every reachable crash state of the migration there is a finite sequence of ordinary
   operations — a PreToolUse admission self-heal (Postcondition 5a(c) / ADR-052 §Decision 5a step
   3.5), or a re-invocation of `backfill-append-logs` — that returns `gate_state` to `OPEN` with
   no human file edit, EXCEPT the explicitly fail-closed integrity anomalies
   (`COMPLETION_RECORD_MISMATCH_ABORT`, `RECOVERY_REQUIRES_REAUTHORIZATION`, abandoned-reservation
   `DRAIN_TIMEOUT_ABORT` before TTL). (b) After any reconciling operation, `completed-backfill-append-logs.json`
   present implies the txn record is COMPLETED (never COMMITTING) and `gate_state = OPEN`. (c)
   The admission gate (Precondition 6(b)) is wired on the production dispatch path; the gate
   function's existence alone never satisfies Precondition 6(b). (d) Quiescence precedes
   snapshot: no `source_sha256` is computed while any reservation younger than
   `MAX_RESERVATION_TTL` exists (Precondition 6(c)).

7. **`admission_failure_cause` is an EXHAUSTIVE `match` over `BcIndexMigrationError` — no `_`
   wildcard arm (v1.10; ADR-052 §Decision 5a "Admission state-integrity variant"; closes
   F-012).** It returns the closed `AdmissionFailureCause { InvalidToolUseId, Io, StateIntegrity }`
   (`fn token(self) -> &'static str`): `InvalidToolUseId → InvalidToolUseId`, `Io → Io`,
   `AdmissionStateIntegrity → StateIntegrity`, and every coordinator-only variant is listed
   EXPLICITLY in one arm mapping to `StateIntegrity` with a comment that the arm is unreachable
   from admission and is the fail-closed classification if ever surfaced. Adding a variant to
   `BcIndexMigrationError` is therefore a compile error until it is classified; no future
   variant is silently mislabelled `state_integrity`.

8. **ONE anchoring function, three call sites (v1.10; ADR-052 §Decision 5a "Single anchoring
   rule"; closes D-2).** The admission leg, the release leg and BOTH coordinator binaries obtain
   the project root only from `resolve_session_project_root` and the factory root only from
   `resolve_factory_root`; `migration-state/` is only ever `FactoryRoot::migration_state_dir()`.
   No `join(".factory/…")` literal exists in `shard_manager.rs` outside the resolver (sibling-sweep
   gate: `grep -n 'join(".factory' crates/factory-dispatcher/src/shard_manager.rs` returns only the
   resolver), so a reservation and the drain that waits on it can never be in different
   directories (Precondition 7; EC-033).

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Postcondition 1 (content-preservation) passes for three files but Postcondition 2 (independent census) fails for the fourth | Migration ABORTS for ALL FOUR files per Postcondition 4 — a per-file failure is a whole-migration failure; the three passing files' original content is also left untouched, not silently migrated ahead of the failing one |
| EC-002 | Migration crashes after completing canonical path moves for 2 of the 4 files (post-pointer-swap, mid-step-7) | ADR-052 §Decision 7c step 7's forward recovery resumes from the intent log's first uncompleted move (matching-destination-hash rule); the 2 already-moved files are NOT re-moved; the 2 remaining files complete on the recovery pass; no file is left half-migrated since each individual file's own `rename(2)` is atomic |
| EC-003 | A prior activation attempt built a fully-staged generation for all four files but crashed before the CURRENT-backfill-append-logs.json pointer swap | Re-running MUST detect txn record state STAGING and resume toward the pointer swap; per ADR-052 §Decision 7c step 3b (referenced by §Decision 4e), the resume path MUST re-run the FULL per-file census (Postconditions 1/2) for all four files before proceeding — it does NOT skip re-verification merely because a staged generation was previously built, mirroring BC-1.18.011 EC-003 |
| EC-004 | `backfill-append-logs` is invoked after a prior activation already reached `completed-backfill-append-logs.json` | `ALREADY_MIGRATED` (exit 0); no manifest required; no migration lock held (only the non-blocking `exclusive.lock` acquisition of Postcondition 5a(a), under which the sentinel is read; if that lock is not acquired the result is exit 1 `MIGRATION_LOCK_CONTENTION`, never `ALREADY_MIGRATED`, EC-053); in the clean steady state (no active txn, gate OPEN) the binary performs zero filesystem mutation and exits immediately; if a stale COMMITTING txn / non-OPEN gate is present, Postcondition 5a reconciliation applies first (EC-009) |
| EC-005 | One of the four canonical files does not exist, or is unreadable, at activation time (S-25.06 story EC-004) | The migration ABORTS before acquiring the flock or building any staging generation (fails the pre-lock/under-exclusion validation, ADR-052 §Decision 4b/4c) with a clear non-zero exit; no partial state is written for any of the four files |
| EC-006 | A single record within one of the four files exceeds `shard_cap_bytes` on its own (BC-1.18.008 EC-002) | NOT a migration-level abort: BC-1.18.008's own oversized-record exception (`oversized_record: true`) applies to that file's shard exactly as BC-1.18.008 specifies; this governing BC's Postcondition 1/2 census-agreement gate tolerates the flagged exception and proceeds with the other files' and that file's remaining shards normally |
| EC-007 | `record_boundary_offsets` computed from the marker regex yields zero boundaries for one of the four files despite non-empty content (S-25.06 story EC-006; BC-1.18.008's O-1 note that this is unreachable in production for the four recognized artifact stems) | Hard failure via BC-1.18.008 Postcondition 6's fail-loud content-preservation gate for that file, surfaced by this migration as `CONTENT_PRESERVATION_ABORT` (exit 2) for the WHOLE migration per Postcondition 4 — never a silent empty-oracle partition |
| EC-008 | `backfill-append-logs` is invoked with a path argument, a `--cycle` flag, or any token outside the exact closed grammar (`backfill-append-logs` / `backfill-append-logs --census`) | REJECTED by the pre-shell classifier (ADR-052 §Decision 5c) before the binary is even invoked; if somehow bypassed, the binary itself rejects with a non-zero exit (ADR-052 §Decision 3) — this supersedes the S-25.06 story's provisional AC-001(b) cycle-directory-argument assumption (Precondition 3) |
| EC-009 | (v1.3, D3) Crash AFTER `completed-backfill-append-logs.json` is durable but BEFORE the txn record is rewritten COMPLETED in step 8: `completed-backfill-append-logs.json` present, txn record COMMITTING (matching `activation_id`/`generation_id`), `gate_state = LOCKED`, all four canonical files at their post-migration hashes | The admission gate (6(b)) blocks `.factory/cycles/` writes with `E-MAINTENANCE-001` ONLY until reconciliation runs. Reconciliation — either a PreToolUse self-heal (Postcondition 5a(c)) or a re-invocation of `backfill-append-logs` (Postcondition 5a(a)–(d)) — verifies `completed-backfill-append-logs.json` against the txn record and the four files' `expected_post_hash`, rewrites the txn record to COMPLETED, THEN flips the gate to OPEN; the invocation exits 0 `ALREADY_MIGRATED`. A second re-run is a zero-mutation `ALREADY_MIGRATED`. A crash during the reconciliation itself leaves the same recoverable state (idempotent) |
| EC-010 | (v1.3, D3) `completed-backfill-append-logs.json` present but cannot be matched to the active non-terminal txn record: unparseable `completed-backfill-append-logs.json`; `canonical_paths_count ≠ 4`; `txn_id`/`generation_id` ≠ the txn record's; txn record STAGING (not COMMITTING — STAGING + terminal record is always fail-closed; no verification is attempted); or any canonical file's `sha256` ≠ the `expected_post_hash` of its DONE record in the intent log located via the txn's Tier 1 `intent_log_path` (v1.12; a missing intent log or a path with no DONE record is the same mismatch) | Fail-closed: NO txn finalization, NO gate flip, gate stays blocking, exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` from the binary; the PreToolUse analogue (Branch C, no binary invocation) is an `E-MAINTENANCE-001` block with the mismatch reason logged (v1.4). Never silently "repair" an inconsistent terminal state — a human investigates; a byte-for-byte snapshot of `.factory/migration-state/` is unchanged by either path |
| EC-011 | (v1.3, D2) An admitted writer holds a reservation (PreToolUse fired, PostToolUse not yet fired) when the coordinator begins DRAINING and it is not released within the 30 s drain timeout | `DRAIN_TIMEOUT_ABORT` (exit 2): txn record → ABORTED, `gate_state` → OPEN, `exclusive.lock` released, no staging generation, all four files byte-identical, no `source_sha256` ever recorded; a later activation retried after the writer completes succeeds (Precondition 6(c)) |
| EC-012 | (v1.3, D2) A reservation file exists whose creating writer crashed (no PostToolUse will ever fire): (a) `now − created_at > MAX_RESERVATION_TTL`; (b) `now − created_at ≤ MAX_RESERVATION_TTL` | (a) removed by drain step 1 TTL GC; drain reaches quiescence normally. (b) NEVER removed by the coordinator (no PID-liveness inference — an in-flight live writer's reservation looks identical); drain waits then `DRAIN_TIMEOUT_ABORT` at 30 s; remediation is operator deletion of the confirmed-stale file (Precondition 6(c)) |
| EC-013 | (v1.3, D1; v1.4 Bash removed) A mutation tool call (`Edit`/`Write`/`MultiEdit` ONLY — write-effect `Bash` is the [D-1232-OBL-4] classifier's, not S-25.06's tested surface; until it ships POL-3 + the §Decision 7c step-5 fingerprint recheck are the backstops) targets a path under `.factory/cycles/` (any cycle) or `.factory/specs/behavioral-contracts/` while a txn record is STAGING or COMMITTING, delivered through the REAL dispatcher entry point (PreToolUse payload to the `factory-dispatcher` binary, not a direct unit call of the precheck) | Blocked: `E-MAINTENANCE-001`, exit 2, `<scope>` = `.factory/cycles/`; a path outside the protected union (e.g. `.factory/STATE.md`) is admitted; with no active txn and gate OPEN the same call is admitted AND `reservations/<tool_use_id>.reservation` exists until the matching PostToolUse removes it (Precondition 6(b)/(c)); the shared admission core is evaluated exactly ONCE per event (the reservation is written once; reconciliation is not re-run) |
| EC-014 | (v1.4, D1 re-scope) The dispatcher receives a PreToolUse envelope for the `Bash` tool — including (a) a write-effect command whose target is a protected path and (b) the coordinator's own closed-grammar `backfill-append-logs` / `backfill-append-logs --census` invocation — through the REAL dispatcher entry | The Rust admission gate leaves `Bash` UNPROCESSED: it creates NO reservation, returns no decision (`None`) and writes nothing under `.factory/migration-state/`, even with txn STAGING/COMMITTING; consequently the coordinator can never self-deadlock through it. Blocking/classification of write-effect `Bash` is [D-1232-OBL-4] (backstops until it ships: POL-3 `^Bash$` guards + §Decision 7c step-5 fingerprint recheck). Any future Bash leg in the Rust gate must carry the §Decision 5c Branch 1–4 coordinator exemption |
| EC-015 | (v1.4, D4) Per-migration terminal-record namespace: (a) `completed.json` / `CURRENT.json` exist (a finished `migrate-bc-index`) but `completed-backfill-append-logs.json` does not; (b) `completed-backfill-append-logs.json` exists but `completed.json` does not | (a) `backfill-append-logs` is NOT `ALREADY_MIGRATED`: it proceeds as a first activation (and never reads `completed.json` as its own terminal record). (b) `migrate-bc-index` is not `ALREADY_MIGRATED` by it, and BC-1.18.010's reader protocol / `detect_migration_read_state` does NOT report the BC-INDEX migration COMPLETE on its strength (BC-INDEX readers do not switch to shard paths that do not exist). This binary never writes `completed.json`/`CURRENT.json` |
| EC-016 | (v1.4, D4; v1.8: **binary recovery path only**) The single live txn (STAGING or COMMITTING) has `migration_id = "migrate-bc-index"` (or lacks the field, read as that) when the `backfill-append-logs` BINARY is invoked, or the reverse | Cross-migration refusal (binary recovery path only; the dispatcher/shared-core path never refuses between the two known ids — it decides a live txn of either against ITS OWN migration's record, EC-027/EC-028): exit 2 `FOREIGN_MIGRATION_REFUSED` (v1.12 extension; ADR-052 v1.23 item 9; EXACT stderr `backfill-append-logs: refused: a live migration transaction owned by migration_id "<id>" is in progress (FOREIGN_MIGRATION_REFUSED, exit 2); this subcommand never recovers, finalizes or aborts another migration's record; nothing was changed`, and for the reverse fixture the `migrate-bc-index: BC-INDEX migration: refused: …` line of BC-1.18.011 Precondition 6(e); replaces the retired "`LockContention`-class" label; flock EWOULDBLOCK is instead exit 1 `MIGRATION_LOCK_CONTENTION` on EVERY path including `completed*.json` present, never exit 0, checked before the loader and before `completed*.json` is read, and winning beside a foreign record — v1.12 third binary-leg extension, EC-053), NO mutation (byte-for-byte snapshot of `.factory/migration-state/` unchanged), `recover()` is NOT run over the foreign record, the foreign txn is not finalized/aborted; the admission gate still blocks the union for either migration's live txn. A foreign terminal record never finalizes a live txn of this migration (Postcondition 5a foreign-migration guard) |
| EC-017 | (v1.4, D5) Reserve-then-verify interleaving of an admitter (W1 = create reservation, W2 = read gate) against the coordinator (C1 = durable DRAINING flip, C2 = read `reservations/`) at every ordering of {W1, C1, C2, W2} | In every ordering EITHER the coordinator's C2 observes the reservation (and waits; `source_sha256` not computed while it exists) OR the admitter's W2 observes DRAINING (admission refused `E-MAINTENANCE-001` and its own reservation removed) — NEVER both miss; in particular an admitter that reads `OPEN` from a gate file the coordinator subsequently replaces via `rename` is still observed by the coordinator |
| EC-018 | (v1.4, D2 release-on-block) Admission creates the reservation, then a LATER stage of the same dispatch (`shard_cap_precheck` or any registry plugin) blocks or errors; or the admitter's own verification fails | The dispatcher removes the reservation its admission created before exiting; NO `reservations/<tool_use_id>.reservation` remains for a blocked/errored event (a harness-level denial the dispatcher cannot observe leaks until PostToolUse/TTL — documented residual) |
| EC-019 | (v1.4; extended v1.8 with the timestamp rules, vectors in EC-025) A reservation file whose `created_at` is parseable vs. absent/unparseable/untrusted | Staleness uses `created_at`; mtime is the fallback ONLY when `created_at` is absent, unparseable, pre-epoch/out-of-range, or a future stamp beyond the 300 s skew tolerance (e.g. `created_at` 4,000 s ago with a fresh mtime ⇒ stale under TTL 3,600 s; absent `created_at` with mtime 4,000 s ago ⇒ stale); both `created_at` and mtime unusable ⇒ age unknown ⇒ NOT stale + warn; no clamping (Precondition 6(c) "Reservation timestamp rules") |
| EC-020 | (v1.7, first-activation race; S-25.08 finding) A protected-path PreToolUse `Edit`/`Write`/`MultiEdit` (valid `tool_use_id`) is admitted in a repository where `.factory/migration-state/` does NOT yet exist (no migration ever run); while that tool call is still in flight (PostToolUse not yet fired) a coordinator activates (creates the directory, flips `gate_state = DRAINING`, polls `reservations/`) | The admitter created `migration-state/reservations/` (idempotently) and `reservations/<tool_use_id>.reservation` BEFORE reading the gate (Precondition 6(c) "Unconditional reservation namespace"); the coordinator's C2 therefore lists the reservation and WAITS — `source_sha256` is not computed while it exists; PostToolUse removes it and the drain reaches quiescence, or (writer exceeds 30 s) `DRAIN_TIMEOUT_ABORT` with all four files byte-identical. The §Decision 7c step-5 fingerprint abort is NOT the mechanism that handles this case. Concurrent first-ever admissions racing on directory creation all succeed (idempotent create; no `EEXIST` error). A payload with no `tool_use_id` still degrades to check-only (no directory, no reservation). Directory/reservation creation failure (EACCES/EROFS) ⇒ `HookResult::Error` fail-closed `E-MAINTENANCE-002` (`io`, v1.8), no reservation left behind, never an untracked admit |
| EC-021 | (v1.8, F-001) A protected-path `Edit`/`Write`/`MultiEdit` is admitted (reservation `reservations/<tool_use_id>.reservation` created), the tool call then FAILS, is interrupted or is cancelled, so the harness sends `PostToolUseFailure` (NOT `PostToolUse`) for the same `tool_use_id` — including (a) `is_interrupt: true`, (b) an envelope with NO `tool_name`, (c) an envelope with a differently-shaped `tool_name` | The reservation is REMOVED (exactly that file; keyed only on `tool_use_id`, no `tool_name` filter); `is_tool_completion_event("PostToolUseFailure")` is true; a `PostToolUseFailure` for an id with no reservation file, or with an invalid-grammar id, is a silent no-op; a release error other than ENOENT is a non-fatal warn, never a verdict; no new `EventType` variant. If neither Post event ever arrives the reservation leaks until TTL (documented residual) |
| EC-022 | (v1.8, F-004; O1–O4) Real dispatcher PreToolUse for a protected-path write with a txn STAGING/COMMITTING (or gate DRAINING/LOCKED) and: (a) `CLAUDE_PLUGIN_ROOT` unset or empty; (b) the registry file missing; (c) the registry unparseable; (d) the registry schema-version mismatched; (e) an empty matched-plugin set; and separately (f) a registry that makes the dispatch fail CLOSED (non-zero) AFTER an admitted verdict; (g) an unparseable stdin payload | (a)–(e) The gate still enforces: blocked `E-MAINTENANCE-001` exit 2 without loading the registry (and, gate OPEN/no txn, admitted with the reservation standing; the matching PostToolUse/PostToolUseFailure still releases under the same broken registry); (f) the reservation created by admission is removed before exit (release-on-block funnel covers registry fail-closed exits); (g) the existing parse-error exit is unchanged, no reservation, no classification; in every case the core runs EXACTLY ONCE (one reservation write) and BEFORE `shard_cap_precheck` |
| EC-023 | (v1.8, F-002) A protected-LOOKING path outside the session's `factory_root`: (a) another project's `.factory/cycles/…` or `.factory/specs/behavioral-contracts/…`; (b) a nested project's `project_root/sub/.factory/cycles/…`; (c) a scratch tree; (d) a look-alike `x.factory/cycles/…` / `.factory-old/cycles/…`; and separately (e) `<project>/.factory` is a SYMLINK to a real directory; (f) the project has NO `.factory` directory; (g) (v1.9, lexical root spellings; the "vector (f)" of ADR-052 v1.20 Downstream item 21, lettered (g) here because (f) is taken) the project directory is reached through a SYMLINK spelling (macOS `/var/…` vs canonical `/private/var/…`; a symlinked checkout) and the PreToolUse `file_path` is spelled through the NON-canonical `CLAUDE_PROJECT_DIR` (as given), with `T_real` UNAVAILABLE (unresolvable-ancestor seam) | (a)–(d) Admitted unconditionally: NO reservation, NO directory creation (no `migration-state/`, no `reservations/` anywhere), NO read of migration-state, even with a txn STAGING/COMMITTING in the session's own tree (debug log only); (e) `factory_root` is the REAL directory — protected writes through either path spelling are gated/reserved in that one migration-state; (f) the gate is out of scope: the write is admitted, NO `.factory` (and nothing under it) is created; (g) IN SCOPE via the as-given lexical spelling (b) even though `T_real` is unavailable: blocked `E-MAINTENANCE-001` while a txn is live, else admitted with a reservation, and the reservation lands in the canonical `<factory_root_real>/migration-state/reservations/` (never under the as-given spelling) |
| EC-024 | (v1.8, F-003) Path aliasing of a protected target while a txn is live (or gate OPEN with the reservation observed): (a) `.factory/specs/./behavioral-contracts/x.md`; (b) `.factory/specs/../specs/behavioral-contracts/x.md`; (c) `.factory//cycles//c1//log.md`; (d) a symlink alias to `.factory/cycles` from elsewhere in the tree; (e) `<tmp>/link/../cycles/c1/log.md` where `link` is a symlink to a directory elsewhere (POSIX resolves `link` BEFORE `..`); (f) a nonexistent tail `.factory/cycles/newcycle/new.md` (deepest existing ancestor resolved, tail appended lexically); (g) mixed-case family names `.FACTORY/Cycles/x`, `.factory/Specs/Behavioral-Contracts/x.md` on a case-sensitive volume; (h) a relative `file_path` with the payload `cwd` set; (i) a `\`-containing name on Unix; (j) an unresolvable component (EACCES/ELOOP/hop limit >40); (k) (v1.9) the SAME protected target spelled once via the canonical root form and once via the as-given `CLAUDE_PROJECT_DIR/.factory` form (symlinked project directory); (l) (v1.9) a look-alike / other-project path that merely shares a symlink-ancestor spelling with NEITHER alias of the session's own root (e.g. a sibling project reached through the same symlinked parent, `<as-given-parent>/other/.factory/cycles/…`, `<as-given-root>-old/cycles/…`) | (a)–(d),(f),(g),(h),(j) in scope: blocked `E-MAINTENANCE-001` with the scope token of the path FAMILY (`BC-INDEX` / `.factory/cycles/`) while a window is active, else admitted with a reservation; (e) classified by the REAL resolved path (`link` resolved first), AND in scope iff either the resolved or the lexical form matches (union, fail-closed); (j) classified on `T_lex` alone (fail-closed); (i) `\` is an ordinary name byte on Unix (no separator rewrite); case comparisons are always case-insensitive with no filesystem probe; a missing/non-string/empty/NUL `file_path` is out of scope, not an error; `~` is never expanded; (k) classified IDENTICALLY (same in-scope verdict, same family/scope token, same block-or-reserve outcome, same canonical migration-state) under both spellings; (l) OUT OF SCOPE (no over-match): admitted, NO reservation, NO directory creation, NO migration-state read |
| EC-025 | (v1.8, F-008; extends EC-019) Reservation timestamp cases at drain step-1 GC with TTL 3,600 s (seam) and `now` fixed: `created_at` (a) `now + 299 s`; (b) `now + 301 s` with mtime `now − 4,000 s`; (c) before 1970-01-01T00:00:00Z (e.g. `1969-12-31T23:59:59Z`) with mtime `now − 4,000 s`; (d) non-RFC-3339 text (e.g. `"yesterday"`, `"2026-13-45"`); (e) year 9999 (`9999-12-31T23:59:59Z`); (f) a value outside `u64` epoch seconds; (g) valid `created_at` with an RFC 3339 non-UTC offset; and `created_at` unusable with (h) mtime in the future; (i) mtime pre-epoch/unavailable | (each `warn X` below denotes exactly one coordinator-drain STDERR advisory line with token X — NOT a dispatcher event; v1.10-rev, item 33(c)) (a) accepted skew, age 0 ⇒ NOT stale; (b) untrusted ⇒ mtime basis ⇒ stale (age 4,000 s > ttl) + warn `created_at_future`; (c) unparseable (never clamped to 0) ⇒ mtime basis ⇒ stale + warn `created_at_pre_epoch`; (d) unparseable ⇒ mtime ⇒ warn `created_at_unparseable`; (e) `> now + 300` ⇒ untrusted ⇒ mtime + warn `created_at_future`; (f) unparseable ⇒ mtime; (g) normalised to UTC epoch seconds, sub-second truncated; (h) age 0 ⇒ NOT stale + warn `mtime_future`; (i) both `None` ⇒ age UNKNOWN ⇒ NOT stale + warn `age_unknown` (the merged `epoch_secs(pre-epoch) = 0 ⇒ reclaimable` is removed); `reservation_is_stale` has signature `(Option<u64>, Option<u64>, u64, u64) -> bool`, constant `RESERVATION_CLOCK_SKEW_TOLERANCE_SECS = 300` |
| EC-026 | (v1.8, F-009 part 2) Protected-path PreToolUse `Edit`/`Write`/`MultiEdit` whose `tool_use_id` is: (a) a JSON number; (b) the empty string; (c) `../x`; (d) a 129-character string; (e) starts with `.` (e.g. `.hidden`); (f) a boolean/array/object; versus (g) ABSENT; (h) JSON `null`; (i) a valid id of exactly 128 characters (e.g. `toolu_` + 122 alphanumerics) or one containing `-`, `_`, `.` after the first character | (a)–(f) FAIL CLOSED: `E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)` (`HookResult::Error`, exit 2), NO reservation created, nothing admitted, NO directory created for that call, the error/log carries the byte length only never the raw value; (g)(h) check-only admission (no directory, no reservation, admitted if gate OPEN and no live txn); (i) valid, reservation created; the release leg treats an invalid id as a silent no-op |
| EC-027 | (v1.8, F-006) A live txn of the OTHER known migration — `migrate-bc-index` (or absent `migration_id`), STAGING with `generation_id` set, or COMMITTING — with ITS OWN terminal record (`completed.json`) ABSENT, while `completed-backfill-append-logs.json` IS present and valid; dispatcher PreToolUse under either path family | `txn_migration_known = true`; the core consults ONLY the txn's own migration's record ⇒ decision row 3 `NoOp` (no txn write, no gate write; the mechanism-A record is NEVER consulted); falls through to Branch B only if STAGING with `generation_id = null`, else the ordinary admission decision blocks with the PLAIN `E-MAINTENANCE-001` (no mismatch suffix); NOT `RefuseForeignMigration`, NOT finalized, NOT a mismatch; migration-state byte-identical |
| EC-028 | (v1.8, F-006) Same live txn of the OTHER known migration, COMMITTING, but with ITS OWN terminal record present and verifying (parses; `txn_id == activation_id`; `generation_id` equal; `canonical_paths_count == N` of THAT migration; every canonical `sha256 == expected_post_hash`) | Branch C finalizes it exactly as for its own migration: txn → COMPLETED THEN gate → OPEN, then the write is admitted — selected by `migration_id`, independent of whether this migration's record exists; the finalize is never performed on the strength of the other migration's record |
| EC-029 | (v1.8, F-006) A live txn whose `migration_id` is a string NOT in K (e.g. `"future-migration"`, a 1,000-character id, an id with control characters), STAGING (`generation_id` null and, separately, non-null) or COMMITTING, any terminal record present or absent | FOREIGN ⇒ `RefuseForeignMigration` (decision row 2, `txn_migration_known = false`): plain `E-MAINTENANCE-001` block (no mismatch suffix); NEVER finalized, NEVER aborted — Branch B's null-generation discard does NOT run; no Branch A/B/C repair; migration-state byte-identical; exactly one `migration.admission_blocked` InternalLog event (`branch = foreign_migration`, `reconciliation = foreign_migration_refused`) is written, and the raw id appears only in its `migration_id` field, sanitized — truncated to 64 chars with control characters escaped (v1.10; Postcondition 10) |
| EC-030 | (v1.8, F-006/F-009) A live txn record whose `migration_id` is NOT a JSON string (number, JSON `null`, array, object, boolean — a PRESENT non-string value; only an ABSENT field defaults to `migrate-bc-index`) | Malformed record ⇒ admission error `E-MAINTENANCE-002: writer-admission check failed (state_integrity)` (`HookResult::Error`, exit 2), fail-closed, NOT classified "foreign", no reservation left behind, no txn/gate write |
| EC-031 | (v1.8, F-009 part 1; reworded v1.10, F-006) The production entry delegates to the crate-private `run_bc_index_migration_with_ttl(project_root, max_reservation_ttl)` passing `DEFAULT_MAX_RESERVATION_TTL`; that function's FIRST statement is `validate_production_reservation_ttl`. Inputs: `max_reservation_ttl` below the 1,800 s floor (120 s, 1,799 s); exactly 1,800 s and 3,600 s; the injectable test seam `drain_bc_index_writers` given a small TTL; and the compile-time default-constant check | Below floor: `BcIndexMigrationError::ReservationTtlBelowFloor { configured_secs, floor_secs: 1800 }` (`RESERVATION_TTL_BELOW_FLOOR`, exit 2) returned BEFORE any gate or drain action — nothing mutated (a byte-identical `migration-state/` snapshot; no gate write, no txn, no reservation GC, no `exclusive.lock` created, no flock); NOT `BINARY_INTEGRITY_FAILURE`; 1,800 s and 3,600 s accepted (proceed); `drain_bc_index_writers` is NOT bound by the floor; `DEFAULT_MAX_RESERVATION_TTL >= MIN_PRODUCTION_RESERVATION_TTL` is enforced at COMPILE time by `const _: () = assert!(…)`. The seam is crate-private — NOT `pub`, NOT environment/argv-injectable (an operator-controlled TTL would defeat the floor). The same entry shape (crate-private `…_with_ttl` seam, first-statement floor validation) applies to the `backfill-append-logs` coordinator (S-25.06) |
| EC-032 | (v1.8, S-25.08 AC-018 ambiguity; Precondition 6(c) "`E-MAINTENANCE-002` `<cause>` classification") An admission-time read of `gate-state.json` (or a `txn-*.json` record, or the Branch C terminal record) that FAILS (OS call error other than ENOENT) versus one that SUCCEEDS but yields unusable content (non-UTF-8, empty/truncated/unparseable, wrong JSON type/shape, unknown state value, non-string `migration_id`, >1 live txn) | OS read failure ⇒ `E-MAINTENANCE-002 (io)`; bytes read but unusable ⇒ `E-MAINTENANCE-002 (state_integrity)`; read failure wins when both would apply (content never examined after a failed call); terminal record: read failure ⇒ `io`, unverifiable content (unparseable, wrong schema, count/id/hash mismatch) ⇒ NOT `E-MAINTENANCE-002` (v1.9: `E-MAINTENANCE-001` WITH the completion-record-mismatch suffix, not finalized); a canonical file read during Branch C verification (v1.11, ADR-052 v1.22 "Read-failure mapping"): read-call failure other than ENOENT ⇒ `io`, ENOENT ⇒ the `canonical_hash_mismatch` check; reservation files are never read by admission; first-failure-wins in the order tool_use_id → W1 → gate-state → txn (ascending filename) → terminal record; always fail-closed, no reservation left behind, no writes (orchestrator-confirmed: create failure of a reservation = `io`; a terminal record that reads but does not verify stays an `E-MAINTENANCE-001` block carrying the mismatch suffix, v1.9); **(v1.10, F-012)** the `state_integrity` carrier is `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }`, NOT `BinaryIntegrityFailure`, with `kind` ∈ {`gate_record_malformed` (gate-state content unusable), `txn_record_malformed` (txn record unparseable / not an object / unknown `state`), `txn_migration_id_not_string`, `multiple_live_txns`, `reservation_serialization`}; its Display `migration admission: state integrity failure (<kind-token>): <detail>` does NOT contain `BINARY_INTEGRITY_FAILURE`; each failing fixture writes exactly one `migration.admission_failed` event carrying `cause` and, for `state_integrity`, the matching `kind` (Postcondition 10; Invariant 7) |
| EC-033 | (v1.10, D-2) The process cwd of the `backfill-append-logs` invocation differs from `CLAUDE_PROJECT_DIR=<P>` (the harness launched the Bash tool from a subdirectory, a worktree or another shell cwd), and admission under the same env left a reservation `T1` | The coordinator operates on — and creates `migration-state/` only under — `<P>/.factory`; its first drain observes `<P>/.factory/migration-state/reservations/T1.reservation` (so `T1` blocks quiescence until its Post event or the drain timeout); NOTHING is created under the process cwd or any other directory; no `join(".factory/…")` literal outside the resolver (sibling-sweep gate, Invariant 8) |
| EC-034 | (v1.10, D-2) `resolve_session_project_root(claude_project_dir, process_cwd)` over: (a) `Some("<abs existing dir>")`; (b) `Some("")`; (c) `None`; (d) `Some("<abs nonexistent path>")` (canonicalize fails); (e) `Some("<symlink to dir>")`; (f) `process_cwd` inside a git work tree whose root has a `.factory` while `process_cwd` itself does not | (a) the canonicalized path; (b) and (c) exactly `process_cwd` (empty ≡ absent); (d) the as-given path — NEVER `process_cwd`; (e) the canonicalized (symlink-resolved) path; (f) `process_cwd` exactly — NO ancestor walk, NO `git rev-parse`; the function is pure (env read in `main.rs`, rule in the function — testable without environment mutation) |
| EC-035 | (v1.10, D-2; v1.12 ADR-052 v1.23) The resolved project root has NO `.factory` directory — ABSENT, the closed set of Precondition 6(b)(iii): `.factory` missing (`ENOENT`), a regular file (or other non-directory), a DANGLING `.factory` symlink (`ENOENT`), or a project root that is itself a regular file (`ENOTDIR`) — when `backfill-append-logs` (or `migrate-bc-index`) runs. (A `stat` failure of any OTHER kind is NOT this case: EC-043.) | Exit 2 `FACTORY_ROOT_NOT_FOUND` (`BcIndexMigrationError::FactoryRootNotFound { project_root, root_source }`, `root_source: Option<ProjectRootSource>`) with the normative stderr line `<subcommand>: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root> (resolved from <source>)`, `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`} — on the CLI the ` (resolved from <source>)` suffix is ALWAYS present (rule (a), Precondition 7: the CLI threads the whole `SessionProjectRoot`); only a path-only library entry (`run_bc_index_migration(&Path)` / crate-private `run_bc_index_migration_with_ttl(&Path, ttl)`) yields `root_source = None` and the suffix-less `Display` (`FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root>`); nothing created or mutated — no `.factory`, no `migration-state/`, no `exclusive.lock`; the directory tree is byte-identical before and after; the coordinator never falls back to another directory |
| EC-036 | (v1.10, D-2) `<P>/.factory` is a symlink to a real factory directory `R/real-factory`, with admission and the coordinator each running under `CLAUDE_PROJECT_DIR=<P>` | `resolve_factory_root` follows the symlink: admission's reservations and the coordinator's drain/gate/txn all live in the ONE real `R/real-factory/migration-state/`; no second namespace is created under `<P>/.factory` |
| EC-037 | (v1.10, D-1) A protected-path write is blocked `E-MAINTENANCE-001` in each branch: gate-only (DRAINING/LOCKED, no txn); live txn (STAGING / COMMITTING, terminal record absent); live coordinator (EWOULDBLOCK); foreign migration (`migration_id ∉ K`); Branch C verification failure | Exactly ONE `migration.admission_blocked` event is appended to `dispatcher-internal-YYYY-MM-DD.jsonl` per blocked dispatch (written before the early return), with `scope`/`family` matching the written path family, `branch` ∈ {`gate_only`, `live_txn`, `live_coordinator`, `foreign_migration`, `completion_record_mismatch`} respectively, `gate_state`, `migration_id`/`txn_id` (sanitized; `null` for gate-only), `check` (non-null only for the mismatch branch), `reconciliation`; the stderr message is unchanged; no field contains a raw `tool_use_id` or record content; the same assertions read the JSONL file, never a `tracing` capture |
| EC-038 | (v1.10, D-1) A protected-path write fails `E-MAINTENANCE-002` for each cause: `invalid_tool_use_id` (e.g. number / `../x`); `io` (e.g. `gate-state.json` is a directory); `state_integrity` (e.g. `gate-state.json` containing `"BOGUS"`; two live txns) | Exactly ONE `migration.admission_failed` event per failed dispatch with `cause` = the cause token; `kind` = the `AdmissionStateIntegrityKind` token when `cause = state_integrity` (e.g. `gate_record_malformed`, `multiple_live_txns`) and `null` otherwise; `detail` = path / `io::Error` kind+message / byte length; NEVER the raw `tool_use_id` (byte length only) and NEVER record content |
| EC-039 | (v1.10, D-1; reworded v1.10-rev per item 33(c)/(d)) Non-verdict anomalies: (a) a non-ENOENT reservation-release error; (b) Branch A gate re-open; (c) Branch B null-generation txn discard; (d) `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` while the finalize effect is undelivered (S-25.09 seam); (e) S-25.06's successful verify-then-finalize; (f) a reservation-timestamp fallback at drain GC | (a)–(e): exactly ONE `migration.admission_advisory` event per anomaly with `reason` = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired` / `branch_c_finalized` respectively; the reason domain is exactly these five; (d) is written IN ADDITION to the one `migration.admission_blocked`; (e) is admitted so there is NO `_blocked`, and neither S-25.08 nor S-25.09 emits it; optional context fields only from {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}, any other field forbidden; no raw `tool_use_id`; the dispatch outcome of (a)–(c) is unchanged. (f): NO dispatcher event — the five timestamp tokens are coordinator-drain stderr lines only |
| EC-040 | (v1.10, F-013) A protected-path write blocked/errored by the migration admission gate versus a write blocked by `shard_cap_precheck` | The admission verdict's stderr summary line carries `blocking_plugins=migration-admission` (`block_reason` = the unchanged `E-MAINTENANCE-001`/`E-MAINTENANCE-002` text, exit 2); the shard-cap block still carries `blocking_plugins=shard-cap-gate` (`NativeGate::MigrationAdmission` vs `NativeGate::ShardCap`) |
| EC-041 | (v1.12, F-S2508-L3-009) ADMISSION leg: PreToolUse `Edit`/`Write`/`MultiEdit` with a valid `file_path` and valid `tool_use_id` while the `stat` of `<project_root>/.factory` is UNSTATABLE: (a) `chmod 000` project root (EACCES); (b) `.factory` is a self-referential symlink (ELOOP); versus the ABSENT controls: (c) `.factory` a regular file; (d) dangling `.factory` symlink; (e) project root that is itself a regular file (ENOTDIR); (f) no `.factory` | (a)(b): `E-MAINTENANCE-002: writer-admission check failed (io)` (`HookResult::Error`, exit 2), FAIL CLOSED — exactly ONE `migration.admission_failed` (`cause=io`, `kind=null`, `detail` = sanitized path + `ErrorKind` + message), NO `migration.admission_blocked`, NO reservation, nothing created, the write NOT admitted, `blocking_plugins=migration-admission`; (c)–(f): OUT OF SCOPE — admitted, no reservation, nothing created, no event; this holds for every written path spelling (the root's identity is unknown, so no path is classified) |
| EC-042 | (v1.12, F-S2508-L3-009) RELEASE leg: PostToolUse / PostToolUseFailure for a `tool_use_id` while the `.factory` `stat` is UNSTATABLE (EACCES, ELOOP); versus ABSENT | Unstatable: NO verdict; exactly ONE `migration.admission_advisory` (`reason=reservation_release_failed`, `detail` = sanitized path + `ErrorKind` + message); nothing created or deleted; a pre-existing reservation is left for the drain-start TTL GC; the dispatch outcome is unchanged. Absent: silent no-op, NO event |
| EC-043 | (v1.12, F-S2508-L3-009) COORDINATOR leg: `migrate-bc-index` and `backfill-append-logs` run under a project root whose `.factory` `stat` is UNSTATABLE (EACCES via `chmod 000` project root, ELOOP via self-symlink); versus ABSENT (EC-035) | Unstatable: exit 2 with the existing `BcIndexMigrationError::Io { path: <project_root>/.factory, source }` (its existing Display), NOT `FACTORY_ROOT_NOT_FOUND`; raised before any lock or write; the directory tree is byte-identical before and after (no `.factory`, no `migration-state/`, no `exclusive.lock`). Absent: `FACTORY_ROOT_NOT_FOUND` exactly as EC-035 |
| EC-044 | (v1.12, F-S2508-L3-001) FOREIGN minimal live record: `txn-*.json` = `{"state": "STAGING", "migration_id": "future-migration"}` and again with `"state": "COMMITTING"` (NO other field), under BOTH path families (`.factory/cycles/…` and `.factory/specs/behavioral-contracts/…`) and gate OPEN / DRAINING / LOCKED | PLAIN `E-MAINTENANCE-001` (path-family keyed message, no mismatch suffix) in all 12 combinations; NOT `E-MAINTENANCE-002`; no Tier 1 field read (foreign precedence); txn and gate byte-identical; exactly one `migration.admission_blocked` (`branch=foreign_migration`, `reconciliation=foreign_migration_refused`) |
| EC-045 | (v1.12, F-S2508-L3-001) KNOWN minimal STAGING WITHOUT `generation_id`: `{"state": "STAGING", "migration_id": "backfill-append-logs"}` (and with the `migration_id` key absent ⇒ `migrate-bc-index`), gate DRAINING, `exclusive.lock` FREE, own terminal record ABSENT (Branch B execution conditions), under either path family | `E-MAINTENANCE-002 (state_integrity)`, `kind=txn_record_malformed` (`BcIndexMigrationError::AdmissionStateIntegrity`), exactly one `migration.admission_failed`; the txn record bytes are UNCHANGED (NOT rewritten to ABORTED), gate UNCHANGED, no reservation; an absent `generation_id` is NEVER read as null. Control: the same record with `exclusive.lock` HELD by a live coordinator ⇒ Branch B does not execute, no field is read ⇒ plain `E-MAINTENANCE-001` (`branch=live_coordinator`). Control: `"generation_id": 7` or `"generation_id": ["x"]` (any non-string, non-null type) ⇒ the same `state_integrity` as the absent key |
| EC-046 | (v1.12, F-S2508-L3-001) KNOWN STAGING with `"generation_id": null` (PRESENT, JSON null), Branch B execution conditions | UNCHANGED discard: txn rewritten in place to `ABORTED` + `abort_reason: "null_generation"`, gate → OPEN, the write admitted, one `migration.admission_advisory` `branch_b_txn_aborted`. Control: `"generation_id": "gen-1"` (string) ⇒ not Branch B ⇒ plain `E-MAINTENANCE-001` live block, txn unchanged |
| EC-047 | (v1.12, F-S2508-L3-001) KNOWN minimal COMMITTING: `{"state": "COMMITTING", "migration_id": "backfill-append-logs"}` (and `migrate-bc-index`), terminal record ABSENT, gate LOCKED, either path family | PLAIN `E-MAINTENANCE-001` (live block; `branch=live_txn`); NO Tier 1 field read (the `NoOp` arm needs none); txn and gate byte-identical; NOT `E-MAINTENANCE-002`. With its terminal record PRESENT the Branch C verification applies Tier 1 reads at each check's position (S-25.06 once wired; in the S-25.09 seam build no field is read and the seam tokens apply). **Binary leg (v1.12 extension; ADR-052 v1.23 "Migration binaries — recovery and finalize legs"):** the same minimal COMMITTING record WITH its terminal record present, as seen by the `backfill-append-logs` verifier/finalize or the `migrate-bc-index` §4e reconciliation (S-25.06 AC-031; fail-closed until then — the interim `migrate-bc-index` short-circuit refuses per BC-1.18.011 Postcondition 9(e) / EC-050 and performs no verification) (no PreToolUse): (a) a Tier 1 field the check consumes is malformed (`activation_id` absent or a number; `generation_id` absent or ill-typed, evaluated only after the `txn_id` check passes) ⇒ exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`, stderr names it, `migration-state/` byte-identical, no finalize, no gate flip; (b) all consumed fields well-formed but disagreeing (`txn_id` ≠ `activation_id`, `generation_id` ≠, count ≠ N, hash ≠) ⇒ exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`, same zero-mutation guarantee; (c) ordering: a `txn_id` mismatch alongside an ABSENT `generation_id` ⇒ `COMPLETION_RECORD_MISMATCH_ABORT` (the earlier check wins). Admission analogue of (a): `E-MAINTENANCE-002 (state_integrity)`, `txn_record_malformed`; of (b)/(c): `E-MAINTENANCE-001` with the mismatch suffix |
| EC-048 | (v1.12, F-S2508-L3-001) Minimal NON-LIVE records of valid Tier 0 shape: `{"state": "COMPLETED", "migration_id": "<known>"}` and `{"state": "ABORTED", "migration_id": "<known>"}`, and the same with `migration_id` `"future-migration"` (foreign), no other field; (a) gate OPEN, (b) gate stuck LOCKED / DRAINING with no live txn; plus a fixture of one minimal non-live foreign record beside ONE live known txn, and one with two minimal non-live records and no live txn | (a) ADMITTED (gate OPEN and no live txn; `is_live` is STAGING / COMMITTING only); (b) Branch A reopens the gate (`branch_a_gate_reopened`) and the write is admitted; the non-live records are neither modified nor deleted; foreign non-live records are NOT refused (the foreign refusal applies to LIVE records only); the "more than one live txn" `state_integrity` counts live records only (the two-non-live fixture does NOT trigger it; two LIVE records, foreign included, DO: `kind=multiple_live_txns`) |
| EC-049 | (v1.12 extension, ADR-052 v1.23 binary leg) BINARY-leg Tier 1 malformation at the verifier: `backfill-append-logs` (verifier/finalize) and `migrate-bc-index` (§4e reconciliation — S-25.06 AC-031; fail-closed until then, BC-1.18.011 EC-050) run with a live COMMITTING txn whose own terminal record is PRESENT and well-formed, and a txn field the executing check consumes is bad: (a) `activation_id` key removed; (b) `activation_id` a number; (c) `activation_id` valid and matching but `generation_id` key removed; (d) `generation_id` a number; versus (e) all fields well-formed but `txn_id` ≠ `activation_id`; (f) a `txn_id` mismatch AND `generation_id` key removed; (g) a STAGING txn with the terminal record present and Tier 1 fields absent | (a)(b)(c)(d): exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` (`AdmissionStateIntegrity`, `txn_record_malformed`), stderr names the code, NO finalize, NO gate flip, byte-identical `migration-state/` snapshot; (e): exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`, same zero-mutation guarantee; (f): `COMPLETION_RECORD_MISMATCH_ABORT` (the `txn_id_mismatch` check is earlier, so it wins over the later malformation); (g): `COMPLETION_RECORD_MISMATCH_ABORT` (`staging_with_terminal_record` reads no field). Admission analogue of (a)–(d): `E-MAINTENANCE-002 (state_integrity)`, `txn_record_malformed`, one `migration.admission_failed`; of (e)–(g): `E-MAINTENANCE-001` with the mismatch suffix |
| EC-050 | (v1.12 extension; v1.13 adds the plan arm, ADR-054 §Decision 3 B-3 / §Decision 2.4) `intent_log_path` ABSENT, JSON `null` or ill-typed (number, array) on a live COMMITTING txn whose own terminal record is present and passes every check before `canonical_hash_mismatch` (parses; `txn_id`, `generation_id`, `canonical_paths_count` agree); (v1.13) likewise `canonical_move_plan` ABSENT, `null`, not an array, or an EMPTY array `[]` (with `canonical_paths_count` 4); (v1.13, pairing rule of Precondition 5) at the COORDINATOR arm entry (STAGING with a string `generation_id`, or COMMITTING) `intent_log_path` null / absent / non-string / ≠ `.factory/migration-state/intent-<generation_id>.log` — controls: the correct pair, and both `generation_id` and `intent_log_path` `null` | Coordinator arm entry, the pairing violations: exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` (`txn_record_malformed`) before any mutation, value never derived or filled in; controls recover (the both-null pre-generation record takes the null-generation discard). Verifier/admission: Evaluated AT the `canonical_hash_mismatch` check: admission ⇒ `E-MAINTENANCE-002 (state_integrity)`, `txn_record_malformed`; binary ⇒ exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`; nothing mutated on either surface; an EMPTY plan NEVER verifies vacuously (never finalizes with `canonical_paths_count` ≠ `len(plan)`). Control: with an EARLIER mismatch (e.g. `canonical_paths_count` ≠ N) the earlier mismatch wins ⇒ `COMPLETION_RECORD_MISMATCH_ABORT` / mismatch-suffixed `E-MAINTENANCE-001` and `intent_log_path` is never read |
| EC-051 | (v1.12 extension; v1.13 adds the hardened-log and plan vectors, ADR-054 §Decision 1.7/1.8, §Decision 3 B-2/B-3) `intent_log_path` is a well-formed string but the intent log is MISSING (ENOENT), or exists but has no DONE record for a canonical path (a TORN TAIL = absent), or its DONE `expected_post_hash` ≠ the file's `sha256`; (v1.13) or the DONE's `expected_post_hash` ≠ the INTENT's (even if equal to the file's hash: circular DONE); or a record in the MIDDLE of the log fails its checksum/grammar and a later valid record follows (mid-log corruption), or L1-L4 is violated (foreign `txn_id`, conflicting INTENTs, DONE without a prior INTENT, decreasing `fencing_generation`); or `canonical_move_plan` is a well-formed non-empty array whose `canonical_path` set differs from the INTENT target set by one entry (missing or extra), or one pair's `staging_path` differs | `canonical_hash_mismatch` (the unverifiable-hash class, NOT `state_integrity`): admission ⇒ `E-MAINTENANCE-001` with the mismatch suffix and `check=canonical_hash_mismatch`; binary ⇒ exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`; no finalize, no gate flip. The torn-tail control (a valid prefix holding all four INTENT+DONE pairs followed by garbage / NUL bytes / invalid UTF-8) VERIFIES — the tail is absent, not corruption. On the COORDINATOR leg (recovery/forward finish) the same mid-log corruption is exit 2 `INTENT_LOG_CORRUPT` and the differing plan is exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` `txn_record_malformed` (BC-1.18.011 Postconditions 14-15). stderr of the coordinator-leg `INTENT_LOG_CORRUPT` is `backfill-append-logs: ` + the ADR-054 §Decision 3.1 line (`INTENT_LOG_CORRUPT (exit 2): intent log <log_file> is corrupt (<kind>) at byte offset <offset>; no further move or append was made and no completion was recorded; operator investigation required`, no `BC-INDEX migration: ` label; "nothing moved or appended" holds only when the read precedes the move loop — at the post-loop re-read this run's moves have already happened, `completed.json` is not written and the txn is untouched) |
| EC-052 | (v1.12 extension; v1.13: read through the shared ADR-054 §Decision 1.7 reader, whose only I/O is the single read) The intent-log READ CALL fails with anything other than ENOENT (EACCES, EISDIR, EIO, short read) while `canonical_hash_mismatch` executes | Admission ⇒ `E-MAINTENANCE-002 (io)` (one `migration.admission_failed`, no `migration.admission_blocked`, no tenth `check` token); binary ⇒ `BcIndexMigrationError::Io` exit 2; content never examined; txn neither finalized nor discarded; `migration-state/` byte-identical |
| EC-053 | (v1.12 third binary-leg extension; ADR-052 v1.23 item 11(c); Precondition 5/6 contention clause, Postcondition 5a(a) — `backfill-append-logs` contention on EVERY path, never exit 0; mirrors BC-1.18.011 EC-048/EC-051) `backfill-append-logs` BINARY while ANOTHER coordinator holds `flock(exclusive.lock, LOCK_EX\|LOCK_NB)`, with `completed-backfill-append-logs.json` PRESENT and: (a) no txn record; (b) a COMPLETED own txn; (c) the OWN live COMMITTING txn; (d) a live FOREIGN record (`migrate-bc-index` or `future-migration`); and with `completed-backfill-append-logs.json` ABSENT and (e) a live own txn. Uncontended CONTROL on (c): flock free | (a)–(e): exit **1** `AppendLogMigrationError::LockContention` / `MIGRATION_LOCK_CONTENTION` — NEVER exit 0 and never `ALREADY_MIGRATED`; nothing read and nothing written (sha256 snapshot of `migration-state/` byte-identical; `completed*.json` not opened); (d) is contention, never `FOREIGN_MIGRATION_REFUSED`; exit 1 is the "no harm done, safe to re-run" class. Control (c): exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` or the verify-then-finalize outcome per Postcondition 5a — the same on-disk state never yields a timing-dependent verdict; the earlier "warning line, exit 0 `ALREADY_MIGRATED`" is REJECTED |
| EC-054 | (v1.12 third binary-leg extension; ADR-052 item 11(c); Postcondition 5a(a2) — terminal record read UNDER the lock, TOCTOU) Via the `Fs` seam, `completed-backfill-append-logs.json` is ABSENT at the pre-lock probe; a running coordinator then writes it, marks its txn COMPLETED and releases the lock; the late `backfill-append-logs` acquires the lock | The terminal record is read UNDER the lock and that read is what the recovery decision receives (NOT a hard-coded `None`): NO fresh run starts (gate not moved to LOCKED/DRAINING, the four canonical files not re-read), the gate is untouched, no txn is created, exit 0 `ALREADY_MIGRATED` (a state read under the lock); `migration-state/` snapshot unchanged. The pre-lock probe never selects the branch |
| EC-055 | (v1.12 third binary-leg extension; ADR-052 item 10(b)(c)(d); Precondition 6(c) rule 9 — Branch B version gate, ADMISSION surface; mirrors BC-1.18.011 EC-055) Pre-generation record `{"state":"STAGING","migration_id":"backfill-append-logs","generation_id":null}` (and with `migration_id` absent ⇒ `migrate-bc-index`), gate DRAINING, `exclusive.lock` FREE, own terminal record ABSENT, PreToolUse `Write` under either path family, with `schema_version`: (a) `2`; (b) `4294967296`; (c) `"1"`; (d) `0`; (e) `1.5`; (f) `null`; (g) ABSENT (CONTROL); (h) `1` + unknown key `"x_extra":7` (CONTROL); (i) `2` + `generation_id` key REMOVED (newer-schema wins over the malformed tri-state) | (a)(b)(i): exit 2 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`, exactly ONE `migration.admission_failed` (`cause=state_integrity`, `kind=txn_record_newer_schema`), NO `migration.admission_blocked`, no reservation left behind, txn bytes UNCHANGED (sha256 before == after; no `abort_reason`; NOT the discard), gate unchanged; (c)–(f): same surface with `kind=txn_record_malformed`, bytes unchanged; (g)(h): the unchanged Branch B discard (txn ABORTED + `abort_reason: "null_generation"`, gate OPEN, write admitted, one `branch_b_txn_aborted` advisory) and the unknown key of (h) is PRESERVED in the ABORTED record. The newer-schema `detail` never contains `txn_record_malformed`, `corrupt` or `BINARY_INTEGRITY_FAILURE` |
| EC-056 | (v1.12 third binary-leg extension; ADR-052 item 10(b); Precondition 6(c) rule 9 — Branch C version gate and lazy-read admission) Live COMMITTING `backfill-append-logs` txn with its OWN terminal record PRESENT and otherwise verifying, PreToolUse `Write` under `.factory/cycles/`, with `schema_version`: (a) `2`; (b) `4294967296`; (c) `3` AND `activation_id` removed; (d) ABSENT (CONTROL); (e) `1` (CONTROL); (f) `"2"`. Plus the minimal hand-built record `{"state":"COMMITTING","migration_id":"backfill-append-logs"}` with the terminal record ABSENT (control: no Tier 1 read) | (a)(b)(c): `E-MAINTENANCE-002 (state_integrity)`, ONE `migration.admission_failed` `kind=txn_record_newer_schema`, evaluated BEFORE every Branch C check (the newer-schema record is NOT reported as a mismatch or as a missing `activation_id`), txn and gate byte-identical, no `migration.admission_blocked`; (d)(e): the Branch C checks run normally ("not consumed" for an absent `schema_version`); (f): `kind=txn_record_malformed`; minimal record: PLAIN `E-MAINTENANCE-001` with no Tier 1 read. Binary leg (S-25.06): a `schema_version` ≥ 2 at the verifier ⇒ exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` (`txn_record_newer_schema`), nothing finalized, no gate flip |
| EC-057 | (v1.12 third binary-leg extension; ADR-052 item 7(e); exit-code classes — `backfill-append-logs`) A compile-time-exhaustive table test enumerating EVERY `AppendLogMigrationError` variant with its expected `process_exit_code` (no wildcard row) | `LockContention` ⇒ 1 (and `ExpiryAbort`-equivalent when S-25.06 maps it); every other named variant ⇒ 2; `_ =>` absent; adding a variant without a row fails the build (S-25.06 when it lands; the `migrate-bc-index` twin is BC-1.18.011 EC-057) |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| All four `v1.0-brownfield-backfill` files at their F4-measured oversized byte counts, no prior migration-state artifacts present | `backfill-append-logs` invoked once: all four files split per BC-1.18.008's algorithm; per-file content-preservation and census both pass; single CURRENT-backfill-append-logs.json pointer swap commits all four; `completed-backfill-append-logs.json` written; stdout census report lists all four files' pre/post record counts | happy-path |
| `backfill-append-logs` invoked a second time after the above completes | `ALREADY_MIGRATED` (exit 0); zero filesystem mutation; no lock acquired (EC-004) | happy-path |
| `lessons.md`'s content-preservation check finds a byte-count mismatch after staging; the other three files stage cleanly | Migration ABORTS for ALL FOUR files (`CONTENT_PRESERVATION_ABORT`, exit 2); `decision-log.md`, `burst-log.md`, and `session-checkpoints.md` remain in their ORIGINAL monolithic form despite having staged cleanly themselves (EC-001) | error |
| Migration crashes between the CURRENT-backfill-append-logs.json pointer swap and the canonical rename of `session-checkpoints.md` (the last of the four in fixed order) | Re-run detects txn state COMMITTING; forward recovery completes the remaining rename via the intent log's matching-destination-hash rule; `decision-log.md`/`burst-log.md`/`lessons.md` (already moved) are untouched; `completed-backfill-append-logs.json` written once all four are verified (EC-002) | error |
| `backfill-append-logs /some/other/cycle/decision-log.md` (a path argument) | Rejected before execution by the pre-shell classifier; non-zero exit; no filesystem mutation (EC-008) | error |
| `backfill-append-logs --census` against an already-completed migration | Read-only census report to stdout reproducing the original activation's per-file record counts and content hashes; no lock, no manifest, no mutation | edge-case |
| (D3) Fixture: `completed-backfill-append-logs.json` present and valid, txn record `state=COMMITTING` with matching `activation_id`/`generation_id`, `gate-state.json=LOCKED`, four files at `expected_post_hash`; then invoke `backfill-append-logs` | Exit 0 `ALREADY_MIGRATED`; txn record now `state=COMPLETED`; `gate-state.json=OPEN`; a subsequent PreToolUse Edit to `.factory/cycles/v1.0-brownfield-backfill/burst-log.md` is admitted; a second invocation performs zero mutation (EC-009) | error |
| (D3) Same fixture, but a PreToolUse Edit to `.factory/cycles/<any>/burst-log.md` arrives with NO binary re-invocation | Self-heal runs first (Postcondition 5a(c)): txn → COMPLETED, gate → OPEN, then the Edit is admitted (not blocked) | error |
| (D3) Same fixture but `completed-backfill-append-logs.json.txn_id` ≠ txn `activation_id` (or one canonical file's hash differs) | Exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`; txn record still COMMITTING; gate still non-OPEN; Edit to `.factory/cycles/` still blocked `E-MAINTENANCE-001` (EC-010) | error |
| (D2) One reservation file `reservations/t1.reservation` with `created_at = now`, never removed; run drain with 30 s timeout injected as 200 ms | `DRAIN_TIMEOUT_ABORT` exit 2; txn `ABORTED`; gate `OPEN`; four files untouched; `source_sha256` never written (EC-011) | error |
| (D2) Reservation with `created_at = now − 4000 s` (TTL 3,600 s); run drain | Reservation removed at step 1; drain reaches quiescence; migration proceeds (EC-012a) | edge-case |
| (D2) Reservation present at drain start, removed (simulated PostToolUse) 100 ms later, before timeout | Drain proceeds; `source_sha256` computed only after the file vanished; migration completes | happy-path |
| (D1) Real dispatcher PreToolUse payload: `Write` to `.factory/cycles/v1.0-brownfield-backfill/lessons.md` with txn record `STAGING`; then the same with `.factory/STATE.md` | First: exit 2 `E-MAINTENANCE-001`; second: admitted (EC-013) | error |
| (D1) Real dispatcher PreToolUse (gate OPEN, no txn) for `Edit` `tool_use_id=T9` under `.factory/cycles/`, then PostToolUse `T9` | After PreToolUse: `reservations/T9.reservation` exists with `created_at` and `tool_use_id`; after PostToolUse: file absent; PostToolUse for an unknown id is a no-op (EC-013) | happy-path |
| (v1.4, D1 re-scope) Real dispatcher PreToolUse for tool `Bash` with command `echo x >> .factory/cycles/v1.0-brownfield-backfill/lessons.md`, and separately with `{project-root}/target/release/factory-dispatcher backfill-append-logs`, txn `STAGING` | Both: Rust gate returns `None`; no `reservations/*` file created; `.factory/migration-state/` byte-identical before/after; (the Rust gate neither blocks nor reserves — OBL-4 owns Bash) (EC-014) | edge-case |
| (v1.4, D5) Scripted admitter/coordinator at each interleave of {W1 create reservation, C1 DRAINING flip, C2 read `reservations/`, W2 read gate} (4 orderings consistent with W1<W2, C1<C2) | Each ordering: either C2 lists the reservation (coordinator waits, no `source_sha256`) or W2 reads `DRAINING` (admission blocked `E-MAINTENANCE-001`, own reservation removed); the "both miss" outcome is never produced (EC-017) | error |
| (v1.4, D2) Admission creates `reservations/T7.reservation`, then `shard_cap_precheck` (or a registry plugin) returns Block for the same event | Dispatcher exits 2; `reservations/T7.reservation` absent at exit (EC-018) | error |
| (v1.4, D2) Gate DRAINING or txn live, PreToolUse `Edit` `tool_use_id=T8` under `.factory/cycles/` | Blocked `E-MAINTENANCE-001`; no `reservations/T8.reservation` remains after the process exits (admitter removed its own) | error |
| (v1.4, D4) `completed.json` + `CURRENT.json` present for a finished `migrate-bc-index`; NO `completed-backfill-append-logs.json`; invoke `backfill-append-logs` | Not `ALREADY_MIGRATED`; runs as a first activation; `completed.json`/`CURRENT.json` byte-identical afterward (EC-015a) | edge-case |
| (v1.4, D4) `completed-backfill-append-logs.json` + `CURRENT-backfill-append-logs.json` present, NO `completed.json`; invoke `migrate-bc-index`; and run BC-INDEX `detect_migration_read_state` | `migrate-bc-index` not `ALREADY_MIGRATED`; reader state is NOT "complete" (EC-015b) | edge-case |
| (v1.4, D4) Live txn `{migration_id: "migrate-bc-index", state: COMMITTING}` (or no `migration_id` field); invoke `backfill-append-logs` (and the reverse fixture for `migrate-bc-index`) | Exit 2 `FOREIGN_MIGRATION_REFUSED` with the exact line of Precondition 5, zero mutation, no `recover()`; foreign txn untouched (EC-016) | error |
| (v1.4, D4) Live txn `{migration_id: "migrate-bc-index", state: COMMITTING}` with a VALID `completed-backfill-append-logs.json` present, PreToolUse Edit under `.factory/cycles/` | NOT finalized by Branch C on that record; Edit blocked `E-MAINTENANCE-001`; txn unchanged (Postcondition 5a foreign-migration guard; v1.8: on the dispatcher path this is NOT a foreign refusal — the live `migrate-bc-index` txn is decided against ITS OWN record `completed.json`, ABSENT ⇒ row 3 `NoOp` ⇒ plain block, EC-027; the binary-side refusal is EC-016, binary recovery path only) | error |
| (v1.4, D3 PreToolUse analogue) Branch C verification fails (e.g. `canonical_paths_count` = 3) on a PreToolUse Edit, no binary re-invocation | `E-MAINTENANCE-001` block whose message is the keyed format string plus ` (completion-record mismatch — operator investigation required)` (v1.6); reason logged names `canonical_paths_count`; txn/gate/terminal record byte-identical (EC-010) | error |
| (v1.6, keying — four-cell matrix) Gate LOCKED, live txn COMMITTING with no terminal record (plain block); PreToolUse `Edit` for each of (a) `.factory/cycles/c1/burst-log.md` with txn `migration_id=backfill-append-logs`, (b) same path with `migration_id=migrate-bc-index`, (c) `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md` with `backfill-append-logs`, (d) same path with `migrate-bc-index` | Exit 2 `E-MAINTENANCE-001`; stderr contains exactly: (a)(b) `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts`; (c)(d) `BC-INDEX write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts`; no `completion-record mismatch` text; txn/gate byte-identical (EC-013, Precondition 6(b) keying rule) | error |
| (v1.6, keying — gate-only) Gate DRAINING, NO txn file, `exclusive.lock` held by a live coordinator; PreToolUse `Edit` under `.factory/cycles/` and separately under `.factory/specs/behavioral-contracts/` | Exit 2; messages are exactly the `.factory/cycles/ …` / `BC-INDEX …` strings above (path-keyed; no migration identity needed); own reservation removed | error |
| (v1.6, mismatch suffix — both migrations, both families) Gate LOCKED, txn STAGING (`generation_id` null and, separately, `gen-1`) of `backfill-append-logs` with `completed-backfill-append-logs.json` present, PreToolUse `Edit` under `.factory/cycles/`; and txn STAGING of `migrate-bc-index` with `completed.json` present, PreToolUse `Edit` under `.factory/specs/behavioral-contracts/` | Exit 2; stderr = the path-family format string followed by ` (completion-record mismatch — operator investigation required)` (e.g. `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts (completion-record mismatch — operator investigation required)`; `BC-INDEX write blocked: … aborts (completion-record mismatch — operator investigation required)`); migration-state byte-identical; Branch B NOT applied (EC-010) | error |
| (v1.6, no terminal record — NoOp cell) Own-migration live txn (STAGING `generation_id=gen-1`, and separately COMMITTING), lock acquired, its terminal record ABSENT, PreToolUse `Edit` under the migration's own path family | Terminal-record reconciliation core returns `NoOp` in both cases (no txn write, no gate write); ordinary admission then blocks `E-MAINTENANCE-001` with the PLAIN keyed message (no mismatch suffix); migration-state byte-identical | error |
| (v1.7, EC-020 — first-activation reservation, no pre-existing directory) Fresh project tempdir with `.factory/` but NO `.factory/migration-state/`; PreToolUse `Edit` with `tool_use_id=T1` on `.factory/cycles/c1/burst-log.md` through the production admission entry point (`migration_writer_admission_precheck` / shared core) | Admitted (`None` verdict); `.factory/migration-state/reservations/T1.reservation` EXISTS with `created_at` + `tool_use_id=T1` and no PID; `gate-state.json` not created and no txn written by the admitter; the matching PostToolUse for `T1` removes exactly that file (directory remains) | edge-case |
| (v1.7, EC-020 — drain observes pre-directory writer) Same fixture; after the PreToolUse admit of `T1` (PostToolUse withheld), the coordinator drain (TTL GC → flock → DRAINING → txn STAGING null → poll) runs with a short injected drain timeout; then variant (b): PostToolUse for `T1` fires mid-poll | (a) the poll sees `T1.reservation`, `source_sha256` stays `null`, exit 2 `DRAIN_TIMEOUT_ABORT`, gate → OPEN, four files byte-identical; (b) the poll reaches quiescence after the release and proceeds to snapshot with the writer's completed write included — never a fingerprint-abort | error |
| (v1.7, EC-020 — concurrent first-ever admissions + no tool_use_id) Two PreToolUse `Edit`s (`T1`, `T2`) on protected paths launched concurrently against a project with no `migration-state/`; separately one PreToolUse `Edit` whose payload carries no `tool_use_id` | Both concurrent admits succeed (idempotent directory creation, no `EEXIST`/`AlreadyExists` surfaced); both `T1.reservation` and `T2.reservation` exist; the no-`tool_use_id` payload is admitted check-only with NO directory and NO reservation created | edge-case |
| (v1.4) Reservation JSON with `created_at = now − 4000 s` but mtime = now; TTL 3,600 s (test seam) | Judged stale (removed at drain step 1); and reservation with no `created_at` field and mtime `now − 4000 s` ⇒ stale via fallback (EC-019) | edge-case |
| (v1.8, EC-021 — PostToolUseFailure release) Real dispatcher PreToolUse (gate OPEN, no txn) `Edit` `tool_use_id=T10` under `.factory/cycles/c1/burst-log.md` ⇒ `reservations/T10.reservation` exists; then a real-binary `PostToolUseFailure` envelope `{hook_event_name: "PostToolUseFailure", tool_name: "Edit", tool_input: {...}, tool_use_id: "T10", error: "boom", is_interrupt: false}`; repeat with `is_interrupt: true`, with `tool_name` omitted, and with `tool_name: "Weird"`; and a `PostToolUseFailure` for `tool_use_id=T99` with no reservation | After each failure envelope `reservations/T10.reservation` is ABSENT (directory remains); the T99 envelope is a silent no-op (exit 0); no `PostToolUse` was sent in any run; `is_tool_completion_event("PostToolUse")` and `("PostToolUseFailure")` are true, `("PreToolUse")`, `("Stop")`, `("")` and `("posttooluse")` are false | happy-path |
| (v1.8, EC-022 — registry-independent, broken registry) Fixture A: `CLAUDE_PLUGIN_ROOT` unset, no registry file; B: registry present but unparseable TOML; C: registry with a mismatched `schema_version`; each with txn STAGING (`generation_id=gen-1`) and a PreToolUse `Write` under `.factory/cycles/c1/lessons.md` through the REAL spawned dispatcher; plus the same three fixtures with gate OPEN/no txn and a `Write` `tool_use_id=T11` followed by `PostToolUse` `T11` | Block fixtures: exit 2, stderr exactly the `.factory/cycles/ write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts` string, registry never loaded; admit fixtures: dispatcher continues (registry fail-open exit 0), `reservations/T11.reservation` created at Pre and REMOVED by the Post under the same broken registry (EC-022) | error |
| (v1.8, EC-022 — registry fail-closed after admit; ordering; once) Gate OPEN/no txn, valid-but-fail-closed registry (schema mismatch that exits non-zero, or a plugin that blocks), PreToolUse `Write` `tool_use_id=T12` to a protected path; and an instrumented run asserting call order and count | Dispatcher exits non-zero; `reservations/T12.reservation` ABSENT at exit (release-on-block funnel); the admission core ran exactly ONCE (single reservation create) and BEFORE `Registry::load`/`resolve_registry_path()` and `shard_cap_precheck`; an unparseable stdin payload exits with the pre-existing parse-error code and creates nothing (EC-022 O1–O4) | error |
| (v1.8, EC-023 — out-of-root path) Project tempdir P (own `.factory/`, txn STAGING live) and a SECOND tempdir Q; PreToolUse `Write` with `file_path` = `Q/.factory/cycles/c1/burst-log.md`; `P/sub/.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md`; `P/x.factory/cycles/c1/log.md`; `P/.factory-old/cycles/log.md`; each with a valid `tool_use_id` | All four admitted (exit 0); NO `reservations/*` file in P or Q; NO `migration-state/` created in Q or under `P/sub` or `P/x.factory`; P's migration-state byte-identical; and the same write inside `P/.factory/cycles/…` IS blocked `E-MAINTENANCE-001` (EC-023 a–d) | edge-case |
| (v1.8, EC-023 — symlinked `.factory`; no-`.factory` project) (e) `P/.factory` is a symlink to `R/real-factory` containing a live txn; PreToolUse `Write` to `P/.factory/cycles/c1/a.md` and to `R/real-factory/cycles/c1/a.md`; (f) project tempdir with NO `.factory` and a PreToolUse `Write` `file_path=<P>/.factory/cycles/c1/a.md` with a valid `tool_use_id` | (e) Both spellings resolve to the SAME real `factory_root`: both blocked `E-MAINTENANCE-001` (txn live) or both reserved in `R/real-factory/migration-state/reservations/` (txn absent); (f) admitted, NO `.factory` directory (nor any `migration-state/`) is created anywhere; the filesystem tree is byte-identical before/after (EC-023 e–f) | edge-case |
| (v1.8, EC-024 — lexical aliasing) txn live; PreToolUse `Edit` with `file_path` = `<P>/.factory/specs/./behavioral-contracts/x.md`; `<P>/.factory/specs/../specs/behavioral-contracts/x.md`; `<P>/.factory//cycles//c1//log.md`; `<P>/.factory/cycles/newcycle/new.md` (nonexistent tail); relative `.factory/cycles/c1/log.md` with payload `cwd=<P>` | All blocked `E-MAINTENANCE-001`; the first two with scope token `BC-INDEX`, the rest `.factory/cycles/`; with gate OPEN/no txn each is admitted and a reservation is created in `<P>/.factory/migration-state/reservations/` (EC-024 a–c,f,h) | error |
| (v1.8, EC-024 — symlink and case aliasing) txn live; `<P>/alias` → symlink to `<P>/.factory/cycles`; `<P>/link` → symlink to `/elsewhere/dir`; PreToolUse `Write` to `<P>/alias/c1/log.md`; to `<P>/link/../.factory/cycles/c1/log.md` (also with `link` → a dir inside P); to `<P>/.FACTORY/Cycles/c1/log.md` and `<P>/.factory/Specs/Behavioral-Contracts/x.md` on a case-sensitive volume; to a path with an unresolvable component (mode-000 directory / symlink loop); to `<P>/.factory/cycles\c1\log.md` on Unix | All the first five blocked `E-MAINTENANCE-001` (alias resolves into `factory_root`; `link/..` resolved POSIX-correctly AND the lexical form also matched; case folded without any filesystem probe); unresolvable ⇒ classified on `T_lex` alone and blocked if lexically protected (fail-closed); the Unix `\` path is a single ordinary name component under `.factory/` and is NOT classified `Cycles` (EC-024 d,e,g,i,j) | error |
| (v1.9, EC-023(g) — as-given spelling, `T_real` unavailable) `CLAUDE_PROJECT_DIR` = `<alias>/proj` where `<alias>` is a symlink to the real parent (canonical `<real>/proj`); a read-failing seam makes `T_real` unavailable; PreToolUse `Write` with `file_path=<alias>/proj/.factory/cycles/c1/a.md` and a valid `tool_use_id`, (1) live txn, (2) gate OPEN/no txn | (1) blocked `E-MAINTENANCE-001` (scope `.factory/cycles/`); (2) admitted, reservation created in `<real>/proj/.factory/migration-state/reservations/` and NOT under any `<alias>/…` path (EC-023 g) | edge-case |
| (v1.9, EC-024(k) — canonical vs as-given spelling) Same fixture; the same protected target once as `<real>/proj/.factory/specs/behavioral-contracts/x.md` and once as `<alias>/proj/.factory/specs/behavioral-contracts/x.md`, under (1) live txn, (2) gate OPEN | Identical outcome for both spellings: (1) both blocked with scope token `BC-INDEX`; (2) both admitted with a reservation in the one canonical migration-state (EC-024 k) | edge-case |
| (v1.9, EC-024(l) — look-alike / other project, no over-match) Same fixture, live txn; PreToolUse `Write` to `<alias>/other/.factory/cycles/c1/a.md` (sibling project through the same symlinked parent), `<alias>/proj-old/.factory/cycles/c1/a.md`, `<alias>/proj/sub/.factory/cycles/c1/a.md` | All admitted: NO reservation, NO directory creation, NO migration-state read; the session's migration-state byte-identical (EC-024 l) | edge-case |
| (v1.9, EC-032 / Precondition 6(b) — terminal record present but unverifiable) Live txn (STAGING `gen-1` and separately COMMITTING) with its own terminal record present as: (a) non-UTF-8 bytes; (b) zero-length; (c) truncated JSON; (d) valid JSON wrong schema (`[]`, `{"foo":1}`); (e) `canonical_paths_count` = 3; (f) mismatching `txn_id`; (g) a hash mismatch; (h) mode `000` (read call fails; read-failing seam as root); PreToolUse `Edit` under the migration's own path family | (a)–(g) `E-MAINTENANCE-001` keyed message WITH ` (completion-record mismatch — operator investigation required)`, txn NOT finalized, gate/txn/terminal record byte-identical, reason logged; (h) `E-MAINTENANCE-002: writer-admission check failed (io)`; with the terminal record ABSENT the message is the PLAIN suffix-less string (EC-032) | error |
| (v1.8, EC-025 — timestamp cases) Eight reservation files with TTL 3,600 s (seam), fixed `now`: `created_at`=`now+299 s`; `now+301 s` + mtime `now−4000 s`; `1969-12-31T23:59:59Z` + mtime `now−4000 s`; `"yesterday"` + mtime `now−4000 s`; `9999-12-31T23:59:59Z` + mtime `now−4000 s`; unusable + mtime `now+1000 s`; unusable + mtime unavailable/pre-epoch; `now−4000 s` written as `…+05:30` offset | In order (each `warn X` = exactly one coordinator-drain stderr advisory line with token X; no `migration.admission_advisory` event): retained (age 0); reclaimed + warn `created_at_future`; reclaimed + warn `created_at_pre_epoch` (never clamped to 0); reclaimed + warn `created_at_unparseable`; reclaimed + warn `created_at_future`; retained + warn `mtime_future`; retained + warn `age_unknown`; reclaimed (UTC-normalised age 4,000 s > ttl) (EC-025) | edge-case |
| (v1.8, EC-026 — invalid `tool_use_id`) PreToolUse `Write` to a protected path (gate OPEN, no txn) with `tool_use_id`: `12345` (number); `""`; `"../x"`; a 129-char string; `".hidden"`; `true`; versus the key ABSENT; `null`; a valid 128-char id | First six: exit 2 `E-MAINTENANCE-002: writer-admission check failed (invalid_tool_use_id)`; no `reservations/` entry, nothing admitted, stderr/log never contains the raw id (byte length only); ABSENT and `null`: admitted check-only (no directory, no reservation); valid id: admitted + reservation (EC-026) | error |
| (v1.8, EC-027 — other known migration, no own record) Live txn `{migration_id: "migrate-bc-index", state: STAGING gen-1}` and `{…, state: COMMITTING}` (and each again with the `migration_id` field absent), `completed.json` ABSENT, `completed-backfill-append-logs.json` present and valid; PreToolUse `Edit` under `.factory/cycles/` and under `.factory/specs/behavioral-contracts/` | Row-3 `NoOp` for each (no txn write, no gate write); plain `E-MAINTENANCE-001` block with the path-family message and NO mismatch suffix; txn NOT finalized and NOT aborted; migration-state byte-identical; the same fixture with STAGING `generation_id=null` and `completed.json` absent ⇒ Branch B (txn → ABORTED + `abort_reason: "null_generation"`, gate → OPEN, admitted) (EC-027) | error |
| (v1.8, EC-028 — other known migration, own verifying record) Live txn `{migration_id: "migrate-bc-index", COMMITTING}` with a valid `completed.json` matching `activation_id`/`generation_id`/N/hashes and gate LOCKED, WITH or WITHOUT `completed-backfill-append-logs.json` present; PreToolUse `Edit` | Branch C finalize: txn → COMPLETED, THEN gate → OPEN, admitted; identical outcome whether or not the mechanism-A record exists; the mirror fixture (live `backfill-append-logs` COMMITTING + its own `completed-backfill-append-logs.json` verifying) finalizes identically (EC-028) | error |
| (v1.8, EC-029/EC-030 — unknown and non-string `migration_id`) Live txn with `migration_id: "future-migration"` (STAGING null-gen, STAGING gen-1, COMMITTING; terminal records present and absent); a 1,000-char id with embedded `\u001b` control chars; and separately `migration_id: 7`, `null`, `["migrate-bc-index"]` | Unknown string ids: plain `E-MAINTENANCE-001`, txn NEVER aborted/finalized (Branch B does NOT run), migration-state byte-identical; exactly one `migration.admission_blocked` event (`branch = foreign_migration`) carries the id in `migration_id`, truncated to 64 chars, control chars escaped; non-string ids: exit 2 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`, no reservation left behind, no writes (EC-029, EC-030) | error |
| (v1.8, EC-031 — production TTL floor; reworded v1.10 for the crate-private seam) In-crate test calling `run_bc_index_migration_with_ttl(project_root, ttl)` (the production `run_bc_index_migration` delegates to it passing `DEFAULT_MAX_RESERVATION_TTL`) with `ttl` = 120 s and = 1,799 s; = 1,800 s; = 3,600 s; and `drain_bc_index_writers` (test seam) with TTL = 1 s | 120 s and 1,799 s: exit 2 `RESERVATION_TTL_BELOW_FLOOR` (`configured_secs`, `floor_secs=1800`) from `validate_production_reservation_ttl`, the FIRST statement of the seam, with a byte-identical `migration-state/` snapshot (gate-state, txn, reservations untouched) and NO `exclusive.lock` created; 1,800 s and 3,600 s: proceeds; the seam call `drain_bc_index_writers` is unaffected by the floor; the production default constant is 3,600 s (EC-031) | error |
| (v1.10, EC-031 — compile-time floor on the default) Source/compile check: `const _: () = assert!(DEFAULT_MAX_RESERVATION_TTL >= MIN_PRODUCTION_RESERVATION_TTL)` is present in `shard_manager.rs`; and `run_bc_index_migration_with_ttl` is `pub(crate)` (not `pub`) and no environment variable or argv token reaches it | A build with the default constant below the floor does not compile; the seam is not callable from outside the crate, and no operator-controlled input can lower the production TTL (EC-031) | edge-case |
| (v1.8, EC-032 — unreadable vs malformed `gate-state.json`; protected-path PreToolUse `Write`, valid `tool_use_id`, no txn record unless stated) Fixtures: (a) `gate-state.json` containing `"OPEN"`; (b) containing `"LOCKED"`; (c) ABSENT; (d) mode `000` (open fails EACCES; skip as root — use a read-failing seam); (e) `gate-state.json` is a DIRECTORY (read fails EISDIR); (f) zero-length file; (g) truncated `"OPE`; (h) invalid UTF-8 bytes `0xFF 0xFE`; (i) unquoted `OPEN`; (j) `"open"` (wrong casing); (k) `"BOGUS"`; (l) `{"state":"OPEN"}`; (m) `null`; (n) `7`; (o) mode-`000` file whose bytes would also be malformed | (a) admitted + reservation; (b) plain `E-MAINTENANCE-001` block (gate LOCKED, per existing rules); (c) admitted as `OPEN` (ENOENT is not an error); (d), (e), (o): exit 2 `E-MAINTENANCE-002: writer-admission check failed (io)` — content never examined, so (o) is `io` not `state_integrity`; (f)–(n): exit 2 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`; for every failing fixture: `HookResult::Error`, no reservation left behind, gate-state/txn byte-identical (no write), message contains only the cause token (path/errno/serde position only in the `detail` field of the single `migration.admission_failed` event); fixture matrix repeats for a txn record (`txn-*.json` unreadable ⇒ `io`; zero-length / truncated / invalid UTF-8 / JSON array top level / `state` unknown ⇒ `state_integrity`; non-string `migration_id` ⇒ `state_integrity` per EC-030; two live txn records ⇒ `state_integrity`); gate-state failure takes precedence over txn failure because `gate-state.json` is evaluated first (first-failure-wins); terminal record: unreadable (EACCES) ⇒ `E-MAINTENANCE-002 (io)`, unparseable / wrong-schema ⇒ `E-MAINTENANCE-001` WITH the ` (completion-record mismatch — operator investigation required)` suffix (v1.9), not finalized, reason logged (EC-032) | error |
| (v1.10, EC-032 / F-012 — carrier and Display) Unit test constructing `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` for each of the SIX kinds (`GateRecordMalformed`, `TxnRecordMalformed`, `TxnMigrationIdNotString`, `MultipleLiveTxns`, `ReservationSerialization`, `TxnRecordNewerSchema`) and calling `format!("{e}")`, `process_exit_code(&e)` and `admission_failure_cause(&e).token()`; the five admission-side raise sites (gate/txn record parse, txn not-an-object in `abort_null_generation_txn`, non-string `migration_id`, more than one live txn, reservation serialize) exercised through fixtures | Each Display equals `migration admission: state integrity failure (<kind-token>): <detail>` with the kind tokens `gate_record_malformed` / `txn_record_malformed` / `txn_migration_id_not_string` / `multiple_live_txns` / `reservation_serialization` / `txn_record_newer_schema`; NO Display contains `BINARY_INTEGRITY_FAILURE`; `process_exit_code` = 2; `admission_failure_cause(..).token()` = `state_integrity`; no admission-side raise site produces `BinaryIntegrityFailure` (EC-032) | error |
| (v1.10, Invariant 7 — exhaustive cause match) Source assertion over `admission_failure_cause`: its `match` over `BcIndexMigrationError` has no `_ =>` arm and lists every coordinator-only variant explicitly in one arm mapping to `StateIntegrity` | The pattern `_ =>` is absent from the function body (e.g. `awk` over the function body returns nothing); adding a new `BcIndexMigrationError` variant without classifying it fails to compile (Invariant 7) | edge-case |
| (v1.10, EC-033 — cwd ≠ `CLAUDE_PROJECT_DIR`, real binary) Project tempdir `<P>` with `.factory/`; a DIFFERENT cwd `<C>` (e.g. `<P>/sub` and, separately, an unrelated tempdir) with `CLAUDE_PROJECT_DIR=<P>`; dispatcher PreToolUse `Edit` `tool_use_id=T1` under `.factory/cycles/c1/log.md` creates a reservation; then the coordinator is launched from `<C>` under the same env | The reservation is in `<P>/.factory/migration-state/reservations/T1.reservation`; the coordinator's first drain lists it; `migration-state/`, `exclusive.lock`, gate and txn files are created only under `<P>/.factory`; NOTHING is created under `<C>`; `grep -n 'join(".factory' crates/factory-dispatcher/src/shard_manager.rs` returns only the resolver (EC-033) | error |
| (v1.10, EC-034 — `resolve_session_project_root` unit table, pure function, no env mutation) Rows (a)–(f) of EC-034 | (a) canonicalized path; (b) `Some("")` ⇒ `process_cwd`; (c) `None` ⇒ `process_cwd`; (d) nonexistent absolute ⇒ the as-given path (never the cwd); (e) symlink ⇒ resolved path; (f) git work tree root with `.factory` above `process_cwd` ⇒ `process_cwd` (no ancestor walk) (EC-034) | edge-case |
| (v1.10, EC-035 — no `.factory`) Project tempdir with NO `.factory` (and a variant where `.factory` is a regular file); invoke the coordinator under `CLAUDE_PROJECT_DIR=<P>` | Exit 2 `FACTORY_ROOT_NOT_FOUND`; stderr is exactly `backfill-append-logs: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <P> (resolved from CLAUDE_PROJECT_DIR)` — and with `CLAUDE_PROJECT_DIR` unset/empty and the cwd `<P>` lacking `.factory`, `… (resolved from process cwd)`; the directory tree is byte-identical before/after (no `.factory`, no `migration-state/`, no `exclusive.lock`) (EC-035) | error |
| (v1.10, EC-036 — symlinked `.factory`) `<P>/.factory` → symlink to `R/real-factory`; admission reserves `T1` under `CLAUDE_PROJECT_DIR=<P>`; the coordinator then runs under the same env | `T1.reservation` is at `R/real-factory/migration-state/reservations/`; the coordinator drains the same directory; no `migration-state/` exists under `<P>/.factory` as a separate namespace (EC-036) | edge-case |
| (v1.10, EC-037 — `migration.admission_blocked`, real dispatcher) For each branch fixture of EC-037 (gate-only DRAINING; live txn STAGING with gen-1 and terminal record absent; live coordinator holding `exclusive.lock`; txn `migration_id = "future-migration\u001b…"` (1,000 chars); STAGING + terminal record present) a PreToolUse `Write` under `.factory/cycles/c1/a.md` with `tool_use_id=toolu_ABC123`, then read `dispatcher-internal-YYYY-MM-DD.jsonl` | Per fixture exactly ONE `migration.admission_blocked` line (type, `trace_id`, `session_id`, `scope=".factory/cycles/"`, `family="cycles"`, `branch` = `gate_only` / `live_txn` / `live_coordinator` / `foreign_migration` / `completion_record_mismatch`, `gate_state`, `migration_id`/`txn_id` null for gate-only, the foreign id truncated to 64 chars with the control character escaped, `check` non-null only for the mismatch fixture, `reconciliation`); the stderr message is the unchanged keyed string; `toolu_ABC123` and any record content appear in NO field of the line (EC-037) | error |
| (v1.10, EC-038 — `migration.admission_failed`, real dispatcher) Fixtures: `tool_use_id=12345`; `gate-state.json` a directory; `gate-state.json` = `"BOGUS"`; two live txn records; then read the JSONL | Exactly ONE `migration.admission_failed` line each: `cause` = `invalid_tool_use_id` / `io` / `state_integrity` / `state_integrity`; `kind` = `null` / `null` / `gate_record_malformed` / `multiple_live_txns`; `detail` = byte length / path + `io::Error` kind / path + parse message / path set — never the raw id nor record content; the stderr line is the unchanged `E-MAINTENANCE-002: writer-admission check failed (<cause>)` (EC-038) | error |
| (v1.10, EC-039 — `migration.admission_advisory`; reworded v1.10-rev) Fixtures: a release error other than ENOENT (read-only reservations directory); stuck gate `LOCKED` with no live txn (Branch A); STAGING `generation_id=null` txn (Branch B); COMMITTING txn with a verifying terminal record while the finalize effect is undelivered (S-25.09); then read the JSONL | Exactly ONE `migration.admission_advisory` line per anomaly with `reason` = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired`; the Branch A/B/release dispatch outcome is unchanged (admitted); the finalize-unwired fixture ALSO has its one `migration.admission_blocked` with `branch="completion_record_mismatch"`, `reconciliation="completion_record_mismatch"`, `check="finalize_unwired"`; no `reason` outside the five-token set ever appears; no optional field outside {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}; no raw `tool_use_id` (EC-039) | edge-case |
| (v1.10-rev, EC-039(f) — timestamp tokens are stderr only) Drain GC over a reservation with unparseable `created_at` (and each of the other four timestamp cases of EC-025) | One coordinator stderr advisory line with the token (`created_at_unparseable`, …); NO `migration.admission_advisory` line (and no other `migration.admission_*` line) appears in `dispatcher-internal-*.jsonl` (EC-039) | edge-case |
| (v1.10-rev, EC-037 — `reconciliation` token and `branch` derivation, one vector per token) Unit table over the pure diagnostic builder with inputs (`reconciliation` token, live txn remains?): (`live_coordinator`, yes); (`nothing_to_reconcile`, yes) and (`nothing_to_reconcile`, no — gate-only DRAINING/LOCKED); (`gate_reopened`, no) and (`gate_reopened`, yes); (`null_generation_txn_aborted`, no) and (`null_generation_txn_aborted`, yes); (`foreign_migration_refused`, yes); (`completion_record_mismatch`, yes). Real-dispatcher fixtures where reachable: `exclusive.lock` held by a live coordinator; live txn STAGING gen-1 with own terminal record ABSENT; DRAINING gate with no txn; txn `migration_id="future-migration"`; STAGING + terminal record present | `branch` = `live_coordinator` / `live_txn` / `gate_only` / `gate_only` / `live_txn` / `gate_only` / `live_txn` / `foreign_migration` / `completion_record_mismatch` respectively (derivation: `live_coordinator` ⇔ `live_coordinator`; `foreign_migration` ⇔ `foreign_migration_refused`; `completion_record_mismatch` ⇔ `completion_record_mismatch`; else `gate_only` if no live txn remains, else `live_txn`); every real-dispatcher `migration.admission_blocked` line carries exactly one of the six `reconciliation` tokens (`live_coordinator`, `nothing_to_reconcile`, `gate_reopened`, `null_generation_txn_aborted`, `foreign_migration_refused`, `completion_record_mismatch`) — the value `none` or any decision-table name (`NoOp`, …) never appears; `check` non-null only for `completion_record_mismatch` (EC-037) | error |
| (v1.10-rev2, EC-037 — closed `check` domain, one vector per token) Pure-core / real-dispatcher fixtures, PreToolUse `Write` under `.factory/cycles/c1/a.md`, own-migration live txn: (1) STAGING gen-1 + own terminal record present; (2) COMMITTING + terminal record bytes `\xff\xfe` (and a variant empty file, and `{"txn_id":`); (3) COMMITTING + terminal record `{"txn_id":7}` (valid JSON, wrong shape); (4) COMMITTING + record `txn_id` ≠ `activation_id`; (5) same but `txn_id` ok, `generation_id` ≠; (6) ids ok, `canonical_paths_count` = 3; (7) all of the above ok, one canonical file's bytes changed (and a variant: one canonical file removed); (8) S-25.09 build: COMMITTING + own terminal record present, any content; (9) S-25.09 build: COMMITTING + a record the pure core decides `FinalizeThenOpenGate` for; (10) COMMITTING + terminal record path is a directory / `chmod 000` (read CALL fails) | Rows (1)–(9): exactly ONE `migration.admission_blocked` with `branch="completion_record_mismatch"`, `reconciliation="completion_record_mismatch"`, `check` = respectively (1) `staging_with_terminal_record`, (2) `terminal_record_unparseable`, (3) `terminal_record_schema_mismatch`, (4) `txn_id_mismatch`, (5) `generation_id_mismatch`, (6) `canonical_paths_count_mismatch`, (7) `canonical_hash_mismatch` (both variants), (8) `terminal_record_unverified`, (9) `finalize_unwired` (+ the `branch_c_finalize_unwired` advisory); the E-MAINTENANCE-001 message carries the mismatch suffix; no txn/gate write. Order check: a fixture failing both (2)-class and (4)-class conditions reports the EARLIER-numbered check. Row (10): NOT Branch C — `E-MAINTENANCE-002` cause `io`, one `migration.admission_failed`, no `migration.admission_blocked`. Rows (8)/(9) exist ONLY while the S-25.09 seam is in place; after S-25.06 no event ever carries `terminal_record_unverified` or `finalize_unwired` (S-25.06 replaces row (8)'s fixture with rows (2)–(7)). Set-membership assertion: across all rows every non-null `check` ∈ the nine-token set (EC-037) | error |
| (v1.10-rev2, EC-035 — `FactoryRootNotFound` optionality, rule (a)) (A) Real `backfill-append-logs` and `migrate-bc-index` binaries, dir lacking `.factory`, once with `CLAUDE_PROJECT_DIR=<P>` and once with it unset (cwd `<P>`); (B) library: `run_bc_index_migration(<P>)` and in-crate `run_bc_index_migration_with_ttl(<P>, DEFAULT_MAX_RESERVATION_TTL)`; (C) library: `run_bc_index_migration_for_session(&SessionProjectRoot{path:<P>, source: ClaudeProjectDir})` and `{source: ProcessCwd}`; (D) pattern match `FactoryRootNotFound { project_root, root_source }` | (A) stderr is exactly one line ending ` (resolved from CLAUDE_PROJECT_DIR)` / ` (resolved from process cwd)` — NEVER suffix-less on either binary; (B) returns `Err(FactoryRootNotFound { root_source: None, .. })`, `to_string()` == `FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <P>` (no trailing space, no `(resolved from`); (C) `root_source: Some(ClaudeProjectDir)` / `Some(ProcessCwd)`, `to_string()` ends ` (resolved from CLAUDE_PROJECT_DIR)` / ` (resolved from process cwd)`; (D) compiles — the field is `root_source` (a field named `source` would not compile under `thiserror`); tree byte-identical before/after in every case (EC-035) | error |
| (v1.10, EC-040 — verdict surface label, real dispatcher stderr) A protected-path `Write` blocked by a live txn; and a `Write` to an enrolled shard path that `shard_cap_precheck` blocks | Stderr summary line of the first carries `blocking_plugins=migration-admission` and `block_reason="…E-MAINTENANCE-001…"` (message unchanged, exit 2); the second carries `blocking_plugins=shard-cap-gate` (EC-040) | error |
| (v1.12, EC-041 — unstatable `.factory`, ADMISSION; real dispatcher) Project tempdir `<P>` with `.factory/` whose `<P>` is `chmod 000` (skip as root — use a stat-failing seam) and, separately, `<P>/.factory` → symlink to itself (ELOOP); PreToolUse `Write` `file_path=<P>/.factory/cycles/c1/a.md` and (second run) `file_path=<P>/README.md`, valid `tool_use_id=T1`; then read `dispatcher-internal-YYYY-MM-DD.jsonl`. Controls: regular-file `.factory`; dangling `.factory` symlink; `<P>` itself a regular file (ENOTDIR); no `.factory` | EACCES and ELOOP, BOTH file paths: exit 2 `E-MAINTENANCE-002: writer-admission check failed (io)`; exactly ONE `migration.admission_failed` (`cause=io`, `kind=null`, `detail` carries the path + `ErrorKind` + message, never `T1`), ZERO `migration.admission_blocked`, no `reservations/` entry, nothing created; stderr `blocking_plugins=migration-admission`. Controls: all four ADMITTED (exit 0), no event, no reservation, filesystem tree byte-identical (EC-041) | error |
| (v1.12, EC-042 — unstatable `.factory`, RELEASE; real dispatcher) Same EACCES / ELOOP fixtures; a real `PostToolUse` envelope and a `PostToolUseFailure` envelope for `tool_use_id=T1` (a reservation `T1.reservation` pre-seeded where the seam permits); controls: ABSENT variants | Unstatable: exit 0 (never a verdict), ONE `migration.admission_advisory` per envelope (`reason=reservation_release_failed`, `detail` with the path + `ErrorKind` + message), nothing created or deleted, the pre-seeded reservation still present (reclaimed later by the drain-start TTL GC). Absent: exit 0, no event (EC-042) | error |
| (v1.12, EC-043 — unstatable `.factory`, COORDINATOR; real binaries) `backfill-append-logs` and `migrate-bc-index` under `CLAUDE_PROJECT_DIR=<P>` with `<P>` `chmod 000` (EACCES) and, separately, a self-referential `<P>/.factory` (ELOOP); controls: no `.factory`, regular-file `.factory`, dangling symlink | Unstatable: exit 2 with `BcIndexMigrationError::Io` carrying `<P>/.factory` and the OS error (the existing `Io` Display); stderr does NOT contain `FACTORY_ROOT_NOT_FOUND`; tree byte-identical (no `.factory`, `migration-state/`, `exclusive.lock`). Controls: exit 2 `FACTORY_ROOT_NOT_FOUND` with the EC-035 line (EC-043, EC-035) | error |
| (v1.12, EC-044 — minimal FOREIGN live record, plain block; per path family and gate state) `txn-f.json` = exactly `{"state":"STAGING","migration_id":"future-migration"}`, then exactly `{"state":"COMMITTING","migration_id":"future-migration"}`; for each, gate-state.json = `"OPEN"`, `"DRAINING"`, `"LOCKED"`; PreToolUse `Write` under `.factory/cycles/c1/a.md` and under `.factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md` | All 12 combinations: exit 2 PLAIN `E-MAINTENANCE-001` with the path-family keyed message and NO mismatch suffix (never `E-MAINTENANCE-002`, never `state_integrity`); one `migration.admission_blocked` (`branch=foreign_migration`); txn and gate byte-identical (EC-044) | error |
| (v1.12, EC-045 — minimal KNOWN STAGING without `generation_id`) `{"state":"STAGING","migration_id":"backfill-append-logs"}` (variants: `migration_id` key absent; `"generation_id": 7`; `"generation_id": ["x"]`), gate `"DRAINING"`, `exclusive.lock` free (no live coordinator), no terminal record; PreToolUse `Write` under `.factory/cycles/c1/a.md`. Control: same record with `exclusive.lock` held by a live coordinator | Exit 2 `E-MAINTENANCE-002: writer-admission check failed (state_integrity)`; ONE `migration.admission_failed` (`cause=state_integrity`, `kind=txn_record_malformed`); txn bytes UNCHANGED (sha256 before == after; NOT rewritten to ABORTED, no `abort_reason` field appears); gate UNCHANGED (`"DRAINING"`); no reservation left. Control: plain `E-MAINTENANCE-001` (`branch=live_coordinator`), no field read (EC-045) | error |
| (v1.12, EC-046 — STAGING with `generation_id` JSON null, unchanged discard) `{"state":"STAGING","migration_id":"backfill-append-logs","generation_id":null}`, gate `"DRAINING"`, `exclusive.lock` free, no terminal record; PreToolUse `Write`. Control: `"generation_id":"gen-1"` | Branch B discard: txn rewritten in place to `ABORTED` + `abort_reason: "null_generation"`, `generation_id` still null, file retained; gate → `OPEN`; the write admitted; ONE `migration.admission_advisory` (`branch_b_txn_aborted`). Control: plain `E-MAINTENANCE-001` live block, txn unchanged (EC-046) | error |
| (v1.12, EC-047 — minimal KNOWN COMMITTING, no terminal record) `{"state":"COMMITTING","migration_id":"backfill-append-logs"}` and `…"migrate-bc-index"`, gate `"LOCKED"`, terminal record ABSENT, `exclusive.lock` free; PreToolUse `Write` under each path family | PLAIN `E-MAINTENANCE-001` (no mismatch suffix; `branch=live_txn`); txn and gate byte-identical; no Tier 1 field read; NOT `E-MAINTENANCE-002` (EC-047) | error |
| (v1.12, EC-048 — minimal NON-LIVE records admitted) `{"state":"COMPLETED","migration_id":"backfill-append-logs"}`, `{"state":"ABORTED","migration_id":"backfill-append-logs"}`, and the same two with `"migration_id":"future-migration"` (and each with `migration_id` absent); (a) gate `"OPEN"`; (b) gate stuck `"LOCKED"` and `"DRAINING"`, no live txn; plus (c) one such non-live foreign record beside one live known txn; (d) two LIVE minimal records (one foreign) | (a) admitted (exit 0), reservation created, files byte-identical; (b) Branch A reopens the gate (`branch_a_gate_reopened`), then admitted; non-live records never modified or deleted; (c) the live known txn decides (plain `E-MAINTENANCE-001`), the non-live foreign record neither refuses nor counts; (d) `E-MAINTENANCE-002 (state_integrity)`, `kind=multiple_live_txns` — live records only are counted (EC-048) | edge-case |
| (v1.12 extension, EC-047/EC-049 — binary leg, malformed vs disagreeing vs ordering) `backfill-append-logs` txn `{"state":"COMMITTING","migration_id":"backfill-append-logs","activation_id":"A1","generation_id":"g1","intent_log_path":"…/intent-g1.log",…}` + `completed-backfill-append-logs.json` `{"txn_id":"A1","generation_id":"g1","canonical_paths_count":4,…}`, four canonical files at their DONE hashes, gate `"LOCKED"`. Mutations (one per run): (a) remove `activation_id`; (b) `"activation_id": 5`; (c) remove `generation_id`; (d) `"generation_id": 7`; (e) terminal `txn_id` = `"A2"`; (f) terminal `txn_id` = `"A2"` AND remove `generation_id`; (g) txn `"state":"STAGING"` with `generation_id` removed. Same set against a `migrate-bc-index` txn + `completed.json` (N from B2). Snapshot `migration-state/` before and after | (a)–(d): exit 2, stderr `MIGRATION_STATE_INTEGRITY_FAILURE`, snapshot byte-identical, txn still COMMITTING, gate `"LOCKED"`; (e)(f)(g): exit 2, stderr `COMPLETION_RECORD_MISMATCH_ABORT`, snapshot byte-identical; PreToolUse analogue of (a)–(d): `E-MAINTENANCE-002 (state_integrity)`, `txn_record_malformed`; of (e)–(g): `E-MAINTENANCE-001` + suffix with `check` = `txn_id_mismatch` / `txn_id_mismatch` / `staging_with_terminal_record` (EC-047, EC-049) | error |
| (v1.12 extension, EC-050/EC-051/EC-052 — intent-log hash source) Same COMMITTING fixture with every check before `canonical_hash_mismatch` passing. (a) remove `intent_log_path`; (b) `"intent_log_path": null`; (c) `"intent_log_path": 3`; (d) path to a MISSING intent log; (e) intent log exists with a DONE record for three of four canonical paths; (f) DONE `expected_post_hash` of one path ≠ file `sha256`; (g) `intent_log_path` is a directory (EISDIR); (h) control: all four DONE records match; (i) `canonical_paths_count` = 3 AND `intent_log_path` removed; (v1.13) (j) `canonical_move_plan: []`; (k) plan missing one of the four canonical paths; (l) plan with one differing `staging_path`; (m) bit flip in the `staging_path` byte of the second record with all later records valid (mid-log corruption); (n) four INTENT+DONE pairs followed by 512 NUL bytes (torn tail); (o) a DONE whose `expected_post_hash` equals the file's `sha256` but differs from its INTENT's | (a)(b)(c): admission `E-MAINTENANCE-002 (state_integrity)` / binary exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`; (d)(e)(f): `canonical_hash_mismatch` ⇒ mismatch-suffixed `E-MAINTENANCE-001` / binary exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` (NOT `state_integrity`); (g): admission `E-MAINTENANCE-002 (io)` / binary `Io` exit 2; (h): verify-then-finalize succeeds (`canonical_move_plan` is never consulted for a hash); (i): `canonical_paths_count_mismatch` wins, `intent_log_path` never read; (j): `state_integrity` `txn_record_malformed` (admission) / `MIGRATION_STATE_INTEGRITY_FAILURE` (binary); (k)(l)(m)(o): `canonical_hash_mismatch` as (d)(e)(f); (n): verify-then-finalize succeeds (tail absent); no mutation in any failing case (EC-050, EC-051, EC-052) | error |
| (v1.12 third binary-leg extension, EC-053 — contention on every path) Second coordinator holds the flock; `backfill-append-logs` with `completed-backfill-append-logs.json` present and (i) no txn; (ii) own live COMMITTING txn; (iii) live foreign record (`future-migration`). Control: flock FREE with the (ii) state | (i)(ii)(iii): exit 1, nothing read or written (`migration-state/` byte-identical), NEVER exit 0 / `ALREADY_MIGRATED`; (iii) is contention, not the foreign refusal; control: Postcondition 5a verdict (verify-then-finalize or exit 2), timing-independent (EC-053) | error |
| (v1.12 third binary-leg extension, EC-054 — TOCTOU) `Fs` seam: terminal record absent at the pre-lock probe; before lock acquisition it and a COMPLETED txn appear | Under-lock read sees the record; no fresh run, gate untouched, exit 0 `ALREADY_MIGRATED`; snapshot unchanged (EC-054) | edge-case |
| (v1.12 third binary-leg extension, EC-055 — Branch B version gate, admission) `{"state":"STAGING","migration_id":"backfill-append-logs","generation_id":null,"schema_version":S}` for S = `2`, `4294967296`, `"1"`, `0`, `1.5`, `null`; and S absent; and S = `1` + `"x_extra":7`; and S = `2` with `generation_id` removed; gate DRAINING, lock free | S = `2`/`4294967296`/`2`+removed key: `E-MAINTENANCE-002 (state_integrity)`, ONE `migration.admission_failed` with `kind=txn_record_newer_schema`, no `_blocked`, txn bytes unchanged (not the discard); S = `"1"`/`0`/`1.5`/`null`: `kind=txn_record_malformed`, bytes unchanged; S absent and S = `1` + extra key: unchanged discard (ABORTED + `abort_reason: "null_generation"`, gate OPEN, `branch_b_txn_aborted`), extra key PRESERVED (EC-055) | error |
| (v1.12 third binary-leg extension, EC-056 — Branch C version gate) Live COMMITTING txn + verifying own terminal record with `schema_version` = `2`, `4294967296`, `3` plus `activation_id` removed, `"2"`, absent, `1` | `2`/`4294967296`/`3`+removed field: `E-MAINTENANCE-002 (state_integrity)` `kind=txn_record_newer_schema`, exactly one `migration.admission_failed`, evaluated before every check, bytes unchanged; `"2"`: `txn_record_malformed`; absent/`1`: Branch C checks run normally (EC-056) | error |
| (v1.12 third binary-leg extension, EC-057 — exit-code table) Exhaustive table test over every `AppendLogMigrationError` variant | `LockContention` ⇒ 1; every other named variant ⇒ 2; no wildcard arm (EC-057) | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-143 | (a) Four-file all-or-nothing atomicity invariant — a simulated crash at any staging/pivot/canonical-move step leaves ALL FOUR target files either fully original or fully split, never a state with some of the four migrated and others not; AND (b) idempotency two-layer consistency invariant — `completed-backfill-append-logs.json` presence and BC-1.18.008's per-file manifest presence never disagree in a way that causes either a false `ALREADY_MIGRATED` before all four files are actually split, or a re-split of a file whose manifest already exists | fault-injection / integration test ((a) simulated crash at each step across all four files; assert post-recovery state is one of exactly two valid whole-migration states; (b) resume-from-STAGING and roll-before-backfill fixtures per file, cross-checked against top-level `completed-backfill-append-logs.json` state) |
| VP-144 | Per-file delegation invariant — this BC's content-preservation/census checks for each file are byte-identical in outcome to invoking BC-1.18.008's own Postcondition 6(a)/(b) checks directly against that file in isolation (no divergent or duplicated verification logic) | property test (differential test: governed-migration per-file check vs. direct BC-1.18.008 invocation on the same fixture, asserting identical PASS/FAIL and identical failure detail) |
| VP-145 | Closed-grammar rejection invariant — every invocation form outside the two accepted forms (`backfill-append-logs`, `backfill-append-logs --census`) is rejected before any filesystem mutation, by either the pre-shell classifier or the binary itself | integration test (fixture table of rejected forms: path arguments, extra flags, shell metacharacters, compound commands) |
| VP-146 | Governed mechanism-A migration crash-recovery decision core — six Kani obligations (seven `#[kani::proof]` functions, `proof_obl_a1`..`a6` in `append_log_kani_proofs.rs`): a1 recovery totality (§Precondition 5, §Invariant 3, §EC-002); a2 recovery safety / content-verified old-or-new-never-torn (§Precondition 5, §Invariant 3, §Postcondition 3, §EC-002); a3 txn state-machine inductive step + bounded sequence (§Precondition 5, §Postcondition 3, §Postcondition 4, §Invariant 3); a4 admission-gate quiescence INV-GATE-TXN (§Precondition 6(b), §Precondition 6(c)); a5 pointer-swap + canonical-move crash atomicity over the abstract `Fs` model (§Precondition 5, §Postcondition 3, §Postcondition 3a, §Invariant 3, §EC-002); a6 recovery idempotence (§Precondition 5, §Invariant 3, §EC-002, §EC-003). Kani prong of ADR-052 §Decision 12; VP-143 remains the real-filesystem fault-injection prong | kani-proof (`cargo kani`, `kani-mechanism-a` CI job, `EXPECTED_PROOFS=7`) |

VP IDs allocated by architect (S-25.06 Spec-First Gate closure, POLICY 9 propagation, 2026-09-25;
VP-INDEX v3.24): **VP-143** (integration; four-file all-or-nothing atomicity + idempotency
two-layer consistency — candidates 1 and 4 consolidated into the single VP-143 row above (facets (a) and (b)) per the
single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established, since both are
same-method integration/fault-injection safety obligations of the SAME governed-migration state
machine), **VP-144** (proptest; per-file delegation-correctness — candidate 2, a differential test
against BC-1.18.008 §PC6(a)/(b) reusing VP-123's existing fixture-generation strategy), and
**VP-145** (integration/safety; closed-grammar rejection — candidate 3, a two-layer
defense-in-depth property with no direct sibling in BC-1.18.011's VP set, since B2's migration
does not expose an equivalent closed CLI subcommand grammar at this layer). Full VP files authored
at `.factory/specs/verification-properties/VP-143.md`, `VP-144.md`, `VP-145.md`. **VP citations
changed in: BC-1.18.013 (this BC).** Propagated same-burst to `VP-INDEX.md` (v3.24),
`verification-architecture.md` (v1.37), and `verification-coverage-matrix.md` (v1.35) per
`vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

## Related BCs

- BC-1.18.008 — this BC's governed migration invokes BC-1.18.008's per-file split algorithm
  unmodified, wrapping it in ADR-052's multi-file atomic envelope (depends on)
- BC-1.18.006 — BC-1.18.008's own reused atomic-write/seal primitives, transitively depended on
  (depends on)
- BC-1.18.005 — the `[[shard]]` config schema and cap-formula this BC's Postcondition 8
  enrollment populates for the four files (depends on)
- BC-1.18.011 — the B2 sibling governed-migration BC this BC's structure is modeled directly on,
  substituting four independent files for one logical body's ten-way partition (related to)
- BC-7.08.001 — the Cohort-B fail-closed flip IS gated on this BC's completion (Postcondition 7 /
  Invariant 4 — the inverse of BC-1.18.011's explicit non-dependency) (depended on by)
- BC-3.08.001 — catalogues the three admission diagnostic events (Events 11–13:
  `migration.admission_blocked` / `migration.admission_failed` / `migration.admission_advisory`)
  this BC's Postcondition 10 mandates; wire-format/field-shape authority only (v1.10) (composes with)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — `run_mechanism_a_backfill_split` (the
  per-file algorithm this BC's governed migration invokes, unmodified) and
  `SHARD_CONFIG_RELATIVE_PATH` (`.factory/shard-config.toml`, the enrollment target for
  Postcondition 8)
- `.factory/cycles/v1.0-brownfield-backfill/{decision-log.md,burst-log.md,lessons.md,session-checkpoints.md}`
  — the exact, fixed four-file scope (Postcondition 6)
- ADR-052 §Decision 1 — mechanism selection: one-time interactive Bash approval
- ADR-052 §Decision 2 — binary placement in `crates/factory-dispatcher/`
- ADR-052 §Decision 3 — closed argument grammar (Precondition 3, Postcondition 6, EC-008)
- ADR-052 §Decision 4 — armed-activation manifest (Precondition 4)
- ADR-052 §Decision 5a — native admission gate in `executor.rs`, scoped to `.factory/cycles/`
  paths for this migration (Precondition 6)
- ADR-052 §Decision 5a v1.20 sub-sections — "Evaluation position" (Precondition 6(b) "Where", O1–O4),
  "Admission scope anchoring" and "Target path resolution" (Precondition 6(b)(i)–(v)), the
  PostToolUse/PostToolUseFailure release events (Precondition 6(c)), "Reservation timestamp rules"
  and "`tool_use_id` presence and validity" (Precondition 6(c)); ADR-052 §Decision 7e "Definition of
  foreign in the shared admission core" (Precondition 6(c) decision cell, Postcondition 5a);
  ADR-052 §Error Code Semantics v1.20 (`E-MAINTENANCE-002`, `RESERVATION_TTL_BELOW_FLOOR`)
- ADR-052 §Decision 5a v1.21 sub-sections — "Entry-point naming" (Precondition 6(b) "Where"),
  "Single anchoring rule" (Precondition 6(b)(i), Precondition 7, Invariant 8), "Admission
  diagnostics channel" (Postcondition 10), "Admission verdict surface label" (Precondition 6(b)
  Decision, EC-040), "Admission state-integrity variant" (Precondition 6(c), EC-032, Invariant 7),
  "EC-031 discharge — production TTL floor" (EC-031); ADR-052 §Error Code Semantics v1.21 additions
  (`MIGRATION_STATE_INTEGRITY_FAILURE`, `FACTORY_ROOT_NOT_FOUND`)
- `crates/factory-dispatcher/src/shard_manager.rs` — `resolve_session_project_root` (pure,
  single anchoring function), `resolve_factory_root`, `FactoryRoot::migration_state_dir()`;
  `crates/factory-dispatcher/src/executor.rs` — `migration_writer_admission`,
  `migration_writer_admission_precheck`, `migration_reservation_release`, `NativeGate`;
  `crates/factory-dispatcher/src/internal_log.rs` — `MIGRATION_ADMISSION_BLOCKED` /
  `MIGRATION_ADMISSION_FAILED` / `MIGRATION_ADMISSION_ADVISORY`; `crates/factory-dispatcher/src/main.rs`
  — drains `AdmissionDiagnostic` data into `InternalLog` before the early return
- ADR-052 §Decision 6 — audit trail (NIST AU-9): census stdout + durable factory-artifacts commit
- ADR-054 §Decision 1/2/3/4/5 (v1.13) — hardened intent-log wire format (1.2 value rules, 1.4
  checksum and golden record, 1.7 reader, 1.8 invariants, 1.9 writer), fixed `canonical_move_plan`
  (rename; no per-move txn rewrite), B-1/B-2/B-3 and the per-move recovery table, the ONE shared
  `intent_log` module, the fault-injection mandate; companion of ADR-052
- ADR-052 §Decision 7a/7b/7c — advisory flock + durable txn record; (7b is a POINTER to ADR-054 since
  v1.24) intent log + WAL
  boundary; single CURRENT-backfill-append-logs.json pointer swap + `completed-backfill-append-logs.json` terminal record (Postconditions
  3/3a/4/5, Invariant 3)
- ADR-052 §Decision 8 — POLICY 22 exception; the ratified allowed-write-targets list naming the
  exact four `v1.0-brownfield-backfill` paths (Postcondition 6)
- ADR-052 §Decision 9 — resolves the S-25.06 Rule 7 contradiction this BC discharges
- ADR-052 §Decision 11 — executable verify-to-execute binding (binary integrity, TOCTOU closure)
- ADR-053 §shard-config.toml registration precedent — the same pre-existing artifact-path-registry
  gap this BC's Postcondition 8 cites for `.factory/shard-config.toml`

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5; no
`grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn run_mechanism_a_backfill_split" crates/factory-dispatcher/src/shard_manager.rs
pub fn run_mechanism_a_backfill_split
```

Confirms BC-1.18.008's algorithm entry point exists and is the function this BC's governed
migration invokes per file (Precondition 1, Invariant 1).

```
$ grep -oE "SHARD_CONFIG_RELATIVE_PATH: &str = \"[^\"]+\"" crates/factory-dispatcher/src/executor.rs
SHARD_CONFIG_RELATIVE_PATH: &str = ".factory/shard-config.toml"
```

Confirms the live ShardRegistry config path Postcondition 8's enrollment targets.

```
$ grep -rn "backfill-append-logs" crates/ 2>/dev/null | grep -v "/tests/" | wc -l
0
```

Confirms `backfill-append-logs` is NOT YET a wired subcommand anywhere in the workspace as of
this BC's authoring — this BC specifies the contract test-writer/implementer build against under
S-25.06's TDD phase; it is not describing already-shipped behavior (distinguishing this BC's
`origin: greenfield` status from a brownfield-extracted contract).

```
$ test -f .factory/shard-config.toml && echo EXISTS || echo ABSENT
ABSENT
```

Confirms `.factory/shard-config.toml` does not yet exist in the committed tree, consistent with
ADR-053's identical observation for the sibling STORY-INDEX sharding effort and with
Postcondition 8's registry-precondition note.

## Story Anchor

S-25.06 — Append-Log Artifact Class: Sanctioned Backfill-Split Executor + ShardRegistry
Enrollment + Cap-Triggered Rotation

Additional implementing stories (v1.11; S-25.08 split into S-25.08 + S-25.09, human decision
D-1252(f)). Each obligation names the story that delivers it:

- **S-25.08** — Shared Admission Core, B2 Conformance, VP-147 Re-baseline — delivers the shared
  admission core obligations of Preconditions 6(a)–(e) and Postconditions 5a/9 (including the
  first-activation reservation namespace, the AC-018 `<cause>` classification, the
  `PostToolUseFailure` release fixture, the Branch A/B/C reconciliation, EC-020, EC-031 base
  seam, EC-032) — every obligation not listed under S-25.09.
- **S-25.09** — Admission v1.21 Anchoring, Diagnostics and Neutral Entry Points — delivers
  Precondition 6(b)(i) (single session-project-root anchoring rule), Precondition 7
  (coordinator anchoring; `FACTORY_ROOT_NOT_FOUND`), Invariant 7 (exhaustive
  `admission_failure_cause`; `AdmissionStateIntegrity`), Invariant 8 (one anchoring function),
  Postcondition 10 (InternalLog admission diagnostics, Events 11–13 of BC-3.08.001, including the
  Branch C seam tokens `terminal_record_unverified` / `finalize_unwired` and the
  `branch_c_finalize_unwired` advisory), the EC-031 crate-private TTL seam (F-006), the F-013
  `migration-admission` label (Precondition 6(b) Decision; EC-040), the F-014 neutral entry-point
  names, and EC-033..EC-040 with their test vectors.

Story attribution of the v1.12 obligations (ADR-052 v1.23 "Code ownership"; for story-writer):

- **S-25.08** owns the `resolve_factory_root` classification (Found / Absent / Unstatable;
  `Result<Option<FactoryRoot>, BcIndexMigrationError>`), the admission leg (Unstatable ⇒
  `E-MAINTENANCE-002 (io)`) and the release leg (Unstatable ⇒ no verdict, diagnostic via the
  S-25.08 channel `tracing::warn!`), and the Tier 0 / Tier 1 txn read (`read_txn_files` is Tier 0
  only; Branch B `generation_id` tri-state resolved before the unchanged planner; diagnostic
  `txn_id` read from the raw object with the `unknown` fallback) — EC-041, EC-044..EC-048.
- **S-25.09** owns the coordinator `Io` mapping (`Absent` ⇒ `FactoryRootNotFound`, `Err` ⇒
  propagate `Io`; EC-043), the `migration.admission_failed` (`cause=io`) event for the unstatable
  admission case, the `migration.admission_advisory` (`reservation_release_failed`) event for the
  unstatable release case (EC-042), and `TxnRecordMalformed` carrying the Branch B
  unusable-`generation_id` failure.
- **S-25.10** (v1.13; ADR-054 v1.0 §Downstream; stacked on S-25.09, blocks S-25.06) owns the
  shared `intent_log` module, the hardened format and reader/writer, the `canonical_move_plan`
  rename and fixed-plan model and B-1/B-2/B-3 (the S-25.06 CONSUMPTION CONTRACT of Precondition 5,
  Postcondition 5a(b) AC-031 verifier steps and EC-050..EC-052 is specified here; S-25.06 rebases
  after S-25.10, deletes its five intent-log copies and `AppendLogIntentLogRecoveryDecision`, and calls
  the shared module). **S-25.09** keeps only the `txn_id`/`intent_log_path`/DONE
  `txn_id`+`fencing_generation` plumbing.
- **S-25.06** owns the Tier 1 Branch C field consumption (`activation_id`, `generation_id`,
  `intent_log_path`, with the hash from the intent-log DONE record, confirmed against the INTENT) when verification is wired,
  and the `backfill-append-logs` binary leg (verifier/finalize: malformed ⇒
  `MIGRATION_STATE_INTEGRITY_FAILURE`, disagreeing ⇒ `COMPLETION_RECORD_MISMATCH_ABORT`; EC-047,
  EC-049..EC-052). **S-25.09** owns the `migrate-bc-index` coordinator leg (Tier 0 loader,
  strict-presence decode, `generation_id` tri-state; BC-1.18.011 EC-043..EC-047). Third
  binary-leg extension (ADR-052 v1.23 items 7(e), 9, 10, 11(c)-(e)): **S-25.08** owns the Branch B
  version gate and the sixth kind `txn_record_newer_schema` on the admission surface (EC-055, EC-056;
  code owned by S-25.09 per ADR v1.22); **S-25.09** owns the `migrate-bc-index` contention-on-every-path
  and under-lock read (BC-1.18.011 EC-051/EC-052); **S-25.06** owns the `backfill-append-logs` mirrors
  (EC-053, EC-054, EC-057), the Branch C version gate at the verifier, and replaces every
  "EWOULDBLOCK ⇒ exit 0" in its own ACs with contention exit 1.

**Delivery / promotion condition (POL-14, v1.11):** this BC is anchored by S-25.06, S-25.08 AND
S-25.09. Precondition 7 and Postcondition 10 land with S-25.09, so draft→active promotion at
merge MUST occur only after BOTH S-25.08 AND S-25.09 have merged (and then after S-25.06 for its
own obligations); a merge of S-25.08 alone does not satisfy this contract.

## VP Anchors

- **VP-143** (integration; allocated by architect, S-25.06 Spec-First Gate closure, 2026-09-25) —
  four-file all-or-nothing atomicity under crash/interruption AND idempotency two-layer
  consistency (`completed-backfill-append-logs.json` vs. BC-1.18.008's per-file manifest), consolidated per the
  single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established.
- **VP-144** (proptest; allocated by architect, S-25.06 Spec-First Gate closure, 2026-09-25) —
  per-file delegation-correctness against BC-1.18.008 §Postcondition 6(a)/(b), a differential test
  reusing VP-123's existing fixture-generation strategy.
- **VP-145** (integration/safety; allocated by architect, S-25.06 Spec-First Gate closure,
  2026-09-25) — closed-grammar rejection invariant for `backfill-append-logs`; two-layer
  defense-in-depth (pre-shell classifier + binary argument parser); no direct sibling in
  BC-1.18.011's VP set.
- **VP-146** (kani-proof; allocated by architect, S-25.06 Kani traceability-gap closure, POLICY 9,
  2026-09-26) — Kani prong of ADR-052 §Decision 12 for the governed mechanism-A migration's pure
  crash-recovery decision core (`decide_append_log_recovery`, txn state machine, modeled admission
  predicate, abstract `Fs` crash model). Six obligations, seven `#[kani::proof]` functions
  (`proof_obl_a1`..`a6`, a3 carrying two) in
  `crates/factory-dispatcher/src/shard_manager/append_log_kani_proofs.rs`. Clause mapping:
  a1 recovery totality — §Precondition 5, §Invariant 3, §EC-002; a2 recovery safety
  (content-verified, old-or-new-never-torn) — §Precondition 5, §Invariant 3, §Postcondition 3,
  §EC-002; a3 txn state-machine inductive step + bounded sequence — §Precondition 5,
  §Postcondition 3, §Postcondition 4, §Invariant 3; a4 admission-gate quiescence (INV-GATE-TXN) —
  §Precondition 6(b), §Precondition 6(c); a5 pointer-swap + canonical-move crash atomicity —
  §Precondition 5, §Postcondition 3, §Postcondition 3a, §Invariant 3, §EC-002; a6 recovery
  idempotence — §Precondition 5, §Invariant 3, §EC-002, §EC-003. Complements (does not replace) VP-143's real-filesystem fault-injection prong.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | Anchoring to CAP-043: "Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding" — because this BC describes the governed one-time migration that ACTUALLY EXECUTES mechanism A's backfill-split (BC-1.18.008) against the live append-log files and closes the loop into mechanism A's ongoing cap-triggered rotation (BC-1.18.005/006), which is exactly what CAP-043 defines per `capabilities.md` §CAP-043: "mechanism A shards four append-only cycle logs... [via] a native, dispatcher-mediated PreToolUse gate [that] intercepts every Edit/Write/MultiEdit" — CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043. No other capability covers a governed one-time migration executing mechanism A's algorithm; CAP-041 (INDETERMINATE detection/quarantine) and CAP-042 (the `rotate_changelog`/`last_amended` write-path fix) are both distinguishable per capabilities.md's own CAP-043 entry and neither covers this migration. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with the sibling BC-1.18.005–012 precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — one-time mechanism-A migration entry point in `shard_manager.rs` / `factory-dispatcher` CLI) |
| ADR | ADR-051 §Decision 2 (mechanism-A backfill obligation this BC discharges the invocation path for); ADR-052 §Decision 1 (mechanism selection); ADR-052 §Decision 2 (binary placement); ADR-052 §Decision 3 (closed argument grammar); ADR-052 §Decision 4 (armed-activation manifest); ADR-052 §Decision 5a (native admission gate); ADR-052 §Decision 6 (audit trail); ADR-052 §Decision 7a/7b/7c (crash-atomicity, lock ownership, atomic publication; 7b is a pointer to ADR-054 since v1.24); ADR-054 §Decision 1/2/3/4/5 (v1.13: hardened intent-log format, fixed `canonical_move_plan`, B-1/B-2/B-3, shared module, fault-injection mandate); ADR-052 §Decision 8 (POLICY 22 exception + allowlist); ADR-052 §Decision 9 (resolves S-25.06 Rule 7); ADR-052 §Decision 11 (executable verify-to-execute binding) |
| Stories | S-25.06; S-25.08 (shared admission core, Preconditions 6(a)–(e), Postconditions 5a/9, EC-020/EC-031 base/EC-032); S-25.09 (Precondition 6(b)(i) anchoring, Precondition 7, Invariants 7/8, Postcondition 10, EC-033..EC-040; ADR-052 v1.21 scope). POL-14: promote draft→active only after BOTH S-25.08 and S-25.09 merge (Precondition 7 and Postcondition 10 land with S-25.09). v1.12: S-25.08 owns the `resolve_factory_root` classification, the admission/release legs and Tier 0/Tier 1 (EC-041, EC-044..EC-048); S-25.09 owns the coordinator `Io` mapping, the `admission_failed` event and the release `admission_advisory` event (EC-042, EC-043); S-25.06 owns Tier 1 Branch C field consumption. v1.13: **S-25.10** (ADR-054) owns the shared `intent_log` module and the hardened format/fixed-plan/B-1..B-3 behavior that Precondition 5, Postcondition 5a(b) and EC-050..EC-052 specify for S-25.06 to consume; S-25.06 consumes (rebased after S-25.10); S-25.09 keeps only the `txn_id`/`intent_log_path`/DONE plumbing |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.13 | 2026-10-08 | product-owner | Application of ADR-054 v1.0 §Downstream "BC wording owed" and ADR-052 v1.24 (human-authorized spec amendments of 2026-10-08, CLAUDE.md rule 12; research `.factory/research/adr-052-intent-log-format-and-move-list-semantics.md`; bugs B-1/B-2/B-3; story S-25.10, S-25.06 consumes); shared-core sibling sweep with BC-1.18.011 v1.21. Additive. (1) Precondition 5: hardened intent-log format cited by ADR-054 §Decision 1 anchors (grammar not copied; POLICY 19); the ONE shared `intent_log` module (S-25.06's five copies and `AppendLogIntentLogRecoveryDecision` deleted); torn tail absent / mid-log corruption fails closed; fixed `canonical_move_plan` (N fixed at 4 = `len(plan)`), never rewritten per move; B-1/B-2/B-3 apply through the shared per-move procedure. (2) Branch C bullets (Precondition 6(c) Tier-1 rule 3 and Postcondition 5a(b)): hash source `DONE == INTENT == sha256(file)`; the "NOT a `pending_canonical_moves` field" bullets replaced by `canonical_move_plan` (rename); mid-log corruption and a plan differing from the INTENT set ⇒ `canonical_hash_mismatch`; absent/ill-typed/empty plan ⇒ `txn_record_malformed` (Tier 1). (3) NEW "AC-031 verifier steps" paragraph in Postcondition 5a(b) (shared module; mid-log corruption ⇒ `canonical_hash_mismatch` on verifier/admission surfaces; B-3 differing plan ⇒ `canonical_hash_mismatch` on verifiers, `txn_record_malformed` on the coordinator). (3a) ADR-054 §Decision 3.1 (architect-ratified stderr text, corrections C-1..C-4): EC-051's coordinator-leg `INTENT_LOG_CORRUPT` stderr line mirrored exactly (truthful "no further move or append was made and no completion was recorded"; no `BC-INDEX migration: ` label); the other two codes' lines are specified in BC-1.18.011 Postcondition 15 / the error-taxonomy rows and apply to this migration unchanged through the shared module. (3b) Orchestrator addition (ADR-052 v1.24 "Branch C hash source" ruling (i)): Precondition 5 gains the `generation_id`/`intent_log_path` PAIRING rule at the coordinator arm entry (mirrors BC-1.18.011 Precondition 6(f)(iii) and EC-069); EC-050 gains the pairing vectors and controls. (4) EC-050/EC-051/EC-052 extended and the intent-log-hash-source canonical vector extended with cases (j)-(o); NO new EC number (EC numbering unchanged). (5) Frontmatter inputs gain ADR-054; Architecture Anchors, Story Anchor (S-25.10) and Traceability updated. HISTORY rows (including the v1.12 row and the `last_amended` history) keep the old field name by design. VP citations changed: see the report (VP-143, VP-146 architect-owned, updated in parallel). Stories affected by BC changes: S-25.10 (new, story-writer), S-25.06, S-25.09. |
| 1.12 (third binary-leg extension, same version) | 2026-10-07 | product-owner | Application of the ADR-052 v1.23 "third binary-leg extension" (independent validation `.factory/research/adr-052-v123-architect-calls-validation.md`; items 7(e), 9, 10(a)-(d), 11(c)); extends v1.12 in place, mirrors BC-1.18.011 v1.20 (normative binary-coordinator text lives there). No new taxonomy CODE; one new kind token (`txn_record_newer_schema`, BC-3.08.001 v1.37). **(1)** Branch B / Branch C version gate (Precondition 6(c) rule 9; Branch B discard order: version gate first, then `generation_id` tri-state): `schema_version` integer ≥ 2 at any Tier 1 consumer ⇒ `E-MAINTENANCE-002` `state_integrity` kind `txn_record_newer_schema` (exactly one `migration.admission_failed` with `cause=state_integrity`, no `_blocked`, txn bytes unchanged); other present values ⇒ `txn_record_malformed`; absent ⇒ "not consumed"; unknown top-level keys preserved by the discard; carrier closed set now SIX tokens; EC-055, EC-056 (NEW) and the EC-032 vector updated. **(2)** `backfill-append-logs` exit-1 class and contention rule (Precondition 5/6 contention clause, Postcondition 5 lead-in, 5a(a) REPLACED "EWOULDBLOCK ⇒ warning, exit 0 `ALREADY_MIGRATED`" by exit 1 `MIGRATION_LOCK_CONTENTION` on every path, new 5a(a2) terminal record read UNDER the lock; EC-004, EC-016 updated; EC-053, EC-054 NEW); exhaustive `AppendLogMigrationError::process_exit_code` (EC-057 NEW). EC numbers EC-053..EC-057 are new (highest prior EC-052). Stories affected: S-25.06, S-25.08, S-25.09 (story-writer must propagate). |
| 1.12 | 2026-10-07 | product-owner | Application of ADR-052 v1.23 §Downstream (S-25.08 local adversary pass 3: F-S2508-L3-009, F-S2508-L3-001); shared-core sibling sweep with BC-1.18.011 v1.20. No state-machine, taxonomy-code, VP-count or module change; two interpretive rulings. **(F-S2508-L3-009, factory-root lookup)** Precondition 6(b)(iii) NEW "Absent vs unstatable `.factory`" paragraph: `factory_root` is OUT OF SCOPE only when `.factory` is ABSENT — the closed set {`stat` succeeds on a non-directory; `ENOENT` incl. a dangling symlink; `ENOTDIR`}; ANY other `stat` failure (EACCES, EPERM, EIO, ESTALE, ELOOP, …) is `E-MAINTENANCE-002` `io`, fail closed, one `migration.admission_failed`, no reservation, write not admitted; `resolve_factory_root` returns `Result<Option<FactoryRoot>, BcIndexMigrationError>` (the `is_ok_and(\|m\| m.is_dir())` collapse removed); per-leg mapping table (admission ⇒ `E-MAINTENANCE-002 (io)`; release ⇒ no verdict + `reservation_release_failed` advisory; coordinators ⇒ existing `Io`, not `FACTORY_ROOT_NOT_FOUND`); blast radius decided (an unstatable `.factory` blocks every Edit/Write/MultiEdit dispatch; bounded and loud). 6(b)(i) text and Precondition 7 conformed (`Ok(None)` = ABSENT; Precondition 7 gains the unstatable-coordinator clause: exit 2 `Io`, nothing created or mutated). Precondition 6(c) rule 2 `io` list gains the `.factory` `stat` (first OS call of the check). Postcondition 10(c): `reservation_release_failed` also fires for an unstatable `.factory` on the release leg. EC-035 reworded (dangling symlink and ENOTDIR are also absent); NEW EC-041 (admission), EC-042 (release), EC-043 (coordinator) + three test vectors. **(F-S2508-L3-001, txn-record tiers)** Precondition 6(c) rule 3 NEW "Txn-record interpretation — two tiers": Tier 0 (object, known `state`, string-if-present `migration_id`) is the only interpretation at read time, for every record; no full typed deserialize at read; Tier 1 fields are read lazily by the consuming branch only (Branch B `generation_id`: PRESENT `null` = null generation, string = not Branch B, ABSENT key or other type ⇒ `state_integrity` `txn_record_malformed` with no txn/gate write — an absent key is NEVER null; Branch C `activation_id`/`generation_id`/`pending_canonical_moves` at the check's position); shape-valid live records no branch consumes block plain `E-MAINTENANCE-001`; foreign records read no field beyond Tier 0; non-live COMPLETED/ABORTED records of valid Tier 0 shape (foreign or known) are admitted or Branch A reopens the gate, never modified; only live records count toward "more than one live txn". NEW EC-044..EC-048 + five test vectors (minimal foreign live per path family × gate state; known STAGING without `generation_id`; STAGING with `generation_id` null; known COMMITTING; non-live records). Story Anchor and Traceability Stories row record the attribution: S-25.08 resolver classification + admission/release legs + Tier 0/Tier 1; S-25.09 coordinator `Io` mapping + `admission_failed` and release `admission_advisory` events; S-25.06 Tier 1 Branch C field consumption. Status UNCHANGED (draft). BC-3.08.001 Events 12/13: no new field or token (`detail` may carry the `.factory` path) — no edit, remains v1.36. **Stories affected by BC changes:** S-25.08, S-25.09, S-25.06 — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change by PO. **VP citations changed in:** none (VP-147 unchanged: the planner is untouched; the shell resolves the Branch B tri-state before calling it). **Same-version extension (ADR-052 v1.23 binary-leg ruling "Migration binaries — recovery and finalize legs" + "Branch C hash source"; no version bump, no new code/variant/kind; sibling BC-1.18.011 v1.20 extended in place):** (1) Postcondition 5a(b) and its failure paragraph, and EC-047, gain the binary leg: a malformed Tier 1 txn field at its check ⇒ exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`; a well-formed disagreeing field ⇒ exit 2 `COMPLETION_RECORD_MISMATCH_ABORT`; an earlier check wins; neither finalizes nor flips. (2) Hash source corrected — every `pending_canonical_moves[].expected_post_hash` citation replaced by the intent-log DONE record's `expected_post_hash` located via the txn's Tier 1 `intent_log_path` (Precondition 6(c) rule 3 item (6) and its Tier 1 field list, Postcondition 5a(b), Postcondition 10(a) check (7), EC-010, Story Anchor; `pending_canonical_moves` is `{staging_path, canonical_path}` only). (3) NEW EC-049 (binary-leg malformed vs disagreeing vs ordering), EC-050 (`intent_log_path` absent/ill-typed ⇒ `state_integrity`), EC-051 (missing intent log or no DONE record ⇒ `canonical_hash_mismatch`), EC-052 (non-ENOENT intent-log read ⇒ `io` / `Io` exit 2) + two test vectors. Story attribution: S-25.06 owns `backfill-append-logs` recovery/verifier/finalize and Branch C verification; S-25.09 owns the `migrate-bc-index` coordinator leg. **Stories affected by BC changes:** S-25.06, S-25.09 (S-25.08 unchanged) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change by PO. **SECOND same-version extension (ADR-052 v1.23 items 8-11, "second binary-leg extension" Downstream block; no version bump):** (Q2/item 9) Precondition 5, the Postcondition 5a foreign-migration guard, EC-016 and its test vector replace "`LockContention`-class" with `FOREIGN_MIGRATION_REFUSED` (exit 2; exact `backfill-append-logs: refused: …` line, `<id>` truncated to 64 chars/control chars escaped; mirrored `AppendLogMigrationError::ForeignMigrationRefused`); flock contention is exit 1 `MIGRATION_LOCK_CONTENTION`, cited from `AppendLogMigrationError::LockContention`, checked before the loader. (Q4/item 11) EC-047(a)-(c) and EC-049 now read "`migrate-bc-index` §4e reconciliation (S-25.06 AC-031; fail-closed until then)" with a pointer to BC-1.18.011 Postcondition 9(e) / EC-050 instead of implying an existing verifier. Normative text for items 8, 10 and 11 lives in BC-1.18.011 (6(f)(iii)/(iv), 9(e), EC-043..EC-050); the `backfill-append-logs` `ExpiryAbort { arm }` equivalent is S-25.06's. No new EC row. **Stories affected by BC changes:** S-25.06, S-25.09 — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs. **VP citations changed in:** none. |
| 1.11 | 2026-10-07 | product-owner | Story-anchor re-attribution after the human-approved split of S-25.08 into S-25.08 + NEW S-25.09 (D-1252(f), amended so AC-027 stays in S-25.08). Documentary only — NO Precondition/Postcondition/Invariant/EC/test-vector semantics changed. Precondition 6(b)(i), Precondition 7, Invariants 7/8, Postcondition 10 (incl. the Branch C seam tokens `terminal_record_unverified` / `finalize_unwired` and the `branch_c_finalize_unwired` advisory), the EC-031 TTL seam, EC-033..EC-040 and their vectors now name S-25.09 as the delivering story (the six "S-25.08 seam" / "emitted by S-25.08" / "S-25.08 never emits it" / "S-25.08 build" / "S-25.08 delivers the function" prose sites re-pointed to S-25.09); the first-activation race, AC-018 `<cause>`, `PostToolUseFailure` fixture and shared-core obligations stay S-25.08. Story Anchor and Traceability Stories row now enumerate S-25.06 / S-25.08 / S-25.09 with the per-story obligation split. POL-14 delivery condition recorded: draft→active only after BOTH S-25.08 and S-25.09 merge. Also (ADR-052 v1.22 "Read-failure mapping" ruling, same version): Precondition 6(c) rule 2 `io` extended to a canonical file read on the Branch C verification path (a non-ENOENT read-call failure is `E-MAINTENANCE-002 (io)`, one `_failed`, never a `check` token; ENOENT of a canonical file = `canonical_hash_mismatch`) and EC-032 extended; "never emits `branch_c_finalized`" prose now reads "neither S-25.08 nor S-25.09" (only S-25.06 emits it). Sibling sweep: BC-1.18.011 v1.19, BC-3.08.001 v1.36. |
| 1.10 | 2026-10-07 | product-owner | Application of ADR-052 v1.21 §Downstream items 23–29 (S-25.08 local adversary pass 2: D-1/D-2/F-006/F-012/F-013/F-014); shared-core sibling sweep with BC-1.18.011 v1.18. (23, D-2) Precondition 6(b)(i): "the same anchor the coordinator binary uses" REPLACED by the single anchoring rule (`resolve_session_project_root` → `resolve_factory_root` → `FactoryRoot::migration_state_dir()`; ONE function used by admission, the release leg AND both coordinator binaries); NEW Precondition 7 (coordinator anchoring; no `.factory` ⇒ exit 2 `FACTORY_ROOT_NOT_FOUND`, `.factory` never created); NEW EC-033..EC-036 + four test vectors; NEW Invariant 8. (24, D-1) all nine `tracing::warn!` diagnostic mandates (Preconditions 6(b)/(c), Postcondition 5a, EC-029/EC-032 and their vectors) REPLACED by the `migration.admission_blocked` / `_failed` / `_advisory` InternalLog events (the 6(b)(iii) `tracing::debug!` developer breadcrumb is retained as a non-obligation); NEW Postcondition 10 (exactly one event per verdict or anomaly in `dispatcher-internal-YYYY-MM-DD.jsonl`; no raw `tool_use_id` or record content; sanitized/truncated fields per EC-029); NEW EC-037..EC-039 + vectors; coordinator diagnostics are stderr lines. (25) BC-3.08.001 v1.35 catalogues Events 11–13 (Related BCs/Architecture Anchors). (26, F-006) EC-031: "the PRODUCTION drain entry point is configured with" replaced by the crate-private `run_bc_index_migration_with_ttl(project_root, max_reservation_ttl)` seam (first statement `validate_production_reservation_ttl`) + compile-time const-assertion vector; the seam is NOT public and NOT env/argv-injectable. (27, F-012) the `state_integrity` carrier is named — `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` (five kinds), NOT `BinaryIntegrityFailure`; Display-lacks-`BINARY_INTEGRITY_FAILURE` vector; NEW Invariant 7 (`admission_failure_cause` is an exhaustive match, no wildcard); EC-032 extended. (28, F-013) Precondition 6(b) Decision: `blocking_plugins=migration-admission` (`NativeGate::MigrationAdmission`; the shard-cap leg keeps `shard-cap-gate`); NEW EC-040 + vector. (29, F-014) Precondition 6(b) "Where": single shared entry points `migration_writer_admission` / `migration_writer_admission_precheck` / `migration_reservation_release`; the "or its successor" hedge and the two per-migration delegate names deleted; EC-020 vector re-pointed. **Folded-in follow-up (ADR-052 v1.21 §Downstream item 33 (a)–(g), same version, uncommitted):** (g) undelivered finalize (`FinalizeThenOpenGate` with the S-25.08 seam) ⇒ the one `_blocked` carries `reconciliation=completion_record_mismatch`, `branch=completion_record_mismatch`, `check=finalize_unwired` plus the `_advisory` `branch_c_finalize_unwired` (replaces the earlier `branch=live_txn`); after S-25.06 delivers the finalize: no `_blocked`, `_advisory` `branch_c_finalized`, `finalize_unwired` retired (Postcondition 10(c), EC-039 vector); (a) Branch C verification failure = ONE `_blocked` (`branch=completion_record_mismatch`, `check`), no `_advisory`; (b) Postcondition 10(a) `reconciliation` replaced by the closed six-token effectful domain (`live_coordinator` / `nothing_to_reconcile` / `gate_reopened` / `null_generation_txn_aborted` / `foreign_migration_refused` / `completion_record_mismatch`; no `none`) + total `branch` derivation + a vector for every token (EC-037; corrects the decision-table names first used); (c) Postcondition 10(c)/EC-039 advisory `reason` domain = `reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired` / `branch_c_finalized`; the five timestamp tokens removed from the event and re-homed as coordinator-drain stderr tokens (6(c) rule (5), EC-025/EC-039(f)); `branch_c_finalize_unwired` fires with `FinalizeThenOpenGate` + undelivered finalize, IN ADDITION to the one `_blocked`, retired by S-25.06; `branch_c_finalized` is S-25.06-only; (d) optional advisory context fields closed set {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}; (e) Precondition 7/EC-035: variant `FactoryRootNotFound { project_root, root_source }` and the normative stderr line `<subcommand>: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root> (resolved from <source>)`, `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`}. **Folded-in follow-up 2 (S-25.08 implementation review @ f8726c34, same version, uncommitted):** (1) field renamed `source` → `root_source` everywhere (`thiserror` treats a field named `source` as the error source; stderr wording unchanged); (2) `root_source: Option<ProjectRootSource>` with normative rule (a): both coordinator CLIs MUST thread the whole `SessionProjectRoot` and ALWAYS print the `(resolved from <source>)` suffix; path-only library entries (`run_bc_index_migration(&Path)`, crate-private `run_bc_index_migration_with_ttl(&Path, ttl)`) are permitted with `root_source = None` and a suffix-less `Display` (Precondition 7, EC-035 + vector); (3) Postcondition 10(a): the Branch C `check` value domain is CLOSED — nine tokens {`staging_with_terminal_record`, `terminal_record_unparseable`, `terminal_record_schema_mismatch`, `txn_id_mismatch`, `generation_id_mismatch`, `canonical_paths_count_mismatch`, `canonical_hash_mismatch`, `terminal_record_unverified`, `finalize_unwired`} with a fixed first-failure-wins order, the two seam tokens retired by S-25.06, read-call failures staying `E-MAINTENANCE-002 (io)` (EC-037 + per-token vector). Status UNCHANGED (draft). |
| 1.9 | 2026-10-07 | product-owner | Application of ADR-052 v1.20 §Downstream item 21 (lexical root spellings, `FactoryRoot.lex_aliases` ratified) plus an internal-consistency ruling. (21) Precondition 6(b) (i)/(ii)/(iv)(e): `T_lex` is compared against the lexical normalization of EITHER spelling of the session's own root — (a) canonical `factory_root`, (b) `<CLAUDE_PROJECT_DIR as given>/.factory` (raw env, ignored if empty/non-absolute, no fs access, deduplicated) — and no other spelling; migration-state anchored on the REAL root regardless of spelling. EC-023 vector (g) (the ADR's "vector (f)"; lettered (g) because (f) was already the no-`.factory` case) and EC-024 vectors (k) canonical-vs-as-given identical classification and (l) look-alike/other-project no over-match; four TV rows. (Ruling) A Branch C terminal record that reads but does not verify (unparseable, non-UTF-8, empty/truncated, wrong schema, count/id/hash mismatch) now carries the completion-record-mismatch suffix everywhere (Precondition 6(b) keying text, 6(c) Scope-boundaries (b), EC-032 + its TVs, new TV row), resolving the prior inconsistency between "plain" (EC-032, 6(c)) and the suffix rows (EC-010/v1.4 D3 analogue, v1.6 TV, Postcondition 9); "plain" now explicitly defined = no suffix (gate-only, terminal record ABSENT, foreign refusal, live coordinator); a failed read CALL stays `E-MAINTENANCE-002 (io)`. Sibling sweep: BC-1.18.011 v1.17, error-taxonomy v1.38. |
| 1.8 | 2026-10-07 | product-owner | Application of ADR-052 v1.20 §Downstream items 14–19 (S-25.08 local adversary pass 1: F-001/F-002/F-003/F-004/F-006/F-008/F-009); shared-core sibling sweep with BC-1.18.011 v1.16. (14, F-004) Precondition 6(b) "Where": evaluation position replaced by obligations O1–O4 — exactly once; BEFORE `Registry::load` / `resolve_registry_path()` / the Tier-1 degraded branch (runs with `CLAUDE_PLUGIN_ROOT` unset, a missing/unparseable/schema-mismatched registry, an empty matched-plugin set); before `shard_cap_precheck` and every plugin tier; unparseable stdin keeps the existing exit; release leg registry-independent; reason recorded (registry-load arms are fail-open, the interlock must not be). (15, F-002/F-003) Precondition 6(b) "Which events / tools": events = `PreToolUse` + `PostToolUse` + `PostToolUseFailure`; new sub-bullets (i) Anchor (session `factory_root`, never created by the gate), (ii) component-wise Classification, (iii) Out-of-root `.factory` paths out of scope + accepted residual + §7c step-5 backstop, (iv) Target path resolution (`resolve_target_path` → `(T_real, T_lex)`; lexical + POSIX symlink walk; either-form match; unresolvable ⇒ lexical; relative path joined to payload `cwd`; `\` separator only on Windows; always case-insensitive, no probe), (v) residuals. (16, F-001/F-008/F-009) Precondition 6(c): release on `PostToolUse` OR `PostToolUseFailure` for the same `tool_use_id`, no `tool_name` filter, neither arriving ⇒ TTL backstop (replaces "(success or failure of the tool)"); NEW "Reservation timestamp rules" bullet (RFC 3339; pre-epoch/out-of-range ⇒ unparseable ⇒ mtime; `created_at > now + 300 s` ⇒ untrusted ⇒ mtime; in-tolerance future ⇒ age 0; both unusable ⇒ NOT stale + warn; no clamping; `reservation_is_stale(Option<u64>, Option<u64>, u64, u64)`); NEW "`tool_use_id` presence and validity" bullet (absent/`null` ⇒ check-only; present-but-invalid ⇒ fail closed `E-MAINTENANCE-002` `invalid_tool_use_id`; grammar `[A-Za-z0-9_.-]{1,128}`, no leading `.`); "Failure semantics" now names `E-MAINTENANCE-002`; production TTL floor violation named `RESERVATION_TTL_BELOW_FLOOR` (exit 2, nothing mutated). (17, F-006) Terminal-record reconciliation decision cell, Precondition 5, Postcondition 5a foreign-migration guard: "foreign" ⇔ live txn's `migration_id ∉ K`, K = {`migrate-bc-index`, `backfill-append-logs`} (absent field ⇒ `migrate-bc-index`); core input `txn_is_own_migration` renamed `txn_migration_known`; a live txn of EITHER known migration is decided against ITS OWN migration's terminal record selected by `migration_id` (the other's record is never consulted), so "foreign" never applies between the two known ids on the dispatcher path; cross-migration refusal (binary exit 2 `LockContention`-class) unchanged and binary-only; non-string `migration_id` ⇒ malformed record ⇒ `E-MAINTENANCE-002` `state_integrity`. (18) NEW append-only EC-021..EC-031 and 13 canonical test vector rows (PostToolUseFailure release; broken/absent registry and unset `CLAUDE_PLUGIN_ROOT`; out-of-root path / symlinked `.factory` / no-`.factory` project; path aliasing `..`/`./`/`//`/case/symlink/`link/..`/nonexistent tail/relative path; timestamp cases; invalid `tool_use_id`; other-known-migration live txn without/with its own record; unknown `migration_id`; non-string `migration_id`; production TTL below floor). EC-016 annotated "(binary recovery path only)"; EC-019 extended with the timestamp rules; EC-020 and Precondition 6(c) failure semantics name `E-MAINTENANCE-002` (`io`). (19, S-25.08 AC-018 ambiguity — folded into v1.8, uncommitted) Precondition 6(c) NEW sub-bullet "`E-MAINTENANCE-002` `<cause>` classification — total decision rule": `invalid_tool_use_id` = payload-only, decided before any filesystem access; `io` = an OS-level call by the admission check failed other than ENOENT of the file itself (create/rename/write/open/read/readdir/stat; EACCES/EPERM/EROFS/ENOSPC/EIO/EISDIR/ELOOP/EMFILE/short read), content never examined after a failed call; `state_integrity` = all bytes read but content unusable (non-UTF-8, empty/truncated/unparseable, wrong JSON type/shape — `gate-state.json` is a bare JSON string `"OPEN"`/`"DRAINING"`/`"LOCKED"` —, unknown state, PRESENT non-string `migration_id`, >1 live txn); first-failure-wins in the order tool_use_id → W1 → gate-state → txn (ascending filename) → terminal record; scope boundaries: reservation files are never read by admission (drain/EC-019 is a binary surface, not `E-MAINTENANCE-002`), terminal record read failure ⇒ `io` while unverifiable content is NOT `E-MAINTENANCE-002` (plain block, not finalized). Consistent with ADR-052 v1.20 §Error Code Semantics, whose "`state_integrity` (unreadable/malformed ...)" wording overlaps its "`io` (... or read gate/txn)" wording; the overlap is resolved by reading ADR "unreadable" as "bytes not interpretable as the record" (a failed read call is `io`), matching the existing `read_admission_gate_state` behaviour (read error ⇒ Io, parse error ⇒ integrity failure). NEW append-only EC-032 and one canonical test vector row (unreadable vs malformed `gate-state.json`, txn record and terminal-record matrix). No clause renumbered/removed. **Stories affected by BC changes:** S-25.08 (ACs/tasks for F-001/F-002/F-003/F-004/F-006/F-008/F-009), S-25.06 (inherits the "foreign" wording and the `txn_migration_known` rename only; no new ACs) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change by PO. **VP citations changed in:** none by PO (VP-133, VP-143, VP-147 bodies amended by the architect in the same ADR-052 v1.20 burst; no VP ID added/removed in this BC). |
| 1.7 | 2026-10-07 | product-owner | S-25.08 implementation finding — first-activation reservation race. The shared admission core carried a pre-existing guard returning "admit, no reservation" when `.factory/migration-state/` did not exist (pinned by test `test_BC_1_18_011_PRECOND6_admission_precheck_returns_none_when_no_migration_state_dir`), so a protected write admitted before the directory first existed was invisible to a coordinator that then created the directory, flipped DRAINING and polled `reservations/` (snapshot could race the write; only the §7c step-5 fingerprint recheck aborted the migration — safe but a spurious liveness abort). **Decision (option (a); (b) bounded quiescence window and (c) accept fingerprint-abort rejected — see rationale in Precondition 6(c)):** NEW Precondition 6(c) bullet "Unconditional reservation namespace — first-activation race closed" — the admitter ensures `migration-state/reservations/` by idempotent recursive create (never an existence guard), then W1/W2; absent gate ⇒ OPEN; the directory-absent bypass is REMOVED (no-op scope reduced to non-PreToolUse / non-Edit-Write-MultiEdit / path outside protected union / missing `tool_use_id` degradation); fail-closed on creation failure; cost accepted (one idempotent dir-create + reservation create/rename/unlink per protected write; no fsync, no lock; ADR-052 v1.18 perf acceptance). New EC-020 + 3 canonical test vector rows. No clause renumbered/removed. **TEST-WRITER CHANGE (exact):** in `crates/factory-dispatcher/tests/bc_1_18_011_b2_migration_test.rs`, test `test_BC_1_18_011_PRECOND6_admission_precheck_returns_none_when_no_migration_state_dir` must be REPLACED (rename to `..._admits_and_reserves_when_no_migration_state_dir`, keep BC-1.18.011 PRECOND6 prefix): same fixture (tempdir, protected-path target, `bc_index_payload` Write) but add `tool_use_id`; assert the verdict is still `None` (admitted) AND that `<dir>/.factory/migration-state/reservations/<tool_use_id>.reservation` now EXISTS (and parses with matching `tool_use_id`); keep a separate assertion that a payload WITHOUT `tool_use_id` creates no directory/reservation. Add the EC-020 vectors as new tests (drain-sees-pre-directory-writer; concurrent first admissions). **Stories affected by BC changes:** S-25.08 (AC/test additions; no `bcs:` array change by PO) — story-writer/test-writer propagate under bc_array_changes_propagate_to_body_and_acs. **VP citations changed in:** none textually (VP-133 reserve-then-verify facet and VP-146 a4 quiescence facet gain an obligation: reservation namespace exists from the first protected write — architect owns under ADR-052). **ADR-052 delta owed (architect):** see BC-1.18.011 v1.15 changelog row (exact text). |
| 1.6 | 2026-10-06 | product-owner | Resolution of four spec ambiguities surfaced by the S-25.08 Red Gate (test-writer, commit 1b28017e). (1) Precondition 6(b): NEW normative `E-MAINTENANCE-001` `<scope>` keying rule — keyed on the WRITTEN PATH FAMILY (`.factory/specs/behavioral-contracts/` ⇒ `BC-INDEX`; `.factory/cycles/` ⇒ `.factory/cycles/`), never on the live txn's migration (a gate-only block has no txn); exact format string and four-cell (path family × live migration) message table; migration identity only in the `tracing::warn!` diagnostic. (2) Gate self-healing: Branch B on-disk marker specified — in-place rewrite to `state=ABORTED` + top-level `abort_reason: "null_generation"`, `generation_id`/`source_sha256` stay null, txn file retained. (3) Own-migration + live txn + lock acquired + terminal record ABSENT ⇒ `NoOp` for STAGING and COMMITTING alike (decision table in BC-1.18.011 Precondition 6(d)). (4) Postcondition 5a: PreToolUse Branch C mismatch block message = keyed format string + ` (completion-record mismatch — operator investigation required)`, identical for STAGING+terminal-record and COMMITTING-verification-failure, both migrations, both path families. Canonical Test Vectors: five rows added, one row amended. No clause renumbered/removed. **Stories affected by BC changes:** S-25.08 (Red Gate test alignment; no `bcs:` array change by PO) — story-writer/test-writer propagate. **VP citations changed in:** none. **ADR-052 delta owed (architect):** see hand-off (marker field name; scope keying text). |
| 1.5 | 2026-10-06 | product-owner | ADR-052 v1.18 follow-up deltas 11 and 12 (architect review of v1.4; exact-text corrections, no behavior change). (11) STAGING wording: Postcondition 5a(b), the 5a failure paragraph and EC-010 now state "STAGING + terminal record is always fail-closed; no verification is attempted" (matches ADR §4e/Branch C; PO already implemented fail-closed). (12) Precondition 6(b) "Decision" bullet reordered: the step-0 reservation is created first; the §5a step-3.5 reconciliation then runs and may flip the gate to OPEN, after which the decision is re-evaluated (reserve-then-verify, 6(c)). **Stories affected by BC changes:** none (no `bcs:` array change; S-25.06 already anchors). **VP citations changed in:** none. inputs: dropped downstream error-taxonomy + S-25.06 to restore an acyclic input-hash graph (orchestrator, 2026-10-06); both remain prose cross-references. |
| 1.4 | 2026-10-06 | product-owner | ADR-052 v1.18 formal-finding exception ratification/amendment of v1.3 (human-approved 2026-10-06; v1.3 was the baseline the architect adjudicated, so the delta is a distinct version rather than an in-place edit of v1.3). **Precondition 6(b):** Tools bullet reduced to `Edit`/`Write`/`MultiEdit` — `Bash` removed from S-25.06's tested surface ([D-1232-OBL-4] §5c classifier ships later; POL-3 + §7c step-5 fingerprint recheck are the stated backstops) and a regression requirement added that the Rust gate leaves `Bash` unprocessed (EC-014); "Where" bullet rewritten from "alongside, never in place of" to a single shared admission core evaluated exactly once per event ahead of `shard_cap_precheck`; Exemption re-scoped to the OBL-4 classifier (Branch 1–4). **Precondition 6(c):** "atomically under `LOCK_SH`" replaced by reserve-then-verify (ADR-052 §5a step 0, Dekker argument recorded); release-on-block incl. dispatcher-side release when a later stage blocks; staleness by `created_at` with mtime fallback; 30 s / 3,600 s / 1,800 s production floor (test seam injectable) / no PID liveness unchanged. **Postcondition 5a / EC-010:** PreToolUse analogue of the mismatch abort is an `E-MAINTENANCE-001` block with reason logged; foreign-migration guard. **Precondition 5:** `migration_id: "backfill-append-logs"` on the txn record (absent ⇒ `migrate-bc-index`), cross-migration recovery refusal, per-migration terminal record/pointer namespace, `canonical_paths_count` (draft `file_count` retired). **Renames throughout:** `completed.json`→`completed-backfill-append-logs.json`, `CURRENT.json`→`CURRENT-backfill-append-logs.json`, `gate-state`→`gate-state.json` (v1.3 changelog row left verbatim as history). **New edge cases (append-only):** EC-014 (Bash unprocessed), EC-015 (D4 per-migration terminal-record namespace), EC-016 (D4 cross-migration refusal), EC-017 (D5 reserve-then-verify race), EC-018 (release-on-block), EC-019 (`created_at`/mtime staleness) + 11 canonical test vector rows; EC-013 and EC-010 text amended. No clause number removed. **Stories affected by BC changes:** S-25.06 (AC/test additions for the v1.4 deltas) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change. **VP citations changed in:** none textually (VP-143 facet (b)/VP-146 a4 clause meanings affected by reserve-then-verify — architect already owns under ADR-052 v1.18). |
| 1.3 | 2026-10-06 | product-owner | Semantic amendment closing three production defects found by S-25.06 formal verification (HEAD 9886cbc1). **D1 (gate never wired):** §Precondition 6(b) gained an explicit, testable WIRING OBLIGATION — `append_log_backfill_admission_precheck` MUST run on the production PreToolUse path in `main.rs` alongside `bc_index_migration_admission_precheck`/`shard_cap_precheck`; events PreToolUse (admit+reserve) / PostToolUse (release); tools Edit/Write/MultiEdit/write-effect Bash; protected path union `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/`; decision = gate OPEN ∧ no STAGING/COMMITTING txn else `E-MAINTENANCE-001`; the coordinator's own closed-grammar invocation is exempt/creates no reservation (no self-deadlock). **D2 (no writer drain):** §Precondition 6(c) made normative — reservation creator/timing (PreToolUse admit, `<tool_use_id>.reservation`, PostToolUse release), mandatory drain ordering (TTL GC → flock → DRAINING → txn STAGING(null) → poll → snapshot → LOCKED), 30 s timeout → `DRAIN_TIMEOUT_ABORT` with abort gate-reset, TTL-only stale reclamation (no PID liveness; 3,600 s default, ≥1,800 s floor), self-heal; injectable timeout/TTL for tests. **D3 (permanent self-lock):** new §Postcondition 5a — terminal-record reconciliation (verify `completed.json` + four files against the COMMITTING txn, finalize txn → COMPLETED, THEN gate → OPEN) precedes `ALREADY_MIGRATED`, mirrored in the PreToolUse self-heal; fail-closed `COMPLETION_RECORD_MISMATCH_ABORT` (NEW error code, error-taxonomy v1.32) on mismatch; §Postcondition 5 and EC-004 text aligned (no longer claims "no lock / zero mutation" unconditionally); new §Invariant 6 (no permanent self-lock; gate live; coherent terminal state). New Edge Cases EC-009..EC-013 (append-only) and 8 Canonical Test Vector rows. No existing clause number changed or removed; VP-146 clause-mapping text in this BC left as-is pending architect (meaning changes listed in the hand-off: a1/a3/a4/a6 and VP-143 facet (b)). ADR-052 amendment owed by architect (§Decision 4e table row + §Decision 5c Branch 2 + §Decision 5a step 3.5 Branch C for COMMITTING+completed.json; coordinator-invocation reservation exemption). **Stories affected by BC changes:** S-25.06 (AC/test additions for D1/D2/D3) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change. **VP citations changed in:** none textually (VP-146 clause meanings affected — architect to review). |
| 1.2 | 2026-09-26 | product-owner | VP-citation-only amendment (POLICY 9 propagation; anchor-back for architect's NEW VP-146, kani-proof, SS-01, anchor story S-25.06). Added VP-146 to §Verification Properties (new table row) and §VP Anchors (new bullet) with the clause mapping a1..a6 -> §Precondition 5 / §Precondition 6(b)/(c) / §Postcondition 3 / §Postcondition 3a / §Postcondition 4 / §Invariant 3 / §EC-002 / §EC-003 (a1/a2/a5/a6 each cite Precondition 5 plus Invariant 3; a3 adds Postconditions 3/4; a4 anchors solely to Precondition 6(b)/(c); all clause IDs verified to exist in this BC; mapping mirrors architect's corrected VP-146). VP-146 is the Kani prong of ADR-052 §Decision 12 (seven `#[kani::proof]` functions in `append_log_kani_proofs.rs`); VP-143 remains the real-filesystem fault-injection prong. Also consolidated the duplicate "VP-143" rows in §Verification Properties (a pre-existing v1.1 artifact of merging candidates 1 and 4) into ONE row with facets (a) atomicity and (b) idempotency two-layer consistency, no information lost; explanatory paragraph adjusted. No Precondition, Postcondition, Invariant, Edge Case, or Canonical Test Vector content changed. **VP citations changed in: BC-1.18.013.** Architect propagates to VP-INDEX/verification-architecture/verification-coverage-matrix. **Stories affected by BC changes:** none via `bcs:` array (BC-1.18.013 already anchored); S-25.06 body VP references -> story-writer. |
| 1.1 | 2026-09-25 | architect | Spec-First Gate 2 closure for S-25.06 (POLICY 9 propagation): allocated VP-143 (integration; four-file all-or-nothing atomicity + idempotency two-layer consistency, consolidating candidates 1 and 4 of the Verification Properties table per the single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established), VP-144 (proptest; per-file delegation-correctness differential test against BC-1.18.008 §PC6(a)/(b), candidate 2), and VP-145 (integration/safety; closed-grammar rejection invariant, candidate 3, no direct sibling in BC-1.18.011's VP set). Replaced the four `VP-NNN (pending)` placeholders in the Verification Properties table with these real IDs; updated VP Anchors accordingly. Full VP files authored at `.factory/specs/verification-properties/VP-143.md`/`VP-144.md`/`VP-145.md`. Propagated same-burst to `VP-INDEX.md` (v3.23→v3.24), `verification-architecture.md` (v1.36→v1.37), and `verification-coverage-matrix.md` (v1.34→v1.35) per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). No change to any Postcondition, Precondition, Invariant, Edge Case, or Canonical Test Vector — VP-citation-only amendment. input-hash recompute owed to state-manager (`compute-input-hash BC-1.18.013.md --update`). |
| 1.0 | 2026-09-25 | product-owner | Initial creation (NEW BC — closes S-25.06's Spec-First Gate, S-7.01). Allocated as BC-1.18.013, confirmed as the next free slot against the live `ss-01/` directory (BC-1.18.001–012 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism A's backfill-split of the four `v1.0-brownfield-backfill` append-log files, invoked via ADR-052's sanctioned execution path (`backfill-append-logs`, `Bash`-tool one-time interactive approval, closed argument grammar, armed-activation manifest, native admission gate, multi-file crash-atomicity across four independent files rather than one file's internal partition). Resolves the S-25.06 Architecture Compliance Rule 7 POL-3/native-CLI contradiction by direct reference to ADR-052 §Decision 9 (already ratified, D-1232, 2026-09-20) — NO new architecture decision was required; ADR-052 was authored with S-25.06 explicitly as an input and already generalizes its governed-migration state machine to mechanism-A migrations throughout (`migration_id: "backfill-append-logs"`, B2-only fields nulled for mechanism-A). Supersedes S-25.06's provisional AC-001(b) auto-discovery assumption with the ratified closed-grammar, fixed-four-file design (Precondition 3/Postcondition 6) — story-writer must update AC-001 accordingly. Adds Postcondition 8 specifying ShardRegistry enrollment (S-25.06 AC-005) as a SEPARATE ordinary Edit/Write step outside ADR-052's migration-binary write-target allowlist, using BC-1.18.005's existing `[[shard]]` schema, gated on a pre-existing `.factory/shard-config.toml` artifact-path-registry gap already identified by ADR-053 (routed to devops-engineer/architect, not blocking this BC's own dispatch-readiness). CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier per the established project convention (BC-1.18.011 v1.0 precedent). **Sibling BC updated in the same burst (Anchor-Back Rule):** BC-1.18.008 Related BCs gains a reciprocal reference to this BC (v1.9→v1.10, documentary-only, no semantic change). **error-taxonomy.md updated in the same burst:** `CENSUS_MISMATCH_ABORT` and `CONTENT_PRESERVATION_ABORT` MIG-category rows widened to explicitly cover the mechanism-A/`backfill-append-logs` trigger case (previously worded exclusively in B2/BC-INDEX per-row terms) — not deferred. **Stories affected by this BC (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.06 — add `BC-1.18.013` to `behavioral_contracts:` frontmatter array; propagate BC table, AC traces (superseding provisional AC-001–AC-008 with BC-1.18.013-anchored ACs), Token Budget, and Architecture Compliance Rule 7's OPEN-gate banner (now CLOSED, citing ADR-052 §Decision 9) in the SAME burst. **VP citations changed in: BC-1.18.013 (new).** Architect must propagate to `VP-INDEX.md`, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). |
