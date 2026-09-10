---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 5
verdict: NOT CLEAN — 1 LOW finding (twin of P3-002), substantive-defect surface EXHAUSTED
finding_count: 1
finding_breakdown: "0 BLOCKER/HIGH/MEDIUM; 1 LOW (F-C3-P5-001, empty-caller-offsets silent no-op); 1 non-blocking already-adjudicated integration observation (re-surfaced, no new routing)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "22ffc00a"
diff_head: "22ffc00a"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "db8f1bd"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-5

> Fresh-context adversarial review (Iron Law: fresh context, no prior-pass visibility beyond this
> cascade's own convention) of `feature/S-25.02-backfill` @ `22ffc00a` against BC-1.18.008
> (mechanism-A one-time backfill-split) v1.5 and BC-1.18.007 (retention/compaction) v1.2, run after
> pass-4's fix-burst (D-1194) landed on the branch — the pass-4 spec-internal wording fix
> (Normalization rule per-artifact-scoping) plus the pass-4 O-2 status-neutral doc-comment rewrite.
> This is the fifth LOCAL BC-5.39.001 cascade pass run against the mechanism-A implementation; per
> the cluster-3 convention established at pass-1, the full Part A finding set is persisted here as a
> standalone artifact.

## Verdict

**NOT CLEAN — 1 LOW finding.** The adversary independently re-derived and re-verified every fix
landed across passes 1-4 (tautological-gate replacement, oracle set-equality cross-check,
`oversized_record` propagation, preamble-seeded `partition_bytes`, Leading-Preamble Handling Rule +
`is_preamble_shard`/`records` fields + per-shard-cap hard gate, `is_known_mechanism_a_artifact_stem`
allow-list, tightened marker-heading predicates, the per-artifact-scoped Normalization rule wording)
and found **zero HIGH/MEDIUM defects** — the correctness surface remains clean for the second
consecutive pass. However, this pass surfaced **1 LOW finding**: an empty-caller-`offsets` input
silently no-ops the mandated split instead of consulting the oracle first (the exact twin shape of
pass-3's F-C3-P3-002, one call-site class removed). Because a finding was present, this pass does
**NOT** satisfy BC-5.39.001's CLEAN bar. BC-5.39.001 cluster-3 LOCAL streak: **0/3 → 0/3** (cycle-level
BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track). With this pass, the cascade has now run 5
consecutive fresh-context passes against a progressively narrowing implementation; no further
Critical/High/Medium-shaped code defects surfaced this pass or last pass — the substantive CODE
defect surface for mechanism-A is assessed EXHAUSTED. Per D-1170/D-1184's asymptotic-acceptance
precedent (cluster-2) and the human's explicit direction (see Next), the LOCAL cascade proceeds to
pass-6 as the FIRST attempt of a full human-authorized 3-CLEAN drive, rather than closing early via
asymptotic acceptance.

## Part A — Findings

### F-C3-P5-001 (LOW) — Empty caller-`offsets` silently no-ops the mandated split instead of consulting the oracle (twin of F-C3-P3-002)

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`,
`mechanism_a_backfill_split_artifact` (the top-level entry point that accepts caller-supplied
`offsets: &[usize]` and delegates boundary detection/content-preservation verification to
`mechanism_a_record_boundary_offsets`).

**Finding:** when the caller passes an EMPTY `offsets` slice for a RECOGNIZED artifact stem whose
real content is non-empty (i.e., the oracle — `mechanism_a_record_boundary_offsets` re-derived from
the marker-table — would independently detect one or more real record boundaries), the pre-pass-3
code path (prior to F-C3-P3-002's `is_known_mechanism_a_artifact_stem` allow-list gate) trusted the
caller's empty list outright and silently returned a no-op (a single "shard" containing the entire
unsplit content) instead of invoking the split at all. F-C3-P3-002 closed the PARALLEL defect for an
UNRECOGNIZED artifact stem (fail-loud `UnrecognizedArtifactStem` instead of blind trust) but did not
extend the same discipline to the RECOGNIZED-stem-but-EMPTY-caller-offsets case: for a recognized
stem, `mechanism_a_backfill_split_artifact` still short-circuits on `offsets.is_empty()` BEFORE
calling `mechanism_a_record_boundary_offsets` to obtain the oracle's own boundary set, so it never
learns that real boundaries exist. Concretely: for `decision-log.md` content containing several
`| D-NNN |` rows, a caller that (incorrectly, e.g. due to an upstream bug) passes `offsets: &[]`
gets a single unsplit shard back with no error and no split performed — the exact "mandated split
silently skipped" failure shape BC-1.18.008 Postcondition 1 requires to fire loudly, not silently.
This is functionally the SAME class of defect as F-C3-P3-002 (an empty/absent input trusted without
oracle cross-check) recurring one call-site layer further out: F-C3-P3-002 closed it at the
oracle-vs-caller CROSS-CHECK layer (`mechanism_a_record_boundary_offsets`'s own PC6(b) gate); this
finding is the EMPTY-CALLER-INPUT layer one function up, at `mechanism_a_backfill_split_artifact`'s
own entry guard, which never reaches the cross-check at all when `offsets` starts empty.

**Impact:** LOW — no production caller exists yet for `mechanism_a_backfill_split_artifact`
(F-C3-P1-006's HUMAN-ADJUDICATED T-12 deferral, re-confirmed this pass, see Observation below), so
this is not reachable in production today. It is a genuine latent correctness gap in the function's
own contract, though: a future T-12 caller that (incorrectly) constructs an empty `offsets` list —
whether via an upstream bug, a stale cache, or a not-yet-fully-initialized boundary scan — would
silently receive an unsplit "success" result instead of a loud failure, for exactly the artifacts
(non-empty, recognized-stem content) BC-1.18.008 Postcondition 1 mandates splitting. Distinguished
from Observation O-1/pass-4 (documented, confirmed COMPLIANT): O-1 covers the reverse direction
(recognized stem + GENUINELY empty CONTENT, where an empty oracle is the CORRECT answer and trusting
the caller is safe because the oracle would agree) — this finding covers recognized stem + NON-EMPTY
content + an EMPTY CALLER LIST, where the oracle would disagree and the code currently never asks it.

**Disposition:** FIXED on `feature/S-25.02-backfill` (implementer, landed at HEAD `26c79f13`,
immediately after `22ffc00a`). The empty-caller-`offsets` arm of `mechanism_a_backfill_split_artifact`
now UNCONDITIONALLY consults `mechanism_a_record_boundary_offsets` (the oracle) before deciding
whether to no-op: if the oracle's own boundary set is also empty (the true O-1 case — recognized
stem, genuinely empty content, or an unrecognized-but-allow-listed edge), the no-op proceeds exactly
as before (behavior-preserving for every real target artifact, since all four always yield a
non-empty oracle set for non-empty content per O-1's own invariant). If the oracle's boundary set is
NON-empty while the caller's was empty, the function now ABORTS with the SAME
`ContentPreservationFailed` error F-C3-P2-001's set-equality cross-check already uses (citing the
oracle's boundary count vs. the caller's 0), rather than silently no-opping. This closes the gap
using the SAME error type and SAME fail-loud philosophy already established by the pass-2/pass-3
gates — no new error variant, no `error-taxonomy.md` change. **No BC/AC/EC/VP/behavior change**: this
is a pure code-side hardening fix inside the already-specified fail-loud contract (Postcondition 6's
"fail loud rather than silently mis-split" clause already covers this case in substance; the code
had not yet fully implemented it for this specific call shape). test-writer added 1 RED test
(`mechanism_a_backfill_split_empty_caller_offsets_nonempty_oracle_aborts`, constructing a recognized
stem with real `| D-NNN |` content and an empty caller `offsets` list, asserting
`ContentPreservationFailed`) plus 1 companion GREEN test
(`mechanism_a_backfill_split_empty_caller_offsets_empty_content_no_ops`, pinning the O-1 valid-empty
path continues to no-op correctly, preventing a future regression from over-tightening the fix into
rejecting the legitimate empty/empty case). Full `cargo test --workspace --all-targets` green (46
tests in the mechanism-A module, +2 from pass-4's 44); `cargo fmt --check --all` clean; `cargo
clippy --workspace --all-targets -- -D warnings` clean.

## Observations (non-blocking)

- **Integration observation (re-surfaced, ALREADY HUMAN-ADJUDICATED, no new routing)** —
  `mechanism_a_record_boundary_offsets`/`mechanism_a_backfill_split_artifact` still has no
  production caller wired in (no PreToolUse/PostToolUse hook, CLI entry point, or scheduled job
  invokes the mechanism-A backfill-split path). This is the SAME item first raised at pass-1
  (F-C3-P1-006) and already HUMAN-ADJUDICATED as a legitimate scope-boundary deferral, recorded in
  STATE.md's Drift Items table as `[D-1191] F-C3-P1-006` — HUMAN-ADJUDICATED DEFERRAL, anchored to
  **T-12** (a real, existing S-25.02 task ID; the Cohort-B-flip capstone, cluster-7), per BC-1.18.008
  Postcondition 1's "once, at F4 activation" scoping. A fresh-context pass independently re-derives
  the same absence of a production caller (as expected — the deferral is still in effect and no
  cluster-3-scope work has changed it), but this is NOT a new finding: it is the identical,
  already-adjudicated item re-surfacing because a fresh-context adversary cannot see prior
  adjudications and necessarily re-notices the same structural fact. **Disposition: no new action, no
  new Drift Item, no re-routing** — the existing `[D-1191]` deferral to T-12 remains the authoritative
  disposition; recorded here per this cascade's convention of noting what a fresh pass re-observes,
  purely for the audit trail.

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P5-001 | LOW | FIXED — empty-caller-`offsets` arm now consults the oracle first, aborts `ContentPreservationFailed` when real boundaries exist; valid empty-content/empty-oracle path preserved (O-1 regression-guarded by a companion GREEN test) | `feature/S-25.02-backfill` @ `26c79f13` |
| Integration observation | — (already-adjudicated, no severity) | No new action — SAME item as F-006/pass-1 (`[D-1191]`), already HUMAN-ADJUDICATED deferred to T-12; re-surfaced by fresh context, no new routing | — |

## Code Gate (this burst)

Full `cargo test --workspace --all-targets` suite: green (0 failed; 46 tests in the mechanism-A
module, all green post-`26c79f13`, +2 new tests from this pass). `cargo fmt --check --all`: clean.
`cargo clippy --workspace --all-targets -- -D warnings`: clean. Feature branch HEAD:
`feature/S-25.02-backfill` @ `26c79f13` (implementer's F-C3-P5-001 fix + test-writer's RED+GREEN
pair, immediately after `22ffc00a`), pushed to `origin`.

## Next

The substantive-defect surface for mechanism-A's CODE is assessed EXHAUSTED after 5 passes (2
consecutive passes — 4 and 5 — with zero HIGH/MEDIUM code defects, only a LOW finding each time, both
now fixed). Per human direction, the cascade does NOT close via asymptotic acceptance here (unlike
cluster-2's D-1184 precedent): the human has authorized a full grind-to-literal-3-CONSECUTIVE-CLEAN
drive (the same standard cluster-1 reached at D-1172), starting fresh at **cluster-3 LOCAL BC-5.39.001
cascade pass-6** — fresh context, against BC-1.18.008 v1.5 / BC-1.18.007 v1.2 / code
`feature/S-25.02-backfill` @ `26c79f13`. Pass-6 is the FIRST attempt of that 3-CLEAN drive (streak
resets to 0/3 for counting purposes going into pass-6, consistent with the literal streak definition
— pass-5 was not clean).
