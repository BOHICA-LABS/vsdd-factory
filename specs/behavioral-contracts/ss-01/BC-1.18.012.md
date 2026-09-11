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
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.009.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md
  - .factory/specs/behavioral-contracts/BC-INDEX.md
input-hash: "0f45b4c"
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

# BC-1.18.012: Governed One-Time B1 Changelog Backfill Migration (Cold-Start `changelog:` Array Trim via `rotate_changelog`)

## Description

BC-1.18.009 specifies mechanism B1's ONGOING, steady-state per-write rotation contract, but that
contract's own read-cost claim (BC-1.18.005 Postcondition 8) is true only AFTER at least one
rotation has occurred. `BC-INDEX.md`'s `changelog:` sequence has never been rotated and, as of
2026-09-05, holds approximately 1,997 items across 177,305 bytes of frontmatter — roughly 40x the
illustrative `N≈50` retention target. This BC governs the ONE-TIME migration that trims this
cold-start excess via the SAME `rotate_changelog` primitive BC-1.18.009 uses ongoing, wrapped in
independent-census verification, content-preservation confirmation, and fail-loud rollback —
modeled directly on BC-1.18.008's governed one-time-migration structure, substituting a
`changelog:`-item-count partition for a byte-count partition. Without this BC, B1's cold-start
excess would be displaced as an incidental side effect of whichever ordinary agent write happens to
be first after F4 activation — an unplanned, unverified migration path, unlike BC-1.18.008's
(mechanism A) and BC-1.18.011's (mechanism B2) governed one-time migrations.

## Preconditions

1. BC-1.18.009's ongoing per-write B1 rotation contract (rotate/trim via `rotate_changelog`, block-
   and-retry, single-actor prepend) is fully specified and available as the STEADY-STATE target
   this migration establishes the precondition for.
