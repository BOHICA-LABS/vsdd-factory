---
document_type: architecture-decision-record
adr_id: ADR-052
level: L3
version: "1.17"
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
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md
  - CLAUDE.md
  - .claude/settings.json
  - crates/factory-dispatcher/src/main.rs
  - plugins/vsdd-factory/hooks-registry.toml
  - .factory/cycles/v1.0-brownfield-backfill/adv-local-adr052-pass3.md
input-hash: "02d5adb"
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
| completed.json exists (terminal) | completed.json present | Exit 0 with `ALREADY_MIGRATED`; no lock needed; no manifest needed; **reconcile stale gate first** (§Decision 5c Branch 2 H-2/v1.12 F6 fix: attempt `flock(exclusive.lock, LOCK_EX\|LOCK_NB)`; on acquired: acquire gate LOCK_EX; if gate≠OPEN and no active txn: flip→OPEN; release gate LOCK_EX; release `exclusive.lock`; then exit 0. On EWOULDBLOCK: skip gate reconciliation; exit 0 with `ALREADY_MIGRATED`) |
| txn COMMITTING + valid manifest | CURRENT.json `status:committing` | Recovery: acquire flock; validate completion-only manifest or unexpired original; complete forward recovery |
| txn COMMITTING + manifest absent or expired | CURRENT.json `status:committing` | Abort with `RECOVERY_REQUIRES_REAUTHORIZATION`; human must issue completion-only manifest |
| txn STAGING + `generation_id=null` + valid manifest **(v1.12 F2 — pre-generation crash sub-state; v1.13 MED-3 — ABORTED+retain)** | CURRENT.json absent (pre-pivot, no gen dir exists) | **DISCARD (v1.13 MED-3)**: set txn state → ABORTED (null_generation disposition marker; retained per GC policy until archival — a pre-generation crash IS a failed activation attempt with audit value); flip gate → OPEN; exit `EXPIRY_ABORT`. No canonical paths changed; state fully recoverable under a fresh activation. **MUST NOT** attempt EC-003 census — no staging generation directory exists. **MUST NOT** compare PC1 against null `source_body_row_sha256`. Re-activate from scratch under a fresh manifest. |
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
  1. Acquire `LOCK_SH` flock on `.factory/migration-state/gate-state` file
  2. Read gate_state value
  3. Check for an active txn record: scan `.factory/migration-state/txn-*.json` for state IN
     (STAGING, COMMITTING)
  3.5. **Flock-gated stale-gate reconciliation (v1.7 F4 / v1.10 HIGH-1 / v1.11 HIGH-1 / v1.13 HIGH-1 — extends predicate to null-generation STAGING crash sub-state):** If gate_state ∈ {LOCKED, DRAINING} AND (no active txn (txn absent, COMPLETED, or ABORTED) OR txn=STAGING AND generation_id=null) — REGARDLESS of completed.json presence:
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
        apply §4e F2 DISCARD logic PreToolUse-side — set txn state → ABORTED (null_generation
        disposition marker; retained per GC policy, do NOT delete); write gate_state = OPEN
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
         COMMITTING: gate is LEGITIMATELY locked — do NOT reconcile; proceed to step 5 (block).
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
  4. If gate_state = OPEN AND no active txn: create `reservations/<tool_use_id>.reservation`
     file; release `LOCK_SH`; proceed (admit)
  5. If gate_state ≠ OPEN (DRAINING or LOCKED) OR active txn exists (STAGING or COMMITTING):
     release `LOCK_SH`; return E-MAINTENANCE-001 immediately (block)
  **Invariant (M-1 — dual-check):** Both conditions must be satisfied for admission: gate_state
  MUST be OPEN AND no active txn may exist. `gate_state` is the durable cross-binary proxy that
  persists txn blocking between per-event dispatcher invocations (the gate is set to LOCKED when
  the txn is STAGING or COMMITTING). Checking both guards against the race window between gate
  flip and txn record write. The txn record check is the authoritative "is maintenance in progress"
  signal per §Decision 7a ordinary-writer blocking; gate_state is the fast-path durable proxy.
  The shared lock in step 1 blocks ONLY while the coordinator holds `LOCK_EX` to flip gate state.
  Between gate-state flips, parallel admissions proceed without blocking each other.

