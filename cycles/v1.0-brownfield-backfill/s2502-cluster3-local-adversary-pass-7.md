---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 7
verdict: NOT CLEAN — 2 findings (1 HIGH, 1 MEDIUM), root-cause was a PC5/PC3 spec incoherence
finding_count: 2
finding_breakdown: "1 HIGH (F-C3-P7-001, DANGEROUS-window heal wrote an unverified Manifest-derived-in-name-only slice — silent-mis-heal-on-corruption); 1 MEDIUM (F-C3-P7-002, decision-log.md marker-table regex misses real sub-clause-suffixed D-NNN rows)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "b1134954"
diff_head: "b1134954"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "5a4f0cb"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-7

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-3-CLEAN using
> LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `b1134954` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.6 and BC-1.18.007 (retention/compaction) v1.2, run immediately after pass-6's cross-vendor
> fix-burst (D-1196) landed on the branch.

## Verdict

**NOT CLEAN — 2 findings (1 HIGH, 1 MEDIUM).** Reviewing `feature/S-25.02-backfill` @ `b1134954`
against `shard_manager.rs`, `bc_1_18_008_backfill_split_test.rs`, BC-1.18.007.md, BC-1.18.008.md,
and `decision-log.md` (both cycles), the adversary found one HIGH data-loss finding and one MEDIUM
correctness finding in the state-manager's own boundary-detection machinery. The HIGH finding's root
cause is a **spec incoherence between BC-1.18.008 v1.6's Postcondition 5 (PC5) and Postcondition 3's
(PC3) Manifest schema**: PC5 said the DANGEROUS-window heal "writes the manifest's own recorded
`final_bytes` content," but PC3's Backfill Recovery Manifest stores only `final_bytes` (a length) and
`final_sha256` (a hash) — no content of any kind. That text was literally unimplementable from the
manifest alone. The shipped implementation, correctly noting the manifest holds no content, instead
derived the slice offset by SUMMING the shard-index's per-shard `bytes_at_seal` fields — a source
independent of, and separately corruptible from, the Manifest — and wrote the resulting slice with
NO verification against `final_bytes`/`final_sha256` at all. A corrupted or stale `bytes_at_seal`
therefore produces a wrong slice written with no check: a silent mis-heal (record duplication or
truncation), exactly the class of defect the Backfill Recovery Manifest (v1.6, F-C3-P6-001) was
introduced to eliminate. BC-5.39.001 cluster-3 LOCAL streak: **0/3 → 0/3** (cycle-level BC-5.39.001
3/3 CONVERGED streak UNCHANGED, separate track). Both findings FIXED this burst (see Disposition
Summary).

## Part A — Findings

### F-C3-P7-001 (HIGH) — DANGEROUS-window heal sliced canonical bytes at a shard-index-summed offset and wrote WITHOUT manifest verification

**Location:** `crates/factory-dispatcher/src/shard_manager.rs` (DANGEROUS-window branch of the
mechanism-A recovery/idempotency arm, `heal_or_confirm_already_migrated`).

