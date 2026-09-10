---
document_type: behavioral-contract
level: L3
version: "1.2"
status: draft
producer: product-owner
timestamp: 2026-09-10T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f1-delta-analysis.md
  - .factory/specs/verification-properties/VP-INDEX.md
input-hash: "d7ab601"
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

# BC-1.18.008: Mandatory One-Time Backfill-Split of the Four Pre-Existing Oversized Cycle Append-Logs

## Description

AC-002/AC-003 as worded only gate FUTURE writes; they do not, by themselves, retroactively split
the four cycle append-log files (`decision-log.md`, `burst-log.md`, `lessons.md`,
`session-checkpoints.md`) that are already far over any calibrated cap. This BC specifies a
mandatory, one-time backfill-split operation, executed once at F4 activation, that splits each
existing monolithic file into `ceil(current_bytes / shard_cap_bytes)` sealed shards plus a fresh
current file, publishing the full shard index for the pre-existing history in the same operation.
Without this BC, Layer 2 would only prevent future overflow and would never actually shrink the
four artifacts currently producing the majority of observed `plugin.indeterminate` events — an
incomplete delivery of the story's own stated purpose, forbidden under CLAUDE.md's production-grade
default (no partial/MVP delivery of a shipped feature).

## Preconditions

1. BC-1.18.005's cap formula has been F4-locked (constants are no longer provisional) for all four
   mechanism-A artifacts.
2. BC-1.18.006's seal+create+index-publish mechanism and BC-1.18.007's retention/compaction policy
   are both implemented and available for reuse (this BC does not invent new split logic — it
   applies the existing seal mechanism retroactively, in a loop, to pre-existing monolithic
   content).
3. At the time this BC's backfill operation is scheduled, direct measurement of the four
   mechanism-A artifacts (as of 2026-09-05, the most recent measurement available at F2) shows:
   `decision-log.md` 908,938 bytes, `burst-log.md` 806,198 bytes, `lessons.md` 234,731 bytes,
   `session-checkpoints.md` 830,621 bytes — every one already 5-19× over the illustrative
   provisional 49,152-byte (48 KiB) cap. Exact byte counts at the actual F4 execution moment MUST
   be re-measured (these will have grown further between F2 and F4); the counts above are cited
   for illustrative scale only, not as the literal input to the F4 backfill run.

## Postconditions

1. **The backfill-split executes exactly once per artifact, as a one-time migration task at F4
   activation — never as an ongoing per-write mechanism.** This BC is distinct from BC-1.18.006's
   per-write roll: BC-1.18.006 fires reactively on every future over-cap write; this BC fires
   proactively, once, against each artifact's PRE-EXISTING content at the moment Layer 2's cap
   check goes live for that artifact.

