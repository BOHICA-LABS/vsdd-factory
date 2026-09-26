---
document_type: behavioral-contract
level: L3
version: "1.1"
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
  - .factory/specs/prd-supplements/error-taxonomy.md
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
input-hash: "7b17750"
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

6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution via the SAME
   two independent mechanisms BC-1.18.011 Precondition 6 specifies: (a) the advisory flock on
   `.factory/migration-state/exclusive.lock`; (b) the native admission gate in `executor.rs`
   (ADR-052 §Decision 5a), which blocks ALL mutation tool calls (Edit/Write/MultiEdit/Bash)
   targeting `.factory/cycles/` paths — not only `.factory/specs/behavioral-contracts/` — while a
   txn record exists in STAGING or COMMITTING state, regardless of PID liveness; (c) the
   OPEN/DRAINING gate with writer reservations. Per the same delivery cross-reference as
   BC-1.18.011 Precondition 6(b) (D-1236 Ruling 3): the Edit/Write/MultiEdit legs ship with
   cluster-5 F4 TDD in `executor.rs`; the Bash leg ships separately as part of [D-1232-OBL-4].

## Postconditions

1. **Per-file content-preservation, delegated entirely to BC-1.18.008.** For EACH of the four
   target files independently, content-preservation is BC-1.18.008 Postcondition 6(a)'s existing
   byte-for-byte record-boundary-safe partition check (concatenation of all resulting shards plus
   the final current file reproduces the original monolithic file byte-for-byte) — this BC
   introduces NO new per-file content-preservation mechanism; it is BC-1.18.011's structured
   per-BC-row equivalence check that has no analogue here, because mechanism A's partition unit
   (a time-ordered record sequence within ONE file) is not a cross-file ID space the way
   BC-INDEX's `BC-X.YY.NNN` rows are. This check is verified at ADR-052 §Decision 7c step 3b
   against each of the four staged generations, BEFORE the CURRENT.json pointer swap.

2. **Per-file independent census, delegated entirely to BC-1.18.008.** For EACH of the four
   target files independently, the independent census is BC-1.18.008 Postcondition 6(b)'s
   existing record-integrity check (every structural record present in the original file appears
   in EXACTLY ONE resulting shard). There is no cross-file ID oracle (unlike B2's `total_bcs`);
   each file's own record set is closed and self-contained. This check is likewise verified at
   ADR-052 §Decision 7c step 3b, per file, before the pointer swap.

3. **All-or-nothing across the FOUR INDEPENDENT FILES, not sub-partitions of one file.** Unlike
   BC-1.18.011 (whose atomic unit is ten-or-more shards of ONE logical body), this migration's
   atomic unit is four SEPARATE, independently-splittable files sharing ONE txn record, ONE
   generation, and ONE CURRENT.json pointer swap. If Postcondition 1 or 2 fails for ANY ONE of
   the four files, the ENTIRE migration aborts (step 3c): none of the four canonical files is
   touched, not merely the failing one. Each atomic file replacement (`rename(2)`) for each of
   the four files, once past the pointer swap, is followed by the platform-appropriate durability
   barrier (Linux: `fsync(file_fd)` + `fsync(parent_dir_fd)`; macOS/APFS: `fcntl(F_FULLFSYNC)` on
   the file, best-effort `fsync(dir_fd)`) per ADR-052 §Decision 7d, exactly as BC-1.18.011
   Postcondition 3 specifies for B2's shard files.

3a. **Pre-commit source-fingerprint recheck (TOCTOU guard), once over all four files.** Performed
    EXACTLY ONCE, immediately before the CURRENT.json pointer swap (ADR-052 §Decision 7c step 5,
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
   `completed.json`'s presence (`ALREADY_MIGRATED`, exit 0, per ADR-052 §Decision 4e/7c step 8) —
   this is checked BEFORE any lock is acquired and is authoritative for "has this governed
   migration already run." Independently, and unchanged, each of the four files' OWN
   `mechanism_a_backfill_already_migrated`/Backfill Recovery Manifest presence check
   (BC-1.18.008 Invariant 3) remains the per-file idempotency basis the migration binary consults
   WHILE performing each file's split — e.g., on resume-from-STAGING (EC-003 below), the binary
   MUST re-run BC-1.18.008's own per-file idempotency check for each of the four files rather
   than assuming none have a pre-existing manifest. These two layers do not conflict: `completed.json`
   answers "did the governed migration finish," BC-1.18.008's manifest answers "has THIS file's
   split already been performed" — the latter can, in principle, already be true for a file that
   was rolled by BC-1.18.006 before this migration ever ran (BC-1.18.008 Postcondition 3's
   Composability clause), and this migration's per-file invocation of
   `run_mechanism_a_backfill_split` handles that case exactly as BC-1.18.008 already specifies.

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
   architect closing out BC-7.08.001's own gate check MUST verify `completed.json` exists for
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
   `completed.json` is written, so the enrolled cap check does not race an in-flight backfill),
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
   record; framed intent log; single CURRENT.json pointer swap across all four files;
   `completed.json` as the permanent terminal record) that BC-1.18.008 alone — a single-file
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
   the single CURRENT.json atomic pointer swap (Precondition 5) and the intent log, exactly as
   BC-1.18.011 Invariant 3 specifies for the ten-subsystem case, generalized here to four
   independent files sharing one txn record instead of one file's ten-way internal partition.
   During the COMMITTING window, readers use the SAME open-with-ENOENT-fallback protocol
   (ADR-052 §Decision 7c step 2a) per file. `completed.json` is the permanent terminal record;
   forward recovery uses the intent log + matching-destination-hash rule to resume from the
   first uncompleted canonical path move among the four.

