---
document_type: behavioral-contract
level: L3
version: "1.7"
status: draft
producer: product-owner
timestamp: 2026-09-10T06:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f1-delta-analysis.md
  - .factory/specs/verification-properties/VP-INDEX.md
input-hash: "dc4b072"
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

2. **Split algorithm: partition pre-existing content into sealed shards via greedy, boundary-preserving
   packing, with `ceil(current_bytes / shard_cap_bytes)` as the algorithm's LOWER BOUND on resulting
   shard count — never its exact target.** The monolithic file's content is partitioned at
   the structural record-boundary markers native to the artifact's own format. Boundary detection
   for all four mandatory artifacts MUST use the exact patterns enumerated in the
   **Record-Boundary Marker Table** below — never at an arbitrary byte offset that could split a
   single record (a D-NNN row, a burst-log record, a lessons.md entry, a session-checkpoint entry)
   across two shard files, and never keyed on heading LEVEL alone: h2-only detection is
   insufficient (see the marker table's confirmed h3-exception record forms, which an h2-only
   detector would either silently skip — causing backfill to no-op on the artifact's true content
   — or mis-treat a nested `### ` sub-heading as a false boundary, splitting a record mid-record).

   **Packing procedure (deterministic, pure function of content + `shard_cap_bytes`).** The packer
   walks the ordered list of records (as delimited by the Record-Boundary Marker Table) and
   accumulates whole records into the CURRENT shard, in original-file order, until appending the
   next whole record would push the current shard's byte size past `shard_cap_bytes`. At that
   point the current shard is sealed AS-IS — even if it has slack remaining below
   `shard_cap_bytes`, since no partial record may ever be appended to close that slack — and a new
   shard begins with that next record. This is the ONLY split-point rule; there is no secondary
   rule that rebalances shards toward an exact target count.

   **`ceil()` is a lower bound, not an exact count.** For a NON-uniform record-size distribution
   (which is the norm, not the exception, across all four mandatory artifacts — see the
   Record-Boundary Marker Table's own cross-cycle evidence), greedy boundary-preserving packing
   does not in general achieve the information-theoretic minimum shard count. Example: five
   40-byte records against a 70-byte cap: `ceil(200/70) = 3`, but no boundary-preserving packing
   can fit two 40-byte records into one 70-byte shard (80 > 70), so the greedy packer actually
   produces 5 shards. In general, the actual resulting shard count for any of the four mandatory
   artifacts is `>= ceil(current_bytes / shard_cap_bytes)`, with equality holding only in the
   special case where every shard's greedy-packed content leaves less than one additional whole
   record's worth of slack under the cap. `ceil(current_bytes / shard_cap_bytes)` remains useful
   as a deterministic LOWER-BOUND sizing estimate (pre-allocating shard-index capacity,
   progress-reporting denominators, and the retention-composition reasoning in Postcondition 4)
   but MUST NOT be asserted, in this BC's own Canonical Test Vectors, in tests, or in any
   downstream documentation, as the exact resulting shard count — the exact count is whatever the
   greedy boundary-preserving packer actually produces for that artifact's real record-size
   distribution at F4 execution time.

   Each resulting shard's byte size is `<= shard_cap_bytes` (BC-1.18.005's per-artifact effective
   cap, via the Cross-Validator Minimum Rule) — this per-shard bound is unaffected by the count
   correction above; it is the packing CONSTRAINT the greedy procedure enforces at every seal
   decision, not a claim about the total number of shards it takes to satisfy that constraint.
   EC-002's oversized-single-record exception (a record itself larger than `shard_cap_bytes`)
   remains the sole documented exception to this per-shard bound. The LAST partition becomes the
   fresh "current" file at the canonical name (per BC-1.18.006's stable-current-filename
   convention); all partitions before it are sealed with sequential `seq` numbers starting at 1,
   in chronological (original-file-order) sequence.

   **Record-Boundary Marker Table (authoritative).** Verified by direct inspection of the real
   on-disk content of both `.factory/cycles/v1.0-feature-engine-discipline-pass-1/` and
   `.factory/cycles/v1.0-brownfield-backfill/` cycle artifacts as of this amendment (2026-09-10).
   The backfill implementation's test fixtures MUST reproduce this heterogeneity, not assume a
   single idealized form:

   | Artifact | Primary record-start pattern | Confirmed exception form(s) | NOT a boundary (nested sub-structure) | Cross-cycle evidence |
   |---|---|---|---|---|
   | `decision-log.md` | `^\| D-[0-9]+(\([a-z0-9/]+\)\|-[A-Za-z]+)? \|` (this amendment, F-C3-P7-002 — a decision-log table row, covering the bare `\| D-NNN \|` form AND the two confirmed sub-clause suffix forms below) | Sub-clause suffix forms already present as PRIMARY table rows (not a separate boundary class — the corrected primary-key regex matches them directly): (i) parenthetical single- or multi-letter suffixes, e.g. `\| D-440(a) \|` and the combined `\| D-446(a/b/c/d/e) \|` form; (ii) the hyphenated non-parenthetical suffix `\| D-355-AMEND \|`. Separately, `^### D-[0-9]+ \(` — an Appendix sub-clause-expansion block under the `## Appendix: Sub-clause Expansion` section; each such h3 block is a secondary atomic unit tied to its D-NNN row and MUST NOT itself be split mid-block, even though the table row remains the PRIMARY partition key | The `## Appendix: Sub-clause Expansion` heading is a section label, not a record | Engine: 144 table rows total (109 bare `\| D-NNN \|` + 34 parenthetical-suffix + 1 hyphenated-suffix `D-355-AMEND` — re-verified by direct inspection, this amendment). Brownfield: 265 table rows as of this amendment (grown from the prior 254-row measurement per EC-005's documented staleness precedent; all 265 are the bare form — 0 parenthetical or hyphenated exceptions in that cycle). Corrected regex CONFIRMED to match every row in both cycles (144/144 engine, 265/265 brownfield) |
   | `burst-log.md` | `^## ` (any h2 heading) | `^### Pass-[0-9]+ Fix Burst\b` — 2 confirmed real records in the engine cycle (`### Pass-39 Fix Burst — F5 Engine Discipline`, `### Pass-40 Fix Burst — F5 Engine Discipline`), sitting between the h2 records `## F5 pass-38 fix burst` and `## Burst: F5 pass-41 fix burst (2026-05-12)`, use h3 as their actual top-level heading instead of h2. The backfill MUST treat this exact pattern as an additional record boundary | `^### Block [0-9]+:` (brownfield's 8-block burst structure — `Block 1: Parent-commit` through `Block 8: factory-artifacts commit`), `^### Extracted banner content`, and any other `### ` line NOT matching the Pass-N-Fix-Burst exception pattern | Engine: 73 h2 records + 2 h3-exception records = 75 total. Brownfield: 81 h2 records, 0 h3-exception records (its 80 `### ` headings are all nested `Block N:` sub-structure) |
   | `lessons.md` | `^## L-<tag>-[0-9]+` (e.g. `## L-EDP1-[0-9]+`), OR `^## LESSON \(D-[0-9]+\)`, OR `^## RECURRENCE NOTE \(D-[0-9]+\)` | `^### L-<tag>-[0-9]+\b` — 2 confirmed real records in the engine cycle (`### L-EDP1-050`, `### L-EDP1-051`) use h3; the h2 form was adopted starting at `## L-EDP1-052`. The backfill MUST treat this exact pattern as an additional record boundary | None confirmed nested under a lessons.md record in either cycle | Engine live file starts at L-EDP1-050 (L-EDP1-001..049 already archived to `lessons-archive-pass1-49.md`, out of this BC's scope — the backfill operates only on the CURRENT monolithic file's content). Brownfield: 5 records, all `LESSON (D-NNNN)` / `RECURRENCE NOTE (D-NNNN)` h2 form, 0 h3-exception records |
   | `session-checkpoints.md` | `^## ` (any h2 heading — `## Session Resume Checkpoint (...)`, `## Checkpoint: ...`, `## Archived: ...`, `## Archived Checkpoint...`) | None confirmed | `^### ` (e.g. `### State`, `### §N. <label>`, `### Resume Path A`) — always nested substructure within the enclosing h2 checkpoint record in both cycles, never a boundary itself | Engine: 12 h2 records, 8 nested h3 sub-headings (all under one archived checkpoint). Brownfield: 181 h2 records, 811 nested h3 sub-headings |

   **Normalization rule (this amendment, F-C3-P4-001: a per-artifact-scoped restatement of the
   Record-Boundary Marker Table above — the table is the authoritative, per-artifact source of
   truth; this rule is a summary predicate for implementation convenience, NOT an independent or
   additive source of boundaries).** A line is a record-boundary line for a given artifact if and
   only if it matches THAT artifact's own row in the Record-Boundary Marker Table, applied
   per-artifact as follows — clause (a)'s "any h2" wording holds AS WRITTEN only for the two
   artifacts whose marker-table row says "any h2 heading"; for the other two artifacts it is
   OVERRIDDEN by the artifact-specific restriction below, never read as a global disjunct that
   applies additively across all four artifacts regardless of which one is being partitioned:

   - **`burst-log.md`** and **`session-checkpoints.md`** (marker-table row: "any h2 heading"):
     (a) it matches `^## ` (any h2) — the PRIMARY partition key for these two artifacts; OR
     (b) — `burst-log.md` only — it matches the documented h3-exception pattern
     `^### Pass-[0-9]+ Fix Burst\b`.
   - **`lessons.md`** (marker-table row: tagged h2 forms only, NOT any h2): (a) it matches one of
     the three documented TAGGED h2 forms — `^## L-<tag>-[0-9]+`, `^## LESSON \(D-[0-9]+\)`, or
     `^## RECURRENCE NOTE \(D-[0-9]+\)` — an untagged `^## ` line (any other h2 heading) is NOT a
     boundary for this artifact; OR (b) it matches the documented h3-exception pattern
     `^### L-<tag>-[0-9]+\b`.
   - **`decision-log.md`** (marker-table row: `| D-NNN |` table rows only, NOT any h2): (a) it
     matches `^\| D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|` (this amendment, F-C3-P7-002) as the
     PRIMARY partition key — this single regex covers the bare `| D-NNN |` form, the parenthetical
     sub-clause-suffix form (`| D-440(a) |`, `| D-446(a/b/c/d/e) |`), and the hyphenated-suffix form
     (`| D-355-AMEND |`) as ONE primary-key class, not three; a bare `^## ` line (e.g.
     `## Decisions Log`, `## Appendix: Sub-clause Expansion`) is a section label, NOT a boundary,
     for this artifact; the `^### D-[0-9]+ \(` Appendix h3 blocks are secondary atomic units that
     must never be split internally, but do not themselves define the primary shard-boundary grid.

   Every other `^#{2,4} ` line not matching one of the per-artifact clauses above (including a
   bare `## Decisions Log` / `## Appendix: Sub-clause Expansion` heading in `decision-log.md`, and
   an untagged `## ` aside in `lessons.md`) is nested sub-structure or a section label, and MUST
   NOT be treated as a boundary. This predicate is exhaustive against every confirmed heading form
   in both `v1.0-feature-engine-discipline-pass-1/` and `v1.0-brownfield-backfill/` as of
   2026-09-10. If a future cycle introduces a heading form outside this enumeration,
   Postcondition 6's fail-loud content-preservation gate MUST reject the backfill run rather than
   silently mis-partition, and this BC MUST be amended to extend the marker table before the
   backfill is re-run.

   **Clarification (this amendment, F-C3-P7-002): the `## Appendix: Sub-clause Expansion` section's
   cap-bounding when it folds into the final record.** Because the Appendix's `### D-NNN (...)` h3
   blocks do not themselves define shard-boundary points for `decision-log.md` (stated above), the
   packer treats the ENTIRE `## Appendix: Sub-clause Expansion` section — its heading plus every h3
   block it contains — as ONE trailing atomic packing unit, appended after the shard that seals the
   file's LAST `| D-NNN(...) |` table row; it is never itself subdivided at an inter-block boundary.
   When this trailing atomic unit's bytes push that shard's total past `shard_cap_bytes`, the shard
   is flagged `oversized_record: true` under the SAME EC-002 oversized-atomic-unit exception already
   established for a single oversized record and for Postcondition 2's degenerate oversized-preamble
   case — NOT a fail-loud abort. This is the REAL, current-measured case for the engine cycle's
   `decision-log.md`: its `## Appendix: Sub-clause Expansion` section measures 74,989 bytes as of
   2026-09-10 (direct inspection, this amendment), already larger on its own than the illustrative
   49,152-byte cap, so it will trigger this exception in practice, not merely in theory.

   **Note (O-1, this amendment): recognized-stem non-empty-oracle invariant.** Each of the four
   recognized artifacts (`decision-log.md`, `burst-log.md`, `lessons.md`,
   `session-checkpoints.md`) ALWAYS yields a non-empty oracle boundary set for non-empty content —
   each always contains at least one `| D-NNN |` row / h2 / tagged-h2 marker per the Record-
   Boundary Marker Table above. Consequently, a "recognized stem + empty oracle ⇒ trust caller"
   code branch is UNREACHABLE in production for any of the four artifacts and exists only to
   support synthetic test inputs that deliberately construct an empty-content fixture; the
   unrecognized-stem case (a file stem outside the four recognized names) is separately and fully
   covered by Postcondition 6's fail-loud gate above. This note documents the invariant; it does
   not change detection behavior.

   **Leading-preamble handling rule (this amendment, F-C3-P3-001).** Every one of the four
   mandatory artifacts begins with content that precedes its first record-boundary line — the
   LEADING PREAMBLE. Per direct inspection of the real `v1.0-brownfield-backfill/` and
   `v1.0-feature-engine-discipline-pass-1/` artifacts as of this amendment (2026-09-10), the
   preamble comprises: the YAML frontmatter block (`---` ... `---`), any title heading and
   introductory prose, and — for `decision-log.md` only — the markdown table header row plus its
   `|---|...|` separator row that precede the first `| D-NNN |` record line. (One observed
   instance, `v1.0-feature-engine-discipline-pass-1/burst-log.md`, has a near-empty preamble — a
   leading blank line and a bare `---` rule, no frontmatter or title — confirming the preamble is a
   real, per-artifact-instance quantity that MUST be measured at backfill time, never assumed
   fixed or non-empty.) The preamble is a single atomic, indivisible packing unit: like a record,
   it is NEVER split across a shard boundary — splitting a YAML frontmatter block or a table header
   row mid-structure would corrupt the artifact's own format, not merely misplace a boundary.

   Let `preamble_bytes` be the leading preamble's byte length and `first_record_bytes` be the byte
   length of the first record in file order. The packer resolves the preamble once, before
   record-based greedy packing begins, as follows:

   - **Normal case — `preamble_bytes + first_record_bytes <= shard_cap_bytes`:** the preamble rides
     in the SAME shard as the first record (and as many subsequent whole records as fit under the
     greedy packing rule) — no separate preamble-only shard is created. This is the expected
     outcome for all four mandatory artifacts at their current measured preamble sizes (a few
     hundred bytes at most — two to three orders of magnitude below any calibrated
     `shard_cap_bytes`), so this rule preserves today's actual packing behavior unless a future
     record happens to leave near-zero headroom under the cap.
   - **Overflow case — `preamble_bytes + first_record_bytes > shard_cap_bytes`:** the preamble
     seals as its OWN shard (`seq=1`), containing zero records, BEFORE record-based packing begins.
     Greedy packing then starts fresh at the first record, which opens the next shard. This is the
     ONLY way the per-shard cap bound below can be guaranteed to hold for shard 1 without folding
     the preamble into a record it does not fit alongside — the unsanctioned cap violation this
     overflow-case rule exists specifically to prevent (F-C3-P3-001: folding the preamble
     unconditionally into the first record's shard, with no overflow check, is the defect this rule
     corrects).
   - **Degenerate case — the preamble ALONE exceeds `shard_cap_bytes`** (not reachable for any of
     the four mandatory artifacts at their current measured preamble sizes, but specified for a
     future artifact or a drastically lowered cap): the preamble seals as its own oversized shard,
     using the SAME EC-002 oversized-atomic-unit exception already established below for an
     oversized single record — NOT a fail-loud abort. EC-002's rationale ("content atomicity beats
     the cap for a single indivisible unit that cannot be split without corrupting structure")
     applies identically to the preamble. This shard is flagged in the shard index with the SAME
     `oversized_record: true` field EC-002 defines, broadened here to cover any oversized atomic
     packing unit — a record OR the leading preamble — not literally only a domain record, so
     downstream tooling sees one consistent oversized-content signal regardless of which kind of
     atomic unit triggered it.

   A shard produced by either the overflow case or the degenerate case is a **preamble shard**: it
   holds zero domain records and is flagged `is_preamble_shard: true` in its `[[shard]]` index
   entry (new field, this amendment), distinguishing it from an ordinary record-bearing shard for
   downstream readers — including Postcondition 3's index publication, Postcondition 4's retention
   composition, and Postcondition 6(b)'s record-count accounting below — without those readers
   having to re-derive record count from file content.

3. **The full shard index is published for the complete pre-existing history in the same
   operation.** The resulting `<artifact-stem>.shard-index.toml` contains one `[[shard]]` entry
   per sealed partition (not just future seals) — `sealed_at` for these backfilled entries is set
   to the backfill operation's own execution timestamp (the true historical seal moments for
   PRE-existing content are not recoverable from the monolithic file alone, since it was never
   previously sharded; this is a documented, accepted approximation, not a defect — genuinely
   time-accurate `sealed_at` values only exist for shards sealed AFTER Layer 2 goes live). A
   preamble shard produced by Postcondition 2's Leading-Preamble Handling Rule receives a
   `[[shard]]` entry exactly like a record-bearing shard — same `seq` numbering, same `sealed_at`
   convention — plus `is_preamble_shard: true` and `records: 0`, and (in the degenerate case only)
   `oversized_record: true`.

   **Backfill Recovery Manifest (this amendment, F-C3-P6-001).** The published shard-index
   additionally carries a `[backfill_manifest]` table, populated in the SAME atomic index-publish
   write as the `[[shard]]` entries themselves. This manifest is the SOLE authoritative basis
   Postcondition 5's crash-recovery/idempotency determination relies on (see Postcondition 5's
   Recovery-Confirmation Rule and Invariant 3) — never a structural comparison of the canonical
   file's own bytes against a re-concatenation of already-sealed shards. It contains four fields,
   each computed exactly once, at split time, from the SAME `original_content` buffer already read
   into memory to drive Postcondition 2's partitioning:

   - `original_bytes` (u64) — the pre-split monolithic file's exact byte length.
   - `original_sha256` (hex-encoded string) — the pre-split monolithic file's SHA-256 content hash.
   - `final_bytes` (u64) — the byte length of the LAST partition (the content the canonical file is
     intended to hold once Postcondition 5's canonical-truncate step completes).
   - `final_sha256` (hex-encoded string) — the SHA-256 content hash of that same last partition.

   Because the manifest is written in the SAME atomic write as the `[[shard]]` entries — durably
   persisted BEFORE the canonical-truncate step that follows it in Postcondition 5's sequence — it
   is guaranteed to exist and be trustworthy for every recovery-confirmation check a later
   invocation performs, including one landing in the crash window Postcondition 5 names below.

4. **The backfill-split composes with BC-1.18.007's retention policy immediately.** If the number
   of shards a backfill-split produces for a given artifact already exceeds `retention_count`
   (plausible for `decision-log.md` at 908,938 bytes ÷ ~49,152-byte cap: `ceil() = 19` shards as a
   LOWER BOUND per Postcondition 2 — the actual greedy-packed count MAY be higher — versus a
   default `retention_count` of 10, the inequality `actual_count >= 19 > 10` holds regardless of
   exactly how much higher than 19 the real packed count turns out to be), the retention/compaction
   archival move (BC-1.18.007 Postcondition 2) applies to the OLDEST backfilled shards in the SAME
   operation, keyed on the ACTUAL packed shard count (never the `ceil()` estimate) — the backfill
   does not first produce an over-retention active set and defer archival to a later event. **A
   preamble shard (Postcondition 2's Leading-Preamble Handling Rule) is NOT exempted from
   `retention_count` accounting or archival eligibility, and is not specially retained as a
   permanent "head."** It counts toward the actual packed shard count and is archived under the
   SAME oldest-first rule as any other backfilled shard; since a preamble shard is always `seq=1`
   (chronologically the oldest possible content), it is typically the FIRST candidate BC-1.18.007's
   archival move considers once the artifact accumulates more than `retention_count` shards. This
   is consistent with BC-1.18.006's own precedent for the ongoing per-write case: a rolled
   canonical file's header/frontmatter is swept into whatever shard is sealed at the time and is
   never treated as permanent or specially preserved once superseded.

5. **Atomicity: the backfill-split for a given artifact is all-or-nothing.** If the split
   operation is interrupted partway (e.g., process crash after writing 3 of 19 shard files), the
   operation MUST be safely re-runnable from scratch: it does not corrupt the original monolithic
   file until every resulting shard file AND the shard-index have been written to a staging
   location and validated (total byte count across all shards + fresh current file equals the
   original monolithic file's byte count, no record duplicated or dropped), at which point the
   staged results atomically replace the original monolithic file's role via the same
   temp-file-then-rename discipline BC-1.18.006 already establishes.

   **Two-phase publish and the index-publish/canonical-truncate crash window.** The
   staged-and-validated results become durable in TWO SEQUENTIAL writes, not one: (i) the
   shard-index — including every `[[shard]]` entry AND the Backfill Recovery Manifest
   (Postcondition 3) — is published first, via `write_atomic`; (ii) the canonical file is then
   truncated-in-place to hold ONLY the final (unsealed) partition's bytes. A crash between (i) and
   (ii) is a real, reachable window distinct from the pre-staging-complete window the paragraph
   above already covers: the index (and manifest) are durable, but the canonical file still holds
   the COMPLETE pre-split content. On any later invocation against the same artifact — where
   Invariant 3's idempotency short-circuit finds the shard-index already exists — this window MUST
   be distinguished from (a) the SAFE window where step (ii) already completed, and from (b) a
   genuinely corrupted/tampered on-disk state matching neither.

   **Recovery-confirmation rule (this amendment, F-C3-P6-001): exact whole-file identity against
   the Backfill Recovery Manifest — NEVER a structural byte-prefix heuristic.** The
   recovery-confirmation determination reads the canonical file's CURRENT on-disk bytes exactly
   once and computes their exact byte length and SHA-256 content hash, then compares that
   `(length, hash)` pair against the Backfill Recovery Manifest's two recorded pairs:

   - If `(length, hash) == (final_bytes, final_sha256)` — the canonical file already holds EXACTLY
     the intended final-partition content — the prior run's step (ii) already completed. SAFE
     window: no write is performed; the invocation reports the artifact as already migrated.
   - If `(length, hash) == (original_bytes, original_sha256)` — the canonical file still holds
     EXACTLY the original pre-split monolithic content, byte-for-byte unmodified — step (ii) never
     ran (or crashed before writing any bytes). DANGEROUS window, unambiguously confirmed: recovery
     completes the interrupted step (ii) via the **Manifest-Authoritative Slice-and-Verify Rule**
     (this amendment, F-C3-P7-001) below — never by writing content the manifest is presumed to
     store (it does not), and never by writing an unverified slice.

   **Manifest-Authoritative Slice-and-Verify Rule (this amendment, F-C3-P7-001): the manifest is a
   VERIFIER of the healed content, not a STORE of it — the healed bytes are always obtained by
   slicing, and a slice is NEVER written without first being confirmed against the manifest.** The
   Backfill Recovery Manifest (Postcondition 3) contains `final_bytes` (a LENGTH) and `final_sha256`
   (a HASH) — it records no content of its own. Consequently the DANGEROUS-window heal has no
   manifest-stored bytes to copy; the healed content MUST be derived by slicing the canonical file's
   own current bytes, and that derivation is safe only because the confirmed DANGEROUS window
   already established `canonical_bytes.len() == original_bytes`. The heal proceeds in three
   ordered steps, each a hard gate before the next:

   1. **Derive the slice offset from the Manifest, never from the shard index.** `offset =
      original_bytes - final_bytes` (both operands read from the Backfill Recovery Manifest itself,
      already confirmed durable and trustworthy per the two-phase publish ordering above). The slice
      is `canonical_bytes[offset..]`. The offset MUST NOT be computed by summing the shard-index's
      per-shard `bytes_at_seal` fields — that field records what each SEALED shard was AT THE TIME
      IT WAS SEALED and is a separate, independently-corruptible piece of on-disk state from the
      Manifest; deriving the recovery offset from it (rather than from the Manifest's own
      `original_bytes`/`final_bytes` arithmetic) is the defect this amendment corrects (see the
      Rationale paragraph below for the concrete counterexample).
   2. **Verify the slice against the Manifest before writing anything.** The heal MUST assert both
      `sliced.len() == final_bytes` AND `sha256(sliced) == final_sha256`. This verification is the
      LOAD-BEARING guarantee this rule exists to provide — it is what makes the healed write safe
      even if step 1's offset derivation were ever wrong (a future regression, a corrupted Manifest
      value, or a legacy caller that still derives the offset from the shard index), because a wrong
      slice fails this check and is never written.
   3. **On success, write; on ANY mismatch, fail loud and write nothing.** If both checks in step 2
      pass, the verified slice is atomically written to the canonical file via `write_atomic`
      (Invariant 1), completing the interrupted step (ii). If EITHER check fails, the heal MUST NOT
      write anything to the canonical file — it fails loud with `E-SHD-012` (NEW — added to
      `prd-supplements/error-taxonomy.md` in this SAME burst) and halts for operator investigation,
      exactly like Postcondition 5's top-level AMBIGUOUS disposition below, but distinguished from
      it: `E-SHD-012` fires ONE LEVEL DEEPER, after the top-level `(length, hash)` check has already
      confirmed the DANGEROUS window, during the mandatory pre-write slice verification — it signals
      that the Manifest's own `final_bytes`/`final_sha256` fields (or the code deriving the slice)
      are themselves in an inconsistent state, not that the canonical file's top-level identity is
      ambiguous.

   **Rationale — why "manifest stores the content" was unimplementable and how a prior pass
   exploited the gap.** v1.6's DANGEROUS-window text said recovery heals by "writing the manifest's
   own recorded `final_bytes` content ... never re-derived by slicing the current canonical bytes at
   an offset." This is incoherent against Postcondition 3's own `[backfill_manifest]` schema, which
   stores only `final_bytes` (a length) and `final_sha256` (a hash) — no content field. A prior
   implementation pass, correctly observing that the manifest holds no content to copy, sliced
   `canonical_bytes[sealed_len..]` where `sealed_len` was computed by SUMMING the shard-index's
   per-shard `bytes_at_seal` entries — a plausible-looking but uncorroborated source — and wrote the
   result WITHOUT verifying it against `final_bytes`/`final_sha256` at all. A corrupted or stale
   `bytes_at_seal` value on any one sealed shard therefore yields a wrong `sealed_len`, a wrong
   slice, and a SILENT mis-heal (record duplication or truncation) written with no check — precisely
   the class of defect the Backfill Recovery Manifest was introduced (v1.6, F-C3-P6-001) to
   eliminate. The Manifest-Authoritative Slice-and-Verify Rule closes this by making the offset
   itself Manifest-derived (removing the shard-index-summation dependency entirely) AND making the
   resulting slice's length and hash a mandatory pre-write gate (removing the "write without
   verifying" step regardless of how the offset was derived).
   - If `(length, hash)` matches NEITHER recorded pair — the on-disk state is AMBIGUOUS (the
     canonical file has been modified by something other than this migration's own two-phase
     publish sequence, or is corrupted). This MUST fail loud (`E-SHD-011`) and MUST NOT write
     anything to the canonical file; recovery halts for operator investigation. Silently defaulting
     to either the SAFE or DANGEROUS disposition on an ambiguous match is exactly the class of
     defect this rule exists to prevent (see Invariant 3).

   **Why a structural byte-prefix heuristic is insufficient (the defect this rule corrects,
   F-C3-P6-001, HIGH).** A prior implementation pass classified the DANGEROUS window by checking
   whether the canonical file's leading bytes structurally reproduce the concatenation of the
   artifact's own already-sealed shards (`canonical_bytes[..sealed_concat.len()] ==
   sealed_concat`). This heuristic FALSE-POSITIVES whenever the artifact's real content
   legitimately repeats a sealed shard's exact bytes as a PREFIX of the SAFE-window final
   partition — which Postcondition 2's own Record-Boundary Marker Table confirms is a realistic,
   non-adversarial occurrence for `session-checkpoints.md` in particular (no uniqueness requirement
   is imposed on checkpoint headings or bodies). Concrete counterexample: let `A =
   "## Checkpoint\nx\n"` (16 bytes) and original content `A + A + "more\n"` (37 bytes) with
   `shard_cap_bytes = 16`. Greedy packing (Postcondition 2) seals the first `A` as shard 1
   (`sealed_concat = A`, 16 bytes) and leaves `A + "more\n"` (21 bytes) as the final, unsealed
   partition — a legitimate, fully-correct SAFE-window state after the first, uninterrupted run. On
   a SECOND invocation, `canonical_bytes = A + "more\n"` structurally satisfies
   `canonical_bytes[..16] == sealed_concat` (its own leading 16 bytes happen to be byte-identical to
   the sealed shard's content, because the artifact's real content legitimately repeats it) — the
   byte-prefix heuristic misclassifies this SAFE state as the DANGEROUS window and overwrites the
   canonical file with `canonical_bytes[16..]` alone (`"more\n"`), DESTROYING the legitimate `A`
   prefix that belongs to the final partition — permanent, silent data loss of a real record's
   heading and body. The Recovery-Confirmation Rule above eliminates this failure mode entirely: it
   never compares the canonical file's bytes against a re-concatenation of sealed shards at all —
   only against the two whole-file `(length, SHA-256)` pairs recorded once, durably, in the
   Backfill Recovery Manifest at split time, which cannot coincidentally collide with a shorter
   prefix repetition the way a prefix-match can.

6. **Content-preservation verification is mandatory before the original monolithic file's role is
   retired.** The backfill-split MUST verify, before completing: (a) the concatenation of all
   sealed shards in `seq` order plus the final current file reproduces the original monolithic
   file's content byte-for-byte (modulo the shard/index metadata itself, which is new), and (b)
   every record (a decision-log.md `D-NNN` table row, a burst-log.md record per the
   Record-Boundary Marker Table — including its two confirmed `### Pass-N Fix Burst`
   h3-exception records, a lessons.md entry per the same table — including its two confirmed
   `### L-EDP1-050`/`### L-EDP1-051` h3-exception records, or a session-checkpoints.md h2
   checkpoint entry) that existed in the original file is present in exactly one resulting shard —
   never zero, never two, and (c) **(this amendment, F-C3-P3-001)** every sealed shard's
   `bytes_at_seal` is `<= shard_cap_bytes`, EXCEPT a shard explicitly flagged `oversized_record:
   true` in the shard index (EC-002's oversized-single-record exception, or this amendment's
   degenerate oversized-preamble exception under Postcondition 2's Leading-Preamble Handling Rule)
   — an unflagged over-cap shard is exactly the unsanctioned Postcondition 2 violation Layer 2
   exists to eliminate, and MUST fail this gate precisely like a byte-count or record-count
   mismatch; this per-shard-cap check MUST be executed explicitly against the actual bytes written
   to each sealed shard file, never merely assumed to hold because the packing procedure ran.

   **Ruling (this amendment, F-C3-P6-002): "the actual bytes written to disk" — in clause (c)
   above, in this Postcondition's own governing sentence, and in Invariant 4 — means a POST-HOC
   READ-BACK of each sealed shard file from disk, via a FRESH file read performed AFTER that
   shard's write completes, compared against the in-memory partition buffer that was intended to
   be written.** Checking only the in-memory partition buffer's length before issuing the write —
   with no subsequent disk read-back — does NOT satisfy checks (a), (b), or (c): it cannot detect a
   write that silently truncated, partially flushed, or otherwise landed corrupted bytes on disk,
   which is precisely the failure mode this gate exists to catch BEFORE the original monolithic
   file is retired and its content becomes unrecoverable except via git history. Concretely, the
   staging sequence is: stage each partition in memory (Postcondition 2) -> write each partition to
   its staged/temp path -> READ BACK each just-written file from disk -> verify the read-back
   bytes' length (and, for checks (a)/(b), their full byte-for-byte content and record accounting)
   against the in-memory partition that was intended -> only THEN proceed to Postcondition 5's
   atomic index-publish and canonical-truncate sequence. This is the safer of the two possible
   readings, and the one CLAUDE.md's production-grade default requires for a ONE-TIME migration
   that overwrites/deletes its own source content: an in-memory-only check cannot detect a failed
   or corrupted disk write before the source is gone, whereas a post-hoc disk read-back can. This
   ruling changes no Postcondition/Invariant/EC/VP semantic content beyond disambiguation — checks
   (a), (b), and (c) already described a post-hoc verification against "actual bytes" — it only
   forecloses the in-memory-only reading a prior implementation pass took as "compliant with
   intent," which the shipped code (checking `partition.bytes.len()` before any write, never
   reading a sealed shard file back from disk) did not in fact satisfy.

   Checks (a), (b), and (c) are each a hard gate: if any fails, the backfill-split aborts and the
   original monolithic file is left untouched (fail-loud, not partial-and-silent).

   **Extension to the DANGEROUS-window heal write (this amendment, F-C3-P7-001): the same
   disk-read-back discipline applies to Postcondition 5's Manifest-Authoritative Slice-and-Verify
   Rule.** The DANGEROUS-window heal performs the SAME kind of one-time, destructive,
   source-overwriting write this clause already governs for sealed-shard writes — it is the final
   write of the interrupted migration, permanently replacing the canonical file's content. It MUST
   therefore receive the SAME post-hoc verification this clause requires for sealed shards: after
   the verified slice (Postcondition 5, step 3) is written via `write_atomic`, the heal MUST perform
   a FRESH read-back of the canonical file from disk and confirm its `(length, SHA-256)` equals
   `(final_bytes, final_sha256)` before reporting the heal complete. An in-memory-only confidence
   that the write succeeded (i.e., trusting `write_atomic`'s own return value with no subsequent
   read) does NOT satisfy this requirement, for the identical reason clause (c) already gives for
   sealed shards: it cannot detect a write that silently truncated, partially flushed, or otherwise
   landed corrupted bytes on disk, at the exact moment (post-heal) when the pre-heal content is
   irretrievably gone. If the read-back does not match, the heal fails loud with `E-SHD-012` — the
   heal's own destructive write already happened at that point, so this final check exists to
   surface the corruption immediately (for operator remediation from git history / backup) rather
   than let a silently-corrupted canonical file pass as healed.

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

3. **The backfill-split is idempotent against a shard-index that already exists for that artifact,
   and the recovery-confirmation determination MUST use Postcondition 5's exact whole-file
   `(length, SHA-256)` comparison against the Backfill Recovery Manifest — NEVER a structural
   byte-prefix comparison against a re-concatenation of already-sealed shards (this amendment,
   F-C3-P6-001). The Manifest is the sole authoritative basis this determination relies on because
   it AUTHORIZES AND VERIFIES the bytes a heal writes — it is NOT required to, and by Postcondition
   3's own schema does NOT, STORE those bytes (this amendment, F-C3-P7-001).** `final_bytes` is a
   length and `final_sha256` is a hash; the healed content itself is always obtained by slicing the
   canonical file's current bytes (Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule),
   and that slice is never written unverified — every healed byte range MUST match the Manifest's
   `(final_bytes, final_sha256)` pair, checked explicitly against the actual sliced bytes, before
   the write is issued. If a partial or complete backfill-split has already run for an artifact
   (e.g., a prior interrupted attempt left a valid, fully-published shard-index), re-running the
   backfill MUST resolve to exactly one of three dispositions: (a) confirm the SAFE window
   (canonical bytes exactly match the manifest's `final_bytes`/`final_sha256`) and take no action;
   (b) complete the interrupted canonical-truncate step for the confirmed DANGEROUS window
   (canonical bytes exactly match the manifest's `original_bytes`/`original_sha256`) by writing the
   Manifest-verified slice per Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule — a
   slice that fails that rule's verification step is NOT this disposition; it fails loud with
   `E-SHD-012` instead (see (c) below, extended); or (c) fail loud for an AMBIGUOUS or
   inconsistent on-disk state: `E-SHD-011` for a top-level `(length, hash)` match to NEITHER
   manifest value, or `E-SHD-012` for a confirmed DANGEROUS window whose Manifest-derived slice
   itself fails the `(final_bytes, final_sha256)` verification (this amendment, F-C3-P7-001). Never
   silently overwrite the canonical file on an ambiguous, unverified, or heuristically-inferred
   match, and never double-split an already-sharded artifact into redundant shards. A structural
   byte-prefix heuristic is explicitly insufficient for this determination — see Postcondition 5's
   concrete counterexample — because it cannot distinguish a genuinely-interrupted
   canonical-truncate from an artifact whose legitimately fully-migrated final partition happens to
   begin with content byte-identical to an already-sealed shard, which Postcondition 2's own
   Record-Boundary Marker Table confirms is a realistic, non-adversarial occurrence for
   `session-checkpoints.md` (no uniqueness requirement is imposed on checkpoint headings or bodies).

4. **(this amendment, F-C3-P3-001; ruling tightened F-C3-P6-002) Every sealed shard satisfies the
   per-shard cap bound, enforced as a fail-loud verification gate, never merely implied by the
   packer's own behavior.** Postcondition 6(c) requires this to be checked explicitly, post-hoc,
   against the actual bytes written to disk for every sealed shard — not inferred from the packing
   procedure (including Postcondition 2's Leading-Preamble Handling Rule) having run correctly, and
   NOT satisfied by checking only the in-memory partition buffer's length before the write is
   issued: "actual bytes written to disk" means a FRESH READ-BACK of the sealed shard file from
   disk after the write completes, per Postcondition 6's F-C3-P6-002 ruling. The two documented
   exceptions — EC-002's oversized single record, and this amendment's degenerate oversized-preamble
   case — are the ONLY conditions under which an over-cap shard is sanctioned, and both MUST be
   explicitly flagged (`oversized_record: true`) in the shard index for the verification gate to
   recognize them as sanctioned rather than as a failure. This invariant is what prevents the
   backfill from re-creating the very over-cap-shard condition Layer 2 exists to eliminate.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | An artifact's byte count at F4 execution time is smaller than `shard_cap_bytes` (e.g., a fresh cycle's `lessons.md` that hasn't yet grown large) | `ceil(current_bytes / shard_cap_bytes) = 1` — no sealing occurs; the file remains at the canonical name unchanged, but a shard-index IS still created (with zero `[[shard]]` entries and `current_shard` pointing at the unchanged file), so the artifact is "shard-index-registered" even though no split was structurally necessary |
| EC-002 | A single record (one D-NNN decision-log row) is itself larger than `shard_cap_bytes` | The backfill-split MUST NOT truncate or split the oversized record; that shard is allowed to exceed `shard_cap_bytes` for that ONE record only, and this is flagged in the shard-index entry (e.g., an `oversized_record: true` field) as a known, documented exception — consistent with `MAX_SINGLE_RECORD_BYTES`'s role in the cap formula as a margin allowance, not an absolute ceiling on any conceivable record |
| EC-003 | Backfill process crashes after writing shard files 1-3 of an expected 19 | Postcondition 5's atomicity guarantee: the original monolithic file is untouched (staging was incomplete), and the partial staged output is discarded on the next backfill attempt, which restarts cleanly from the original file |
| EC-004 | Content-preservation verification (Postcondition 6) finds a byte-count or record-count mismatch | Backfill aborts; original file left untouched; fail-loud error surfaced to the operator running the F4 migration (analogous to ADR-049's own one-time migration pattern, which this BC is explicitly modeled on) |
| EC-005 | The four artifacts have grown further between F2 (this BC's authoring) and F4 (its execution) — the illustrative byte counts in Precondition 3 are stale by then | Not a defect: Precondition 3 explicitly requires re-measurement at actual F4 execution time; the illustrative F2-era counts exist only to establish scale (5-19× over cap), not as literal backfill inputs |
| EC-006 | A monolithic `burst-log.md` or `lessons.md` contains a record whose heading level deviates from that artifact's dominant h2 form (e.g. burst-log.md's `### Pass-39 Fix Burst`/`### Pass-40 Fix Burst`, or lessons.md's pre-`L-EDP1-052` `### L-EDP1-050`/`### L-EDP1-051`) | The backfill MUST detect these via the Postcondition 2 Record-Boundary Marker Table's documented h3-exception patterns and treat them as record boundaries identical in kind to the artifact's h2 records; an h2-only detector that misses these records (silently no-ops or mid-record-splits) fails Postcondition 6's content-preservation gate and MUST abort per EC-004 |
| EC-007 | **(this amendment, F-C3-P3-001)** The leading preamble (YAML frontmatter + title/intro, plus — for decision-log.md — the table header/separator rows) combined with the first record's bytes exceeds `shard_cap_bytes`, though the first record ALONE does not | Per Postcondition 2's Leading-Preamble Handling Rule overflow case, the preamble seals as its own zero-record shard (`is_preamble_shard: true`) BEFORE record packing begins; the first record then opens a fresh shard. Distinct from EC-002: the record itself is under cap — only the record-plus-preamble combination is not — so folding them together (the naive behavior this finding corrects) would be an unsanctioned Postcondition 2 violation, not a documented exception |
| EC-008 | **(this amendment, F-C3-P3-001)** The leading preamble ALONE exceeds `shard_cap_bytes` (not reachable for any of the four mandatory artifacts at their current measured preamble sizes — two to three orders of magnitude below any calibrated cap — but specified for completeness) | Per Postcondition 2's Leading-Preamble Handling Rule degenerate case, the preamble seals as its own oversized shard using the SAME EC-002 oversized-atomic-unit exception (`oversized_record: true`), NOT a fail-loud abort — content atomicity for an indivisible structural unit (a frontmatter/title block that cannot be split without corrupting the artifact's format) takes precedence over the cap, identically to EC-002's rationale for an oversized record |
| EC-009 | **(this amendment, F-C3-P6-001)** Repeated-prefix content: a sealed shard's content recurs byte-identically as the LEADING PREFIX of the legitimately-migrated final partition's own content (realistic for `session-checkpoints.md`, since Postcondition 2's Record-Boundary Marker Table imposes no uniqueness requirement on checkpoint headings/bodies) — e.g. `A + A + "more\n"` with `A` as both the sole sealed shard's content and the final partition's own leading bytes | Postcondition 5's Recovery-Confirmation Rule correctly classifies this as the SAFE window (canonical bytes exactly match the Backfill Recovery Manifest's `final_bytes`/`final_sha256`) and takes NO action, preserving the canonical file byte-for-byte. A structural byte-prefix heuristic would misclassify this as the DANGEROUS window and destroy the final partition's legitimate leading bytes — this is the exact false-positive data-loss failure mode F-C3-P6-001 corrects |
| EC-010 | **(this amendment, F-C3-P6-001)** At recovery-confirmation time, the canonical file's `(length, SHA-256)` matches NEITHER the Backfill Recovery Manifest's `original_bytes`/`original_sha256` NOR its `final_bytes`/`final_sha256` (e.g. an operator manually edited the canonical file between a crash and the recovery attempt, or the file is corrupted) | AMBIGUOUS state per Postcondition 5's Recovery-Confirmation Rule: fails loud with `E-SHD-011`; the canonical file is NOT written to under any circumstance; recovery halts pending operator investigation — never silently defaults to either the SAFE or DANGEROUS disposition |
| EC-011 | **(this amendment, F-C3-P7-001)** At the confirmed DANGEROUS window (canonical bytes already match the Manifest's `original_bytes`/`original_sha256`), the slice derived at the Manifest-authoritative offset `original_bytes - final_bytes` does NOT satisfy `sliced.len() == final_bytes AND sha256(sliced) == final_sha256` (e.g. the Manifest's own `final_bytes`/`final_sha256` fields are corrupted or internally inconsistent with the canonical file's actual content, or a legacy/buggy caller derived the slice from a corrupted shard-index `bytes_at_seal` sum instead of the Manifest) | Distinct from EC-010: the TOP-LEVEL `(length, hash)` check already confirmed DANGEROUS, but the Manifest-Authoritative Slice-and-Verify Rule's own pre-write verification (Postcondition 5, step 2) fails ONE LEVEL DEEPER. The heal MUST fail loud with `E-SHD-012` (NEW, distinct from `E-SHD-011`) and MUST NOT write anything to the canonical file; recovery halts pending operator investigation — never silently writes an unverified slice |
| EC-012 | **(this amendment, F-C3-P7-002)** A `decision-log.md` fixture contains sub-clause-suffixed rows — `\| D-440(a) \|`, the combined-suffix form `\| D-446(a/b/c/d/e) \|`, and the hyphenated form `\| D-355-AMEND \|` — interleaved with ordinary bare `\| D-NNN \|` rows | The corrected primary-key regex (`^\| D-[0-9]+(\([a-z0-9/]+\)\|-[A-Za-z]+)? \|`) detects EVERY one of these forms as a record boundary identical in kind to a bare `\| D-NNN \|` row; the prior bare-only regex (`^\| D-[0-9]+ \|`) silently failed to match any of them, under-segmenting the artifact by absorbing each sub-clause row into the preceding record — this is the defect F-C3-P7-002 corrects |

## Canonical Test Vectors

**NEEDS-UPDATE (F-S2502-F4-004, this amendment):** the three rows marked `NEEDS-UPDATE` below
previously asserted an EXACT shard count equal to `ceil(bytes/cap)`. Postcondition 2 now documents
`ceil()` as a LOWER BOUND only — the exact count is whatever greedy boundary-preserving packing
produces against the real record-size distribution of the actual fixture file, which is `>= ceil()`
and equal to it only when records happen to pack without slack. **test-writer MUST replace the `N`
placeholders below with the actual measured packed-shard count** obtained by running the real
packing algorithm against the real (or a byte-faithful synthetic) `decision-log.md` /
`lessons.md` fixture — NOT by re-asserting the `ceil()` arithmetic as if it were exact.

| Input | Expected Output | Category |
|-------|----------------|----------|
| `decision-log.md` at 908,938 bytes, cap 49,152 bytes, records never spanning a boundary | **NEEDS-UPDATE:** `N` shards produced (`decision-log.0001.md`..`decision-log.000<N-1 zero-padded>.md` sealed + `decision-log.md` fresh current), where `N = ` the actual greedy-packed count and `N >= ceil(908938/49152) = 19` (lower bound only, not the asserted value); shard-index with `N-1` `[[shard]]` entries | happy-path |
| `lessons.md` at 234,731 bytes, cap 49,152 bytes | **NEEDS-UPDATE:** `N` shards (`N-1` sealed + 1 current), where `N >= ceil(234731/49152) = 5` (lower bound only, not the asserted value) | happy-path |
| Artifact at 40,000 bytes, cap 49,152 bytes (under cap) | 1 "shard" total = the unchanged current file; shard-index created with 0 sealed `[[shard]]` entries (unaffected by this amendment: a single-shard, under-cap artifact never has a record-boundary slack question — `ceil(40000/49152) = 1` is always exactly achievable) | edge-case |
| A single decision-log row of 60,000 bytes (exceeds 49,152-byte cap alone) | That shard's `bytes_at_seal = 60000 > shard_cap_bytes`, flagged `oversized_record: true`; NOT split mid-record (unaffected by this amendment: this vector asserts a per-shard bound exception via EC-002, not a total shard count) | edge-case |
| Backfill interrupted after 3/`N` shards written (`N` = the actual greedy-packed count for `decision-log.md`, `N >= 19` per the row above), restarted | **NEEDS-UPDATE (count only; behavior unchanged):** original file byte-identical to pre-crash state; restart produces the same `N`-shard result as an uninterrupted run — deterministic, since the greedy packer is a pure function of the original content and `shard_cap_bytes` | error |
| `burst-log.md` fixture containing `## F5 pass-38 fix burst`, `### Pass-39 Fix Burst — F5 Engine Discipline`, `### Pass-40 Fix Burst — F5 Engine Discipline`, `## Burst: F5 pass-41 fix burst` in sequence, with `### Block N:` sub-headings nested inside the h2 records | Boundary detector produces exactly 4 records (pass-38, pass-39, pass-40, pass-41) — the two h3-exception records are each their own record; nested `### Block N:` sub-headings do NOT create additional record boundaries (unaffected by this amendment: this vector asserts record-boundary DETECTION, not shard COUNT) | edge-case |
| **(this amendment, F-C3-P3-001, EC-007)** A `decision-log.md`-shaped fixture whose leading preamble (frontmatter + title + intro + table header/separator rows, measured at fixture-build time) plus its first `\| D-NNN \|` record row together exceed `shard_cap_bytes`, while the first record alone does not | Shard `seq=1` contains ONLY the preamble — `is_preamble_shard: true`, `records: 0` in the shard-index entry, `bytes_at_seal = preamble_bytes <= shard_cap_bytes`; shard `seq=2` opens with the first record; Postcondition 6(c)'s per-shard-cap gate passes for both shards without either being flagged `oversized_record` | edge-case |
| **(this amendment, F-C3-P3-001, EC-008)** A synthetic fixture whose leading preamble alone (independent of any record) exceeds `shard_cap_bytes` | Shard `seq=1` contains ONLY the oversized preamble, flagged `oversized_record: true` AND `is_preamble_shard: true` in the shard-index entry, `bytes_at_seal > shard_cap_bytes` for this one shard only; backfill does NOT abort (Postcondition 2's degenerate case is a documented, index-flagged exception, not a fail-loud condition); Postcondition 6(c)'s gate recognizes the flag and treats this shard as sanctioned | edge-case |
| **(this amendment, F-C3-P6-001, EC-009)** `session-checkpoints.md`-shaped fixture: `A + A + "more\n"` where `A = "## Checkpoint\nx\n"` (16 bytes), `shard_cap_bytes = 16`. Backfill run ONCE (seals shard 1 = `A`; final/current partition = `A + "more\n"`, 21 bytes; Backfill Recovery Manifest records `original_bytes=37`/`original_sha256=<hash of A+A+"more\n">`, `final_bytes=21`/`final_sha256=<hash of A+"more\n">`), then `run_mechanism_a_backfill_split` invoked a SECOND time against the SAME artifact with no intervening crash | Recovery-confirmation reads canonical bytes (`A + "more\n"`, 21 bytes) and finds `(21, hash) == (final_bytes, final_sha256)` — SAFE window — returns `AlreadyMigrated`; NO write is issued; canonical file remains byte-for-byte `A + "more\n"` (21 bytes) — the second `A` and `"more\n"` are NOT lost. Pins the F-C3-P6-001 fix directly (a byte-prefix heuristic would instead overwrite the canonical file with just `"more\n"`, 5 bytes) | edge-case |
| Same `A + A + "more\n"` / `shard_cap_bytes = 16` fixture, but the canonical file is set to still hold the FULL pre-split content (`A + A + "more\n"`, 37 bytes) while the shard-index + Backfill Recovery Manifest are already durably published (simulating a crash between Postcondition 5's index-publish and canonical-truncate writes) | **(this amendment, F-C3-P7-001, row content corrected)** Recovery-confirmation finds `(37, hash) == (original_bytes, original_sha256)` — DANGEROUS window, unambiguously confirmed. The Manifest-Authoritative Slice-and-Verify Rule derives `offset = original_bytes - final_bytes = 37 - 21 = 16` (from the Manifest alone, never from summed shard-index `bytes_at_seal`), takes `sliced = canonical_bytes[16..]` = `A + "more\n"` (21 bytes), and verifies `sliced.len() == final_bytes` (`21 == 21`, holds) AND `sha256(sliced) == final_sha256` (holds — this fixture's slice is genuinely correct). Both checks pass, so the slice is atomically written to the canonical file, then read back from disk and re-verified against `(final_bytes, final_sha256)`; returns `Healed { sealed_count: 1 }` | error |
| **(this amendment, F-C3-P7-001, EC-011)** Same `A + A + "more\n"` / `shard_cap_bytes = 16` fixture and the same confirmed-DANGEROUS on-disk state as the row above (`canonical_bytes = A + A + "more\n"`, 37 bytes, matching `original_bytes`/`original_sha256`), but the Backfill Recovery Manifest's `final_bytes`/`final_sha256` fields are corrupted on disk to record `final_bytes=21` with a `final_sha256` that does NOT match `sha256(A + "more\n")` (e.g. bit-rot, or a manually-edited shard-index TOML) | The Manifest-Authoritative Slice-and-Verify Rule derives the SAME `offset = 16` and the SAME `sliced = A + "more\n"` (21 bytes) — `sliced.len() == final_bytes` still holds (`21 == 21`), but `sha256(sliced) == final_sha256` FAILS (the corrupted hash does not match). The heal MUST NOT write anything to the canonical file; fails loud with `E-SHD-012`; recovery halts pending operator investigation. Pins the F-C3-P7-001 fix directly — a heal that skipped this verification step (or derived the slice from a corrupted shard-index `bytes_at_seal` sum instead of the Manifest) would have written the unverified/wrong slice silently | error |
| Same fixture, but the canonical file's bytes match NEITHER `(original_bytes, original_sha256)` NOR `(final_bytes, final_sha256)` (e.g. an operator appended text to the canonical file between the crash and the recovery attempt) | AMBIGUOUS: recovery fails loud with `E-SHD-011`; the canonical file is NOT written to; the artifact is left exactly as found, pending operator investigation (EC-010) | error |
| **(this amendment, F-C3-P7-002, EC-012)** `decision-log.md` fixture containing, in original-file order: `\| D-100 \|` (bare), `\| D-101(a) \|`, `\| D-101(b) \|` (single-letter parenthetical suffixes), `\| D-102(a/b/c) \|` (combined-suffix parenthetical), `\| D-103-AMEND \|` (hyphenated suffix), `\| D-104 \|` (bare) | The corrected primary-key regex (`^\| D-[0-9]+(\([a-z0-9/]+\)\|-[A-Za-z]+)? \|`) detects all 6 rows as 6 distinct record boundaries, in original-file order — none is absorbed into a neighboring record. Against the OLD bare-only regex (`^\| D-[0-9]+ \|`), only the 2 bare rows (`D-100`, `D-104`) would be detected, silently under-segmenting the fixture to 2 records instead of 6 — the exact defect F-C3-P7-002 corrects | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-123 | Content-preservation invariant — concatenation of all resulting shards (in `seq` order) plus the final current file reproduces the original monolithic file byte-for-byte | property test / golden-file round-trip against real (or synthetic fixture) monolithic files |
| VP-123 | Record-integrity invariant — every structural record present in the original file appears in EXACTLY ONE resulting shard | property test (record-count-conservation check against synthetic fixtures with known record counts) |
| VP-123 | **(this amendment, F-C3-P3-001)** Per-shard-cap invariant — every sealed shard's `bytes_at_seal` is `<= shard_cap_bytes` UNLESS explicitly flagged `oversized_record: true` in the shard index (EC-002 or the degenerate oversized-preamble case, EC-008), enforced as Postcondition 6(c)'s fail-loud hard gate | property test / golden-file (assert `bytes_at_seal <= shard_cap_bytes` for every `[[shard]]` entry not carrying `oversized_record: true`; assert the two flagged-exception fixtures — EC-002, EC-008 — are the ONLY entries where the bound is exceeded) |
| VP-124 | Atomicity-under-interruption invariant — a simulated crash at any point during the split leaves the original file either fully intact or the split fully complete, never a partial/corrupt intermediate state | integration test / fault-injection (simulated crash at each of N write steps; assert post-recovery state is one of the two valid states) |
| VP-124 | Idempotency invariant — running the backfill-split twice against an already-sharded artifact does not produce duplicate or additional shards | integration test (double-invocation against a fixture with a pre-existing shard-index) |
| VP-124 | **(this amendment, F-C3-P6-001)** Recovery-confirmation correctness invariant — the SAFE/DANGEROUS/AMBIGUOUS disposition of a re-invocation against an existing shard-index is determined SOLELY by exact whole-file `(length, SHA-256)` comparison against the Backfill Recovery Manifest (Postcondition 5), NEVER by a structural byte-prefix comparison; in particular, content where a sealed shard's bytes recur as a genuine prefix of the legitimately-migrated final partition (EC-009) is correctly classified SAFE and left byte-for-byte untouched, and an on-disk state matching neither manifest value fails loud (`E-SHD-011`, EC-010) rather than silently overwriting | integration test / fixture-based (the three Canonical Test Vector rows above — SAFE no-op, DANGEROUS heal, AMBIGUOUS fail-loud — plus a repeated-prefix fixture family generalizing EC-009 across varying prefix-repeat lengths) |
| VP-124 | **(this amendment, F-C3-P7-001)** Heal slice-verification invariant — at the confirmed DANGEROUS window, the healed content is ALWAYS obtained by slicing the canonical file's current bytes at the Manifest-authoritative offset (`original_bytes - final_bytes`, never from a summed shard-index `bytes_at_seal`), and that slice is NEVER written unless it independently satisfies `sliced.len() == final_bytes AND sha256(sliced) == final_sha256`; a slice that fails this verification (EC-011) fails loud (`E-SHD-012`) with no write, and the completed heal's own write additionally receives the SAME post-hoc disk read-back verification Postcondition 6(c)/Invariant 4 requires for sealed-shard writes | integration test / fixture-based (the corrected DANGEROUS-heal row above — genuine slice passes and is written and read-back-verified — plus the EC-011 corrupted-Manifest row — slice fails verification, `E-SHD-012`, no write) |

**Fix-burst note (F-S2502-F2-003):** the second VP-124 row's Proof Method previously read "unit
test"; reconciled to the authoritative `VP-INDEX.md` v3.02 catalog assignment — VP-124 =
integration — matching the sibling VP-124 row above. No property content changed.

**Fix-burst note (F-C3-P6-001/F-C3-P6-002, this amendment):** VP-124 gains a third facet
(Recovery-confirmation correctness invariant, Postcondition 5) — VP citation change; architect
must propagate this facet to `VP-INDEX.md`, `verification-architecture.md`, and
`verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). No new
VP-NNN allocated — this is an additional PROPERTY STATEMENT under the existing VP-124 grant
(mirroring how VP-123 gained its per-shard-cap facet under F-C3-P3-001 in v1.4), not a request for
a new ID. F-C3-P6-002 (the PC6(c)/Invariant-4 disk-read-back ruling) introduces no new VP row: it
is a wording disambiguation of the EXISTING VP-123 per-shard-cap facet's "actual bytes" language,
not a new property.

**Fix-burst note (F-C3-P7-001, this amendment):** VP-124 gains a FOURTH facet (Heal
slice-verification invariant, Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule) — VP
citation change; architect must propagate this facet to `VP-INDEX.md`,
`verification-architecture.md`, and `verification-coverage-matrix.md` per
`vp_index_is_vp_catalog_source_of_truth` (POLICY 9), landing alongside the still-outstanding
third-facet propagation from v1.6 (and the VP-123 per-shard-cap facet propagation still outstanding
from v1.4). No new VP-NNN allocated — same additive-facet convention as the prior three.

**Fix-burst note (F-C3-P7-002, this amendment):** No new VP-NNN or facet is warranted for the
decision-log.md marker-table regex correction — it is a mechanical fix to an existing detection
predicate, not a new property. The EXISTING VP-123 Record-integrity facet's Proof Method (second
VP-123 row above) is scoped to include `decision-log.md` fixtures containing the parenthetical and
hyphenated sub-clause-suffix forms (EC-012) among its "synthetic fixtures with known record
counts" — test-writer must ensure the corrected regex's fixture coverage (EC-012's CTV row) is
exercised under this existing facet, not a new one.

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

- VP-123, VP-124 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-123 (proptest / golden-file; content-preservation byte-for-byte + record integrity), VP-124 (integration; atomicity-under-interruption + fail-loud preservation gate E-SHD-003 + idempotency). Cap-constant numeric bound PROVISIONAL-until-F4. **(this amendment, F-C3-P3-001):** VP-123 gains a third facet (per-shard-cap fail-loud invariant, Postcondition 6(c)) — VP citation change; architect must propagate this facet to `VP-INDEX.md`, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). **(this amendment, F-C3-P6-001):** VP-124 gains a third facet (Recovery-confirmation correctness invariant — exact whole-file manifest comparison, never a byte-prefix heuristic, Postcondition 5) — a second VP citation change routed to architect under the SAME `vp_index_is_vp_catalog_source_of_truth` propagation obligation (POLICY 9), to land in the SAME propagation pass as the still-outstanding VP-123 third-facet propagation from v1.4. **(this amendment, F-C3-P7-001):** VP-124 gains a FOURTH facet (Heal slice-verification invariant — the DANGEROUS-window heal's slice is Manifest-derived and Manifest-verified before every write, Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule) — a third VP citation change routed to architect under the SAME `vp_index_is_vp_catalog_source_of_truth` propagation obligation (POLICY 9), to land in the SAME propagation pass as the still-outstanding VP-123 third-facet propagation (v1.4) and VP-124 third-facet propagation (v1.6). F-C3-P7-002 (the decision-log.md marker-table regex correction) introduces no VP citation change — it extends the EXISTING VP-123 Record-integrity facet's fixture coverage, not a new facet or ID (see the Verification Properties section's F-C3-P7-002 fix-burst note).

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
| 1.7 | 2026-09-10 | product-owner | Adjudication of two findings from a FRESH LOCAL adversarial pass on S-25.02 F4 cluster-3 (the pass immediately following the v1.6 cross-vendor burst). **F-C3-P7-001 (HIGH) — PC5 DANGEROUS-heal incoherent with the PC3 Manifest schema (manifest stores only `final_bytes`/`final_sha256` — a LENGTH and a HASH, no content — yet v1.6 said heal 'writes the manifest's own recorded `final_bytes` content ... never re-derived by slicing the current canonical bytes at an offset'), RESOLVED.** The v1.6 text was literally unimplementable from the manifest alone; a prior implementation pass, correctly noting the manifest holds no content, instead sliced `canonical_bytes[sealed_len..]` with `sealed_len` computed by SUMMING the shard-index's per-shard `bytes_at_seal` fields — an independently-corruptible source distinct from the Manifest — and wrote the result WITHOUT verifying it against `final_bytes`/`final_sha256` at all. A corrupted or stale `bytes_at_seal` therefore yields a wrong slice written with no check: a silent mis-heal (record duplication/truncation), exactly the class of defect the Backfill Recovery Manifest (v1.6, F-C3-P6-001) was introduced to eliminate. ADJUDICATED: added the **Manifest-Authoritative Slice-and-Verify Rule** to Postcondition 5 — the healed content is derived by slicing `canonical_bytes` at the Manifest-derived offset `original_bytes - final_bytes` (both fields read from the Manifest; the shard-index `bytes_at_seal` sum is no longer used for this purpose at all), and that slice MUST satisfy `sliced.len() == final_bytes AND sha256(sliced) == final_sha256` BEFORE it is written — on any mismatch, the heal fails loud with **NEW `E-SHD-012`** (added to `prd-supplements/error-taxonomy.md` in this SAME burst) and writes nothing. Invariant 3 rewritten: the Manifest "authorizes/verifies the bytes written, it is not required to store them"; the old "never re-derived by slicing" phrasing is replaced with "never written unverified — every healed byte range MUST match the manifest's `(final_bytes, final_sha256)` before the write," and the idempotency disposition list is extended to name `E-SHD-012` alongside `E-SHD-011` for the two distinct fail-loud cases (top-level three-way mismatch vs. a confirmed-DANGEROUS slice that itself fails Manifest verification). **Also closes the adversary's PC6(c)/Invariant-4 observation:** the DANGEROUS heal's own destructive write previously got no post-hoc disk read-back (unlike sealed-shard writes, per PC6(c)'s F-C3-P6-002 ruling); Postcondition 6(c) gained an explicit **Extension to the DANGEROUS-window heal write** paragraph, and Invariant 4's scope note now covers it, requiring the SAME fresh-read-back-and-compare-to-`(final_bytes, final_sha256)` discipline after the heal's `write_atomic` call. Added EC-011 (Manifest-verification mismatch at a confirmed DANGEROUS window — fails loud, `E-SHD-012`, no write) and a matching Canonical Test Vector; corrected the existing DANGEROUS-heal CTV row's expected-behavior text (it previously asserted the now-incoherent "NOT re-derived by slicing" claim — corrected to walk through the Manifest-derived offset, the two-part verification, and the post-hoc read-back, all passing for that fixture). Added VP-124's FOURTH facet (Heal slice-verification invariant) — VP citation change routed to architect per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9), to land alongside the still-outstanding VP-123 third-facet (v1.4) and VP-124 third-facet (v1.6) propagations. **F-C3-P7-002 (MEDIUM) — decision-log.md marker-table regex misses real `\| D-NNN(x) \|` sub-clause rows, RESOLVED.** The PC2 Record-Boundary Marker Table's `decision-log.md` primary key, `^\| D-[0-9]+ \|`, requires digits immediately followed by ` \|` and cannot match a real sub-clause row like `\| D-440(a) \|`. Direct re-inspection of the live `.factory/cycles/v1.0-feature-engine-discipline-pass-1/decision-log.md` (2026-09-10) confirms 144 total `\| D-...` rows = 109 bare `\| D-NNN \|` + 34 parenthetical-suffix rows (single-letter forms `(a)`..`(e)` and the combined form `(a/b/c/d/e)`, e.g. `D-446(a/b/c/d/e)`..`D-449(a/b/c/d/e)`) + 1 previously-undocumented hyphenated-suffix row, `D-355-AMEND` — none of the 35 non-bare rows matched the old regex, so the marker table's own "144 table rows ... CONFIRMED CORRECT" evidence was unachievable by the regex it cited. The sibling brownfield cycle's `decision-log.md` was also re-inspected: 265 rows as of this amendment (grown from the prior 254-row measurement, consistent with EC-005's documented staleness precedent), all bare form, 0 sub-clause exceptions in that cycle. ADJUDICATED: corrected the primary-key regex to `^\| D-[0-9]+(\([a-z0-9/]+\)\|-[A-Za-z]+)? \|` — confirmed by direct grep to match every row in both cycles (144/144 engine, 265/265 brownfield, zero unmatched). Updated the Normalization rule's `decision-log.md` bullet to cite the corrected regex and enumerate the two suffix forms as ONE primary-key class (not three). Reconciled the marker table's evidence cell with the exact 109+34+1=144 breakdown and the re-measured 265-row brownfield count. Added EC-012 (sub-clause rows detected as boundaries) and a matching Canonical Test Vector (6-row fixture: bare, single-letter-parenthetical ×2, combined-parenthetical, hyphenated, bare — corrected regex detects all 6 as distinct boundaries; old regex would detect only 2). **Answered the adversary's Appendix cap-bounding question (one-line clarification, no spec-behavior change):** added a clarification paragraph confirming the `## Appendix: Sub-clause Expansion` section — since its h3 blocks do not themselves define shard-boundary points — is packed as ONE trailing atomic unit after the shard sealing the file's last `\| D-NNN(...) \|` row, and is flagged `oversized_record: true` under the EXISTING EC-002 exception when it does not fit; direct measurement confirms this is the REAL case for the engine cycle (Appendix section = 74,989 bytes, already over the illustrative 49,152-byte cap on its own). No new VP citation for this finding: the EXISTING VP-123 Record-integrity facet's fixture coverage is extended (fix-burst note added) to include sub-clause-suffix forms; no new facet or ID. **Implementer scope (routed, not yet executed by this burst):** `heal_or_confirm_already_migrated`'s DANGEROUS-window branch must be restructured to derive the slice offset from `(original_bytes, final_bytes)` (never from summed `bytes_at_seal`), verify `(sliced.len(), sha256(sliced))` against `(final_bytes, final_sha256)` before writing, surface `E-SHD-012` on mismatch, and perform a post-hoc disk read-back verification after the heal's write, mirroring the sealed-shard staging sequence's existing read-back step; the decision-log.md boundary-detection regex must be updated to the corrected pattern. **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 — no `bcs:` frontmatter array change (already listed); story-writer should confirm the story body's PC5/PC6/Invariant-3/4 summaries and any decision-log.md boundary-regex mentions reflect this amendment. **`prd-supplements/error-taxonomy.md` amended in the SAME burst** (v1.10→v1.11, `E-SHD-012` added) — not deferred. **Input-hash recompute owed to state-manager** (this file's own content changed v1.6→v1.7). |
| 1.6 | 2026-09-10 | product-owner | Adjudication of two findings from a fresh-context CROSS-VENDOR (OpenAI Codex) adversarial review of S-25.02 F4 cluster-3 — the first cross-vendor pass on this BC after five prior same-vendor (Claude) adversary passes missed both gaps. **F-C3-P6-001 (HIGH) — recovery/idempotency contract too weak, causes silent data loss on repeated-prefix content, RESOLVED.** The shipped `heal_or_confirm_already_migrated` (`crates/factory-dispatcher/src/shard_manager.rs`) classified an interrupted-vs-completed migration using ONLY a structural byte-prefix match (`canonical_bytes[..sealed_concat.len()] == sealed_concat`), then overwrote `canonical_bytes[sealed_concat.len()..]` when it matched. Concrete counterexample verified against the shipped code: `session-checkpoints.md` content `A + A + "more\n"` (`A = "## Checkpoint\nx\n"`, 16 bytes), `shard_cap_bytes = 16` — first migration correctly seals `A` as shard 1 and leaves `A + "more\n"` (21 bytes) as the legitimate final partition; a SECOND invocation false-positives (the canonical file's own leading 16 bytes happen to equal `sealed_concat`, since the content legitimately repeats it) and OVERWRITES the canonical file with `canonical_bytes[16..]` = `"more\n"` alone — permanently destroying the second `A` record's heading and body. Postcondition 2's Record-Boundary Marker Table already documents that `session-checkpoints.md` imposes no uniqueness requirement on checkpoint headings/bodies, making this a realistic, non-adversarial occurrence, not a contrived edge case. ADJUDICATED: added a **Backfill Recovery Manifest** to Postcondition 3 — `[backfill_manifest]` in the shard-index, populated in the SAME atomic write as the `[[shard]]` entries, recording `original_bytes`/`original_sha256` (the pre-split monolithic file's exact length + SHA-256) and `final_bytes`/`final_sha256` (the intended final-partition's exact length + SHA-256), both computed once at split time from the same in-memory `original_content` buffer — reusing the ALREADY-PERSISTED shard-index rather than inventing a separate mechanism, per the finding's own suggested direction. Postcondition 5 gained a new **Two-phase publish and the index-publish/canonical-truncate crash window** subsection (naming the exact crash window `heal_or_confirm_already_migrated` targets, previously undocumented) and a new **Recovery-confirmation rule**: the determination now compares the canonical file's exact whole-file `(length, SHA-256)` against the Manifest's two recorded pairs — match `final_bytes`/`final_sha256` ⇒ SAFE (no-op); match `original_bytes`/`original_sha256` ⇒ DANGEROUS (heal by writing the manifest's own recorded `final_bytes` content, never re-derived by slicing); match NEITHER ⇒ AMBIGUOUS, fails loud (`E-SHD-011`, NEW — added to `prd-supplements/error-taxonomy.md` v1.10 in this SAME burst), never silently overwrites. Invariant 3 rewritten to require this exact-manifest-comparison basis and explicitly forbid the byte-prefix heuristic. Added EC-009 (repeated-prefix content correctly classified SAFE) and EC-010 (ambiguous state fails loud) with three matching Canonical Test Vectors (SAFE no-op, DANGEROUS heal, AMBIGUOUS fail-loud, all built on the verified counterexample) and a new VP-124 third facet (Recovery-confirmation correctness invariant) — VP citation change routed to architect per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9), to land alongside the still-outstanding VP-123 third-facet propagation from v1.4. **F-C3-P6-002 (HIGH) — PC6(c)/Invariant 4 authoritative interpretation: disk read-back vs in-memory, RULED (a) — post-hoc DISK read-back is REQUIRED, confirming the spec's existing literal wording.** The shipped code checks the in-memory `partition.bytes` length before writing and never reads a sealed shard back from disk after writing; a prior Claude pass had concluded this was "compliant with intent" despite PC6(c)/Invariant 4's existing text already saying "actual bytes written to disk" / "post-hoc, against the actual bytes written to disk." RULED: reading (a) is authoritative — a one-time migration that overwrites/deletes its own source content must verify the destination actually landed on disk before the source is gone, since an in-memory-only check cannot detect a truncated, partially-flushed, or corrupted write. Added an explicit **Ruling** paragraph to Postcondition 6 (immediately after clause (c)) and a matching parenthetical to Invariant 4, both stating "actual bytes written to disk" means a FRESH READ-BACK after the write completes — foreclosing the in-memory-only reading — and specifying the corrected staging sequence (stage in memory → write → READ BACK from disk → verify → THEN publish index/truncate). No new Postcondition/Invariant/EC/VP semantic content beyond this disambiguation; the existing VP-123 per-shard-cap facet (v1.4) already covers the property, this ruling only removes the ambiguity a prior implementation pass exploited. **Implementer scope (routed, not yet executed by this burst):** `heal_or_confirm_already_migrated` must be restructured around the Recovery-Confirmation Rule (SHA-256 hashing of canonical-file reads, Manifest-comparison disposition logic, new `MechanismABackfillOutcome`/`MechanismABackfillError` variant for the AMBIGUOUS/`E-SHD-011` case); the shard-write staging sequence must gain the post-hoc disk read-back step. **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 — no `bcs:` frontmatter array change (already listed); story-writer should confirm the story body's PC5/PC6/Invariant-3/4 summaries (if any) reflect the Recovery-Confirmation Rule and the disk-read-back ruling. **`prd-supplements/error-taxonomy.md` amended in the SAME burst** (v1.9→v1.10, `E-SHD-011` added) since this is the product-owner-owned PRD supplement this new error code belongs in — not deferred to a follow-up burst. Input-hash recompute owed to state-manager (this file's own content changed v1.5→v1.6). |
| 1.5 | 2026-09-10 | product-owner | Adjudication of two findings from a fresh-context adversarial review of S-25.02 F4 cluster-3. **F-C3-P4-001 (MEDIUM) — PC2 "Normalization rule" ⟂ Record-Boundary Marker Table contradiction, RESOLVED.** v1.4's Normalization rule phrased clause (a) ("it matches `^## ` (any h2)") as an additive global disjunct applying across all four artifacts, which directly contradicted the same Postcondition's own Record-Boundary Marker Table rows for `decision-log.md` (boundary = `^\| D-[0-9]+ \|` only; a bare `## Decisions Log` / `## Appendix: Sub-clause Expansion` heading is a section label, not a record) and `lessons.md` (boundary = tagged h2 forms only — `## L-<tag>-NNN` / `## LESSON (D-NNN)` / `## RECURRENCE NOTE (D-NNN)` — not any untagged `## ` aside). Applied literally, clause (a) would make every `## ` heading in those two artifacts a record boundary, silently over-splitting decision-log.md at its section labels and lessons.md at untagged asides — a spec-internal contradiction, though the shipped implementation was never defective (it correctly follows the marker table, not the literal disjunctive predicate). ADJUDICATED: the Normalization rule is now explicitly PER-ARTIFACT-SCOPED and subordinate to the Record-Boundary Marker Table (restated as the table's authoritative predicate form, not an independent additive source of boundaries) — clause (a)'s "any h2" wording is stated to hold as written ONLY for `burst-log.md` and `session-checkpoints.md` (whose marker-table rows say "any h2 heading"), and is explicitly OVERRIDDEN for `decision-log.md` (primary key `^\| D-[0-9]+ \|`; bare `## ` headings are section labels) and `lessons.md` (only the three tagged h2 forms; untagged `## ` asides are not boundaries). The fail-loud clause for an unrecognized future heading form (Postcondition 6's gate) is preserved verbatim in substance — no detection-behavior change, wording-only consistency fix. **O-1 (LOW) — recognized-stem-empty-oracle invariant, documented.** Added a note (Postcondition 2, immediately after the Normalization rule) recording that each of the four recognized artifacts always yields a non-empty oracle boundary set for non-empty content (each always contains at least one `\| D-NNN \|`/h2/tagged-h2 marker), so a "recognized stem + empty oracle ⇒ trust caller" code branch is unreachable in production and exists only for synthetic test inputs; the unrecognized-stem case remains covered by Postcondition 6's fail-loud path. Documentation only, no behavior change. **No AC/EC/VP/behavior change** — pure spec-internal consistency fix; the shipped code already implements the marker-table-scoped behavior this amendment now states unambiguously. No Canonical Test Vector requires updating (all existing vectors already assert marker-table-consistent expected outputs, not the contradictory literal disjunctive reading). **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 — no `bcs:` frontmatter array change; no story body propagation required since no AC/behavior changed, but story-writer should confirm the story body's PC2 summary (if any) does not itself repeat the additive-disjunct phrasing this amendment corrects. |
| 1.4 | 2026-09-10 | product-owner | Adjudication of two findings from a fresh-context adversarial review of S-25.02 F4 cluster-3 (mechanism-A backfill). **F-C3-P3-001 (HIGH) — leading-preamble handling under Postcondition 2's per-shard cap, RESOLVED.** All four mandatory artifacts have a leading preamble (YAML frontmatter + title/intro, plus — for `decision-log.md` — table header/separator rows) before their first record-boundary line, which Postcondition 2 never addressed; the shipped implementation folds the preamble unconditionally into the first record's shard, so when `preamble_bytes + first_record_bytes > shard_cap_bytes` the first sealed shard exceeds cap — an unsanctioned Postcondition 2 violation distinct from EC-002 (the record itself is under cap), re-creating the exact over-cap-shard condition Layer 2 exists to eliminate. Verified by direct inspection of the real preamble content of all four artifacts in both `v1.0-brownfield-backfill/` and `v1.0-feature-engine-discipline-pass-1/` (one instance, that cycle's `burst-log.md`, has a near-empty preamble — a bare leading `---` with no frontmatter/title — confirming preamble size is a real per-instance quantity, not a fixed assumption). ADJUDICATED (Postcondition 2, new **Leading-preamble handling rule** sub-clause): the preamble is an atomic, indivisible packing unit resolved once before record packing begins — normal case (`preamble_bytes + first_record_bytes <= shard_cap_bytes`) rides with the first record's shard unchanged from today's behavior; overflow case seals the preamble as its own zero-record shard (`seq=1`, new `is_preamble_shard: true` index field) before record packing starts fresh; degenerate case (preamble alone exceeds cap — not reachable for any of the four artifacts at current measured preamble sizes, 2-3 orders of magnitude below any calibrated cap, but specified for completeness) reuses EC-002's oversized-atomic-unit exception (`oversized_record: true`, broadened to cover the preamble as well as a record) rather than a fail-loud abort, since the same atomicity rationale applies. Postcondition 3 extended to specify the preamble shard's index entry shape. Postcondition 4 extended: a preamble shard is NOT exempt from `retention_count`/archival — it is the typical FIRST archival candidate (always `seq=1`), consistent with BC-1.18.006's own precedent that a rolled canonical file's header is swept into whatever shard seals it and is never specially preserved. **ADDED Postcondition 6(c) and new Invariant 4 (the second requested addition, also F-C3-P3-001): the split MUST verify, as a hard fail-loud gate, that EVERY sealed shard's `bytes_at_seal <= shard_cap_bytes` except a shard explicitly flagged `oversized_record: true`** — this per-shard-cap bound is now checked explicitly against actual on-disk bytes, never merely implied by the packer having run. Added EC-007 (overflow case) and EC-008 (degenerate case) with matching Canonical Test Vectors, and a new VP-123 facet row (per-shard-cap fail-loud invariant) — VP citation change routed to architect per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9) for propagation to `VP-INDEX.md`, `verification-architecture.md`, `verification-coverage-matrix.md`. **F-C3-P3-002 — fail-loud-on-unrecognized-heading-form language, CONFIRMED, no amendment needed.** The reviewer asked whether Postcondition 2's Normalization rule already states that an unrecognized future heading form must fail loud rather than silently mis-partition. Confirmed present and unchanged in v1.3 (carried forward verbatim into v1.4): "If a future cycle introduces a heading form outside this enumeration, Postcondition 6's fail-loud content-preservation gate MUST reject the backfill run rather than silently mis-partition, and this BC MUST be amended to extend the marker table before the backfill is re-run." This is already sufficiently authoritative (an explicit MUST binding Postcondition 6's gate) for the implementer to align the code's empty-oracle path to fail loud; no strengthening required. **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 — no `bcs:` frontmatter array change (this BC was already listed), but the story body's BC-1.18.008 content summary/AC trace (if it echoes Postcondition 2/6 mechanics) should be reviewed for propagation of the Leading-Preamble Handling Rule, Postcondition 6(c), and Invariant 4. **Canonical Test Vectors test-writer must add/update:** the two new EC-007/EC-008 rows (net-new fixtures); no existing row's expected output changed. No change to Preconditions, the Record-Boundary Marker Table, the `ceil()`-is-a-lower-bound reconciliation (v1.3), or Postconditions 1/5. |
| 1.3 | 2026-09-10 | product-owner | Fresh-context adversarial review of S-25.02 F4 cluster-3 (finding F-004, MEDIUM) found Postcondition 2's `ceil(current_bytes / shard_cap_bytes)` shard-count formula jointly unsatisfiable with the same postcondition's "preserving record boundaries" requirement for non-uniform record sizes (counterexample: five 40-byte records against a 70-byte cap — `ceil(200/70)=3`, but no boundary-preserving packing fits two 40-byte records into one 70-byte shard, so the actual greedy packer produces 5), while the Canonical Test Vectors table asserted the `ceil()` value as an EXACT expected shard count (19 for `decision-log.md`, 5 for `lessons.md`), which only passes when a fixture's records happen to pack without slack. Reconciled: Postcondition 2 now specifies `ceil()` as a documented LOWER BOUND on shard count, adds an explicit deterministic greedy boundary-preserving packing procedure as the algorithm's actual split-point rule, and states the general inequality `actual_count >= ceil(current_bytes / shard_cap_bytes)` with the equality condition spelled out. Postcondition 4's retention-composition reasoning reworded from an approximate `≈ 19 shards` framing to an explicit lower-bound inequality (`actual_count >= 19 > retention_count of 10`, robust regardless of the real packed count). Marked the two Canonical Test Vectors asserting exact `ceil()` counts (`decision-log.md` 19-shard row, `lessons.md` 5-shard row) plus the dependent interrupted-restart row as `NEEDS-UPDATE`, with an explicit instruction that test-writer must measure the actual packed-shard count from the real fixture rather than re-asserting `ceil()` arithmetic; the three unaffected rows (under-cap single-shard, oversized-single-record, burst-log h3-exception-detection) are annotated as out-of-scope for this correction, with the reason stated inline. No new `E-SHD-NNN` error-taxonomy code is warranted: a packed shard count exceeding the `ceil()` lower bound is expected, correct algorithm behavior, not a failure condition — the existing `E-SHD-003` content-preservation gate (Postcondition 6, EC-004) already covers the actual failure mode (a boundary violation or record loss/duplication), which this amendment does not touch. Finding F-002 (session-checkpoints.md marker-heuristic narrowing) from the same review was adjudicated separately and required NO spec change: direct inspection of both real `session-checkpoints.md` files (`v1.0-brownfield-backfill/`, 182 h2 records; `v1.0-feature-engine-discipline-pass-1/`, 12 h2 records) confirmed every h2 heading in both files is a genuine checkpoint record with zero legitimate non-record h2 asides, so this BC's existing `session-checkpoints.md` marker-table row ("any h2 = boundary", no confirmed exception forms) is already correct as written — the code-side `is_checkpoint_record_heading` heuristic (filtering h2 on `starts_with("Archived")`/`contains("Checkpoint")`) is the defective party and is routed to implementer for deletion/revert-to-`^## `, not a spec issue; evidence includes a real record the code's case-sensitive heuristic itself would silently drop (`## ARCHIVED CHECKPOINT: 2026-08-27 — pass-60 CLEAN D-1117...`, all-caps, matches neither `starts_with("Archived")` nor `contains("Checkpoint")`). No other Postcondition, Invariant, Edge Case, or Verification Property content changed. |
| 1.2 | 2026-09-10 | product-owner | Fresh-context adversarial review found PC2 internally self-contradictory on burst-log record-boundary granularity: PC2 named an h3 (`### <burst-heading>`) boundary phrase in one clause while PC2's own "never split" clause and PC6(b)'s record-integrity clause both correctly said h2 — the erroneous h3 phrase misled implementation toward `### ` as the burst-log/lessons.md record marker, which silently no-ops backfill on the two largest artifacts (finds ≤2 boundaries against real h2-keyed content) and splits records mid-record where nested `### Block N:` sub-headings occur inside brownfield burst records. Reconciled: removed the erroneous h3 phrase from PC2; PC2 and PC6(b) now unambiguously key burst-log.md and session-checkpoints.md on `## ` (h2) boundaries. Added an authoritative Record-Boundary Marker Table to PC2, derived from direct inspection of the real `v1.0-feature-engine-discipline-pass-1/` and `v1.0-brownfield-backfill/` cycle artifacts, enumerating the exact record-start pattern per artifact and two confirmed real-world h3-exception record forms that a naive h2-only rule would miss: burst-log.md's `### Pass-39/40 Fix Burst` records (engine cycle, sitting between two h2 records) and lessons.md's pre-L-EDP1-052 `### L-EDP1-050`/`### L-EDP1-051` records (the L-EDP1-052+ form switched to h2). Added a single implementable normalization predicate covering all confirmed forms across both cycles, including decision-log.md's table-row primary key plus its Appendix sub-clause h3 blocks as a secondary non-splittable unit. Tightened Invariant 2 to cite the marker table and to forbid keying partition points on heading level alone. Added EC-006 and a corresponding canonical test vector covering the h3-exception detection requirement. No change to the split algorithm's core semantics (Postconditions 1, 3, 4, 5), Atomicity, Idempotency, or the artifact's CAP-043 anchor. |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-003 + F-S2502-F2-007): VP-124's idempotency row Proof Method reconciled from "unit test" to "integration" per VP-INDEX v3.02's authoritative method assignment (both VP-124 rows now consistently read "integration test"); the atomicity row's wording tightened to lead with "integration test" for internal consistency. Added `## SDK Grounding Evidence` section with literal stable-anchor grep output for `write_atomic`, `rotate_changelog`, and `HookResult`. No postcondition/invariant content change. Related BCs gained a cross-reference to the new BC-1.18.011 (B2 migration BC modeled on this BC's governance pattern). |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, not in the original F1 enumeration — required per ADR-051 Decision 2's finding that AC-002/AC-003 only gate future writes). One-time backfill-split of the four pre-existing oversized cycle append-log files, record-boundary-safe partitioning, content-preservation verification gate, composition with the retention policy. CAP-043 capability anchor. ADR-051 §D2 citation. |