2. `rotate_changelog` (`crates/last-amended-migrate/src/rotate.rs`) is available as a library
   function callable from the migration's entry point, reached via the SAME generalized,
   explicit-`archive_path`-parameterized call surface BC-1.18.009 Postcondition 2 requires (the
   bounded, additive extension to `resolve_archive_path`, not yet implemented — this BC does not
   introduce a SECOND path-resolution extension; it reuses BC-1.18.009's).
3. `BC-INDEX.md`'s live frontmatter `changelog:` sequence is readable and its item count is
   treated as an independent count-oracle for the pre-migration census (Postcondition 3) — a fresh
   enumeration, not reused from any cached count.
4. The migration is scheduled to execute at F4 activation, BEFORE BC-1.18.009's per-write gate is
   treated as steady-state-bounded (this BC's successful completion is what MAKES BC-1.18.005
   Postcondition 8's steady-state characterization true) — though it has no ordering dependency on
   BC-1.18.008's mechanism-A backfill-split or BC-1.18.011's mechanism-B2 migration (independent
   one-time operations scheduled at the same F4 activation boundary, per Postcondition 6 below).

## Postconditions

1. **Executes exactly once, at F4 activation, before the ongoing per-write B1 gate is treated as
   steady-state-bounded.** This BC is distinct from BC-1.18.009's per-write rotation: BC-1.18.009
   fires reactively on every future over-N-item write; this BC fires proactively, once, against
   `BC-INDEX.md`'s PRE-EXISTING `changelog:` sequence at the moment Layer 2's B1 gate goes live.
   This BC's successful completion is what MAKES BC-1.18.005 Postcondition 8's steady-state
   "bounded" characterization true; that characterization is false as a description of the COLD
   state, which this BC exists to eliminate.

2. **Uses the SAME `rotate_changelog` primitive as BC-1.18.009's ongoing rotation, via the SAME
   generalized `archive_path`-parameterized call surface — `keep_recent = low_water_mark`
   (**CORRECTED, fix-burst pass-3, F-P3-005, ADR-051 v1.3 Decision 14 — NEVER `keep_recent = N`**;
   `low_water_mark` is BC-1.18.005 Postcondition 8's rotation-target config, the SAME config value
   BC-1.18.009 Postcondition 2 uses for its ongoing per-write rotation) — no new rotation logic,
   only a governed ONE-TIME CALLER with pre/post verification wrapped around it.** This mirrors
   BC-1.18.008's exact relationship to BC-1.18.006's primitives: apply an existing, already-shipped
   mechanism retroactively, once, rather than inventing new migration machinery. **Why
   `low_water_mark`, not `N` (F-P3-005):** migrating to `N` would leave the live sequence AT the
   trigger boundary immediately after go-live, and the very FIRST ordinary post-migration write
   would immediately re-trigger rotation — reproducing the exact steady-state-block-on-every-write
   pathology `low_water_mark` exists to eliminate, from write #1 of the new steady state. Both this
   one-time migration and BC-1.18.009's ongoing per-write gate MUST target the SAME
   `low_water_mark` for the corrected steady state to hold from the first post-migration write
   onward. The overflow tail (≈1,972 items at today's measured count, `N=50`/`low_water_mark=25`
   illustrative) is appended to the SAME single evergreen archive file
   BC-1.18.009 Postcondition 2 establishes (`.factory/specs/behavioral-contracts/
   BC-INDEX-changelog-archive.md`) — never a separate migration-specific archive location.

3. **Independent-census integrity check.** Before invoking `rotate_changelog`, capture the exact
   pre-migration `changelog:` item count (a fresh enumeration, not reused from any cached count —
   e.g., BC-INDEX's own `total_bcs` field is a DIFFERENT count-oracle, for BC rows, not changelog
   items, so it cannot substitute here). After `rotate_changelog` completes, verify:
   `(items retained in the live frontmatter) + (items appended to the archive) ==
   (pre-migration count)` exactly. This is BC-1.18.008 Postcondition 6(b)'s exact analogue
   (record-integrity), applied to `changelog:` items instead of decision-log rows, and BC-1.18.011
   Postcondition 2's exact analogue applied to B1's item-count model instead of B2's ID-census
   model.

4. **Content-preservation, byte-for-byte.** Every migrated item's `date:`/`summary:` text is
   preserved verbatim in the archive file — `rotate_changelog` already guarantees this internally
   (its own `write_atomic`-backed accumulation logic), but this BC's independent verification
   re-confirms it externally, exactly as BC-1.18.008/BC-1.18.011 re-confirm their own respective
   primitives' internal guarantees rather than trusting them un-verified. This is BC-1.18.008
   Postcondition 6(a)'s exact analogue.

5. **Fail-loud on verification failure.** If the independent census (Postcondition 3) does not
   reconcile, the migration ABORTS: `BC-INDEX.md`'s frontmatter is left in its exact pre-migration
   state (fail-loud, not partial-and-silent) — no partially-rotated `changelog:` sequence and no
   partial archive-file append is ever treated as authoritative. Product-owner selects the exact
   staging mechanic against `rotate_changelog`'s actual write ordering at F4 implementation time
   (either a dry-run census computed BEFORE `rotate_changelog` executes, or a restorable
   pre-migration snapshot retained until the post-execution census passes) — mirroring whichever of
   BC-1.18.008/BC-1.18.011's two staging patterns fits `rotate_changelog`'s actual write ordering.
   The existing artifact-generic error code `E-SHD-003` ("backfill-split content-preservation
   verification failed for `<artifact>`") MAY be reused for this BC's failure path rather than
   allocating a new code — its wording is already artifact-generic, not decision-log-specific.

6. **No new Cohort-B dependency, and no ordering dependency on BC-1.18.008 or BC-1.18.011.** This
   migration operates entirely on `BC-INDEX.md`'s frontmatter `changelog:` sequence — a DIFFERENT
   size driver than mechanism A's four append-logs (BC-1.18.008's scope) or mechanism B2's per-BC
   body tables (BC-1.18.011's scope). `regression-gate`/`convergence-tracker` do not read
   `BC-INDEX.md` at all (confirmed per the F2 architecture-delta doc §5 migration-impact map), so
   `BC-7.08.001`'s Cohort B fail-closed flip gating is UNCHANGED by this BC — the same "no new
   Cohort-B dependency" confirmation BC-1.18.011 Postcondition 7 already establishes for mechanism
   B2, mirrored here for B1's own migration.

7. **Idempotency against an already-steady-state sequence.** Re-running this migration against a
   `changelog:` sequence that is already `<= N` items (either because a prior rotation — this
   migration's own prior successful run, or a subsequent BC-1.18.009 per-write rotation — already
   brought it under N) is a safe no-op: the migration detects the sequence is already within bound
   and exits without invoking `rotate_changelog` again. No special-casing is needed beyond
   BC-1.18.009's own EC-002 (under-N no-rotation `Continue`) — this migration's idempotency check
   IS that same under-N condition, evaluated once at F4 activation instead of per-write.

8. **Corrects BC-1.18.005 Postcondition 8's "bounded" claim.** BC-1.18.005 Postcondition 8's
   read-cost characterization for the `"frontmatter-changelog-array"` shape is split into two
   explicit states: COLD (pre-this-BC — a one-time, oversized-but-finite, non-N-relative-bounded
   read) and STEADY-STATE (post-this-BC — genuinely `<= N`-item-bounded per read, by construction).
   This BC's successful completion is the event that transitions BC-INDEX.md from the cold state to
   the steady state; BC-1.18.005 has been amended (fix-burst pass-2, F-P2-007) to reflect this split
   directly rather than asserting a single unqualified "bounded" claim.

## Invariants

1. **This BC's migration logic is a caller of `rotate_changelog`, not a reimplementation.** No
   duplicate changelog-rotation, trim, validation, or write logic exists anywhere in this BC's
   implementation — it reuses the SAME `rotate_changelog` function and the SAME generalized
   `archive_path`-parameterized call surface BC-1.18.009 Postcondition 2 establishes. This BC
   differs from BC-1.18.009 only in WHEN it runs (once, at F4 activation) and in the GOVERNANCE
   wrapper around the call (independent census + content-preservation verification + fail-loud
   rollback), never in the underlying rotation mechanism itself.

2. **No `changelog:` item is ever counted twice or dropped.** The independent census (Postcondition
   3) is the sole source of truth for "did every item survive the migration" — a migration that
   passes content-preservation (Postcondition 4) but fails the census (e.g., a byte-identical
   archive append that nonetheless duplicates one item and drops another via a compensating error)
   is STILL a failing migration; both checks are independently mandatory, neither substitutes for
   the other. This is BC-1.18.011 Invariant 2's exact analogue, applied to B1's item-census model.

3. **The migration is never partially applied.** At every observable point in time — before the
   migration runs, during its execution, and after it completes — `BC-INDEX.md`'s frontmatter
   `changelog:` sequence is either the FULL original (cold, oversized) sequence or the FULL
   post-migration (steady-state, `<= N`-item) sequence; it is never observed holding some archived
   items still duplicated in the live frontmatter, or missing items that were neither retained nor
   archived.

4. **This BC's execution does NOT gate BC-7.08.001's Cohort B flip, and does NOT gate BC-1.18.008 or
   BC-1.18.011's execution.** Per Postcondition 6, no implementation may introduce an undocumented
   sequencing dependency between this migration and any of the three unrelated one-time/ongoing
   operations named above — all are independent operations that happen to be scheduled at the same
   F4 activation boundary.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | `BC-INDEX.md`'s `changelog:` sequence at ~1,997 items (today's measured cold-start count), `N=50`, `low_water_mark=25` (illustrative config) | Migration invokes `rotate_changelog` once with `keep_recent=25` (**CORRECTED, fix-burst pass-3, F-P3-005 — NEVER `keep_recent=N`**); census verifies `25 (retained) + 1972 (archived) == 1997 (pre-migration)` exactly; archive file gains 1,972 items appended in existing content-preservation order |
| EC-002 | Census reconciliation fails (e.g., a byte-count or item-count mismatch between pre- and post-migration state) | Migration ABORTS per Postcondition 5; `BC-INDEX.md`'s frontmatter left in its exact pre-migration state; fail-loud error surfaced (`E-SHD-003`, reused per Postcondition 5) |
| EC-003 | Migration is re-run after already completing successfully (sequence already `<= N` items) | Idempotent no-op (Postcondition 7): the migration detects the already-steady-state sequence and exits without invoking `rotate_changelog` again |
| EC-004 | Migration is re-run after a PARTIAL prior attempt left `rotate_changelog`'s own `write_atomic` calls incomplete (e.g., archive file written but frontmatter trim not yet applied, or vice versa) | Product-owner's selected staging mechanic (Postcondition 5) determines the exact recovery path at F4 implementation time; in all cases, the invariant that must hold is: the migration NEVER treats a partially-applied state as successfully complete, and re-running either resumes cleanly or restarts cleanly from the last verified-consistent state — never double-archives or drops items |
| EC-005 | An implementer mistakenly makes this migration a precondition for BC-7.08.001's Cohort B flip, or for BC-1.18.008/BC-1.18.011's execution | Scope violation of Postcondition 6/Invariant 4 — the F2 architecture-delta doc's migration-impact map already confirms zero dependency; this BC introduces none |
| EC-006 | The migration runs against a `changelog:` sequence that is ALREADY partially rotated (e.g., an operator manually ran the pre-existing manual `rotate_changelog` CLI tool per ADR-049 §Decision 6 before F4 activation) | Treated identically to EC-003 (idempotency) if the sequence is already `<= N`; if the manual run left the sequence still `> N` (e.g., a manual `keep_recent` value different from this BC's config `N`), the migration proceeds normally against the CURRENT (partially-reduced) count, re-deriving its own fresh census rather than assuming the manual run's prior state |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `BC-INDEX.md` `changelog:` at 1,997 items, `N=50`, `low_water_mark=25` | Pre-migration census = 1,997; `rotate_changelog` invoked once with `keep_recent=25` (**CORRECTED, fix-burst pass-3, F-P3-005 — NEVER `keep_recent=N`**); post-migration live sequence = 25 items; archive file gains 1,972 appended items; census reconciles `25 + 1972 == 1997` | happy-path |
| `changelog:` sequence already at 40 items (`<= N=50`), migration invoked | Idempotent no-op: `rotate_changelog` NOT invoked; frontmatter unchanged; zero archive-file writes | edge-case |
| Census reconciliation finds a mismatch (simulated: 1 item lost during a corrupted `rotate_changelog` run) | Migration ABORTS; original frontmatter left byte-identical to pre-attempt state; fail-loud error surfaced (`E-SHD-003`) naming the reconciliation mismatch | error |
| Migration re-run after a prior successful completion (sequence already at steady-state `N=50`) | No-op (EC-003): zero writes, frontmatter unchanged | edge-case |
| Migration crashes mid-execution, restarted | Recovery per the selected staging mechanic (Postcondition 5); the restart NEVER produces a double-archived item or a dropped item — verified by re-running the independent census after recovery | error |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-136 | Independent-census integrity invariant — `(items retained in live frontmatter) + (items appended to archive) == pre-migration count` exactly, for any starting item count | integration test (fixture-driven census comparison against synthetic `changelog:` sequences of varying sizes, including a corrupted-run negative-control fixture) |
| VP-135 | Content-preservation invariant — every migrated item's `date:`/`summary:` text is preserved verbatim in the archive file | proptest / golden-file round-trip against synthetic `changelog:` fixtures |
| VP-136 | Idempotency invariant — re-running the migration against an already-steady-state (`<= N`-item) sequence is a safe no-op (zero writes, zero `rotate_changelog` invocations) | integration test (double-invocation fixture) |
| VP-136 | Fail-loud rollback invariant — a simulated census-reconciliation failure leaves `BC-INDEX.md`'s frontmatter byte-identical to its pre-migration state | fault-injection / integration test (simulated mismatch at the census-verification step) |
| VP-137 | No-new-Cohort-B/no-cross-migration-dependency invariant — this BC's completion is never referenced as a precondition in `hooks-registry.toml`'s `failure_policy` sequencing, nor in BC-1.18.008/BC-1.18.011's own execution gating | static-check (config/PR-template audit) |

VP IDs ALLOCATED by formal-verifier (S-25.02 F2 fix-burst pass-2; VP-INDEX v3.04): **VP-135**
(proptest; content-preservation), **VP-136** (integration; independent-census + fail-loud rollback
[reuses `E-SHD-003`] + idempotency — three same-method obligations consolidated per the
single-method-per-VP convention), **VP-137** (static-check; no-new-Cohort-B / no-cross-migration
dependency). This trio is the direct B1 analogue of VP-123/VP-124 (BC-1.18.008's content-preservation
+ atomicity/idempotency pair) and VP-132/133/134 (BC-1.18.011's proptest/integration/static-check
set), keyed to the `changelog:` item-count model instead of decision-log's byte-count model or
BC-INDEX's ID-census model.

## Related BCs

- BC-1.18.009 — this BC governs the ONE-TIME cold-start migration that establishes the precondition for BC-1.18.009's ONGOING steady-state per-write rotation contract; BC-1.18.009 specifies the steady state, this BC specifies the transition (depends on)
- BC-1.18.005 — Postcondition 8's cold/steady-state split (fix-burst pass-2, F-P2-007) is corrected BY this BC's existence; this BC's successful completion is what makes the steady-state characterization true (related to)
- BC-1.18.006 — this BC reuses the same `write_atomic`/staging discipline pattern BC-1.18.006 establishes for mechanism A, analogous to BC-1.18.008's relationship to BC-1.18.006 (depends on)
- BC-1.18.008 — this BC's structure is modeled directly on BC-1.18.008's governed one-time-migration pattern (content-preservation, census, fail-loud rollback, idempotency), substituting an item-count partition for a byte-count partition (related to)
- BC-1.18.011 — sibling governed one-time migration BC for mechanism B2 (BC-INDEX per-subsystem body split); both this BC and BC-1.18.011 close the "ungoverned lazy-first-write migration" gap for their respective mechanisms, independently and with no ordering dependency between them (related to)
- BC-7.08.001 — explicitly has NO dependency on this BC (Postcondition 6); cited here only to document the absence of a relationship an implementer might otherwise assume by analogy to BC-1.18.008 (related to)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — one-time migration entry point for the B1 cold-start changelog backfill, reusing BC-1.18.009's `rotate_changelog` call surface
- `crates/last-amended-migrate/src/rotate.rs` — `rotate_changelog` (reused, not reimplemented; same generalized `archive_path`-parameterized call surface BC-1.18.009 requires, not yet implemented — see BC-1.18.009's SDK Grounding Evidence for the current-state grep)
- `.factory/specs/behavioral-contracts/BC-INDEX.md` frontmatter `changelog:` field — the cold-start artifact this migration operates on

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn rotate_changelog" crates/last-amended-migrate/src/rotate.rs
pub fn rotate_changelog
```

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

Confirms both existing primitives (`rotate_changelog`, `write_atomic`) this BC's Invariant 1 states
are reused, not reimplemented, for the one-time migration operation.

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

- VP-135 (proptest), VP-136 (integration), VP-137 (static-check) — ALLOCATED by formal-verifier (S-25.02 F2 fix-burst pass-2; VP-INDEX v3.04), keyed to B1's `changelog:` item-count model, the direct analogue of VP-123/VP-124 (mechanism A) and VP-132/133/134 (mechanism B2). The candidate properties enumerated in `## Verification Properties` above consolidate per the single-method-per-VP convention (mirroring VP-124/VP-133): content-preservation → VP-135; independent-census + fail-loud rollback + idempotency → VP-136; no-new-Cohort-B / no-cross-migration dependency → VP-137.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | Anchoring to CAP-043: "Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding" — because this BC describes the governed one-time migration that closes B1's cold-start ungoverned-migration gap for `BC-INDEX.md`'s `changelog:` array, which is exactly what CAP-043 defines per `capabilities.md` §CAP-043: "A mandatory one-time backfill-split retroactively shards the four EXISTING append-log files... without it, this capability would only prevent future overflow and never shrink the artifacts already producing the majority of observed INDETERMINATE events" — this BC applies the IDENTICAL production-grade-completeness principle to B1's cold-start `changelog:` excess, the one remaining sharded-artifact class (of mechanism A/B1/B2) that lacked a governed one-time migration prior to this BC. CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — no existing capability other than CAP-043 covers a governed one-time migration establishing B1's steady-state precondition; CAP-041 (INDETERMINATE detection/quarantine) and CAP-042 (the `rotate_changelog`/`last_amended` write-path fix) are both distinguishable per capabilities.md's own CAP-043 entry, and neither covers a BC-INDEX `changelog:`-array cold-start migration. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with the sibling BC-1.18.005–011 precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` one-time B1 backfill migration logic) |
| ADR | ADR-051 §Decision 13 (governed one-time B1 changelog backfill migration, fix-burst pass-2 addition, F-P2-007); §Decision 7 (B1 ongoing rotation contract and the archive-path extension this migration reuses); §Decision 1 (item-count trigger and cold/steady-state read-cost split, BC-1.18.005 Postcondition 8) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution fix-burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-3 finding F-P3-005 MEDIUM, ADR-051 v1.3 Decision 14): CORRECTED Postcondition 2's `rotate_changelog` invocation target from `keep_recent = N` to `keep_recent = low_water_mark` (BC-1.18.005 Postcondition 8's rotation-target config, the SAME value BC-1.18.009's ongoing per-write rotation uses) — migrating to `N` would leave the live sequence at the trigger boundary immediately after go-live, causing the FIRST post-migration write to immediately re-trigger rotation, reproducing the every-write block+retry pathology `low_water_mark` exists to eliminate from write #1 of the new steady state. Updated EC-001 and the matching Canonical Test Vector's illustrative numbers from `keep_recent=50`/`50+1947==1997` to `keep_recent=25`/`25+1972==1997` (`N=50`, `low_water_mark=25` illustrative). All other Postcondition/Invariant/Edge-Case obligations (items 1, 3-8) UNCHANGED. |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, fix-burst pass-2 addition per F-P2-007 MEDIUM, ADR-051 v1.2 Decision 13). Allocated as BC-1.18.012 — confirmed as the next free SS-01 slot against the live `ss-01/` directory (BC-1.18.001–011 all pre-existing) and BC-INDEX.md at authoring time; no collision. Governed one-time migration for mechanism B1's cold-start `changelog:` backfill (~1,997 unrotated items): reuses `rotate_changelog` via BC-1.18.009's generalized `archive_path` call surface, `keep_recent = N`; independent-census integrity (pre-migration count == retained + archived, exactly); content-preservation byte-for-byte; fail-loud rollback on verification failure (reuses `E-SHD-003`); idempotency against an already-steady-state sequence; confirmed NO new Cohort-B dependency and no ordering dependency on BC-1.18.008/BC-1.18.011. Corrects BC-1.18.005 Postcondition 8's "bounded" claim by establishing the steady state that claim now explicitly conditions on. Modeled directly on BC-1.18.008's structure per the F2 architecture-delta doc §4b authorship input. CAP-043 capability anchor. VP citations left `(pending)` for formal-verifier (next free VP-135 against VP-INDEX v3.03). ADR-051 §D13/§D7/§D1 citations. |
