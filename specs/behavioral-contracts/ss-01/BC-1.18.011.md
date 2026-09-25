---
document_type: behavioral-contract
level: L3
version: "1.10"
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
  - .factory/specs/behavioral-contracts/BC-INDEX.md
input-hash: "15bedba"
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
       both deliveries are activated.
   (c) OPEN/DRAINING gate with writer reservations spanning PreToolUse→tool-completion ensures
       the migration coordinator waits for all in-flight admitted writers to complete before
       snapshotting source files (ADR-052 §Decision 5a).

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
     migration on this condition. A non-blocking `tracing::warn!` is logged so the anomaly remains
     visible. This is a bounded case, not an unbounded hazard: BC-1.18.005 Postcondition 6's
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

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Content-preservation (Postcondition 1) passes but independent census (Postcondition 2) finds a duplicated row across two shard files | Migration ABORTS per Postcondition 4 — passing ONE check is not sufficient; both are independently mandatory (Invariant 2) |
| EC-002 | Migration crashes after writing 6 of 10 first-level shard files to staging | Postcondition 3's atomicity guarantee: `BC-INDEX.md`'s original body is untouched (staging was incomplete and never promoted); the partial staged output is discarded on the next attempt, which restarts cleanly |
| EC-003 | A prior migration attempt left a complete, verified shard set in staging but crashed before the atomic-replace step | Re-running MUST detect the verified-complete staged state (txn record state = STAGING) and resume toward the pointer swap. **§Decision 7c step 3b (H3 fix):** resume-from-STAGING MUST re-run the full census (ADR-052 §Decision 7c step 3b, referenced by §Decision 4e) before proceeding to the pointer swap — the census gate is not skippable on resume even when the staged generation was previously verified complete. The census re-run uses the existing staged generation files; it does NOT re-run the full split from scratch (Postcondition 5's idempotency). |
| EC-004 | SS-05's second-level sub-split (Postcondition 6) produces sub-shards `.a`/`.b`/`.c` whose combined row count does not match an independent pre-split count of `BC-5.*` rows | Migration ABORTS for the entire operation (not just SS-05) per Postcondition 4 — a sub-shard-level census failure is treated with the same severity as a top-level census failure, since a partial-success outcome (nine subsystems split correctly, SS-05 corrupted) would still violate Invariant 3's all-or-nothing guarantee |
| EC-005 | An implementer mistakenly makes this migration a precondition for BC-7.08.001's Cohort B flip | Scope violation of Postcondition 7/Invariant 4 — the F2 architecture-delta doc's migration-impact map already confirms zero dependency; this BC introduces none |
| EC-006 | The migration is re-run after already completing successfully (no partial state, fully migrated) | Idempotent no-op: the migration detects `BC-INDEX.md`'s body is already in the split end-state (zero per-BC rows remain in the body, per BC-1.18.010 Invariant 3) and exits without re-splitting or re-writing any shard file |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `BC-INDEX.md` at 2,005 total BCs across 10 `### SS-NN` sections, `total_bcs: 2005` in frontmatter | Pre-split census enumerates exactly 2,005 unique `BC-X.YY.NNN` IDs, matching `total_bcs`; post-split, the union of all 10 (or more, with SS-05/SS-06 sub-shards) shard files' row counts is exactly 2,005; `BC-INDEX.md`'s body retains zero per-BC rows | happy-path |
| SS-05 (661 BCs, ~88,695 bytes) and SS-06 (592 BCs, ~85,407 bytes) both exceed the provisional cap | Both receive second-level sub-splits (e.g. SS-05 → `.a`/`.b`/`.c`) in the SAME migration operation; SS-05's sub-shard row counts sum to exactly 661, SS-06's to exactly 592 | happy-path |
| Content-preservation check finds a byte mismatch (one row's trailing whitespace altered during extraction) | Migration ABORTS; original `BC-INDEX.md` body untouched; fail-loud CONTENT_PRESERVATION_ABORT (process exit code) | error |
| Independent census finds a `BC-3.14.002` row present in BOTH `shards/BC-INDEX-SS-03.md` and (erroneously) `shards/BC-INDEX-SS-04.md` | Migration ABORTS per Postcondition 4/EC-001; fail-loud CENSUS_MISMATCH_ABORT (process exit code) naming the duplicated ID | error |
| Migration crashes mid-staging, restarted from scratch | Original `BC-INDEX.md` byte-identical to pre-crash state; restart produces the same split result as an uninterrupted run | error |
| Migration re-run after a prior successful completion | No-op: zero shard files rewritten, `BC-INDEX.md` body unchanged (EC-006) | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-132 | Content-preservation invariant — BC-X.YY.NNN table rows extracted from all staged shard files and sorted in canonical BC-ID order produce a SHA-256 matching `source_body_row_sha256` from the txn record (the SHA-256 of the per-BC-row content from the original pre-split BC-INDEX.md body in canonical BC-ID sort order, excluding §Summary, §Subsystem Shard Manifest, cross-cutting invariants, and non-row separator lines); `source_sha256` (whole-file fingerprint) is used ONLY by step 5 fingerprint recheck, not by PC1 | proptest / golden-file round-trip against the live (or a synthetic fixture) `BC-INDEX.md` body |
| VP-133 | Independent-census integrity invariant — every `BC-X.YY.NNN` ID in the pre-split census appears in EXACTLY ONE post-split shard (or sub-shard) file; the union of all shard row counts equals the pre-split census count; `BC-INDEX.md`'s post-split body contains zero per-BC rows | integration test (full-corpus census comparison against synthetic fixtures with known BC-ID sets, including a duplicated-row negative-control fixture) |
| VP-133 | Atomicity-under-interruption invariant — a simulated crash at any staging step leaves `BC-INDEX.md`'s body either fully original or fully split, never a partial/corrupt intermediate state | fault-injection / integration test (simulated crash at each of N staging steps; assert post-recovery state is one of the two valid states) |
| VP-133 | Idempotency invariant — running the migration twice against an already-split `BC-INDEX.md`, or resuming from a verified-complete staged state, does not re-split, re-duplicate, or corrupt any shard | integration test (double-invocation + resume-from-staged-checkpoint fixtures) |
| VP-133 | SS-05/SS-06 second-level sub-split coverage invariant — the same content-preservation/census/atomicity/rollback obligations hold at the sub-shard level for SS-05 and SS-06 specifically, verified against an independent `BC-5.*`/`BC-6.*`-scoped count | integration test (sub-shard-scoped census comparison for SS-05/SS-06 fixtures) |
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

## Related BCs

- BC-1.18.008 — this BC's structure is modeled directly on BC-1.18.008's governed one-time-migration pattern (content-preservation, census, atomicity, rollback, idempotency), substituting a content partition (by subsystem) for a time partition (by seal sequence) (related to)
- BC-1.18.010 — this BC governs the ONE-TIME transition to BC-1.18.010's end-state addressing scheme; BC-1.18.010 specifies the end-state, this BC specifies the transition (depends on)
- BC-1.18.006 — this BC reuses BC-1.18.006's atomic-write primitives (staging + verify + atomic replace) (depends on)
- BC-1.18.005 — the shard-cap formula that determines whether SS-05/SS-06 (and, empirically, any other subsystem) require second-level sub-sharding (related to)
- BC-7.08.001 — explicitly has NO dependency on this BC (Postcondition 7); cited here only to document the absence of a relationship an implementer might otherwise assume by analogy to BC-1.18.008 (related to)

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
- VP-142 — allocated by formal-verifier per ADR-051 §Decision 18's authoring instruction (S-25.02-b2-sharding cluster-5 spec-closure chain; human-approved 2026-09-22 design proposal). Hosted on THIS BC's Postcondition 6 (proptest; chunk-boundary determinism and correctness for `chunk_subsystem_rows_into_sub_shards`), cross-referenced from BC-1.18.010 Postcondition 4 since the property holds identically for the one-time migration and the steady-state rebuild path.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | Anchoring to CAP-043: "Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding" — because this BC describes the governed one-time migration that establishes mechanism B2's split end-state (BC-1.18.010) correctly and safely for `BC-INDEX.md`, which is exactly what CAP-043 defines per `capabilities.md` §CAP-043: "This capability has two mechanisms for two artifact shapes: mechanism A shards four append-only cycle logs... mechanism B shards `BC-INDEX.md`... via two sub-mechanisms: B1... and B2 splits the file's ten already-existing `### SS-NN` per-subsystem body sections into individually-addressable shard files." CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — no existing capability other than CAP-043 covers a governed one-time migration establishing B2's split end-state; CAP-041 (INDETERMINATE detection/quarantine) and CAP-042 (the `rotate_changelog`/`last_amended` write-path fix) are both distinguishable per capabilities.md's own CAP-043 entry, and neither covers a BC-INDEX body-structure migration. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with the sibling BC-1.18.005–010 precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` one-time B2 migration logic) |
| ADR | ADR-051 §Decision 10 (governed one-time migration for the B2 BC-INDEX body split, fix-burst addition F-S2502-F2-002); ADR-051 §Decision 7 (B2 end-state design this migration produces); ADR-051 §Decision 8 (shard-manifest schema this migration publishes); ADR-052 §Decision 4 (armed-activation manifest governing pre-mutation authorization); ADR-052 §Decision 5a (native admission gate: OPEN/DRAINING gate with writer reservations; txn state check blocks ordinary writers regardless of PID liveness); ADR-052 §Decision 7a (advisory flock on stable never-unlinked inode; durable txn record separate from lock file); ADR-052 §Decision 7b (framed checksummed intent log + WAL boundary + matching-destination-hash recovery decision table); ADR-052 §Decision 7c (single atomic CURRENT.json pointer swap + completed.json permanent terminal record + generation-first/canonical-fallback reader protocol); ADR-052 §Decision 8 (POLICY 22 exception declaration with enumerated allowed write targets) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution fix-burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
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