- **PostToolUse:** after tool completion (success or failure): remove the reservation file
  `reservations/<tool_use_id>.reservation` created by this invocation's PreToolUse hook, using
  the same `tool_use_id` the harness provides to both hook events for the same tool invocation.
  If the reservation file does not exist (writer crashed between Pre and Post): no-op (normal).

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
   `pending_canonical_moves=[]`). This step now runs strictly AFTER the DRAINING flip in
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
  txn→ABORTED (null_generation disposition; retained) → flips gate→OPEN → admits. Verify
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
  (d) sets txn→ABORTED (null_generation disposition; RETAINED — do NOT delete); writes gate→OPEN;
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
  generation_id=null in the txn record; (2) sets txn state → ABORTED (null_generation disposition
  marker; RETAINED per GC policy — do NOT delete the txn record); (3) flips gate → OPEN;
  (4) exits `EXPIRY_ABORT`. VERIFY it does NOT: (5) attempt EC-003 census (no gen dir exists);
  (6) compare PC1 against null `source_body_row_sha256` (unconditional CONTENT_PRESERVATION_ABORT).
  VERIFY txn record is still present (ABORTED state) after the operation — preserved for audit
  trail per GC policy. Post-recovery: verify a fresh activation with a new manifest proceeds
  normally from drain step 1.

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
       it will self-heal via the next PreToolUse step 3.5. Proceed directly to exit 0 with
       `ALREADY_MIGRATED`.
     - **If acquired (returns 0):** No live coordinator. Proceed to gate reconciliation steps 1–4.
  1. Acquire `LOCK_EX` on gate-state file
  2. Read gate_state value
  3. If gate_state ≠ OPEN AND completed.json present AND no active txn record exists
     (txn absent, or txn state = COMPLETED or ABORTED): write gate_state = OPEN (atomic)
  4. Release `LOCK_EX`; release `exclusive.lock` flock fd (step 0 acquisition)
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
  (b) Verify that Branch 2 with `exclusive.lock` held by a live process logs a warning and
      exits 0 with `ALREADY_MIGRATED` without modifying gate_state.

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
- `intent_log_path`: path to the framed intent log (§Decision 7b)
- `pending_canonical_moves`: list of `{staging_path, canonical_path}` not yet completed
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

The intent log is `.factory/migration-state/intent-<generation_uuid>.log` — a sequence of
framed, checksummed records. Records are written in append-only mode and validated on read.

**Record format:**
```
--- INTENT_LOG_RECORD v1 ---
txn_id: <UUID>
fencing_generation: <integer>
record_type: INTENT | DONE | ABORTED
target_canonical: <path>
staging_path: <generation-path>
expected_post_hash: <sha256-hex>
expected_pre_state: missing | <sha256-hex>
timestamp_utc: <ISO-8601>
record_checksum: <sha256 of all above fields concatenated>
--- END_RECORD ---
```

A **torn record** (truncated, checksum mismatch, missing END_RECORD marker) MUST be treated
as absent — never as a partial INTENT or DONE. Recovery reads from the last valid record.

**Durable ordering (WAL boundary):**
1. Write + sync all staging generation files (fsync per §Decision 7d)
2. Append INTENT record per target to intent log; `fsync(intent_log_fd)`
   **WAL boundary: after this fsync, every rename is recoverable.**
3. `rename(staging_path, canonical_path)` (same filesystem — guaranteed since staging and
   targets are both under `.factory/`)
4. `sync_dir(parent_dir_of_canonical_path)` per §Decision 7d platform branches
5. Append DONE record for target; `fsync(intent_log_fd)`

**Recovery decision table (all cases fail closed except the two explicitly marked safe):**

| Canonical path state | Staging path state | Intent log record | Safe decision |
|---|---|---|---|
| `hash(canonical) == expected_post_hash` | any | INTENT present | **Treat DONE; append DONE record** |
| `hash(canonical) == expected_post_hash` | any | DONE present | Already complete; skip |
| `hash(canonical) == expected_pre_state` | `hash(staging) == expected_post_hash` | INTENT present | **Redo rename; sync dir; append DONE** |
| `canonical` missing, `expected_pre_state = missing` | `hash(staging) == expected_post_hash` | INTENT present | **Redo rename; sync dir; append DONE** |
| `hash(canonical) != expected_post_hash`, no INTENT in log | any | absent | FAIL CLOSED — ambiguous state |
| `hash(canonical) != expected_post_hash` | staging missing | any | FAIL CLOSED — no recovery copy |
| torn/invalid intent log frame | any | any | FAIL CLOSED — no trustworthy authority |
| `hash(canonical)` matches neither expected_post_hash nor expected_pre_state | any | any | FAIL CLOSED |

