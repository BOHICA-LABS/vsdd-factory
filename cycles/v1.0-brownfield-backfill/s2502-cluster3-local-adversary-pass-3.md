---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 3
verdict: NOT CLEAN
finding_count: 3
finding_breakdown: "1 HIGH + 1 MEDIUM + 1 MINOR + 1 non-blocking observation (ADVISORY-class; see Disposition Summary)"
streak: 0/3
adversary_model: claude-opus-4
review_date: 2026-09-10
diff_base: "5d195519"
diff_head: "5d195519"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "348863d"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-3

> Fresh-context adversarial review (Iron Law: fresh context, no prior-pass visibility beyond this
> cascade's own convention) of `feature/S-25.02-backfill` against BC-1.18.008 (mechanism-A one-time
> backfill-split) v1.3 and BC-1.18.007 (retention/compaction) v1.2, run after pass-2's fix-burst
> (`5d195519`, D-1192) landed on the branch. This is the third LOCAL BC-5.39.001 cascade pass run
> against the mechanism-A implementation; per the cluster-3 convention established at pass-1, the
> full Part A finding set is persisted here as a standalone artifact.

## Verdict

**NOT CLEAN** — 3 in-scope findings disposed this burst (F-C3-P3-001..003): 1 HIGH, 1 MEDIUM, 1
MINOR, plus 1 non-blocking observation recorded alongside the findings (folded into the Disposition
Summary, not separately numbered against BC-5.39.001's finding count). BC-5.39.001 LOCAL cluster-3
streak: **0/3 → 0/3** (still not clean; cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED,
separate track).

## Part A — Findings

### F-C3-P3-001 (HIGH) — Leading-preamble-bearing first shard can exceed `shard_cap_bytes`

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`,
`mechanism_a_partition_for_backfill` / `run_mechanism_a_backfill_split`.

**Finding:** pass-2's fix (F-C3-P2-003) seeded `partition_bytes` with the leading preamble's length
so the cap-flush DECISION correctly counts preamble bytes toward the first partition's budget.
However, once a real (non-empty) leading preamble is large enough that the preamble ALONE, plus the
first genuine record, would together exceed `shard_cap_bytes`, the packing procedure has no
sanctioned way to flush the preamble on its own — the greedy boundary-preserving packer only flushes
at a RECORD boundary, and the preamble is not a record. The result: the first sealed shard's
`bytes_at_seal` can exceed `shard_cap_bytes` with `oversized_record` left `false` (this is not
EC-002's single-oversized-record exception — the "oversized" content is a non-record preamble, not a
record), silently defeating Postcondition 2's cap bound for exactly the shard class pass-2's own
observation O-C3-P2-002 had already flagged as untested. Reachable against a hypothetical fifth
target artifact with a large banner/header block preceding its first record; not reachable against
the 4 real target artifacts today (none has a preamble anywhere near `shard_cap_bytes` in size), but
BC-1.18.008 Postcondition 6(c) makes no such carve-out and the packing procedure has no defined
behavior for this input shape.

**Impact:** the one gate class BC-1.18.008 Postcondition 2/6 exists to guarantee (every sealed
shard's `bytes_at_seal <= shard_cap_bytes` unless explicitly EC-002-flagged) has an unflagged,
undetected escape hatch for preamble-dominated content. Meets the HIGH bar (not BLOCKER: unreachable
against the 4 real target artifacts today; a genuine per-shard-cap violation nonetheless, and
silent).

**Disposition:** ROUTED to product-owner + implementer. product-owner amended **BC-1.18.008
v1.3→v1.4** (`v1.4` this burst): new **Leading-Preamble Handling Rule** — a first partition whose
accumulated preamble bytes alone reach or exceed `shard_cap_bytes` MUST be flushed as its own
preamble-only shard (no records included) before any record is packed into it, and that flush is
EXPLICITLY flagged in the shard index; new **Postcondition 6(c)** (fail-loud hard gate: every sealed
shard's `bytes_at_seal <= shard_cap_bytes` UNLESS flagged `oversized_record: true` — the
degenerate-preamble case is a SANCTIONED sub-case of this same flag, not a new one) and a matching
**Invariant 4** restatement scoping the per-shard-cap bound explicitly to cover the preamble-only
shard case; new EC-007 (large-but-under-cap preamble folds into the first record-bearing shard,
unflagged, per pass-2's existing behavior) and EC-008 (preamble alone at/over cap: preamble-only
shard sealed and flagged, degenerate-oversized-record semantics). Implementer (`10f49d1c`): added
`is_preamble_shard: bool` and `records: Vec<...>` fields to the partition/shard-index bookkeeping
(TD-VSDD-060 sibling-swept across every `ShardIndexEntry`/partition construction site touched by
this change), a preamble-only flush path executed before the greedy record-packing loop begins when
the preamble alone is at/over cap, and a new hard gate
`mechanism_a_verify_backfill_per_shard_cap_preserved` that explicitly checks every sealed shard's
`bytes_at_seal` against `shard_cap_bytes`, failing loud (not merely asserting the packing procedure
ran) unless the shard carries `oversized_record: true`. RED fixtures added (`16effd52`) covering a
preamble-at-cap partition (EC-008) and a preamble-under-cap partition folding into the first record
shard (EC-007); both exercised pre-fix (preamble-at-cap fails the new gate pre-fix, passes post-fix).

### F-C3-P3-002 (MEDIUM) — Empty-oracle fallback trusts the caller for any unrecognized artifact stem

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, the PC6(b)/set-equality oracle
cross-check added at pass-2 (F-C3-P2-001).

**Finding:** the set-equality gate's documented fallback — "when the oracle recognizes no boundary
at all for a given `artifact_stem`/content pair, the caller's offsets are trusted at face value" —
is implemented as a blanket `match`-style fallback keyed only on whether the oracle returned zero
boundaries, not on whether the `artifact_stem` is actually one of the 4 legitimate mechanism-A
targets. Any artifact_stem the oracle's record-boundary heuristics do not recognize (a typo'd stem, a
future 5th artifact not yet wired into the oracle, or a stem the oracle's marker-table has a gap for)
silently falls into the "trust the caller" fallback rather than failing loud — indistinguishable, from
the gate's own perspective, from the legitimately-empty-content case the fallback was designed for.

**Impact:** MEDIUM — narrows the PC6(b) gate's own trust boundary; a caller-supplied offsets list for
an unrecognized/mistyped artifact stem sails through with zero independent verification, silently
reintroducing the class of blind trust pass-1 and pass-2 spent two passes closing for the recognized
stems.

**Disposition:** ROUTED to implementer + test-writer. Fixed (`10f49d1c`): added
`is_known_mechanism_a_artifact_stem` (an explicit allow-list check against the 4 real target
stems — `decision-log`, `burst-log`, `lessons`, `session-checkpoints`), gating the empty-oracle
fallback so it only fires for a RECOGNIZED stem with genuinely no detected boundaries (e.g.
zero-record content); an unrecognized stem now returns
`MechanismABackfillError::UnrecognizedArtifactStem` fail-loud instead of silently trusting the
caller. test-writer (`16effd52`) rebuilt the existing F-004 fixtures — which had been using an
artifact_stem the oracle could not detect boundaries for, incidentally routing them through the
"trust the caller" fallback rather than exercising the real independent cross-check — to use a
stem/content pairing the oracle DOES detect, so the fixtures now exercise the genuine set-equality
cross-check rather than accidentally validating the (now-closed) blind-trust fallback path.

### F-C3-P3-003 (MINOR) — Two record-heading predicates over-match against marker-table siblings

**Location:** `crates/factory-dispatcher/src/shard_manager.rs`, `is_lesson_h2_record_heading` and
`is_pass_fix_burst_heading`.

**Finding:** both predicates use a loose `starts_with`/substring-style match against their respective
h2-heading prefixes, which over-matches a small number of legitimate sibling headings that share a
common prefix in BC-1.18.008's own marker-table (e.g. a `## LESSON` heading over-matching a
`## LESSON CATEGORY:` summary heading that is not itself a lesson record boundary; a `## PASS N FIX
BURST` heading over-matching an unrelated `## PASS NOTES` heading). Not reachable against the 4 real
target artifacts' actual current content (no such sibling headings exist in them today), but a
structurally-present over-match risk against the marker-table's own documented heading vocabulary.