**Finding:** at the confirmed DANGEROUS window (top-level `(length, hash)` recovery-confirmation
check already matched the Manifest's `original_bytes`/`original_sha256`), the shipped heal computed
the slice offset by summing the shard-index's per-shard `bytes_at_seal` fields — an
independently-corruptible source distinct from the Backfill Recovery Manifest — and wrote the
resulting slice to the canonical file with **no verification of any kind** against the Manifest's
`final_bytes`/`final_sha256`. If `bytes_at_seal` is corrupted, stale, or was computed under a
different assumption than the Manifest's own `final_bytes`, the heal silently writes the wrong
bytes — a silent mis-heal indistinguishable, from the caller's perspective, from a correct recovery.

**Root cause:** BC-1.18.008 v1.6's own Postcondition 5 text was incoherent with Postcondition 3's
Manifest schema — PC5 said the heal "writes the manifest's own recorded `final_bytes` content,"
but PC3's Manifest stores only a length (`final_bytes`) and a hash (`final_sha256`), never content.
The v1.6 spec text could not literally be implemented; the implementation that shipped instead
picked an alternative (shard-index-summed offset, no verification) that reintroduced exactly the
un-verified-write risk the Manifest was created to close.

**Impact:** silent, undetectable data corruption of the canonical file during recovery from an
interrupted mechanism-A backfill-split, on any input where `bytes_at_seal` disagrees with the
Manifest's `final_bytes`/`final_sha256` — a corrupted or manually-edited shard-index TOML, a legacy
caller, or bit-rot in the shard-index file are all plausible real-world triggers.

**Disposition:** FIXED. product-owner amended **BC-1.18.008 v1.6→v1.7**: added the
**Manifest-Authoritative Slice-and-Verify Rule** to Postcondition 5 — the heal's offset is now
`original_bytes - final_bytes`, both operands read from the Manifest itself (the shard-index
`bytes_at_seal` sum is no longer consulted for this purpose at all), and the resulting slice MUST
satisfy `sliced.len() == final_bytes AND sha256(sliced) == final_sha256` BEFORE it is written; on
any mismatch the heal fails loud with **NEW `E-SHD-012`** and writes nothing. Invariant 3 rewritten:
the Manifest "authorizes/verifies the bytes written, it is not required to store them." Also closes
a companion observation: the DANGEROUS heal's own destructive write previously received no post-hoc
disk read-back (unlike sealed-shard writes, per the F-C3-P6-002/PC6(c) ruling) — Postcondition 6(c)
and Invariant 4 are extended to require the SAME fresh-read-back-and-compare-to-`(final_bytes,
final_sha256)` discipline after the heal's `write_atomic` call. Added EC-011 (Manifest-verification
mismatch at a confirmed DANGEROUS window — fails loud, `E-SHD-012`, no write) and a matching
Canonical Test Vector; corrected the existing DANGEROUS-heal CTV row's expected-behavior text.
`error-taxonomy.md` v1.10→v1.11 gained `E-SHD-012` in the SAME burst. architect propagated VP-124's
FOURTH facet (heal slice-verification invariant) per POLICY 9 — VP-INDEX v3.11→v3.12,
verification-architecture.md v1.28→v1.29, verification-coverage-matrix.md v1.26→v1.27, `total_vps`
UNCHANGED 141. implementer rewrote the DANGEROUS-window heal around the Manifest-derived offset and
extracted a shared `write_and_read_back` helper (reused by both the heal write and the sealed-shard
write path, closing a would-be duplication between the F-C3-P6-002 and this fix's read-back logic).
test-writer added the EC-011 corrupted-Manifest fixture, a genuine-slice-passes-and-is-written-and
-read-back-verified positive fixture, and regression-guarded the shared read-back helper.

### F-C3-P7-002 (MEDIUM) — `decision-log.md` marker-table regex misses real sub-clause-suffixed rows

**Location:** BC-1.18.008 Postcondition 2 Record-Boundary Marker Table (`decision-log.md` primary
key); `is_decision_log_row_marker` (`crates/factory-dispatcher/src/shard_manager.rs`).

**Finding:** the `decision-log.md` primary partition-key regex, `^\| D-[0-9]+ \|`, requires digits
immediately followed by ` \|` and cannot match a real sub-clause-suffixed row such as
`\| D-440(a) \|`, the combined-suffix form `\| D-446(a/b/c/d/e) \|`, or the hyphenated form
`\| D-355-AMEND \|`. Direct re-inspection of the live
`.factory/cycles/v1.0-feature-engine-discipline-pass-1/decision-log.md` (2026-09-10) found 144 total
`\| D-...` rows = 109 bare `\| D-NNN \|` + 34 parenthetical-suffix rows + 1 hyphenated-suffix row
(`D-355-AMEND`) — none of the 35 non-bare rows matched the old regex, so every one of them would be
silently absorbed into the preceding record rather than detected as its own boundary, under-
segmenting the artifact. The sibling brownfield `decision-log.md` was also re-inspected: 265 rows as
of this amendment (grown from the prior 254-row measurement, consistent with EC-005's documented
staleness precedent), all bare form, 0 sub-clause exceptions in that cycle — so this defect is
currently latent for the brownfield cycle's own decision-log.md but live for the engine cycle's.

**Impact:** a Layer-2 shard-boundary-detection pass run against a `decision-log.md` containing
sub-clause-suffixed rows (the engine cycle's own decision-log.md, 35 such rows as of this burst)
would mis-segment the file, merging each sub-clause row into its preceding bare-row record instead
of treating it as an independent atomic unit.

**Disposition:** FIXED. product-owner corrected the primary-key regex to
`^\| D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|` — confirmed by direct grep to match every row in both
cycles (144/144 engine, 265/265 brownfield, zero unmatched). Updated the Normalization rule's
`decision-log.md` bullet to cite the corrected regex and enumerate the two suffix forms as ONE
primary-key class (not three, and not a separate boundary class from the Appendix h3 form). Added
EC-012 (sub-clause rows detected as boundaries) and a matching Canonical Test Vector (6-row fixture:
bare, single-letter-parenthetical ×2, combined-parenthetical, hyphenated, bare). No new VP citation
— extends the EXISTING VP-123 Record-integrity facet's fixture coverage, not a new facet or ID.
Also answered the adversary's Appendix cap-bounding question (clarification only, no behavior
change): the `## Appendix: Sub-clause Expansion` section is packed as ONE trailing atomic unit after
the shard sealing the file's last `\| D-NNN(...) \|` row, flagged `oversized_record: true` under the
EXISTING EC-002 exception when it does not fit — confirmed the REAL case for the engine cycle
(Appendix section = 74,989 bytes, already over the illustrative 49,152-byte cap on its own).
implementer updated `is_decision_log_row_marker` to the corrected regex, TD-VSDD-060 sibling-swept.
test-writer added the EC-012 6-row fixture and a negative case confirming the old regex under-counts.

## Observations (non-blocking)

- **PC3 "manifest stores no content" reconciliation (now closed):** the adversary confirmed
  BC-1.18.008 v1.7's PC5 rewrite is now internally coherent with PC3's length-plus-hash-only Manifest
  schema — no residual incoherence remains.
- **Heal-write read-back extension (now added):** confirmed Postcondition 6(c)/Invariant 4's
  disk-read-back requirement now explicitly covers the DANGEROUS-window heal's own write, closing the
  asymmetry with sealed-shard writes noted informally during this pass's own investigation.
- **decision-log.md Appendix cap-bounding (confirmed EC-002-covered):** re-derived compliant, no
  further action — see F-C3-P7-002's disposition above.

## Disposition Summary

Both findings FIXED same-burst. F-C3-P7-001 (HIGH): BC-1.18.008 v1.6→v1.7 (Manifest-Authoritative
Slice-and-Verify Rule, Invariant 3 rewrite, EC-011, PC6(c)/Invariant 4 heal-write read-back
extension, `E-SHD-012`); implementer rewrote the DANGEROUS-window heal + extracted
`write_and_read_back`; test-writer added EC-011 fixtures. F-C3-P7-002 (MEDIUM): BC-1.18.008 v1.7
decision-log.md regex fix + EC-012; implementer updated `is_decision_log_row_marker`; test-writer
added the EC-012 fixture family. architect propagated VP-124's fourth facet (VP-INDEX v3.11→v3.12,
verification-architecture.md v1.28→v1.29, verification-coverage-matrix.md v1.26→v1.27, `total_vps`
UNCHANGED 141). Because findings were present, pass-7 is NOT CLEAN — BC-5.39.001 cluster-3 LOCAL
streak stays **0/3**.

## Code Gate (this burst)

Feature branch `feature/S-25.02-backfill` @ `915b898c` (implementer's F-C3-P7-001/002 fixes +
test-writer's new fixtures, immediately after `b1134954`, pushed): full
`cargo test --workspace --all-targets` suite green; `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean.

## Next

**NEXT = cluster-3 LOCAL adversary pass-8, fresh context, against BC-1.18.008 v1.7 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `915b898c` — continuing the human-authorized full 3-CLEAN
drive using LOCAL adversary only, no further cross-vendor rotation unless the human specifies.**
