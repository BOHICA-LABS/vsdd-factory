---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 8
verdict: NOT CLEAN — 2 findings (2 MEDIUM), one doc-drift + one spec-internal asymmetry closing the destructive-write-read-back class
finding_count: 2
finding_breakdown: "2 MEDIUM (F-C3-P8-001, E-SHD-012 Message Format documentation drift vs shipped Display — 4th recurrence of this defect class in the cluster; F-C3-P8-002, happy-path canonical-truncate write lacked the disk read-back the heal/sealed-shard writes already require — spec-internal asymmetry + latent silent-data-loss)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "915b898c"
diff_head: "915b898c"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "2027698"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-8

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-3-CLEAN using
> LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `915b898c` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.7 and BC-1.18.007 (retention/compaction) v1.2, run immediately after pass-7's fix-burst
> (D-1197) landed on the branch.

## Verdict

**NOT CLEAN — 2 findings (2 MEDIUM).** Reviewing `feature/S-25.02-backfill` @ `915b898c` against
`shard_manager.rs`, `bc_1_18_008_backfill_split_test.rs`, BC-1.18.008.md v1.7, and
`prd-supplements/error-taxonomy.md` v1.11, the adversary found one MEDIUM documentation-drift finding
(the recurring "Message Format ≠ shipped Display" class, now its 4th occurrence in this cluster) and
one MEDIUM spec-internal asymmetry finding with a latent silent-data-loss consequence: the
happy-path (ordinary, non-recovery) canonical-truncate write never received the same post-hoc disk
read-back the DANGEROUS-window heal write gained in pass-7 (F-C3-P7-001), despite being structurally
identical in kind — a one-time, destructive, source-overwriting write. BC-5.39.001 cluster-3 LOCAL
streak: **0/3 → 0/3** (cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track). Both
findings FIXED this burst (see Disposition Summary).

## Part A — Findings

### F-C3-P8-001 (MEDIUM) — `E-SHD-012` Message Format cell documents a fixed structured wording; the shipped `Display` emits a `{detail}`-placeholder form

**Location:** `prd-supplements/error-taxonomy.md` `E-SHD-012` row; shipped
`MechanismABackfillError::SliceVerificationFailed` `Display` impl
(`crates/factory-dispatcher/src/shard_manager.rs`).

**Finding:** the taxonomy row documented `E-SHD-012` with a single fixed structured Message Format.
The shipped `Display` impl instead emits a `{detail}`-placeholder form —
`E-SHD-012: backfill recovery heal slice-verification failed for artifact_stem "{artifact_stem}":
{detail}` — populated with FIVE distinct `<detail>` shapes across the code's actual call sites:
manifest-inconsistency `checked_sub` underflow, `usize` offset-arithmetic overflow, offset-exceeds-
canonical-length, the pre-write slice mismatch (EC-011) the row previously documented as if it were
the only case, and a post-hoc read-back mismatch after the heal's write already completed. This is
the **4th recurrence of this exact defect class** in cluster-3 — a taxonomy Message Format cell
drifting out of sync with the shipped `Display` text — following prior occurrences on `E-SHD-008`/
`E-SHD-009` (S-25.02 cluster-2 pass-9, F-C2-P9-002) and this cluster's own prior taxonomy rows.

**Companion observation (folded into disposition, not a separate finding):** `E-SHD-011`'s row also
documented only the top-level three-way `(length, hash)` mismatch form. `MechanismABackfillError::
MissingBackfillManifest` emits a SECOND, distinct message under the SAME code (a durable shard-index
exists but carries no `[backfill_manifest]` table — e.g. a pre-v1.6 shard-index) that the row never
documented. BC-1.18.008 v1.7 Invariant 3(c) already sanctions reusing `E-SHD-011` for this
precondition-failure case (not a distinct determination), so this is a taxonomy completeness gap, not
a code defect and not a new code allocation.

**Impact:** an operator or downstream tooling consulting the taxonomy row for `E-SHD-012` to
recognize or parse the emitted error text would fail to match against 4 of the 5 real emitted forms,
and would not recognize `E-SHD-011`'s second (missing-manifest) emission at all.

