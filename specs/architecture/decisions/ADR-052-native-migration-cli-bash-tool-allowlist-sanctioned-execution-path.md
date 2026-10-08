---
document_type: architecture-decision-record
adr_id: ADR-052
level: L3
version: "1.26"
status: accepted
date: 2026-09-20
producer: architect
timestamp: 2026-09-20T00:00:00Z
phase: F1
supersedes: null
superseded_by: null
subsystems_affected:
  - SS-01
traces_to: .factory/specs/architecture/ARCH-INDEX.md
inputs:
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-adr052-v12-closure-2026-09-13.md
  - .factory/cycles/v1.0-brownfield-backfill/research-adr-052-v13-atomic-publication-2026-09-13.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-adr052-v11-closure-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-dir-cluster5-F1-direction-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-adr052-cluster5-F1-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/research-adr-052-assumption-validation-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md
  - CLAUDE.md
  - .claude/settings.json
  - crates/factory-dispatcher/src/main.rs
  - crates/factory-dispatcher/src/payload.rs
  - plugins/vsdd-factory/hooks/hooks.json.template
  - plugins/vsdd-factory/hooks-registry.toml
  - .factory/cycles/v1.0-brownfield-backfill/adv-local-adr052-pass3.md
  - .factory/research/adr-052-v123-architect-calls-validation.md
  - .factory/research/adr-052-intent-log-format-and-move-list-semantics.md
  - .factory/specs/architecture/decisions/ADR-054-governed-migration-intent-log-format-fixed-move-plan-and-crash-recovery.md
input-hash: "4637aa0"
# input-hash: run compute-input-hash --update at state-manager registration burst
---

# ADR-052: Native Migration CLI — Sanctioned Execution Path for Governed One-Time Shard Migrations

## Status

ACCEPTED — ratified by the human per POLICY 22 (D-1232, 2026-09-20), on the strength of the
D-1231 mechanical Kani proof: the v1.14 fix-burst's DEF-1 (HIGH) concurrency regression fix
(§Decision 5a Option-B structural drain reorder — the initial txn-record write moved to
step 3a, strictly AFTER the step-3 DRAINING flip) was re-verified with 7/7 VP proofs PROVED,
the INV-GATE-TXN invariant (`gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}`)
UNSAT under the corrected model, non-vacuity CONFIRMED, and 5/5 regression + 7/7
fault-injection tests PASS. This satisfies the §Verification Strategy re-verification
requirement stated below and closes the formal-finding exception opened by v1.14 against
the v1.13 prose-freeze clause.
**Outstanding items carried forward as post-ratification activation-boundary obligations
(NOT ratification blockers):** (1) the macOS exec-TOCTOU sub-instruction stat→execve
residual window and (2) the APFS darwin-arm64 directory-fsync durability test were the two
explicit human sign-off items at v1.13/v1.14; both are resolved as accepted residual risk
by this ratification. The two concrete pre-activation deliverables remain open and are
tracked as obligations against cluster-5 F4 activation: [D-1232-OBL-3] apply the §Decision 8
CLAUDE.md amendment text to the repo-root `CLAUDE.md` at the actual activation boundary;
[D-1232-OBL-4] devops-engineer implements the 4 dispatcher-guard amendments specified in
§Decision 5b/5c against this ADR's frozen spec text.
**Pipeline state:** the factory pipeline was PAUSED and cluster-5 TDD dispatch BLOCKED
pending this ratification; this status flip is the human gate that unblocks the on-ramp to
S-25.02 Phase F4 cluster-5 activation. §Decision 4e/5a/7c concurrency state machine remains
the frozen, now-ratified design; any further change to that state machine requires either a
new formal-finding exception (per the pattern established by v1.14/DEF-1) or a superseding
ADR — not prose-only amendment.
**v1.18 formal-finding exception (S-25.06 formal verification, HEAD 9886cbc1 — defects D1/D2/D3
plus architect-found D4/D5):** the S-25.06 formal-verifier found three production defects in the
mechanism-A (`backfill-append-logs`) realization of this ratified design — D1 the admission gate
never invoked from the dispatcher's real PreToolUse path, D2 no writer-reservation drain before
STAGING, D3 a permanent self-lock when a crash lands after `completed.json` is durable but before
the txn record reaches COMPLETED. Adjudicating them against merged B2 code exposed two further
spec-level defects in THIS ADR's frozen text: D4 (the migration-state namespace is shared by both
migrations but the terminal record `completed.json` and pointer `CURRENT.json` were specified once,
so either migration's terminal record falsely signals the OTHER migration complete) and D5 (the
§5a atomic-admission protocol took `flock` on the gate-state file while the gate flip replaces that
file by `rename`, which does not serialize against an admitter that opened the replaced inode).
Each is closed by v1.18 (§Decision 4e rows, §5a Branch C / reserve-then-verify / coordinator
exemption / release-on-block, §5c Branch 2, new §Decision 7e). This is a formal-finding exception
under the v1.14/DEF-1 pattern, NOT a prose-only amendment: the §Verification Strategy
re-verification obligation is re-opened for the delta (re-run the B2 `obl1` Kani suite and the
VP-146 harnesses a1/a3/a4/a6 as respecified, plus the VP-143 facet-(b) fault-injection and D1/D2
black-box tests) before S-25.06 may merge (post-split ownership: the shared admission core, the B2
`obl1` suite re-baseline [VP-147] and the VP-143 D1/D5 black-box tests are delivered by S-25.08; the
VP-146 harnesses and the VP-143 D2/D3/D4 facets by S-25.06). The POLICY 22 ratification of the v1.15 status flip is
not reopened (no §Decision 1/2/3 mechanism change), but the orchestrator MUST surface this
exception to the human for acknowledgement per the v1.14 precedent.
**v1.20 rulings (S-25.08 local adversary pass 1 — spec-level gaps F-001/F-002/F-003/F-004/F-006/
F-008/F-009 in the shared admission core):** normative architectural rulings only; no §Decision
1/2/3/4 mechanism change and no change to the OPEN/DRAINING/LOCKED state machine, the Dekker
reserve-then-verify argument, or any ratified Kani-proved transition. They add: release on
`PostToolUseFailure` (§5a "Release events"), the admission scope anchor and Edit/Write/MultiEdit
path resolution (§5a "Admission scope anchoring" / "Target path resolution"), the
registry-independent evaluation position (§5a "Evaluation position"), reservation timestamp
rules (§5a "Reservation timing parameters"), the shared-core definition of "foreign" (§7e), and
named error variants (§Error Code Semantics). The §Verification Strategy re-verification
obligation is extended only for the pure-core input rename and the new black-box facets listed in
the §5a "v1.20 test mandate"; the human-acknowledgement requirement of the v1.18 exception is
unchanged.
**v1.24 human-authorized amendment (2026-10-08; CLAUDE.md rule 12; research
`.factory/research/adr-052-intent-log-format-and-move-list-semantics.md`):** the human authorized two
substantial amendments of §Decision 7a/7b/7c, applied here and in the NEW companion **ADR-054**
("Governed Migration On-Disk Formats and Crash Recovery"). (1) §Decision 7b (intent-log framing) is MOVED to
ADR-054 Decision 1 and rewritten as the HARDENED format (fixed key order, ABNF value grammar, SHA-256 over
the exact record bytes, byte-level reader with torn-tail-absent / mid-log-corruption-fail-closed, tail
truncation under the flock, `F_FULLFSYNC` + directory sync, one shared module for both migrations). (2) The
move list is the FIXED, immutable plan renamed `pending_canonical_moves` → `canonical_move_plan`; intent-log
`DONE` records are the completion record; §7c step 7 sub-step d is deleted; bugs B-1 (`TreatDone` must append
`DONE`), B-2 (`DONE` bound to the txn and compared with the INTENT's `expected_post_hash`; truthful
`expected_pre_state`) and B-3 (a COMMITTING txn needs a non-empty plan equal to the INTENT target set) are
specified normatively in ADR-054 Decision 3. This closes both v1.23 open items (the intent-log byte
encoding "human gate" and the `pending_canonical_moves` semantics "open item for the human"). All of it is
delivered by the NEW story S-25.10 (stacked on S-25.09; blocks S-25.06 and any `migrate-bc-index` release);
S-25.09 keeps only the `txn_id` / `intent_log_path` / `DONE` `txn_id`+`fencing_generation` plumbing. No
§Decision 1/2/3/4 mechanism change, no change to the OPEN/DRAINING/LOCKED state machine or any ratified
Kani-proved transition; the §Verification Strategy re-verification obligation is extended only for the
recovery pure core (`decide_recovery`, VP-146 a1 / VP-147 h1) and the new intent-log fault-injection facets.
Hereafter ADR-054 is the normative home of the intent-log format, the move-plan/completion model and the
recovery table; this ADR keeps the stable §7b heading as a pointer.
**v1.25 (S-25.09 local adversary pass 1; architect rulings, no state-machine, taxonomy-code
or VP-count change):** item 11(c) clarification (the under-lock `completed.json` read has four verdicts —
dangling symlink / non-`NotFound` stat or read error / non-record content are `Io` exit 2, never absent and never
exit 0; presence-only semantics kept for branch selection; what `recover()` receives; reader parity; closes
F-S2509-L1-004) and new item 11(f) (the BC-1.18.013 Postcondition 10 "Coordinators" clause is an open set; advisory
line shape and closed advisory-token domain; exit-0 silence amended for advisories; canonical-move halt reasons
belong to S-25.11 (story AC-008 = ADR-054 AC-015; ADR-054 v1.1 widens the reason domain to all six halt sites);
closes F-S2509-L1-006), with implementation rulings: advisory `<subject>`/`<cause>` capped 256 PER SLOT, fixed
clause never truncated, no whole-line cap; new item 11(g) (64-char cap is InternalLog-event-only, operator stderr
is 256, data-derived slots carried raw and sanitized at render time; coordinator-only advisory emission rules). Both rulings are recorded under the v1.25 changelog row, not under v1.24.
**v1.26 (S-25.09 local adversary pass 2; architect rulings, no state-machine, taxonomy-code, exit-code or VP-count
change; v1.25 is committed at factory-artifacts 81b44098 and immutable, so these are a NEW version):** (1) the
`OVERSIZED_ROW_SUBSHARD` advisory is collected as DATA by the pure chunker's caller and printed only when the run
that chunked returns `Completed` (its fixed clause "and the migration completed" is then true at emission); the
`pub` chunker stays pure and emits nothing; a resumed run that did not chunk prints none (item 11(f)(2), 11(g)(3);
closes F-S2509-L2-002/003). (2) The fresh-run `abort_staging` closure reports a failed inert generation-directory
removal as `STAGING_DIR_REMOVE_FAILED`, exactly as `discard_incomplete_staging` does (item 11(d), 11(f)(2); closes
F-S2509-L2-006). (3) `CURRENT.json` gets the same lstat-first classification as `completed.json` in BOTH of its
consumers, the reader `detect_migration_read_state` and the coordinator's resume probe, through one shared
`read_current_pointer_marker` helper (item 11(c) point 6; closes F-S2509-L2-009). (4) Confirmation made explicit:
`BcIndexMigrationError::Io` `path` AND `source` are both data-derived and subject to the item 11(g)(2) per-substring
256 rule, including an `InvalidData` source and `CleanupWriteFailure`'s two substrings; a serde error is never
carried as its message (item 11(g)(2); closes F-S2509-L2-001). Both CURRENT.json and Display rulings are recorded
under the v1.26 changelog row.

## Context

Two governed one-time shard migrations are specified in S-25.02:

1. **Mechanism-A append-log backfill-split** (BC-1.18.008) — wired and executed by S-25.06.
   `run_mechanism_a_backfill_split` exists in `shard_manager.rs` with zero production callers.
   S-25.06 adds the CLI entry point and executes the migration against the four live cycle
   append-log files.

2. **Mechanism-B2 BC-INDEX body-split migration** (BC-1.18.011) — cluster-5 of S-25.02.
   The `run_mechanism_b2_bc_index_split` function (to be authored in cluster-5 TDD) is the
   native migration entry point in `shard_manager.rs`.

**Core context:** Both migrations share a structural challenge requiring
a narrowly-authorized native binary exception to POL-3 / TD-FACTORY-HOOK-BYPASS-001 P0. The
constraints (native binary needed; hook-validated Edit/Write path insufficient for multi-MB
files), the Option A/B/C evaluation, and the selection of Option B (one-time interactive Bash
approval) are unchanged. This ADR now additionally adopts a crash-safe atomic publication model
recommended by research to resolve the 3rd Codex review findings.

**v1.3 root-cause correction:** The 3rd Codex review correctly identified that publishing a
multi-file migration by renaming each target file in place through an ordered sequence cannot
provide all-or-nothing multi-file visibility regardless of journaling. Readers observing any
intermediate state see a partially-committed generation. v1.3 replaces this model with a single
atomic pointer swap over an immutable staging generation, a framed checksummed intent log for
per-target recovery, an advisory flock on a stable never-unlinked inode, and a real drain
(OPEN/DRAINING gate with writer reservations spanning PreToolUse→tool-completion).

**Correction from v1.2:** `.claude/settings.json` contains ONLY `enabledPlugins`; no existing
Bash permission entries. **Option A viability** is documented in §Rationale (confirmed viable;
not chosen on operational complexity grounds — see "Head-to-head mechanism evaluation" §Option A).
The **native admission gate** design (OPEN/DRAINING/LOCKED state machine with durable txn record
and writer reservations) is specified in §Decision 5a.

## Finding → Resolution (v1.3 + v1.4)

All 11 findings from the 3rd Codex cross-vendor closure review (`adv-cv-adr052-v12-closure-2026-09-13.md`) are resolved by the v1.3 redesign. Nine additional findings from the 4th adversarial review (local cascade pass-1, D-1221) are resolved by this v1.4 fix-burst:

| Finding | Sev | Root cause in v1.2 | Resolution in v1.3 |
|---------|-----|-------------------|-------------------|
| F1 | HIGH | Per-file rename: reader between renames sees mixed generation; "COMMITTED absent → read legacy" heuristic is ambiguous after crash | §Decision 7c: single atomic CURRENT.json pointer swap; reader resolves pointer once, pins one generation; completed.json as permanent terminal record eliminates ambiguous-absence heuristic |
| F2 | HIGH | Crash after rename but before `completed_renames` append: staging gone, no record, retry cannot recover | §Decision 7b: framed checksummed intent log records per-target expected post-content hash + expected pre-state BEFORE first rename; recovery accepts matching destination hash as completion; fail closed on ambiguity |
| F3 | HIGH | "Drain protocol" only detects writes already completed; admits writers before lock; nothing waits for in-flight writer to finish | §Decision 5a: real OPEN/DRAINING gate; writer reservations span PreToolUse→tool-completion; coordinator waits for active_writer_count=0 before snapshotting; fingerprint recheck kept as defense-in-depth |
| F4 | HIGH | O_CREAT\|O_EXCL + unlink-reclaim: two reclaimers both classify inode as stale, one unlinks the other's live inode; empty/corrupt lock classified as "dead → unlink" | §Decision 7a: advisory flock(LOCK_EX) on stable pre-created never-unlinked inode; stale reclamation automatic on process death; fail closed on empty/corrupt metadata by acquire-first (never unlink) |
| F5 | HIGH | No exclusive takeover path for dead-PID STAGING transaction; BC-1.18.011 and error-taxonomy.md block writers only for alive PID, contradicting "block for any STAGING" | §Decision 7a: durable txn record separate from flock; ordinary writers blocked by txn record state (STAGING or COMMITTING) regardless of PID liveness; recovery-owner bumps fencing generation to claim ownership; releasing flock never deletes txn record |
| F6 | HIGH | Expiry recheck placed after all renames (§4d in v1.2); expiry can delete PREPARED after canonical content already changed; no authorized path for new activation to recover old transaction | §Decision 4d: authorization gate moved to immediately before CURRENT.json pointer swap (single pivot); post-pivot COMMITTING state retained regardless of expiry; completion-only recovery manifest bound to old activation_id + staged-generation hash + monotonic fencing generation |
| F7 | HIGH | Admission gate conditions on "target path under protected directory" for Bash, but Bash commands yield no intrinsic target paths; unknown write effects undefined | §Decision 5c: conservative Bash admission block: while txn record exists in STAGING/COMMITTING state, block all Bash commands with unknown write effects; classify only sanctioned migration commands from config-driven target sets; canonicalization and alias rejection specified |
| F8 | MED | Guard requires unexpired manifest for --census (no manifest needed) and COMMITTED reruns (terminal no-op needs no manifest); both paths blocked after manifest archival | §Decision 5c: four separate guard branches: read-only census, validated terminal-state no-op, new activation, and recovery; manifest requirement and flock acquisition apply only to new-activation and recovery branches |
| F9 | HIGH | After CLEANED archives COMMITTED, absent COMMITTED is indistinguishable between "never ran" and "completed long ago"; reader and rerun logic cannot determine steady state | §Decision 7c: completed.json written at stable path after all target moves; permanent, never deleted, never archived; supersedes CURRENT.json for reader protocol; closes steady-state ambiguity |
| F10 | HIGH | Allowed-paths list in §Decision 8 omits sub-shards (.a.md/.b.md), BC-INDEX.shard-manifest.toml, BC-INDEX-SS-05.manifest.toml; CLAUDE.md amendment uses "shards/" directory which is broader scope | §Decision 8: single authoritative allowlist enumerating all required migration output paths including sub-shards and manifest files; CLAUDE.md amendment generated from that exact same list with containment checks |
| F11 | HIGH | Guard hashes binary at path, then shell executes same path by name; concurrent cargo build can replace binary between hash check and exec; TOCTOU window implicit | §Decision 11: Linux: open fd, hash through fd, execveat(fd, "", argv, envp, AT_EMPTY_PATH) — eliminates TOCTOU; macOS: fexecve unavailable per Apple documentation; decision: freeze build under maintenance lock + document residual window; requires human sign-off (see §Decision 11) |

**v1.4 findings (4th adversarial review — this fix-burst):**

| Finding | Sev | Root cause in v1.3 | Resolution in v1.4 |
|---------|-----|-------------------|-------------------|
| C1 | CRITICAL | Reader no-content window: step 6 flips CURRENT.json to `committing` before step 7 renames files out of `gen-<uuid>/`; reader protocol says use `gen-<uuid>/` during committing, but those files are being renamed away → ENOENT; also violates "IMMUTABLE" invariant claim | §Decision 7c: reader protocol changed to canonical-first / generation-dir-fallback during `committing`; each file is always accessible at canonical path (if already moved) or `gen-<uuid>/` path (not yet moved) due to rename atomicity; "IMMUTABLE" clarified to CONTENT-IMMUTABLE — content never modified, but files may be moved to canonical paths during step 7 |
| C2 | CRITICAL | F10 allowlist still not closed: sub-shard names use hyphen-uppercase (`-A.md`, `-B.md`) but canonical ADR-051 + BC-1.18.010/011 use dot-lowercase (`.a.md`, `.b.md`); SS-05 needs `.c` shard (661 BCs over cap) — allowlist stops at `-B`; only `BC-INDEX-SS-05.manifest.toml` hardcoded — `BC-INDEX-SS-06.manifest.toml` rejected; `validate_write_target()` would reject all canonical sub-shard names | §Decision 8: allowlist normalized to dot-lowercase `BC-INDEX-SS-<NN>.a.md` / `.b.md` / `.c.md` (any single lowercase letter a-z); sub-manifest generic `BC-INDEX-SS-<NN>.manifest.toml` (covers all subsystems); CLAUDE.md amendment regenerated from corrected list; ratification-time test mandate: every BC-1.18.010/011 path passes `validate_write_target()` |
| H1 | HIGH | Abort leaves gate stuck LOCKED: drain-timeout (§5a step 3), expiry abort (§7c step 4), fingerprint abort (§7c step 5), census abort (§7c step 3b) all release flock but never reset gate → OPEN; crash-recovery with no live txn also leaves stale LOCKED/DRAINING forever | §Decision 5a: every abort/rollback path atomically flips gate → OPEN before returning; crash-recovery: if recovery process acquires flock and finds gate=LOCKED/DRAINING with no active txn (absent, COMPLETED, or ABORTED), MUST reconcile gate → OPEN; fault-injection test mandate: "abort during DRAINING/LOCKED restores OPEN" |
| H2 | HIGH | Drain quiescence unsound across processes: §5a persists only gate_state, not active_writer_count; dispatcher is per-event binary (not persistent daemon), so in-process mutex/condvar cannot observe cross-process reservations; no atomic check-gate-and-increment | §Decision 5a: dispatcher per-event process model explicitly stated; active_writer_count replaced by durable reservation directory `.factory/migration-state/reservations/`; PreToolUse admission is atomic (read gate_state file under lock, create reservation file, release); coordinator polls reservation dir for quiescence; test: writer admitted just before DRAINING → coordinator waits |
| H3 | HIGH | §7c sequence dropped BC-1.18.011 PC1 content-preservation + PC2 independent census pre-pivot gate; no step verifies partition correctness before the irreversible pointer swap; E-SHD-005 not checked before pivot; resume-from-STAGING (§4e) does not re-run census (EC-003) | §Decision 7c: new step 3b (pre-pivot content-preservation + census gate) inserted between intent-log WAL (step 3) and authorization gate (step 4); verifies content-preservation (PC1), independent census (PC2), and E-SHD-005 shard boundary invariants against staged generation; on failure: ABORT cleanly (delete gen, txn → ABORTED, gate → OPEN); §4e resume-from-STAGING: RE-RUN full census (EC-003) before proceeding |
| H4 | HIGH | §5c conservative Bash admission and Finding→Resolution rows F5, F7 key on txn state `PREPARED` which does not exist in §7a enum; §7a defines `STAGING\|COMMITTING\|COMPLETED\|ABORTED`; §5c Bash classifier never fires during STAGING | §Decision 5c: all `PREPARED` occurrences replaced with `STAGING`; F5/F7 rows in Finding→Resolution table updated; §7a enum is authoritative: no PREPARED state exists in this ADR |
| M1 | MED | Fencing token never enforced: §7a recovery-owner writes carry `fencing_generation` "to prove authority" but no resource rejects a stale token; only the completion manifest checks it; enforcement language implies a distributed-system fence that is not implemented | §Decision 7a: `fencing_generation` downgraded to AUDIT-ONLY metadata; advisory flock provides actual mutual exclusion (only one process can hold LOCK_EX at a time); `fencing_generation` is a monotonic traceability counter for crash-restart audit trail; "prove authority" language removed |
| M2 | MED | Pivot step-number contradiction: §4d cites "§7c step 4" as the pointer swap / single pivot; §7c labels step 4 = authorization gate and step 6 = commit/pointer swap; BC-1.18.011 PC3a additionally cites "step 5" (all three citations disagree) | §Decision 4d: corrected to reference "§7c step 6" as the CURRENT.json pointer swap (commit point); authorization gate = step 4, fingerprint recheck = step 5, pointer swap = step 6; BC-1.18.011 PC3a cites step 5 → must be corrected to step 6 (see BC impact handoff) |
| M5 | MED | macOS risk framing: §Decision 11 / Consequences say TOCTOU is "eliminated on the primary development platform (Linux)" but the primary operator platform IS macOS/darwin-arm64; "no concurrent cargo build" is a soft recommendation, not a programmatic guard | §Decision 11: macOS/darwin-arm64 identified as primary operator platform carrying the documented residual TOCTOU; Linux is where TOCTOU is eliminated via fd-binding; "no concurrent cargo build" elevated to hard pre-flight checklist item with programmatic mtime guard; /proc dependency for Linux fexecve fallback noted; §Consequences updated accordingly |

**v1.5 findings (5th adversarial review — this fix-burst):**

| Finding | Sev | Root cause in v1.4 | Resolution in v1.5 |
|---------|-----|-------------------|-------------------|
| C-1 | CRITICAL | Reader protocol regression: canonical-first/generation-fallback is correct for net-new shard files but WRONG for BC-INDEX.md (in-place overwrite target); canonical path holds OLD monolithic body throughout COMMITTING window until step 7's final rename, so canonical-first returns stale content for BC-INDEX.md before its rename; violates BC-1.18.010/011 Invariant 3 | §Decision 7c: reader protocol inverted to generation-first/canonical-fallback — try gen-\<uuid\>/\<file\> FIRST (present ⟹ not-yet-moved ⟹ new content); on absence fall back to canonical (absent from gen ⟹ already moved ⟹ new content); correct for BOTH net-new shard files AND in-place-overwrite targets; ENOENT impossible for any file during COMMITTING window; fault-injection reader test mandate added |
| H-1 | HIGH | ADR not self-contained: §Decision 6 says "(See v1.2 §Decision 6 — no change)"; §Decision 8 says "See v1.2 §Decision 8 skipped-control inventory table"; §Downstream says Amendments 1–3 "UNCHANGED from v1.2. Apply verbatim as specified in v1.2 §Downstream"; v1.2 no longer exists in the file — ratifier/PO cannot read load-bearing content | §Decision 6: full audit-trail decision inlined; §Decision 8: full skipped-control inventory table inlined; §Downstream: Amendments 1–3 fully inlined; no "see v1.2 §" dangling references remain |
| H-2 | HIGH | macOS mtime guard inert on successful exec: §Decision 11 records mtime "immediately after the digest check" and re-checks "after exec returns" — a successful execve never returns (it replaces the calling process); guard fires only on exec failure; successful substitution+exec (the dangerous case) is never caught | §Decision 11: mtime re-stat moved to IMMEDIATELY BEFORE the execve call; window narrowed from (digest-check → exec) to (pre-exec-re-stat → execve-syscall); corrected residual-risk text states guard detects substitution before the pre-exec re-stat, not after; "Human sign-off required" updated accordingly |
| H-3 | HIGH | Cross-process reservation UUID uncorrelated: PreToolUse generates a fresh random UUID for the reservation filename; PostToolUse (separate process) cannot recompute a random value from a prior process → reservation leaks → quiescence never achieved → spurious DRAIN_TIMEOUT_ABORT | §Decision 5a: reservation filename changed to `<tool_use_id>.reservation` using the stable harness tool-invocation identifier shared across the Pre/Post hook pair for the same tool call; test mandates added: Pre creates, Post (separate process, same tool_use_id) removes; stale-PID cleanup reclaims dead creator's reservation |
| H-4 | HIGH | Load-bearing ADR version pins in body narrative: CLAUDE.md amendment text says "ADR-052 v1.4 EXCEPTION"; dangling "see v1.2 §" references (addressed by H-1); version-history form belongs in changelog only | CLAUDE.md amendment text updated to "ADR-052 EXCEPTION" (stable, version-agnostic); all "see v1.2 §" dangling refs inlined (H-1); BC-impact handoff directs PO to use stable §Decision N form in BC-1.18.010/011 + error-taxonomy |
| M-2 | MED | ADR-051 §Decision 10 still says SS sub-sharding happens "at the SAME F4 activation moment mechanism A's own backfill (BC-1.18.008) runs," contradicting BC-1.18.011 Precondition 4 (decoupled, any order) and ADR-052 §Decision 1 | ADR-051 §Decision 10 item 6 amended to remove activation-moment coupling; B2 sub-split runs within same one-time B2 operation independently of mechanism A's activation schedule; ADR-051 bumped to v1.14 |
| M-3 | MED | ADR uses bare `E-MAINTENANCE` throughout; taxonomy canonical code is `E-MAINTENANCE-001` (error-taxonomy.md is SoT for codes) | `E-MAINTENANCE` replaced with `E-MAINTENANCE-001` throughout the ADR body, §5a drain language, §7a ordinary-writer blocking, and §Error Code Semantics table |
| M-4 | MED | §Downstream BC-1.18.010 §Reader Integration carries stale "use gen-\<uuid\>/ paths for reads" v1.3 instruction alongside the v1.4 canonical-first correction; two conflicting instructions present; neither is the correct generation-first protocol | §Downstream BC-1.18.010 §Reader Integration: stale v1.3 text deleted; v1.4 canonical-first text replaced; single correct generation-first/canonical-fallback instruction retained, consistent with §Decision 7c C-1 fix |
| M-6 | MED | §5a admits Bash mutations but never states when a Bash invocation creates/holds a durable reservation or whether quiescence waits on it; underspecified for the coordinator | §Decision 5a: explicit text added: every admitted Bash with write effect creates a `<tool_use_id>.reservation` file; §5c classifier determines write effect; quiescence waits for all Bash reservations; test mandate added |

**v1.6 findings (6th adversarial review — local cascade pass-3, D-1223 — this fix-burst):**

| Finding | Sev | Root cause in v1.5 | Resolution in v1.6 |
|---------|-----|-------------------|-------------------|
| C-1 | CRITICAL | Step 3b census gate is a tautology: step 3 sets `expected_post_hash = sha256(staging_file)`; step 3b PC1 checks `sha256(staged_file) == expected_post_hash` = sha256(x)==sha256(x) — always true, inert as a correctness gate. Does NOT perform BC-1.18.011 PC1 (byte-for-byte concat against `source_sha256`) nor PC2 (per-ID exactly-one-shard set membership). EC-001 (byte-identical concat with compensating dup+drop, same total count) passes undetected. | §Decision 7c step 3b: existing sha256(staged)==expected_post_hash check renamed "staging-file integrity" (what it actually is — guards against silent modification after staging sync); new PC1 added (reconstruct concatenation of staged shard files + retained lean body → compare SHA-256 against `source_sha256` from txn record); PC2 rewritten as per-ID set check (fresh enumeration of ID set from original-census; each ID must appear in EXACTLY ONE staged shard, ZERO in retained body; count comparison is necessary but not sufficient — EC-001 MUST abort). |
| H-1 | HIGH | §4e recovery table keyed on "CURRENT.json status:staging" — impossible discriminator. CURRENT.json does not exist during STAGING (first written at step 6, the COMMITTING pivot). A STAGING crash (txn=STAGING, CURRENT.json absent, manifest present) matches no row correctly. | §Decision 4e: table re-keyed on TXN-RECORD state (STAGING / COMMITTING / COMPLETED / ABORTED) as the primary discriminator, per BC-1.18.011 Invariant 3. The impossible "CURRENT.json status:staging" row is deleted. |
| H-2 | HIGH | Crash between step 8 (completed.json write) and gate→OPEN flip: gate=LOCKED, completed.json present, no live owner. Branch 2 ALREADY_MIGRATED path acquires no lock and performs no gate reconciliation; gate stays permanently LOCKED; all `.factory/specs/behavioral-contracts/` and `.factory/cycles/` mutations blocked forever. | §Decision 5c Branch 2: before exiting 0, MUST reconcile stale gate: acquire gate-state LOCK_EX; if gate ≠ OPEN and completed.json present with no active txn (txn absent, COMPLETED, or ABORTED): flip gate → OPEN; release gate lock; then exit 0 with `ALREADY_MIGRATED`. Fault-injection test mandate added. |
| H-3 | HIGH | Source/Origin and References cite `adv-cv-adr052-v13-closure-2026-09-13.md` which does NOT exist. No source for 4th–6th reviews cited accurately. Status finding-count (v1.5: "7 MEDIUM") mismatches the 4 mediums in the v1.5 Finding→Resolution table. | Source/Origin: replaced non-existent file citation with real provenance: local cascade D-1221 (pass-1), D-1222 (pass-2), D-1223 (pass-3); adv-local-adr052-pass3.md (written this burst by state-manager). Status finding-count corrected to 1C+4H+5M+2L. References cleaned of non-existent file. inputs[] + adv-local-adr052-pass3.md. |
| H-4 | HIGH | §Downstream Amendments 7/8/9 contained deferred-deferral placeholder text (the architect deferred providing the exact replacement text to a future dispatch). Product-owner cannot apply a placeholder; the text was a no-op instruction that blocked BC application. | §Downstream Amendments 7/8/9: placeholder removed; exact replacement text inlined, mirroring BC-1.18.011 v1.5 (CURRENT.json pointer swap as commit-point; intent log + matching-hash recovery; completed.json permanent terminal record; generation-first/canonical-fallback reader protocol; ADR-052 §Decision 7a/7b/7c citations). |
| M-1 | MED | Admission discriminator ambiguous across four sites: §5a PreToolUse describes reading only gate_state; §7a, BC-1.18.011 PC6(b), and error-taxonomy say block is driven by TXN-RECORD state regardless of PID; sites are inconsistent. | §5a: explicit text added: PreToolUse checks BOTH gate_state AND txn-record state; gate_state is the durable proxy that persists the txn-state block between per-event binary invocations; admit iff gate_state=OPEN AND no active txn (STAGING or COMMITTING); either condition alone triggers E-MAINTENANCE-001. |
| M-2 | MED | macOS `verify_and_exec_binary` Rust code sample omits the pre-execve mtime re-stat that §Decision 11 rationale item 4 mandates; prose-code gap from H-2(v1.4) closure. | §Decision 11 macOS code sample: mtime re-stat and comparison added immediately before `exec_by_pathname` call; code sample now matches the prose specification exactly. |
| M-3 | MED | Census/content abort error identifier triple-coded: step 3c uses "applicable error code"; §Error Code Semantics has CENSUS_MISMATCH_ABORT and CONTENT_PRESERVATION_ABORT as separate codes; trigger definitions reference the old (tautological) PC1 definition. Binary surfaces process exit codes, not HookResult. | Step 3c, §Error Code Semantics, and step 3b body aligned: CONTENT_PRESERVATION_ABORT = process exit code for PC1 (byte-for-byte concat SHA-256 mismatch against source_sha256) failures; CENSUS_MISMATCH_ABORT = process exit code for PC2 (ID-set exactly-one-shard violation) and E-SHD-005 (shard boundary) failures; binary is a process, not a hook; triggers updated to match corrected definitions. |
| M-4 | MED | Mechanism-A CLAUDE.md amendment scope uses `.factory/cycles/*/` wildcard, making POLICY-22 human-auditable scope non-literal; four exact config-driven target paths are known. | CLAUDE.md amendment text: `.factory/cycles/*/` wildcard replaced with the four exact append-log paths: `.factory/cycles/v1.0-brownfield-backfill/decision-log.md`, `.factory/cycles/v1.0-brownfield-backfill/burst-log.md`, `.factory/cycles/v1.0-brownfield-backfill/lessons.md`, `.factory/cycles/v1.0-brownfield-backfill/session-checkpoints.md`. |
| M-5 | MED | Stale-reservation cleanup runs only at recovery startup; a crashed agent's reservation causes spurious DRAIN_TIMEOUT_ABORT on a first-run migration (not a recovery). | §5a drain procedure: stale-PID reservation GC promoted to step 1 of EVERY drain (before DRAINING flip), not only recovery startup. |
| L-1 | LOW | H1 title contains "(v1.4 Fix-Burst)" version pin — stale; version lives in frontmatter only. | H1 title: "(v1.4 Fix-Burst)" stripped. |
| L-2 | LOW | "completed.json" and "completed.json" used interchangeably in prose and code; canonical path is lowercase `completed.json` per §Decision 7c step 8 which writes `.factory/migration-state/completed.json`. | All path-bearing occurrences aligned to lowercase `completed.json`. |

**v1.7 findings (7th adversarial review — local cascade pass-4, D-1224 — this fix-burst):**

| Finding | Sev | Root cause in v1.6 | Resolution in v1.7 |
|---------|-----|-------------------|-------------------|
| F1 | HIGH | PC1 gate unsatisfiable: step 3b PC1 computed SHA-256 of (staged shards + staged lean body) and compared against `source_sha256` (SHA-256 of the original pre-split BC-INDEX.md body). This can NEVER equal `source_sha256` because (a) the staged lean body adds the new §Subsystem Shard Manifest section (extra bytes not present in the original), and (b) content is reordered. The ADR over-corrected the v1.5 tautology by replacing it with a whole-concat hash that is mathematically unsatisfiable. EC-001 was unreachable. | §Decision 5a drain step 5(c): added `source_body_row_sha256` capture — SHA-256 of the per-BC-row table-row content from the original BC-INDEX.md body in canonical BC-ID sort order, excluding §Summary, §Subsystem Shard Manifest, cross-cutting invariants, and non-row lines. Stored in txn record. §Decision 7a txn record: `source_body_row_sha256` field added. §Decision 7c step 3b PC1: rewritten as structured-equivalence — extract BC-X.YY.NNN table rows from all staged shards, sort by BC-ID, hash → compare against `source_body_row_sha256`; `source_sha256` scoped explicitly to step 5 fingerprint recheck only. EC-001 remains reachable: PC1 passes on a compensating dup+drop (same sorted row bytes), PC2 independently catches the ID-set violation. |
| F2 | HIGH | ADR error surface triple-inconsistent: (a) step 3b "Shard boundary invariants (E-SHD-005)" attributed a HookResult code to the migration binary; (b) step 3c CENSUS_MISMATCH_ABORT exit code listed "OR E-SHD-005 shard boundary violation" mixing HookResult with process exit code; (c) §Error Code Semantics CENSUS_MISMATCH_ABORT trigger also referenced E-SHD-005. E-SHD-005 is a `HookResult` emitted by the steady-state native admission gate (BC-1.18.006/BC-1.18.010), not a process exit code of the migration binary. | §Decision 7c step 3b: "Shard boundary invariants" bullet renamed "Shard boundary invariants (internal capacity check)" with explicit note that this is NOT E-SHD-005. §Decision 7c step 3c: CENSUS_MISMATCH_ABORT trigger updated to say "OR shard-boundary capacity violation (internal check; NOT E-SHD-005 which is a HookResult of the steady-state gate)". §Error Code Semantics CENSUS_MISMATCH_ABORT row: same correction. BC impact handoff (§BC Impact v1.7) directs PO to: scope E-SHD-005 to steady-state gate only; update CENSUS_MISMATCH_ABORT in error-taxonomy.md; update BC-1.18.011 PC2/EC-001 trigger language to cite process exit codes not HookResult. |
| F4 | MED | Completion-crash reconciliation only via binary re-invocation: after a crash between step 8 (completed.json written) and the gate→OPEN flip, gate=LOCKED with completed.json present and no live txn. Branch 2 (ALREADY_MIGRATED) reconciles the gate — but only when the migration binary is re-invoked. Until re-invocation, ALL writes to `.factory/specs/behavioral-contracts/` and `.factory/cycles/` are permanently blocked. No operator runbook existed for this scenario. | §Decision 5a PreToolUse admission: stale-locked-gate reconciliation added between steps 3 and 4. If gate_state=LOCKED AND completed.json present AND no active txn: (1) release LOCK_SH; (2) acquire LOCK_EX; (3) re-read gate_state under LOCK_EX; (4) if still LOCKED and no active txn and completed.json present: write gate_state=OPEN; (5) release LOCK_EX; (6) re-acquire LOCK_SH and re-check admission (gate now OPEN → admit). This allows ordinary PreToolUse to self-heal the stuck gate without requiring binary re-invocation. Fault-injection test mandate expanded. |
| F5 | MED | Residual dangling reference: §Context contained "Dispatcher constraint, Option A viability, and native admission gate context are unchanged from v1.2 §Context." v1.2 §Context no longer exists in-file (v1.5 H-1 inlined all "see v1.2 §" content), making this a dangling dead reference that a ratifier/implementer cannot resolve. | §Context: removed "are unchanged from v1.2 §Context" clause; replaced with explicit forward references to extant in-file content: §Rationale (Option A viability) and §Decision 5a (native admission gate design). |
| F8 | MED | APFS dir-fsync durability test was a post-ratification deliverable: §Decision 7d stated "A darwin-arm64 empirical durability test is required before this ADR is considered fully validated on macOS" but did not gate ratification. The POLICY 22 sign-off block in §Decision 11 did not mention the APFS test. The ADR could be ratified without completing the durability validation, leaving macOS power-loss behavior unverified on the PRIMARY OPERATOR PLATFORM. | §Decision 7d: elevated darwin-arm64 empirical durability test to RATIFICATION PREREQUISITE. §Decision 11 POLICY 22 sign-off block: added F8 as a second explicit sign-off item alongside exec-TOCTOU; human ratifying this ADR MUST either (a) confirm the test is complete, or (b) explicitly acknowledge unverified APFS dir-fsync durability. Status block updated accordingly. |
| F9 | OBS | Stale "step 5" v1.3-handoff row in BC-Impact table: the v1.3 BC-Impact table row for Postcondition 3a (Amendment 7) still said "step 5 in §Decision 7c" as the pointer swap location. The v1.4 additional changes table had ALREADY corrected this to step 6, but the v1.3 row was never updated, creating an inconsistency within the handoff table itself. | §BC Impact v1.3 table: Postcondition 3a "change required" column updated inline to say "step 6 in §Decision 7c" (superseding the stale v1.3 "step 5" text). |
| F10 | OBS | PID-reuse hazard in stale-reservation GC: §5a drain step 1 described stale-PID detection as "files whose creating PID is no longer alive (file content = PID)." On a long-running system, the OS may reuse a crashed process's PID for an unrelated live process, causing the GC to falsely classify a stale reservation as live — leading to spurious DRAIN_TIMEOUT_ABORT on the next drain. | §Decision 5a drain step 1 + stale-reservation cleanup text: reservation files updated to store `(pid, process_start_time)` tuple. GC checks BOTH pid AND start_time against live process; a PID match with start_time mismatch indicates PID reuse → reclaim the stale reservation. Fallback to PID-only check with warning when start_time is unavailable. |

**v1.8 findings (8th adversarial review — local cascade pass-5 — RATIFY-WITH-CHANGES):**

| Finding | Sev | Root cause in v1.7 | Resolution in v1.8 |
|---------|-----|-------------------|-------------------|
| F-2 | HIGH | §Files to Change: the `shard_manager.rs` row still described PC1 as "step 3b pre-pivot census gate — staging-integrity + PC1 (byte-for-byte reconstruct vs source_sha256)" — the v1.6 whole-concat form that v1.7 F1 superseded. The `tests/` row still said "PC1 concat-SHA vs source_sha256" and listed `E-SHD-005` inside the migration census-gate test description. `E-SHD-005` is a `HookResult` emitted by the steady-state native admission gate (BC-1.18.006/BC-1.18.010); it is NOT a process exit code of the migration binary and does not belong in the migration census-gate test inventory. Both rows also carried stale "v1.6" version labels. | §Files to Change `shard_manager.rs` row: rewrites PC1 phrase to "PC1 structured per-BC-row equivalence vs `source_body_row_sha256`"; removes `source_sha256` from the PC1 bullet (it remains only in the step-5 fingerprint recheck); updates version scope label from v1.6 to v1.7. `tests/` row: replaces "PC1 concat-SHA vs source_sha256" with "PC1 structured per-BC-row equivalence vs `source_body_row_sha256`"; removes `E-SHD-005` from the census-gate test listing (the test verifies the shard-boundary internal capacity check, NOT the steady-state HookResult); updates version scope label from v1.6 to v1.7. |
| F-4 | MED | §Decision 4e recovery-modes table was inexhaustive: the `txn STAGING + manifest absent or expired` case had no row. This scenario occurs when (a) the activation manifest was never written, (b) the manifest was deleted, or (c) the manifest has expired before the resume attempt. Without a defined row, the coordinator has no specified action for this valid operational state. | §Decision 4e: added row `txn STAGING + manifest absent or expired` → clean-abort: delete staging generation, update txn record → ABORTED, flip gate → OPEN, exit `EXPIRY_ABORT`. This applies the §4d clean-abort logic (expired-pre-pivot branch) to the case where the manifest is absent or expired at resume time rather than expiring mid-run. The table is now exhaustive for all re-run and recovery TXN-RECORD states; first-run activation (no txn record + valid manifest present) is handled by §Decision 5c Branch 3 and is intentionally absent from this table (see v1.9 L2 scope note added to §Decision 4e). |

**v1.10 findings (10th adversarial review — local cascade pass-7, D-1227 — this fix-burst):**

| Finding | Sev | Root cause in v1.9 | Resolution in v1.10 |
|---------|-----|-------------------|-------------------|
| HIGH-1 | HIGH | §Decision 5a step 3.5 self-heal predicate: triggers only when gate=LOCKED AND no active txn AND completed.json present. Two symmetric crash windows leave the gate permanently stuck: (i) an abort path (census §7c step 3b / fingerprint §7c step 5 / expiry §7c step 4) crashes AFTER writing txn→ABORTED but BEFORE the gate→OPEN flip — durable state gate=LOCKED, txn=ABORTED, NO completed.json — step 3.5 skips (requires completed.json); (ii) drain-timeout abort fires with gate=DRAINING and NO txn record (txn first written at drain step 7, after LOCKED) — step 3.5 required gate=LOCKED so DRAINING was never considered. Writers blocked permanently in both cases. | §Decision 5a step 3.5: predicate generalised to "gate ∈ {LOCKED, DRAINING} AND no active txn (absent, COMPLETED, or ABORTED) — regardless of completed.json presence." Step title updated. Sub-item (d) lists all four covered scenarios. "Note" updated to reflect ∈ {LOCKED, DRAINING} phrasing. Two fault-injection tests added: "abort-crash before gate→OPEN (txn=ABORTED, no completed.json) → next PreToolUse reconciles gate→OPEN" and "drain-timeout crash (gate=DRAINING, no txn) → next PreToolUse reconciles gate→OPEN." PO handoff note added for E-MAINTENANCE-001 "always TEMPORARY" reconciliation. §Files-to-Change executor.rs row updated. |
| MED-1 | MED | §Error Code Semantics EXPIRY_ABORT trigger described solely as "manifest has expired … AND txn=STAGING" but §Decision 4e (v1.8 F-4) routes both expired AND absent manifest at STAGING resume to EXPIRY_ABORT; trigger is narrower than the actual behavior. | §Error Code Semantics EXPIRY_ABORT trigger widened to "manifest has expired OR is absent at STAGING resume (pre-pivot; no canonical paths changed)"; PO handoff note added to row pointing to error-taxonomy.md EXPIRY_ABORT row requiring the same widening. |
| MED-2 | MED | TTL-based reservation GC (v1.9 H1) retains a crashed-harness-session's abandoned reservation for up to MAX_RESERVATION_TTL (3,600 s); every activation attempt aborts with DRAIN_TIMEOUT_ABORT until the TTL elapses; no operator remediation documented. | §Decision 5a Stale reservation cleanup: added "Operator remediation for abandoned-reservation stall" paragraph documenting how to identify a confirmed-stale reservation (tool_use_id not owned by any live session), that SAFE MANUAL DELETION is SANCTIONED pre-activation, that the TTL lower-bound is a heuristic constraint (not structurally enforced), that activation SHOULD be scheduled during quiescence, and that MAX_RESERVATION_TTL MAY be lowered toward the lower bound to shorten stall duration. §Files-to-Change executor.rs row updated. |
| LOW-1 | LOW | §Error Code Semantics section header "emitted by the migration binary" included E-MAINTENANCE-001 which is emitted by the native admission gate in executor.rs (dispatcher hook path), not the migration binary. | §Error Code Semantics restructured into two sub-tables: "Migration binary process exit codes" and "Guard-layer admission code (executor.rs / native admission gate)"; E-MAINTENANCE-001 moved to the guard-layer sub-table with a header note distinguishing HookResult from migration-binary process exit codes. |
| LOW-2 | LOW | §BC Impact provenance tables still used imperative future-directive phrasing ("v1.3 change required", "Replace with:", "v1.4 change required", "Add catalog rows…", "Each row must include…", "Codes to add:") despite all changes having been applied in prior bursts (BC-1.18.011 v1.7, BC-1.18.010 v1.8, error-taxonomy v1.25). | BC-1.18.011 main table column header changed from "v1.3 change required" to "Applied (BC-1.18.011 v1.7)"; v1.4 additional changes table column header changed to "Applied (BC-1.18.011 v1.7)"; v1.4 additional changes to error-taxonomy.md prose block converted from imperative ("Add catalog rows…") to past-tense provenance ("Applied: catalog rows added for…"). |

**v1.9 findings (9th adversarial review — local cascade pass-6, D-1226 — this fix-burst):**

| Finding | Sev | Root cause in v1.8 | Resolution in v1.9 |
|---------|-----|-------------------|-------------------|
| H1 | HIGH | Drain step 1 GC keys on `(pid, process_start_time)` tuple stored in reservation files. The factory-dispatcher is a per-event binary: the PreToolUse invocation that created the reservation has already exited by the time any drain runs. Its PID is dead. A liveness check therefore classifies EVERY in-flight reservation (Pre fired, tool running, Post NOT yet fired) as stale and reclaims it. Step 4 quiescence poll then sees an empty directory and reports quiescence, causing the coordinator to snapshot source files MID-WRITE. This directly contradicts the fault-injection test mandate ("writer admitted just before DRAINING flip: coordinator waits for its PostToolUse"). The (pid, start_time) tuple (v1.7 F10) mitigates PID-reuse false-negatives for persistent daemons; it does NOT rescue the per-event model where the creating process is dead by design. | §Decision 5a drain step 1: PID-liveness GC removed entirely. Replaced with TTL-only cleanup: reservation files store ONLY a `created_at` ISO-8601 timestamp (no PID, no start_time); step 1 removes files older than MAX_RESERVATION_TTL (default: 3,600 seconds). MAX_RESERVATION_TTL lower bound: MUST NOT be set below 1,800 seconds — must be longer than the worst-case legitimate tool-call duration. Normal flow: PostToolUse removes the reservation by tool_use_id before any drain sees it. Crash/restart flow: the reservation ages out within MAX_RESERVATION_TTL; the next drain proceeds normally. Fault-injection test mandate: new test "in-flight reservation (Pre fired, Post NOT yet) is NEVER reclaimed by drain step-1 TTL GC; the coordinator does NOT snapshot source MID-WRITE." §Decision 5a "Stale reservation cleanup" summary paragraph updated to match. |
| M2 | MED | §Downstream to Product-Owner (Amendments 1–9) and §BC Impact "what must change" tables and "product-owner must apply" subsections still used imperative future-directive framing ("product-owner MUST apply", "Replace X with:", "required amendments") for BC and taxonomy changes that were all applied in prior bursts (BC-1.18.011 v1.7, BC-1.18.010 v1.8, error-taxonomy v1.25: casing sweep, E-SHD-005 rescope, §Decision 7 machinery disclosure, write_indeterminate_marker removal). | §Downstream to Product-Owner: headings changed from "required amendments" to "amendments applied"; Amendment preambles changed from imperative ("Replace X with:") to past-tense ("Applied: X replaced with:"); "must apply" headings converted to "Applied:" form with artifact version. §BC Impact: headings changed from "what must change to match v1.N" to "changes applied to match v1.N"; "product-owner MUST" phrases converted to "Applied:"; v1.5/v1.7 additional changes section headings updated. §Error Code Semantics: "product-owner MUST add catalog rows" → "The product-owner added catalog rows." Genuinely-pending items (crates/ implementation, 4 devops dispatcher-guard amendments, darwin-arm64 durability test) left future-tense — these remain in §Files to Change, §Decision 5b, and §Decision 7d/11. |
| L1 | LOW | §Source/Origin cited adversarial passes only through "local pass-3 (D-1223) … producing this v1.6 fix-burst" — stale self-reference in a v1.9 document. §References contained a duplicate "this v1.6 fix-burst" self-reference. Pass-4 (D-1224, producing v1.7), pass-5 (D-1225, producing v1.8), and pass-6 (this burst, producing v1.9) provenance rows were absent from both sections. | §Source/Origin: appended pass-4 (D-1224 / v1.7), pass-5 (D-1225 / v1.8), pass-6 (D-1226 / v1.9) provenance rows; corrected "this v1.6 fix-burst" to past-tense "the v1.6 fix-burst." §References: same self-reference corrected; pass-4/5/6 decision log entries added. |
| L2 | LOW | §Decision 4e and the v1.8 F-4 Finding→Resolution row claimed "The table is now exhaustive for all TXN-RECORD states." The no-txn + manifest-PRESENT first-run case (new activation, nothing previously in flight) is handled by §Decision 5c Branch 3, not by §4e. The §4e table correctly covers re-run and recovery modes but overstated its scope. | §Decision 4e: added a note below the table: "Scope note: this table covers re-run and recovery behavior only. First-run activation (no txn record + valid manifest present, no prior migration attempted) is handled by §Decision 5c Branch 3 and intentionally has no row in this table." v1.8 F-4 Finding→Resolution row updated: "The table is now exhaustive for all TXN-RECORD states" → "The table is now exhaustive for all re-run and recovery TXN-RECORD states; first-run activation (no txn record + valid manifest present) is handled by §Decision 5c Branch 3 and is intentionally absent from this table." |

**v1.11 findings (11th adversarial review — local cascade pass-8, D-1228 — this fix-burst):**

| Finding | Sev | Root cause in v1.10 | Resolution in v1.11 |
|---------|-----|-------------------|-------------------|
| HIGH-1 | HIGH | §Decision 5a step 3.5 self-heal checks ONLY gate_state + txn — no flock check on `exclusive.lock`. But gate ∈ {DRAINING,LOCKED} + no txn is also the normal state of a LIVE coordinator mid-drain: drain step 3 writes gate=DRAINING and releases LOCK_EX on gate-state, while the txn record is not written until drain step 7; the coordinator holds `exclusive.lock` (LOCK_EX, acquired at drain step 2) throughout. A concurrent ordinary-writer PreToolUse running step 3.5 sees gate=DRAINING + no txn → matches reconciliation predicate → acquires LOCK_EX on gate-state → flips gate→OPEN → admits itself — breaking BC-1.18.011 Precondition 6 writer-exclusion while the coordinator snapshots/stages. This is a REGRESSION introduced by the v1.10 generalization (which removed the `completed.json` requirement without adding the flock check that the "Crash-recovery gate reconciliation" paragraph already uses). | §Decision 5a step 3.5: added `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` attempt as new sub-step a, the FIRST action when the predicate matches. On EWOULDBLOCK (live coordinator holds the lock): release LOCK_SH; return E-MAINTENANCE-001 (block); do NOT reconcile. On success (flock acquired, no live coordinator): proceed with reconciliation sub-steps b–h (existing logic relabeled). Sub-steps relabeled a→h throughout. Note updated with two new non-reconciliation cases. Parity note added: same "acquire-flock → re-read-under-lock → flip → release" discipline as the existing "Crash-recovery gate reconciliation" paragraph. Defense-in-depth: §Decision 5a drain procedure reordered — new step 2.5 writes initial txn record (state=STAGING, source_sha256=null, source_body_row_sha256=null, generation_id=null) BEFORE the DRAINING flip at step 3; ensures gate=DRAINING in a live drain always implies txn=STAGING (step 3.5 active-txn check blocks reconciliation); step 7 updated to "update txn record with source hashes" (completes fields left null in step 2.5); drain timeout abort (step 4) now also updates txn→ABORTED before gate→OPEN. Fault-injection test mandate added: "live coordinator mid-drain (flock held, gate=DRAINING, no txn); concurrent PreToolUse MUST receive EWOULDBLOCK → block with E-MAINTENANCE-001 → MUST NOT flip gate→OPEN." §Files-to-Change executor.rs row swept to cite flock-gated self-heal mechanism. |
| MED-1 | MED | §Decision 7c reader protocol step 2a uses existence-check-then-use: "Try `gen-<id>/` path FIRST. If the file EXISTS at its generation-dir path, use it." This is TOCTOU-racy: a rename gen→canonical between exists() and open() yields ENOENT to the reader. The v1.5 C-1 fix correctly inverted the protocol from canonical-first to generation-first but did not change the implementation from existence-check to open-based. | §Decision 7c reader protocol: step 2a changed to `open(gen-<generation_id>/<file>)`; on ENOENT fall back to `open(canonical)`. Because files move one-directionally (gen→canonical), open-with-ENOENT-fallback is race-free: if open(gen) returns ENOENT, the file has already been renamed to canonical; it cannot disappear from both paths simultaneously (rename(2) is atomic). "ENOENT is impossible" rationale updated to reference the open-with-fallback implementation. Fault-injection reader test mandate language updated. **BC-impact handoff for PO (applied in v1.11 burst):** BC-1.18.010 §Reader Integration step 2 AND BC-1.18.011 Invariant 3 must mirror the open-with-ENOENT-fallback wording (remove "If the file EXISTS" existence-check language; replace with the open-with-ENOENT-fallback implementation description). |
| MED-2 | MED | §BC Impact "BC-1.18.010 — changes applied to match v1.3–v1.5" table still had header "v1.3 change required" and imperative cells ("Replace entire section with…", "Replace: completed.json…", "Update to:…"). The "v1.4 additional changes to BC-1.18.010" table had header "v1.4 change required" and imperative cell ("v1.5 M-4: replace BOTH stale instructions with…"). These were NOT swept when the BC-1.18.011 + error-taxonomy tables were converted to past-tense in v1.10 LOW-2. | §BC Impact BC-1.18.010 main table: column header "v1.3 change required" → "Applied (BC-1.18.010 v1.8)"; all three imperative cells converted to past-tense provenance form. §BC Impact BC-1.18.010 v1.4 additional changes table: column header "v1.4 change required" → "Applied (BC-1.18.010 v1.8)"; imperative cell converted to past-tense. Sibling sweep of v1.10 LOW-2 complete. |
| LOW-1 | LOW | §Decision 9 heading carried "(unchanged from v1.2)", §Decision 10 heading carried "(unchanged from v1.2)", §Decision 1 body carried "(unchanged from v1.2)" at end of Options sentence, §5b body opened with "The same 4 guards as v1.2 require amendment" — four sites the v1.8 strip missed (v1.8 stripped these from bold-section labels and Decision 1/2/3 headings but not these locations). The referenced content is inlined in-file so the self-references are dead. | Strip "(unchanged from v1.2)" from §Decision 9 heading and §Decision 10 heading; strip "(unchanged from v1.2)" from §Decision 1 body; replace "The same 4 guards as v1.2 require amendment" with "The following 4 guards require amendment" in §5b. Legitimate historical explanatory mentions in §Rationale and changelog left untouched. |

**v1.13 findings (13th adversarial review — accept-at-floor fix-burst, D-1230):**

| Finding | Sev | Root cause in v1.12 | Resolution in v1.13 |
|---------|-----|-------------------|-------------------|
| HIGH-1 | HIGH | After v1.11 step 2.5, a mid-drain coordinator crash leaves gate∈{DRAINING,LOCKED} + txn=STAGING (ACTIVE, generation_id=null). §5a step 3.5's predicate "gate∈{LOCKED,DRAINING} AND no active txn" does NOT match (txn=STAGING is active) → step 3.5 never runs the flock check → ordinary writers block on E-MAINTENANCE-001 with no self-heal → requires manual binary re-invocation. Also the v1.10 HIGH-1 fault-injection test mandate said "txn is first written at drain step 7, after LOCKED" — flatly contradicted by step 2.5. | §Decision 5a step 3.5: predicate extended to "gate∈{LOCKED,DRAINING} AND (no active txn (absent, COMPLETED, or ABORTED) OR txn=STAGING AND generation_id=null)". New sub-step e null-generation branch: attempt flock(exclusive.lock, LOCK_EX\|LOCK_NB); if EWOULDBLOCK → block E-MAINTENANCE-001 (live coordinator, step 2.5 written but step 1 not yet reached); if acquired → set txn→ABORTED (null_generation disposition marker; retained per GC policy), flip gate→OPEN, admit. v1.10 HIGH-1 fault-injection test mandate rewritten to reflect step-2.5 reality. New v1.13 HIGH-1 fault-injection test added. Applied same-burst note updated: self-heal now covers no-active-txn AND active-STAGING-with-null-generation-dead-coordinator. |
| MED-1 | MED | §Error Code Semantics EXPIRY_ABORT trigger enumerated only two arms ("manifest has expired OR is absent at STAGING resume") but §4e F2 DISCARD also exits EXPIRY_ABORT for a pre-generation crash sub-state despite a VALID manifest. Trigger is narrower than actual behavior. | §Error Code Semantics EXPIRY_ABORT trigger: widened to add the third arm "OR a pre-generation crash sub-state (txn=STAGING, generation_id=null) is discarded despite a valid manifest (v1.12 F2)". PO handoff note added: widen error-taxonomy.md EXPIRY_ABORT trigger with the same third arm. |
| MED-2 | MED | Three residual "unchanged from v1.2" dangling annotations: (a) §Downstream BC-1.18.010 "Invariant 2 amendment: Applied per v1.2 text (unchanged from v1.2)" — actual three-way parity text not inlined; (b) §Decision 10 body "CI parity test … unchanged from v1.2" — annotation on an in-file decision; (c) §5c negative-tests preamble "unchanged from v1.2 with addition" — annotation on in-file content. | (a) §Downstream BC-1.18.010 Invariant-2 amendment: inlined actual three-way parity text (config.arch_index_sha == manifest.approved_arch_index_sha == live ARCH-INDEX SHA per §Decision 10); "unchanged from v1.2" annotation removed. (b) §Decision 10 body: "unchanged from v1.2" annotation stripped. (c) §5c negative-tests preamble: "unchanged from v1.2 with addition" annotation stripped. |
| MED-3 | MED | §4e F2 null-generation row said "DISCARD: delete the partial null txn record" (hard delete) contradicting the F4 GC policy ("ABORTED records MUST NOT be deleted before archival — preserve crash-debug audit trail for any failed activation attempt"). The "manifest absent or expired" row said the ambiguous "discard partial txn record → ABORTED" without resolving delete-vs-retain. step 3.5 HIGH-1 fix (this burst) would have applied the same delete logic PreToolUse-side, propagating the inconsistency. | §4e F2 null-generation row: changed "delete the partial null txn record" to "set txn state → ABORTED (null_generation disposition marker; retained per GC policy until archival)". §4e manifest-absent/expired row: changed "discard partial txn record → ABORTED" to "set txn state → ABORTED (retained per GC policy)". §5a step 3.5 new null-generation branch: uses same set→ABORTED+null_generation disposition (not delete). GC policy is now internally consistent: all failed activation attempts (including null-generation crashes) produce retained ABORTED records. |
| LOW-1 | LOW | §Decision 7c step 1 did not state that generation_id is persisted to the txn record ONLY AFTER the generation directory is durably created (fsync). The entire §4e generation_id-presence partition (v1.12 F2) is sound only under this ordering. §4e had no defined handling for "generation_id set + gen dir absent/incomplete" (which must not occur under the invariant — treat as corruption). | §Decision 7c step 1: added ordering invariant "generation_id MUST be persisted to the txn record ONLY AFTER the generation directory is durably created and the fsync barrier is applied." Added corruption case: if recovery finds generation_id set but gen dir absent/incomplete → treat as filesystem corruption → fail-closed abort (set txn→ABORTED, flip gate→OPEN, exit EXPIRY_ABORT with diagnostic log entry). |
| LOW-2 | LOW | ADR-052 changelog and §Source/Origin cite "ADR-051 v1.14" but ADR-051 frontmatter carries no `version:` field (body-changelog versioning convention only). The version reference is not verifiable against frontmatter. | Clarifying note added in §References where ADR-051 is cited: ADR-051's version is its body-changelog convention, not a frontmatter field. Follow-up action noted for adding `version:` frontmatter to ADR-051. Do NOT edit ADR-051 here. |

**v1.12 findings (12th adversarial review — local cascade pass-9, D-1229 — this fix-burst):**

| Finding | Sev | Root cause in v1.11 | Resolution in v1.12 |
|---------|-----|-------------------|-------------------|
| F2 | HIGH | §4e null-STAGING crash sub-state unhandled: v1.11 drain step 2.5 writes txn=STAGING with `generation_id=null`, `source_sha256=null`, `source_body_row_sha256=null` BEFORE snapshot (drain step 5) and generation UUID assignment (§7c step 1). A crash between step 2.5 and §7c step 1 leaves txn=STAGING with NO staged generation directory and null hashes. §4e's single "txn STAGING + valid manifest" row routes to EC-003 (resume: RE-RUN full census against staged generation, step 3b must pass before proceeding) — but there is no generation directory to census, and PC1 at step 3b would attempt to compare staged rows against `source_body_row_sha256=null` from the txn record, yielding an unconditional CONTENT_PRESERVATION_ABORT on every resume attempt. The recovery path is structurally wrong for this sub-state. | §Decision 4e: split the "txn STAGING + valid original manifest" row into two sub-rows keyed on `generation_id` presence: (a) `txn=STAGING AND generation_id=null` (pre-snapshot / pre-generation crash): DISCARD — delete the partial null txn record, flip gate → OPEN, exit `EXPIRY_ABORT`. No canonical paths changed; state fully recoverable under a fresh activation. MUST NOT attempt EC-003 census; MUST NOT compare PC1 against null `source_body_row_sha256`. (b) `txn=STAGING AND generation_id set AND staged generation dir present`: the existing EC-003 resume path — RE-RUN full census (step 3b), proceed to authorization gate (step 4), fingerprint recheck (step 5), pointer swap (step 6). Exhaustiveness scope note updated to reflect the `generation_id` partition. Fault-injection test mandate added to §Decision 5a. |
| F3 | MED | Residual/inverted reader wording in §Downstream + §BC Impact: (a) §Downstream BC-1.18.010 §Reader Integration blockquote step 2 says "try `gen-<generation_id>/` path FIRST; if absent (already renamed to canonical), fall back to canonical path" — existence-check language ("if absent"), not the race-free open-with-ENOENT-fallback form required by §Decision 7c step 2a. (b) §BC Impact v1.4 additional changes Invariant-3 "Applied (BC-1.18.011 v1.7)" cell says "new-generation content is accessible via canonical-first / generation-fallback protocol; ENOENT is not possible" — the INVERTED direction (canonical-first = v1.4 known regression, corrected in v1.5 C-1) described in an Applied column, directly contradicting §Decision 7c and BC-1.18.011 Invariant 3. (c) "BC-1.18.010 v1.8" provenance labels in §Downstream header and §BC Impact table headers are stale: BC-1.18.010 was updated to v1.9 when the v1.11 MED-1 open-with-ENOENT-fallback handoff was applied. (d) §BC Impact v1.5 C-1 entry still uses "if absent" existence-check language. | §Downstream BC-1.18.010 §Reader Integration blockquote step 2: changed from "try gen FIRST; if absent … fall back to canonical" to "`open(gen-<generation_id>/<file>)`; on ENOENT, `open(canonical/<file>)`." §BC Impact v1.4 additional changes Invariant-3 Applied cell: corrected from "canonical-first / generation-fallback; ENOENT is not possible" to "generation-first / open-with-ENOENT-fallback: `open(gen-<uuid>/<file>)`; on ENOENT → `open(canonical/<file>)`. ENOENT-safe: files move one-directionally gen→canonical via rename(2); required file never absent from both paths during COMMITTING window." §BC Impact v1.5 C-1 applied descriptions: updated from "if absent, fall back" to "open-with-ENOENT-fallback" language. §Downstream header and §BC Impact table headers: "BC-1.18.010 v1.8" → "BC-1.18.010 v1.9". |
| F4 | MED | §4e ABORTED disposition absent + txn-selection ambiguity + terminal-txn GC unspecified: (a) Every abort writes txn=ABORTED and retains the file indefinitely; a fresh re-activation writes a new `txn-<newuuid>.json`; §4e has no row for the case where an ABORTED txn record exists alongside a new activation attempt, leaving re-activation behavior undefined. (b) Multiple `txn-*.json` files can coexist (one per prior activation attempt); §4e has no disambiguation rule for which record governs admission/recovery. (c) Terminal txn records (ABORTED/COMPLETED) accumulate in `.factory/migration-state/` with no documented GC or archival policy — unbounded file growth. | §Decision 4e: (a) Added explicit ABORTED disposition row: "`txn=ABORTED` (any terminal ABORTED record)" → "treated as no active txn; fresh activation permitted; no recovery action required; record retained until GC archival per policy below." (b) Added txn-selection disambiguation rule below the table: when multiple `txn-*.json` exist, select the record whose `activation_id` matches the CURRENT manifest's `activation_id`; if no current manifest: select highest `created_at`; ABORTED records not matching the current manifest are inert for recovery. (c) Added GC/archival policy: terminal records (ABORTED/COMPLETED) archived to `.factory/migration-audit/txn-archive/` by state-manager at the next successful migration completion audit burst; ABORTED records MUST NOT be deleted before archival. |
| F6 | LOW | §5c Branch 2 gate reconciliation acquires gate LOCK_EX without first verifying `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` acquirability. §Decision 5a step 3.5 and the "Crash-recovery gate reconciliation" paragraph both require the flock-acquirable precondition before gate manipulation; Branch 2 silently diverges, creating an inconsistent reconciliation protocol. Benign today (Branch 2 triggers only when `completed.json` present — terminal state — so no live coordinator can legitimately hold `exclusive.lock`), but a parity gap that could admit a writer if Branch-2 trigger conditions are ever broadened. | §Decision 5c Branch 2: gate reconciliation now first attempts `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` as a new pre-step (step 0) before acquiring gate LOCK_EX. On EWOULDBLOCK: log a warning ("unexpected: completed.json present but exclusive.lock held"); exit 0 with `ALREADY_MIGRATED` WITHOUT reconciling gate (benign: `completed.json` terminal state is authoritative; gate self-heals at next PreToolUse step 3.5). On acquired: proceed with gate reconciliation steps 1–4 (existing logic); release `exclusive.lock` after step 4. Rationale documented inline: parity with step 3.5 and crash-recovery paragraph; safe against future Branch-2 trigger broadening. Fault-injection test mandate updated. Note: all literal `\|` inside this section's markdown table cells are escaped. |

**v1.14 findings (Kani model-checking pass — this fix-burst; DEF-1; human-directed spec/design
convergence per D-386 Option C, prior to accept-at-floor ratification — pipeline remains PAUSED,
cluster-5 TDD remains BLOCKED):**

| Finding | Sev | Root cause in v1.13 | Resolution in v1.14 |
|---------|-----|-------------------|-------------------|
| DEF-1 | HIGH | Uncovered self-heal window, fix-induced regression from v1.11 step 2.5: v1.11 introduced a "defense-in-depth" reordering that wrote the initial txn record (state=STAGING, `generation_id=null`) at a "step 2.5" positioned BEFORE the step-3 DRAINING flip, on the stated rationale that this was redundant with the flock check (step 3.5 sub-step a) — v1.11's own text called it "defense-in-depth," not the primary guard. This reordering opened a NEW crash sub-window that did not exist before v1.11: a coordinator crash strictly between step 2.5 (txn written, gate STILL OPEN) and step 3 (DRAINING flip) left durable state `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead coordinator`. §5a step 3.5's self-heal predicate requires `gate_state ∈ {LOCKED, DRAINING}` by design (gate=OPEN is defined to mean "no maintenance in progress"), so it never matched this state — no self-heal fires. §5a step 5's ordinary admission check blocks any writer whenever an active STAGING/COMMITTING txn exists, REGARDLESS of gate_state, so the writer is blocked. §4e's `txn=STAGING AND generation_id=null` DISCARD row exists but is reachable only via migration-binary re-invocation (not an ordinary PreToolUse self-heal), directly contradicting this ADR's own v1.10/v1.13 guarantee that E-MAINTENANCE-001 always self-heals by the next PreToolUse. Found by Kani model-checking (bounded crash-interleaving check over the drain procedure) during the cluster-5-pre-implementation formal convergence pass authorized by human direction; symmetric in kind to the pass-8/9/13 findings (each an ADR self-heal branch introducing a new regression) but landing in the previously-unexamined `gate=OPEN` window rather than the `gate ∈ {LOCKED, DRAINING}` window those passes covered. | §Decision 5a drain procedure: REORDERED (structural fix, not a new branch) — the initial txn-record write moves from "step 2.5" (before the DRAINING flip) to new step **3a**, strictly AFTER step 3 (the DRAINING flip) completes. Because the coordinator is a single sequential process holding `exclusive.lock` continuously from step 2 through completion/abort, this ordering establishes the invariant `gate_state = OPEN ⟹ no txn record exists with state STAGING or COMMITTING` — `gate=OPEN` co-existing with an active txn becomes structurally unreachable, not merely covered by an extra predicate branch. The flock-gated step-3.5 sub-step a check (v1.11 HIGH-1) remains the sole load-bearing writer-exclusion mechanism during the drain, unchanged — consistent with v1.11's own rationale that the pre-DRAINING txn write added no exclusion guarantee beyond what the flock check already provided. No new step-3.5 branch was added; Branch A and Branch B predicates are unchanged and remain exhaustive under the new ordering (Branch B's precondition `gate ∈ {LOCKED, DRAINING}` is now provably always true whenever `txn=STAGING(generation_id=null)` exists). Drain-timeout abort (step 4) simplified: the txn record now always exists by the time step 4 runs (step 3a is unconditional and precedes step 4), so no conditional "does a txn exist to mark ABORTED" branching is needed. Fault-injection test mandate expanded (§Decision 5a): (a) crash between step 3 and step 3a → gate=DRAINING, no txn → Branch A self-heals (pre-existing branch, new crash point, no new coverage needed); (b) DEF-1 regression/unreachability test — construct-and-assert-UNSAT that `gate=OPEN` with an active STAGING/COMMITTING txn cannot be produced by any crash-injection point between step 2 and the completion of step 3 under the v1.14 ordering; (c) reproduction of the exact pre-v1.14 defect scenario, asserted unconstructible. §Files-to-Change `executor.rs` and `tests/` rows swept to cite the new step-3a position and the DEF-1 regression suite. See §Consequences → Verification Strategy for the Kani harness re-verification requirement (the property the formal-verifier's Kani pass labels VP-M7) and the BC-1.18.011 handoff note (§Downstream to Product-Owner) confirming no BC-visible behavior changes. |

---

## Decision

### Decision 1 — Mechanism selection: one-time interactive Bash approval (Option B)

Governed one-time shard migrations are invoked via the **`Bash` tool with one-time interactive
human approval at F4 activation time — NO standing allowlist entry in settings.json.**

The agent invokes `{project-root}/target/release/factory-dispatcher migrate-bc-index` at the
F4 activation step. The Claude Code harness shows a permission prompt; the human approves once.
The migration runs, completes, and the permission expires. This is Option B. See §Rationale for
the head-to-head evaluation of Options A, B, C.

### Decision 2 — Binary placement

Both migration entry points live in the `factory-dispatcher` binary
(`crates/factory-dispatcher/`), co-located with `shard_manager.rs`. Rationale is unchanged:
co-location with `run_mechanism_a_backfill_split`, consistency with existing CLI dispatch layer.

### Decision 3 — Invocation: absolute-path-pinned, closed argument grammar

The agent MUST invoke the migration binary at its **absolute trusted path**:

```
{project-root}/target/release/factory-dispatcher migrate-bc-index
{project-root}/target/release/factory-dispatcher backfill-append-logs
{project-root}/target/release/factory-dispatcher migrate-bc-index --census
{project-root}/target/release/factory-dispatcher backfill-append-logs --census
```

The accepted argument grammar is CLOSED: exactly the above four forms. No additional flags,
no path arguments, no shell metacharacters, no compound commands. Any deviation is rejected by
the guard's pre-shell classifier (§Decision 5c) BEFORE shell execution, and rejected again by
the binary with a non-zero exit if somehow bypassed. No entry is added to `.claude/settings.json`.

### Decision 4 — Activation authorization (v1.3 REDESIGNED — closes F6)

#### 4a — Armed-activation manifest structure

State-manager writes the manifest to `.factory/activation/migrate-bc-index-YYYY-MM-DD.json`
(or `backfill-append-logs-YYYY-MM-DD.json`) at the F4 human-directed activation step.

The manifest MUST contain:
- `activation_id`: unique UUID for this activation
- `migration_id`: `"migrate-bc-index"` or `"backfill-append-logs"`
- `repo_root_sha`: factory-artifacts HEAD SHA at approval time
- `approved_arch_index_sha`: ARCH-INDEX committed SHA at approval time (§Decision 10)
- `expected_total_bcs`: integer from BC-INDEX frontmatter at approval time (B2 only)
- `approved_by`: `"human-F4-interactive"`
- `timestamp_utc`: ISO-8601 manifest creation timestamp
- `expires_after_hours`: `24`

**v1.3 addition:** `staged_generation_id` is NOT pre-specified in the manifest; it is filled
in by the migration binary when it creates the staging generation (see §Decision 7c step 1).
The manifest is then amended with `staged_generation_id` before the authorization gate check.

#### 4b — Pre-lock manifest checks (lightweight, before flock acquisition)

Before acquiring the advisory flock (§Decision 7a), the binary performs only:
1. Manifest file exists at the expected path — fails CLOSED if absent
2. `migration_id` matches the running subcommand — fails CLOSED on mismatch
3. Manifest is parseable — fails CLOSED on parse failure

These prevent flock acquisition against a clearly invalid activation state.

#### 4c — Under-exclusion validation (after flock acquisition, before staging generation build)

After acquiring the advisory flock (§Decision 7a), before any filesystem mutation:
1. Re-verify manifest exists and `migration_id` still matches
2. Current factory-artifacts HEAD SHA matches `repo_root_sha` — fails CLOSED on mismatch
3. `expected_total_bcs` matches current BC-INDEX frontmatter `total_bcs` — fails CLOSED on mismatch
4. Three-way ARCH-INDEX parity check (§Decision 10): `config.arch_index_sha` ==
   `manifest.approved_arch_index_sha` == live ARCH-INDEX SHA — fails CLOSED if any pair diverges
5. All readiness preconditions per BC-1.18.011 §Preconditions pass

**Note:** The expiry check (`expires_after_hours`) is NOT performed here. It is performed
immediately before the single destructive step (§Decision 4d). This keeps the under-exclusion
validation lightweight while positioning the authorization gate at the correct architectural point.

#### 4d — Authorization gate: immediately before CURRENT.json pointer swap (v1.3 — closes F6)

**The expiry check (authorization gate) occurs at §Decision 7c step 4, preceding the
fingerprint recheck (step 5) and the CURRENT.json pointer swap (step 6 — the single commit
point).** This location is (a) after all staging work is complete (so a timeout during staging
does not abort recoverable work) and (b) before any irreversible canonical-path visibility
change (the pointer swap at step 6). Note: the authorization gate and pointer swap are NOT
"immediately before" each other — step 5 (fingerprint recheck) intervenes between them.

**Authorization check logic:**
- If current UTC time is within `expires_after_hours` of `timestamp_utc`: pass; proceed to
  pointer swap.
- If expired AND txn record shows state=STAGING (pre-pivot, no canonical paths changed):
  ABORT cleanly — delete staging generation, update txn record → ABORTED, **flip gate →
  OPEN** (atomic write to gate_state file), release flock. The activation window elapsed
  before the pivot was reached. (H1 fix: gate MUST return to OPEN on every abort path.)
- If expired AND txn record shows state=COMMITTING (post-pivot already): this branch is only
  reached via the recovery path (§Decision 4e recovery case), which uses a separate
  completion-only authorization (see below). Ordinary expiry check is inapplicable here.

**Post-pivot invariant (NEVER violate this):** After the CURRENT.json pointer swap transitions
the txn record to state=COMMITTING, the transaction recovery record (txn record + intent log)
MUST be retained regardless of original manifest expiry. The COMMITTING state represents an
irreversible publication commitment: canonical readers are now directed to use staging-generation
paths. The authorization window governs ENTRY to the irreversible phase, not COMPLETION of it.

**Completion-only recovery re-authorization (replaces fresh activation for F6):**
When a recovery process encounters a COMMITTING transaction whose original manifest has expired,
it MUST use a **completion-only recovery manifest** to proceed. This is a distinct manifest type:
- Bound to the OLD `activation_id` (references, does not reuse)
- Bound to the staged-generation hash (computed from CURRENT.json `staged_generation_id`)
- Bound to `allowed_steps`: the set of remaining forward-recovery steps only (no pre-pivot steps)
- Contains the current `fencing_generation` from the txn record
- Approved by the human via a separate F4-level approval labelled `"human-F4-recovery"`

The completion-only manifest MUST be rejected by the binary if: `activation_id` does not match
the original, `staged_generation_id` does not match CURRENT.json, `allowed_steps` includes
pre-pivot steps, or the txn record `fencing_generation` does not match the manifest's value.

#### 4e — Manifest consumption and recovery modes

After migration reaches COMPLETED (completed.json written), the binary adds `consumed_at` to
the manifest (atomic overwrite). State-manager archives manifest to `.factory/migration-audit/`.

Re-run behavior is governed by TXN-RECORD state (primary discriminator per BC-1.18.011
Invariant 3) + completed.json presence. CURRENT.json is NOT the discriminator here:
it does not exist during STAGING (first written at step 6, the COMMITTING pivot) and
"CURRENT.json status:staging" is an impossible state.

| TXN-RECORD state | completed.json / manifest | Action |
|---|---|---|
| completed.json exists (terminal) **and no active (STAGING/COMMITTING) txn of this migration** (txn absent, COMPLETED, or ABORTED) | completed.json present | Exit 0 with `ALREADY_MIGRATED`; no migration lock needed; no manifest needed; **reconcile stale gate first** (§Decision 5c Branch 2 H-2/v1.12 F6 fix: attempt `flock(exclusive.lock, LOCK_EX\|LOCK_NB)`; on acquired: acquire gate LOCK_EX; if gate≠OPEN and no active txn: flip→OPEN; release gate LOCK_EX; release `exclusive.lock`; then exit 0. On EWOULDBLOCK: **(v1.23 item 11(c) — amended; the earlier "exit 0 with `ALREADY_MIGRATED`" is superseded)** skip everything, read nothing, write nothing, exit 1 `MIGRATION_LOCK_CONTENTION`; `completed.json` is read only AFTER the lock is acquired). In the clean steady state (gate OPEN) this row performs zero filesystem mutation. |
| **(v1.18 D3 — closes the permanent self-lock)** txn **COMMITTING** whose `activation_id`/`generation_id` match the terminal record, **and** the terminal record verifies (parses; `txn_id == activation_id`; `generation_id` equal; `canonical_paths_count == N` for this migration — 4 for `backfill-append-logs`; for EVERY canonical path `sha256(path) == expected_post_hash` per the txn record/intent log) | completed.json present (crash fell between §7c step 8's `completed.json` fsync and the txn→COMPLETED rewrite; typically gate=LOCKED) | **Verify-then-finalize, then exit 0 `ALREADY_MIGRATED`.** Under `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` (EWOULDBLOCK ⇒ live coordinator ⇒ no action, exit 1 `MIGRATION_LOCK_CONTENTION` — v1.23 item 11(c) amended this from "exit 0 with the §4e warning"): (1) re-select txn and re-verify under the lock; (2) atomically rewrite txn → COMPLETED (write-temp + fsync + rename + dir-sync, §7d); (3) THEN, under gate `LOCK_EX`, flip gate → OPEN if ≠ OPEN; (4) release locks. Order is mandatory (txn finalize BEFORE gate open) so `gate=OPEN ⇒ no active txn` is never violated. Idempotent; a crash after (2) leaves txn=COMPLETED + gate≠OPEN, which the §Decision 5c Branch 2 / §5a step-3.5 Branch A paths already repair. The SAME action is performed by the §5a step-3.5 PreToolUse self-heal (Branch C) so the lock never waits on an operator re-invoking the binary. |
| **(v1.18 D3)** **(a)** txn **STAGING (any `generation_id`, including null) with the migration's terminal record present — ALWAYS fail-closed, no verification is attempted or can succeed** (a terminal record is written only at §7c step 8, after the COMMITTING pivot; its presence beside a STAGING txn is an integrity anomaly, never a finalizable or discardable state; this row takes precedence over the `generation_id=null` DISCARD row below); **or (b)** txn **COMMITTING** with the terminal record present but **verification fails** (unparseable; `txn_id`/`generation_id` ≠ txn's; `canonical_paths_count` ≠ N; any canonical file hash ≠ `expected_post_hash`; or COMMITTING with a different `activation_id`) | completed.json present | **FAIL CLOSED:** NO txn finalization, NO gate flip, gate stays blocking; exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` (binary) / E-MAINTENANCE-001 block with the mismatch reason logged (PreToolUse). A human investigates; the state is never silently "repaired". |
| txn COMMITTING + valid manifest | CURRENT.json `status:committing` | Recovery: acquire flock; validate completion-only manifest or unexpired original; complete forward recovery |
| txn COMMITTING + manifest absent or expired | CURRENT.json `status:committing` | Abort with `RECOVERY_REQUIRES_REAUTHORIZATION`; human must issue completion-only manifest |
| txn STAGING + `generation_id=null` + valid manifest **(v1.12 F2 — pre-generation crash sub-state; v1.13 MED-3 — ABORTED+retain)** | CURRENT.json absent (pre-pivot, no gen dir exists) | **DISCARD (v1.13 MED-3)**: set txn state → ABORTED (txn record rewritten in place to `state: ABORTED` with top-level string field `abort_reason: "null_generation"`; optional on read, informational only, never consulted for admission or reconciliation decisions; retained per GC policy until archival — a pre-generation crash IS a failed activation attempt with audit value); flip gate → OPEN; exit `EXPIRY_ABORT`. No canonical paths changed; state fully recoverable under a fresh activation. **MUST NOT** attempt EC-003 census — no staging generation directory exists. **MUST NOT** compare PC1 against null `source_body_row_sha256`. Re-activate from scratch under a fresh manifest. |
| txn STAGING + `generation_id` set + staged gen dir present + valid original manifest | CURRENT.json absent (pre-pivot) | Resume from staging: validate under exclusion; **RE-RUN full census against staged generation (EC-003 per H3 fix — step 3b must pass before proceeding)**; re-run authorization gate (step 4); proceed to fingerprint recheck (step 5) and pointer swap (step 6) |
| txn STAGING + manifest absent or expired (any `generation_id`) | CURRENT.json absent (pre-pivot) | Clean-abort: delete staging generation directory (if present — a staged gen dir may exist when generation_id is set); set txn state → ABORTED (retained per GC policy until archival); flip gate → OPEN; exit `EXPIRY_ABORT`. (§4d clean-abort logic. No canonical paths were changed; state is fully recoverable after a fresh activation.) |
| txn ABORTED (terminal) **(v1.12 F4 — explicit ABORTED disposition)** | any | Treated as no active txn; fresh activation permitted; no recovery action required. ABORTED record retained until GC archival (see GC policy below). |
| no txn record + manifest absent | CURRENT.json absent | Reject as unauthorized; exit non-zero |
| `--census` flag | any | Read-only; no txn record, no manifest, no flock required |

**Txn-selection disambiguation rule (v1.12 F4):** When multiple `txn-*.json` files exist in
`.factory/migration-state/` (one per prior activation attempt), recovery and admission select
the record whose `activation_id` matches the CURRENT manifest's `activation_id`. If no current
manifest is present: select the record with the highest `created_at` timestamp. An ABORTED
record whose `activation_id` does not match the current manifest is inert for recovery purposes
(it does not trigger STAGING/COMMITTING blocking — those require state IN (STAGING, COMMITTING)).

**Terminal-txn GC/archival policy (v1.12 F4):** Terminal txn records (state ABORTED or
COMPLETED) SHALL be archived to `.factory/migration-audit/txn-archive/` by state-manager at the
next successful migration completion audit burst (the NIST AU-9 audit commit per §Decision 6).
Until archived, terminal records are inert: ABORTED records do not block admission; COMPLETED
records are superseded by `completed.json`. ABORTED records MUST NOT be deleted before archival
— they preserve the crash-debug audit trail for any failed activation attempt.

**Scope note (v1.9 L2 / v1.12 F2):** This table covers re-run and recovery behavior only.
First-run activation (no txn record + valid manifest present, no prior migration attempted) is
handled by §Decision 5c Branch 3 (New activation) and intentionally has no row in this table.
The "no txn record + manifest absent" row handles the distinct case where both the txn record
and the manifest are absent — that is an unauthorized invocation, not a first-run activation.
The STAGING row is now partitioned by `generation_id` presence (v1.12 F2): generation_id=null
means crash between drain step 3a and §7c step 1 (pre-generation) — **(v1.14 DEF-1): this
sub-state is now reachable ONLY with `gate_state ∈ {DRAINING, LOCKED}`**, never `gate=OPEN`,
because step 3a (txn write) executes strictly after step 3 (DRAINING flip); generation_id set
means crash after generation was assigned (normal resume path).

### Decision 5 — Dispatcher PreToolUse guard stack + native admission gate (v1.3 REDESIGNED)

#### 5a — Native maintenance admission gate: OPEN/DRAINING with writer reservations (v1.4 — closes F3, H1, H2)

**Dispatcher process model (v1.4 — closes H2):** The factory-dispatcher is a **per-event
binary** invoked once per PreToolUse or PostToolUse hook event and then exits. It is NOT a
persistent daemon. Consequently, an in-process `Mutex<{gate_state, active_writer_count}>`
cannot observe cross-process writer reservations — a PreToolUse binary increment and a
PostToolUse binary decrement are in SEPARATE PROCESSES. The admission mechanism MUST use
durable, file-system-level state for both gate_state and writer reservations.

**Gate states** (persisted to `.factory/migration-state/gate-state`, atomic replace-via-rename):
- `OPEN`: normal operation; new writer reservations accepted
- `DRAINING`: maintenance intent declared; no new writer reservations; coordinator awaiting quiescence
- `LOCKED`: coordinator has reached quiescence; migration in progress; no mutations admitted

**Writer reservation directory (v1.4 — replaces in-process counter, closes H2; v1.5 H-3 — tool_use_id correlation fix; v1.9 H1 — reservation content: created_at timestamp only, no PID):**
`.factory/migration-state/reservations/` — each admitted writer creates a
`<tool_use_id>.reservation` file in this directory when admitted (PreToolUse), where
`tool_use_id` is the stable harness tool-invocation identifier shared across the PreToolUse
and PostToolUse hook pair for the same tool call. This allows the PostToolUse hook (a SEPARATE
PROCESS from PreToolUse, since the dispatcher is per-event) to identify and remove the exact
reservation file without any shared in-process state. Reservation file content:
`{ "created_at": "<ISO-8601>", "tool_use_id": "<id>" }`. Do NOT store PID or
`process_start_time` in reservation files: the creating PID is the per-event PreToolUse binary
which exits immediately after writing the file — its PID is dead by the time any drain step 1
runs, making PID-liveness GC unsound for this model (v1.9 H1). A fresh random UUID is INCORRECT
for the filename: PostToolUse cannot recompute a random value from a prior process, causing
permanent reservation leaks. The count of files in this directory is the cross-process-durable
`active_writer_count`. Quiescence = directory is empty.

**Atomic admission protocol (closes H2 atomicity gap; v1.6 M-1 — explicit dual-check):**
- **PreToolUse:** before admitting a mutation (Edit/Write/MultiEdit/Bash) targeting
  `.factory/specs/behavioral-contracts/` or `.factory/cycles/`:
  0. **(v1.18 D5 — reserve-then-verify; supersedes the v1.4–v1.17 `LOCK_SH`-on-gate-state
     wording of steps 1 and 4 below, which was unsound: the coordinator's gate flip REPLACES
     `gate-state` via `rename`, and `flock` on the replaced inode does not serialize against an
     admitter that opened the old inode before the rename and then reads its stale `OPEN`
     content after the coordinator's `LOCK_EX` is released.)** The admitter MUST **create its
     `reservations/<tool_use_id>.reservation` file FIRST** (atomic temp+rename; step 4's file
     body), and only THEN perform steps 2–3.5 (read gate, scan txn). If the verification fails
     (step 5), the admitter MUST remove its own just-created reservation before returning the
     block (see "Release-on-block" below). **Why this is race-free (Dekker ordering; requires
     only that file create/rename/readdir are mutually coherent on one local filesystem —
     true on APFS/ext4):** the coordinator performs C1 = durable `gate_state=DRAINING` flip,
     then C2 = read `reservations/`. The admitter performs W1 = create reservation, then W2 =
     read gate. If W2 observes `OPEN` then W2 precedes C1, so W1 < W2 < C1 < C2 and the
     coordinator's C2 sees the reservation and waits; if C2 sees an empty directory then W1
     follows C2, so W2 follows C1 and observes DRAINING and backs off. No lock is held by the
     admitter, so there is nothing for the gate-file inode replacement to invalidate. The
     `LOCK_SH`/`LOCK_EX` on `gate-state` in steps 1/3/3.5(c)/6/§5c Branch 2 are RETAINED only
     as coordinator-vs-reconciler mutual exclusion on writers of the gate FILE (all of whom
     take `exclusive.lock` first anyway); they are no longer relied on for admitter ordering.
     **(v1.19 — first-activation race closed; S-25.08 implementation finding; normative twin
     BC-1.18.013 v1.7 Precondition 6(c) / BC-1.18.011 v1.15.)** The admitter first ensures
     `.factory/migration-state/reservations/` exists by idempotent recursive directory creation
     (already-exists is success; no existence pre-check, no lock). Admission, reservation and
     release MUST NOT be conditioned on the pre-existence of `.factory/migration-state/`; an
     absent `gate-state.json` is `OPEN`, an absent txn set is "no live txn". This makes the
     Dekker argument hold from the first protected write ever. Failure to create the directory
     or reservation fails the PreToolUse closed. **Cost note (v1.19; accepted, not a
     deferral):** the added work — one idempotent directory-create plus the reservation
     create/rename/unlink, no fsync, no lock — applies to EVERY protected
     `Edit`/`Write`/`MultiEdit` in a factory project, not only while a migration runs; it stays
     within the ≤ low-single-digit-ms increment already accepted under v1.18 (ADR-020 Class A
     budget). Writes outside the protected union, non-PreToolUse events, other tools, and a
     payload with no `tool_use_id` (check-only degradation, no directory and no reservation;
     backstopped by the §Decision 7c step-5 fingerprint recheck) are unchanged no-ops.
  1. *(superseded by step 0 — no admitter lock is taken; the "release/re-acquire `LOCK_SH`"
     clauses of step 3.5 sub-steps a, b, h and of steps 4–5 are likewise vacuous.)*
  2. Read gate_state value
  3. Check for an active txn record: scan `.factory/migration-state/txn-*.json` for state IN
     (STAGING, COMMITTING)
  3.5. **Flock-gated stale-gate reconciliation (v1.7 F4 / v1.10 HIGH-1 / v1.11 HIGH-1 / v1.13 HIGH-1 — extends predicate to null-generation STAGING crash sub-state; v1.18 D3 — adds Branch C, the verified-terminal-record COMMITTING sub-state):** If gate_state ∈ {LOCKED, DRAINING} AND (no active txn (txn absent, COMPLETED, or ABORTED) OR txn=STAGING AND generation_id=null OR **a live txn (STAGING any generation, or COMMITTING) AND the migration's terminal record is present (Branch C — for COMMITTING, verification decides finalize vs fail-closed; for STAGING it is ALWAYS fail-closed, no finalize path exists)**) — Branch A is REGARDLESS of completed.json presence; **precedence (v1.18): when the terminal record is present alongside a live txn, Branch C's rules govern and Branch B does not apply** (an unexplained terminal record next to a live STAGING txn is an integrity anomaly, never a discardable pre-generation crash):
     a. **Attempt `flock(exclusive.lock, LOCK_EX|LOCK_NB)` (v1.11 HIGH-1 primary fix):**
        - **If EWOULDBLOCK (errno EWOULDBLOCK/EAGAIN):** A live coordinator holds
          `exclusive.lock`. Maintenance is legitimately in progress — NOT a crash scenario.
          Release `LOCK_SH`; return E-MAINTENANCE-001 (block). Do NOT proceed to sub-step b.
        - **If acquired (returns 0):** No live cooperating process holds `exclusive.lock`.
          This IS a stuck-gate crash or abort scenario. Proceed to sub-steps b–h.
     b. Release `LOCK_SH`
     c. Acquire `LOCK_EX` on gate-state file
     d. Re-read gate_state and re-check txn record under exclusive lock (state may have
        changed between the initial LOCK_SH read and this exclusive acquisition)
     e. Re-check conditions under LOCK_EX and act based on which predicate branch matched:
        **Branch A — no active txn (txn absent, COMPLETED, or ABORTED):** If gate_state ∈
        {LOCKED, DRAINING} AND no active txn: write gate_state = OPEN (atomic
        replace-via-rename per normal gate-flip procedure). This repairs four stuck-gate
        scenarios:
        (i)  crash between step 8 (completed.json write) and the COMPLETED gate→OPEN flip
             (gate=LOCKED, no active txn, completed.json present — original v1.7 F4 case);
        (ii) abort path (census §7c step 3b / fingerprint §7c step 5 / expiry §7c step 4)
             crashes AFTER writing txn→ABORTED but BEFORE gate→OPEN flip
             (gate=LOCKED, txn=ABORTED, NO completed.json — covered by v1.10 HIGH-1
             generalisation);
        (iii) drain-timeout process crashes after writing txn→ABORTED (step 4) but before
             gate→OPEN reset; exclusive.lock released by crash; flock acquires → reconcile;
        (iv) any other abort/rollback path that set gate to LOCKED or DRAINING but crashed
             before completing the gate→OPEN reset.
        Idempotent: a newly-started STAGING (generation_id set) or COMMITTING txn blocks
        reconciliation. A legitimately locked gate with a live coordinator is blocked
        (EWOULDBLOCK from sub-step a).
        **Branch B — txn=STAGING AND generation_id=null (v1.13 HIGH-1 — null-generation crash;
        v1.14 DEF-1 — precondition now structurally gate-restricted):**
        If gate_state ∈ {LOCKED, DRAINING} AND txn=STAGING AND generation_id=null:
        apply §4e F2 DISCARD logic PreToolUse-side — set txn state → ABORTED (txn record
        rewritten in place to `state: ABORTED` with top-level string field
        `abort_reason: "null_generation"`; optional on read, informational only, never
        consulted for admission or reconciliation decisions; retained per GC policy, do NOT
        delete); write gate_state = OPEN
        (atomic). This covers: coordinator crashed between drain step 3a (txn written with
        generation_id=null) and §7c step 1 (generation UUID assigned and gen dir created);
        exclusive.lock was released by the crash; no staged generation directory exists.
        Idempotent: a live coordinator holding flock between step 3a and §7c step 1 is blocked
        by EWOULDBLOCK (sub-step a). **(v1.14 DEF-1):** because step 3a now executes strictly
        AFTER the step-3 DRAINING flip (never before it — see the DEF-1 rationale in the drain
        procedure above), `txn=STAGING AND generation_id=null` can now arise ONLY with
        `gate_state ∈ {DRAINING, LOCKED}`; `gate_state=OPEN` while this txn sub-state is active
        is structurally unreachable, so Branch B's precondition is exhaustive by construction —
        no `gate=OPEN` variant of this branch is needed.
        **Branch C — txn=COMMITTING AND the txn's own migration's terminal record is present
        and verifies (v1.18 D3 — PreToolUse-side verify-then-finalize; closes the permanent
        self-lock; the Branch A/B/C decision table is the single shared pure core
        `decide_terminal_record_reconciliation` in `shard_manager.rs`, used by both migrations
        and both Kani suites):** If gate_state ∈ {LOCKED, DRAINING} AND txn=COMMITTING AND the terminal
        record (`completed.json` for `migrate-bc-index`; `completed-backfill-append-logs.json`
        for `backfill-append-logs`, §Decision 7e) exists: **VERIFY** — it parses; its `txn_id`
        equals the txn's `activation_id`; its `generation_id` equals the txn's `generation_id`;
        its `canonical_paths_count` equals the migration's fixed N (4 for
        `backfill-append-logs`); and for EVERY canonical path of the migration
        `sha256(path) == expected_post_hash` as recorded in the txn record / intent log
        (content-verified, never assumed). **On success** (all under the `exclusive.lock`
        acquired in sub-step a and the gate `LOCK_EX` of sub-step c): (1) atomically rewrite
        the txn record to `state=COMPLETED` (write-temp + fsync + rename + dir-sync, §7d);
        (2) THEN write `gate_state=OPEN` (atomic). Order is mandatory — txn finalize before
        gate open — so `gate_state=OPEN ⇒ no txn in {STAGING, COMMITTING}` (INV-GATE-TXN) is
        never violated, including across a crash between (1) and (2) (that residue is
        txn=COMPLETED + gate≠OPEN, i.e. Branch A). **On any verification failure** (unparseable
        record; id/generation mismatch; count ≠ N; any hash mismatch; terminal record present
        alongside txn=STAGING (any `generation_id`); or COMMITTING whose `activation_id` ≠
        `txn_id`): finalize NOTHING, flip NOTHING, leave the gate blocking, block this PreToolUse with
        E-MAINTENANCE-001 (v1.21: that verdict's ONE `migration.admission_blocked` InternalLog event —
        `branch=completion_record_mismatch`, `check` naming the failed check, `migration_id`, `txn_id`,
        `reconciliation=completion_record_mismatch` — IS the diagnostic; NO separate
        `migration.admission_advisory` is written for a verification failure, see "Admission diagnostics
        channel") — the PreToolUse analogue of the binary's
        exit-2 `COMPLETION_RECORD_MISMATCH_ABORT` (a hook cannot emit a process exit code
        distinct from its block; the mismatch reason travels in the logged diagnostic and the
        block message suffix `(completion-record mismatch — operator investigation required)`).
        **E-MAINTENANCE-001 `<scope>` keying (v1.18; normative text in BC-1.18.013
        Precondition 6(b), same rule in BC-1.18.011 Precondition 6(b)/(d) and error-taxonomy):**
        every E-MAINTENANCE-001 block emitted by this admission path — including the Branch C
        mismatch block above and the ordinary live-txn/gate-only blocks — keys its `<scope>`
        token on the WRITTEN PATH FAMILY, never on the migration owning the live txn:
        `BC-INDEX` for a target under `.factory/specs/behavioral-contracts/`, `.factory/cycles/`
        for a target under `.factory/cycles/`. Rationale: the shared core also blocks when no
        txn exists (gate-only DRAINING/LOCKED), where no `migration_id` is available; a
        path-keyed scope is total over the dispatch. The live txn's `migration_id` (when one
        exists) goes to the structured `migration.admission_blocked` InternalLog diagnostic only (field `migration_id`; v1.21),
        never into the message. The message is exactly the format string
        `<scope> write blocked: migration window active (txn record in STAGING or COMMITTING state); retry after migration completes or aborts`
        with only `<scope>` substituted (the fixed parenthetical is emitted verbatim even for a
        gate-only block); the mismatch suffix above is appended only for Branch C verification
        failures.
        **Safety analysis (why a PreToolUse path may mutate txn state here):** (i) Branch C
        performs NO migration work — no canonical-path rename, no staging build; it only
        advances bookkeeping to the state the durable evidence already asserts (the terminal
        record is written only after ALL moves are hash-verified, §7c step 8, and Branch C
        re-verifies every hash itself). This is deliberately distinct from COMMITTING
        *forward recovery* (renames), which remains exclusively the binary's job (§4e rows 2–3;
        the B2 PreToolUse reconciler must not run it). (ii) It is monotone and idempotent
        (COMMITTING→COMPLETED is the existing §7a edge; re-running over COMPLETED is Branch A).
        (iii) It is flock-gated: a live coordinator holds `exclusive.lock` from drain step 2
        through completion, so EWOULDBLOCK ⇒ no action — Branch C cannot race a live migration.
        (iv) Precedent: Branch B already mutates txn state (STAGING-null→ABORTED) on this same
        path. (v) The gate is a consistency/safety interlock against concurrent agent writers,
        not an authorization boundary against a hostile local writer: anyone able to forge
        `completed.json` + a matching intent log + four matching canonical file hashes under
        `.factory/migration-state/` can already delete the txn record or rewrite `gate-state`
        directly, so Branch C grants no capability that principal lacks. (vi) Concurrent
        PreToolUse processes serialize on `flock(exclusive.lock, LOCK_EX|LOCK_NB)`: the loser
        sees EWOULDBLOCK and returns one transient E-MAINTENANCE-001 for that single dispatch
        (indistinguishable from a live coordinator by design; retry succeeds). (vii) Cost: Branch
        C is evaluated only while gate∈{LOCKED,DRAINING} with a COMMITTING txn — never on the
        gate=OPEN hot path — and hashes the migration's N canonical files once per attempt;
        under a persistent mismatch each blocked protected-path attempt repeats the hashing
        (bounded by the agent's own retry rate; acceptable for a fail-closed anomaly state).
        **Branch selection under LOCK_EX re-read:** if conditions have changed (e.g., a new
        legitimate txn started between the initial read and sub-step c acquisition), act on
        the re-read state. If no predicate branch matches under re-read: release locks and
        proceed to step 5 (block normally).
     f. Release `LOCK_EX` on gate-state file
     g. Release `exclusive.lock` flock fd (close — releases LOCK_EX acquired in sub-step a)
     h. Re-acquire `LOCK_SH` on gate-state file; re-read gate_state and re-check txn record
        (go back to step 4)
     **Note — non-reconciliation cases:**
     (1) gate_state ∈ {LOCKED, DRAINING} WITH an active txn: STAGING (generation_id SET) or
         COMMITTING **without a verifying terminal record (v1.18: a COMMITTING txn WITH a
         verifying terminal record is Branch C and IS reconciled; one whose terminal record is
         absent, or present but failing verification, is not)**: gate is LEGITIMATELY locked —
         do NOT reconcile; proceed to step 5 (block).
         **Exception (v1.13 HIGH-1):** txn=STAGING AND generation_id=null is NOT legitimately
         locked — it matches Branch B of the reconciliation predicate (crashed coordinator).
     (2) gate_state ∈ {LOCKED, DRAINING} AND the predicate matches (Branch A or B) BUT
         `exclusive.lock` is held by a live coordinator (EWOULDBLOCK in sub-step a): maintenance
         legitimately in progress — do NOT reconcile; block with E-MAINTENANCE-001. Covers both:
         (a) Branch A (no-txn): live coordinator mid-recovery; and
         (b) Branch B (txn=STAGING, generation_id=null): live coordinator between drain step 3a
             (txn written) and §7c step 1 (gen UUID assigned) — the coordinator is legitimately
             active even though txn has null generation_id.
     (3) gate_state = OPEN: no reconciliation needed; proceed to admission check.
         **(v1.14 DEF-1 closure):** by construction (step 3a always executes after step 3),
         `gate_state = OPEN` and an active txn (STAGING or COMMITTING) are now mutually
         exclusive — this case can never coexist with an active txn that would otherwise need
         Branch A/B reconciliation. Before v1.14, a crash between the old pre-DRAINING txn
         write and the DRAINING flip could leave `gate=OPEN` with an active
         `txn=STAGING(generation_id=null)` record, which this step-3.5 predicate (gated on
         `gate_state ∈ {LOCKED, DRAINING}`) never matched — see DEF-1 in the drain procedure
         above and the v1.14 changelog entry. That state is no longer reachable.
     **Parity note (v1.11 HIGH-1):** This "acquire-flock → re-read gate/txn under lock →
     flip if no active txn → release" discipline is identical to the "Crash-recovery gate
     reconciliation" paragraph below, which also acquires `exclusive.lock` first. Both paths
     use the same flock-gated reconciliation protocol.
     **Applied same-burst (v1.10 HIGH-1 / v1.13 HIGH-1):** error-taxonomy.md v1.27 reconciled
     the E-MAINTENANCE-001 clause to self-healing. E-MAINTENANCE-001 unblocks either when the
     migration completes/aborts normally (gate flips to OPEN) OR by the next PreToolUse
     self-heal of any stuck gate with no active txn AND no live coordinator (Branch A, this
     step 3.5 path). **(v1.13 HIGH-1 extension):** self-heal now ALSO covers Branch B: gate∈
     {LOCKED,DRAINING} + txn=STAGING(generation_id=null) + no live coordinator (dead coordinator
     between step 3a and §7c step 1). PO must widen error-taxonomy.md E-MAINTENANCE-001
     "self-healing" description to cover this case. All routes are automatic; no operator
     action required for routine stuck-gate cases. **(v1.14 DEF-1):** the same self-healing
     guarantee now also covers the crash window immediately AFTER the step-3 DRAINING flip and
     BEFORE step 3a's txn write completes (gate=DRAINING, no active txn — Branch A handles it,
     already listed among the (i)-(iv) scenarios above); the window that was NOT self-healing
     under v1.11–v1.13 (`gate=OPEN` with an active null-generation STAGING txn) no longer
     arises at all.
  4. If gate_state = OPEN AND no active txn: the reservation created at step 0 stands
     (`{ "created_at": "<ISO-8601>", "tool_use_id": "<id>" }`, no PID; atomic temp+rename;
     durability `fsync` is NOT required — only cross-process visibility, which `rename`
     provides — so the implementation MAY omit the fsync); proceed (admit)
  5. If gate_state ≠ OPEN (DRAINING or LOCKED) OR active txn exists (STAGING or COMMITTING)
     after step 3.5: **remove this invocation's own reservation (release-on-block)** and
     return E-MAINTENANCE-001 immediately (block)
  **Invariant (M-1 — dual-check):** Both conditions must be satisfied for admission: gate_state
  MUST be OPEN AND no active txn may exist. `gate_state` is the durable cross-binary proxy that
  persists txn blocking between per-event dispatcher invocations (the gate is set to LOCKED when
  the txn is STAGING or COMMITTING). Checking both guards against the race window between gate
  flip and txn record write. The txn record check is the authoritative "is maintenance in progress"
  signal per §Decision 7a ordinary-writer blocking; gate_state is the fast-path durable proxy.
  (v1.18 D5: admissions take no lock — parallel admissions never block each other; ordering
  against the coordinator is the step-0 reserve-then-verify Dekker argument.)

**Shared protected-path union and shared admission state (v1.18; ratifies BC-1.18.013 v1.3
Precondition 6(b) "union" clause).** Both governed migrations (`migrate-bc-index`,
`backfill-append-logs`) share ONE `exclusive.lock`, ONE `gate-state.json` (the physical name of
the logical `gate-state` file used throughout this ADR; `OPEN`/`DRAINING`/`LOCKED`, uppercase
JSON string), ONE `txn-*.json` directory, ONE `reservations/` directory. The protected path set
is the UNION `.factory/specs/behavioral-contracts/` ∪ `.factory/cycles/` (as this section has
always stated); a live (STAGING/COMMITTING) txn of EITHER migration blocks mutations under BOTH
families, and at most ONE live txn may exist across both migrations at a time. Admission is
implemented ONCE in a shared core in `shard_manager.rs` that the migration-neutral executor entry
points (`executor::migration_writer_admission` / `migration_writer_admission_precheck` /
`migration_reservation_release`, v1.21 — formerly `bc_index_migration_*`) delegate to; `main.rs` evaluates that core
exactly ONCE per PreToolUse event, at the REGISTRY-INDEPENDENT position specified in "Evaluation
position (v1.20)" below — before `Registry::load`, hence structurally before `shard_cap_precheck`
(whose fired branch performs a destructive seal-and-truncate and must never run for a blocked
protected-path write) and before every registry plugin tier. (v1.20 supersedes the v1.18 wording
"at the position and fire-before-any-registry-plugin ordering `bc_index_migration_admission_precheck`
already holds": that position is AFTER `Registry::load` in the merged code and so inherits its
fail-open arms.) It MUST NOT be evaluated twice (a second evaluation would re-create the
reservation and re-run reconciliation). Which of the two named entry points `main.rs` invokes is
an implementation detail ("or its successor", BC-1.18.013 Precondition 6(b)); the observable
obligation is the black-box test through the real dispatcher PreToolUse entry. The tool set of
the Rust gate is `Edit`/`Write`/`MultiEdit` only; the `Bash` write-effect leg remains the
separately-tracked [D-1232-OBL-4] §5c classifier (devops-engineer, F4 activation) exactly as
the §Status block and the BC-1.18.011/013 delivery cross-references state. **Residual until
OBL-4 ships:** a write-effect `Bash` command targeting a protected path is neither reserved nor
blocked by this Rust gate; the backstops are the existing `^Bash$` PreToolUse guards (POL-3) and
the §7c step-5 pre-commit fingerprint recheck (`FINGERPRINT_MISMATCH_ABORT` if any source file
changed after the quiescence snapshot).

**Coordinator exemption (v1.18; ratifies BC-1.18.013 v1.3 Precondition 6(b) "Exemption").** The
migration binary's own closed-grammar invocation is the coordinator, not a writer: it creates NO
writer reservation and is not blocked by the admission gate (otherwise the coordinator's own
step-4 poll would wait on its own reservation and always end in `DRAIN_TIMEOUT_ABORT`, and a
crash-recovery re-invocation could never be admitted while txn=COMMITTING). The exemption is
keyed EXCLUSIVELY on the §5c classifier's Branch 1–4 verdict — exact command-string match,
`realpath()`-resolved absolute `{canonical-binary-path}`, metacharacter rejection, executable
digest verification (§Decision 11) — never on a substring or prefix match; a compound command
(`&&`, `;`, `|`, redirection) fails classification and is treated as a writer. Because the
Rust gate does not process `Bash` (above), the exemption is implemented in the OBL-4
classifier; S-25.06's obligation is a regression test that the Rust precheck leaves `Bash`
unprocessed (creates no reservation, returns `None`) so the coordinator can never self-deadlock
through it, and that any future wiring of a `Bash` leg into the Rust precheck must carry the
exemption. The coordinator's own filesystem writes are made by the binary through `std::fs`,
not through hook-mediated tool calls, so they are never subject to the gate.

**Release-on-block (v1.18 D2 — closes avoidable reservation leaks).** Reservation creation at
admission (step 0) is followed, within the SAME dispatcher process, by `shard_cap_precheck` and
the registry plugin tiers, any of which may return Block/Error, in which case the tool never
runs and (by the harness contract) PostToolUse is not guaranteed to fire. Therefore: if the
dispatch's final aggregated outcome for a PreToolUse event is a block (exit 2) or an error, the
dispatcher MUST remove the reservation its own admission created for that `tool_use_id` before
exiting. (Harness-level denials the dispatcher cannot observe — a user denying the permission
prompt, or a block by a hook process other than this dispatcher — still leak until PostToolUse
or the TTL; that residual is exactly what the TTL and the operator remediation below cover.)

**Evaluation position (v1.20 — closes F-004; replaces the v1.18 "position
`bc_index_migration_admission_precheck` already holds" wording).** The native admission (PreToolUse)
and the reservation release (PostToolUse / PostToolUseFailure) are REGISTRY-INDEPENDENT. In
`main.rs::run` they execute immediately after the stdin payload parse and `resolve_project_cwd()`
and BEFORE the `CLAUDE_PLUGIN_ROOT` tiering, `resolve_registry_path()` and `Registry::load`. Why:
the registry-load outcomes are, per `main.rs::run` as read at HEAD 292ffed5 (this is the
authoritative current behaviour, not a ruling): `CLAUDE_PLUGIN_ROOT` unset/empty ⇒ Tier-1 degraded
empty registry, dispatch continues (final exit 0); `Registry::load` `RegistryError::SchemaVersion`
⇒ stderr `E-REG-001`, **exit 2 (fail-closed)**; `AsyncBlockConflict` ⇒ `E-REG-002`, **exit 2
(fail-closed)**; `DuplicateEntry` ⇒ `E-REG-003`, **exit 2 (fail-closed)**; every other load error
(file not found, TOML parse failure, regex error) ⇒ `return Ok(0)` **fail-open** per BC-1.08.001;
`resolve_registry_path()?` Err ⇒ `main` maps the `Err` to exit 0 (fail-open). (The `E-REG-NNN`
numbering above is the CODE's; `error-taxonomy.md` rows E-REG-001..003 currently list different
meanings and exit 0 — a taxonomy/code mismatch outside this ADR, flagged to product-owner; this
ADR does not change registry fail-open/closed policy.) `main.rs`'s own "Corrected scope"
note already accepts that every registry-ordered native leg is disabled by a broken
`hooks-registry.toml`. That acceptance is correct for a janitor (`reconcile_replace_all_overcap…`,
`shard_cap_precheck`) and WRONG for this gate: the admission gate is the writer-exclusion interlock
of a governed migration, so a missing or unparseable registry on a degraded install would silently
let protected writers run unreserved while a coordinator drains and snapshots (mid-write snapshot;
safety then rests only on the §7c step-5 abort). Ordering obligations (all testable black-box):
**O1** the core is evaluated exactly once per PreToolUse event; **O2** before `Registry::load` /
`resolve_registry_path()` / the Tier-1 degraded-registry branch, so it runs identically with
`CLAUDE_PLUGIN_ROOT` unset or empty, with a missing, unparseable or schema-mismatched registry, and
with an empty matched-plugin set; **O3** before `shard_cap_precheck` and every registry plugin tier;
**O4** an unparseable stdin payload cannot be classified (existing parse-error exit, unchanged — no
protected path is known, so nothing is reservable). Verdict handling at this position: a
`Block`/`Error` verdict terminates the dispatch directly through the SAME exit mapping the
empty-tier short-circuit already uses (`shard_gate_verdict_outcomes` → exit 2 + reason), without
loading the registry; an `Admitted` outcome stores the reservation handle in `admission_reservation`
and the dispatch continues into the registry stages unchanged. The release-on-block funnel then
covers EVERY later exit-2 outcome (the funnel keys on `code == 2`), including the registry
fail-CLOSED exits (schema-version mismatch `E-REG-001`, async+block conflict `E-REG-002`,
duplicate entry `E-REG-003` — all exit 2 in the code), shard-cap blocks and plugin blocks; a registry
fail-OPEN exit 0 (not found / parse / regex error, `resolve_registry_path` Err, Tier-1 degraded)
leaves the reservation for the matching Post event, which is itself registry-independent.
**Entry-point naming (v1.21 — closes F-014; supersedes the v1.20 sentence naming two per-migration delegates).**
The shared core serves BOTH governed migrations over ONE protected-path union, so its executor entry
points are migration-NEUTRAL and exist exactly once: `executor::migration_writer_admission(payload, cwd)
-> MigrationAdmission` (the reservation-returning form `main.rs` calls),
`executor::migration_writer_admission_precheck(payload, cwd) -> Option<HookResult>` (verdict-only form
over the same core) and `executor::migration_reservation_release(payload, cwd)`. These are renames of
`bc_index_migration_admission` / `bc_index_migration_admission_precheck` /
`bc_index_migration_reservation_release` (no behavior change). NO per-migration delegates are created:
`append_log_backfill_admission_precheck` / `append_log_backfill_reservation_release` never existed in
the code and must not be added — a second name for the same function with no distinct behavior is false
surface and invites a second implementation, the exact drift the shared core exists to prevent. This is
the "successor" BC-1.18.013 Precondition 6(b) allows for; every spec that names a per-migration entry
point (BC-1.18.011 Precondition 6, BC-1.18.013 Precondition 6(b), S-25.06, S-25.08 AC-002/AC-009, VP
text) is re-pointed to the neutral names (§Downstream v1.21).

**Admission scope anchoring (v1.20 — closes F-002).** The v1.18–v1.19 classification
`contains(".factory/specs/behavioral-contracts/")` / `contains(".factory/cycles/")` matched ANY
`.factory/…` path anywhere on disk while the migration-state namespace was read from
`CLAUDE_PROJECT_DIR`; a protected-looking path in another project, a nested project, a scratch
directory or a look-alike (`x.factory/cycles/…`) therefore created a spurious
`<project>/.factory/migration-state/reservations/` and was gated against a migration it does not
belong to. Ruling — the gate guards exactly ONE factory root per dispatch, the session's own:
(1) `project_root` = `resolve_project_cwd()` (canonicalized `CLAUDE_PROJECT_DIR`, else the process
cwd). (2) `factory_root` = `resolve_target_path(project_root/.factory)` (below). If `factory_root`
does not exist as a directory the gate is OUT OF SCOPE for the dispatch (no migration can be in
flight without it) and the gate NEVER creates `.factory` itself; `create_dir_all` is applied only to
`<factory_root>/migration-state/reservations` after that check. (3) The migration-state directory is
ALWAYS `<factory_root>/migration-state` — the same anchor the coordinator binary uses — never
derived from the written path. (4) Classification is COMPONENT-WISE against that root: a target is
`BcIndex`-family iff it equals or descends from `<factory_root>/specs/behavioral-contracts`, and
`Cycles`-family iff it equals or descends from `<factory_root>/cycles`; never a substring test.
(5) A `.factory/…` path that is not under the session's `factory_root` — another project's
`.factory`, a nested project's `project_root/sub/.factory`, a scratch tree, a look-alike — is OUT OF
SCOPE: admitted unconditionally, NO reservation, NO directory creation, NO read of any
migration-state, `tracing::debug!` only. Rationale: the interlock serializes a session against ITS
OWN project's governed migration (the coordinator operates on `<project>/.factory/migration-state`);
holding state for a foreign tree would mean writing outside the session's project, creating
`migration-state/` in trees that have no governed migration, and gating by a migration that tree's
sessions — not this one — are bound to; a foreign project's migration is guarded by sessions rooted
there. Residual (accepted, documented): a session in project A writing into project B's protected
paths is not serialized by B's gate; B's §7c step-5 pre-commit fingerprint recheck
(`FINGERPRINT_MISMATCH_ABORT`) is the backstop. (6) Worktrees need no special case: each session
anchors on its own `project_root`. If a worktree's `.factory` is a symlink (or bind mount) to the
shared factory-artifacts checkout, `resolve_target_path` follows it, so the worktree session and the
main session resolve to the SAME real `factory_root` and share ONE migration-state (correct: one
namespace per real directory). If a worktree has its own separate `.factory` checkout it is a
separate directory with its own namespace: a migration of the main checkout does not govern it and
vice versa, and an absolute-path write from such a session into the other checkout's `.factory` is
out of scope under (5). (7) A write whose target matches under EITHER the resolved comparison
(`T_real` vs `factory_root_real`) OR the lexical comparison (`T_lex` vs `factory_root_lex`, see
"Target path resolution") is in scope: the union is deliberately fail-closed (its only cost is a
spurious reservation, or a spurious block while a window is active, for a path that merely looks
protected through a symlink pointing out of the tree).

**Single anchoring rule — admission AND both coordinator binaries (v1.21 — closes D-2).** Rule (3)
above ("the same anchor the coordinator binary uses") was a one-sided assertion: the dispatcher
admission anchors on `resolve_project_cwd()` (canonicalized `CLAUDE_PROJECT_DIR`, else the process
cwd) while `run_migrate_bc_index_cli` passed the PROCESS cwd to `run_bc_index_migration`, which
derives `migration-state/` and every other `.factory/…` path from it (`_cwd.join(".factory/…")`); when
`CLAUDE_PROJECT_DIR` ≠ the process cwd (the harness launching a Bash tool from a subdirectory, a
worktree, a different shell cwd) the writer reservations land in one directory and the drain/gate
operate on another — the Dekker interlock silently guards nothing. Ruling — ONE rule, ONE function,
used by the dispatcher admission/release legs and by BOTH coordinator binaries (`migrate-bc-index`
and S-25.06's `backfill-append-logs`):
(a) *Project root.* A single pure `pub` function in the dispatcher library (`shard_manager`),
`resolve_session_project_root(claude_project_dir: Option<&OsStr>, process_cwd: &Path) -> SessionProjectRoot`
(`{ path: PathBuf, source: ProjectRootSource }`):
a PRESENT non-empty `CLAUDE_PROJECT_DIR` wins, canonicalized (a canonicalize failure falls back to the
as-given path — never to the cwd); an absent or empty value falls back to `process_cwd` exactly (NO
ancestor walk for a `.factory`, NO `git rev-parse`). `main.rs::resolve_project_cwd()` is reduced to a
call to it (env read in the shell, rule in the pure function — testable without env mutation).
(b) *Factory root.* `resolve_factory_root(project_root)` (the existing function, §5a rule (2)) is the
ONLY way any of the three call sites obtains a factory root. The coordinators map `None` (no
`.factory` directory) to the new fail-closed `BcIndexMigrationError::FactoryRootNotFound {
project_root, source }` (migration-binary exit 2, taxonomy `FACTORY_ROOT_NOT_FOUND`; `source` is the
`ProjectRootSource { ClaudeProjectDir, ProcessCwd }` the resolver reports alongside the path — the
resolver returns `SessionProjectRoot { path, source }`, not a bare `PathBuf`): NORMATIVE operator
line (single line on stderr, nothing on stdout): `<subcommand>: FACTORY_ROOT_NOT_FOUND: no .factory
directory under project root <project_root> (resolved from <source>)` where `<subcommand>` ∈
{`migrate-bc-index`, `backfill-append-logs`}, `<project_root>` is the resolved project root with control
characters escaped, and `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`}; the variant's Display is
exactly the text after `<subcommand>: `. The coordinator NEVER
creates `.factory`, never falls back to another directory, and mutates nothing.
(c) *Derivation.* `migration-state/` is `FactoryRoot::migration_state_dir()` and every other path the
coordinators read or write (`specs/behavioral-contracts/BC-INDEX.md`, `…/shards`, `cycles/…`, the
activation manifest) is derived from the SAME resolved real factory root; the six
`_cwd.join(".factory/…")` literals in `run_bc_index_migration` and its helpers are removed. The
public entry `run_migrate_bc_index_cli`/`run_bc_index_migration` take the resolved PROJECT ROOT (the
parameter is renamed from `_cwd` — it was never a cwd); `main.rs` computes it with (a) from
`std::env::var_os("CLAUDE_PROJECT_DIR")` and `std::env::current_dir()` for BOTH subcommand routes.
(d) *Where it lands.* S-25.09 (v1.22: the v1.21 scope was split out of S-25.08 by the human-approved
S-25.08/S-25.09 split, D-1252(f); the story is a sibling of S-25.08 in the same crate and depends on it).
Production-grade default: this is B2 conformance — the merged B2
coordinator is the non-conformant party, the files are the same `main.rs`/`shard_manager.rs`, and the
shared admission core's correctness argument depends on it. S-25.06's `backfill-append-logs` CLI does
not exist in code yet and MUST be built on (a)–(c) from its first line; S-25.06 gains the AC, S-25.09
delivers the function, the B2 re-anchoring and the tests (S-25.09 AC-001).
(e) *Verification.* Real-binary tests (cwd ≠ `CLAUDE_PROJECT_DIR`): the coordinator operates on —
and creates `migration-state/` only under — `<CLAUDE_PROJECT_DIR>/.factory`, a reservation created by
admission under the same env is visible to the coordinator's first drain; `CLAUDE_PROJECT_DIR` unset
or empty ⇒ process cwd; no `.factory` ⇒ exit 2 `FACTORY_ROOT_NOT_FOUND`, nothing created; a
`.factory` symlink resolves to one shared real namespace for admission and coordinator alike;
sibling-sweep gate `grep -n 'join(".factory' crates/factory-dispatcher/src/shard_manager.rs` returns
only the resolver. (f) Both coordinators' diagnostics use stderr (see "Admission diagnostics
channel"); a process with no subscriber must not rely on `tracing`.

**Factory-root lookup mapping (v1.23 ruling — closes F-S2508-L3-009; part of the single anchoring
rule).** Rule (2) of "Admission scope anchoring" ("if `factory_root` does not exist as a directory the
gate is OUT OF SCOPE") and rule (b) above ("no `.factory` directory ⇒ `FactoryRootNotFound`") both
speak of the directory NOT EXISTING. They never licensed treating a FAILED lookup as "does not exist".
`resolve_factory_root(project_root)` therefore classifies the ONE symlink-following `stat`
(`std::fs::metadata`) of `<project_root>/.factory` into exactly three outcomes, and MUST NOT collapse an
OS failure into "absent":
(a) *Found* — `stat` succeeds and the target is a directory ⇒ `Some(FactoryRoot)`.
(b) *Absent* ("no `.factory` directory") — a CLOSED set: `stat` succeeds but the target is NOT a
directory (a regular file, FIFO, socket, device, or a symlink resolving to one); OR `stat` fails with
`ENOENT` (includes a dangling `.factory` symlink); OR `stat` fails with `ENOTDIR` (a component of the
path prefix — e.g. the project root itself — is not a directory, so `.factory` cannot exist). ⇒ `None`.
(c) *Unstatable* — every other `stat` failure (EACCES, EPERM, EIO, ESTALE, ELOOP, ENAMETOOLONG, EMFILE,
ETIMEDOUT, …): whether the directory exists is UNKNOWN. ⇒ `Err(BcIndexMigrationError::Io { path:
<project_root>/.factory, source })`.
The function's return type is therefore `Result<Option<FactoryRoot>, BcIndexMigrationError>`; the
`is_ok_and(|m| m.is_dir())` collapse is removed. **A `.factory` that is a regular file is Absent**
(BC-1.18.013 EC-035 "absent, or a regular file" confirmed): there is no `.factory` DIRECTORY, so no
migration state can exist under it, the admission gate is out of scope, and the coordinator's
`FACTORY_ROOT_NOT_FOUND` text ("no .factory directory under project root …") is literally true.
Mapping by leg (identical classification, leg-appropriate verdict):

| Leg | Absent | Unstatable (Err) |
|---|---|---|
| PreToolUse admission (Edit/Write/MultiEdit with a valid `file_path`, after the input guards) | OUT OF SCOPE: admitted, no reservation, nothing created (`tracing::debug!` breadcrumb stays a non-obligation) | `E-MAINTENANCE-002 (io)`, FAIL CLOSED: one `migration.admission_failed` (`cause=io`; `detail` = sanitized path + `ErrorKind` + message), no `_blocked`, no reservation, nothing created, the write is NOT admitted |
| PostToolUse / PostToolUseFailure release | silent no-op (no reservation can exist) | NO verdict (release is never a verdict, §5a F-001): one `migration.admission_advisory` (`reason=reservation_release_failed`, `detail` as above), nothing created or deleted; the reservation, if one exists, is reclaimed by the drain-start TTL GC |
| Coordinators (`migrate-bc-index`, `backfill-append-logs`) | exit 2 `FACTORY_ROOT_NOT_FOUND` (unchanged, normative line unchanged) | exit 2 with the EXISTING `BcIndexMigrationError::Io { path: <project_root>/.factory, source }` (its existing Display and exit mapping); NOT `FACTORY_ROOT_NOT_FOUND`; nothing created or mutated; raised before any lock or write |

*Rationale, from the existing rules:* (1) fail-closed — the gate's whole safety argument (Dekker
interlock) is that a protected write is either admitted with a visible reservation or refused; admitting
it silently because the lookup that would have established scope FAILED converts an infrastructure fault
into a bypass, the exact failure class the v1.22 read-failure ruling forbids for canonical files; (2) the
total `<cause>` rule — "every way the check can fail to complete maps to EXACTLY ONE cause … an OS call
returned an error other than `ENOENT` on the file itself ⇒ `io`"; the `stat` of `.factory` is an OS call
the check makes, so its non-absent failure is `io` (AC-018), and `ENOENT` keeps its absent semantics
exactly as for `gate-state.json`/txn/terminal records (`ENOTDIR` and not-a-directory are the same
"cannot exist" fact for a directory lookup); (3) D-2 single anchoring rule — one function, one
classification, so the three legs cannot disagree about what "no `.factory`" means; only the verdict
differs by leg, following each leg's existing failure surface (guard verdict; best-effort release;
process exit code); (4) `FACTORY_ROOT_NOT_FOUND` asserts a fact (no directory) that an `EACCES`/`EIO`
stat did not establish — reporting it would mislead the operator, and an `Io` carrying the path and OS
error is the diagnosable truth, with no new taxonomy code or variant. *Blast radius, decided:* an
unstatable `.factory` blocks every Edit/Write/MultiEdit dispatch of the session (the root's identity is
unknown, so a path cannot be classified as protected or not; narrowing by a lexical `.factory` substring
is the F-002 over-match and would miss writes via a symlinked spelling); this is bounded and loud —
`Bash` is not gated by this leg, so the operator can repair the mount/permission and the next dispatch
recovers. Test vectors (one per row): `chmod 000` project root (EACCES), self-referential `.factory`
symlink (ELOOP), regular-file `.factory`, dangling `.factory` symlink, project root that is a regular file
(ENOTDIR), plus the three legs for the EACCES case.

**Admission diagnostics channel (v1.21 — closes D-1).** Fact check (HEAD 25e5464e): the dispatcher
binary installs NO `tracing` subscriber (`factory-dispatcher/Cargo.toml` has `tracing` but not
`tracing-subscriber`; no `set_global_default`; `sinks/mod.rs` records `Router`/`main.rs` wiring as
unfinished), so EVERY `tracing::warn!`/`error!`/`info!` in the dispatcher — admission's included — is
discarded in production. No story or ADR owns installing a subscriber (S-1.01 merely lists the
dependency); the dispatcher's durable, spec-owned diagnostic channel is the dispatcher-internal JSONL
log (`InternalLog` → `dispatcher-internal-YYYY-MM-DD.jsonl`, ADR-024 log-dir resolution; BC-3.08.001
"Durable sink target"; BC-1.12.001/002), which `main.rs::run` already holds as `internal_log` at the
admission call site. Ruling: (1) every admission/reconciliation/release diagnostic that this ADR or
a BC mandates MUST be emitted as an `InternalEvent` written to that log — NOT through `tracing`;
`tracing::debug!` for developer-only breadcrumbs (the out-of-scope notes) is not an obligation and
may remain; installing a global subscriber is explicitly NOT the fix (it would be a dispatcher-wide
logging-policy change that belongs to a separate decision, and is not needed here). (2) Purity is
preserved: the shared core and `executor::migration_writer_admission*` stay free of the log — they
return the diagnostics as DATA (`MigrationAdmission` gains `diagnostics: Vec<AdmissionDiagnostic>`;
`AdmissionOutcome::{Admitted,Blocked}` gain the same; the release leg returns its own list); `main.rs`
(the effectful shell) writes them with `internal_log.write(&InternalEvent::now(<type>)
.with_trace_id(..).with_session_id(..).with_field(..))` immediately after each call, BEFORE the early
return on a verdict (so a blocked/failed dispatch is logged too). Reasoning for not using
`HostContext::emit_internal`: no `HostContext` exists at the registry-independent admission position
(§5a "Evaluation position"); this is the one deliberate deviation from the BC-3.08.001 "dual-sink
helper" pattern, recorded in the catalog entries. (3) Three dispatcher-native event types, catalogued
in BC-3.08.001 as Events 11–13 (constants `MIGRATION_ADMISSION_BLOCKED`,
`MIGRATION_ADMISSION_FAILED`, `MIGRATION_ADMISSION_ADVISORY` in `internal_log.rs`):
`migration.admission_blocked` — one per `E-MAINTENANCE-001` verdict; fields `scope`
(`BC-INDEX` | `.factory/cycles/`), `family` (`bc_index` | `cycles`), `branch` ∈ {`gate_only`,
`live_txn`, `foreign_migration`, `live_coordinator`, `completion_record_mismatch`}, `gate_state`,
`migration_id`/`txn_id` (sanitized, length-capped, control characters escaped; `null` when no live txn),
`check` (non-null iff `branch = completion_record_mismatch`, then exactly one token of the CLOSED nine-token
domain stated under "Closed `check` domain (v1.22)" below; else `null`), `reconciliation` (CLOSED domain — the snake_case
token of the effectful `StaleGateReconciliation` OUTCOME the verdict followed, not the pure plan:
`live_coordinator` | `nothing_to_reconcile` | `gate_reopened` | `null_generation_txn_aborted` |
`foreign_migration_refused` | `completion_record_mismatch`; reconciliation always runs before a block, so
there is no `none`; the decision-table plan tokens `NoOp` / `RefuseForeignMigration` /
`FinalizeThenOpenGate` / `FailClosedMismatch` are NOT used on the wire). `branch` is derived: `live_coordinator`
⇔ reconciliation `live_coordinator`; `foreign_migration` ⇔ `foreign_migration_refused`;
`completion_record_mismatch` ⇔ `completion_record_mismatch`; otherwise `gate_only` when no live txn
remains after reconciliation, else `live_txn`. `migration.admission_failed` — one
per `E-MAINTENANCE-002` verdict; fields `cause` ∈ {`invalid_tool_use_id`, `io`, `state_integrity`},
`kind` (the `AdmissionStateIntegrityKind` token when `cause=state_integrity`, else `null`), `detail`
(path / `io::Error` kind+message / byte length — never a raw `tool_use_id` or record content).
`migration.admission_advisory` — non-verdict anomalies of the DISPATCHER legs; field `reason` ∈ {`reservation_release_failed`; `branch_a_gate_reopened`; `branch_b_txn_aborted`;
`branch_c_finalize_unwired`; `branch_c_finalized`}. Only `reason` is mandatory; the optional context
fields are a CLOSED set — `migration_id`, `txn_id`, `check`, `detail` (all sanitized and length-capped as
for `_blocked`), and `tool_use_id_len` (u64 byte length) — any other field is forbidden, and a `tool_use_id` is NEVER logged raw. Emission rules: (i)
`branch_a_gate_reopened` / `branch_b_txn_aborted` — written when the PreToolUse reconciliation
performs that repair (the dispatch then continues to its verdict; a later block still writes its own
`_blocked`); (ii) `branch_c_finalize_unwired` — written when the pure
`decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` (COMMITTING + a terminal record
whose checks verify) but the finalize effect is not delivered: S-25.08's fail-closed seam (the seam's code is
S-25.08's; its diagnostics mapping is S-25.09 AC-002(e)). The verdict is
the E-MAINTENANCE-001 block with the mismatch suffix, so ONE `_blocked` (`branch=completion_record_mismatch`) is
ALSO written, with `reconciliation=completion_record_mismatch` (the effectful outcome the code returns
for the unwired seam — NO seventh token), `branch=completion_record_mismatch` (never `live_txn`: the
verdict carries the mismatch suffix) and `check=finalize_unwired` — the advisory records the anomalous
seam-reached condition, the `_blocked` records the verdict. This reason is RETIRED when S-25.06 delivers the finalize (the seam no longer exists);
(iii) `branch_c_finalized` — written by S-25.06's verify-then-finalize on SUCCESS (txn→COMPLETED then
gate→OPEN; the write is then admitted, so NO `_blocked` follows) with `migration_id`/`txn_id`; it is
catalogued now so the closed `reason` domain is stable, but neither S-25.08 nor S-25.09 emits it; (iv)
`reservation_release_failed` — a non-ENOENT release error (never a verdict); (v) the five reservation-timestamp
fallback outcomes (`created_at_unparseable`, `created_at_pre_epoch`, `created_at_future`,
`mtime_future`, `age_unknown`) are NOT event reasons: admission never reads reservation files — only the
coordinator's drain GC calls `reservation_is_stale` — so they are single-line STDERR diagnostics of the
coordinator process (item (5) above), each carrying the same token. The `reason` domain is exactly the
five tokens in this list's items (i)–(iv). (4) The verdict message the operator sees is unchanged (single line,
cause token only); the diagnostics are the queryable detail. (5) The two coordinator binaries are
CLI processes whose stderr IS the operator surface: each failure/advisory they previously sent only to
`tracing` (`run_migrate_bc_index_cli`'s `migrate-bc-index: migration failed`, the drain's
reservation-staleness advisories) is written to stderr as one `E-…`/`<VARIANT>`-prefixed line;
`tracing::error!` there is retained only as an additional developer trace. (6) Owner/scope: S-25.09
(v1.22: split out of S-25.08, D-1252(f); S-25.09 AC-002)
delivers (2)–(3) for admission/release (small: three constants, a data type, ~6 call-site
conversions, the `main.rs` drain of diagnostics) and the coordinator stderr line for
`migrate-bc-index`; S-25.06 inherits (5) for `backfill-append-logs`. No deferral: the channel exists
(`InternalLog`) and the change is mechanical. The PRE-EXISTING dispatcher-wide discard of other
`tracing` call sites (≈60, none admission-mandated) is out of this ADR's scope and is not an
S-25.08 or S-25.09 obligation.

**Closed `check` domain (v1.22 — restates the v1.21 `_blocked` `check` field verbatim with BC-1.18.013
v1.10-rev2 Postcondition 10(a); ADR/BC consistency).** `check` is `null` unless `branch =
completion_record_mismatch`, in which case it is EXACTLY one token of the CLOSED NINE-TOKEN domain,
each a compile-time constant of an exhaustive `enum` with `fn token(self) -> &'static str` (never derived
from on-disk content): *verification checks*, evaluated in this fixed order, first failure wins, over the
live txn's OWN migration's terminal record — (1) `staging_with_terminal_record` (txn STAGING and its own
terminal record present; always fail-closed, no verification attempted), (2) `terminal_record_unparseable`,
(3) `terminal_record_schema_mismatch`, (4) `txn_id_mismatch`, (5) `generation_id_mismatch`, (6)
`canonical_paths_count_mismatch`, (7) `canonical_hash_mismatch` (first canonical path, in canonical order,
whose `sha256` ≠ its recorded `expected_post_hash` OR which is ENOENT); *seam tokens*, emitted only while
the verify-then-finalize effect is undelivered and retired when S-25.06 delivers the verifier — (8)
`terminal_record_unverified` (COMMITTING, own record present, the build cannot verify it), (9)
`finalize_unwired` (the pure core decided `FinalizeThenOpenGate` but finalize is undelivered; also yields
the `branch_c_finalize_unwired` advisory). The domain is exactly these nine tokens. Tokens (2)–(7) are
emitted by S-25.06 once it delivers the verifier; this build (S-25.08/S-25.09) emits only (1), (8) and
(9). A read CALL that fails with a non-ENOENT OS error is NOT a `check` token — see "Read-failure
mapping" under §Error Code Semantics (`E-MAINTENANCE-002 (io)`, no `_blocked`).

**Admission verdict surface label (v1.21 — closes F-013).** `shard_gate_block_outcome` /
`shard_gate_error_outcome` hard-code `plugin_name = "shard-cap-gate"`, so a migration-window
`E-MAINTENANCE-001/002` verdict prints `blocking_plugins=shard-cap-gate` — an operator would chase the
shard-cap gate for a migration block. Ruling: the admission verdict is reported as
`blocking_plugins=migration-admission` (constant `MIGRATION_ADMISSION_GATE_NAME`). Implementation
shape: `shard_gate_verdict_outcomes` (and the two outcome synthesizers) take the gate name as a
parameter (a `NativeGate { ShardCap, MigrationAdmission }` enum with `fn plugin_name(self) -> &'static str`
— closed, no free-form strings); `main.rs`'s admission leg passes `MigrationAdmission`, the
`shard_cap_precheck` leg keeps `ShardCap`. Spec impact: none beyond this ADR — no BC/VP/ADR names the
`shard-cap-gate` label or specifies the TD #71 `blocking_plugins=` stderr format for native gates
(ADR-048 §Decision 1 / BC-1.18.002 PC5 concern the `block_if_marker` crash-block reason; the
`blocking_plugins`/`block_reason` stderr line itself is a dispatcher-internal summary format, only
described in ADR-042/ADR-053 prose as "TD #71"); the E-MAINTENANCE message text and the exit code are
unchanged. S-25.09 gains the AC (S-25.09 AC-005; black-box: a blocked protected write's stderr line carries
`blocking_plugins=migration-admission`; a shard-cap block still carries `shard-cap-gate`).

**Admission state-integrity variant (v1.21 — closes F-012).** v1.20 let admission reuse
`BcIndexMigrationError::BinaryIntegrityFailure` (whose Display is the digest/TOCTOU code
`BINARY_INTEGRITY_FAILURE`) for malformed gate/txn records, a non-string `migration_id` and more than
one live txn, and `admission_failure_cause` ended in `_ => "state_integrity"`, so any future variant
would be silently mislabelled. Ruling: (1) NEW variant `BcIndexMigrationError::AdmissionStateIntegrity {
kind: AdmissionStateIntegrityKind, detail: String }` with a closed `AdmissionStateIntegrityKind` ∈
{`GateRecordMalformed`, `TxnRecordMalformed`, `TxnMigrationIdNotString`, `MultipleLiveTxns`,
`ReservationSerialization`} (each with a `&'static str` token: `gate_record_malformed`,
`txn_record_malformed`, `txn_migration_id_not_string`, `multiple_live_txns`,
`reservation_serialization`) **(v1.23 item 10(c): a sixth variant `TxnRecordNewerSchema` / `txn_record_newer_schema` is added; the domain is five tokens only as of v1.21)**. Display: `"migration admission: state integrity failure (<kind-token>):
<detail>"` — it must NOT contain `BINARY_INTEGRITY_FAILURE`. `process_exit_code` = 2 (default arm,
unchanged). `detail` is the path/parse message, sanitized by the surface that renders it (never record content; v1.25 item 11(g): 64 characters per data-derived substring in the InternalLog event, 256 on the operator stderr line). All five
current admission-side `BinaryIntegrityFailure` raises (gate/txn parse, txn not-an-object in
`abort_null_generation_txn`, non-string `migration_id`, >1 live txn, reservation serialize) become
`AdmissionStateIntegrity`; the shared loaders are also used by the coordinator, where the same variant
surfaces with exit 2 (no coordinator behavior change besides the message token). (2)
`admission_failure_cause` returns a closed `AdmissionFailureCause { InvalidToolUseId, Io,
StateIntegrity }` (`fn token(self) -> &'static str`) and its `match` over `BcIndexMigrationError` is
EXHAUSTIVE — no `_` arm: `InvalidToolUseId → InvalidToolUseId`, `Io → Io`, `AdmissionStateIntegrity →
StateIntegrity`, and every coordinator-only variant is listed explicitly in one arm that maps to
`StateIntegrity` with a comment that the arm is unreachable from admission and is the fail-closed
classification if ever surfaced; adding a variant is therefore a compile error until classified.
(3) Taxonomy: `E-MAINTENANCE-002`'s `state_integrity` row text names the variant
(`AdmissionStateIntegrity { kind }`, kinds above) instead of "any other integrity failure"; ONE new
migration-binary row `MIGRATION_STATE_INTEGRITY_FAILURE` (exit 2, blocked — the coordinator-side
surfacing of the same variant: malformed gate/txn record, non-string `migration_id`, more than one live
txn), explicitly "distinct from `E-BINARY-INTEGRITY-FAILURE` (digest/TOCTOU)"; and a second new row
`FACTORY_ROOT_NOT_FOUND` (exit 2, blocked — see "Single anchoring rule"). `E-BINARY-INTEGRITY-FAILURE`
text unchanged.

**EC-031 discharge — production TTL floor (v1.21 — closes F-006).** The compile-time
`const _: () = assert!(DEFAULT_MAX_RESERVATION_TTL >= MIN_PRODUCTION_RESERVATION_TTL)` makes a
below-floor PRODUCTION configuration unbuildable, but it does NOT discharge EC-031 by itself: the
BC's vector asserts a runtime ordering obligation ("rejected BEFORE any gate or drain action,
nothing mutated, byte-identical snapshot") that a constant cannot exercise, and a floor check that is
never exercised through the real call path can be silently reordered below the first mutation.
Ruling: keep BOTH layers and add a crate-PRIVATE injectable seam, NOT a public or
environment/argv-injectable input (an operator-controlled TTL would defeat the floor):
`pub fn run_bc_index_migration(project_root)` is a one-line delegation to
`pub(crate) fn run_bc_index_migration_with_ttl(project_root, max_reservation_ttl: Duration)`, passing
`DEFAULT_MAX_RESERVATION_TTL`; `run_bc_index_migration_with_ttl` calls
`validate_production_reservation_ttl(max_reservation_ttl)?` as its FIRST statement (before the
factory-root resolution's `create_dir_all`, `exclusive.lock`, gate write, GC). The vector is asserted
by in-crate tests on that seam: TTL 120 s and 1,799 s ⇒ `ReservationTtlBelowFloor` with a
byte-identical `migration-state/` snapshot (and no `exclusive.lock` created); 1,800 s and 3,600 s ⇒
proceeds; the test seam `drain_bc_index_writers` stays unbound by the floor; the const assertion is
vector (e) "default constant ≥ floor, compile-time". The same entry shape applies to
`backfill-append-logs` (S-25.06).

**Target path resolution (v1.20 — closes F-003; applies to `Edit`/`Write`/`MultiEdit` `file_path`;
the §5c Bash write-effect classifier (OBL-4) MUST use the same function for every extracted
write-target, mirroring §5c's `realpath()` discipline for the executable).** One shared function
`resolve_target_path` in `shard_manager.rs` returns the pair `(T_real, T_lex)`:
(a) *Input guards.* A missing, non-string, empty or NUL-containing `file_path` is out of scope (the
tool itself cannot write it); this is not an error. A relative `file_path` (defensive — the harness
sends absolute paths) is joined onto the payload's `cwd` when that is an absolute path, else onto
`project_root`; `~` is never expanded. (b) *Separators.* On Windows targets `\` is a separator and
`Path::components` handles drive/UNC prefixes; on Unix `\` is an ordinary name byte (the v1.19
unconditional `replace('\\','/')` over-matched and is removed). (c) *Lexical form `T_lex`.* Collapse
`//`, drop `.`, apply `..` lexically; no filesystem access. (d) *Real form `T_real`
(POSIX-correct; plain lexical normalization BEFORE resolution is WRONG because `a/link/../b`
resolves `link` first).* Walk the components left to right keeping `resolved` (always
symlink-free): `.` and empty components are skipped; `..` pops `resolved` (never above the root);
a normal component is `lstat`ed — a symlink is read and its target spliced in front of the
remaining components (an absolute target resets `resolved` to the root), bounded to 40 hops; an
existing non-symlink is pushed; on the first `NotFound` that component and ALL remaining
components are applied lexically without further `lstat` (a nonexistent component cannot be a
symlink; this realpaths "the deepest existing ancestor" and appends the unresolved tail). Any
non-`NotFound` error (EACCES, ELOOP, ENOTDIR, hop limit exceeded) makes `T_real` unavailable and
classification proceeds on `T_lex` alone — a protected-looking path that cannot be resolved is
treated as protected (fail-closed). (e) *Case.* All path-component comparisons — the
`factory_root` prefix AND the `specs`/`behavioral-contracts`/`cycles` family names — are ALWAYS
case-insensitive: ASCII case-fold for ASCII, `str::to_lowercase` equality for other UTF-8
components, ASCII-fold bytewise for non-UTF-8. Decision: no filesystem case-sensitivity probe —
sensitivity is a per-volume property (APFS default is insensitive; ext4 sensitive; a project can
mix mounts), a probe is racy and per-directory, and on a case-insensitive volume a probe-guided
exact compare would let `.FACTORY/Cycles/x` bypass the gate. Always-fold only over-matches on a
case-sensitive volume (a distinct-case sibling directory nobody uses), costing at worst a spurious
reservation (released at the Post event) or a spurious block during an active window. Unicode
NFC/NFD folding is NOT applied: the family names are ASCII and the `factory_root` prefix is
compared realpath-form to realpath-form, both kernel-produced. (f) *Residuals (accepted; same
consistency-interlock trust model as Branch C safety point (v) — same-user local, not an
authorization boundary):* hard links into a protected file from outside the tree, bind mounts not
visible to `lstat`, and a symlink swapped between admission and the tool's actual write (TOCTOU);
backstopped by the §7c step-5 fingerprint recheck.

**Lexical root spellings — `FactoryRoot.lex_aliases` RATIFIED (v1.20, S-25.08 implementation
ruling; refines "Admission scope anchoring" (7) and "Target path resolution" (c)).** The harness
may spell a target `file_path` through a non-canonical form of the project directory (macOS
`/var/...` for canonical `/private/var/...`; a symlinked checkout), so a lexical comparison against
ONLY the canonicalized root fails whenever `T_real` is unavailable. The lexical side of the union
therefore compares `T_lex` against the SET of lexical spellings of the session's own
`<project>/.factory`, exactly these two and no others: (i) the lexical normalization of the
canonical `factory_root` (`FactoryRoot.lex`), and (ii) the lexical normalization of
`<CLAUDE_PROJECT_DIR as given>/.factory` (the raw environment value before `canonicalize`; ignored
when the raw value is empty or not an absolute path; no filesystem access, no symlink resolution,
`.`/`..`/`//` collapsed lexically; deduplicated against (i)). Both spellings derive from the SAME
`CLAUDE_PROJECT_DIR` value, so each names the session's own project by construction; no
`file_path`-derived, payload-`cwd`-derived or `$HOME`/ancestor-derived spelling is ever added.
**Why it cannot over-match into another project:** classification stays component-wise (descendant
of `<spelling>/specs/behavioral-contracts` or `<spelling>/cycles`, case-insensitive per (e)); a path
under another project's `.factory` shares no prefix with either spelling unless that other project
IS the one `CLAUDE_PROJECT_DIR` names (then it is the session's own), a nested project's
`<project>/sub/.factory` is not a descendant of `<spelling>/specs|cycles`, and a look-alike
(`x.factory`, `.factory-old`) fails the whole-component match. The migration-state directory is NOT
affected: it remains `<factory_root_real>/migration-state` (real form when available, else
`lex`), never derived from an alias, so aliasing can only widen WHICH writes are gated, never WHERE
state lives. Cost: at most one extra lexical compare per protected-candidate write (no I/O).

**Reservation timing parameters (v1.18; confirms the v1.9 H1 / v1.10 MED-2 values and corrects
merged B2 code).** Drain quiescence timeout: 30 s (`DEFAULT_DRAIN_TIMEOUT`), poll interval ≤ 100
ms. `MAX_RESERVATION_TTL`: default 3,600 s; production floor 1,800 s (the production entry
point passes the constant; a value below the floor is a configuration error). The merged B2
constant `DEFAULT_MAX_RESERVATION_TTL = 120 s` VIOLATES this floor (a human permission-prompt
delay or a long `MultiEdit` could exceed 120 s while its reservation is live, letting the
step-1 GC reclaim it and the coordinator snapshot mid-write) and MUST be raised to 3,600 s; the
TTL/timeout remain injectable parameters of the drain function for tests (the floor binds the
production entry point, not the test seam). Staleness is judged from the reservation file's
`created_at` field, falling back to the file mtime only if the field is absent/unparseable
(the merged B2 code uses mtime alone). Stale reclamation is TTL-only: NO PID liveness, ever
(unchanged, v1.9 H1).

**Reservation timestamp rules (v1.20 — closes F-008).** The staleness basis is computed by the ONE
pure predicate `reservation_is_stale(created_at: Option<u64>, mtime: Option<u64>, now: u64,
ttl: u64) -> bool` (epoch seconds; `mtime` is `Option` because it can be unavailable). Constant
`RESERVATION_CLOCK_SKEW_TOLERANCE_SECS = 300` (5 min — covers NTP step and VM-resume skew, an order
of magnitude below the 1,800 s floor). Rules, in order: (1) `created_at` is parsed as RFC 3339 (any
UTC offset, normalised to UTC epoch seconds, sub-second truncated); a parse failure, a value before
1970-01-01T00:00:00Z (negative epoch) or a value outside `u64` is UNPARSEABLE ⇒ `None` — never
clamped (clamping a pre-epoch stamp to 0 would make a live writer's reservation maximally old and
reclaimable). (2) `created_at > now + 300` (a FUTURE stamp beyond the skew tolerance) is treated as
UNTRUSTED ⇒ `None`; a stamp in `(now, now + 300]` is accepted as ordinary skew and ages as 0.
(3) `basis = created_at.or(mtime)`; if both are `None` (mtime unavailable, including a pre-epoch
mtime) the age is UNKNOWN and the reservation is NOT stale — the merged B2 behaviour
`epoch_secs(pre-epoch) = 0 ⇒ reclaimable` is REMOVED. (4) `age = now.saturating_sub(basis)`; stale
iff `age > ttl`; a future `mtime` therefore also ages as 0. (5) Every fallback or unknown-age
outcome is reported as a single-line STDERR diagnostic of the coordinator process (v1.21; token ∈
{`created_at_unparseable`, `created_at_pre_epoch`, `created_at_future`, `mtime_future`,
`age_unknown`}) — only the coordinator's drain GC judges staleness, and the coordinators have no
InternalLog; these tokens are NOT `migration.admission_advisory` reasons). **Direction rationale:** the two possible errors are asymmetric. Wrongfully
RECLAIMING a live writer's reservation lets the coordinator observe quiescence and snapshot
mid-write (an integrity hazard). Wrongfully RETAINING one yields a bounded, visible stall —
`DRAIN_TIMEOUT_ABORT` with gate returned to OPEN — and the sanctioned manual-deletion remediation
above. Every ambiguous timestamp therefore resolves toward retention; clamping a future stamp to
`now` per evaluation (age 0 forever) and clamping a pre-epoch stamp to 0 (reclaimable) are both
rejected. A forged far-future `created_at` falls back to the file mtime, which the host filesystem
sets at creation and which the forger would also have to control; forging both requires write
access to `migration-state/`, which already implies the ability to delete the txn record or rewrite
`gate-state.json` (Branch C safety point (v)).

**`tool_use_id` presence and validity (v1.20 — closes F-009 part 2).** Grammar:
`[A-Za-z0-9_.-]{1,128}`, not starting with `.` (the harness form is `toolu_<alphanumerics>`).
Absent key or JSON `null` ⇒ check-only admission (unchanged v1.19 degradation: no directory, no
reservation). A key that is PRESENT but is not a string, is empty, or violates the grammar ⇒
`BcIndexMigrationError::InvalidToolUseId { len }` — the admission FAILS CLOSED (error verdict
`E-MAINTENANCE-002`, no reservation created, nothing admitted). It is not silently downgraded to
check-only: a harness that sends an id this core cannot key would otherwise turn every protected
write into an untracked admit with no signal. The error carries the byte length only, never the raw
value (log-injection / path-traversal hygiene). Contract drift in the harness id alphabet thus
surfaces loudly on the first protected write and is fixed by amending the grammar; the black-box
suite pins the grammar against the documented id form. The release leg treats an invalid id as a
silent no-op (it cannot have created a reservation).

- **PostToolUse AND PostToolUseFailure — release events (v1.20 — closes F-001):** after tool
  completion, remove the reservation file `reservations/<tool_use_id>.reservation` created by
  this tool call's PreToolUse admission, using the same `tool_use_id` the harness provides to
  both events of the pair. If the reservation file does not exist (writer crashed between Pre
  and Post, was never created, or was already released): no-op (normal).
  **Ruling.** Reservation release happens on BOTH hook events for the same `tool_use_id`:
  `PostToolUse` (the tool call succeeded) and `PostToolUseFailure` (the tool call failed, was
  interrupted, or was cancelled mid-execution). Claude Code delivers a failed tool call as
  `PostToolUseFailure` and does NOT fire `PostToolUse` for it; the v1.4–v1.19 text "after tool
  completion (success or failure)" therefore described an event the harness never sends for the
  failure leg and leaked the reservation of every failed protected write until the TTL.
  **Event payload shape (confirmed).** `PostToolUseFailure` is registered for the dispatcher in
  `plugins/vsdd-factory/hooks/hooks.json.template` (and the generated per-platform
  `hooks.json.*`), so the dispatcher is invoked for it. Its stdin envelope carries the common
  fields plus top-level `tool_name`, `tool_input`, `tool_use_id`, `error`, `is_interrupt` (Claude
  Code hook schema; the same `tool_use_id` value as the paired PreToolUse; repo corroboration:
  BC-4.08.001 Related-BCs names `tool_use_id` as the `PostToolUseFailure` join key, BC-4.08.006).
  The dispatcher's `HookPayload` has no named `tool_use_id` field; `#[serde(flatten)] extra`
  captures it on EVERY event (so `payload.extra["tool_use_id"]` is populated for
  `PostToolUseFailure` exactly as for `PostToolUse`), and `hook_event_name` is accepted via
  `serde(alias)`. **Gap to close in the test suite:** no in-repo fixture currently contains a
  `PostToolUseFailure` envelope with `tool_use_id` (the tool-failure-hooks fixtures read
  `tool_name`/`error_message` from `tool_input` only); S-25.08 MUST add one (real-binary
  black-box, shape above) — if a captured real payload ever lacks the id, the TTL backstop below
  is the only recovery and this ruling must be re-opened with the captured payload.
  **Dispatch-side obligation.** `invoke.rs::EventType::from_event_str("PostToolUseFailure")`
  returns `EventType::Other`; the release MUST NOT be conditioned on `EventType::PostToolUse`
  alone. Ruling: NO new `EventType` variant is added (BC-1.15.001 INV1's closed enum has
  sibling-sweep cost far beyond this fix and the variant would change routing semantics for
  every `Other` consumer); instead a single named pure predicate
  `is_tool_completion_event(event_name: &str) -> bool` (true iff `"PostToolUse"` or
  `"PostToolUseFailure"`, exact match on the already-aliased `payload.event_name`) gates the
  release. The release is keyed ONLY on `tool_use_id` — it applies NO `tool_name` filter (a
  failure envelope whose `tool_name` were absent or differently shaped must still release; the
  `tool_use_id` is globally unique, the unlink is idempotent, and a non-protected tool's id has no
  reservation file, so the cost is one `unlink` returning ENOENT; this also pre-wires the future
  Bash leg). A `tool_use_id` that fails the §Error-Code-Semantics grammar can never have created
  a reservation and is a silent no-op on release; release errors other than ENOENT are
  a non-fatal `migration.admission_advisory` InternalLog event (never a verdict). Release runs at the registry-independent
  position (§5a "Evaluation position").
  **If neither Post event arrives** (harness crash, session killed, hook timeout, a user denial
  of the permission prompt, a block by a hook process other than this dispatcher — the tool
  never ran or its completion event was lost): the reservation leaks until
  `MAX_RESERVATION_TTL` (3,600 s default, 1,800 s production floor) elapses, at which point drain
  step 1 GC reclaims it by `created_at` (TTL-only, no PID liveness); before that, the sanctioned
  operator remediation (manual deletion of a confirmed-stale `*.reservation`) applies. That is
  the designed residual and the sole one for this leg.

**Bash admission and reservation (v1.5 — closes M-6):** Every admitted Bash command whose
write effect targets `.factory/specs/behavioral-contracts/` or `.factory/cycles/` creates a
`<tool_use_id>.reservation` file by the same mechanism as Edit/Write/MultiEdit. The §5c
conservative-Bash-admission classifier determines whether a Bash command has a write effect on
those paths; commands the classifier determines to have NO write effect (read-only forms such as
ls, cat, grep, git log, git status, git diff) do not require a reservation and are not subject
to the gate admission protocol above. Quiescence waits for ALL in-flight reservations,
including Bash ones.

**Drain procedure (executed by maintenance coordinator before building staging generation):**
1. **Stale reservation TTL GC (v1.6 M-5 — runs at start of EVERY drain, not only recovery;
   v1.9 H1 — PID-liveness GC removed; TTL-only cleanup):**
   Scan `reservations/` directory for stale reservation files. Reservation files store ONLY a
   `created_at` ISO-8601 timestamp (NOT a PID or start-time — PID-liveness GC is unsound for
   per-event binaries; see rationale below). For each file: if
   `(current_time - created_at) > MAX_RESERVATION_TTL` (default: 3,600 seconds): the reservation
   is stale — remove the file. Otherwise: leave it in place (the corresponding tool call may still
   be in-flight or its PostToolUse may not yet have fired).
   **MAX_RESERVATION_TTL lower bound:** MAX_RESERVATION_TTL MUST be set to a value that CANNOT
   expire during a legitimate tool call. Tool calls complete in seconds to minutes; a 1-hour
   (3,600-second) default provides at least a 60× margin over the worst-case legitimate tool-call
   duration. Do NOT set MAX_RESERVATION_TTL below 1,800 seconds.
   **Why PID-liveness GC is unsound (v1.9 H1):** The factory-dispatcher is a per-event binary.
   The PreToolUse invocation creates the reservation file and then EXITS — microseconds later,
   before the tool executes. From that moment forward, the creating PID is dead. A liveness check
   (including the (pid, start_time) tuple from v1.7 F10) therefore classifies EVERY in-flight
   reservation as stale, because the creating PreToolUse process is dead by design. Reclaiming
   such a reservation causes the step-4 quiescence poll to see an empty directory while the tool
   is still running and its PostToolUse has not yet fired — the coordinator then snapshots source
   files MID-WRITE. The (pid, start_time) tuple correctly handles PID-reuse false-negatives for
   persistent daemon models; it does NOT rescue the per-event model where the creating process is
   dead by construction. TTL-based cleanup is the only sound mechanism for per-event reservations.
   This cleanup runs here (before DRAINING flip) so it does not race with the quiescence check
   in step 4. It prevents a crashed harness session's abandoned reservations from causing spurious
   DRAIN_TIMEOUT_ABORT on a subsequent migration (the abandoned reservation ages out within
   MAX_RESERVATION_TTL; the next drain proceeds normally after the TTL elapses).
2. Acquire advisory flock on `exclusive.lock` (§Decision 7a)
3. Acquire `LOCK_EX` flock on `gate-state` file; write gate_state = DRAINING; release `LOCK_EX`
   (LOCK_EX blocks new PreToolUse admissions while the flip is in progress)
3a. **(v1.14 DEF-1 — structural fix; supersedes v1.11 HIGH-1's pre-DRAINING ordering) Write
   initial txn record with state=STAGING and null source hashes** (`generation_id=null`,
   `source_sha256=null`, `source_body_row_sha256=null`, `intent_log_path=null`,
   `canonical_move_plan=[]` — v1.24 rename; the empty array is legal ONLY in the pre-swap STAGING
   sub-state, see ADR-054 Decision 2 / B-3). This step now runs strictly AFTER the DRAINING flip in
   step 3, not before it (v1.11 wrote it at a "step 2.5" positioned before the flip — see
   DEF-1 rationale below for why that ordering was defective).
   **Invariant established by this ordering (v1.14 DEF-1):** because the coordinator is a
   single sequential process holding `exclusive.lock` continuously from step 2 through
   completion/abort, and step 3a executes only after step 3 completes, a durable txn record
   with state=STAGING can come into existence ONLY when gate_state is already DRAINING (or
   later LOCKED). Equivalently: **`gate_state = OPEN` implies no txn record exists with
   state STAGING or COMMITTING.** This makes `gate=OPEN` together with an active txn a
   structurally unreachable combination — not merely a case handled by an extra branch.
   **DEF-1 rationale (why the v1.11 pre-DRAINING ordering was defective):** v1.11 wrote
   this same txn record BEFORE the DRAINING flip (at "step 2.5"), as a *defense-in-depth*
   measure whose own stated rationale was that "the flock check (step 3.5 sub-step a) is
   the primary guard ... writing txn=STAGING before DRAINING is defense-in-depth" — i.e.
   the flock check alone was already sufficient to preserve writer-exclusion during a live
   drain (verified by the "Live coordinator mid-drain — flock exclusion" fault-injection
   test below), and the pre-flip txn write added no exclusion guarantee that the flock
   check did not already provide. That redundant "belt" opened a NEW crash sub-window that
   did not exist before v1.11: a coordinator crash strictly between the old step 2.5
   (txn written, gate still OPEN) and step 3 (DRAINING flip) left durable state
   `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead
   coordinator`. The step-3.5 self-heal predicate (below) requires `gate_state ∈ {LOCKED,
   DRAINING}` — by design, since gate=OPEN is supposed to mean "no maintenance in
   progress" — so it never matched this state, and the ordinary admission check (step 5)
   blocked on the active STAGING txn with no self-heal path, contradicting this ADR's own
   guarantee that E-MAINTENANCE-001 always self-heals by the next PreToolUse. Before v1.11
   this state was unreachable (the txn record did not exist until after the coordinator
   had already flipped the gate). v1.14 restores that ordering guarantee while KEEPING the
   flock check (sub-step a) as the sole load-bearing writer-exclusion mechanism — exactly
   as v1.11's own rationale said it already was.
   Source hashes (`source_sha256`, `source_body_row_sha256`) remain null here; filled in at
   step 7 after quiescence snapshot.
4. Poll `reservations/` directory until empty; timeout at 30s → **ABORT: update txn record
   state → ABORTED (the txn record written at step 3a always exists by the time this
   timeout path runs, since step 3a executes unconditionally before step 4 begins — no
   conditional handling is needed); flip gate → OPEN (under LOCK_EX); release both flocks.**
   (H1 fix: abort path MUST flip gate → OPEN)
   (v1.5 L-4 LIVENESS NOTE: The 30s drain timeout is a safety net, not a production
   operating assumption. F4 activation SHOULD be scheduled when the system is quiescent with
   no concurrent agent Edit/Write/MultiEdit/Bash operations; the operator is responsible for
   choosing an activation window with no concurrent agent activity to avoid spurious
   DRAIN_TIMEOUT_ABORT.)
5. Snapshot now-quiescent inputs: (a) compute `source_sha256` = SHA-256 of all source files
   (used ONLY by step 5 fingerprint recheck — NOT by step 3b PC1); (b) capture the original
   BC-INDEX.md body ID census — the complete set of `BC-X.YY.NNN` IDs — and store in txn
   record for use by step 3b PC2; (c) **B2 migration only** — compute `source_body_row_sha256`
   = SHA-256 of the per-BC-row table-row content extracted from the original `BC-INDEX.md` body
   in canonical BC-ID sort order (all `BC-X.YY.NNN` table rows, sorted lexicographically by ID,
   concatenated as raw bytes), excluding §Summary, §Subsystem Shard Manifest, cross-cutting
   invariants, and non-row separator lines — and store in txn record for use by step 3b PC1.
   `source_sha256` and `source_body_row_sha256` are DISTINCT fields with DISTINCT scopes:
   `source_sha256` covers all source file bytes (fingerprint recheck); `source_body_row_sha256`
   covers only the per-BC-row content bytes (content-preservation PC1 check). Store all in txn
   record.
6. Acquire `LOCK_EX`; write gate_state = LOCKED; release `LOCK_EX`
7. **Update** txn record (first written at step 3a with state=STAGING and null hashes):
   fill in `source_sha256`, `source_body_row_sha256`, and original ID census snapshot from
   step 5. After this update, the txn record is fully populated for use by §Decision 7c.
   (`generation_id` and `intent_log_path` remain null here; filled in at §Decision 7c step 1
   when the migration binary assigns the staging generation UUID.) See §Decision 7a for the
   full txn record field schema.

**COMPLETED release:** When completed.json written (step 8), flip gate `LOCKED → OPEN` (under
`LOCK_EX` on gate-state file). This MUST succeed before the flock is released.

**Abort gate-reset obligation (v1.4 — closes H1):** EVERY abort and rollback path MUST flip
gate → OPEN before returning. This applies to:
- Drain-timeout abort (step 4 above) **(v1.14 in-place correction: this citation previously
  read "step 3 above," a stale step-number reference from before the step 3/3a renumbering;
  the drain-timeout abort has always been the step-4 reservation-poll-timeout path, not the
  step-3 DRAINING flip)**
- Expiry abort at §7c step 4 (authorization gate check)
- Fingerprint abort at §7c step 5
- Census abort at §7c step 3b (new in v1.4)
- Any other early-exit path that sets txn → ABORTED

**Crash-recovery gate reconciliation (v1.4 — closes H1):** A recovery process that acquires
the flock on `exclusive.lock` and finds gate_state = LOCKED or DRAINING MUST:
- Check for an active txn record (`txn-*.json` with state STAGING or COMMITTING)
- If NO active txn exists (txn absent, COMPLETED, or ABORTED): flip gate → OPEN; proceed
- If active txn exists (STAGING or COMMITTING): gate is legitimately LOCKED; proceed with
  forward recovery per §Decision 4e and §Decision 7b

**Stale reservation cleanup (v1.6 M-5 / v1.9 H1 — TTL-only; PID-liveness GC removed):**
Stale reservation TTL GC runs at the START OF EVERY DRAIN (drain procedure step 1, above) —
not only at recovery startup. Reservation files store ONLY a `created_at` timestamp; GC removes
files older than MAX_RESERVATION_TTL (default: 3,600 seconds). PID-based cleanup is REMOVED:
the creating PID is the per-event PreToolUse binary, which exits immediately, making all
reservations appear stale to a liveness check (v1.9 H1). TTL-based cleanup is the only sound
mechanism for per-event reservations. On crash recovery startup (separate from a first-run
drain), the same TTL GC also runs before re-attempting the drain.

**Operator remediation for abandoned-reservation stall (v1.10 MED-2):** If a harness session
crashes while a tool call was in-flight, it leaves an abandoned reservation file in
`.factory/migration-state/reservations/`. Every subsequent drain attempt will abort with
DRAIN_TIMEOUT_ABORT (30-second poll timeout) until MAX_RESERVATION_TTL elapses — up to 3,600
seconds by default — with no automatic recovery until then. Operator remediation BEFORE the
next activation attempt:
1. Identify the reservation file(s): list `.factory/migration-state/reservations/*.reservation`;
   each file contains `{"created_at": "...", "tool_use_id": "..."}`.
2. Confirm the reservation is stale: verify the `tool_use_id` does not correspond to any
   currently running harness session (no live Claude Code session active that could be its
   PostToolUse counterpart). If the creating session has terminated, the reservation is stale.
3. **SAFE MANUAL DELETION is SANCTIONED:** once confirmed stale (no live session owns it),
   deleting the reservation file from `.factory/migration-state/reservations/` is an explicitly
   authorised pre-activation operator action. This is the only exception to the principle that
   reservation files are lifecycle-managed by Pre/PostToolUse hooks.
4. After deletion, the next drain will see an empty reservations directory and reach quiescence
   immediately, without waiting for TTL expiry.
**Operational note:** The TTL lower-bound (MAX_RESERVATION_TTL MUST exceed the worst-case
legitimate tool-call duration) is a HEURISTIC operational constraint, not a structurally-enforced
invariant. The implementation cannot distinguish a crashed session's reservation from a very-long
legitimate tool call. F4 activation SHOULD be scheduled during quiescence (per the L-4 note on
the 30-second drain timeout) — a system with no concurrent agent operations eliminates both the
TTL stall risk and the drain-timeout risk. If the default MAX_RESERVATION_TTL is unnecessarily
long for the deployment, it MAY be lowered toward the lower bound (1,800 seconds) to shorten the
stall duration for confirmed-crash scenarios, at the cost of a narrower margin against very-long
legitimate tool calls.

**Fault-injection test mandate (v1.4 — H1; v1.5 — H-3, M-6; v1.6 — M-5; v1.7 — F4; v1.9 — H1;
v1.14 — DEF-1):** Tests MUST verify:
- Drain-timeout abort: gate returns to OPEN; next PreToolUse is admitted
- Expiry abort at step 4: gate returns to OPEN; txn = ABORTED
- Fingerprint abort at step 5: gate returns to OPEN; txn = ABORTED
- Census abort at step 3b: gate returns to OPEN; txn = ABORTED
- Writer admitted just before DRAINING flip: coordinator waits for its PostToolUse before
  proceeding past step 4 (quiescence wait); the admitted reservation was created within
  MAX_RESERVATION_TTL seconds and is NOT reclaimed by drain step 1 TTL GC
- (H-3) PreToolUse creates `reservations/<tool_use_id>.reservation`; PostToolUse (separate
  process, same tool_use_id provided by harness) removes it; directory is empty after both
  hooks fire for the same tool invocation
- (v1.9 H1) In-flight reservation NEVER reclaimed by drain step-1 TTL GC: simulate in-flight
  tool call (PreToolUse fired, tool running, PostToolUse NOT yet fired) — reservation file is
  less than MAX_RESERVATION_TTL old; drain step 1 MUST leave it in place; quiescence poll (step
  4) MUST block until PostToolUse removes it; verify coordinator does NOT snapshot source
  MID-WRITE. Reservation file stores `created_at` timestamp only (no PID, no start_time).
- (v1.9 H1) Stale harness session TTL cleanup: a reservation file older than MAX_RESERVATION_TTL
  is removed by drain step 1 GC on a FIRST-RUN migration (not only recovery); subsequent drain
  reaches quiescence and proceeds normally
- (M-6) Long-running admitted Bash with write effect creates a reservation; coordinator waits
  for PostToolUse reservation removal before proceeding past step 4 of drain procedure
- (F4) Stale-gate reconciliation via PreToolUse: simulate crash between step 8
  (completed.json written) and gate→OPEN flip — gate=LOCKED, completed.json present, no
  active txn. Verify that the NEXT PreToolUse admission (not a binary re-invocation) reconciles
  gate→OPEN; verify subsequent Edit/Write PreToolUse admissions are accepted; verify that a
  LOCKED gate with an active STAGING txn is NOT reconciled by this path (legitimate lock)
- (v1.10 HIGH-1) Abort-crash before gate→OPEN (txn=ABORTED, no completed.json): simulate an
  abort path (e.g., expiry abort at §7c step 4) that has written txn→ABORTED but crashes before
  the gate→OPEN flip — resulting state: gate=LOCKED, txn=ABORTED, no completed.json. Verify
  that the NEXT PreToolUse reconciles gate→OPEN via step 3.5 (gate=LOCKED, no active txn,
  regardless of completed.json); verify subsequent Edit/Write PreToolUse admissions are accepted;
  verify that a LOCKED gate with an active STAGING txn is NOT reconciled (legitimate lock)
- (v1.10 HIGH-1 / v1.13 HIGH-1 / v1.14 DEF-1 corrected) Drain-timeout or coordinator crash with
  step-3a txn written: simulate step 3 writing gate=DRAINING, then step 3a writing the initial
  txn record (state=STAGING, generation_id=null, source hashes null), then the process CRASHING
  before drain step 4 sets txn→ABORTED — resulting state: gate=DRAINING, txn=STAGING
  (generation_id=null), NO staged generation directory, flock released by crash. **NOTE
  (v1.14 DEF-1): as of v1.14, step 3a always writes the txn record AFTER the step-3 DRAINING
  flip (reversed from the v1.11–v1.13 ordering, which wrote it at a "step 2.5" BEFORE the flip).
  Consequently "gate=DRAINING, NO txn record" IS reachable again (from a crash between step 3
  and step 3a) — see the new step-3/step-3a interstitial test below — and is handled by the
  pre-existing Branch A predicate, unchanged.** Verify that the NEXT PreToolUse step 3.5 matches
  Branch B predicate (gate=DRAINING, txn=STAGING, generation_id=null) →
  attempts flock(exclusive.lock, LOCK_EX\|LOCK_NB) → acquires (no live coordinator) → sets
  txn→ABORTED (record rewritten in place to `state: ABORTED` + top-level string field
  `abort_reason: "null_generation"`; optional on read, informational only, never consulted for
  admission or reconciliation decisions; retained) → flips gate→OPEN → admits. Verify
  subsequent Edit/Write PreToolUse admissions are accepted. (Sub-case: if the crash happens
  AFTER drain step 4's txn→ABORTED write but BEFORE gate→OPEN flip: gate=DRAINING, txn=ABORTED
  → existing Branch A "no active txn" predicate handles it — no change.)
- (v1.14 DEF-1 — NEW) Coordinator crash strictly between step 3 (DRAINING flip) and step 3a
  (txn write): resulting state gate=DRAINING, NO txn record, flock released by crash, no gen
  dir. Verify that the NEXT PreToolUse step 3.5 matches Branch A predicate (gate=DRAINING, no
  active txn) → attempts flock(exclusive.lock, LOCK_EX\|LOCK_NB) → acquires (dead coordinator,
  since the crash released the flock) → flips gate→OPEN (no txn record exists to update) →
  admits. This is the structural replacement for the v1.11–v1.13 pre-DRAINING txn-write window;
  it was already covered by Branch A and by the "Live coordinator mid-drain" flock-exclusion
  test below, so no new predicate branch is required — this test only confirms the crash-point
  relocation did not open a gap.
- (v1.14 DEF-1 — NEW, regression / unreachability check) `gate=OPEN` with an active
  `txn=STAGING(generation_id=null)` record MUST be unreachable: attempt to inject a coordinator
  crash at every point between step 2 (flock acquired, gate still OPEN) and the completion of
  step 3 (DRAINING flip) and confirm the txn record is NEVER written before step 3 completes —
  i.e., no crash-injection point in this interval can produce `gate=OPEN` co-existing with a
  STAGING or COMMITTING txn record. This is the DEF-1 regression test: it was constructible
  under the v1.11–v1.13 ordering (crash between the old pre-DRAINING step 2.5 and step 3) and
  MUST be proven UNSAT under the v1.14 ordering. See §Consequences → Verification Strategy for
  the corresponding Kani model-checking harness requirement (re-verifies the property the
  formal-verifier's Kani pass labels VP-M7).
- (v1.13 HIGH-1) PreToolUse self-heal for null-generation STAGING crash — NEW test:
  Simulate the null-generation crash state (gate∈{LOCKED,DRAINING}, txn=STAGING, generation_id=null,
  no gen dir, flock released). Verify NEXT PreToolUse step 3.5: (a) matches Branch B predicate;
  (b) attempts flock → ACQUIRES (dead coordinator); (c) re-reads under LOCK_EX — confirms Branch B;
  (d) rewrites the txn record in place to `state: ABORTED` with top-level string field
  `abort_reason: "null_generation"` (optional on read, informational only, never consulted for
  admission or reconciliation decisions; RETAINED — do NOT delete); writes gate→OPEN;
  (e) releases both locks; (f) admits the writer. VERIFY no EC-003 census attempted. VERIFY that
  with a LIVE coordinator holding flock (step 2 acquired, between step 3a txn-write and §7c step 1),
  the concurrent PreToolUse receives EWOULDBLOCK → blocks with E-MAINTENANCE-001 → MUST NOT flip
  gate→OPEN and MUST NOT set txn→ABORTED.
- (v1.11 HIGH-1) Live coordinator mid-drain — flock exclusion (no-flip test): simulate a live
  coordinator that holds `flock(exclusive.lock, LOCK_EX)` (drain step 2) and has written
  gate=DRAINING (drain step 3) with NO txn record yet (the normal v1.14 window between step 3
  and step 3a, or a delayed step 3a). A concurrent ordinary-writer PreToolUse enters step 3.5
  (gate=DRAINING, no active txn predicate matches): MUST attempt
  `flock(exclusive.lock, LOCK_EX|LOCK_NB)` → receive EWOULDBLOCK (coordinator holds it) → MUST
  block with E-MAINTENANCE-001 → MUST NOT flip gate→OPEN. Verify BC-1.18.011 Precondition 6
  writer-exclusion is preserved throughout the coordinator's drain. **(v1.14 DEF-1):** this test
  is the PRIMARY guard for the gate=DRAINING-no-txn window and remains sufficient on its own;
  the txn record written at step 3a is a secondary/defense-in-depth signal, not required for
  correctness of this test.
- (v1.14 DEF-1 — supersedes v1.11 HIGH-1 "defense-in-depth" framing) Normal live drain — txn
  written after DRAINING: verify that after step 3 flips gate=DRAINING and step 3a writes the
  initial txn record (state=STAGING), a concurrent PreToolUse step 3.5 sees `gate=DRAINING +
  txn=STAGING (active) AND exclusive.lock EWOULDBLOCK` for the remainder of the drain — both
  guards independently block reconciliation once step 3a has run; for the brief interval between
  step 3 and step 3a, `exclusive.lock EWOULDBLOCK` alone blocks reconciliation (see the new
  step-3/step-3a interstitial test above). Verify drain timeout abort (step 4) correctly sets
  txn→ABORTED (the txn record always exists by step 4, since step 3a is unconditional) and THEN
  flips gate→OPEN.
- (v1.12 F2 / v1.13 MED-3 / v1.14 DEF-1 updated) Null-generation crash recovery
  (migration-binary path): simulate crash between drain step 3a (txn record written with
  state=STAGING, `generation_id=null`, `source_sha256=null`, `source_body_row_sha256=null`) and
  §7c step 1 (generation UUID generated, gen dir created). Resulting state: txn=STAGING,
  generation_id=null, no gen dir, gate ∈ {DRAINING, LOCKED} depending on crash timing (never
  OPEN — v1.14 DEF-1 invariant). Verify that recovery via binary re-invocation: (1) detects
  generation_id=null in the txn record; (2) rewrites the txn record in place to `state: ABORTED` with
  top-level string field `abort_reason: "null_generation"` (optional on read, informational only,
  never consulted for admission or reconciliation decisions; RETAINED per GC policy — do NOT delete the txn record); (3) flips gate → OPEN;
  (4) exits `EXPIRY_ABORT`. VERIFY it does NOT: (5) attempt EC-003 census (no gen dir exists);
  (6) compare PC1 against null `source_body_row_sha256` (unconditional CONTENT_PRESERVATION_ABORT).
  VERIFY txn record is still present (ABORTED state) after the operation — preserved for audit
  trail per GC policy. Post-recovery: verify a fresh activation with a new manifest proceeds
  normally from drain step 1.

**v1.18 fault-injection and black-box test mandate (S-25.06 D1/D2/D3/D5; applies to BOTH
migrations through the shared core):**
- (D3) completed.json + txn COMMITTING + gate LOCKED + all canonical hashes at
  `expected_post_hash` (crash between §7c step 8's `completed.json` fsync and the txn→COMPLETED
  rewrite): (a) the NEXT PreToolUse protected-path Edit/Write — with NO binary re-invocation —
  runs Branch C, txn→COMPLETED, gate→OPEN, and is admitted; (b) a binary re-invocation instead
  exits 0 `ALREADY_MIGRATED` with the same end state; (c) a second run of either is a
  zero-mutation no-op; (d) a crash injected between the txn rewrite and the gate flip leaves
  txn=COMPLETED + gate≠OPEN, repaired by Branch A on the next dispatch; (e) with
  `exclusive.lock` held (live coordinator) Branch C takes no action and blocks.
- (D3 mismatch) the same fixture with `completed.json.txn_id` ≠ txn `activation_id`, or
  `generation_id` ≠, or `canonical_paths_count` ≠ N, or one canonical hash ≠
  `expected_post_hash`, or txn=STAGING: NO txn write, NO gate write, PreToolUse blocks
  E-MAINTENANCE-001, binary exits 2 `COMPLETION_RECORD_MISMATCH_ABORT`; a byte-for-byte
  snapshot of `.factory/migration-state/` is unchanged by the attempt.
- (D1) black-box through the REAL dispatcher entry (spawned `factory-dispatcher` binary with a
  PreToolUse envelope on stdin, not a direct call of the precheck function): `Write` to a
  `.factory/cycles/<any>/` file with txn STAGING ⇒ exit 2 E-MAINTENANCE-001; the same to
  `.factory/STATE.md` ⇒ admitted; gate OPEN + no live txn ⇒ admitted AND
  `reservations/<tool_use_id>.reservation` exists until the matching PostToolUse removes it;
  PostToolUse for an unknown id is a no-op; with the gate OPEN but a live txn of EITHER
  migration the same block applies to BOTH path families (union test); the gate is evaluated
  exactly once per event (assert the reservation file is written once / reconcile not re-run).
- (D2) coordinator drain: a reservation with `created_at = now` never removed ⇒
  `DRAIN_TIMEOUT_ABORT` (timeout injected small), txn ABORTED, gate OPEN, `source_sha256` never
  recorded, files untouched; a reservation older than TTL (injected) ⇒ GC'd at step 1 and the
  drain proceeds; a writer admitted before the DRAINING flip completes before the snapshot
  (no `source_sha256` computed while any non-stale reservation exists).
- (D5) interleaving test of the reserve-then-verify order: with a scripted admitter/coordinator
  interleave at each of {W1, C1, C2, W2} orderings, either the coordinator observes the
  reservation or the admitter observes DRAINING; never both miss.
- (Release-on-block) a PreToolUse whose later stage (shard-cap gate or a registry plugin) blocks
  leaves no reservation file behind.
- (Cross-migration, §7e) a live txn of one migration is never recovered/finalized by the other
  migration's binary (foreign live txn ⇒ refusal), and one migration's terminal record never
  satisfies the other's `ALREADY_MIGRATED`/reader checks.

**v1.20 test mandate (S-25.08 share; real spawned `factory-dispatcher` unless stated):**
- (F-001) `PostToolUseFailure` envelope `{hook_event_name:"PostToolUseFailure", tool_name:"Edit",
  tool_input:{…}, tool_use_id:<id>, error:"…", is_interrupt:false|true}` following an admitted
  protected PreToolUse of the same id removes `reservations/<id>.reservation` (directory empty
  after the pair); `is_interrupt:true` likewise; a failure event whose `tool_name` is absent still
  releases; a failure event for an unknown/never-reserved id, an invalid id, or with
  `migration-state/` absent is a zero-mutation no-op exit 0; `PostToolUse` release is unchanged.
- (F-004) with a live STAGING txn, a protected `Write` is blocked E-MAINTENANCE-001 (exit 2) under
  each of: `CLAUDE_PLUGIN_ROOT` unset; registry file unparseable; registry empty/no matching plugin;
  and with the gate OPEN the reservation is created under each. Under a schema-mismatched registry
  (`RegistryError::SchemaVersion`, code `E-REG-001`, fail-closed exit 2 in `main.rs`) the dispatch
  exits 2 and NO reservation remains (release-on-block); the same for an async+block-conflict and a
  duplicate-entry registry (exit 2); a not-found / TOML-parse-error registry is fail-open (exit 0)
  and the admission verdict is unaffected.
  PostToolUse/PostToolUseFailure release works under an unparseable registry.
- (F-002) a protected-looking path outside the session's `factory_root` (sibling temp dir
  `…/other/.factory/cycles/x`, `project/sub/.factory/cycles/x`, `x.factory/cycles/y`) is admitted
  with NO `reservations/` or `migration-state/` created anywhere and a live txn in the session's own
  state does not block it; a session whose `project_root` has no `.factory` directory creates none;
  `.factory` as a symlink to a real checkout resolves to the real `factory_root` (one shared
  namespace, reservation lands in the real directory).
- (F-003) in-scope under a live txn (blocked) and, with gate OPEN, reserved: `…/./cycles//a/../x`,
  an alias symlink `/tmp/alias → <factory_root>/cycles` target `/tmp/alias/x`, `link/../` ordering
  (`<factory_root>/cycles/sub/../x` where `sub` is a symlink elsewhere resolves the symlink first),
  a nonexistent tail `<factory_root>/cycles/new-cycle/new.md`, `.FACTORY/CYCLES/x` and
  `.Factory/Specs/Behavioral-Contracts/BC-INDEX.md` (always-fold), a relative `file_path` joined to
  the payload `cwd`; out of scope: `<factory_root>/STATE.md`, `<factory_root>/stories/x`, a
  backslash-named file on Unix. An unresolvable (EACCES) ancestor falls back to lexical and is still
  protected.
- (F-006) pure core: `txn_migration_known=false` ⇒ `RefuseForeignMigration` for every input
  vector (precedence over record checks); dispatcher: a live `backfill-append-logs` txn with NO
  `completed-backfill-append-logs.json` and a present B2 `completed.json` ⇒ plain E-MAINTENANCE-001
  block, txn not finalized, gate not opened; a live `backfill-append-logs` COMMITTING txn WITH its
  own verifying record ⇒ Branch C finalize + admit (dispatch on `migration_id`); an unknown
  `migration_id` string ⇒ plain block, never aborted/finalized, diagnostic only.
- (F-008) `reservation_is_stale` table: `created_at` = now+299 ⇒ age 0; now+301 ⇒ falls back to
  mtime; pre-1970 / non-RFC3339 / year-9999 ⇒ falls back to mtime; mtime future ⇒ not stale;
  mtime unavailable and `created_at` unusable ⇒ not stale (warn `age_unknown`); `created_at` 4,000
  s ago with fresh mtime ⇒ stale at TTL 3,600 (basis is `created_at`).
- (F-009) `validate_production_reservation_ttl(1_799 s)` ⇒ `ReservationTtlBelowFloor{1799,1800}`
  (not `BinaryIntegrityFailure`); `tool_use_id` present as a number / `""` / `"../x"` / 129 chars ⇒
  `InvalidToolUseId`, verdict `E-MAINTENANCE-002`, no reservation, no directory; absent/`null` ⇒
  check-only admit.

**Fingerprint recheck:** Retained at §Decision 7c step 5 as defense-in-depth after quiescence.
Not the drain mechanism.

**Non-upgrade:** Do NOT use a shared→exclusive flock upgrade as the drain mechanism. Use the
explicit OPEN→DRAINING→LOCKED state machine with the gate-state file as described above.

#### 5b — Guard amendments for `^Bash$` PreToolUse guards (v1.3 — updated for new state machine)

The following 4 guards require amendment (updated to reference txn record state and
CURRENT.json rather than exclusive.lock PID check):

| Guard | Required action |
|---|---|
| `destructive-command-guard` | Amend to recognize absolute-path-pinned migration commands after §5c classifier pass; reference txn record state for maintenance check |
| `validate-factory-path-staging` | Full-command pre-shell classifier (§5c) + manifest content validation; txn record state check replaces exclusive.lock PID check |
| `validate-heavy-op-delegation` | Allow migration command when completion-only or ordinary armed-activation manifest is present and validated |
| `validate-factory-path-staged` (PostToolUse) | Recognize governed migration subcommands; pass through without Edit/Write staging semantics |

#### 5c — Full-command pre-shell classifier (v1.3 REDESIGNED — closes F7, F8)

The guard receives the full proposed Bash command string BEFORE shell execution. The classifier
operates in four distinct branches:

**Branch 1 — Read-only census (closes F8 census gap):**
- Trigger: command matches `{canonical-binary-path} {migrate-bc-index|backfill-append-logs} --census`
- No manifest required; no flock required; no CURRENT.json check required
- Executable digest verification still applies (§Decision 11)
- Pass immediately after digest check

**Branch 2 — Terminal-state no-op (closes F8 COMPLETED-rerun gap; v1.6 H-2 — gate reconciliation; v1.12 F6 — flock-parity):**
- Trigger: `completed.json` exists at `.factory/migration-state/completed.json`
- No migration manifest required
- **Gate reconciliation before exit (H-2 fix; v1.12 F6 flock-parity):** Before exiting 0, the
  guard MUST check and reconcile a stale gate. Gate reconciliation now requires the same
  flock-acquirable precondition as step 3.5 and the "Crash-recovery gate reconciliation"
  paragraph, for parity (v1.12 F6):
  0. **(v1.12 F6)** Attempt `flock(exclusive.lock, LOCK_EX|LOCK_NB)`.
     - **If EWOULDBLOCK:** A live coordinator holds the lock. Unexpected when `completed.json`
       is present (terminal state means no live coordinator should hold it) — log a warning:
       "unexpected: completed.json present but exclusive.lock held." Do NOT reconcile the gate;
       it will self-heal via the next PreToolUse step 3.5. **(v1.23 item 11(c) — amended:)** the
       binary exits 1 `MIGRATION_LOCK_CONTENTION` (nothing read, nothing written), NOT exit 0
       `ALREADY_MIGRATED`; `completed.json` is (re-)read only after the lock is acquired.
     - **If acquired (returns 0):** No live coordinator. Proceed to gate reconciliation steps 1–4.
  0.5. **(v1.18 D3 — closes the Branch-2 gap: "no active txn" was the only reconciled shape,
     so a COMMITTING txn + durable terminal record left gate AND txn stuck forever)** Select the
     txn by the §4e rule. If it is STAGING (any `generation_id`) alongside the terminal
     record: fail closed immediately (below) — no verification, no finalize. If it is
     COMMITTING alongside the terminal record, run the §5a step-3.5 Branch C verification
     (terminal record parses; `txn_id`/`generation_id` match; `canonical_paths_count == N`;
     every canonical hash `== expected_post_hash`). On COMMITTING-verification
     success: atomically rewrite txn → COMPLETED FIRST (step 0.5 is under the `exclusive.lock`
     of step 0), then continue to steps 1–4 (the "no active txn" condition of step 3 is now
     true). On failure (including every STAGING + terminal-record case): finalize nothing, flip nothing, release `exclusive.lock`, exit 2
     `COMPLETION_RECORD_MISMATCH_ABORT`. Mirrored — same predicate, same order — by the
     PreToolUse self-heal Branch C so neither path is the sole route.
  1. Acquire `LOCK_EX` on gate-state file
  2. Read gate_state value
  3. If gate_state ≠ OPEN AND completed.json present AND no active txn record exists
     (txn absent, or txn state = COMPLETED or ABORTED — including a txn finalized in step
     0.5): write gate_state = OPEN (atomic)
  4. Release `LOCK_EX`; release `exclusive.lock` flock fd (step 0 acquisition)
  "completed.json" in this Branch is the invoking migration's own terminal record (§Decision
  7e: `completed.json` for `migrate-bc-index`, `completed-backfill-append-logs.json` for
  `backfill-append-logs`). **Clean steady state (gate OPEN, no active txn): zero filesystem
  mutation.** The merged B2 short-circuit that rewrote ANY non-COMPLETED txn to COMPLETED on
  `completed.json` presence without verification (and could therefore finalize a foreign
  migration's STAGING/ABORTED txn) is REPLACED by this verified, own-migration-only path.
  This reconciliation is a no-op when the gate is already OPEN (normal case). It is also a
  no-op when an active STAGING or COMMITTING txn exists (gate is legitimately LOCKED).
  **Parity note (v1.12 F6):** This "acquire exclusive.lock → acquire gate LOCK_EX → re-read
  → flip if no active txn → release" discipline matches step 3.5 sub-steps a→h and the
  "Crash-recovery gate reconciliation" paragraph. Benign under current semantics (Branch 2
  triggers only when completed.json is present — terminal state — so EWOULDBLOCK at step 0 is
  unexpected); parity-safe against future Branch-2 trigger broadening.
- Binary exits 0 with `ALREADY_MIGRATED`
- **Fault-injection test mandate (H-2; v1.12 F6):** Tests MUST:
  (a) Simulate the crash window between completed.json write and gate→OPEN flip. Verify that
      a subsequent Branch 2 invocation reconciles gate → OPEN (step 0 acquires `exclusive.lock`,
      steps 1–4 flip gate, step 4 releases both locks); verify subsequent Edit/Write PreToolUse
      admissions are accepted after this reconciliation.
  (b) Verify that Branch 2 with `exclusive.lock` held by a live process exits 1
      `MIGRATION_LOCK_CONTENTION` (v1.23 item 11(c); formerly "exits 0 with `ALREADY_MIGRATED`")
      without reading or modifying txn/gate state; and (v1.23) that `completed.json` appearing between
      an existence probe and lock acquisition never starts a fresh run.

**Branch 3 — New activation:**
- Trigger: no `completed.json` + no CURRENT.json with `status: committing`
- Full validation: exact command string match; metacharacter rejection; alternate-path rejection;
  manifest content validation (parseable, `migration_id` matches, not expired, `activation_id`
  is valid UUID); executable digest verification (§Decision 11)
- If any check fails: REJECT with descriptive error

**Branch 4 — Recovery:**
- Trigger: CURRENT.json `status: committing` exists
- Full validation: command string match; metacharacter rejection; executable digest verification;
  completion-only manifest validation (§Decision 4d) OR unexpired original manifest
- If any check fails: REJECT

**Conservative Bash admission for all other commands (closes F7, H4):**
When the txn record exists with state=STAGING or COMMITTING, ANY Bash command not matching
one of the four exact sanctioned forms (§Decision 3) MUST be BLOCKED if the command has any
write effect on paths overlapping the allowed-paths list (§Decision 8). "Unknown write effect"
means: shell metacharacters (`>`, `>>`, `|`, `&&`, `;`, `||`) OR commands not in an allowlist
of known read-only forms (ls, cat, grep, git log, git status, git diff, etc.). Unknown-effect
commands are fail-closed: if the guard cannot determine write effect is absent, BLOCK.

**Canonicalization and alias rejection:** The executable path is resolved via `realpath()` at
guard evaluation time. A relative path, a symlink pointing outside the project tree, or a
PATH-resolved name (no path separator) is rejected at Branch 3/4 regardless of final resolution.
The canonical project root is resolved via `git rev-parse --show-toplevel` at guard load time.
**(v1.20, F-002/F-003)** That git-toplevel root is used for validating the executable path only. The
write-effect TARGET classification of this classifier (OBL-4) MUST use the same anchor and the same
`resolve_target_path` (lexical + POSIX-correct symlink resolution of the deepest existing ancestor +
always-case-insensitive family compare) as the Rust Edit/Write/MultiEdit gate (§5a "Admission scope
anchoring" / "Target path resolution"): the session's own `factory_root`, component-wise, so the
two legs can never disagree about whether a path is protected.

**Negative tests (v1.16 CORRECTED — cluster-5 F4 ACTIVATION deliverable, devops-engineer,
per §Status block [D-1232-OBL-4] and the §Files-to-Change `validate-factory-path-staging` /
`validate-factory-path-staged` rows — NOT cluster-5 TDD scope):** this §5c classifier (and the
4 dispatcher-guard amendments it feeds, §Decision 5b) is explicitly carried forward in the
ratified §Status block as "[D-1232-OBL-4] devops-engineer implements the 4 dispatcher-guard
amendments specified in §Decision 5b/5c against this ADR's frozen spec text" — a post-ratification
activation-boundary obligation, distinct from and NOT gating cluster-5 TDD completion (contrast
[D-1232-OBL-1], the impl-phase Kani obligation, which explicitly DOES block cluster-5 TDD
completion). Cluster-5's `bc_index_migration_admission_precheck` native gate (`executor.rs`/
`shard_manager.rs`, T-10/T-11, implementer-owned) covers only the `Edit`/`Write`/`MultiEdit`
admission path per BC-1.18.011 Precondition 6(b) — it does not implement this §5c `^Bash$`
full-command classifier. The stale "(cluster-5 TDD scope)" label below predates the D-1232
ratification's explicit OBL-4 split and is corrected here; the negative-test list itself is
unchanged and remains this classifier's own acceptance criteria, to be authored by
devops-engineer at cluster-5 F4 activation, not by test-writer/implementer during cluster-5 TDD:
- `{root}/target/release/factory-dispatcher migrate-bc-index && rm -rf /tmp/test` → REJECTED
- `/tmp/evil/factory-dispatcher migrate-bc-index` → REJECTED
- `{root}/target/release/factory-dispatcher migrate-bc-index --extra-flag` → REJECTED
- `{root}/target/release/factory-dispatcher` (no subcommand) → REJECTED
- Any Bash command with `>` targeting `.factory/specs/behavioral-contracts/` while txn=STAGING or COMMITTING → REJECTED (F7/H4 test)
- `{root}/target/release/factory-dispatcher migrate-bc-index` (when completed.json exists) → BRANCH-2 pass
- Abort during DRAINING (drain-timeout) → gate returns to OPEN; next PreToolUse mutation is admitted (H1 test)

### Decision 6 — Audit trail: durable, tamper-evident record (NIST AU-9) (v1.5 H-1: inlined; completed.json replaces COMMITTED phase marker per v1.3 redesign)

Census-to-stdout is correctness evidence for D-449(a); it is NOT an adequate audit record.
NIST SP 800-53r5 AU-9 requires audit records to be tamper-evident and protected from
modification or deletion, independent of the current session. Two independent audit artifacts
are required:

1. **D-449(a) evidence (stdout capture):** The migration CLI MUST write a structured census
   report to stdout on completion (success or idempotent no-op). Minimum required fields:
   - Source file path(s) operated on
   - Pre-migration record count (from `total_bcs` oracle for B2; from known file count for A)
   - Independent-census record count (fresh enumeration of actual rows/entries)
   - Per-shard record counts and file paths (including sub-shards for SS-05/SS-06 in B2)
   - Content-preservation hash (SHA-256 of source body before migration)
   - Idempotency status (`FIRST_RUN` | `ALREADY_MIGRATED` | `RESUMED_FROM_CHECKPOINT`)
   - Migration binary version and build SHA
   - Exit code taxonomy (0 = success, non-zero with E-SHD-* error code on failure)

   State-manager captures the stdout verbatim as the D-449(a) burst-log Dim-2 attestation.

2. **Durable audit record (NIST AU-9):** Immediately after the migration reaches COMPLETED
   state (completed.json written at §Decision 7c step 8), state-manager commits to
   factory-artifacts:
   - The captured census stdout as `migration-audit/migrate-bc-index-YYYY-MM-DD-census.txt`
     (or `backfill-append-logs-...`)
   - The armed-activation manifest (consumed copy from `.factory/activation/`)
   - The completed.json terminal record
   - The migration binary's `--version` output

   The factory-artifacts commit hash is recorded in STATE.md's burst-log for the activation
   burst. This factory-artifacts commit is the tamper-evident record: it is immutable in git
   history and not subject to session compaction.

### Decision 7 — Crash-atomicity, lock ownership, and atomic publication (v1.3 REDESIGNED)

#### 7a — Lock ownership: advisory flock on stable never-unlinked inode (closes F4, F5)

**Lock inode:** `.factory/migration-state/exclusive.lock` — created ONCE by devops-engineer
at cluster-5 activation preparation and NEVER deleted, NEVER unlinked. Its presence as a
stable inode is guaranteed before any migration activation.

**Flock acquisition:**
```
fd = open(".factory/migration-state/exclusive.lock", O_RDWR)
result = flock(fd, LOCK_EX | LOCK_NB)
```
- If `result == EWOULDBLOCK`: another process holds the lock → return E-MAINTENANCE-001 immediately.
  Do NOT inspect, unlink, or attempt to reclaim based on lock file contents.
- If `result == 0`: no live cooperating owner holds the lock (kernel guarantees: lock released
  on all fd closes including process death). The new owner MAY now update the lock file's
  diagnostic JSON body.

**Lock file diagnostic body (written under the held flock, loop until complete):**
```json
{
  "pid": <integer>,
  "activation_id": "<UUID from manifest>",
  "fencing_generation": <integer>,
  "timestamp_utc": "<ISO-8601>"
}
```
This body is INFORMATIONAL ONLY. It is NOT the authorization source of truth. Fail-closed
behavior: if, under the held flock, the file is found empty or corrupt from a prior partial
write, the new owner TRUNCATES and rewrites the full body under the held lock. The body is
never used to authorize reclamation decisions — only the flock state (held/not-held) is
authoritative for lock ownership.

**Durable txn record (separate from lock file — closes F5):**
`.factory/migration-state/txn-<activation_uuid>.json` is the PERSISTENT maintenance intent
record. It contains:
- `txn_id`: UUID (same as `activation_id`)
- `activation_id`: from manifest
- `fencing_generation`: monotonic integer, starts at 1, incremented by each recovery-owner
- `state`: `STAGING` | `COMMITTING` | `COMPLETED` | `ABORTED`
- `generation_id`: UUID of the staging generation (`null` until assigned at §Decision 7c step 1)
- `source_sha256`: SHA-256 of ALL source files at quiescence snapshot (used ONLY by step 5
  fingerprint recheck, NOT by step 3b PC1; captured at §Decision 5a drain step 5(a))
- `source_body_row_sha256`: **B2 migration only** — SHA-256 of the per-BC-row table-row
  content from the original `BC-INDEX.md` body in canonical BC-ID sort order, excluding
  §Summary, §Subsystem Shard Manifest, cross-cutting invariants, and non-row lines; captured
  at §Decision 5a drain step 5(c); used by step 3b PC1 for structured row-content equivalence.
  `null` for mechanism-A migrations. These two fields are DISTINCT: `source_sha256` is a
  whole-file fingerprint; `source_body_row_sha256` is a row-content-scoped hash.
- `intent_log_path`: path to the framed intent log (§Decision 7b → ADR-054 Decision 1)
- `canonical_move_plan` (v1.24: RENAMED from `pending_canonical_moves`): the COMPLETE, ordered,
  IMMUTABLE list of `{staging_path, canonical_path}` moves, persisted in full before the `CURRENT.json`
  pointer swap (§Decision 7c step 6) and NEVER modified afterwards — it does not shrink. Per-move
  completion is recorded ONLY by intent-log `DONE` records; what remains = plan minus the targets with a
  valid, txn-bound, INTENT-confirmed `DONE` whose canonical file hashes to its `expected_post_hash`.
  Normative definition, shape, strict decode and B-1/B-2/B-3: **ADR-054 Decision 2 and Decision 3**. (A
  COMMITTING record whose plan is empty, or differs from the set of distinct INTENT `target_canonical`
  values, is `txn_record_malformed`: ADR-054 B-3.) `TXN_RECORD_SCHEMA_VERSION` stays 1 (no record has ever
  been written under the old name by a released build).
- `created_at`, `updated_at`: ISO-8601 timestamps

**Recovery-owner claim protocol (closes F5 exclusive takeover gap; M1 — fencing_generation
is AUDIT-ONLY):**
A recovery process that acquires the flock after detecting a stale owner (txn record exists
with state=STAGING or COMMITTING) MUST claim the transaction by:
1. Reading the current `fencing_generation` from the txn record
2. Incrementing it by 1 (bump = claim; monotonic counter, never decremented)
3. Writing the updated txn record with the new `fencing_generation` and its own PID in the
   lock file body — under the held flock
4. Every subsequent write step by the recovery owner records the current `fencing_generation`
   in its write metadata — **AUDIT-ONLY, for traceability** (crash-restart PID-reuse
   detection, not an enforcement gate). The advisory flock provides actual mutual exclusion:
   only one process can hold `LOCK_EX` on `exclusive.lock` at a time; a stale recovery
   process cannot hold the flock and a live recovery process simultaneously. No resource
   rejects a write based on fencing_generation value alone.

**Ordinary-writer blocking (closes F5 regardless-of-PID-liveness gap):**
The native admission gate (§Decision 5a) blocks ALL mutation tool calls targeting
`.factory/specs/behavioral-contracts/` or `.factory/cycles/` whenever a txn record exists at
`.factory/migration-state/txn-*.json` with state IN (STAGING, COMMITTING). This check is
independent of flock state and PID liveness. A txn record in STAGING or COMMITTING state means
the migration has exclusive rights to those paths, even if the flock-holding process is
temporarily absent (crash recovery scenario). Writers receive E-MAINTENANCE-001.

**Lock release rule:** The flock is released by closing the fd (including on process death via
kernel). RELEASING THE FLOCK NEVER DELETES THE TXN RECORD. A COMMITTING txn record survives
the release and blocks ordinary writers until a recovery owner resolves it to COMPLETED or
ABORTED.

#### 7b — Intent log: framed, checksummed, per-target expected hash (v1.3 — closes F2)

**MOVED to ADR-054 in v1.24 (human-authorized amendment, 2026-10-08).** The intent-log wire format, its
checksum, the reader/writer algorithms, the move-plan completion model, the recovery decision table and the
fault-injection mandate are normative in **ADR-054 "Governed Migration On-Disk Formats and Crash
Recovery"**; they are NOT duplicated here. This heading and its sub-claims remain citable: every existing
"§Decision 7b" citation resolves through the pointer table below. What stays true in this ADR, unchanged:
the intent log is `.factory/migration-state/intent-<generation_uuid>.log`, append-only under the held
`exclusive.lock` flock; INTENT records are durable BEFORE any rename (the WAL boundary: after that barrier
every rename is recoverable); a torn record is ABSENT, never a partial INTENT or DONE.

| §Decision 7b sub-claim (v1.23 text) | Normative home (ADR-054) | v1.24 change |
|---|---|---|
| Record format (`--- INTENT_LOG_RECORD v1 ---`, `key: value`, `--- END_RECORD ---`, `missing`) | Decision 1.2 grammar (ABNF); tokens `INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD` / `MISSING`; fixed order; value rules | Hardened; the code's tokens are adopted |
| `record_checksum` "of all above fields concatenated" | Decision 1.4 (SHA-256 over the exact first nine lines) + 1.5 operator recipe | Byte-exact; operator-verifiable |
| Torn record = absent; "recovery reads from the last valid record" | Decision 1.7 reader (torn tail absent; corruption followed by a valid record fails closed) and 1.9 tail repair | Mid-log corruption now fails closed |
| Durable ordering (WAL boundary), steps 1-5 | Decision 1.9 (barrier, directory sync on create) and Decision 3 (per-move procedure) | Strict `F_FULLFSYNC` on macOS; dir sync on creation |
| Recovery decision table (8 rows) | Decision 3 table (7 rows; adds `AlreadyDone`) | B-1: row 1 MUST append `DONE`; B-2 binding; B-3 plan check |
| Fault-injection test mandate | Decision 5 | Extended fault points; reader byte-offset property test |
| `DONE` record content (implicit) | Decision 3 B-2 | `DONE` bound to txn, copied from and compared with the INTENT |

#### 7c — Atomic publication: single CURRENT.json pointer swap (v1.3 — closes F1, F9)

**Generation directory layout:**
- Staging generation: `.factory/migration-state/gen-<uuid>/` — contains all new target files
  under their relative paths mirroring canonical structure
- Example: `.factory/migration-state/gen-<uuid>/BC-INDEX.md`,
  `.factory/migration-state/gen-<uuid>/shards/BC-INDEX-SS-01.md`, etc.
- This directory is CONTENT-IMMUTABLE after all content is written and synced (step 2):
  file contents are never modified. Files may be moved (renamed) to canonical paths during
  step 7; a moved file is no longer accessible at its `gen-<uuid>/` path (its content is at
  the canonical path instead). The reader's open-with-ENOENT-fallback to canonical (see Reader
  Protocol below) ensures the new content is always accessible at one of the two paths. (C1 fix)

**Pointer file:** `.factory/migration-state/CURRENT.json` — the authoritative generation pointer.
The pointer transition is the single atomic commit point for the entire multi-file migration.

**Full atomic publication sequence:**

1. **Assign generation UUID.** Generate `generation_id = UUIDv4()`. Create generation directory
   `.factory/migration-state/gen-<uuid>/`. Write all new target file content under this directory.
   Apply a platform-appropriate durability barrier to the generation directory (per §Decision 7d)
   before proceeding. **Ordering invariant (v1.13 LOW-1):** `generation_id` MUST be persisted to
   the txn record ONLY AFTER the generation directory is durably created and the fsync barrier is
   applied. This ordering is load-bearing for the §4e `generation_id`-presence partition (v1.12
   F2): a set `generation_id` in the txn record implies the generation directory is durable.
   Update manifest with `staged_generation_id` (atomic overwrite of manifest). Update txn record
   with `generation_id` and `intent_log_path`.
   **Corruption case (v1.13 LOW-1):** If a recovery process finds `generation_id` set in the txn
   record but the generation directory is absent or incomplete (empty or missing expected files),
   this violates the ordering invariant above and indicates filesystem corruption. Treat as
   fail-closed abort: set txn → ABORTED, flip gate → OPEN, exit `EXPIRY_ABORT` with a diagnostic
   log entry identifying the corruption. Do NOT attempt EC-003 census on an absent or incomplete
   generation directory.

2. **Sync all staging files.** For each file in the staging generation: `sync_file(fd)` per
   §Decision 7d platform branches. Then `sync_dir(staging_gen_dir_fd)` per §Decision 7d.

3. **Write intent log.** For each (staging_path → canonical_path) target: append INTENT record
   with `expected_post_hash = sha256(staging_file)` and `expected_pre_state = sha256(canonical_file)
   if exists else MISSING` (recorded truthfully; ADR-054 Decision 1.6). v1.24: the records are written by
   the ONE shared intent-log module in the ADR-054 Decision 1 format, in ONE batch, after tail repair
   (ADR-054 Decision 1.9) and after every `staging_path`/`canonical_path` of the plan has passed the
   ADR-054 value rules (a violation is `INTENT_LOG_VALUE_REJECTED`, a pre-commit abort, nothing staged or
   appended). After all INTENT records: the log barrier (ADR-054 Decision 1.9 step 5; `F_FULLFSYNC` on
   macOS; directory sync when the call created the file).
   **This is the WAL boundary: after this barrier, all renames are recoverable.**

3b. **Pre-pivot content-preservation + census gate (v1.4 — closes H3; BC-1.18.011 PC1/PC2;
    v1.6 C-1 — rewrites PC1 and PC2 to remove tautology).**
    Before the authorization gate, verify the staged generation is correct and complete.
    This step MUST execute before the irreversible CURRENT.json pointer swap at step 6.

    - **Staging-file integrity (distinct from PC1):** For each staged file, verify
      `sha256(staged_file) == expected_post_hash` (from the INTENT record written in step 3).
      This confirms no file was silently modified after the staging sync in step 2. On any
      mismatch → step 3c (abort) with `CONTENT_PRESERVATION_ABORT`.
      **NOTE (v1.6 C-1):** This check is a STAGING INTEGRITY guard, NOT content-preservation
      (PC1). It compares a staged file's hash against its own intent-log entry — a necessary
      but trivially-satisfiable check (sha256(x)==sha256(x) when the intent log was written
      from the same file). It does not verify the split is semantically correct. PC1 and PC2
      below are the real correctness gates.

    - **Content-preservation (PC1 — BC-1.18.011 PC1; v1.7 F1 — structured equivalence):**
      Verify per-BC-row content equivalence using STRUCTURED EXTRACTION, not a whole-file hash.
      A whole-concat SHA against `source_sha256` is UNSATISFIABLE: the staged lean body adds
      the new §Subsystem Shard Manifest section (extra bytes) and content is reordered vs. the
      original, so the concatenation can never equal the original body hash.
      BC-1.18.011 PC1 SoT: "byte-for-byte MODULO the newly-introduced §Subsystem Shard Manifest
      section." The structured equivalence implements this correctly:
      (a) Extract all `BC-X.YY.NNN` table rows from EVERY staged shard file (union across all
          shards); sort by canonical BC-ID (lexicographic on `BC-X.YY.NNN`); concatenate the
          raw row bytes in this sorted order → `staged_row_content`.
      (b) Compute `staged_row_sha256 = SHA-256(staged_row_content)`.
      (c) Compare `staged_row_sha256 == source_body_row_sha256` from the txn record
          (captured at quiescence per §Decision 5a drain step 5(c), using the same extraction
          and sort applied to the original BC-INDEX.md body before the split).
      On any mismatch → step 3c (abort) with `CONTENT_PRESERVATION_ABORT`.
      **Scope of `source_sha256` vs `source_body_row_sha256`:** `source_sha256` covers all
      source file bytes; it is used ONLY by step 5 (fingerprint recheck) and MUST NOT be used
      here. `source_body_row_sha256` is the dedicated per-BC-row hash, stored separately in the
      txn record, used exclusively by PC1.
      **EC-001 remains reachable:** a compensating dup+drop migration that duplicates ID X into
      two shards and drops ID Y (same total count, same sorted row bytes if the duplicate row
      has the same content as the dropped row) may pass PC1 but WILL FAIL PC2 (ID-set
      exactly-one-shard violation), which independently checks that each ID appears in exactly
      one shard. PC1 and PC2 are independently mandatory (BC-1.18.011 Invariant 2).

    - **Independent census (PC2 — BC-1.18.011 PC2):** Perform a fresh enumeration of ALL
      `BC-X.YY.NNN` IDs in the ORIGINAL `BC-INDEX.md` body (the ID set captured at quiescence
      and stored in the txn record). For each ID in this original-census set, verify:
      (a) the ID appears in EXACTLY ONE staged shard file (`gen-<uuid>/shards/BC-INDEX-SS-NN.md`
          or a sub-shard);
      (b) the ID does NOT appear in the staged lean `BC-INDEX.md` retained body (zero
          occurrences, per BC-1.18.010 Invariant 3).
      Verify the union count of shard IDs equals the original-census count exactly.
      **EC-001 MUST abort:** A byte-identical concatenation that nonetheless duplicates one ID
      in two shards while dropping another ID with compensating count — so that total count
      matches but the ID SET does not — is a FAILING migration (BC-1.18.011 Invariant 2: "no BC
      row is ever counted twice or dropped"). Count comparison alone cannot detect EC-001; the
      ID-set check is mandatory. On any violation → step 3c (abort) with `CENSUS_MISMATCH_ABORT`.

    - **Shard boundary invariants (internal capacity check — NOT E-SHD-005; v1.7 F2):** Verify
      each sub-shard file's BC row count is within shard capacity bounds (same cap formula as
      the steady-state shard gate). On violation → step 3c (abort) with `CENSUS_MISMATCH_ABORT`.
      **IMPORTANT:** This check is an INTERNAL INVARIANT CHECK within the migration binary; it
      surfaces as the process exit code `CENSUS_MISMATCH_ABORT` (exit 2). It is NOT `E-SHD-005`.
      `E-SHD-005` is a `HookResult` emitted by the steady-state native admission gate
      (BC-1.18.006 / BC-1.18.010); it is scoped to hook-dispatched writes, not to migration
      binary process exits. The migration binary emits process exit codes only (see §Error Code
      Semantics); it never emits `HookResult` values.

3c. **Gate failure abort (on any step 3b check failure; v1.6 M-3 — exit code aligned):**
    - Delete staging generation directory `gen-<uuid>/`
    - Update txn record: state → ABORTED
    - Flip gate → OPEN (atomic write to gate_state file, under `LOCK_EX`) (H1 fix)
    - Release flock on `exclusive.lock`
    - Exit non-zero with the applicable process exit code:
      - `CONTENT_PRESERVATION_ABORT` (exit 2): staging-file integrity failure (sha256 mismatch
        vs intent-log expected_post_hash) OR PC1 structured-equivalence failure (staged
        per-BC-row SHA-256 in canonical BC-ID order does not match `source_body_row_sha256`
        from txn record, indicating row bytes were altered, dropped, or reordered during split)
      - `CENSUS_MISMATCH_ABORT` (exit 2): PC2 failure (ID-set exactly-one-shard violation,
        including EC-001 dup+drop case) OR shard-boundary capacity violation (internal check;
        NOT `E-SHD-005`, which is a `HookResult` of the steady-state admission gate in
        BC-1.18.006/BC-1.18.010, not a process exit code of the migration binary)
    These are PROCESS EXIT CODES emitted by the migration binary, not HookResult values.
    See §Error Code Semantics for full trigger, severity, and exit behavior.

4. **Authorization gate check (§Decision 4d).** Verify expiry. If expired and state=STAGING:
   ABORT cleanly (update txn → ABORTED, **flip gate → OPEN** (H1 fix), release flock).
   If passes: proceed.

5. **Fingerprint recheck (defense-in-depth).** Re-read all source files, recompute SHA-256.
   Compare against `source_sha256` in txn record (captured at quiescence). If any differ:
   ABORT; delete staging generation; txn record → ABORTED; **flip gate → OPEN** (H1 fix);
   release flock; exit with FINGERPRINT_MISMATCH_ABORT. Require re-activation.

6. **Single atomic pointer swap (the commit point).** Write
   `.factory/migration-state/CURRENT.tmp.json` with content:
   ```json
   {"generation_id": "<uuid>", "status": "committing", "txn_id": "<activation_id>"}
   ```
   `sync_file(CURRENT_tmp_fd)` per §Decision 7d. Then:
   `rename(".factory/migration-state/CURRENT.tmp.json",
           ".factory/migration-state/CURRENT.json")` (atomic on POSIX/APFS).
   `sync_dir(migration_state_dir_fd)` per §Decision 7d.
   Update txn record: state → COMMITTING, with `canonical_move_plan` (v1.24) persisted IN FULL in this
   same write or in the txn-record write immediately preceding the swap (the plan is derived from the
   immutable staged generation; it equals the set of INTENT targets, ADR-054 B-3). `fsync(txn_record_fd)`.
   The plan is immutable from here on (ADR-054 Decision 2).
   **After this rename, the migration is in COMMITTING state. There is no turning back.
   The authorization window is now irrelevant to completion.**

7. **Execute canonical path moves (forward-recoverable via intent log).** Before the first move
   the plan is integrity-checked (ADR-054 B-3: `canonical_move_plan` non-empty and equal to the INTENT
   target set). Then for each `(staging_path → canonical_path)` entry of `canonical_move_plan`, in array
   order (ADR-054 Decision 3 "Per-move procedure"):
   a. `rename(staging_path, canonical_path)` (same filesystem guaranteed)
   b. `sync_dir(parent_dir_of_canonical_path)` per §Decision 7d
   c. Append DONE record to intent log (txn-bound, `expected_post_hash` copied from and compared with the
      INTENT's, `expected_pre_state` copied; ADR-054 B-2) and apply the log barrier. Recovery that finds the
      canonical file already at the post-hash (a crash between b and c) MUST append this DONE (ADR-054 B-1).
   d. **(v1.24: DELETED.)** The txn record is NOT rewritten per move; `canonical_move_plan` is the
      immutable plan and DONE records are the completion record (ADR-054 Decision 2).
   If rename or dir-sync fails: DO NOT abort. Record failure; halt further renames; enter
   recovery state (exit 2 `CANONICAL_MOVE_HALTED`, txn stays COMMITTING). Forward recovery (§Decision 7b →
   ADR-054 Decision 3) re-evaluates EVERY plan entry against the log and the on-disk hashes; already-DONE
   entries are no-ops.

8. **Write COMPLETED record (terminal — closes F9).** After ALL canonical path moves complete
   and verified (each `hash(canonical_path) == expected_post_hash`; v1.24: verified by re-reading the log
   and calling the shared `verify_plan_completion` — every plan target has a txn-bound `DONE` equal to its
   INTENT; `canonical_paths_count` = `len(canonical_move_plan)`, ADR-054 Decision 3 step 5):
   Write `.factory/migration-state/completed.json`:
   ```json
   {"generation_id": "<uuid>", "txn_id": "<activation_id>",
    "completed_at": "<ISO-8601>", "canonical_paths_count": <N>}
   ```
   `sync_file(completed_json_fd)`. Update txn record: state → COMPLETED.
   This file is **PERMANENT. It is NEVER deleted, NEVER archived.** It is the authoritative
   steady-state terminal record. Its presence alone allows any reader or rerun to determine
   that migration is complete without consulting any other file.

9. **Cleanup (optional post-COMPLETED housekeeping).** Remove staging generation directory
   (no longer needed; canonical paths are authoritative). This step is optional; failure to
   clean up does not affect correctness. Cleanup is NOT a migration-state transition.

**Reader protocol (v1.5 C-1 — generation-first / canonical-fallback; replaces v1.4 canonical-first which was a regression for in-place-overwrite targets):**
Readers that access BC-INDEX paths DURING the migration window MUST:
1. Check `.factory/migration-state/completed.json` — if exists: migration complete; canonical
   paths are current and authoritative. Use canonical paths.
2. Check `.factory/migration-state/CURRENT.json` — if `status: committing`: migration is in
   progress. For each required file (v1.11 MED-1 — open-based, ENOENT-fallback, race-free):
   a. **`open(gen-<generation_id>/<file>)`** — open the file at its generation-dir path.
      If the open succeeds: the file has not yet been moved to canonical; read from this fd.
      (File present in gen ⟹ not yet moved to canonical ⟹ new-generation content. Correct
      for both net-new shard files and BC-INDEX.md before its rename in step 7.)
   b. **On ENOENT from step a: `open(canonical/<file>)`** — the file has already been
      renamed to canonical by step 7 progress; canonical now holds new-generation content
      (guaranteed by rename(2) atomicity). Read from this fd.
   **Why open-based, not exists-then-open (v1.11 MED-1):** The prior protocol ("If the file
   EXISTS at its generation-dir path, use it") was TOCTOU-racy: a rename gen→canonical
   occurring between the exists() check and the subsequent open() yields ENOENT to the
   reader. Because files move one-directionally (gen→canonical), open-with-ENOENT-fallback
   is race-free: if open(gen) returns ENOENT, the rename to canonical has already completed;
   the file cannot be absent from both paths simultaneously because rename(2) is atomic.
   This protocol is correct for BOTH net-new shard files (canonical doesn't exist until
   created by step 7) AND in-place-overwrite targets such as BC-INDEX.md (canonical holds
   OLD monolithic body until step 7's specific rename for that file).
   **ENOENT from open(canonical) is impossible** (programmer error if reached): a file
   committed to the staging generation is either at gen-uuid/ (not yet moved) or at
   canonical (already moved by rename(2)). It cannot be absent from both paths.
   **BC-impact handoff (v1.11 MED-1):** BC-1.18.010 §Reader Integration step 2 AND
   BC-1.18.011 Invariant 3 must be updated to mirror this open-with-ENOENT-fallback
   wording: remove "If the file EXISTS" existence-check language; replace with the
   open-with-ENOENT-fallback description above. Applied same-burst by product-owner.
3. If neither file exists: migration not started; use BC-INDEX.md (legacy form).
This protocol is unambiguous in all states including after crash recovery and cleanup.

**Fault-injection reader test mandate (v1.5 — C-1):** A test MUST read BC-INDEX.md at every
point of step-7 progress while CURRENT.json `status=committing` and assert the LEAN (new)
body is always observed — never the monolithic body. This test must cover: (a) before
gen-uuid/BC-INDEX.md is renamed (generation-first returns new lean body from gen-uuid/); (b)
after gen-uuid/BC-INDEX.md is renamed (canonical fallback returns new lean body from canonical
path). The stale monolithic body MUST never be returned at any step-7 progress point.

#### 7d — Platform durability barriers (v1.3 — corrects v1.2 Amendment 6)

**Linux (ext4/xfs):**
- File sync: `fsync(fd)` — persists data + inode metadata
- Directory sync: `fsync(dir_fd)` — persists directory entry; MANDATORY after each `rename()`
  per Pillai et al. OSDI'14 — ensures directory entry survives power loss

**macOS/APFS (darwin-arm64):**
- File sync: `fcntl(fd, F_FULLFSYNC)` — required for power-loss durability on APFS. Apple's
  `fsync(2)` explicitly states: "the drive itself may not physically write the data to the
  platters for quite some time." `F_FULLFSYNC` asks the drive to flush all buffered data to
  permanent storage. **This is the only Apple-documented durability lever for APFS.**
- Directory sync: `fsync(dir_fd)` — Apple's `fsync(2)` and `fcntl(2)` do NOT document that
  fsync on an APFS directory fd provides power-loss durability. This call is BEST-EFFORT on
  macOS: execute it for ordering semantics, but do NOT treat it as a power-loss durability
  guarantee. **Documented residual risk:** under a power-loss scenario on APFS, a rename may
  survive without the directory entry being durable. A darwin-arm64 empirical durability test
  is required before this ADR is considered fully validated on macOS (see §Files to Change).
- `F_BARRIERFSYNC` is NOT a substitute for `F_FULLFSYNC`: it provides ordering but returns
  before earlier data necessarily reaches permanent media.
- **APFS dir-fsync — RATIFICATION PREREQUISITE (v1.7 F8):** The darwin-arm64 empirical
  durability test (`crates/factory-dispatcher/tests/darwin_arm64_durability_test.rs`) is a
  RATIFICATION PREREQUISITE for macOS safety. This ADR MUST NOT be ratified as safe on macOS
  until this test completes and confirms whether `fsync(dir_fd)` on APFS provides any
  power-loss durability for rename directory entries. Until the test runs: the statement "this
  ADR is fully validated on macOS" is NOT VALID. The residual risk (rename directory entry may
  not survive a power-loss event on APFS) is documented in §Decision 11's POLICY 22 sign-off
  block, where the human ratifier must explicitly acknowledge it.

**Intent-log append and tail-truncation barrier (v1.24 — closes research finding A-5).** The WAL boundary
(§Decision 7b → ADR-054 Decision 1.9) is the precondition for every canonical-path mutation and MUST meet
the same durability standard as those mutations. Every intent-log append, and the tail-repair truncation that
precedes the first append, therefore applies `sync_file_durable` (below) to the log handle — on macOS
`F_FULLFSYNC`, with NO silent fall-back to plain `fsync` (a failing barrier is an `Io` error and the WAL
boundary is NOT reached). When the append call CREATED the log file, the parent directory
(`.factory/migration-state/`) is then synced with the same directory primitive this section assigns to
rename durability (`sync_dir_durable`; on macOS the strict directory variant adopted by D-1232-OBL-2(a))
BEFORE the boundary counts as reached. The `Fs` seam exposes `append_durable` (returns whether the file was
created) and `truncate_durable` so OBL-1 fault injection can fail each step.

**Implementation:**
```rust
#[cfg(target_os = "macos")]
fn sync_file_durable(fd: RawFd) -> io::Result<()> {
    // F_FULLFSYNC required; plain fsync is insufficient for APFS power-loss durability
    if unsafe { libc::fcntl(fd, libc::F_FULLFSYNC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn sync_file_durable(fd: RawFd) -> io::Result<()> {
    if unsafe { libc::fsync(fd) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// Both platforms: call but treat as best-effort on macOS/APFS
fn sync_dir_best_effort(dir_fd: RawFd) -> io::Result<()> {
    let _ = unsafe { libc::fsync(dir_fd) };
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn sync_dir_durable(dir_fd: RawFd) -> io::Result<()> {
    // Linux: mandatory for rename durability per Pillai et al. OSDI'14
    if unsafe { libc::fsync(dir_fd) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
```

**v1.2 Amendment 6 correction:** v1.2 Amendment 6 to BC-1.18.011 mandated "dir-fsync is
mandatory, not best-effort." This was correct for Linux but incorrect for macOS/APFS. The
BC-1.18.011 Amendment 6 replacement text is specified in §Downstream to Product-Owner.

#### 7e — Shared migration-state namespace: what is shared, what is per-migration (v1.18 — closes D4)

Both migrations operate in `.factory/migration-state/`. Resolved ownership:

| Artifact | Scope | Rule |
|---|---|---|
| `exclusive.lock`, `gate-state.json`, `reservations/`, `txn-<activation_uuid>.json`, `intent-<generation_uuid>.log`, `gen-<generation_uuid>/` | **shared** (names are unique by UUID or are single global interlocks) | One gate, one lock, one reservation dir. At most ONE live txn across both migrations. |
| Terminal record | **per-migration** | `migrate-bc-index` → `completed.json` (UNCHANGED: BC-1.18.010 §Reader Integration step 1 and merged B2 code `detect_migration_read_state` consume this exact name as "the BC-INDEX migration is complete"). `backfill-append-logs` → `completed-backfill-append-logs.json`. Schema is §7c step 8's for both: `{generation_id, txn_id, completed_at, canonical_paths_count}` (the S-25.06 draft field `file_count` is renamed `canonical_paths_count`; the spec wins). |
| Pointer | **per-migration** | `migrate-bc-index` → `CURRENT.json` (UNCHANGED, consumed by the B2 reader protocol). `backfill-append-logs` → `CURRENT-backfill-append-logs.json`. No reader other than the owning binary's recovery consumes the mechanism-A pointer. |
| Txn record discriminator | **new field** | `migration_id: "migrate-bc-index" \| "backfill-append-logs"` added to the txn-record schema (§7a). Absent ⇒ `"migrate-bc-index"` (serde default; records written by the merged B2 code before this field existed remain valid). |

**Why (D4):** with a single `completed.json`/`CURRENT.json`, whichever migration finishes first
(a) makes the other's idempotency check return `ALREADY_MIGRATED` without having run, or fail on
a schema mismatch; (b) for the A→B2 direction makes `detect_migration_read_state` report the
BC-INDEX migration COMPLETE so BC-INDEX readers switch to shard paths that do not exist; and (c)
lets the merged B2 `completed.json` short-circuit rewrite the OTHER migration's live txn to
COMPLETED and open the gate mid-migration. Neither migration has ever been activated in this
repository (no txn record, terminal record or pointer exists; the empty
`migration-state/reservations/` namespace may exist from admission), so the A-side rename carries no data
migration.

**Cross-migration recovery refusal:** each coordinator recovers, resumes, finalizes or aborts
ONLY a live txn whose `migration_id` equals its own. If the single live txn belongs to the other
migration, the invoking binary refuses (exit 2 `FOREIGN_MIGRATION_REFUSED` — v1.23 item 9 retires the
former "`LockContention`-class" label; no mutation) — it MUST
NOT run `recover()` over a foreign record. The admission gate is migration-agnostic (any live
txn blocks the union). **Label discipline (v1.18 follow-up):** the foreign-txn refusal is NOT
`COMPLETION_RECORD_MISMATCH_ABORT` — that code is reserved for "the invoking migration's OWN terminal
record cannot be proven to describe the live txn of that SAME migration" (§4e, §5a Branch C).
A foreign live txn never enters Branch C / §5c step 0.5 at all (the terminal record consulted is
the LIVE TXN's own migration's, selected by `migration_id`; B2's `completed.json` is irrelevant to a
`backfill-append-logs` txn and vice versa): the binary refuses with exit 2 `FOREIGN_MIGRATION_REFUSED`
and the PreToolUse path blocks with the ordinary `E-MAINTENANCE-001` (live txn), with no
mismatch reason and no mismatch-suffix. Shared reconciliation (§5a Branches A/B/C, §5c Branch 2) dispatches on
`migration_id` to select the terminal-record path, the canonical-path set and N.

**Definition of "foreign" in the shared admission core (v1.20 — closes F-006).** The shared core
(§5a step 3.5 Branches A/B/C on the dispatcher's PreToolUse path, and the §5c Branch-2 hook-side
reconciliation) serves BOTH migrations and has NO "evaluating migration"; the v1.18 phrase "the
live txn is not the evaluating migration's own" is meaningful only in a migration BINARY. Rulings:
(1) Let `K = {"migrate-bc-index", "backfill-append-logs"}` be the `migration_id` values THIS build
recognises (absent field ⇒ `"migrate-bc-index"`). (2) In the shared core, a live txn is **foreign
iff its `migration_id ∉ K`** (an id written by a newer or alien build). A foreign txn is NEVER
finalized, NEVER aborted — including Branch B's null-generation discard, because a record this
build cannot interpret is not this build's to discard — and no Branch A/B/C repair runs; the gate
keeps blocking the union with the ordinary `E-MAINTENANCE-001` (path-family `<scope>`, NO mismatch
suffix); the raw id (truncated to 64 chars, control characters escaped) goes only to the
`migration.admission_failed` InternalLog diagnostic (v1.21). A `migration_id` that is not a JSON string is a malformed record
(admission error, fail-closed `E-MAINTENANCE-002`), not "foreign". (3) A live txn whose
`migration_id ∈ K` is, for the core, simply ITS OWN: the core selects the terminal-record path, the
canonical-path set and N by that `migration_id` (this table) and applies the five-row decision
table; the OTHER known migration's terminal record is never consulted. Hence a live
`backfill-append-logs` txn beside a present B2 `completed.json` and NO
`completed-backfill-append-logs.json` yields row 3 (`NoOp`) ⇒ Branch B only if STAGING with
`generation_id=null`, otherwise the ordinary plain live-txn block; and a live `migrate-bc-index`
txn symmetrically. Neither is `RefuseForeignMigration`, neither is a mismatch, and neither is
finalized on the strength of the other's record (the pre-existing "never finalize a txn on the
strength of the other migration's record" invariant is preserved by selection-by-`migration_id`,
not by refusal). (4) The pure core input `TerminalReconcileInputs.txn_is_own_migration` is RENAMED
`txn_migration_known: bool` (true iff `migration_id ∈ K`); decision row 2 reads "live txn's
`migration_id ∉ K` ⇒ `RefuseForeignMigration` (plain `E-MAINTENANCE-001`, precedence over every
record check)"; rows 1, 3, 4, 5 and the five-row shape are unchanged:
| # | Condition (first match wins) | Decision |
|---|---|---|
| 1 | lock not acquired, or no live txn | `NoOp` |
| 2 | `migration_id ∉ K` | `RefuseForeignMigration` |
| 3 | known, own terminal record ABSENT (STAGING and COMMITTING alike) | `NoOp` |
| 4 | record present, COMMITTING, every verification check passes | `FinalizeThenOpenGate` |
| 5 | record present otherwise (STAGING any generation, or any failed check) | `FailClosedMismatch` |
(5) The cross-migration refusal above ("each coordinator recovers … ONLY a live txn whose
`migration_id` equals its own", exit 2 `FOREIGN_MIGRATION_REFUSED`) is a property of the migration
BINARIES' recovery path, where "own" is the invoking subcommand; it is unchanged and is the only
place "own vs other known migration" is a refusal. A formally-unreachable-for-two-known-ids
`RefuseForeignMigration` in the core is retained (not removed) because the totality of the pure
core over `txn_migration_known=false` is what makes forward-compatible ids fail closed.

**Note on `gate-state.json`:** the physical file name is `gate-state.json` (merged B2 code);
every "gate-state" reference in this ADR and in BC-1.18.011/013 denotes that file.

### Decision 8 — Policy exception: single authoritative allowlist (v1.3 REDESIGNED — closes F10)

This ADR declares a **narrowly authorized exception** to CLAUDE.md's TD-FACTORY-HOOK-BYPASS-001
governing rule ("Use Edit/Write tools ONLY for `.factory/` mutations").

#### Skipped-control inventory (v1.5 H-1: inlined from v1.2; unchanged in substance)

| Skipped control | What it actually enforces | Equivalent enforcement in migration binary or ratified waiver |
|-----------------|---------------------------|---------------------------------------------------------------|
| `brownfield-discipline` PreToolUse (Edit\|Write\|MultiEdit) | **Protects `.reference/` from governed writes** (script lines 4-9 and 34-38 check write targets against the `.reference/` path prefix) | Migration binary does not write to `.reference/`; all targets are explicitly enumerated in §Decision 8 allowed-paths list. **No `.reference/` writes possible by construction.** |
| `factory-branch-guard` PreToolUse (Edit\|Write\|MultiEdit) | **Checks that the active worktree is NOT the factory-artifacts branch before permitting a mutation** (script lines 67-82 verify worktree identity and branch name) | Migration binary operates on the main working tree, not the factory-artifacts worktree. The migration targets (shard files, BC-INDEX.md) are in the main worktree. This guard does not apply to the migration binary's execution context. **EXPLICIT WAIVER (POLICY 22 authorized):** migration is always run in the main worktree; the guard's branch-check intent is satisfied because main-worktree BC mutations are exactly what this migration is authorized to perform. |
| `validate-factory-path-staging` PreToolUse (Bash) | Validates `.factory/` writes via Bash use proper channels | Guard amended (§Decision 5b) with full-command pre-shell classifier and manifest content validation (§5c); the amendment IS the equivalent enforcement |
| `validate-factory-path-staged` PostToolUse (Bash) | **IS registered for PostToolUse Bash** (hooks-registry.toml entry for PostToolUse Bash); verifies `.factory/` mutations from Bash are properly staged for factory-artifacts | **MUST be amended** by devops-engineer to recognize the governed migration subcommands (same absolute-path-pinned forms as §Decision 3) and pass them through without requiring Edit/Write staging semantics. Post-migration, devops-engineer performs the factory-artifacts commit manually as the audit record (§Decision 6). Guard amendment is a cluster-5 activation deliverable. |
| `validate-count-propagation.sh` PostToolUse behavior | Verifies count propagation across BC-INDEX | Migration binary's independent census cross-check against `total_bcs` oracle provides equivalent verification before any write; count propagation is a post-migration steady-state concern |
| POL-3 "NEVER use Python/sed/echo bypass" | Prevents unstructured, unvalidated `.factory/` writes | **EXPLICIT WAIVER (POLICY 22 authorized):** The migration binary is VSDD-authored, TDD-covered (VP-132/VP-133/VP-134), adversarially-reviewed. It implements the full BC-1.18.008/BC-1.18.011 specification. This waiver is narrowly scoped to the two governed migration subcommands and expires when the migration completes. |

#### Single authoritative allowed-write-targets list (v1.4 — closes F10, C2)

The exception covers writes to the following paths ONLY. This list is the SINGLE source of
truth for both the binary's containment checks AND the CLAUDE.md amendment text below:

```
# B2 migration targets (BC-INDEX body-split) — v1.4 NORMALIZED (closes C2)
.factory/specs/behavioral-contracts/BC-INDEX.md
.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.md
.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.a.md
.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.b.md
.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.c.md
  (and any additional single-lowercase-letter suffix in [a-z]; validate_write_target()
   accepts the pattern BC-INDEX-SS-<NN>.<letter>.md for any letter in [a-z])
.factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml
.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.manifest.toml
  (generic pattern; covers BC-INDEX-SS-05.manifest.toml, BC-INDEX-SS-06.manifest.toml,
   and any other subsystem sub-manifest)

# A migration targets (four exact config-specified append-log files — v1.6 M-4)
.factory/cycles/v1.0-brownfield-backfill/decision-log.md
.factory/cycles/v1.0-brownfield-backfill/burst-log.md
.factory/cycles/v1.0-brownfield-backfill/lessons.md
.factory/cycles/v1.0-brownfield-backfill/session-checkpoints.md
  (validate_write_target() MUST verify the exact path matches one of these four paths, not
   merely the `.factory/cycles/` prefix; the wildcard form is NOT an accepted pattern)

# Migration operational state (both migrations)
.factory/migration-state/
.factory/migration-state/gen-<uuid>/

# Activation and audit (both migrations)
.factory/activation/
.factory/migration-audit/
```

Where `<NN>` is a two-digit subsystem number, `<letter>` is any single lowercase ASCII letter
in `[a-z]`, and `<uuid>` is the runtime generation UUID.

**Sub-shard naming rationale (v1.4 — closes C2):** The v1.3 allowlist used hyphen-uppercase
suffixes (`-A.md`, `-B.md`) which diverged from the canonical naming established in ADR-051
§Decision 7 (`BC-INDEX-SS-NN.a.md`, `.b.md`, `.c.md`) and referenced in BC-1.18.011 EC-004.
`validate_write_target()` MUST implement the pattern check as a programmatic guard, not a
hardcoded list of specific letters, to support SS-05's three-way sub-split (`.a`, `.b`, `.c`)
and any future subsystem sub-sharding without requiring an allowlist update.

**Ratification-time test mandate (v1.4 — closes C2):** The cluster-5 TDD test suite MUST
include a test asserting that every path referenced in BC-1.18.010 and BC-1.18.011 is accepted
by `validate_write_target()`. At minimum:
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.a.md` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.b.md` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.c.md` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-06.a.md` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.manifest.toml` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-06.manifest.toml` → ACCEPTED
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05-A.md` → REJECTED (hyphen-upper format)
- `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05-B.md` → REJECTED (hyphen-upper format)
- `.factory/specs/behavioral-contracts/BC-INDEX.md` → ACCEPTED (v1.5 L-2: the in-place overwrite target itself must pass)
- `.factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml` → ACCEPTED (v1.5 L-2: top-level shard manifest)
This test MUST pass before cluster-5 TDD proceeds to implementation.

**Containment check (required before every write):** Before any write to any target path, the
migration binary MUST verify the resolved canonical path prefix matches one of the above entries.
Any target that does not match MUST be rejected with a non-zero exit regardless of manifest state.
This check is implemented in a `validate_write_target(path: &Path) -> Result<(), MigrationError>`
function called by every path that produces a filesystem write.

#### CLAUDE.md amendment text (v1.4 — generated from the same allowlist above)

The human MUST apply the following amendment to `CLAUDE.md` as part of POLICY 22 ratification.

**Location:** `## Conventions (Code-Level)` section, `### Forbidden patterns` table, the row
for TD-FACTORY-HOOK-BYPASS-001 P0.

**Amendment (generated from the authoritative allowlist above — no scope expansion):**

```
ADR-052 EXCEPTION (POLICY 22 ratified): The governed one-time shard migration binary
(`{project-root}/target/release/factory-dispatcher migrate-bc-index` and
`backfill-append-logs`) may write to the following paths ONLY:
  `.factory/specs/behavioral-contracts/BC-INDEX.md`
  `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.md`
  `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.a.md`
  `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.b.md`
  `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.c.md`
  (and any additional dot-lowercase letter suffix [a-z] — validated by validate_write_target())
  `.factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml`
  `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-<NN>.manifest.toml`
  (generic; covers all subsystem sub-manifests including SS-05 and SS-06)
  `.factory/cycles/v1.0-brownfield-backfill/decision-log.md`
  `.factory/cycles/v1.0-brownfield-backfill/burst-log.md`
  `.factory/cycles/v1.0-brownfield-backfill/lessons.md`
  `.factory/cycles/v1.0-brownfield-backfill/session-checkpoints.md`
  (these four are the config-specified append-log targets for mechanism A; validate_write_target()
   verifies the exact path matches one of the above four, not merely the cycle prefix)
  `.factory/migration-state/` (flock inode, txn record, intent log, CURRENT.json, completed.json,
                                gate-state, reservation dir, staging generation)
  `.factory/activation/`
  `.factory/migration-audit/`

PRECONDITIONS (all must hold before any canonical-path mutation):
(a) Invoked via Bash tool with one-time interactive human approval at F4 activation boundary.
(b) Armed-activation manifest present, validated under exclusion: repo root SHA, expected_total_bcs,
    three-way ARCH-INDEX parity (config == manifest == live), activation_id correlation (ADR-052
    §Decision 4); OR completion-only recovery manifest per ADR-052 §Decision 4d.
(c) Migration binary invoked at absolute trusted path with executable digest verified via
    fd-binding on Linux (execveat AT_EMPTY_PATH) or freeze-build protocol on macOS
    (ADR-052 §Decision 11); closed argument grammar per ADR-052 §Decision 3.
(d) All 4 dispatcher guard amendments deployed per ADR-052 §Decision 5b.
(e) Native OPEN/DRAINING admission gate in `executor.rs` deployed (ADR-052 §Decision 5a);
    txn record state (STAGING/COMMITTING) blocks ordinary writers regardless of PID liveness.

POST-SUCCESS OBLIGATIONS (after completed.json written):
(f) Durable factory-artifacts commit records census stdout, activation manifest, completed.json,
    and binary version as NIST AU-9 audit record (ADR-052 §Decision 6).
(g) State-manager archives activation manifest to `.factory/migration-audit/`.

All other `.factory/` writes by agents remain subject to the Edit/Write-only constraint.
```

### Decision 9 — Resolving the S-25.06 Rule 7 contradiction

S-25.06 Rule 7's original text is WITHDRAWN and replaced with:
> "The activation step is executed via the `Bash` tool with one-time interactive human approval
> at F4 activation (no standing settings.json allowlist). The migration binary is invoked at its
> absolute trusted path: `{project-root}/target/release/factory-dispatcher backfill-append-logs`
> (or `migrate-bc-index`). The invocation is NOT an Edit/Write tool call. It is a Bash execution
> of the native migration binary under ADR-052 §Decision 3's closed argument grammar, activated
> under ADR-052 §Decision 4's armed-activation manifest, producing a census report captured per
> ADR-052 §Decision 6."
Story-writer updates S-25.06 Rule 7 text accordingly.

### Decision 10 — BC-1.18.010 Invariant 2: ARCH-INDEX mapping three-way revision binding

Three-way activation-time parity check: `config.arch_index_sha` ==
`manifest.approved_arch_index_sha` == live ARCH-INDEX SHA. All three must be equal; fails CLOSED
if any pair diverges. CI parity test (`arch_index_parity`) and stale-installed-config test both
required. The mapping is read from deserialized config only, never from ARCH-INDEX at runtime.

### Decision 11 — Executable verify-to-execute binding (NEW — closes F11)

**Problem:** The v1.2 design hashes `target/release/factory-dispatcher` at a path and stores a
digest, then executes that same path via a shell later. This is CWE-367 TOCTOU: between the hash
check and exec, a concurrent `cargo build` can replace the binary with legitimately different bytes.

**Fix: open once → hash through the open fd → execute the same fd (never return to pathname).**

#### Linux path (fully closes TOCTOU)

```rust
let fd = open("{project-root}/target/release/factory-dispatcher", O_RDONLY | O_CLOEXEC);
// Hash through the open fd (read() loop); compare against stored digest
let computed = sha256_read_through_fd(fd)?;
assert_eq!(computed, stored_digest, "binary integrity check failed");
// Execute through the SAME fd — no re-resolution of pathname possible
// glibc >= 2.27: execveat(fd, "", argv, envp, AT_EMPTY_PATH)
// Fallback on older glibc: fexecve(fd, argv, envp) via /proc/self/fd/<n>
// DEPENDENCY: /proc must be mounted. Fails closed if /proc unavailable (container, chroot).
// In environments without /proc, the fallback errors; only execveat AT_EMPTY_PATH is available.
execveat(fd, CStr::from_bytes_with_nul(b"\0")?, argv, envp, AT_EMPTY_PATH)?;
```

`AT_EMPTY_PATH` on Linux 3.19+ (glibc 2.34 wrapper) executes the file referred to by `fd`
without re-resolving a pathname. The binary being executed is byte-for-byte identical to the
one that was hashed.

**O_PATH caveat:** Do NOT open with `O_PATH` for this purpose — `O_PATH` fds cannot be `read()`
through, so hashing is not possible. Use `O_RDONLY`.

#### macOS/darwin-arm64 path (primary operator platform — documented residual TOCTOU — NEEDS HUMAN SIGN-OFF at POLICY 22)

`fexecve` and `execveat AT_EMPTY_PATH` are **unavailable on macOS** (darwin-arm64). Apple's
documented exec interfaces (`execl`, `execle`, `execlp`, `execv`, `execvp`, `execvP`, `execve`)
are all pathname-based. No Apple-documented fd-binding exec primitive exists. The strongest
available evidence is Apple's exec man-page set exposing only pathname-based APIs; corroborating
non-Apple SDK/source reports confirm the missing symbol. This is not backed by an explicit
Apple negative statement, but confidence is HIGH.

**Architect decision for macOS/darwin-arm64 (PRIMARY OPERATOR PLATFORM — v1.4 M5 reframe):
freeze build under maintenance lock + programmatic mtime guard + accept documented residual
TOCTOU window (requires POLICY 22 human acknowledgment).**

macOS/darwin-arm64 is the primary operator runtime. The residual exec-TOCTOU window is the
DEFAULT runtime condition for this product, not a secondary concern. This section documents
the production-grade mitigations and the residual risk that requires explicit human sign-off.

Rationale:
1. macOS/darwin-arm64 is the primary operator platform. The risk-framing of the residual TOCTOU
   window MUST reflect that this is the default runtime, not a fallback.
2. The maintenance lock (advisory flock) is held before the digest check. No agent edit/write
   paths can replace the binary (they target `.factory/`, not `target/release/`).
3. The realistic threat is a concurrent `cargo build` replacing the binary.
4. **Programmatic mtime guard (v1.4 M5; v1.5 H-2 corrected: re-stat immediately before execve):**
   Record `mtime(target/release/factory-dispatcher)` immediately after the digest check.
   IMMEDIATELY BEFORE the `execve` pathname call, re-stat the binary's mtime and compare
   against the recorded value. If mtime changed between the initial digest check and the
   pre-exec re-stat: abort with `E-BINARY-INTEGRITY-FAILURE` BEFORE the exec call. Do NOT
   re-hash through a fresh fd on this path (to minimize the window between re-stat and execve);
   mtime change is sufficient signal to abort.
   **Correction from v1.4:** the previous wording said "after exec returns" — a successful
   `execve` NEVER returns (it replaces the calling process), so the post-exec check fired only
   on exec failure, leaving the dangerous successful-substitution-then-exec case completely
   undetected. Moving the re-stat to immediately before the `execve` syscall closes this gap:
   the detectable window shrinks from (digest-check → exec) to (pre-exec-re-stat → execve),
   which is sub-CPU-instruction under normal OS scheduling.
   This does NOT cryptographically close the TOCTOU window, but it narrows the undetectable
   surface to near-zero under a quiescent system with no concurrent cargo build.
5. **Hard pre-flight checklist item (v1.4 M5):** The activation procedure MUST require as a
   HARD PREREQUISITE (not a soft recommendation) before running the migration: "Verify no
   cargo build process is running: `pgrep -x cargo-build` returns empty." This item MUST
   appear in the human-visible activation checklist at the F4 POLICY 22 gate.
6. Protected staging (copy to `/tmp/` + `UF_IMMUTABLE`) does not provide meaningful security:
   `UF_IMMUTABLE` is owner-changeable, `/tmp/` is owner-writable, and the staging step has
   its own TOCTOU window.
7. The residual window is intra-process (sub-millisecond under quiescent system) with no
   concurrent build running and the mtime guard active.

**macOS implementation (v1.6 M-2 — mtime re-stat added immediately before exec):**
```rust
#[cfg(target_os = "macos")]
fn verify_and_exec_binary(binary_path: &Path, ...) -> Result<(), ExecError> {
    let fd = File::open(binary_path)?;
    let computed = sha256_read_through_fd(&fd)?;
    if computed != stored_digest {
        return Err(ExecError::DigestMismatch);
    }
    // Record mtime immediately after digest check for pre-exec comparison.
    let mtime_after_digest = stat_mtime(binary_path)?;

    // No fd-binding exec available on macOS; execute by pathname under held maintenance lock
    // DOCUMENTED RESIDUAL TOCTOU: a concurrent cargo build between hash and exec could
    // substitute different bytes. Mitigation: no concurrent build during F4 activation.
    // This residual risk requires human acknowledgment at ratification (see §Decision 11).

    // Programmatic mtime guard: re-stat IMMEDIATELY BEFORE execve syscall.
    // If mtime changed between digest check and this re-stat, a substitution may have
    // occurred — abort BEFORE executing. This narrows the undetectable window from
    // (digest-check → exec) to (pre-exec-re-stat → execve), which is sub-instruction
    // under a quiescent system. Does NOT close the TOCTOU window cryptographically.
    let mtime_pre_exec = stat_mtime(binary_path)?;
    if mtime_pre_exec != mtime_after_digest {
        return Err(ExecError::BinaryIntegrityFailure);
    }

    exec_by_pathname(binary_path, ...)?
}
```

**Test:** A test MUST verify that digest mismatch (binary replaced between open and exec check)
causes the migration to abort before exec with `E-BINARY-INTEGRITY-FAILURE` rather than silently
executing the replacement. On Linux, this test also verifies the execveat AT_EMPTY_PATH path via
a test double that intercepts the execveat call and confirms the fd argument matches the opened fd.

**Human sign-off required (v1.5 H-2 — corrected mtime-guard semantics; v1.7 F8 — APFS
durability prerequisite added):** POLICY 22 ratification requires explicit acknowledgment of
TWO items:

**Sign-off item 1 — macOS exec-TOCTOU residual window (v1.5 H-2):** The macOS/darwin-arm64
(primary operator platform) residual TOCTOU window is the sub-instruction gap between the
pre-exec mtime re-stat and the `execve` pathname call — narrowed from the v1.4 framing of
(digest-check → exec). The corrected mtime guard (§Decision 11 rationale item 4) detects
substitution that occurs BEFORE the pre-exec re-stat; it does NOT detect substitution in the
remaining sub-instruction window between re-stat and execve. The migration activation procedure
MUST include as a HARD PREREQUISITE checklist item: "Verify no cargo build process is running
(`pgrep -x cargo-build` returns empty)."

**Sign-off item 2 — APFS dir-fsync power-loss durability unverified (v1.7 F8):** The
darwin-arm64 empirical durability test (`crates/factory-dispatcher/tests/darwin_arm64_durability_test.rs`)
is a RATIFICATION PREREQUISITE. Before ratifying this ADR as safe on macOS, the human MUST
do ONE of the following:
- (a) **Confirm the test has been run** on darwin-arm64 and the results have been reviewed.
  If the test shows `fsync(dir_fd)` on APFS provides power-loss durability for rename directory
  entries, record the test result in the D-1232 ratification entry.
- (b) **Explicitly acknowledge unverified durability:** If ratifying before the test runs, the
  ratification entry MUST state: "macOS/APFS power-loss durability for rename directory entries
  (from `fsync(dir_fd)`) is NOT verified on darwin-arm64. Under a power-loss scenario, a rename
  may survive in memory but the directory entry update may not persist. Operator accepts this
  residual risk until the empirical test runs." Migration MAY proceed on macOS with this
  acknowledgment, with the understanding that the power-loss durability guarantee is weaker than
  on Linux.

Both acknowledgments MUST be recorded in the D-1232 entry that ratifies ADR-052.

### Decision 12 — OBL-1 verification architecture: `Fs` trait seam + two-pronged Kani/fault-injection discharge (NEW — discharges [D-1232-OBL-1])

**v1.18 re-baseline (supersedes the "discharged / 7-of-7 PROVED / 30-of-30 green" closure below
for the v1.18 DELTA only).** The PROVED/green results below were obtained against the pre-v1.18
merged B2 model and code, which deviated from this ADR in four ways now canonically labelled
**B2-1** (`reconcile_stale_admission_gate` never called — harness 4's "drain/reservation wiring now
reachable in the implementation" was true of the model, false of the production call graph),
**B2-2** (unverified `completed.json` short-circuit — outside harness 1's `recover()` domain, so
"`recover()` totality" never covered it), **B2-3** (120 s TTL + mtime staleness vs. 3,600 s / 1,800 s
floor + `created_at`) and **B2-4** (check-then-reserve race vs. reserve-then-verify). Consequences:
(i) [D-1232-OBL-1] is re-opened for the delta exactly as §Verification Strategy's v1.18 paragraph
states; (ii) harnesses 1, 3, 4, 6 must be re-run/extended for B2 against the shared core (new
terminal-reconciliation inputs to the single shared pure decision core
`decide_terminal_record_reconciliation` incl. the `migration_id` foreign-refusal outcome, the
`FinalizeFromTerminalRecord` event, the reserve-then-verify/TTL/lock model with INV-GATE-TXN
UNSAT-for-violation and `kani::cover!` non-vacuity re-confirmed) — mirrored for mechanism-A by
VP-146 v1.2; (iii) the 30 fault-injection tests did not cover the crash point "terminal record
durable ∧ txn COMMITTING" via the PreToolUse route, so the claim "the txn record reaches a terminal
state" holds only for the binary-recovery route until VP-133 facet 8 / VP-143 facet (b) run; (iv)
B2-1/B2-3 are call-graph/constant defects invisible to Kani by construction — VP-133 facets 6-7
(real-binary black-box) carry them. **Catalog anchor (v1.18):** the seven `proof_obl1_*` harnesses
are now anchored by **VP-147** (kani-proof, BC-1.18.011; mirror of VP-146), which records 7/7
PROVED against the pre-v1.18 model, the exact v1.11 clauses the suite does NOT yet prove, and the
owed extension (`EXPECTED_PROOFS` raised in the same commit).

**Status: REFINES/IMPLEMENTS — did not reopen or amend §Decision 4e/5a/7c at v1.17 (v1.18 does, see above).** This
Decision documents how [D-1232-OBL-1] — the implementation-phase Kani + fault-injection
obligation that the §Verification Strategy subsection above (D-386 Option C accept-at-floor
ratification) already mandated as a condition of POLICY 22 ratification, and that explicitly
blocks cluster-5 TDD *completion* — is discharged in code. It adds documentation of a
verification harness architecture; it is not a change to the frozen drain/gate/txn state
machine, so it does not trigger the "formal-finding exception or superseding ADR" bar in
§Status. The governance analysis establishing this (clause-by-clause cross-check against
§Decision 4e/5a/7c, finding zero design gaps — only code-vs-spec deviations, principally the
WAL-ordering defect closed by §Decision 7b conformance below) is on file as
`obl1-recover-refactor-design.md` (architect work product, cluster-5 F4 scope).

**Problem: the ratified state machine's crash-safety and liveness properties were asserted
by 10 prose adversary passes but not mechanically proven, and its I/O effects were not
independently fault-injectable.** Kani cannot model real filesystem I/O; a decision function
that calls `std::fs::rename`/`fsync`/`F_FULLFSYNC` directly is neither Kani-provable nor
independently crash-testable without conflating the abstract decision logic with the real
OS's actual (and platform-varying) durability semantics.

**Fix — the `Fs` trait seam.** `crates/factory-dispatcher/src/shard_manager/migration_fs.rs`
defines a trait of 7 core filesystem operations (`write_temp`, `fsync_file`, `rename`,
`fsync_dir`, `read`, `exists`, `pointer_swap`, `remove` — the distinct `pointer_swap` op is
kept separate from `rename` solely so a proof harness can assert the commit predicate fires
on exactly that call, never on a canonical-path-move rename) plus a separate `append` op (for
intent-log record writes). Every I/O effect the migration's crash-recovery decision logic
performs — `recover()`, `execute_canonical_path_moves`, and the §Decision 7c atomic-publication
sequence — goes through this seam; no direct `std::fs::*` call remains inline in the decision
functions. This single abstraction boundary is what makes the same decision logic simultaneously:
- **Kani-modelable:** an in-memory, two-namespace (`live`/`durable`) abstract implementation
  models POSIX crash semantics (`crash()` collapses `live` into `durable`, i.e. "anything
  fsynced survives; anything only in `live` is lost") without requiring Kani to reason about
  real syscalls.
- **Fault-injectable:** a `fail`-crate-instrumented implementation (feature-gated behind
  `factory-dispatcher/failpoints`, a no-op under the default feature set) inserts `abort()`
  crash points at real `rename`/`F_FULLFSYNC`/`fsync` syscall boundaries in a child process,
  exercising the actual OS.

`StdFs` (the production implementation, same file) delegates `write_temp`/`fsync_file`/
`fsync_dir` to the existing `last_amended_migrate` `F_FULLFSYNC`-on-macOS /
`fsync`-elsewhere durable-write primitives (§Decision 7d), and `rename`/`pointer_swap`/`read`/
`exists`/`remove` to the corresponding unchanged `std::fs`/`Path` calls. This is a pure
extraction with **zero behavioral change to production I/O** — the same underlying syscalls
run in the same order (once the §Decision 7b conformance fix below is also applied); the seam
exists so one decision-logic implementation can run against three call-through targets
(production, Kani model, fault-injection double) rather than duplicating the logic three times.

**The two-pronged verification split.** Kani and fault-injection are deliberately
complementary, not redundant — each proves what the other structurally cannot:

- **Kani proves the finite/sequential/pure core** — 7 `#[kani::proof]` harnesses in
  `crates/factory-dispatcher/src/shard_manager/obl1_kani_proofs.rs`, all PROVED (executed via
  the `kani` CI job, `.github/workflows/kani.yml`, `#[cfg(kani)]`-gated so it never compiles
  into a normal build and this job is the only place the harnesses run):
  1. `recover()` totality — the recovery-decision match, driven against arbitrary bounded
     inputs, always terminates in exactly one `RecoveryDecision` variant; never panics, never
     `unreachable!()`.
  2. Recovery safety predicate — every reachable durable outcome resolves old-or-new-committed,
     never torn/partial.
  3. Transition inductive-invariant preservation for the txn-record state machine (one harness
     for the single-step inductive case, one for a bounded multi-step sequence) — the state
     enum itself is unchanged from §Decision 7a; this proves totality/safety of transitions
     already specified there.
  4. Admission-gate quiescence — re-proves and extends **INV-GATE-TXN**
     (`gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}`, the invariant the v1.14
     DEF-1 formal-finding exception above introduced) against the model as extended with the
     drain/reservation wiring now reachable in the implementation.
  5. Bounded pointer-swap crash-atomicity + WAL ordering — a symbolic bounded crash-trace
     harness (nondeterministic crash point across the write/fsync/rename/fsync/pointer-swap/
     append sequence) proving `recover()` applied to the post-crash durable state always
     converges to exactly the OLD or NEW generation, never torn — run as a before/after
     regression pair against the pre-fix and post-fix call ordering, the same
     "prove the fix actually fixes it" form this ADR's DEF-1/INV-GATE-TXN precedent already
     established.
  6. Recovery idempotence — `recover()` → apply decision → `recover()` again converges to the
     same (or next-table-row-deterministic) decision; never a repeated destructive action.
- **Fault-injection covers the real-OS refinement Kani cannot** — 30 tests in
  `crates/factory-dispatcher/tests/bc_1_18_011_b2_migration_crash_injection_test.rs`, all
  green, feature-gated behind `factory-dispatcher/failpoints` (a no-op under the default
  feature set). Each test restores an identical fixture directory, runs the migration in a
  **child process** with a named `migration_fs::*` failpoint configured to call
  `std::process::abort()` (a genuine SIGABRT crash — no stack unwinding, no destructors run;
  deliberately never `fail`'s own `"panic"` action, which is not power-loss-equivalent) at a
  real `rename`/`F_FULLFSYNC`/`fsync` syscall boundary, then runs `recover()` via `StdFs`
  against the crashed on-disk state and asserts: resolution is exactly OLD or NEW generation;
  every referenced shard parses and passes its recorded hash; the txn record reaches a
  terminal state; a second recovery pass is idempotent. This is the empirical confirmation
  that the real filesystem satisfies the abstract `Fs` model's axioms (the two-namespace
  live/durable model and the atomic-rename axiom) — a separate refinement obligation from the
  Kani safety proof itself, not a re-derivation of it.

**§Decision 7b WAL-ordering conformance (not a spec change).** §Decision 7b's durable-ordering
sequence was always specified as: sync staging files → append+fsync the INTENT record for
every target (the WAL boundary — "after this fsync, every rename is recoverable") →
pre-commit fingerprint recheck → pointer swap → per-target rename → fsync parent dir →
append DONE record. The implementation had deviated from this by constructing and appending
each target's INTENT record only after that target's `rename` (inside
`execute_canonical_path_moves`, post-swap), leaving a crash window between rename and its
recovery record with no durable explanation — a code-vs-spec conflict, not a spec gap (per
CLAUDE.md's Standing Rule: spec wins). The OBL-1 discharge moves INTENT-record
construction+append+fsync to a new pre-swap loop over all pending targets, strictly before
the pre-commit fingerprint recheck and pointer swap, exactly matching the sequence §Decision
7b already specifies; `execute_canonical_path_moves` is unchanged in its post-swap
rename→fsync-dir→append-DONE sequence. Kani harness 5 above is the before/after regression
proof that this reorder closes the gap.

**Discharges [D-1232-OBL-1].** With 7/7 Kani harnesses PROVED and 30/30 fault-injection tests
green, both verification arms this ADR's own §Verification Strategy subsection specified —
"Kani model-checking harnesses on the Rust state machine (executor.rs + shard_manager.rs)...
Exhaustive crash-interleaving fault-injection tests" — are complete. This closes the cluster-5
TDD completion blocker [D-1232-OBL-1] set by the ratification. §Decision 4e/5a/7c's frozen
text is unchanged; the prose-freeze clause in §Consequences → Verification Strategy remains in
force for any future amendment not grounded in a formal/model-check finding.

---

## Error Code Semantics

This section covers two distinct layers: (1) **migration binary process exit codes** — emitted
by the `crates/factory-dispatcher` migration subcommands (`migrate-bc-index`,
`backfill-append-logs`); these are operating-system process exit codes. (2) **guard-layer
admission code** — emitted by the native admission gate in `executor.rs` (dispatcher hook
path); this is a `HookResult` value, not a migration binary process exit code. The two layers
are distinguished by sub-table below. The product-owner added catalog rows to `error-taxonomy.md`
(v1.25) for each code. This section is the definitive record of the trigger, severity, and exit
behavior for each code. **(v1.10 LOW-1):** Prior versions incorrectly shelved E-MAINTENANCE-001
under the migration binary section; it has been moved to its own sub-table.

**Migration binary process exit codes** (emitted by `crates/factory-dispatcher` migration
subcommands; these are OS process exit codes, not HookResult values):

| Error Code | Trigger | Severity | Exit |
|---|---|---|---|
| `E-BINARY-INTEGRITY-FAILURE` | SHA-256 digest of the binary (computed through opened fd or path) does not match the stored/expected digest; OR programmatic mtime guard detects mtime change between digest-check and exec attempt on macOS | BLOCKED — migration aborted before exec | exit 2 |
| `RECOVERY_REQUIRES_REAUTHORIZATION` | Recovery process encounters a COMMITTING transaction whose original manifest has expired AND no completion-only recovery manifest is present or valid | BLOCKED — cannot forward-recover without fresh authorization | exit 2 |
| `EXPIRY_ABORT` | Activation manifest has expired (timestamp_utc + expires_after_hours < now) OR is absent at STAGING resume; OR a pre-generation crash sub-state (txn=STAGING, generation_id=null) is discarded despite a valid manifest (v1.12 F2 / v1.13 MED-1 third arm) — in all cases txn record state is STAGING (pre-pivot; no canonical paths changed); clean abort (delete staging generation directory if present, set txn → ABORTED, gate → OPEN). **Applied same-burst (v1.10 MED-1):** error-taxonomy.md v1.27 EXPIRY_ABORT trigger widened to "expired OR absent at STAGING resume." **(v1.13 MED-1 PO handoff):** PO must widen error-taxonomy.md EXPIRY_ABORT trigger to add: "OR pre-generation crash sub-state (txn=STAGING, generation_id=null) discarded despite valid manifest (v1.12 F2 / v1.13 HIGH-1 step-3.5 path)." | NON-ERROR termination — no canonical paths changed; re-activation required. **Stderr (v1.23 item 8):** one line, two arm-specific normative texts (manifest expired/absent vs null-generation), printed only after the txn → ABORTED then gate → OPEN writes succeed | exit 1 |
| `FINGERPRINT_MISMATCH_ABORT` | Source files changed between quiescence snapshot and fingerprint recheck at step 5 (sha256 of source != source_sha256 in txn record) | BLOCKED — abort before irreversible step | exit 2 |
| `DRAIN_TIMEOUT_ABORT` | Coordinator waited 30 seconds for active writer reservations to drain to zero; quiescence not achieved | BLOCKED — migration deferred; gate returned to OPEN | exit 2 |
| `ARCH_INDEX_PARITY_ABORT` | Three-way ARCH-INDEX parity check fails: config.arch_index_sha != manifest.approved_arch_index_sha OR != live ARCH-INDEX SHA | BLOCKED — abort before any staging work | exit 2 |
| `COMPLETION_MANIFEST_REJECTION` | Completion-only recovery manifest fails validation: activation_id mismatch, staged_generation_id mismatch, fencing_generation mismatch, or allowed_steps includes pre-pivot steps | BLOCKED — abort recovery; require new completion-only manifest | exit 2 |
| `CENSUS_MISMATCH_ABORT` | Pre-pivot census (step 3b PC2): any `BC-X.YY.NNN` ID from the original-census set appears in zero or more than one staged shard file (ID-set exactly-one-shard violation; detects EC-001 dup+drop even when total count is equal); OR shard-boundary capacity violation (step 3b internal check — distinct from `E-SHD-005`, which is a `HookResult` of the steady-state admission gate in BC-1.18.006/BC-1.18.010, NOT a process exit code of the migration binary). Process exit code. | BLOCKED — abort before pointer swap; txn → ABORTED; gate → OPEN | exit 2 |
| `CONTENT_PRESERVATION_ABORT` | Pre-pivot checks (step 3b): staging-file integrity failure (sha256(staged_file) != expected_post_hash for any staged file, indicating modification after staging sync); OR PC1 structured-equivalence failure (SHA-256 of staged per-BC-row content in canonical BC-ID sort order does not match `source_body_row_sha256` from txn record, indicating row bytes were altered, dropped, or reordered during the split). NOTE: `source_sha256` (whole-file hash) is used ONLY by step 5 fingerprint recheck; PC1 uses the dedicated `source_body_row_sha256` field. Process exit code. | BLOCKED — abort before pointer swap; txn → ABORTED; gate → OPEN | exit 2 |
| `ALREADY_MIGRATED` | the invoking migration's own terminal record exists (`.factory/migration-state/completed.json` for `migrate-bc-index`; `.factory/migration-state/completed-backfill-append-logs.json` for `backfill-append-logs`, §Decision 7e) AND (clean steady state OR the §4e verify-then-finalize reconciliation succeeded) | NON-ERROR sentinel — migration previously completed; no action in the clean state; a stale COMMITTING txn/non-OPEN gate is first reconciled (txn→COMPLETED, THEN gate→OPEN) | exit 0 |
| `COMPLETION_RECORD_MISMATCH_ABORT` (v1.18 D3) | the terminal record is present but cannot be proven to describe the same finished migration as the non-terminal txn record: unparseable; `txn_id`/`generation_id` ≠ the txn's; `canonical_paths_count` ≠ N; any canonical file `sha256` ≠ `expected_post_hash`; txn STAGING alongside a terminal record; or COMMITTING with a different `activation_id` | BLOCKED — fail-closed integrity anomaly; NO txn finalization, NO gate flip; human investigation. The PreToolUse analogue (§5a Branch C failure) is an E-MAINTENANCE-001 block with the mismatch reason logged | exit 2 |

**Guard-layer admission code** (emitted by the native admission gate in `executor.rs`,
dispatcher hook path — this is a `HookResult` value, NOT a migration binary process exit code):

| Error Code | Trigger | Severity | Exit |
|---|---|---|---|
| `E-MAINTENANCE-001` | Mutation tool call (Edit/Write/MultiEdit/Bash) attempted while gate state is DRAINING or LOCKED, OR while txn record exists with state STAGING or COMMITTING. Emitted by the native admission gate in `executor.rs` (§Decision 5a), NOT by the migration binary. | BLOCKED — writer must retry after migration completes or after PreToolUse step-3.5 self-heals a stuck gate | HookResult (guard layer) |

**v1.20 additions (closes F-009; dedicated `BcIndexMigrationError` variants — neither condition may
reuse `BinaryIntegrityFailure`, whose Display is the digest/TOCTOU code `BINARY_INTEGRITY_FAILURE`):**

| Error / variant | Layer | Trigger | Severity | Exit / surface |
|---|---|---|---|---|
| `RESERVATION_TTL_BELOW_FLOOR` — `BcIndexMigrationError::ReservationTtlBelowFloor { configured_secs: u64, floor_secs: u64 }` | migration binary (production drain entry point) | the production entry point is configured with a `MAX_RESERVATION_TTL` below the 1,800 s floor (§5a "Reservation timing parameters"); a configuration error, raised by `validate_production_reservation_ttl` BEFORE any gate or drain action (the injectable test seam `drain_bc_index_writers` is not bound) | BLOCKED — nothing mutated | exit 2 |
| `E-MAINTENANCE-002` — `BcIndexMigrationError::InvalidToolUseId { len: usize }` and the other admission-check failures | guard layer (native admission gate) | the admission check could not be completed and FAILS CLOSED: `invalid_tool_use_id` (present but non-string/empty/grammar-violating id, §5a "`tool_use_id` presence and validity"), `io` (an OS-level filesystem call made by the check failed with anything other than `ENOENT` on the file itself — `stat` of `<project_root>/.factory` (v1.23; Absent set `ENOENT`/`ENOTDIR`/not-a-directory), directory/reservation create, or open/read/readdir/stat of `migration-state/`, `gate-state.json`, a `txn-*.json`, the terminal record or — on the Branch C verification path — a canonical file; v1.22 "Read-failure mapping" below), `state_integrity` (bytes were read but are unusable: invalid UTF-8, empty/truncated/unparseable JSON, wrong JSON type or shape, a present non-string `migration_id`, more than one live txn). Total decision rule below. No reservation is left behind. | BROKEN — write not admitted | `HookResult::Error`; exit 2 at the PreToolUse hook surface |

**`E-MAINTENANCE-002` `<cause>` — total decision rule (aligned verbatim in substance with
BC-1.18.013 v1.8 Precondition 6(c) and EC-032).** Every way the check can fail to complete maps
to EXACTLY ONE cause, evaluated in this order, stopping at the FIRST failure (no aggregation):
`tool_use_id` check → `reservations/` creation and the reservation create (W1) → `gate-state.json`
→ `txn-*.json` records in ascending filename order (W2) → the terminal record (only on the Branch C
verification path). **(v1.24 RULING, F-S2508-L4-002 — the list above is the order of the CAUSE-BEARING
CHECKS AFTER SCOPE IS DECIDED; it is not the order of the whole function. Normative whole-function order:
(0) input guards, payload-only and out-of-scope-not-error: event is PreToolUse, tool is
`Edit`/`Write`/`MultiEdit`, `file_path` is a non-empty NUL-free string; (A) the `stat` of
`<project_root>/.factory` (first OS call; ABSENT set ⇒ out of scope, other failure ⇒ `io`); (B) target
resolution (`resolve_target_path`) and protected-path classification — an OUT-OF-SCOPE target is admitted
with NO state read and NO `tool_use_id` examination, so an invalid id on an out-of-scope write is NOT an
error; (C) ONLY for an in-scope protected write: the `tool_use_id` presence/validity check
(`invalid_tool_use_id`), then W1, `gate-state.json`, W2, terminal record. Rationale: scope (which decides
whether the gate applies at all) needs the `.factory` `stat` and the target classification, so "before any
filesystem access" cannot be literal; the id is only meaningful as a reservation key and the gate must not
block writes it does not guard (fail closed applies to protected writes only; blocking every out-of-scope
write on a malformed id would widen the blast radius beyond the v1.23 unstatable-`.factory` ruling for no
safety gain). Consequence: with an unstatable `.factory` and id `../x` the cause is `io`; with a statable
`.factory` and an in-scope target the cause is `invalid_tool_use_id`; "payload-only" in (1) means the
VALIDITY DECISION reads no filesystem state, not that no filesystem access precedes it. All outcomes fail
closed, exit 2; only the cause token differs, and the order is therefore total and deterministic.)**
(1) `invalid_tool_use_id`: payload-only (the decision reads nothing from the filesystem), evaluated at
position (C) above, i.e. after scope is established.
(2) `io`: an OS call returned an error other than `ENOENT` on the file itself (EACCES, EPERM, EROFS,
ENOSPC, EIO, EISDIR, ELOOP, EMFILE, short/interrupted read, …); once a call has failed the content is
NOT examined — an unreadable file is `io` even if its bytes would also be malformed. `ENOENT` of
`gate-state.json`, the txn set or a terminal record is NOT an error (absent semantics: OPEN / no live
txn / no terminal record). (3) `state_integrity`: every byte was read but cannot be accepted — invalid
UTF-8; empty, truncated or unparseable JSON; wrong JSON type/shape (`gate-state.json` must be exactly
one JSON string `"OPEN"`/`"DRAINING"`/`"LOCKED"`; a txn record must be an object with a known `state`
and, if PRESENT, a string `migration_id` — an ABSENT one defaults to `migrate-bc-index` and is not
this cause); or more than one live txn. In this ADR's wording "unreadable" never means a failed read
call: a failed read call is `io`; "unreadable record" means readable bytes not interpretable as the
record. Scope boundaries: reservation files are only CREATED by the check (failure ⇒ `io`) and NEVER
read by admission (reading/parsing them is the binary drain GC, a process-exit-code surface); a
terminal record that READS but does not verify (unparseable, non-UTF-8, empty/truncated, wrong
schema, `canonical_paths_count` ≠ N, id/hash mismatch) is NOT `E-MAINTENANCE-002` — it yields
`E-MAINTENANCE-001` WITH the ` (completion-record mismatch — operator investigation required)`
suffix (the Branch C `FailClosedMismatch` outcome), txn NOT finalized, because an unverifiable
terminal record is an expected recovery input, not a failure of the check; a failed read CALL of the
terminal record is `io`. Terminology: throughout this ADR a "plain" `E-MAINTENANCE-001` means
NO suffix and arises in exactly four cases — gate-only block (no live txn), terminal record ABSENT
(`NoOp`), foreign refusal (`migration_id ∉ K`), live coordinator (EWOULDBLOCK); every Branch C
verification failure carries the suffix. Both
`io` and `state_integrity` are fail-closed: no reservation left behind, no txn/gate write, no Branch
A/B/C repair; the message carries only the cause token, details go to the `migration.admission_failed` InternalLog event (v1.21; §5a "Admission diagnostics channel").

**Read-failure mapping (v1.22 ruling — a canonical-file or terminal-record read CALL that fails during
admission).** A read CALL (`open`/`read`/`stat`) of the live txn's terminal record, or of any canonical
file read during Branch C verification, that fails with anything other than `ENOENT` on the file itself
(EACCES, EPERM, EISDIR — e.g. the path is a directory —, ELOOP, EIO, EMFILE, a short or interrupted
read, …) maps to **`E-MAINTENANCE-002 (io)`**, NOT to a Branch C `check` token and NOT to
`E-MAINTENANCE-001`. Exactly: one `migration.admission_failed` (`cause=io`), NO `migration.admission_blocked`,
no `_advisory`, no reservation left behind, no txn/gate write, txn not finalized, not discarded. `ENOENT`
keeps its absent semantics: ENOENT of the terminal record is "absent" (plain `NoOp` / Branch B path);
ENOENT of a canonical file during verification is the `canonical_hash_mismatch` check (domain item (7)).
*Rationale, from the existing rules:* (1) the total `<cause>` rule above says every way the check can
fail to complete maps to exactly one cause and a failed OS call is `io` ("once a call has failed the
content is NOT examined"; the enumerated file list is illustrative — "…" — and the rule's trigger is the
failed call on the file itself, so the terminal record and the canonical files it names are covered by
(2), not by (3)); (2) BC-1.18.013 EC-032 already decides the terminal-record leg: "terminal record: read
failure ⇒ `io`, unverifiable content … ⇒ NOT `E-MAINTENANCE-002`" — "read failure wins when both would
apply"; extending the same split to the canonical files is the only reading under which one rule holds for
every file the check reads; (3) the item-33 rulings make `check` a CLOSED NINE-token domain of
verification OUTCOMES, each a compile-time constant that is never derived from on-disk content: a read
that failed produced no outcome about the record (the content was never examined), so a tenth
`terminal_record_unreadable` token would both break the closed domain and report an infrastructure fault
as evidence that a corrupt or mismatching completion record exists; (4) fail-closed safety: classifying an
unreadable record as ABSENT would let Branch B discard a txn whose record merely could not be read; and
(5) the exactly-once invariants stay clean — one verdict (`E-MAINTENANCE-002`) ⇒ one `_failed`, with
`_blocked` reserved for `E-MAINTENANCE-001`. Conformance vector: BC-1.18.013 EC-037 Canonical Test Vector
row (10) (record path a directory / `chmod 000` ⇒ `E-MAINTENANCE-002 (io)`, no `_blocked`).

**Factory-root stat failure (v1.23 ruling).** The first OS call of the check is the `stat` of
`<project_root>/.factory` (§5a "Factory-root lookup mapping"): a failure other than the closed Absent set
(`ENOENT`, `ENOTDIR`, not-a-directory) is `E-MAINTENANCE-002 (io)` on the admission leg — same event
shape as every other `io` failure, no tenth cause, no new code.

**Txn-record interpretation — tiers (v1.23 ruling — closes F-S2508-L3-001).** The shape rule above
("a txn record must be a JSON object with a known `state` and, if PRESENT, a string `migration_id`") is
**Tier 0** and is the ONLY interpretation applied to a record that no executing branch consumes. Every
other field is **Tier 1**: read LAZILY, from the raw JSON object, by the branch that consumes it, only
when that branch executes. Admission MUST NOT deserialize a `txn-*.json` into the full
`BcIndexMigrationTxnRecord` (Decision 7a schema) at read time: whether a record is interpretable must not
depend on a schema version the reader may not share, which would make a foreign (newer-build) record
`state_integrity` instead of the foreign refusal that §7e / BC-1.18.013 EC-029 give precedence over
every record check.
*Tier 0* (every `txn-*.json`, ascending filename order, first failure wins): valid UTF-8 JSON object;
`state` ∈ {STAGING, COMMITTING, COMPLETED, ABORTED}; `migration_id`, if PRESENT, a JSON string. Failure ⇒
`state_integrity` (`TxnRecordMalformed` / `TxnMigrationIdNotString`). The read yields the raw object, the
parsed `state`, the effective `migration_id` (absent ⇒ `migrate-bc-index`) and the path — nothing else.
*Tier 1*, by consumer, in planner order:

| Consumer (executes when) | Fields read | Absent / ill-typed field ⇒ |
|---|---|---|
| Non-live record (COMPLETED/ABORTED), foreign or known; Branch A (no live txn) | none | n/a — admitted / gate repair; the record is neither modified nor deleted |
| Foreign refusal (live, `migration_id ∉ K`) | none | n/a — plain `E-MAINTENANCE-001`, precedence over every other record check |
| Live block (live known txn, no branch below decides otherwise) | none (the diagnostic `txn_id` is the field if a string, else the literal `unknown`; informational, never a verdict input) | n/a — plain `E-MAINTENANCE-001` |
| Branch B (live known, STAGING, lock acquired, terminal record ABSENT — the planner's `NoOp`/STAGING arm) | `schema_version` (only if PRESENT; version gate FIRST, v1.23 item 10(b)/(d): integer ≥ 2 ⇒ `state_integrity` kind `txn_record_newer_schema`, no mutation; other non-1 ⇒ `txn_record_malformed`; absent ⇒ proceed) then `generation_id` | key PRESENT with JSON `null` ⇒ null generation ⇒ discard; PRESENT string ⇒ not Branch B ⇒ live block; ABSENT key or any other type ⇒ `E-MAINTENANCE-002 (state_integrity)`, `TxnRecordMalformed`, NO mutation. The discard rewrites the raw object and needs no other field |
| Branch C (live known, terminal record present) | S-25.08 build: none (verification is the unwired seam: every check reports unverified ⇒ `FailClosedMismatch`/`finalize_unwired`). S-25.06 once wired: `activation_id` (string; `txn_id_mismatch`), `generation_id` (string; `generation_id_mismatch`, COMMITTING only), `intent_log_path` (string) and, v1.24, `canonical_move_plan` (non-empty array of `{staging_path, canonical_path}`; ADR-054 Decision 2/B-3) — both read only when the `canonical_hash_mismatch` check executes, see "Branch C hash source" below; `staging_with_terminal_record` reads none | `E-MAINTENANCE-002 (state_integrity)`, `TxnRecordMalformed` (absent, ill-typed or EMPTY plan counts as malformed), evaluated AT that check's position in the fixed first-failure-wins order (an earlier check's mismatch wins) |

**Absent is not null.** serde's `Option` default would read a missing `generation_id` as `None` and so as
"null generation", turning a hand-edited or truncated-but-parseable record into a Branch B DISCARD
(a txn mutation on an unproven precondition). Branch B requires the key PRESENT and JSON `null`.
*Rationale, from the existing rules:* (1) the total `<cause>` rule item (3): `state_integrity` means
"every byte was read but cannot be accepted" — acceptance is per consumer, so a field-poor record is
unacceptable exactly to a branch that needs the missing field, and irrelevant to a verdict that needs
none; (2) foreign precedence (EC-029: "a record this build cannot interpret is not this build's to
discard") forbids any Tier 1 read before the foreign decision; (3) fail-closed in both directions — never
mutate on an unproven precondition (Branch B), and never answer a corrupt record that a mutating branch
must consume with the generic "retry after migration completes" block that would hide the corruption
forever, so the consuming branch reports `state_integrity`; (4) the live block needs no field, so a
shape-valid field-poor live record blocks plain (option "b" for that leg, option "a" for the consuming
legs); (5) the planner `plan_stale_gate_reconciliation` and its VP-147 Kani proofs are UNCHANGED: the
shell resolves the Branch B tri-state (null / non-null / unusable) BEFORE calling the planner, only under
the exact Branch B execution conditions above, and returns the `state_integrity` error instead of
calling it when unusable. Non-live records with a valid Tier 0 shape are admitted in all cases
("gate OPEN AND no live txn"; `is_live` covers STAGING/COMMITTING only); the "more than one live txn"
`state_integrity` counts live records only, foreign included.

**Branch C hash source (v1.23 correction — found while ruling the binary leg).** The first v1.23 draft of
the Tier 1 table named `pending_canonical_moves[].expected_post_hash` as a txn-record field. No such field
exists: Decision 7a defines the plan field (v1.24 name `canonical_move_plan`; v1.23 name
`pending_canonical_moves`) as a list of `{staging_path, canonical_path}` only (and the plan element struct in
`shard_manager.rs` has exactly those two fields); `expected_post_hash` is a field of the intent-log record
(ADR-054 Decision 1), written at INTENT time and CARRIED into the DONE record by COPY and COMPARISON (v1.24:
ADR-054 B-2 — a DONE whose hash was merely observed from the file it is later verified against would make
verification circular). The "as recorded in the txn record / intent log" wording of §4e/§5a therefore
resolves to the intent log. The `canonical_hash_mismatch` check reads the txn's `intent_log_path` and
`canonical_move_plan` (Tier 1: an absent, null, non-string `intent_log_path`, or an absent, null, ill-typed or
EMPTY plan at that check ⇒ `state_integrity`, `TxnRecordMalformed`) and, per canonical path of the plan, the
intent log's INTENT and DONE records read by the ADR-054 Decision 1.7 reader (a torn tail is absent). The
check requires the plan's `canonical_path` set to equal the distinct INTENT `target_canonical` set, and for
EVERY plan path a DONE with `DONE.expected_post_hash == INTENT.expected_post_hash == sha256(file)` (and the
log-level invariants L1-L4). A missing intent log, a path with no DONE record, a DONE/INTENT/file hash
disagreement, a plan that disagrees with the INTENT set, or mid-log corruption (ADR-054 Decision 1.7) is
`canonical_hash_mismatch` (the unverifiable-hash class, NOT `state_integrity`: the intent log is an ADR-054
artifact with its own torn-record semantics, not a txn-record field). An intent-log read failure other than
`ENOENT` is `io` on the admission leg and `Io` (exit 2) on the binary leg. On the COORDINATOR (acting, not
verifying) the same plan-versus-INTENT disagreement is `MIGRATION_STATE_INTEGRITY_FAILURE` kind
`txn_record_malformed` and mid-log corruption is `INTENT_LOG_CORRUPT` (ADR-054 B-3 and Decision 1.7).

*Rulings added to this paragraph (S-25.09 operator-runbook findings; same uncommitted v1.23).* (i)
**`intent_log_path` is written by the coordinator, not derived.** Decision 7c step 1 (and the drain-step-7
note) already require the coordinator to persist `intent_log_path` in the SAME txn-record write that assigns
`generation_id`; the value is the Decision 7b path `.factory/migration-state/intent-<generation_id>.log`
(exactly the path the coordinator appends to). It is JSON `null` only while `generation_id` is null (the
pre-7c-step-1 STAGING sub-state). A record that is COMMITTING, or STAGING with a string `generation_id`, and
carries a null `intent_log_path` therefore violates the spec'd write; the Tier 1 rule above classifies it as
`state_integrity` / `TxnRecordMalformed` at the `canonical_hash_mismatch` check, and the verifier MUST NOT
fall back to deriving the path from `generation_id` (a derived fallback would hide the corruption forever —
the same reasoning as "Absent is not null"). The strict-presence decode is unchanged: the key MUST be PRESENT
on every record; `null` is decodable (it is the legitimate pre-7c-step-1 value), and it is the consuming
check that rejects it. (ii) **N for `migrate-bc-index` is txn-specific.** "The migration's fixed N" in §5a
Branch C and the taxonomy denotes the fixed 4 for `backfill-append-logs` only; for `migrate-bc-index`,
`canonical_paths_count` and N are the number of canonical paths in the txn's move plan
(`canonical_move_plan`; staged shard, sub-shard and manifest files plus `BC-INDEX.md`), equal to the number of
distinct `target_canonical` values among the intent log's `INTENT` records, and at least 1 (v1.24: a COMMITTING
record with an empty plan or a plan/INTENT-set disagreement is NOT verifiable — ADR-054 B-3). (iii) **`txn_id` is `activation_id`.** Decision 7a ("same as
`activation_id`"), the Decision 7c step 6/8 payloads, and every Branch C `txn_id == activation_id` check bind
one value; an implementation that writes any other `txn_id` (e.g. a `txn-` prefix) makes every genuine
`completed.json` fail `txn_id_mismatch`. (iv) **Intent-log vocabulary.** The intent log is plain text
(Decision 7b → ADR-054 Decision 1), never JSON; the canonical-path key of an intent-log record is
`target_canonical`, while `canonical_path` is the key of a txn record's `canonical_move_plan[]` elements
(v1.24 rename of `pending_canonical_moves[]`). (v) **Exit-0 silence.**
`ALREADY_MIGRATED` and the fresh `Completed` outcome are exit status `0` and print nothing to stdout or
stderr (no spec text requires a token on exit 0, and the exit-code table gives the sentinel its meaning by the
status alone); only a non-zero exit prints one stderr line (`<subcommand>: ` + the variant text). Callers MUST
branch on the exit status, never parse output for `ALREADY_MIGRATED`.

**Migration binaries — recovery and finalize legs (v1.23 ruling; closes S-25.06 AC-023 / T-8n(d) open
question).** The tier rules above bind the migration binaries (`migrate-bc-index` recovery;
`backfill-append-logs` recovery, verifier and finalize) exactly as they bind admission. There is ONE record
interpretation with two surfaces, mirroring the D-2 single anchoring rule: the surfaces differ only in
how the verdict is delivered.

1. *Variant, code, exit.* A txn record that fails Tier 0, or a Tier 1 field that the executing arm
   consumes and finds absent or ill-typed, is `BcIndexMigrationError::AdmissionStateIntegrity { kind:
   TxnRecordMalformed | TxnMigrationIdNotString, detail }` — taxonomy `MIGRATION_STATE_INTEGRITY_FAILURE`,
   process exit **2**. It is NOT `BinaryIntegrityFailure` (`BINARY_INTEGRITY_FAILURE` is reserved for the
   binary digest/TOCTOU check; v1.21 F-012), NOT `ExpiryAbort` (exit 1: "no harm done, re-activation
   fixes it" — a corrupt record is not fixed by re-activating), and, in the verifier, NOT
   `COMPLETION_RECORD_MISMATCH_ABORT`: that code means "every field read was well-formed but the terminal
   record and the txn disagree", which is a different operator action (investigate a completed-looking
   migration) from a corrupt txn record. A txn Tier 1 malformation is always `MIGRATION_STATE_INTEGRITY_FAILURE`;
   a well-formed field that disagrees is always the mismatch class — on both surfaces. The variant's
   `Display` keeps the v1.21 token wording; no new variant, kind or code.
2. *Fail closed, nothing mutated.* The error is raised BEFORE the arm's first mutation: the txn record, the
   gate, the terminal record, any `gen-<id>/` directory, the intent log, staging and canonical files are
   byte-identical; no gate flip; no `COMPLETED` rewrite; no discard/abort; the migration lock is released
   only by the process exiting. A corrupt record is never "repaired" by the tool that cannot interpret it.
3. *Loader.* The coordinator reads every `txn-*.json` with the Tier 0 reader (UTF-8 failure included — it is
   `TxnRecordMalformed`, not `Io`) and the SAME order and precedence as admission: ascending filename, first
   failure wins; a live record of another migration or with `migration_id ∉ K` is refused (the binary's
   `FOREIGN_MIGRATION_REFUSED` refusal, item 9) with no Tier 1 field read; COMPLETED/ABORTED records, foreign or
   known, are never modified by recovery and are never rejected for a missing Tier 1 field (a stale
   terminal record's archive rename is derived from the file path, not from its `activation_id` field).
   The coordinator MUST NOT deserialize a record into `BcIndexMigrationTxnRecord` at read time.
4. *Strict-presence typed decode, at arm entry.* A recovery arm that rewrites the txn record
   (`ResumeFromStaging`, `ForwardRecovery`, `CleanAbortExpiredStaging`) decodes the ONE live known record
   into `BcIndexMigrationTxnRecord` from the raw object at the top of the arm, before its first mutation,
   with every key REQUIRED to be present (an `Option` field must be present, JSON `null` allowed;
   `#[serde(default)]` is forbidden on txn fields — a defaulted field written back by `write_txn_record`
   would fabricate state). Any absent, ill-typed or — for a field the arm requires to hold a value — null
   field ⇒ item 1. Every record this build wrote carries every key (serialize emits `None` as `null`), so
   the strictness rejects only hand-edited, truncated-but-parseable or foreign-schema records.
5. *`generation_id`, the one field with a distinct null meaning.* Same tri-state as Branch B: key PRESENT
   and JSON `null` on a STAGING record ⇒ the null-generation discard (exit and abort semantics unchanged:
   ADR §4e `EXPIRY_ABORT` third arm); PRESENT string ⇒ resume/clean-abort per the existing planner; ABSENT
   key or non-string/non-null ⇒ item 1 with NO mutation. On a COMMITTING record `generation_id` MUST be a
   string: absent, null or ill-typed ⇒ item 1 (this supersedes the coordinator's former
   `Quarantine { CommittingWithoutGenerationId }` → `BinaryIntegrityFailure`, which labelled a record-field
   malformation with the digest code). The null-generation discard rewrites the raw object and needs no
   other field, so it is ONE shared raw-object primitive (`abort_null_generation_txn`) used by the admission
   Branch B and by the coordinator: the same record is discarded by both surfaces or by neither.
6. *Verifier/finalize (S-25.06, `backfill-append-logs`; and the `migrate-bc-index` §4e reconciliation).*
   Same closed nine-token `check` domain and the same first-failure-wins order as the PreToolUse Branch C;
   `activation_id`, `generation_id`, `intent_log_path` are read from the raw object only at their check;
   an earlier check's mismatch (`COMPLETION_RECORD_MISMATCH_ABORT`, exit 2) wins over a later field's
   malformation (`MIGRATION_STATE_INTEGRITY_FAILURE`, exit 2); `staging_with_terminal_record` reads none.
   Both exit 2, neither finalizes or flips; the operator-visible difference is the code and the stderr
   line. The terminal record's OWN malformation (unparseable `completed*.json`, wrong shape) is not a txn
   field and stays `COMPLETION_RECORD_MISMATCH_ABORT`.
7. *Rationale, from the existing rules.* (a) The coordinator error taxonomy already routes malformed
   gate/txn state to `MIGRATION_STATE_INTEGRITY_FAILURE` exit 2 (v1.21) and reserves
   `BINARY_INTEGRITY_FAILURE` for the digest; this ruling applies that row to the field level rather than
   inventing a code. (b) Fail-closed: never mutate on an unproven precondition — `Option` serde defaulting
   turns an absent `generation_id` into a STAGING discard, a mutation on a record nobody proved is
   pre-generation. (c) D-2 / total cause rule: one record, one interpretation — if admission answers
   `state_integrity` for a field the binary silently defaults (or the reverse), the two surfaces disagree
   about whether the migration state is corrupt, and the operator is told "retry after migration
   completes" while the tool is quietly rewriting the record. (d) Foreign precedence (EC-029): a newer
   build's record must be refused as foreign before any field the older build cannot interpret is read.
   (e) **Exit-code classes (amended after independent validation, `.factory/research/adr-052-v123-architect-calls-validation.md`
   Call 1; supersedes the earlier "exit 1 = re-activation" definition, which `MIGRATION_LOCK_CONTENTION` contradicted).**
   Within the migration subcommands (`migrate-bc-index`, `backfill-append-logs`), exit **0** = the requested end
   state holds and was read under the lock (`Completed`, or `AlreadyMigrated` established by a read under
   `exclusive.lock`; item 11(c)). Exit **1** = *no harm done, safe to re-run; the stderr code token says what
   to do next*: it is a class, not one trigger, and today it covers exactly {`EXPIRY_ABORT` ⇒ re-activate,
   then re-run; `MIGRATION_LOCK_CONTENTION` ⇒ re-run after the holding coordinator exits}. Both are safe to
   re-invoke blindly: after `EXPIRY_ABORT` the txn is ABORTED and the gate OPEN, so a re-run takes the
   `AbortedTerminal` → fresh-run path; after contention nothing was read or written. Exit **2** = fail
   closed, an operator must act (every other variant). The two exit-1 conditions are told apart by the stderr
   token, never by the exit status alone; a caller that branches on exit codes alone MUST treat exit 1 as
   "re-run is safe" and exit 2 as "stop". Exit 75 (`EX_TEMPFAIL`) was evaluated and NOT adopted: no
   cross-tool convention maps lock contention to 75 (`flock(1)` and Terraform use 1), FreeBSD marks
   `sysexits.h` discouraged, and S-25.06's already-built `AppendLogMigrationError::LockContention` uses 1;
   revisit only if a non-agent automated caller that branches on exit codes is planned, and then change both
   binaries in one change. The migration subcommand's exit status reaches its caller as a Bash-tool result,
   never as a hook verdict (`main.rs` routes `migrate-bc-index` before the hook-envelope read), so Claude Code's
   hook meaning of exit 2 does not apply to it. **`process_exit_code` is an EXHAUSTIVE `match` over
   `BcIndexMigrationError` (and the `AppendLogMigrationError` mirror) with NO wildcard arm** — exit 1 is listed
   by name (`ExpiryAbort { .. } | MigrationLockContention`), exit 2 is listed by name for every other variant —
   the same discipline as `admission_failure_cause`. A future variant must be assigned an exit class
   explicitly at compile time instead of silently inheriting 2. Red test owed: a compile-time-exhaustive
   table test enumerating every variant with its expected exit code (adding a variant without a row fails
   the build).
8. *`EXPIRY_ABORT` stderr — one code, one exit, two normative lines (closes S-25.09 red-test question 1).*
   `EXPIRY_ABORT` stays ONE taxonomy code, exit **1**, with three trigger arms (manifest expired, manifest
   absent, null-generation discard — §4e). The binary renders it as ONE stderr line, `<subcommand>: ` + the
   variant `Display`, and the `Display` carries an ARM-SPECIFIC DETAIL because the operator-visible facts
   differ: the manifest arms discard a staged generation, the null-generation arm has none (the prior run
   crashed before §7c step 1 created one). The variant becomes `BcIndexMigrationError::ExpiryAbort { arm:
   ExpiryAbortArm }` with `ExpiryAbortArm ∈ { ManifestExpiredOrAbsent, NullGeneration }` — two renderings, not
   three, because `recover()` returns the single decision `CleanAbortExpiredStaging` for "expired" and "absent"
   (§4e row 407 gives them one action) and today `ManifestStatus` is fixed at `StillValid` (no armed-manifest
   reader exists), so the binary cannot and need not tell expired from absent; the shared line says
   "expired or absent" and is therefore true for both. Normative `Display` text (single line, no newline;
   the stderr line is `migrate-bc-index: ` or `backfill-append-logs: ` followed by exactly this):
   - `ManifestExpiredOrAbsent`: `BC-INDEX migration: activation manifest expired or absent at STAGING resume
     (EXPIRY_ABORT, exit 1); the staged generation was discarded, the txn record is ABORTED and the writer gate
     is OPEN; no canonical path changed; re-activation required`
   - `NullGeneration`: `BC-INDEX migration: pre-generation STAGING record discarded (EXPIRY_ABORT, exit 1); the
     prior run crashed before any generation was created (generation_id null), nothing was staged, the txn
     record is ABORTED and the writer gate is OPEN; no canonical path changed; re-activation required`
   Rules: (a) the token `EXPIRY_ABORT` and the literal `exit 1` appear in both; neither contains
   `BINARY_INTEGRITY_FAILURE` or `state integrity failure`; (b) the past-tense claims ("discarded", "is ABORTED",
   "is OPEN") are TRUE when printed: the line is produced only AFTER the txn record was rewritten ABORTED (kept
   at its original path; `abort_reason: "null_generation"` for the null arm) and THEN the gate was written OPEN
   (txn before gate — the `gate=OPEN ⇒ no live txn` order of §5c Branch 2). If either write fails the result is
   that write's own error (`Io`, exit 2), NEVER `EXPIRY_ABORT` — the existing `let _ = discard_incomplete_staging(..)`
   best-effort-then-claim-success pattern is a TD-VSDD-059 paper-fix and is removed; (c) the internal
   identifier `RecoveryDecision::CleanAbortExpiredStaging` and the retired "so the gate self-heals" wording
   are removed from the `Display`; (d) the discard is the SAME for both binaries and both surfaces (item 5);
   the `backfill-append-logs` line differs only by the `backfill-append-logs: ` prefix and the absence of the
   `BC-INDEX migration: ` label (its own variant, S-25.06).
9. *Live foreign record on the binary — `FOREIGN_MIGRATION_REFUSED` (closes red-test question 2).* No existing
   code fits: `E-MAINTENANCE-001` is a `HookResult` of the guard layer, never a process exit code, and
   `BINARY_INTEGRITY_FAILURE` is the digest/TOCTOU code (v1.21 F-012). The term "`LockContention`-class" was
   prose with no variant, token or text; it is RETIRED in this ADR and in every mirror (BC-1.18.011 Precondition
   6(e)/(f)(ii), EC-046(e)(f); BC-1.18.013 EC-016; error-taxonomy rows; S-25.06 AC-027/AC-031): wherever it
   denotes the cross-migration/foreign refusal it means the code below. New variant
   `BcIndexMigrationError::ForeignMigrationRefused { live_migration_id: String }` (and the mirrored
   `AppendLogMigrationError` variant), taxonomy code `FOREIGN_MIGRATION_REFUSED`, process exit **2**
   (an operator must complete or recover that migration with the build/subcommand that owns it; re-running
   this subcommand cannot help). Normative `Display` (stderr = `<subcommand>: ` + it):
   `BC-INDEX migration: refused: a live migration transaction owned by migration_id "<id>" is in progress
   (FOREIGN_MIGRATION_REFUSED, exit 2); this subcommand never recovers, finalizes or aborts another
   migration's record; nothing was changed` (backfill: `backfill-append-logs: refused: …` with the same tail).
   `<id>` is the record's `migration_id` rendered by the SAME function as the v1.21 admission diagnostic
   (v1.25 item 11(g) operator-stderr rule: control characters escaped, each data-derived substring capped at
   **256** characters, NOT 64 — the 64-character cap is the InternalLog-event bound only; a `migration_id` is a
   token of at most 128 characters in the txn-record domain, so 256 never truncates a well-formed id and only
   bounds a hostile one) — a hostile id must not forge terminal output. Scope
   and precedence: it fires on the binary's recovery AND short-circuit paths (item 11) for a single LIVE record
   whose `migration_id` is the OTHER known migration's or `∉ K`; it fires AFTER the Tier 0 loader has read every
   record (a Tier 0 failure of any record wins, ascending filename) and after the `multiple_live_txns` check
   (two live records ⇒ `MIGRATION_STATE_INTEGRITY_FAILURE`, not this), BEFORE `recover()` and with no Tier 1
   field read; it mutates nothing (a foreign STAGING record with `generation_id: null` is not discarded). A
   PRESENT non-string `migration_id` is `TxnMigrationIdNotString` (integrity), not foreign. The PreToolUse
   surface is unchanged: plain `E-MAINTENANCE-001`. SIBLING (TD-VSDD-060, same function): the only lock text
   the binary prints today, `BinaryIntegrityFailure("another migration coordinator already holds the exclusive
   migration lock")`, mislabels `flock(exclusive.lock, LOCK_EX|LOCK_NB)` EWOULDBLOCK with the digest code
   (the F-012 class). It is a different condition and gets its own variant `MigrationLockContention`, code
   `MIGRATION_LOCK_CONTENTION`, exit **1** (retry later, nothing changed — the exit S-25.06's already-built
   `AppendLogMigrationError::LockContention` uses), `Display`: `BC-INDEX migration: another migration
   coordinator holds the exclusive migration lock (MIGRATION_LOCK_CONTENTION, exit 1); nothing was changed;
   retry after it exits`. Flock contention is checked BEFORE the loader (the records cannot be read safely
   without the lock), so contention beside a foreign record reports contention. **(Amended, item 11(c))**
   this holds on EVERY path of both coordinators, including when `completed.json` is present: flock not
   acquired ⇒ `MIGRATION_LOCK_CONTENTION`, exit 1, never exit 0.
10. *Strict-presence scope (closes red-test question 3): the FULL reading is confirmed; EC-045's three named
   keys were examples, not the set.* At a rewriting arm (`ResumeFromStaging`, `ForwardRecovery`,
   `CleanAbortExpiredStaging`) the live known record is decoded with EVERY field of
   `BcIndexMigrationTxnRecord` required to be PRESENT with the right type: `txn_id`, `activation_id`,
   `created_at`, `updated_at` (strings, not null); `fencing_generation` (JSON non-negative integer fitting
   `u64`, not null, not a float or string); `canonical_move_plan` (v1.24 rename of `pending_canonical_moves`;
   array, not null, every element strictly decoded the same way; a COMMITTING record additionally needs it
   NON-EMPTY — ADR-054 B-3, checked by the consuming arm, not by the presence decode); `state` (known, already Tier 0); and the four `Option` keys `generation_id`,
   `source_sha256`, `source_body_row_sha256`, `intent_log_path` (key PRESENT; string or JSON `null`; any other
   type ⇒ item 1). Arm-specific must-hold-a-value rules sit on top (COMMITTING `generation_id` string, item 5);
   this ruling promotes no further `Option` field to non-null. `migration_id` is Tier 0, not part of the
   decode: it may be present (`"migrate-bc-index"`) or absent and a PRESENT value is preserved verbatim on
   rewrite. Any OTHER top-level key on a live known record at a rewriting arm whose `schema_version` is the
   supported one (sub-rule (b)) ⇒ item 1 (`txn_record_malformed`): the typed rewrite cannot preserve a field it
   does not model, and an older build silently dropping it is the same fabricated-state hazard the
   `#[serde(default)]` ban exists for (D-2 fail-closed). The same rule applies one level down (sub-rule (a)).
   The required key set is TWELVE after sub-rule (b) (the eleven above plus `schema_version`; v1.24: the
   count is UNCHANGED by the `pending_canonical_moves` → `canonical_move_plan` rename — a record still
   carrying the OLD key is an unknown top-level key and fails item 1). The `generation_id` tri-state (item 5) is resolved FIRST: a STAGING record with the key PRESENT
   and `null` takes the raw-object discard and decodes NO other key; every other record is decoded whole,
   BEFORE the arm's first mutation. Non-rewriting outcomes (`AlreadyMigrated`, quarantine, the foreign
   refusal) decode nothing. Rationale: a partial decode leaves exactly the unnamed keys able to be defaulted
   and written back; "every key this build wrote is present" makes the full set free for legitimate records.

   **Amendments after independent validation** (`.factory/research/adr-052-v123-architect-calls-validation.md`
   Call 2: rejecting, rather than preserving, unknown state on a record the tool ADVANCES is confirmed — git
   `extensions.*`, SQLite header versions, PNG unsafe-to-copy, X.509/JWS `crit` and PostgreSQL `pg_control` all
   refuse when they cannot interpret what they would rewrite, and no criticality marker exists in this record to
   license pass-through; four gaps are closed here):

   (a) *Nested strictness — `canonical_move_plan[]` (v1.24 rename; v1.23 name `pending_canonical_moves[]`; the
   element struct is `PlannedCanonicalMove`, was `PendingCanonicalMove`).* The same fail-closed rule binds every element, so
   an unknown key one level down cannot be silently dropped by `write_txn_record`'s re-serialization of the typed
   struct (TD-VSDD-060 sibling sweep of the top-level rule). Each element MUST be a JSON object whose key set is
   exactly `{staging_path, canonical_path}`, both JSON strings (not null); a non-object element, a missing or
   ill-typed key, or ANY other key ⇒ item 1 (`txn_record_malformed`), nothing mutated, `detail` naming
   `canonical_move_plan[<index>]` and the offending key rendered by the SAME sanitizer as the top-level
   check (control characters escaped; per-surface cap per item 11(g): 64 characters in the InternalLog event
   `detail`, 256 on the operator stderr line — a hostile key must not forge terminal output).
   The check is an explicit key-set comparison on the raw element performed BEFORE the typed decode;
   `#[serde(deny_unknown_fields)]` on `PendingCanonicalMove` is permitted as defense in depth (no `flatten`
   is involved, so serde supports it) but a serde error string MUST NEVER reach stderr or `detail`
   unsanitized. Evaluated after the version gate (b), so a newer-schema element is reported as newer-schema.

   (b) *`schema_version` — added to the txn record NOW.* Ruling: `BcIndexMigrationTxnRecord` gains a REQUIRED
   top-level `schema_version: u32` whose current value is **1** (`const TXN_RECORD_SCHEMA_VERSION: u32 = 1`),
   written by every `write_txn_record` call and by record creation, with no `#[serde(default)]`, and added to
   `TXN_RECORD_REQUIRED_KEYS` (twelve keys). Why now: ADR item 11 and §Decision 5 record that no activation has
   ever run in this repository, so no legacy record lacks the key and the cost is zero; the sibling on-disk
   records in the same module already carry `schema_version: u32` (= 1); once a release ships the binary the
   addition is no longer free. Why a version and not only the unknown-key check: "newer schema" inferred from
   extra keys misses a newer build that changes the MEANING of an existing key, or the state machine, without
   adding one; every precedent (git, SQLite, PostgreSQL) uses an explicit version as the primary gate and
   unknown members as the secondary gate. *Bump rule:* a build MUST increment the constant for any change that
   adds, removes or renames a key, or changes the meaning of an existing key or of a state transition
   (additive keys included — an older build refuses unknown keys at a known version, so the bump is what lets
   it report "newer build" instead of "corrupt"). *Version gate (newer-schema handling)* — evaluated FIRST at
   every Tier 1 consumer that reads the record (the three rewriting arms; the null-generation discard, sub-rule
   (d); the Branch C / backfill verifier when wired), before any other field, nested element or unknown key is
   examined: (i) key PRESENT and a JSON integer ≥ 2 (any magnitude) ⇒ NEWER SCHEMA: exit 2, kind
   `txn_record_newer_schema` (sub-rule (c)), nothing mutated, no gate flip; (ii) PRESENT and integer 1 ⇒
   proceed to the strict decode; (iii) PRESENT and anything else (0, negative, float, string, null, array,
   object) ⇒ item 1 `txn_record_malformed`; (iv) ABSENT ⇒ `txn_record_malformed` at the rewriting arms (a
   required key), and "not consumed, proceed" at the discard and the lazy Branch C reads, which consume only
   the fields they name (the Tier 1 lazy-read discipline; a minimal hand-built `{state, migration_id,
   generation_id}` record stays admissible there). The version gate does not alter Tier 0, the foreign refusal,
   the one-live-txn check or terminal-record handling: a record of a foreign migration is still refused as
   foreign before ANY field is read, and non-rewriting outcomes decode nothing. Downgrade is therefore
   "refuse, mutate nothing, tell the operator to use the newer build" — never "interpret and rewrite".

   (c) *A distinct kind for a newer-schema record.* Ruling: `AdmissionStateIntegrityKind` gains a sixth
   variant `TxnRecordNewerSchema`, token `txn_record_newer_schema`. No new taxonomy CODE: the condition is
   still `MIGRATION_STATE_INTEGRITY_FAILURE`, exit 2 (the operator must act; re-running this build cannot
   help), but the operator is told the true cause, because the follow-ups differ (investigate corruption vs.
   run the build that wrote the record). Normative `Display` (the existing form, `<subcommand>: ` + variant text):
   `migration admission: state integrity failure (txn_record_newer_schema): txn record schema_version <N> is newer
   than the supported 1; it was probably written by a newer build; recover it with that build; nothing was
   changed`, where `<N>` is the decimal rendering of the integer truncated to 20 characters. It MUST NOT
   contain `txn_record_malformed`, `corrupt` or `BINARY_INTEGRITY_FAILURE`. `admission_failure_cause` stays
   exhaustive and maps the new kind to `state_integrity` (the `E-MAINTENANCE-002` `<cause>` domain
   {`invalid_tool_use_id`, `io`, `state_integrity`} is UNCHANGED). **Wire-domain consequence (checked against
   BC-3.08.001 v1.36 Event 12):** the `migration.admission_failed` field `kind` is a CLOSED five-token
   domain (`gate_record_malformed | txn_record_malformed | txn_migration_id_not_string | multiple_live_txns |
   reservation_serialization`), so a sixth token is a wire-domain change and BC-3.08.001 needs a version bump
   (v1.36 → v1.37; additive token; no new event, no new field, no event-count change). It is reachable on the
   admission surface through the version gate at Branch B (sub-rule (d)); it is NOT left unemitted. Any
   consumer of the closed `kind` domain (VP-079 Event 12 vectors, VP-133 facet 7(e) emission check) gains the
   token.

   (d) *Why `abort_null_generation_txn` may preserve unknown fields while the three rewriting arms reject them
   — justified, and bounded by the version gate.* The distinction is what the rewrite DOES with the record,
   not who calls it. The three rewriting arms continue a state machine: they act on the filesystem from the
   record's values (resume staging, perform the canonical moves, clean-abort a generation) and re-serialize a
   record that remains LIVE, so a field they do not understand can make the next transition wrong while the
   preserved bytes look intact (the PNG "unsafe-to-copy while modifying critical data" and protobuf
   stale-field hazard). The null-generation discard acts on NO filesystem object (`generation_id: null` means,
   by the §7c step order — txn STAGING is written BEFORE the generation directory is created — that nothing was
   staged) and writes the TERMINAL state `ABORTED` plus `abort_reason: "null_generation"`; a terminal record is
   never advanced again and is archived by rename keeping its bytes, so every preserved field is inert
   forensic data, and preserving it is the better record of a crashed run. Both surfaces use the one shared
   primitive (admission Branch B and the coordinator, item 5), so they preserve identically. The justification
   assumes THIS schema's meaning of `generation_id: null`, which a newer record may change — therefore the
   discard is NOT exempt from the version gate: it reads `schema_version` first (sub-rule (b)(i)-(iii); absent
   proceeds, (b)(iv)); a present version ≥ 2 refuses with `txn_record_newer_schema` on BOTH surfaces and
   mutates nothing, before the `generation_id` tri-state is evaluated (a newer schema may rename
   `generation_id`, and that must read as "newer", not "malformed"). Unknown top-level keys at a supported
   version do NOT block the discard (they are preserved): the unknown-key rejection exists only for rewrites
   that keep the record live.
11. *`migrate-bc-index` §4e reconciliation — where it lands, and the interim (closes red-test question 4).*
   The `migrate-bc-index` verify-then-finalize (`completed.json` + live COMMITTING txn: verify, txn → COMPLETED,
   THEN gate → OPEN; §4e row 401, §5c Branch 2 step 0.5) is **S-25.06 AC-031 (B2-2)**, which already owns "the
   unverified `completed.json` short-circuit … REMOVED" and the effectful verifier shared with
   `backfill-append-logs` (same nine-token `check` domain, same order, items 1 and 6). It is NOT S-25.09's:
   S-25.09 has no verifier and must not grow one. But the short-circuit in `run_bc_index_migration_core` is
   defective TODAY on three counts that the S-25.09 recovery leg already governs, so until S-25.06 AC-031
   merges it is narrowed (S-25.09) to the fail-closed subset of the §5c Branch 2 table: (a) it reads txn
   records with the Tier 0 loader (item 3), and a Tier 0 failure is item 1, never swallowed (the current
   `if let Ok(Some(..))` discards `Err`); (b) a LIVE record beside `completed.json` is never finalized and the
   gate is never flipped on it: a foreign record ⇒ `FOREIGN_MIGRATION_REFUSED` (item 9); the own migration's
   live record (STAGING or COMMITTING, no verification possible in this build) ⇒ exit 2
   `COMPLETION_RECORD_MISMATCH_ABORT` with the line `BC-INDEX migration: the terminal record completed.json
   cannot be proven to describe the live txn (COMPLETION_RECORD_MISMATCH_ABORT, exit 2); no verification was
   performed in this build; txn and gate unchanged; operator investigation required` — this is the binary
   twin of the S-25.08 fail-closed Branch C seam, so both surfaces refuse identically until S-25.06 supplies the
   verifier. Independent validation (Call 3a) CONFIRMED fail-closed as the production-grade choice —
   finalizing on an unbound completion marker would be a heuristic commit in the XA heuristic-hazard class, and
   no surveyed recovery protocol (ARIES, RocksDB 2PC XID binding, PostgreSQL `PREPARE TRANSACTION`, `e2fsck -p`)
   finalizes on an unverified marker. It also corrected the premise: "no activation has ever run, so the interim
   blocks no real recovery" holds only until a release ships the binary, and this build can reach the blocked
   state ON ITS OWN normal path (sub-rule (d)). The interim line therefore now has a documented remedy,
   sub-rule (e), and the release-gating question is the human's (see §Downstream "third binary-leg
   extension"); (c) with NO live record the gate is reconciled only under `flock(exclusive.lock, LOCK_EX|LOCK_NB)`
   (see (c) for the not-acquired outcome), only when it is not already OPEN (clean steady state = zero filesystem
   writes), per §5c Branch 2 steps 0–4 — the current unconditional `write_admission_gate_state(.., Open)`
   without the flock can open a live `backfill-append-logs` coordinator's gate (the §7e D4 hazard). When
   S-25.06 AC-031 lands, (b) becomes the shared verify-then-finalize and the interim line is retired (the
   check-specific `COMPLETION_RECORD_MISMATCH_ABORT` / `MIGRATION_STATE_INTEGRITY_FAILURE` lines of item 6
   apply). BC wording that says "`migrate-bc-index` §4e reconciliation" (BC-1.18.013 EC-047(a)-(c), EC-049) is
   therefore S-25.06 AC-031's vectors, not S-25.09's.

   **Amendments after independent validation** (`.factory/research/adr-052-v123-architect-calls-validation.md`
   Calls 3a and 3b):

   (c) *Lock-not-acquired is `MIGRATION_LOCK_CONTENTION`, exit 1, on EVERY path; `completed.json` is
   re-read under the lock and that read is what `recover()` receives.* This REPLACES the earlier
   "EWOULDBLOCK ⇒ skip, exit 0 `ALREADY_MIGRATED`" behavior of the `completed.json` short-circuit (call 3b
   REJECTED), and supersedes every earlier "EWOULDBLOCK ⇒ exit 0" statement in this ADR: the v1.12 F6 row of
   the Finding → Resolution table, the §4e rows for "completed.json exists", the §5c Branch 2 step 0 and its
   fault-injection mandate (b) (each amended in place). Reasons, each sufficient: (1) exit 0 contradicts the
   `ALREADY_MIGRATED` predicate ("own terminal record exists AND (clean steady state OR the §4e
   verify-then-finalize succeeded)"): under contention neither the txn records nor the gate were read, so
   neither disjunct was established and exit 0 would claim something unchecked; (2) the same on-disk state
   would yield different verdicts depending on timing (state S = `completed.json` + own live COMMITTING txn
   is exit 2 uncontended and exit 0 contended; a live foreign record likewise), and fail-closed verdicts
   MUST NOT depend on who else holds a lock; (3) it contradicts the stated precedence (flock is acquired FIRST
   on every path, including the `completed.json` short-circuit; item 9) and the item 9 rule that contention
   beside a foreign record reports contention; (4) no surveyed tool treats advisory-lock contention as
   "already done" (`flock(1)` reports 1; mapping to 0 needs an explicit `-E 0`, a scheduling policy for skippable
   cron ticks). Normative behavior: the coordinator attempts `flock(exclusive.lock, LOCK_EX|LOCK_NB)` BEFORE it
   reads ANY state it will act on. Not acquired ⇒ `MigrationLockContention` (exit 1, item 9 line, zero reads of
   txn/gate state, zero writes), whether or not `completed.json` exists. Acquired ⇒ THEN read `completed.json`
   (existence and content as the code already does) UNDER the lock; any pre-lock existence probe is a hint
   only and MUST NOT select the branch. Present ⇒ `completed_json_interim_short_circuit` (the (a)/(b)/gate
   reconciliation above); absent ⇒ the recovery path, with `recover(&planner_records, None, <the under-lock
   completed read>, ..)` (the real signature is `recover(txn_records, _current_pointer, completed, gen_dir_exists, manifest_status)`: the completed-marker read is the THIRD argument; the second, `_current_pointer`, stays `None`) — the hard-coded `completed = None` and its "honest current read" comment are
   removed. This closes a TOCTOU race: a probe made before the lock can find `completed.json` absent, a running
   coordinator can then write `completed.json`, mark its txn COMPLETED and release the lock, and the late
   process acquires the lock, reads only a COMPLETED terminal txn record, gets `NoActiveTransaction` and starts
   a FRESH run on an already-migrated tree (gate DRAINING → LOCKED, the sharded `BC-INDEX.md` re-read), which
   contradicts `recover()`'s own contract that `completed.json` is "checked FIRST, unconditionally" (git
   `update-ref` likewise verifies under the lock; a check before the lock is only a hint). `ALREADY_MIGRATED`
   (exit 0) is emitted only for a state read under the lock. Caller compatibility: idempotent re-invokers still
   get exit 0 in the steady state ("callers that re-invoke after completion must not treat this as a failure");
   they see exit 1 only during real concurrent execution, which is the correct moment to retry. The
   `backfill-append-logs` binary has the identical rule (item 11 is mirrored there; S-25.06).

   **(c) clarification — the under-lock `completed.json` read has FOUR verdicts (v1.25;
   closes F-S2509-L1-004).** The v1.24 text said "existence and content ... UNDER the lock" but not
   what a stat/read FAILURE or a non-record is. `fs.exists(..)` follows symlinks and maps every stat error to
   `false`, so a dangling symlink, an ELOOP, an EACCES or an EIO on `completed.json` all read as "absent" and
   start a FRESH run on a tree whose other markers may say migrated — the failure class the lock rule exists to
   close, and a fail-open verdict on a fail-closed surface. Rulings (all under `exclusive.lock`, all BEFORE any
   txn/gate read; one function, `read_completed_under_lock`, used by every consumer of the marker):
   1. *Absence is a fact about the DIRECTORY ENTRY, never about the link target.* The probe is
      `symlink_metadata` (lstat-class) on `.factory/migration-state/completed.json`. ONLY `NotFound` from that
      call means absent. This is the v1.22 read-failure rule applied unchanged ("ENOENT keeps its absent
      semantics" only when it is ENOENT **on the file itself**): a dangling symlink has an entry, and the ENOENT
      its read returns is the ENOENT of the TARGET, not of the file. Any other lstat error (EACCES, ELOOP in a
      parent component, ENOTDIR, EIO, EMFILE, ...) leaves existence UNKNOWN. The `.factory`-root precedent
      (v1.23: dangling `.factory` ⇒ absent ⇒ `FactoryRootNotFound`) does not transfer: there "absent" is itself the
      fail-closed outcome, here "absent" authorizes a mutating fresh run.
   2. *Verdicts.* (a) **dangling symlink** (lstat sees an entry; the read returns `NotFound`) ⇒
      `Io { path: completed.json, source }`, exit 2, ONE stderr line
      `migrate-bc-index: BC-INDEX migration: I/O error at <path>: <os error>`; NOT absent (a fresh run), and NOT
      exit 0 `ALREADY_MIGRATED` (the predicate requires the own terminal RECORD to exist, and an unreadable
      entry is not a record: exit 0 would claim something unchecked, the same argument that rejected
      EWOULDBLOCK ⇒ exit 0). (b) **lstat or read fails with ELOOP, EACCES, EIO (or any non-`NotFound` error;
      EISDIR when it is a directory)** ⇒ the same `Io`, exit 2, the failed call's own error. Rerunning is safe
      (nothing was read further or written) after the operator fixes the entry. (c) **reads, but is not a
      record** (not UTF-8, empty, truncated, unparseable JSON, not an object, or an object that does not
      deserialize as `CompletedMigrationRecord`: `generation_id` string, `txn_id` string, `completed_at` string,
      `canonical_paths_count` non-negative integer fitting `u64`; unknown extra keys are ignored) ⇒
      `Io { path: completed.json, source: InvalidData(<serde message>) }`, exit 2, same one-line shape; never
      exit 0, never a fresh run. This is the coordinator-surface twin of the admission rule "reads but does
      not verify ⇒ not `E-MAINTENANCE-002`": at admission an unverifiable terminal record is an expected
      recovery INPUT (`E-MAINTENANCE-001` + mismatch suffix, a block that needs no code path in the guard);
      in the coordinator, where the next action would be an irreversible exit-0 claim or a fresh run, an
      unusable marker is `Io`. It is checked BEFORE the Tier 0 loader, so a corrupt marker beside a live txn
      reports the marker. (d) **a readable record** (a regular file, or a symlink whose target reads and
      validates as in (c)) ⇒ present: `Some(CompletedMigrationRecord)`, the interim short-circuit of item 11
      (a)/(b)/gate-reconciliation proceeds exactly as written.
   3. *Presence-only semantics are KEPT for BRANCH SELECTION and are NOT extended to content verification.*
      A present, valid-shaped marker selects the terminal-state branch regardless of any other file (§7c step 8,
      `recover()`'s "checked FIRST, unconditionally"); NO field of it is compared with a txn, a generation or a
      hash by S-25.09 (that binding is the Branch C / S-25.06 AC-031 verifier, and item 11(b)'s interim line
      refuses exactly because it is absent). What changes is only that the marker must be a READABLE,
      SCHEMA-SHAPED record before it may select the exit-0 branch: the permanent marker is written with
      write-temp + `F_FULLFSYNC` + rename (a torn marker cannot be produced by the writer), so an unusable one is
      external damage, and the existing reader already refuses it (`detect_migration_read_state` parses the
      content and maps a parse failure to `Io`). Presence-only never meant "a directory entry that cannot be
      read counts as migrated": it meant "no OTHER file need be consulted once a record exists".
   4. *What `recover()` receives.* `read_completed_under_lock` returns `Result<Option<CompletedMigrationRecord>,
      BcIndexMigrationError>`: `Err` (verdicts a/b/c) returns from the coordinator before `recover()`; `Some`
      (verdict d) is routed to the item-11 interim short-circuit BEFORE the planner records are built (so Tier 0
      ordering is unchanged; `recover(Some(_), ..)` is by construction `AlreadyMigrated`); `None` (lstat
      `NotFound`) is the value `recover(&planner_records, None, completed_under_lock.as_ref(), ..)` receives.
      The argument is the DERIVED read result, never a literal `None` (the test-mandated source gate); in the
      shipped flow `recover()` therefore observes `None`, which is the honest consequence of presence selecting
      the branch earlier, not a paper-fix: the TOCTOU property is carried by the read being made under the lock.
   5. *Reader parity (TD-VSDD-060 sibling).* `detect_migration_read_state` (BC-1.18.010 §Reader Integration
      step 1) probes the same marker with `read_to_string`, whose dangling-symlink `NotFound` falls through to
      `CURRENT.json` and reports `NotStarted`, i.e. a reader that says "not migrated" beside a coordinator that
      says "unknown". It MUST use the same lstat-first classification and the same record-shape validation
      (verdicts a-d, `BcIndexAddressingError::Io` for a-c), through a shared `read_completed_marker` helper that
      both functions call. Other `fs.exists(..)` uses are not changed by this ruling (the generation-directory
      probe fails CLOSED on a stat error: `false` ⇒ `GenerationIdWithoutGenDir` quarantine); the implementer
      records the audit of every `fs.exists` feeding a fail-open branch. `backfill-append-logs` applies the
      identical rule to `completed-backfill-append-logs.json` (S-25.06).
   6. *`CURRENT.json` gets the same classification (v1.26; closes F-S2509-L2-009).* `detect_migration_read_state`
      read `CURRENT.json` with `read_to_string` and mapped `NotFound` to `NotStarted`: a dangling `CURRENT.json`
      during COMMITTING (the link target is gone, or the entry is damaged) therefore reported "legacy
      `BC-INDEX.md` is current" while the pointer says the new generation is. That is the fail-open class this item
      closes for `completed.json`: "absent" is a fact about the DIRECTORY ENTRY, and here it authorizes reading the
      PRE-migration file. Rulings: (a) *Scope — both consumers of the pointer, one helper.* A shared
      `read_current_pointer_marker(fs, path) -> Result<Option<CurrentGenerationPointer>, io::Error>` (sibling of
      `read_completed_marker`, same `Fs`-seam lstat probe) is called by `detect_migration_read_state` (reader;
      errors become `BcIndexAddressingError::Io`, exit-class unchanged) AND by
      `read_current_generation_pointer_if_present` (the coordinator's STAGING-resume probe: `Err` is returned as
      `BcIndexMigrationError::Io`, exit 2, before any recheck, swap, discard or write). Leaving the coordinator
      probe on `fs.read(..)` + `.ok()` would let a dangling or corrupt pointer read as "this generation has not
      committed", route a possibly COMMITTED generation into `discard_incomplete_staging` and violate Invariant 3;
      the pointer is written write-temp + `F_FULLFSYNC` + rename, so an unreadable or non-record pointer is
      external damage, exactly the argument of point 3. (b) *Verdicts.* (i) lstat `NotFound` ⇒ `Ok(None)`: absent
      (reader: continue to step 3, `NotStarted`; coordinator: "no evidence of a committed swap"). (ii) Any other
      lstat error, a dangling symlink (the read returns `NotFound` of the TARGET), a directory, EACCES, ELOOP,
      EIO ⇒ `Err(io)`. (iii) Content that is not a pointer record ⇒ `Err(InvalidData)`: not UTF-8, empty,
      truncated, unparseable, not an object, or an object that does not deserialize as `CurrentGenerationPointer`
      (`generation_id`, `status`, `txn_id` all strings; unknown extra keys ignored). (iv) A record whose `status`
      is not exactly `"committing"` ⇒ `Err(InvalidData)` (the writer never produces another value while the file
      exists; the earlier "any other status ⇒ `NotStarted`" is superseded: an uninterpretable pointer is not
      "migration not started"); a `generation_id` that is empty or is not a single path component (contains `/`,
      `\`, NUL, or equals `.` / `..`) ⇒ `Err(InvalidData)` (the reader joins it onto `migration-state/`; the earlier
      `unwrap_or_default()` produced `Committing { generation_id: "" }`). (v) A valid record ⇒ `Some`. The serde
      error is never carried as its message (item 11(g)(2)): the `InvalidData` source text is the category form
      `parse_failure_message`. (c) *Reader precedence unchanged:* `completed.json` is classified first; an `Err`
      there is returned before `CURRENT.json` is touched; a valid `completed.json` still wins over any
      `CURRENT.json` (no `CURRENT.json` read at all). (d) BC-1.18.010 §Reader Integration step 2 is the owed
      wording; `backfill-append-logs` applies the identical rule to its own pointer
      (`CURRENT-backfill-append-logs.json`) wherever S-25.06 reads it.

   (d) *The build MUST NOT create the blocked state itself and report success — COMPLETED-write errors
   propagate.* `finish_committing_migration` writes `completed.json` (the commit point) and then rewrites the
   txn `COMPLETED`. The txn rewrite is no longer best-effort: its error MUST be propagated (`Io`, exit 2) and
   the function MUST NOT return `Completed` (exit 0) while the own txn is still live beside `completed.json`;
   the best-effort `let _ = write_txn_record(..)` is a TD-VSDD-059 paper-fix of exactly the class item 8(b)
   removed from the `EXPIRY_ABORT` path (otherwise ENOSPC/EIO/EACCES on the rename yields a run that reports
   success and leaves the state that (b) then blocks forever). On that error the gate is NOT written OPEN
   (`gate=OPEN ⇒ no live txn` must stay true) and the stderr line is the write's own `Io` error. This does
   not remove the crash window (a crash between `completed.json` and the txn rewrite cannot be closed by
   code: `completed.json` is the commit point and the txn update is bookkeeping after it), which is why (e)
   exists. The gate-OPEN write after a SUCCESSFUL txn COMPLETED write stays best-effort (warn on error): that
   state self-heals through Branch A / §5c Branch 2 and no live txn remains. *Sibling sweep (TD-VSDD-060)* of
   every `let _ =` that discards a txn-record or gate write in `shard_manager.rs`: (1) the second
   `let _ = write_txn_record(..)` in the step-3b abort closure (`abort_staging`: census / content-preservation
   failure) — it ignores the ABORTED write and then writes gate OPEN regardless, which can leave a live
   STAGING txn beside an OPEN gate; (2) the four `let _ = discard_incomplete_staging(..)` calls in the
   `ResumeFromStaging` and fingerprint/canonical-I/O failure arms, each followed by an unconditional
   `let _ = write_admission_gate_state(.., Open)` in two of them (item 8(b) already removed this pattern from
   the `EXPIRY_ABORT` arms; it is extended here). Rule for every abort-path cleanup: write the txn record
   ABORTED and check the result; ONLY if it succeeded write the gate OPEN and check that; the first
   failure is returned as that write's `Io` (exit 2) with the ORIGINAL failure's code token named in its
   `detail`, so the root cause is still visible; the original error alone is returned only when both writes
   succeeded. (The inert-gen-dir `fs.remove` stays best-effort, since a failure only leaves an unreferenced
   directory, but **(v1.26, closes F-S2509-L2-006) the failure is never swallowed:** the step-3b `abort_staging`
   closure's `let _ = fs.remove(gen_dir)` is replaced by the same shared helper `discard_incomplete_staging`
   uses; when the removal fails, the closure emits `STAGING_DIR_REMOVE_FAILED` (item 11(f)(2)) AFTER its
   ABORTED txn write has succeeded (the advisory clause "the txn is ABORTED" must be true) and BEFORE it writes
   the gate OPEN, so a later gate-write failure still leaves the advisory true and visible beside the exit-2 `Io`
   line. The fresh-run abort and the resume-path abort are one rule; consistency is the whole reason the token
   exists.)

   (e) *Operator procedure for the interim block (documented remedy for the "operator investigation
   required" line, until S-25.06 AC-031 supplies the verifier).* Applies ONLY to the state `completed.json`
   present + own live txn (`COMMITTING`, or `STAGING` per (b)) + exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` with
   the "no verification was performed in this build" line, and to the identical `E-MAINTENANCE-001` +
   mismatch-suffix block at admission. It is performed by the HUMAN operator in a terminal OUTSIDE any agent
   session (the guard layer blocks agent writes to `migration-state/`; agents MUST NOT perform or script it),
   with NO migration coordinator process running (`flock(exclusive.lock)` acquirable, e.g. `flock -n
   .factory/migration-state/exclusive.lock true` exits 0). Steps, in order; stop at the first failed check and
   leave all files unchanged (a failed check means a real mismatch — escalate, do not force):
   1. *Snapshot.* Copy the whole `.factory/migration-state/` directory and the four affected canonical
      paths to a safe location before any edit.
   2. *Identity binding.* Read `completed.json` (`txn_id`, `generation_id`, `canonical_paths_count`) and the
      live `txn-<activation_uuid>.json` (`txn_id`, `activation_id`, `generation_id`, `state`,
      `canonical_move_plan`, `intent_log_path`). Require: `completed.json.txn_id` == the txn's
      `activation_id` (Decision 7a: the txn's own `txn_id` is "same as `activation_id`", so the txn's `txn_id`
      must equal it too; a value such as `txn-<activation_id>` is NOT equal — it is a mismatch, stop);
      `completed.json.generation_id` == the txn's `generation_id` (string); the txn's `migration_id`, if
      present, == the migration being finalized; `canonical_paths_count` == N, the number of canonical paths in
      the txn's move plan, i.e. the number of `canonical_move_plan` entries (v1.24 rename; N >= 1 — an EMPTY
      plan fails this step). N is txn-specific for
      `migrate-bc-index` (one entry per staged shard/sub-shard/manifest file plus `BC-INDEX.md`), NOT 1 (the
      fixed N = 4 belongs to `backfill-append-logs`). Cross-check (now mandatory, ADR-054 B-3): the set of
      `canonical_path` values of the plan must EQUAL the set of distinct `target_canonical` values among the
      intent log's `INTENT` records (the WAL, written before any rename), and each plan `staging_path` must
      equal the INTENT's `staging_path` for the same target (checked in step 3 once the log is verified).
   3. *Content binding.* `intent_log_path` MUST be a non-null string denoting
      `.factory/migration-state/intent-<generation_id>.log` (Decision 7b; Decision 7c step 1 persists it
      together with `generation_id`); a null or absent value, or any other path, fails this step (stop;
      never substitute the derived path). The log is plain text — 11-line `key=value` record blocks
      (§Decision 7b → ADR-054 Decision 1.2), NOT JSON. VERIFY EVERY RECORD with the ADR-054 Decision 1.5
      recipe (`sed -n "S,S+8p" intent-<generation_id>.log | shasum -a 256`, `S` = 1, 12, 23, ... — the
      record's checksum is its line S+9); walk from `S = 1` and stop at the first record that fails. Bytes
      after the last verifying record are a TORN TAIL (absent) ONLY IF no later `INTENT_LOG_RECORD_V1` line
      starts a record that verifies; if one does, the log is corrupt mid-stream: STOP and escalate (do not
      proceed to step 4). All records must carry `txn_id` == the txn's `activation_id`. Then (a) the plan /
      INTENT-set equality of step 2 holds, with each plan `staging_path` equal to the INTENT's; and (b) for
      EVERY `canonical_path` in `canonical_move_plan` find the verifying block whose `target_canonical`
      equals it and whose `record_type` is `DONE` (never only `INTENT`) AND the verifying `INTENT` block
      for the same target, and require `DONE.expected_post_hash` == `INTENT.expected_post_hash` ==
      `sha256(<canonical_path file>)` (a DONE hash that equals the file but not the INTENT proves nothing).
   4. *Finalize the txn.* Only if 2 and 3 all hold: rewrite the txn file with `state` set to `COMPLETED` and
      `updated_at` set to the current UTC time, every other key byte-for-byte preserved, using
      write-temp + fsync + rename + directory fsync (the §7d discipline; never edit in place).
   5. *Open the gate — only after step 4 is durable.* Set `.factory/migration-state/gate-state.json` (the
      physical name of the logical gate-state file) to the bare JSON string `"OPEN"` (uppercase; the file's
      whole content is that one JSON string, as for `"DRAINING"`/`"LOCKED"`), by the same atomic write. This
      order is mandatory (`gate=OPEN ⇒ no live txn`).
   6. *Confirm.* Re-run `factory-dispatcher migrate-bc-index` (the pinned absolute path): it must exit 0 with
      zero writes. The binary prints NOTHING on exit 0 (v1.23 ruling "Exit-0 silence" below), so the check is
      the exit status (`echo $?` is `0`) plus the on-disk state (txn `state` is `COMPLETED`, `gate-state.json`
      is `"OPEN"`); then confirm a normal `.factory/` write is admitted.
   If step 2 or 3 fails, do NOT advance the txn or open the gate: the on-disk migration is in an unproven
   state; restore from the step-1 snapshot or the pre-migration git history and escalate. S-25.06 AC-031
   replaces this manual procedure with the shared verifier executing exactly steps 2-5 under
   `flock(exclusive.lock)`; the procedure also enters the operator runbook (devops-engineer deliverable of
   S-25.09).

   (f) *Coordinator stderr: the "Coordinators" clause of BC-1.18.013 Postcondition 10 is an OPEN set (v1.25;
   closes F-S2509-L1-006).* The clause reads "**each** failure or advisory they
   previously sent only to `tracing` (`migrate-bc-index: migration failed`, the drain's reservation-staleness
   advisories, the live-coordinator warning of Postcondition 5) is written to stderr". The universal "each" is
   the operative term and the parenthetical names three heterogeneous instances (a terminal failure, a
   non-fatal advisory, a cross-process warning), so it is a list of EXAMPLES; reading it as closed would leave
   every other tracing-only anomaly invisible to the CLI operator, which is the defect the clause exists to
   remove (SOUL.md #4: no silent partial failure). Its bounds: it covers `warn!`/`error!` conditions on the
   coordinator process that change what the operator must know about the on-disk outcome (a failure, or a
   completed-but-degraded state, or a skipped step); it does not cover `debug!`/`info!` progress traces, and
   it does not duplicate a condition whose own exit-2 line already carries the reason (`RecoveryRequiresReauthorization`,
   the item-8 `EXPIRY_ABORT` lines, every returned variant). Shapes (stdout stays empty; one line per
   condition; the detail is rendered by the same `sanitize_diagnostic` as the drain's lines: control characters
   escaped, 256 characters):
   1. *Failures* keep the existing form: `<subcommand>: ` + the variant text, exit 2 (or 1 for the exit-1 class).
   2. *Non-fatal advisories* (exit status unchanged, normally 0): the general shape is `<subcommand>: <TOKEN>
      (advisory): <subject>: <cause>; <one clause saying what is true now>`, with exactly three slots after the
      token. `<subject>` is what the condition is about: the affected filesystem path for an OS-failure advisory,
      or the affected entity identifier (a BC id) for a non-OS advisory. `<cause>` is why the line exists: the
      OS error text for an OS-failure advisory, or the measured fact that tripped the condition for a non-OS
      advisory. The OS-failure form is therefore `<subcommand>: <TOKEN> (advisory): <path>: <os error>; <what is
      true now>` (the three OS advisories below); the non-OS form carries no `<os error>` and no fabricated
      path. A non-OS advisory fixes its own `<subject>`/`<cause>` template in the domain list below. All
      interpolated values (path, id, error text, numbers rendered as decimal) pass through `sanitize_diagnostic`
      **per slot (v1.25 clarification, F-S2509-L1-006 implementation ruling):** `<subject>` and `<cause>` are
      EACH escaped and capped at 256 characters (the cap includes the one-character truncation marker, exactly
      as `sanitize_diagnostic` defines it); the trailing `<one clause saying what is true now>` is FIXED text,
      is NEVER truncated or sanitized, and is NOT counted against either slot's cap. There is deliberately NO
      whole-line cap: a whole-detail cap would have to truncate the path (the identifier the operator must act
      on) or the clause (which must stay true and complete), and the line length is bounded by construction
      (at most 256 + 256 interpolated characters plus fixed text of the closed token/clause domain below). The
      earlier wording "the whole rendered line is capped at 256 characters of detail after the `(advisory): `
      marker" is superseded by this per-slot rule. The same per-slot rule applies to the numeric fields of the
      `OVERSIZED_ROW_SUBSHARD` template (rendered as decimal, never truncated; they are inside `<cause>`, whose
      cap still applies to the rendered `<cause>` text).
      `<TOKEN>` is UPPER_SNAKE and drawn from a CLOSED
      coordinator-advisory domain (not taxonomy codes: no `error-taxonomy.md` row; exactly the status of the five
      reservation-timestamp stderr tokens, which keep their existing `drain_bc_index_writers: ...` lines
      unchanged). The domain for this build: `GATE_OPEN_RESET_FAILED` — `finish_committing_migration`'s
      best-effort gate-OPEN write after the txn reached COMPLETED failed (exit stays 0; "the migration completed;
      the gate stays <state> until the next admission check or `migrate-bc-index` run reconciles it", which must
      be true and must not claim OPEN); `TERMINAL_TXN_ARCHIVE_FAILED` — `archive_terminal_txn_record`'s
      best-effort rename failed with a non-ENOENT error ("the stale terminal record stays in place and is
      skipped as non-live"); `STAGING_DIR_REMOVE_FAILED` — the inert
      generation-directory removal of EVERY abort path failed (v1.26: `discard_incomplete_staging` AND the
      fresh-run step-3b `abort_staging` closure, item 11(d); "the orphaned generation directory is inert; the txn is
      ABORTED"); and `OVERSIZED_ROW_SUBSHARD` for the lone-oversized-row condition of
      `chunk_subsystem_rows_into_sub_shards` (CONFIRMED reachable from a coordinator process by the test-writer's
      trace `run_bc_index_migration_core` fresh-run -> `chunk_subsystem_rows_into_sub_shards`;
      exit stays 0). **Collection and emission rule (v1.26; closes F-S2509-L2-002/003).** The fixed clause ends
      "and the migration completed", which is a claim about the WHOLE run; emitted from inside the chunker at
      STAGING time it is false whenever a later step fails (census, fingerprint recheck, intent append, pointer swap,
      a canonical-move halt) and the run exits 2. Therefore: (1) *The chunker is pure and emits nothing.*
      `chunk_subsystem_rows_into_sub_shards` and `close_sub_shard_chunk` call neither `emit_coordinator_advisory`
      nor `tracing::warn!` and keep their signatures; the oversized fact is derived from the returned chunks by a
      pure function `oversized_row_advisories(chunks: &[SubShardChunk], shard_cap_bytes: u64) ->
      Vec<OversizedRowAdvisory>` (`OversizedRowAdvisory { bc_id: BcId, sub_shard_id: String, body_bytes: u64,
      shard_cap_bytes: u64 }`; a chunk is oversized iff `body.len() as u64 > shard_cap_bytes`, which by the packing
      invariant holds only for a lone-row chunk; `bc_id` = the chunk's `range_start`; order = input order). It may
      be `pub` because it returns data; it performs no I/O and no logging. (2) *Collected, then printed on
      `Completed` only.* `run_bc_index_migration_core` appends the advisories of every over-cap subsystem to one
      `Vec<OversizedRowAdvisory>` in build order (subsystem ascending, then sub-shard order) while it stages, and
      prints them (one line each, plus the retained `tracing::warn!`, from a module-private
      `emit_oversized_row_advisories`) ONLY when the fresh run's final result is `Ok(Completed)`, i.e. immediately
      after `finish_committing_migration` returns `Ok`, after any line `finish_committing_migration` itself emitted
      (`GATE_OPEN_RESET_FAILED`, `TERMINAL_TXN_ARCHIVE_FAILED`). On any `Err` return, or on any non-`Completed`
      `Ok`, they are DISCARDED unprinted: the failure line carries what the operator must act on, and an advisory
      whose clause says "completed" would be false. (3) *Resumed runs: neither re-derived nor persisted.* A resumed
      STAGING txn (`ResumeFromStaging`) re-reads its staged generation and a COMMITTING recovery
      (`ForwardRecovery`/Branch C) never builds shards, so neither has a chunker output, and neither prints this
      advisory. The advisory is a property of the build performed by the printing process; it is not part of the
      durable migration state. Persisting it would need a twelve-key txn-record schema change (item 10
      strict-presence) or a new durable file for an informational line about a condition that does not affect
      correctness, and re-deriving it from staged files is possible on one resume arm only and would make the
      three arms disagree. The over-cap sub-shard itself stays visible on disk (its file is larger than
      `shard_cap_bytes`). A run that crashed before `Completed` therefore leaves no advisory, by design, and this
      is not a deferred item. It involves no OS failure, so it uses the non-OS form, one line per oversized sub-shard:
      `<subcommand>: OVERSIZED_ROW_SUBSHARD (advisory): <bc_id>: sub-shard <sub_shard_id> body is <body_bytes>
      bytes, exceeding shard_cap_bytes <shard_cap_bytes>; the row is emitted as its own over-cap sub-shard (not
      split, not failed) and the migration completed`. Fields: `<bc_id>` = the lone row's BC id
      (`range_start`, equal to `range_end` for a lone row); `<sub_shard_id>` = the sub-shard identifier that
      names its file (the chunk's `sub_shard_id`; the function holds no directory, so no path is rendered and
      none is invented); `<body_bytes>` = the closed chunk's byte length (preamble + row + newline, i.e. the
      value compared to the cap, so the line is never understated); `<shard_cap_bytes>` = the cap in force. The
      trailing clause is true BECAUSE the line is printed only on `Completed` (v1.26; the earlier "true by
      construction (the chunk is pushed unconditionally)" proved only the over-cap-chunk half of the clause, not
      the "migration completed" half). The `tracing::warn!` is retained alongside the stderr line, emitted by
      `emit_oversized_row_advisories` at the same moment (not from the chunker). The
      implementer greps every `tracing::warn!`/`error!` reachable from `run_bc_index_migration_core`,
      `drain_bc_index_writers` and `finish_committing_migration`, classifies each by the bounds above, and
      records the list in the burst evidence; a site not classified is a finding.
   3. *Exit-0 silence (the v1.23 "(v) Exit-0 silence" ruling) is amended:* an exit-0 run prints nothing EXCEPT advisory
      lines of item 2 (a degraded success is not silent success); stdout remains empty, callers still branch on
      the exit status only.
   4. *Canonical-move halt reasons are NOT an S-25.09 deliverable.* Each halt site in
      `execute_canonical_path_moves` (decision `FailClosed`, parent-directory create, rename, directory sync,
      post-rename verification read missing/failed, `DONE` append failure) is converted by S-25.11 (ADR-054
      Decision 3, ADR-054 AC-015 = S-25.11 story AC-008) and its reason is carried by `CANONICAL_MOVE_HALTED (exit 2): move to <target>
      halted: <reason>; ...` (ADR-054 §3.1 final text, already specified). Any S-25.09 work on those sites
      (a reason threaded through the `BINARY_INTEGRITY_FAILURE` count-shortfall carrier, which also mislabels an
      I/O halt with the digest code, the F-012 class) would be deleted by S-25.11. This is a story-partition
      ruling anchored to a concrete AC (ADR-054 AC-015 = S-25.11 story AC-008), made under the human decisions
      of 2026-10-08 that created the S-25.10/11/12 chain; not a tech-debt entry. S-25.09 emits NO interim halt
      line. Binding on S-25.11 AC-008: `<reason>` covers ALL SIX halt sites above (ADR-054 v1.1 §Decision 3
      widens the trigger text and the `<reason>` domain from the earlier four to all six), each reason
      sanitized as above; and the pass-1 black-box vector for halt reasons moves from S-25.09 to S-25.11 with
      the halt-site list as its vectors.

   (g) *Sanitization caps are per SURFACE, and a data-derived value is sanitized at RENDER time, never at
   construction (v1.25; closes the Event 12 / operator-line cap conflation).* Two surfaces render the same
   `AdmissionStateIntegrity` / `Io` facts and have different caps:
   1. *`InternalLog` events (BC-3.08.001 Events 11-13, incl. `migration.admission_failed.detail`):* each
      data-derived substring is escaped and capped at **64** characters (the BC-3.08.001 Event 12 rule; unchanged).
      This is the log-injection hygiene bound for a structured, queryable field.
   2. *Operator stderr (the coordinator `migrate-bc-index: MIGRATION_STATE_INTEGRITY_FAILURE ...` / failure
      line, and the Display of any `BcIndexMigrationError`):* each data-derived substring is escaped and capped
      at **256** characters (the item 11(f) rule: "the same `sanitize_diagnostic` as the drain's lines"). The
      operator line is the only place the operator learns WHICH file is malformed; a 64-character cap would cut
      a macOS project path (~108 characters) before its file name. The 64-character rule does NOT apply here.
      The `E-MAINTENANCE-002 (<cause>)` hook verdict message carries the cause token only and neither cap.
   Rule for the code: `AdmissionStateIntegrity` (and every variant whose Display/event both quote data) carries
   its data-derived slots RAW (an unsanitized `subject`, e.g. the path, and a `message` that is a fixed string or
   a serde error category) in addition to `kind`; it is NOT a pre-sanitized `detail: String`. Display renders
   `sanitize_diagnostic(subject, 256)` + `message`; `admission_error` builds the event `detail` from the SAME
   fields with `sanitize_diagnostic(subject, 64)` + `message`. Two renderings, one source, no re-truncation of an
   already-escaped string (re-truncating would also bypass the per-substring semantics). The v1.21 phrase
   "`detail` is the already-sanitized path/parse message" in §Admission state-integrity variant is read as "the
   path/parse message, sanitized by the surface that renders it". `message` MUST NOT carry on-disk record
   content (unchanged); an unknown-key name is data-derived and goes through the same per-surface cap.
   **`Io` and the other path-bearing variants (v1.26 confirmation; closes F-S2509-L2-001).** The scope sentence
   "the Display of any `BcIndexMigrationError`" is operative for EVERY variant, and for the two interpolations of
   `Io { path, source }` it is stated here explicitly: (a) `path` is data-derived (it can be a link target or a
   name taken from a record, e.g. a plan `canonical_path` up to 4096 bytes) and is rendered
   `sanitize_diagnostic(path, 256)`; (b) `source` is data-derived (an OS error text can embed a path; an
   `InvalidData` source wraps a parser message) and is rendered `sanitize_diagnostic(source, 256)`, as a SEPARATE
   substring, so a 256-character path never starves the source of its cap, and the two are never joined and then
   capped as one string. `#[error("BC-INDEX migration: I/O error at {path}: {source}")]` interpolating the raw
   values is therefore a violation; the Display is written with the sanitized values. (c) *Serde messages are
   never carried.* A `serde_json` error message echoes record content (`invalid type: string "<content>"`,
   `unknown variant`, a duplicate key name); an `InvalidData` source built from one is built from the category form
   (`parse_failure_message`: category, line, column) at the SOURCE of the `io::Error`, matching the existing
   `AdmissionStateIntegrity` `message` rule, and the 256 render-time cap remains as the second line of defence for
   every other source text. Sites to sweep (TD-VSDD-060): every `io::Error::new(InvalidData, <serde error or
   e.to_string()>)` in `shard_manager.rs` that reaches an operator or hook message — `read_completed_marker`, the
   `detect_migration_read_state` `CURRENT.json` parse (now `read_current_pointer_marker`), the txn/intent-log
   decode sites and the `e.to_string()` site; the implementer greps `InvalidData` and records the list.
   (d) `CleanupWriteFailure` (the `Io` source of an abort-path cleanup failure, item 11(d)) has TWO data-derived
   substrings: `cause` (the write's own OS error) and `while_handling` (the original failure's code token, else
   its rendered text, which is data-derived when the original is an `Io`). Its Display renders each as
   `sanitize_diagnostic(_, 256)` — `<cause> (while handling <while_handling>)` with two independent caps. Because
   `Io`'s Display sanitizes `source` as a whole, it MUST special-case this source (downcast the `io::Error`'s inner
   error to `CleanupWriteFailure` and render its own two-substring form rather than re-capping the joined
   string): a single 256 cap on the joined text would truncate exactly the "while handling" tail, which is the
   root cause item 11(d) exists to keep visible. `sanitize_diagnostic` is idempotent on already-escaped text, so
   the nested rendering never double-escapes. (e) *Siblings.* `BcIndexAddressingError::{Io, Toml, MalformedBcId}`
   (the reader's variants, rendered into a hook result) follow the same rule: the path and the source/candidate
   text are each escaped and capped at 256 (the `Toml` message is replaced by its error category and span, never
   the offending source line). The 64-character cap is not involved: no `InternalLog` event renders these.
   Hence: BC-1.18.013 Postcondition 10 "Coordinators" clause states the 256 operator cap; BC-3.08.001 Event 12
   (`detail`) and Invariant 7 keep 64 and add "event only".
   **Scope of the rule (v1.25 consistency extension): EVERY operator stderr line, not only the
   `AdmissionStateIntegrity` / `Io` lines.** One rule, no per-code exceptions: the 64-character cap applies to
   `InternalLog` events and to nothing else. The operator-stderr lines that render a data-derived value are, in
   addition to the coordinator failure line above: item 9 `FOREIGN_MIGRATION_REFUSED` `<id>` (256), and the three
   ADR-054 §Decision 3.1 lines `INTENT_LOG_CORRUPT` `<log_file>`, `INTENT_LOG_VALUE_REJECTED` `<field>` (a closed
   token, never truncated in practice) and `CANONICAL_MOVE_HALTED` `<target>` (256 per substring; ADR-054 3.1
   restated accordingly). No deliberate 64 on stderr was identified: a 64-character stderr cap would cut a
   canonical path (validated up to 4096 bytes, ADR-054 Decision 1.2) mid-name, which is the defect this item
   closes. The shared sanitizer takes the cap as a parameter (`sanitize_diagnostic(s, cap)`); the admission
   diagnostic and the coordinator renderers pass 256, `admission_error`'s event `detail` passes 64.
   **Code affected:** S-25.09 (item 9 `FOREIGN_MIGRATION_REFUSED` `<id>` renderer in `BcIndexMigrationError` /
   `AppendLogMigrationError` Display, 64 → 256; plus a red test with a 200-character hostile `migration_id` asserting
   the full escaped id on stderr). ADR-054 sites are not implemented yet (S-25.10 owns `INTENT_LOG_VALUE_REJECTED`
   and `INTENT_LOG_CORRUPT`; S-25.11 owns `CANONICAL_MOVE_HALTED`): they implement 256 from the start, no rework.
   **Owed by the product-owner (mirror, POLICY 8/9 same burst):** BC-1.18.011 (the ~1466 / ~1487 / ~1806 sites that
   state 64 for the stderr placeholders and `<id>`), BC-1.18.013 (the ~1549 / ~1559 / ~1832 sites), and
   error-taxonomy lines ~113-116 (the `INTENT_LOG_*`, `CANONICAL_MOVE_HALTED` and `FOREIGN_MIGRATION_REFUSED`
   rows): replace "truncated to 64" with "each data-derived substring escaped and capped at 256 (operator stderr)",
   and leave any `InternalLog`-event 64 untouched.
   3. *Coordinator advisories are emitted only from the coordinator process.* `emit_coordinator_advisory` (plain
      `eprintln!`) is correct ONLY because every call site is in the coordinator call tree
      (`run_bc_index_migration_core` -> `finish_committing_migration`, `discard_incomplete_staging`,
      `archive_terminal_txn_record`, `chunk_subsystem_rows_into_sub_shards`/`close_sub_shard_chunk`), none
      reachable from the dispatcher's PreToolUse/PostToolUse admission path (stderr of a hook process is hook
      feedback to the agent, not an operator line). Binding rules: (i) `emit_coordinator_advisory` and its
      call-site helpers stay module-private (`fn`, not `pub`/`pub(crate)`); **(v1.26)** the "call-site helpers"
      are the EMITTING helpers (the `discard_incomplete_staging` / `abort_staging` removal reporter,
      `archive_terminal_txn_record`'s reporter, `finish_committing_migration`'s gate reporter and
      `emit_oversized_row_advisories`); a function that only RETURNS advisories as data
      (`oversized_row_advisories`, and the pure chunker, which returns none) is not an emitting helper, may be
      `pub`, and MUST NOT print, log or touch stderr — that is what keeps the `pub` chunker "pure"
      (deterministic, no I/O, no logging). No function in the call tree of `chunk_subsystem_rows_into_sub_shards`
      may call an emitting helper; (ii) no function reachable from
      `migration_writer_admission`, `migration_writer_release` or `read_active_txn_record` may call them (the
      implementer records the grep of callers in the burst evidence and adds a unit test that drives the
      admission entry points over a state that WOULD trip each advisory and asserts nothing is written to
      stderr); (iii) a future advisory condition reachable from a dispatcher entry point MUST be an Event 13
      (`migration.admission_advisory`) instead, or the shared helper MUST return the advisory as data for the
      coordinator caller to print. A structural data-returning refactor of the four existing sites is NOT
      required **except the `OVERSIZED_ROW_SUBSHARD` site (v1.26)**, which is the one site that is not a
      best-effort effect reporting its own outcome but a statement about the whole run: it is returned as data
      and printed on `Completed` (item 11(f)(2) "Collection and emission rule"). For the other three the original
      reasoning stands (sink plumbing through `Fs`-seam helpers whose only callers are coordinator-side, for no
      reachable benefit); rules (i)-(iii) make the invariant checkable and a violation a test failure.

**v1.21 additions (closes F-012 / D-2; dedicated variants — see §5a "Admission state-integrity
variant" and "Single anchoring rule"):**

| Error / variant | Layer | Trigger | Severity | Exit / surface |
|---|---|---|---|---|
| `MIGRATION_STATE_INTEGRITY_FAILURE` — `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }` | migration binary (coordinator reading the shared gate/txn state); the SAME variant is the `state_integrity` cause of `E-MAINTENANCE-002` on the guard layer | malformed `gate-state.json`; malformed txn record (not an object / unknown `state` / unparseable); a PRESENT non-string `migration_id`; more than one live txn; reservation serialization failure; **(v1.23)** a Tier 1 txn field the executing recovery/verify/finalize arm consumes that is absent or ill-typed (`generation_id` tri-state, COMMITTING `generation_id` not a string, `activation_id`, `intent_log_path`, any field of the strict-presence decode — §Error Code Semantics "Migration binaries — recovery and finalize legs"). **(v1.23, post-validation)** unknown key inside a `canonical_move_plan[]` element (v1.24 rename of `pending_canonical_moves[]`), an EMPTY or ill-typed `canonical_move_plan` or one that differs from the intent log's INTENT target set on a COMMITTING record (v1.24, ADR-054 B-3), or a missing/ill-typed `schema_version` (item 10(a)/(b)), are `txn_record_malformed`; a `schema_version` integer ≥ 2 is the DISTINCT kind `txn_record_newer_schema` (item 10(c): "written by a newer build; recover with that build", not corruption; same code, same exit). Distinct from `E-BINARY-INTEGRITY-FAILURE` (digest/TOCTOU) and from `COMPLETION_RECORD_MISMATCH_ABORT` (well-formed fields that disagree) | BLOCKED — nothing finalized, no gate flip | exit 2 |
| `FOREIGN_MIGRATION_REFUSED` — `BcIndexMigrationError::ForeignMigrationRefused { live_migration_id }` (v1.23 item 9; mirrored by `AppendLogMigrationError`) | migration binary (both coordinators, recovery and `completed*.json` short-circuit) | a single LIVE txn record whose `migration_id` is the other known migration's or `∉ K`; read after the Tier 0 loader and the one-live-txn check, before `recover()`, no Tier 1 field read | BLOCKED — nothing mutated, nothing recovered/finalized/aborted | exit 2 |
| `MIGRATION_LOCK_CONTENTION` — `BcIndexMigrationError::MigrationLockContention` (v1.23 item 9 sibling; replaces the digest-coded `BinaryIntegrityFailure` lock message) | migration binary (both coordinators) | `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` EWOULDBLOCK on a coordinator run, on EVERY path including when `completed.json` is present (v1.23 item 11(c); never exit 0) | NON-ERROR termination — nothing read or changed; retry after the other coordinator exits | exit 1 |
| `FACTORY_ROOT_NOT_FOUND` — `BcIndexMigrationError::FactoryRootNotFound { project_root, source }` | migration binary (both coordinators) | the resolved session project root (`resolve_session_project_root`) has no `.factory` directory; the coordinator never creates `.factory` | BLOCKED — nothing created or mutated | exit 2 |
| `INTENT_LOG_CORRUPT` — `BcIndexMigrationError::IntentLogCorrupt { path, kind, offset }` (v1.24; ADR-054 Decision 1.7/1.8) | migration binary (both coordinators; the verification surfaces map the same condition to `canonical_hash_mismatch`) | intent-log mid-log corruption (bytes after the valid prefix followed by a valid record) or an L1-L4 log-invariant violation | BLOCKED — nothing moved or appended | exit 2 |
| `INTENT_LOG_VALUE_REJECTED` — `BcIndexMigrationError::IntentLogValueRejected { field, reason }` (v1.24; ADR-054 Decision 1.2/1.9) | migration binary (both coordinators) | a plan path or intent-log field violates the ADR-054 value rules at plan-build or write time (LF/CR/NUL/C0/DEL, non-UTF-8, over 4096 bytes, bad hash/timestamp/`txn_id`) | BLOCKED — nothing staged or appended; pre-commit abort path if raised after the generation directory exists | exit 2 |
| `CANONICAL_MOVE_HALTED` — `BcIndexMigrationError::CanonicalMoveHalted { target, reason }` (v1.24; ADR-054 Decision 3) | migration binary (both coordinators) | during COMMITTING a move cannot be completed: recovery decision `FailClosed`, rename or directory-sync failure, or post-hash divergence from the INTENT's `expected_post_hash` | BLOCKED — txn stays COMMITTING; forward recovery re-runs idempotently | exit 2 |

`E-MAINTENANCE-002` message format (single line):
`E-MAINTENANCE-002: writer-admission check failed (<cause>)` with `<cause>` ∈
{`invalid_tool_use_id`, `io`, `state_integrity`}; it replaces the unnamed
`BC-1.18.011: writer-admission check failed: {e}` string. The variant's Display carries its
code token; the underlying detail (path, `io::Error`, byte length — never a raw id or record
content) goes to the `migration.admission_failed` InternalLog event (v1.21). `error-taxonomy.md` therefore gains exactly two catalog rows
(`E-MAINTENANCE-002`, `RESERVATION_TTL_BELOW_FLOOR`) — see §Downstream v1.20 deltas.

**Note:** One catalog row per code was added to `error-taxonomy.md` (v1.25). The `exit 0`
for `ALREADY_MIGRATED` is a deliberate sentinel — callers that re-invoke the migration binary
after completion must not treat this as a failure; `ALREADY_MIGRATED` is emitted only for a state read
UNDER `exclusive.lock` (lock not acquired is `MIGRATION_LOCK_CONTENTION`, never `ALREADY_MIGRATED`; item 11(c)).
**`exit 1` means "no harm done, safe to re-run; the stderr code token says what to do next"** — it is a
class covering `EXPIRY_ABORT` (re-activate, then re-run) and `MIGRATION_LOCK_CONTENTION` (re-run after the
holding coordinator exits), distinct from the `exit 2` fail-closed codes that need an operator (item 7(e), as
amended). `process_exit_code` is an exhaustive match with no wildcard arm.

---

## Rationale

### Head-to-head mechanism evaluation

#### Option A — Hook-driven explicitly-armed one-shot native action

**Viability: CONFIRMED.** Rationale unchanged from v1.2. Option A remains viable; not chosen on
operational complexity grounds. A future recurring migration use case should reconsider Option A.

#### Option B — One-time interactive Bash approval at F4 (CHOSEN)

Unchanged from v1.2. Least privilege; F4 human gate; direct D-449(a) evidence; zero standing
permission surface after migration completes.

#### Option C — Hardened standing allowlist

Unchanged from v1.2. Fails F3 (standing permission ≠ activation authorization); post-migration
cleanup burden; no governance benefit over Option B.

#### Why v1.3 atomic-pointer model was adopted over incremental v1.2 amendment

The 3rd Codex review correctly identified that F1, F2, F5, F9 are all symptoms of the
same root cause: per-file in-place rename cannot provide all-or-nothing multi-file visibility.
Incremental amendments to a per-file-rename model would continue to accrue findings at each
review cycle (confirmed by the diverging trajectory: 7→8→11). The atomic-pointer model dissolves
the shared root cause in one structural change:

- **F1** dissolves because readers now resolve CURRENT.json once and pin one generation — no
  "partially renamed" state is ever visible to a reader.
- **F2** dissolves because the intent log + matching-destination-hash recovery rule closes the
  rename/journal gap without requiring staging files to survive.
- **F5** dissolves because the txn record is separate from the flock; recovery ownership is a
  separate acquisition that does not require clearing intent.
- **F9** dissolves because completed.json is permanent and never archived; its presence is
  unambiguous in all steady states.
- **F6** collapses because there is now a SINGLE pivot point (CURRENT.json pointer swap) rather
  than N sequential renames, making the authorization gate placement unambiguous.

The git-worktree adaptation (prefer committed manifest file over directory symlink; retain fixed
canonical shard paths; use generation staging directory as intermediate read path) was chosen
over the full "generation directories as canonical storage" model to preserve shard files as
ordinary tracked git files while still providing the atomic visibility guarantee.

### Why the advisory flock replaces O_CREAT|O_EXCL + unlink-reclaim (F4, F5)

The `O_CREAT|O_EXCL` design has an inherent inode-split race: two processes can both decide a
lock file is stale, one replaces it, and the other's already-decided unlink removes the live
replacement. Stale lock reclamation via unlink is fundamentally unsafe because `unlink()` removes
a pathname, not an inode's validity. An advisory flock on a stable, never-unlinked inode has no
reclamation race: the kernel releases the lock atomically on process death or fd close, and the
next contender acquires it without any pathname manipulation. PID metadata becomes informational,
not authoritative.

### Why the authorization gate is at the CURRENT.json pointer swap, not at PREPARED→COMMITTED (F6)

v1.2 §Decision 4d placed the expiry check before the "PREPARED → COMMITTED" transition — but
that transition was itself defined as occurring AFTER all renames. An expiry check after the
first rename is on the wrong side of the pivot. The CURRENT.json pointer swap is the first
irreversible action (after it, readers see the migration as in-progress and may use generation
paths). Moving the expiry check to immediately before this single pivot is the correct
architectural position: before, expiry can safely abort with no canonical state changed; after,
expiry must not affect the ability to forward-recover.

### Why fexecve / execveat AT_EMPTY_PATH on Linux, not macOS (F11)

The research brief (§RQ4) confirms `fexecve` and `execveat AT_EMPTY_PATH` are Linux primitives
absent from Apple's documented exec API surface. The "protected staging" alternative (copy to
trusted dir + `UF_IMMUTABLE`) does not achieve fd-binding: it uses a pathname for the exec call,
the staged pathname is still subject to unlink/replace, and `UF_IMMUTABLE` is owner-changeable
without root on macOS. The freeze-build-under-maintenance-lock alternative is weaker but is more
operationally honest: it documents the residual window rather than claiming false security.

---

## Consequences

### Positive

- Resolves all 11 findings from the 3rd Codex cross-vendor closure review.
- Atomic CURRENT.json pointer swap provides genuine all-or-nothing multi-file visibility.
- completed.json as a permanent terminal record eliminates all reader/rerun steady-state
  ambiguity and survives cleanup.
- Framed checksummed intent log with matching-destination-hash recovery makes crash recovery
  both self-describing and idempotent.
- Advisory flock on stable inode eliminates lock-split race and makes stale-owner reclamation
  automatic (kernel-guaranteed on process death).
- Txn record separate from flock provides durable maintenance intent that blocks ordinary writers
  regardless of PID liveness.
- Authorization gate at single pivot point makes the authorization window unambiguous.
- Linux fd-binding exec (execveat AT_EMPTY_PATH) eliminates exec TOCTOU on Linux.
- macOS/darwin-arm64 (primary operator platform) residual TOCTOU window is documented,
  mitigated with programmatic mtime guard and hard pre-flight "no concurrent build" checklist
  item, and requires explicit POLICY 22 human acknowledgment at ratification.
- Platform-branched durability barriers correctly apply F_FULLFSYNC on macOS (Apple-documented)
  vs. fsync+dir-fsync on Linux (POSIX-standard).

### Negative

- Significantly more implementation complexity than v1.2: staging generation dir, framed intent
  log, advisory flock, txn record, CURRENT.json + completed.json pointer, platform-branched
  durability, guard-branch separation, fd-binding exec.
- macOS/darwin-arm64 (primary operator platform) exec TOCTOU residual window requires human
  acknowledgment at POLICY 22 ratification; mitigated by programmatic mtime guard and hard
  pre-flight "no concurrent cargo build" checklist item (v1.4 M5).
- APFS directory-fsync durability is unverified: empirical darwin-arm64 durability test is a
  RATIFICATION PREREQUISITE for macOS safety (§Decision 7d, §Decision 11 sign-off item 2).
- Fault injection test suite between every rename/fsync/intent-record-write step adds test scope.

### Neutral

- BC-1.18.011 Amendment 6 correction (Linux-vs-macOS dir-fsync) changes the spec but not the
  fundamental migration approach.
- The Option B selection (one-time interactive Bash approval) is unchanged.
- The three-way ARCH-INDEX parity check (§Decision 10) is unchanged.

### Verification Strategy (accept-at-floor ratification — D-386 Option C, POLICY 22)

The flock/gate/drain/txn/self-heal concurrency state machine specified in §Decision 4e,
§Decision 5a, and §Decision 7c has been reviewed through **10 adversary passes** (passes 1–9
leading to v1.1–v1.12; pass 10 this v1.13 fix-burst) and has reached the asymptotic floor of
prose adversarial review per **D-386 Option C** (accept-at-floor). Each pass uncovered genuine
defects (4 CRITICAL, 12+ HIGH across the full sequence); the converging finding count and
diminishing structural severity satisfy the Option C asymptotic-floor criterion.

**Definitive verification deferred to implementation-phase formal methods (cluster-5):** The
crash-safety and liveness properties of this state machine — including all reachable stuck-states,
self-heal predicate completeness (§Decision 5a step 3.5), drain quiescence under concurrent writers
(§Decision 5a drain procedure), recovery-mode exhaustiveness (§Decision 4e), and the generation_id
ordering invariant (§Decision 7c step 1) — SHALL be definitively verified in the cluster-5
IMPLEMENTATION via:
- **Kani model-checking harnesses** on the Rust state machine (executor.rs + shard_manager.rs):
  safety properties (no invalid state reachable), liveness properties (no permanent stuck-gate),
  and the self-heal predicate soundness under all crash interleavings.
- **Exhaustive crash-interleaving fault-injection tests:** every drain step → crash → recovery
  path enumerated in §Decision 5a and §Decision 4e, including the null-generation sub-state
  (v1.13 HIGH-1) and the generation-dir absent/incomplete corruption case (v1.13 LOW-1).

**Prose edits to §Decision 4e/5a/7c drain/gate/self-heal machinery are FROZEN** pending
implementation-phase formal verification. Further prose adversary passes on this subsystem will
not produce new actionable findings beyond what is captured in this v1.13 record. Any future
amendment to §Decision 4e/5a/7c machinery MUST be accompanied by a formal proof or exhaustive
model-check result demonstrating the amended invariant holds.

**v1.14 DEF-1 — sanctioned formal-finding exception, applied:** A pre-implementation Kani
model-checking pass against the drain/gate/txn state machine (run as human-directed spec/design
convergence work, ahead of the cluster-5 IMPLEMENTATION Kani harnesses described above) found
DEF-1: the v1.11 step-2.5 reordering made `gate_state = OPEN` reachable while an active
`txn=STAGING(generation_id=null)` record existed with no live coordinator holding
`exclusive.lock` — a state the self-heal predicate (gated on `gate_state ∈ {LOCKED, DRAINING}`)
could never match. §Decision 5a was amended per this frozen-clause's own exception ("MUST be
accompanied by a formal proof or exhaustive model-check result"): the drain ordering was
restructured (initial txn record moved from "step 2.5" to step 3a, after the DRAINING flip) so
that `gate_state = OPEN` and an active STAGING/COMMITTING txn become mutually exclusive by
construction, rather than by an additional self-heal branch. **This exception does NOT reopen
general prose adversarial review of §Decision 4e/5a/7c** — the freeze remains in force for any
change not grounded in a formal/model-check finding.

**Formal-verifier re-verification requirement for this v1.14 fix (VP-M7):** The next Kani pass
against `executor.rs` MUST update its model of the drain procedure and re-prove the joint
gate/txn reachability invariant under the corrected step ordering. Precisely, the harness MUST:
1. Encode the drain procedure's step ordering as: step 2 (flock acquire) → step 3 (gate-state
   write = DRAINING, under gate-state LOCK_EX, released) → step 3a (txn-record write:
   state=STAGING, `generation_id=null`, source hashes null) → step 4 (reservation-quiescence
   poll / timeout-abort) → step 5 (snapshot) → step 6 (gate-state write = LOCKED) → step 7
   (txn-record update with hashes). The txn-record-write transition (step 3a) MUST be modeled
   as occurring strictly after the gate-state-write transition (step 3) in the coordinator's
   program order — this ordering IS the fix; a harness that models step 3a before step 3
   re-introduces the DEF-1 state space and MUST fail the proof below.
2. Enumerate a symbolic crash point after EVERY step boundary in the sequence above (bounded
   model check over all step-boundary crash injections, consistent with the existing "exhaustive
   crash-interleaving" mandate), including the boundary between step 2 and step 3 and the
   boundary between step 3 and step 3a — these two boundaries did not need separate treatment
   under the pre-v1.11 ordering and are the specific boundaries DEF-1 concerns.
3. Prove invariant **INV-GATE-TXN**: for every reachable durable state (crash-truncated or not),
   `gate_state = OPEN` implies no txn record exists with state STAGING or COMMITTING —
   equivalently, `txn.state ∈ {STAGING, COMMITTING}` implies `gate_state ∈ {DRAINING, LOCKED}`.
   The harness must show this holds at every crash point enumerated in step 2 above, i.e. that
   `gate=OPEN ∧ txn.state=STAGING(generation_id=null)` is UNSAT under the corrected model. Under
   the OLD (v1.11–v1.13) step ordering this property is SAT (falsifiable) at the boundary between
   the old pre-DRAINING txn write and the DRAINING flip — the harness change in point 1 is what
   makes it UNSAT, so a diff between the old-model and new-model proof runs is the expected
   re-verification evidence.
4. Re-run the existing self-heal-predicate-soundness and liveness (no-permanent-stuck-gate)
   proofs against the updated model to confirm they are unaffected (Branch A and Branch B of
   step 3.5 are unchanged; only the precondition for when Branch B's state is reachable has
   narrowed).
This is the specific re-verification the formal-verifier must perform before this ADR can be
re-evaluated for accept-at-floor ratification; it is scoped narrowly to the step-3/step-3a
reordering and does not require re-deriving the rest of the state machine's proofs from scratch.

This is the ratification-time acknowledgment for **POLICY 22** (the narrowly-authorized exception
to TD-FACTORY-HOOK-BYPASS-001 P0) — the concurrency mechanism that POLICY 22 gates has been
reviewed to the prose floor (plus this v1.14 formal-finding exception) and its formal
verification is a tracked deliverable of cluster-5 TDD.

**v1.18 formal-finding exception — re-verification obligation (S-25.06 D1–D5).** The v1.18
delta (Branch C, reserve-then-verify admission, coordinator exemption, release-on-block, §7e
namespacing) changes the frozen §4e/§5a/§5c state machine and therefore re-opens the
re-verification requirement for exactly the delta, before S-25.06 may merge: (1) the B2
`obl1_kani_proofs` suite re-run unmodified-or-extended against the shared core (INV-GATE-TXN
must remain UNSAT-for-violation, non-vacuity re-confirmed); (2) VP-146 harnesses a1/a3/a4/a6 as
respecified in VP-146 v1.2 (new terminal-reconciliation input space of the shared pure core
`decide_terminal_record_reconciliation` — ONE function in `shard_manager.rs` used by both migrations
and both Kani suites — the COMMITTING→COMPLETED
reconciliation event, the reservation/quiescence/TTL/self-heal admission model, reconciliation
idempotence); (3) VP-143 facet (b) fault-injection + the D1/D2 black-box tests listed in the v1.18
test mandate. **Honest claim boundary:** Kani proves the pure decision cores and the abstract
gate/txn/reservation/lock model (safety: INV-GATE-TXN, quiescence-before-snapshot, no
admission while live, self-heal never fires under a live lock; bounded progress: from every
modeled crash state without a mismatch anomaly a bounded number of self-heal steps reaches
`gate=OPEN`). Kani does NOT prove, and VP-143/the black-box tests carry the empirical
obligation for: real `flock` semantics, wall-clock TTL behavior, the harness's PostToolUse
delivery guarantees, filesystem-visibility coherence underlying the Dekker argument, and — most
importantly for D1 — that the gate is actually wired into `main.rs`'s production dispatch
(a model cannot see a missing call site; only the real-binary black-box test can).

---

## Alternatives Considered

See §Rationale for the full head-to-head evaluation of Options A, B, and C.

- **Option A (Hook-driven explicitly-armed one-shot native action):** Viable; not chosen due to
  operational complexity for a one-time migration. Recommended for reconsideration if recurring
  migration use cases emerge.
- **Option B (One-time interactive Bash approval at F4) — CHOSEN:** Least privilege; one-time
  human gate; zero standing permission surface after migration completes; direct D-449(a)
  evidence capture; chosen per §Decision 1.
- **Option C (Hardened standing allowlist):** Fails the activation-authorization requirement
  (standing permission ≠ activation authorization per §Decision 4); post-migration cleanup
  burden; no governance benefit over Option B.

For the atomic-pointer model vs. incremental per-file rename: §Rationale
"Why v1.3 atomic-pointer model was adopted" explains the shared root cause across F1/F2/F5/F9
that made incremental amendment non-viable (diverging finding trajectory 7→8→11).

## Source / Origin

- BC-1.18.011 `S-25.02` cluster-5 specification (governs B2 body-split migration)
- BC-1.18.010 §Reader Integration (governs BC-INDEX reader protocol during migration window)
- S-25.06 Rule 7 (governed mechanism-A backfill-split activation; corrected by §Decision 9)
- In-house local adversary cascade — all passes recorded in decision log:
  - External pass-1 (`adv-cv-adr052-cluster5-F1-2026-09-12.md`, D-1214): 1st RATIFY-WITH-CHANGES
    (7 findings), producing v1.1 fix-burst
  - External pass-2 (`adv-cv-adr052-v11-closure-2026-09-12.md`, D-1216): 2nd closure review
    (8 findings), producing v1.2 redesign
  - External pass-3 (`adv-cv-adr052-v12-closure-2026-09-13.md`, D-1218): 3rd Codex cross-vendor
    (11 findings NOT RATIFIABLE), driving v1.3 full redesign
  - Local pass-1 (D-1221): 4th review (9 ADR-owned findings, 2C+5H+2M), producing v1.4 fix-burst
  - Local pass-2 (D-1222): 5th review (9 ADR-owned findings, 1C+4H+4M), producing v1.5 fix-burst
  - Local pass-3 (`adv-local-adr052-pass3.md`, D-1223): 6th review (12 findings NOT RATIFIABLE,
    1C+4H+5M+2L), producing the v1.6 fix-burst
  - Local pass-4 (D-1224): 7th review (2 HIGH + 6 MED observations), producing the v1.7 fix-burst
  - Local pass-5 (D-1225): 8th review (2 HIGH + 1 MED, RATIFY-WITH-CHANGES), producing the v1.8 fix-burst
  - Local pass-6 (D-1226): 9th review (1 HIGH + 1 MED + 2 LOW, RATIFY-WITH-CHANGES), producing the v1.9 fix-burst
  - Local pass-7 (D-1227): 10th review (1 HIGH + 2 MED + 2 LOW, RATIFY-WITH-CHANGES), producing the v1.10 fix-burst
  - Local pass-8 (D-1228): 11th review (1 HIGH + 2 MED + 1 LOW, NOT-RATIFIABLE), producing the v1.11 fix-burst
  - Local pass-9 (D-1229): 12th review (1 HIGH + 2 MED + 1 LOW, NOT-RATIFIABLE), producing the v1.12 fix-burst
  - Local pass-10 (D-1230): 13th review (1 HIGH + 3 MED + 2 LOW, accept-at-floor per D-386 Option C), producing the v1.13 fix-burst
  - Kani pass-1 (D-1231): pre-implementation Kani model-checking pass (1 HIGH — DEF-1,
    uncovered self-heal window in the `gate=OPEN` sub-state), run as human-directed spec/design
    convergence work on 2026-09-20, producing this v1.14 fix-burst
- `research-adr-052-v13-atomic-publication-2026-09-13.md` — research brief grounding
  the atomic-pointer architecture, advisory flock design, and F_FULLFSYNC / fexecve guidance
- CLAUDE.md TD-FACTORY-HOOK-BYPASS-001 P0 — governing rule this ADR excepts via §Decision 8
- ADR-051 §Decision 1 (WASM fuel-budget constraint), §Decision 7 (B2 end-state);
  canonical sub-shard naming `BC-INDEX-SS-NN.a.md` / `.b.md` per ADR-051 §Decision 7

---

## Downstream to Product-Owner

The architect specified the following amendments; the product-owner applied all BC body changes.
All BC and taxonomy changes in this section were applied in prior bursts (BC-1.18.011 v1.7,
BC-1.18.010 v1.8, error-taxonomy v1.25). Content is retained as provenance record.

### BC-1.18.011 amendments (v1.3–v1.7 bursts) — Applied: BC-1.18.011 v1.7

**Amendment 1 — Precondition 4 (scheduling coupling removal — applied BC-1.18.011 v1.7):**

Applied: Precondition 4 replaced with:

> "The migration is independently gated on the F4 activation boundary. It has NO timing or
> ordering dependency on BC-1.18.008's mechanism-A backfill-split; the two migrations activate
> independently (each via its own armed-activation manifest per ADR-052 §Decision 4) and may
> run in any order. They share an F4 activation window by operational convenience, not by
> specification."

**Amendment 2 — Postcondition 6 (A/B2 coupling removal — applied BC-1.18.011 v1.7):**

Applied: In Postcondition 6 final sentence, "at the SAME F4 activation moment mechanism A's
own backfill (BC-1.18.008) runs" was replaced with "at F4 activation, as part of the same
one-time B2 migration operation (independently of mechanism A's activation schedule)." The
requirement that SS-05/SS-06 sub-split occurs WITHIN the same B2 operation (not a separate
follow-on) was PRESERVED; only the A/B2 simultaneous-activation coupling was removed.

**Amendment 3 — Postcondition 7 scope clarification (applied BC-1.18.011 v1.7):**

Applied: Appended to end of Postcondition 7:

> "Note: this postcondition governs B2/Cohort-B independence only. A/B2 scheduling independence
> (that mechanism A and B2 activate independently at F4) is governed by Precondition 4 [as
> amended per ADR-052 §Decision 1]."

**Amendment 4 — Precondition 5: Phase markers (applied BC-1.18.011 v1.7):**

Applied: Amendment 4 text from v1.2 replaced with:

> "5. A durable transaction record at `.factory/migration-state/txn-<activation_uuid>.json`
>    and a framed checksummed intent log at
>    `.factory/migration-state/intent-<generation_uuid>.log` are maintained across the full
>    migration lifecycle:
>    - State STAGING: flock held; quiescence reached; staging generation built; intent log
>      written with per-target expected hashes + pre-states; all fsync barriers applied;
>      authorization gate check pending.
>    - State COMMITTING (the pivot): CURRENT.json pointer swap executed atomically; from this
>      point forward recovery is mandatory; authorization expiry does NOT abort.
>    - State COMPLETED: all canonical path moves complete and hash-verified; completed.json
>      written at stable path; PERMANENT.
>    EC-003 resume logic reads the intent log + txn record to determine which canonical path
>    moves succeeded (matching-destination-hash rule) and resumes from first uncompleted move."

**Amendment 5 — Precondition 6: Writer exclusion / maintenance boundary (applied BC-1.18.011 v1.7):**

Applied: Amendment 5 text from v1.2 replaced with:

> "6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution via two
>    independent mechanisms:
>    (a) Advisory flock on `.factory/migration-state/exclusive.lock` (pre-created, never unlinked):
>        the migration binary holds an exclusive flock for the full execution window; kernel
>        releases automatically on process death; stale-owner detection is automatic.
>    (b) Txn record at `.factory/migration-state/txn-<uuid>.json` with state STAGING or COMMITTING:
>        ALL mutation tool calls (Edit/Write/MultiEdit/Bash) targeting BC-INDEX paths are blocked
>        by the native admission gate in `executor.rs` (ADR-052 §Decision 5a) when a txn record
>        exists in STAGING or COMMITTING state — regardless of whether the flock is currently held.
>        This ensures ordinary writers remain blocked even during crash recovery when no process
>        holds the flock.
>    (c) OPEN/DRAINING gate with writer reservations spanning PreToolUse→tool-completion ensures
>        the migration coordinator waits for all in-flight admitted writers to complete before
>        snapshotting source files (ADR-052 §Decision 5a)."

**Amendment 6 — Postcondition 3: Dir-fsync mandate (applied BC-1.18.011 v1.7):**

Applied: Amendment 6 text from v1.2 (which incorrectly treated dir-fsync as mandatory on all
platforms) replaced with:

> "Each atomic file replacement (`rename(2)` call) MUST be followed by a platform-appropriate
> durability barrier before proceeding to the next replacement:
> - Linux (ext4/xfs): `fsync(file_fd)` + `fsync(parent_dir_fd)` — mandatory; ensures directory
>   entry survives a system crash per Pillai et al. OSDI'14.
> - macOS/APFS: `fcntl(file_fd, F_FULLFSYNC)` — mandatory for power-loss durability (Apple
>   `fsync(2)` does NOT flush the drive cache; `F_FULLFSYNC` is the documented durability lever);
>   `fsync(parent_dir_fd)` — best-effort only; Apple docs do not guarantee APFS directory-fsync
>   provides power-loss durability.
> This platform-branched durability guarantee is implemented in `sync_file_durable()` and
> `sync_dir_best_effort()` per ADR-052 §Decision 7d."

**Amendment 7 — Postcondition 3a: TOCTOU fingerprint recheck (applied BC-1.18.011 v1.7):**

Applied: Amendment 7 text from v1.2 replaced with:

> "3a. **Pre-commit source-fingerprint recheck (TOCTOU guard).** This check is performed EXACTLY
>     ONCE, immediately before the CURRENT.json pointer swap (step 6 in ADR-052 §Decision 7c)
>     — not before the first rename. The migration binary re-reads BC-INDEX.md's source content,
>     computes SHA-256, and compares against the `source_sha256` field recorded in the txn record
>     at quiescence. If they differ: ABORT. The txn record is set to state ABORTED. No canonical
>     paths have been changed at this point (the pointer swap has not occurred). The migration
>     requires re-activation. This single-check design eliminates the v1.1 contradiction where a
>     re-check after BC-INDEX.md's own rename would find a fingerprint mismatch and incorrectly
>     trigger abort."

**Amendment 8 — Invariant 3 commit-pointer (applied BC-1.18.011 v1.7):**

Applied: Amendment 8 text from v1.2 replaced with:

> "3. **The migration is never partially applied.** At every observable point in time — before
>    the migration runs, during staging, and after it completes — `BC-INDEX.md`'s body is either
>    the FULL original monolithic form or the FULL split end-state form; it is never observed in
>    a state where some subsystems are split and others are not. The all-or-nothing guarantee is
>    implemented via the CURRENT.json atomic pointer swap and the intent log: before the CURRENT.json
>    pointer swap (ADR-052 §Decision 7c step 6), the state machine is either STAGING (staging
>    generation in progress, intent log recording per-target expected hashes) or has no txn record;
>    after the CURRENT.json pointer swap transitions to COMMITTING, canonical readers see the
>    migration as in-progress and access new-generation content via the open-with-ENOENT-fallback
>    protocol (ADR-052 §Decision 7c step 2a): readers `open(gen-<uuid>/<file>)`; on ENOENT,
>    `open(canonical/<file>)` (the file was already renamed to canonical by step 7 progress).
>    This protocol is correct for BOTH net-new shard files AND in-place-overwrite targets such
>    as BC-INDEX.md. ENOENT-safe: the open-with-fallback protocol is race-free because files
>    move one-directionally gen→canonical via atomic rename(2); a required file is never absent
>    from BOTH gen-dir and canonical during the COMMITTING window. The CURRENT.json atomic pointer swap is the sole commit-point for the multi-file
>    atomic operation; composing N independent write_atomic calls without this commit-pointer does
>    not satisfy this invariant. completed.json (written at §Decision 7c step 8) is the permanent
>    terminal record; forward recovery uses the intent log + matching-destination-hash rule
>    (ADR-052 §Decision 7b) to resume from the first uncompleted canonical path move."

**Amendment 9 — Architecture Anchors (applied BC-1.18.011 v1.7):**

Applied: Amendment 9 text from v1.2 replaced with the following complete Architecture Anchors section:

> - `crates/factory-dispatcher/src/shard_manager.rs` — one-time B2 migration entry point reusing
>   BC-1.18.006's staging/atomic-replace primitives
> - `.factory/specs/behavioral-contracts/BC-INDEX.md` §Summary / `total_bcs` frontmatter field —
>   the independent count-oracle this BC's census check (Postcondition 2) cross-checks against
> - `.factory/specs/architecture/ARCH-INDEX.md` §Subsystem Registry — the BC-S Prefix→SS-NN
>   mapping this BC's per-subsystem partition boundaries follow
> - ADR-052 §Decision 4 — armed-activation manifest (two-phase validation: pre-lock and
>   under-exclusion)
> - ADR-052 §Decision 5a — native admission gate in executor.rs: OPEN/DRAINING gate with writer
>   reservations (PreToolUse-acquire/PostToolUse-release); txn record state check (STAGING/COMMITTING)
>   blocks ordinary writers regardless of PID liveness
> - ADR-052 §Decision 7a — advisory flock on stable pre-created never-unlinked inode
>   (`.factory/migration-state/exclusive.lock`); durable txn record separate from lock file with
>   `fencing_generation` (AUDIT-ONLY) for recovery-owner claim
> - ADR-052 §Decision 7b — framed checksummed intent log with per-target expected post-hash +
>   pre-state; WAL boundary after intent fsync; matching-destination-hash recovery decision table
> - ADR-052 §Decision 7c — single atomic CURRENT.json pointer swap (the commit point at step 6);
>   completed.json as permanent terminal record; generation-first/canonical-fallback reader protocol
> - ADR-052 §Decision 8 — POLICY 22 exception declaration with accurate skipped-control inventory
>   and enumerated allowed write targets

### BC-1.18.010 amendments (v1.3–v1.5 bursts) — Applied: BC-1.18.010 v1.9

**Invariant 2 amendment (v1.13 MED-2 — inlined; residual "unchanged from v1.2" annotation stripped):**

Applied to BC-1.18.010 Invariant 2:

> "Invariant 2: At activation time, a three-way ARCH-INDEX revision parity check MUST pass:
> `config.arch_index_sha` == `manifest.approved_arch_index_sha` == SHA-256 of the live
> `.factory/specs/architecture/ARCH-INDEX.md` file. All three must be equal; any pair
> divergence causes the activation to abort (`ARCH_INDEX_PARITY_ABORT`, exit 2). The
> ARCH-INDEX SHA is read from deserialized config only, never from ARCH-INDEX at runtime.
> Verified by the `arch_index_parity` CI parity test and stale-installed-config test
> (ADR-052 §Decision 10)."

**Reader integration amendment (applied BC-1.18.010 v1.9):**

Applied: §Reader Integration section replaced with (v1.5 M-4: stale v1.3 "use gen-uuid/" and stale v1.4 "canonical-first" text removed; v1.11 MED-1: open-with-ENOENT-fallback; v1.12 F3: existence-check "if absent" replaced with open-based form in this blockquote):

> "During the B2 migration window (after CURRENT.json pointer swap, before completed.json
> written), readers accessing BC-INDEX paths MUST use the following protocol:
> 1. Check `.factory/migration-state/completed.json` — if exists: canonical paths are current.
> 2. Check `.factory/migration-state/CURRENT.json` — if `status: committing`: for each
>    required file, `open(gen-<generation_id>/<file>)`; on ENOENT, `open(canonical/<file>)`.
>    (generation-first / open-with-ENOENT-fallback per ADR-052 §Decision 7c step 2a;
>    race-free: files move one-directionally gen→canonical via atomic rename(2))
> 3. If neither CURRENT.json nor completed.json exists: legacy BC-INDEX.md path is current.
> In steady state (completed.json present), canonical paths are always authoritative."

### error-taxonomy.md corrections (v1.3–v1.7 bursts) — Applied: error-taxonomy v1.25

Applied: v1.2 correction replaced with:

> "The maintenance lock (E-MAINTENANCE-001) is enforced by two independent mechanisms:
> (1) The native admission gate in `executor.rs` (ADR-052 §Decision 5a) blocks ALL mutation tool
>     calls (Edit, Write, MultiEdit, Bash) targeting BC-INDEX paths when a txn record at
>     `.factory/migration-state/txn-*.json` exists with state STAGING or COMMITTING — this check
>     is independent of PID liveness and fires before any registry plugin.
> (2) The OPEN/DRAINING gate state prevents new writer reservations when the maintenance
>     coordinator has declared maintenance intent (gate state = DRAINING or LOCKED).
> The `validate-factory-path-staging` guard (Bash path) is a secondary classifier layer that
> enforces the full-command classifier and four-branch guard logic (ADR-052 §Decision 5c)."

---

## BC Impact for Product-Owner Re-hardening

The v1.3 architectural changes required the following specific changes to BC-1.18.010,
BC-1.18.011, and error-taxonomy.md. All changes in this section were applied in prior bursts
(BC-1.18.011 v1.7, BC-1.18.010 v1.8, error-taxonomy v1.25). Content is retained as provenance
record. BC bodies were not edited directly — all changes were routed via product-owner.

### BC-1.18.011 — changes applied to match v1.3–v1.7

| Section | v1.2 text (superseded) | Applied (BC-1.18.011 v1.7) |
|---|---|---|
| Precondition 5 (§Amendment 4) | PREPARED/COMMITTED/CLEANED phase markers; `completed_renames` tracking; EC-003 reads `completed_renames` | Replace with: STAGING/COMMITTING/COMPLETED txn record; intent log for recovery; EC-003 reads intent log + txn record |
| Precondition 6 (§Amendment 5) | O_CREAT\|O_EXCL lock file with alive-PID check; Edit/Write gate via `validate-factory-path-staging` only | Replace with: advisory flock on stable inode; txn record state blocks writers regardless of PID liveness; OPEN/DRAINING gate with writer reservations |
| Postcondition 3 (§Amendment 6) | "dir-fsync is mandatory, not best-effort" (applies to all platforms) | Replace with: Linux mandatory dir-fsync; macOS F_FULLFSYNC on file mandatory; dir-fsync best-effort only on APFS |
| Postcondition 3a (§Amendment 7) | Single TOCTOU check before first rename; abort leaves BC-INDEX.md untouched | Update: fingerprint check still single; but now occurs before CURRENT.json pointer swap (**step 6** in §Decision 7c — NOT step 5; step 5 = fingerprint recheck, step 6 = CURRENT.json pointer swap; v1.3 text said "step 5" which was corrected to "step 6" in v1.4; see v1.4 additional changes table below and v1.7 F9 fix), not before first rename |
| Invariant 3 (§Amendment 8) | "COMMITTED marker is the sole commit-point"; references `completed_renames` | Replace with: CURRENT.json pointer swap (atomic rename) is the sole commit-point; forward recovery uses intent log + matching-hash rule |
| Architecture Anchors (§Amendment 9) | References §Decision 7a PID+activation_id lock file; §Decision 7 per-file rename sequence | Replace with: §Decision 7a advisory flock + txn record; §Decision 7b intent log; §Decision 7c CURRENT.json pointer swap + completed.json |

**v1.4 additional changes to BC-1.18.011 (Applied: BC-1.18.011 v1.7):**

| Section | v1.3 text (superseded) | Applied (BC-1.18.011 v1.7) |
|---|---|---|
| Invariant 3 | "CURRENT.json atomic pointer swap is the sole commit-point" — does not reflect C1 reader protocol change | Applied: during COMMITTING state, new-generation content is accessible via generation-first / open-with-ENOENT-fallback protocol: `open(gen-<uuid>/<file>)`; on ENOENT → `open(canonical/<file>)` (file already moved to canonical by step 7 progress). ENOENT-safe: files move one-directionally gen→canonical via atomic rename(2); required file never absent from both paths during COMMITTING window. (v1.5 C-1 corrected from canonical-first direction; v1.11 MED-1 corrected existence-check to open-based; v1.12 F3 corrected the inverted "canonical-first / generation-fallback; ENOENT is not possible" wording in this Applied cell) |
| Postcondition 1/2 (census gate cross-ref) | No explicit pre-pivot gate step referenced | Add cross-reference to ADR-052 §Decision 7c step 3b (pre-pivot content-preservation + census gate); state that PC1/PC2 are verified at step 3b against the staged generation BEFORE the pointer swap; failure at step 3b aborts cleanly per step 3c |
| Postcondition 3a (Amendment 7) / PC3a | "step 5 in ADR-052 §Decision 7c" cited as the pointer swap location | CORRECTION: the CURRENT.json pointer swap is at **step 6** (not step 5). Step 5 = fingerprint recheck; step 6 = CURRENT.json pointer swap (the commit point). Update all PC3a and related references from "step 5" to "step 6" |
| State names (any remaining PREPARED occurrences) | Any "PREPARED" in BC body or preconditions | Replace with STAGING. ADR-052 §7a enum authoritative: STAGING, COMMITTING, COMPLETED, ABORTED — no PREPARED state exists |
| Resume logic EC-003 | EC-003 resumes from first uncompleted intent log move | Add: EC-003 for resume-from-STAGING MUST RE-RUN the full census (step 3b) before proceeding to the pointer swap; resume is not permitted to skip the census gate |

**Note on M3 (stale total_bcs stdout re-grounding — BC-owned finding, not ADR-owned):**
BC-1.18.011 may have `total_bcs` stdout references that need updating to match current
BC-INDEX frontmatter values. This is a BC-owned finding for the product-owner to assess and
correct independently of the ADR changes.

### v1.5 additional BC changes (Applied: BC-1.18.011 v1.7, BC-1.18.010 v1.9, error-taxonomy v1.25)

**C-1 (generation-first reader protocol — applied; v1.11 MED-1 / v1.12 F3 open-with-ENOENT-fallback upgrade):**
- BC-1.18.010 §Reader Integration step 2: replaced with generation-first / open-with-ENOENT-fallback
  instruction per §Decision 7c step 2a: "for each required file, `open(gen-<generation_id>/<file>)`;
  on ENOENT, `open(canonical/<file>)`." Race-free: files move one-directionally gen→canonical via
  rename(2). Supersedes all prior versions of this instruction. (v1.5 C-1 established generation-first
  direction; v1.11 MED-1 + v1.12 F3 upgraded from existence-check "if absent" to open-based form)
- BC-1.18.011 Invariant 3: "canonical-first / generation-fallback" language (added in v1.4)
  corrected to "generation-first / open-with-ENOENT-fallback": `open(gen-<uuid>/<file>)`; on
  ENOENT → `open(canonical/<file>)`. (v1.5 C-1 corrected direction from canonical-first;
  v1.11 MED-1 + v1.12 F3 corrected existence-check to open-based form)

**H-4 (version-pin cleanup — applied):**
- BC-1.18.010 body: "ADR-052 v1.N §..." version-pinned references replaced with stable
  "ADR-052 §Decision N" form.
- BC-1.18.011 body: Same — all "ADR-052 v1.N §..." pins replaced with "ADR-052 §Decision N".
- error-taxonomy.md: "ADR-052 v1.3+" or "ADR-052 v1.4" version pins in E-MAINTENANCE-001
  catalog entry replaced with stable "ADR-052 §Decision 5a" or "ADR-052 §Decision 7c" anchors.

**M-1 (ADR-052 traceability row in BC-1.18.011 Architecture Anchors — applied):**
Applied: BC-1.18.011 Architecture Anchors section updated to add ADR-052 as an additional
anchor alongside ADR-051; ADR-052 §Decision 4–11 cited as authoritative source for one-time B2
migration mechanics, crash-atomicity guarantees, reader protocol, and executive write-exclusion
model.

**L-1 (error-taxonomy header — applied):**
Applied: error-taxonomy.md header updated to acknowledge the MIG and MAINTENANCE categories
added by v1.19–v1.21; header summary cross-checked against category list in the file body.

**L-5 (BC-1.18.011 EC-003 step citation — applied):**
Applied: BC-1.18.011 EC-003 citation updated from "§4e" to "ADR-052 §Decision 7c step 3b" as
the authority for the re-run-census requirement.

### v1.7 additional BC and taxonomy changes (Applied: BC-1.18.011 v1.7, error-taxonomy v1.25)

The following changes arose from v1.7 ADR-owned findings that had BC/taxonomy side-effects (F2),
plus BC-owned findings from pass-4 adversarial review routed to PO (F3, F6, F7). All applied.

**F2 (ADR side fixed; BC/taxonomy alignment — applied):**

Applied: error-taxonomy.md (v1.25) — `CENSUS_MISMATCH_ABORT` catalog row updated: "OR E-SHD-005
shard-boundary violation" removed from the trigger description; replaced with "OR shard-boundary
capacity violation (internal check within the migration binary — distinct from `E-SHD-005`, which
is a `HookResult` of BC-1.18.006/BC-1.18.010)."

Applied: error-taxonomy.md (v1.25) — `E-SHD-005` row description confirmed to scope E-SHD-005
ONLY to the steady-state hook context; language implying E-SHD-005 is emitted by the migration
binary or by `CENSUS_MISMATCH_ABORT` paths removed.

Applied: BC-1.18.011 (v1.7) — PC2 / PC4 / EC-001 / EC-004 / CTVs / VP-133: all references to
the census gate failure updated to use process exit code language (`CENSUS_MISMATCH_ABORT`,
"process exit code", "migration binary exits non-zero"); `HookResult` language ("E-SHD-005",
"HookResult::Error") removed from migration binary exit-code contexts.

**F3 (BC-owned — applied: `completed.json` casing sweep):**

Applied: BC-1.18.010 (v1.8) and BC-1.18.011 (v1.7): comprehensive casing sweep performed;
all occurrences of `COMPLETED.json` and `Completed.json` corrected to `completed.json` per
§Decision 7c step 8 canonical form; sweep covered preconditions, postconditions, invariants,
EC rows, reader protocol text, and Architecture Anchors sections.

**F6 (BC-owned — applied: Invariant 1 / Precondition 2 crash-atomicity machinery citation):**

Applied: BC-1.18.011 (v1.7) Invariant 1 and Precondition 2 updated to acknowledge that
crash-atomicity is provided by the ADR-052 §Decision 7 machinery (§Decision 7a: advisory flock +
durable txn record; §Decision 7b: framed intent log with WAL boundary; §Decision 7c: CURRENT.json
pointer swap as the single commit-point; completed.json as the permanent terminal record).
Language claiming no new crash-atomicity machinery exists was removed.

**F7 (BC-owned — applied: SDK Grounding: write_indeterminate_marker removed):**

Applied: BC-1.18.011 (v1.7) SDK Grounding Evidence section updated:
1. `write_indeterminate_marker` removed from SDK Grounding Evidence (not a shipped function).
2. Invariant 1 grounded against BC-1.18.006's ACTUAL shipped atomic-write primitive in
   `crates/factory-dispatcher/src/shard_manager.rs` (`write_atomic()` or equivalent
   copy-then-atomic-truncate-in-place mechanism per BC-1.18.006 §Postcondition 1 steps (a)-(b)).
3. BC-1.18.011 Architecture Anchors section verified to reference `shard_manager.rs` with the
   correct function name.

---

### v1.8 additional BC and taxonomy changes (applied in same v1.8 burst)

The following changes arose from the v1.8 F-2 sibling-sweep: the v1.7 F1 fix introduced
`source_body_row_sha256` and the structured per-BC-row PC1 model, but §BC Impact v1.7 did
NOT propagate the corresponding update directives to error-taxonomy.md, BC-1.18.011, or VP-132.
All three propagations were applied in the same v1.8 fix burst by PO (BC/taxonomy) and
formal-verifier (VP-132). This section records the completed propagations for provenance.

**F1 propagation (a) — error-taxonomy `CONTENT_PRESERVATION_ABORT` update (applied v1.8 burst):**

Applied in this v1.8 burst: error-taxonomy.md (v1.25) — `CONTENT_PRESERVATION_ABORT` catalog
row updated from the v1.4 / v1.6 "byte-for-byte reconstruct concat SHA-256 against
`source_sha256`" trigger to the v1.7 structured per-BC-row model:
> "PC1 structured-equivalence failure: SHA-256 of per-BC-row content extracted from all staged
> shard files in canonical BC-ID sort order does not match `source_body_row_sha256` from the
> txn record. (`source_sha256`, the whole-file hash of the original BC-INDEX.md body, is NOT
> used here — it is used ONLY by step 5 fingerprint recheck.) Also fires on staging-file
> integrity failure: sha256(staged_file) != intent-log expected_post_hash for any staged file."
`source_sha256` no longer appears in the PC1 context.

**F1 propagation (b) — BC-1.18.011 PC1 reconciliation to structured per-BC-row model (applied v1.8 burst):**

Applied in this v1.8 burst: BC-1.18.011 (v1.7) PC1 reconciled from the v1.6 whole-concat
SHA-256 model to the structured per-BC-row equivalence model:
> PC1 content-preservation is verified by: (1) extracting all BC-X.YY.NNN table rows from every
> staged shard file; (2) sorting extracted rows by canonical BC-ID; (3) computing SHA-256 of the
> sorted row bytes; (4) comparing against `source_body_row_sha256` (stored in the txn record at
> drain step 5, capturing the same extraction-sort-hash over the ORIGINAL BC-INDEX.md body).
> `source_sha256` (SHA-256 of the entire original BC-INDEX.md file) governs step 5 fingerprint
> recheck only and does NOT appear in PC1 predicate language.
All ACs, CTVs, and SDK Grounding Evidence previously citing `source_sha256` as the PC1 comparand
were updated to cite `source_body_row_sha256`. References to "concat" or "byte-for-byte
reconstruct" in the PC1 context were replaced with "structured per-BC-row extraction and sort."

**F1 propagation (c) — VP-132 reconciliation to structured per-BC-row model (applied v1.8 burst):**

Applied in this v1.8 burst: VP-132 / VP-132.md (v1.1) proof harness and invariant statement
reconciled from the v1.6 whole-concat PC1 model to the v1.7 structured per-BC-row model.
The proof target is equivalence of sorted-row-set SHA-256 hashes, not concatenation equality
against `source_sha256`. Feasibility assessment and proof strategy updated accordingly.

---

### BC-1.18.010 — changes applied to match v1.3–v1.5

| Section | v1.2 text (superseded) | Applied (BC-1.18.010 v1.9) |
|---|---|---|
| §Reader Integration | "COMMITTED absent → read legacy BC-INDEX.md"; COMMITTED archived at CLEANED | Applied: replaced entire section with v1.3 reader protocol (completed.json check first; CURRENT.json generation pinning second; legacy only if neither exists) |
| §Reader Integration steady state | "after CLEANED, shard paths are canonical; COMMITTED archived" | Applied: completed.json is permanent + authoritative; no archiving of any terminal record |
| Invariant 3 (if present) | References COMMITTED marker as commit-pointer | Applied: updated to CURRENT.json pointer swap as the commit-point; completed.json as the terminal record |

**v1.4 additional changes to BC-1.18.010 (Applied: BC-1.18.010 v1.9):**

| Section | v1.3 text (superseded) | Applied (BC-1.18.010 v1.9) |
|---|---|---|
| §Reader Integration step 2 | "if `status: committing`: use gen-uuid/ paths for reads" (v1.3 stale text); "try canonical path first; fall back to gen-uuid/" (v1.4 canonical-first — regression for BC-INDEX.md overwrite target) | Applied (v1.5 M-4 + v1.11 MED-1 + v1.12 F3): replaced BOTH stale instructions with generation-first / open-with-ENOENT-fallback per §Decision 7c step 2a: "`open(gen-<generation_id>/<file>)`; on ENOENT, `open(canonical/<file>)`." Correct for net-new shard files AND BC-INDEX.md (in-place overwrite target); race-free. |

### error-taxonomy.md — changes applied to match v1.3

| Location | v1.2 text (superseded) | Applied (error-taxonomy v1.25) |
|---|---|---|
| E-MAINTENANCE-001 definition (near original line 84) | "enforced by `validate-factory-path-staging` on Edit/Write"; "when exclusive.lock exists with alive PID" | Applied: replaced with dual-mechanism: (1) txn record state STAGING/COMMITTING blocks all mutation tools via native gate regardless of PID; (2) DRAINING gate prevents new reservations; guard is secondary classifier only |

**v1.4 additional changes to error-taxonomy.md (Applied: error-taxonomy v1.25):**

Applied: catalog rows added for all new error codes defined in §Error Code Semantics above.
Each row includes: code name, trigger description, severity, exit behavior. Codes added:
`E-BINARY-INTEGRITY-FAILURE`, `RECOVERY_REQUIRES_REAUTHORIZATION`, `EXPIRY_ABORT`,
`FINGERPRINT_MISMATCH_ABORT`, `DRAIN_TIMEOUT_ABORT`, `ARCH_INDEX_PARITY_ABORT`,
`COMPLETION_MANIFEST_REJECTION`, `CENSUS_MISMATCH_ABORT`, `CONTENT_PRESERVATION_ABORT`,
`ALREADY_MIGRATED` (exit-0 non-error sentinel), `E-MAINTENANCE-001` (existing entry updated
per v1.3 correction above). All rows present in error-taxonomy.md v1.25.
*(Historical record: this block records the v1.4 directive that was applied in the v1.4
fix-burst. All changes were completed; no pending actions remain here.)*

### v1.14 (DEF-1) — no BC or taxonomy changes required

The v1.14 fix reorders internal coordinator steps within §Decision 5a (moving the initial
txn-record write from before to after the DRAINING gate flip). It does not change any
BC-visible invariant, precondition, postcondition, error code, or trigger condition:
- `E-MAINTENANCE-001`'s trigger ("gate state is DRAINING or LOCKED, OR txn record exists with
  state STAGING or COMMITTING") is unchanged and remains accurate — it already covered the
  DEF-1 defect state correctly (the writer WAS blocked; the defect was the absence of a
  self-heal path, not an incorrect block).
- BC-1.18.011 Precondition 6 (writer-exclusion) and Invariant 3 (txn-record state as the
  re-run/recovery discriminator) are unaffected: the flock-gated exclusion mechanism
  (§Decision 5a step 3.5 sub-step a) that those clauses rely on is unchanged by this fix.
- No new error code, no new recovery-table row, and no change to any externally observable
  timing or ordering guarantee BC-1.18.011/BC-1.18.010 depend on.
**No product-owner action is required for this v1.14 burst.** The pre-existing PO handoff item
from v1.13 HIGH-1 (widen error-taxonomy.md's E-MAINTENANCE-001 "self-healing" description to
explicitly cover the null-generation STAGING sub-state) remains open and unrelated to DEF-1;
it is not expanded or narrowed by this fix.

### v1.18 (S-25.06 D1–D5) — BC and taxonomy changes required of product-owner

BC-1.18.013 v1.3 and error-taxonomy v1.32 (applied by product-owner ahead of this adjudication)
are RATIFIED with the amendments below; the exact required deltas are:

1. **BC-1.18.013 Precondition 6(b) — Tools bullet and EC-013/CTV rows:** the Rust admission gate
   covers `Edit`/`Write`/`MultiEdit` only; the `Bash` write-effect leg is [D-1232-OBL-4] (§5c
   classifier), exactly as the Precondition 6 preamble already says. Remove `Bash` from the
   Tools bullet's testable-in-S-25.06 set and from EC-013 / the D1 test-vector rows (keep a
   sentence: write-effect Bash is covered by OBL-4; until then POL-3 + the §7c step-5 fingerprint
   recheck are the backstops). The coordinator-exemption bullet is RATIFIED but re-scoped:
   implemented in the §5c classifier (OBL-4); S-25.06 owes the regression test that the Rust
   precheck leaves `Bash` unprocessed.
2. **BC-1.18.013 Precondition 6(b) "Where" bullet:** replace "added alongside them, never in
   place of either" with: the gate is a single shared admission core evaluated exactly once per
   PreToolUse event at the position `bc_index_migration_admission_precheck` already holds; both
   named precheck entry points delegate to it (§5a "Shared protected-path union").
3. **BC-1.18.013 Precondition 6(c):** replace "atomically under `LOCK_SH` on the gate-state file,
   creates …reservation" with the reserve-then-verify order (§5a step 0); add release-on-block;
   state the TTL by `created_at` (mtime fallback); keep 30 s / 3,600 s / 1,800 s floor / no PID
   liveness (ratified unchanged).
4. **BC-1.18.013 Postcondition 5a(c) / Invariant 6 / EC-009 / EC-010:** RATIFIED as written, plus:
   Postcondition 5a(b) and EC-010 must say the PreToolUse analogue of
   `COMPLETION_RECORD_MISMATCH_ABORT` is an E-MAINTENANCE-001 block with the reason logged;
   every `completed.json` / `CURRENT.json` reference in BC-1.18.013 becomes
   `completed-backfill-append-logs.json` / `CURRENT-backfill-append-logs.json` (§7e);
   `gate-state` ⇒ `gate-state.json`; the field is `canonical_paths_count` (already as written).
5. **BC-1.18.013 Precondition 5:** add `migration_id: "backfill-append-logs"` to the txn-record
   schema; add the cross-migration refusal sentence (§7e).
6. **BC-1.18.011 (B2) — amend to v1.11 (shared-core sibling sweep; ADR-052 is the spec, merged B2
   code deviates):** (a) Precondition 6(c): reserve-then-verify admission order and
   release-on-block; (b) Precondition 6(c)/Invariant: `MAX_RESERVATION_TTL` default 3,600 s,
   floor 1,800 s (merged code: 120 s); (c) NEW: the PreToolUse stale-gate reconciliation
   (§5a step 3.5 Branches A/B/C) is part of the contract and MUST be wired on the production
   admission path (merged code: `reconcile_stale_admission_gate` is never called); (d)
   Postcondition/EC for the verified `completed.json`+COMMITTING finalize (replaces the merged
   unverified short-circuit) and `COMPLETION_RECORD_MISMATCH_ABORT` for `migrate-bc-index`;
   (e) txn-record `migration_id` field (absent ⇒ `migrate-bc-index`) and cross-migration
   refusal; (f) a sentence that `completed.json`/`CURRENT.json` are B2's own and are never
   written by `backfill-append-logs`.
7. **error-taxonomy.md:** `COMPLETION_RECORD_MISMATCH_ABORT` Trigger and `ALREADY_MIGRATED`
   Trigger name the per-migration terminal-record path; `E-MAINTENANCE-001` text unchanged.

**v1.18 follow-up deltas (architect review of the PO-applied BC-1.18.011 v1.11 / BC-1.18.013 v1.4 /
error-taxonomy v1.33; all are exact-text corrections, still ADR v1.18):**

8. **error-taxonomy.md `COMPLETION_RECORD_MISMATCH_ABORT` Trigger:** DELETE clause (e) ("the live
   txn's `migration_id` is the OTHER migration's …"). A foreign live txn is the §7e cross-migration
   refusal (`LockContention`-class exit 2; PreToolUse: ordinary `E-MAINTENANCE-001`), not a
   mismatch abort. Clause (c) wording is correct; append "— STAGING + terminal record is ALWAYS
   fail-closed, no verification is attempted". Update the v1.34 changelog row accordingly.
9. **BC-1.18.011 EC-012 and its canonical test-vector row (D4 row):** remove "or the live txn is
   `backfill-append-logs`'s" from EC-012's input column and the "(also …)" fixtures; the foreign-txn
   case is covered by EC-013 only. Postcondition 9(a): replace "(cross-migration refusal,
   Precondition 6(e))" with "(cross-migration refusal, Precondition 6(e): binary exit 2
   `LockContention`-class, NOT `COMPLETION_RECORD_MISMATCH_ABORT`; PreToolUse: plain
   `E-MAINTENANCE-001`, no mismatch reason)". The vector row "`completed.json` valid, live txn
   `backfill-append-logs` COMMITTING" expected column becomes "refusal exit 2 (LockContention-class) /
   plain `E-MAINTENANCE-001` block; txn NOT finalized, gate NOT opened (EC-013)" — drop "EC-012" from
   its anchors.
10. **BC-1.18.011 Precondition 6(d) Branch C and Postcondition 9 lead-in:** "this migration's terminal
    record (`completed.json`)" → "the live txn's own migration's terminal record, selected by
    `migration_id` (`completed.json` for `migrate-bc-index`/absent field;
    `completed-backfill-append-logs.json` for `backfill-append-logs`, BC-1.18.013 Postcondition 5a)
    — the shared core dispatches on `migration_id` (ADR-052 §7e)". The B2-specific clauses
    (schema, N, verification) of Postcondition 9 stay as written for `migrate-bc-index` txns.
11. **STAGING wording (BC-1.18.011 PC 6(d)/9(d), BC-1.18.013 PC 5a(b)/EC-010):** add "(STAGING +
    terminal record is always fail-closed; no verification is attempted)" so the text matches ADR
    §4e/Branch C exactly. No behavior change (PO already implements fail-closed).
12. **BC-1.18.013 Precondition 6(b) "Decision" bullet:** "The stale-state self-heal … runs first and
    may flip the gate to OPEN" → "The step-0 reservation is created first; the §5a step-3.5
    reconciliation then runs and may flip the gate to OPEN, after which the decision is re-evaluated
    (reserve-then-verify, 6(c))". Ordering nit only.
13. **BC-1.18.011 §Verification Properties table + "VP Anchors" paragraph — the rows needed so
    VP-133 v1.1 and the new VP-147 are anchored; apply in the same pass as 8-12:**
    - **VP-133** (integration) — add rows: (i) *Production-path reconciliation wiring* — real
      dispatcher PreToolUse with gate LOCKED/DRAINING reconciles Branch A/B and admits; live
      coordinator ⇒ no action + `E-MAINTENANCE-001` (Precondition 6(d), EC-010; defect B2-1);
      (ii) *Reserve-then-verify / release-on-block / TTL* — never both-miss at every W1/W2/C1/C2
      ordering; no reservation survives a blocked event; production TTL 3,600 s / floor 1,800 s by
      `created_at` (Precondition 6(c), EC-007/EC-008/EC-009; B2-3, B2-4); (iii) *Verified finalize,
      mismatch fail-closed, migration discriminator, per-migration namespace* (Postcondition 9,
      Precondition 6(e), EC-011..EC-014; B2-2). Also amend the existing VP-133 "Idempotency" row:
      "`completed.json` beside a live txn is the verified own-migration finalize, not an
      unverified no-op".
    - **VP-147** (NEW, kani-proof) — add one row: *B2 crash-recovery / admission-gate decision core
      (Kani h1..h6, the `obl1_kani_proofs` suite): recovery totality, recovery safety (old-or-new,
      never torn), txn state-machine inductive invariant, INV-GATE-TXN admission quiescence,
      pointer-swap crash atomicity, recovery idempotence* | kani-proof (CI job `kani`,
      `--harness proof_obl1`, EXPECTED_PROOFS=7; 7/7 PROVED pre-v1.18; v1.18 extension —
      reserve-then-verify, TTL, Branches A/B/C, verified finalize, `migration_id` — owed). Anchors:
      Precondition 5, 6(b), 6(c), 6(d), 6(e); Postconditions 3, 4, 5, 9; Invariant 3; EC-002,
      EC-003, EC-006, EC-007..EC-014. Update the "VP Anchors" paragraph to name VP-147 as allocated
      by the architect under ADR-052 v1.18 (VP-INDEX v3.26).
    - BC-1.18.011 changelog row (v1.12) lists `VP citations changed in: VP-133 (facets/anchors),
      VP-147 (new)`; the story-writer propagates the VP citations to S-25.02/S-25.06.

**Complete product-owner delta list (one pass): items 1-7 (earlier v1.18 deltas, already applied
as BC-1.18.013 v1.4 / BC-1.18.011 v1.11 / error-taxonomy v1.33) PLUS items 8-13 above.**
Items 8-9 need error-taxonomy v1.34; items 9-11 and 13 need BC-1.18.011 v1.12; items 11-12 also
BC-1.18.013 v1.5.

### v1.20 (S-25.08 local adversary pass 1: F-001/F-002/F-003/F-004/F-006/F-008/F-009) — BC, taxonomy, VP and story deltas

Target versions: BC-1.18.013 v1.7 → v1.8; BC-1.18.011 v1.15 → v1.16; error-taxonomy v1.35 → v1.36;
VP-133, VP-143, VP-147 amended by the architect in the same burst (below; VP catalog unchanged ⇒
VP-INDEX / verification-architecture / verification-coverage-matrix counts untouched, POLICY 9).
Apply to BOTH BCs (shared-core sibling sweep) unless a delta names one.

14. **Precondition 6(b) "Where" bullet (both BCs):** replace "at the position
    `bc_index_migration_admission_precheck` already holds on the production PreToolUse dispatch
    path — structurally AHEAD of `shard_cap_precheck` … and before any registry plugin" with the
    §5a "Evaluation position (v1.20)" obligations O1–O4: exactly once; BEFORE `Registry::load` /
    `resolve_registry_path()` / the Tier-1 degraded branch (so it runs with `CLAUDE_PLUGIN_ROOT`
    unset, a missing/unparseable/schema-mismatched registry, and an empty matched-plugin set);
    before `shard_cap_precheck` and every plugin tier; the release leg is registry-independent too.
    Add the reason (the registry not-found/parse/regex arms and the Tier-1/`resolve_registry_path`
    paths are fail-open; the interlock must not be; schema/async-block/duplicate arms exit 2 today
    — see ADR §5a "Evaluation position" for the per-error table).
15. **Precondition 6(b) "Which events / tools" bullet (both BCs):** hook events become
    `PreToolUse` (admission + reservation creation) and `PostToolUse` **and `PostToolUseFailure`**
    (reservation release; Precondition 6(c)). Tools unchanged (`Edit`/`Write`/`MultiEdit`; `Bash`
    stays OBL-4). Replace "every target path outside the protected union (e.g. `.factory/STATE.md`,
    `.factory/stories/`) is NOT affected" with the anchoring + resolution rules: NEW sub-bullets
    (i) **Anchor** — the gate guards the session's own `factory_root` = resolved
    `<CLAUDE_PROJECT_DIR>/.factory` (must exist as a directory; the gate never creates `.factory`);
    migration-state is `<factory_root>/migration-state`; (ii) **Classification** — component-wise
    descendant-or-equal of `<factory_root>/specs/behavioral-contracts` (`BC-INDEX` scope) or
    `<factory_root>/cycles` (`.factory/cycles/` scope), never a substring; (iii) **Out-of-root
    `.factory` paths** (other project, nested project, scratch tree, look-alike `x.factory/…`) are
    out of scope: admitted, no reservation, no directory creation, no state read; state the
    accepted residual + §7c step-5 backstop; (iv) **Target path resolution** — lexical
    normalisation of `.`/`..`/`//`, POSIX-correct symlink resolution of the deepest existing
    ancestor with the nonexistent tail appended lexically, in-scope iff EITHER the resolved OR the
    lexical form matches, unresolvable ⇒ lexical (fail-closed), relative `file_path` joined to the
    payload `cwd`, `\` a separator only on Windows, ALWAYS case-insensitive component compare (no
    filesystem probe); (v) residuals (hard links, bind mounts, TOCTOU).
16. **Precondition 6(c) (both BCs):** (a) the release is "on `PostToolUse` or `PostToolUseFailure`
    for the same `tool_use_id`" (replace "(success or failure of the tool)", which named an event the
    harness never sends for failure); keyed only on `tool_use_id`, no `tool_name` filter; neither
    event arriving ⇒ TTL backstop; (b) NEW **timestamp rules** paragraph (ADR §5a v1.20 rules 1-5:
    RFC 3339 parse, pre-epoch/out-of-range ⇒ unparseable ⇒ mtime, `created_at > now + 300 s` ⇒
    untrusted ⇒ mtime, in-tolerance future ⇒ age 0, both unusable ⇒ NOT stale + warn, no clamping);
    extend EC-019 (011: the equivalent EC) with these vectors; (c) NEW **`tool_use_id` presence and
    validity** paragraph (absent/`null` ⇒ check-only; present-but-invalid ⇒ fail-closed
    `E-MAINTENANCE-002` `invalid_tool_use_id`; grammar `[A-Za-z0-9_.-]{1,128}`, no leading `.`);
    (d) "Failure semantics" sentence: the "writer-admission-check error result" is now named
    `E-MAINTENANCE-002` (message `E-MAINTENANCE-002: writer-admission check failed (<cause>)`).
17. **Precondition 6(d) decision table (BC-1.18.011) / "Terminal-record reconciliation decision
    cell" (BC-1.18.013 Precondition 6(d)) and Precondition 5 / Postcondition 5a / Postcondition 9
    wording:** replace "live txn is NOT the evaluating migration's own (`migration_id` differs)"
    (row 2) with "live txn's `migration_id ∉ K`, K = {`migrate-bc-index`, `backfill-append-logs`}
    (absent field ⇒ `migrate-bc-index`) ⇒ `RefuseForeignMigration`"; rename the core input
    `txn_is_own_migration` → `txn_migration_known`; add: in the shared (dispatcher/§5c) core a live
    txn of EITHER known migration is decided against ITS OWN migration's terminal record selected by
    `migration_id` (the other migration's record is never consulted), so "foreign" never applies
    between the two known ids on the dispatcher path; cross-migration refusal (binary exit 2
    `LockContention`-class) is unchanged and binary-only. A non-string `migration_id` is a malformed
    record ⇒ `E-MAINTENANCE-002` `state_integrity`.
18. **Edge cases + test vectors (both BCs; IDs continue each BC's sequence):** NEW EC rows (a)
    `PostToolUseFailure` releases the reservation (incl. `is_interrupt`, absent `tool_name`);
    (b) broken/missing/schema-mismatched registry, `CLAUDE_PLUGIN_ROOT` unset ⇒ gate still
    enforces and releases; (c) protected-looking path outside the session `factory_root` ⇒ admitted,
    nothing created; `.factory` symlink ⇒ real root; no-`.factory` project ⇒ no directory created;
    (d) path aliasing — `.`/`..`/`//`, symlink alias, `link/..` ordering, nonexistent tail,
    mixed-case family names, relative `file_path`; (e) `created_at` future (+299 s / +301 s),
    pre-epoch, non-RFC3339, year-9999, mtime future/unavailable; (f) invalid `tool_use_id`
    (number / empty / `../x` / 129 chars) vs absent/`null`; (g) a live txn of the OTHER known
    migration with no own record (plain block, not finalized) and with its own verifying record
    (Branch C finalize); unknown `migration_id` (plain block, never aborted). BC-1.18.013 EC-016 and
    BC-1.18.011 EC-013 keep their binary-refusal meaning; add "(binary recovery path only)".
19. **error-taxonomy.md v1.36:** two new catalog rows — `E-MAINTENANCE-002` (category Migration
    window guards; severity broken; `HookResult::Error`, exit 2 at the PreToolUse hook surface;
    message `E-MAINTENANCE-002: writer-admission check failed (<cause>)`,
    `<cause>` ∈ {`invalid_tool_use_id`, `io`, `state_integrity`}; "no reservation left behind";
    ADR-052 §5a/§Error Code Semantics v1.20) and `RESERVATION_TTL_BELOW_FLOOR` (category Migration
    binary exit codes; blocked; exit 2; production entry point configured below the 1,800 s floor;
    nothing mutated); update the MAINTENANCE category prose ("`E-MAINTENANCE-NNN` … exit 2 blocks")
    to mention the `-002` HookResult::Error member and that `E-MAINTENANCE-001`'s `PostToolUse`
    wording becomes "PostToolUse / PostToolUseFailure"; `E-MAINTENANCE-001` text/format UNCHANGED.
21. **Lexical root spellings (both BCs; ADR §5a "Lexical root spellings"):** BC-1.18.013 v1.8 →
    v1.9 and BC-1.18.011 v1.16 → v1.17. Precondition 6(b) anchor/classification sub-bullets (i)/(ii)
    and (iv): replace "the lexical comparison (`T_lex` vs the lexical `factory_root`)" with "`T_lex`
    vs the lexical normalization of EITHER spelling of the session's own factory root: (a) the
    canonical `factory_root`, (b) `<CLAUDE_PROJECT_DIR as given>/.factory` (raw env value, absolute
    only, lexical normalization only); no other spelling (never from the target, payload `cwd` or an
    ancestor)"; add "migration-state is anchored on the real root regardless of spelling". EC-023: add
    vector (f) — project directory reached via a symlink spelling (macOS `/var` vs `/private/var`;
    symlinked checkout): `file_path` spelled through the NON-canonical `CLAUDE_PROJECT_DIR` is in
    scope (blocked under a live txn / reserved when OPEN) even when `T_real` is unavailable
    (unresolvable ancestor seam), and the reservation lands in the canonical
    `<factory_root_real>/migration-state/reservations/`. EC-024: add vector — the same protected target
    spelled via the canonical form and via the as-given form are classified identically; and a
    look-alike/other-project path that merely shares a symlink-ancestor spelling with neither
    alias is out of scope (no over-match). Test vectors: one row per new vector.
22. **`executor::resolve_shard_gate_precedence` REMOVED (S-25.08 implementation ruling).** With
    admission evaluated before `Registry::load` and returning directly on a Block/Error verdict
    (§5a "Evaluation position"), the "never invoke `shard_cap_precheck` when the migration gate
    fires" guarantee of BC-1.18.011 Architect Ruling 1 is STRUCTURAL (control flow of `main.rs::run`:
    an early return precedes the only `shard_cap_precheck` call) and is exercised by the real-binary
    black-box test (O3). The helper has no remaining production role, and a `pub` function used only
    by its own unit tests is dead production surface and a false-assurance seam (the tests prove the
    helper, not `main`). Ruling: delete `resolve_shard_gate_precedence` and its four unit tests
    (`test_BC_1_18_011_PC6_RULING1_resolve_shard_gate_precedence_*` in
    `bc_1_18_011_b2_migration_test.rs`) and the import; replace their coverage with ONE real-binary
    test asserting that with a live txn a protected `Write` to a path that `shard_cap_precheck` would
    roll (an over-cap `[[shard]]` canonical under `.factory/cycles/`/BC-INDEX) exits 2
    `E-MAINTENANCE-001` AND leaves the canonical byte-identical (no seal/truncate) — the behaviour the
    helper's tests existed to protect. No BC text names the helper (grep clean); BC-1.18.011's
    Architect Ruling 1 prose stays valid, re-anchored to the structural early return — product-owner
    to word one sentence in BC-1.18.011 v1.17 Precondition 6(b) ("structurally skipped: admission
    returns before `shard_cap_precheck` is reachable").
20. **Story deltas (story-writer; no BC authoring):** S-25.08 gains ACs/tasks for each of F-001
    (release on `PostToolUseFailure` + fixture), F-004 (evaluation before `Registry::load`; O1–O3
    black-box under broken/absent registry; release-on-block under a schema-mismatch/async-block/duplicate registry, exit 2), F-002 (anchor +
    out-of-root no-op), F-003 (`resolve_target_path` shared fn + alias/case vectors), F-006
    (`txn_migration_known` rename + dispatcher-path decision vectors), F-008 (`reservation_is_stale`
    signature `(Option<u64>, Option<u64>, u64, u64)` + skew constant 300 + pre-epoch-mtime change),
    F-009 (`ReservationTtlBelowFloor`, `InvalidToolUseId`, `E-MAINTENANCE-002` mapping); file list
    gains `crates/factory-dispatcher/src/invoke.rs` (`is_tool_completion_event`) and
    `shard_manager/admission.rs`; cite BC-1.18.011 v1.16 / BC-1.18.013 v1.8 and the VP amendments.
    S-25.06 (mechanism-A) inherits the "foreign" wording and the rename only (no new ACs). STORY-INDEX
    row/version bump per the story-writer's normal propagation (POLICY 8 bcs-array atomicity: only if
    the `bcs:` frontmatter set changes — it does not).

### v1.21 (S-25.08 local adversary pass 2: D-1/D-2/F-006/F-012/F-013/F-014) — BC, taxonomy, VP and story deltas

Target versions: BC-1.18.013 v1.9 → v1.10; BC-1.18.011 v1.17 → v1.18; BC-3.08.001 v1.34 → v1.35;
error-taxonomy v1.38 → v1.39. VP-143 v1.4→v1.5 and VP-146 v1.3→v1.4 were amended by the architect in
the same burst (VP-INDEX v3.30; catalog counts unchanged ⇒ `verification-architecture.md` /
`verification-coverage-matrix.md` untouched, POLICY 9). Apply to BOTH BCs (shared-core sibling sweep)
unless a delta names one. Numbering continues the v1.20 list.

**product-owner**

23. **D-2 anchoring (both BCs):** Precondition 6(b)(i) Anchor: replace "the same anchor the
    coordinator binary uses" (BC-1.18.013 Precondition 6(b)(i); the equivalent sentence in
    BC-1.18.011 Precondition 6(b)) with the §5a "Single anchoring rule (v1.21)" rule: project root =
    `resolve_session_project_root` (non-empty `CLAUDE_PROJECT_DIR`, canonicalized with as-given
    fallback; else the process cwd; no ancestor walk); factory root = `resolve_factory_root(project
    root)`; `migration-state/` = `FactoryRoot::migration_state_dir()`; admission, the release leg AND
    both coordinator binaries use this one function. NEW coordinator Preconditions (BC-1.18.011 new
    Precondition 7; BC-1.18.013 new Precondition 7): the coordinator resolves its factory root by the
    same rule, derives EVERY `.factory/…` path from it, never creates `.factory`, and on a missing
    `.factory` exits 2 `FACTORY_ROOT_NOT_FOUND` with nothing created. NEW EC rows (next free in each BC):
    process cwd ≠ `CLAUDE_PROJECT_DIR` ⇒ coordinator state/drain under `<CLAUDE_PROJECT_DIR>/.factory`,
    visible to admission's reservations; unset/empty ⇒ cwd; no `.factory` ⇒ exit 2
    `FACTORY_ROOT_NOT_FOUND`; `.factory` symlink ⇒ one real namespace for both. One test-vector row each.
24. **D-1 diagnostics (both BCs):** replace every `tracing::warn!` / `tracing::error!`
    diagnostic mandate in Precondition 6(b)/(c)/(d) and Failure semantics (BC-1.18.011: 8
    `tracing::` mentions; BC-1.18.013: 11) with the corresponding `migration.admission_blocked` /
    `migration.admission_failed` / `migration.admission_advisory` InternalLog event (§5a "Admission
    diagnostics channel (v1.21)"): E-MAINTENANCE-001 verdict ⇒ `_blocked` (fields scope, family,
    branch, gate_state, migration_id, txn_id, check, reconciliation); E-MAINTENANCE-002 ⇒ `_failed`
    (cause, kind, detail); release failure and Branch A/B/C repair events ⇒ `_advisory` (reason);
    the AC-016 timestamp reasons are coordinator STDERR tokens, not events (see item 33). Add a Postcondition (BC-1.18.013 Postcondition 10, BC-1.18.011
    Postcondition 10): "every admission verdict or anomaly produces exactly one such event in
    `dispatcher-internal-YYYY-MM-DD.jsonl`; no field carries a raw `tool_use_id` or record content";
    add one EC + test vector per event type. The coordinators' diagnostics are stderr lines (§5a
    "Admission diagnostics channel" (5)).
25. **BC-3.08.001 v1.35:** catalog Events 11–13 — `migration.admission_blocked`,
    `migration.admission_failed`, `migration.admission_advisory` — in the existing Event-N format:
    trigger = the `main.rs` shell writing the `AdmissionDiagnostic` data returned by the shared core;
    fields as in §5a; emission path = `InternalLog::write` directly (NOT `HostContext::emit_internal` —
    no HostContext exists at the registry-independent admission position; recorded deviation);
    durable sink = `dispatcher-internal-{date}.jsonl`; no `plugin_name`.
26. **F-006 (both BCs):** EC-031 (BC-1.18.013) / EC-009(d) (BC-1.18.011): replace "the PRODUCTION drain
    entry point is configured with" with: "the production entry delegates to the crate-private
    `run_bc_index_migration_with_ttl(project_root, max_reservation_ttl)` passing
    `DEFAULT_MAX_RESERVATION_TTL`; that function's FIRST statement is
    `validate_production_reservation_ttl`; below-floor input ⇒ `ReservationTtlBelowFloor` before any
    gate/drain action". Keep the vectors (120 s, 1,799 s ⇒ error + byte-identical `migration-state/`
    snapshot, no `exclusive.lock`; 1,800 s, 3,600 s accepted; `drain_bc_index_writers` unbound). ADD
    vector: "default constant ≥ floor is enforced at compile time (`const _: () = assert!(…)`)". The
    test seam is crate-private — NOT public, NOT env/argv-injectable.
27. **F-012:** BC-1.18.013 EC-032 / Precondition 6(c) "`<cause>` classification" and BC-1.18.011
    Precondition 6(d) / EC-026: name the carrier — `state_integrity` is
    `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }`, kind ∈ {`gate_record_malformed`,
    `txn_record_malformed`, `txn_migration_id_not_string`, `multiple_live_txns`,
    `reservation_serialization`}; it is NOT `BinaryIntegrityFailure`; add "the variant's Display must not
    contain `BINARY_INTEGRITY_FAILURE`" as a test vector; add "`admission_failure_cause` is an exhaustive
    match (no wildcard)" to Invariants.
28. **F-013 (both BCs):** Precondition 6(b) "Decision" bullet: add "an admission verdict is reported to
    the operator as `blocking_plugins=migration-admission` (`block_reason` = the unchanged
    E-MAINTENANCE-001/002 text, exit 2); a shard-cap-gate verdict keeps `shard-cap-gate`". EC + vector.
29. **F-014 (both BCs):** BC-1.18.013 Precondition 6(b) "Where" (the sentence naming
    `bc_index_migration_admission_precheck` and `append_log_backfill_admission_precheck`) and
    BC-1.18.011 Precondition 6: name the single shared entry points `migration_writer_admission` /
    `migration_writer_admission_precheck` / `migration_reservation_release`; delete the "or its
    successor" hedge; no per-migration delegates.
30. **error-taxonomy v1.39:** (a) `E-MAINTENANCE-002` row: `state_integrity` names the variant and the
    five kinds (replaces "any other integrity failure"); replace "goes to `tracing::warn!`" (2
    `tracing::` mentions in the file) with "goes to the `migration.admission_failed` InternalLog
    event"; (b) NEW MIG rows `MIGRATION_STATE_INTEGRITY_FAILURE` (exit 2, blocked) and
    `FACTORY_ROOT_NOT_FOUND` (exit 2, blocked) per §Error Code Semantics "v1.21 additions"; (c)
    `RESERVATION_TTL_BELOW_FLOOR` row: "raised by `validate_production_reservation_ttl`, the first
    statement of `run_bc_index_migration_with_ttl`"; (d) paragraph near the MAINTENANCE category prose
    (line mentioning `RESERVATION_TTL_BELOW_FLOOR` is a process exit code): add the two new rows.
    `E-BINARY-INTEGRITY-FAILURE` unchanged.

33. **Product-owner follow-up rulings (v1.21 completion; apply with items 23-30):**
    (a) *Branch C failure = ONE event.* Every E-MAINTENANCE-001 verdict writes exactly one
    `migration.admission_blocked`; a Branch C verification failure is `branch=completion_record_mismatch`
    with `check` = the failing check and NO `_advisory` (ADR §5a Branch C prose corrected). Ratifies the
    PO's `branch` usage.
    (b) *`reconciliation` domain (CORRECTS the PO's decision-table tokens).* BC-3.08.001 Event 11
    wire-format line and Field semantics, and every BC mention: replace "`NoOp` | `RefuseForeignMigration` |
    `FinalizeThenOpenGate` | `FailClosedMismatch` or `none`" with the closed six-token domain of the
    effectful outcome: `live_coordinator` | `nothing_to_reconcile` | `gate_reopened` |
    `null_generation_txn_aborted` | `foreign_migration_refused` | `completion_record_mismatch` (no `none` —
    reconciliation always runs before a block); add the `branch` derivation table (`live_coordinator` ⇔
    `live_coordinator`; `foreign_migration` ⇔ `foreign_migration_refused`; `completion_record_mismatch` ⇔
    `completion_record_mismatch`; else `gate_only` if no live txn remains, else `live_txn`) and a test
    vector per reconciliation token.
    (c) *`_advisory` reason domain.* Event 13 `reason` ∈ {`reservation_release_failed`,
    `branch_a_gate_reopened`, `branch_b_txn_aborted`, `branch_c_finalize_unwired`, `branch_c_finalized`}.
    REMOVE the five timestamp tokens (`created_at_unparseable`, `created_at_pre_epoch`, `created_at_future`,
    `mtime_future`, `age_unknown`) from Event 13, BC-1.18.013 Postcondition 10(c) and BC-1.18.011
    Postcondition 10: they are coordinator-drain STDERR tokens (only the drain GC calls
    `reservation_is_stale`; admission never reads reservation files). `branch_c_finalize_unwired` fires
    when `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` and the finalize effect is
    undelivered (S-25.08 seam) — written IN ADDITION to the one `_blocked`; retired when S-25.06
    delivers the finalize. `branch_c_finalized` is written by S-25.06's successful verify-then-finalize
    (admitted ⇒ no `_blocked`); neither S-25.08 nor S-25.09 emits it (v1.22).
    (d) *Optional `_advisory` context fields:* closed set `migration_id`, `txn_id`, `check`, `detail`,
    `tool_use_id_len`; any other field forbidden (replace "e.g." wording in Event 13).
    (e) *`FACTORY_ROOT_NOT_FOUND` text:* normative single stderr line `<subcommand>:
    FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <project_root> (resolved from
    <source>)`, `<source>` ∈ {`CLAUDE_PROJECT_DIR`, `process cwd`}; variant `FactoryRootNotFound {
    project_root, source }`. State it in BC-1.18.011/013 Precondition 7 and the error-taxonomy row.
    (f) *BC-3.08.001 §VP Anchors:* VP-079 is extended — v1.24 (Events 11–13 wire-format rows,
    `dispatcher-internal-*.jsonl` only; Property 6 SITES not extended); VP-028 v1.1 gains a scope note
    (Events 11–13 are not SinkEvents; out of fan-out scope); emission semantics are VP-133 v1.6 facet 7(e)
    / VP-143 v1.5. Clear the "VP-079 staleness OPEN" flag and cite those versions. No new VP; counts
    unchanged.

    (g) *Undelivered-finalize token (S-25.08 seam).* When `decide_terminal_record_reconciliation`
    returns `FinalizeThenOpenGate` and the finalize effect is undelivered, the one `_blocked` carries
    `reconciliation=completion_record_mismatch`, `branch=completion_record_mismatch`,
    `check=finalize_unwired` (the closed six-token `reconciliation` domain is unchanged; no seventh token —
    the code already maps this case to `StaleGateReconciliation::CompletionRecordMismatch` and the
    verdict has the mismatch suffix), plus the `_advisory reason=branch_c_finalize_unwired`. Once S-25.06
    delivers finalize, the same input (COMMITTING + verifying record) finalizes (txn→COMPLETED, gate→OPEN),
    the write is admitted, NO `_blocked` is written, and `_advisory reason=branch_c_finalized` is
    written; `branch_c_finalize_unwired` and `check=finalize_unwired` are then retired. BC delta
    (BC-3.08.001 Event 11 Field semantics + BC-1.18.011/013 Postcondition 10): add `finalize_unwired` to
    the `check` value list; state the mapping above; add one test vector ("COMMITTING + verifying record,
    S-25.08 seam ⇒ `_blocked{branch=completion_record_mismatch, reconciliation=completion_record_mismatch,
    check=finalize_unwired}` + `_advisory{branch_c_finalize_unwired}`"); replace PO's `branch=live_txn`
    for this case.

**story-writer** (no BC authoring)

31. **S-25.08 (as authored at v1.21; since v1.22 these ACs live in S-25.09 as AC-001..AC-006 — see
    "v1.22 split mapping" below):** NEW ACs, each with the black-box/unit evidence named: AC-021 (D-2; §5a "Single
    anchoring rule": `resolve_session_project_root` pure fn unit table; real-binary cwd ≠
    `CLAUDE_PROJECT_DIR`; unset/empty; no-`.factory` exit 2; symlink; `grep 'join(".factory'` sweep gate;
    both `migrate-bc-index` routes in `main.rs`); AC-022 (D-1; three InternalLog event types +
    `AdmissionDiagnostic` data type, `main.rs` drain before the early return, constants in
    `internal_log.rs`, no raw `tool_use_id`/record content in any field, `migrate-bc-index` failure line on
    stderr; asserts read `dispatcher-internal-*.jsonl`, never a tracing capture); AC-023 (F-006:
    `run_bc_index_migration_with_ttl` crate-private seam, first-statement validation, 120/1,799 s
    byte-identical snapshot test, 1,800/3,600 s proceed, const assertion present); AC-024 (F-012:
    `AdmissionStateIntegrity`/`AdmissionStateIntegrityKind`, five raise-site conversions,
    `AdmissionFailureCause` enum, exhaustive match, Display lacks `BINARY_INTEGRITY_FAILURE`, taxonomy
    rows); AC-025 (F-013: `NativeGate` enum, black-box `blocking_plugins=migration-admission` vs
    `shard-cap-gate`); AC-026 (F-014: the three renames, sibling sweep of every call site including
    `main.rs`, tests and Kani harness comments; no `append_log_backfill_*` symbol exists). Edit
    AC-002/AC-009 (and the §Architecture Mapping / file-list rows at the lines naming the entry points):
    replace every `append_log_backfill_admission_precheck` / `append_log_backfill_reservation_release`
    reference with the neutral names and delete "delegate". File list gains
    `crates/factory-dispatcher/src/internal_log.rs`, `src/lib.rs` (if the pure resolver is exported
    there) and the existing `main.rs`/`shard_manager.rs`/`executor.rs`/`shard_manager/admission.rs`.
    Cite BC-1.18.013 v1.10 / BC-1.18.011 v1.18 / BC-3.08.001 v1.35 / error-taxonomy v1.39 / VP-143 v1.5
    / VP-146 v1.4. Tasks: T-16..T-21 mirror AC-021..AC-026.
32. **S-25.06:** re-point every `append_log_backfill_admission_precheck` /
    `append_log_backfill_reservation_release` mention (catalog/§Architecture rows, the file-list row,
    the two executor-scope paragraphs) to the neutral names; NEW AC: the `backfill-append-logs`
    CLI resolves its factory root with the single anchoring rule (reusing S-25.09's function since v1.22,
    formerly S-25.08's; exit 2 `FACTORY_ROOT_NOT_FOUND`), its production entry delegates to a crate-private
    `…_with_ttl` seam (EC-031 shape), and its failures/advisories go to stderr; S-25.06 depends on
    S-25.09's function (S-25.09 must be in `depends_on`, transitively S-25.08). STORY-INDEX row/version bump per normal propagation
    (no `bcs:` change ⇒ POLICY 8 atomicity not triggered).

### v1.22 split mapping (S-25.08 → S-25.08 + NEW S-25.09; human decision D-1252(f), amended: AC-027 stays in S-25.08)

Story-ownership change only: **no design, state-machine, BC, taxonomy or VP-count change.** The split cut is
`feature/S-25.08` commit `5091f88f` (S-25.08 = `ce2421be..5091f88f`; S-25.09 = the 11 commits
`5091f88f..39e89c59`). Story-writer already carries the stories (S-25.08 v1.5, S-25.09 v1.0); this table is the
ADR-side anchor so every story citation in this ADR resolves.

| ADR-052 item | Rulings | Story | AC (S-25.09 numbering; formerly S-25.08) |
|---|---|---|---|
| v1.18–v1.20: shared admission core, reserve-then-verify, Branches A/B/C, release on `PostToolUse`/`PostToolUseFailure`, `factory_root` gate anchoring, `resolve_target_path`, registry-independent position, TTL/timestamp rules, named variants; B2 conformance; VP-147 re-baseline (items 14–22) | F-001..F-009, B2-1..B2-4 | **S-25.08** | AC-001..AC-020 |
| Registry fail-closed exit mapping (BC-7.06.001 / BC-1.08.001) | — | **S-25.08** (stays; code `0cc54c94`, tests before the cut) | AC-027 |
| v1.21 D-2 single anchoring rule (§5a "Single anchoring rule"; items 23, 31) | D-2 | **S-25.09** | AC-001 (formerly AC-021) |
| v1.21 D-1 admission diagnostics as InternalLog events + `check` domain (§5a "Admission diagnostics channel", "Closed `check` domain"; items 24, 25, 33(a)–(g)) | D-1 | **S-25.09** | AC-002 (formerly AC-022) |
| v1.21 F-006 EC-031 crate-private TTL seam (item 26) | F-006 | **S-25.09** | AC-003 (formerly AC-023) |
| v1.21 F-012 `AdmissionStateIntegrity` + exhaustive cause match (item 27) | F-012 | **S-25.09** | AC-004 (formerly AC-024) |
| v1.21 F-013 `NativeGate` / `blocking_plugins=migration-admission` (item 28) | F-013 | **S-25.09** | AC-005 (formerly AC-025) |
| v1.21 F-014 neutral entry-point names (item 29) | F-014 | **S-25.09** | AC-006 (formerly AC-026) |
| v1.22 read-failure mapping (§Error Code Semantics) | — | conformance vector under S-25.09 AC-002 (BC-1.18.013 EC-037 row (10)); implementation already conformant at `39e89c59` | AC-002 |

Dependency/sequencing consequences (story-writer/STORY-INDEX already carry them; stated so the ADR is
self-consistent): S-25.09 `depends_on` S-25.08; S-12.16, S-25.06, S-26.06 and S-6.03 depend on S-25.09; the
S-25.06 `backfill-append-logs` coordinator AC (item 32) consumes S-25.09's `resolve_session_project_root` /
`FactoryRootNotFound { root_source }`; the "S-25.08 seam" wording in items 33(c)/(g) names the fail-closed
finalize seam whose CODE is S-25.08's and whose `finalize_unwired` diagnostics are S-25.09's; the verification
property anchors follow the same split (VP-133 v1.7 facet 7(e), VP-143 v1.6 v1.5-D1 vectors, VP-079 v1.25 →
S-25.09; VP-147 and VP-146 stay with S-25.08/S-25.06).

**implementer (S-25.09 since v1.22, formerly S-25.08; for the story-writer to carry as tasks)** — beyond the AC list: remove the
`_cwd` underscore names in the coordinator signatures (they are used); `FactoryRoot::migration_state_dir()`
is already `pub`; the executor outcome synthesizers gain the `NativeGate` parameter; no change to
the E-MAINTENANCE message strings or exit codes.

---

### v1.23 (S-25.08 local adversary pass 3: F-S2508-L3-009, F-S2508-L3-001) — BC, taxonomy and story deltas

No state-machine, taxonomy-code, VP-count or module change. Two interpretive rulings (§5a "Factory-root
lookup mapping"; §Error Code Semantics "Txn-record interpretation — tiers").

**product-owner (BC-1.18.013 v1.11→v1.12; BC-1.18.011 v1.19→v1.20 shared-core sibling sweep):**
- BC-1.18.013 Precondition 6(b)(iii): append "`factory_root` is OUT OF SCOPE only when `<project_root>/.factory`
  is ABSENT — `stat` reports `ENOENT` or `ENOTDIR`, or succeeds on a non-directory (including a regular
  file). A `stat` failure of any other kind (EACCES, EPERM, EIO, ESTALE, ELOOP, …) leaves the existence of
  the directory unknown and FAILS CLOSED: `E-MAINTENANCE-002` `<cause>`=`io`, one
  `migration.admission_failed`, no reservation, the write not admitted (ADR-052 §5a 'Factory-root lookup
  mapping')."
- BC-1.18.013 Precondition 6(c) `io` bullet: add "`stat` of `<project_root>/.factory`" to the enumerated
  calls. Release leg (Postcondition 10(c) advisory `reservation_release_failed`): add "also when
  `resolve_factory_root` reports the `.factory` stat as unstatable; `detail` carries the sanitized path,
  `ErrorKind` and message; never a verdict."
- BC-1.18.013 Precondition 7: append "A `stat` failure of `<project_root>/.factory` other than
  `ENOENT`/`ENOTDIR`/not-a-directory exits 2 with `BcIndexMigrationError::Io { path, source }`, NOT
  `FACTORY_ROOT_NOT_FOUND`; nothing is created or mutated." EC-035: keep "(absent, or a regular file)",
  add "dangling symlink and a project root that is itself a regular file (ENOTDIR) are also absent". NEW
  EC rows (next free numbers): unstatable `.factory` on each of admission / release / coordinator
  (chmod-000 project root, ELOOP self-symlink), with vectors.
- BC-1.18.013 Precondition 6(c) rule 3: append "Rule 3 is the ONLY record interpretation applied to a
  record that no executing branch consumes. All other fields are read lazily by the consuming branch only
  (Branch B: `generation_id`; Branch C verification: `activation_id`, `generation_id`,
  `pending_canonical_moves`): a PRESENT `generation_id` that is JSON `null` is the null generation, a
  string is not, and an ABSENT key or any other type is `E-MAINTENANCE-002` `state_integrity` with no txn
  or gate write — an absent key is NEVER read as null. A shape-valid live record that no consuming branch
  reaches blocks with plain `E-MAINTENANCE-001`; a foreign record (`migration_id ∉ K`) is refused plain
  with no field beyond rule 3 read; COMPLETED/ABORTED records, foreign or known, of valid rule-3 shape are
  not live and are admitted (Branch A reads no field)." NEW EC rows + vectors: minimal `{state,
  migration_id}` foreign live STAGING/COMMITTING (plain block, both families, gate OPEN/DRAINING/LOCKED);
  minimal known STAGING without `generation_id` at gate DRAINING with flock free (`state_integrity`, txn
  bytes unchanged); known STAGING with `"generation_id": null` (discard, unchanged); minimal known
  COMMITTING, no terminal record (plain block); minimal COMPLETED/ABORTED, foreign and known (admitted;
  gate stuck ⇒ Branch A reopen).
- BC-1.18.011 v1.20: mirror the 6(b)/(c)/(d) wording above in the shared-core preconditions (no divergence
  between the two BCs). error-taxonomy: `E-MAINTENANCE-002` `io` trigger gains the `.factory` stat;
  `FACTORY_ROOT_NOT_FOUND` trigger gains "stat reports absent (never a non-ENOENT stat failure, which is
  `Io`)". BC-3.08.001 Events 12/13: no new field or token; `detail` may carry the `.factory` path.
**story-writer:** S-25.08 gains the admission/release `.factory`-stat ACs and the tiered txn-read AC
(below); S-25.09 AC-001 gains the coordinator `Io` leg and AC-002 the `_failed`/`_advisory` events for the
unstatable case; S-25.06 Branch C field-consumption rule (Tier 1 table) enters its verification AC.
**v1.23 binary-leg extension (same burst; closes S-25.06 AC-023 / T-8n(d) open question):**
- **product-owner — BC-1.18.011 v1.20 (extend in place):** migrate-bc-index recovery preconditions/postconditions
  gain the "Migration binaries — recovery and finalize legs" rule: Tier 0 loader (UTF-8/shape/`migration_id`
  ⇒ `MIGRATION_STATE_INTEGRITY_FAILURE` `TxnRecordMalformed`/`TxnMigrationIdNotString`, exit 2, nothing
  mutated); strict-presence typed decode of the one live known record at arm entry; STAGING `generation_id`
  tri-state (absent / non-string non-null ⇒ exit 2 no mutation; `null` ⇒ discard unchanged); COMMITTING
  `generation_id` must be a string; terminal/foreign records never rejected for a missing Tier 1 field. NEW
  EC rows + vectors: STAGING with `generation_id` key removed (exit 2, txn bytes unchanged — NOT the discard);
  COMMITTING with `generation_id` `null`/absent/number (exit 2, unchanged); live record missing
  `activation_id`/`fencing_generation`/`pending_canonical_moves` (exit 2, unchanged); COMPLETED record from a
  newer schema beside a live known record (not rejected).
- **product-owner — BC-1.18.013 v1.12 (extend in place):** Postcondition 5a(b) and EC-047 gain the binary leg:
  a malformed Tier 1 txn field at its check is `MIGRATION_STATE_INTEGRITY_FAILURE` (exit 2), a well-formed
  disagreeing field is `COMPLETION_RECORD_MISMATCH_ABORT` (exit 2), earlier check wins; neither finalizes
  nor flips. **Correct the hash source:** Precondition 6(c) rule 3 / Postcondition 5a(b) / EC-010 /
  EC-047 cite `pending_canonical_moves[].expected_post_hash` — replace by "the intent-log DONE record's
  `expected_post_hash` located via the txn's Tier 1 `intent_log_path`" (see §Error Code Semantics "Branch C
  hash source"); NEW EC rows: `intent_log_path` absent/ill-typed (state_integrity both surfaces), intent log
  missing / path without DONE (`canonical_hash_mismatch`).
- **product-owner — error-taxonomy:** `MIGRATION_STATE_INTEGRITY_FAILURE` trigger gains the Tier 1 clause;
  `COMPLETION_RECORD_MISMATCH_ABORT` trigger gains "fields well-formed but disagreeing; a malformed txn field
  is `MIGRATION_STATE_INTEGRITY_FAILURE`"; `E-MAINTENANCE-002` `state_integrity` row's Branch C field list
  becomes `activation_id`/`generation_id`/`intent_log_path`.
- **story-writer:** S-25.06 AC-023 "Tier 1 Branch C lazy-field rule" paragraph: replace the "ruling does NOT
  specify the migration BINARY's exit mapping … reported to the architect" sentence with this ruling, fix the
  hash-source field, extend T-8n(d) with binary vectors (exit 2, stderr names `MIGRATION_STATE_INTEGRITY_FAILURE`,
  byte-identical `migration-state/` snapshot, mismatch-wins-over-malformation ordering). S-25.09 gains a
  coordinator-leg AC (below) and AC-004's conversion list gains the loader/`Quarantine` sites.
- **Code ownership of the binary leg.** S-25.09 (owns the `shard_manager.rs` coordinator and its loaders):
  replace `read_all_txn_records`' full typed `serde_json::from_str::<BcIndexMigrationTxnRecord>` with the
  Tier 0 raw reader; add the strict-presence decode at the `ResumeFromStaging` / `ForwardRecovery` /
  `CleanAbortExpiredStaging` arm entries; resolve the `generation_id` tri-state before `recover()` (the pure
  decision table and its OBL-1 / VP-147 harnesses are unchanged; only the shell-side input adaptation moves);
  route `DiscardPreGeneration` through the shared raw `abort_null_generation_txn`; map
  `Quarantine::CommittingWithoutGenerationId` and the malformed-record raises to `AdmissionStateIntegrity`
  (exit 2) instead of `BinaryIntegrityFailure`; derive the terminal-record archive path from the file path.
  S-25.08 (admission-side, S-25.08 tip `e7ce7b62`): no change beyond making `abort_null_generation_txn`
  callable from the coordinator (visibility only). S-25.06: the Branch C verifier and the
  `backfill-append-logs` verify-then-finalize read `activation_id` / `generation_id` / `intent_log_path`
  lazily under item 6, and S-25.06's coordinator reuses S-25.09's loader (no second loader).
- **Observed code state (S-25.09 @ `1120f900`, read-only).** `read_all_txn_records` deserializes every
  `txn-*.json` fully and maps failure (and a missing `activation_id`/`pending_canonical_moves`) to
  `BinaryIntegrityFailure` exit 2 — right exit, wrong variant, over-strict for terminal/foreign records;
  `generation_id: Option<String>` with no strict presence reads an ABSENT key as `None`, so
  `recover()` returns `DiscardPreGeneration` and `discard_incomplete_staging` moves a STAGING txn to
  ABORTED on an unproven precondition — a fail-closed violation that DISAGREES with this ruling, in
  S-25.09 scope; COMMITTING with no `generation_id` ⇒ `Quarantine` ⇒ `BinaryIntegrityFailure` (right exit and
  no mutation, wrong variant). Adjacent, same arm: the `DiscardPreGeneration` arm returns
  `BinaryIntegrityFailure("resumed STAGING txn record has no generation_id")` after discarding, where §4e
  assigns `EXPIRY_ABORT` (exit 1) to the null-generation discard — S-25.09 to reconcile the arm in the same
  change. `Quarantine::GenerationIdWithoutGenDir` / `MultipleLiveTxnRecords` are not Tier 1 field
  questions: the latter is the v1.21 `MultipleLiveTxns` kind (S-25.09 AC-004 sibling site); the former is a
  well-typed record contradicting disk and keeps exit 2 / no mutation, its variant label unchanged here.
**v1.23 second binary-leg extension (same uncommitted v1.23; closes the four S-25.09 recovery-leg red-test
questions, commit `37a93210`) — owed deltas.** Rulings: §Error Code Semantics "Migration binaries — recovery
and finalize legs" items 8-11.
- **product-owner — BC-1.18.011 v1.20 (extend in place):** (Q1) Precondition 6(f)(iv) and EC-047 gain the two
  normative `EXPIRY_ABORT` lines of item 8 (`ExpiryAbort { arm }`; printed only after txn → ABORTED then gate →
  OPEN succeeded; a failed write is `Io` exit 2); EC-043 control row cites the null-generation line. (Q2)
  Precondition 6(e), 6(f)(ii) and EC-046(e)(f): replace every "`LockContention`-class exit 2" with
  `FOREIGN_MIGRATION_REFUSED` exit 2 and the item-9 line; state the precedence (Tier 0 loader → one-live-txn →
  foreign refusal → `recover()`); NEW EC-048 flock EWOULDBLOCK ⇒ exit 1 `MIGRATION_LOCK_CONTENTION` (before the
  loader; beside a foreign record the contention wins), replacing the digest-coded text. (Q3) Precondition
  6(f)(iii) enumerates the full key set and types of item 10 (replacing "EVERY key" shorthand with the list),
  adds the unknown-top-level-key rule, the `migration_id` preservation rule and "`generation_id` tri-state
  first; null-generation decodes no other key"; EC-045 gains one vector per remaining key (also
  wrong-typed `fencing_generation`: `"3"`, `3.5`, `-1`) and NEW EC-049 live known STAGING/COMMITTING record with
  an unknown top-level key (exit 2 `txn_record_malformed`, nothing mutated; control: `migration_id` present and
  preserved on rewrite). (Q4) Postcondition 9 and NEW EC-050 describe the interim short-circuit of item 11
  (Tier 0 read, `Err` not swallowed; foreign ⇒ `FOREIGN_MIGRATION_REFUSED`; own live record ⇒ exit 2
  `COMPLETION_RECORD_MISMATCH_ABORT` interim line; no live record ⇒ flock-gated gate reconciliation, zero writes
  in the clean steady state), marked "retired by S-25.06 AC-031".
- **product-owner — BC-1.18.013 v1.12 (extend in place):** EC-016 and Postcondition 5a foreign wording:
  `FOREIGN_MIGRATION_REFUSED` replaces "`LockContention`-class" for the `backfill-append-logs` binary, with the
  `backfill-append-logs: refused: …` line; EC-047(a)-(c) and EC-049 say "`migrate-bc-index` §4e reconciliation
  (S-25.06 AC-031; until it ships the binary is fail-closed per BC-1.18.011 EC-050)" instead of implying an
  existing verifier. The flock-contention exit-1 code is cited from `AppendLogMigrationError::LockContention`.
- **product-owner — error-taxonomy (v1.40, extend in place):** `EXPIRY_ABORT` row gains `Message` with the two
  item-8 lines; NEW rows `FOREIGN_MIGRATION_REFUSED` (Migration binary exit codes, blocked, exit 2, item-9 line)
  and `MIGRATION_LOCK_CONTENTION` (non-error sentinel, exit 1); `COMPLETION_RECORD_MISMATCH_ABORT` row: replace
  "`LockContention`-class exit 2" with `FOREIGN_MIGRATION_REFUSED` and add the interim `migrate-bc-index` line as
  a "no verifier in this build" instance; the v1.40 header says "one new code pair" (the earlier "No new error
  code" statement is superseded for these two rows only).
- **story-writer:** S-25.09: AC for the recovery leg gains the four rulings (items 8-11); S-25.09 owns the
  interim short-circuit and the `ExpiryAbort`/`ForeignMigrationRefused`/`MigrationLockContention` variants and
  their exhaustive-match sites. S-25.06: AC-027 and AC-031 replace "`LockContention`-class" by
  `FOREIGN_MIGRATION_REFUSED`; AC-031 states it OWNS `migrate-bc-index` verify-then-finalize and retires the
  interim; the `backfill-append-logs` binary gets the mirrored `ForeignMigrationRefused`, an `ExpiryAbort { arm }`
  equivalent for the shared null-generation discard, and aligns its `LockContention` to `MIGRATION_LOCK_CONTENTION`.
- **Code and tests (S-25.09 @ `37a93210`, read-only):** variants `ExpiryAbort` → `ExpiryAbort { arm }` (the
  unit-variant uses in `s2508_v121_units_test.rs`, `bc_1_18_011_b2_migration_test.rs` and the two
  exhaustive matches in `shard_manager.rs` follow), `ForeignMigrationRefused`, `MigrationLockContention`
  (`process_exit_code`: `ExpiryAbort`, `MigrationLockContention` ⇒ 1, all else 2); the `CleanAbortExpiredStaging`
  and null-generation arms stop using `let _ = discard_…` and write gate OPEN after txn ABORTED;
  `read_all_txn_records` → Tier 0 raw loader; strict decode over the 11-key set plus unknown-key rejection;
  short-circuit narrowed per item 11(a)-(c). Red-test corrections: EC046 `(e)(f)` and the EC047 test pin the
  item-8/9 lines exactly instead of `contains`; add tests for unknown key, wrong-typed `fencing_generation`,
  flock contention (exit 1, line, nothing mutated), gate-write failure ⇒ exit 2 not exit 1, and the three
  short-circuit vectors.
**Code ownership.** S-25.08 (resolver at the cut): `resolve_factory_root` returns
`Result<Option<FactoryRoot>, BcIndexMigrationError>` with the closed classification; the admission leg maps
`Err` to the existing `E-MAINTENANCE-002 (io)` path; the release leg maps `Err` to no verdict + the
S-25.08 diagnostic channel (`tracing::warn!`); `read_txn_files` is Tier 0 only (raw object, `state`,
`migration_id`); Branch B tri-state resolved before the unchanged planner; diagnostic `txn_id` read from
raw with `unknown` fallback. S-25.09 (rebases onto S-25.08): the coordinator leg (`Absent` ⇒
`FactoryRootNotFound`, `Err` ⇒ propagate `Io`), the `_failed` (`cause=io`) and `_advisory`
(`reservation_release_failed`) InternalLog events for the unstatable case, `TxnRecordMalformed` carries
the Branch B unusable-`generation_id` failure. S-25.06: Branch C Tier 1 reads when verification is wired.

**v1.23 third binary-leg extension (same uncommitted v1.23; independent validation of items 9-11,
`.factory/research/adr-052-v123-architect-calls-validation.md`) — owed deltas.** Rulings: §Error Code Semantics
items 7(e), 9 (cross-ref), 10(a)-(d), 11(c)-(e). No taxonomy CODE is added; one `AdmissionStateIntegrityKind`
token is added (wire-domain change: BC-3.08.001 bump). Supersedes, where they conflict, the preceding "second
binary-leg extension" bullets on exit-0 contention and the five-kind domain.
- **product-owner — BC-1.18.011 v1.20 (extend in place):** (1) Precondition 6(f)/EC-047/Postcondition 9: exit-1
  definition "no harm done, safe to re-run; the stderr code says what to do next" covering `EXPIRY_ABORT` AND
  `MIGRATION_LOCK_CONTENTION`; remove any "exit 1 = re-activation" wording. (2) EDIT EC-048: flock not
  acquired ⇒ exit 1 `MIGRATION_LOCK_CONTENTION` on EVERY path; add the vector `completed.json` present + flock held
  ⇒ exit 1 (NOT `ALREADY_MIGRATED` exit 0), nothing read/written; vector: `completed.json` + own live COMMITTING
  txn + flock held ⇒ exit 1 (and uncontended ⇒ exit 2: same state, timing-independent fail-closed verdict);
  vector: live foreign record + `completed.json` + flock held ⇒ exit 1. (3) Postcondition 9 / EC-050 (interim
  short-circuit): `completed.json` is read UNDER the lock; its read is what `recover()` receives; new EC for the
  TOCTOU window (`completed.json` + COMPLETED txn appear between the probe and lock acquisition ⇒ no fresh run, gate
  untouched). (4) Precondition 6(f)(iii): key set is TWELVE (adds `schema_version`, required u32 = 1, no default);
  nested rule: each `pending_canonical_moves[]` element is an object with exactly `{staging_path, canonical_path}`
  strings; version gate wording and order (item 10(b)); EC-049 vectors: unknown key inside an element (exit 2
  `txn_record_malformed`, detail names `pending_canonical_moves[i]`, nothing mutated), `schema_version` absent
  (malformed), `0`/`"1"`/`1.5`/`null` (malformed), `2` and `4294967296` (newer-schema, even when the record also has
  unknown keys or lacks keys), `1` + unknown key (malformed); null-generation discard vectors: `schema_version: 2`
  ⇒ `txn_record_newer_schema`, txn bytes unchanged (NOT the discard); `schema_version` absent or `1` +
  unknown extra key ⇒ discard succeeds and the extra key is PRESERVED in the ABORTED record (control). (5) NEW
  postcondition: `finish_committing_migration` propagates a failed COMPLETED txn write (`Io`, exit 2, gate not
  opened, no success claim); abort-path cleanup rule of item 11(d) (txn ABORTED checked before gate OPEN; failure
  is the write's `Io` naming the original code). (6) NEW "Operator recovery of the interim block" subsection
  mirroring item 11(e) steps 1-6, marked "retired by S-25.06 AC-031".
- **product-owner — BC-1.18.013 v1.12 (extend in place):** (1) Branch C / Postcondition 5a: a `schema_version` ≥ 2
  at any Tier 1 consumer is `E-MAINTENANCE-002` `state_integrity` kind `txn_record_newer_schema` (version gate
  precedes every field read); Branch B discard row: version gate first, then `generation_id` tri-state; NEW EC rows
  and vectors mirroring the BC-1.18.011 discard vectors (admission surface: `E-MAINTENANCE-002`, one
  `migration.admission_failed` with `cause=state_integrity`, `kind=txn_record_newer_schema`, no `_blocked`, txn
  bytes unchanged). (2) Exit-1 class wording and the `backfill-append-logs` contention rule (EC-016 / EC-047 area)
  identical to BC-1.18.011: `MIGRATION_LOCK_CONTENTION` exit 1 on every path incl. `completed*.json` present;
  `completed*.json` read under the lock.
- **product-owner — BC-3.08.001 (v1.36 → v1.37, REQUIRED bump):** Event 12 `migration.admission_failed` `kind`
  closed domain gains `txn_record_newer_schema` (six tokens: `gate_record_malformed | txn_record_malformed |
  txn_migration_id_not_string | multiple_live_txns | reservation_serialization | txn_record_newer_schema`); every
  prose count of "five" kinds in the BC, its invariants and canonical test vectors becomes six; no new event, no
  new field, no event-count change; changelog row "additive wire-domain token". Propagate (same burst, POLICY 9
  neighborhood): VP-079 Event 12 vectors and VP-133 facet 7(e) emission check gain the token (VP-INDEX version
  bump only if their text changes; no VP-count change); BC-INDEX row for BC-3.08.001 version cite.
- **product-owner — error-taxonomy (v1.40, extend in place):** (1) `EXPIRY_ABORT` row and
  `MIGRATION_LOCK_CONTENTION` row: define exit 1 as "NON-ERROR termination — no harm done, safe to re-run; the
  stderr code says what to do next" (re-activate vs retry after the holder exits); add a one-line "Exit-code
  classes" note under the Migration binary exit codes table (0 / 1 / 2 by class, exhaustive match). (2)
  `MIGRATION_LOCK_CONTENTION` trigger: "on every path including `completed.json` present; never exit 0".
  (3) `ALREADY_MIGRATED` row: "emitted only for a state read under `exclusive.lock`". (4)
  `MIGRATION_STATE_INTEGRITY_FAILURE` row: trigger gains "unknown key in a `pending_canonical_moves[]` element;
  missing/ill-typed `schema_version`"; NEW Message variant for kind `txn_record_newer_schema` with the
  normative `Display` of item 10(c) ("written by a newer build; recover it with that build"), same code and exit 2;
  `E-MAINTENANCE-002` `state_integrity` row lists SIX kinds (the v1.39 "five kinds" text). (5)
  `COMPLETION_RECORD_MISMATCH_ABORT` interim-line row: add "operator procedure: ADR-052 item 11(e)". (6) v1.40
  header: no new CODE beyond the earlier pair; one new kind token.
- **story-writer:** S-25.09 AC list gains: exit-code exhaustiveness (item 7(e)); nested strictness and
  `schema_version` (item 10(a)-(d)); lock-first/under-lock `completed.json` read and contention-on-every-path
  (11(c)); COMPLETED-write and abort-path propagation (11(d)); operator runbook entry (11(e)), devops-engineer
  deliverable. S-25.08 (admission surface, code now owned by S-25.09 per v1.22): AC for the Branch B version
  gate and the sixth kind. S-25.06: AC-031 states it retires the interim AND the manual procedure, and replaces
  every "EWOULDBLOCK ⇒ exit 0" with contention exit 1 in its own `backfill-append-logs` ACs.
- **Code changes (S-25.09, `shard_manager.rs` / `shard_manager/admission.rs` / `main.rs` @ `46169f0d`):**
  (a) `BcIndexMigrationError::process_exit_code`: delete `_ => 2`; list every variant (`ExpiryAbort { .. } |
  MigrationLockContention => 1`; all others named => 2); same for `AppendLogMigrationError` when S-25.06 lands.
  (b) `run_bc_index_migration_core`: remove the pre-lock `completed_present` branch selection; acquire
  `exclusive.lock` first, `None` ⇒ `MigrationLockContention` on every path (delete the `AlreadyMigrated` return
  for the not-acquired case); then read `completed.json` under the lock, branch on that read, pass it to
  `recover()` in place of the hard-coded `None` (and drop the "honest current read" comment).
  (c) `finish_committing_migration`: `write_txn_record(fs, migration_state_dir, txn)?` (no `let _ =`); do not
  write the gate or return `Completed` on failure. (d) step-3b `abort_staging` closure and the four
  `let _ = discard_incomplete_staging(..)` sites: checked txn ABORTED write, then gate OPEN only on success,
  failure returned as `Io` carrying the original code token (closure becomes fallible). (e) `PendingCanonicalMove`:
  explicit element key-set check with the shared sanitizer (+ optional `deny_unknown_fields`). (f) txn record:
  `schema_version: u32` field, `TXN_RECORD_SCHEMA_VERSION = 1`, set on creation and kept by
  `write_txn_record`, added to `TXN_RECORD_REQUIRED_KEYS` (12); `decode_txn_record_strict` runs the version gate
  first. (g) `AdmissionStateIntegrityKind::TxnRecordNewerSchema` + `as_str` + Display; `admission_failure_cause`
  arm (stays exhaustive); `abort_null_generation_txn` applies the version gate before the tri-state (both
  surfaces) and keeps preserving unknown keys otherwise; `main.rs` emits the new `kind` token unchanged through the
  existing `migration.admission_failed` writer.
- **Red tests owed (all fail before the code change):** S-25.09: (1) exit-code table test enumerating every
  `BcIndexMigrationError` variant (exhaustive, no wildcard); (2) flock held + `completed.json` present ⇒ exit 1
  `MIGRATION_LOCK_CONTENTION`, byte-identical `migration-state/` snapshot; same with own live COMMITTING txn and
  with a foreign live record; (3) TOCTOU: via the `Fs` seam, `completed.json` + COMPLETED txn appear between the
  probe and lock acquisition ⇒ no fresh run, gate untouched, exit 0 `AlreadyMigrated` read under the lock; (4)
  `completed.json` present, lock free, own live COMMITTING txn ⇒ exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` (control for
  timing independence); (5) fault-injected failure of the COMPLETED txn write in `finish_committing_migration`
  (`Fs` seam) ⇒ exit 2 `Io`, not exit 0, gate not OPEN, no `Completed` outcome; (6) fault-injected failure of the
  ABORTED txn write in `abort_staging` and in each `discard_incomplete_staging` caller ⇒ `Io` exit 2 naming the
  original token, gate NOT written OPEN; (7) unknown key inside a `pending_canonical_moves[]` element at each of
  the three rewriting arms ⇒ exit 2 `txn_record_malformed`, record bytes unchanged, detail sanitized (64-char
  cap, control chars escaped); non-object element; missing `staging_path`; (8) `schema_version` vector table of
  item 10(b) (absent, 0, "1", 1.5, null, 1, 2, u32::MAX+1) at each rewriting arm with exact kind/token and
  unchanged bytes; newer-schema wins over unknown/missing keys and nested keys; written records carry
  `schema_version: 1` (round-trip through `write_txn_record`); (9) newer-schema `Display` pinned exactly and free
  of `txn_record_malformed`/`corrupt`; (10) null-generation discard: `schema_version: 2` ⇒ `txn_record_newer_schema`
  on BOTH surfaces (coordinator and admission Branch B), bytes unchanged; absent/`1` + unknown key ⇒ discard with the
  key preserved. S-25.08 surface: Branch B version-gate vector with `migration.admission_failed`
  `kind=txn_record_newer_schema`, exactly one event. BC-3.08.001 closed-domain test gains the token. S-25.06: the
  `backfill-append-logs` mirrors of (2)-(3) and (7)-(8) when it lands.

**v1.23 fourth extension (same uncommitted v1.23; S-25.09 operator-runbook findings at `c4e9f717`, six
spec/code disagreements adjudicated against the merged decisions; no design/state-machine/taxonomy-code/VP-count
change).** Classification: item 1 `txn_id` = CODE DEFECT (spec wins; no spec edit). Item 2 `intent_log_path` =
CODE GAP against Decision 7c step 1 (spec wins; the v1.23 text only states the existing rule and forbids a
derived fallback). Item 3 = runbook error for "JSON"/`canonical_path` (spec and code agree); the byte
encoding of the log differs from the Decision 7b literal block = CODE DEFECT strictly (human decision gate
below). Item 4 = new ruling (no earlier spec; exit-0 silence). Item 5 = runbook error (spec already says
`gate-state.json` / bare uppercase JSON string; step 5 tightened). Item 6 = v1.23 11(e) text error (corrected to
the merged "N = number of canonical paths in the txn record / intent log").
- **product-owner owes (BC-1.18.011, then BC-1.18.013 sibling sweep, extend in place at their current
  versions):** (a) BC-1.18.011 "Operator recovery of the interim block" steps 2-3 and 5-6 (the section mirrored
  from item 11(e)): replace "`canonical_paths_count` == the number of `pending_canonical_moves` (1 for
  `migrate-bc-index`)" with the N rule above; add the `txn_id` ≠ `txn-<id>` note, the non-null
  `intent_log_path` + `intent-<generation_id>.log` equality requirement, "plain-text framed blocks, key
  `target_canonical`", and the exit-0-silent step 6; step 5 names the bare `"OPEN"` string. (b) BC-1.18.011
  Precondition 6(f)(iii) / Postcondition 9(b): add that `intent_log_path` is persisted with `generation_id`
  (7c step 1), is `null` only while `generation_id` is null, and that a null value at the Branch C hash check is
  `state_integrity` with NO derived fallback; EC-047 gains the null-`intent_log_path`-on-COMMITTING vector.
  (c) Postcondition 8/9 and EC-006: `AlreadyMigrated`/`Completed` print nothing; exit status is the whole
  signal. (d) BC-1.18.013 Postcondition 5a / Precondition 6(c): the same `intent_log_path` and silence wording
  for `backfill-append-logs`; its "fixed N = 4" stays. (e) **error-taxonomy.md:** `ALREADY_MIGRATED` Message
  Format gains "No output is printed (exit status 0 is the sentinel)"; `COMPLETION_RECORD_MISMATCH_ABORT`
  trigger (a) "fixed target count N (4 for `backfill-append-logs`)" gains "; for `migrate-bc-index` the number
  of canonical paths in the txn's move plan"; the row's (b) already states `activation_id` == terminal
  `txn_id` and needs no change. (f) **story-writer:** S-25.09 (or the story that owns the coordinator) AC
  for the code fixes below; S-25.06 AC-023 Tier 1 Branch C lazy-field rule already lists `intent_log_path` and
  is unchanged.
- **Code fixes owed to the implementer (`shard_manager.rs`, spec wins):** (1) `txn_id`: create the record
  with `txn_id = activation_id` (currently `format!("txn-{activation_id}")`); the same value flows to the
  intent log `txn_id`, `CURRENT.json.txn_id` and `completed.json.txn_id`; Red test: a genuine run's
  `completed.json.txn_id == txn.activation_id`, and the Branch C `txn_id_eq` fact is true on a record this build
  wrote. (2) `intent_log_path`: set `txn.intent_log_path = Some(<the intent-<generation_id>.log path>)` in the
  same `write_txn_record` that assigns `generation_id` (fresh-run path, after the generation directory is
  durable) and idempotently on the STAGING-resume path; Red test: after a fresh run the txn record's
  `intent_log_path` is a string equal to the appended log's path; key still always present. (3) Intent-log
  byte encoding (human gate): Decision 7b's literal block is `--- INTENT_LOG_RECORD v1 ---` /
  `key: value` lines / `--- END_RECORD ---` with `expected_pre_state: missing`; the code writes
  `INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD` with `MISSING`. By CLAUDE.md rule 12 the
  code is brought to the ADR unless the human authorizes amending Decision 7b to the shipped encoding (the
  amendment text would be a one-line replacement of the 7b block); not decided here.
  **RESOLVED in v1.24 (human authorization 2026-10-08):** the human authorized amending 7b to the code's
  tokens PLUS the hardened format (ADR-054 Decision 1); delivered by S-25.10, not by S-25.09.
- **Open item for the human (architect finding, not a ruling):** Decision 7a describes
  `pending_canonical_moves` as "not yet completed" and 7c step 7d says each completed move updates it; the code
  retains the FULL plan for the life of the txn (completion is tracked only by intent-log `DONE` records). The
  v1.23 consumers (N, "every canonical path", 11(e) step 3) require the retained full plan or the intent log's
  `INTENT` set; a list that shrank to empty would verify vacuously. Until decided, the 11(e) text above
  cross-checks the two sources and the verifier (S-25.06) MUST enumerate from the intent log's `INTENT` records.
  **RESOLVED in v1.24 (human authorization 2026-10-08):** the fixed-plan model is adopted, the field is
  renamed `canonical_move_plan`, and B-1/B-2/B-3 are specified (ADR-054 Decisions 2-3); the "enumerate from
  the INTENT records" fallback is superseded by the mandatory plan/INTENT-set equality (B-3).

### v1.24 (human-authorized amendment 2026-10-08: hardened intent log, fixed move plan) — pointer block

The BC, taxonomy, VP and story deltas of v1.24 are NOT written here (this section is already the largest
part of this ADR and exhausts the hook fuel budget): the complete **S-25.10 downstream block** (S-25.10
acceptance criteria AC-001..AC-019, the red-test plan T1-T17, the code changes per story, and the owed
BC-1.18.011 / BC-1.18.013 / error-taxonomy / VP-133 / VP-143 / VP-146 / VP-147 / VP-INDEX wording) lives in
**ADR-054 §"Downstream to Product-Owner / Story-Writer / Test-Writer — S-25.10 block"** and is the
authority for those edits. Read-rule for this ADR's own history: every occurrence of `pending_canonical_moves`,
`PendingCanonicalMove`, `--- INTENT_LOG_RECORD v1 ---` or `--- END_RECORD ---` in the v1.18-v1.23 Downstream
blocks above and in the Changelog is the then-current name recorded as history; the v1.24 names are
`canonical_move_plan`, `PlannedCanonicalMove`, `INTENT_LOG_RECORD_V1` and `END_INTENT_LOG_RECORD`, and where
a v1.20-v1.23 block instructs the product-owner to write the old name, the v1.24 names in ADR-054 govern the
wording that is applied.

**v1.24 same-version ruling — admission evaluation order (S-25.08 local adversary pass 4, F-S2508-L4-002,
LOW).** Ruled in §Error Code Semantics "`E-MAINTENANCE-002` `<cause>` — total decision rule": whole-function
order = input guards → `.factory` `stat` → target resolution/classification (scope) → `tool_use_id` validity
(in-scope protected writes only) → W1 → gate-state → txn records → terminal record. **Wording owed by the
product-owner (BC-1.18.013 v1.14 Precondition 6(c) "total decision rule", and the mirror in
BC-1.18.011):** (a) replace the parenthetical "evaluation order: the `tool_use_id` check, then …" with
"evaluation order, after the input guards, the `.factory` `stat` and the protected-path classification
(an out-of-scope write is admitted without examining `tool_use_id`): the `tool_use_id` check, then …";
(b) rule 1: replace "Decided from the payload alone, BEFORE any filesystem access." with "Decided from the
payload alone (the validity decision reads no filesystem state); evaluated only for an in-scope protected
write, i.e. after the `.factory` `stat` and the target classification, so a failing `.factory` `stat`
reports `io` even when the id is also invalid."; (c) rule 2 (v1.12): keep "the FIRST OS call of the check
is the `stat` of `<project_root>/.factory`" unchanged — it is now consistent; (d) add EC vector (no new EC
number; extend EC-032 or EC-041): unstatable `.factory` + `tool_use_id` `../x` ⇒ `io`; statable `.factory`
+ out-of-scope target + invalid id ⇒ admitted, no event; statable `.factory` + in-scope target + `../x` ⇒
`invalid_tool_use_id`. BC-1.18.011 carries the same sentence in its admission-core text (sibling sweep:
grep `BEFORE any filesystem access` and `the \`tool_use_id\` check, then` in both BCs and in
`error-taxonomy.md` `E-MAINTENANCE-002` and fix all occurrences). **Code:** S-25.08 `bc_index_migration_admission`
(`executor.rs`) ALREADY implements this order (guards → `resolve_factory_root` → `resolve_target_path` →
`classify_target` → `classify_tool_use_id`); NO code change; the test-writer should add the three vectors
above as black-box cases. The PostToolUse/Failure release leg is unaffected (an invalid id is a silent no-op
there). Story mapping: **S-25.10** (NEW, stacked on S-25.09; blocks S-25.06 and any
`migrate-bc-index` release) owns the shared intent-log module, the format hardening, the rename and B-1/B-2/B-3;
**S-25.09** keeps only `txn_id = activation_id`, `intent_log_path`, and the `DONE` `txn_id` /
`fencing_generation` plumbing into the move executor; **S-25.06** consumes the shared module after S-25.10.

### v1.26 (S-25.09 local adversary pass 2: F-S2509-L2-001/002/003/006/009) — BC, code and test deltas

Story: **S-25.09** (all code and tests; no new story). No taxonomy code, exit code, VP or VP-count change
(VP-INDEX / verification-architecture / verification-coverage-matrix UNCHANGED); BC versions below are bumps of
committed files.

**Owed by the product-owner (mirror, same burst):**
- **BC-1.18.013 v1.14→v1.15, Postcondition 10 "Coordinators" clause and EC-058..EC-061:** (a) shape (2) token
  domain text: `STAGING_DIR_REMOVE_FAILED` is "an abort path's inert generation-directory removal failed
  (`discard_incomplete_staging` and the fresh-run abort)"; `OVERSIZED_ROW_SUBSHARD` loses the "ONLY if the
  implementer's call-graph audit shows it reachable" qualifier (reachability CONFIRMED) and gains "collected by
  the run that chunked and printed only when that run returns `Completed`; discarded on any error return; not
  emitted by a resumed STAGING or COMMITTING run (no re-derivation, no persistence)"; (b) EC-060 split into
  EC-060a (removal failure at a `discard_incomplete_staging` caller, unchanged) and EC-060b (removal failure
  inside the fresh-run abort closure: ONE `STAGING_DIR_REMOVE_FAILED` line printed after the ABORTED write
  landed, then the original failure's own exit-2 line; if the ABORTED write itself failed, NO advisory, only the
  `Io`), and new vectors EC-064 (oversized row + `Completed` ⇒ exactly one `OVERSIZED_ROW_SUBSHARD` line per
  oversized sub-shard, stdout empty, exit 0), EC-065 (oversized row + a later failure — census/fingerprint/
  intent-append/pointer-swap/canonical-move halt ⇒ NO `OVERSIZED_ROW_SUBSHARD` line, exit 2 line only),
  EC-066 (oversized row, crash after staging, resumed run completes ⇒ no `OVERSIZED_ROW_SUBSHARD` line); (c) the
  per-surface sanitization sentence gains "`Io` renders `path` and `source` as two separately escaped, 256-capped
  substrings; `CleanupWriteFailure` renders its two substrings the same way". (Check EC-064..EC-066 are free in
  the file at edit time; renumber upward if not.) Story Anchor / Traceability / changelog: S-25.09.
- **BC-1.18.010 v1.11→v1.12, §Reader Integration step 2:** replace "Check `CURRENT.json` — if `status:
  committing`" by the lstat-first classification of ADR-052 v1.26 item 11(c) point 6: lstat `NotFound` ⇒ continue
  to step 3; dangling symlink, any other lstat/read error, a directory, content that is not a
  `CurrentGenerationPointer` (string `generation_id`/`status`/`txn_id`), a `status` other than `"committing"`, or an
  empty / multi-component `generation_id` ⇒ `BcIndexAddressingError::Io`, never "migration not started"; the
  shared helper is `read_current_pointer_marker`; the coordinator's resume probe uses the same helper. Story
  Anchor: S-25.09 also delivers step 2 classification. No Precondition/Postcondition/Invariant change.
- **BC-1.18.011 v1.22→v1.23:** (a) Postcondition 9(e)(a0) / reader-parity text gains the `CURRENT.json` clause
  (mirror of BC-1.18.010 step 2); new vectors **EC-078..EC-083** next to EC-070..EC-075: `CURRENT.json` as (EC-078)
  a dangling symlink during COMMITTING with `completed.json` absent ⇒ reader `Io`, NOT `NotStarted`; (EC-079) a
  symlink loop / directory / mode 000 ⇒ `Io`; (EC-080) unparseable / empty / non-UTF-8 / wrong-schema content ⇒
  `Io`; (EC-081) `status` not `"committing"` ⇒ `Io`; (EC-082) empty / `..` / `a/b` `generation_id` ⇒ `Io`;
  (EC-083) valid pointer ⇒ `Committing`, absent ⇒ `NotStarted`, and the coordinator STAGING-resume probe returns
  `Io` (exit 2, nothing mutated, no discard) for EC-078..EC-082; (b) the `abort_staging` removal-failure and the
  `Io` Display sentence (a mirror line, with BC-1.18.013 as the owner). (c) Check EC-078..EC-083 are free; renumber
  upward if not.
- **error-taxonomy.md:** no row change (advisory tokens are not taxonomy codes). If a row quotes the `Io` Display
  as "I/O error at <path>: <os error>", add "(each of `<path>` and `<os error>` escaped and capped at 256
  characters)".
- **BC-3.08.001:** no change (the 64-character InternalLog rule is untouched; no event renders these facts).

**Code deltas (S-25.09, implementer, `crates/factory-dispatcher/src/shard_manager.rs`):**
1. *Item 1.* Delete both calls in `close_sub_shard_chunk` (`tracing::warn!` and `emit_coordinator_advisory`);
   the function keeps its signature. Add `struct OversizedRowAdvisory` and the pure
   `oversized_row_advisories(&[SubShardChunk], u64) -> Vec<OversizedRowAdvisory>`. In `run_bc_index_migration_core`
   (the single `chunk_subsystem_rows_into_sub_shards` call site) append its result to a local
   `oversized_advisories` vector; after `finish_committing_migration` returns `Ok(Completed)` on the FRESH-run path
   call the module-private `emit_oversized_row_advisories(&oversized_advisories)` (the retained `tracing::warn!`
   plus `emit_coordinator_advisory`, the exact line of item 11(f)(2)), then return; every `?`/`Err` return and the
   resume arms drop the vector. Rewrite the doc comment of `chunk_subsystem_rows_into_sub_shards` ("a non-blocking
   `tracing::warn!` is logged" ⇒ "the oversized fact is reported by [`oversized_row_advisories`]; this function
   logs nothing") and of `close_sub_shard_chunk`. Do not add a field to the txn record.
2. *Item 2.* Extract `try_remove_staging_dir(fs, gen_dir) -> Option<(PathBuf, String)>` (performs the removal, logs
   the existing `tracing::warn!`, returns the failure) and `emit_staging_dir_remove_failed(gen_dir, cause)` (the
   one advisory call); use them in `discard_incomplete_staging` and in the `abort_staging` closure, which then
   reads: remove (capturing the failure) ⇒ txn ABORTED write (`?`) ⇒ emit the advisory if a failure was captured
   ⇒ gate OPEN write. Update the closure's doc comment and the `discard_incomplete_staging` doc ("Gen-dir removal
   is best-effort" ⇒ "best-effort and reported").
3. *Item 3.* Add `read_current_pointer_marker<F: Fs>(fs: &F, path: &Path) -> Result<Option<CurrentGenerationPointer>,
   io::Error>` beside `read_completed_marker` (lstat via `symlink_metadata`, then `read`, then
   `serde_json::from_slice::<CurrentGenerationPointer>` with `InvalidData(parse_failure_message)`, then the
   `status == "committing"` and single-component `generation_id` checks). `detect_migration_read_state` calls it
   (replacing the `read_to_string` + `Value` probe, with `StdFs`, mirroring the `completed.json` leg);
   `read_current_generation_pointer_if_present` calls it (replacing `fs.read` + `.ok()`; `Err` ⇒
   `BcIndexMigrationError::Io { path, source }`) and its doc comment ("`Ok(None)` covers BOTH ... not valid JSON")
   is rewritten to the new verdicts. Audit the other `CURRENT.json` readers and record the list.
4. *Item 4.* Replace the `Io` `#[error]` with a Display that renders `sanitize_diagnostic(&path.display().to_string(),
   256)` and a `render_io_source(&source)` (256-capped; special-cased downcast for `CleanupWriteFailure`);
   `CleanupWriteFailure::fmt` renders `cause` and `while_handling` each through `sanitize_diagnostic(_, 256)`;
   the same for `BcIndexAddressingError::{Io, Toml, MalformedBcId}`; replace each content-bearing
   `io::Error::new(InvalidData, <serde error>)` by `parse_failure_message(&e)` (make it reachable from
   `shard_manager.rs`; it is `pub(crate)` in `admission.rs`). `AppendLogMigrationError` (the sibling coordinator
   error) receives the identical `Io` change if it has an `Io`-shaped variant.

**Test deltas (test-writer; red first):**
- *Item 1* (extend `s2509_pass1_l1_findings_test.rs` F-006(e)): (T1) a unit test that
  `chunk_subsystem_rows_into_sub_shards` over a lone oversized row writes NOTHING to stderr (child-process or
  `gag`-style capture) and `oversized_row_advisories` returns exactly one record with the expected fields, and is
  empty for a within-cap input; (T2) black-box: oversized fixture + failure injected at each of census
  (`verify_independent_census`), fingerprint recheck, intent append, pointer swap and a canonical-move halt ⇒
  stderr contains NO `OVERSIZED_ROW_SUBSHARD` and has the failure's own exit-2 line (five cases); (T3) black-box:
  oversized fixture, `Completed` ⇒ exactly one line per oversized sub-shard, shape and fields exactly as item
  11(f)(2), emitted AFTER any `GATE_OPEN_RESET_FAILED` line; (T4) crash after staging + resumed run completes ⇒
  no `OVERSIZED_ROW_SUBSHARD` line (both `ResumeFromStaging` and COMMITTING recovery); (T5) the admission-entry
  grep/unit test of 11(g)(3)(ii) still passes; add a source-gate that the chunker's call tree contains no
  `emit_coordinator_advisory`/`eprintln!`/`tracing::` call (the existing source-gate style).
- *Item 2:* (T6) fresh-run abort closure with `Fs::remove` injected to fail ⇒ ONE `STAGING_DIR_REMOVE_FAILED
  (advisory)` line, printed after the ABORTED record is durable (assert the txn is ABORTED at the moment of
  print via the failing-remove hook order) and the original failure's exit-2 line follows; (T7) the same with the
  ABORTED write failing ⇒ NO advisory, the write's `Io` naming the original token; (T8) the gate-OPEN write
  failing after the advisory ⇒ advisory present, `Io` exit 2.
- *Item 3:* (T9) `detect_migration_read_state` over the six `CURRENT.json` shapes EC-078..EC-083 (parameterised like
  the `completed.json` parity test at `s2509_pass1_l1_findings_test.rs` ~658-800, same `parity_verdict` shape);
  the existing `bc_1_18_010_b2_addressing_test.rs` READERINT tests (`gen-xyz`, `txn-abc`) stay green; (T10)
  coordinator STAGING-resume with a dangling and with a corrupt `CURRENT.json` ⇒ `Io` exit 2, no discard, no
  write, generation directory intact (Invariant 3 regression); (T11) a valid `completed.json` beside a dangling
  `CURRENT.json` ⇒ `Completed` (precedence unchanged).
- *Item 4:* (T12) `Io` Display with a 400-character hostile path containing `\n`/ESC and a 400-character
  source text ⇒ one line, both substrings escaped, each at most 256 characters, the fixed text intact; (T13) an
  `InvalidData` source from a `completed.json` / `CURRENT.json` with a secret-looking string in a wrong-typed
  field ⇒ the secret does not appear in stderr (category/line/column only); (T14) an `abort_cleanup_outcome`
  `Io` with a 300-character `while_handling` and a 300-character cause ⇒ both capped separately and the
  "(while handling" tail present; (T15) the same for `BcIndexAddressingError::Io`/`Toml`/`MalformedBcId`.

## References

- `ADR-054` — companion ADR (v1.24): normative home of the intent-log format, the fixed move plan (`canonical_move_plan`), B-1/B-2/B-3 completion semantics, the shared intent-log module and the S-25.10 downstream block
- `.factory/research/adr-052-intent-log-format-and-move-list-semantics.md` — independent research (2026-10-08) that grounds the v1.24 human-authorized amendment (verdicts A CHANGE-BOTH, B AMEND-SPEC-TO-CODE)
- `adv-local-adr052-pass3.md` — local cascade pass-3 (D-1223): 12 findings NOT RATIFIABLE (1C+4H+5M+2L), producing the v1.6 fix-burst
- `adv-cv-adr052-v12-closure-2026-09-13.md` — 3rd Codex cross-vendor closure review (11 findings, D-1218) that prompted the v1.3 redesign
- `research-adr-052-v13-atomic-publication-2026-09-13.md` — research brief grounding the v1.3 architecture (atomic publication, intent log, advisory flock, F_FULLFSYNC, fexecve/execveat)
- `adv-cv-adr052-v11-closure-2026-09-12.md` — 2nd Codex closure review, 8 findings (D-1216)
- `adv-cv-adr052-cluster5-F1-2026-09-12.md` — 1st Codex RATIFY-WITH-CHANGES verdict (7 findings, D-1214)
- Decision log D-1221 (local pass-1, 9 findings, producing v1.4), D-1222 (local pass-2, 9 findings, producing v1.5), D-1223 (local pass-3, 12 findings, producing v1.6), D-1224 (local pass-4, 2H+6M, producing v1.7), D-1225 (local pass-5, 2H+1M RATIFY-WITH-CHANGES, producing v1.8), D-1226 (local pass-6, 1H+1M+2L RATIFY-WITH-CHANGES, producing v1.9), D-1227 (local pass-7, 1H+2M+2L RATIFY-WITH-CHANGES, producing v1.10), D-1228 (local pass-8, 1H+2M+1L NOT-RATIFIABLE, producing v1.11), D-1229 (local pass-9, 1H+2M+1L NOT-RATIFIABLE, producing v1.12), D-1230 (local pass-10, 1H+3M+2L accept-at-floor, producing v1.13), D-1231 (Kani pass-1, 1 HIGH — DEF-1, human-directed spec/design convergence, producing this v1.14 fix-burst)
- `research-adr-052-assumption-validation-2026-09-12.md` — prior research Q1-Q5 validation
- `BC-1.18.011` — migration BC; §Downstream to Product-Owner specifies amendments
- `BC-1.18.010` — §Reader Integration amendment + Invariant 2 amendment
- `S-25.06-append-log-backfill-split-executor.md` Rule 7 — corrected per §Decision 9
- `ADR-051` §Decision 1 (WASM fuel-budget constraint), §Decision 7 (B2 end-state) **(v1.13 LOW-2 note: ADR-051 frontmatter carries no `version:` field; "ADR-051 v1.14" cited in this ADR's changelog refers to ADR-051's body-changelog versioning convention, not a frontmatter field. Follow-up action: add `version:` frontmatter to ADR-051 for cross-document verifiability — do NOT edit ADR-051 in this burst.)**
- `CLAUDE.md` — TD-FACTORY-HOOK-BYPASS-001 P0 governing rule; human-only edit target
- POSIX rename(2) [man7.org]: namespace atomicity guarantee; same-filesystem constraint; NOT multi-file transaction
- Pillai et al. OSDI'14 "All File Systems Are Not Created Equal": crash vulnerabilities across 6 Linux filesystems; dir-fsync requirement for rename durability
- Apple fsync(2) [Apple Developer]: "the drive itself may not physically write the data to the platters for quite some time" — F_FULLFSYNC required for APFS power-loss durability
- Apple fcntl(2) [Apple Developer]: `F_FULLFSYNC` asks drive to flush all buffered data; `F_BARRIERFSYNC` is ordering-only
- man7 flock(2), fcntl(2): advisory lock semantics; OFD lock on Linux; `F_SETLK` any-close hazard
- man7 execveat(2), fexecve(3): fd-binding exec on Linux; `AT_EMPTY_PATH` for executing by fd
- LMDB / SQLite super-journal: prior art for single-pointer atomic commit over multi-file publication
- Kleppmann 2016 / Chubby OSDI'06: lease + fencing tokens for distributed locking
- CWE-367 (TOCTOU), CWE-88 (argument injection), CWE-78 (OS command injection), CWE-22 (path traversal)
- NIST SP 800-53r5 AU-9 (audit trail tamper-evidence)

## Files to Change

| File | Change | Owner |
|---|---|---|
| `CLAUDE.md` | Apply §Decision 8 CLAUDE.md amendment text | **Human only** |
| `.claude/settings.json` | No change — Option B requires no settings.json mutation | — |
| `crates/factory-dispatcher/src/main.rs`, `src/executor.rs`, `src/shard_manager.rs` (v1.18, S-25.06 scope) | Shared admission core evaluated once per PreToolUse before `shard_cap_precheck` (D1); reserve-then-verify order + release-on-block (D5/D2); (v1.19) `executor.rs` admit path: REMOVE the early return that skips admission/reservation when `.factory/migration-state/` is absent (and its PostToolUse-release counterpart's directory-exists precondition becomes a best-effort no-op only) — replaced by an idempotent `create_dir_all(migration-state/reservations)` immediately before reservation W1; wire §5a step-3.5 Branches A/B/C into the production admission path for BOTH migrations (merged `reconcile_stale_admission_gate` is uncalled dead code) + verified `completed*.json`+COMMITTING finalize replacing the merged unverified B2 short-circuit (D3); `migration_id` txn field + cross-migration refusal + per-migration terminal-record/pointer names for `backfill-append-logs` + `file_count`→`canonical_paths_count` (D4); `DEFAULT_MAX_RESERVATION_TTL` 120 s→3,600 s with `created_at`-based staleness; mechanism-A coordinator drain (steps 1–7) writing `gate-state.json`. B2 test fixtures that model STAGING without holding `exclusive.lock` must be updated by test-writer. | implementer / test-writer (S-25.06) |
| `crates/factory-dispatcher/src/main.rs`, `src/executor.rs`, `src/invoke.rs`, `src/shard_manager.rs`, `src/shard_manager/admission.rs` (v1.20, S-25.08 scope) | (F-004) move the admission call and the release call in `main.rs::run` to immediately after payload parse + `resolve_project_cwd()`, BEFORE the `CLAUDE_PLUGIN_ROOT` tiering / `resolve_registry_path()` / `Registry::load`; a Block/Error verdict exits via the existing `shard_gate_verdict_outcomes` mapping without loading the registry; (F-001) `executor.rs` release gated by new pure `is_tool_completion_event` (`"PostToolUse"` \| `"PostToolUseFailure"`), keyed on `tool_use_id` only (drop the `Edit/Write/MultiEdit` tool filter), NO new `EventType` variant; (F-002/F-003) `ProtectedPathFamily::classify` replaced by component-wise classification against `factory_root` using new shared `resolve_target_path` → `(T_real, T_lex)`; admission/release derive `migration_state_dir` from `factory_root` (must exist; never create `.factory`); out-of-root paths out of scope; (F-006) rename `TerminalReconcileInputs.txn_is_own_migration` → `txn_migration_known` and set it from `migration_id ∈ {migrate-bc-index, backfill-append-logs}`; (F-008) `reservation_is_stale(Option<u64>, Option<u64>, u64, u64)`, `RESERVATION_CLOCK_SKEW_TOLERANCE_SECS = 300`, pre-epoch/unavailable mtime ⇒ NOT stale (replaces `epoch_secs ⇒ 0`); (F-009) add `BcIndexMigrationError::ReservationTtlBelowFloor` and `InvalidToolUseId`, map admission errors to `E-MAINTENANCE-002 (<cause>)`, `validate_production_reservation_ttl` stops returning `BinaryIntegrityFailure`; key-present-but-invalid `tool_use_id` fails closed. Tests: the §5a "v1.20 test mandate". | implementer / test-writer (S-25.08) |
| `crates/factory-dispatcher/src/main.rs`, `src/executor.rs`, `src/internal_log.rs`, `src/shard_manager.rs`, `src/shard_manager/admission.rs` (v1.21 scope; owned by S-25.09 since v1.22, formerly S-25.08) | (D-2) pure `resolve_session_project_root`; `main.rs::resolve_project_cwd` delegates; both subcommand routes pass the resolved project root; `run_bc_index_migration` derives every `.factory/…` path from `resolve_factory_root` (six `_cwd.join(".factory/…")` literals removed); `FactoryRootNotFound`; (D-1) `AdmissionDiagnostic` data returned by the core/`MigrationAdmission`, `main.rs` writes `migration.admission_*` InternalLog events before the early return, three constants in `internal_log.rs`, `migrate-bc-index` failure line on stderr; (F-006) `run_bc_index_migration_with_ttl` crate-private seam; (F-012) `AdmissionStateIntegrity`/`AdmissionStateIntegrityKind`/`AdmissionFailureCause`, exhaustive cause match; (F-013) `NativeGate` enum + `migration-admission` label; (F-014) rename `bc_index_migration_admission{,_precheck}` / `bc_index_migration_reservation_release` → `migration_writer_admission{,_precheck}` / `migration_reservation_release` | implementer (S-25.08) |
| `crates/factory-dispatcher/src/shard_manager/intent_log.rs` (NEW), `src/shard_manager.rs`, `src/shard_manager/migration_fs.rs`, `src/shard_manager/obl1_kani_proofs.rs`, `tests/s2510_*.rs` (v1.24, S-25.10 scope; S-25.06 consumes) | Shared intent-log module in the ADR-054 hardened format; `Fs::append_durable` / `truncate_durable`; delete the B2 and S-25.06 intent-log copies; rename `pending_canonical_moves` → `canonical_move_plan` / `PendingCanonicalMove` → `PlannedCanonicalMove`; B-1 (`TreatDone` appends DONE), B-2 (txn-bound DONE compared with the INTENT), B-3 (non-empty plan equal to the INTENT set); `CANONICAL_MOVE_HALTED` / `INTENT_LOG_CORRUPT` / `INTENT_LOG_VALUE_REJECTED`; step 7d removed. Full per-story breakdown and BC/taxonomy/VP wording owed: ADR-054 §Downstream (S-25.10 block) | implementer / test-writer / product-owner (S-25.10) |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md` | Apply §Downstream Amendments 1–9 (v1.3 versions) | product-owner |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md` | Apply §Downstream Invariant 2 amendment + §Reader Integration v1.3 replacement | product-owner |
| `.factory/specs/prd-supplements/error-taxonomy.md` | Apply §Downstream error-taxonomy v1.3 correction | product-owner or technical-writer |
| `.factory/migration-state/exclusive.lock` | Create empty pre-seeded flock inode (never deleted) | devops-engineer (cluster-5 activation preparation) |
| `plugins/vsdd-factory/hooks-registry.toml` | Amend 4 guards per §Decision 5b (txn record state awareness replacing PID-liveness check) | devops-engineer |
| `plugins/vsdd-factory/hooks/destructive-command-guard.sh` (or WASM) | 4-branch classifier per §5c; txn record state check | devops-engineer |
| `plugins/vsdd-factory/hooks/validate-factory-path-staging.sh` (or WASM) | 4-branch classifier; conservative Bash admission; executable digest verification + fd-binding per §Decision 11; **v1.12 F6: Branch 2 gate reconciliation — new step 0: attempt `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` before gate LOCK_EX; on EWOULDBLOCK log warning + exit 0 ALREADY_MIGRATED without gate reconciliation; on acquired proceed with steps 1–4 + release both locks**; **v1.13 HIGH-1: step-3.5 Branch B — null-generation STAGING predicate extension; EWOULDBLOCK path; set txn→ABORTED(null_generation; RETAINED) path; Branch A vs Branch B selection under LOCK_EX re-read** | devops-engineer |
| `plugins/vsdd-factory/hooks/validate-factory-path-staged.sh` (or WASM) | Recognize governed migration subcommands; pass through | devops-engineer |
| `crates/factory-dispatcher/src/executor.rs` | OPEN/DRAINING gate with writer reservations (PreToolUse-acquire/PostToolUse-release); durable reservation dir `.factory/migration-state/reservations/` (H2 v1.4); atomic admission under gate_state flock; every abort path flips gate → OPEN (H1 v1.4); **v1.10 HIGH-1: step-3.5 stale-gate reconciliation generalised — triggers on gate ∈ {LOCKED, DRAINING} AND no active txn (absent/COMPLETED/ABORTED), regardless of completed.json presence; covers (i) abort-crash-before-gate-reset (txn=ABORTED, no completed.json) and (ii) drain-timeout-crash (gate=DRAINING, no txn record) scenarios**; **v1.11 HIGH-1: step-3.5 flock-gated reconciliation — FIRST attempts `flock(exclusive.lock, LOCK_EX\|LOCK_NB)`; reconcile gate→OPEN ONLY if flock ACQUIRABLE (no live coordinator); on EWOULDBLOCK block with E-MAINTENANCE-001 (no gate flip); sub-steps a→h; parity with crash-recovery paragraph; drain reordered (v1.11): initial txn (state=STAGING, source hashes null) written at "step 2.5" BEFORE DRAINING flip (defense-in-depth — `gate=DRAINING + no-txn` eliminated from normal live drains); step 7 is now UPDATE txn with source hashes; step 4 drain-timeout abort now also sets txn→ABORTED**; **v1.12 F2: null-generation crash recovery path — admission reads `generation_id` from txn record on recovery; if null (pre-generation crash sub-state: drain "step 2.5" written but §7c step 1 not yet run): rewrite txn in place to `state: ABORTED` + top-level string `abort_reason: "null_generation"` (optional on read, informational only, never consulted for admission or reconciliation decisions; RETAINED per GC policy), flip gate→OPEN, exit EXPIRY_ABORT; MUST NOT call census or compare PC1 against null `source_body_row_sha256`**; **v1.12 F4: txn-selection disambiguation — admission scans all `txn-*.json` and selects by activation_id match to current manifest; fallback: highest created_at; ABORTED record is inert (treated as absent for selection); terminal-txn GC: on migration completion archive ABORTED + COMPLETED records to `.factory/migration-audit/txn-archive/`**; **v1.13 HIGH-1: step-3.5 predicate extended to cover txn=STAGING AND generation_id=null (Branch B) — PreToolUse self-heal for null-generation crash sub-state; on flock acquired (dead coordinator): set txn→ABORTED(null_generation; RETAINED), flip gate→OPEN, admit; on EWOULDBLOCK: block E-MAINTENANCE-001**; **v1.14 DEF-1 (SUPERSEDES the v1.11 ordering cited above): drain reordering REVERSED — the initial txn record is now written at step 3a, strictly AFTER the step-3 DRAINING flip, not before it; this closes an uncovered crash window (`gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead coordinator`) that the v1.11 pre-DRAINING ordering introduced, because that state matched neither the step-3.5 self-heal predicate (requires `gate ∈ {LOCKED, DRAINING}`) nor the ordinary OPEN-admission path (an active txn exists); implementer MUST write the txn record in the step-3a position (after gate_state=DRAINING is durably written and its LOCK_EX released), never before; the flock-gated step-3.5 sub-step a check remains the sole load-bearing writer-exclusion mechanism during the drain, unchanged by this reordering**; crash-recovery gate reconciliation (§Decision 5a Crash-recovery gate reconciliation paragraph — also covers gate=DRAINING per v1.4 H1); stale reservation cleanup with **v1.10 MED-2: operator runbook for abandoned-reservation stall — confirmed-stale reservation files (no live harness session) MAY be manually deleted pre-activation per §Decision 5a operator remediation guidance**; txn record state check blocking mutations | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/src/shard_manager.rs` | Full v1.7 migration implementation: advisory flock; txn record (`source_body_row_sha256` field for PC1); intent log (framed+checksummed); step 3b pre-pivot census gate — staging-integrity (sha256(staged_file) vs intent-log expected_post_hash) + PC1 (structured per-BC-row equivalence: extract BC-X.YY.NNN rows from staged shards, sort by BC-ID, sha256 → compare vs `source_body_row_sha256`; `source_sha256` whole-file hash is the step-5 fingerprint recheck only, NOT PC1) + PC2 (per-ID exactly-one-shard set check; EC-001 dup+drop detection); CURRENT.json pointer swap + generation-first/open-with-ENOENT-fallback reader; completed.json; **v1.12 F2 / v1.13 MED-3: recovery path checks `generation_id` field in txn record — if null (pre-generation crash sub-state): rewrite txn in place to `state: ABORTED` + top-level string `abort_reason: "null_generation"` (optional on read, informational only, never consulted for admission or reconciliation decisions; RETAINED per GC policy — do NOT delete), flip gate→OPEN, exit EXPIRY_ABORT; MUST NOT census or compare PC1 against null `source_body_row_sha256`**; **v1.12 F4: txn-selection reads all `txn-*.json`, selects by activation_id match to current manifest (fallback: highest created_at); terminal-txn GC at migration completion archives ABORTED/COMPLETED records to `.factory/migration-audit/txn-archive/`**; platform-branched durability (F_FULLFSYNC on macOS); fd-binding exec on Linux with /proc dependency note; programmatic mtime guard on macOS (M-2: mtime re-stat immediately before exec); TTL-based stale-reservation GC at start of every drain; all abort paths flip gate → OPEN; fencing_generation AUDIT-ONLY | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/tests/` | v1.7 test suite: advisory flock; txn record state machine (`source_body_row_sha256` field); intent log write+recovery+fault-injection; step 3b census gate (staging-integrity sha256 vs intent-log expected_post_hash + PC1 structured per-BC-row equivalence vs `source_body_row_sha256` + PC2 ID-set exactly-one-shard + EC-001 dup+drop + shard-boundary internal capacity check [NOT `E-SHD-005` — that is the steady-state HookResult, not a migration process exit code]); CURRENT.json pointer swap + generation-first/open-with-ENOENT-fallback reader; completed.json permanence; all abort paths → gate OPEN; Branch 2 gate reconciliation (**v1.12 F6: step 0 flock acquire + EWOULDBLOCK warning-and-exit path + fault injection for crash between step 8 and gate→OPEN**); reservation dir cross-process quiescence; TTL-based stale-reservation GC on first-run drain; test that an IN-FLIGHT reservation is NEVER reclaimed by drain step-1 GC (v1.9 H1); **v1.12 F2 / v1.13 MED-3: null-generation crash test (migration-binary path) — crash between drain step 3a (generation_id=null) and §7c step 1 → binary re-invocation: rewrites txn in place to `state: ABORTED` + top-level string `abort_reason: "null_generation"` (optional on read, informational only, never consulted for admission or reconciliation decisions; RETAINED per GC policy — do NOT delete), flips gate→OPEN, exits EXPIRY_ABORT; VERIFY no EC-003 census attempted; VERIFY no PC1 comparison against null source_body_row_sha256; VERIFY txn record still present in ABORTED state after operation**; **v1.12 F4: txn-selection disambiguation test — multiple txn-*.json coexist; recovery selects by activation_id match; ABORTED record inert**; **v1.13 HIGH-1: PreToolUse step-3.5 null-generation self-heal test — gate∈{LOCKED,DRAINING}, txn=STAGING(generation_id=null), no gen dir, flock released; VERIFY step 3.5 matches Branch B → flock acquired → sets txn→ABORTED(null_generation; RETAINED) → gate→OPEN → admits; VERIFY live coordinator (flock held) scenario receives EWOULDBLOCK → E-MAINTENANCE-001 → NO gate flip → NO txn→ABORTED**; **v1.14 DEF-1: drain-reorder regression suite — (a) crash between step 3 (DRAINING flip) and step 3a (txn write) → gate=DRAINING, no txn → Branch A self-heals (unchanged branch, new crash point); (b) construct-and-assert-UNSAT test: no crash-injection point between step 2 (flock acquired) and step-3 completion can produce a durable txn record with state STAGING/COMMITTING while gate_state=OPEN — i.e. `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir` (the DEF-1 defect state) MUST be unconstructible under the v1.14 step ordering; (c) regression test reproducing the exact pre-v1.14 defect scenario (crash immediately after the old pre-DRAINING txn write, before the DRAINING flip) asserts the harness CANNOT reach it because step 3a no longer precedes step 3** | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/tests/darwin_arm64_durability_test.rs` | Empirical darwin-arm64 directory-fsync durability characterization test — **RATIFICATION PREREQUISITE (v1.7 F8)**: this test MUST be completed and reviewed before POLICY 22 ratification of macOS safety. Test validates whether `fsync(dir_fd)` on APFS provides power-loss durability for rename directory entries beyond `F_FULLFSYNC` alone. Results feed §Decision 11 sign-off item 2. | implementer (cluster-5 TDD) — darwin-arm64 CI required; result reviewed at POLICY 22 ratification gate |
| `.factory/activation/factory-dispatcher.sha256` | SHA-256 of built binary | devops-engineer (cluster-5 activation) |
| `.factory/activation/` | Created at F4 by state-manager | state-manager |
| `.factory/migration-state/` | Created by migration binary at runtime; `exclusive.lock` pre-seeded by devops-engineer; `reservations/` subdir created by executor.rs for cross-process writer reservations (H2 v1.4) | migration binary (runtime) |
| `.factory/migration-audit/` | Created by state-manager post-migration | state-manager |

## Changelog

| Version | Date | Author | Change |
|---|---|---|---|
| 1.26 | 2026-10-08 | architect | **Four rulings from the S-25.09 local adversary pass 2; no state-machine, taxonomy-code, exit-code, `<cause>` or VP-count change; v1.25 is committed (`.factory` 81b44098) and immutable, so these are a NEW version.** (1) F-S2509-L2-002/003 — item 11(f)(2) "Collection and emission rule" and 11(g)(3)(i): `OVERSIZED_ROW_SUBSHARD` is no longer emitted from `close_sub_shard_chunk` at STAGING time (its clause "and the migration completed" was false when a later census / fingerprint / intent-append / pointer-swap / canonical-move step failed); the `pub` chunker is pure (no stderr, no `tracing`), a pure `oversized_row_advisories(chunks, cap)` derives the advisories as data, `run_bc_index_migration_core` collects them and prints them only when the fresh run returns `Completed` (discarded on any `Err`); resumed STAGING / COMMITTING runs neither re-derive nor persist (no txn-schema change; documented rationale). (2) F-S2509-L2-006 — item 11(d)/11(f)(2): the fresh-run `abort_staging` closure's `let _ = fs.remove(gen_dir)` is replaced by the helper `discard_incomplete_staging` uses and reports `STAGING_DIR_REMOVE_FAILED` after its ABORTED write succeeded, before the gate-OPEN write. (3) F-S2509-L2-009 — item 11(c) point 6: `CURRENT.json` gets the lstat-first four-verdict classification with record-shape validation in BOTH consumers (reader `detect_migration_read_state` → `BcIndexAddressingError::Io`; coordinator resume probe → `Io` exit 2, protecting Invariant 3) through a shared `read_current_pointer_marker`; `status != "committing"` and an empty / multi-component `generation_id` are `InvalidData`, not `NotStarted` / `Committing { "" }`. (4) F-S2509-L2-001 — item 11(g)(2): confirmed and made explicit that `Io` `path` AND `source` are two separately escaped, 256-capped data-derived substrings; serde messages are never carried (category/line/column via `parse_failure_message` at the source); `CleanupWriteFailure`'s `cause` and `while_handling` are two independent 256 substrings and `Io` special-cases it so the "while handling" tail is not truncated; `BcIndexAddressingError` siblings follow. §Downstream v1.26 carries the BC (BC-1.18.013 v1.15, BC-1.18.010 v1.12, BC-1.18.011 v1.23), code and test deltas (S-25.09). VP-INDEX / verification-architecture / verification-coverage-matrix UNCHANGED. |
| 1.25 | 2026-10-08 | architect | **Two interpretive rulings from the S-25.09 local adversary pass 1; no state-machine, taxonomy-code, exit-code, `<cause>` or VP-count change; v1.24 is committed (`.factory` 61709b2d) and immutable, so these are recorded as a NEW version.** (1) Item 11(c) clarification (closes F-S2509-L1-004): the under-lock `completed.json` read has FOUR verdicts — a dangling symlink, a non-`NotFound` stat or read error, and non-record content are `Io` exit 2 (never absent, never exit 0); presence-only semantics are kept for branch selection; what `recover()` receives; reader parity. (2) NEW item 11(f) (closes F-S2509-L1-006): the "Coordinators" clause of BC-1.18.013 Postcondition 10 is an OPEN set (bounds: `warn!`/`error!` conditions that change what the operator must know; not `debug!`/`info!`, not conditions already carried by an exit-2 line); failures keep the existing form; non-fatal advisories use `<subcommand>: <TOKEN> (advisory): <path>: <os error>; <clause>` with a CLOSED coordinator-advisory token domain (`GATE_OPEN_RESET_FAILED`, `TERMINAL_TXN_ARCHIVE_FAILED`, `STAGING_DIR_REMOVE_FAILED`, and `OVERSIZED_ROW_SUBSHARD` only if reachable from a coordinator process; these are NOT taxonomy codes and have no exit status of their own); the v1.23 exit-0 silence ruling is amended so advisory lines are permitted on an exit-0 run (stdout stays empty); canonical-move halt reasons are NOT an S-25.09 deliverable — S-25.09 emits no interim halt line — and belong to S-25.11 (ADR-054 AC-015 = S-25.11 story AC-008), where ADR-054 v1.1 widens the `CANONICAL_MOVE_HALTED` reason domain to all six halt sites. The same-version extension text previously placed in the v1.24 §Status preamble is moved here; v1.24 was NOT extended. **Consistency extension (same uncommitted v1.25, 2026-10-08):** item 11(g) scope widened to ALL operator stderr lines — item 9 `FOREIGN_MIGRATION_REFUSED` `<id>` changes from "truncated to 64" to 256 per data-derived substring (reusing the one parametrized sanitizer), and the ADR-054 §3.1 stderr placeholders follow the same rule (ADR-054 v1.1 extension 2); 64 is the InternalLog-event cap only. No taxonomy-code, exit-code, kind or VP-count change. Code: S-25.09 (item 9 renderer); S-25.10/S-25.11 not yet implemented. PO mirrors: BC-1.18.011, BC-1.18.013, error-taxonomy. |
| 1.24 | 2026-10-08 | architect | **Human-authorized amendment (CLAUDE.md rule 12; 2026-10-08) of §Decision 7a/7b/7c/7d after independent research (`.factory/research/adr-052-intent-log-format-and-move-list-semantics.md`); no §Decision 1/2/3/4 mechanism change, no OPEN/DRAINING/LOCKED state-machine change, no VP added or retired.** (1) NEW companion **ADR-054**; §Decision 7b MOVED there (this ADR keeps the exact `#### 7b` heading as a pointer table so every citation resolves) and rewritten as the HARDENED format: code tokens + `V1` kept; ABNF grammar; fixed key order; strict value grammar enforced at plan-build and write time; SHA-256 over the exact first nine record lines (golden vector, `sed`+`shasum` operator recipe); byte-level line-anchored reader (torn tail absent; corruption followed by a valid record fails closed); torn-tail truncation under the flock before append; `F_FULLFSYNC` + directory sync on creation (§7d gains the append-barrier paragraph); ONE shared module for both migrations. (2) FIXED-PLAN model: `pending_canonical_moves` RENAMED `canonical_move_plan` (§7a field, §3a, §7c steps 3/6/7/8, §Error Code Semantics Tier 1 / Branch C hash source / rulings (ii) and (iv) / item 10 strict-presence [still TWELVE keys] and 10(a) nested strictness / item 11(e) steps 2-3 / MIGRATION_STATE_INTEGRITY_FAILURE row); §7c step 7 sub-step d DELETED (the txn record is not rewritten per move); DONE records are the completion record. (3) B-1 (`TreatDone` appends DONE), B-2 (DONE bound to `txn_id`/`fencing_generation`, copied from and compared with the INTENT's `expected_post_hash`, truthful `expected_pre_state`), B-3 (a COMMITTING txn needs a non-empty plan equal to the INTENT target set, else `txn_record_malformed`) specified normatively in ADR-054 Decision 3; three NEW exit-2 codes `INTENT_LOG_CORRUPT`, `INTENT_LOG_VALUE_REJECTED`, `CANONICAL_MOVE_HALTED` (§Error Code Semantics table). (4) Both v1.23 open items (byte-encoding "human gate", `pending_canonical_moves` "open item for the human") marked RESOLVED. (5) §Downstream gets only a pointer block; the S-25.10 block (AC-001..AC-019, red tests T1-T17, code changes per story, BC/taxonomy/VP wording owed) is in ADR-054 because ADR-052 exhausts the hook fuel budget. NEW story S-25.10 (stacked on S-25.09; blocks S-25.06 and any `migrate-bc-index` release); S-25.09 keeps only the txn_id / intent_log_path / DONE-binding plumbing. **Recommendation to the orchestrator (not a deferral of this amendment):** ADR-052 remains ~4.9k lines and still exhausts hook fuel; a further split (§Decision 5a, §Error Code Semantics, the v1.18-v1.23 Downstream history) needs its own architect decision with a stable-anchor map. |
| 1.23 | 2026-10-07 | architect | S-25.08 local adversary pass 3 — two interpretive rulings; no design/state-machine/taxonomy-code/VP-count change. **F-S2508-L3-009** §5a "Factory-root lookup mapping": `resolve_factory_root` classifies the `.factory` stat as Found / Absent (closed set: ENOENT, ENOTDIR, not-a-directory incl. regular file and dangling symlink) / Unstatable (any other error); return type `Result<Option<FactoryRoot>, BcIndexMigrationError>`; admission ⇒ `E-MAINTENANCE-002 (io)` fail closed, release ⇒ no verdict + `reservation_release_failed` advisory, coordinators ⇒ existing `Io` exit 2 (not `FACTORY_ROOT_NOT_FOUND`); regular-file `.factory` = Absent (EC-035 confirmed). Rationale: fail-closed, total cause rule, D-2 single anchoring rule. **F-S2508-L3-001** §Error Code Semantics "Txn-record interpretation — tiers": Tier 0 shape only at read (no full typed deserialize), Tier 1 fields read lazily by the consuming branch (Branch B `generation_id`: absent is NOT null ⇒ `state_integrity`; Branch C fields ⇒ `state_integrity` at the check's position); foreign / live-block / Branch A read no field; valid-shape COMPLETED/ABORTED records admitted; planner and VP-147 unchanged. §Downstream v1.23 deltas (BC-1.18.013 v1.12, BC-1.18.011 v1.20, error-taxonomy, stories). **Binary-leg extension (same uncommitted v1.23, S-25.06 AC-023 / T-8n(d) question):** §Error Code Semantics "Migration binaries — recovery and finalize legs" — a malformed/absent Tier 1 txn field consumed by migrate-bc-index recovery or the backfill-append-logs verifier/finalize is `AdmissionStateIntegrity { TxnRecordMalformed }` = `MIGRATION_STATE_INTEGRITY_FAILURE`, exit 2, nothing mutated (NOT `BinaryIntegrityFailure`, NOT `EXPIRY_ABORT`, NOT `COMPLETION_RECORD_MISMATCH_ABORT`); Tier 0 loader + strict-presence typed decode at arm entry; `generation_id` tri-state shared with Branch B (absent ≠ null) and COMMITTING requires a string; verifier order shared with Branch C (mismatch wins over malformation). **Correction:** Branch C's hash source is the intent-log DONE record via txn `intent_log_path`, NOT a nonexistent `pending_canonical_moves[].expected_post_hash` txn field (Decision 7a/7b). Code disagreement recorded: S-25.09 `read_all_txn_records` / absent-`generation_id` ⇒ `DiscardPreGeneration` mutation. No taxonomy code, kind, VP-count or state-machine change. **Second binary-leg extension (same uncommitted v1.23; S-25.09 red tests `37a93210`):** items 8-11 — (1) `EXPIRY_ABORT` keeps one code/exit 1 with two normative arm-specific stderr lines (`ExpiryAbort { arm }`), printed only after txn → ABORTED then gate → OPEN succeed (a failed write is `Io` exit 2); (2) live foreign record on the binary = NEW code `FOREIGN_MIGRATION_REFUSED` exit 2 with an exact line, the "`LockContention`-class" label retired, and the sibling flock-contention message gets NEW `MIGRATION_LOCK_CONTENTION` exit 1 instead of the digest-coded `BinaryIntegrityFailure`; (3) strict-presence = the full 11-key set with types, unknown top-level key ⇒ `txn_record_malformed`; (4) `migrate-bc-index` verify-then-finalize is S-25.06 AC-031, with a fail-closed interim short-circuit owned by S-25.09 (Tier 0 read, foreign refusal, own live record ⇒ `COMPLETION_RECORD_MISMATCH_ABORT`, flock-gated gate reconciliation). This SUPERSEDES the row's earlier "no taxonomy-code change": two taxonomy codes are added (`FOREIGN_MIGRATION_REFUSED`, `MIGRATION_LOCK_CONTENTION`); still no VP-count or state-machine change. **Third binary-leg extension (same uncommitted v1.23; independent validation `.factory/research/adr-052-v123-architect-calls-validation.md`, human-requested):** (1) exit 1 redefined as the class "no harm done, safe to re-run; the stderr code says what to do next" (item 7(e) + closing Note; covers `EXPIRY_ABORT` and `MIGRATION_LOCK_CONTENTION`); `process_exit_code` an exhaustive match, no wildcard; exit 75 evaluated, not adopted. (2) Item 10: unknown-key rejection extended to `pending_canonical_moves[]` elements; txn record gains a required `schema_version: u32` = 1 now (no activation has ever run), with a version gate evaluated first (integer ≥ 2 ⇒ newer-schema, other non-1 ⇒ malformed); NEW `AdmissionStateIntegrityKind::TxnRecordNewerSchema` / `txn_record_newer_schema` (same code `MIGRATION_STATE_INTEGRITY_FAILURE`, exit 2, "recover with the newer build") — a wire-domain change to the BC-3.08.001 Event 12 `kind` domain (five → six tokens; BC-3.08.001 v1.36 → v1.37); `abort_null_generation_txn` keeps preserving unknown fields (terminal ABORTED rewrite acting on no filesystem object, justified) but is NOT exempt from the version gate, on both surfaces. (3) Item 11: (c) the exit-0 `ALREADY_MIGRATED`-under-contention behavior is REPLACED (validation: REJECTED) by `MIGRATION_LOCK_CONTENTION` exit 1 on every path, with `completed.json` re-read under the lock and passed to `recover()` (closes a TOCTOU fresh-run race); earlier "EWOULDBLOCK ⇒ exit 0" rows amended in place (§4e rows, §5c Branch 2 step 0 and fault mandate (b)); (d) NEW normative: COMPLETED txn-write errors propagate (the build must not create the blocked state and report success), with a sibling sweep of the abort-path `let _ =` sites; (e) NEW: manual verify-then-finalize operator procedure as the documented remedy for the interim block. The release-gating question for `migrate-bc-index` vs S-25.06 AC-031 is left to the human. |
| 1.22 | 2026-10-07 | architect | Story-mapping and consistency amendment after the human-approved split of S-25.08 into S-25.08 + NEW S-25.09 (D-1252(f); AC-027 stays in S-25.08); NO design, state-machine, BC, taxonomy or VP-count change. (1) §Downstream "v1.22 split mapping": the v1.21 rulings (D-2, D-1, F-006, F-012, F-013, F-014; items 23–33) are now owned by S-25.09 AC-001..AC-006 (formerly S-25.08 AC-021..AC-026); v1.18–v1.20 scope and AC-027 stay with S-25.08; in-line ownership statements in §5a (anchoring "Where it lands", diagnostics (6), label AC), items 31/32 and the Files-to-Change v1.21 row re-pointed. (2) §5a "Closed `check` domain (v1.22)": states the closed NINE-token `check` domain (staging_with_terminal_record, terminal_record_unparseable, terminal_record_schema_mismatch, txn_id_mismatch, generation_id_mismatch, canonical_paths_count_mismatch, canonical_hash_mismatch, terminal_record_unverified, finalize_unwired) consistently with BC-1.18.013 v1.10-rev2 Postcondition 10(a) (v1.21 text said only "Branch C failing check"); no disagreement with the BC (BC not edited). (3) §Error Code Semantics "Read-failure mapping (v1.22)": RULING — a canonical-file or terminal-record read CALL failing with non-ENOENT during admission is `E-MAINTENANCE-002 (io)` (one `_failed`, no `_blocked`, no tenth `check` token), derived from the total `<cause>` rule, EC-032 and the item-33 closed-domain rulings; code at `39e89c59` already conforms. Refs: S-25.08 v1.5, S-25.09 v1.0, VP-INDEX v3.31, ARCH-INDEX v4.52. |
| 1.21 | 2026-10-07 | architect | S-25.08 local adversary pass 2 (code `.worktrees/S-25.08` @ 25e5464e reviewed read-only) — normative rulings on six gaps. **D-2** one anchoring rule for admission AND both coordinator binaries (`resolve_session_project_root` → `resolve_factory_root`; coordinator derives every `.factory/…` path from the resolved real root, never creates `.factory`, new fail-closed `FactoryRootNotFound` exit 2; lands in S-25.08 as B2 conformance, `backfill-append-logs` built on it in S-25.06). **D-1** the dispatcher installs no `tracing` subscriber (all `tracing::*!` is discarded in production; no spec/story owns one), so admission diagnostics MUST use the spec-owned dispatcher-internal `InternalLog` channel: three `migration.admission_{blocked,failed,advisory}` events returned as data by the pure core and written by `main.rs`; coordinator diagnostics to stderr; installing a global subscriber explicitly NOT the fix; the seven `tracing::warn!` mandates in this ADR re-pointed. **F-006** const assertion alone does not discharge EC-031; crate-private `run_bc_index_migration_with_ttl` seam (first-statement validation) keeps the byte-identical-snapshot vector assertable; no public/env/argv TTL input. **F-012** dedicated `AdmissionStateIntegrity { kind }` variant (five kinds), exhaustive `admission_failure_cause` (closed `AdmissionFailureCause`, no wildcard), taxonomy rows `MIGRATION_STATE_INTEGRITY_FAILURE` / `FACTORY_ROOT_NOT_FOUND`. **F-013** admission verdict label `migration-admission` via a closed `NativeGate` enum; no spec names `shard-cap-gate`, TD #71 stderr format unchanged. **F-014** shared entry points renamed migration-neutral (`migration_writer_admission` / `_precheck` / `migration_reservation_release`), per-migration delegates NOT created (supersedes the v1.20 "thin delegates" sentence). VP-143 v1.5 / VP-146 v1.4 amended (VP-INDEX v3.30; no count change). **Same-version completion (product-owner gaps):** undelivered-finalize case ruled — `_blocked{branch=completion_record_mismatch, reconciliation=completion_record_mismatch, check=finalize_unwired}` (no seventh token; item 33(g)); Branch C verification failure writes ONE `_blocked` (`branch=completion_record_mismatch`) and NO `_advisory` (stale §5a prose fixed); `reconciliation` domain ratified/corrected to the six effectful-outcome tokens (no `none`) with a `branch` derivation table; `_advisory` reason domain closed to five dispatcher-leg tokens (+ `branch_c_finalize_unwired` fires when the S-25.08 fail-closed seam is reached, retired by S-25.06; `branch_c_finalized` reserved for S-25.06), the five timestamp tokens moved to coordinator stderr; optional advisory fields closed; `FACTORY_ROOT_NOT_FOUND` stderr line made normative (resolver returns `SessionProjectRoot { path, source }`); VP-079 v1.24 extended for Events 11–13, VP-028 v1.1 scope note, VP-133 v1.6 facet 7(e) (VP-INDEX v3.30 amended; no count change). §Downstream deltas 23-33 (product-owner: BC-1.18.011 v1.18, BC-1.18.013 v1.10, BC-3.08.001 v1.35, error-taxonomy v1.39; story-writer: S-25.08, S-25.06). |
| 1.20 | 2026-10-07 | architect | S-25.08 local adversary pass 1 — normative rulings on seven spec-level gaps in the shared admission core (HEAD 292ffed5 reviewed read-only). **F-001** release on BOTH `PostToolUse` and `PostToolUseFailure` for the same `tool_use_id` (Claude Code delivers a failed tool call only as `PostToolUseFailure`; registered in `hooks.json.template`; payload carries `tool_use_id`, captured by `HookPayload.extra`); gate by a named pure predicate, no new `EventType` variant, no `tool_name` filter; neither event ⇒ TTL backstop; fixture gap recorded. **F-002** anchoring: the gate guards the session's own `factory_root` = resolved `<CLAUDE_PROJECT_DIR>/.factory` (must exist; never created); migration-state always `<factory_root>/migration-state`; component-wise classification replaces `contains(…)`; out-of-root `.factory` paths are out of scope (admitted, no reservation, no directory); worktree/symlink behaviour specified; residual + backstop stated. **F-003** shared `resolve_target_path`: lexical + POSIX-correct symlink resolution of the deepest existing ancestor, union of resolved/lexical match, unresolvable ⇒ lexical (fail-closed), `\` separator only on Windows, ALWAYS case-insensitive component compare (no FS probe; rationale); §5c Bash leg bound to the same function/anchor. **F-004** admission and release are registry-independent: evaluated before `Registry::load`/`resolve_registry_path`; obligations O1–O4; supersedes the "position … already holds" wording. **F-006** "foreign" in the shared core = `migration_id ∉ K` (K = the two known ids); a live txn of either known migration is decided against its own record selected by `migration_id`; `txn_is_own_migration` → `txn_migration_known`; five-row table restated; binary cross-migration refusal unchanged. **F-008** timestamp rules: skew tolerance 300 s, future-beyond-tolerance and pre-epoch/out-of-range `created_at` ⇒ unparseable ⇒ mtime, unknown age ⇒ NOT stale, never clamp; retention-over-reclamation rationale. **F-009** variants `ReservationTtlBelowFloor`, `InvalidToolUseId` (fail-closed when key present but invalid), new codes `E-MAINTENANCE-002` and `RESERVATION_TTL_BELOW_FLOOR`; `tool_use_id` grammar stated. Added §5a v1.20 test mandate, §Downstream deltas 14-20 (BC-1.18.011 v1.16 / BC-1.18.013 v1.8 / error-taxonomy v1.36 / stories), §Files to Change row. No state-machine, Dekker, or §Decision 1–4/7a–d/8–12 change. VP-133/VP-143/VP-147 anchors amended same burst. **Same-v1.20 consistency correction (product-owner conflict report):** the registry-error behaviour cited in §5a "Evaluation position", the F-004 test mandate and Downstream item 20 is restated from `main.rs::run` (HEAD 292ffed5): schema-version mismatch (`E-REG-001`), async+block conflict (`E-REG-002`) and duplicate entry (`E-REG-003`) exit 2 (fail-closed); not-found/parse/regex errors, `resolve_registry_path` Err and Tier-1 degraded are fail-open (exit 0); release-on-block keys on `code == 2`. **Same-v1.20 implementation rulings (S-25.08 HEAD 83f0f549):** (a) `FactoryRoot.lex_aliases` RATIFIED — the lexical side of the scope union compares against exactly two spellings of the session's own root (canonical, and as-given `CLAUDE_PROJECT_DIR/.factory`), both from the same env value, so no cross-project over-match; migration-state stays on the real root (§5a "Lexical root spellings"; Downstream item 21). (b) `executor::resolve_shard_gate_precedence` REMOVED with its tests as superseded by the structural early return (Downstream item 22; one real-binary test replaces them). **Same-v1.20 alignment with product-owner ruling (BC-1.18.013 v1.9 / BC-1.18.011 v1.17 / error-taxonomy v1.38):** a Branch C terminal record that reads but does not verify (unparseable, non-UTF-8, empty/truncated, wrong schema, count ≠ N, id/hash mismatch) yields `E-MAINTENANCE-001` WITH the completion-record-mismatch suffix (not plain); a failed read call is `E-MAINTENANCE-002` (`io`); "plain" = no suffix, only for gate-only block, terminal record absent (`NoOp`), foreign refusal, live coordinator. §Error Code Semantics E-MAINTENANCE-002 rule corrected; every other "plain" usage in this ADR audited and already matches. The earlier text calling the whole registry-load path "fail-open" was imprecise. **Same-v1.20 wording fix (E-MAINTENANCE-002 causes):** the Error Code Semantics row's `io` ("cannot … read gate/txn") and `state_integrity` ("unreadable …") overlapped; replaced by the total decision rule aligned with BC-1.18.013 v1.8 Precondition 6(c)/EC-032 — `io` = OS call failed (non-ENOENT), `state_integrity` = bytes read but unusable, ENOENT = absent semantics, fixed evaluation order, reservation files never read by admission, a terminal record that reads but does not verify = plain E-MAINTENANCE-001. The code's E-REG numbering disagrees with error-taxonomy rows E-REG-001..003 (which list other meanings, exit 0) — flagged to product-owner; no registry policy change. |
| 1.19 | 2026-10-07 | architect | S-25.08 implementation finding — first-activation reservation race (ratifies product-owner BC-1.18.013 v1.7 Precondition 6(c) / BC-1.18.011 v1.15; same decision, option (a)). (1) §5a step 0 appended: admitter first ensures `.factory/migration-state/reservations/` by idempotent recursive create (already-exists is success; no existence pre-check, no lock); admission/reservation/release never conditioned on pre-existence of `migration-state/`; absent `gate-state.json` = OPEN, absent txn set = no live txn; Dekker argument holds from the first protected write ever; creation failure fails PreToolUse closed. (2) Cost note added: cost applies to every protected Edit/Write/MultiEdit (not only during migrations), still ≤ low-single-digit ms, accepted. (3) §7e "Why (D4)" parenthetical corrected: "(`.factory/migration-state/` does not exist)" → "(no txn record, terminal record or pointer exists; the empty `migration-state/reservations/` namespace may exist from admission)". (4) §Files-to-Change `executor.rs` row: removal of the directory-exists early return on the admit path. Verification: VP-133 reserve-then-verify facet, VP-146 a4, VP-147 h4 gain the obligation "reservation namespace exists from the first protected write" (see those VPs for Kani-provable vs black-box split). Versioned v1.19 (not an in-place edit of v1.18) because v1.18 was already committed. |
| 1.18 | 2026-10-06 | architect | Formal-finding exception (v1.14/DEF-1 pattern) adjudicating S-25.06 formal-verification defects D1/D2/D3 (HEAD 9886cbc1) and BC-1.18.013 v1.3 / error-taxonomy v1.32; two further spec defects found while adjudicating against merged B2 code. **Ratified from PO v1.3:** PreToolUse Branch C verify-then-finalize (COMMITTING + verifying terminal record ⇒ txn→COMPLETED THEN gate→OPEN; mismatch ⇒ fail-closed; safety analysis recorded in §5a Branch C); coordinator exemption (keyed solely on §5c Branch 1–4 classification; implemented in the OBL-4 classifier, Rust gate leaves Bash unprocessed); shared protected-path union and shared gate/txn-dir/reservation-dir/`exclusive.lock` (this ADR already specified the union; merged B2 code already scopes to it, but deviates elsewhere); drain parameters 30 s / 3,600 s default / 1,800 s floor / no PID liveness CONFIRMED (merged B2 `DEFAULT_MAX_RESERVATION_TTL`=120 s corrected). **Amended/added:** §4e table — completed.json+COMMITTING verify-then-finalize row and mismatch row; §5c Branch 2 step 0.5 (the "no active txn"-only reconciliation gap); §5a step-3.5 Branch C + non-reconciliation note (1); §5a "Shared protected-path union and shared admission state" (single shared core evaluated once, before `shard_cap_precheck`; Bash leg stays OBL-4 with stated residual/backstops); "Coordinator exemption"; "Release-on-block" (dispatcher removes its own reservation when a later stage blocks); "Reservation timing parameters"; **D5** reserve-then-verify admission order replacing the unsound `LOCK_SH`-on-rename-replaced-gate-file ordering (Dekker argument recorded); **D4** new §Decision 7e — per-migration terminal record/pointer (`completed.json`/`CURRENT.json` stay B2's; `backfill-append-logs` uses `completed-backfill-append-logs.json`/`CURRENT-backfill-append-logs.json`), `migration_id` txn discriminator, cross-migration recovery refusal, physical gate file name `gate-state.json`; §Error Code Semantics (`COMPLETION_RECORD_MISMATCH_ABORT`; `ALREADY_MIGRATED` per-migration); v1.18 test mandate; §Verification Strategy re-verification obligation for the delta with the Kani-vs-fault-injection claim boundary; §Downstream BC/taxonomy deltas for product-owner; §Files to Change. Performance (point 5): per-protected-mutation reservation create+delete (no fsync, no admitter lock) measured against ADR-020 Class A (p95 ≤ 1,500 ms; observed 1,050–1,161 ms process-spawn-dominated) is a ≤ low-single-digit-ms increment confined to Edit/Write/MultiEdit on protected paths — accepted, no alternative required. §Decision 1/2/3/4a–d/7a–d/9–12 unchanged. **Same-amendment follow-up (human approved the formal-finding exception + B2-1..B2-4 in S-25.06, 2026-10-06):** (1) §4e mismatch row, §5a step-3.5 predicate and §5c step 0.5 reworded so STAGING + terminal record is unambiguously ALWAYS fail-closed (never finalizable, no verification attempted) and only COMMITTING is finalizable via Branch C — wording-only, matches PO's reading; (2) §7e label discipline: foreign-txn refusal is a `LockContention`-class refusal, not `COMPLETION_RECORD_MISMATCH_ABORT`; (3) §Downstream deltas 8-12 to product-owner; (4) §Verification Strategy / VP facet meanings re-baselined (VP-143/146/133/124-class, see VP-INDEX). Canonical B2 defect labels: B2-1 `reconcile_stale_admission_gate` dead code → wire Branches A/B/C; B2-2 unverified `completed.json` short-circuit → verified own-migration finalize; B2-3 120 s TTL + mtime staleness → 3,600 s / 1,800 s floor + `created_at`; B2-4 check-then-reserve race → reserve-then-verify. **Same-version alignment amendment (uncommitted v1.18; matches product-owner BC-1.18.011 v1.13 PC6(d) / BC-1.18.013 v1.6 PC6(b)):** (a) Branch B on-disk marker made exact in §4e F2 row, §5a step 3.5 Branch B and the test plan — txn record rewritten in place to `state: ABORTED` with top-level string field `abort_reason: "null_generation"`, optional on read, informational only, never consulted for admission or reconciliation decisions (replaces the underspecified "null_generation disposition marker"); (b) §5a now states the E-MAINTENANCE-001 `<scope>` keying rule (written path family: `BC-INDEX` / `.factory/cycles/`; live txn `migration_id` only in the `tracing::warn!` diagnostic) with the exact BC-1.18.013 PC6(b) format string; (c) §5a Branch C names the shared pure decision core `decide_terminal_record_reconciliation`. **inputs:** dropped downstream artifacts (S-25.06, BC-1.18.011, BC-1.18.013, error-taxonomy) to restore an acyclic input-hash graph; still cross-referenced in §Downstream. |
| 1.17 | 2026-09-23 | architect | Adds §Decision 12 — OBL-1 verification architecture (documentary; discharges [D-1232-OBL-1], the implementation-phase Kani + fault-injection obligation this ADR's own §Verification Strategy subsection mandated). Documents the `Fs` trait seam (`crates/factory-dispatcher/src/shard_manager/migration_fs.rs`, 7 core ops + `append`) as the abstraction boundary enabling both Kani model-checking (in-memory two-namespace `live`/`durable` model) and real-filesystem fault-injection (feature-gated `fail`-instrumented crash points), with `StdFs` delegating to the existing §Decision 7d `F_FULLFSYNC`/`fsync` durable-write primitives — zero behavioral change to production I/O. Records the two-pronged verification split: 7/7 `#[kani::proof]` harnesses PROVED in `crates/factory-dispatcher/src/shard_manager/obl1_kani_proofs.rs` (recover() totality, recovery-safety predicate, transition inductive invariants, INV-GATE-TXN admission-gate quiescence re-verification, bounded pointer-swap crash-atomicity + WAL-ordering before/after regression, recovery idempotence), executed via the `kani` CI job (`.github/workflows/kani.yml`); 30/30 green fault-injection tests in `crates/factory-dispatcher/tests/bc_1_18_011_b2_migration_crash_injection_test.rs` (child-process `abort()`-based crash injection at real syscall boundaries, empirically confirming the `Fs` model's axioms against the real OS). Cross-references the §Decision 7b WAL-ordering conformance fix: the implementation had deviated from the always-ratified INTENT-before-pointer-swap ordering (constructing/appending INTENT records post-rename instead of pre-swap); the fix brings code into conformance with the unchanged §Decision 7b text (spec wins, per CLAUDE.md Standing Rule) — Kani harness 5 is the before/after regression proof. **Governance verdict: REFINES/IMPLEMENTS, not CHANGES-RATIFIED** — §Decision 4e/5a/7c's frozen state-machine text is unchanged by this entry; it documents an already-mandated verification harness architecture, not a new design decision, so it does not trigger the "formal-finding exception or superseding ADR" bar in §Status. Full governance analysis on file as `obl1-recover-refactor-design.md` (architect work product). No BC-1.18.011/BC-1.18.010/error-taxonomy amendment required — this entry changes how the ratified behavior is verified and how the already-ratified ordering is correctly implemented, not what any BC promises callers. ARCH-INDEX / VP-INDEX / BC-INDEX / STATE.md cross-document propagations are out of scope for this edit (state-manager owns those). |
| 1.16 | 2026-09-22 | architect | In-place clarification (S-25.02 cluster-5 F4 stub-architect ambiguity adjudication; no design/substance change — the §Decision 4e/5a/7c state-machine freeze is unaffected). §Decision 5c's "Negative tests (cluster-5 TDD scope)" label was internally inconsistent with this ADR's own §Status block, which (since v1.15's D-1232 ratification) explicitly tracks the §5c classifier and its 4 dispatcher-guard amendments as "[D-1232-OBL-4] devops-engineer implements the 4 dispatcher-guard amendments specified in §Decision 5b/5c against this ADR's frozen spec text" — a post-ratification cluster-5 F4 ACTIVATION-boundary deliverable, distinct from cluster-5 TDD (contrast [D-1232-OBL-1], which explicitly DOES block cluster-5 TDD completion) — and with the §Files-to-Change table, where `validate-factory-path-staging`/`validate-factory-path-staged` are both explicitly devops-engineer/cluster-5-activation rows, never an implementer/cluster-5-TDD row. The stale label (predating the D-1232 OBL split) risked test-writer authoring Bash-admission Red Gate tests inside cluster-5's T-10/T-11 TDD scope for logic that `bc_index_migration_admission_precheck` (BC-1.18.011 Precondition 6(b), Edit/Write/MultiEdit only) does not and should not implement. Fixed: label corrected to attribute the negative-test list to devops-engineer's OBL-4 activation deliverable; the test list itself (REJECTED/BRANCH-2/H1 assertions) is unchanged — it remains this classifier's own acceptance criteria, just correctly attributed. No change to §Decision 1–11 substance, the ratified state machine, or any BC/error-taxonomy content. |
| 1.15 | 2026-09-20 | architect | Status flip: `proposed` → `accepted`, per POLICY 22 ratification (D-1232, 2026-09-20). Basis = the D-1231 mechanical Kani proof: DEF-1 (HIGH) concurrency regression fixed in v1.14 via the Option-B structural drain reorder (initial txn-record write moved to step 3a, strictly after the step-3 DRAINING flip); re-verification confirmed 7/7 VP proofs PROVED, INV-GATE-TXN invariant (`gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}`) UNSAT under the corrected model, non-vacuity CONFIRMED, and 5/5 regression + 7/7 fault-injection tests PASS. §Status block updated from the v1.13/v1.14 "PROPOSED" narrative to an ACCEPTED disposition reflecting this ratification; the two outstanding v1.13 human sign-off items ((i) macOS exec-TOCTOU sub-instruction stat→execve gap residual window, (ii) APFS darwin-arm64 directory-fsync durability test) and the remaining cluster-5 activation-boundary obligations ([D-1232-OBL-3] CLAUDE.md §Decision 8 amendment application, [D-1232-OBL-4] the 4 dispatcher-guard amendments per §Decision 5b) are UNCHANGED by this burst and remain tracked as post-ratification activation-boundary work, not ratification blockers. No content change to §Decision 1–11 substance; this is a status/lifecycle-only amendment. ARCH-INDEX / VP-INDEX / BC-INDEX / STATE.md updates are out of scope for this edit (state-manager owns those cross-document propagations). |
| 1.14 | 2026-09-20 | architect | Fix-burst (Kani model-checking pass-1, DEF-1, human-directed spec/design convergence per D-386 Option C — "more convergence before accept-at-floor ratification"; pipeline PAUSED, cluster-5 TDD BLOCKED, unaffected by this burst). DEF-1 (HIGH) — uncovered self-heal window: v1.11's step-2.5 reordering (writing the initial txn record BEFORE the DRAINING flip, as a stated "defense-in-depth" measure) opened a NEW crash sub-window that did not exist before v1.11 — a coordinator crash strictly between the old step 2.5 (txn written, gate STILL OPEN) and step 3 (DRAINING flip) left durable state `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead coordinator`. §5a step 3.5's self-heal predicate (gated on `gate_state ∈ {LOCKED, DRAINING}`) never matched this state; the ordinary admission check blocked the writer on the active STAGING txn regardless of gate_state; §4e's DISCARD row for this sub-state is reachable only via migration-binary re-invocation, contradicting this ADR's own v1.10/v1.13 guarantee that E-MAINTENANCE-001 always self-heals by the next PreToolUse. Root cause: v1.11's own rationale for the pre-DRAINING txn write was that the flock check (step 3.5 sub-step a) was already the primary/sufficient exclusion guard — the pre-DRAINING write was redundant defense-in-depth that introduced this regression without adding real safety. Three candidate fixes were evaluated: (A) extend step 3.5 Branch B to also cover `gate=OPEN` (adds another special-case branch — the exact anti-pattern behind the pass-8/9/13 regressions); (B) reorder the drain so the DRAINING flip (step 3) precedes the txn write (structural fix, chosen); (C) make the txn-write and gate-flip a single atomic critical section across two separate files (rejected — the two writes are already effectively serialized by the coordinator's continuous hold on `exclusive.lock`, so "atomicity" does not address the actual defect, which is a *durable-state reachability* problem, not a concurrency-atomicity problem, and cross-file atomic writes would be a materially larger redesign for no additional safety). Fixed (Option B): §Decision 5a drain procedure reordered — the initial txn-record write moves from "step 2.5" to new step **3a**, strictly AFTER step 3 (DRAINING flip) completes, restoring the pre-v1.11 ordering guarantee (`gate_state = OPEN` implies no active STAGING/COMMITTING txn record can exist) while KEEPING the v1.11 flock-gated step-3.5 sub-step-a check as the sole load-bearing writer-exclusion mechanism, unchanged. No new step-3.5 predicate branch was added; Branch A and Branch B are unchanged, and Branch B's precondition (`gate ∈ {LOCKED, DRAINING}`) is now provably always true whenever `txn=STAGING(generation_id=null)` exists, closing the gap by construction rather than by enumeration. Drain-timeout abort (step 4) simplified: the txn record now always exists by step 4 (step 3a is unconditional and precedes it), removing a conditional branch. Swept: §4e scope note, step-3.5 Branch B text and non-reconciliation Note, "Applied same-burst" self-heal description, fault-injection test mandate (2 new DEF-1 tests: step-3/step-3a interstitial-crash test and a construct-and-assert-UNSAT regression test proving `gate=OPEN` with an active txn is unconstructible), §Files-to-Change `executor.rs` and `tests/` rows, Status block. §Consequences → Verification Strategy: added the v1.14 DEF-1 sanctioned formal-finding exception to the §Decision 4e/5a/7c prose-freeze clause, plus a precise re-verification directive (model-order change, crash-point enumeration, and the INV-GATE-TXN invariant statement — `gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}` — that the formal-verifier's next Kani pass, labeled VP-M7, must re-prove) for the formal-verifier follow-up pass. §BC Impact: added v1.14 note confirming no BC or error-taxonomy changes are required (the fix is purely an internal step-ordering change; `E-MAINTENANCE-001`'s trigger condition was already accurate and remains unchanged). §Source/Origin and §References updated with Kani pass-1 provenance (D-1231). In-place v1.14 correction (unrelated stale citation found while restructuring §Decision 5a): "Abort gate-reset obligation" list item "Drain-timeout abort (step 3 above)" corrected to "(step 4 above)" — the drain-timeout abort has always been the step-4 reservation-poll-timeout path, not the step-3 DRAINING flip; this was a pre-existing stale citation unrelated to DEF-1, fixed in-scope per the mechanical-fix production-grade default. |
| 1.13 | 2026-09-13 | architect | Fix-burst (adversary pass-10 accept-at-floor, ADR-owned findings). HIGH-1 (HIGH) — §5a step 3.5 self-heal predicate did not cover `txn=STAGING AND generation_id=null` crash sub-state: after v1.11 step-2.5 writes txn=STAGING(null) before DRAINING flip, a mid-drain coordinator crash leaves gate∈{DRAINING,LOCKED} + txn=STAGING(active, generation_id=null); step 3.5's "no active txn" check does not match → no self-heal → ordinary writers permanently blocked with E-MAINTENANCE-001. Stale v1.10 HIGH-1 test mandate said "txn is first written at drain step 7" — flatly contradicted by v1.11 step 2.5. Fixed: step 3.5 predicate extended to "gate∈{LOCKED,DRAINING} AND (no active txn OR txn=STAGING AND generation_id=null)"; new sub-step e null-generation branch: set txn→ABORTED(null_generation disposition; retained per GC policy), flip gate→OPEN, admit; EWOULDBLOCK → block E-MAINTENANCE-001 (no flip). Stale v1.10 HIGH-1 test mandate rewritten to reflect step-2.5 reality. New v1.13 HIGH-1 fault-injection test added. Applied same-burst note updated: self-heal now covers no-active-txn AND active-STAGING-with-null-generation-dead-coordinator. MED-1 (MED) — §Error Code Semantics EXPIRY_ABORT trigger enumerated only two arms (expired manifest; absent manifest at STAGING resume) but §4e F2 DISCARD also exits EXPIRY_ABORT for a valid manifest when discarding a null-generation STAGING record. Fixed: EXPIRY_ABORT trigger widened with third arm "OR pre-generation crash sub-state (txn=STAGING, generation_id=null) discarded despite valid manifest (v1.12 F2)"; PO handoff note added to widen error-taxonomy.md EXPIRY_ABORT trigger accordingly. MED-2 (MED) — three residual "unchanged from v1.2" dangling annotations: (a) §Downstream BC-1.18.010 Invariant-2 amendment said "Applied per v1.2 text (unchanged from v1.2)" with no inlined text; (b) §Decision 10 body said "CI parity test … unchanged from v1.2"; (c) §5c negative-tests preamble said "unchanged from v1.2 with addition". Fixed: (a) Invariant-2 amendment inlined: three-way parity text (config.arch_index_sha == manifest.approved_arch_index_sha == live ARCH-INDEX SHA, §Decision 10). (b) §Decision 10 annotation stripped. (c) §5c preamble annotation stripped. MED-3 (MED) — §4e disposition contradiction: F2 null-generation row said "DISCARD: delete the partial null txn record" (hard delete) but F4 GC policy said "ABORTED records MUST NOT be deleted before archival"; manifest-absent/expired row said ambiguous "discard partial txn record → ABORTED". Fixed: both rows now say "set txn state → ABORTED (retained per GC policy)"; F2 row adds null_generation disposition marker; GC policy is internally consistent; step 3.5 new branch uses same ABORTED+retain disposition. LOW-1 (LOW) — §Decision 7c step 1 did not state that generation_id is persisted ONLY AFTER gen-dir durable-create + fsync; entire §4e generation_id-partition is only sound under this ordering. Fixed: ordering invariant added to step 1; corruption case (generation_id set but gen dir absent/incomplete) defined as fail-closed EXPIRY_ABORT. LOW-2 (LOW) — ADR-052 changelog/§Source cites "ADR-051 v1.14" but ADR-051 frontmatter carries no version: field (body-changelog versioning). Fixed: clarifying note added in §References; follow-up action noted. §Verification Strategy subsection added to §Consequences: §Decision 4e/5a/7c concurrency state machine frozen for prose review after 10 adversary passes (D-386 Option C asymptotic floor); definitive formal verification deferred to cluster-5 implementation-phase Kani harnesses. §Source/Origin and §References updated with pass-10 (D-1230) provenance. |
| 1.12 | 2026-09-13 | architect | Fix-burst (adversary pass-9 NOT-RATIFIABLE, ADR-owned findings). F2 (HIGH) — §4e null-STAGING crash sub-state: v1.11 drain step 2.5 writes txn=STAGING with generation_id=null before snapshot (step 5) and generation build (§7c step 1); a crash between step 2.5 and §7c step 1 leaves null-generation STAGING; §4e single STAGING row routes to EC-003 census but no generation exists and PC1 would compare against null source_body_row_sha256. Fixed: §4e STAGING row split by generation_id presence — (a) generation_id=null: DISCARD null txn, flip gate→OPEN, exit EXPIRY_ABORT; MUST NOT census a null generation; MUST NOT compare PC1 against null source_body_row_sha256; (b) generation_id set + gen dir present: existing EC-003 resume path. Exhaustiveness scope note updated. Fault-injection test mandate added. F3 (MED) — residual/inverted reader wording: §Downstream BC-1.18.010 §Reader Integration step 2 used existence-check "if absent" form (not open-with-ENOENT-fallback); §BC Impact v1.4 Invariant-3 Applied cell described "canonical-first / generation-fallback; ENOENT is not possible" (inverted/known-regression direction in an Applied column); BC-1.18.010 provenance labels stale at v1.8 after v1.11 MED-1 open-with-ENOENT-fallback handoff applied BC-1.18.010 v1.9. Fixed: §Downstream step 2 → open(gen-id/file); on ENOENT open(canonical/file); §BC Impact Invariant-3 Applied cell corrected to generation-first/open-with-ENOENT-fallback; v1.5 C-1 applied description updated; all BC-1.18.010 v1.8 provenance labels bumped to v1.9; existence-check language swept from §Downstream + §BC Impact. F4 (MED) — §4e ABORTED row absent + txn-selection ambiguity + terminal-txn GC unspecified: multiple txn-*.json files can coexist; no disambiguation rule; no GC/archival policy → unbounded file growth. Fixed: added explicit ABORTED row ("treated as no active txn; fresh activation permitted"); added activation_id-matching txn-selection rule below table (highest created_at fallback); added GC/archival policy (ABORTED/COMPLETED records archived to .factory/migration-audit/txn-archive/ at next migration completion audit burst; ABORTED records not deleted before archival). F6 (LOW) — §5c Branch 2 gate reconciliation acquired gate LOCK_EX without exclusive.lock flock-acquirable precondition, diverging from step 3.5 and crash-recovery paragraph parity. Fixed: Branch 2 gate reconciliation now first attempts flock(exclusive.lock, LOCK_EX\|LOCK_NB); on acquired proceeds with steps 1–4 + releases both locks; on EWOULDBLOCK logs warning and exits 0 ALREADY_MIGRATED without gate reconciliation. Fault-injection test mandate updated. §Source/Origin and §References updated with pass-9 (D-1229) provenance. |
| 1.11 | 2026-09-13 | architect | Fix-burst (adversary pass-8 NOT-RATIFIABLE, ADR-owned findings). HIGH-1 (HIGH) — step 3.5 self-heal reopened gate during a LIVE drain: no flock check on exclusive.lock meant gate ∈ {DRAINING,LOCKED} + no-txn matched the reconciliation predicate even while a live coordinator held exclusive.lock mid-drain (between drain step 3 DRAINING flip and drain step 7 txn write), allowing a concurrent PreToolUse to flip gate→OPEN and admit itself — breaking BC-1.18.011 Precondition 6 writer-exclusion while the coordinator snapshots/stages. Fixed: step 3.5 now first attempts `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` before any gate manipulation; on EWOULDBLOCK (live coordinator) block with E-MAINTENANCE-001 without flipping gate; on LOCK_EX acquired (no live coordinator) proceed with existing gate→OPEN flip; sub-steps relabeled a→h; flock-acquire → re-read-under-lock → flip → release discipline matches the existing "Crash-recovery gate reconciliation" paragraph (parity). Defense-in-depth: drain reordered — initial txn record (state=STAGING, source hashes null) written at new step 2.5 BEFORE DRAINING flip at step 3 — ensures gate=DRAINING in a live drain always implies txn=STAGING; step 7 updated to "update txn record with source hashes"; drain timeout abort (step 4) now also sets txn→ABORTED. Fault-injection test mandate added. §Files-to-Change executor.rs row swept. MED-1 (MED) — §Decision 7c reader protocol used existence-check-then-open (TOCTOU-racy): "If the file EXISTS at its generation-dir path, use it" — a rename gen→canonical between exists() and open() yields ENOENT. Fixed: reader protocol step 2a changed to open(gen-<id>/<file>); on ENOENT fall back to open(canonical). Race-free: files move one-directionally gen→canonical. "ENOENT is impossible" rationale updated. BC-impact handoff for PO: mirror open-with-ENOENT-fallback in BC-1.18.010 §Reader Integration step 2 AND BC-1.18.011 Invariant 3. MED-2 (MED) — §BC Impact BC-1.18.010 provenance tables still imperative after v1.10 LOW-2 partial sweep: "v1.3 change required"/"v1.4 change required" headers and imperative cells not swept. Fixed: both BC-1.18.010 table headers → "Applied (BC-1.18.010 v1.8)"; imperative cells converted to past-tense provenance. Sibling sweep complete. LOW-1 (LOW) — §Decision 9/10 headings, §Decision 1 body, and §5b body carried residual "unchanged from v1.2"/"as v1.2" self-references. Fixed: stripped from headings and body; "The same 4 guards as v1.2" → "The following 4 guards" in §5b. §Source/Origin and §References updated with pass-8 (D-1228) provenance. In-place v1.11 correction (sibling-sweep residue — MED-1 handoff blockquote): §BC Impact v1.7 Invariant 3 handoff blockquote updated from existence-check/generation-first framing to open-with-ENOENT-fallback wording per §Decision 7c step 2a; "ENOENT is impossible" claim replaced with accurate ENOENT-safe rationale (files move one-directionally gen→canonical via atomic rename(2); required file never absent from both paths simultaneously during COMMITTING window). In-place structural fix: escaped `LOCK_EX\|LOCK_NB` flag notation in §Files-to-Change executor.rs table cell (validate-table-cell-count hook). |
| 1.10 | 2026-09-13 | architect | Fix-burst (adversary pass-7 RATIFY-WITH-CHANGES, ADR-owned findings). HIGH-1 (HIGH) — step-3.5 PreToolUse self-heal predicate gated on completed.json presence, missing two symmetric crash windows that leave the gate permanently stuck: (i) any abort path (census §7c step 3b / fingerprint §7c step 5 / expiry §7c step 4) crashes AFTER writing txn→ABORTED but BEFORE the gate→OPEN flip — gate=LOCKED, txn=ABORTED, no completed.json — step 3.5 skips because completed.json is absent; (ii) drain-timeout abort fires with gate=DRAINING and NO txn record (txn first written at drain step 7, after LOCKED) — step 3.5 required gate=LOCKED so DRAINING was never considered. Fixed: step-3.5 predicate generalised from "gate=LOCKED AND no active txn AND completed.json present" to "gate ∈ {LOCKED, DRAINING} AND no active txn (absent/COMPLETED/ABORTED) — regardless of completed.json presence"; step 3.5 title updated; sub-items (d) lists all four covered scenarios; "Note" at end updated to ∈ {LOCKED, DRAINING} phrasing; two new fault-injection tests added ("abort-crash before gate→OPEN — txn=ABORTED, no completed.json" and "drain-timeout crash — gate=DRAINING, no txn record"); §Files-to-Change executor.rs row updated to cite generalised reconciliation; PO handoff note added to step 3.5 for E-MAINTENANCE-001 "always TEMPORARY" reconciliation. MED-1 (MED) — §Error Code Semantics EXPIRY_ABORT trigger row described trigger solely as "manifest has expired … AND txn=STAGING" but §Decision 4e (v1.8 F-4) routes both expired AND absent manifest at STAGING resume to EXPIRY_ABORT. Fixed: EXPIRY_ABORT trigger widened to "manifest has expired OR is absent at STAGING resume (pre-pivot; no canonical paths changed)"; PO handoff note added to the row pointing to the matching error-taxonomy.md correction needed. MED-2 (MED) — TTL-based reservation GC (v1.9 H1) retains an abandoned reservation for up to MAX_RESERVATION_TTL (3,600 s); every drain attempt aborts with DRAIN_TIMEOUT_ABORT until TTL elapses, with no operator remediation documented. Fixed: added "Operator remediation for abandoned-reservation stall" paragraph to §Decision 5a Stale reservation cleanup section: identify confirmed-stale reservation (tool_use_id not owned by any live harness session); SAFE MANUAL DELETION is SANCTIONED pre-activation; TTL lower-bound is heuristic, not structurally enforced; activation SHOULD be scheduled during quiescence; TTL MAY be lowered toward lower bound to shorten stall duration. LOW-1 (LOW) — §Error Code Semantics section opened "emitted by the migration binary" but included E-MAINTENANCE-001 which is emitted by the native admission gate in executor.rs (dispatcher hook path), not the migration binary. Fixed: §Error Code Semantics restructured into two sub-tables: "Migration binary process exit codes" and "Guard-layer admission code"; E-MAINTENANCE-001 moved to the guard-layer sub-table with a header note distinguishing HookResult from process exit codes. LOW-2 (LOW) — BC Impact provenance tables under §BC Impact still used imperative future-directive phrasing ("v1.3 change required", "Replace with:", "v1.4 change required", "Add catalog rows…", "Each row must include…", "Codes to add:") despite all changes being applied in prior bursts. Fixed: BC-1.18.011 main table column header changed from "v1.3 change required" to "Applied (BC-1.18.011 v1.7)"; v1.4 additional changes table column header changed to "Applied (BC-1.18.011 v1.7)"; v1.4 additional changes to error-taxonomy.md block converted to past-tense provenance form. §Source/Origin and §References updated with pass-7 (D-1227) provenance. Status block updated. In-place v1.10 correction (no version bump): PO handoff notes for HIGH-1 (E-MAINTENANCE-001 "always TEMPORARY") and MED-1 (EXPIRY_ABORT trigger) converted to completed-provenance form — error-taxonomy.md v1.27 applied both same-burst. |
| 1.9 | 2026-09-13 | architect | Fix-burst (adversary pass-6 RATIFY-WITH-CHANGES, ADR-owned findings). H1 (HIGH) — drain quiescence unsound: drain step 1 used (pid, start_time) liveness GC, but the creating PID is the per-event PreToolUse dispatcher binary which exits microseconds after writing the reservation; every in-flight reservation appears stale to a liveness check, so step 1 would reclaim it while the tool is still running, causing the step-4 quiescence poll to see an empty dir and snapshot source MID-WRITE. Fixed: PID-liveness GC removed entirely from drain step 1 and from "Stale reservation cleanup" summary; reservation files now store only `created_at` ISO-8601 timestamp (no PID, no start_time); drain step 1 removes only files older than MAX_RESERVATION_TTL (default 3,600 s), a lower bound CANNOT be shorter than the maximum expected tool-call duration; fault-injection test mandate updated: new test "in-flight reservation (Pre fired, Post NOT yet) is NEVER reclaimed by drain step-1 TTL GC; coordinator waits for PostToolUse before snapshotting." M2 (MED) — §Downstream to Product-Owner (Amendments 1–9) and §BC Impact "what must change to match v1.3/v1.4" / "v1.4/v1.5/v1.7 additional changes … product-owner MUST apply" tables still carried imperative future-directive framing, but all BC/taxonomy changes are already applied (BC-1.18.011 v1.7, BC-1.18.010 v1.8, error-taxonomy v1.25). Converted to past-tense completed-provenance form: section headings changed from "required amendments" / "must apply" to "amendments applied" / "Applied:"; body preambles changed from imperative to past-tense; §Error Code Semantics product-owner MUST add → was added. Genuinely-pending items (crates/ implementation, 4 devops dispatcher-guard amendments, darwin-arm64 durability test) left future-tense. L1 (LOW) — §Source/Origin + §References cited passes only through "local pass-3 … producing this v1.6 fix-burst"; stale self-reference. Fixed: appended pass-4 (v1.7 / D-1224), pass-5 (v1.8 / D-1225), pass-6 (v1.9 / this burst) provenance rows; corrected "this v1.6 fix-burst" to past-tense in both sections. L2 (LOW) — §Decision 4e table and the v1.8 F-4 Finding→Resolution row claimed "exhaustive for all TXN-RECORD states" but the no-txn + manifest-present first-run case is handled by §5c Branch 3, not §4e. Fixed: added note below §4e table that first-run activation is out of scope; corrected F-4 row claim to "exhaustive for all re-run and recovery TXN-RECORD states; first-run handled by §5c Branch 3." In-place v1.9 correction: §Files-to-Change shard_manager.rs + tests/ rows swept from the superseded (pid,start_time) reservation-GC to the v1.9 H1 TTL model (created_at + tool_use_id); prose-vs-Files-to-Change gap closed. |
| 1.8 | 2026-09-13 | architect | Fix-burst (adversary pass-5 RATIFY-WITH-CHANGES, ADR-owned findings only). F-2 (HIGH) — §Files to Change rows for shard_manager.rs and tests/ still described PC1 as "byte-for-byte reconstruct vs source_sha256" / "concat-SHA vs source_sha256" (v1.6 whole-concat form that v1.7 F1 superseded); tests/ row also listed E-SHD-005 inside the migration census-gate test description (E-SHD-005 is a HookResult of the steady-state gate, not a migration process exit code). Fixed: shard_manager.rs row rewrites PC1 to "structured per-BC-row equivalence vs source_body_row_sha256"; source_sha256 removed from PC1 context (it is the step-5 fingerprint recheck hash only); E-SHD-005 removed from migration census-gate test listing; stale "v1.6" version labels updated to "v1.7". F-4 (MED) — §Decision 4e recovery-modes table was inexhaustive: the case `txn STAGING + manifest absent or expired` had no row. This is a valid scenario (manifest expires during slow staging build, or is missing at resume time); without a defined action the coordinator has no specified path. Fixed: new row added mapping `txn STAGING + manifest absent or expired` to clean-abort — delete staging generation, update txn → ABORTED, flip gate → OPEN, exit EXPIRY_ABORT (matching §4d clean-abort logic for the expired-pre-pivot branch). Root-cause handoff to PO + formal-verifier: error-taxonomy CONTENT_PRESERVATION_ABORT trigger must be updated from whole-concat-vs-source_sha256 to structured-equivalence-vs-source_body_row_sha256; BC-1.18.011 PC1 and VP-132 must be reconciled to the structured per-BC-row model (parallel PO + formal-verifier burst — completed same burst). In-place v1.8 tense correction (no decision content change): §BC Impact propagation blocks (a)/(b)/(c) converted from future-directive framing to completed same-burst-propagation form. LOW cleanliness: stripped residual "(unchanged)" / "(unchanged from v1.X)" annotations from §Context bold-section label and Decision 1/2/3 headings (version-agnostic phrasing; not historical explanations). |
| 1.7 | 2026-09-13 | architect | Fix-burst resolving all ADR-owned findings from 7th adversarial review (pass-4, D-1224): 2 HIGH + 6 MED observations. F1 (HIGH) — PC1 gate was unsatisfiable: whole-concat SHA against source_sha256 cannot match because the staged lean body adds the new §Subsystem Shard Manifest section and content is reordered. Fixed by introducing a dedicated `source_body_row_sha256` field in the txn record (captured at drain step 5 as SHA-256 of per-BC-row content in canonical BC-ID sort order, excluding header sections) and rewriting PC1 as structured-equivalence: extract BC-X.YY.NNN table rows from all staged shards in canonical BC-ID order, hash → compare against `source_body_row_sha256`; `source_sha256` (whole-file hash) is used only by step 5 fingerprint recheck; EC-001 remains reachable because PC2 independently checks ID-set. F2 (HIGH) — ADR error surface inconsistency: CENSUS_MISMATCH_ABORT trigger and step 3b shard-boundary bullet incorrectly referenced E-SHD-005 (a HookResult of the steady-state admission gate, not a process exit code of the migration binary). Fixed by removing all E-SHD-005 references from migration binary process-exit-code contexts; shard-boundary check renamed "internal capacity check (NOT E-SHD-005)"; §Error Code Semantics CENSUS_MISMATCH_ABORT trigger updated; BC impact handoff directs PO to scope E-SHD-005 to steady-state gate only. F4 (MED) — completion-crash reconciliation required binary re-invocation: after a crash between step 8 (completed.json write) and gate→OPEN flip, all writes were permanently blocked until binary was manually re-run. Fixed by adding stale-locked-gate reconciliation to §Decision 5a PreToolUse admission: if gate=LOCKED AND completed.json present AND no active txn, PreToolUse acquires LOCK_EX, flips gate→OPEN, then proceeds with admission. Fault-injection test mandate added. F5 (MED) — dangling "unchanged from v1.2 §Context" reference: §Context cited v1.2 §Context which no longer exists in-file. Fixed: removed dangling cross-reference; replaced with forward references to extant §Rationale (Option A viability) and §Decision 5a (native admission gate design). F8 (MED) — APFS dir-fsync durability test was a post-ratification deliverable: §Decision 7d listed the darwin-arm64 empirical test as "required before this ADR is considered fully validated on macOS" but did not block ratification. Elevated to RATIFICATION PREREQUISITE: this ADR MUST NOT be ratified as safe on macOS until the test completes; explicit residual-risk acknowledgment added to §Decision 11 POLICY 22 sign-off block. F9 (OBS) — stale "step 5" v1.3-handoff row in BC-Impact table: the v1.3 BC-Impact row for Postcondition 3a still said "step 5 in §Decision 7c" for the pointer swap; corrected inline to "step 6" per v1.4 fix (step 5 = fingerprint recheck; step 6 = CURRENT.json pointer swap). F10 (OBS) — PID-reuse hazard in stale-reservation GC: §5a stale-PID cleanup used PID alone, but OS PID reuse can cause false-negative (live process at reused PID). Fixed by specifying (pid, process_start_time) tuple in reservation files; GC compares both pid and start_time; PID match + start_time mismatch = PID reuse → reclaim. BC impact for product-owner: F2 (scope E-SHD-005 to steady-state gate; update CENSUS_MISMATCH_ABORT in error-taxonomy to remove E-SHD-005 reference; update BC-1.18.011 PC2/PC4/EC-001 trigger language to cite process exit codes not HookResult); F3 (completed.json casing sweep in BC-1.18.010/011); F6 (BC-1.18.011 Invariant 1/Precondition 2 must cite ADR-052 §Decision 7 crash-atomicity machinery, not claim no new machinery exists); F7 (BC-1.18.011 SDK Grounding: drop write_indeterminate_marker as atomic primitive; ground Invariant 1 against BC-1.18.006 shipped primitive in shard_manager.rs). POLICY 22 sign-off items: (1) macOS exec-TOCTOU residual window (existing); (2) F8 APFS dir-fsync durability test must complete as ratification prerequisite before ratifying macOS safety. |
| 1.6 | 2026-09-13 | architect | Fix-burst resolving all 12 findings from 6th adversarial review (local cascade pass-3, D-1223): 1 CRITICAL + 4 HIGH + 5 MEDIUM + 2 LOW. C-1 — Step 3b census gate was a tautology: renamed sha256(staged)==expected_post_hash check as staging-integrity (what it actually is, not PC1); added true PC1 (reconstruct concatenation → compare SHA-256 against source_sha256 from txn record); rewrote PC2 as per-ID set check (each ID in EXACTLY ONE shard, ZERO in retained body; count comparison alone insufficient — EC-001 dup+drop case must abort). H-1 — §4e table re-keyed on TXN-RECORD state (STAGING/COMMITTING/COMPLETED/ABORTED) per BC-1.18.011 Invariant 3's discriminator; impossible "CURRENT.json status:staging" row deleted. H-2 — Branch 2 ALREADY_MIGRATED path now reconciles stale gate before exit 0: acquire gate LOCK_EX; if gate≠OPEN and completed.json present with no active txn: flip→OPEN; fault-injection test mandate added. H-3 — Source/Origin and References: removed all citations to non-existent adv-cv-adr052-v13-closure-2026-09-13.md; replaced with real provenance (local cascade D-1221/D-1222/D-1223; adv-local-adr052-pass3.md being written this burst); Status finding-count corrected. H-4 — §Downstream Amendments 7/8/9 placeholder text replaced with exact replacement text mirroring BC-1.18.011 v1.5 (CURRENT.json pointer swap as commit-point; intent log recovery; completed.json permanent terminal record). M-1 — §5a PreToolUse: explicit text that admission checks BOTH gate_state AND txn-record state; gate_state is the durable proxy; either gate≠OPEN OR active txn (STAGING/COMMITTING) → E-MAINTENANCE-001. M-2 — macOS verify_and_exec_binary code sample: mtime re-stat added immediately before exec_by_pathname call (prose-code gap from H-2 v1.4 closure). M-3 — step 3c and §Error Code Semantics aligned: CONTENT_PRESERVATION_ABORT for PC1 (concat-vs-source_sha256) failures; CENSUS_MISMATCH_ABORT for PC2 (ID-set) and E-SHD-005 (boundary) failures; binary surfaces process exit codes, not HookResult. M-4 — CLAUDE.md amendment: .factory/cycles/*/ wildcard replaced with the four exact append-log paths (decision-log.md, burst-log.md, lessons.md, session-checkpoints.md in v1.0-brownfield-backfill). M-5 — Stale-reservation GC moved to start of EVERY drain (step 1 of drain procedure), not only recovery startup. L-1 — H1 title: "(v1.4 Fix-Burst)" version pin stripped. L-2 — Casing: all path-bearing references aligned to lowercase completed.json. inputs[] + adv-local-adr052-pass3.md. |
| 1.5 | 2026-09-13 | architect | Fix-burst resolving 9 ADR-owned findings from 5th adversarial review (1 CRITICAL + 4 HIGH + 7 MEDIUM). C-1 — Reader protocol regression fixed: v1.4's canonical-first/generation-fallback inverted to generation-first/canonical-fallback; generation-first is correct for BOTH net-new shards AND BC-INDEX.md (in-place overwrite target whose canonical path holds OLD monolithic body until step 7's rename); fault-injection reader test mandate added. H-1 — ADR made self-contained: §Decision 6 audit-trail decision inlined from v1.2 (completed.json substituted for COMMITTED phase marker per v1.3 redesign); §Decision 8 skipped-control inventory table inlined from v1.2; §Downstream Amendments 1–3 fully inlined from v1.2; no "see v1.2 §" dangling references remain. H-2 — mtime guard corrected: re-stat moved to IMMEDIATELY BEFORE execve call; previous "after exec returns" fired only on exec failure (execve never returns on success); "Human sign-off required" updated with corrected residual-risk semantics (sub-instruction window, not digest-check-to-exec window). H-3 — Reservation UUID corrected: PreToolUse creates `<tool_use_id>.reservation` using stable harness tool-invocation ID shared across Pre/Post hook pair; PostToolUse removes it by same tool_use_id without shared in-process state; test mandates added (Pre-creates/Post-removes; stale-PID cleanup). H-4 — Load-bearing version pins removed: CLAUDE.md amendment text updated from "ADR-052 v1.4 EXCEPTION" to "ADR-052 EXCEPTION"; all "see v1.2 §" refs inlined (H-1); BC-impact handoff directs PO to use stable §Decision N form throughout. M-2 — ADR-051 §Decision 10 amended: "at the SAME F4 activation moment mechanism A's backfill runs" coupling removed; B2 sub-split occurs within same one-time B2 operation independently of mechanism A schedule; ADR-051 bumped to v1.14. M-3 — E-MAINTENANCE corrected to E-MAINTENANCE-001 throughout (error-taxonomy.md SoT). M-4 — §Downstream BC-1.18.010 §Reader Integration: stale v1.3 "use gen-uuid/ paths" instruction deleted; single correct generation-first/canonical-fallback instruction kept consistent with C-1. M-6 — Bash admission and reservation: explicit text added to §5a that admitted Bash mutations with write effect create `<tool_use_id>.reservation`; §5c classifier determines write-effect; quiescence waits for all Bash reservations; test mandate added. Observations: L-2 validate_write_target() positive test cases added for BC-INDEX.md and BC-INDEX.shard-manifest.toml; L-3 mechanism-A wildcard bounded by config-driven exact paths documented; L-4 30s drain-timeout liveness note added. BC impact for product-owner: C-1 generation-first reader update (BC-1.18.010 §Reader Integration + BC-1.18.011 Invariant 3); H-4 version-pin cleanup (BC-1.18.010/011 + error-taxonomy); M-1 ADR-052 traceability row to BC-1.18.011 Architecture Anchors; L-1 error-taxonomy header audit; L-5 BC-1.18.011 EC-003 step citation correction. inputs: fix: removed non-existent adv-cv-adr052-v13-closure-2026-09-13.md. |
| 1.4 | 2026-09-13 | architect | Fix-burst resolving 9 ADR-owned findings from 4th adversarial review (2 CRITICAL + 5 HIGH + 2 MEDIUM). C1 — Reader no-content window: reader protocol changed to canonical-first / generation-dir-fallback during committing; gen-uuid/ clarified as CONTENT-IMMUTABLE (not moved-out-immutable); readers never see ENOENT during step 7 progress. C2 — F10 allowlist normalization: dot-lowercase BC-INDEX-SS-NN.a.md / .b.md / .c.md; generic BC-INDEX-SS-NN.manifest.toml; validate_write_target() accepts dot-letter pattern; CLAUDE.md amendment regenerated; ratification-time test mandate added. H1 — Abort gate-stuck: every abort path now atomically flips gate to OPEN; crash-recovery reconciles stale LOCKED/DRAINING with no active txn to OPEN; fault-injection test mandate added. H2 — Cross-process writer reservation: per-event dispatcher model stated; active_writer_count replaced by durable reservation dir .factory/migration-state/reservations/; PreToolUse admission atomic under gate_state lock; coordinator polls dir for emptiness. H3 — Missing pre-pivot census gate: new step 3b (content-preservation + census gate) inserted between intent-log WAL and authorization gate; verifies PC1+PC2+E-SHD-005; on failure: ABORT, txn ABORTED, gate OPEN; resume-from-STAGING RE-RUNs census (EC-003). H4 — PREPARED state name: all PREPARED occurrences in §5c and F5/F7 rows replaced with STAGING; §7a enum STAGING\|COMMITTING\|COMPLETED\|ABORTED is authoritative. M1 — Fencing token audit-only: fencing_generation downgraded from enforcement to AUDIT-ONLY metadata; flock provides actual mutual exclusion; prove-authority language removed. M2 — Pivot step contradiction: §4d corrected to reference step 6 (not step 4) for pointer swap; auth gate=step 4, fingerprint=step 5, pointer swap=step 6; BC-1.18.011 PC3a step correction noted for PO. M5 — macOS primary operator platform: darwin-arm64 identified as primary runtime with residual TOCTOU; Linux is where TOCTOU is eliminated; no-concurrent-build elevated to hard checklist with programmatic mtime guard; /proc dependency documented. Error Code Semantics section added. BC Impact handoff expanded with v1.4-specific changes. |
| 1.3 | 2026-09-13 | architect | Full redesign per D-1218 (3rd Codex cross-vendor closure review, 11 findings, NOT RATIFIABLE). Adopts atomic-pointer architecture from research-adr-052-v13-atomic-publication-2026-09-13.md. Closes all 11 findings: F1/F9 — single atomic CURRENT.json pointer swap over immutable staging generation; completed.json as permanent terminal record; eliminates ambiguous-absence heuristic. F2 — framed checksummed intent log with per-target expected post-hash + pre-state; matching-destination-hash recovery; fail-closed recovery decision table; fault-injection test mandate. F3 — real OPEN/DRAINING admission gate with PreToolUse-acquire/PostToolUse-release writer reservations; wait for active_writer_count=0 before snapshot; no non-atomic shared→exclusive flock upgrade. F4 — advisory flock on stable pre-created never-unlinked inode; fail-closed on empty/corrupt metadata by acquire-first; automatic stale reclamation via kernel on process death. F5 — durable txn record separate from flock; ordinary writers blocked by txn state (STAGING/COMMITTING) regardless of PID liveness; recovery-owner bumps fencing_generation to claim ownership; releasing flock never deletes txn record. F6 — authorization gate moved to immediately before CURRENT.json pointer swap (single pivot); post-pivot COMMITTING state retained regardless of expiry; completion-only recovery manifest bound to old activation_id + staged-generation hash + fencing generation (replaces fresh activation). F7 — conservative Bash admission: block unknown-write-effect commands while txn record in STAGING/COMMITTING; classify sanctioned commands from config-driven target sets; canonicalization + alias rejection. F8 — four guard branches: census (no manifest), terminal-state no-op (completed.json check), new activation (full validation), recovery (completion-only manifest); manifest/flock required only for new activation and recovery branches. F10 — single authoritative allowed-write-targets list; CLAUDE.md amendment generated from that exact list with containment check at every write; sub-shards (.a.md/.b.md), BC-INDEX.shard-manifest.toml, BC-INDEX-SS-05.manifest.toml added. F11 — Linux: fexecve/execveat(AT_EMPTY_PATH) for fd-binding exec; macOS: fexecve UNAVAILABLE per Apple docs; decision: freeze build under maintenance lock + documented residual TOCTOU; human sign-off required at ratification. Platform durability: Linux fsync+dir-fsync (mandatory); macOS F_FULLFSYNC on file (mandatory per Apple docs); APFS directory-fsync best-effort only (Apple docs inconclusive); darwin-arm64 empirical durability test required. Corrects v1.2 Amendment 6 which incorrectly treated dir-fsync as mandatory on all platforms. |
| 1.2 | 2026-09-13 | architect | Full redesign per D-1216 (2nd Codex RATIFY-WITH-CHANGES 8 findings). Resolves: F1 — native admission gate added in `executor.rs` covering ALL mutation tools (Edit/Write/MultiEdit/Bash) before shard_cap_precheck, replacing the `^Bash$`-only guard-level check; drain protocol via TOCTOU abort. F2 — per-target completion tracking in PREPARED marker + single TOCTOU check before first rename (not repeated between renames) resolves COMMITTED-after-last-rename vs "original untouched" contradiction; reader integration protocol specified (COMMITTED marker as read-path selector for BC-1.18.010). F3 — content-bearing lock file with PID+activation_id separates persistent maintenance intent from OS advisory lock; pre-PREPARED crash recovery path defined; stale-lock recovery specified. F4 — two-phase manifest validation: pre-lock (lightweight) then under-exclusion (repo_root_sha, expected_total_bcs, three-way ARCH-INDEX parity, readiness); pre-publication expiry recheck before COMMITTED; manifest CONSUMED marking replaces deletion (resolves "absent=rejected" vs "already-migrated=exit0" contradiction); three expiry-safe recovery modes defined. F5 — explicit three-way activation-time parity check: config.arch_index_sha == manifest.approved_arch_index_sha == live ARCH-INDEX SHA; catches stale-binary case (revision A config + revision B manifest + revision B live). F6 — accurate skipped-control inventory: brownfield-discipline description corrected to ".reference/ write protection"; factory-branch-guard row added with explicit waiver; validate-factory-path-staged PostToolUse Bash corrected from "bypassed" to "MUST be amended." F7 — full-command pre-shell classifier moved to guard layer (validate-factory-path-staging + destructive-command-guard) with executable digest verification; removed impossible "binary rejects metacharacters post-shell-parse" claim; negative tests specified. F8 — CLAUDE.md amendment text expanded to include BC-INDEX.md + migration-state/ + activation/ + migration-audit/ + config-specified mech-A targets; preconditions (a-e) separated from post-success obligations (f-g); CLAUDE.md rule referenced by text anchor not line number; 4 guard amendments listed (vs 3 in v1.1). Downstream to Product-Owner: Amendment 5 corrected (Edit/Write → ALL mutation tools via native admission gate); error-taxonomy.md correction added; BC-1.18.010 §Reader Integration section added. NOT RATIFIED per D-1218. |
| 1.1 | 2026-09-12 | architect | Full revision per D-1214 (1st Codex RATIFY-WITH-CHANGES 7 findings + research Q1-Q5). Mechanism changed from Option C (hardened standing allowlist) to Option B (one-time interactive Bash approval at F4, no settings.json change). Option A re-evaluated honestly. Declares explicit narrowly-authorized policy exception (§Decision 8). Activation-manifest authorization mechanism added (§Decision 4). 9 PreToolUse `^Bash$` dispatcher guards enumerated (§Decision 5). Audit trail upgraded to factory-artifacts commit satisfying NIST AU-9 (§Decision 6). Crash-atomicity phase markers, TOCTOU pre-commit guard, dir-fsync mandate added (§Decision 7). Config snapshot bound to ARCH-INDEX revision with activation-time parity check (§Decision 10). BC-1.18.011 Amendments 1-9 and BC-1.18.010 Invariant 2 amendment specified. NOT RATIFIED per D-1216. |
| 1.0 | 2026-09-12 | architect | Initial authoring. Resolves CV-DIR-F2 (impossible execution path). Specifies Bash-tool-with-allowlist as sanctioned invocation path for both mechanism-A (S-25.06) and mechanism-B2 (BC-1.18.011) migrations. NOT RATIFIED per D-1214. |
