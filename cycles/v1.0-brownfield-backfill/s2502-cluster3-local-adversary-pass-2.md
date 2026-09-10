---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 2
verdict: NOT CLEAN
finding_count: 7
finding_breakdown: "2 HIGH + 2 MEDIUM + 3 observations (ADVISORY-class, non-blocking; see Disposition Summary)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "41c81fc4"
diff_head: "5d195519"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "16da643"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-2

> Fresh-context adversarial review (Iron Law: fresh context, no prior-pass visibility beyond this
> cascade's own convention) of `feature/S-25.02-backfill` against BC-1.18.008 (mechanism-A one-time
> backfill-split) v1.3 and BC-1.18.007 (retention/compaction) v1.2, run after pass-1's fix-burst
> (`41c81fc4`, D-1191) landed on the branch. This is the second LOCAL BC-5.39.001 cascade pass run
> against the mechanism-A implementation; per the cluster-3 convention established at pass-1, the
> full Part A finding set is persisted here as a standalone artifact.

## Verdict

**NOT CLEAN** — 4 in-scope findings disposed this burst (F-C3-P2-001..004): 2 HIGH, 2 MEDIUM, plus
3 non-blocking observations recorded alongside the findings (folded into the Disposition Summary,
not separately numbered against BC-5.39.001's finding count). BC-5.39.001 LOCAL cluster-3 streak:
**0/3 → 0/3** (still not clean; cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate
track).

## Part A — Findings

### F-C3-P2-001 (HIGH) — Postcondition 6(b) gate blind to over-detection (spurious extra boundary)

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, `run_mechanism_a_backfill_split`'s
Postcondition 6(b) independent-recompute gate, as fixed by pass-1's F-C3-P1-001.

**Finding:** pass-1's fix replaced the tautological gate with an independent recompute via
`mechanism_a_record_boundary_offsets`, but implemented the cross-check as a **union**
(`combined_offsets`) of the caller-supplied `record_boundary_offsets` and the oracle's own detected
boundaries, then compared cardinalities. A union is structurally blind to **over-detection**: a
caller-supplied offsets list that is a strict superset of the true boundaries — every genuine
boundary present, plus one spurious extra landing mid-record — contributes nothing new when unioned
with the (smaller) oracle set, so `|caller ∪ oracle| == |caller|` and both sides of the Postcondition
6(b) comparison stay tautologically equal even though the spurious offset would physically split a
real record across two shard files. The gate also remained separately vulnerable to a
**same-count swap**: two caller offsets each individually wrong but summing to the same cardinality
as the true set would likewise pass a cardinality-only check. Combined with F-C3-P1-001's original
under-detection fix, this is the third distinct mis-detection direction found across two passes on
the same gate.

**Impact:** the one gate BC-1.18.008 Postcondition 6(b) relies on to catch content-loss/corruption
during the mandatory one-time backfill-split of 4 large, irreplaceable cycle append-logs remained
incomplete after pass-1's own fix — a record-splitting defect in this direction would still sail
through undetected. Meets the HIGH bar (not BLOCKER: pass-1's fix already closed the more severe
under-detection direction; this is the gate's second-order incompleteness, not a fully-open gate).

**Disposition:** ROUTED to implementer. Fixed on `feature/S-25.02-backfill` (`5d195519`): the gate
now performs an oracle **SET-EQUALITY** cross-check — the caller-supplied offsets (sorted,
deduplicated) must exactly match the independently-recomputed true boundary set whenever the oracle
recognizes any genuine boundary at all; any divergence in either direction returns
`MechanismABackfillError::ContentPreservationFailed` with a detail message explicitly citing both
the under- and over-detection failure classes. When the oracle recognizes no boundary at all for a
given artifact_stem/content pair, the caller's offsets are trusted at face value (unchanged
fallback behavior, preserving pass-1's original semantics for that case). RED fixtures added
(`3bdf83f7`) covering strict-superset over-detection and same-count-swap mis-detection; both fail
pre-fix, pass post-fix.

### F-C3-P2-002 (HIGH) — EC-002 `oversized_record` flag dropped from published shard-index entry

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`,
`mechanism_a_partition_for_backfill` (computes the flag) vs. `run_mechanism_a_backfill_split`
(publishes the `ShardIndexEntry`, previously omitted it) and `ShardIndexEntry` itself (had no field
to carry it).

**Finding:** BC-1.18.008 EC-002 documents a single-oversized-record exception — a record that alone
exceeds `shard_cap_bytes` is sealed whole rather than split mid-record, so its shard's
`bytes_at_seal` may legitimately exceed the cap for that one entry. The partitioning function
already computed this per-partition, but the value was silently dropped at the shard-index
publication boundary: `ShardIndexEntry` had no field to carry it, so every downstream reader of the
shard-index (including a future validator distinguishing "legitimately oversized per EC-002" from
"a cap-accounting bug") had no way to tell the two cases apart from the index alone — only from
re-deriving it against the source content, defeating the point of an index.

**Impact:** loses the one piece of state that lets the shard-index be trusted as a self-contained
record of EC-002 exceptions, rather than requiring every consumer to re-derive the exception from
first principles. HIGH: not a content-loss defect, but a structural information-loss defect on the
canonical index BC-1.18.008 exists to produce.

**Disposition:** ROUTED to implementer. Fixed (`5d195519`): added `pub oversized_record: bool` to
`ShardIndexEntry` (`#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, mirroring the
existing `sealed_retroactively` field's additive-compatibility precedent — backward compatible with
every `[[shard]]` index entry produced before this field existed). `run_mechanism_a_backfill_split`
now surfaces the already-computed `partition.oversized_record` through to the published entry.
TD-VSDD-060 sibling-sweep performed across every other `ShardIndexEntry` construction site
(`execute_roll`, `self_heal_resume_from_truncate`, `self_heal_reconcile_missing_index_entries`, and
all in-module test fixtures) — each explicitly set `oversized_record: false` with an inline comment
noting the mechanism-A-only scope of the flag, since none of those call sites is a backfill-split
partition.

### F-C3-P2-003 (MEDIUM) — leading preamble bytes excluded from cap accounting

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`,
`mechanism_a_partition_for_backfill`.

**Finding:** BLOCKER-1's prior fix (landed before pass-1) seeded the first partition's `bytes`
accumulator with the leading preamble content so the preamble is physically folded into the first
shard's on-disk bytes. However, the separate `partition_bytes` accumulator — the value the cap
decision actually reads to decide when to flush a partition — was still seeded at `0`, not with the
same preamble length. A real leading preamble could therefore be physically written into the first
partition while never counting toward the Postcondition 2 `<= shard_cap_bytes` decision that
governs that partition, letting a sealed shard's true on-disk size silently exceed
`shard_cap_bytes` without the accounting variable that was supposed to prevent it ever seeing those
bytes.

**Impact:** MEDIUM — a real (non-empty) leading preamble on one of the 4 target artifacts could
produce an over-cap shard undetected by the cap-accounting logic itself (distinct from EC-002's
sanctioned single-oversized-record exception, which is explicit and flagged; this was silent).

**Disposition:** ROUTED to implementer. Fixed (`5d195519`): `partition_bytes` is now seeded with
`record_boundary_offsets[0] as u64` (the exact preamble length — `content[0..record_boundary_offsets[0])`),
matching BLOCKER-1's `bytes` seeding. No-op when there is no preamble (`record_boundary_offsets[0] ==
0`).

### F-C3-P2-004 (MEDIUM) — stale test-module header mislabels F-001/F-002 as "expected to fail"

**Location:** test module doc comment, `crates/factory-dispatcher/tests/bc_1_18_008_backfill_split_test.rs`.

**Finding:** the test-module header still described the F-001/F-002 fixtures in transient
"expected to fail pre-fix" framing dating from pass-1's RED-fixture authoring. Once pass-1's fixes
(`41c81fc4`) landed, that framing went stale — the fixtures now pass, and a reader encountering the
module in isolation (without pass-1's own history) would be misled about the fixtures' current
status and purpose. This is the second time this class of transient-status-in-a-doc-comment defect
has been found in this cluster's own cascade (the first, F-C3-P1-005 at pass-1, described a
withdrawn `ceil()` framing in the same module) — see the Drift Items PROCESS-WATCH entry recorded
in this burst's STATE.md update.

**Impact:** MEDIUM — documentation-only, no functional defect, but actively misleading to a future
maintainer about test intent/status.

**Disposition:** ROUTED to test-writer. Fixed (`3bdf83f7`): module header rewritten to
status-neutral prose — describes what each fixture verifies (by BC/EC/finding-ID citation) rather
than a transient pass/fail expectation tied to a specific commit's pre/post-fix state.

## Observations (non-blocking)

- **O-C3-P2-001** — the Postcondition 6(b) gate's first check (content-preservation, byte-identity
  comparison, from F-C3-P1-001) is tautological in isolation once F-C3-P2-001's set-equality check
  is also in place: any offsets divergence that set-equality would catch is a strict subset of what
  would also fail byte-identity reconstruction for most realistic corruption shapes. Not a defect —
  the two checks are intentional defense-in-depth (independent failure modes: set-equality catches
  boundary-list divergence before partitioning even runs; byte-identity catches a partitioning bug
  downstream of a correct offsets list) — recorded for completeness, no action needed.
- **O-C3-P2-002** — a preamble-only, zero-record shard (an artifact whose entire content precedes
  the first record boundary, with no records following) is an untested edge case for the
  partitioning path. No evidence this is reachable against the 4 real target artifacts
  (`decision-log.md`, `burst-log.md`, `lessons.md`, `session-checkpoints.md` all have records
  throughout), but not structurally ruled out for a hypothetical fifth. Non-blocking; flagged for a
  future pass or the eventual production-caller wiring at T-12.
- **O-C3-P2-003 (SPEC-WORDING)** — BC-1.18.008 Postcondition 2's normalization clause (a) reads, in
  isolation, as if "any `^## ` h2 heading" is a record boundary for ALL FOUR target artifacts
  uniformly, including `decision-log.md`. The actual shipped code correctly keys `decision-log.md`'s
  record boundaries on `^\| D-[0-9]+ \|` (its own distinct record marker convention, per the BC's own
  marker-table elsewhere), not on any h2. The code is correct; the PC2 clause (a) prose is loosely
  worded in a way that could mislead a future reader who encounters only that clause. Non-blocking —
  recorded as a Drift Item (SPEC-HYGIENE) anchored to the next BC-1.18.008 spec touch, not fixed this
  burst (no spec amendment this pass; product-owner not dispatched for a wording-only, non-defect
  tightening).

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P2-001 | HIGH | FIXED — oracle SET-EQUALITY cross-check (catches under-, over-, and same-count-swap mis-detection) | `feature/S-25.02-backfill` `5d195519` |
| F-C3-P2-002 | HIGH | FIXED — `oversized_record: bool` added to `ShardIndexEntry`, populated, sibling-swept | `feature/S-25.02-backfill` `5d195519` |
| F-C3-P2-003 | MEDIUM | FIXED — `partition_bytes` seeded with preamble length | `feature/S-25.02-backfill` `5d195519` |
| F-C3-P2-004 | MEDIUM | FIXED — status-neutral test-module header rewrite | `feature/S-25.02-backfill` `3bdf83f7` |
| O-C3-P2-001 | ADVISORY (observation) | No action — intentional defense-in-depth, not a defect | — |
| O-C3-P2-002 | ADVISORY (observation) | No action — untested edge, not reachable against real targets, flagged for future pass | — |
| O-C3-P2-003 | ADVISORY (observation, SPEC-WORDING) | No action this burst — Drift Item recorded, anchored next BC-1.18.008 spec touch | Drift Item (STATE.md) |

## Code Gate (post-fix, this burst)

`bc_1_18_008` test suite: 34/34 green. `cargo fmt --check --all`: clean. `cargo clippy --workspace
--all-targets -- -D warnings`: clean. Feature branch HEAD: `feature/S-25.02-backfill` @ `5d195519`
(pushed to `origin`).

## Next

Cluster-3 LOCAL BC-5.39.001 cascade pass-3, fresh context, against BC-1.18.008 v1.3 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `5d195519`.