**Fault injection test mandate:** Tests MUST inject faults between every step:
staging-sync → intent-record-write → rename → dir-sync → done-record-write. Each fault
injection point must verify that recovery converges to the correct terminal state. A second
recovery pass on the already-recovered state must produce identical results (idempotent
forward recovery). Test tooling: `dm-log-writes` (Linux) or equivalent crash-simulation
at the `fsync`/`rename` boundary in test doubles.

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
   if exists else missing`. After all INTENT records: `fsync(intent_log_fd)`.
   **This is the WAL boundary: after this fsync, all renames are recoverable.**

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
   Update txn record: state → COMMITTING. `fsync(txn_record_fd)`.
   **After this rename, the migration is in COMMITTING state. There is no turning back.
   The authorization window is now irrelevant to completion.**

7. **Execute canonical path moves (forward-recoverable via intent log).** For each
   `(staging_path → canonical_path)` target:
   a. `rename(staging_path, canonical_path)` (same filesystem guaranteed)
   b. `sync_dir(parent_dir_of_canonical_path)` per §Decision 7d
   c. Append DONE record to intent log; `fsync(intent_log_fd)`
   d. Update `pending_canonical_moves` in txn record; `fsync(txn_record_fd)`
   If rename or dir-sync fails: DO NOT abort. Record failure; halt further renames; enter
   recovery state. Forward recovery (§Decision 7b) will resume from first uncompleted move.

8. **Write COMPLETED record (terminal — closes F9).** After ALL canonical path moves complete
   and verified (each `hash(canonical_path) == expected_post_hash`):
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

**Status: REFINES/IMPLEMENTS — does not reopen or amend §Decision 4e/5a/7c.** This
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
| `EXPIRY_ABORT` | Activation manifest has expired (timestamp_utc + expires_after_hours < now) OR is absent at STAGING resume; OR a pre-generation crash sub-state (txn=STAGING, generation_id=null) is discarded despite a valid manifest (v1.12 F2 / v1.13 MED-1 third arm) — in all cases txn record state is STAGING (pre-pivot; no canonical paths changed); clean abort (delete staging generation directory if present, set txn → ABORTED, gate → OPEN). **Applied same-burst (v1.10 MED-1):** error-taxonomy.md v1.27 EXPIRY_ABORT trigger widened to "expired OR absent at STAGING resume." **(v1.13 MED-1 PO handoff):** PO must widen error-taxonomy.md EXPIRY_ABORT trigger to add: "OR pre-generation crash sub-state (txn=STAGING, generation_id=null) discarded despite valid manifest (v1.12 F2 / v1.13 HIGH-1 step-3.5 path)." | NON-ERROR termination — no canonical paths changed; re-activation required | exit 1 |
| `FINGERPRINT_MISMATCH_ABORT` | Source files changed between quiescence snapshot and fingerprint recheck at step 5 (sha256 of source != source_sha256 in txn record) | BLOCKED — abort before irreversible step | exit 2 |
| `DRAIN_TIMEOUT_ABORT` | Coordinator waited 30 seconds for active writer reservations to drain to zero; quiescence not achieved | BLOCKED — migration deferred; gate returned to OPEN | exit 2 |
| `ARCH_INDEX_PARITY_ABORT` | Three-way ARCH-INDEX parity check fails: config.arch_index_sha != manifest.approved_arch_index_sha OR != live ARCH-INDEX SHA | BLOCKED — abort before any staging work | exit 2 |
| `COMPLETION_MANIFEST_REJECTION` | Completion-only recovery manifest fails validation: activation_id mismatch, staged_generation_id mismatch, fencing_generation mismatch, or allowed_steps includes pre-pivot steps | BLOCKED — abort recovery; require new completion-only manifest | exit 2 |
| `CENSUS_MISMATCH_ABORT` | Pre-pivot census (step 3b PC2): any `BC-X.YY.NNN` ID from the original-census set appears in zero or more than one staged shard file (ID-set exactly-one-shard violation; detects EC-001 dup+drop even when total count is equal); OR shard-boundary capacity violation (step 3b internal check — distinct from `E-SHD-005`, which is a `HookResult` of the steady-state admission gate in BC-1.18.006/BC-1.18.010, NOT a process exit code of the migration binary). Process exit code. | BLOCKED — abort before pointer swap; txn → ABORTED; gate → OPEN | exit 2 |
| `CONTENT_PRESERVATION_ABORT` | Pre-pivot checks (step 3b): staging-file integrity failure (sha256(staged_file) != expected_post_hash for any staged file, indicating modification after staging sync); OR PC1 structured-equivalence failure (SHA-256 of staged per-BC-row content in canonical BC-ID sort order does not match `source_body_row_sha256` from txn record, indicating row bytes were altered, dropped, or reordered during the split). NOTE: `source_sha256` (whole-file hash) is used ONLY by step 5 fingerprint recheck; PC1 uses the dedicated `source_body_row_sha256` field. Process exit code. | BLOCKED — abort before pointer swap; txn → ABORTED; gate → OPEN | exit 2 |
| `ALREADY_MIGRATED` | `completed.json` exists at `.factory/migration-state/completed.json` | NON-ERROR sentinel — migration previously completed; no action taken | exit 0 |

**Guard-layer admission code** (emitted by the native admission gate in `executor.rs`,
dispatcher hook path — this is a `HookResult` value, NOT a migration binary process exit code):

| Error Code | Trigger | Severity | Exit |
|---|---|---|---|
| `E-MAINTENANCE-001` | Mutation tool call (Edit/Write/MultiEdit/Bash) attempted while gate state is DRAINING or LOCKED, OR while txn record exists with state STAGING or COMMITTING. Emitted by the native admission gate in `executor.rs` (§Decision 5a), NOT by the migration binary. | BLOCKED — writer must retry after migration completes or after PreToolUse step-3.5 self-heals a stuck gate | HookResult (guard layer) |

**Note:** One catalog row per code was added to `error-taxonomy.md` (v1.25). The `exit 0`
for `ALREADY_MIGRATED` is a deliberate sentinel — callers that re-invoke the migration binary
after completion must not treat this as a failure. The `exit 1` for `EXPIRY_ABORT` signals
"no harm done, but re-activation required" (distinct from the `exit 2` hard-block codes).

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

---

## References

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
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md` | Apply §Downstream Amendments 1–9 (v1.3 versions) | product-owner |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md` | Apply §Downstream Invariant 2 amendment + §Reader Integration v1.3 replacement | product-owner |
| `.factory/specs/prd-supplements/error-taxonomy.md` | Apply §Downstream error-taxonomy v1.3 correction | product-owner or technical-writer |
| `.factory/migration-state/exclusive.lock` | Create empty pre-seeded flock inode (never deleted) | devops-engineer (cluster-5 activation preparation) |
| `plugins/vsdd-factory/hooks-registry.toml` | Amend 4 guards per §Decision 5b (txn record state awareness replacing PID-liveness check) | devops-engineer |
| `plugins/vsdd-factory/hooks/destructive-command-guard.sh` (or WASM) | 4-branch classifier per §5c; txn record state check | devops-engineer |
| `plugins/vsdd-factory/hooks/validate-factory-path-staging.sh` (or WASM) | 4-branch classifier; conservative Bash admission; executable digest verification + fd-binding per §Decision 11; **v1.12 F6: Branch 2 gate reconciliation — new step 0: attempt `flock(exclusive.lock, LOCK_EX\|LOCK_NB)` before gate LOCK_EX; on EWOULDBLOCK log warning + exit 0 ALREADY_MIGRATED without gate reconciliation; on acquired proceed with steps 1–4 + release both locks**; **v1.13 HIGH-1: step-3.5 Branch B — null-generation STAGING predicate extension; EWOULDBLOCK path; set txn→ABORTED(null_generation; RETAINED) path; Branch A vs Branch B selection under LOCK_EX re-read** | devops-engineer |
| `plugins/vsdd-factory/hooks/validate-factory-path-staged.sh` (or WASM) | Recognize governed migration subcommands; pass through | devops-engineer |
| `crates/factory-dispatcher/src/executor.rs` | OPEN/DRAINING gate with writer reservations (PreToolUse-acquire/PostToolUse-release); durable reservation dir `.factory/migration-state/reservations/` (H2 v1.4); atomic admission under gate_state flock; every abort path flips gate → OPEN (H1 v1.4); **v1.10 HIGH-1: step-3.5 stale-gate reconciliation generalised — triggers on gate ∈ {LOCKED, DRAINING} AND no active txn (absent/COMPLETED/ABORTED), regardless of completed.json presence; covers (i) abort-crash-before-gate-reset (txn=ABORTED, no completed.json) and (ii) drain-timeout-crash (gate=DRAINING, no txn record) scenarios**; **v1.11 HIGH-1: step-3.5 flock-gated reconciliation — FIRST attempts `flock(exclusive.lock, LOCK_EX\|LOCK_NB)`; reconcile gate→OPEN ONLY if flock ACQUIRABLE (no live coordinator); on EWOULDBLOCK block with E-MAINTENANCE-001 (no gate flip); sub-steps a→h; parity with crash-recovery paragraph; drain reordered (v1.11): initial txn (state=STAGING, source hashes null) written at "step 2.5" BEFORE DRAINING flip (defense-in-depth — `gate=DRAINING + no-txn` eliminated from normal live drains); step 7 is now UPDATE txn with source hashes; step 4 drain-timeout abort now also sets txn→ABORTED**; **v1.12 F2: null-generation crash recovery path — admission reads `generation_id` from txn record on recovery; if null (pre-generation crash sub-state: drain "step 2.5" written but §7c step 1 not yet run): set txn→ABORTED (null_generation disposition; RETAINED per GC policy), flip gate→OPEN, exit EXPIRY_ABORT; MUST NOT call census or compare PC1 against null `source_body_row_sha256`**; **v1.12 F4: txn-selection disambiguation — admission scans all `txn-*.json` and selects by activation_id match to current manifest; fallback: highest created_at; ABORTED record is inert (treated as absent for selection); terminal-txn GC: on migration completion archive ABORTED + COMPLETED records to `.factory/migration-audit/txn-archive/`**; **v1.13 HIGH-1: step-3.5 predicate extended to cover txn=STAGING AND generation_id=null (Branch B) — PreToolUse self-heal for null-generation crash sub-state; on flock acquired (dead coordinator): set txn→ABORTED(null_generation; RETAINED), flip gate→OPEN, admit; on EWOULDBLOCK: block E-MAINTENANCE-001**; **v1.14 DEF-1 (SUPERSEDES the v1.11 ordering cited above): drain reordering REVERSED — the initial txn record is now written at step 3a, strictly AFTER the step-3 DRAINING flip, not before it; this closes an uncovered crash window (`gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead coordinator`) that the v1.11 pre-DRAINING ordering introduced, because that state matched neither the step-3.5 self-heal predicate (requires `gate ∈ {LOCKED, DRAINING}`) nor the ordinary OPEN-admission path (an active txn exists); implementer MUST write the txn record in the step-3a position (after gate_state=DRAINING is durably written and its LOCK_EX released), never before; the flock-gated step-3.5 sub-step a check remains the sole load-bearing writer-exclusion mechanism during the drain, unchanged by this reordering**; crash-recovery gate reconciliation (§Decision 5a Crash-recovery gate reconciliation paragraph — also covers gate=DRAINING per v1.4 H1); stale reservation cleanup with **v1.10 MED-2: operator runbook for abandoned-reservation stall — confirmed-stale reservation files (no live harness session) MAY be manually deleted pre-activation per §Decision 5a operator remediation guidance**; txn record state check blocking mutations | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/src/shard_manager.rs` | Full v1.7 migration implementation: advisory flock; txn record (`source_body_row_sha256` field for PC1); intent log (framed+checksummed); step 3b pre-pivot census gate — staging-integrity (sha256(staged_file) vs intent-log expected_post_hash) + PC1 (structured per-BC-row equivalence: extract BC-X.YY.NNN rows from staged shards, sort by BC-ID, sha256 → compare vs `source_body_row_sha256`; `source_sha256` whole-file hash is the step-5 fingerprint recheck only, NOT PC1) + PC2 (per-ID exactly-one-shard set check; EC-001 dup+drop detection); CURRENT.json pointer swap + generation-first/open-with-ENOENT-fallback reader; completed.json; **v1.12 F2 / v1.13 MED-3: recovery path checks `generation_id` field in txn record — if null (pre-generation crash sub-state): set txn→ABORTED (null_generation disposition; RETAINED per GC policy — do NOT delete), flip gate→OPEN, exit EXPIRY_ABORT; MUST NOT census or compare PC1 against null `source_body_row_sha256`**; **v1.12 F4: txn-selection reads all `txn-*.json`, selects by activation_id match to current manifest (fallback: highest created_at); terminal-txn GC at migration completion archives ABORTED/COMPLETED records to `.factory/migration-audit/txn-archive/`**; platform-branched durability (F_FULLFSYNC on macOS); fd-binding exec on Linux with /proc dependency note; programmatic mtime guard on macOS (M-2: mtime re-stat immediately before exec); TTL-based stale-reservation GC at start of every drain; all abort paths flip gate → OPEN; fencing_generation AUDIT-ONLY | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/tests/` | v1.7 test suite: advisory flock; txn record state machine (`source_body_row_sha256` field); intent log write+recovery+fault-injection; step 3b census gate (staging-integrity sha256 vs intent-log expected_post_hash + PC1 structured per-BC-row equivalence vs `source_body_row_sha256` + PC2 ID-set exactly-one-shard + EC-001 dup+drop + shard-boundary internal capacity check [NOT `E-SHD-005` — that is the steady-state HookResult, not a migration process exit code]); CURRENT.json pointer swap + generation-first/open-with-ENOENT-fallback reader; completed.json permanence; all abort paths → gate OPEN; Branch 2 gate reconciliation (**v1.12 F6: step 0 flock acquire + EWOULDBLOCK warning-and-exit path + fault injection for crash between step 8 and gate→OPEN**); reservation dir cross-process quiescence; TTL-based stale-reservation GC on first-run drain; test that an IN-FLIGHT reservation is NEVER reclaimed by drain step-1 GC (v1.9 H1); **v1.12 F2 / v1.13 MED-3: null-generation crash test (migration-binary path) — crash between drain step 3a (generation_id=null) and §7c step 1 → binary re-invocation: sets txn→ABORTED(null_generation disposition; RETAINED per GC policy — do NOT delete), flips gate→OPEN, exits EXPIRY_ABORT; VERIFY no EC-003 census attempted; VERIFY no PC1 comparison against null source_body_row_sha256; VERIFY txn record still present in ABORTED state after operation**; **v1.12 F4: txn-selection disambiguation test — multiple txn-*.json coexist; recovery selects by activation_id match; ABORTED record inert**; **v1.13 HIGH-1: PreToolUse step-3.5 null-generation self-heal test — gate∈{LOCKED,DRAINING}, txn=STAGING(generation_id=null), no gen dir, flock released; VERIFY step 3.5 matches Branch B → flock acquired → sets txn→ABORTED(null_generation; RETAINED) → gate→OPEN → admits; VERIFY live coordinator (flock held) scenario receives EWOULDBLOCK → E-MAINTENANCE-001 → NO gate flip → NO txn→ABORTED**; **v1.14 DEF-1: drain-reorder regression suite — (a) crash between step 3 (DRAINING flip) and step 3a (txn write) → gate=DRAINING, no txn → Branch A self-heals (unchanged branch, new crash point); (b) construct-and-assert-UNSAT test: no crash-injection point between step 2 (flock acquired) and step-3 completion can produce a durable txn record with state STAGING/COMMITTING while gate_state=OPEN — i.e. `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir` (the DEF-1 defect state) MUST be unconstructible under the v1.14 step ordering; (c) regression test reproducing the exact pre-v1.14 defect scenario (crash immediately after the old pre-DRAINING txn write, before the DRAINING flip) asserts the harness CANNOT reach it because step 3a no longer precedes step 3** | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/tests/darwin_arm64_durability_test.rs` | Empirical darwin-arm64 directory-fsync durability characterization test — **RATIFICATION PREREQUISITE (v1.7 F8)**: this test MUST be completed and reviewed before POLICY 22 ratification of macOS safety. Test validates whether `fsync(dir_fd)` on APFS provides power-loss durability for rename directory entries beyond `F_FULLFSYNC` alone. Results feed §Decision 11 sign-off item 2. | implementer (cluster-5 TDD) — darwin-arm64 CI required; result reviewed at POLICY 22 ratification gate |
| `.factory/activation/factory-dispatcher.sha256` | SHA-256 of built binary | devops-engineer (cluster-5 activation) |
| `.factory/activation/` | Created at F4 by state-manager | state-manager |
| `.factory/migration-state/` | Created by migration binary at runtime; `exclusive.lock` pre-seeded by devops-engineer; `reservations/` subdir created by executor.rs for cross-process writer reservations (H2 v1.4) | migration binary (runtime) |
| `.factory/migration-audit/` | Created by state-manager post-migration | state-manager |

## Changelog

| Version | Date | Author | Change |
|---|---|---|---|
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
