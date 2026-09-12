---
document_type: behavioral-contract
level: L3
version: "1.1"
status: draft
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
input-hash: "26d2920"
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
2. BC-1.18.006's atomic-write primitives (temp-file-then-rename staging/publish discipline) are
   implemented and available for reuse — this BC does not invent new atomic-write machinery.
3. `BC-INDEX.md`'s live frontmatter `total_bcs` field is readable and is treated as an independent
   count-oracle against which the pre-split census (Postcondition 2) is cross-checked — not as the
   census itself (the census is a fresh enumeration of the body's actual `BC-X.YY.NNN` rows,
   `total_bcs` is a sanity bound the fresh enumeration must match).
4. The migration is independently gated on the F4 activation boundary. It has NO timing or
   ordering dependency on BC-1.18.008's mechanism-A backfill-split; the two migrations activate
   independently (each via its own armed-activation manifest per ADR-052 §Decision 4) and may
   run in any order. They share an F4 activation window by operational convenience, not by
   specification.

5. A durable phase-marker file at `.factory/migration-state/migrate-bc-index-state.json` is
   written at each phase transition and survives crashes:
   - PREPARED: all staging writes complete, source fingerprint recorded, all verification
     checks pass — BC-INDEX.md original body untouched.
   - COMMITTED: all atomic file replacements complete; this marker is the single commit-pointer
     for the multi-file atomic operation (ADR-052 §Decision 7).
   - CLEANED: staging temporaries removed.
   EC-003 resume logic reads this marker to determine the correct entry point without
   re-running the full split from scratch.

6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution. The
   migration binary acquires an exclusive advisory lock file at
   `.factory/migration-state/exclusive.lock` before any read of source files. Ordinary governed
   writers (Edit/Write tool calls validated by the `validate-factory-path-staging` dispatcher
   guard) check for this lock file and fail with E-MAINTENANCE-001 before writing to BC-INDEX
   paths during a migration window. The lock is released only after the COMMITTED phase marker
   is written (success) or on migration abort (cleanup). The `validate-factory-path-staging`
   guard amendment (ADR-052 §Decision 5) implements the lock-check side of this invariant.

## Postconditions

1. **Content-preservation, byte-for-byte.** The concatenation of the ten (or more, once
   second-level sub-shards exist) resulting shard files, in `SS-01`..`SS-10` order, plus
   `BC-INDEX.md`'s own retained lean top-level body (`§Summary` + `§Subsystem Shard Manifest` +
   cross-cutting invariants — BC-1.18.010 Postcondition 1's end-state), reproduces the ORIGINAL
   (pre-split) `BC-INDEX.md`'s full per-BC-row content byte-for-byte — modulo the newly-introduced
   `§Subsystem Shard Manifest` section itself, which is new structural metadata, not migrated
   content. This is BC-1.18.008 Postcondition 6(a)'s exact analogue, applied to a content
   partition (by subsystem) instead of a time partition (by seal sequence).

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
   first place.