**Disposition:** FIXED, DOC-ONLY (no code change). product-owner corrected `E-SHD-012`'s Message
Format cell to the `{detail}`-placeholder form, documented per this table's existing convention for
other `{detail}`-bearing variants, enumerating all five detail shapes with representative wording and
noting that cases (1)-(4) leave the canonical file untouched while case (5) does not (the write
already happened by that point). Completed `E-SHD-011`'s row to document BOTH real emissions as forms
(a) (three-way mismatch) and (b) (missing manifest). `prd-supplements/error-taxonomy.md` v1.11→v1.12.
No BC/AC/EC/VP change — pure taxonomy-accuracy correction.

### F-C3-P8-002 (MEDIUM) — happy-path canonical-truncate write lacks the post-hoc read-back the heal write requires (asymmetry + latent silent-data-loss)

**Location:** BC-1.18.008 Postcondition 5 step (ii) / Postcondition 6(c) (`decision-log.md`
migration write path); `run_mechanism_a_backfill_split`'s happy-path completion
(`crates/factory-dispatcher/src/shard_manager.rs`).

**Finding:** Postcondition 6(c)/Invariant 4 already mandate a post-hoc disk read-back for every
sealed-shard write (F-C3-P6-002), and pass-7's F-C3-P7-001 fix already extends that requirement to
the DANGEROUS-window heal write, on the rationale that the heal write "permanently replaces the
canonical file's content at the moment the pre-heal content is irretrievably gone." That rationale
applies IDENTICALLY to the happy-path canonical-truncate write in Postcondition 5 step (ii) — the
ORDINARY, first-time/uninterrupted `write_atomic` that discards the original monolithic file's
content in the same act the final partition's bytes become the canonical file's only copy — but the
spec text enumerated only the DANGEROUS-window heal for this discipline, not the happy-path write a
spec-literal implementation would leave unverified. The more-frequently-executed path (every
uninterrupted migration run, not merely its crash-recovery counterpart) was therefore LESS verified
than the path that exists specifically to recover from ITS OWN failure.

**Root cause:** an unsanctioned asymmetry — the disk-read-back discipline had been extended
incrementally, site by site (sealed-shard write in v1.4/F-C3-P3-001; heal write in v1.7/F-C3-P7-001),
without ever generalizing to a single principle covering every destructive one-time-migration write.
The happy-path canonical-truncate write, structurally identical in kind to the heal write it exists
to be recovered from, was the one site left ungoverned.

**Impact:** a silent truncation, partial flush, or other on-disk corruption of the happy-path
canonical-truncate write (e.g. a disk-full or power-loss condition mid-write that `write_atomic`'s
own return value does not surface as an error) would go undetected at migration time — discovered, if
at all, only much later, after the original monolithic file's content is already gone (recoverable
only from git history). This is the exact silent-data-loss condition every other destructive-write
check in this BC exists to foreclose.

