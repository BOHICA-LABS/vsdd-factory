---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 11
verdict: BEHAVIORAL CLEAN — NOT CLEAN OVERALL (1 MEDIUM doc-staleness finding, streak RESET)
finding_count: 1
finding_breakdown: "0 behavioral/code findings (Critical/High/Medium/Low); 1 MEDIUM doc-staleness finding (F-C3-P11-001, stale transient-status test doc comments — 4th recurrence of the stale-header class); 2 LOW non-blocking observations (O-1 archival-move orphan on crash, no atomicity violation; O-2 write_atomic non-UTF-8 fail-loud, unreachable for UTF-8 markdown)"
streak: 0/3
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

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-11

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-3-CLEAN using
> LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `8e2a37f4` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.8 and BC-1.18.007 (retention/compaction) v1.2, run against the SAME frozen code pass-10 left
> in place, per the convergence-discipline directive that passes 11 and 12 run on stable code. This
> is the eleventh pass of the cluster-3 LOCAL cascade.

## Verdict

**BEHAVIORAL CLEAN — NOT CLEAN OVERALL.** Reviewing `feature/S-25.02-backfill` @ `8e2a37f4` against
`shard_manager.rs`, `bc_1_18_007_shard_retention_test.rs`, `bc_1_18_008_backfill_split_test.rs`,
BC-1.18.007.md v1.2, BC-1.18.008.md v1.8, and `prd-supplements/error-taxonomy.md` v1.13, the
adversary independently re-verified the full v1.8 contract end to end and found the shipped code
spec-conformant in every behavioral dimension checked — zero Critical/High/Medium/Low behavioral or
code defects. One MEDIUM doc-staleness finding is recorded (see below); per BC-5.39.001, any
non-LOW finding resets the streak — **BC-5.39.001 cluster-3 LOCAL streak: 1/3 → 0/3 (RESET)** (cycle
-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

## Part A — Findings

**F-C3-P11-001 (MEDIUM, doc-staleness).** `bc_1_18_008_backfill_split_test.rs` carries several test
doc comments still phrased in transient, commit-relative "expected to fail today" / "MUST fail
today" / "there is no post-hoc read-back at all yet" framing, and one comment names the wrong
primitive (`write_atomic_bytes`, a function that does not exist in the shipped implementation —
the real helper is `write_and_read_back`). All of this prose contradicts the shipped fixes landed
across passes 1-10 (every one of the three destructive-write sites has carried a genuine post-hoc
disk read-back since pass-8, F-C3-P8-002) and contradicts the file's own POLICY-11 anti-staleness
rule (test doc comments must describe current, stable behavior by BC/EC/finding-ID citation, not a
transient pass/fail claim). This is the **4th recurrence** of the exact same defect shape previously
codified at D-1192/D-1194 (`L-BB-D1194-transient-status-test-doc-comment-recurring-3x-process-gap`,
routed to follow-up story S-12.09): F-C3-P1-005 (pass-1), F-C3-P2-004 (pass-2), O-2/pass-4, and now
F-C3-P11-001 (pass-11). The prior codification and its S-12.09 follow-up story — a `vsdd-factory:
test-writer` agent-prompt amendment forbidding transient-status prose — has NOT prevented recurrence,
because S-12.09 remains a draft stub; no BC has been authored and no agent-prompt change has landed.
This 4th occurrence is evidence the prose-only codification (a lesson entry + a draft story with no
enforcement mechanism) is, by itself, insufficient to close this recurrence class.

## Observations (non-blocking)

- **O-1 (LOW).** The archival-move step in the retention path is not atomic with respect to a
  process crash mid-move: a crash between the source removal and the destination write could leave
  an orphaned partial file. Not a data-loss risk in practice — a re-run of the retention pass
  overwrites the orphan deterministically, and no caller observes a torn intermediate state. No
  BC/spec/code change required.
- **O-2 (LOW).** `write_and_read_back`'s underlying `write_atomic` helper treats any non-UTF-8 byte
  sequence as a hard failure (fail-loud) rather than attempting a lossy or best-effort write. This
  is correct and intentional for this cluster's artifacts (all three are UTF-8 markdown), and the
  fail-loud path is unreachable for any content this mechanism actually writes today. Recorded for
  completeness only, no action required.

Both observations are LOW severity and non-blocking per BC-5.39.001 — neither resets the streak on
its own; the streak reset this pass is driven entirely by F-C3-P11-001 (MEDIUM).

## Disposition Summary

Zero behavioral/code defects — the full v1.8 contract (recovery/heal/manifest, all 3 destructive-
write read-backs, decision-log regex, boundary detection, preamble handling, per-shard-cap
accounting, oracle set-equality, error-taxonomy Message-Format↔Display parity, spec-internal
consistency, POLICY-11 test integrity) independently re-verified spec-conformant across every
dimension in scope, matching pass-10's clean behavioral result. One MEDIUM doc-staleness finding
(F-C3-P11-001) — the 4th recurrence of the stale-transient-status-test-header class — FIXED
same-burst by test-writer via an EXHAUSTIVE sweep of the file (8 stale-comment sites converted to
past-tense, status-neutral historical narrative, comment-only, all 59 tests still green). 2 LOW
observations recorded, non-blocking. **BC-5.39.001 cluster-3 LOCAL streak: 1/3 → 0/3 — RESET** (the
pass-10 clean result is voided by this reset; the streak restarts fresh on the comment-fixed code).
`feature/S-25.02-backfill` advances `8e2a37f4` → `2dd39bbb` (comment-only fix, no behavior change).

## Code Gate (this burst)

Finding fixed this burst (comment-only, no production code change): `feature/S-25.02-backfill` @
`2dd39bbb` (immediately after `8e2a37f4`, pushed). Full `cargo test --workspace --all-targets` suite
green (59 `bc_1_18_008` tests); `cargo fmt --check --all` clean; `cargo clippy --workspace
--all-targets -- -D warnings` clean.

## Next

**NEXT = pass-12, fresh context, against the NEW frozen `2dd39bbb` code** (the comment-only fix is
this streak's new baseline — the frozen-code discipline applies to the restarted streak the same way
it applied to `8e2a37f4` before this reset). Per the standing escalation from this 4th recurrence,
S-12.09 (E-12 Engine Governance — test-writer agent-prompt amendment forbidding transient-status
doc-comment prose) should be PRIORITIZED: prose-level codification alone has not closed this class
across 4 occurrences; the agent-prompt amendment is the mechanism that would.
