---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 10
verdict: CLEAN — zero blocking findings; 2 LOW non-blocking observations (deferred, do not reset streak)
finding_count: 0
finding_breakdown: "0 blocking findings; 2 LOW non-blocking observations (O-C3-P10-001 spawn_temp_file_corruptor race-determinism risk; O-C3-P10-002 E-SHD-002/E-SHD-003 wrap-nesting diagnostic-clarity note)"
streak: 1/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "8e2a37f4"
diff_head: "8e2a37f4"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "2027698"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-10

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-3-CLEAN using
> LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `8e2a37f4` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.8 and BC-1.18.007 (retention/compaction) v1.2, run immediately after pass-9's fix-burst
> (D-1199) landed on the branch. This is the tenth pass of the cluster-3 LOCAL cascade and the
> **FIRST CLEAN pass** — the pass-10 VSDD 10-pass guardrail referenced at the close of pass-9's
> report is superseded by this clean result; no human convergence-disposition decision is required
> this burst.

## Verdict

**CLEAN — zero blocking findings.** Reviewing `feature/S-25.02-backfill` @ `8e2a37f4` against
`shard_manager.rs`, `bc_1_18_007_shard_retention_test.rs`, `bc_1_18_008_backfill_split_test.rs`,
BC-1.18.007.md v1.2, BC-1.18.008.md v1.8, and `prd-supplements/error-taxonomy.md` v1.13, the
adversary independently re-verified the full v1.8 contract end to end and found the shipped code
spec-conformant in every dimension checked — zero Critical/High/Medium/Low blocking findings. Two
LOW non-blocking observations are recorded (see below); per BC-5.39.001, a LOW/non-blocking-only
observation does not reset the streak. **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3** (cycle-level
BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

## Part A — Findings

None. Full re-verification coverage, all CONFIRMED spec-conformant:

- **Recovery / heal / manifest correctness (BC-1.18.008 Postcondition 3/5, Invariant 3, `E-SHD-011`,
  `E-SHD-012`):** the Backfill Recovery Manifest identity check, the Manifest-Authoritative
  Slice-and-Verify Rule, and the DANGEROUS-window heal's manifest-derived offset arithmetic
  independently re-derived and confirmed correct against BC-1.18.008 v1.8 text.
- **All 3 destructive-write read-backs (Postcondition 6(c), Invariant 5):** sealed-shard write,
  DANGEROUS-window heal write, and happy-path canonical-truncate write each independently confirmed
  to route through the shared `write_and_read_back` helper with a genuine post-hoc disk read-back and
  manifest-verified `(length, sha256)` comparison before any write is treated as durable.
- **`decision-log.md` marker-table regex (EC-012):** `is_decision_log_row_marker` re-tested against
  the corrected `^\| D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|` pattern; bare, parenthetical-suffixed,
  and hyphen-suffixed rows all match correctly.
- **Boundary detection / oracle set-equality (Postcondition 6(b), EC-002, EC-007/EC-008):** the
  oracle set-equality cross-check, the leading-preamble-alone-reaches-cap flush gate, and the
  empty-caller-offsets-consults-oracle-first guard all re-verified against their respective BC
  clauses and test fixtures.
- **Preamble handling (Postcondition 6(c), Invariant 4):** `is_preamble_shard` flagging and
  `partition_bytes` preamble-length seeding re-confirmed consistent with the spec's per-shard-cap
  accounting.
- **Oracle set-equality error taxonomy / cross-references:** `E-SHD-007` through `E-SHD-013` messages
  re-checked against `error-taxonomy.md` v1.13's Message Format column, including the three cells
  corrected at pass-9 (`E-SHD-002`, `E-SHD-003`, `E-SHD-011` form (b)) — all now MATCH the shipped
  `Display` text verbatim.
- **`error-taxonomy.md` v1.13 Message-Format↔`Display` parity, full re-sweep:** independently
  re-performed the same 13-code / 17-emission mechanical re-diff pass-9's companion sweep ran,
  across `ShardRollError`, `MechanismABackfillError`, and `ShardRetentionError` — every row MATCH,
  zero new drift found (confirms pass-9's fix burst did not introduce a new instance of the
  recurring drift class it was closing).
- **Spec-internal consistency (BC-1.18.007 + BC-1.18.008 cross-references, Normalization rule vs.
  Record-Boundary Marker Table scoping):** re-verified coherent; no re-emergence of the pass-4
  (F-C3-P4-001) contradiction shape.
- **POLICY-11 test integrity:** every test asserting a BC-1.18.007/BC-1.18.008 postcondition or edge
  case re-confirmed to exercise real production code paths (no tautological self-referential
  assertions, no test-only shortcuts bypassing the production gate).

## Observations (non-blocking)

- **O-C3-P10-001 (LOW, `[process-gap]`).** The 3 disk-read-back fault-injection tests
  (`FC3P6002`/`FC3P7001`/`FC3P8002`, covering the sealed-shard, DANGEROUS-window heal, and
  happy-path canonical write-and-read-back sites respectively) each spawn a
  `spawn_temp_file_corruptor` background thread that races the production write path to corrupt the
  just-written temp file before the read-back verification runs, bounded to 20 retries and
  hard-failing the test if the race never lands within that bound. This is correct and
  non-tautological — it genuinely exercises the read-back-catches-corruption path rather than
  asserting a mock — but the race-based construction carries a non-zero risk of a rare spurious
  FALSE-FAIL (the corruptor thread never wins the race within 20 retries) on a heavily-loaded CI
  runner, which would surface as CI flake rather than a real regression. **Suggested improvement**
  (not required to close this observation, not a blocking finding): a deterministic
  `#[cfg(test)]` fault-injection seam — mirroring the existing `FORCE_STAGE_FAILURE` thread-local
  pattern already used elsewhere in `shard_manager.rs` — that deterministically corrupts the target
  file at the read-back checkpoint rather than racing a background thread, removing the flakiness
  risk without weakening what the test actually covers.
- **O-C3-P10-002 (LOW).** A PC4 archival-move failure inside `ShardRetentionError::ArchivalMoveFailed`
  (`E-SHD-002`) is wrapped, at the one call site where mechanism-A's backfill retry surfaces it, as
  `MechanismABackfillError::Io` (`E-SHD-003`) — so a diagnostic message carrying the literal
  `E-SHD-002:` prefix can appear nested inside `{source}` slot of an `E-SHD-003:`-prefixed outer
  message. This is defensible, not a taxonomy-parity violation: `E-SHD-003`'s own Message Format cell
  documents a generic `<io-error>` placeholder in its `{source}` position specifically to permit
  wrapping arbitrary I/O-shaped failures, and the wrapping correctly signals
  abort-original-untouched-rerunnable semantics (the `E-SHD-003` contract) rather than misrepresenting
  the failure as directly retention-layer-caused. The only cost is diagnostic-clarity noise — an
  operator reading the nested message sees two code prefixes in one string. No BC/spec/code change
  required; recorded for the same follow-up story as O-C3-P10-001 in case a future taxonomy-lint gate
  (S-12.11's proposed remedy) wants an explicit nested-wrap annotation convention.

Both observations are LOW severity and non-blocking per BC-5.39.001 — the streak is NOT reset.
DEFERRED to a follow-up story (see this burst's STORY-INDEX registration) rather than fixed in-scope
this burst, per the human's explicit convergence-discipline direction: the remaining 2 passes needed
to reach literal 3-CLEAN (passes 11 and 12) must run against the SAME frozen code (`8e2a37f4`) as
this clean pass, so no code change is made this burst.

## Disposition Summary

Zero blocking findings. Full v1.8 contract independently re-verified spec-conformant across every
dimension in scope (recovery/heal/manifest, all 3 destructive-write read-backs, decision-log regex,
boundary detection, preamble handling, oracle set-equality, error-taxonomy Message-Format↔Display
parity — including a full independent re-sweep of pass-9's own fix — spec-internal consistency, and
POLICY-11 test integrity). 2 LOW non-blocking observations recorded and DEFERRED to a new draft
follow-up story (E-12 Engine Governance) — code intentionally UNCHANGED this burst to keep the
streak on stable, frozen code. **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3** (first clean pass).
`feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4`.

## Code Gate (this burst)

No code change this burst (CLEAN pass, no findings to fix; the 2 LOW observations are deferred, not
fixed). Feature branch `feature/S-25.02-backfill` remains at `8e2a37f4`: full
`cargo test --workspace --all-targets` suite green; `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean (re-confirmed current at `8e2a37f4`, no
new commit required).

## Next

**NEXT = pass-11, fresh context, against the SAME frozen `8e2a37f4` code** — per convergence
discipline, the 2 remaining passes toward literal 3-CLEAN (11 and 12) must run on stable code, not on
a moving target; the 2 LOW observations from this pass are deferred to a follow-up story rather than
fixed in-scope specifically to preserve that stability. If pass-11 is also CLEAN, streak advances to
2/3; a third consecutive CLEAN pass (12) would reach literal BC-5.39.001 3-CLEAN convergence for
cluster-3.
