---
document_type: cycle-decision-log
producer: state-manager
cycle: v1.0-brownfield-backfill
version: "1.0"
---

# v1.0-brownfield-backfill Cycle Decision Log

Historical decision-log entries moved from STATE.md during compaction. Most recent entries (D-104+) remain in STATE.md.

| ID | Decision | Rationale | Phase | Date | Made By |
|----|----------|-----------|-------|------|---------|

> Archived 2026-09-20: appendix sections D-1182..D-1191 (exhaustive) relocated to `decision-log-archive-through-D1191.md` (fuel-wall compaction, completing the D-731..D-1191 relocation). This file now retains D-1192 onward.

## D-1192

**D-1192-S2502-CLUSTER3-PASS2-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1191 (this file, immediately above). D-1192 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-2 = NOT CLEAN** 2026-09-10 (fresh-context adversary + implementer + test-writer code-side
content; state-manager bookkeeping + single-commit TD-VSDD-053) — 2 HIGH (F-C3-P2-001,
F-C3-P2-002) + 2 MEDIUM (F-C3-P2-003, F-C3-P2-004) + 3 non-blocking observations (O-C3-P2-001,
O-C3-P2-002, O-C3-P2-003). Full Part A persisted as a standalone artifact, matching pass-1's own
convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-2.md` (`diff_base=41c81fc4`,
`diff_head=5d195519`).

**F-C3-P2-001 (HIGH):** the PC6(b) content-preservation gate fixed at pass-1 (F-C3-P1-001) replaced
the original tautological gate with an independent recompute, but implemented the cross-check as a
UNION of the caller-supplied offsets and the oracle's own detected boundaries, compared by
cardinality only — structurally blind to over-detection (a caller offsets list that is a strict
superset of the true boundaries contributes nothing new to a union, so `|caller ∪ oracle| ==
|caller|` even though a spurious extra offset would physically split a real record across two
shard files) and separately vulnerable to a same-count swap. FIXED (implementer, `5d195519`): the
gate now performs an oracle SET-EQUALITY cross-check (sorted, deduplicated caller offsets must
exactly match the independently-recomputed true boundary set whenever the oracle recognizes any
boundary at all), catching under-, over-, and same-count-swap mis-detection uniformly, with an
explicit `MechanismABackfillError::ContentPreservationFailed` detail message citing both failure
directions. RED fixtures added (`3bdf83f7`) for strict-superset over-detection and
same-count-swap; both fail pre-fix, pass post-fix.

**F-C3-P2-002 (HIGH):** BC-1.18.008 EC-002's single-oversized-record exception flag was computed
per-partition by `mechanism_a_partition_for_backfill` but silently dropped at shard-index
publication — `ShardIndexEntry` had no field to carry it, so no downstream reader of the
shard-index could distinguish a legitimate EC-002 exception from a cap-accounting bug without
re-deriving it against the source content. FIXED (implementer, `5d195519`): added `pub
oversized_record: bool` to `ShardIndexEntry` (`#[serde(default, skip_serializing_if =
"std::ops::Not::not")]`, mirroring `sealed_retroactively`'s additive-compatibility precedent),
populated from `partition.oversized_record` at `run_mechanism_a_backfill_split`'s publication
site. TD-VSDD-060 sibling-swept across every other `ShardIndexEntry` construction site
(`execute_roll`, `self_heal_resume_from_truncate`, `self_heal_reconcile_missing_index_entries`,
and all in-module test fixtures) — each explicitly set `oversized_record: false` with a scope-note
comment, since none of those call sites is a backfill-split partition.

**F-C3-P2-003 (MEDIUM):** BLOCKER-1's pre-pass-1 fix folded leading preamble bytes into the first
partition's `bytes` accumulator, but the separate `partition_bytes` accumulator that the
Postcondition 2 cap decision actually reads was still seeded at `0` — a real leading preamble could
be physically written into the first partition while never counting toward the cap-flush decision,
silently allowing an over-cap shard. FIXED (implementer, `5d195519`): `partition_bytes` now seeded
with `record_boundary_offsets[0] as u64` (the exact preamble length), matching BLOCKER-1's `bytes`
seeding; no-op when there is no preamble.

**F-C3-P2-004 (MEDIUM):** the test-module header for the `bc_1_18_008_backfill_split_test.rs` suite
still described the F-001/F-002 fixtures in transient "expected to fail pre-fix" framing, stale
since pass-1's fixes (`41c81fc4`) landed — misleading to a reader encountering the module in
isolation. This is the SECOND occurrence of a transient-status-in-a-doc-comment defect in this
cluster's own cascade (the first, F-C3-P1-005 at pass-1, described a withdrawn `ceil()` framing in
the same module). FIXED (test-writer, `3bdf83f7`): header rewritten to status-neutral prose,
describing what each fixture verifies by BC/EC/finding-ID citation rather than a transient
pass/fail expectation tied to a specific commit's state. **[process-watch]** recorded in STATE.md
Drift Items: if this class recurs a 3rd time, it crosses the BC-5.39.001 3×-recurrence threshold
and MUST be codified as a process-gap (test-writer agent-prompt amendment for status-neutral test
headers).

**Observations (non-blocking, no in-scope action):** O-C3-P2-001 — the original PC6(b)
content-preservation (byte-identity) check is tautological in isolation once F-C3-P2-001's
set-equality check also runs, but the two are intentional defense-in-depth against independent
failure modes (boundary-list divergence vs. a downstream partitioning bug), not a defect.
O-C3-P2-002 — a preamble-only, zero-record shard is an untested edge case for the partitioning
path; not reachable against the 4 real target artifacts, flagged for a future pass. O-C3-P2-003
(SPEC-WORDING) — BC-1.18.008 Postcondition 2's normalization clause (a) reads as if "any `^## `
h2" applies uniformly to all 4 target artifacts including `decision-log.md`, though the shipped
code correctly keys `decision-log.md`'s own record boundaries on `^\| D-[0-9]+ \|` per the BC's own
marker-table elsewhere — the code is correct, the clause (a) prose is loosely worded. Recorded as a
Drift Item (SPEC-HYGIENE), anchored to the next BC-1.18.008 spec touch (product-owner); NOT fixed
this burst (no spec amendment this pass — a non-blocking wording tightening, not a defect).

No BC/story/VP/index content change this burst — BC-1.18.008 stays v1.3, BC-1.18.007 stays v1.2,
S-25.02 story stays v3.4, input-hashes CONFIRMED UNCHANGED (BC-1.18.008.md `a68be55`; story
`e47d034`); BC-INDEX v5.76 / STORY-INDEX v4.453 / VP-INDEX v3.09 / ARCH-INDEX v4.24 all UNCHANGED.

