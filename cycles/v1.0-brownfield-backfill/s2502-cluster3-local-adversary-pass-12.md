---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 12
verdict: CLEAN — zero blocking findings; 1 LOW non-blocking observation (deferred, does not reset streak)
finding_count: 0
finding_breakdown: "0 blocking findings; 1 LOW non-blocking observation (O-C3-P12-001, MissingBackfillManifest/E-SHD-011 form (b) fail-loud path untested)"
streak: 1/3
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

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-12

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-true-3-CLEAN
> using LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `2dd39bbb` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.8 and BC-1.18.007 (retention/compaction) v1.2, run against the NEW frozen baseline pass-11's
> exhaustive comment sweep left in place (streak RESET to 0/3 at pass-11 on a MEDIUM doc-staleness
> finding, zero behavioral/code defects). This is the twelfth pass of the cluster-3 LOCAL cascade and
> the **first CLEAN pass of the restarted streak** — the second CLEAN pass overall in this cascade
> (after pass-10).

## Verdict

**CLEAN — zero blocking findings.** Reviewing `feature/S-25.02-backfill` @ `2dd39bbb` against
`shard_manager.rs`, `bc_1_18_007_shard_retention_test.rs`, `bc_1_18_008_backfill_split_test.rs`,
BC-1.18.007.md v1.2, BC-1.18.008.md v1.8, and `prd-supplements/error-taxonomy.md` v1.13, the
adversary independently re-verified the full v1.8 contract end to end and found the shipped code
spec-conformant in every dimension checked — zero Critical/High/Medium/Low blocking findings. One
LOW non-blocking observation is recorded (see below); per BC-5.39.001, a LOW/non-blocking-only
observation does not reset the streak. **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3** (streak
restart; cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

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
  (confirms pass-11's comment-only fix introduced no taxonomy drift).
- **Spec-internal consistency (BC-1.18.007 + BC-1.18.008 cross-references, Normalization rule vs.
  Record-Boundary Marker Table scoping):** re-verified coherent; no re-emergence of the pass-4
  (F-C3-P4-001) contradiction shape.
- **POLICY-11 test integrity:** every test asserting a BC-1.18.007/BC-1.18.008 postcondition or edge
  case re-confirmed to exercise real production code paths (no tautological self-referential
  assertions, no test-only shortcuts bypassing the production gate); pass-11's 8-site comment sweep
  re-inspected and confirmed comment-only (no assertion text or test logic altered).

## Observations (non-blocking)

- **O-C3-P12-001 (LOW).** The `MechanismABackfillError::MissingBackfillManifest` / `E-SHD-011` form
  (b) fail-loud path (a shard index present with `[[shard]]` entries but no `[backfill_manifest]`
  table) is UNTESTED — the prior manifest-less-index test was retired at pass-6 when the recovery arm
  was rebuilt around manifest identity. The code path was independently verified CORRECT on inspection
  against BC-1.18.008 v1.8's Postcondition 3/5 text and the shipped `Display`. This manifest-less state
  cannot arise in the normal course of this one-time F4 migration — it is a defensive guard against a
  pre-v1.6/legacy shard index reaching the recovery arm, and `write_shard_index_for_backfill` always
  writes a manifest alongside the index on every code path that can produce one. **Suggested
  improvement** (not required to close this observation, not a blocking finding): a fixture that
  hand-writes a shard-index file with `[[shard]]` entries and no `[backfill_manifest]` table, then
  asserts the code returns `E-SHD-011` form (b), would close the coverage gap without requiring any
  behavior change.

This observation is LOW severity and non-blocking per BC-5.39.001 — the streak is NOT reset. DEFERRED
to the existing follow-up story S-12.12 (see this burst's STATE.md Drift Items registration) rather
than fixed in-scope this burst, per the human's explicit convergence-discipline direction: the code
stays frozen at `2dd39bbb` through the remaining passes needed to reach literal 3-CLEAN.

## Disposition Summary

Zero blocking findings. Full v1.8 contract independently re-verified spec-conformant across every
dimension in scope (recovery three-way classification, manifest-authoritative slice-and-verify, all 3
destructive-write read-backs, `[backfill_manifest]` persistence, decision-log regex + boundary
fidelity, error-taxonomy Message-Format↔Display parity, spec-internal consistency, and POLICY-11 test
integrity). 1 LOW non-blocking observation recorded and DEFERRED to the existing follow-up story
S-12.12 (E-12 Engine Governance) — code intentionally UNCHANGED this burst to keep the streak on
stable, frozen code. **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3** (streak restart; first clean
pass of the restarted streak, second clean pass overall this cascade). `feature/S-25.02-backfill`
stays UNCHANGED at `2dd39bbb`.

## Code Gate (this burst)

No code change this burst (CLEAN pass, no findings to fix; the 1 LOW observation is deferred, not
fixed). Feature branch `feature/S-25.02-backfill` remains at `2dd39bbb`: full
`cargo test --workspace --all-targets` suite green; `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean (re-confirmed current at `2dd39bbb`, no
new commit required).

## Next

**NEXT = pass-13, fresh context, against the SAME frozen `2dd39bbb` code** — per convergence
discipline, the remaining passes toward literal 3-CLEAN (13 and 14) must run on stable code, not on a
moving target; the 1 LOW observation from this pass is deferred to S-12.12 rather than fixed in-scope
specifically to preserve that stability. If pass-13 is also CLEAN, streak advances to 2/3; a third
consecutive CLEAN pass (14) would reach literal BC-5.39.001 3-CLEAN convergence for cluster-3.