**Impact:** MINOR — no reachable defect against real content today; a latent boundary-detection
precision gap that could silently over-split a shard if a sibling heading is ever introduced.

**Disposition:** ROUTED to implementer. Fixed (`10f49d1c`): both predicates tightened to match the
exact marker-table heading forms (anchored prefix plus a required following delimiter/whitespace
boundary, not a bare `starts_with`), closing the over-match against the documented sibling headings
while continuing to match every genuine record-boundary heading form in the marker table.

## Observations (non-blocking)

- **O-C3-P3-001** — `heal_or_confirm_already_migrated`'s structural-prefix heuristic (used to detect
  whether a target artifact has already been backfill-split on a prior, possibly-interrupted run)
  relies on a structural-prefix match against the expected post-split shard-index header shape rather
  than a stronger content-hash-based confirmation. Not reachable as a defect against the current
  single-operator, single-invocation activation model (F-C3-P1-006's own scope-boundary deferral to
  T-12 means this code path has no production caller yet), and no counterexample was constructed this
  pass. Recorded for completeness only — no test added, no fix made, no Drift Item opened (the
  function's actual load-bearing behavior is still owed the T-12 production-wiring review, which is
  the more precise place to re-examine this heuristic against a real invocation).

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P3-001 | HIGH | FIXED — BC-1.18.008 v1.3→v1.4 (Leading-Preamble Handling Rule, PC6(c), Invariant 4, EC-007/EC-008); `is_preamble_shard`/`records` fields + preamble-only flush + `mechanism_a_verify_backfill_per_shard_cap_preserved` hard gate | `feature/S-25.02-backfill` `10f49d1c` (RED `16effd52`) |
| F-C3-P3-002 | MEDIUM | FIXED — `is_known_mechanism_a_artifact_stem` allow-list gate on the empty-oracle fallback; F-004 fixtures rebuilt to be oracle-detectable | `feature/S-25.02-backfill` `10f49d1c` (RED `16effd52`) |
| F-C3-P3-003 | MINOR | FIXED — `is_lesson_h2_record_heading` + `is_pass_fix_burst_heading` tightened against marker-table sibling over-match | `feature/S-25.02-backfill` `10f49d1c` |
| O-C3-P3-001 | ADVISORY (observation) | No action — `heal_or_confirm_already_migrated` structural-prefix heuristic noted, not reachable today, deferred to T-12 production-wiring review | — |

## Code Gate (post-fix, this burst)

Full `cargo test --workspace --all-targets` suite: green (0 failed). `cargo fmt --check --all`:
clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean. Feature branch HEAD:
`feature/S-25.02-backfill` @ `10f49d1c` (RED-fixtures commit `16effd52` immediately prior), both
pushed to `origin`.

## Next

Cluster-3 LOCAL BC-5.39.001 cascade pass-4, fresh context, against BC-1.18.008 v1.4 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `10f49d1c`.
