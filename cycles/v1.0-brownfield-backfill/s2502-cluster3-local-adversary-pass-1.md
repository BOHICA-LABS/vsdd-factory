---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 1
verdict: NOT CLEAN
finding_count: 8
finding_breakdown: "1 BLOCKER + 1 HIGH + 3 MEDIUM + 1 MINOR + 1 ADVISORY (plus 1 additional MEDIUM-labeled scope-boundary deferral, F-C3-P1-006 — see Disposition Summary)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "2757b7c4"
diff_head: "41c81fc4"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "16da643"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-1

> Fresh-context adversarial review (Iron Law: fresh context, no prior-pass visibility beyond this
> cascade's own convention) of `feature/S-25.02-backfill` against BC-1.18.007 (retention/compaction)
> and BC-1.18.008 (mechanism-A one-time backfill-split) v1.2, run after implementer's RESUME STEP 1
> work (pattern-based record-boundary rewrite `882dfb61`, PC6(b) load-bearing gate `cdd8457b`,
> stale-doc-comment correction `2757b7c4`) landed on the branch. This is the first LOCAL BC-5.39.001
> cascade pass run against the completed mechanism-A implementation.
>
> **Why this report exists as a standalone artifact (not narrative-only):** the immediately prior
> adversary attempt this cycle (`adv-cluster3-p1`, referenced at D-1189/STATE.md
> SESSION-WRAP-PAUSE-2026-09-10) never persisted a report file — its findings survive only as
> STATE.md prose. To avoid repeating that loss, this pass's full Part A finding set is persisted
> here verbatim, independent of STATE.md/decision-log.md/INDEX.md's own (necessarily more compact)
> summaries.

## Verdict

**NOT CLEAN** — 8 items disposed this burst (F-C3-P1-001..008): 1 BLOCKER, 1 HIGH, 3 MEDIUM
(F-C3-P1-003/004/005) + 1 additional MEDIUM-labeled scope-boundary item (F-C3-P1-006, adjudicated
a deferral rather than an in-scope fix — see disposition below), 1 MINOR, 1 ADVISORY (folded into
F-C3-P1-002's resolution). BC-5.39.001 LOCAL cluster-3 streak: **0/3 → 0/3** (still not clean;
cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

## Part A — Findings

### F-C3-P1-001 (BLOCKER) — PC6(b) record-count content-preservation gate is tautological

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, the PC6(b) verification gate
introduced by RESUME STEP 1's `cdd8457b` ("MED-C make PC6(b) record-count gate load-bearing").

**Finding:** the gate compares a record count derived from `mechanism_a_record_boundary_offsets`'s
own output against a second count ALSO derived from calling the same function again (or from a
value trivially re-derivable from `offsets.len()`) — both sides of the equality check share the
same source of truth, so the gate can never observe a genuine content-preservation defect (a record
silently dropped, merged, or duplicated during packing). This defeats BC-1.18.008 Postcondition 6's
actual intent: an INDEPENDENT verification that the split's shards, concatenated, byte-reproduce the
original file. A boundary-detection bug that undercounts or overcounts records would sail through
this gate undetected.

**Impact:** silently defeats the one gate BC-1.18.008 relies on to catch content-loss during the
mandatory one-time backfill-split of 4 large, irreplaceable cycle append-logs (`decision-log.md`,
`burst-log.md`, `lessons.md`, `session-checkpoints.md`). A content-preservation defect here is
data loss with no test/gate to catch it — meets the BLOCKER bar.

**Disposition:** ROUTED to implementer. Fixed on `feature/S-25.02-backfill` (part of `41c81fc4`):
the gate now derives its second count from an INDEPENDENT source — a direct byte-level
re-concatenation-and-compare against the original pre-split content (BC-1.18.008 Postcondition 6's
own byte-identity check), not a re-derivation of the same `offsets` value. Red Gate fixture added at
`08c3c131`/`1e16dfe4` (fails pre-fix, passes post-fix).

### F-C3-P1-002 (HIGH) — `is_checkpoint_record_heading` heuristic is case-sensitive-broken against real content

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, `is_checkpoint_record_heading`
(session-checkpoints.md h2/h3-exception marker detection).

**Finding:** the heuristic (`starts_with("Archived") || contains("Checkpoint")`) is demonstrably
broken against real on-disk content — it misses the genuine checkpoint record heading
`## ARCHIVED CHECKPOINT: 2026-08-27 — pass-60 CLEAN D-1117...` (all-caps `ARCHIVED CHECKPOINT`,
matches neither predicate arm case-sensitively). Direct inspection of both real
`session-checkpoints.md` files confirms every h2 heading in both is a genuine checkpoint record
(v1.0-brownfield-backfill: 182 h2 records; v1.0-feature-engine-discipline-pass-1: 12 h2 records) —
zero legitimate non-record h2 asides exist that would justify a narrowing heuristic in the first
place.

**Impact:** a real record silently excluded from the shard partition — the same content-loss class
as F-C3-P1-001, on a different artifact.

**Disposition:** ADJUDICATED by product-owner as **REVERT-TO-ANY-H2, no BC-1.18.008 spec change** —
the BC's existing marker-table row ("any h2 = boundary", no confirmed exceptions) was already
correct; the code-side heuristic was the defective party, not the spec. ROUTED to implementer:
`is_checkpoint_record_heading` DELETED, `session-checkpoints.md` boundary detection reverted to the
BC's already-correct any-`^## `-is-a-boundary rule (part of `41c81fc4`).

### F-C3-P1-008 (ADVISORY) — folded into F-C3-P1-002

Observation that the case-sensitivity bug pattern (ad-hoc string-prefix/contains heuristics
diverging from an already-correct declarative spec rule) has now recurred across this cluster's own
history; no standalone action — folded into F-C3-P1-002's disposition and closed with it (heuristic
removed entirely, not patched to be case-insensitive, eliminating the whole defect class for this
artifact).

### F-C3-P1-003 (MEDIUM) — missing panic guard on malformed record-boundary input

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, `mechanism_a_record_boundary_offsets`
and its callers.

**Finding:** a malformed/adversarial input (e.g. a byte offset computed past the file's actual
length by an off-by-one in a boundary-pattern match) could reach a slicing operation without a
bounds check, panicking the dispatcher process rather than returning a typed `HookResult::Error`.

**Disposition:** ROUTED to implementer. Fixed (`41c81fc4`): explicit bounds validation before every
slice operation in the record-partitioning path, returning `E-SHD-003` (content/boundary violation)
on an out-of-range offset instead of panicking.

### F-C3-P1-004 (MEDIUM) — PC2 `ceil()` shard-count formula jointly unsatisfiable with boundary-preservation

**Location:** BC-1.18.008 Postcondition 2 + Canonical Test Vectors.

**Finding:** PC2's `ceil(current_bytes / shard_cap_bytes)` shard count was jointly unsatisfiable
with "preserving record boundaries" for non-uniform record sizes. Counterexample: five 40-byte
records against a 70-byte cap — `ceil(200/70) = 3`, but no boundary-preserving packing can fit two
40-byte records into one 70-byte shard (80 > 70), so a correct boundary-preserving greedy packer
actually produces 5 shards. Yet the Canonical Test Vectors asserted the `ceil()` value as an EXACT
expected shard count (19 for `decision-log.md`, 5 for `lessons.md`).

**Disposition:** ADJUDICATED by product-owner — BC-1.18.008 **v1.2 → v1.3** (factory-artifacts
`03b9c1bc`). Postcondition 2 now documents `ceil()` as a LOWER BOUND only, specifies the actual
deterministic greedy boundary-preserving packing procedure as the real split-point rule
(`actual_count >= ceil(...)`, equality only when records pack without slack), and Postcondition 4's
retention-composition math is reworded from an approximate framing to an explicit lower-bound
inequality. Three Canonical Test Vector rows marked NEEDS-UPDATE; three unaffected rows annotated
out-of-scope with reasons. No new `E-SHD-NNN` code warranted — `E-SHD-003` already covers the actual
failure mode (boundary/content violation); exceeding the `ceil()` lower bound is expected correct
behavior. test-writer filled in the three NEEDS-UPDATE rows with the actual measured packed-shard
counts against the real fixture content (part of `08c3c131`).

### F-C3-P1-005 (MEDIUM) — stale test doc comment describing withdrawn exact-`ceil()` behavior

**Location:** test module doc comment in the mechanism-A backfill test suite (references the
withdrawn exact-`ceil()`-as-target framing superseded by F-C3-P1-004's fix).

**Finding:** a test-suite-level doc comment still described the shard count as "exactly
`ceil(bytes/cap)`", now false per F-C3-P1-004's BC-1.18.008 v1.3 correction — would mislead future
maintainers reading the test file in isolation.

**Disposition:** ROUTED to test-writer. Fixed (`08c3c131`): doc comment corrected to describe
`ceil()` as a lower bound and cite the greedy packing procedure, matching BC-1.18.008 v1.3
Postcondition 2 verbatim in substance.

### F-C3-P1-006 (MEDIUM) — `mechanism_a_record_boundary_offsets` / backfill-split has no production caller

**Location:** dispatcher wiring — `mechanism_a_record_boundary_offsets` and the backfill-split
orchestration function it feeds are fully implemented and unit-tested on this branch, but no
production code path (PreToolUse/PostToolUse hook, CLI entry point, or scheduled job) actually
invokes the one-time backfill-split migration described by BC-1.18.008 Postcondition 1 ("executed
exactly once, at F4 activation").

**Finding:** as of this pass, the mechanism-A backfill-split is inert in production — fully correct
per its own unit tests, but never actually runs against the four real oversized artifacts it exists
to migrate.

**Disposition:** HUMAN-ADJUDICATED as a legitimate scope-boundary DEFERRAL, not an in-scope fix.
BC-1.18.008 Postcondition 1 is explicit that the migration fires "once, at F4 activation" — F4
activation for mechanism-A backfill is the Cohort-B flip (cluster-7, CAPSTONE per D-1170's
sequencing), gated on cluster-3 (this cluster) being merged plus the F4 calibration harness locking
cap constants. The one-time migration-runner invocation is correctly wired at **T-12** (the S-25.02
task that owns Cohort-B-flip activation wiring), not at cluster-3's own TDD delivery scope — wiring
it here would invoke an unactivated migration against production content prematurely, outside this
cluster's own scope per D-1170. Recorded as a Drift Item anchored to T-12 (a real, existing task ID
in S-25.02's own task list — not a fabricated placeholder), per CLAUDE.md Canonical Principle Rule 3.

### F-C3-P1-007 (MINOR) — stray `| D-` marker

**Location:** a stray `| D-` literal-pipe-prefixed marker left over from an earlier edit pass,
inside a doc comment / narrative string in `shard_manager.rs`.

**Finding:** cosmetic — a leftover `| D-` fragment (artifact of a prior table-row-style edit) with
no functional effect, but confusing to a future reader who might mistake it for a real decision
citation.

**Disposition:** ROUTED to implementer. Fixed (`41c81fc4`): stray marker removed.

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P1-001 | BLOCKER | FIXED — independent byte-identity PC6(b) gate | `feature/S-25.02-backfill` `41c81fc4` |
| F-C3-P1-002 | HIGH | ADJUDICATED (REVERT-TO-ANY-H2, no spec change) — FIXED | `feature/S-25.02-backfill` `41c81fc4` |
| F-C3-P1-003 | MEDIUM | FIXED — panic guard | `feature/S-25.02-backfill` `41c81fc4` |
| F-C3-P1-004 | MEDIUM | AMENDED — BC-1.18.008 v1.2→v1.3 | `factory-artifacts` `03b9c1bc` |
| F-C3-P1-005 | MEDIUM | FIXED — stale doc comment | `feature/S-25.02-backfill` `08c3c131` |
| F-C3-P1-006 | MEDIUM | HUMAN-ADJUDICATED — DEFERRED to T-12 (scope-boundary, not a defect) | Drift Item, anchored T-12 |
| F-C3-P1-007 | MINOR | FIXED — stray marker removed | `feature/S-25.02-backfill` `41c81fc4` |
| F-C3-P1-008 | ADVISORY | Folded into F-C3-P1-002 | — |

## Code Gate (post-fix, this burst)

`bc_1_18_008` test suite: 30/30 green. `cargo fmt --check --all`: clean. `cargo clippy --workspace
--all-targets -- -D warnings`: clean. Feature branch HEAD: `feature/S-25.02-backfill` @ `41c81fc4`
(pushed to `origin`).

## Next

Cluster-3 LOCAL BC-5.39.001 cascade pass-2, fresh context, against BC-1.18.008 v1.3 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `41c81fc4`.