**Disposition:** FIXED. product-owner amended **BC-1.18.008 v1.7→v1.8**: added an **Extension to the
happy-path canonical-truncate write** paragraph to Postcondition 6(c) — after step (ii)'s
`write_atomic` call returns, the backfill-split MUST perform a FRESH read-back of the canonical file
and verify its `(length, SHA-256)` equals the Backfill Recovery Manifest's `(final_bytes,
final_sha256)` (already durably published in step (i), so no new value needs computing) BEFORE the
migration reports success; a mismatch fails loud with **NEW `E-SHD-013`**. Added a **Summary**
paragraph naming all three destructive-write sites (sealed shard / heal / happy-path canonical) now
under the same discipline, and a **NEW Invariant 5** generalizing the principle once: no destructive
write of this BC's one-time migration is ever trusted on its own return value alone. Added EC-013
(happy-path write silently corrupted on disk, detected via read-back, fails loud `E-SHD-013`, with a
follow-on note that a subsequent recovery attempt against the resulting on-disk state correctly
reports the AMBIGUOUS `E-SHD-011` disposition rather than silently accepting the corruption) and a
matching Canonical Test Vector. `error-taxonomy.md` v1.11→v1.12 gained `E-SHD-013` in the SAME burst
(folded into F-C3-P8-001's taxonomy pass). architect propagated VP-124's FIFTH facet (happy-path
canonical-write verification invariant) per POLICY 9 — VP-INDEX v3.12→v3.13,
verification-architecture.md v1.29→v1.30, verification-coverage-matrix.md v1.27→v1.28, `total_vps`
UNCHANGED 141. Implementer scope (routed, code-side, delivered on `feature/S-25.02-backfill`):
`run_mechanism_a_backfill_split`'s happy-path completion now routes the canonical-truncate write
through the SAME `write_and_read_back` helper F-C3-P7-001 extracted, verifying `(length, sha256)`
against the shard-index's already-published `[backfill_manifest]` `(final_bytes, final_sha256)` and
surfacing the NEW `E-SHD-013` variant (`CanonicalWriteVerificationFailed`, mirroring
`SliceVerificationFailed`'s shape but scoped to the non-recovery path) on mismatch. All THREE
destructive write sites this BC specifies (sealed-shard write, DANGEROUS-window heal write,
happy-path canonical-truncate write) are now independently read-back-guarded via the same shared
helper — the class of "destructive write lacks post-hoc verification" findings this cluster has
progressively closed across passes 6, 7, and 8 is now COMPLETE.

## Observations (non-blocking)

- **Destructive-write-read-back class now closed:** the adversary confirms this finding completes a
  three-pass progression (F-C3-P6-002 sealed-shard ruling → F-C3-P7-001 heal-write extension →
  F-C3-P8-002 happy-path extension) — every one-time destructive write this BC specifies now shares
  the identical Manifest-verified post-hoc disk-read-back discipline via one shared helper. No
  residual asymmetry remains among the three write sites.
- **Taxonomy Message Format drift (4th recurrence, now tracked as a pattern):** this is the fourth
  instance of a shipped `Display` diverging from its taxonomy row's documented Message Format across
  this cluster (E-SHD-008/E-SHD-009 at cluster-2 pass-9; this cluster's own prior corrections). No new
  process-gap codification this burst (already tracked informally); a systemic prevention (e.g. a
  golden-file test asserting taxonomy Message Format cells against shipped `Display` output) remains a
  candidate future hardening item, not actioned this burst.

## Disposition Summary

Both findings FIXED same-burst. F-C3-P8-001 (MEDIUM, doc-only): `error-taxonomy.md` v1.11→v1.12
(`E-SHD-012` Message Format cell corrected to the `{detail}`-placeholder form with all five detail
shapes; `E-SHD-011`'s row completed to document both real emissions). F-C3-P8-002 (MEDIUM):
BC-1.18.008 v1.7→v1.8 (Postcondition 6(c) happy-path extension, NEW Invariant 5, EC-013, NEW
`E-SHD-013`); architect propagated VP-124's fifth facet (VP-INDEX v3.12→v3.13,
verification-architecture.md v1.29→v1.30, verification-coverage-matrix.md v1.27→v1.28, `total_vps`
UNCHANGED 141); implementer routed the happy-path write through the shared `write_and_read_back`
helper with manifest verification, delivered on `feature/S-25.02-backfill` @ `8e2a37f4`. Because
findings were present, pass-8 is NOT CLEAN — BC-5.39.001 cluster-3 LOCAL streak stays **0/3**.

## Code Gate (this burst)

Feature branch `feature/S-25.02-backfill` @ `8e2a37f4` (implementer's F-C3-P8-002 fix, immediately
after `915b898c`, pushed): full `cargo test --workspace --all-targets` suite green; `cargo fmt
--check --all` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Next

**NEXT = cluster-3 LOCAL adversary pass-9, fresh context, against BC-1.18.008 v1.8 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `8e2a37f4` — continuing the human-authorized full 3-CLEAN
drive using LOCAL adversary only, no further cross-vendor rotation unless the human specifies.**
