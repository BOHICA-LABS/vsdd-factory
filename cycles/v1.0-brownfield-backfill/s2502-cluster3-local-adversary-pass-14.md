---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 14
verdict: CLEAN — zero blocking findings; 1 LOW non-blocking observation; BC-5.39.001 3-CLEAN CONVERGED
finding_count: 0
finding_breakdown: "0 blocking findings; 1 LOW non-blocking observation (O-C3-P14-001, archive_overflow_shards failure re-coded as MechanismABackfillError::Io/E-SHD-003 in the backfill context, flattening the #[source] chain — defensible uniform-treatment choice, flagged for OPTIONAL product-owner adjudication, not a defect)"
streak: 3/3 — CONVERGED
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "2dd39bbb"
diff_head: "2dd39bbb"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "2027698"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-14

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-true-3-CLEAN
> using LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `2dd39bbb` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.8 and BC-1.18.007 (retention/compaction) v1.2, run against the SAME frozen `2dd39bbb` code
> passes 12 and 13 left in place. This is the fourteenth pass of the cluster-3 LOCAL cascade and the
> **third consecutive CLEAN pass of the restarted streak** (after passes 12 and 13; fourth clean pass
> overall this cascade, after pass-10, pass-12, and pass-13) — reaching **literal BC-5.39.001 3-CLEAN
> CONVERGENCE**.

## Verdict

**CLEAN — zero blocking findings.** Reviewing `feature/S-25.02-backfill` @ `2dd39bbb` against
`shard_manager.rs`, `bc_1_18_007_shard_retention_test.rs`, `bc_1_18_008_backfill_split_test.rs`,
BC-1.18.007.md v1.2, BC-1.18.008.md v1.8, and `prd-supplements/error-taxonomy.md` v1.13, the
adversary independently re-verified the full v1.8 contract end to end and found the shipped code
spec-conformant in every dimension checked — zero Critical/High/Medium/Low blocking findings. One
LOW non-blocking observation is recorded (see below); per BC-5.39.001, a LOW/non-blocking-only
observation does not reset the streak. **BC-5.39.001 cluster-3 LOCAL streak: 2/3 → 3/3 —
CONVERGED.** This is the THIRD consecutive clean pass (12, 13, 14) on frozen code `2dd39bbb`. The
LOCAL adversarial cascade for cluster-3 is **CLOSED** (cycle-level BC-5.39.001 3/3 CONVERGED streak
UNCHANGED, separate track).

## Part A — Findings

None. Full re-verification coverage, all CONFIRMED spec-conformant:

- **Recovery three-way classification (BC-1.18.008 Postcondition 3/5, Invariant 3):** the
  not-started / in-progress-recoverable / already-migrated classification independently re-derived
  against the shipped `heal_or_confirm_already_migrated` logic and BC-1.18.008 v1.8 text — all three
  arms CONFIRMED spec-conformant.
- **Manifest-authoritative slice-and-verify (Postcondition 5, `E-SHD-012`):** the Manifest-Authoritative
  Slice-and-Verify Rule — offset `original_bytes - final_bytes`, both Manifest-derived, verified
  `(sliced.len(), sha256(sliced))` against `(final_bytes, final_sha256)` before every DANGEROUS-window
  heal write — re-derived and confirmed correct.
- **All 3 destructive-write read-backs (Postcondition 6(c), Invariant 5):** sealed-shard write,
  DANGEROUS-window heal write, and happy-path canonical-truncate write each independently confirmed
  to route through the shared `write_and_read_back` helper with a genuine post-hoc disk read-back and
  manifest-verified `(length, sha256)` comparison before any write is treated as durable.
- **`[backfill_manifest]` persistence (Postcondition 3):** the Backfill Recovery Manifest's on-disk
  persistence, field shape, and write-then-read-back sequencing re-confirmed consistent with the BC
  text and with `write_shard_index_for_backfill`'s always-writes-a-manifest guarantee.
- **`decision-log.md` marker regex + boundary fidelity (EC-012, Postcondition 6(b)/EC-002,
  EC-007/EC-008):** `is_decision_log_row_marker` re-tested against bare, parenthetical-suffixed, and
  hyphen-suffixed rows; oracle set-equality cross-check, leading-preamble-alone-reaches-cap flush gate,
  and empty-caller-offsets-consults-oracle-first guard all re-verified against their respective BC
  clauses and test fixtures.
