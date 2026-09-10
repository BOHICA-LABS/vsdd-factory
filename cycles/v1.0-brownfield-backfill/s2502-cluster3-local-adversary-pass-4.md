---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 4
verdict: CODE CLEAN — NOT CLEAN OVERALL (1 SPEC-internal MEDIUM)
finding_count: 1
finding_breakdown: "0 CODE findings (Critical/High/Medium); 1 MEDIUM SPEC-internal contradiction (F-C3-P4-001); 3 non-blocking observations (O-1 LOW documentary, O-2 LOW process-gap-class — 3RD recurrence, O-3 LOW confirmed-compliant)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "10f49d1c"
diff_head: "10f49d1c"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "5a5b563"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-4

> Fresh-context adversarial review (Iron Law: fresh context, no prior-pass visibility beyond this
> cascade's own convention) of `feature/S-25.02-backfill` @ `10f49d1c` against BC-1.18.008
> (mechanism-A one-time backfill-split) v1.4 and BC-1.18.007 (retention/compaction) v1.2, run after
> pass-3's fix-burst (D-1193) landed on the branch. This is the fourth LOCAL BC-5.39.001 cascade
> pass run against the mechanism-A implementation; per the cluster-3 convention established at
> pass-1, the full Part A finding set is persisted here as a standalone artifact.

## Verdict

**CODE CLEAN — NOT CLEAN OVERALL.** The adversary independently re-derived and re-verified every
fix landed across passes 1-3 (tautological-gate replacement, oracle set-equality cross-check,
`oversized_record` propagation, preamble-seeded `partition_bytes`, Leading-Preamble Handling Rule +
`is_preamble_shard`/`records` fields + per-shard-cap hard gate, `is_known_mechanism_a_artifact_stem`
allow-list, tightened marker-heading predicates) and found **zero Critical/High/Medium CODE
defects** — the first pass in this cascade to certify the CODE surface fully clean. However, the
pass surfaced **1 MEDIUM finding that is SPEC-internal, not code-internal**: a wording
contradiction inside BC-1.18.008 Postcondition 2 itself (F-C3-P4-001). Because a MEDIUM-severity
finding was present — even though it identifies a spec-prose defect rather than a code defect — this
pass does **NOT** satisfy BC-5.39.001's CLEAN bar. BC-5.39.001 cluster-3 LOCAL streak: **0/3 → 0/3**
(cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

## Part A — Findings

### F-C3-P4-001 (MEDIUM) — Postcondition 2 "Normalization rule" clause (a) contradicts the same Postcondition's own Record-Boundary Marker Table

**Location:** `.factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md`, Postcondition 2, the
"Normalization rule (single implementable predicate)" sub-clause introduced at v1.2 and carried
forward unchanged through v1.4.

**Finding:** the Normalization rule's clause (a) reads "it matches `^## ` (any h2)" as an
unqualified disjunct in a predicate stated to hold across all four mandatory backfill-split
target artifacts. Read literally and applied uniformly, this makes EVERY `^## ` heading in EVERY
one of the four artifacts a record boundary. This directly contradicts the SAME Postcondition's own
authoritative Record-Boundary Marker Table two clauses earlier, whose `decision-log.md` row keys
record boundaries on `^\| D-[0-9]+ \|` table rows (NOT any h2 — a bare `## Decisions Log` or
`## Appendix: Sub-clause Expansion` heading is an explicit section label, not a record) and whose
`lessons.md` row keys record boundaries on three TAGGED h2 forms only (`## L-<tag>-NNN`,
`## LESSON (D-NNN)`, `## RECURRENCE NOTE (D-NNN)`) — an untagged `## ` aside in `lessons.md` is
explicitly NOT a boundary per that same table. Applied literally, clause (a) would silently
over-split `decision-log.md` at its own section-label headings and `lessons.md` at untagged asides,
directly contradicting the marker table two paragraphs above it in the same Postcondition. This is
the SAME clause O-C3-P2-003 (pass-2, non-blocking observation, "spec-wording imperfection, code is
right, prose is loose") flagged as a Drift Item — this pass independently re-derives it as a
BLOCKING contradiction (a genuine internal inconsistency within one Postcondition's own text, not
merely an isolated-reading ambiguity) and escalates it from an observation to a finding.

**Impact:** MEDIUM — a spec-internal contradiction inside the authoritative source document for
mechanism-A's own record-boundary detection. Confirmed NOT a code defect: `mechanism_a_record_boundary_offsets`
was independently re-verified this pass to already implement the marker-table-scoped behavior
correctly (`decision-log.md` keyed on `^\| D-[0-9]+ \|` only; `lessons.md` keyed on the three tagged
h2 forms only) against all four real target artifacts in both cycle directories — the shipped
implementation was never defective. The defect is purely in the BC's own prose: a future reader or
implementer relying on clause (a) in isolation, without cross-referencing the marker table, could be
misled into a genuinely incorrect re-implementation.

**Disposition:** ROUTED to product-owner. product-owner amended **BC-1.18.008 v1.4→v1.5**: the
Normalization rule is now explicitly PER-ARTIFACT-SCOPED and subordinate to the Record-Boundary
Marker Table (restated as the table's own authoritative predicate form, not an independent additive
source of boundaries) — clause (a)'s "any h2" wording is stated to hold as written ONLY for
`burst-log.md` and `session-checkpoints.md` (whose marker-table rows say "any h2 heading"), and is
explicitly OVERRIDDEN for `decision-log.md` (primary key `^\| D-[0-9]+ \|`; bare `## ` headings are
section labels, not boundaries) and `lessons.md` (only the three tagged h2 forms; untagged `## `
asides are not boundaries). The fail-loud clause for an unrecognized future heading form
(Postcondition 6's gate) is preserved verbatim in substance. **No AC/EC/VP/behavior change** — this
is a pure spec-internal consistency fix; the shipped code already implements the marker-table-scoped
behavior this amendment now states unambiguously. No Canonical Test Vector requires updating (every
existing vector already asserts marker-table-consistent expected outputs, never the contradictory
literal-disjunctive reading).

## Observations (non-blocking)

- **O-1 (LOW)** — recognized-stem-empty-oracle invariant: each of the four recognized artifacts
  (`decision-log.md`, `burst-log.md`, `lessons.md`, `session-checkpoints.md`) always yields a
  non-empty oracle boundary set for non-empty content (each always contains at least one
  `| D-NNN |` row / h2 / tagged-h2 marker per the Record-Boundary Marker Table). Consequently, a
  "recognized stem + empty oracle ⇒ trust caller" code branch is UNREACHABLE in production for any
  of the four artifacts and exists only to support synthetic test inputs that deliberately construct
  an empty-content fixture; the unrecognized-stem case is separately and fully covered by
  Postcondition 6's fail-loud gate (closed at pass-3, F-C3-P3-002). Documented via a new note in
  BC-1.18.008 v1.5 (Postcondition 2, immediately after the amended Normalization rule) recording the
  invariant for future readers; no detection-behavior change.

- **O-2 (LOW) — [process-gap-class]** — the test module's doc comment describing the
  `mechanism_a_verify_backfill_per_shard_cap_preserved` hard-gate fixtures (added at pass-3 for
  F-C3-P3-001) used transient "expected to fail pre-fix" / "does not yet exist" framing that had
  gone stale the moment pass-3's own implementer fix landed in the same burst — a reader encountering
  the module in isolation post-fix would be misled about current test status/intent. This is the
  **THIRD occurrence** of the exact same transient-status-header defect shape in this cluster's own
  cascade: pass-1's F-C3-P1-005 (stale doc comment describing a withdrawn exact-`ceil()` framing),
  pass-2's F-C3-P2-004 (test-module header still describing F-001/F-002 in "expected to fail pre-fix"
  framing), and now this pass-4 instance. Per the project's established 3+-recurrence rule (the same
  threshold that triggered `[process-gap]` codification for the weak-substring-error-assertion class
  at D-1183/cluster-2 pass-9), this THIRD recurrence MUST be codified as a `[process-gap]` rather
  than tracked as a further one-off fix. **Fixed this burst** (test-writer, `feature/S-25.02-backfill`
  @ `22ffc00a`, comments only — no test logic change; 44 tests still green). **Escalated**: the
  STATE.md `[D-1192] [process-watch]` Drift Item tracking this class at 2/3 recurrences is
  CODIFIED this burst as a `[process-gap]`, routed to a new draft follow-up story (S-12.09, E-12
  Engine Governance) recommending a test-writer agent-prompt amendment forbidding transient-status
  prose ("expected to fail" / "does not yet exist" / "RED surface") in test doc-comment headers,
  requiring status-neutral "this test pins X" framing instead.

- **O-3 (LOW)** — Postcondition 6(c)'s prose wording ("every sealed shard's `bytes_at_seal <=
  shard_cap_bytes` unless flagged `oversized_record: true`") versus the implementation's actual
  grain (the hard gate `mechanism_a_verify_backfill_per_shard_cap_preserved` checks this per-shard,
  post-hoc, over the full sealed shard index) was independently re-derived and confirmed COMPLIANT
  this pass — no wording/implementation mismatch found, no action needed.

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P4-001 | MEDIUM (spec-internal, no code defect) | FIXED — BC-1.18.008 v1.4→v1.5 (Normalization rule per-artifact-scoped, subordinate to Record-Boundary Marker Table); NO AC/EC/VP/behavior change | `.factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md` (factory-artifacts, this burst) |
| O-1 | LOW (documentary) | Documented — new note in BC-1.18.008 v1.5, no behavior change | `.factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md` |
| O-2 | LOW (process-gap-class, 3RD recurrence) | FIXED this burst (status-neutral doc-comment rewrite) + CODIFIED as `[process-gap]`, routed to new draft follow-up story S-12.09 (E-12) | `feature/S-25.02-backfill` `22ffc00a`; `.factory/stories/STORY-INDEX.md` (S-12.09 stub) |
| O-3 | LOW (confirmed compliant) | No action — PC6(c) wording vs. implementation grain re-derived compliant | — |

## Code Gate (this burst)

Full `cargo test --workspace --all-targets` suite: green (0 failed; 44 tests in the mechanism-A
module, all green post-`22ffc00a`). `cargo fmt --check --all`: clean. `cargo clippy
--workspace --all-targets -- -D warnings`: clean. Feature branch HEAD: `feature/S-25.02-backfill` @
`22ffc00a` (test-writer's O-2 comment-only fix, immediately after `10f49d1c`), pushed to `origin`.

## Next

Cluster-3 LOCAL BC-5.39.001 cascade pass-5, fresh context, against BC-1.18.008 v1.5 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `22ffc00a`.