2. **Split algorithm: partition pre-existing content into `ceil(current_bytes / shard_cap_bytes)`
   sealed shards, preserving record boundaries.** The monolithic file's content is partitioned at
   the structural record-boundary markers native to the artifact's own format. Boundary detection
   for all four mandatory artifacts MUST use the exact patterns enumerated in the
   **Record-Boundary Marker Table** below — never at an arbitrary byte offset that could split a
   single record (a D-NNN row, a burst-log record, a lessons.md entry, a session-checkpoint entry)
   across two shard files, and never keyed on heading LEVEL alone: h2-only detection is
   insufficient (see the marker table's confirmed h3-exception record forms, which an h2-only
   detector would either silently skip — causing backfill to no-op on the artifact's true content
   — or mis-treat a nested `### ` sub-heading as a false boundary, splitting a record mid-record).
   Each resulting shard's byte size is `<= shard_cap_bytes` (BC-1.18.005's per-artifact effective
   cap, via the Cross-Validator Minimum Rule). The LAST partition becomes the fresh "current" file
   at the canonical name (per BC-1.18.006's stable-current-filename convention); all partitions
   before it are sealed with sequential `seq` numbers starting at 1, in chronological
   (original-file-order) sequence.

   **Record-Boundary Marker Table (authoritative).** Verified by direct inspection of the real
   on-disk content of both `.factory/cycles/v1.0-feature-engine-discipline-pass-1/` and
   `.factory/cycles/v1.0-brownfield-backfill/` cycle artifacts as of this amendment (2026-09-10).
   The backfill implementation's test fixtures MUST reproduce this heterogeneity, not assume a
   single idealized form:

   | Artifact | Primary record-start pattern | Confirmed exception form(s) | NOT a boundary (nested sub-structure) | Cross-cycle evidence |
   |---|---|---|---|---|
   | `decision-log.md` | `^\| D-[0-9]+ \|` (a decision-log table row) | `^### D-[0-9]+ \(` — an Appendix sub-clause-expansion block under the `## Appendix: Sub-clause Expansion` section; each such h3 block is a secondary atomic unit tied to its D-NNN row and MUST NOT itself be split mid-block, even though the table row remains the PRIMARY partition key | The `## Appendix: Sub-clause Expansion` heading is a section label, not a record | Engine: 144 table rows. Brownfield: 254 table rows. Table-row form CONFIRMED CORRECT already in both cycles |
   | `burst-log.md` | `^## ` (any h2 heading) | `^### Pass-[0-9]+ Fix Burst\b` — 2 confirmed real records in the engine cycle (`### Pass-39 Fix Burst — F5 Engine Discipline`, `### Pass-40 Fix Burst — F5 Engine Discipline`), sitting between the h2 records `## F5 pass-38 fix burst` and `## Burst: F5 pass-41 fix burst (2026-05-12)`, use h3 as their actual top-level heading instead of h2. The backfill MUST treat this exact pattern as an additional record boundary | `^### Block [0-9]+:` (brownfield's 8-block burst structure — `Block 1: Parent-commit` through `Block 8: factory-artifacts commit`), `^### Extracted banner content`, and any other `### ` line NOT matching the Pass-N-Fix-Burst exception pattern | Engine: 73 h2 records + 2 h3-exception records = 75 total. Brownfield: 81 h2 records, 0 h3-exception records (its 80 `### ` headings are all nested `Block N:` sub-structure) |
   | `lessons.md` | `^## L-<tag>-[0-9]+` (e.g. `## L-EDP1-[0-9]+`), OR `^## LESSON \(D-[0-9]+\)`, OR `^## RECURRENCE NOTE \(D-[0-9]+\)` | `^### L-<tag>-[0-9]+\b` — 2 confirmed real records in the engine cycle (`### L-EDP1-050`, `### L-EDP1-051`) use h3; the h2 form was adopted starting at `## L-EDP1-052`. The backfill MUST treat this exact pattern as an additional record boundary | None confirmed nested under a lessons.md record in either cycle | Engine live file starts at L-EDP1-050 (L-EDP1-001..049 already archived to `lessons-archive-pass1-49.md`, out of this BC's scope — the backfill operates only on the CURRENT monolithic file's content). Brownfield: 5 records, all `LESSON (D-NNNN)` / `RECURRENCE NOTE (D-NNNN)` h2 form, 0 h3-exception records |
   | `session-checkpoints.md` | `^## ` (any h2 heading — `## Session Resume Checkpoint (...)`, `## Checkpoint: ...`, `## Archived: ...`, `## Archived Checkpoint...`) | None confirmed | `^### ` (e.g. `### State`, `### §N. <label>`, `### Resume Path A`) — always nested substructure within the enclosing h2 checkpoint record in both cycles, never a boundary itself | Engine: 12 h2 records, 8 nested h3 sub-headings (all under one archived checkpoint). Brownfield: 181 h2 records, 811 nested h3 sub-headings |

   **Normalization rule (single implementable predicate).** A line is a record-boundary line if
   and only if: (a) it matches `^## ` (any h2); OR (b) it matches one of the two documented
   artifact-specific h3-exception patterns above (`^### Pass-[0-9]+ Fix Burst\b` for
   `burst-log.md`; `^### L-<tag>-[0-9]+\b` for `lessons.md`); OR (c) — for `decision-log.md` only —
   it matches `^\| D-[0-9]+ \|` as the PRIMARY partition key (the `### D-NNN (` Appendix h3 blocks
   are secondary atomic units that must never be split internally, but do not themselves define
   the primary shard-boundary grid). Every other `^#{3,4} ` line is nested sub-structure and MUST
   NOT be treated as a boundary. This predicate is exhaustive against every confirmed heading form
   in both `v1.0-feature-engine-discipline-pass-1/` and `v1.0-brownfield-backfill/` as of
   2026-09-10. If a future cycle introduces a heading form outside this enumeration,
   Postcondition 6's fail-loud content-preservation gate MUST reject the backfill run rather than
   silently mis-partition, and this BC MUST be amended to extend the marker table before the
   backfill is re-run.

3. **The full shard index is published for the complete pre-existing history in the same
   operation.** The resulting `<artifact-stem>.shard-index.toml` contains one `[[shard]]` entry
   per sealed partition (not just future seals) — `sealed_at` for these backfilled entries is set
   to the backfill operation's own execution timestamp (the true historical seal moments for
   PRE-existing content are not recoverable from the monolithic file alone, since it was never
   previously sharded; this is a documented, accepted approximation, not a defect — genuinely
   time-accurate `sealed_at` values only exist for shards sealed AFTER Layer 2 goes live).

4. **The backfill-split composes with BC-1.18.007's retention policy immediately.** If the number
   of shards a backfill-split produces for a given artifact already exceeds `retention_count`
   (plausible for `decision-log.md` at 908,938 bytes ÷ ~49,152-byte cap ≈ 19 shards, versus a
   default `retention_count` of 10), the retention/compaction archival move (BC-1.18.007
   Postcondition 2) applies to the OLDEST backfilled shards in the SAME operation — the backfill
   does not first produce an over-retention active set and defer archival to a later event.

5. **Atomicity: the backfill-split for a given artifact is all-or-nothing.** If the split
   operation is interrupted partway (e.g., process crash after writing 3 of 19 shard files), the
   operation MUST be safely re-runnable from scratch: it does not corrupt the original monolithic
   file until every resulting shard file AND the shard-index have been written to a staging
   location and validated (total byte count across all shards + fresh current file equals the
   original monolithic file's byte count, no record duplicated or dropped), at which point the
   staged results atomically replace the original monolithic file's role via the same
   temp-file-then-rename discipline BC-1.18.006 already establishes.

6. **Content-preservation verification is mandatory before the original monolithic file's role is
   retired.** The backfill-split MUST verify, before completing: (a) the concatenation of all
   sealed shards in `seq` order plus the final current file reproduces the original monolithic
   file's content byte-for-byte (modulo the shard/index metadata itself, which is new), and (b)
   every record (a decision-log.md `D-NNN` table row, a burst-log.md record per the
   Record-Boundary Marker Table — including its two confirmed `### Pass-N Fix Burst`
   h3-exception records, a lessons.md entry per the same table — including its two confirmed
   `### L-EDP1-050`/`### L-EDP1-051` h3-exception records, or a session-checkpoints.md h2
   checkpoint entry) that existed in the original file is present in exactly one resulting shard —
   never zero, never two. This verification is a hard gate: if it fails, the backfill-split aborts
   and the original monolithic file is left untouched (fail-loud, not partial-and-silent).

## Invariants

1. **This BC's split logic is a caller of BC-1.18.006's atomic-write primitives, not a
   reimplementation.** The backfill-split reuses the SAME temp-file-then-rename atomic-write
   pattern and the SAME shard-index schema BC-1.18.006 defines for the ongoing per-write case —
   this BC differs only in WHEN it runs (once, at F4 activation) and WHAT it operates on
   (pre-existing monolithic content, iteratively partitioned, rather than a single new block).

2. **No record is ever split across a shard boundary.** The partition points (Postcondition 2) are
   always at the structural record-boundary markers enumerated in Postcondition 2's
   Record-Boundary Marker Table, never mid-record, and never determined from heading level (h2 vs
   h3) alone — the marker table's documented h3-exception forms (burst-log.md's `### Pass-39/40
   Fix Burst` records; lessons.md's pre-`L-EDP1-052` `### L-EDP1-050`/`### L-EDP1-051` records)
   are record boundaries despite their heading level, while nested `### ` sub-headings (e.g.
   burst-log.md's `### Block N:` blocks) are never boundaries despite matching the same heading
   level.

3. **The backfill-split is idempotent against a shard-index that already exists for that
   artifact.** If a partial or complete backfill-split has already run for an artifact (e.g., a
   prior interrupted attempt left a valid partial shard-index), re-running the backfill MUST
   either resume from the last verified-complete shard or detect the already-sharded state and
   skip re-splitting (never double-split an already-sharded artifact into redundant shards).

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | An artifact's byte count at F4 execution time is smaller than `shard_cap_bytes` (e.g., a fresh cycle's `lessons.md` that hasn't yet grown large) | `ceil(current_bytes / shard_cap_bytes) = 1` — no sealing occurs; the file remains at the canonical name unchanged, but a shard-index IS still created (with zero `[[shard]]` entries and `current_shard` pointing at the unchanged file), so the artifact is "shard-index-registered" even though no split was structurally necessary |
| EC-002 | A single record (one D-NNN decision-log row) is itself larger than `shard_cap_bytes` | The backfill-split MUST NOT truncate or split the oversized record; that shard is allowed to exceed `shard_cap_bytes` for that ONE record only, and this is flagged in the shard-index entry (e.g., an `oversized_record: true` field) as a known, documented exception — consistent with `MAX_SINGLE_RECORD_BYTES`'s role in the cap formula as a margin allowance, not an absolute ceiling on any conceivable record |
| EC-003 | Backfill process crashes after writing shard files 1-3 of an expected 19 | Postcondition 5's atomicity guarantee: the original monolithic file is untouched (staging was incomplete), and the partial staged output is discarded on the next backfill attempt, which restarts cleanly from the original file |
| EC-004 | Content-preservation verification (Postcondition 6) finds a byte-count or record-count mismatch | Backfill aborts; original file left untouched; fail-loud error surfaced to the operator running the F4 migration (analogous to ADR-049's own one-time migration pattern, which this BC is explicitly modeled on) |
| EC-005 | The four artifacts have grown further between F2 (this BC's authoring) and F4 (its execution) — the illustrative byte counts in Precondition 3 are stale by then | Not a defect: Precondition 3 explicitly requires re-measurement at actual F4 execution time; the illustrative F2-era counts exist only to establish scale (5-19× over cap), not as literal backfill inputs |
| EC-006 | A monolithic `burst-log.md` or `lessons.md` contains a record whose heading level deviates from that artifact's dominant h2 form (e.g. burst-log.md's `### Pass-39 Fix Burst`/`### Pass-40 Fix Burst`, or lessons.md's pre-`L-EDP1-052` `### L-EDP1-050`/`### L-EDP1-051`) | The backfill MUST detect these via the Postcondition 2 Record-Boundary Marker Table's documented h3-exception patterns and treat them as record boundaries identical in kind to the artifact's h2 records; an h2-only detector that misses these records (silently no-ops or mid-record-splits) fails Postcondition 6's content-preservation gate and MUST abort per EC-004 |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `decision-log.md` at 908,938 bytes, cap 49,152 bytes, records never spanning a boundary | `ceil(908938/49152) = 19` shards produced (`decision-log.0001.md`..`decision-log.0018.md` sealed + `decision-log.md` fresh current); shard-index with 18 `[[shard]]` entries | happy-path |
| `lessons.md` at 234,731 bytes, cap 49,152 bytes | `ceil(234731/49152) = 5` shards (4 sealed + 1 current) | happy-path |
| Artifact at 40,000 bytes, cap 49,152 bytes (under cap) | 1 "shard" total = the unchanged current file; shard-index created with 0 sealed `[[shard]]` entries | edge-case |
| A single decision-log row of 60,000 bytes (exceeds 49,152-byte cap alone) | That shard's `bytes_at_seal = 60000 > shard_cap_bytes`, flagged `oversized_record: true`; NOT split mid-record | edge-case |
| Backfill interrupted after 3/19 shards written, restarted | Original file byte-identical to pre-crash state; restart produces the same 19-shard result as an uninterrupted run | error |
| `burst-log.md` fixture containing `## F5 pass-38 fix burst`, `### Pass-39 Fix Burst — F5 Engine Discipline`, `### Pass-40 Fix Burst — F5 Engine Discipline`, `## Burst: F5 pass-41 fix burst` in sequence, with `### Block N:` sub-headings nested inside the h2 records | Boundary detector produces exactly 4 records (pass-38, pass-39, pass-40, pass-41) — the two h3-exception records are each their own record; nested `### Block N:` sub-headings do NOT create additional record boundaries | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-123 | Content-preservation invariant — concatenation of all resulting shards (in `seq` order) plus the final current file reproduces the original monolithic file byte-for-byte | property test / golden-file round-trip against real (or synthetic fixture) monolithic files |
| VP-123 | Record-integrity invariant — every structural record present in the original file appears in EXACTLY ONE resulting shard | property test (record-count-conservation check against synthetic fixtures with known record counts) |
| VP-124 | Atomicity-under-interruption invariant — a simulated crash at any point during the split leaves the original file either fully intact or the split fully complete, never a partial/corrupt intermediate state | integration test / fault-injection (simulated crash at each of N write steps; assert post-recovery state is one of the two valid states) |
| VP-124 | Idempotency invariant — running the backfill-split twice against an already-sharded artifact does not produce duplicate or additional shards | integration test (double-invocation against a fixture with a pre-existing shard-index) |

**Fix-burst note (F-S2502-F2-003):** the second VP-124 row's Proof Method previously read "unit
test"; reconciled to the authoritative `VP-INDEX.md` v3.02 catalog assignment — VP-124 =
integration — matching the sibling VP-124 row above. No property content changed.

## Related BCs

- BC-1.18.005 — this BC applies BC-1.18.005's cap formula retroactively (depends on)
- BC-1.18.006 — this BC reuses BC-1.18.006's atomic-write and shard-index-schema primitives (depends on)
- BC-1.18.007 — this BC's output composes immediately with the retention policy if the backfill produces more shards than `retention_count` (depends on)
- BC-1.18.011 — the B2 BC-INDEX migration mirrors this BC's content-preservation/census/atomicity/rollback governance pattern for a content partition instead of a time partition (related to)
- BC-7.08.001 — the Cohort B fail-closed flip is gated on THIS BC completing (the existing oversized files must be split before flipping fail-closed, or the flip would immediately re-trigger the exact INDETERMINATE loop Layer 2 exists to eliminate) (depended on by)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — backfill-split entry point, reusing the seal/index-publish primitives
- `crates/last-amended-migrate/` — the crate's existing one-time-migration pattern (ADR-049's precedent) this BC's operational model follows

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

```
$ grep -oE "^pub fn rotate_changelog" crates/last-amended-migrate/src/rotate.rs
pub fn rotate_changelog
```

Confirms the `last-amended-migrate` crate's existing one-time-migration primitives this BC's
"applies an existing primitive retroactively, once" pattern (Invariant 1) is modeled on.

```
$ grep -oE "^pub enum HookResult" crates/hook-sdk/src/result.rs
pub enum HookResult
```

Confirms `HookResult::Error` (EC-004's fail-loud abort outcome) is a real SDK contract variant.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-123, VP-124 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-123 (proptest / golden-file; content-preservation byte-for-byte + record integrity), VP-124 (integration; atomicity-under-interruption + fail-loud preservation gate E-SHD-003 + idempotency). Cap-constant numeric bound PROVISIONAL-until-F4.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies the "mandatory one-time backfill-split" the capability's own description names explicitly: "without it, this capability would only prevent future overflow and never shrink the artifacts already producing the majority of observed INDETERMINATE events, an incomplete delivery of its own stated purpose." |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-01 (Hook Dispatcher Core — one-time backfill migration logic in `shard_manager.rs`) |
| ADR | ADR-051 §Decision 2 ("Immediate consequence the story draft did not anticipate — a one-time backfill split is required") |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.2 | 2026-09-10 | product-owner | Fresh-context adversarial review found PC2 internally self-contradictory on burst-log record-boundary granularity: PC2 named an h3 (`### <burst-heading>`) boundary phrase in one clause while PC2's own "never split" clause and PC6(b)'s record-integrity clause both correctly said h2 — the erroneous h3 phrase misled implementation toward `### ` as the burst-log/lessons.md record marker, which silently no-ops backfill on the two largest artifacts (finds ≤2 boundaries against real h2-keyed content) and splits records mid-record where nested `### Block N:` sub-headings occur inside brownfield burst records. Reconciled: removed the erroneous h3 phrase from PC2; PC2 and PC6(b) now unambiguously key burst-log.md and session-checkpoints.md on `## ` (h2) boundaries. Added an authoritative Record-Boundary Marker Table to PC2, derived from direct inspection of the real `v1.0-feature-engine-discipline-pass-1/` and `v1.0-brownfield-backfill/` cycle artifacts, enumerating the exact record-start pattern per artifact and two confirmed real-world h3-exception record forms that a naive h2-only rule would miss: burst-log.md's `### Pass-39/40 Fix Burst` records (engine cycle, sitting between two h2 records) and lessons.md's pre-L-EDP1-052 `### L-EDP1-050`/`### L-EDP1-051` records (the L-EDP1-052+ form switched to h2). Added a single implementable normalization predicate covering all confirmed forms across both cycles, including decision-log.md's table-row primary key plus its Appendix sub-clause h3 blocks as a secondary non-splittable unit. Tightened Invariant 2 to cite the marker table and to forbid keying partition points on heading level alone. Added EC-006 and a corresponding canonical test vector covering the h3-exception detection requirement. No change to the split algorithm's core semantics (Postconditions 1, 3, 4, 5), Atomicity, Idempotency, or the artifact's CAP-043 anchor. |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-003 + F-S2502-F2-007): VP-124's idempotency row Proof Method reconciled from "unit test" to "integration" per VP-INDEX v3.02's authoritative method assignment (both VP-124 rows now consistently read "integration test"); the atomicity row's wording tightened to lead with "integration test" for internal consistency. Added `## SDK Grounding Evidence` section with literal stable-anchor grep output for `write_atomic`, `rotate_changelog`, and `HookResult`. No postcondition/invariant content change. Related BCs gained a cross-reference to the new BC-1.18.011 (B2 migration BC modeled on this BC's governance pattern). |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, not in the original F1 enumeration — required per ADR-051 Decision 2's finding that AC-002/AC-003 only gate future writes). One-time backfill-split of the four pre-existing oversized cycle append-log files, record-boundary-safe partitioning, content-preservation verification gate, composition with the retention policy. CAP-043 capability anchor. ADR-051 §D2 citation. |