4. **This BC's completion DOES gate BC-7.08.001's Cohort-B fail-closed flip** (Postcondition 7)
   — the INVERSE of BC-1.18.011 Invariant 4, which states B2's migration explicitly does NOT
   gate that flip. An implementer or architect verifying BC-7.08.001's readiness MUST check
   `completed.json` for `backfill-append-logs`, not merely BC-1.18.008's code presence.

5. **Exactly the four ratified `v1.0-brownfield-backfill` files — never a wildcard, never
   auto-discovered.** No implementation may add a cycle-directory argument, a glob, or an
   auto-discovery mode to `backfill-append-logs`'s argument grammar without a superseding
   ADR-052 amendment (ADR-052 §Decision 3's closed-grammar guarantee; Precondition 3/
   Postcondition 6 above).

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Postcondition 1 (content-preservation) passes for three files but Postcondition 2 (independent census) fails for the fourth | Migration ABORTS for ALL FOUR files per Postcondition 4 — a per-file failure is a whole-migration failure; the three passing files' original content is also left untouched, not silently migrated ahead of the failing one |
| EC-002 | Migration crashes after completing canonical path moves for 2 of the 4 files (post-pointer-swap, mid-step-7) | ADR-052 §Decision 7c step 7's forward recovery resumes from the intent log's first uncompleted move (matching-destination-hash rule); the 2 already-moved files are NOT re-moved; the 2 remaining files complete on the recovery pass; no file is left half-migrated since each individual file's own `rename(2)` is atomic |
| EC-003 | A prior activation attempt built a fully-staged generation for all four files but crashed before the CURRENT.json pointer swap | Re-running MUST detect txn record state STAGING and resume toward the pointer swap; per ADR-052 §Decision 7c step 3b (referenced by §Decision 4e), the resume path MUST re-run the FULL per-file census (Postconditions 1/2) for all four files before proceeding — it does NOT skip re-verification merely because a staged generation was previously built, mirroring BC-1.18.011 EC-003 |
| EC-004 | `backfill-append-logs` is invoked after a prior activation already reached `completed.json` | `ALREADY_MIGRATED` (exit 0); no lock acquired; no manifest required; the binary performs zero filesystem mutation and exits immediately (Postcondition 5) |
| EC-005 | One of the four canonical files does not exist, or is unreadable, at activation time (S-25.06 story EC-004) | The migration ABORTS before acquiring the flock or building any staging generation (fails the pre-lock/under-exclusion validation, ADR-052 §Decision 4b/4c) with a clear non-zero exit; no partial state is written for any of the four files |
| EC-006 | A single record within one of the four files exceeds `shard_cap_bytes` on its own (BC-1.18.008 EC-002) | NOT a migration-level abort: BC-1.18.008's own oversized-record exception (`oversized_record: true`) applies to that file's shard exactly as BC-1.18.008 specifies; this governing BC's Postcondition 1/2 census-agreement gate tolerates the flagged exception and proceeds with the other files' and that file's remaining shards normally |
| EC-007 | `record_boundary_offsets` computed from the marker regex yields zero boundaries for one of the four files despite non-empty content (S-25.06 story EC-006; BC-1.18.008's O-1 note that this is unreachable in production for the four recognized artifact stems) | Hard failure via BC-1.18.008 Postcondition 6's fail-loud content-preservation gate for that file, surfaced by this migration as `CONTENT_PRESERVATION_ABORT` (exit 2) for the WHOLE migration per Postcondition 4 — never a silent empty-oracle partition |
| EC-008 | `backfill-append-logs` is invoked with a path argument, a `--cycle` flag, or any token outside the exact closed grammar (`backfill-append-logs` / `backfill-append-logs --census`) | REJECTED by the pre-shell classifier (ADR-052 §Decision 5c) before the binary is even invoked; if somehow bypassed, the binary itself rejects with a non-zero exit (ADR-052 §Decision 3) — this supersedes the S-25.06 story's provisional AC-001(b) cycle-directory-argument assumption (Precondition 3) |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| All four `v1.0-brownfield-backfill` files at their F4-measured oversized byte counts, no prior migration-state artifacts present | `backfill-append-logs` invoked once: all four files split per BC-1.18.008's algorithm; per-file content-preservation and census both pass; single CURRENT.json pointer swap commits all four; `completed.json` written; stdout census report lists all four files' pre/post record counts | happy-path |
| `backfill-append-logs` invoked a second time after the above completes | `ALREADY_MIGRATED` (exit 0); zero filesystem mutation; no lock acquired (EC-004) | happy-path |
| `lessons.md`'s content-preservation check finds a byte-count mismatch after staging; the other three files stage cleanly | Migration ABORTS for ALL FOUR files (`CONTENT_PRESERVATION_ABORT`, exit 2); `decision-log.md`, `burst-log.md`, and `session-checkpoints.md` remain in their ORIGINAL monolithic form despite having staged cleanly themselves (EC-001) | error |
| Migration crashes between the CURRENT.json pointer swap and the canonical rename of `session-checkpoints.md` (the last of the four in fixed order) | Re-run detects txn state COMMITTING; forward recovery completes the remaining rename via the intent log's matching-destination-hash rule; `decision-log.md`/`burst-log.md`/`lessons.md` (already moved) are untouched; `completed.json` written once all four are verified (EC-002) | error |
| `backfill-append-logs /some/other/cycle/decision-log.md` (a path argument) | Rejected before execution by the pre-shell classifier; non-zero exit; no filesystem mutation (EC-008) | error |
| `backfill-append-logs --census` against an already-completed migration | Read-only census report to stdout reproducing the original activation's per-file record counts and content hashes; no lock, no manifest, no mutation | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-143 | Four-file all-or-nothing atomicity invariant — a simulated crash at any staging/pivot/canonical-move step leaves ALL FOUR target files either fully original or fully split, never a state with some of the four migrated and others not | fault-injection / integration test (simulated crash at each step across all four files; assert post-recovery state is one of exactly two valid whole-migration states) |
| VP-144 | Per-file delegation invariant — this BC's content-preservation/census checks for each file are byte-identical in outcome to invoking BC-1.18.008's own Postcondition 6(a)/(b) checks directly against that file in isolation (no divergent or duplicated verification logic) | property test (differential test: governed-migration per-file check vs. direct BC-1.18.008 invocation on the same fixture, asserting identical PASS/FAIL and identical failure detail) |
| VP-145 | Closed-grammar rejection invariant — every invocation form outside the two accepted forms (`backfill-append-logs`, `backfill-append-logs --census`) is rejected before any filesystem mutation, by either the pre-shell classifier or the binary itself | integration test (fixture table of rejected forms: path arguments, extra flags, shell metacharacters, compound commands) |
| VP-143 | Idempotency two-layer consistency invariant — `completed.json` presence and BC-1.18.008's per-file manifest presence never disagree in a way that causes either a false `ALREADY_MIGRATED` before all four files are actually split, or a re-split of a file whose manifest already exists | integration test (resume-from-STAGING and roll-before-backfill fixtures per file, cross-checked against top-level `completed.json` state) |

VP IDs allocated by architect (S-25.06 Spec-First Gate closure, POLICY 9 propagation, 2026-09-25;
VP-INDEX v3.24): **VP-143** (integration; four-file all-or-nothing atomicity + idempotency
two-layer consistency — candidates 1 and 4 above consolidated into ONE VP per the
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
  boundary; single CURRENT.json pointer swap + `completed.json` terminal record (Postconditions
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
  consistency (`completed.json` vs. BC-1.18.008's per-file manifest), consolidated per the
  single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established.
- **VP-144** (proptest; allocated by architect, S-25.06 Spec-First Gate closure, 2026-09-25) —
  per-file delegation-correctness against BC-1.18.008 §Postcondition 6(a)/(b), a differential test
  reusing VP-123's existing fixture-generation strategy.
- **VP-145** (integration/safety; allocated by architect, S-25.06 Spec-First Gate closure,
  2026-09-25) — closed-grammar rejection invariant for `backfill-append-logs`; two-layer
  defense-in-depth (pre-shell classifier + binary argument parser); no direct sibling in
  BC-1.18.011's VP set.

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
| 1.1 | 2026-09-25 | architect | Spec-First Gate 2 closure for S-25.06 (POLICY 9 propagation): allocated VP-143 (integration; four-file all-or-nothing atomicity + idempotency two-layer consistency, consolidating candidates 1 and 4 of the Verification Properties table per the single-method-per-VP convention BC-1.18.011's VP-133/VP-124 established), VP-144 (proptest; per-file delegation-correctness differential test against BC-1.18.008 §PC6(a)/(b), candidate 2), and VP-145 (integration/safety; closed-grammar rejection invariant, candidate 3, no direct sibling in BC-1.18.011's VP set). Replaced the four `VP-NNN (pending)` placeholders in the Verification Properties table with these real IDs; updated VP Anchors accordingly. Full VP files authored at `.factory/specs/verification-properties/VP-143.md`/`VP-144.md`/`VP-145.md`. Propagated same-burst to `VP-INDEX.md` (v3.23→v3.24), `verification-architecture.md` (v1.36→v1.37), and `verification-coverage-matrix.md` (v1.34→v1.35) per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). No change to any Postcondition, Precondition, Invariant, Edge Case, or Canonical Test Vector — VP-citation-only amendment. input-hash recompute owed to state-manager (`compute-input-hash BC-1.18.013.md --update`). |
| 1.0 | 2026-09-25 | product-owner | Initial creation (NEW BC — closes S-25.06's Spec-First Gate, S-7.01). Allocated as BC-1.18.013, confirmed as the next free slot against the live `ss-01/` directory (BC-1.18.001–012 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism A's backfill-split of the four `v1.0-brownfield-backfill` append-log files, invoked via ADR-052's sanctioned execution path (`backfill-append-logs`, `Bash`-tool one-time interactive approval, closed argument grammar, armed-activation manifest, native admission gate, multi-file crash-atomicity across four independent files rather than one file's internal partition). Resolves the S-25.06 Architecture Compliance Rule 7 POL-3/native-CLI contradiction by direct reference to ADR-052 §Decision 9 (already ratified, D-1232, 2026-09-20) — NO new architecture decision was required; ADR-052 was authored with S-25.06 explicitly as an input and already generalizes its governed-migration state machine to mechanism-A migrations throughout (`migration_id: "backfill-append-logs"`, B2-only fields nulled for mechanism-A). Supersedes S-25.06's provisional AC-001(b) auto-discovery assumption with the ratified closed-grammar, fixed-four-file design (Precondition 3/Postcondition 6) — story-writer must update AC-001 accordingly. Adds Postcondition 8 specifying ShardRegistry enrollment (S-25.06 AC-005) as a SEPARATE ordinary Edit/Write step outside ADR-052's migration-binary write-target allowlist, using BC-1.18.005's existing `[[shard]]` schema, gated on a pre-existing `.factory/shard-config.toml` artifact-path-registry gap already identified by ADR-053 (routed to devops-engineer/architect, not blocking this BC's own dispatch-readiness). CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier per the established project convention (BC-1.18.011 v1.0 precedent). **Sibling BC updated in the same burst (Anchor-Back Rule):** BC-1.18.008 Related BCs gains a reciprocal reference to this BC (v1.9→v1.10, documentary-only, no semantic change). **error-taxonomy.md updated in the same burst:** `CENSUS_MISMATCH_ABORT` and `CONTENT_PRESERVATION_ABORT` MIG-category rows widened to explicitly cover the mechanism-A/`backfill-append-logs` trigger case (previously worded exclusively in B2/BC-INDEX per-row terms) — not deferred. **Stories affected by this BC (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.06 — add `BC-1.18.013` to `behavioral_contracts:` frontmatter array; propagate BC table, AC traces (superseding provisional AC-001–AC-008 with BC-1.18.013-anchored ACs), Token Budget, and Architecture Compliance Rule 7's OPEN-gate banner (now CLOSED, citing ADR-052 §Decision 9) in the SAME burst. **VP citations changed in: BC-1.18.013 (new).** Architect must propagate to `VP-INDEX.md`, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). |