- **`error-taxonomy.md` v1.13 Message-Format↔`Display` parity, full re-sweep:** independently
  re-performed the 13-code / 17-emission mechanical re-diff across `ShardRollError`,
  `MechanismABackfillError`, and `ShardRetentionError` — every row MATCH, zero new drift found
  (confirms passes 11/12/13's comment-only and doc-only fixes introduced no taxonomy drift).
- **Spec-internal consistency (BC-1.18.007 + BC-1.18.008 cross-references, Normalization rule vs.
  Record-Boundary Marker Table scoping):** re-verified coherent; no re-emergence of the pass-4
  (F-C3-P4-001) contradiction shape.
- **POLICY-11 test integrity:** every test asserting a BC-1.18.007/BC-1.18.008 postcondition or edge
  case re-confirmed to exercise real production code paths (no tautological self-referential
  assertions, no test-only shortcuts bypassing the production gate).

## Observations (non-blocking)

- **O-C3-P14-001 (LOW).** An `archive_overflow_shards` failure (`ShardRetentionError::ArchivalMoveFailed`,
  taxonomy `E-SHD-002`) is re-coded as `MechanismABackfillError::Io`/`E-SHD-003` when surfaced in the
  backfill context, flattening the `#[source]` chain by one level — the same nesting shape independently
  observed at pass-10 as O-C3-P10-002, now re-confirmed unchanged at pass-14. Both codes still appear in
  the surfaced text (the `E-SHD-003` `Display`'s documented `<io-error>` placeholder carries the
  `E-SHD-002` inner message verbatim), and the fail-loud disposition plus canonical-untouched guarantee
  are fully satisfied per PC5/EC-003 — this is a diagnostic-clarity observation, not a correctness or
  parity defect. Defensible uniform-treatment choice (one abort path for all `MechanismABackfillError`
  I/O-class failures, regardless of which retention-layer error triggered it); flagged for OPTIONAL
  product-owner adjudication — whether a dedicated `MechanismABackfillError` variant preserving the
  `E-SHD-002` code and `#[source]` chain one level deeper would be preferable to the current
  reuse-of-`E-SHD-003` treatment. Not a defect; no fix required or warranted this burst.

Non-blocking, LOW severity — the streak is NOT reset.

## Disposition Summary

Zero blocking findings. Full v1.8 contract independently re-verified spec-conformant across every
dimension in scope (recovery three-way classification, manifest-authoritative slice-and-verify, all 3
destructive-write read-backs, `[backfill_manifest]` persistence, decision-log regex + boundary
fidelity, error-taxonomy Message-Format↔Display parity, spec-internal consistency, and POLICY-11 test
integrity). 1 LOW non-blocking observation recorded — O-C3-P14-001 (re-confirms the same
E-SHD-002/E-SHD-003 diagnostic-nesting shape already tracked at O-C3-P10-002; OPTIONAL
product-owner adjudication flagged, not a defect) — deferred, code intentionally UNCHANGED this burst.
**BC-5.39.001 cluster-3 LOCAL streak: 2/3 → 3/3 — CONVERGED** (3rd consecutive clean pass of the
restarted streak, 4th clean pass overall this cascade, after pass-10, pass-12, and pass-13).
`feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb`. **The cluster-3 LOCAL adversarial cascade is
CLOSED.**

## Code Gate (this burst)

No code change this burst (CLEAN pass, no findings to fix; the one LOW observation is deferred, not
fixed). Feature branch `feature/S-25.02-backfill` remains at `2dd39bbb`: full
`cargo test --workspace --all-targets` suite green; `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean (re-confirmed current at `2dd39bbb`, no
new commit required).

## Next

**Literal BC-5.39.001 3-CLEAN convergence reached for cluster-3** on frozen `2dd39bbb` (passes 12, 13,
14). The LOCAL adversarial cascade is CLOSED — no further adversary passes are scheduled for this
cluster absent a future code change. **NEXT = code CONVERGED and ready for per-story delivery**
(demo-recorder per-AC → push → pr-manager 9-step PR cycle → merge), pending human GO to proceed with
delivery.
