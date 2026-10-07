---
document_type: behavioral-contract
level: L3
version: "1.5"
status: draft
producer: product-owner
timestamp: 2026-09-25T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
input-hash: "c94d624"
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

5. A durable transaction record at `.factory/migration-state/txn-<activation_uuid>.json` and a
   framed checksummed intent log at `.factory/migration-state/intent-<generation_uuid>.log` are
   maintained across the full migration lifecycle (STAGING → COMMITTING → COMPLETED, or ABORTED),
   per ADR-052 §Decision 7a/7b exactly as BC-1.18.011 Precondition 5 specifies for B2. The
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
   belongs to `migrate-bc-index` it exits 2 (`LockContention`-class), performs NO mutation and
   MUST NOT run `recover()` over the foreign record (and symmetrically, per BC-1.18.011). The
   admission gate itself is migration-agnostic: any live txn of either migration blocks the union.

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
   - **Where (v1.4: single shared admission core, evaluated exactly once).** The gate is ONE
     shared admission core (ADR-052 §Decision 5a "Shared protected-path union and shared
     admission state"; implemented in `shard_manager.rs`) that BOTH named precheck entry points
     (`bc_index_migration_admission_precheck` and `append_log_backfill_admission_precheck`, or
     their successor) delegate to. `crates/factory-dispatcher/src/main.rs` MUST evaluate that
     core EXACTLY ONCE per PreToolUse event, at the position `bc_index_migration_admission_precheck`
     already holds on the production PreToolUse dispatch path — structurally AHEAD of
     `shard_cap_precheck` (whose fired branch performs a destructive seal-and-truncate and must
     never run for a blocked protected-path write) and before any registry plugin. It MUST NOT be
     evaluated twice (a second evaluation would re-create the reservation and re-run
     reconciliation). Which of the two entry points `main.rs` invokes is an implementation
     detail; the observable obligation is the black-box test through the real dispatcher entry
     (EC-013). Because both migrations share ONE
     gate-state.json file, ONE txn-record directory and ONE reservation directory under
     `.factory/migration-state/`, the protected path set enforced by the admission gate is the
     UNION `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/` (ADR-052 §Decision 5a):
     a STAGING/COMMITTING txn of EITHER migration blocks mutations under BOTH path families.
   - **Which events / tools.** Hook event `PreToolUse` (admission + reservation creation) and
     hook event `PostToolUse` (reservation release; see 6(c)). Tools: `Edit`, `Write`,
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
     target path outside the protected union (e.g. `.factory/STATE.md`,
     `.factory/stories/`), is NOT affected by this gate.
   - **Decision.** Admit iff `gate_state = OPEN` AND no txn record in state STAGING or
     COMMITTING exists (dual check, ADR-052 §Decision 5a step 4/5; PID liveness and flock
     ownership are irrelevant to the decision). Otherwise block with `E-MAINTENANCE-001`
     (`<scope>` = `.factory/cycles/` for a path under `.factory/cycles/`), exit 2 at the
     PreToolUse hook-block surface. The step-0 reservation is created first; the ADR-052
     §Decision 5a step-3.5 reconciliation (plus Postcondition 5a(c) below) then runs and may
     flip the gate to OPEN, after which the decision is re-evaluated (reserve-then-verify,
     6(c)).
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
     `tool_use_id` is the harness identifier shared by the PreToolUse/PostToolUse pair of the same
     tool call, and ONLY THEN read `gate-state.json`, scan `txn-*.json` and run the §Decision 5a
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
     first.) The PostToolUse hook for the same `tool_use_id` removes exactly that file (success
     or failure of the tool); a missing file at PostToolUse is a no-op. No reservation survives
     a blocked admission: `active_writer_count` = number of files in `reservations/`;
     quiescence = directory empty.
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
     `now − created_at > MAX_RESERVATION_TTL` (default 3,600 s; the PRODUCTION entry point MUST
     NOT be configured below the 1,800 s floor — a lower value is a configuration error; the
     TTL and drain timeout stay injectable parameters of the drain function for tests, the floor
     binds the production entry point, not the test seam) is removed. **Staleness is judged by
     the reservation file's `created_at` field (v1.4), falling back to the file mtime ONLY when
     the field is absent or unparseable**; never by PID/liveness (v1.9 H1, unchanged); a younger reservation is never removed by the coordinator and so
     blocks quiescence until its PostToolUse fires or the 30 s timeout elapses. A crashed
     session's abandoned reservation therefore yields `DRAIN_TIMEOUT_ABORT` on activations
     attempted before its TTL elapses; the sanctioned operator remediation is manual deletion of
     the confirmed-stale `*.reservation` file (ADR-052 §Decision 5a "Operator remediation").
   - **Gate self-healing.** After any crash, a stuck `LOCKED`/`DRAINING` gate with no active
     txn (or a STAGING txn with `generation_id=null` and no live coordinator) is returned to
     `OPEN` by the next PreToolUse via the flock-gated reconciliation of ADR-052 §Decision 5a
     step 3.5; `E-MAINTENANCE-001` never persists across a crashed abort.

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
   is the sole lock operation permitted on this path; and `completed-backfill-append-logs.json`'s authority is
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
    (a) Attempt a non-blocking `flock(exclusive.lock, LOCK_EX|LOCK_NB)`. On `EWOULDBLOCK` (live
        coordinator — unexpected in terminal state): log a warning, perform NO reconciliation and
        NO mutation, exit 0 `ALREADY_MIGRATED`.
    (b) On acquisition, select the txn record by the ADR-052 §Decision 4e txn-selection rule. If
        no txn record is in state STAGING or COMMITTING (clean steady state, COMPLETED, or
        ABORTED), skip to (d). If a STAGING txn exists alongside `completed-backfill-append-logs.json`,
        fail closed per the failure paragraph below (STAGING + terminal record is always
        fail-closed; no verification is attempted). If a COMMITTING txn exists, VERIFY before trusting `completed-backfill-append-logs.json`:
        `completed-backfill-append-logs.json` parses; its `txn_id` equals the txn record's `activation_id` and its
        `generation_id` equals the txn record's `generation_id`; its `canonical_paths_count` is 4;
        and for each of the four canonical files `sha256(file)` equals the `expected_post_hash`
        recorded in the txn record/intent log (content-verified — the claim in `completed-backfill-append-logs.json` is
        checked, not assumed). On success: atomically rewrite the txn record to `state =
        COMPLETED` (write-temp + fsync + rename + dir-sync, per ADR-052 §Decision 7d). This
        finalization is idempotent and a crash during it leaves the same recoverable state
        (re-running repeats (a)–(d)).
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
    `COMPLETION_RECORD_MISMATCH_ABORT` (fail-closed integrity anomaly; EC-010). **PreToolUse
    analogue (v1.4):** when the same verification fails on the §Decision 5a step-3.5 Branch C
    path (no binary invocation), the PreToolUse outcome is an `E-MAINTENANCE-001` block with the
    mismatch reason logged (a structured `tracing::warn!` naming `migration_id`, `txn_id` and
    the failing check; block message suffix `(completion-record mismatch — operator
    investigation required)`) — NOT the binary exit code — and the same
    zero-write guarantee holds (no txn write, no gate write; a byte-for-byte snapshot of
    `.factory/migration-state/` is unchanged by the attempt). **Foreign-migration guard (v1.4):**
    this reconciliation concerns ONLY a live txn with `migration_id = "backfill-append-logs"`
    and ONLY this migration's own terminal record (`completed-backfill-append-logs.json`,
    Precondition 5); it MUST NEVER finalize a txn whose `migration_id` is `"migrate-bc-index"` on
    the strength of this record (nor the reverse); a foreign live txn is refused per Precondition
    5 / EC-016. In the clean
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

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Postcondition 1 (content-preservation) passes for three files but Postcondition 2 (independent census) fails for the fourth | Migration ABORTS for ALL FOUR files per Postcondition 4 — a per-file failure is a whole-migration failure; the three passing files' original content is also left untouched, not silently migrated ahead of the failing one |
| EC-002 | Migration crashes after completing canonical path moves for 2 of the 4 files (post-pointer-swap, mid-step-7) | ADR-052 §Decision 7c step 7's forward recovery resumes from the intent log's first uncompleted move (matching-destination-hash rule); the 2 already-moved files are NOT re-moved; the 2 remaining files complete on the recovery pass; no file is left half-migrated since each individual file's own `rename(2)` is atomic |
| EC-003 | A prior activation attempt built a fully-staged generation for all four files but crashed before the CURRENT-backfill-append-logs.json pointer swap | Re-running MUST detect txn record state STAGING and resume toward the pointer swap; per ADR-052 §Decision 7c step 3b (referenced by §Decision 4e), the resume path MUST re-run the FULL per-file census (Postconditions 1/2) for all four files before proceeding — it does NOT skip re-verification merely because a staged generation was previously built, mirroring BC-1.18.011 EC-003 |
| EC-004 | `backfill-append-logs` is invoked after a prior activation already reached `completed-backfill-append-logs.json` | `ALREADY_MIGRATED` (exit 0); no manifest required; no migration lock held (only the non-blocking `exclusive.lock` probe of Postcondition 5a(a)); in the clean steady state (no active txn, gate OPEN) the binary performs zero filesystem mutation and exits immediately; if a stale COMMITTING txn / non-OPEN gate is present, Postcondition 5a reconciliation applies first (EC-009) |
| EC-005 | One of the four canonical files does not exist, or is unreadable, at activation time (S-25.06 story EC-004) | The migration ABORTS before acquiring the flock or building any staging generation (fails the pre-lock/under-exclusion validation, ADR-052 §Decision 4b/4c) with a clear non-zero exit; no partial state is written for any of the four files |
| EC-006 | A single record within one of the four files exceeds `shard_cap_bytes` on its own (BC-1.18.008 EC-002) | NOT a migration-level abort: BC-1.18.008's own oversized-record exception (`oversized_record: true`) applies to that file's shard exactly as BC-1.18.008 specifies; this governing BC's Postcondition 1/2 census-agreement gate tolerates the flagged exception and proceeds with the other files' and that file's remaining shards normally |
| EC-007 | `record_boundary_offsets` computed from the marker regex yields zero boundaries for one of the four files despite non-empty content (S-25.06 story EC-006; BC-1.18.008's O-1 note that this is unreachable in production for the four recognized artifact stems) | Hard failure via BC-1.18.008 Postcondition 6's fail-loud content-preservation gate for that file, surfaced by this migration as `CONTENT_PRESERVATION_ABORT` (exit 2) for the WHOLE migration per Postcondition 4 — never a silent empty-oracle partition |
| EC-008 | `backfill-append-logs` is invoked with a path argument, a `--cycle` flag, or any token outside the exact closed grammar (`backfill-append-logs` / `backfill-append-logs --census`) | REJECTED by the pre-shell classifier (ADR-052 §Decision 5c) before the binary is even invoked; if somehow bypassed, the binary itself rejects with a non-zero exit (ADR-052 §Decision 3) — this supersedes the S-25.06 story's provisional AC-001(b) cycle-directory-argument assumption (Precondition 3) |
| EC-009 | (v1.3, D3) Crash AFTER `completed-backfill-append-logs.json` is durable but BEFORE the txn record is rewritten COMPLETED in step 8: `completed-backfill-append-logs.json` present, txn record COMMITTING (matching `activation_id`/`generation_id`), `gate_state = LOCKED`, all four canonical files at their post-migration hashes | The admission gate (6(b)) blocks `.factory/cycles/` writes with `E-MAINTENANCE-001` ONLY until reconciliation runs. Reconciliation — either a PreToolUse self-heal (Postcondition 5a(c)) or a re-invocation of `backfill-append-logs` (Postcondition 5a(a)–(d)) — verifies `completed-backfill-append-logs.json` against the txn record and the four files' `expected_post_hash`, rewrites the txn record to COMPLETED, THEN flips the gate to OPEN; the invocation exits 0 `ALREADY_MIGRATED`. A second re-run is a zero-mutation `ALREADY_MIGRATED`. A crash during the reconciliation itself leaves the same recoverable state (idempotent) |
| EC-010 | (v1.3, D3) `completed-backfill-append-logs.json` present but cannot be matched to the active non-terminal txn record: unparseable `completed-backfill-append-logs.json`; `canonical_paths_count ≠ 4`; `txn_id`/`generation_id` ≠ the txn record's; txn record STAGING (not COMMITTING — STAGING + terminal record is always fail-closed; no verification is attempted); or any canonical file's `sha256` ≠ `expected_post_hash` | Fail-closed: NO txn finalization, NO gate flip, gate stays blocking, exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` from the binary; the PreToolUse analogue (Branch C, no binary invocation) is an `E-MAINTENANCE-001` block with the mismatch reason logged (v1.4). Never silently "repair" an inconsistent terminal state — a human investigates; a byte-for-byte snapshot of `.factory/migration-state/` is unchanged by either path |
| EC-011 | (v1.3, D2) An admitted writer holds a reservation (PreToolUse fired, PostToolUse not yet fired) when the coordinator begins DRAINING and it is not released within the 30 s drain timeout | `DRAIN_TIMEOUT_ABORT` (exit 2): txn record → ABORTED, `gate_state` → OPEN, `exclusive.lock` released, no staging generation, all four files byte-identical, no `source_sha256` ever recorded; a later activation retried after the writer completes succeeds (Precondition 6(c)) |
| EC-012 | (v1.3, D2) A reservation file exists whose creating writer crashed (no PostToolUse will ever fire): (a) `now − created_at > MAX_RESERVATION_TTL`; (b) `now − created_at ≤ MAX_RESERVATION_TTL` | (a) removed by drain step 1 TTL GC; drain reaches quiescence normally. (b) NEVER removed by the coordinator (no PID-liveness inference — an in-flight live writer's reservation looks identical); drain waits then `DRAIN_TIMEOUT_ABORT` at 30 s; remediation is operator deletion of the confirmed-stale file (Precondition 6(c)) |
| EC-013 | (v1.3, D1; v1.4 Bash removed) A mutation tool call (`Edit`/`Write`/`MultiEdit` ONLY — write-effect `Bash` is the [D-1232-OBL-4] classifier's, not S-25.06's tested surface; until it ships POL-3 + the §Decision 7c step-5 fingerprint recheck are the backstops) targets a path under `.factory/cycles/` (any cycle) or `.factory/specs/behavioral-contracts/` while a txn record is STAGING or COMMITTING, delivered through the REAL dispatcher entry point (PreToolUse payload to the `factory-dispatcher` binary, not a direct unit call of the precheck) | Blocked: `E-MAINTENANCE-001`, exit 2, `<scope>` = `.factory/cycles/`; a path outside the protected union (e.g. `.factory/STATE.md`) is admitted; with no active txn and gate OPEN the same call is admitted AND `reservations/<tool_use_id>.reservation` exists until the matching PostToolUse removes it (Precondition 6(b)/(c)); the shared admission core is evaluated exactly ONCE per event (the reservation is written once; reconciliation is not re-run) |
| EC-014 | (v1.4, D1 re-scope) The dispatcher receives a PreToolUse envelope for the `Bash` tool — including (a) a write-effect command whose target is a protected path and (b) the coordinator's own closed-grammar `backfill-append-logs` / `backfill-append-logs --census` invocation — through the REAL dispatcher entry | The Rust admission gate leaves `Bash` UNPROCESSED: it creates NO reservation, returns no decision (`None`) and writes nothing under `.factory/migration-state/`, even with txn STAGING/COMMITTING; consequently the coordinator can never self-deadlock through it. Blocking/classification of write-effect `Bash` is [D-1232-OBL-4] (backstops until it ships: POL-3 `^Bash$` guards + §Decision 7c step-5 fingerprint recheck). Any future Bash leg in the Rust gate must carry the §Decision 5c Branch 1–4 coordinator exemption |
| EC-015 | (v1.4, D4) Per-migration terminal-record namespace: (a) `completed.json` / `CURRENT.json` exist (a finished `migrate-bc-index`) but `completed-backfill-append-logs.json` does not; (b) `completed-backfill-append-logs.json` exists but `completed.json` does not | (a) `backfill-append-logs` is NOT `ALREADY_MIGRATED`: it proceeds as a first activation (and never reads `completed.json` as its own terminal record). (b) `migrate-bc-index` is not `ALREADY_MIGRATED` by it, and BC-1.18.010's reader protocol / `detect_migration_read_state` does NOT report the BC-INDEX migration COMPLETE on its strength (BC-INDEX readers do not switch to shard paths that do not exist). This binary never writes `completed.json`/`CURRENT.json` |
| EC-016 | (v1.4, D4) The single live txn (STAGING or COMMITTING) has `migration_id = "migrate-bc-index"` (or lacks the field, read as that) when `backfill-append-logs` is invoked, or the reverse | Cross-migration refusal: exit 2 (`LockContention`-class), NO mutation (byte-for-byte snapshot of `.factory/migration-state/` unchanged), `recover()` is NOT run over the foreign record, the foreign txn is not finalized/aborted; the admission gate still blocks the union for either migration's live txn. A foreign terminal record never finalizes a live txn of this migration (Postcondition 5a foreign-migration guard) |
| EC-017 | (v1.4, D5) Reserve-then-verify interleaving of an admitter (W1 = create reservation, W2 = read gate) against the coordinator (C1 = durable DRAINING flip, C2 = read `reservations/`) at every ordering of {W1, C1, C2, W2} | In every ordering EITHER the coordinator's C2 observes the reservation (and waits; `source_sha256` not computed while it exists) OR the admitter's W2 observes DRAINING (admission refused `E-MAINTENANCE-001` and its own reservation removed) — NEVER both miss; in particular an admitter that reads `OPEN` from a gate file the coordinator subsequently replaces via `rename` is still observed by the coordinator |
| EC-018 | (v1.4, D2 release-on-block) Admission creates the reservation, then a LATER stage of the same dispatch (`shard_cap_precheck` or any registry plugin) blocks or errors; or the admitter's own verification fails | The dispatcher removes the reservation its admission created before exiting; NO `reservations/<tool_use_id>.reservation` remains for a blocked/errored event (a harness-level denial the dispatcher cannot observe leaks until PostToolUse/TTL — documented residual) |
| EC-019 | (v1.4) A reservation file whose `created_at` is parseable vs. absent/unparseable | Staleness uses `created_at`; mtime is the fallback ONLY when `created_at` is absent/unparseable (e.g. `created_at` 4,000 s ago with a fresh mtime ⇒ stale under TTL 3,600 s; absent `created_at` with mtime 4,000 s ago ⇒ stale) |

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
| (v1.4, D4) Live txn `{migration_id: "migrate-bc-index", state: COMMITTING}` (or no `migration_id` field); invoke `backfill-append-logs` (and the reverse fixture for `migrate-bc-index`) | Exit 2, zero mutation, no `recover()`; foreign txn untouched (EC-016) | error |
| (v1.4, D4) Live txn `{migration_id: "migrate-bc-index", state: COMMITTING}` with a VALID `completed-backfill-append-logs.json` present, PreToolUse Edit under `.factory/cycles/` | NOT finalized by Branch C on that record; Edit blocked `E-MAINTENANCE-001`; txn unchanged (EC-016, Postcondition 5a foreign-migration guard) | error |
| (v1.4, D3 PreToolUse analogue) Branch C verification fails (e.g. `canonical_paths_count` = 3) on a PreToolUse Edit, no binary re-invocation | `E-MAINTENANCE-001` block; reason logged names `canonical_paths_count`; txn/gate/terminal record byte-identical (EC-010) | error |
| (v1.4) Reservation JSON with `created_at = now − 4000 s` but mtime = now; TTL 3,600 s (test seam) | Judged stale (removed at drain step 1); and reservation with no `created_at` field and mtime `now − 4000 s` ⇒ stale via fallback (EC-019) | edge-case |

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
- ADR-052 §Decision 6 — audit trail (NIST AU-9): census stdout + durable factory-artifacts commit
- ADR-052 §Decision 7a/7b/7c — advisory flock + durable txn record; framed intent log + WAL
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
| ADR | ADR-051 §Decision 2 (mechanism-A backfill obligation this BC discharges the invocation path for); ADR-052 §Decision 1 (mechanism selection); ADR-052 §Decision 2 (binary placement); ADR-052 §Decision 3 (closed argument grammar); ADR-052 §Decision 4 (armed-activation manifest); ADR-052 §Decision 5a (native admission gate); ADR-052 §Decision 6 (audit trail); ADR-052 §Decision 7a/7b/7c (crash-atomicity, lock ownership, atomic publication); ADR-052 §Decision 8 (POLICY 22 exception + allowlist); ADR-052 §Decision 9 (resolves S-25.06 Rule 7); ADR-052 §Decision 11 (executable verify-to-execute binding) |
| Stories | S-25.06 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.5 | 2026-10-06 | product-owner | ADR-052 v1.18 follow-up deltas 11 and 12 (architect review of v1.4; exact-text corrections, no behavior change). (11) STAGING wording: Postcondition 5a(b), the 5a failure paragraph and EC-010 now state "STAGING + terminal record is always fail-closed; no verification is attempted" (matches ADR §4e/Branch C; PO already implemented fail-closed). (12) Precondition 6(b) "Decision" bullet reordered: the step-0 reservation is created first; the §5a step-3.5 reconciliation then runs and may flip the gate to OPEN, after which the decision is re-evaluated (reserve-then-verify, 6(c)). **Stories affected by BC changes:** none (no `bcs:` array change; S-25.06 already anchors). **VP citations changed in:** none. inputs: dropped downstream error-taxonomy + S-25.06 to restore an acyclic input-hash graph (orchestrator, 2026-10-06); both remain prose cross-references. |
| 1.4 | 2026-10-06 | product-owner | ADR-052 v1.18 formal-finding exception ratification/amendment of v1.3 (human-approved 2026-10-06; v1.3 was the baseline the architect adjudicated, so the delta is a distinct version rather than an in-place edit of v1.3). **Precondition 6(b):** Tools bullet reduced to `Edit`/`Write`/`MultiEdit` — `Bash` removed from S-25.06's tested surface ([D-1232-OBL-4] §5c classifier ships later; POL-3 + §7c step-5 fingerprint recheck are the stated backstops) and a regression requirement added that the Rust gate leaves `Bash` unprocessed (EC-014); "Where" bullet rewritten from "alongside, never in place of" to a single shared admission core evaluated exactly once per event ahead of `shard_cap_precheck`; Exemption re-scoped to the OBL-4 classifier (Branch 1–4). **Precondition 6(c):** "atomically under `LOCK_SH`" replaced by reserve-then-verify (ADR-052 §5a step 0, Dekker argument recorded); release-on-block incl. dispatcher-side release when a later stage blocks; staleness by `created_at` with mtime fallback; 30 s / 3,600 s / 1,800 s production floor (test seam injectable) / no PID liveness unchanged. **Postcondition 5a / EC-010:** PreToolUse analogue of the mismatch abort is an `E-MAINTENANCE-001` block with reason logged; foreign-migration guard. **Precondition 5:** `migration_id: "backfill-append-logs"` on the txn record (absent ⇒ `migrate-bc-index`), cross-migration recovery refusal, per-migration terminal record/pointer namespace, `canonical_paths_count` (draft `file_count` retired). **Renames throughout:** `completed.json`→`completed-backfill-append-logs.json`, `CURRENT.json`→`CURRENT-backfill-append-logs.json`, `gate-state`→`gate-state.json` (v1.3 changelog row left verbatim as history). **New edge cases (append-only):** EC-014 (Bash unprocessed), EC-015 (D4 per-migration terminal-record namespace), EC-016 (D4 cross-migration refusal), EC-017 (D5 reserve-then-verify race), EC-018 (release-on-block), EC-019 (`created_at`/mtime staleness) + 11 canonical test vector rows; EC-013 and EC-010 text amended. No clause number removed. **Stories affected by BC changes:** S-25.06 (AC/test additions for the v1.4 deltas) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change. **VP citations changed in:** none textually (VP-143 facet (b)/VP-146 a4 clause meanings affected by reserve-then-verify — architect already owns under ADR-052 v1.18). |
| 1.3 | 2026-10-06 | product-owner | Semantic amendment closing three production defects found by S-25.06 formal verification (HEAD 9886cbc1). **D1 (gate never wired):** §Precondition 6(b) gained an explicit, testable WIRING OBLIGATION — `append_log_backfill_admission_precheck` MUST run on the production PreToolUse path in `main.rs` alongside `bc_index_migration_admission_precheck`/`shard_cap_precheck`; events PreToolUse (admit+reserve) / PostToolUse (release); tools Edit/Write/MultiEdit/write-effect Bash; protected path union `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/`; decision = gate OPEN ∧ no STAGING/COMMITTING txn else `E-MAINTENANCE-001`; the coordinator's own closed-grammar invocation is exempt/creates no reservation (no self-deadlock). **D2 (no writer drain):** §Precondition 6(c) made normative — reservation creator/timing (PreToolUse admit, `<tool_use_id>.reservation`, PostToolUse release), mandatory drain ordering (TTL GC → flock → DRAINING → txn STAGING(null) → poll → snapshot → LOCKED), 30 s timeout → `DRAIN_TIMEOUT_ABORT` with abort gate-reset, TTL-only stale reclamation (no PID liveness; 3,600 s default, ≥1,800 s floor), self-heal; injectable timeout/TTL for tests. **D3 (permanent self-lock):** new §Postcondition 5a — terminal-record reconciliation (verify `completed.json` + four files against the COMMITTING txn, finalize txn → COMPLETED, THEN gate → OPEN) precedes `ALREADY_MIGRATED`, mirrored in the PreToolUse self-heal; fail-closed `COMPLETION_RECORD_MISMATCH_ABORT` (NEW error code, error-taxonomy v1.32) on mismatch; §Postcondition 5 and EC-004 text aligned (no longer claims "no lock / zero mutation" unconditionally); new §Invariant 6 (no permanent self-lock; gate live; coherent terminal state). New Edge Cases EC-009..EC-013 (append-only) and 8 Canonical Test Vector rows. No existing clause number changed or removed; VP-146 clause-mapping text in this BC left as-is pending architect (meaning changes listed in the hand-off: a1/a3/a4/a6 and VP-143 facet (b)). ADR-052 amendment owed by architect (§Decision 4e table row + §Decision 5c Branch 2 + §Decision 5a step 3.5 Branch C for COMMITTING+completed.json; coordinator-invocation reservation exemption). **Stories affected by BC changes:** S-25.06 (AC/test additions for D1/D2/D3) — story-writer must propagate under bc_array_changes_propagate_to_body_and_acs; no `bcs:` array change. **VP citations changed in:** none textually (VP-146 clause meanings affected — architect to review). |
| 1.2 | 2026-09-26 | product-owner | VP-citation-only amendment (POLICY 9 propagation; anchor-back for architect's NEW VP-146, kani-proof, SS-01, anchor story S-25.06). Added VP-146 to §Verification Properties (new table row) and §VP Anchors (new bullet) with the clause mapping a1..a6 -> §Precondition 5 / §Precondition 6(b)/(c) / §Postcondition 3 / §Postcondition 3a / §Postcondition 4 / §Invariant 3 / §EC-002 / §EC-003 (a1/a2/a5/a6 each cite Precondition 5 plus Invariant 3; a3 adds Postconditions 3/4; a4 anchors solely to Precondition 6(b)/(c); all clause IDs verified to exist in this BC; mapping mirrors architect's corrected VP-146). VP-146 is the Kani prong of ADR-052 §Decision 12 (seven `#[kani::proof]` functions in `append_log_kani_proofs.rs`); VP-143 remains the real-filesystem fault-injection prong. Also consolidated the duplicate "VP-143" rows in §Verification Properties (a pre-existing v1.1 artifact of merging candidates 1 and 4) into ONE row with facets (a) atomicity and (b) idempotency two-layer consistency, no information lost; explanatory paragraph adjusted. No Precondition, Postcondition, Invariant, Edge Case, or Canonical Test Vector content changed. **VP citations changed in: BC-1.18.013.** Architect propagates to VP-INDEX/verification-architecture/verification-coverage-matrix. **Stories affected by BC changes:** none via `bcs:` array (BC-1.18.013 already anchored); S-25.06 body VP references -> story-writer. |
| 1.1 | 2026-09-25 | architect | Spec-First Gate 2 closure for S-25.06 (POLICY 9 propagation): allocated VP-143 (integration; four-file all-or-nothing atomicity + idempotency two-layer consistency, consolidating candidates 1 and 4 of the Verification Properties table per the single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established), VP-144 (proptest; per-file delegation-correctness differential test against BC-1.18.008 §PC6(a)/(b), candidate 2), and VP-145 (integration/safety; closed-grammar rejection invariant, candidate 3, no direct sibling in BC-1.18.011's VP set). Replaced the four `VP-NNN (pending)` placeholders in the Verification Properties table with these real IDs; updated VP Anchors accordingly. Full VP files authored at `.factory/specs/verification-properties/VP-143.md`/`VP-144.md`/`VP-145.md`. Propagated same-burst to `VP-INDEX.md` (v3.23→v3.24), `verification-architecture.md` (v1.36→v1.37), and `verification-coverage-matrix.md` (v1.34→v1.35) per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). No change to any Postcondition, Precondition, Invariant, Edge Case, or Canonical Test Vector — VP-citation-only amendment. input-hash recompute owed to state-manager (`compute-input-hash BC-1.18.013.md --update`). |
| 1.0 | 2026-09-25 | product-owner | Initial creation (NEW BC — closes S-25.06's Spec-First Gate, S-7.01). Allocated as BC-1.18.013, confirmed as the next free slot against the live `ss-01/` directory (BC-1.18.001–012 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism A's backfill-split of the four `v1.0-brownfield-backfill` append-log files, invoked via ADR-052's sanctioned execution path (`backfill-append-logs`, `Bash`-tool one-time interactive approval, closed argument grammar, armed-activation manifest, native admission gate, multi-file crash-atomicity across four independent files rather than one file's internal partition). Resolves the S-25.06 Architecture Compliance Rule 7 POL-3/native-CLI contradiction by direct reference to ADR-052 §Decision 9 (already ratified, D-1232, 2026-09-20) — NO new architecture decision was required; ADR-052 was authored with S-25.06 explicitly as an input and already generalizes its governed-migration state machine to mechanism-A migrations throughout (`migration_id: "backfill-append-logs"`, B2-only fields nulled for mechanism-A). Supersedes S-25.06's provisional AC-001(b) auto-discovery assumption with the ratified closed-grammar, fixed-four-file design (Precondition 3/Postcondition 6) — story-writer must update AC-001 accordingly. Adds Postcondition 8 specifying ShardRegistry enrollment (S-25.06 AC-005) as a SEPARATE ordinary Edit/Write step outside ADR-052's migration-binary write-target allowlist, using BC-1.18.005's existing `[[shard]]` schema, gated on a pre-existing `.factory/shard-config.toml` artifact-path-registry gap already identified by ADR-053 (routed to devops-engineer/architect, not blocking this BC's own dispatch-readiness). CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier per the established project convention (BC-1.18.011 v1.0 precedent). **Sibling BC updated in the same burst (Anchor-Back Rule):** BC-1.18.008 Related BCs gains a reciprocal reference to this BC (v1.9→v1.10, documentary-only, no semantic change). **error-taxonomy.md updated in the same burst:** `CENSUS_MISMATCH_ABORT` and `CONTENT_PRESERVATION_ABORT` MIG-category rows widened to explicitly cover the mechanism-A/`backfill-append-logs` trigger case (previously worded exclusively in B2/BC-INDEX per-row terms) — not deferred. **Stories affected by this BC (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.06 — add `BC-1.18.013` to `behavioral_contracts:` frontmatter array; propagate BC table, AC traces (superseding provisional AC-001–AC-008 with BC-1.18.013-anchored ACs), Token Budget, and Architecture Compliance Rule 7's OPEN-gate banner (now CLOSED, citing ADR-052 §Decision 9) in the SAME burst. **VP citations changed in: BC-1.18.013 (new).** Architect must propagate to `VP-INDEX.md`, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). |