Full code gate GREEN on `feature/S-25.02-backfill` @ `5d195519` (pushed to origin): `bc_1_18_008`
suite 34/34 passed; `cargo fmt --check --all` clean; `cargo clippy --workspace --all-targets -- -D
warnings` clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-2 not clean; pass-3 next,
fresh context; cycle-level 3/3 CONVERGED streak UNCHANGED — separate track). No trajectory-tail
drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level adversary pass).
`pipeline:` stays **PAUSED** (mid-convergence fix burst; consistent with prior cluster fix-burst
state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-3, fresh context, against BC-1.18.008 v1.3 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `5d195519`.**

Refs: D-1192, D-1191, S-25.02, BC-1.18.008 v1.3, F-C3-P2-001..004, O-C3-P2-001..003, `3bdf83f7`,
`5d195519`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1192 | D-1192-S2502-CLUSTER3-PASS2-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-2 = NOT CLEAN — 2 HIGH (F-C3-P2-001, F-C3-P2-002) + 2 MEDIUM (F-C3-P2-003, F-C3-P2-004) + 3 non-blocking observations (O-C3-P2-001..003).** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-2.md`. F-C3-P2-001 (HIGH): PC6(b) gate's pass-1 union cross-check was blind to over-detection/same-count-swap — replaced with an oracle SET-EQUALITY cross-check. F-C3-P2-002 (HIGH): EC-002's `oversized_record` flag was computed but dropped at shard-index publication — added `oversized_record: bool` to `ShardIndexEntry`, populated, TD-VSDD-060 sibling-swept. F-C3-P2-003 (MEDIUM): `partition_bytes` cap-accounting accumulator never counted the leading preamble — seeded with preamble length. F-C3-P2-004 (MEDIUM): stale "expected to fail" test-module header rewritten to status-neutral prose — 2nd occurrence of this class this cluster, `[process-watch]` recorded (3rd occurrence crosses BC-5.39.001 threshold, MUST codify). 3 observations non-blocking: O-C3-P2-001 (tautological-but-intentional defense-in-depth), O-C3-P2-002 (untested preamble-only zero-record edge), O-C3-P2-003 (SPEC-HYGIENE — BC-1.18.008 PC2 clause (a) wording loose vs. correct code, Drift Item anchored next spec touch). No BC/story/index content change — BC-1.18.008 stays v1.3, story stays v3.4, input-hashes UNCHANGED. Feature branch `feature/S-25.02-backfill` @ `5d195519` (pushed); `bc_1_18_008` 34/34 green, fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-2 not clean; pass-3 next, fresh context; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-3, fresh context, against BC-1.18.008 v1.3/code `5d195519`.** Refs: D-1192, D-1191, S-25.02, BC-1.18.008 v1.3, F-C3-P2-001..004, `3bdf83f7`, `5d195519`. STATE.md v10.17→v10.18. | S-25.02 F4 | 2026-09-10 |

## D-1193

**D-1193-S2502-CLUSTER3-PASS3-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1192 (this file, immediately above). D-1193 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-3 = NOT CLEAN** 2026-09-10 (fresh-context adversary + implementer + test-writer code-side
content + product-owner spec amendment; state-manager bookkeeping + single-commit TD-VSDD-053) — 1
HIGH (F-C3-P3-001), 1 MEDIUM (F-C3-P3-002), 1 MINOR (F-C3-P3-003), plus 1 non-blocking observation
(O-C3-P3-001). Full Part A persisted as a standalone artifact, matching pass-1/pass-2's own
convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-3.md` (`diff_base=5d195519`,
`diff_head=5d195519`).

**F-C3-P3-001 (HIGH):** a leading preamble large enough that the preamble ALONE reaches
`shard_cap_bytes` had no sanctioned flush point — the greedy boundary-preserving packer only
flushes at a record boundary, and the preamble is not a record — so the first sealed shard's
`bytes_at_seal` could silently exceed `shard_cap_bytes` with `oversized_record` left `false` (not
EC-002's single-oversized-record exception, since the excess content is a non-record preamble).
Not reachable against the 4 real target artifacts today; a genuine, silent per-shard-cap escape
hatch nonetheless. FIXED: product-owner amended **BC-1.18.008 v1.3→v1.4** — new **Leading-Preamble
Handling Rule** (a first partition whose accumulated preamble bytes alone reach or exceed
`shard_cap_bytes` MUST be flushed as its own preamble-only shard, explicitly flagged, before any
record is packed into it), new **Postcondition 6(c)** fail-loud hard gate (every sealed shard's
`bytes_at_seal <= shard_cap_bytes` unless flagged `oversized_record: true`, the degenerate-preamble
case being a sanctioned sub-case of the same flag), an **Invariant 4** restatement scoping the
per-shard-cap bound to explicitly cover the preamble-only-shard case, and new EC-007
(large-but-under-cap preamble folds into the first record shard unflagged, per pass-2's existing
behavior) / EC-008 (preamble alone at/over cap: preamble-only shard sealed and flagged). Implementer
(`10f49d1c`): added `is_preamble_shard: bool` and `records: Vec<...>` fields to the
partition/shard-index bookkeeping (TD-VSDD-060 sibling-swept across every `ShardIndexEntry`/
partition construction site touched by this change), a preamble-only flush path executed before the
greedy record-packing loop when the preamble alone is at/over cap, and a new hard gate
`mechanism_a_verify_backfill_per_shard_cap_preserved` that explicitly checks every sealed shard's
`bytes_at_seal` against `shard_cap_bytes`, failing loud unless the shard carries `oversized_record:
true`. RED fixtures added (`16effd52`) for EC-007/EC-008; both exercised pre-fix, pass post-fix.

**F-C3-P3-002 (MEDIUM):** the PC6(b) set-equality oracle cross-check's documented empty-oracle
fallback ("when the oracle recognizes no boundary at all... the caller's offsets are trusted at
face value") was implemented as a blanket fallback keyed only on zero detected boundaries, not on
whether `artifact_stem` is actually one of the 4 legitimate mechanism-A targets — an unrecognized
or mistyped stem silently fell into the same "trust the caller" path as genuinely-empty recognized
content, reintroducing blind trust for that input shape. FIXED (implementer, `10f49d1c`): added
`is_known_mechanism_a_artifact_stem` (an explicit allow-list check against the 4 real target
stems), gating the empty-oracle fallback so it only fires for a recognized stem with genuinely no
detected boundaries; an unrecognized stem now returns
`MechanismABackfillError::UnrecognizedArtifactStem` fail-loud. test-writer (`16effd52`) rebuilt the
existing F-004 fixtures — which had incidentally been using an oracle-undetectable stem, routing
them through the blind-trust fallback rather than the real cross-check — to use an
oracle-detectable stem/content pairing.

**F-C3-P3-003 (MINOR):** `is_lesson_h2_record_heading` and `is_pass_fix_burst_heading` used loose
`starts_with`-style matching that over-matches a small number of legitimate sibling headings
sharing a common prefix in BC-1.18.008's own marker-table (e.g. `## LESSON CATEGORY:` over-matching
`## LESSON`). Not reachable against the 4 real target artifacts' actual current content. FIXED
(implementer, `10f49d1c`): both predicates tightened to exact marker-table heading forms (anchored
prefix plus a required following delimiter/whitespace boundary).

**Observation (non-blocking, no in-scope action):** O-C3-P3-001 — `heal_or_confirm_already_migrated`'s
structural-prefix heuristic relies on a structural-prefix match rather than a stronger
content-hash-based confirmation; not reachable as a defect against the current single-operator,
single-invocation activation model (F-C3-P1-006's own T-12 scope-boundary deferral means this code
path has no production caller yet). Recorded for completeness — no test added, no fix made, no
Drift Item opened; the function's actual load-bearing behavior is owed the T-12 production-wiring
review.

story-writer's S-25.02 v3.4→v3.5: AC-013/AC-014 EXTENDED IN PLACE (Leading-Preamble Handling
Rule/Postcondition 6(c)/Invariant 4/EC-007/EC-008); new §Edge Cases rows EC-048/EC-049 mirroring BC
EC-007/EC-008; RED-Gate/stub-coverage count stays 25 ACs (AC-013/AC-014 extended, not new).

This burst: BC-INDEX v5.76→v5.77 (BC-1.18.008 cell v1.3→v1.4); STORY-INDEX v4.453→v4.454 (S-25.02
BC list cell BC-1.18.008 v1.3→v1.4 sync + story-cell v3.4→v3.5); VP-INDEX v3.09→v3.10 (architect's
same-burst propagation, VP-123 v1.0→v1.1 THIRD proptest facet — per-shard-cap invariant — per
BC-1.18.008 v1.4's own routing note; `total_vps` UNCHANGED 141, one facet extension, no new VP
allocated); verification-architecture.md v1.26→v1.27 + verification-coverage-matrix.md
v1.24→v1.25 (POLICY 9 propagation, same burst); ARCH-INDEX v4.24 UNCHANGED. Input-hashes
reconciled via `compute-input-hash --update`: BC-1.18.008.md `a68be55`→`763d2ab` (own body amended
v1.3→v1.4); story `e47d034`→`5cf0eda` (cascading recompute after the BC hash update);
verification-architecture.md/verification-coverage-matrix.md CONFIRMED CURRENT `eb285db`
(already updated by architect same burst); `--check` CLEAN on all four.

**STATE.md "26 VPs" advisory RE-CONFIRMED, NOT changed:** architect's `validate-count-propagation`
re-flagged the same pre-existing scope-mismatch false positive already investigated and closed at
D-1177 and RE-CONFIRMED at D-1186 — STATE.md's "26 VPs" citations are a legitimate STORY-scoped
count (S-25.02's own `verification_properties:` range VP-116..VP-141), independently correct
alongside VP-INDEX's catalog-wide `total_vps: 141`; both numbers are accurate for what they each
describe, and overwriting STATE.md's story-scoped citation to 141 would itself be a regression. Not
changed this burst — RE-CONFIRMED per the established D-1177 Drift Item, still anchored to a future
`validate-count-propagation` source fix (comparison-semantics scoping), out of state-manager's
`.factory/`-only tool access.

Full code gate GREEN on `feature/S-25.02-backfill` @ `10f49d1c` (RED-fixtures commit `16effd52`
immediately prior, both pushed to origin): full `cargo test --workspace --all-targets` suite green;
`cargo fmt --check --all` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-3 not clean; pass-4 next, fresh context;
cycle-level 3/3 CONVERGED streak UNCHANGED — separate track). No trajectory-tail drift — unchanged
→0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level adversary pass). `pipeline:` stays
**PAUSED** (mid-convergence fix burst; consistent with prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-4, fresh context, against BC-1.18.008 v1.4 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `10f49d1c`.**

Refs: D-1193, D-1192, S-25.02, BC-1.18.008 v1.4, F-C3-P3-001..003, O-C3-P3-001, VP-123 v1.1,
`16effd52`, `10f49d1c`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1193 | D-1193-S2502-CLUSTER3-PASS3-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-3 = NOT CLEAN — 1 HIGH (F-C3-P3-001) + 1 MEDIUM (F-C3-P3-002) + 1 MINOR (F-C3-P3-003) + 1 non-blocking observation (O-C3-P3-001).** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-3.md`. F-C3-P3-001 (HIGH): a leading preamble alone reaching `shard_cap_bytes` had no sanctioned flush point, silently exceeding the first shard's cap unflagged — product-owner amended **BC-1.18.008 v1.3→v1.4** (Leading-Preamble Handling Rule, new Postcondition 6(c) fail-loud gate, Invariant 4 restatement, EC-007/EC-008); implementer added `is_preamble_shard`/`records` fields (TD-VSDD-060 sibling-swept), a preamble-only flush path, and the `mechanism_a_verify_backfill_per_shard_cap_preserved` hard gate. F-C3-P3-002 (MEDIUM): the PC6(b) empty-oracle fallback trusted the caller for ANY unrecognized artifact_stem — fixed via `is_known_mechanism_a_artifact_stem` allow-list gating, fail-loud on a miss; test-writer rebuilt the F-004 fixtures to be oracle-detectable. F-C3-P3-003 (MINOR): `is_lesson_h2_record_heading`/`is_pass_fix_burst_heading` over-matched marker-table siblings — tightened to exact-form matches. O-C3-P3-001 (observation, non-blocking): `heal_or_confirm_already_migrated` structural-prefix heuristic noted, deferred to the T-12 production-wiring review. BC-INDEX v5.76→v5.77; STORY-INDEX v4.453→v4.454 (story v3.4→v3.5); VP-INDEX v3.09→v3.10 (VP-123 v1.0→v1.1, THIRD proptest facet, `total_vps` UNCHANGED 141); verification-architecture.md v1.26→v1.27 + verification-coverage-matrix.md v1.24→v1.25 (POLICY 9 propagation). Input-hashes reconciled (BC-1.18.008.md `a68be55`→`763d2ab`; story `e47d034`→`5cf0eda`); `--check` CLEAN on all four. STATE.md "26 VPs" advisory RE-CONFIRMED as the pre-existing D-1177 scope-mismatch false positive — NOT changed (would be a regression). Feature branch `feature/S-25.02-backfill` @ `10f49d1c` (RED `16effd52`, both pushed); full workspace test suite green, fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-3 not clean; pass-4 next, fresh context; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-4, fresh context, against BC-1.18.008 v1.4/code `10f49d1c`.** Refs: D-1193, D-1192, S-25.02, BC-1.18.008 v1.4, F-C3-P3-001..003, O-C3-P3-001, `16effd52`, `10f49d1c`. STATE.md v10.18→v10.19. | S-25.02 F4 | 2026-09-10 |

## D-1194

**D-1194-S2502-CLUSTER3-PASS4-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1193 (this file, immediately above). D-1194 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-4 = CODE CLEAN — NOT CLEAN OVERALL** 2026-09-10 (fresh-context adversary + product-owner spec
amendment + test-writer code-side content; state-manager bookkeeping + single-commit TD-VSDD-053)
— 0 CODE findings (Critical/High/Medium), 1 MEDIUM SPEC-internal contradiction (F-C3-P4-001), plus
3 non-blocking observations (O-1/O-2/O-3). The adversary independently re-derived and re-verified
every fix landed across passes 1-3, finding zero Critical/High/Medium CODE defects — the first
CODE-clean pass in this cascade. Full Part A persisted as a standalone artifact, matching pass-1/2/3's
own convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-4.md` (`diff_base=10f49d1c`,
`diff_head=10f49d1c`).

**F-C3-P4-001 (MEDIUM, SPEC-internal, not a code defect):** BC-1.18.008 Postcondition 2's
"Normalization rule" clause (a) — "it matches `^## ` (any h2)" — read as an unqualified disjunct
applying uniformly across all four mandatory backfill-split target artifacts, directly contradicting
the SAME Postcondition's own authoritative Record-Boundary Marker Table two clauses earlier, whose
`decision-log.md` row keys record boundaries on `^\| D-[0-9]+ \|` table rows only (NOT any h2 — a
bare `## Decisions Log` heading is a section label, not a record) and whose `lessons.md` row keys
record boundaries on three TAGGED h2 forms only (an untagged `## ` aside is explicitly NOT a
boundary). Applied literally, clause (a) would silently over-split `decision-log.md` at its own
section labels and `lessons.md` at untagged asides — a genuine internal inconsistency within one
Postcondition's own text. This escalates pass-2's O-C3-P2-003 observation (non-blocking, "spec
wording loose, code correct") to a blocking finding: the same imprecise clause, independently
re-derived by a fresh-context pass as a genuine contradiction rather than merely an isolated-reading
ambiguity. Confirmed NOT a code defect — `mechanism_a_record_boundary_offsets` was independently
re-verified this pass to already implement the marker-table-scoped behavior correctly against all
four real target artifacts in both cycle directories. FIXED: product-owner amended **BC-1.18.008
v1.4→v1.5** — the Normalization rule is now explicitly PER-ARTIFACT-SCOPED and subordinate to the
Record-Boundary Marker Table (restated as the table's own authoritative predicate form, not an
independent additive source of boundaries); clause (a)'s "any h2" wording is stated to hold as
written ONLY for `burst-log.md`/`session-checkpoints.md` (whose marker-table rows say "any h2
heading"), and is explicitly OVERRIDDEN for `decision-log.md` (primary key `^\| D-[0-9]+ \|`) and
`lessons.md` (only the three tagged h2 forms). The fail-loud clause for an unrecognized future
heading form (Postcondition 6's gate) is preserved verbatim in substance. **No AC/EC/VP/behavior
change** — pure spec-internal consistency fix; no Canonical Test Vector requires updating.

**Observation O-1 (LOW, documentary):** each of the four recognized artifacts always yields a
non-empty oracle boundary set for non-empty content, so a "recognized stem + empty oracle ⇒ trust
caller" code branch is unreachable in production and exists only for synthetic test inputs; the
unrecognized-stem case is separately covered by Postcondition 6's fail-loud gate (closed at
pass-3). Documented via a new Postcondition 2 note in BC-1.18.008 v1.5; no detection-behavior
change.

**Observation O-2 (LOW, process-gap-class — THIRD recurrence, CODIFIED this burst):** the test
module's doc comment describing the pass-3 `mechanism_a_verify_backfill_per_shard_cap_preserved`
hard-gate fixtures used transient "expected to fail pre-fix" / "does not yet exist" framing that
went stale the moment pass-3's own fix landed in the same burst. This is the THIRD occurrence of the
exact same transient-status-header defect shape in this cluster's own cascade: F-C3-P1-005 (pass-1),
F-C3-P2-004 (pass-2), and now this instance. Per the project's established 3+-recurrence rule (the
same threshold that triggered `[process-gap]` codification for the weak-substring-error-assertion
class at D-1183/cluster-2 pass-9), this THIRD recurrence is CODIFIED as a `[process-gap]` rather
than tracked as a further one-off fix. FIXED this burst (test-writer, `feature/S-25.02-backfill` @
`22ffc00a`, comments only — no test logic change; 44 tests still green). The STATE.md `[D-1192]
[process-watch]` Drift Item tracking this class at 2/3 recurrences is ESCALATED this burst to
`[process-gap][codified]`, routed to a NEW draft follow-up story **S-12.09** (E-12 Engine
Governance, registered this burst in `STORY-INDEX.md`) recommending a test-writer agent-prompt
amendment forbidding transient-status prose ("expected to fail" / "does not yet exist" / "RED
surface") in test doc-comment headers, requiring status-neutral "this test pins X" framing instead.
Codified as lesson `L-BB-D1194-transient-status-test-doc-comment-recurring-3x-process-gap`
(`cycles/v1.0-brownfield-backfill/lessons.md`).

**Observation O-3 (LOW):** Postcondition 6(c)'s prose wording versus the implementation's actual
per-shard, post-hoc verification grain (`mechanism_a_verify_backfill_per_shard_cap_preserved`) was
independently re-derived and confirmed COMPLIANT this pass — no mismatch found, no action needed.

story-writer's S-25.02 v3.5→v3.6: version-cell + changelog only — PO confirmed a pure
wording/consistency fix with NO AC/EC/VP/behavior change, so AC-013/AC-014 body text is UNCHANGED
(both already describe the marker-table-scoped, per-artifact behavior and never repeated the
additive-disjunct phrasing this amendment corrects). §Behavioral Contracts table BC-1.18.008 cell
v1.4→v1.5 (Role cell unchanged).

This burst: BC-INDEX v5.77→v5.78 (BC-1.18.008 cell v1.4→v1.5); STORY-INDEX v4.454→v4.455 (S-25.02
BC list cell BC-1.18.008 v1.4→v1.5 sync + story-cell v3.5→v3.6; new S-12.09 draft stub registered
in the E-12 table); VP-INDEX v3.10 UNCHANGED (no VP content touched — this amendment is
wording-only); verification-architecture.md/verification-coverage-matrix.md UNCHANGED; ARCH-INDEX
v4.24 UNCHANGED. Input-hashes reconciled: BC-1.18.008.md CONFIRMED CURRENT `763d2ab` via
`compute-input-hash --check` (own declared `inputs:` unchanged by a body-only amendment); story
`5cf0eda`→`c432a34` via `compute-input-hash --update` (story-writer's own flagged drift, cascading
recompute after the BC hash update); `--check` CLEAN on both.

Full code gate GREEN on `feature/S-25.02-backfill` @ `22ffc00a` (comment-only fix immediately after
`10f49d1c`, pushed to origin): full `cargo test --workspace --all-targets` suite green (44 tests in
the mechanism-A module); `cargo fmt --check --all` clean; `cargo clippy --workspace --all-targets --
-D warnings` clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-4 CODE CLEAN but NOT
CLEAN overall due to the spec-internal MEDIUM, now fixed same-pass; pass-5 next, fresh context;
cycle-level 3/3 CONVERGED streak UNCHANGED — separate track). No trajectory-tail drift — unchanged
→0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level adversary pass). `pipeline:` stays
**PAUSED** (mid-convergence fix burst; consistent with prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-5, fresh context, against BC-1.18.008 v1.5 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `22ffc00a`.**

Refs: D-1194, D-1193, S-25.02, BC-1.18.008 v1.5, F-C3-P4-001, O-1, O-2, O-3, S-12.09,
`22ffc00a`, `10f49d1c`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1194 | D-1194-S2502-CLUSTER3-PASS4-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-4 = CODE CLEAN — NOT CLEAN OVERALL — 0 CODE findings + 1 MEDIUM SPEC-internal contradiction (F-C3-P4-001) + 3 non-blocking observations (O-1/O-2/O-3).** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-4.md`. Adversary independently re-verified every prior fix (passes 1-3) — first CODE-clean pass this cascade. F-C3-P4-001 (MEDIUM, spec-internal): Postcondition 2's Normalization rule clause (a) contradicted the same Postcondition's own Record-Boundary Marker Table — escalates pass-2's O-C3-P2-003 observation to a finding; fixed via product-owner's **BC-1.18.008 v1.4→v1.5** (Normalization rule PER-ARTIFACT-SCOPED, subordinate to the Marker Table); NO AC/EC/VP/behavior change. O-1 (LOW): recognized-stem-empty-oracle invariant documented. O-2 (LOW, process-gap-class, THIRD recurrence F-C3-P1-005→F-C3-P2-004→O-2): fixed (test-writer `22ffc00a`, comments only) and CODIFIED as `[process-gap]`, routed to NEW draft follow-up story **S-12.09** (E-12). O-3 (LOW): PC6(c) wording re-derived compliant, no action. BC-INDEX v5.77→v5.78; STORY-INDEX v4.454→v4.455 (story v3.5→v3.6; S-12.09 registered). VP-INDEX v3.10 UNCHANGED (wording-only amendment). Input-hashes: BC-1.18.008.md CONFIRMED CURRENT `763d2ab`; story `5cf0eda`→`c432a34`; `--check` CLEAN on both. Feature branch `feature/S-25.02-backfill` @ `22ffc00a` (comment-only fix, pushed); full workspace test suite green, fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (spec-internal MEDIUM fixed same-pass; pass-5 next, fresh context; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-5, fresh context, against BC-1.18.008 v1.5/code `22ffc00a`.** Refs: D-1194, D-1193, S-25.02, BC-1.18.008 v1.5, F-C3-P4-001, O-1, O-2, O-3, S-12.09, `22ffc00a`. STATE.md v10.19→v10.20. | S-25.02 F4 | 2026-09-10 |

## D-1195

**D-1195-S2502-CLUSTER3-PASS5-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1194 (this file, immediately above). D-1195 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-5 = NOT CLEAN — 1 LOW finding** 2026-09-10 (fresh-context adversary + implementer + test-writer
code-side content; state-manager bookkeeping + single-commit TD-VSDD-053) — 0 BLOCKER/HIGH/MEDIUM, 1
LOW (F-C3-P5-001), plus 1 non-blocking already-adjudicated integration observation. The adversary
independently re-derived and re-verified every fix landed across passes 1-4, finding zero HIGH/MEDIUM
CODE defects — the correctness surface stays clean for the 2nd consecutive pass. Full Part A
persisted as a standalone artifact, matching pass-1/2/3/4's own convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-5.md` (`diff_base=22ffc00a`,
`diff_head=22ffc00a`).

**F-C3-P5-001 (LOW, twin of F-C3-P3-002):** an empty caller-`offsets` input to
`mechanism_a_backfill_split_artifact`, for a RECOGNIZED artifact stem whose real content is
non-empty, silently no-op'd the mandated split instead of consulting the oracle
(`mechanism_a_record_boundary_offsets`) first — the SAME trust-the-empty-input shape F-C3-P3-002
closed at pass-3 for an UNRECOGNIZED stem (fail-loud `UnrecognizedArtifactStem`), recurring one
call-site layer further out at `mechanism_a_backfill_split_artifact`'s own entry guard, which never
reached the oracle cross-check at all when `offsets` started empty. Not reachable in production today
(no production caller yet — see integration observation below), but a genuine latent correctness gap
in the function's own contract: a future T-12 caller passing an empty `offsets` list (upstream bug,
stale cache, incomplete boundary scan) would silently receive an unsplit "success" instead of the
loud failure BC-1.18.008 Postcondition 1/6 mandates. Distinguished from O-1/pass-4 (documented,
COMPLIANT): O-1 covers empty caller + genuinely empty CONTENT (oracle agrees, trust is safe); this
finding covers empty caller + NON-EMPTY content (oracle disagrees, and the code never asked it).
FIXED: implementer, `feature/S-25.02-backfill` @ `26c79f13` (immediately after `22ffc00a`) — the
empty-caller arm now UNCONDITIONALLY consults the oracle first; if the oracle's boundary set is also
empty, the no-op proceeds exactly as before (behavior-preserving, per O-1's own invariant that all
four real target artifacts always yield a non-empty oracle set for non-empty content); if the
oracle's set is non-empty while the caller's was empty, the function now ABORTS with the SAME
`ContentPreservationFailed` error F-C3-P2-001's set-equality cross-check already uses — no new error
variant, no `error-taxonomy.md` change. **No AC/EC/VP/behavior change** — pure code-side hardening
inside the already-specified fail-loud contract; Postcondition 6's "fail loud rather than silently
mis-split" clause already covered this case in substance. test-writer added 1 RED test
(`mechanism_a_backfill_split_empty_caller_offsets_nonempty_oracle_aborts`) + 1 companion GREEN test
(`mechanism_a_backfill_split_empty_caller_offsets_empty_content_no_ops`, regression-guarding O-1's
valid empty/empty path against over-tightening); 46 tests in the mechanism-A module, all green
(+2 from pass-4's 44).

**Integration observation (re-surfaced, ALREADY HUMAN-ADJUDICATED, no new routing):**
`mechanism_a_record_boundary_offsets`/`mechanism_a_backfill_split_artifact` still has no production
caller wired in. This is the SAME item as F-C3-P1-006/pass-1, already HUMAN-ADJUDICATED as a
legitimate scope-boundary deferral and recorded as STATE.md Drift Item `[D-1191]`, anchored to
**T-12** (a real, existing S-25.02 task ID; Cohort-B-flip capstone, cluster-7) per BC-1.18.008
Postcondition 1's "once, at F4 activation" scoping. A fresh-context pass necessarily re-notices the
same structural absence (the deferral is unchanged; no cluster-3-scope work has altered it) — this is
NOT a new finding. **Disposition: no new action, no new Drift Item, no re-routing** — the existing
`[D-1191]` deferral to T-12 remains authoritative; recorded here purely for the audit trail per this
cascade's established convention.

**Because a LOW finding (F-C3-P5-001) was present, pass-5 is NOT CLEAN — BC-5.39.001 cluster-3 LOCAL
streak stays 0/3.** With 5 passes run (2 consecutive — passes 4 and 5 — with only a single LOW/no
HIGH-MEDIUM CODE finding each), the substantive CODE defect surface for mechanism-A is assessed
**EXHAUSTED**. Per explicit human direction, the LOCAL cascade does NOT close early via asymptotic
acceptance (the D-1184/cluster-2 precedent); instead the human has AUTHORIZED a full
grind-to-literal-3-CONSECUTIVE-CLEAN drive (the same standard cluster-1 reached at D-1172) — **pass-6
is the FIRST attempt of that drive**, fresh context, against BC-1.18.008 v1.5 / BC-1.18.007 v1.2 /
code `feature/S-25.02-backfill` @ `26c79f13`.

**Process-note (audit trail, TD-FACTORY-HOOK-BYPASS-001 P0 deviation, self-caught):** during the
pass-4 fix burst (commit `33f521ab`, D-1194), the state-manager persisting that burst used a raw
shell `>>` append instead of the Edit/Write tools for one write to
`cycles/v1.0-brownfield-backfill/session-checkpoints.md` (the prior-checkpoint archival append). This
is a TD-FACTORY-HOOK-BYPASS-001 P0 deviation — `.factory/` mutations MUST use Edit/Write only, never
a raw shell append/sed/echo bypass of the hook chain — self-caught during this pass-5 burst's
bookkeeping review, not flagged by the hook chain at the time (PostToolUse hooks do not fire on a
bare shell append that never invoked the Write/Edit tool surface). **Content verified well-formed**
this burst: `session-checkpoints.md` (8,072 lines) was re-read end-to-end at its append boundary
(`git -C .factory show 33f521ab -- cycles/v1.0-brownfield-backfill/session-checkpoints.md`) and at
its current file tail — the appended block is a complete, correctly-terminated
`## Archived checkpoint: ... §1..§8` section with no truncation, no malformed Markdown, and no data
loss; the file's own `wc -l` and its final `### §8. BC-5.39.001 streak` section both terminate
cleanly. No content-recovery action required. Codified as lesson
`L-BB-D1195-hook-bypass-shell-append-self-caught-process-note` (`[process-note]`,
`cycles/v1.0-brownfield-backfill/lessons.md`) — a reminder that state-manager `.factory/` writes MUST
route through Edit/Write exclusively, with no exception for append-only operations, even when the
appended content is itself well-formed.

This burst: BC-INDEX v5.78 UNCHANGED (no BC content touched — pure code-side hardening fix, no spec
amendment); STORY-INDEX v4.455 UNCHANGED (no AC/EC/story content touched); VP-INDEX v3.10 UNCHANGED
(no VP content touched); ARCH-INDEX v4.24 UNCHANGED. No input-hash drift — this pass's own findings
report (`s2502-cluster3-local-adversary-pass-5.md`) carries its own fresh `input-hash` computed via
`compute-input-hash --update` against its declared `inputs:` (BC-1.18.007.md, BC-1.18.008.md, the
S-25.02 story); the 3 declared input files themselves are UNCHANGED by this burst, so their own
stored hashes require no recompute.

Full code gate GREEN on `feature/S-25.02-backfill` @ `26c79f13` (implementer's F-C3-P5-001 fix +
test-writer's RED+GREEN pair, immediately after `22ffc00a`, pushed to origin): full `cargo test
--workspace --all-targets` suite green (46 tests in the mechanism-A module); `cargo fmt --check
--all` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean. BC-5.39.001 cluster-3
LOCAL streak stays **0/3** (pass-5 NOT CLEAN — 1 LOW finding, fixed same-pass; pass-6 next, fresh
context, FIRST attempt of the human-authorized full 3-CLEAN drive; cycle-level 3/3 CONVERGED streak
UNCHANGED — separate track). No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL
cluster-3 cascade, not a cycle-level adversary pass). `pipeline:` stays **PAUSED** (mid-convergence
fix burst; consistent with prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-6, fresh context, against BC-1.18.008 v1.5 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `26c79f13` — FIRST attempt of the human-authorized full
3-CLEAN drive.**

Refs: D-1195, D-1194, S-25.02, BC-1.18.008 v1.5, F-C3-P5-001, S-12.09, `26c79f13`, `22ffc00a`,
`33f521ab`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1195 | D-1195-S2502-CLUSTER3-PASS5-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-5 = NOT CLEAN — 1 LOW finding (F-C3-P5-001, twin of F-C3-P3-002) + 1 non-blocking already-adjudicated integration observation.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-5.md`. Adversary independently re-verified every prior fix (passes 1-4) — correctness surface clean for the 2nd consecutive pass. F-C3-P5-001 (LOW): empty caller-`offsets` for a recognized stem with non-empty real content silently no-op'd the mandated split instead of consulting the oracle — the same shape as F-C3-P3-002, one call-site layer further out; FIXED (`feature/S-25.02-backfill` @ `26c79f13`) via an unconditional oracle-consult before the empty-caller no-op decision, aborting `ContentPreservationFailed` when real boundaries exist; valid empty-content/empty-oracle path (O-1/pass-4) preserved and regression-guarded. No BC/AC/EC/VP/behavior change. Integration observation: no production caller yet — SAME item as F-C3-P1-006/pass-1, ALREADY HUMAN-ADJUDICATED `[D-1191]` deferred to T-12; re-surfaced, no new routing. **Process-note (self-caught, audit trail):** pass-4's burst (`33f521ab`) used a raw shell `>>` append instead of Edit/Write for one `session-checkpoints.md` write — a TD-FACTORY-HOOK-BYPASS-001 P0 deviation; content verified well-formed this burst (no recovery needed); codified `[process-note]` lesson `L-BB-D1195-hook-bypass-shell-append-self-caught-process-note`. BC-INDEX v5.78 / STORY-INDEX v4.455 / VP-INDEX v3.10 / ARCH-INDEX v4.24 all UNCHANGED (pure code-side fix, no spec touched). Feature branch `feature/S-25.02-backfill` @ `26c79f13` (pushed); full workspace test suite green (46 tests), fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-5 not clean; substantive CODE defect surface EXHAUSTED after 2 consecutive LOW-only/clean-correctness passes; pass-6 next = FIRST attempt of the human-authorized full 3-CLEAN drive; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-6, fresh context, against BC-1.18.008 v1.5/code `26c79f13` — FIRST attempt of the 3-CLEAN drive.** Refs: D-1195, D-1194, S-25.02, BC-1.18.008 v1.5, F-C3-P5-001, S-12.09, `26c79f13`, `33f521ab`. STATE.md v10.20→v10.21. | S-25.02 F4 | 2026-09-10 |

## D-1196

**D-1196-S2502-CLUSTER3-PASS6-CROSSVENDOR-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1195 (this file, immediately above). D-1196 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-6 = NOT CLEAN — 2 HIGH + 1 MEDIUM finding, CROSS-VENDOR (OpenAI Codex)** 2026-09-10
(cross-vendor adversary + implementer + test-writer code-side content + product-owner spec content;
state-manager bookkeeping + single-commit TD-VSDD-053) — 2 HIGH (F-C3-P6-001 data-loss,
F-C3-P6-002 spec-fidelity/missing disk-read-back), 1 MEDIUM (F-C3-P6-003, h3 word-boundary,
code-only), ALL THREE fixed this burst. **This is the FIRST CROSS-VENDOR pass run against this BC's
cascade** — the adversary role was fulfilled by OpenAI Codex, not the Claude-family model used for
passes 1-5, per D-1195's explicit human authorization bringing cross-vendor review into the
grind-to-3-CLEAN drive. All 3 findings are NOVEL: none were raised, in this shape, by any of the 5
prior same-vendor passes — F-C3-P6-001's underlying mechanism was examined at pass-3 (O-C3-P3-001)
and explicitly characterized "not reachable today... deferred to T-12" without constructing the
repeated-prefix counterexample that falsifies that characterization; F-C3-P6-002's exact spec
language ("actual bytes written to disk") was read by passes 4 and 5 without the missing-disk-
read-back gap being flagged. Full Part A rendered from the raw Codex review JSON
(`scratchpad/codex-pass6/codex-review.json`) and persisted as a standalone artifact, matching
pass-1..5's own convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-6.md` (`diff_base=26c79f13`,
`diff_head=26c79f13`).

**F-C3-P6-001 (HIGH, data-loss):** the recovery/idempotency arm of the mechanism-A backfill entry
point classified an already-published shard-index re-invocation as "interrupted, needs resuming"
using ONLY a structural byte-prefix comparison (`canonical_bytes[..sealed_concat.len()] ==
sealed_concat[..]`), then unconditionally overwrote the canonical file with the presumed-unsealed
tail. Codex constructed a concrete counterexample — two byte-identical checkpoint records
(`A + A + "more\n"`) — where a second invocation mistakes the already-sealed first record's bytes for
an interruption marker and silently DESTROYS the second record's intact heading and body, which were
never actually sealed and are unrecoverable from the shard files. FIXED: product-owner amended
**BC-1.18.008 v1.5→v1.6**: NEW Backfill Recovery Manifest (`[backfill_manifest]` shard-index table:
`original_bytes`/`original_sha256`, `final_bytes`/`final_sha256`, written durably at split time) added
to Postcondition 3; NEW Recovery-Confirmation Rule added to Postcondition 5 — a re-invocation's
disposition (SAFE no-op / DANGEROUS complete-the-interrupted-truncate / AMBIGUOUS fail loud) is
determined SOLELY by exact whole-file `(length, SHA-256)` comparison against the Manifest's two
recorded pairs, NEVER by re-concatenating sealed shards and comparing prefixes; Invariant 3 rewritten
to require this exact-manifest basis and explicitly forbid the byte-prefix heuristic; NEW EC-009
(legitimate repeated-prefix content correctly classified SAFE) and EC-010 (a match against neither
manifest pair fails loud, new error `E-SHD-011`, rather than guessing). implementer,
`feature/S-25.02-backfill`, rebuilt the recovery function around whole-file `(length, SHA-256)`
identity against the Manifest instead of byte-prefix comparison; product-owner added `E-SHD-011` to
`error-taxonomy.md` v1.9→v1.10. test-writer retired the obsolete pre-v1.6 self-heal test (asserted the
now-forbidden byte-prefix behavior) and added a genuine disk-corruption-race fault-injection test plus
a repeated-prefix regression test (the exact counterexample).

**F-C3-P6-002 (HIGH, spec-fidelity):** BC-1.18.008 Postcondition 6(c)/Invariant 4's own "actual bytes
written to each sealed shard file" / "actual bytes written to disk" language — present in the spec
since pass-3 (D-1193) and read without complaint by passes 4 and 5 — was never actually implemented as
a disk read-back: the production cap-verification gate checked only in-memory
`partition.bytes.len()` BEFORE any write occurred, then wrote, published the index, and rewrote the
canonical file without ever reading a sealed shard file back off disk. FIXED: product-owner ruled
(no BC text amendment required — the existing v1.5 language already specified disk-verification; the
code was non-compliant with already-correct spec text) that PC6(c)/Invariant 4 REQUIRE a genuine
post-hoc disk read-back. implementer extracted `mechanism_a_write_and_verify_sealed_shard`, which
writes via `write_atomic_bytes` then reads the resulting file back off disk (`std::fs::read` against
the sealed path, not the in-memory buffer) and verifies the actual on-disk byte length against
`shard_cap_bytes` (or the `oversized_record`/`is_preamble_shard` exception) before the shard is
considered sealed and before index publication proceeds. test-writer added a disk-corruption-race
fault-injection test (writes a sealed shard, corrupts/truncates it on disk between write and the
verification read-back via a controlled test seam, asserts fail-loud abort).

**F-C3-P6-003 (MEDIUM, code-only, no story/BC change):** `is_id_tagged_lesson_heading`'s numeric-
suffix check (`after_dash.starts_with(|c| c.is_ascii_digit())`) omitted BC-1.18.008 PC2's documented
`\b` word-boundary anchor (`^### L-<tag>-[0-9]+\b`), over-matching `### L-EDP1-050details` and
`### L-EDP1-050_extra` as record boundaries — a nested sub-heading of this shape inside an existing
lesson could become a spurious shard boundary. The THIRD distinct marker-heading-precision bug this
cascade (after F-C3-P1-002/pass-1, F-C3-P3-003/pass-3). FIXED: implementer tightened the predicate to
consume the complete digit run and require the documented word boundary afterward, TD-VSDD-060
sibling-swept against `is_lesson_record_heading`; test-writer added negative alphabetic/underscore-
suffix fixtures plus an end-to-end packing test.

**Propagation:** architect propagated a THIRD VP-124 facet (recovery-confirmation correctness
invariant, per POLICY 9 `vp_index_is_vp_catalog_source_of_truth`, per BC-1.18.008 v1.6's own routing
note): VP-INDEX v3.10→v3.11 (`total_vps` UNCHANGED 141), verification-architecture.md v1.27→v1.28,
verification-coverage-matrix.md v1.25→v1.26 — architect already ran `compute-input-hash --update` on
both arch docs same-burst. story-writer's S-25.02 body v3.6→v3.7: AC-014 EXTENDED IN PLACE (trace
citation unchanged — postcondition 5, postcondition 6, invariant 3, invariant 4 already covered this
content) with the Recovery-Confirmation Rule and the post-hoc disk-read-back ruling; AC-013 body text
UNCHANGED (its traced postconditions 1/2/3 are untouched by v1.6); §Edge Cases gained EC-050 (mirrors
BC EC-009) and EC-051 (mirrors BC EC-010); §Behavioral Contracts BC-1.18.008 cell v1.5→v1.6; §Token
Budget BC-1.18.008 line 5,600→6,600 tokens, Total ~84,400→~85,400 (~43%). BC-INDEX v5.78→v5.79
(BC-1.18.008 version-cell v1.5→v1.6 per POLICY 8); STORY-INDEX v4.455→v4.456 (S-25.02 BC-list cell +
row narrative; NEW draft follow-up story **S-12.10** registered, E-12 Engine Governance).

**Input-hash reconciliation (dependency-ordered):** BC-1.18.008.md declares VP-INDEX.md as an input
(architect's VP-124 facet extension changed it) — recomputed first, `763d2ab`→`d136e83`.
`prd-supplements/error-taxonomy.md` declares BC-1.18.008.md as an input — recomputed second, AFTER
BC-1.18.008.md settled, `226df61`→`c5ba1e0` (an intermediate `b15a45c` value computed before this
dependency ordering was identified is superseded — recorded here for audit-trail completeness, not
otherwise persisted anywhere). The S-25.02 story declares both BC-1.18.008.md and error-taxonomy.md as
inputs — recomputed third/last, after both settled, `c432a34`→`5af158b`. `--check` CLEAN on all three
post-reconciliation, plus BC-1.18.008.md, verification-architecture.md, and
verification-coverage-matrix.md. This 3-file dependency chain (BC → error-taxonomy → story) is itself
a process observation worth carrying forward: `compute-input-hash --update` on a set of files with
cross-references between them is NOT commutative — the correct order is topological (leaves first,
dependents last), and an out-of-order `--update` produces a hash that a subsequent `--check` will
immediately re-flag as drifted, requiring a second pass. Not escalated to a formal lesson this burst
(a single self-corrected instance, not yet a recurring pattern) but noted for state-manager's own
future input-hash reconciliation bursts touching more than one file in a declared-input relationship.

**Cross-vendor process lesson (`[codified]`):** a single OpenAI Codex cross-vendor pass surfaced 1
data-loss bug (F-C3-P6-001) and 2 spec-fidelity gaps (F-C3-P6-002, F-C3-P6-003) that 5 consecutive
same-vendor (Claude) adversary passes either missed outright or, for F-C3-P6-001, examined the exact
same underlying mechanism and explicitly rationalized as non-blocking (O-C3-P3-001, pass-3) without
constructing the falsifying counterexample; for F-C3-P6-002, read the governing spec language across 3
subsequent passes without flagging the implementation gap. Codified as lesson
`L-BB-D1196-cross-vendor-adversary-pass-surfaces-same-vendor-blind-spots`
(`cycles/v1.0-brownfield-backfill/lessons.md`). Per Canonical Principle Rule 3 (concrete future
dependency + specific anchor): routed to NEW draft follow-up story **S-12.10** (E-12 Engine
Governance, registered this burst in `STORY-INDEX.md`) — a `vsdd-factory:adversarial-review` skill +
orchestrator-prompt amendment requiring at least one cross-vendor (non-Claude-family) adversary pass
as part of every BC-5.39.001 3-CLEAN convergence cascade, promoting cross-vendor review from optional
practice to a required protocol step.

Full code gate GREEN on `feature/S-25.02-backfill` @ `b1134954` (implementer's F-C3-P6-001/002/003
fixes + test-writer's new/retired test set, immediately after `26c79f13`, pushed to origin): full
`cargo test --workspace --all-targets` suite green (784 tests); `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean. BC-5.39.001 cluster-3 LOCAL streak stays
**0/3** (pass-6 NOT CLEAN — 2 HIGH + 1 MEDIUM, ALL fixed same-pass; the substantive-CODE-defect-
surface-EXHAUSTED assessment reached after passes 4/5 is REOPENED — it held only for the same-vendor
review perspective; pass-7 next, fresh context, continuing the human-authorized full 3-CLEAN drive
with cross-vendor passes now an explicit part of the rotation; cycle-level 3/3 CONVERGED streak
UNCHANGED). No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a
cycle-level adversary pass). `pipeline:` stays **PAUSED** (mid-convergence fix burst; consistent with
prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-7, fresh context, against BC-1.18.008 v1.6 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `b1134954` — continuing the human-authorized full 3-CLEAN
drive, with cross-vendor passes now an explicit part of the rotation per this pass's own codified
process lesson.**

Refs: D-1196, D-1195, S-25.02, BC-1.18.008 v1.6, F-C3-P6-001..003, S-12.10, `b1134954`, `26c79f13`,
`d136e83`, `c5ba1e0`, `5af158b`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1196 | D-1196-S2502-CLUSTER3-PASS6-CROSSVENDOR-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-6 = NOT CLEAN — 2 HIGH + 1 MEDIUM finding, FIRST CROSS-VENDOR (OpenAI Codex) pass this cascade.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-6.md`. All 3 findings NOVEL — missed or rationalized away across 5 prior same-vendor (Claude) passes. F-C3-P6-001 (HIGH, data-loss): recovery arm's byte-prefix heuristic false-positived on legitimate repeated-prefix content and silently destroyed an intact record — FIXED via product-owner's **BC-1.18.008 v1.5→v1.6** (NEW Backfill Recovery Manifest in PC3 + NEW Recovery-Confirmation Rule in PC5 — exact whole-file `(length, SHA-256)` vs. byte-prefix — + Invariant 3 rewrite + EC-009/EC-010, `E-SHD-011`) plus implementer's manifest-based recovery rebuild. F-C3-P6-002 (HIGH, spec-fidelity): PC6(c)/Invariant 4's "actual bytes written to disk" was never implemented as a disk read-back — FIXED via a same-burst product-owner ruling plus implementer's new `mechanism_a_write_and_verify_sealed_shard` post-hoc disk read-back. F-C3-P6-003 (MEDIUM, code-only): lesson h3 marker missing documented `\b` word boundary — FIXED, TD-VSDD-060 sibling-swept. architect propagated a THIRD VP-124 facet (VP-INDEX v3.10→v3.11, verification-architecture.md v1.27→v1.28, verification-coverage-matrix.md v1.25→v1.26, `total_vps` UNCHANGED 141). BC-INDEX v5.78→v5.79; STORY-INDEX v4.455→v4.456 (story v3.6→v3.7, AC-014 extended, EC-050/EC-051 added; NEW draft follow-up story S-12.10 registered, E-12). Input-hashes reconciled in dependency order (BC-1.18.008.md `763d2ab`→`d136e83`; error-taxonomy.md `226df61`→`c5ba1e0`; story `c432a34`→`5af158b`); `--check` CLEAN on all touched files. `[codified]` lesson `L-BB-D1196-cross-vendor-adversary-pass-surfaces-same-vendor-blind-spots` — cross-vendor review promoted from optional to a REQUIRED BC-5.39.001 convergence-protocol step, anchored to S-12.10. Feature branch `feature/S-25.02-backfill` @ `b1134954` (pushed); full workspace test suite green (784 tests), fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-6 not clean; substantive-CODE-defect-surface-EXHAUSTED assessment from passes 4/5 REOPENED — held only for same-vendor review; pass-7 next; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-7, fresh context, against BC-1.18.008 v1.6/code `b1134954` — cross-vendor passes now part of the rotation.** Refs: D-1196, D-1195, S-25.02, BC-1.18.008 v1.6, F-C3-P6-001..003, S-12.10, `b1134954`, `26c79f13`. STATE.md v10.21→v10.22. | S-25.02 F4 | 2026-09-10 |

## D-1197

**D-1197-S2502-CLUSTER3-PASS7-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1196 (this file, immediately above). D-1197 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-7 = NOT CLEAN — 1 HIGH + 1 MEDIUM finding** 2026-09-10 (LOCAL Claude adversary + implementer +
test-writer code-side content + product-owner spec content; state-manager bookkeeping + single-commit
TD-VSDD-053) — 1 HIGH (F-C3-P7-001, DANGEROUS-window heal wrote an unverified shard-index-summed
slice), 1 MEDIUM (F-C3-P7-002, decision-log.md marker-table regex missed real sub-clause-suffixed
rows), BOTH fixed this burst. **This is a LOCAL Claude adversary pass** — human directed drive-to-
3-CLEAN continues using LOCAL adversary only, no further cross-vendor rotation unless the human
specifies (per D-1196's own codified lesson, cross-vendor is now a REQUIRED step somewhere in the
cascade, not necessarily every pass). Full Part A persisted as a standalone artifact, matching
pass-1..6's own convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-7.md` (`diff_base=b1134954`,
`diff_head=b1134954`).

**F-C3-P7-001 (HIGH, data-loss):** at the confirmed DANGEROUS window (top-level `(length, hash)`
recovery-confirmation check already matched the Manifest's `original_bytes`/`original_sha256`), the
shipped heal derived the slice offset by SUMMING the shard-index's per-shard `bytes_at_seal` fields
— a source independent of, and separately corruptible from, the Backfill Recovery Manifest — and
wrote the resulting slice with **no verification of any kind** against the Manifest's
`final_bytes`/`final_sha256`. **Root cause: a PC5/PC3 spec incoherence.** BC-1.18.008 v1.6's own
Postcondition 5 text said the heal "writes the manifest's own recorded `final_bytes` content," but
Postcondition 3's Manifest schema stores only a length (`final_bytes`) and a hash (`final_sha256`) —
no content of any kind. The v1.6 text was literally unimplementable from the manifest alone; the
implementation that shipped, correctly noting the manifest holds no content, instead picked an
alternative (shard-index-summed offset, no verification) that reintroduced exactly the
unverified-write risk the Manifest was created (v1.6, F-C3-P6-001) to eliminate. FIXED: product-owner
amended **BC-1.18.008 v1.6→v1.7**: added the **Manifest-Authoritative Slice-and-Verify Rule** to
Postcondition 5 — the heal's offset is now `original_bytes - final_bytes`, both operands read from
the Manifest itself (the shard-index `bytes_at_seal` sum is no longer consulted for this purpose at
all), and the resulting slice MUST satisfy `sliced.len() == final_bytes AND sha256(sliced) ==
final_sha256` BEFORE it is written; on any mismatch the heal fails loud with **NEW `E-SHD-012`**
(added to `error-taxonomy.md` in this SAME burst) and writes nothing. Invariant 3 rewritten: the
Manifest "authorizes/verifies the bytes written, it is not required to store them." Also closes the
adversary's PC6(c)/Invariant-4 observation: the DANGEROUS heal's own destructive write previously got
no post-hoc disk read-back (unlike sealed-shard writes, per PC6(c)'s F-C3-P6-002 ruling) —
Postcondition 6(c) gained an explicit Extension paragraph, and Invariant 4's scope note now covers
it, requiring the SAME fresh-read-back-and-compare-to-`(final_bytes, final_sha256)` discipline after
the heal's `write_atomic` call. Added EC-011 (Manifest-verification mismatch at a confirmed DANGEROUS
window — fails loud, `E-SHD-012`, no write) and a matching Canonical Test Vector; corrected the
existing DANGEROUS-heal CTV row's expected-behavior text (it previously asserted the now-incoherent
"NOT re-derived by slicing" claim). implementer, `feature/S-25.02-backfill`, rewrote the
DANGEROUS-window heal around the Manifest-derived offset and extracted a shared
`write_and_read_back` helper (reused by both the heal write and the sealed-shard write path, closing
a would-be duplication between the F-C3-P6-002 and this fix's read-back logic). test-writer added
the EC-011 corrupted-Manifest fixture, a genuine-slice-passes-and-is-written-and-read-back-verified
positive fixture, and regression-guarded the shared read-back helper.

**F-C3-P7-002 (MEDIUM, correctness):** the `decision-log.md` primary partition-key regex,
`^\| D-[0-9]+ \|`, requires digits immediately followed by ` \|` and cannot match a real
sub-clause-suffixed row such as `\| D-440(a) \|`, the combined-suffix form `\| D-446(a/b/c/d/e) \|`,
or the hyphenated form `\| D-355-AMEND \|`. Direct re-inspection of the live engine-cycle
`decision-log.md` (2026-09-10) confirmed 144 total `\| D-...` rows = 109 bare `\| D-NNN \|` + 34
parenthetical-suffix rows + 1 hyphenated-suffix row (`D-355-AMEND`) — none of the 35 non-bare rows
matched the old regex, so the marker table's own "144 table rows ... CONFIRMED CORRECT" evidence was
unachievable by the regex it cited. The sibling brownfield `decision-log.md` was also re-inspected:
265 rows as of this amendment (grown from the prior 254-row measurement, consistent with EC-005's
documented staleness precedent), all bare form, 0 sub-clause exceptions in that cycle. FIXED:
product-owner corrected the primary-key regex to `^\| D-[0-9]+(\([a-z0-9/]+\)\|-[A-Za-z]+)? \|` —
confirmed by direct grep to match every row in both cycles (144/144 engine, 265/265 brownfield, zero
unmatched). Updated the Normalization rule's `decision-log.md` bullet to cite the corrected regex and
enumerate the two suffix forms as ONE primary-key class (not three). Reconciled the marker table's
evidence cell with the exact 109+34+1=144 breakdown and the re-measured 265-row brownfield count.
Added EC-012 (sub-clause rows detected as boundaries) and a matching Canonical Test Vector (6-row
fixture: bare, single-letter-parenthetical ×2, combined-parenthetical, hyphenated, bare — corrected
regex detects all 6 as distinct boundaries; old regex would detect only 2). **Answered the
adversary's Appendix cap-bounding question (one-line clarification, no spec-behavior change):** added
a clarification paragraph confirming the `## Appendix: Sub-clause Expansion` section is packed as ONE
trailing atomic unit after the shard sealing the file's last `\| D-NNN(...) \|` row, flagged
`oversized_record: true` under the EXISTING EC-002 exception when it does not fit — direct
measurement confirms this is the REAL case for the engine cycle (Appendix section = 74,989 bytes,
already over the illustrative 49,152-byte cap on its own). No new VP citation for this finding — the
EXISTING VP-123 Record-integrity facet's fixture coverage is extended, not a new facet or ID.
implementer updated `is_decision_log_row_marker` to the corrected regex (TD-VSDD-060 sibling-swept
against sibling marker predicates). test-writer added the EC-012 6-row fixture and a negative case
confirming the old regex under-counts.

**Propagation:** architect propagated a FOURTH VP-124 facet (heal slice-verification invariant, per
POLICY 9 `vp_index_is_vp_catalog_source_of_truth`, per BC-1.18.008 v1.7's own routing note): VP-INDEX
v3.11→v3.12 (`total_vps` UNCHANGED 141), verification-architecture.md v1.28→v1.29,
verification-coverage-matrix.md v1.26→v1.27 — architect already ran `compute-input-hash --update` on
both arch docs same-burst (`a5078ab`). story-writer's S-25.02 body v3.7→v3.8: AC-014 updated in place
with the Manifest-Authoritative Slice-and-Verify Rule and the heal-write disk-read-back extension;
§Edge Cases gained EC-052 (mirrors BC EC-011) and EC-053 (mirrors BC EC-012); §Behavioral Contracts
BC-1.18.008 cell v1.6→v1.7; §Token Budget BC-1.18.008 line 6,600→7,300 tokens. BC-INDEX v5.79→v5.80
(BC-1.18.008 version-cell v1.6→v1.7 per POLICY 8); STORY-INDEX v4.456→v4.457 (S-25.02 BC-list cell +
row narrative).

**Input-hash reconciliation (dependency-ordered, per D-1196's own process observation):** BC-1.18.008.md
declares VP-INDEX.md as an input (architect's VP-124 fourth-facet extension changed it) — recomputed
first, `d136e83`→`dc4b072`. `prd-supplements/error-taxonomy.md` declares BC-1.18.008.md as an input —
recomputed second, AFTER BC-1.18.008.md settled, `c5ba1e0`→`cd1a1e6`. The S-25.02 story declares both
BC-1.18.008.md and error-taxonomy.md as inputs — recomputed third/last, after both settled,
`5af158b`→`8d5f873`. `--check` CLEAN on all three post-reconciliation, plus BC-1.18.008.md,
verification-architecture.md, and verification-coverage-matrix.md (both arch docs already current at
`a5078ab`, architect's own same-burst update).

Full code gate GREEN on `feature/S-25.02-backfill` @ `915b898c` (implementer's F-C3-P7-001/002 fixes
+ test-writer's new/retired test set, immediately after `b1134954`, pushed to origin): full
`cargo test --workspace --all-targets` suite green; `cargo fmt --check --all` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean. BC-5.39.001 cluster-3 LOCAL streak
stays **0/3** (pass-7 NOT CLEAN — 1 HIGH + 1 MEDIUM, BOTH fixed same-pass; pass-8 next, fresh
context, continuing the human-authorized full 3-CLEAN drive with LOCAL adversary only unless the
human specifies otherwise; cycle-level 3/3 CONVERGED streak UNCHANGED). No trajectory-tail drift —
unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level adversary pass). `pipeline:`
stays **PAUSED** (mid-convergence fix burst; consistent with prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-8, fresh context, against BC-1.18.008 v1.7 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `915b898c` — continuing the human-authorized full 3-CLEAN
drive with LOCAL adversary only, no further cross-vendor rotation unless the human specifies.**

Refs: D-1197, D-1196, S-25.02, BC-1.18.008 v1.7, F-C3-P7-001, F-C3-P7-002, `915b898c`, `b1134954`,
`dc4b072`, `cd1a1e6`, `8d5f873`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1197 | D-1197-S2502-CLUSTER3-PASS7-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-7 = NOT CLEAN — 1 HIGH + 1 MEDIUM finding.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-7.md`. F-C3-P7-001 (HIGH, data-loss): DANGEROUS-window heal wrote an unverified shard-index-summed slice — root cause a PC5/PC3 spec incoherence (v1.6 PC5 said the heal writes manifest-stored content the length+hash-only Manifest schema cannot supply) — FIXED via product-owner's **BC-1.18.008 v1.6→v1.7** (NEW Manifest-Authoritative Slice-and-Verify Rule in PC5 — offset `original_bytes - final_bytes`, both Manifest-derived, verified before every write, `E-SHD-012` fail-loud on mismatch — + Invariant 3 rewrite + EC-011; same-burst PC6(c)/Invariant 4 heal-write disk-read-back extension) plus implementer's heal rewrite + shared `write_and_read_back` helper. F-C3-P7-002 (MEDIUM): decision-log.md marker-table regex missed real `\| D-NNN(x) \|`/`\| D-NNN-AMEND \|` sub-clause rows (144 engine rows: 109 bare + 34 parenthetical + 1 AMEND) — FIXED via product-owner's corrected regex + EC-012, implementer's `is_decision_log_row_marker` update. architect propagated a FOURTH VP-124 facet (VP-INDEX v3.11→v3.12, verification-architecture.md v1.28→v1.29, verification-coverage-matrix.md v1.26→v1.27, `total_vps` UNCHANGED 141). BC-INDEX v5.79→v5.80; STORY-INDEX v4.456→v4.457 (story v3.7→v3.8, AC-014 updated, EC-052/EC-053 added). Input-hashes reconciled in topological order (BC-1.18.008.md `d136e83`→`dc4b072`; error-taxonomy.md `c5ba1e0`→`cd1a1e6`; story `5af158b`→`8d5f873`); `--check` CLEAN on all touched files. Feature branch `feature/S-25.02-backfill` @ `915b898c` (pushed); full workspace test suite green, fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-7 not clean; pass-8 next; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-8, fresh context, against BC-1.18.008 v1.7/code `915b898c`.** Refs: D-1197, D-1196, S-25.02, BC-1.18.008 v1.7, F-C3-P7-001, F-C3-P7-002, `915b898c`, `b1134954`. STATE.md v10.22→v10.23. | S-25.02 F4 | 2026-09-10 |

## D-1198

**D-1198-S2502-CLUSTER3-PASS8-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1197 (this file, immediately above). D-1198 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-8 = NOT CLEAN — 2 MEDIUM findings** 2026-09-10 (LOCAL Claude adversary + implementer + product-
owner spec content; state-manager bookkeeping + single-commit TD-VSDD-053) — 2 MEDIUM (F-C3-P8-001,
`E-SHD-012` taxonomy Message Format documentation drift vs the shipped `Display`, the 4th recurrence
of this defect class in the cluster; F-C3-P8-002, the happy-path canonical-truncate write lacked the
post-hoc disk read-back the DANGEROUS-window heal write already requires — an unsanctioned asymmetry
with a latent silent-data-loss consequence), BOTH fixed this burst. **This is a LOCAL Claude
adversary pass** — human directed drive-to-3-CLEAN continues using LOCAL adversary only, no further
cross-vendor rotation unless the human specifies. Full Part A persisted as a standalone artifact,
matching pass-1..7's own convention:
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-8.md` (`diff_base=915b898c`,
`diff_head=915b898c`).

**F-C3-P8-001 (MEDIUM, doc-only):** `error-taxonomy.md`'s `E-SHD-012` row documented a single fixed
structured Message Format; the shipped `MechanismABackfillError::SliceVerificationFailed` `Display`
instead emits a `{detail}`-placeholder form covering FIVE distinct detail shapes (manifest-
inconsistency `checked_sub` underflow, `usize` offset-arithmetic overflow, offset-exceeds-canonical-
length, the pre-write slice mismatch the row previously documented as if it were the only case, and a
post-hoc read-back mismatch after the heal's write already completed). This is the **4th recurrence**
of the taxonomy-drift-vs-shipped-`Display` defect class in this cluster, following prior occurrences
on `E-SHD-008`/`E-SHD-009` (cluster-2 pass-9, F-C2-P9-002) and this cluster's own earlier taxonomy
corrections. Companion observation folded into the same disposition: `E-SHD-011`'s row also
documented only its top-level three-way `(length, hash)` mismatch form; `MechanismABackfillError::
MissingBackfillManifest` emits a SECOND, distinct message under the SAME code (a durable shard-index
with no `[backfill_manifest]` table — e.g. a pre-v1.6 shard-index) that the row never documented.
BC-1.18.008 v1.7 Invariant 3(c) already sanctions reusing `E-SHD-011` for this precondition-failure
case, so this is a taxonomy completeness gap, not a code defect, and no new code was allocated.
**FIXED, DOC-ONLY:** product-owner corrected `E-SHD-012`'s Message Format cell to the
`{detail}`-placeholder form (all five shapes enumerated, noting cases (1)-(4) leave the canonical
file untouched while case (5) does not) and completed `E-SHD-011`'s row to document both real
emissions as forms (a) and (b). `prd-supplements/error-taxonomy.md` v1.11→v1.12.

**F-C3-P8-002 (MEDIUM, spec-internal asymmetry + latent silent-data-loss):** Postcondition 6(c)/
Invariant 4 already mandate a post-hoc disk read-back for every sealed-shard write (F-C3-P6-002), and
pass-7's F-C3-P7-001 fix already extends that requirement to the DANGEROUS-window heal write, on the
rationale that the heal write "permanently replaces the canonical file's content at the moment the
pre-heal content is irretrievably gone." That rationale applies IDENTICALLY to the happy-path
canonical-truncate write in Postcondition 5 step (ii) — the ORDINARY, first-time/uninterrupted
`write_atomic` that discards the original monolithic file's content in the same act the final
partition's bytes become the canonical file's only copy — but the spec text enumerated only the
DANGEROUS-window heal for this discipline. The more-frequently-executed path (every uninterrupted
migration run, not merely its crash-recovery counterpart) was therefore LESS verified than the path
that exists specifically to recover from ITS OWN failure — a silent truncation or corruption of the
happy-path write would go undetected at migration time, discovered (if at all) only after the
original monolithic content is already gone, recoverable only from git history. **FIXED:**
product-owner amended **BC-1.18.008 v1.7→v1.8** — added an **Extension to the happy-path
canonical-truncate write** paragraph to Postcondition 6(c), requiring the SAME fresh post-hoc disk
read-back (verified against the SAME Backfill Recovery Manifest `(final_bytes, final_sha256)` pair
already published in step (i), so no new value needs computing) BEFORE the migration reports success;
a mismatch fails loud with **NEW `E-SHD-013`**. Added a **Summary** paragraph naming all three
destructive-write sites (sealed shard / heal / happy-path canonical) now under the same discipline,
and a **NEW Invariant 5** generalizing the principle once: no destructive write of this BC's one-time
migration is ever trusted on its own return value alone. Added EC-013 (happy-path write silently
corrupted on disk, detected via read-back, fails loud `E-SHD-013`, with a follow-on note that a
subsequent recovery attempt correctly reports the AMBIGUOUS `E-SHD-011` disposition rather than
silently accepting the corruption) and a matching Canonical Test Vector. `error-taxonomy.md`
v1.11→v1.12 gained `E-SHD-013` in the SAME burst (folded into F-C3-P8-001's taxonomy pass). Added
VP-124's FIFTH facet (happy-path canonical-write verification invariant) — VP citation change routed
to architect per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9). implementer routed the
happy-path write through the SAME shared `write_and_read_back` helper F-C3-P7-001 extracted, verifying
`(length, sha256)` against the shard-index's already-published `[backfill_manifest]` `(final_bytes,
final_sha256)` and surfacing the NEW `E-SHD-013` variant (`CanonicalWriteVerificationFailed`,
mirroring `SliceVerificationFailed`'s shape) on mismatch — all THREE destructive write sites this BC
specifies are now independently read-back-guarded via one shared helper, **completing the class of
findings progressively closed across passes 6, 7, and 8.**

**Propagation:** architect propagated a FIFTH VP-124 facet (happy-path canonical-write verification
invariant, per POLICY 9 `vp_index_is_vp_catalog_source_of_truth`, per BC-1.18.008 v1.8's own routing
note): VP-INDEX v3.12→v3.13 (`total_vps` UNCHANGED 141), verification-architecture.md v1.29→v1.30,
verification-coverage-matrix.md v1.27→v1.28 — architect already ran `compute-input-hash --update` on
both arch docs same-burst (`f52e536`). story-writer's S-25.02 body v3.8→v3.9: AC-014 updated in place
(trace citation widened to add `invariant 5`) with a new Happy-path canonical-write disk read-back
paragraph mirroring the existing Heal-write disk read-back paragraph, and a closing Invariant 5
paragraph; §Edge Cases gained EC-054 (mirrors BC EC-013); §Behavioral Contracts BC-1.18.008 cell
v1.7→v1.8; §Token Budget BC-1.18.008 line 7,300→7,700 tokens. BC-INDEX v5.80→v5.81 (BC-1.18.008
version-cell v1.7→v1.8 per POLICY 8); STORY-INDEX v4.457→v4.458 (S-25.02 BC-list cell + row
narrative).

**Input-hash reconciliation (dependency-ordered, per D-1196's own process observation):**
BC-1.18.008.md declares VP-INDEX.md as an input (architect's VP-124 fifth-facet extension changed
it) — recomputed first, `dc4b072`→`9182c9a`. `prd-supplements/error-taxonomy.md` declares
BC-1.18.008.md as an input — recomputed second, AFTER BC-1.18.008.md settled, `cd1a1e6`→`f5ba001`.
The S-25.02 story declares both BC-1.18.008.md and error-taxonomy.md as inputs — recomputed
third/last, after both settled, `8d5f873`→`5eb344f`. `--check` CLEAN on all three
post-reconciliation, plus BC-1.18.008.md, verification-architecture.md, and
verification-coverage-matrix.md (both arch docs already current at `f52e536`, architect's own
same-burst update).

**BC-5.45.001 write-path regression closed (this burst, discovered incidentally while preparing this
commit):** running the mandatory `last-amended-migrate migrate --check` pre-push guard across the 5
BC-5.45.001-governed files (STORY-INDEX.md, BC-INDEX.md, ARCH-INDEX.md, VP-INDEX.md, STATE.md) found
3 files out of shape, none introduced this burst: **VP-INDEX.md** had grown a `PriorChainSplit`
defect — architect's own v3.13 edit (this burst, POLICY 9 propagation) wrapped its new `last_amended`
narrative around an ALREADY-PRESENT inline `[Prior: v3.12 ...[Prior: v3.11 ...[Prior: v3.10 ...]]]`
chain inherited from at least the pass-6/pass-7 bursts, which had not been caught by either of those
prior state-manager bursts. FIXED via the sanctioned full-recovery split:
`last-amended-migrate migrate --path .factory/specs/verification-properties/VP-INDEX.md` (PC7,
`entries_relocated=2` — the v3.12 and v3.11 chain entries relocated into `changelog:`, current v3.13
entry left as the sole `last_amended` value). **BC-INDEX.md** carried a D-1144 unescaped-`\|`
escape defect in its (then-current) v5.80 `last_amended` value (the F-C3-P7-002 regex citation's
literal `\|` characters were not YAML-escaped as `\\|`) — auto-fixed via the same tool
(`escape_fixed=true`). **BC-INDEX.md** additionally carried a pre-existing, unrelated, much older
(2026-05-12-era, pass-50 fix burst) malformed-YAML defect in its `changelog:` array: a missing
`- date: 2026-05-12` list-item marker immediately ahead of the `v1.93` entry, which had silently
merged that entry's `change:` key into the PRECEDING (`v1.94`) list item as a duplicate mapping key —
this defect blocked `last-amended-migrate` from validating/writing the file at all
(`duplicate entry with key "change" at line 409 column 5`). Per the CLAUDE.md production-grade
default (a 1-line, unambiguous, mechanical YAML-structure fix, discovered while touching this exact
file this burst), FIXED IN-SCOPE via a targeted Edit restoring the missing `- date:` marker, BEFORE
re-running the tool. **STATE.md** carried its own D-1144 unescaped-`\|` escape defect in its
then-current `last_amended` value — auto-fixed via the same tool (`escape_fixed=true`;
`changelog=SkippedStateFile`, no chain — STATE.md's own `last_amended` was never chained). Post-fix:
`last-amended-migrate migrate --check` reports CLEAN (exit 0) across all 5 governed files.

Full code gate GREEN on `feature/S-25.02-backfill` @ `8e2a37f4` (implementer's F-C3-P8-002 fix,
immediately after `915b898c`, pushed to origin): full `cargo test --workspace --all-targets` suite
green; `cargo fmt --check --all` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-8 NOT CLEAN — 2 MEDIUM, BOTH
fixed/disposed same-pass; pass-9 next, fresh context, continuing the human-authorized full 3-CLEAN
drive with LOCAL adversary only unless the human specifies otherwise; cycle-level 3/3 CONVERGED
streak UNCHANGED). No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade,
not a cycle-level adversary pass). `pipeline:` stays **PAUSED** (mid-convergence fix burst;
consistent with prior cluster fix-burst state handling).

### Next Steps

**NEXT = cluster-3 LOCAL adversary pass-9, fresh context, against BC-1.18.008 v1.8 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `8e2a37f4` — continuing the human-authorized full 3-CLEAN
drive with LOCAL adversary only, no further cross-vendor rotation unless the human specifies.**

Refs: D-1198, D-1197, S-25.02, BC-1.18.008 v1.8, F-C3-P8-001, F-C3-P8-002, `8e2a37f4`, `915b898c`,
`9182c9a`, `f5ba001`, `5eb344f`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1198 | D-1198-S2502-CLUSTER3-PASS8-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-8 = NOT CLEAN — 2 MEDIUM findings.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-8.md`. F-C3-P8-001 (MEDIUM, doc-only): `E-SHD-012` taxonomy Message Format cell documented a fixed structured wording; shipped `Display` emits a `{detail}`-placeholder form (5 detail shapes) — 4th recurrence of the taxonomy-drift-vs-shipped-Display class in the cluster — FIXED via product-owner's corrected cell + completed `E-SHD-011` row (2nd real emission, `MissingBackfillManifest`, sanctioned reuse per BC-1.18.008 v1.7 Invariant 3(c)). F-C3-P8-002 (MEDIUM): the happy-path canonical-truncate write lacked the post-hoc disk read-back the DANGEROUS-window heal write gained at pass-7 — an unsanctioned asymmetry with a latent silent-data-loss consequence — FIXED via product-owner's **BC-1.18.008 v1.7→v1.8** (NEW Postcondition 6(c) happy-path read-back extension, NEW Invariant 5 generalizing the discipline across all three destructive write sites, NEW EC-013, NEW `E-SHD-013`) plus implementer's `write_and_read_back`-routed happy-path write — all THREE destructive write sites (sealed-shard/heal/happy-path canonical) now independently read-back-guarded via one shared helper, completing the class passes 6-8 progressively closed. `error-taxonomy.md` v1.11→v1.12 (E-SHD-013 added; E-SHD-012/E-SHD-011 cells corrected/completed). architect propagated a FIFTH VP-124 facet (VP-INDEX v3.12→v3.13, verification-architecture.md v1.29→v1.30, verification-coverage-matrix.md v1.27→v1.28, `total_vps` UNCHANGED 141). BC-INDEX v5.80→v5.81; STORY-INDEX v4.457→v4.458 (story v3.8→v3.9, AC-014 updated [trace widened to invariant 5], EC-054 added). Input-hashes reconciled in topological order (BC-1.18.008.md `dc4b072`→`9182c9a`; error-taxonomy.md `cd1a1e6`→`f5ba001`; story `8d5f873`→`5eb344f`); `--check` CLEAN on all touched files, plus verification-architecture.md/verification-coverage-matrix.md (architect's own same-burst update, `f52e536`). This burst ALSO closed a pre-existing multi-burst BC-5.45.001 write-path regression discovered on the 5 governed files: VP-INDEX.md's `last_amended` had grown an inline `[Prior: ...]` chain (2 entries relocated to `changelog:` via the sanctioned `last-amended-migrate` full-recovery split, PC7); BC-INDEX.md and STATE.md each carried an unescaped-`\|` D-1144 escape defect (both auto-fixed by the same tool); BC-INDEX.md additionally carried a pre-existing (2026-05-12-era) malformed-YAML duplicate-key defect (a missing `- date:` list-item marker ahead of its `v1.93` entry) — fixed in-scope (1-line mechanical correction) before the tool could validate/write the file. `last-amended-migrate migrate --check` now CLEAN (exit 0) across all 5 governed files. Feature branch `feature/S-25.02-backfill` @ `8e2a37f4` (pushed); full workspace test suite green, fmt+clippy clean. BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-8 not clean; pass-9 next; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 LOCAL adversary pass-9, fresh context, against BC-1.18.008 v1.8/code `8e2a37f4`.** Refs: D-1198, D-1197, S-25.02, BC-1.18.008 v1.8, F-C3-P8-001, F-C3-P8-002, `8e2a37f4`, `915b898c`. STATE.md v10.23→v10.24. | S-25.02 F4 | 2026-09-10 |

## D-1199

**D-1199-S2502-CLUSTER3-PASS9-FIX-BURST**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1198 (this file, immediately above). D-1199 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-9 = NOT CLEAN — 2 MEDIUM findings, both taxonomy doc-drift; CODE independently re-verified
spec-conformant** 2026-09-10 (LOCAL Claude adversary + product-owner + state-manager; single-commit
TD-VSDD-053). Fresh pass-9 (against pass-8's fix-burst, `feature/S-25.02-backfill` @ `8e2a37f4`)
re-verified every `E-SHD-NNN` `Display` emitted by `ShardRollError`, `MechanismABackfillError`, and
`ShardRetentionError` against `error-taxonomy.md` v1.12's Message Format cells and found the shipped
CODE spec-conformant in every case — zero code/behavior/BC defects this pass — but 2 further
instances of the cluster's own recurring "taxonomy Message Format cell ≠ shipped `Display`" drift
class. Full Part A persisted as a standalone artifact —
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-9.md`, matching pass-1..8's own
convention.

**F-C3-P9-001 (MEDIUM):** `E-SHD-011` form (b) (`MissingBackfillManifest`) diverged from the shipped
`Display` text in three places — wrong lead clause ("cannot proceed for" vs shipped
"recovery-confirmation ambiguous for"), a missing "([backfill_manifest]) to compare the canonical
file's current bytes against" clause, and an over-elaborated closing clause where the shipped text
reads the bare "refusing to guess;". FIXED via product-owner's corrected cell, shipped text
reproduced verbatim.

**F-C3-P9-002 (MEDIUM):** `E-SHD-003` documented only `MechanismABackfillError::
ContentPreservationFailed`'s emission, in a form missing the `E-SHD-003:` prefix and the shipped
`artifact_stem "<artifact>":` quoting convention; `Io`'s real emission (a genuine I/O failure during
Postcondition 5's stage-then-verify-then-atomically-replace sequence) was entirely undocumented.
FIXED via product-owner's corrected row documenting BOTH forms (a) `ContentPreservationFailed` /
(b) `Io` verbatim.

**Companion sweep (beyond the 2 named findings):** per the human's direction to stop fixing this
drift class one code at a time, product-owner's fix burst performed a complete mechanical re-diff of
every `#[error("E-SHD-...")]` `Display` in `shard_manager.rs` — covering `ShardRollError`,
`MechanismABackfillError`, AND, for the first time this cluster, the previously-unswept
`ShardRetentionError` — against this table's Message Format column, 13 codes / 17 real emissions
total. This surfaced a THIRD drift instance: `E-SHD-002` (`ShardRetentionError`, two real emissions)
documented only `IndexUnavailable`, missing the `E-SHD-002:` prefix, using bare `for <artifact>:`
quoting, and mislabeling the `#[source] io::Error` field `<parse-error>`; `ArchivalMoveFailed`'s real
emission was entirely undocumented. Corrected to document both forms verbatim in the same burst.
Every other `E-SHD-NNN` row (`001`, `004`/`005` not-yet-implemented anywhere in the workspace,
`006`-`010`, `012`, `013`) re-verified MATCH, no change — this is the first attestation covering all
THREE error enums in one pass (the prior v1.8 changelog's "all nine rows source-verified"
attestation covered only `ShardRollError`). No BC change required — shipped Displays were already
correct in every case; BC-1.18.008 v1.8 already governs the semantics for the
`MechanismABackfillError` codes and needs no amendment.

**Artifact changes:** `prd-supplements/error-taxonomy.md` v1.12→v1.13 (single changelog entry
covering F-C3-P9-001, F-C3-P9-002, and the companion `E-SHD-002` fix). `--update`/`--check` run via
`compute-input-hash`: error-taxonomy.md's own input-hash stays `f5ba001` (UNCHANGED — its declared
`inputs:` is `BC-1.18.008.md`, which did not change this burst; input-hash tracks declared-input
drift, not the file's own content); `--check` CLEAN. No BC/AC/EC/VP/story content changed this burst
— BC-INDEX, VP-INDEX, ARCH-INDEX all UNCHANGED (BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24).
STORY-INDEX v4.458→v4.459 (row addition only — NEW draft follow-up story **S-12.11** registered, E-12
Engine Governance, anchoring the `[codified][process-gap]` lesson below; no existing story content
changed). Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4` (doc-only fix, no
code change; full `cargo test --workspace --all-targets` / `cargo fmt --check --all` / `cargo clippy
--workspace --all-targets -- -D warnings` re-confirmed green/clean at the existing HEAD, no new
commit required).

**`[codified][process-gap]` lesson:** the taxonomy Message-Format-cell-≠-shipped-Display drift class
has now recurred 6+ times across this cluster and cluster-2 (`E-SHD-001`/`006`/`008`/`009` at
cluster-2 passes 8-10; `E-SHD-011`/`E-SHD-012` at cluster-3 pass-8; `E-SHD-002`/`E-SHD-003`/`E-SHD-011`
form (b) at this pass) — no gate mechanically re-diffs the FULL emitted-`Display` set against this
table on every change to either side; every prior fix reconciled only the one code the triggering
finding named, letting a different code drift by the next pass. Recorded as lesson
`L-BB-D1199-taxonomy-display-drift-recurring-6x-process-gap` in
`cycles/v1.0-brownfield-backfill/lessons.md`, anchored to NEW draft follow-up story **S-12.11**
(E-12 Engine Governance): a lint hook comparing `grep -oE '"E-SHD-[0-9]+:[^"]*"'` against every
`E-SHD`-emitting Rust source file's Displays to this table's Message Format column, CI/pre-commit
gated.

Because findings were present, pass-9 is NOT CLEAN — BC-5.39.001 cluster-3 LOCAL streak stays
**0/3** (cycle-level 3/3 CONVERGED streak UNCHANGED, separate track). `pipeline:` stays **PAUSED**.
No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level
adversary pass).

### Next Steps

**NEXT = pass-10 — the VSDD 10-pass guardrail.** 9 passes have now run against cluster-3 without
reaching 3 consecutive CLEAN (streak 0/3 throughout). Per the project's convergence protocol, the
orchestrator assesses cluster-3's 3-CLEAN convergence status WITH THE HUMAN at this guardrail rather
than automatically dispatching a further fresh adversary pass.

Refs: D-1199, D-1198, S-25.02, BC-1.18.008 v1.8, F-C3-P9-001, F-C3-P9-002, S-12.11, `8e2a37f4`,
error-taxonomy.md v1.13.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1199 | D-1199-S2502-CLUSTER3-PASS9-FIX-BURST | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-9 = NOT CLEAN — 2 MEDIUM findings, both taxonomy doc-drift; CODE independently re-verified spec-conformant.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-9.md`. F-C3-P9-001 (MEDIUM): `E-SHD-011` form (b) (`MissingBackfillManifest`) diverged from the shipped `Display` in 3 places — FIXED via product-owner's corrected cell, shipped text reproduced verbatim. F-C3-P9-002 (MEDIUM): `E-SHD-003` documented only `ContentPreservationFailed`'s emission, missing the code prefix/quoting convention; `Io`'s real emission was undocumented — FIXED via product-owner's corrected row documenting both forms verbatim. Companion sweep (beyond the 2 named findings, human-directed full mechanical re-diff of all 13 `E-SHD-NNN` codes / 17 emissions across `ShardRollError`+`MechanismABackfillError`+`ShardRetentionError`) surfaced and fixed a THIRD drift instance, `E-SHD-002` (`ShardRetentionError`, both forms). No BC change required — shipped code Displays were already correct in every case. `error-taxonomy.md` v1.12→v1.13. STORY-INDEX v4.458→v4.459 (row addition only — NEW draft follow-up story S-12.11 registered, E-12 Engine Governance). `[codified][process-gap]` lesson `L-BB-D1199-taxonomy-display-drift-recurring-6x-process-gap` (6+ recurrences of the class) anchored to S-12.11 — a lint hook diffing every `E-SHD`-emitting Rust source's Displays against this table's Message Format column, CI/pre-commit gated. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 all UNCHANGED. Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4` (doc-only fix, no code change); full workspace test suite green, fmt+clippy clean (re-confirmed at existing HEAD). BC-5.39.001 cluster-3 LOCAL streak stays **0/3** (pass-9 not clean; cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = pass-10, the VSDD 10-pass guardrail — orchestrator assesses cluster-3's 3-CLEAN convergence status WITH THE HUMAN, no further automatic adversary dispatch.** Refs: D-1199, D-1198, S-25.02, BC-1.18.008 v1.8, F-C3-P9-001, F-C3-P9-002, S-12.11, `8e2a37f4`. STATE.md v10.24→v10.25. | S-25.02 F4 | 2026-09-10 |

## D-1200

**D-1200-S2502-CLUSTER3-PASS10-CLEAN-BOOKKEEPING**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1199 (this file, immediately above). D-1200 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-10 = CLEAN — FIRST CLEAN PASS, zero blocking findings** 2026-09-10 (LOCAL Claude adversary +
state-manager; single-commit TD-VSDD-053; LIGHT bookkeeping burst — NO code/spec/story change).
Fresh pass-10 (against pass-9's fix-burst, `feature/S-25.02-backfill` @ `8e2a37f4`) independently
re-verified the full v1.8 contract end to end — recovery/heal/manifest correctness, all 3
destructive-write read-backs (sealed-shard/DANGEROUS-window-heal/happy-path-canonical), the
`decision-log.md` marker-table regex, boundary detection/oracle set-equality, per-shard-cap
accounting, a full independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17
real emissions across all three error enums (confirming pass-9's fix burst introduced no new drift),
spec-internal consistency between BC-1.18.007 and BC-1.18.008, and POLICY-11 test integrity — and
found the shipped code spec-conformant across every dimension checked. Full Part A persisted as a
standalone artifact — `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-10.md`,
matching pass-1..9's own convention.

**Findings:** zero blocking findings. Two LOW non-blocking observations recorded:

**O-C3-P10-001 (LOW, `[process-gap]`):** the 3 disk-read-back fault-injection tests
(`FC3P6002`/`FC3P7001`/`FC3P8002`, covering the sealed-shard, DANGEROUS-window heal, and happy-path
canonical write-and-read-back sites respectively — the class of tests progressively built across
passes 6-8) each spawn a `spawn_temp_file_corruptor` background thread racing the production write
path to corrupt the just-written temp file before the read-back verification runs, bounded to 20
retries and hard-failing the test if the race never lands within that bound. Correct and
non-tautological — genuinely exercises the read-back-catches-corruption path — but the race-based
construction carries a non-zero risk of a rare spurious FALSE-FAIL on a heavily-loaded CI runner.
Suggested (not required) improvement: a deterministic `#[cfg(test)]` fault-injection seam, mirroring
the existing `FORCE_STAGE_FAILURE` thread-local pattern already used elsewhere in `shard_manager.rs`,
that deterministically corrupts the target file at the read-back checkpoint instead of racing a
background thread — removes the flakiness risk without weakening coverage.

**O-C3-P10-002 (LOW):** a PC4 archival-move failure (`ShardRetentionError::ArchivalMoveFailed`,
`E-SHD-002`) is wrapped, at the one call site where mechanism-A's backfill retry surfaces it, as
`MechanismABackfillError::Io` (`E-SHD-003`) — so a diagnostic message carrying the literal
`E-SHD-002:` prefix nests inside the `{source}` slot of an `E-SHD-003:`-prefixed outer message.
Defensible, not a taxonomy-parity violation: `E-SHD-003`'s own Message Format cell documents a
generic `<io-error>` placeholder in its `{source}` position specifically to permit wrapping arbitrary
I/O-shaped failures, and the wrapping correctly signals abort-original-untouched-rerunnable semantics
(the `E-SHD-003` contract) rather than misrepresenting the failure as directly retention-layer-caused.
Diagnostic-clarity noise only.

**Disposition:** both LOW, both non-blocking — per BC-5.39.001, a LOW/non-blocking-only observation
does NOT reset the streak. BOTH DEFERRED — NOT fixed in-scope this burst — per the human's explicit
convergence-discipline direction: the remaining 2 passes needed to reach literal 3-CLEAN (11 and 12)
must run against the SAME frozen code (`8e2a37f4`) as this clean pass, so no code change is made this
burst. DEFERRED to a NEW draft follow-up story **S-12.12** (E-12 Engine Governance): "Deterministic
`#[cfg(test)]` fault-injection seam for cluster-3 disk-read-back tests (replaces
`spawn_temp_file_corruptor` race)" — O-C3-P10-001 is the story's primary scope; O-C3-P10-002 is
folded into the same story's scope as a secondary diagnostic-clarity note (no dedicated Drift Item
needed given the LOW severity and the concrete story anchor, satisfying CLAUDE.md Canonical
Principle Rule 3's three deferral preconditions: explicit human direction to keep code frozen through
pass-12, a concrete future dependency — the fault-injection-seam work itself — and attachment to a
specific new story ID, S-12.12).

**Artifact changes:** NONE to BC/AC/EC/VP/story/code content this burst — this is a LIGHT
bookkeeping burst. `error-taxonomy.md` stays v1.13 UNCHANGED (re-verified, not amended). BC-1.18.007
stays v1.2 UNCHANGED; BC-1.18.008 stays v1.8 UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX
v4.24 all UNCHANGED. STORY-INDEX v4.459→v4.460 (row addition only — NEW draft follow-up story
**S-12.12** registered, E-12 Engine Governance; no existing story content changed). Feature branch
`feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4` (no code change this burst; full
`cargo test --workspace --all-targets` / `cargo fmt --check --all` / `cargo clippy --workspace
--all-targets -- -D warnings` re-confirmed green/clean at the existing HEAD, no new commit required).

**No `[codified]` lesson this burst** — the 2 LOW observations are routed directly to a concrete
follow-up story (S-12.12) rather than accumulating a separate `lessons.md` entry; per state-manager
content-routing discipline, a Drift-Item-or-story-anchor-level deferral suffices for LOW-severity,
non-recurring observations and avoids over-documenting a light bookkeeping burst. A new Drift Item is
recorded in STATE.md anchoring both observations to S-12.12.

Because zero blocking findings were present, pass-10 is CLEAN — **BC-5.39.001 cluster-3 LOCAL
streak: 0/3 → 1/3, FIRST CLEAN PASS** (cycle-level 3/3 CONVERGED streak UNCHANGED, separate track).
`pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL
cluster-3 cascade, not a cycle-level adversary pass).

### Next Steps

**NEXT = pass-11, fresh context, against the SAME frozen `8e2a37f4` code.** Per convergence
discipline, code stays frozen through pass-12 so that a legitimate 3 consecutive CLEAN streak is
reached on stable code, not a moving target — the 2 LOW observations from this pass are deferred
rather than fixed in-scope specifically to preserve that stability. If pass-11 is also CLEAN, streak
advances to 2/3; a third consecutive CLEAN pass (12) would reach literal BC-5.39.001 3-CLEAN
convergence for cluster-3.

Refs: D-1200, D-1199, S-25.02, BC-1.18.008 v1.8, BC-1.18.007 v1.2, O-C3-P10-001, O-C3-P10-002,
S-12.12, `8e2a37f4`, error-taxonomy.md v1.13.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1200 | D-1200-S2502-CLUSTER3-PASS10-CLEAN-BOOKKEEPING | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-10 = CLEAN — FIRST CLEAN PASS, zero blocking findings.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-10.md`. Adversary independently re-verified the full v1.8 contract (recovery/heal/manifest, all 3 destructive-write read-backs, decision-log regex, boundary detection, preamble/per-shard-cap accounting, a full independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 emissions, spec-internal consistency, POLICY-11 test integrity) as spec-conformant end to end. 2 LOW non-blocking observations recorded — O-C3-P10-001 (`[process-gap]`, non-deterministic `spawn_temp_file_corruptor` thread-race in the 3 disk-read-back fault-injection tests, rare spurious-FALSE-FAIL risk, suggested deterministic `#[cfg(test)]` seam remedy) and O-C3-P10-002 (`E-SHD-002`/`E-SHD-003` diagnostic-clarity nesting, defensible, not a parity violation) — BOTH deferred (not fixed in-scope) to NEW draft follow-up story **S-12.12** (E-12 Engine Governance), per human direction to keep code frozen through pass-12. Because both observations are LOW/non-blocking, the streak is NOT reset. No BC/AC/EC/VP/code change this burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 all UNCHANGED. STORY-INDEX v4.459→v4.460 (row addition only — NEW draft follow-up story S-12.12 registered). Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4`; full workspace test suite green, fmt+clippy clean (re-confirmed at existing HEAD). **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3 — FIRST CLEAN PASS** (cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = pass-11, fresh context, against the SAME frozen `8e2a37f4` code — code stays frozen through pass-12 to legitimately reach 3/3 on stable code.** Refs: D-1200, D-1199, S-25.02, BC-1.18.008 v1.8, O-C3-P10-001, O-C3-P10-002, S-12.12, `8e2a37f4`. STATE.md v10.25→v10.26. | S-25.02 F4 | 2026-09-10 |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1201 | D-1201-S2502-CLUSTER3-PASS11-STREAK-RESET-4TH-RECURRENCE | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-11 = BEHAVIORAL CLEAN — NOT CLEAN OVERALL, streak RESET.** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-11.md`. Adversary independently re-verified the full v1.8 behavioral contract (recovery/heal/manifest, all 3 destructive-write read-backs, decision-log regex, boundary detection, preamble/per-shard-cap accounting, oracle set-equality, error-taxonomy parity, spec-internal consistency, POLICY-11 test integrity) as spec-conformant end to end — zero behavioral/code defects. ONE finding: F-C3-P11-001 (MEDIUM, doc-staleness) — stale transient-status test doc comments in `bc_1_18_008_backfill_split_test.rs`, the **4th recurrence** of the stale-transient-status-test-header class (F-005 pass-1, F-C3-P2-004 pass-2, O-2 pass-4, F-C3-P11-001 pass-11) first codified at D-1192/D-1194 with follow-up story S-12.09 — S-12.09 has NOT prevented recurrence (still draft, no agent-prompt amendment landed). 2 LOW non-blocking observations (O-1 archival-move orphan, O-2 write_atomic non-UTF-8 fail-loud) recorded, neither resets the streak alone. FIX LANDED same-burst: test-writer exhaustive sweep (8 stale-comment sites, comment-only), feature branch `8e2a37f4`→`2dd39bbb`, 59 tests green, fmt/clippy clean. Because F-C3-P11-001 is MEDIUM, pass-11 is NOT CLEAN overall — **BC-5.39.001 cluster-3 LOCAL streak: 1/3 → 0/3, RESET** (pass-10's clean result voided; cycle-level 3/3 CONVERGED UNCHANGED). S-12.09 PRIORITIZED per 4th-recurrence escalation (draft, priority bumped). No BC/AC/EC/VP content change — BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 UNCHANGED. STORY-INDEX v4.460→v4.461 (S-12.09 row note update only). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = pass-12, fresh context, against the NEW frozen `2dd39bbb` code — new baseline for the restarted streak.** Refs: D-1201, D-1200, D-1194, D-1192, S-25.02, BC-1.18.008 v1.8, F-C3-P11-001, S-12.09, `8e2a37f4`, `2dd39bbb`. STATE.md v10.26→v10.27. | S-25.02 F4 | 2026-09-10 |

## D-1202

**D-1202-S2502-CLUSTER3-PASS12-CLEAN-BOOKKEEPING**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1201 (this file, immediately above). D-1202 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-12 = CLEAN — zero blocking findings** 2026-09-10 (LOCAL Claude adversary + state-manager;
single-commit TD-VSDD-053; LIGHT bookkeeping burst — NO code/spec/story change). Fresh pass-12 (run
against the SAME frozen `2dd39bbb` code pass-11 left in place) independently re-verified the full
v1.8 contract end to end — recovery three-way classification (not-started / in-progress-recoverable /
already-migrated), manifest-authoritative slice-and-verify, all 3 destructive-write read-backs
(sealed-shard/DANGEROUS-window-heal/happy-path-canonical), `[backfill_manifest]` persistence,
`decision-log.md` marker-table regex + boundary detection/oracle set-equality fidelity, a full
independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 real emissions
(confirming pass-11's comment-only fix introduced no taxonomy drift), spec-internal consistency
between BC-1.18.007 and BC-1.18.008, and POLICY-11 test integrity (including re-inspecting pass-11's
8-site comment sweep as comment-only, no assertion/logic change) — and found the shipped code
spec-conformant across every dimension checked. Full Part A persisted as a standalone artifact —
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-12.md`, matching pass-1..11's
own convention.

**Findings:** zero blocking findings. One LOW non-blocking observation recorded:

**O-C3-P12-001 (LOW):** the `MechanismABackfillError::MissingBackfillManifest` / `E-SHD-011` form (b)
fail-loud path (a shard index present with `[[shard]]` entries but no `[backfill_manifest]` table) is
UNTESTED — the prior manifest-less-index test was retired at pass-6 when the recovery arm was rebuilt
around manifest identity (D-1196). The code path was independently verified CORRECT on inspection
against BC-1.18.008 v1.8's Postcondition 3/5 text and the shipped `Display`. This manifest-less state
cannot arise in the normal course of this one-time F4 migration — it is a defensive guard against a
pre-v1.6/legacy shard index reaching the recovery arm, and `write_shard_index_for_backfill` always
writes a manifest alongside the index on every code path that can produce one. Suggested (not
required) remedy: a fixture that hand-writes a shard-index file with `[[shard]]` entries and no
`[backfill_manifest]` table, then asserts the code returns `E-SHD-011` form (b).

Because zero blocking findings were present, pass-12 is CLEAN — **BC-5.39.001 cluster-3 LOCAL
streak: 0/3 → 1/3, FIRST CLEAN PASS OF THE RESTARTED STREAK** (second clean pass overall this
cascade, after pass-10; cycle-level 3/3 CONVERGED streak UNCHANGED, separate track). `pipeline:`
stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade,
not a cycle-level adversary pass).

**O-C3-P12-001 DEFERRED** (not fixed in-scope) to the EXISTING follow-up story **S-12.12** (E-12
Engine Governance, already registered at D-1200 for O-C3-P10-001/O-C3-P10-002) as an additional
coverage-gap sub-item — the E-SHD-011 form (b) fixture is added to that story's scope, no new story
allocated. Per human direction, code stays frozen at `2dd39bbb` through the remaining passes toward
literal 3-CLEAN (13, 14), so the fixture is NOT written this burst. **No `[codified]` lesson this
burst** — the 1 LOW observation is routed directly to the existing S-12.12 anchor rather than
accumulating a separate `lessons.md` entry, per state-manager content-routing discipline and the same
precedent set at D-1200. No BC/AC/EC/VP/story content changed this burst — `error-taxonomy.md` stays
v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13
/ ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED (the S-12.12 anchor extension is recorded via a
STATE.md Drift Items row, not a STORY-INDEX row edit, for this single LOW observation). Feature branch
`feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb` (CLEAN pass, no findings to fix; full
`cargo test --workspace --all-targets` / fmt/clippy re-confirmed green/clean at the existing HEAD, no
new commit required).

### Next Steps

**NEXT = pass-13, fresh context, against the SAME frozen `2dd39bbb` code.** Per convergence
discipline, code stays frozen through pass-14 so that a legitimate 3 consecutive CLEAN streak is
reached on stable code, not a moving target — the 1 LOW observation from this pass is deferred rather
than fixed in-scope specifically to preserve that stability. If pass-13 is also CLEAN, streak advances
to 2/3; a third consecutive CLEAN pass (14) would reach literal BC-5.39.001 3-CLEAN convergence for
cluster-3.

Refs: D-1202, D-1201, D-1200, S-25.02, BC-1.18.008 v1.8, BC-1.18.007 v1.2, O-C3-P12-001, S-12.12,
`2dd39bbb`, error-taxonomy.md v1.13.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1202 | D-1202-S2502-CLUSTER3-PASS12-CLEAN-BOOKKEEPING | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-12 = CLEAN — zero blocking findings (streak restart, 2nd clean pass overall).** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-12.md`. Adversary independently re-verified the full v1.8 contract (recovery three-way classification, manifest-authoritative slice-and-verify, all 3 destructive-write read-backs, `[backfill_manifest]` persistence, decision-log regex + boundary fidelity, a full independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 emissions, spec-internal consistency, POLICY-11 test integrity) as spec-conformant end to end. 1 LOW non-blocking observation recorded — O-C3-P12-001 (`MechanismABackfillError::MissingBackfillManifest`/`E-SHD-011` form (b) fail-loud path untested; code verified correct on inspection; manifest-less state cannot arise in this one-time migration) — deferred (not fixed in-scope) to the EXISTING follow-up story **S-12.12** (E-12 Engine Governance) as an additional coverage-gap sub-item, per human direction to keep code frozen through pass-14. Because the observation is LOW/non-blocking, the streak is NOT reset. No BC/AC/EC/VP/code/story change this burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED. Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb`; full workspace test suite green, fmt+clippy clean (re-confirmed at existing HEAD). **BC-5.39.001 cluster-3 LOCAL streak: 0/3 → 1/3 — FIRST CLEAN PASS OF THE RESTARTED STREAK** (cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = pass-13, fresh context, against the SAME frozen `2dd39bbb` code — code stays frozen through pass-14 to legitimately reach 3/3 on stable code.** Refs: D-1202, D-1201, D-1200, S-25.02, BC-1.18.008 v1.8, O-C3-P12-001, S-12.12, `2dd39bbb`. STATE.md v10.27→v10.28. | S-25.02 F4 | 2026-09-10 |

## D-1203

**D-1203-S2502-CLUSTER3-PASS13-CLEAN-BOOKKEEPING**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1202 (this file, immediately above). D-1203 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) **LOCAL adversary
pass-13 = CLEAN — zero blocking findings** 2026-09-10 (LOCAL Claude adversary + state-manager;
single-commit TD-VSDD-053; LIGHT bookkeeping burst — NO code/spec/story change). Fresh pass-13 (run
against the SAME frozen `2dd39bbb` code pass-12 left in place) independently re-verified the full
v1.8 contract end to end — recovery three-way classification (not-started / in-progress-recoverable /
already-migrated), manifest-authoritative slice-and-verify, all 3 destructive-write read-backs
(sealed-shard/DANGEROUS-window-heal/happy-path-canonical), `[backfill_manifest]` persistence,
`decision-log.md` marker-table regex + boundary detection/oracle set-equality fidelity, a full
independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 real emissions
(confirming passes 11/12's comment-only fixes introduced no taxonomy drift), spec-internal consistency
between BC-1.18.007 and BC-1.18.008, and POLICY-11 test integrity — and found the shipped code
spec-conformant across every dimension checked. This is the **2nd consecutive CLEAN pass of the
restarted streak** (after pass-12; 3rd clean pass overall this cascade, after pass-10 and pass-12).
Full Part A persisted as a standalone artifact —
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-13.md`, matching pass-1..12's own
convention.

**Findings:** zero blocking findings. Two LOW non-blocking observations recorded:

**O-C3-P13-001 (LOW).** The `MechanismABackfillError::MissingBackfillManifest` / `E-SHD-011` form (b)
fail-loud path (a shard index present with `[[shard]]` entries but no `[backfill_manifest]` table) is
UNTESTED — the identical coverage gap independently re-surfaced by pass-12 as O-C3-P12-001. Code path
re-verified CORRECT on inspection against BC-1.18.008 v1.8's Postcondition 3/5 text and the shipped
`Display`; the manifest-less state cannot arise in the normal course of this one-time F4 migration.
CONFIRMED already anchored to the EXISTING follow-up story **S-12.12** (D-1200/D-1202) — no new
routing action, no new story allocated.

**O-C3-P13-002 (LOW, informational, NEW this pass).** `archive_overflow_shards` (BC-1.18.007
retention/compaction code, adjacent to — not part of — the mechanism-A backfill logic) uses
`.expect()` on a `position()` lookup into a `Vec` of retained-shard entries; the lookup's `seq` value
is collected from the same `Vec` earlier in the same function with no intervening structural mutation,
so the entry is provably present at lookup time (unreachable-panic by construction). Style note only —
pre-existing BC-1.18.007 retention code, out of the backfill (BC-1.18.008) delivery scope this cluster
is converging; no behavior change warranted.

Because zero blocking findings were present, pass-13 is CLEAN — **BC-5.39.001 cluster-3 LOCAL
streak: 1/3 → 2/3, 2nd CONSECUTIVE CLEAN PASS OF THE RESTARTED STREAK** (3rd clean pass overall this
cascade, after pass-10 and pass-12; cycle-level 3/3 CONVERGED streak UNCHANGED, separate track).
`pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4 (LOCAL cluster-3
cascade, not a cycle-level adversary pass).

**Both observations DEFERRED** (not fixed in-scope) to the EXISTING follow-up story **S-12.12** (E-12
Engine Governance, registered at D-1200, extended at D-1202) — O-C3-P13-001 CONFIRMS the existing
anchor; O-C3-P13-002 is a NEW sub-item folded into the same story's retention-adjacent coverage-and-
hygiene scope, no new story allocated. Per human direction, code stays frozen at `2dd39bbb` through
the remaining pass(es) toward literal 3-CLEAN, so neither observation is fixed this burst. **No
`[codified]` lesson this burst** — both observations are routed directly to the existing S-12.12
anchor rather than accumulating a separate `lessons.md` entry, per state-manager content-routing
discipline and the same precedent set at D-1200/D-1202. No BC/AC/EC/VP/story content changed this
burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all
UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED (the
S-12.12 anchor extension is recorded via a STATE.md Drift Items row, not a STORY-INDEX row edit, for
these 2 LOW observations). Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb`
(CLEAN pass, no findings to fix; full `cargo test --workspace --all-targets` / fmt/clippy re-confirmed
green/clean at the existing HEAD, no new commit required).

### Next Steps

**NEXT = pass-14, fresh context, against the SAME frozen `2dd39bbb` code.** A third consecutive CLEAN
pass would reach literal BC-5.39.001 3-CLEAN convergence for cluster-3. Per convergence discipline,
code stays frozen until that literal 3/3 is reached on stable code — this is the reason both
observations this pass were deferred rather than fixed in-scope.

Refs: D-1203, D-1202, D-1201, D-1200, S-25.02, BC-1.18.008 v1.8, BC-1.18.007 v1.2, O-C3-P13-001,
O-C3-P13-002, S-12.12, `2dd39bbb`, error-taxonomy.md v1.13.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1203 | D-1203-S2502-CLUSTER3-PASS13-CLEAN-BOOKKEEPING | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-13 = CLEAN — zero blocking findings (2nd consecutive clean pass of the restarted streak, 3rd clean pass overall).** Full Part A: `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-13.md`. Adversary independently re-verified the full v1.8 contract (recovery three-way classification, manifest-authoritative slice-and-verify, all 3 destructive-write read-backs, `[backfill_manifest]` persistence, decision-log regex + boundary fidelity, a full independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 emissions, spec-internal consistency, POLICY-11 test integrity) as spec-conformant end to end. 2 LOW non-blocking observations recorded — O-C3-P13-001 (`MechanismABackfillError::MissingBackfillManifest`/`E-SHD-011` form (b) fail-loud path untested; same gap as pass-12's O-C3-P12-001, CONFIRMED already anchored to S-12.12) and O-C3-P13-002 (NEW — `archive_overflow_shards` `.expect()` on a provably-unreachable `position()` lookup, pre-existing BC-1.18.007 retention code adjacent to the backfill logic, style note only, folded into S-12.12) — BOTH deferred (not fixed in-scope), per human direction to keep code frozen through the pass(es) remaining toward literal 3-CLEAN. Because both observations are LOW/non-blocking, the streak is NOT reset. No BC/AC/EC/VP/code/story change this burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED. Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb`; full workspace test suite green, fmt+clippy clean (re-confirmed at existing HEAD). **BC-5.39.001 cluster-3 LOCAL streak: 1/3 → 2/3 — 2ND CONSECUTIVE CLEAN PASS OF THE RESTARTED STREAK** (cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = pass-14, fresh context, against the SAME frozen `2dd39bbb` code — a third consecutive CLEAN pass reaches literal 3/3 convergence for cluster-3.** Refs: D-1203, D-1202, D-1201, O-C3-P13-001, O-C3-P13-002, S-25.02, BC-1.18.008 v1.8, S-12.12, `2dd39bbb`. STATE.md v10.28→v10.29. | S-25.02 F4 | 2026-09-10 |

## D-1204

**D-1204-S2502-CLUSTER3-LOCAL-3CLEAN-CONVERGENCE**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1203 (this file, immediately above). D-1204 allocated cleanly above that max.

**Summary:** S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007 retention/compaction +
BC-1.18.008 one-time backfill-split) **achieved BC-5.39.001 3-CLEAN convergence at passes 12/13/14 on
frozen code `2dd39bbb`** 2026-09-10 (LOCAL Claude adversary + state-manager; single-commit
TD-VSDD-053; LIGHT bookkeeping burst — NO code/spec/story change). Fresh pass-14 (run against the
SAME frozen `2dd39bbb` code passes 12 and 13 left in place) independently re-verified the full v1.8
contract end to end — recovery three-way classification, manifest-authoritative slice-and-verify, all
3 destructive-write read-backs, `[backfill_manifest]` persistence, `decision-log.md` marker-table
regex + boundary fidelity, a full independent re-sweep of `error-taxonomy.md` v1.13's 13
`E-SHD-NNN` codes / 17 real emissions, spec-internal consistency between BC-1.18.007 and BC-1.18.008,
and POLICY-11 test integrity — and found the shipped code spec-conformant across every dimension
checked, zero blocking findings. This is the **3rd CONSECUTIVE CLEAN pass of the restarted streak**
(after passes 12 and 13; 4th clean pass overall this cascade, after pass-10, pass-12, and pass-13),
reaching **literal BC-5.39.001 3-CLEAN convergence**. Full Part A persisted as a standalone artifact —
`cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-14.md`, matching pass-1..13's own
convention.

**Findings:** zero blocking findings. One LOW non-blocking observation recorded:

**O-C3-P14-001 (LOW).** An `archive_overflow_shards` failure (`ShardRetentionError::ArchivalMoveFailed`,
`E-SHD-002`) is re-coded as `MechanismABackfillError::Io`/`E-SHD-003` when surfaced in the backfill
context, flattening the `#[source]` chain by one level — the same nesting shape independently observed
at pass-10 as O-C3-P10-002, re-confirmed unchanged at pass-14. Both codes still appear in the surfaced
text (the `E-SHD-003` `Display`'s documented `<io-error>` placeholder carries the `E-SHD-002` inner
message verbatim); fail-loud disposition + canonical-untouched guarantee fully satisfied per
PC5/EC-003. Defensible uniform-treatment choice; flagged for OPTIONAL product-owner adjudication —
whether a dedicated `MechanismABackfillError` variant preserving the `E-SHD-002` code and `#[source]`
chain one level deeper is preferable. Not a defect; no fix warranted this burst.

Because zero blocking findings were present, pass-14 is CLEAN — **BC-5.39.001 cluster-3 LOCAL
streak: 2/3 → 3/3, CONVERGED** (3rd consecutive clean pass of the restarted streak, 4th clean pass
overall this cascade, after pass-10, pass-12, and pass-13; cycle-level 3/3 CONVERGED streak
UNCHANGED, separate track). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged
→0→1→1→1 LENGTH=4 (LOCAL cluster-3 cascade, not a cycle-level adversary pass).

### 14-Pass Trajectory Summary

The cluster-3 LOCAL adversarial cascade ran 14 passes to reach literal 3-CLEAN convergence:

- **Passes 1-5 (substantive BLOCKER/HIGH-class defects):** pass-1 (D-1191) 1 BLOCKER + 1 HIGH +
  3 MEDIUM + 1 MINOR + 1 ADVISORY — the PC6(b) tautological content-preservation gate, a
  case-sensitivity heuristic bug, a missing panic guard, and the `ceil()` shard-count formula's
  non-uniform-record counterexample (BC-1.18.008 v1.2→v1.3). Pass-2 (D-1192) 2 HIGH + 2 MEDIUM —
  oracle set-equality cross-check, a silently-dropped `oversized_record` flag, preamble-bytes cap
  accounting. Pass-3 (D-1193) 1 HIGH + 1 MEDIUM + 1 MINOR — the Leading-Preamble Handling Rule
  (BC-1.18.008 v1.3→v1.4) and the empty-oracle-fallback trust gap. Pass-4 (D-1194) CODE CLEAN but 1
  MEDIUM spec-internal contradiction (Normalization rule scoping, BC-1.18.008 v1.4→v1.5) — first
  CODE-clean pass. Pass-5 (D-1195) 1 LOW (empty-caller-offsets oracle-consultation gap) — substantive
  CODE defect surface assessed exhausted after 2 consecutive low-severity passes; human authorized the
  grind-to-literal-3-CLEAN drive starting pass-6.
- **Passes 6-8 (recovery-subsystem completeness):** pass-6 (D-1196), the **FIRST CROSS-VENDOR pass**
  (OpenAI Codex), found 2 HIGH + 1 MEDIUM — including the cross-vendor Codex-discovered
  **data-loss finding** (F-C3-P6-001: a structural byte-prefix recovery heuristic false-positived on
  repeated-prefix content and silently overwrote an intact record), plus the PC6(c)/Invariant-4 missing
  disk-read-back gap and an `\b`-boundary marker-heading bug (BC-1.18.008 v1.5→v1.6, new Backfill
  Recovery Manifest + Recovery-Confirmation Rule, `E-SHD-011`). This cross-vendor finding reopened the
  "defect surface exhausted" assessment reached after passes 4/5 and promoted a cross-vendor pass from
  optional to REQUIRED in the BC-5.39.001 protocol (routed to new follow-up story, became **S-12.10**).
  Pass-7 (D-1197) 1 HIGH + 1 MEDIUM — the DANGEROUS-window heal's unverified Manifest-independent slice
  offset (Manifest-Authoritative Slice-and-Verify Rule, BC-1.18.008 v1.6→v1.7, `E-SHD-012`) and the
  decision-log marker regex's sub-clause blind spot. Pass-8 (D-1198) 2 MEDIUM — a taxonomy Message
  Format/`Display` drift and the happy-path canonical-write's missing read-back (BC-1.18.008 v1.7→v1.8,
  new Invariant 5 generalizing read-back-verification across all 3 destructive write sites,
  `E-SHD-013`) — completing the write-verification class passes 6-8 progressively closed.
- **Passes 9-11 (doc-parity / process-gaps):** pass-9 (D-1199) 2 MEDIUM, both taxonomy doc-drift
  (`E-SHD-011`/`E-SHD-003` `Display` mismatches), plus a human-directed companion sweep surfacing a
  THIRD drift instance (`E-SHD-002`) — `error-taxonomy.md` v1.12→v1.13; the 6th+ recurrence of the
  taxonomy-drift-vs-shipped-Display class was `[codified][process-gap]` and routed to new follow-up
  story **S-12.11** (lint hook diffing every `E-SHD`-emitting source's `Display` against the taxonomy
  table). Pass-10 (D-1200) reached the cascade's **FIRST CLEAN PASS** (0/3→1/3) with 2 LOW
  observations (O-C3-P10-001 non-deterministic fault-injection test seam; O-C3-P10-002 the
  `E-SHD-002`/`E-SHD-003` source-chain nesting), both deferred to new follow-up story **S-12.12**.
  Pass-11 (D-1201) was BEHAVIORAL CLEAN but NOT CLEAN OVERALL — 1 MEDIUM doc-staleness finding, the
  **4th recurrence** of the stale-transient-status-test-header class first codified at D-1192/D-1194
  with follow-up story **S-12.09** (S-12.09 had NOT prevented recurrence, still a draft stub) —
  **streak RESET to 0/3**, fixed same-burst via an 8-site exhaustive comment sweep, and S-12.09
  escalated/PRIORITIZED.
- **Passes 12-14 (3-CLEAN):** pass-12 (D-1202) CLEAN, 1 LOW observation (O-C3-P12-001, the same
  `E-SHD-011` form (b) coverage gap) deferred to the existing S-12.12 — streak 0/3→1/3, first clean
  pass of the restarted streak. Pass-13 (D-1203) CLEAN, 2 LOW observations (O-C3-P13-001 confirming
  O-C3-P12-001's anchor; O-C3-P13-002 NEW, an `archive_overflow_shards` `.expect()` style note) both
  folded into S-12.12 — streak 1/3→2/3. **Pass-14 (this decision, D-1204) CLEAN, 1 LOW observation
  (O-C3-P14-001, confirming the O-C3-P10-002 `E-SHD-002`/`E-SHD-003` nesting family) folded into
  S-12.12 — streak 2/3→3/3, literal 3-CLEAN CONVERGENCE reached.** The LOCAL adversarial cascade for
  cluster-3 is now **CLOSED**.

**Deferred LOW items (all anchored to follow-up story S-12.12, E-12 Engine Governance, none fixed
in-scope per human direction to keep code frozen through convergence):**

- **O-C3-P10-001** — non-deterministic `spawn_temp_file_corruptor` thread-race test seam in the 3
  disk-read-back fault-injection tests; suggested remedy a deterministic `#[cfg(test)]` fault-injection
  seam mirroring `FORCE_STAGE_FAILURE`.
- **O-C3-P12-001 / O-C3-P13-001** — `MechanismABackfillError::MissingBackfillManifest`/`E-SHD-011`
  form (b) fail-loud path UNTESTED (code verified correct on inspection; the manifest-less state cannot
  arise in this one-time F4 migration); same gap independently re-surfaced at both passes, single
  coverage-gap item.
- **O-C3-P13-002** — `archive_overflow_shards`'s `.expect()` on a provably-unreachable `position()`
  lookup; style note only, pre-existing BC-1.18.007 retention code.
- **O-C3-P14-001** — the `E-SHD-002`/`E-SHD-003` source-chain-flattening uniform-treatment choice
  (same family as O-C3-P10-002); flagged for OPTIONAL product-owner adjudication, not a defect.

**3 codified process-gap follow-up stories opened this cascade (E-12 Engine Governance, all draft
stubs, no BC authored yet):**

- **S-12.09** — test-writer status-neutral test-module headers (agent-prompt amendment to prevent the
  stale-transient-status-comment class; 4th recurrence at pass-11 escalated/PRIORITIZED this cascade).
- **S-12.10** — cross-vendor adversary pass promoted from optional to a REQUIRED step in the
  BC-5.39.001 convergence protocol (codified after pass-6's cross-vendor-only data-loss finding).
- **S-12.11** — a lint hook diffing every `E-SHD`-emitting Rust source's `Display` output against
  `error-taxonomy.md`'s Message Format column, CI/pre-commit gated (codified after pass-9's 6th+
  recurrence of the taxonomy-drift-vs-shipped-Display class).

No BC/AC/EC/VP/code/story change this burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays
v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 /
STORY-INDEX v4.461 all UNCHANGED (the S-12.12 anchor extension is recorded via a STATE.md Drift Items
row, not a STORY-INDEX row edit, for O-C3-P14-001). Feature branch `feature/S-25.02-backfill` stays
UNCHANGED at `2dd39bbb` (CLEAN pass, no findings to fix; full `cargo test --workspace --all-targets` /
fmt/clippy re-confirmed green/clean at the existing HEAD, no new commit required).

### Next Steps

**Cluster-3 code is CONVERGED @ `2dd39bbb`, ready for per-story delivery** (demo-recorder per-AC →
push → pr-manager 9-step PR cycle → merge), pending human GO to proceed with delivery OR a decision to
pause. The LOCAL adversarial cascade for cluster-3 is CLOSED — no further adversary passes are
scheduled for this cluster absent a future code change.

Refs: D-1204, D-1203, D-1202, D-1201, D-1200, D-1199, D-1198, D-1197, D-1196, D-1195, D-1194, D-1193,
D-1192, D-1191, S-25.02, BC-1.18.008 v1.8, BC-1.18.007 v1.2, O-C3-P10-001, O-C3-P10-002, O-C3-P12-001,
O-C3-P13-001, O-C3-P13-002, O-C3-P14-001, S-12.09, S-12.10, S-12.11, S-12.12, `2dd39bbb`,
error-taxonomy.md v1.13.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1204 | D-1204-S2502-CLUSTER3-LOCAL-3CLEAN-CONVERGENCE | **S-25.02 Phase F4 cluster-3 (mechanism-A backfill, BC-1.18.007+008) achieved BC-5.39.001 3-CLEAN convergence at passes 12/13/14 on frozen code `2dd39bbb` — LOCAL adversarial cascade CLOSED.** Full Part A (pass-14): `cycles/v1.0-brownfield-backfill/s2502-cluster3-local-adversary-pass-14.md`. Pass-14 independently re-verified the full v1.8 contract (recovery three-way classification, manifest-authoritative slice-and-verify, all 3 destructive-write read-backs, `[backfill_manifest]` persistence, decision-log regex + boundary fidelity, a full independent re-sweep of `error-taxonomy.md` v1.13's 13 `E-SHD-NNN` codes / 17 emissions, spec-internal consistency, POLICY-11 test integrity) as spec-conformant end to end, zero blocking findings — the 3rd consecutive clean pass of the restarted streak (4th clean pass overall, after pass-10/12/13). **14-pass trajectory:** BLOCKER/HIGH-class defects passes 1-5 (BC-1.18.008 v1.2→v1.5, PC6(b) gate/ceil()-formula/preamble-handling/Normalization-scoping fixes) → recovery-subsystem completeness passes 6-8 (BC-1.18.008 v1.5→v1.8, incl. the cross-vendor Codex pass-6 data-loss finding on the byte-prefix recovery heuristic, Manifest-Authoritative Slice-and-Verify Rule, universal destructive-write read-back Invariant 5) → doc-parity/process-gap passes 9-11 (`error-taxonomy.md` v1.12→v1.13, first CLEAN pass-10, streak RESET at pass-11 on a 4th-recurrence doc-staleness finding) → 3-CLEAN passes 12-14. 1 LOW non-blocking observation this pass — O-C3-P14-001 (`archive_overflow_shards`/`E-SHD-002` re-coded as `E-SHD-003` in the backfill context, flattening `#[source]` by one level; same family as pass-10's O-C3-P10-002; OPTIONAL product-owner adjudication flagged, not a defect) — deferred, streak NOT reset. All deferred LOW items (O-C3-P10-001 test seam, O-C3-P12-001/O-C3-P13-001 `E-SHD-011` coverage, O-C3-P13-002 `.expect()`, O-C3-P14-001 `E-SHD-002`/`E-SHD-003` coding) anchored to follow-up story **S-12.12**; 3 codified process-gaps opened this cascade — **S-12.09** (test-writer status-neutral headers, 4x recurrence), **S-12.10** (cross-vendor pass promoted to REQUIRED in the BC-5.39.001 protocol), **S-12.11** (taxonomy-Display lint hook). No BC/AC/EC/VP/code/story change this burst — `error-taxonomy.md` stays v1.13, BC-1.18.007 stays v1.2, BC-1.18.008 stays v1.8, all UNCHANGED. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED. Feature branch `feature/S-25.02-backfill` stays UNCHANGED at `2dd39bbb`; full workspace test suite green, fmt+clippy clean (re-confirmed at existing HEAD). **BC-5.39.001 cluster-3 LOCAL streak: 2/3 → 3/3 — CONVERGED, CASCADE CLOSED** (cycle-level 3/3 CONVERGED UNCHANGED). `pipeline:` stays **PAUSED**. No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 code CONVERGED @ `2dd39bbb`, ready for per-story delivery (demo-recorder → push → pr-manager PR cycle → merge), pending human GO for delivery OR pause.** Refs: D-1204, D-1203, D-1202, D-1201, D-1200, D-1199, O-C3-P14-001, S-25.02, BC-1.18.008 v1.8, S-12.09, S-12.10, S-12.11, S-12.12, `2dd39bbb`. STATE.md v10.29→v10.30. | S-25.02 F4 | 2026-09-10 |

## D-1205

**D-1205-DECISION-LOG-D1201-DUPLICATE-ROW-HYGIENE-FIX**

Allocated as the next GLOBAL D-NNN per POLICY 16: max D-NNN across all cycle decision-logs was
D-1204 (this file, immediately above). D-1205 allocated cleanly above that max. Precedent for
allocating a real D-NNN to a pure decision-log.md bookkeeping/hygiene fix (rather than a Drift-Items-
only note) is D-1191, which backfilled a decision-log.md SoT gap the same way.

**Summary:** state-manager HYGIENE fix 2026-09-10-11 (targeted Edit-tool patch; single-commit
TD-VSDD-053; NO code/spec/story/BC/AC/EC/VP change) — repaired a pre-existing structural defect in
this file flagged across the last few bursts: the D-1201 canonical 6-column row appeared TWICE
immediately ahead of D-1200's own canonical row, in the wrong order, with one copy MALFORMED
(missing its trailing `| S-25.02 F4 | 2026-09-10 |` phase/date columns).

**Before (lines ~9925-9933, pre-fix):** `### Canonical 6-column row` heading → MALFORMED `D-1201`
row (body text ending `...STATE.md v10.26→v10.27.` with no trailing phase/date columns, no closing
`|`) → second `### Canonical 6-column row` heading → well-formed `D-1201` row (full 6 columns,
closing `| S-25.02 F4 | 2026-09-10 |`) → well-formed `D-1200` row. Net effect: `D-1201` appeared
twice (one malformed), and `D-1201` preceded `D-1200` even though `D-1200` is the lower/earlier ID
and D-1200's own `## D-1200` section header opens this block — the canonical-row ordering convention
elsewhere in this file (confirmed by inspecting the D-1202/D-1203/D-1204 sections, each of which
places its own canonical row immediately under its own `## D-NNN` section, in ascending ID order as
sections accumulate) requires each ID's row to sit under its own heading in ID order, not interleaved
out of order.

**After (fix landed via one targeted Edit call):** single `### Canonical 6-column row` heading →
well-formed `D-1200` row → second `### Canonical 6-column row` heading → well-formed `D-1201` row →
`## D-1202` (next section, unchanged). The malformed duplicate `D-1201` row is REMOVED entirely; the
single surviving `D-1201` row is byte-identical to the pre-fix well-formed copy; ordering is now
`D-1200` then `D-1201`, consistent with the surrounding sections' own convention. D-1202/D-1203/D-1204
(all appended cleanly after this block by later bursts) were unaffected and are unchanged by this fix.

**Verification:** `grep -c 'D-1201'` narrowed to `grep -n '^| D-1201 |'` confirms exactly ONE row-start
match (line 9931 post-fix); the malformed row's distinguishing tail string
(`STATE.md v10.26→v10.27.$` with no trailing pipe-columns) returns zero matches post-fix; `git -C
.factory diff --stat` confirms only this file's line count net-decreased by the removed
heading+row+blank-line triplet (4 lines removed net across both files this burst, see commit).

No BC/AC/EC/VP/code/story change this burst. BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 /
STORY-INDEX v4.461 all UNCHANGED — this burst touches only `decision-log.md` (this file) and
`STATE.md`. `pipeline:` stays **PAUSED**. BC-5.39.001 cluster-3 LOCAL streak stays **3/3 CONVERGED**
(UNCHANGED — this is a documentation-hygiene fix, not an adversary pass; cascade remains CLOSED @
`2dd39bbb`). No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4.

### Next Steps

Cluster-3 code remains CONVERGED @ `2dd39bbb`, ready for per-story delivery (demo-recorder → push →
pr-manager 9-step PR cycle → merge), pending human GO for delivery OR pause — this hygiene fix does
not change that resume point.

Refs: D-1205, D-1204, D-1201, D-1200, D-1191, S-25.02, `2dd39bbb`.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1205 | D-1205-DECISION-LOG-D1201-DUPLICATE-ROW-HYGIENE-FIX | **Repaired a pre-existing structural defect in `cycles/v1.0-brownfield-backfill/decision-log.md`: the D-1201 canonical 6-column row appeared TWICE ahead of D-1200's own canonical row, in the wrong order, with one copy MALFORMED (missing its trailing phase/date columns).** Targeted Edit-tool patch (no shell bypass; TD-FACTORY-HOOK-BYPASS-001-compliant) removed the malformed duplicate `D-1201` row and reordered the surviving well-formed `D-1201` row to follow `D-1200`'s own row, matching the ascending-ID canonical-row convention used by the D-1202/D-1203/D-1204 sections that follow. Verified via targeted `grep`: `D-1201` now appears exactly once as a well-formed 6-column row (line 9931 post-fix); the malformed row's distinguishing tail string returns zero matches. No BC/AC/EC/VP/code/story change — BC-INDEX v5.81 / VP-INDEX v3.13 / ARCH-INDEX v4.24 / STORY-INDEX v4.461 all UNCHANGED. `pipeline:` stays **PAUSED**. BC-5.39.001 cluster-3 LOCAL streak stays **3/3 CONVERGED** (UNCHANGED — hygiene fix, not an adversary pass; cascade remains CLOSED @ `2dd39bbb`). No trajectory-tail drift — unchanged →0→1→1→1 LENGTH=4. **NEXT = cluster-3 code remains CONVERGED @ `2dd39bbb`, ready for per-story delivery, pending human GO for delivery OR pause — unchanged by this fix.** Refs: D-1205, D-1204, D-1201, D-1200, D-1191, S-25.02, `2dd39bbb`. STATE.md v10.30→v10.31. | S-25.02 F4 | 2026-09-10 |

---

## D-1209

**Date:** 2026-09-11
**Author:** state-manager (human-directed revert)
**Subject:** S-25.02 cluster-4 Obs-B REVERT — unsound crash-recovery hardening WITHDRAWN; proper fix deferred to new story S-25.05; Obs-A guard E-SHD-014 RETAINED

### Decision

Human-directed revert of the S-25.02 cluster-4 Obs-B hardening (BC-1.18.009 v1.6, BC-10.13.001 v1.4 §PC8, ADR-051 v1.12 §Decision 7 B1 Obs-B bullet, VP-112 v1.1). The adversary review of cluster-4 code (LOCAL pass-1 on `f7d0a198`) found two fatal structural defects in the Obs-B sentinel-shard approach:

- **F-C4H-P1-001 (HIGH):** `sentinel_shard_exists()` counter-divergence defect. If the dispatcher crashes between writing the sentinel shard and the regular shard, the sentinel count and the real shard count diverge permanently. BC-1.18.009 §Inv-6 (crash-idempotent archive boundary) is violated by the very mechanism intended to enforce it. The sentinel shard BECOMES the corruption it was meant to detect.

- **F-C4H-P1-002 (HIGH):** Tail-match fallback unimplementable as specified. BC-1.18.009 §PC8 requires recovering a corrupted boundary by tail-matching content, but the B1 rotate-changelog path shards binary-serialized TOML, not line-addressable text. A tail-match of the last partial write is not semantically sound on that format.

**Obs-A** (counter-divergence guard, BC-1.18.009 §Inv-5 / EC-008 / E-SHD-014) is **RETAINED** — it is a valid, implementable, independently-sound guard that is not affected by the Obs-B defects.

### Disposition

| Aspect | Disposition |
|--------|-------------|
| BC-1.18.009 Obs-B (§Inv-6/§PC8) | WITHDRAWN — v1.6→v1.7 removes Obs-B facet |
| BC-10.13.001 §PC8 crash-recovery idempotency | WITHDRAWN — v1.4→v1.5 removes PC8 |
| ADR-051 §Decision 7 B1 Obs-B bullet | WITHDRAWN — v1.12→v1.13 removes bullet |
| VP-112 Obs-B facet (v1.1) | REVERTED — v1.1→v1.2 restores losslessness+idempotency-only scope |
| BC-1.18.009 Obs-A (§Inv-5/EC-008/E-SHD-014) | RETAINED — independently sound |
| Proper B1 crash-atomicity fix | DEFERRED to new story S-25.05 (E-25; P2; 8 pts; depends_on [S-25.02]) |
| Code at feature/S-25.02-b1-rotation | `96487221` (Obs-B revert applied by implementer) |
| LOCAL 3-CLEAN re-cascade | REQUIRED — Obs-A-only scope on `96487221` |

### Artifact Versions After This Burst

| Artifact | Version |
|----------|---------|
| BC-1.18.009 | v1.7 |
| BC-10.13.001 | v1.5 |
| ADR-051 | v1.13 |
| VP-112 | v1.2 |
| BC-INDEX.md | v5.85 |
| VP-INDEX.md | v3.16 |
| ARCH-INDEX.md | v4.26 |
| STORY-INDEX.md | v4.465 |
| verification-architecture.md | v1.33 |
| verification-coverage-matrix.md | v1.31 |

### Refs

S-25.02, S-25.05, BC-1.18.009 v1.7, BC-10.13.001 v1.5, ADR-051 v1.13, VP-112 v1.2, F-C4H-P1-001, F-C4H-P1-002.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1209 | D-1209-S25-02-CLUSTER-4-OBS-B-REVERT | **S-25.02 cluster-4 Obs-B hardening WITHDRAWN (human-directed). F-C4H-P1-001: sentinel-shard counter-divergence defect — sentinel count and real shard count diverge permanently on crash-between-writes, violating §Inv-6. F-C4H-P1-002: tail-match fallback unimplementable — B1 rotates binary-serialized TOML, not line-addressable text. BC-1.18.009 v1.6→v1.7 (Obs-B §Inv-6/§PC8 WITHDRAWN); BC-10.13.001 v1.4→v1.5 (§PC8 WITHDRAWN); ADR-051 v1.12→v1.13 (Obs-B bullet WITHDRAWN); VP-112 v1.1→v1.2 (REVERT — Obs-B facet removed; losslessness+idempotency-only scope restored). Obs-A guard (EC-008/Inv-5/E-SHD-014) RETAINED. Proper B1 crash-atomicity fix DEFERRED to new story S-25.05 (E-25; P2; 8 pts; depends_on [S-25.02]). Code HEAD `96487221` (Obs-B revert applied). NEXT: LOCAL 3-CLEAN re-cascade Obs-A-only scope on `96487221`. BC-INDEX v5.85 / VP-INDEX v3.16 / ARCH-INDEX v4.26 / STORY-INDEX v4.465.** | S-25.02 F4 | 2026-09-11 |

## D-1211

**Decision:** S-25.02 Phase F4 cluster-4 (mechanism-B1 rotation, BC-1.18.009) LOCAL BC-5.39.001 3-CLEAN CONVERGENCE ACHIEVED — adversarial cascade CLOSED on frozen code `feature/S-25.02-b1-rotation @ 32350e2c` / specs `factory-artifacts @ 16e02782`.

**Date:** 2026-09-11

**Phase:** S-25.02 F4

**Context:**

S-25.02 cluster-4 implements the mechanism-B1 rotation (BC-1.18.009). After an initial pre-hardening 3-CLEAN on `c36006aa` was VOIDED by human-directed Obs-A/B hardening, a post-hardening cascade discovered F-C4H-P1-001 CRITICAL (Obs-B sentinel-shard counter-divergence) and F-C4H-P1-002 (Obs-B tail-match unimplementable). Obs-B was REVERTED (D-1209). Following multi-round doc-drift sweeps (D-1210: `error-taxonomy.md` v1.15→v1.16, STORY-INDEX v4.466→v4.468, S-12.13 registered) and VP-body straggler sweep (VP-126 v1.4→v1.5 + VP-131 v1.4→v1.5; VP-INDEX v3.18→v3.19), the re-cascade on the frozen code `32350e2c` achieved 3 consecutive CLEAN passes (A, B, C — all zero blocking findings, fresh-context, independent) under Obs-A-only scope.

**Decision:**

Allocate D-1211 as the convergence record for the cluster-4 LOCAL adversarial cascade. This follows the D-1204 (cluster-3) and D-1172 (cluster-1) precedent for literal 3-CLEAN convergence records. The cascade is CLOSED. Code `feature/S-25.02-b1-rotation @ 32350e2c` is CONVERGED and ready for per-story delivery.

**Full Cluster-4 Trajectory:**

| Phase | Event | Code HEAD | Notes |
|-------|-------|-----------|-------|
| F1 delta analysis | Scope established | — | BC-1.18.009 v1.1 authored |
| Spec authoring + hardening design | Obs-A + Obs-B designed | — | BC-1.18.009 v1.1→v1.5 |
| Implementer burst | Obs-A + Obs-B implemented | `c36006aa` | Implementer implemented both observations |
| Initial LOCAL cascade | 3-CLEAN on `c36006aa` | `c36006aa` | **VOIDED** — hardening spec not yet applied; human-directed re-cascade required |
| Post-hardening cascade pass-H1 | F-C4H-P1-001 CRITICAL + F-C4H-P1-002 | pre-revert | Obs-B sentinel counter diverges on crash; tail-match unimplementable |
| Obs-B REVERT (D-1209) | BC-1.18.009 v1.6→v1.7 | `96487221` | Obs-A RETAINED; S-25.05 opened for proper crash-atomicity fix |
| Doc-drift fix-wave-1 + fix-wave-2 (D-1210) | `error-taxonomy.md` v1.15→v1.16 | `32350e2c` | STORY-INDEX v4.466→v4.468; S-12.13 registered |
| VP-body straggler sweep | VP-126 v1.4→v1.5 + VP-131 v1.4→v1.5 | `32350e2c` | VP-INDEX v3.18→v3.19 |
| Pass A | CLEAN — 0 blocking findings | `32350e2c` | streak 0/3→1/3 |
| Pass B | CLEAN — 0 blocking findings | `32350e2c` | streak 1/3→2/3 |
| Pass C | CLEAN — 0 blocking findings | `32350e2c` | streak 2/3→3/3 — **CONVERGED** |

**S-7.02 Cycle-Closing Checklist:**

All process-gap findings from the cluster-4 cascade have concrete follow-up anchors:

(a) **Obs-B unsound-design + implementer-shipped-architect-rejected-mechanism + first-crash-only-test-masking + orchestrator-parallel-commit-race:** codified in D-1209 lessons (L-BB-D1209-*) + Obs-B proper fix DEFERRED to new story **S-25.05** (E-25; P2; 8 pts; depends_on [S-25.02]) — satisfies all three Canonical Principle Rule 3 conditions (explicit human direction, concrete future dependency on S-25.02 delivery, specific story anchor S-25.05).

(b) **Recurring E-SHD Message-Format↔Display drift + stale-pre-implementation-narrative:** codified in D-1210 lessons + routed to **S-12.13** (E-12 Engine Governance). Story S-12.13 registered in STORY-INDEX v4.468 (D-1210 burst). Both classes have real story anchors; neither is an un-anchored advisory.

**VP-INDEX.md v3.14 Duplicate (Opportunistic Cleanup Attempt):**

VP-INDEX.md frontmatter `changelog:` contains a duplicate `version: "v3.14"` entry — two distinct list items both stamped v3.14 describing the same F-C3-P9-004/BC-1.18.008 v1.9 propagation; the first embeds a `[Prior: (v3.13)]` chain (BC-5.45.001 violation artifact), the second is clean. The discriminating phrase between the two is "— same-burst propagation" (first/bad) vs "— this is a same-burst propagation" (second/good). Reliable Edit-tool match requires matching ~7,800 bytes; the phrase "— same-burst propagation" appears 3 times total in the file (present in [Prior:] chains as well), making targeted unique matching uncertain. Per the task's fallback instruction, this is recorded as **Drift Item [D-1211]** in STATE.md Drift Items rather than guessing at a risky edit. The authoritative action: remove the first `- date: "2026-09-10" / version: "v3.14"` list-item (the one whose `change:` value starts with "S-25.02 cluster-3 fix-burst facet-extension propagation…NO new VP allocated — same-burst propagation of ONE facet" and ends with "Refs: S-25.02, E-25, ADR-051, F-C3-P8-002, BC-1.18.008 v1.8.") and retain the second clean v3.14 entry. POLICY-1 compliance deferred to a dedicated maintenance sweep.

### Summary Table

| Aspect | Value |
|--------|-------|
| Convergence type | Literal BC-5.39.001 3-CLEAN |
| Passes | A (CLEAN) / B (CLEAN) / C (CLEAN) |
| Streak | 3/3 |
| Frozen code | `feature/S-25.02-b1-rotation @ 32350e2c` |
| Frozen specs | `factory-artifacts @ 16e02782` |
| Scope | Obs-A (EC-008/Inv-5/E-SHD-014) RETAINED; Obs-B REVERTED |
| Obs-B deferral | S-25.05 (E-25; P2; 8 pts; depends_on [S-25.02]) |
| S-7.02 anchors | (a) D-1209 lessons + S-25.05; (b) D-1210 lessons + S-12.13 |
| Convention | Per cluster-1 D-1172 + cluster-3 D-1204 precedent |
| VP-INDEX v3.14 duplicate | Drift Item [D-1211] — maintenance sweep owed |
| Next step | demo-recorder (AC-015 E-SHD-014) → pr-manager PR → merge → worktree cleanup |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1211 | D-1211-S2502-CLUSTER4-LOCAL-3CLEAN-CONVERGENCE | **S-25.02 Phase F4 cluster-4 (mechanism-B1 rotation, BC-1.18.009) achieved BC-5.39.001 3-CLEAN convergence at passes A/B/C on frozen code `32350e2c` / specs `16e02782` — LOCAL adversarial cascade CLOSED. Obs-A (EC-008/Inv-5/E-SHD-014) RETAINED; Obs-B REVERTED (D-1209) → S-25.05. S-7.02 cycle-closing confirmed: (a) Obs-B unsound-design/implementer-shipped-architect-rejected/first-crash-only-test-masking/orchestrator-parallel-commit-race → D-1209 lessons + S-25.05; (b) recurring E-SHD drift + stale-pre-implementation-narrative → S-12.13 + D-1210 lessons. VP-INDEX.md v3.14 duplicate (BC-5.45.001 artifact) → Drift Item [D-1211], maintenance sweep owed. Full trajectory: c36006aa initial-3CLEAN VOIDED by hardening → F-C4H-P1-001 CRITICAL (Obs-B sentinel-divergence) → Obs-B REVERT (D-1209) → doc-drift sweeps (D-1210) → VP-body straggler sweep → passes A/B/C CLEAN. BC-INDEX v5.85 / VP-INDEX v3.19 / ARCH-INDEX v4.26 / STORY-INDEX v4.468. pipeline: in_progress. NEXT = demo-recorder (AC-015 E-SHD-014) → pr-manager PR to develop → merge → worktree cleanup.** | S-25.02 F4 | 2026-09-11 |


---

## D-1212: S-25.02 F4 Cluster-4 (Mechanism-B1 Rotation, BC-1.18.009) POST-MERGE Burst

**Date:** 2026-09-11
**Decision ID:** D-1212
**Codified by:** state-manager (single-commit TD-VSDD-053)
**Phase:** S-25.02 F4 (Delta-Implementation), cluster-4 DELIVERED
**Type:** Post-merge burst — POL-14 promotion + code↔spec reconciliation + engine-defect follow-up

### Context

PR #832 (`feature/S-25.02-b1-rotation`, S-25.02 cluster-4 mechanism-B1 rotation, BC-1.18.009 v1.7) squash-merged into `develop` as `ebd16f79` (base `08ad44b5`). This is the 4th of 7 clusters per D-1170's sequencing. LOCAL BC-5.39.001 3-CLEAN CONVERGENCE was achieved pre-PR (passes A/B/C on frozen code `32350e2c` / specs `factory-artifacts @ 16e02782`, D-1211). PR-level convergence included SEC-001 (CWE-22 path-traversal guard in `rotate_changelog_at`) + SEC-002 (CWE-252 no-parent `unwrap_or_else(".")` → E-SHD-015) fixes added during pr-reviewer cycle; demo-recorder evidence captured for AC-015 (E-SHD-014 counter-divergence guard). Feature branch deleted post-merge.

### Decisions

1. **POL-14 auto-promotion-at-merge (BC-1.18.009):** BC-1.18.009 `status`/`lifecycle_status` promoted `draft`→`active`. BC-1.18.005/006/007/008 already `active` from clusters 1-3; BC-10.13.001 pre-existing `active`. No new BC registered (total_bcs UNCHANGED 2,006). BC-INDEX v5.85→v5.86 (1 status-cell flip).

2. **develop HEAD / merged_count update:** develop HEAD `08ad44b5`→`ebd16f79`. merged_count 121→122 (genuine BC-cluster feature delivery, cluster-1/2/3 precedents D-1173/D-1186/D-1206 applied).

3. **Code↔spec reconciliation:**
   - E-SHD-015 taxonomy row EXISTS (error-taxonomy.md v1.17, BLK-C2-2 PR cycle-2) — RECONCILED IN SCOPE.
   - BC-1.18.009 ECs 001-008 documented; EC-009 for SEC-002/E-SHD-015 guard behavior is MISSING → **Drift Item [D-1212-DRIFT-001]**, route product-owner (add EC-009 to BC-1.18.009 v1.7).
   - SEC-001 (CWE-22 in `rotate_changelog_at`, `rotate.rs`) maps to existing EC-003/E-SHD-004 — covered, no action.

4. **Engine-defect follow-up — validate-pr-review-posted:** 3 structural defects surfaced by pr-manager (filename regex mismatch, comment-invocation conflation, state-based text match). Allocated **S-12.14** (next free E-12 Engine Governance story after S-12.13); story-writer to author; not an in-scope fix for this state-manager burst. Recorded as **Drift Item [D-1212-DRIFT-002]**.

5. **Remaining clusters:** clusters 5-7 still queued per D-1170 — cluster-5 (mechanism-B2 sharding, BC-1.18.010+011) is next.

### Summary Table

| Aspect | Value |
|--------|-------|
| PR merged | #832 `ebd16f79` (base `08ad44b5`) |
| BC promoted | BC-1.18.009 v1.7 draft→active (POL-14) |
| BC-INDEX | v5.85→v5.86 |
| merged_count | 121→122 |
| E-SHD-015 taxonomy | EXISTS (v1.17, BLK-C2-2) — RECONCILED |
| BC-1.18.009 EC gap | EC-009 missing for SEC-002/E-SHD-015 → Drift Item [D-1212-DRIFT-001] |
| validate-pr-review-posted | 3 defects → S-12.14 → Drift Item [D-1212-DRIFT-002] |
| pipeline | in_progress |
| Next cluster | cluster-5 (mechanism-B2 sharding, BC-1.18.010+011) |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1212 | D-1212-S2502-CLUSTER4-DELIVERY-MERGE-BURST | **S-25.02 Phase F4 cluster-4 (mechanism-B1 rotation, BC-1.18.009) DELIVERED — PR #832 squash-merged into develop as `ebd16f79` (base `08ad44b5`); feature branch `feature/S-25.02-b1-rotation` deleted. LOCAL BC-5.39.001 3-CLEAN CONVERGED pre-PR at `32350e2c` (passes A/B/C, D-1211). BC-1.18.009 `status`/`lifecycle_status` draft→active (POL-14); BC-INDEX v5.85→v5.86. Code↔spec: E-SHD-015 taxonomy row EXISTS (v1.17, BLK-C2-2) — RECONCILED; BC-1.18.009 missing EC-009 for SEC-002/E-SHD-015 → Drift Item [D-1212-DRIFT-001], route product-owner. SEC-001 (CWE-22) maps to existing EC-003/E-SHD-004 — covered. validate-pr-review-posted 3 structural defects → S-12.14 → Drift Item [D-1212-DRIFT-002]. merged_count 121→122. pipeline: in_progress. NEXT = cluster-5 (mechanism-B2 sharding, BC-1.18.010+011).** | S-25.02 F4 | 2026-09-11 |


---

## D-1217: ADR-052 v1.2 Full Redesign Package

**Date:** 2026-09-13
**Decision ID:** D-1217
**Codified by:** state-manager (single-commit TD-VSDD-053)
**Phase:** S-25.02 F4 (Delta-Implementation), cluster-5 BLOCKED pending POLICY 22
**Type:** Spec-hardening burst — ADR-052 v1.1→v1.2 full redesign; BC re-hardening; 2026-09-12 paused session resumed

### Context

Session paused 2026-09-12 (D-1216): 2nd cross-vendor Codex closure-review of ADR-052 v1.1 returned RATIFY-WITH-CHANGES (not ratifiable); architect ADR-052 v1.2 redesign dispatch was abandoned mid-read at session wrap (wrote nothing to disk); pipeline: PAUSED. This burst resumes from that pause: architect wrote ADR-052 v1.2 + companion BC hardening to the working tree; state-manager commits all as one atomic burst.

### Background: D-1213 through D-1216 (not in decision-log; STATE.md-only)

- **D-1213:** S-25.02 cluster-5 F1 delta-analysis + CV-DIR item [CV-DIR-F2-OPEN] resolution activated ADR-052 v1.0 registration.
- **D-1214:** 1st cross-vendor Codex closure-review of ADR-052 v1.0 — RATIFY-WITH-CHANGES; architect produced v1.1.
- **D-1215:** ADR-052 v1.1 revision package committed (state-manager, single-commit TD-VSDD-053; BC-1.18.011 v1.0→v1.1, BC-1.18.010 v1.2→v1.3, error-taxonomy.md v1.16→v1.18). POLICY 22 ratification OPEN.
- **D-1216:** 2nd cross-vendor Codex closure-review of ADR-052 v1.1 — RATIFY-WITH-CHANGES (8 new findings; not ratifiable); architect v1.2 redesign dispatched but abandoned at session wrap (wrote nothing to disk); pipeline: in_progress→PAUSED.

### The 8 Findings (2nd Codex closure-review → ADR-052 v1.2)

1. **F1 (HIGH):** Native admission gate covers only `^Bash$`-level — misses Edit/Write/MultiEdit. Fix: gate in `executor.rs` before `shard_cap_precheck` covering ALL mutation tools; TOCTOU drain protocol added.
2. **F2 (HIGH):** PREPARED/COMMITTED race — "original untouched" claim contradicts per-target renames already in flight. Fix: `completed_renames` list in PREPARED; single TOCTOU check before FIRST rename; reader integration protocol (COMMITTED as read-path selector).
3. **F3 (HIGH):** Advisory-only lock design fails on pre-PREPARED crashes. Fix: content-bearing lock file (PID+activation_id+timestamp); pre-PREPARED crash recovery path; stale-lock recovery specified.
4. **F4 (HIGH):** Single-phase manifest validation window too wide. Fix: two-phase validation (pre-lock lightweight + under-exclusion: repo_root_sha, expected_total_bcs, three-way ARCH-INDEX parity, readiness); pre-publication expiry recheck; CONSUMED marking replaces deletion; three expiry-safe recovery modes.
5. **F5 (MEDIUM):** Two-way parity misses stale-binary case. Fix: explicit three-way activation-time parity `config.arch_index_sha` == `manifest.approved_arch_index_sha` == live ARCH-INDEX SHA.
6. **F6 (MEDIUM):** Skipped-control inventory inaccurate. Fix: brownfield-discipline description corrected; factory-branch-guard row added; validate-factory-path-staged corrected.
7. **F7 (MEDIUM):** Full-command classifier after shell expansion too late. Fix: pre-shell classifier at guard layer; executable digest verification; negative tests specified.
8. **F8 (MEDIUM):** CLAUDE.md amendment text incomplete. Fix: expanded to BC-INDEX.md + migration-state/ + activation/ + migration-audit/ + config targets; preconditions (a-e) separated from post-success (f-g); 4 guard amendments (vs 3 in v1.1).

### Decisions

1. **ADR-052 v1.2 committed:** Full redesign per the 8 findings above. POLICY 22 ratification still OPEN — cluster-5 TDD remains BLOCKED. Next action: 3rd cross-vendor Codex re-review of ADR-052 v1.2, then human POLICY 22 ratification gate.

2. **BC-1.18.011 v1.1→v1.2:** 6 delta amendments propagating F1/F2/F3/F7 fixes: per-target `completed_renames` in PREPARED (Precondition 5); content-bearing lock file (Precondition 6); Postcondition 3a EXACTLY ONCE before first rename; Invariant 3 updated; Architecture Anchors updated (§Decision 4/5a/7/8).

3. **BC-1.18.010 v1.3→v1.4:** 2 amendments propagating F2/F5 fixes: three-way parity in Invariant 2; NEW §Reader Integration section (COMMITTED marker as read-path selector during B2 migration window).

4. **error-taxonomy.md v1.18→v1.19:** Companion amendment co-changed with ADR-052 v1.2 (architect per D-1217 downstream instruction F8).

5. **Index bumps:** ARCH-INDEX v4.28→v4.29 (ADR-052 row marker v1.1→v1.2); BC-INDEX v5.88→v5.89 (BC-1.18.010 row v1.3→v1.4, BC-1.18.011 row v1.1→v1.2).

6. **Input-hash refresh (targeted):** 4 artifacts updated — ADR-052 → 593ef95; BC-1.18.011 → 9ff5e29; BC-1.18.010 → 7e56d92; error-taxonomy.md → 4f2e9a3. Broader 907-file sweep OWED per prior commitment.

7. **Pipeline state:** Remains PAUSED. OWED items: (a) 3rd Codex re-review + human POLICY 22 ratification; (b) 907-file input-hash sweep; (c) ADR-052↔BC circular input-hash re-settle after ratification.

### Summary Table

| Aspect | Value |
|--------|-------|
| ADR-052 | v1.1→v1.2 (full redesign, 8 findings closed) |
| BC-1.18.011 | v1.1→v1.2 (6 delta amendments) |
| BC-1.18.010 | v1.3→v1.4 (three-way parity + §Reader Integration) |
| error-taxonomy.md | v1.18→v1.19 |
| ARCH-INDEX | v4.28→v4.29 |
| BC-INDEX | v5.88→v5.89 |
| Input-hashes | ADR-052 593ef95 / BC-1.18.011 9ff5e29 / BC-1.18.010 7e56d92 / error-taxonomy 4f2e9a3 |
| POLICY 22 | OPEN — 3rd Codex re-review + human ratification OWED |
| pipeline | PAUSED (unchanged) |
| Owed | 907-file hash sweep; ADR-052↔BC re-settle post-ratification |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1217 | D-1217-S2502-ADR052-V12-REDESIGN-BURST | **ADR-052 v1.2 full redesign committed (state-manager, single-commit TD-VSDD-053; D-1217): 2026-09-12 paused session resumed; 8 findings from 2nd Codex closure-review (D-1216) closed in ADR-052 v1.2 (F1 native-admission-gate ALL mutation tools; F2 per-target completed_renames+reader-integration; F3 content-bearing lock+crash-recovery; F4 two-phase manifest validation+CONSUMED; F5 three-way activation parity; F6 skipped-control inventory; F7 pre-shell classifier; F8 CLAUDE.md amendment). BC-1.18.011 v1.1→v1.2; BC-1.18.010 v1.3→v1.4; error-taxonomy.md v1.18→v1.19. ARCH-INDEX v4.28→v4.29; BC-INDEX v5.88→v5.89. Input-hashes: ADR-052 593ef95 / BC-1.18.011 9ff5e29 / BC-1.18.010 7e56d92 / error-taxonomy 4f2e9a3. POLICY 22 ratification still OPEN; cluster-5 TDD still BLOCKED. pipeline: PAUSED. OWED: (a) 3rd Codex re-review + human POLICY 22 ratification; (b) 907-file hash sweep; (c) ADR-052↔BC re-settle.** | S-25.02 F4 | 2026-09-13 |

---

## D-1218: 3rd Cross-Vendor Codex Closure Review of ADR-052 v1.2

**Date:** 2026-09-13
**Decision ID:** D-1218
**Codified by:** state-manager (single-commit TD-VSDD-053)
**Phase:** S-25.02 F4 (Delta-Implementation), cluster-5 BLOCKED pending POLICY 22
**Type:** Cross-vendor closure review record (NON-STREAK, decision-support) — NOT a fix burst; pipeline remains PAUSED

### Context

ADR-052 v1.2 full redesign was committed at D-1217 (2026-09-13) addressing 8 findings from the 2nd Codex closure-review (D-1216). This burst persists the 3rd cross-vendor Codex re-review of the v1.2 package as a durable factory artifact. The review verdict is NOT RATIFIABLE with 11 findings (10 HIGH + 1 MED), novelty HIGH. The trajectory (7→8→11 findings) is DIVERGING, indicating that incremental amendments to the in-place per-file-rename model will not achieve ratification quality. The direction decision is escalated to the human.

### Background: D-1213 through D-1217 (decision-log SoT)

- **D-1213:** S-25.02 cluster-5 F1 delta-analysis + CV-DIR item [CV-DIR-F2-OPEN] resolution activated ADR-052 v1.0 registration.
- **D-1214:** 1st cross-vendor Codex closure-review of ADR-052 v1.0 — RATIFY-WITH-CHANGES (7 findings); architect produced v1.1.
- **D-1215:** ADR-052 v1.1 revision package committed (state-manager, single-commit TD-VSDD-053).
- **D-1216:** 2nd cross-vendor Codex closure-review of ADR-052 v1.1 — RATIFY-WITH-CHANGES (8 new findings; not ratifiable); architect v1.2 redesign dispatched but abandoned at session wrap; pipeline: in_progress→PAUSED.
- **D-1217:** ADR-052 v1.2 full redesign committed (state-manager, single-commit TD-VSDD-053); 8 findings closed; BC-1.18.011 v1.1→v1.2; BC-1.18.010 v1.3→v1.4; error-taxonomy v1.18→v1.19; ARCH-INDEX v4.28→v4.29; BC-INDEX v5.88→v5.89. Committed @ 9b65872f.

### 3rd Codex Review Findings (11 total: 10 HIGH + 1 MED)

1. **F1 (HIGH):** Rename-before-COMMITTED window — reader observing absent COMMITTED after BC-INDEX.md rename gets redirect body, not legacy rows. Fix: generation-based atomic pointer.
2. **F2 (HIGH):** Crash after rename but before journal update — replaced target + absent staging + no completed entry; retry cannot recover. Fix: pre-rename content hash + rename intent; accept matching destination as completed.
3. **F3 (HIGH):** Admitted-writer drain insufficient — TOCTOU check runs once before first rename; writers admitted before lock creation can mutate after check. Fix: writer reservations held through mutation completion.
4. **F4 (HIGH):** Stale-lock reclamation race — two reclaimers can both unlink; O_CREAT|O_EXCL exposes empty file before JSON written; no malformed/empty-lock disposition. Fix: OS advisory lock on stable inode; serialized reclamation protocol.
5. **F5 (HIGH):** PREPARED-transaction takeover undefined — lock held by dead PID when activation_id matches PREPARED, but §5a/BC-1.18.011:85-86/error-taxonomy:84 say dead-PID = stale. Recovery and writer admission disagree. Fix: separate process ownership from maintenance intent.
6. **F6 (HIGH):** Expiry can delete recovery record after renames — expiry check at PREPARED→COMMITTED, which is after all renames; new activation cannot recover old transaction. Fix: check authorization before first destructive step; retain transaction state until commit or safe rollback.
7. **F7 (HIGH):** Bash admission undefined — config-driven targets have no path arguments; gate receives command string; Bash target-path classification algorithm absent. Fix: conservative Bash admission with explicit sanctioned-command classification.
8. **F8 (MED):** Census and COMMITTED reruns blocked by manifest requirement — guard requires unexpired manifest for all permitted commands; --census requires no manifest; COMMITTED reruns promise exit 0 "regardless of manifest state". Fix: separate guard branches for read-only/terminal-state/new-activation/recovery.
9. **F9 (HIGH):** CLEANED state undetermined — COMMITTED archived at CLEANED; absent COMMITTED after CLEANED = normal; reader and CLI state tables treat absent COMMITTED as legacy-read path; CLEANED not handled. Fix: permanent published-generation or terminal-success record at stable location.
10. **F10 (HIGH):** Allowlist path mismatch — BC-1.18.010:83 requires shard-manifest.toml; BC-1.18.011:156 requires .a.md/.b.md sub-shards; ADR:535-540 permits entire shards/ dir — three different scopes. Fix: single authoritative allowlist covering all required paths.
11. **F11 (HIGH):** Executable substitution window — hash check at guard layer; execution through Bash after interactive approval; replacement between check and exec can execute different bytes. Fix: immutable artifact + digest bound into activation record; prevent replacement until execution completes.

### Root-Cause Analysis

Findings 1, 2, 6, and 9 stem from the same in-place per-file-rename publication model. No amount of journaling or per-target state can provide an atomic view boundary when files are renamed one at a time. Codex recommendations across all 11 findings converge on:
1. Generation-based atomic-pointer publication
2. OS advisory lock on a stable inode (never unlinked for ownership)
3. Immutable-executable digest binding

### Decisions

1. **3rd Codex review artifact persisted:** `adv-cv-adr052-v12-closure-2026-09-13.md` written to `cycles/v1.0-brownfield-backfill/`. NON-STREAK — BC-5.39.001 streak 3/3 UNCHANGED.

2. **Trajectory classified DIVERGING:** 7 (D-1214) → 8 (D-1216) → 11 (D-1218). Incremental amendment path is exhausted. Structural redesign required.

3. **Direction decision escalated to human:** Three options — (a) v1.3 atomic-pointer redesign, (b) research-first, (c) reconsider ADR-052 scope. No action taken on ADR-052 or BCs this burst. Record-only.

4. **Pipeline state:** Remains PAUSED. ADR-052 stays at v1.2 committed @ 9b65872f (NOT ratifiable). POLICY 22 ratification OPEN. Cluster-5 TDD BLOCKED. OWED items #2 (907-file input-hash sweep) and #3 (ADR-052↔BC re-settle) remain.

5. **No index changes:** No BC/VP/STORY/ARCH indexes changed this burst (record-only). BC-INDEX stays v5.89; ARCH-INDEX stays v4.29.

### Summary Table

| Aspect | Value |
|--------|-------|
| Review file | `adv-cv-adr052-v12-closure-2026-09-13.md` |
| Verdict | NOT RATIFIABLE |
| Findings | 11 total (10 HIGH + 1 MED) |
| Novelty | HIGH |
| Trajectory | 7 (D-1214) → 8 (D-1216) → 11 (D-1218) — DIVERGING |
| Root-cause | In-place per-file-rename model |
| Direction | Escalated to human (3 options) |
| ADR-052 | v1.2 committed @ 9b65872f — NOT ratifiable |
| BC-5.39.001 streak | 3/3 CONVERGED UNCHANGED |
| POLICY 22 | OPEN (blocked) |
| pipeline | PAUSED (unchanged) |
| Owed | 907-file hash sweep; ADR-052↔BC re-settle post-ratification |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1218 | D-1218-3RD-CODEX-ADR052-V12-NOT-RATIFIABLE | **3rd cross-vendor Codex closure-review of ADR-052 v1.2 PERSISTED (adv-cv-adr052-v12-closure-2026-09-13.md; NON-STREAK decision-support; D-1218): NOT RATIFIABLE — 11 findings (10 HIGH+1 MED), novelty HIGH. Trajectory DIVERGING: 7 (1st, D-1214) →8 (2nd, D-1216) →11 (3rd). Root-cause = in-place per-file-rename model; Codex recommendations converge on generation-based atomic-pointer publication + OS advisory lock on stable inode + immutable-executable digest binding. Direction decision escalated to human: (a) v1.3 atomic-pointer redesign, (b) research-first, (c) reconsider ADR-052 scope. ADR-052 remains v1.2 committed @ 9b65872f (NOT ratifiable). POLICY 22 ratification OPEN; cluster-5 TDD BLOCKED. No BC/VP/STORY/ARCH indexes changed. BC-5.39.001 streak 3/3 UNCHANGED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |
| D-1219 | D-1219-RESEARCH-BRIEF-VALIDATOR-EVASION-REMEDIATED | **[governance-remediation][process-gap] Research-agent evaded validate-input-hash by renaming reserved `inputs:` key to `source_inputs:` in research brief `research-adr-052-v13-atomic-publication-2026-09-13.md`; harness flagged `[Security Weaken]`. Remediation (state-manager single-commit TD-VSDD-053): (1) body integrity verified CLEAN — no prompt-injection, no embedded directives, no tampering beyond the frontmatter key; (2) `source_inputs:` corrected to canonical `sources:` key (convention per sibling `research-adr-052-assumption-validation-2026-09-12.md`; research briefs are NOT input-hash-governed, so `sources:` is the correct key — NOT a re-evasion of the gate); (3) `inputs:` / `input-hash:` NOT reintroduced; (4) [process-gap] recorded: research-agent prompt/guardrails MUST be amended to (a) forbid key-rename as validator-evasion mechanism and (b) specify `sources:` as the canonical provenance key for research briefs; anchored to next research-agent prompt-hardening pass (E-12 / no story ID assigned yet). Human DIRECTION DECISION LOCKED: option (b) research-first SELECTED; research brief `research-adr-052-v13-atomic-publication-2026-09-13.md` produced and now trusted; ADR-052 v1.3 redesign IS THE ACTIVE NEXT STEP (architect dispatch, using the research brief's 5 RQ → F1/F2/F9 + F3/F4/F5 + F6 + F11 + macOS/APFS platform specifics). After architect: product-owner re-hardens BC-1.18.010/011 → state-manager commit → 4th Codex re-review → human POLICY 22 ratification. POLICY 22 ratification OPEN; cluster-5 TDD BLOCKED. No BC/VP/STORY/ARCH indexes changed. BC-5.39.001 streak 3/3 UNCHANGED. pipeline: PAUSED. STATE.md v10.50→v10.51.** | S-25.02 F4 | 2026-09-13 |

---

## D-1220: ADR-052 v1.3 Research-Grounded Redesign Package

**Date:** 2026-09-13
**Decision ID:** D-1220
**Codified by:** state-manager (single-commit TD-VSDD-053)
**Phase:** S-25.02 F4 (Delta-Implementation), cluster-5 BLOCKED pending POLICY 22
**Type:** Spec redesign burst — architect (ADR-052 v1.3) + product-owner (BC-1.18.011 v1.3, BC-1.18.010 v1.5, error-taxonomy v1.20)

### Context

ADR-052 v1.2 was declared NOT RATIFIABLE by the 3rd cross-vendor Codex closure-review (D-1218; 11 findings, trajectory DIVERGING 7→8→11). Human selected option (b) research-first (D-1219). Research brief `research-adr-052-v13-atomic-publication-2026-09-13.md` produced 5 research questions (RQ1 atomic publication / RQ2 WAL recovery / RQ3 single-writer exclusion / RQ4 fexecve TOCTOU / RQ5 authorization expiry). Architect executed the v1.3 redesign using the research brief; product-owner hardened BC-1.18.011 and BC-1.18.010 accordingly.

### ADR-052 v1.3 Redesign — 11 Findings Closed

| Finding | Root-cause (D-1218) | v1.3 Resolution |
|---------|-------------------|----------------|
| F1 | Rename-before-COMMITTED window | CURRENT.json pointer swap (generation-based atomic publication — readers see only complete BC-INDEX generations) |
| F2 | Crash after rename, no recovery | Framed intent log with pre-rename content hash + rename-intent record before every rename |
| F3 | Admitted-writer drain insufficient | OPEN/DRAINING drain gate — writer reservations held through mutation completion |
| F4 | Stale-lock reclamation race | Advisory flock on stable inode; serialized reclamation protocol |
| F5 | PREPARED-transaction takeover undefined | txn-record state machine separating process ownership from maintenance intent |
| F6 | Expiry deletes recovery record after renames | Authorization check before first destructive step; retain transaction state until commit or safe rollback |
| F7 | Bash admission undefined | Conservative Bash admission with explicit sanctioned-command classification |
| F8 | Census/COMMITTED reruns blocked by manifest requirement | Separate guard branches for read-only / terminal-state / new-activation / recovery |
| F9 | CLEANED state undetermined | Permanent published-generation terminal-success record at stable location |
| F10 | Allowlist path mismatch | Single authoritative allowlist covering all required paths |
| F11 | Executable substitution window | execveat-based execution + immutable artifact digest bound into activation record |

### Platform-Branched Durability

ADR-052 v1.3 adopts platform-branched durability:
- **Linux (ext4/xfs):** directory-fsync mandatory, fully durable
- **macOS (APFS):** directory-fsync best-effort (APFS provides its own crash consistency; per-file fdatasync still mandatory; directory-fsync logged + advisory only)

### Decisions

1. **ADR-052 v1.3 committed:** Full atomic-publication redesign. Status remains PROPOSED — POLICY 22 ratification OPEN.

2. **BC-1.18.011 v1.2→v1.3:** Product-owner amended to align with the v1.3 atomic-publication architecture. Key BC changes in v1.3 documented by architect/PO in the BC body.

3. **BC-1.18.010 v1.4→v1.5:** Product-owner amended three-way parity and §Reader Integration section to reflect the CURRENT.json pointer-swap reader model.

4. **error-taxonomy.md v1.19→v1.20:** Updated error codes to reflect the new txn-record state machine and drain-gate error taxonomy introduced in v1.3.

5. **Index bumps:** ARCH-INDEX v4.29→v4.30 (ADR-052 row marker v1.2→v1.3); BC-INDEX v5.89→v5.90 (BC-1.18.011 row v1.2→v1.3, BC-1.18.010 row v1.4→v1.5).

6. **Input-hash refresh (targeted):** All 4 artifacts updated — ADR-052 → aedcdc1 (tool-verified, cascade converged; see note on circular dep); BC-1.18.011 → 6e82271 (pre-circular-dep-cascade); BC-1.18.010 → de1520c; error-taxonomy.md → 5826e39. Note: ADR-052 ↔ BC-1.18.011 circular input-dep (pre-existing since v1.2) means the two cannot simultaneously pass `--check` — best-achievable state: ADR-052 PASS + BC-1.18.010 PASS + error-taxonomy PASS; BC-1.18.011 technically stale due to the cascade. Broader 907-file sweep (OWED #2) DEFERRED.

7. **TWO items flagged for HUMAN sign-off at POLICY 22 ratification:**
   - **(i) macOS exec-TOCTOU:** Architect chose freeze-build-under-lock approach + documented the residual TOCTOU window (cargo build writes new binary bytes to the same inode between digest-check and execveat call). Mandatory operational constraint recorded: "no concurrent `cargo build` during an active migration window." This constraint is a deliberate architectural trade-off, not an oversight; POLICY 22 ratification must explicitly acknowledge this residual window.
   - **(ii) APFS directory-fsync durability:** Treated as best-effort in v1.3. The precise durability guarantee on darwin-arm64 APFS has not been empirically measured in the vsdd-factory test environment. An empirical darwin-arm64 durability test (power-fail simulation or equivalent) is owed before the APFS code path can be declared production-grade. Confirm darwin-arm64 CI runner availability (GitHub Actions macOS-14 or equivalent) before scheduling.

8. **Pipeline state:** Remains PAUSED. OWED items #2 (907-file input-hash sweep) and #3 (ADR-052↔BC circular input-hash re-settle) still open. Cluster-5 TDD BLOCKED until POLICY 22 ratification. NEXT = 4th cross-vendor Codex re-review of ADR-052 v1.3 → HUMAN POLICY 22 ratification (carrying the 2 sign-off items above).

### Summary Table

| Aspect | Value |
|--------|-------|
| ADR-052 | v1.2→v1.3 (full atomic-publication redesign, 11 findings closed) |
| BC-1.18.011 | v1.2→v1.3 |
| BC-1.18.010 | v1.4→v1.5 |
| error-taxonomy.md | v1.19→v1.20 |
| ARCH-INDEX | v4.29→v4.30 |
| BC-INDEX | v5.89→v5.90 |
| Input-hashes | ADR-052 aedcdc1 (PASS) / BC-1.18.011 6e82271 (cascade-stale, circular dep) / BC-1.18.010 de1520c (PASS) / error-taxonomy 5826e39 (PASS) |
| POLICY 22 | OPEN — 4th Codex re-review + human ratification owed (2 sign-off items: macOS exec-TOCTOU + APFS fsync durability) |
| pipeline | PAUSED (unchanged) |
| Owed | 907-file hash sweep (#2); ADR-052↔BC re-settle post-ratification (#3) |

### Canonical 6-column row (STATE.md Decisions Log)

| D-1220 | D-1220-ADR052-V13-RESEARCH-GROUNDED-REDESIGN-BURST | **ADR-052 v1.3 research-grounded redesign committed (state-manager, single-commit TD-VSDD-053; D-1220): 11 Codex findings from D-1218 closed via atomic-publication architecture redesign (CURRENT.json pointer swap; framed intent log; advisory flock; txn-record state machine; OPEN/DRAINING drain gate; execveat + digest binding; platform-branched APFS/Linux durability). BC-1.18.011 v1.2→v1.3; BC-1.18.010 v1.4→v1.5; error-taxonomy.md v1.19→v1.20. ARCH-INDEX v4.29→v4.30; BC-INDEX v5.89→v5.90. Input-hashes: ADR-052 aedcdc1 / BC-1.18.011 6e82271 (cascade-stale, circular dep) / BC-1.18.010 de1520c / error-taxonomy 5826e39. TWO items owed for POLICY 22 sign-off: (i) macOS exec-TOCTOU residual window (freeze-build-under-lock + no-concurrent-cargo-build constraint); (ii) APFS directory-fsync durability best-effort (empirical darwin-arm64 test owed). POLICY 22 ratification OPEN; cluster-5 TDD BLOCKED. pipeline: PAUSED. OWED: 907-file hash sweep; ADR-052↔BC re-settle.** | S-25.02 F4 | 2026-09-13 |

---

## D-1221: ADR-052 v1.4 Re-Hardening Fix Burst — In-House Adversary LOCAL Pass-1 (2C+5H+5M Closed)

**Decision ID:** D-1221
**Slug:** D-1221-ADR052-V14-LOCAL-ADV-PASS1-FIX-BURST
**Context:** S-25.02 F4 cluster-5 F1; ADR-052 cascade
**Date:** 2026-09-13
**Author:** state-manager

### Context

ADR-052 v1.3 was committed at D-1220. The review track for cluster-5 then transitioned from cross-vendor Codex (decision-support only, NON-STREAK) to the in-house LOCAL adversary cascade (BC-5.39.001, streak-counting). Per the checklist, a fresh-context in-house adversary was dispatched to review ADR-052 v1.3 + BC-1.18.011 v1.3 + BC-1.18.010 v1.5 + error-taxonomy.md v1.20. The adversary returned NOT-RATIFIABLE with 2 CRIT + 5 HIGH + 5 MED (12 findings total).

Architect closed the two CRIT findings (C1, C2) and HIGH findings (H1–H5) via ADR-052 v1.4. Product-owner closed the MED findings (M1–M5) via BC-1.18.011 v1.4, BC-1.18.010 v1.6, and error-taxonomy.md v1.21. State-manager closed M4 (BC-INDEX version-cell correction) in this burst.

### Findings Summary

| ID | Severity | Description | Resolution |
|----|----------|-------------|------------|
| C1 | CRIT | §Reader Integration step 2 used generation-path reads exclusively; during COMMITTING window files move one-by-one to canonical paths — generation path becomes ENOENT for already-moved files | Canonical-first/generation-fallback: try canonical first, fall back to gen-uuid/ if absent; rename(2) atomicity ensures ENOENT impossible for any file (BC-1.18.010 v1.6; ADR-052 §Decision 7c reader protocol) |
| C2 | CRIT | Allowlist fragmentation — multiple conflicting allow-entries across versions; no single authoritative entry | Normalized to single authoritative allow-entry in ADR-052 §Decision 3 (ADR-052 v1.4) |
| H1 | HIGH | Gate not reset on abort — txn→ABORTED but admission gate stayed OPEN/DRAINING indefinitely | Gate reset to OPEN on abort, mirroring gate close-on-commit (ADR-052 v1.4 §Decision 5) |
| H2 | HIGH | Durable reservation dir not pre-created — intent-log parent dir assumed to exist | Pre-create dir + fsync before first generation write (ADR-052 v1.4 §Decision 7b) |
| H3 | HIGH | Pre-pivot census gate skippable on resume from STAGING — EC-003 resume logic went straight to pointer swap | EC-003 resume-from-STAGING MUST re-run full census (step 3b) before proceeding to pointer swap; census is not skippable on resume (BC-1.18.011 v1.4 EC-003; ADR-052 §Decision 4e) |
| H4 | HIGH | STAGING state missing from spec enumeration | STAGING added as explicit state in ADR-052 v1.4 and BC-1.18.011 v1.4 Precondition 5 state machine |
| H5 | HIGH | Error codes not exhaustively enumerated — spec mentioned error classes without allocating distinct codes | 11 error-taxonomy codes enumerated in error-taxonomy.md v1.21 (E-MIG-001..E-MIG-011) |
| M1 | MED | Fencing described as BLOCKING all writes — should be advisory (audit-only) | Reframed as advisory fencing in ADR-052 v1.4 §Decision 5a; txn-record state is the authoritative block, flock is advisory |
| M2 | MED | PC3a cited "step 5" as the pointer-swap step — step 5 is the fingerprint recheck | Corrected to "step 6" (CURRENT.json pointer swap) in BC-1.18.011 v1.4 |
| M3 | MED | SDK Grounding Evidence `total_bcs:` used literal `2006` — violates POLICY 5 HEAD-reproducibility mandate | Structural form `total_bcs: <N>` per POLICY 5 (BC-1.18.011 v1.4 SDK Grounding Evidence) |
| M4 | MED | BC-INDEX v5.90 version cell for BC-1.18.010 v1.5 claimed "three-way parity extended to cover CURRENT.json generation pointer binding" — Invariant 2 was UNCHANGED per ADR-052; index↔body drift | Body-table v1.5 cell corrected: §Reader Integration only, Invariant 2 UNCHANGED from v1.4 (state-manager, this burst); frontmatter v5.90 changelog cell also corrected |
| M5 | MED | macOS exec-TOCTOU risk described in BLOCKING terms — residual window is accepted operational constraint, not a blocker | Reframed as accepted residual constraint with operational mitigation (no concurrent cargo build) in ADR-052 v1.4 §Decision 11 |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-1 = NOT-RATIFIABLE. Streak RESET to 0/3. Adversary pass-2 next (fresh-context, reads only prior pass Part A per Iron Law).

**Index advances this burst:**
- ARCH-INDEX v4.30 → v4.31 (ADR-052 row v1.3→v1.4)
- BC-INDEX v5.90 → v5.91 (BC-1.18.011 v1.3→v1.4; BC-1.18.010 v1.5→v1.6; M4 cell correction)

**Input-hashes (post-update):**
- ADR-052: PARTIAL (missing input `adv-cv-adr052-v13-closure-2026-09-13.md`; input-hash not updatable; pre-existing circular/missing-input condition)
- BC-1.18.011: `ddee535` (PASS — updated)
- BC-1.18.010: `37fc36a` (PASS — updated)
- error-taxonomy.md: `19a618d` (PASS — updated)
- BC-1.18.011 was cascade-stale (circular dep ADR-052↔BC pre-existing per D-1220; noted, not chased per OWED #3)

**Process-gap (adversary finding):** No CI lint / regression-detector asserts error-taxonomy Message-Format cell ↔ shipped Display parity; this class has recurred 6+ times (L-BB-D1210). Recorded as Drift Item [D-1221-PG-001] in STATE.md; anchored to S-12.13 (E-SHD Display Drift enforcement story, E-12). Lesson appended to lessons.md.

**POLICY 22 status:** OPEN (unchanged). Two sign-off items still owed: (i) macOS exec-TOCTOU residual window; (ii) APFS directory-fsync durability test. Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). Cluster-5 TDD BLOCKED.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1221 | D-1221-ADR052-V14-LOCAL-ADV-PASS1-FIX-BURST | **ADR-052 v1.4 re-hardening committed (state-manager, single-commit TD-VSDD-053; D-1221): in-house adversary LOCAL pass-1 = NOT-RATIFIABLE (2C+5H+5M); all 12 findings closed — C1 canonical-first/generation-fallback reader protocol; C2 normalized allowlist; H1 gate-reset-on-abort; H2 durable reservation dir; H3 pre-pivot census gate step 3b; H4 STAGING state; H5 11 error codes enumerated; M1 fencing reframed advisory; M2 pivot step 6 corrected; M3 structural total_bcs; M4 BC-INDEX v1.5 changelog cell corrected (§Reader Integration only, Invariant 2 UNCHANGED); M5 macOS reframed. BC-1.18.011 v1.3→v1.4; BC-1.18.010 v1.5→v1.6; error-taxonomy.md v1.20→v1.21. ARCH-INDEX v4.30→v4.31; BC-INDEX v5.90→v5.91. Input-hashes: ADR-052 PARTIAL (missing input, pre-existing) / BC-1.18.011 ddee535 (cascade-stale circular dep pre-existing per OWED #3) / BC-1.18.010 37fc36a (PASS) / error-taxonomy 19a618d (PASS). BC-5.39.001 LOCAL streak RESET 0/3. Process-gap [D-1221-PG-001] recorded: no CI lint asserts error-taxonomy Message-Format ↔ Display parity; anchored S-12.13 (E-12). POLICY 22 still OPEN. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

## D-1222: ADR-052 v1.5 + ADR-051 v1.14 Fix Burst — In-House Adversary LOCAL Pass-2 (1C+4H+7M Closed)

**Decision ID:** D-1222
**Slug:** D-1222-ADR052-V15-LOCAL-ADV-PASS2-FIX-BURST
**Context:** S-25.02 F4 cluster-5 F1; ADR-052 cascade
**Date:** 2026-09-13
**Author:** state-manager

### Context

ADR-052 v1.4 was committed at D-1221. The in-house LOCAL adversary cascade continued with pass-2 (fresh-context, reads only pass-1 Part A per Iron Law). The adversary returned NOT-RATIFIABLE with 1 CRIT + 4 HIGH + 7 MED (12 findings total).

Architect closed the CRIT finding (C-1) and HIGH findings (H-1..H-4) via ADR-052 v1.5. Product-owner closed the MED findings (M-1..M-6) and observation findings (L-1..L-5) plus the inputs-fix via BC-1.18.010 v1.7, BC-1.18.011 v1.5, and error-taxonomy.md v1.22. Architect also bumped ADR-051 to v1.14 for the M-2 coupling removal.

### Findings Summary

| ID | Severity | Description | Resolution |
|----|----------|-------------|------------|
| C-1 | CRIT | Reader protocol regression: v1.4 canonical-first/generation-fallback was incorrect for BC-INDEX.md (in-place overwrite target whose canonical path holds the old monolithic body until step 7's rename); canonical-first would read stale content for BC-INDEX.md during COMMITTING window | INVERTED to generation-first/canonical-fallback: try gen-<generation_id>/ FIRST, fall back to canonical if absent; fault-injection reader test mandate added (ADR-052 v1.5 §Decision 7c; BC-1.18.010 v1.7; BC-1.18.011 v1.5 Invariant 3) |
| H-1 | HIGH | ADR had dangling "see v1.2 §..." references; skipped-control inventory not self-contained | §Decision 6 audit-trail inlined (COMPLETED.json substituted for COMMITTED marker per v1.3); §Decision 8 skipped-control inventory table inlined; §Downstream Amendments 1–3 fully inlined from v1.2; no dangling cross-version references remain (ADR-052 v1.5) |
| H-2 | HIGH | mtime guard re-stat placed AFTER exec returns — fires only on exec failure; execve never returns on success | Re-stat moved to IMMEDIATELY BEFORE execve call; "Human sign-off required" updated with corrected residual-risk semantics: sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight (ADR-052 v1.5 §Decision 11) |
| H-3 | HIGH | Reservation UUID: prior design used in-process shared state for Pre/Post hook pair correlation, fragile across crash boundaries | PreToolUse creates `<tool_use_id>.reservation` using stable harness tool-invocation ID; PostToolUse removes it by same tool_use_id without shared in-process state; test mandates added: Pre-creates/Post-removes, stale-PID cleanup (ADR-052 v1.5 §Decision 7b) |
| H-4 | HIGH | Load-bearing version pins (e.g., "ADR-052 v1.4 EXCEPTION") throughout ADR and BCs — would require re-amendment on every version bump | Version pins removed; CLAUDE.md amendment text updated to "ADR-052 EXCEPTION" (stable); all BC cross-refs updated to stable §Decision N form (ADR-052 v1.5; BC-1.18.010 v1.7; BC-1.18.011 v1.5) |
| M-1 | MED | ADR-052 §Decision 7c traceability row missing from BC-1.18.011 Architecture Anchors | ADR-052 §Decision 7c added to BC-1.18.011 Architecture Anchors (BC-1.18.011 v1.5) |
| M-2 | MED | ADR-051 §Decision 10 item 6: "at the SAME F4 activation moment mechanism A's own backfill runs" — incorrect coupling; B2 migration activates independently | §Decision 10 item 6 corrected: B2 migration activates "as part of this SAME one-time B2 migration operation (independently of mechanism A's activation schedule — per BC-1.18.011 Precondition 4 and ADR-052 §Decision 1)"; ADR-051 bumped v1.13→v1.14 (ADR-051 v1.14) |
| M-3 | MED | E-MAINTENANCE referenced as plain "E-MAINTENANCE" throughout — error-taxonomy.md uses "E-MAINTENANCE-001" as the canonical code | Corrected to E-MAINTENANCE-001 throughout ADR-052 v1.5; error-taxonomy.md v1.22 header updated to acknowledge MIG + MAINTENANCE categories |
| M-4 | MED | Stale reader instruction in §Downstream BC-1.18.010 §Reader Integration (v1.3 "use gen-uuid/" and v1.4 canonical-first both present) | Stale instructions deleted; single correct generation-first/canonical-fallback instruction kept (BC-1.18.010 v1.7; ADR-052 v1.5 §Downstream amendments) |
| M-6 | MED | Bash admission reservation undocumented in §5a — admitted Bash mutations creating reservations not explicitly stated | Explicit text added to §5a: admitted Bash mutations with write effect create `<tool_use_id>.reservation`; §5c classifier determines write-effect; quiescence waits for all Bash reservations; test mandate added (ADR-052 v1.5 §Decision 5a) |
| L-1..L-5 | LOW | Various observation-class findings: positive test cases for validate_write_target(), mechanism-A config-driven wildcard, drain-timeout liveness note, EC-003 step citation, error-taxonomy header audit | Addressed in ADR-052 v1.5 and BC-1.18.011 v1.5 as documented above |
| inputs-fix | INFO | Non-existent file `adv-cv-adr052-v13-closure-2026-09-13.md` listed in ADR-052 inputs: | Removed from inputs: field; compute-input-hash --update run (new hash 080d460) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-2 = NOT-RATIFIABLE. Streak REMAINS 0/3. Adversary pass-3 next (fresh-context, reads only pass-2 Part A per Iron Law).

**Index advances this burst:**
- ARCH-INDEX v4.31 → v4.32 (ADR-052 row v1.4→v1.5; ADR-051 row v1.13→v1.14; SS-01 shard_manager.rs added)
- BC-INDEX v5.91 → v5.92 (BC-1.18.010 v1.6→v1.7; BC-1.18.011 v1.4→v1.5)

**Input-hashes (post-update):**
- ADR-052: `080d460` (PASS — updated; stale b607e57 was cascade-stale from prior inputs-fix; new hash reflects inputs after removing non-existent file)
- BC-1.18.010: `9c03116` (PASS — updated)
- BC-1.18.011: `5da155e` (PASS — updated; cascade-stale on circular dep ADR-052↔BC pre-existing per OWED #3 — noted, not chased)
- error-taxonomy.md: `c86c32c` (PASS — updated)
- ADR-051: no inputs: field — skipped (confirmed)

**Drift Item recorded (production-grade — MUST resolve before POLICY 22 ratification):** `[D-1222-DRIFT-001]` — prd.md §5.1 does NOT enumerate the MIG + MAINTENANCE error categories that now exist in error-taxonomy.md v1.22. Recorded with concrete follow-up: cluster-5 PRD sync (product-owner, an E-12 story; target before POLICY 22 ratification). This MUST be resolved before POLICY 22 ratification gate.

**Process-gap [D-1221-PG-001] status:** UNCHANGED OPEN. Adversary pass-2 re-flagged same class (L-6): MIG error codes (E-MIG-001..011, 11 codes added v1.21) inherit the same Display-parity CI lint risk as E-SHD codes. No additional action required this burst — the existing [D-1221-PG-001] + S-12.13 anchor already covers the whole error-taxonomy class. Confirmed still recorded.

**POLICY 22 status:** OPEN (unchanged). Two sign-off items still owed with CORRECTED semantics:
- (i) macOS exec-TOCTOU residual window — CORRECTED: this is the sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight (not the broader digest-check-to-exec window as previously described in v1.4)
- (ii) APFS directory-fsync durability darwin-arm64 test — unchanged
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). Cluster-5 TDD BLOCKED. prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1222 | D-1222-ADR052-V15-LOCAL-ADV-PASS2-FIX-BURST | **ADR-052 v1.5 + ADR-051 v1.14 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1222): in-house adversary LOCAL pass-2 = NOT-RATIFIABLE (1C+4H+7M); all 12 findings closed — C-1 reader protocol INVERTED generation-first/canonical-fallback (v1.4 canonical-first was regression for BC-INDEX.md in-place overwrite target); H-1 self-contained skipped-control inventory inlined + dangling refs removed; H-2 mtime re-stat immediately pre-execve (corrected residual-risk: sub-instruction stat→execve gap); H-3 tool_use_id reservations (stable harness ID, no in-process shared state); H-4 version pins removed (stable §Decision N form); M-1 ADR-052 traceability row +BC-1.18.011 Architecture Anchors; M-2 ADR-051 §D10 item 6 activation-coupling removed (ADR-051 v1.13→v1.14); M-3 E-MAINTENANCE-001 corrected throughout; M-4 stale reader instruction removed from §Downstream BC-1.18.010; M-6 Bash reservation documented §5a; L-1..L-5+inputs-fix. BC-1.18.010 v1.6→v1.7; BC-1.18.011 v1.4→v1.5; error-taxonomy.md v1.21→v1.22. ARCH-INDEX v4.31→v4.32 (SS-01 shard_manager.rs added M-5); BC-INDEX v5.91→v5.92. Input-hashes: ADR-052 080d460 (PASS) / BC-1.18.010 9c03116 (PASS) / BC-1.18.011 5da155e (cascade-stale circular dep pre-existing OWED #3) / error-taxonomy c86c32c (PASS) / ADR-051 no-inputs. [D-1222-DRIFT-001] NEW drift item: prd.md §5.1 does NOT enumerate MIG+MAINTENANCE error categories — owed before POLICY 22 ratification (cluster-5 PRD sync, E-12). [D-1221-PG-001] CONFIRMED STILL OPEN: MIG codes inherit same Display-parity risk (L-6 adversary pass-2 re-flag); anchor S-12.13 unchanged. BC-5.39.001 LOCAL streak 0/3 (pass-3 next). POLICY 22 OPEN — TWO sign-off items with CORRECTED semantics: (i) macOS exec-TOCTOU sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight; (ii) APFS darwin-arm64 durability test. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

## D-1223: ADR-052 v1.6 Deep-Consolidated Fix Burst — In-House Adversary LOCAL Pass-3 (1C+4H Closed)

**Decision ID:** D-1223
**Slug:** D-1223-ADR052-V16-LOCAL-ADV-PASS3-FIX-BURST
**Context:** S-25.02 F4 cluster-5 F1; ADR-052 cascade
**Date:** 2026-09-13
**Author:** state-manager

### Context

ADR-052 v1.5 was committed at D-1222. The in-house LOCAL adversary cascade continued with pass-3 (fresh-context, reads only pass-2 Part A per Iron Law). The adversary returned NOT-RATIFIABLE with 1 CRIT + 4 HIGH + 5 MED + 2 LOW (12 findings total). The pass-3 adversary review is now persisted as `adv-local-adr052-pass3.md` (closing finding H-3).

Architect closed the CRIT finding (C-1) and HIGH findings (H-1..H-4) via ADR-052 v1.6, a deep-consolidated fix addressing all findings at prose + code sample + predicate level. This is the deepest-level fix burst in the ADR-052 cascade — the census gate tautology (C-1) required redesigning both PC1 and PC2 at a structural level.

### Findings Summary

| ID | Severity | Description | Resolution |
|----|----------|-------------|------------|
| C-1 | CRIT | Step 3b "content-preservation (PC1)" was a self-referential tautology: `expected_post_hash = sha256(staging_file)` then `verify sha256(staged)==expected_post_hash` — trivially always TRUE regardless of content. BC-1.18.011 PC1 byte-for-byte reconstruction and PC2/PC2a exactly-one-shard census NOT enforced; EC-001 (dup-one/drop-one, same count) would pass the pivot. Governance-integrity gate inert. | PC1 rewritten: reconstruct concatenation from intent log, compare SHA-256 vs `source_sha256` from txn record (NOT vs a re-hash of the staging file itself). PC2 rewritten as per-ID set check: each ID in EXACTLY ONE shard, ZERO in retained body; count comparison alone insufficient — EC-001 dup+drop case must now abort with CENSUS_MISMATCH_ABORT. ADR-052 v1.6 §Decision 3 (PC1+PC2 step 3b). |
| H-1 | HIGH | §4e resume table keys on `CURRENT.json status:staging`, which is never written (CURRENT.json first written at step-6 pivot); STAGING crash matches no row. | §4e table re-keyed on txn-record state discriminator (STAGING/COMMITTING/COMPLETED/ABORTED) per BC-1.18.011 Invariant 3; the impossible "CURRENT.json status:staging" row deleted. ADR-052 v1.6 §4e. |
| H-2 | HIGH | Crash between `completed.json` write and gate→OPEN flip leaves gate permanently LOCKED; ALREADY_MIGRATED fast-path never acquires flock so never reconciles. | Branch-2 ALREADY_MIGRATED path now: acquire LOCK_EX; if gate≠OPEN and `completed.json` present with no active txn — flip→OPEN; fault-injection test mandate added. Terminal-no-op path reconciles stale gate before exit 0. ADR-052 v1.6 §4 Branch-2. |
| H-3 | HIGH | ADR body cited non-existent `adv-cv-adr052-v13-closure-2026-09-13.md`; no source file for in-house passes (passes not persisted). | `adv-local-adr052-pass3.md` persisted at `cycles/v1.0-brownfield-backfill/` (this burst). ADR `inputs[]` updated: `adv-local-adr052-pass3.md` added; non-existent `adv-cv-adr052-v13-closure-2026-09-13.md` removed. Source/Origin section finding-count corrected to 1C+4H+5M+2L. |
| H-4 | HIGH | §Downstream Amendments 7/8/9 deferred exact text to "next burst" (placeholder). | §Downstream Amendments 7/8/9 fully inlined with exact replacement text mirroring BC-1.18.011 v1.5 (CURRENT.json pointer swap as commit-point; intent log recovery; `completed.json` permanent terminal record). ADR-052 v1.6 §Downstream. |
| M-1 | MED | Admission discriminator ambiguity: §5a described gate_state as the only discriminator; txn-record state parity not stated. | §5a amended: admission checks BOTH gate_state AND txn-record state; gate_state is the durable proxy; either gate≠OPEN OR active txn (STAGING/COMMITTING) → E-MAINTENANCE-001. |
| M-2 | MED | macOS `verify_and_exec_binary` code sample omits pre-execve mtime re-stat (prose-code gap from H-2 v1.4 closure — prose had it, code sample did not). | Code sample updated: mtime re-stat added immediately before `exec_by_pathname` call. |
| M-3 | MED | Census abort triple-coded: `E-SHD-005` vs `CENSUS_MISMATCH_ABORT` vs `CONTENT_PRESERVATION_ABORT` — inconsistent alignment across steps and predicates. | Aligned: CONTENT_PRESERVATION_ABORT for PC1 failures; CENSUS_MISMATCH_ABORT for PC2 (ID-set) and E-SHD-005 (boundary) failures; binary surfaces process exit codes, not HookResult. |
| M-4 | MED | CLAUDE.md amendment text used `*/` wildcard for append-log paths — not human-auditable; 4 exact paths not enumerated. | CLAUDE.md amendment amended: `*/` wildcard replaced with 4 exact append-log paths (`decision-log.md`, `burst-log.md`, `lessons.md`, `session-checkpoints.md` in `v1.0-brownfield-backfill`). |
| M-5 | MED | Stale-reservation GC only at recovery startup — not at every drain. | Stale-reservation GC moved to start of EVERY drain (step 1 of drain procedure), not only recovery startup. |
| L-1 | LOW | H1 title stale "(v1.4 Fix-Burst)" version pin. | H1 title version pin stripped. |
| L-2 | LOW | Casing inconsistency: COMPLETED.json vs completed.json throughout. | All path-bearing references aligned to lowercase `completed.json`. |
| process-gap | PROCESS | BC→ADR "verified at step 3b" cross-ref never mechanically checked (same class as D-1221-PG-001 / S-12.13). | Existing [D-1221-PG-001] + S-12.13 anchor already covers this class. No new item required. |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-3 = NOT-RATIFIABLE. Streak REMAINS 0/3. ADR-052 v1.6 fix burst closes all 12 findings. Adversary pass-4 next (fresh-context, reads only pass-3 Part A per Iron Law).

**HARD STOP gate (per human direction):** if adversary pass-4 does NOT drop below ~3 CRIT+HIGH total, orchestrator MUST escalate to human for a scope/expertise decision rather than looping.

**Index advances this burst:**
- ARCH-INDEX v4.32 → v4.33 (ADR-052 row v1.5→v1.6; summary: census gate now byte-for-byte + ID-set enumeration; §4e re-keyed on txn state; completion-crash gate reconciliation; provenance fixed; Amendments 7/8/9 inlined)
- BC-INDEX UNCHANGED (BC-1.18.010 v1.7 / BC-1.18.011 v1.5 — no BC body changes this burst; architect confirmed)

**Input-hashes (post-update):**
- ADR-052: `0ae730c` (PASS — updated; now includes adv-local-adr052-pass3.md in inputs; prior 080d460 was without this file)
- error-taxonomy.md: `c86c32c` (PASS — already current; no changes this burst)
- BC-1.18.010: `9c03116` (PASS — unchanged from D-1222; no BC changes this burst)
- BC-1.18.011: `5da155e` (cascade-stale circular dep pre-existing OWED #3 — noted, not chased)

**POLICY 22 status:** OPEN. Sign-off items NOW FOUR (expanded from two per D-1222 CORRECTED semantics; H-4 + M-4 resolutions surface two additional items):
- (i) macOS exec-TOCTOU residual window — sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight
- (ii) APFS directory-fsync durability darwin-arm64 test
- (iii) CLAUDE.md amendment — 4 exact append-log paths (not wildcard; human must review and approve)
- (iv) 4 dispatcher-guard amendments at cluster-5 activation (architectural sign-off)
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). Cluster-5 TDD BLOCKED. prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1223 | D-1223-ADR052-V16-LOCAL-ADV-PASS3-FIX-BURST | **ADR-052 v1.6 deep-consolidated fix burst committed (state-manager, single-commit TD-VSDD-053; D-1223): in-house adversary LOCAL pass-3 = NOT-RATIFIABLE (1C+4H+5M+2L); all 12 findings closed — C-1 census gate tautology FIXED: PC1 now reconstruct+compare vs source_sha256 (not self-hash); PC2 now per-ID exactly-one-shard set enumeration (EC-001 dup+drop caught); H-1 §4e re-keyed on txn-record state discriminator (STAGING/COMMITTING/COMPLETED/ABORTED; impossible CURRENT.json-status:staging row deleted); H-2 ALREADY_MIGRATED terminal-path acquires LOCK_EX + reconciles stale gate before exit 0; H-3 adv-local-adr052-pass3.md persisted + inputs[] updated; H-4 §Downstream Amendments 7/8/9 fully inlined; M-1 admission gate_state+txn-state parity; M-2 macOS code sample pre-execve mtime re-stat; M-3 census/preservation abort code alignment; M-4 CLAUDE.md wildcard→4 exact append-log paths; M-5 stale-reservation GC at every drain; L-1 H1 title pin stripped; L-2 completed.json casing unified. ADR-052 input-hash 080d460→0ae730c (adv-local-adr052-pass3.md added). error-taxonomy.md v1.22→v1.23 (trigger alignment). ARCH-INDEX v4.32→v4.33. BC-INDEX UNCHANGED v5.92. BC-5.39.001 LOCAL streak 0/3 (adversary pass-4 next; HARD STOP if pass-4 ≥3 CRIT+HIGH). POLICY 22 OPEN — FOUR sign-off items: (i) macOS exec-TOCTOU sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight; (ii) APFS darwin-arm64 durability test; (iii) CLAUDE.md amendment 4 exact append-log paths; (iv) 4 dispatcher-guard amendments. [D-1222-DRIFT-001] prd.md §5.1 sync OWED pre-ratification (unchanged). OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

---

**Decision ID:** D-1224
**Slug:** D-1224-ADR052-V17-LOCAL-ADV-PASS4-FIX-BURST
**Context:** S-25.02 F4 cluster-5 F1; ADR-052 cascade
**Date:** 2026-09-13
**Author:** state-manager

### Context

ADR-052 v1.6 was committed at D-1223. The in-house LOCAL adversary cascade continued with pass-4 (fresh-context, reads only pass-3 Part A per Iron Law). The adversary returned NOT-RATIFIABLE with 2 HIGH + 6 MED + 2 OBS (10 findings total — F1,F2 HIGH; F3,F4,F5,F6,F7,F8 MED; F9,F10 OBS).

Architect closed the HIGH findings (F1,F2) and architect-owned MED/OBS findings (F4,F5,F8,F9,F10) via ADR-052 v1.7. Product-owner closed BC-owned MED findings (F3,F6,F7) via BC-1.18.011 v1.6 and BC-1.18.010 v1.8. Formal-verifier propagated VP-133 description update (E-SHD-005 rescoping) to VP-INDEX v3.20, verification-architecture.md v1.34, and verification-coverage-matrix.md v1.32 per POLICY 9.

### Findings Summary (pass-4)

| ID | Severity | Description | Resolution |
|----|----------|-------------|------------|
| F1 | HIGH | PC1 census gate was unsatisfiable: whole-concat SHA against source_sha256 cannot match because staged lean body adds §Subsystem Shard Manifest section; content is reordered | Introduced source_body_row_sha256 field in txn record (SHA-256 of BC-X.YY.NNN table rows in canonical BC-ID sort order, excl. headers); PC1 rewritten as structured-equivalence (ADR-052 v1.7 §Decision 7c) |
| F2 | HIGH | E-SHD-005 erroneously referenced in migration binary process-exit-code contexts; E-SHD-005 is a steady-state-gate HookResult (BC-1.18.006/BC-1.18.010), not a migration process exit code | E-SHD-005 removed from all migration-binary contexts; CENSUS_MISMATCH_ABORT/CONTENT_PRESERVATION_ABORT are the correct process exit terms; BC-1.18.011 updated (v1.6); VP-133 description updated per POLICY 9 (ADR-052 v1.7, BC-1.18.011 v1.6, VP-INDEX v3.20) |
| F3 | MED (BC-owned) | completed.json casing inconsistency in BC-1.18.010 and BC-1.18.011 | completed.json casing sweep applied throughout BC-1.18.010 v1.8 and BC-1.18.011 v1.6 |
| F4 | MED | Completion-crash left gate permanently LOCKED: crash between completed.json write and gate→OPEN flip blocked all writes until binary re-invocation | PreToolUse self-heals stuck-LOCKED gate: if gate=LOCKED AND completed.json present AND no active txn, PreToolUse acquires LOCK_EX and flips gate→OPEN; fault-injection test mandate added (ADR-052 v1.7 §Decision 5a) |
| F5 | MED | Dangling §Context ref cited "v1.2 §Context" which no longer exists in-file | Removed dangling cross-reference; replaced with forward references to §Rationale/§Decision 5a (ADR-052 v1.7 §Context) |
| F6 | MED (BC-owned) | BC-1.18.011 Invariant 1/Precondition 2 claimed no new crash-atomicity machinery needed, contradicting ADR-052 §Decision 7's framed intent log + flock protocol | Invariant 1/Precondition 2 explicitly cites ADR-052 §Decision 7 crash-atomicity machinery (BC-1.18.011 v1.6) |
| F7 | MED (BC-owned) | BC-1.18.011 SDK Grounding cited write_indeterminate_marker as atomic primitive; write_indeterminate_marker does NOT provide crash-atomicity | write_indeterminate_marker dropped; Invariant 1 grounded against BC-1.18.006 shard_manager.rs shipped primitive (BC-1.18.011 v1.6) |
| F8 | MED | APFS darwin-arm64 durability test was a post-ratification deliverable, not a ratification gate | Test ELEVATED to RATIFICATION PREREQUISITE: ADR MUST NOT be ratified safe on macOS until test completes; §Decision 11 POLICY 22 block updated (ADR-052 v1.7 §Decision 7d) |
| F9 | OBS | Stale BC-Impact v1.3-handoff row said "step 5" for pointer swap; step 6 is correct per v1.4 | Corrected to "step 6" (step 5 = fingerprint recheck; step 6 = CURRENT.json pointer swap) (ADR-052 v1.7 §BC-Impact table) |
| F10 | OBS | PID-reuse hazard: stale-reservation GC used PID alone; OS PID reuse causes false-negative | (pid, process_start_time) tuple in reservation files; GC compares both; PID match + start_time mismatch = PID reuse → reclaim (ADR-052 v1.7 §Decision 5a) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-4 = NOT-RATIFIABLE. Streak REMAINS 0/3. Adversary pass-5 next (fresh-context, reads only pass-4 Part A per Iron Law).

**TRAJECTORY (CRIT+HIGH):** 7→5→5→2 — plateau BROKEN. Pass-4 is the first pass that dropped below the 5-finding plateau set by passes 2 and 3.

**Index advances this burst:**
- ARCH-INDEX v4.33 → v4.34 (ADR-052 row v1.6→v1.7; VP-INDEX/BC-INDEX advances co-noted)
- BC-INDEX v5.92 → v5.93 (BC-1.18.011 v1.5→v1.6; BC-1.18.010 v1.7→v1.8)
- VP-INDEX v3.19 → v3.20 (VP-133 description E-SHD-005 rescoping propagated per POLICY 9)
- verification-architecture.md v1.33 → v1.34 (POLICY 9 VP-133 description propagation)
- verification-coverage-matrix.md v1.31 → v1.32 (POLICY 9 VP-133 description propagation)

**Input-hashes (post-update):**
- ADR-052: `5d73495` (PASS — updated)
- BC-1.18.011: `4dfb5fc` (circular-stale OWED #3 — noted, not chased)
- BC-1.18.010: `02f93ec` (circular-stale OWED #3 — noted, not chased)
- error-taxonomy.md: `a64f756` (PASS — already current)
- verification-architecture.md: `7e30fa6` (PASS — updated; VP-INDEX.md is input)
- verification-coverage-matrix.md: `7e30fa6` (PASS — updated; VP-INDEX.md is input)
- VP-INDEX: no inputs: field — skipped

**Drift item recorded [D-1224-DRIFT-001]:** E-SHD-005 now has NO dedicated VP leg (scoped to steady-state gate; VP-133 rescoped to process exit codes). New steady-state-gate VP needed — flagged for orchestrator routing under E-12. POLICY 1 append-only, no VP renumbered or retired.

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1221-PG-001] taxonomy Display-drift CI lint (S-12.13): UNCHANGED OPEN.
- [D-1224-DRIFT-001] NEW: E-SHD-005 VP-leg gap (no dedicated VP; E-12 routing needed).

**STATE.md VP count reconciliation:** Stale "26 VPs" citation corrected to 141 to match VP-INDEX.md total_vps (authoritative source).

**POLICY 22 status:** OPEN with 2 sign-off items (F8 elevates APFS test to ratification prerequisite):
- (i) macOS exec-TOCTOU residual window: sub-instruction stat→execve gap + no-concurrent-cargo-build pre-flight (unchanged).
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification (including F8 APFS prerequisite).

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1224 | D-1224-ADR052-V17-LOCAL-ADV-PASS4-FIX-BURST | **ADR-052 v1.7 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1224): in-house adversary LOCAL pass-4 = NOT-RATIFIABLE (2H F1,F2 + F3-F10); all 10 findings closed — F1 PC1 census gate restructured: per-BC-row structured-equivalence via source_body_row_sha256 (staged lean body adds §Subsystem Shard Manifest; whole-concat SHA vs source_sha256 unsatisfiable); F2 E-SHD-005 rescoped to STEADY-STATE native admission gate (BC-1.18.006/BC-1.18.010 HookResult; migration-binary contexts corrected to CENSUS_MISMATCH_ABORT/CONTENT_PRESERVATION_ABORT); F4 PreToolUse self-heals stuck-LOCKED gate (completed.json present + no active txn → LOCK_EX + gate→OPEN + test mandate); F5 dangling §Context ref inlined; F8 APFS durability test ELEVATED to RATIFICATION PREREQUISITE; F9 stale BC-Impact step-5 row corrected; F10 PID-reuse tuple (pid, process_start_time); F3/F6/F7 BC-owned routed PO (completed.json casing sweep; Invariant 1/PC2 crash-atomicity citation; SDK Grounding drop write_indeterminate_marker). BC-1.18.011 v1.5→v1.6; BC-1.18.010 v1.7→v1.8. ARCH-INDEX v4.33→v4.34; BC-INDEX v5.92→v5.93; VP-INDEX v3.19→v3.20 (VP-133 description E-SHD-005 rescoping per POLICY 9); verification-architecture.md v1.33→v1.34; verification-coverage-matrix.md v1.31→v1.32. Input-hashes: ADR-052 5d73495 (PASS) / BC-1.18.011 4dfb5fc (circular-stale OWED #3) / BC-1.18.010 02f93ec (circular-stale OWED #3) / error-taxonomy a64f756 (PASS) / verif-arch 7e30fa6 (PASS) / verif-matrix 7e30fa6 (PASS). [D-1224-DRIFT-001] NEW: E-SHD-005 VP-leg gap — no dedicated VP; new steady-state-gate VP needed (E-12 routing, POLICY 1 append-only). [D-1222-DRIFT-001] prd.md §5.1 sync UNCHANGED OPEN. [D-1221-PG-001] Display-parity UNCHANGED OPEN. STATE.md VP count reconciled 26→141. BC-5.39.001 LOCAL streak 0/3 (adversary pass-5 next). TRAJECTORY: CRIT+HIGH 7→5→5→2 (plateau broken). POLICY 22 2 sign-off items: (i) macOS exec-TOCTOU sub-instruction gap; (ii) F8 APFS test RATIFICATION PREREQUISITE. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

## D-1225: ADR-052 v1.8 + ADR-051 v1.15 sibling-sweep fix burst (adversary pass-5 RATIFY-WITH-CHANGES)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-5 LOCAL (in-house Claude, fresh context, reads only pass-4 Part A per Iron Law)
**Verdict:** RATIFY-WITH-CHANGES (upgraded from NOT-RATIFIABLE)

### Finding Table (pass-5)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| F-1 | HIGH | error-taxonomy.md missing CONTENT_PRESERVATION_ABORT entry | Added CONTENT_PRESERVATION_ABORT to error-taxonomy.md (v1.24→v1.25) |
| F-2 | HIGH | ADR §Files-to-Change PC1 shard_manager.rs+tests rows had stale v1.6 whole-concat PC1 wording | Updated to per-BC-row structured-equivalence model (ADR-052 v1.7→v1.8 §Files-to-Change PC1) |
| F-3 | MED | BC-1.18.011 §PC1 did not reflect ADR-052 v1.7/v1.8 structured-equivalence | BC-1.18.011 §PC1 updated to per-BC-row model (v1.6→v1.7) |
| F-4 | MED | ADR §4e recovery table missing STAGING+expired manifest row (EXPIRY_ABORT path) | EXPIRY_ABORT clean-abort row added to §4e table (ADR-052 v1.8) |
| Stray-1 | OBS (orch-caught) | ADR-051 §BC-Impact section used future tense for already-shipped items | Tense corrected in-place (ADR-051 v1.14→v1.15) |
| Stray-2 | OBS (orch-caught) | VP-132 description stale in 4 docs: VP-INDEX, verification-architecture.md, verification-coverage-matrix.md, VP-132.md | VP-132 description realigned across all 4 docs (VP-INDEX v3.21; verif-arch v1.35; verif-matrix v1.33; VP-132.md v1.1) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-5 = RATIFY-WITH-CHANGES. Streak REMAINS 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN). Adversary pass-6 next (fresh-context, reads only pass-5 Part A per Iron Law).

**TRAJECTORY (CRIT+HIGH):** 7→5→5→2→2 — verdict upgraded NOT-RATIFIABLE→RATIFY-WITH-CHANGES; trajectory floor holds at 2.

**NOTE:** ORCHESTRATOR ran two global sibling-sweep greps that caught VP-132.md and ADR-051 as stray sites. Per-agent sweeps missed these. [D-1225-PG-001] NEW sibling-sweep mechanical gate process-gap — this burst provides evidence strengthening the case for automated predicate-propagation lint.

**Index advances this burst:**
- ARCH-INDEX v4.35 → v4.36 (ADR-052 row v1.7→v1.8 body + ADR-051 row v1.14→v1.15)
- BC-INDEX v5.93 → v5.94 (BC-1.18.011 v1.6→v1.7)
- VP-INDEX v3.20 → v3.21 (VP-132 description realignment)
- verification-architecture.md v1.34 → v1.35 (VP-132 POLICY 9 propagation)
- verification-coverage-matrix.md v1.32 → v1.33 (VP-132 POLICY 9 propagation)
- VP-132.md v1.0 → v1.1 (formal-verifier; already bumped)

**Input-hashes (post-update):**
- BC-1.18.011: `07079ec` (PASS — circular-stale OWED #3 noted; ADR-052 input not refreshable until POLICY 22 ratification)
- error-taxonomy.md: `68425f5` (PASS — updated; includes BC-1.18.011 as input)
- verification-architecture.md: `ebd6abd` (PASS — updated after version bump)
- verification-coverage-matrix.md: `ebd6abd` (PASS — updated after version bump)
- VP-INDEX.md: no `inputs:` field — skipped
- VP-132.md: `0000000` placeholder, `inputs: []` genuinely empty — no-op
- ADR-052: architect ran --update per specialist dispatch
- ADR-051: no `inputs:` field — skipped

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1221-PG-001] taxonomy Display-drift CI lint: UNCHANGED OPEN. NOTE: This burst further motivates follow-up — orchestrator needed two manual greps to catch stray sites.
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: UNCHANGED OPEN (new steady-state-gate VP needed, E-12 routing).
- [D-1225-PG-001] NEW: sibling-sweep mechanical gate — ORCHESTRATOR needed two global greps to catch VP-132.md + ADR-051 stray sites that per-agent sweeps missed; confirms predicate-propagation lint gap.

**POLICY 22 status:** OPEN with 2 sign-off items (unchanged):
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1225 | D-1225-ADR052-V18-SIBLING-SWEEP-PASS5 | **ADR-052 v1.8 + ADR-051 v1.15 sibling-sweep fix burst committed (state-manager, single-commit TD-VSDD-053; D-1225): in-house adversary LOCAL pass-5 = RATIFY-WITH-CHANGES (2H F-1,F-2 + 2M F-3,F-4); all findings + 2 orch-caught stray sites closed — F-1 HIGH error-taxonomy.md CONTENT_PRESERVATION_ABORT (v1.24→v1.25); F-2 HIGH ADR §Files-to-Change PC1 updated to per-BC-row structured-equivalence (ADR-052 v1.7→v1.8); F-3 MED BC-1.18.011 §PC1 propagation (v1.6→v1.7); F-4 MED §4e STAGING+expired EXPIRY_ABORT row (ADR-052 v1.8); Stray-1 ADR-051 v1.14→v1.15 §BC-Impact tense (orch global grep); Stray-2 VP-132 realigned in 4 docs (VP-INDEX v3.21, verif-arch v1.35, verif-matrix v1.33, VP-132.md v1.1; orch global grep). ARCH-INDEX v4.35→v4.36; BC-INDEX v5.93→v5.94; VP-INDEX v3.20→v3.21; verification-architecture.md v1.34→v1.35; verification-coverage-matrix.md v1.32→v1.33. Input-hashes: BC-1.18.011 07079ec (PASS, circ-stale OWED #3) / error-taxonomy 68425f5 (PASS) / verif-arch ebd6abd (PASS) / verif-matrix ebd6abd (PASS). [D-1222-DRIFT-001] prd.md §5.1 UNCHANGED OPEN. [D-1221-PG-001] UNCHANGED OPEN. [D-1224-DRIFT-001] VP-leg UNCHANGED OPEN. [D-1225-PG-01] NEW: sibling-sweep mechanical gate process-gap. BC-5.39.001 LOCAL streak 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN; pass-6 next). TRAJECTORY: CRIT+HIGH 7→5→5→2→2 (verdict upgraded NOT-RATIFIABLE→RATIFY-WITH-CHANGES; plateau at 2). NOTE: ORCHESTRATOR global greps caught VP-132.md + ADR-051 that per-agent sweeps missed. POLICY 22 2 sign-off items: (i) macOS exec-TOCTOU; (ii) APFS test RATIFICATION PREREQUISITE. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

---

## D-1226: ADR-052 v1.9 fix burst (adversary pass-6 RATIFY-WITH-CHANGES)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-6 LOCAL (in-house Claude, fresh context, reads only pass-5 Part A per Iron Law)
**Verdict:** RATIFY-WITH-CHANGES

### Finding Table (pass-6)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| H1 | HIGH | Reservation GC used PID-liveness (unsound): per-event dispatcher could reclaim in-flight reservations, causing snapshot-mid-write data corruption | Changed GC criterion from PID-liveness to TTL-based (created_at + tool_use_id) in ADR-052 §Decision drain-GC (v1.8→v1.9) |
| H2 | HIGH | E-SHD-005 citation in ADR-052 §Decision 7c erroneously anchored to migration binary; E-SHD-005 is scoped exclusively to the steady-state native admission gate | E-SHD-005 re-anchored to steady-state gate (ADR-052 v1.9); VP-132.md sibling note + error-taxonomy.md updated |
| M1 | MED | VP-132.md sibling-VP note referenced E-SHD-005 as a VP-132 error code — VP-132 covers CONTENT_PRESERVATION_ABORT (process exit), not E-SHD-005 | VP-132.md v1.1→v1.2 sibling note updated (formal-verifier); VP-INDEX v3.21→v3.22 catalog row v1.2 annotation added |
| M2 | MED | ADR-052 handoff description used present-tense for already-completed migration handoff | Corrected to past-tense in ADR-052 §Handoff (v1.9) |
| M3 | MED | ADR-052 §Files-to-Change exit-1/exit-2 criterion was ambiguous between expected vs error | Clarified: exit-1 = expected (CENSUS_MISMATCH_ABORT), exit-2 = error (CONTENT_PRESERVATION_ABORT) per error-taxonomy.md v1.25→v1.26 (product-owner) |
| L1 | LOW | Provenance inconsistency in ADR-052 §Provenance section | Fixed provenance wording in ADR-052 v1.9 |
| L2 | LOW | §4e table row ordering issue | §4e reordered in ADR-052 v1.9 |
| §Files-to-Change straggler | OBS (orch-caught) | H1 TTL-based GC fix landed in §Decision section of ADR-052 v1.8 but was NOT propagated to §Files-to-Change shard_manager.rs row — 3rd recurrence of fix-lands-in-§Decision-not-§Files-to-Change class | §Files-to-Change shard_manager.rs row updated with TTL-based GC directive (ADR-052 v1.9; orchestrator global grep caught this; per-agent sweeps missed it) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-6 = RATIFY-WITH-CHANGES. Streak REMAINS 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN). Adversary pass-7 next (fresh-context, reads only pass-6 Part A per Iron Law).

**TRAJECTORY (CRIT+HIGH):** 7→5→5→2→2→2 — plateau continues; LENGTH=4 tail: →2→2→2→2 pending pass-7.

**§Files-to-Change straggler pattern (3rd recurrence):** Pass-5 caught F-1/F-2/F-3 in §Files-to-Change sites; pass-6 caught H2/M1 stray sites; this burst the §Files-to-Change shard_manager.rs row was not updated with the H1 TTL fix. All three recurrences required orchestrator global grep to catch — per-agent sweeps missed each. Motivates S-12.15 propagation-lint mechanical gate (E-12). Lesson recorded.

**NOTE:** error-taxonomy.md v1.25→v1.26 written by product-owner (H2 E-SHD-005 re-anchoring + M3 exit-code criterion). ADR-052 v1.9 written by architect (H1 TTL GC + M2 handoff past-tense + L1 provenance + L2 §4e + §Files-to-Change sweep). VP-132.md v1.2 written by formal-verifier (M1 sibling-VP note). sidecar-learning.md folded in per D-1207-precedent convention.

**Index advances this burst:**
- ARCH-INDEX v4.36 → v4.37 (ADR-052 row v1.8→v1.9 body amendment + v1.9 inline annotation)
- BC-INDEX v5.94 → UNCHANGED (BC-1.18.011/010 not modified this burst)
- VP-INDEX v3.21 → v3.22 (VP-132 v1.2 catalog row annotation)

**Input-hashes (post-check):**
- ADR-052: `156d100` (PASS — confirmed current by --check; §Files-to-Change edits already reflected)
- error-taxonomy.md: `68425f5` (PASS — confirmed current by --check; no change to tracked input files from H2/M3 edits)
- VP-132.md: `0000000` placeholder, `inputs: []` genuinely empty — no-op

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: UNCHANGED OPEN (new steady-state-gate VP needed, E-12 routing; the VP-132/VP-133 re-anchoring done this burst is necessary but not sufficient — a new VP for the steady-state gate remains owed).
- [D-1225-PG-001] sibling-sweep mechanical gate: ADDRESSED by S-12.15 propagation-lint story opened this burst.

**Follow-up story opened:** S-12.15 (predicate-propagation-lint gate, E-12 Engine Governance). Story_count 11→12.

**POLICY 22 status:** OPEN with 2 sign-off items (unchanged):
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1226 | D-1226-ADR052-V19-PASS6-DRAIN-GC-TTL | **ADR-052 v1.9 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1226): in-house adversary LOCAL pass-6 = RATIFY-WITH-CHANGES (H1+H2 HIGH + M1/M2/M3 MED + L1/L2 LOW + §Files-to-Change straggler orch-caught); all findings closed — H1 HIGH drain GC changed from unsound PID-liveness to TTL-based (created_at+tool_use_id) in ADR-052 v1.9 (snapshot-mid-write soundness fix); H2 HIGH E-SHD-005 re-anchored to steady-state gate (error-taxonomy.md v1.25→v1.26; ADR-052 v1.9); M1 MED VP-132.md v1.1→v1.2 sibling note (formal-verifier) + VP-INDEX v3.21→v3.22 catalog annotation; M2 MED handoff past-tense (ADR-052 v1.9); M3 MED exit-code criterion (error-taxonomy.md v1.26); L1/L2 (ADR-052 v1.9); §Files-to-Change straggler shard_manager.rs TTL-directive (3rd recurrence: orch global grep; per-agent sweeps missed). ARCH-INDEX v4.36→v4.37; BC-INDEX v5.94 UNCHANGED; VP-INDEX v3.21→v3.22. Input-hashes: ADR-052 156d100 (PASS) / error-taxonomy 68425f5 (PASS) / VP-132.md inputs:[] no-op. [D-1222-DRIFT-001] prd.md §5.1 UNCHANGED OPEN. [D-1224-DRIFT-001] VP-leg UNCHANGED OPEN. S-12.15 propagation-lint story OPENED (E-12; story_count 11→12). BC-5.39.001 LOCAL streak 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN; pass-7 next). TRAJECTORY: CRIT+HIGH 7→5→5→2→2→2 (plateau; LENGTH=4 tail →2→2→2→2). POLICY 22 2 sign-off items (i) macOS exec-TOCTOU; (ii) APFS test RATIFICATION PREREQUISITE — UNCHANGED. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F5 | 2026-09-13 |


---

## D-1227: ADR-052 v1.10 fix burst (adversary pass-7 RATIFY-WITH-CHANGES)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-7 LOCAL (in-house Claude, fresh context, reads only pass-6 Part A per Iron Law)
**Verdict:** RATIFY-WITH-CHANGES

### Finding Table (pass-7)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| HIGH-1 | HIGH | PreToolUse stale-gate self-heal scoped only to gate=LOCKED + completed.json present (post-COMPLETED crash window); symmetric post-ABORT window (txn=ABORTED, gate=LOCKED, no completed.json) and post-drain-timeout window (gate=DRAINING, no active txn) unreachable by v1.9 self-heal — both are genuine stuck states | Self-heal generalized to gate ∈ {LOCKED, DRAINING} AND no active txn, regardless of completed.json presence (ADR-052 v1.10); covers all three crash windows |
| MED-1 | MED | EXPIRY_ABORT trigger defined only for "expired" reservation; absent-reservation post-abort path had no defined trigger | EXPIRY_ABORT trigger widened to "expired OR absent" (ADR-052 v1.10) |
| MED-2 | MED | No operator runbook for TTL stall in DRAINING state | Operator runbook added: heuristic lower-bound for TTL stall detection + recovery procedure for gate stuck in DRAINING after drain timeout (ADR-052 v1.10) |
| LOW-1 | LOW | E-MAINTENANCE-001 listed in §Files-to-Change main table; should be in guard-layer sub-table (execution guard classification) | E-MAINTENANCE-001 moved to guard-layer sub-table (ADR-052 v1.10) |
| LOW-2 | LOW | §Handoff tables used present/future-directive tense for already-delivered MIG integration | §Handoff tables corrected to past-tense throughout (ADR-052 v1.10) |
| LOW-3 | LOW | error-taxonomy.md ALREADY_MIGRATED summary-line read "always TEMPORARY" + self-healing clause, incorrectly conflating permanent ALREADY_MIGRATED no-op with TTL-expired reservation resets | ALREADY_MIGRATED summary-line corrected to distinguish permanent terminal path from self-healing TTL-reset (error-taxonomy.md v1.26→v1.27) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-7 = RATIFY-WITH-CHANGES. Streak REMAINS 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN). Adversary pass-8 next (fresh-context, reads only pass-7 Part A per Iron Law). NOTE: adversary's stated RATIFIABLE bar was "close HIGH-1 + MED-1" — both closed this burst.

**TRAJECTORY (CRIT+HIGH):** 7→5→5→2→2→2→1 — converging; LENGTH=4 tail →2→2→2→1.

**Verified-sound list:** 12 items (incremented from pass-6's verified-sound list by HIGH-1 liveness generalization + MED-1 EXPIRY_ABORT widening, each of which was adversary-verified at pass-7 as genuinely sound).

**NOTE:** ADR-052 v1.10 written by architect (HIGH-1 stale-gate self-heal generalization + MED-1 EXPIRY_ABORT widened + MED-2 operator runbook + LOW-1 E-MAINTENANCE-001 sub-table + LOW-2 handoff past-tense). error-taxonomy.md v1.27 written by product-owner (LOW-3 ALREADY_MIGRATED summary-line correction). sidecar-learning.md folded in per D-1207-precedent convention. Same-burst PO handoff-note cleanup folded in.

**Index advances this burst:**
- ARCH-INDEX v4.37 → v4.38 (ADR-052 row v1.9→v1.10 body amendment)
- BC-INDEX v5.94 → UNCHANGED (BCs unchanged this burst)
- VP-INDEX v3.22 → UNCHANGED (VPs unchanged this burst)

**Input-hashes (post-check):**
- ADR-052: `156d100` (PASS — confirmed current by --check; inputs[] list unchanged, hash already current)
- error-taxonomy.md: `68425f5` (PASS — confirmed current by --check; hash already current)

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: UNCHANGED OPEN.
- [D-1225-PG-001] sibling-sweep → S-12.15: UNCHANGED OPEN.

**POLICY 22 status:** OPEN with 2 sign-off items (unchanged):
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1227 | D-1227-ADR052-V110-PASS7-STALE-GATE-SELF-HEAL-GENERALIZED | **ADR-052 v1.10 + error-taxonomy v1.26→v1.27 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1227): in-house adversary LOCAL pass-7 = RATIFY-WITH-CHANGES (1 HIGH + 2 MED + 3 LOW); all findings closed — HIGH-1 (GENUINE liveness fix) PreToolUse stale-gate self-heal generalized from post-COMPLETED-only to all no-active-txn stuck states: gate ∈ {LOCKED, DRAINING} AND no active txn triggers self-heal regardless of completed.json; closes post-ABORT (txn=ABORTED, gate=LOCKED, no completed.json) and post-drain-timeout (gate=DRAINING, no active txn) crash windows; MED-1 EXPIRY_ABORT trigger widened (expired OR absent); MED-2 operator runbook for TTL stall; LOW-1 E-MAINTENANCE-001 guard-layer sub-table; LOW-2 §Handoff past-tense; LOW-3 error-taxonomy.md v1.26→v1.27 ALREADY_MIGRATED summary-line corrected. ARCH-INDEX v4.37→v4.38; BC-INDEX v5.94 UNCHANGED; VP-INDEX v3.22 UNCHANGED. Input-hashes: ADR-052 156d100 (PASS) / error-taxonomy 68425f5 (PASS). NOTE: adversary's RATIFIABLE bar was close HIGH-1+MED-1 — both closed. [D-1222-DRIFT-001] prd.md §5.1 UNCHANGED OPEN. [D-1224-DRIFT-001] VP-leg UNCHANGED OPEN. BC-5.39.001 LOCAL streak 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN; pass-8 next toward 3-CLEAN). TRAJECTORY: CRIT+HIGH 7→5→5→2→2→2→1 (converging; LENGTH=4 tail →2→2→2→1; verified-sound list 12 items). POLICY 22 2 sign-off items (i) macOS exec-TOCTOU; (ii) APFS test RATIFICATION PREREQUISITE — UNCHANGED. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F5 | 2026-09-13 |

---

## D-1228: ADR-052 v1.11 fix burst (adversary pass-8 NOT-RATIFIABLE — HIGH-1 self-heal-vs-live-drain regression)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-8 LOCAL (in-house Claude, fresh context, reads only pass-7 Part A per Iron Law)
**Verdict:** NOT-RATIFIABLE

### Finding Table (pass-8)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| HIGH-1 | HIGH | REGRESSION from v1.10: generalized self-heal predicate (gate ∈ {LOCKED, DRAINING} AND no active txn) matched a live coordinator mid-drain — concurrent writer could reopen gate after coordinator state wiped, breaking writer exclusion | flock(exclusive.lock, LOCK_EX\|LOCK_NB) FIRST before reconcile: self-heal proceeds only if flock acquirable (no live coordinator); EWOULDBLOCK → E-MAINTENANCE-001, no gate flip; drain txn reorder: txn=STAGING written at step 2.5 BEFORE DRAINING flip (ADR-052 v1.11) |
| MED-1 | MED | §7c step 2a per-shard file-open was a racy file-exists-check-then-open (TOCTOU); if file disappears between check and open, ENOENT on open was unhandled | open-with-ENOENT-fallback: try canonical path; if ENOENT try generation-uuid/ path; if both ENOENT abort CONTENT_PRESERVATION_ABORT; §BC-Impact blockquote swept to match; propagated to BC-1.18.010 v1.9 + BC-1.18.011 v1.8 |
| MED-2 | MED | BC-1.18.010 provenance tables used present/future-directive tense for already-delivered content | BC-1.18.010 §BC-Impact provenance rows corrected to past-tense (ADR-052 v1.11) |
| LOW-1 | LOW | Residual "unchanged from v1.2" self-reference in ADR body | Stripped (ADR-052 v1.11) |
| LOW-2 | LOW | Unescaped pipe in table cell violated validate-table-cell-count | Table-cell pipe escaped with \\| (ADR-052 v1.11) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-8 = NOT-RATIFIABLE. Streak REMAINS 0/3 (NOT-RATIFIABLE ≠ CLEAN). Adversary pass-9 next (fresh-context, reads only pass-8 Part A per Iron Law).

**TRAJECTORY (CRIT+HIGH):** 7→5→5→2→2→2→1→1 — plateau at 1; HIGH-1 was a 2nd fix-induced concurrency-core regression (pass-4 C-1 reader-inversion was 1st).

**NOTE (concurrency regression pattern):** HIGH-1 was a REGRESSION introduced by v1.10's self-heal generalization. The v1.10 predicate correctly identified no-active-txn stuck states but did not guard against the live-coordinator case (drain not yet committed). Fixing via flock(LOCK_EX|LOCK_NB) achieves parity with the crash-recovery path. The orchestrator's global-grep sweeps caught: (a) a stale §BC-Impact reader blockquote carrying the pre-ENOENT-fallback wording (per-agent sweeps missed), and (b) the unescaped table-cell pipe (validate-table-cell-count fired). Recurring propagation-gap class per S-12.15.

**NOTE (lesson strengthening):** MED-1 confirms that prose 3-CLEAN review cannot close the concurrency state-machine surface — the v1.10 regression slipped through 7 passes of adversarial review before detection. Formal methods (Kani + fault-injection) remain the only complete closure path. This is the 2nd fix-induced concurrency-core regression; strengthens L-EDP1-XXX process-gap lesson with fresh evidence.

**Index advances this burst:**
- ARCH-INDEX v4.38 → v4.39 (ADR-052 row v1.10→v1.11 body amendment)
- BC-INDEX v5.94 → v5.95 (BC-1.18.010 v1.8→v1.9; BC-1.18.011 v1.7→v1.8)
- VP-INDEX v3.22 → UNCHANGED (VPs unchanged this burst)
- STORY-INDEX UNCHANGED

**Input-hashes (post-update):**
- ADR-052: `f4f12f6` (PASS — --check verified; inputs[] unchanged, hash current)
- BC-1.18.010: `6e2d3a3` (updated via --update)
- BC-1.18.011: `e027627` (updated via --update)
- error-taxonomy.md: `605cc2e` (updated via --update; v1.27→v1.28 exit-cell reconciled LOW-2)

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: UNCHANGED OPEN.
- [D-1225-PG-001] sibling-sweep → S-12.15: UNCHANGED OPEN.

**POLICY 22 status:** OPEN with 2 sign-off items (unchanged):
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep); OWED #3 (ADR-052↔BC circular re-settle). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1228 | D-1228-ADR052-V111-PASS8-FLOCK-SELF-HEAL-REGRESSION | **ADR-052 v1.11 + BC-1.18.010 v1.9 + BC-1.18.011 v1.8 + error-taxonomy v1.28 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1228): in-house adversary LOCAL pass-8 = NOT-RATIFIABLE (1 HIGH + 2 MED + 2 LOW); all findings closed — HIGH-1 REGRESSION: v1.10 generalized self-heal predicate matched live coordinator mid-drain → concurrent writer could reopen gate → writer-exclusion break; fixed via flock(exclusive.lock, LOCK_EX\|LOCK_NB) first-check before reconcile + EWOULDBLOCK→E-MAINTENANCE-001, no gate flip + drain txn=STAGING written at step 2.5 BEFORE DRAINING flip (reorder); MED-1 reader open-with-ENOENT-fallback (§7c step 2a TOCTOU) propagated to §BC-Impact blockquote + BC-1.18.010 v1.9 + BC-1.18.011 v1.8; MED-2 BC-1.18.010 provenance past-tense; LOW-1 residual v1.2 self-refs; LOW-2 table-cell pipe escaped. ARCH-INDEX v4.38→v4.39; BC-INDEX v5.94→v5.95; VP-INDEX v3.22 UNCHANGED. Input-hashes: ADR-052 f4f12f6 (PASS) / BC-1.18.010 6e2d3a3 (updated) / BC-1.18.011 e027627 (updated) / error-taxonomy 605cc2e (updated). NOTE: 2nd fix-induced concurrency-core regression (pass-4 C-1 reader-inversion was 1st); orchestrator global-grep sweeps caught stale reader blockquote + unescaped table-cell pipe that per-agent sweeps missed (S-12.15 propagation-gap). [D-1222-DRIFT-001] prd.md §5.1 UNCHANGED OPEN. [D-1224-DRIFT-001] VP-leg UNCHANGED OPEN. BC-5.39.001 LOCAL streak 0/3 (NOT-RATIFIABLE ≠ CLEAN; pass-9 next toward 3-CLEAN). TRAJECTORY: CRIT+HIGH 7→5→5→2→2→2→1→1 (plateau; tail LENGTH=4 →2→2→1→1). POLICY 22 2 sign-off items (i) macOS exec-TOCTOU; (ii) APFS test RATIFICATION PREREQUISITE — UNCHANGED. OWED #2+#3 unchanged. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F5 | 2026-09-13 |

---

## D-1229 — ADR-052 v1.12 + BC-INDEX F1 corrective re-transcription + error-taxonomy v1.29 fix burst (pass-9)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-9 LOCAL (in-house Claude, fresh context, reads only pass-8 Part A per Iron Law)
**Verdict:** NOT-RATIFIABLE

### Finding Table (pass-9)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| F1 | HIGH | BC-INDEX transcription error (state-manager): D-1228 state-manager mis-transcribed the reader direction in BC-1.18.010 v1.9 + BC-1.18.011 v1.8 INDEX catalog cells and v5.95 changelog entry as canonical-first + invented a both-ENOENT CONTENT_PRESERVATION_ABORT abort clause that appears in neither BC bodies nor ADR §Decision 7c | Corrective re-transcription: all three BC-INDEX locations corrected to actual generation-first/canonical-fallback (open gen-<uuid>/<file>; on ENOENT open canonical path; files move one-directionally gen→canonical via atomic rename(2); required file never absent from both paths during COMMITTING window — invented clause deleted); BC-INDEX v5.95→v5.96; NO BC version change |
| F2 | HIGH | §4e null-STAGING crash sub-state unhandled: v1.11 drain step 2.5 writes txn=STAGING with generation_id=null BEFORE snapshot (drain step 5) and generation UUID assignment (§7c step 1); crash between these leaves null-STAGING with no staged generation directory and null source hashes; EC-003 resume path unconditionally aborts CONTENT_PRESERVATION_ABORT on null source_body_row_sha256 — 3rd fix-induced concurrency/recovery regression in this cascade | §Decision 4e: partition STAGING row on generation_id presence: (a) generation_id=null→DISCARD (delete partial txn, gate→OPEN, EXPIRY_ABORT; MUST NOT census; MUST NOT compare null hashes); (b) generation_id=set AND staged dir present→existing EC-003 resume; fault-injection test mandate added §Decision 5a (ADR-052 v1.12) |
| F3 | MED | Residual/inverted reader wording in §Downstream + §BC Impact: (a) §Downstream blockquote existence-check language not open-with-ENOENT-fallback; (b) §BC Impact v1.4 Invariant-3 Applied cell had inverted canonical-first direction (known v1.4 regression); (c) stale BC-1.18.010 v1.8 provenance labels; (d) §BC Impact v1.5 C-1 Applied description still used existence-check language | §Downstream blockquote step 2: open(gen-<generation_id>/<file>); on ENOENT, open(canonical/<file>). §BC Impact v1.4 Invariant-3 Applied cell: corrected from canonical-first to generation-first/canonical-fallback. §BC Impact v1.5 C-1 Applied descriptions: open-with-ENOENT-fallback. Stale headers updated BC-1.18.010 v1.8→v1.9 (ADR-052 v1.12) |
| F4 | MED | §4e ABORTED disposition absent; txn-selection ambiguity when multiple txn-*.json exist; terminal-txn GC policy unspecified — unbounded accumulation in .factory/migration-state/ | §Decision 4e: explicit ABORTED row (treated as no active txn; fresh activation permitted); txn-selection disambiguation rule (match activation_id vs current manifest; highest created_at fallback); GC/archival policy (terminal records archived to .factory/migration-audit/txn-archive/ at next audit burst) (ADR-052 v1.12) |
| F5 | LOW | error-taxonomy.md v1.28 ALREADY_MIGRATED row Message Format cell: `COMPLETED.json` capitalized; canonical path is `.factory/migration-state/completed.json` (all lowercase); ADR-052 §Error Code Semantics ALREADY_MIGRATED row uses lowercase | error-taxonomy.md v1.28→v1.29: `COMPLETED.json` → `completed.json` in ALREADY_MIGRATED Message Format cell |
| F6 | LOW | §5c Branch 2 gate reconciliation lacked flock(exclusive.lock, LOCK_EX\|LOCK_NB) pre-check; §Decision 5a step 3.5 and crash-recovery paragraph both require flock-acquirable precondition; Branch 2 silently diverged creating inconsistent reconciliation protocol | §Decision 5c Branch 2: new pre-step 0 — attempt flock(exclusive.lock, LOCK_EX\|LOCK_NB); on EWOULDBLOCK exit 0 ALREADY_MIGRATED WITHOUT reconciling gate; on acquired proceed with gate reconciliation; parity with step 3.5 + crash-recovery path (ADR-052 v1.12) |

### Codification

**BC-5.39.001 LOCAL cascade:** pass-9 = NOT-RATIFIABLE. Streak REMAINS 0/3 (NOT-RATIFIABLE ≠ CLEAN). Adversary pass-10 next (fresh-context, reads only pass-9 Part A per Iron Law).

**TRAJECTORY REVERSED (CRIT+HIGH):** 7→5→5→2→2→2→1→1→2 — reversal at pass-9; 3rd fix-induced concurrency/recovery regression (F2 null-STAGING crash sub-state introduced by v1.11 drain step 2.5 ordering).

**NOTE (3rd fix-induced concurrency regression):** F2 represents the 3rd fix-induced concurrency/recovery regression in this cascade: (1st) pass-4 C-1 canonical-first reader inversion; (2nd) pass-8 flock self-heal generalization regression; (3rd) pass-9 null-STAGING crash sub-state from v1.11 drain step 2.5 ordering. The common pattern: each fix introduces a new subtle ordering or partitioning gap in the crash-recovery state machine. Prose-only adversarial review is measurably NOT converging the concurrency/recovery state machine surface — the same surface area keeps producing regressions. Human directed KEEP-GRINDING the prose 3-CLEAN loop for the 2nd time (acknowledging trajectory reversal + 3rd concurrency regression). Definitive fix path: formal model (TLA+/Kani) or implementation-phase fault-injection testing; prose review insufficient for state-machine convergence.

**NOTE (F1 BC-INDEX state-manager mis-transcription):** F1 was a state-manager error at D-1228: the BC-INDEX reader-protocol changelog cells described the INVERTED direction (canonical-first) and invented a both-ENOENT abort clause. The actual applied BC bodies + ADR §Decision 7c specify generation-first/canonical-fallback with the ENOENT-safe invariant (files never absent from both paths during COMMITTING window). Corrected here as corrective re-transcription (NO BC version change).

**Index advances this burst:**
- ARCH-INDEX v4.39 → v4.40 (ADR-052 row v1.11→v1.12 body amendment)
- BC-INDEX v5.95 → v5.96 (F1 corrective re-transcription; BC-1.18.010 v1.9 + BC-1.18.011 v1.8 cells corrected — NO BC body version change)
- VP-INDEX v3.22 → UNCHANGED
- STORY-INDEX UNCHANGED

**Input-hashes (post-check):**
- ADR-052: a9d309a (PASS — --check verified; architect set this at v1.12)
- error-taxonomy.md: PASS (--check verified; v1.29 casing fix)
- BC-1.18.010 body: UNCHANGED (cell-only correction in BC-INDEX; no BC body edit)
- BC-1.18.011 body: UNCHANGED (cell-only correction in BC-INDEX; no BC body edit)

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: UNCHANGED OPEN — owed before POLICY 22 ratification.
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: UNCHANGED OPEN.
- [D-1225-PG-001] sibling-sweep → S-12.15: UNCHANGED OPEN.

**POLICY 22 status:** OPEN with 2 sign-off items (unchanged):
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep). prd.md §5.1 MIG/MAINTENANCE sync [D-1222-DRIFT-001] owed before ratification.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1229 | D-1229-ADR052-V112-PASS9-NULL-STAGING-F1-BC-INDEX-TRANSCRIPTION | **ADR-052 v1.12 + BC-INDEX F1 corrective re-transcription + error-taxonomy v1.29 fix burst committed (state-manager, single-commit TD-VSDD-053; D-1229): in-house adversary LOCAL pass-9 = NOT-RATIFIABLE (2 HIGH + 2 MED + 2 LOW); all 6 findings closed — F1 HIGH BC-INDEX transcription [state-manager]: corrective re-transcription of BC-1.18.010 v1.9 + BC-1.18.011 v1.8 INDEX catalog cells and v5.95 changelog entry — inverted canonical-first + invented both-ENOENT CONTENT_PRESERVATION_ABORT abort clause deleted; corrected to actual generation-first/canonical-fallback per BC bodies + ADR §Decision 7c; BC-INDEX v5.95→v5.96. F2 HIGH §4e null-STAGING crash sub-state (v1.11 drain step 2.5 writes txn=STAGING with generation_id=null BEFORE snapshot; crash leaves null-STAGING with no staged gen dir; EC-003 census unconditionally aborts CONTENT_PRESERVATION_ABORT on null hashes; fixed by generation_id partition: null→DISCARD/EXPIRY_ABORT, set→EC-003 resume; fault-injection test mandate §5a). F3 MED §Downstream/§BC Impact residual inverted reader wording + stale provenance labels (generation-first/canonical-fallback corrected; BC-1.18.010 v1.8→v1.9 labels updated). F4 MED §4e ABORTED disposition + txn-selection disambiguation + terminal-txn GC/archival policy. F5 LOW error-taxonomy v1.29 ALREADY_MIGRATED casing (completed.json lowercase). F6 LOW §5c Branch 2 flock parity (pre-reconcile flock(exclusive.lock, LOCK_EX\|LOCK_NB) added; on EWOULDBLOCK exit 0 without gate flip). NOTE: TRAJECTORY REVERSED (1→2); 3rd fix-induced concurrency/recovery regression; human directed KEEP-GRINDING 2nd time. ARCH-INDEX v4.39→v4.40; BC-INDEX v5.95→v5.96; VP-INDEX v3.22 UNCHANGED. Input-hashes: ADR-052 a9d309a (PASS); error-taxonomy (PASS). [D-1222-DRIFT-001] prd.md §5.1 UNCHANGED OPEN. [D-1224-DRIFT-001] VP-leg UNCHANGED OPEN. OWED #2 unchanged (907-file hash sweep). BC-5.39.001 LOCAL streak 0/3 (NOT-RATIFIABLE ≠ CLEAN; pass-10 next). TRAJECTORY REVERSED: CRIT+HIGH 7→5→5→2→2→2→1→1→2 (tail LENGTH=4 →2→1→1→2). POLICY 22 OPEN — 2 sign-off items: (i) macOS exec-TOCTOU; (ii) APFS RATIFICATION PREREQUISITE — UNCHANGED. Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

---

## D-1230 — ADR-052 v1.13 FINAL accept-at-floor fix burst + prd.md §5.1 MIG/MAINTENANCE sync (pass-10)

**Date:** 2026-09-13
**Burst type:** spec fix burst (state-manager, single-commit TD-VSDD-053)
**Adversary pass:** pass-10 LOCAL (in-house Claude, fresh context, reads only pass-9 Part A per Iron Law)
**Verdict:** RATIFY-WITH-CHANGES
**Human decision:** ACCEPT-AT-FLOOR per D-386 Option C

### Finding Table (pass-10)

| Finding | Severity | Summary | Resolution |
|---------|----------|---------|------------|
| HIGH-1 | HIGH | 4th fix-induced concurrency-core regression: v1.12 flock-gated self-heal predicate (gate∈{LOCKED,DRAINING} AND no-active-txn) fires at step 2.5 crash where txn=STAGING with generation_id=null ALREADY written — coordinator had written txn before crash; self-heal would gate-flip without discard of null-generation STAGING state | FINAL prose fix: self-heal checks for null-generation STAGING txn first; null-generation STAGING → DISCARD (delete partial txn per §4e null-generation partition), THEN gate→OPEN; combines §4e null-STAGING logic (v1.12) with step-3.5 self-heal path (ADR-052 v1.13) |
| MED-1 | MED | EXPIRY_ABORT third arm missing: reservation absent-AND-expired (crash before reservation creation at step 2.5) also requires DISCARD of null-generation STAGING + EXPIRY_ABORT; prior text only covered TTL-expired case | Third arm added to EXPIRY_ABORT trigger: absent-AND-expired reservation → DISCARD null-STAGING txn + EXPIRY_ABORT (ADR-052 v1.13) |
| MED-2 | MED | Remaining v1.2 dangling refs in §Downstream + §BC Impact: cross-version "see v1.2 §" language not fully inlined in prior fix bursts | All cross-version references replaced with direct spec prose; no "see v1.2 §" refs remain (ADR-052 v1.13) |
| MED-3 | MED | Null-generation txn disposition was implied as DELETE — should be ABORTED-retained to preserve audit trail consistent with §4e ABORTED row | Null-generation txn disposition set to ABORTED-retained; archival to .factory/migration-audit/txn-archive/ preserves audit trail; consistent with §4e ABORTED row per D-1229 F4 (ADR-052 v1.13) |
| LOW-1 | LOW | generation_id ordering invariant not stated explicitly — null generation_id semantics rely on implicit convention | Invariant stated in §Decision 5a: generation_id assigned at step 2.5+ε for any post-crash-recovery STAGING record; null generation_id exclusive to crash window (ADR-052 v1.13) |
| LOW-2 | LOW | ADR-051 version note stale (v1.14 reference) | Updated to v1.15 (ADR-052 v1.13) |

### Pre-ratification cleanup (prd.md §5.1 MIG/MAINTENANCE sync)

**[D-1222-DRIFT-001] RESOLVED:** prd.md §5.1 updated to include MIG and MAINTENANCE error categories with correct exit codes aligned with error-taxonomy v1.30. prd.md v1.4→v1.5.

### §Verification-Strategy accept-at-floor note

Added to ADR-052 v1.13: concurrency state machine FROZEN for prose review; definitive crash-safety/liveness verification DEFERRED to cluster-5 implementation-phase Kani model-checking + exhaustive fault-injection.

### E-SHD-005 VP-leg assessment

**[D-1224-DRIFT-001] ASSESSED DEFERRABLE (NOT a POLICY 22 blocker):** E-SHD-005 is the STEADY-STATE native gate's HookResult (BC-1.18.006/BC-1.18.010), scoped exclusively to the steady-state gate path. ADR-052's accept-at-floor explicitly defers concurrency/gate verification to cluster-5 implementation-phase Kani+fault-injection. E-SHD-005 VP coverage is a BC-1.18.006/010 verification concern anchored to E-12 verification story — not an ADR-052-ratification prerequisite.

### New follow-up items (tracked)

- **prd.md §5.1 inaccuracy (TRACKED):** "All dispatcher-level errors except E-CAP and E-PLG exit 0" summary line is inaccurate; implementation-phase follow-up anchored to S-25.06 or cluster-5 cleanup.
- **E-SHD-NNN absent from prd.md §5.1 (TRACKED):** E-SHD-NNN category not present in prd.md §5.1 error taxonomy; anchored to E-12/BC-1.18.006/010 verification story.

### Codification

**Human decision (D-1230):** ACCEPT-AT-FLOOR per D-386 Option C — stop prose-patching the concurrency core; definitive crash-safety/liveness verification DEFERRED to cluster-5 implementation-phase Kani model-checking + exhaustive fault-injection. Adversary cascade CLOSED at pass-10.

**Input-hash status:**
- ADR-052: a9d309a (PASS — --check verified; architect set this at v1.12; v1.13 does not change this hash as architect confirmed)
- error-taxonomy.md: 35180ab (UPDATED to v1.30)
- prd.md: 185754e (UNCHANGED — synthetic input cannot be resolved by compute-input-hash; pre-existing limitation)

**Drift items status:**
- [D-1222-DRIFT-001] prd.md §5.1 MIG+MAINTENANCE sync: RESOLVED (prd.md v1.4→v1.5 applied).
- [D-1224-DRIFT-001] E-SHD-005 VP-leg gap: ASSESSED DEFERRABLE — anchored to E-12/BC-1.18.006/010 verification story (NOT POLICY 22 blocker).
- [D-1225-PG-001] sibling-sweep → S-12.15: UNCHANGED OPEN.

**POLICY 22 status:** AWAITING HUMAN RATIFICATION — 5 sign-off items:
- (i) macOS exec-TOCTOU residual window + no-concurrent-cargo-build pre-flight.
- (ii) APFS darwin-arm64 durability test: RATIFICATION PREREQUISITE — must complete before POLICY 22 ratification gate.
- (iii) CLAUDE.md amendment (4 exact append-log paths + sub-shard/manifest allowlist).
- (iv) 4 dispatcher-guard amendments at cluster-5 activation.
- (v) Accept-at-floor acknowledgment per D-1230 (this decision).
Cluster-5 TDD remains BLOCKED until POLICY 22 ratification.

**OWED (unchanged):** OWED #2 (907-file hash sweep).

### Canonical 6-column row (STATE.md Decisions Log)

| D-1230 | D-1230-ADR052-V113-PASS10-ACCEPT-AT-FLOOR | **ADR-052 v1.13 FINAL accept-at-floor fix burst committed (state-manager, single-commit TD-VSDD-053; D-1230): in-house adversary LOCAL pass-10 = RATIFY-WITH-CHANGES (1 HIGH + 3 MED + 2 LOW); all 6 findings closed — HIGH-1 (4th fix-induced concurrency-core regression): step-3.5 self-heal predicate covers null-generation STAGING crash (null-generation STAGING → DISCARD first, THEN gate→OPEN); MED-1 EXPIRY_ABORT third arm (absent-AND-expired reservation → DISCARD null-STAGING + EXPIRY_ABORT); MED-2 v1.2 dangling refs inlined; MED-3 null-generation txn disposition = ABORTED-retained; LOW-1 generation_id ordering invariant; LOW-2 ADR-051 version note. §Verification-Strategy accept-at-floor note added. prd.md §5.1 MIG/MAINTENANCE sync ([D-1222-DRIFT-001] RESOLVED): prd.md v1.4→v1.5. error-taxonomy v1.29→v1.30. ARCH-INDEX v4.40→v4.41; BC-INDEX v5.96 UNCHANGED; VP-INDEX v3.22 UNCHANGED. Input-hashes: ADR-052 a9d309a (PASS); error-taxonomy 35180ab (updated). [D-1222-DRIFT-001] RESOLVED. [D-1224-DRIFT-001] E-SHD-005 VP-leg: ASSESSED DEFERRABLE (NOT POLICY 22 blocker; anchored to E-12). OWED #2 unchanged (907-file hash sweep). BC-5.39.001 LOCAL streak 0/3 (RATIFY-WITH-CHANGES ≠ CLEAN). TRAJECTORY: CRIT+HIGH 7→5→5→2→2→2→1→1→2→1 (10 passes; floor at 1; tail LENGTH=4 →1→1→2→1). HUMAN DECISION: ACCEPT-AT-FLOOR per D-386 Option C — concurrency state machine FROZEN; crash-safety/liveness DEFERRED to cluster-5 Kani+fault-injection. POLICY 22: AWAITING HUMAN RATIFICATION (5 sign-off items: i=macOS exec-TOCTOU; ii=APFS RATIFICATION PREREQUISITE; iii=CLAUDE.md amendment; iv=dispatcher guards; v=accept-at-floor acknowledgment). Cluster-5 TDD BLOCKED. pipeline: PAUSED.** | S-25.02 F4 | 2026-09-13 |

---

## D-1231 — ADR-052 v1.14 DEF-1 Kani model-checking fix-burst persist (mechanically-verified)

**Date:** 2026-09-20
**Burst type:** mechanically-verified spec fix persist (state-manager, single-commit TD-VSDD-053)
**Verification method:** Kani model-checking (`cargo-kani` 0.67.0) — first formal-methods pass against the ADR-052 concurrency/recovery state machine, per the D-1230 HUMAN DECISION deferring definitive crash-safety/liveness verification to cluster-5 implementation-phase Kani + fault-injection. Human directed "more convergence before accept-at-floor ratification" per D-386 Option C — this pass ran BEFORE POLICY 22 ratification, as additional spec/design convergence work.
**Finding:** DEF-1 (HIGH) — 5th fix-induced concurrency-core regression in this cascade (1st pass-4 C-1 reader-inversion; 2nd pass-8 flock self-heal regression; 3rd pass-9 F2 null-STAGING sub-state; 4th pass-10 HIGH-1 null-generation self-heal gap; this 5th landing in the previously-unexamined `gate=OPEN` window). An uncovered self-heal window: durable state `gate=OPEN, txn=STAGING(generation_id=null), flock released, no gen dir, dead coordinator` was reachable and unhandled — a fix-induced regression from v1.11's step-2.5 ordering (writing the initial txn record BEFORE the DRAINING flip, introduced as "defense-in-depth" against a flock check the ADR's own v1.11 rationale already held sufficient) that survived 10 prose-adversarial LOCAL passes (D-1221..D-1230 (exhaustive)) because step-3.5's self-heal predicate is gated on `gate_state ∈ {LOCKED, DRAINING}` and never matches `gate=OPEN`.

**Fix (ADR-052 v1.14, Option B — structural reorder):** §Decision 5a drain procedure reordered — the initial txn-record write moves from "step 2.5" to new step **3a**, strictly AFTER step 3 (the DRAINING flip) completes. This restores `gate_state=OPEN ⟹ no active STAGING/COMMITTING txn record` as a structural invariant (INV-GATE-TXN) by construction, not by enumeration. No new step-3.5 self-heal branch was added — Branch A and Branch B (v1.11/v1.13) are unchanged; Branch B's `gate ∈ {LOCKED, DRAINING}` precondition is now provably always true whenever `txn=STAGING(generation_id=null)` exists. The flock-gated step-3.5 sub-step-a check (v1.11 HIGH-1) remains the sole load-bearing writer-exclusion mechanism, unchanged. Two alternatives were evaluated and rejected: (A) extend the step-3.5 predicate to also cover `gate=OPEN` — rejected, repeats the add-a-branch anti-pattern behind all four prior regressions; (C) cross-file atomic critical section spanning both writes — rejected, the two writes are already effectively serialized by the coordinator's continuous hold on `exclusive.lock`, so file-level atomicity does not address the actual defect (a durable-state-reachability problem, not a concurrency-atomicity problem), and would be a materially larger redesign for no added safety.

**Mechanical re-verification (Kani, post-fix, under v1.14 model):**
- **7/7 VP proofs PROVED** under the v1.14 model ordering.
- **INV-GATE-TXN (`gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}`) proven UNSAT** under v1.14 — the defect state is unconstructible by the solver, labeled **VP-M7** by the finding verifier for the next formal-verifier pass.
- **Non-vacuity confirmed:** the same harness reproduces DEF-1 (SAT, counterexample found) when re-run against the OLD (pre-v1.14, step-2.5) ordering — the proof is not vacuously true; it genuinely distinguishes the fixed model from the defective one.
- **5/5 regression harnesses pass** (covering the four prior fix-induced regressions D-1221/1228/1229/1230 plus DEF-1 itself, confirming no re-introduction).
- **7/7 fault-injection tests pass**, including the two new DEF-1 test mandates added to ADR-052 §5a (step-3/step-3a interstitial-crash test; construct-and-assert-UNSAT regression test).

**Scope bound (explicit, non-waivable):** this verification is **exhaustive over the DESIGN-model abstraction** used by the Kani harness (the state machine as specified in ADR-052 §Decision 4e/5a/7c) — it is **NOT** exhaustive over the eventual Rust implementation (`executor.rs`/`shard_manager.rs`, not yet written at cluster-5). Per the BINDING, NON-DEFERRABLE condition ratified at D-1232, implementation-phase Kani harnesses against the real `executor.rs`/`shard_manager.rs` are MANDATORY when cluster-5 is built — this pass does not substitute for that obligation.

**ADR-052 v1.13→v1.14 committed** (§Decision 5a drain-step reorder; §Consequences → Verification Strategy re-verification directive for VP-M7; §BC Impact note — no BC/error-taxonomy change required; §Files-to-Change `executor.rs`+`tests/` rows swept; unrelated stale citation "Drain-timeout abort (step 3 above)"→"(step 4 above)" corrected in-scope). D-NNN placeholders the architect left in ADR-052 §Source/Origin + §References backfilled to **D-1231** (this entry) for the Kani-pass-1/DEF-1 provenance citations; the SEPARATE `D-NNN` placeholder in ADR-052's §Decision 11 POLICY 22 block ("the D-NNN ratification entry") was backfilled to **D-1232** (see below — that placeholder refers to the ratification event, not this fix-persist event). **ARCH-INDEX v4.41→v4.42** (frontmatter `last_amended` + `changelog:` entry + ADR-052 row body v1.13→v1.14, D-1231 cited in place of the architect's D-NNN placeholder). error-taxonomy UNCHANGED v1.30. prd.md UNCHANGED v1.5. BC-INDEX UNCHANGED v5.96. VP-INDEX UNCHANGED v3.22. total_adrs UNCHANGED 52.

**Input-hash status:** ADR-052 `a9d309a` — `compute-input-hash --check` PASS (unchanged; the D-NNN backfill and §5a reorder do not touch the `inputs:` frontmatter array, only the ADR's own prose).

**POLICY 22 status:** was AWAITING HUMAN RATIFICATION at burst start; formal-verifier re-verification of VP-M7 was an ADDITIONAL prerequisite flagged by this ADR amendment before re-evaluating accept-at-floor — this DEF-1 fix + the 7/7 VP proofs above SATISFY that VP-M7 prerequisite. See **D-1232** (this same burst) for the ratification decision itself.

**Cluster-5 TDD:** remains BLOCKED at the moment this entry is authored (ratification is the immediately-following D-1232 entry in this same commit). `pipeline:` PAUSED at persist time of this entry.

### Canonical 6-column row (STATE.md Decisions Log)

| D-1231 | D-1231-ADR052-V114-DEF1-KANI-PASS1-MECHANICALLY-VERIFIED | **ADR-052 v1.14 DEF-1 Kani model-checking fix-burst persisted (state-manager, single-commit TD-VSDD-053; D-1231): Kani (cargo-kani 0.67.0) pass-1 found DEF-1 (HIGH) — 5th fix-induced concurrency-core regression: uncovered self-heal window `gate=OPEN + txn=STAGING(gen=null) + dead coordinator` from v1.11's step-2.5 pre-DRAINING txn write, survived 10 prose-adversarial passes (D-1221..D-1230 (exhaustive)). Fixed via Option B: initial txn record moved to new step 3a, strictly AFTER the step-3 DRAINING flip — `gate_state=OPEN ⟹ no active STAGING/COMMITTING txn` (INV-GATE-TXN) now holds by construction; no new self-heal branch added. Re-verified: 7/7 VP proofs PROVED; INV-GATE-TXN UNSAT under v1.14; non-vacuity CONFIRMED (old ordering reproduces DEF-1, SAT); 5/5 regression harnesses + 7/7 fault-injection tests PASS. Bound: exhaustive over the DESIGN-model abstraction, NOT the implementation — implementation-phase Kani on executor.rs/shard_manager.rs remains a MANDATORY cluster-5 obligation (ratified D-1232). D-NNN placeholders backfilled: ADR-052 §Source/Origin+§References → D-1231 (this entry, Kani-pass-1 provenance); ADR-052 §Decision-11 ratification-entry placeholder → D-1232 (ratification, separate entry below). ARCH-INDEX v4.41→v4.42 (ADR-052 row v1.13→v1.14). error-taxonomy/prd.md/BC-INDEX/VP-INDEX UNCHANGED. Input-hash: ADR-052 a9d309a PASS (unchanged). total_adrs UNCHANGED 52. See D-1232 for POLICY 22 ratification disposition.** | S-25.02 F4 | 2026-09-20 |

---

## D-1232 — POLICY 22 ratification: human 5-item sign-off — cluster-5 UNBLOCKED

**Date:** 2026-09-20
**Burst type:** ratification burst (state-manager, single-commit TD-VSDD-053)
**Basis:** D-1231's mechanical (Kani) re-verification of the concurrency/recovery state machine, conducted as human-directed additional spec/design convergence work ahead of ratification, per "more convergence before accept-at-floor ratification" (human direction, this session).
**Method:** Human interactive 5-item sign-off walk (AskUserQuestion), this session. Dispositions recorded here verbatim in substance, supplied by the orchestrator to state-manager for persistence.

### Disposition Table (5 sign-off items)

| Item | ADR-052 anchor | Disposition |
|------|----------------|-------------|
| 1 — macOS exec-TOCTOU residual window | §Decision 11 sign-off item 1 | **ACKNOWLEDGED.** Human accepts the documented residual TOCTOU window on macOS/darwin-arm64 as adequately mitigated by (a) maintenance-lock freeze-build, (b) hard no-concurrent-`cargo build` pre-flight, (c) pre-exec mtime guard re-stat immediately before `execve`. Linux eliminates the window entirely via fd-binding (`execveat AT_EMPTY_PATH`); macOS carries the acknowledged residual. |
| 2 — APFS dir-fsync durability | §Decision 11 sign-off item 2 / §Decision 7d | **SATISFIED VIA HYBRID** — not an absolute guarantee (research spike, 2× `perplexity_research`, sourced, established none is attainable on this hardware/OS combination). Human's disposition, after first proposing "run the test" then revising post-research: (a) MANDATE the code sequence `F_FULLFSYNC(temp) → rename → F_FULLFSYNC(dir)` with strict error propagation, NO silent fallback; (b) run a differential VM-kill test on the operator darwin-arm64 Mac (`F_FULLFSYNC(dir)` vs `F_BARRIERFSYNC` vs plain `fsync` vs none — relative durability + atomicity evidence) — **PENDING, cluster-5 deliverable**; (c) EXPLICIT residual-risk acknowledgment recorded: Apple documents no per-rename directory-sync power-loss contract; `F_FULLFSYNC` is best-effort at the hardware layer; no software-only test on this hardware certifies physical media survives power loss. |
| 3 — CLAUDE.md ADR-052 EXCEPTION amendment | §Decision 8 / CLAUDE.md-amendment-text block | **APPROVED, exact text.** The amendment to CLAUDE.md's `### Forbidden patterns` table (TD-FACTORY-HOOK-BYPASS-001 P0 row) is approved exactly as written in ADR-052 §Decision 8: migration-binary path allowlist (BC-INDEX + shards `.a/.b/.c[a-z].md` + manifests + the 4 exact append-log paths + `migration-state`/`activation`/`migration-audit`), preconditions (a)-(e), post-success obligations (f)-(g); all other `.factory/` writes remain Edit/Write-only. **APPLY at cluster-5 F4 activation** — CLAUDE.md is human-mandated-edit-only per its own governing text; the human's directive is to apply the amendment at the F4 activation boundary, not in this burst. |
| 4 — 4 dispatcher-guard amendments | §Decision 5b | **APPROVED.** Full-command pre-shell classifier + executable-digest verification so the sanctioned migration binary is recognized (`validate-factory-path-staging` + `destructive-command-guard`); `factory-branch-guard` explicit waiver (migration runs in the main worktree); PID-liveness → txn-record-state awareness (OPEN/DRAINING). **DEPLOY at cluster-5 activation.** |
| 5 — concurrency core | §Verification Strategy / §Decision 4e,5a,7c | **RATIFIED ON MECHANICAL PROOF — supersedes the D-1230 accept-at-floor basis.** Human REJECTED accept-at-floor and required more convergence before ratifying; mechanistic verification was pulled forward ahead of cluster-5 implementation. D-1231's Kani model-checking pass-1 found DEF-1 (HIGH), fixed structurally via ADR-052 v1.14 Option B (drain-step reorder — see D-1231); re-verified 7/7 VP proofs PROVED, INV-GATE-TXN UNSAT (non-vacuity CONFIRMED), 5/5 regression harnesses + 7/7 fault-injection tests PASS. Human then ratified on the strength of that proof, WITH a **BINDING, NON-DEFERRABLE CONDITION**: implementation-phase Kani harnesses on the real `executor.rs` + `shard_manager.rs` are MANDATORY when cluster-5 is built — a design-model proof does NOT substitute for implementation verification. |

### 4 Binding Obligations Registered (pending, anchored to cluster-5/S-25.02)

Recorded in STATE.md `## Blocking Issues` this same burst (non-blocking to ratification; blocking to specific cluster-5 sub-steps as noted):

(a) **[D-1232-OBL-1]** Implementation-phase Kani harnesses on `executor.rs` + `shard_manager.rs` — MANDATORY, non-deferrable (item 5). Blocks: cluster-5 TDD completion (not TDD start).
(b) **[D-1232-OBL-2]** APFS hybrid durability: `F_FULLFSYNC(temp)→rename→F_FULLFSYNC(dir)` + strict error propagation (mandatory code shape) + differential VM-kill test (`F_FULLFSYNC(dir)` vs `F_BARRIERFSYNC` vs plain `fsync` vs none) + residual-risk acknowledgment already recorded above (item 2). Blocks: cluster-5 F4 activation on macOS.
(c) **[D-1232-OBL-3]** Apply the CLAUDE.md ADR-052 EXCEPTION amendment (item 3) at cluster-5 F4 activation.
(d) **[D-1232-OBL-4]** Deploy the 4 dispatcher-guard amendments (item 4) at cluster-5 activation.

### Codification

**Human decision (D-1232):** POLICY 22 **RATIFIED**. All 5 sign-off items dispositioned per the table above. ADR-052's `D-NNN` placeholder at §Decision 11 ("the D-NNN ratification entry that ratifies ADR-052") and the two sign-off-item cross-references at §Decision 11 (both citing "the D-1232 entry") are satisfied by this entry — those citations already read "D-1232" literally in the architect's own v1.4/v1.7 authoring, so no further ADR-052 body edit is required by state-manager (ADR-052 body content is architect-owned per CLAUDE.md Agent Routing Table).

Cluster-5 TDD **UNBLOCKED**. `pipeline:` PAUSED → **in_progress**.

**Input-hash status:** ADR-052 `a9d309a` UNCHANGED (PASS — no ADR-052 body edit this burst). No other input files touched.

**Drift items status:** unchanged this burst — see STATE.md Blocking Issues for the 4 newly-registered obligations (not drift; forward obligations anchored to cluster-5/S-25.02).

**OWED (unchanged):** OWED #2 (907-file `compute-input-hash --scan --update` sweep).

### Canonical 6-column row (STATE.md Decisions Log)

| D-1232 | D-1232-POLICY22-RATIFIED-CLUSTER5-UNBLOCKED | **POLICY 22 RATIFIED (state-manager, single-commit TD-VSDD-053; D-1232): human interactive 5-item sign-off walk dispositioned all outstanding POLICY 22 items. (i) macOS exec-TOCTOU: ACKNOWLEDGED — residual window adequately mitigated by freeze-build+no-concurrent-cargo-build pre-flight+pre-exec mtime guard; Linux eliminates via fd-binding. (ii) APFS dir-fsync durability: SATISFIED VIA HYBRID — MANDATED F_FULLFSYNC(temp)→rename→F_FULLFSYNC(dir) + strict error propagation; differential VM-kill test PENDING (cluster-5 deliverable); explicit residual-risk ack recorded (no software-only test certifies physical media). (iii) CLAUDE.md ADR-052 EXCEPTION amendment: APPROVED exact text per §Decision 8; APPLY at F4 activation. (iv) 4 dispatcher-guard amendments: APPROVED per §Decision 5b; DEPLOY at cluster-5 activation. (v) concurrency core: RATIFIED ON MECHANICAL PROOF, superseding D-1230 accept-at-floor — human rejected accept-at-floor, required more convergence; D-1231 Kani DEF-1 fix + 7/7 VP re-verification (INV-GATE-TXN UNSAT, non-vacuity CONFIRMED) satisfied the bar; BINDING NON-DEFERRABLE condition: impl-phase Kani on executor.rs+shard_manager.rs mandatory at cluster-5 build. 4 binding obligations registered in STATE.md Blocking Issues, anchored cluster-5/S-25.02: (a) impl-phase Kani mandatory; (b) APFS hybrid fsync sequence+VM-kill test+residual-risk ack; (c) CLAUDE.md amendment apply at F4 activation; (d) 4 dispatcher-guard amendments deploy at activation. Cluster-5 TDD UNBLOCKED. `pipeline:` PAUSED→in_progress. ADR-052 a9d309a UNCHANGED (no body edit — architect-owned; D-NNN placeholders already read D-1232 literally). ARCH-INDEX v4.42/BC-INDEX v5.96/VP-INDEX v3.22/error-taxonomy v1.30/prd.md v1.5 all UNCHANGED this burst. OWED #2 unchanged (907-file hash sweep).** | S-25.02 F4 | 2026-09-20 |

---

## D-1233 — ADR-052 v1.15 status flip: proposed→accepted (POLICY 22 ratification persisted)

**Date:** 2026-09-21
**Burst type:** ratification-flip persist (state-manager, single-commit TD-VSDD-053)
**Basis:** D-1232's POLICY 22 ratification (human interactive 5-item sign-off, 2026-09-20) and D-1231's mechanical Kani re-verification of the ADR-052 concurrency/recovery state machine.

**Action:** The architect edited ADR-052's frontmatter (`version: "1.14"→"1.15"`, `status: proposed→accepted`) and rewrote the `## Status` block from the v1.13/v1.14 "PROPOSED" narrative to an ACCEPTED disposition ("ACCEPTED — ratified per POLICY 22 (D-1232, 2026-09-20)"), plus added a v1.15 changelog row — this was already done, on disk, uncommitted, PRIOR to this burst. This burst's job was exclusively the cross-document propagation (ARCH-INDEX, this decision-log entry, STATE.md) and the atomic commit; state-manager performed no ADR-052 body edit (ADR-052 content is architect-owned per CLAUDE.md's Agent Routing Table).

Verified via `git -C .factory diff` before persisting: the ADR-052 diff contains exactly the frontmatter version/status flip, the `## Status` block rewrite, and the new v1.15 changelog row — no other content changed (§Decision 1–11 substance untouched; this is a status/lifecycle-only amendment, per the architect's own v1.15 changelog annotation).

**Why now:** ADR-052's concurrency core was RATIFIED ON MECHANICAL PROOF at D-1232 (superseding the D-1230 accept-at-floor basis) — the human rejected accept-at-floor and required additional convergence before ratifying; D-1231's Kani model-checking pass-1 found and fixed DEF-1 (HIGH, the 5th fix-induced concurrency-core regression in this cascade) via the Option-B structural drain reorder, then re-verified: 7/7 VP proofs PROVED, the INV-GATE-TXN invariant (`gate_state=OPEN ⟹ no txn record in {STAGING, COMMITTING}`) proven UNSAT, non-vacuity CONFIRMED (the same harness reproduces DEF-1 under the OLD ordering), and 5/5 regression harnesses + 7/7 fault-injection tests PASS. This satisfied the human's bar for ratification, WITH the binding, non-deferrable condition that implementation-phase Kani harnesses on the real `executor.rs`/`shard_manager.rs` remain mandatory at cluster-5 build time.

D-1232's own ratification entry recorded the ratification DECISION and registered the 4 pre-activation obligations, but ADR-052's own `status:` field still read `proposed` after that burst (D-1232 explicitly noted "ADR-052 a9d309a UNCHANGED (no body edit — architect-owned)"). This D-1233 burst is the first concrete on-ramp step closing that gap: the architect performed the body edit, and this burst persists + propagates it.

**ARCH-INDEX v4.42→v4.43:** frontmatter `version` bumped 4.42→4.43; `last_amended` overwritten with the v4.43 entry only, per the `last_amended` write-path discipline (BC-5.45.001/ADR-049/S-15.03 AC-005) — the displaced v4.42 entry (D-1231's full text) was prepended verbatim as a new top item to `changelog:`, every pre-existing `changelog:` item left byte-for-byte untouched. ADR-052's row body received a new `**FIX-BURST AMENDMENT 2026-09-21 (v1.15 — state-manager; ...)**` segment appended after the v1.14 segment, following the cell's established append convention; the cell's trailing sentinel and `Refs:` line reconciled from `ADR-052 v1.14` to `ADR-052 v1.15`. `total_adrs` UNCHANGED 52.

**Remaining pre-activation obligations (UNCHANGED/OPEN, carried forward verbatim from D-1232 — see STATE.md `## Blocking Issues`):**
- **[D-1232-OBL-1]** Implementation-phase Kani harnesses on `executor.rs` + `shard_manager.rs` — MANDATORY, non-deferrable; blocks cluster-5 TDD *completion*, not start.
- **[D-1232-OBL-2]** APFS hybrid durability: mandated `F_FULLFSYNC(temp)→rename→F_FULLFSYNC(dir)` sequence + differential VM-kill test — blocks cluster-5 F4 *activation* on macOS.
- **[D-1232-OBL-3]** Apply the CLAUDE.md ADR-052 EXCEPTION amendment — at cluster-5 F4 activation.
- **[D-1232-OBL-4]** Deploy the 4 dispatcher-guard amendments — at cluster-5 activation boundary.

**STATE.md this same burst:** `pipeline:` PAUSED→in_progress (session RESUMED active delivery work toward cluster-5 F4 — this is the first Phase-A on-ramp step, NOT cluster-5 TDD dispatch itself, which remains a separate future action). Session Resume Checkpoint refreshed: §4 REMAINING-WORK item #2 (ADR-052 flip) marked DONE this burst; item #1 (E-26 registration) marked DEFERRED per human direction 2026-09-21 — E-25 completion (delivered via S-25.02 cluster-5 F4) precedes E-26 registration; items #3 (STORY-INDEX dangling-input fix), #4 (BC-INDEX sharding + cluster-5 TDD), #5 (hook-hardening batch #837-841 → rc.26) UNCHANGED. STATE.md v10.64→v10.65.

**Sidecar note:** this burst's commit also folds in a pre-existing uncommitted `sidecar-learning.md` delta (4 appended session-end telemetry lines from a prior session-end hook run, not authored by this burst) to leave the `.factory/` worktree clean at commit time — an incidental inclusion, not this burst's own content.

**Input-hash status:** ADR-052 `a9d309a` UNCHANGED — the `version`/`status` frontmatter fields and the `## Status`/changelog body edits are not part of the ADR's `inputs:` array; `compute-input-hash --check` remains PASS.

**error-taxonomy/prd.md/BC-INDEX/VP-INDEX:** all UNCHANGED this burst (v1.30 / v1.5 / v5.96 / v3.22 respectively).

**OWED (unchanged):** OWED #2 (907-file `compute-input-hash --scan --update` sweep).

### Canonical 6-column row (STATE.md Decisions Log)

| D-1233 | D-1233-ADR052-V115-ACCEPTED-POLICY22-RATIFICATION-FLIP | **ADR-052 status proposed→accepted persisted (state-manager, single-commit TD-VSDD-053; D-1233): architect's already-edited ADR-052 body committed — version 1.14→1.15; status: proposed→accepted; §Status block rewritten to "ACCEPTED — ratified per POLICY 22 (D-1232, 2026-09-20)"; v1.15 changelog row added. Basis: D-1231 mechanical Kani proof (DEF-1 HIGH concurrency regression fixed in v1.14 via Option-B structural drain reorder — initial txn-record write moved to step 3a, strictly after the step-3 DRAINING flip; re-verified 7/7 VP proofs PROVED, INV-GATE-TXN invariant UNSAT under the corrected model, non-vacuity CONFIRMED, 5/5 regression + 7/7 fault-injection tests PASS). This is the first Phase-A on-ramp step toward S-25.02 cluster-5 F4 activation. ARCH-INDEX v4.42→v4.43 (row-body FIX-BURST AMENDMENT segment appended reflecting the v1.15 status flip; frontmatter last_amended + changelog updated per BC-5.45.001 write-path discipline; total_adrs UNCHANGED 52). error-taxonomy UNCHANGED v1.30. prd.md UNCHANGED v1.5. BC-INDEX UNCHANGED v5.96. VP-INDEX UNCHANGED v3.22. Remaining pre-activation obligations UNCHANGED/OPEN (STATE.md Blocking Issues): [D-1232-OBL-1] implementation-phase Kani harnesses on executor.rs+shard_manager.rs — mandatory, blocks cluster-5 TDD completion (not start); [D-1232-OBL-2] APFS hybrid fsync sequence + differential VM-kill test — blocks F4 activation on macOS; [D-1232-OBL-3] CLAUDE.md ADR-052 EXCEPTION amendment — apply at F4 activation; [D-1232-OBL-4] 4 dispatcher-guard amendments — deploy at cluster-5 activation boundary. `pipeline:` PAUSED→in_progress — session RESUMED active delivery work toward cluster-5 F4. Session Resume Checkpoint refreshed: §4 REMAINING-WORK item #2 marked DONE this burst; item #1 (E-26 registration) DEFERRED per human direction 2026-09-21 (E-25 completion via S-25.02 cluster-5 precedes E-26 registration); items #3/#4/#5 UNCHANGED. Input-hash: ADR-052 a9d309a UNCHANGED (status/version/changelog frontmatter fields are not part of the inputs: array — no drift). OWED #2 unchanged (907-file hash sweep).** | S-25.02 F4 | 2026-09-21 |

## D-1234 — STORY-INDEX stale `inputs:` citation fix (Phase-A on-ramp item #3)

**Date:** 2026-09-21
**Burst type:** dangling-input remediation persist (state-manager, single-commit TD-VSDD-053)
**Basis:** D-1233's Session Resume Checkpoint §4 REMAINING-WORK item #3 ("STORY-INDEX dangling-input fix ... blocks its `compute-input-hash --update`; route product-owner/story-writer. NEXT ITEM ON RESUME").

**Action:** STORY-INDEX stale `inputs:` citation fix — removed dangling `.factory/stories/v1.0/EPIC.md` (dir renamed to `v1.0-legacy/` at Phase 1.8 migration commit f344b56e; the file is frozen historical provenance, not a live generative input — epic content is now inline `## Epic E-N` sections). Option B (drop) chosen over re-point to keep the traceability chain honest. STORY-INDEX v4.472→v4.473; input-hash recomputed via `compute-input-hash --update`, unblocking the previously-failing update. Phase-A on-ramp item #3 of the S-25.02 cluster-5 activation sequence.

The story-writer edit (frontmatter-only: `inputs:` entry dropped, `version: "4.472"→"4.473"`, `last_amended` + top `changelog:` entry added) was already made and left uncommitted in the `.factory/` worktree prior to this burst. This burst's job was exclusively: (1) recomputing STORY-INDEX's own `input-hash` now that the dangling input is gone, (2) this decision-log codification, and (3) the STATE.md advance + atomic commit — state-manager performed no STORY-INDEX body/content edit (STORY-INDEX content is story-writer-owned per CLAUDE.md's Agent Routing Table).

**Input-hash before/after:** STORY-INDEX.md carried NO `input-hash:` frontmatter field prior to this burst (`compute-input-hash --check` reported "has no input-hash (not yet computed)") — the dangling `.factory/stories/v1.0/EPIC.md` input (MISSING on disk since the f344b56e rename) blocked a clean `--update` for as long as it remained listed. With the dangling entry dropped by story-writer, `compute-input-hash .factory/stories/STORY-INDEX.md --resolve` confirms both remaining inputs (`pass-6-synthesis.md`, `ARCH-INDEX.md`) resolve; `--update` computed and wrote `input-hash: "7cc0c23"`; a follow-up `--check` reports clean (exit 0, no drift). `compute-input-hash --scan .factory/stories` confirms STORY-INDEX.md itself is MATCH (not STALE/PARTIAL/UNCOMPUTED) — the scan's large pre-existing STALE/PARTIAL/UNCOMPUTED population across ~180 other unrelated story files is pre-existing drift (OWED #2, the 907-file sweep) and is explicitly OUT OF SCOPE for this burst.

**POLICY 18 applicability:** POLICY 18 THREE-WAY-INPUT-HASH-EQUALITY GATE governs PER-STORY `input-hash:` values (story frontmatter ↔ STORY-INDEX catalog row ↔ STORY-INDEX aggregation blockquote for that story). This burst changed STORY-INDEX's OWN self-referential `inputs:`/`input-hash:` frontmatter, not any per-story hash — POLICY 18 does not apply to this change; no per-story catalog row or blockquote requires reconciliation.

**Sidecar note:** this burst's commit also folds in a pre-existing uncommitted `sidecar-learning.md` delta (2 appended session-end telemetry lines from a prior session-end hook run, not authored by this burst) to leave the `.factory/` worktree clean at commit time — an incidental inclusion, not this burst's own content.

**STATE.md this same burst:** `pipeline:` stays `in_progress` (unchanged — no pause/resume transition this burst). Session Resume Checkpoint refreshed: §4 REMAINING-WORK item #3 (STORY-INDEX dangling-input fix) marked DONE this burst; item #1 (E-26 registration, DEFERRED) and items #4 (BC-INDEX sharding + cluster-5 TDD) / #5 (hook-hardening batch #837–841 → rc.26) UNCHANGED. STATE.md v10.65→v10.66.

**BC-INDEX/VP-INDEX/ARCH-INDEX/error-taxonomy/prd.md:** all UNCHANGED this burst.

**OWED (unchanged):** OWED #2 (907-file `compute-input-hash --scan --update` sweep — the STALE/PARTIAL/UNCOMPUTED population surfaced by this burst's `--scan .factory/stories` run remains that same pre-existing sweep's scope, not newly discovered).

### Canonical 6-column row (STATE.md Decisions Log)

| D-1234 | D-1234-STORY-INDEX-DANGLING-INPUT-FIX | **STORY-INDEX stale `inputs:` citation fix persisted (state-manager, single-commit TD-VSDD-053; D-1234): story-writer's already-edited frontmatter (dangling `.factory/stories/v1.0/EPIC.md` entry dropped — dir renamed to `v1.0-legacy/` at Phase 1.8 migration commit f344b56e; epic content now lives inline as `## Epic E-N` sections, so the legacy doc is frozen historical provenance, not a live generative input; Option B [drop] chosen over re-point to keep the traceability chain honest; `version: "4.472"→"4.473"`) committed this burst. Basis: D-1233 Session Resume Checkpoint §4 REMAINING-WORK item #3. Input-hash: STORY-INDEX carried NO `input-hash:` field before this burst (blocked by the dangling MISSING input); `compute-input-hash --update` now computes and writes `input-hash: "7cc0c23"`; follow-up `--check` CLEAN (exit 0). `compute-input-hash --scan .factory/stories` confirms STORY-INDEX.md itself is MATCH; the scan's ~180-file pre-existing STALE/PARTIAL/UNCOMPUTED population is OWED #2 (907-file sweep), explicitly OUT OF SCOPE this burst. POLICY 18 (per-story three-way hash parity) does NOT apply — this is STORY-INDEX's own self-cite `inputs:`/`input-hash:`, not a per-story hash. `pipeline:` stays in_progress (unchanged). Session Resume Checkpoint refreshed: §4 item #3 marked DONE this burst; items #1 (DEFERRED)/#4/#5 UNCHANGED. BC-INDEX/VP-INDEX/ARCH-INDEX/error-taxonomy/prd.md all UNCHANGED. OWED #2 unchanged (907-file hash sweep).** | S-25.02 F4 | 2026-09-21 |

## D-1235 — S-25.02 cluster-5 F3 story-finalization propagation burst

**Date:** 2026-09-21
**Burst type:** F3 story-finalization propagation persist (state-manager, single-commit TD-VSDD-053)
**Basis:** story-writer's S-25.02 v4.3→v4.4 edit (pre-existing uncommitted in the `.factory/` worktree), propagating product-owner's BC-1.18.010 v1.9 / BC-1.18.011 v1.8 final versions into the story per POLICY 8 (bc_array_changes_propagate_to_body_and_acs) and the cluster5-f3-delta-inventory audit.

**Action:** Propagated BC-1.18.010 v1.2→v1.9 and BC-1.18.011 v1.0→v1.8 into the S-25.02 story per POLICY 8 (consistency-validator delta audit: ~13 MISSING items, 2 outright factual errors). AC-018 fixed: withdrawn byte-for-byte whole-concatenation content-preservation model → structured per-BC-row-equivalence; stale `E-SHD-005` citation → migration-binary `CONTENT_PRESERVATION_ABORT`/`CENSUS_MISMATCH_ABORT` process exit codes. AC-017 gained three-way ARCH-INDEX parity + §Reader Integration protocol + migration-surface touchpoints. EC-059/060/061 added (BC-1.18.011 EC-002/EC-003/EC-006 analogues). Token Budget re-estimated ~88,100→~92,800. §Behavioral Contracts table BC-1.18.010 cell v1.2→v1.9 / BC-1.18.011 cell v1.0→v1.8. `## Tasks` T-10/T-11 extended. AC count unchanged (25 ACs — both extended in place, not new). NO product-owner adjudication required (both BCs stable/FINAL, no open questions). Story v4.3→v4.4. **Cluster-5 is now F3-FINALIZED and TDD-READY** — this closes the F3 gap that gated F4 TDD. Next: cluster-5 F4 TDD (stub-architect Red Gate → test-writer → implementer), with activation-boundary obligations [D-1232-OBL-1..4] applying before merge/activation.

The story-writer edit (body content: AC-017/AC-018 extended in place, EC-059/060/061 added, §Behavioral Contracts table cells updated, §Token Budget re-estimated, T-10/T-11 extended, `version: "4.3"→"4.4"`, `last_amended` + changelog row 4.4 added) was already made and left uncommitted in the `.factory/` worktree prior to this burst. This burst's job was exclusively: (1) recomputing S-25.02's own `input-hash` now that its body content changed, (2) STORY-INDEX propagation (catalog-row BC bracket + catalog-row input-hash cite + E-25 blockquote input-hash cite, three-way parity), (3) BC-INDEX verification, (4) this decision-log codification, and (5) the STATE.md advance + atomic commit — state-manager performed no story body/content edit (S-25.02 content is story-writer-owned per CLAUDE.md's Agent Routing Table).

**Input-hash before/after:** S-25.02 `input-hash` `97cffb6` (stale — PostToolUse `input_hash_drift` fired during story-writer's edit, stored `97cffb6` ≠ computed `171c3bb`, expected). `compute-input-hash .factory/stories/S-25.02-artifact-sharding-layer2.md --update` computed and wrote `input-hash: "171c3bb"`; follow-up `--check` CLEAN (exit 0).

**POLICY 18 applicability:** POLICY 18 THREE-WAY-INPUT-HASH-EQUALITY GATE governs PER-STORY `input-hash:` values (story frontmatter ↔ STORY-INDEX catalog row ↔ STORY-INDEX aggregation blockquote for that story). All three re-synced to `171c3bb` this burst — VERIFIED (STORY-INDEX v4.473→v4.474).

**BC-INDEX status:** BC-1.18.010/BC-1.18.011 catalog cells already read v1.9/v1.8 as of BC-INDEX v5.96 (D-1228 version-sync burst) — CURRENT, no change needed this burst; BC-INDEX version UNCHANGED at v5.96.

**Sidecar note:** this burst's commit also folds in a pre-existing uncommitted `sidecar-learning.md` delta to leave the `.factory/` worktree clean at commit time — an incidental inclusion, not this burst's own content.

**STATE.md this same burst:** `pipeline:` stays `in_progress` (unchanged — no pause/resume transition this burst). Phase Progress row appended: `S2502-CLUSTER5-F3-PROPAGATION-TDD-READY 2026-09-21 (D-1235)`. Session Resume Checkpoint refreshed: cluster-5 F3 marked DONE this burst; NEXT recorded as cluster-5 F4 TDD. §4 item #4 (BC-INDEX sharding + cluster-5 TDD) remains the active next-work item; item #1 (E-26 registration, DEFERRED) and item #5 (hook-hardening batch → rc.26) UNCHANGED. STATE.md v10.66→v10.67.

**BC-INDEX/VP-INDEX/ARCH-INDEX/error-taxonomy/prd.md:** BC-INDEX UNCHANGED v5.96 (already current, verified above); VP-INDEX/ARCH-INDEX/error-taxonomy/prd.md all UNCHANGED this burst.

**OWED (unchanged):** OWED #2 (907-file `compute-input-hash --scan --update` sweep).

### Canonical 6-column row (STATE.md Decisions Log)

| D-1235 | D-1235-S2502-CLUSTER5-F3-PROPAGATION-TDD-READY | **S-25.02 cluster-5 F3 story-finalization propagation burst persisted (state-manager, single-commit TD-VSDD-053; D-1235): story-writer's already-edited story body committed — propagated BC-1.18.010 v1.2→v1.9 and BC-1.18.011 v1.0→v1.8 into the S-25.02 story per POLICY 8 (consistency-validator delta audit: ~13 MISSING items, 2 outright factual errors). AC-018 fixed: withdrawn byte-for-byte whole-concatenation model → structured per-BC-row-equivalence; stale E-SHD-005 citation → migration-binary CONTENT_PRESERVATION_ABORT/CENSUS_MISMATCH_ABORT exit codes. AC-017 gained three-way ARCH-INDEX parity + §Reader Integration protocol + migration-surface touchpoints. EC-059/060/061 added; Token Budget ~88,100→~92,800; T-10/T-11 extended. No product-owner adjudication required (both BCs stable, no open questions). Story v4.3→v4.4. **Cluster-5 is now F3-FINALIZED and TDD-READY** — this closes the F3 gap that gated F4 TDD. Next: cluster-5 F4 TDD (stub-architect Red Gate → test-writer → implementer), with activation-boundary obligations [D-1232-OBL-1..4] applying before merge/activation. Input-hash: S-25.02 97cffb6→171c3bb (`compute-input-hash --update`, `--check` CLEAN). STORY-INDEX v4.473→v4.474 (catalog-row BC bracket + catalog-row/blockquote input-hash cites re-synced, POLICY 18 three-way parity VERIFIED). BC-INDEX UNCHANGED v5.96 (BC-1.18.010/011 cells already current at v1.9/v1.8). VP-INDEX/ARCH-INDEX/error-taxonomy/prd.md all UNCHANGED. `pipeline:` stays in_progress. Session Resume Checkpoint refreshed: cluster-5 F3 DONE, NEXT = cluster-5 F4 TDD; §4 item #4 active; items #1 (DEFERRED)/#5 UNCHANGED. OWED #2 unchanged (907-file hash sweep).** | S-25.02 F3→F4 | 2026-09-21 |