3. **Crash-atomicity: staging + verify + atomic replace, all-or-nothing.** Write all ten (or more)
   resulting shard files and the shard-manifest TOML to a staging location first; only after
   Postcondition 1 (content-preservation) and Postcondition 2 (independent census) both verify
   clean does the operation atomically replace `BC-INDEX.md`'s body and publish the shard-manifest
   at its canonical path, via the same temp-file-then-rename discipline BC-1.18.006 already
   establishes. This is BC-1.18.008 Postcondition 5's exact analogue. Each atomic file replacement
   (rename(2) call) MUST be followed by fsync on the parent directory of the target file before
   proceeding to the next replacement, ensuring directory entries survive a system crash (POSIX
   rename(2) + dir-fsync semantics per Pillai et al. OSDI'14). The dir-fsync is mandatory, not
   best-effort.

3a. **Pre-commit source-fingerprint recheck (TOCTOU guard).** Immediately before the
    atomic-replace step (PREPARED → COMMITTED transition), the migration binary re-verifies
    that BC-INDEX.md's source content has not changed since the census (Postcondition 2). It
    re-computes SHA-256 of the source and compares against the fingerprint recorded at census
    time. If they differ, the migration ABORTS: Postcondition 4's rollback applies, the
    PREPARED marker is deleted, and BC-INDEX.md's original body is left untouched. This closes
    the TOCTOU window between census and write introduced by concurrent governed writers or
    human edits.

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
   operation, not a separate follow-on.** Both subsystems already exceed the provisional cap on
   their own section size alone (SS-05 ~88,695 bytes / 661 BCs; SS-06 ~85,407 bytes / 592 BCs,
   both measured 2026-09-05) and require immediate second-level sub-sharding at F4 activation,
   as part of the same one-time B2 migration operation (independently of mechanism A's activation
   schedule). This BC's content-preservation,
   independent-census, atomicity, and rollback obligations (Postconditions 1-5 above) apply
   IDENTICALLY at the sub-shard level for SS-05/SS-06 — i.e., the census for SS-05 verifies every
   `BC-5.YY.NNN` row lands in exactly one of `shards/BC-INDEX-SS-05.a.md`/`.b.md`/etc., with the
   SS-05-scoped total matching an independent pre-split count of `BC-5.*` rows specifically.

7. **No new Cohort-B dependency.** Unlike BC-1.18.008 (which BC-7.08.001's fail-closed flip depends
   on, since `regression-gate`/`convergence-tracker` read the four mechanism-A artifacts), this
   migration has NO Cohort-B sequencing dependency: the F2 architecture-delta doc's §5
   migration-impact map confirms `regression-gate`/`convergence-tracker` do not read `BC-INDEX.md`.
   `BC-7.08.001`'s scope and gating conditions are UNCHANGED by this BC. Note: this postcondition
   governs B2/Cohort-B independence only. A/B2 scheduling independence (that mechanism A and B2
   activate independently at F4) is governed by Precondition 4 [as amended by ADR-052 v1.1].

8. **Relationships.** This BC depends on BC-1.18.010 (the end-state addressing scheme this
   migration produces) and BC-1.18.006 (reuses its atomic-write primitives) — the same "applies an
   existing primitive retroactively, once" relationship BC-1.18.008 has to BC-1.18.006, mirrored
   here for B2's own end-state BC.

## Invariants

1. **This BC's migration logic is a caller of BC-1.18.006's atomic-write primitives, not a
   reimplementation.** The migration reuses the SAME temp-file-then-rename atomic-write pattern
   BC-1.18.006 defines for the ongoing per-write case — this BC differs only in WHEN it runs (once,
   at F4 activation) and WHAT it operates on (BC-INDEX's existing body, partitioned by subsystem,
   rather than a single new append).

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
   guarantee is implemented via the single COMMITTED phase marker (Precondition 5 [as amended]):
   before COMMITTED exists, the state is PREPARED (original body untouched, staging in progress) or
   CLEAN (no migration in progress); after COMMITTED is written, the state is the split end-state.
   The COMMITTED marker is the sole commit-point for the multi-file atomic operation; composing N
   independent `write_atomic` calls without this commit-pointer does not satisfy this invariant.

4. **This BC's own execution does NOT gate BC-7.08.001's Cohort B flip.** Per Postcondition 7, no
   implementation may introduce an undocumented sequencing dependency between this migration and
   the Cohort B fail-closed flip; the two are independent one-time operations that happen to be
   scheduled at the same F4 activation boundary.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Content-preservation (Postcondition 1) passes but independent census (Postcondition 2) finds a duplicated row across two shard files | Migration ABORTS per Postcondition 4 — passing ONE check is not sufficient; both are independently mandatory (Invariant 2) |
| EC-002 | Migration crashes after writing 6 of 10 first-level shard files to staging | Postcondition 3's atomicity guarantee: `BC-INDEX.md`'s original body is untouched (staging was incomplete and never promoted); the partial staged output is discarded on the next attempt, which restarts cleanly |
| EC-003 | A prior migration attempt left a complete, verified shard set in staging but crashed before the atomic-replace step | Re-running MUST detect the verified-complete staged state and resume directly to the atomic-replace step, not re-run the full split from scratch (Postcondition 5's idempotency, resume-from-verified-checkpoint variant) |
| EC-004 | SS-05's second-level sub-split (Postcondition 6) produces sub-shards `.a`/`.b`/`.c` whose combined row count does not match an independent pre-split count of `BC-5.*` rows | Migration ABORTS for the entire operation (not just SS-05) per Postcondition 4 — a sub-shard-level census failure is treated with the same severity as a top-level census failure, since a partial-success outcome (nine subsystems split correctly, SS-05 corrupted) would still violate Invariant 3's all-or-nothing guarantee |
| EC-005 | An implementer mistakenly makes this migration a precondition for BC-7.08.001's Cohort B flip | Scope violation of Postcondition 7/Invariant 4 — the F2 architecture-delta doc's migration-impact map already confirms zero dependency; this BC introduces none |
| EC-006 | The migration is re-run after already completing successfully (no partial state, fully migrated) | Idempotent no-op: the migration detects `BC-INDEX.md`'s body is already in the split end-state (zero per-BC rows remain in the body, per BC-1.18.010 Invariant 3) and exits without re-splitting or re-writing any shard file |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `BC-INDEX.md` at 2,005 total BCs across 10 `### SS-NN` sections, `total_bcs: 2005` in frontmatter | Pre-split census enumerates exactly 2,005 unique `BC-X.YY.NNN` IDs, matching `total_bcs`; post-split, the union of all 10 (or more, with SS-05/SS-06 sub-shards) shard files' row counts is exactly 2,005; `BC-INDEX.md`'s body retains zero per-BC rows | happy-path |
| SS-05 (661 BCs, ~88,695 bytes) and SS-06 (592 BCs, ~85,407 bytes) both exceed the provisional cap | Both receive second-level sub-splits (e.g. SS-05 → `.a`/`.b`/`.c`) in the SAME migration operation; SS-05's sub-shard row counts sum to exactly 661, SS-06's to exactly 592 | happy-path |
| Content-preservation check finds a byte mismatch (one row's trailing whitespace altered during extraction) | Migration ABORTS; original `BC-INDEX.md` body untouched; fail-loud error surfaced (E-SHD-005) | error |
| Independent census finds a `BC-3.14.002` row present in BOTH `shards/BC-INDEX-SS-03.md` and (erroneously) `shards/BC-INDEX-SS-04.md` | Migration ABORTS per Postcondition 4/EC-001; fail-loud error surfaced (E-SHD-005) naming the duplicated ID | error |
| Migration crashes mid-staging, restarted from scratch | Original `BC-INDEX.md` byte-identical to pre-crash state; restart produces the same split result as an uninterrupted run | error |
| Migration re-run after a prior successful completion | No-op: zero shard files rewritten, `BC-INDEX.md` body unchanged (EC-006) | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-132 | Content-preservation invariant — concatenation of all resulting first-level (and, where applicable, second-level) shard files in `SS-01`..`SS-10` order plus the retained lean top-level body reproduces the original monolithic `BC-INDEX.md` body byte-for-byte, modulo the new `§Subsystem Shard Manifest` section | proptest / golden-file round-trip against the live (or a synthetic fixture) `BC-INDEX.md` body |
| VP-133 | Independent-census integrity invariant — every `BC-X.YY.NNN` ID in the pre-split census appears in EXACTLY ONE post-split shard (or sub-shard) file; the union of all shard row counts equals the pre-split census count; `BC-INDEX.md`'s post-split body contains zero per-BC rows | integration test (full-corpus census comparison against synthetic fixtures with known BC-ID sets, including a duplicated-row negative-control fixture) |
| VP-133 | Atomicity-under-interruption invariant — a simulated crash at any staging step leaves `BC-INDEX.md`'s body either fully original or fully split, never a partial/corrupt intermediate state | fault-injection / integration test (simulated crash at each of N staging steps; assert post-recovery state is one of the two valid states) |
| VP-133 | Idempotency invariant — running the migration twice against an already-split `BC-INDEX.md`, or resuming from a verified-complete staged state, does not re-split, re-duplicate, or corrupt any shard | integration test (double-invocation + resume-from-staged-checkpoint fixtures) |
| VP-133 | SS-05/SS-06 second-level sub-split coverage invariant — the same content-preservation/census/atomicity/rollback obligations hold at the sub-shard level for SS-05 and SS-06 specifically, verified against an independent `BC-5.*`/`BC-6.*`-scoped count | integration test (sub-shard-scoped census comparison for SS-05/SS-06 fixtures) |
| VP-134 | No-new-Cohort-B-dependency invariant — this BC's migration completion is never referenced as a precondition in `hooks-registry.toml`'s `failure_policy` deployment sequencing for `regression-gate`/`convergence-tracker` | static-check (config/PR-template audit confirming BC-7.08.001's gating conditions cite only BC-1.18.005/006/008, never this BC) |

VP IDs allocated by formal-verifier (S-25.02 F2 verification-property fix-burst; VP-INDEX v3.03):
**VP-132** (proptest; content-preservation byte-for-byte), **VP-133** (integration; independent-census
integrity + crash-atomicity + fail-loud rollback E-SHD-005 + idempotency + SS-05/SS-06 sub-split
census — the four same-method safety obligations consolidated per the single-method-per-VP convention,
mirroring VP-124), and **VP-134** (static-check; no-new-Cohort-B-dependency). This is the B2 analogue
of VP-123/VP-124 (BC-1.18.008's content-preservation + atomicity/idempotency pair) but keyed to
BC-INDEX's ID-census model instead of decision-log's byte-count model. Traceability-reference
completion only (VP-side of the trace); no BC body/postcondition/version change.

## Related BCs

- BC-1.18.008 — this BC's structure is modeled directly on BC-1.18.008's governed one-time-migration pattern (content-preservation, census, atomicity, rollback, idempotency), substituting a content partition (by subsystem) for a time partition (by seal sequence) (related to)
- BC-1.18.010 — this BC governs the ONE-TIME transition to BC-1.18.010's end-state addressing scheme; BC-1.18.010 specifies the end-state, this BC specifies the transition (depends on)
- BC-1.18.006 — this BC reuses BC-1.18.006's atomic-write primitives (staging + verify + atomic replace) (depends on)
- BC-1.18.005 — the shard-cap formula that determines whether SS-05/SS-06 (and, empirically, any other subsystem) require second-level sub-sharding (related to)
- BC-7.08.001 — explicitly has NO dependency on this BC (Postcondition 7); cited here only to document the absence of a relationship an implementer might otherwise assume by analogy to BC-1.18.008 (related to)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — one-time migration entry point for the B2 body split, reusing BC-1.18.006's staging/atomic-replace primitives
- `.factory/specs/behavioral-contracts/BC-INDEX.md` §Summary / `total_bcs` frontmatter field — the independent count-oracle this BC's census check (Postcondition 2) cross-checks against
- `.factory/specs/architecture/ARCH-INDEX.md` §Subsystem Registry — the `BC-S Prefix`→`SS-NN` mapping this BC's per-subsystem partition boundaries follow (same mapping BC-1.18.010 Postcondition 2 reuses)
- ADR-052 §Decision 4 — armed-activation manifest governing pre-mutation authorization (Precondition 4 [as amended])
- ADR-052 §Decision 7 — crash-atomicity phase markers (PREPARED/COMMITTED/CLEANED) (Precondition 5 [as amended])
- ADR-052 §Decision 8 — POLICY 22 exception declaration with enumerated skipped controls

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

```
$ grep -oE "^pub fn write_indeterminate_marker" crates/factory-dispatcher/src/indeterminate_marker.rs
pub fn write_indeterminate_marker
```

Confirms both existing atomic-write primitives (`write_atomic`, `write_indeterminate_marker`) this
BC's Invariant 1 states are reused, not reimplemented, for the staging+verify+atomic-replace
sequence (Postcondition 3).

```
$ grep -oE "^total_bcs: [0-9]+" .factory/specs/behavioral-contracts/BC-INDEX.md
total_bcs: 2005
```

Confirms the live `total_bcs` frontmatter field's current value, grounding this BC's Postcondition
2 independent-count-oracle claim (the exact figure is expected to change as BC-INDEX grows further
before F4 execution — this BC's own authoring-time snapshot, not a literal migration input, mirrors
BC-1.18.008 Precondition 3's identical illustrative-scale caveat).

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

- VP-132, VP-133, VP-134 — allocated by formal-verifier (S-25.02 F2 verification-property fix-burst; VP-INDEX v3.03), analogous to VP-123/VP-124 (content-preservation + record-integrity; atomicity-under-interruption + idempotency) but keyed to BC-INDEX's ID-census model instead of decision-log's byte-count model, per the F2 architecture-delta doc §4a authorship input for this BC. VP-132 (proptest; content-preservation byte-for-byte), VP-133 (integration; independent-census integrity + crash-atomicity + fail-loud rollback E-SHD-005 + idempotency + SS-05/SS-06 second-level sub-split census — four same-method obligations consolidated per the single-method-per-VP convention), VP-134 (static-check; no-new-Cohort-B-dependency). The six candidate properties enumerated in `## Verification Properties` above map to these three VPs: candidate 1 → VP-132; candidates 2/3/4/5 → VP-133; candidate 6 → VP-134.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | Anchoring to CAP-043: "Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding" — because this BC describes the governed one-time migration that establishes mechanism B2's split end-state (BC-1.18.010) correctly and safely for `BC-INDEX.md`, which is exactly what CAP-043 defines per `capabilities.md` §CAP-043: "This capability has two mechanisms for two artifact shapes: mechanism A shards four append-only cycle logs... mechanism B shards `BC-INDEX.md`... via two sub-mechanisms: B1... and B2 splits the file's ten already-existing `### SS-NN` per-subsystem body sections into individually-addressable shard files." CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — no existing capability other than CAP-043 covers a governed one-time migration establishing B2's split end-state; CAP-041 (INDETERMINATE detection/quarantine) and CAP-042 (the `rotate_changelog`/`last_amended` write-path fix) are both distinguishable per capabilities.md's own CAP-043 entry, and neither covers a BC-INDEX body-structure migration. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with the sibling BC-1.18.005–010 precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` one-time B2 migration logic) |
| ADR | ADR-051 §Decision 10 (governed one-time migration for the B2 BC-INDEX body split, fix-burst addition F-S2502-F2-002); §Decision 7 (B2 end-state design this migration produces); §Decision 8 (shard-manifest schema this migration publishes) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution fix-burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.1 | 2026-09-12 | product-owner | ADR-052 v1.1 hardening (9 amendments): Amendment 1 — removed A/B2 simultaneous-activation coupling from Precondition 4 ("at the SAME moment BC-1.18.008 runs" language deleted; each migration independently gated via its own armed-activation manifest per ADR-052 §Decision 4). Amendment 2 — Postcondition 6 last sentence "at the SAME F4 activation moment mechanism A's own backfill (BC-1.18.008) runs" replaced with "at F4 activation, as part of the same one-time B2 migration operation (independently of mechanism A's activation schedule)" (SS-05/SS-06 B2-internal atomicity preserved; only A/B2 simultaneous-activation coupling removed). Amendment 3 — appended Postcondition 7 scope clarification ("governs B2/Cohort-B independence only; A/B2 scheduling independence governed by Precondition 4 as amended"). Amendment 4 — new Precondition 5: durable phase-marker file at `.factory/migration-state/migrate-bc-index-state.json` (PREPARED/COMMITTED/CLEANED transitions, EC-003 resume logic). Amendment 5 — new Precondition 6: writer-exclusion advisory lock at `.factory/migration-state/exclusive.lock`, governed writers fail E-MAINTENANCE-001 during migration window, lock released on COMMITTED or abort (ADR-052 §Decision 5 guard). Amendment 6 — appended mandatory dir-fsync mandate to Postcondition 3 (each rename(2) followed by fsync on parent directory; mandatory, not best-effort; POSIX rename(2) + Pillai et al. OSDI'14). Amendment 7 — new Postcondition 3a: TOCTOU pre-commit source-fingerprint recheck immediately before PREPARED→COMMITTED transition (SHA-256 recompute vs. census-time fingerprint; abort + rollback + delete PREPARED marker on mismatch). Amendment 8 — appended COMMITTED-marker commit-pointer specification to Invariant 3 (sole commit-point for the multi-file atomic operation; N independent write_atomic calls without it do not satisfy all-or-nothing). Amendment 9 — added ADR-052 §Decision 4/7/8 to Architecture Anchors. ADR-052 added to inputs. No existing guarantee weakened. |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, fix-burst addition per F-S2502-F2-002 HIGH, ADR-051 v1.1 Decision 10). Allocated as BC-1.18.011 — confirmed as the next free SS-01 slot against the live `ss-01/` directory (BC-1.18.001–010 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism B2's BC-INDEX body split: byte-for-byte content-preservation, independent-census integrity (every BC row in exactly one shard, cross-checked against `total_bcs`), staging+verify+atomic-replace crash-atomicity, fail-loud rollback on verification failure, idempotency against a partial prior attempt, and the SS-05/SS-06 second-level sub-split covered within the SAME one-time operation. Explicitly confirmed NO new Cohort-B sequencing dependency (unlike BC-1.18.008). Modeled directly on BC-1.18.008's structure per the F2 architecture-delta doc §4a authorship input. CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier per the established project convention. ADR-051 §D10/§D7/§D8 citations. |
