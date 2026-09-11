---
document_type: behavioral-contract
level: L3
version: "1.1"
status: draft
producer: product-owner
timestamp: 2026-09-05T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-047-indeterminate-outcome-model-durable-mutation-marker-next-advance-gate.md
  - .factory/specs/architecture/decisions/ADR-039-validator-failure-policy-resource-exhaustion-fail-closed.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - plugins/vsdd-factory/hooks-registry.toml
input-hash: "3427627"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-07"
capability: "CAP-041"
lifecycle_status: draft
introduced: v1.0-brownfield-backfill
modified: []
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-7.08.001: Cohort B `failure_policy = fail-closed` Flip for `validate-burst-log` (Edit/Write/MultiEdit Arm Only), `regression-gate`, and `convergence-tracker` — Gated on Shard-Cap Enforcement AND Backfill-Split Completion

## Description

Once Layer 2's shard-cap gate is live and F4-locked (BC-1.18.005/BC-1.18.006) AND the mandatory
one-time backfill-split of the four pre-existing oversized cycle append-logs has completed
(BC-1.18.008), the three Cohort B validators — `validate-burst-log`'s
`event="PostToolUse" tool="^(Edit|Write|MultiEdit)$"` arm ONLY (its separate `tool="^Bash$"`
git-commit chain-detection arm is unrelated, exec-free, and explicitly out of Cohort B scope),
`regression-gate`, and `convergence-tracker` — are assigned `failure_policy = "fail-closed"` in
`hooks-registry.toml`. This is the corrected plugin identity (`validate-burst-log`, not the
non-existent `validate-burst-log-structure` cited in ADR-047 §8a pre-v1.6 and S-25.02's own
pre-correction AC-006) and the corrected sequencing (gated on the backfill-split completing, not
merely on the cap-check gate existing) that make this flip safe.

## Preconditions

1. BC-1.18.005's shard-cap formula constants are F4-locked (no longer provisional) for
   `burst-log.md` (read by `validate-burst-log`'s Edit/Write/MultiEdit arm AND `regression-gate`
   AND `convergence-tracker` — the Cross-Validator Minimum Rule's three-reader case) and for every
   other artifact `regression-gate`/`convergence-tracker` read that is in Layer-2 scope.
2. BC-1.18.006's roll-before-write gate is live in production — every future write to a
   mechanism-A sharded artifact is structurally bounded below `shard_cap_bytes`.
3. **BC-1.18.008's one-time backfill-split has COMPLETED for all four mechanism-A artifacts.**
   This is the hard sequencing gate ADR-051 Decision 9 and the F2 architecture-delta doc both
   flag explicitly: flipping fail-closed BEFORE the pre-existing oversized files are split would
   immediately re-trigger the exact INDETERMINATE loop this story exists to eliminate, because the
   Cohort B validators would still be reading multi-hundred-KB monolithic files on their very
   first post-flip invocation.
4. ADR-039 §Decision 3's calibration-precedes-fail-closed-flip ordering constraint is satisfied
   for each of the three validators independently: each validator's effective `fuel_cap` (or the
   shard-cap-bounded artifact size it now reads) has been measured against the shard-capped
   production corpus and verified sufficient for the largest post-split shard size.

## Postconditions

1. **`hooks-registry.toml` assigns `failure_policy = "fail-closed"` to exactly the three named
   Cohort B entries, using the CORRECTED plugin identity.** The `[[hooks]] name = "validate-burst-log"
   event = "PostToolUse" tool = "^(Edit|Write|MultiEdit)$"` entry (currently at priority 152,
   `path_allow = [".factory/cycles", ".factory/hooks"]`) gains `failure_policy = "fail-closed"`.
   The SIBLING `[[hooks]] name = "validate-burst-log" event = "PostToolUse" tool = "^Bash$"` entry
   (the git-commit chain-detection arm, same WASM binary, different `tool` pattern) is explicitly
   NOT touched by this BC — it remains at its current (absent/fail-open) `failure_policy`, per
   ADR-051's OQ-5 correction that this arm is exec-free, never scans file content, and is not part
   of the Cohort B fuel-exhaustion problem. `regression-gate` (`event = "PostToolUse"`, no `tool`
   filter, priority 230) and `convergence-tracker` (`event = "PostToolUse" tool =
   "^(Edit|Write|MultiEdit)$"`, priority 210, routed via `legacy-bash-adapter.wasm`, `async = true`)
   each independently gain `failure_policy = "fail-closed"` on their own single registry entry.

2. **The flip is gated on BC-1.18.005/006/008's postconditions holding, sequenced explicitly in
   `hooks-registry.toml` deployment order, not merely documented as a precondition.** The registry
   change that adds `failure_policy = "fail-closed"` to the three Cohort B entries MUST NOT be
   deployed in the same release as (or before) the shard-cap gate and the backfill-split; it is a
   SUBSEQUENT deployment, after production observation confirms the shard-capped corpus no longer
   produces `cause=fuel` INDETERMINATE events for these three plugins.

3. **Calibration evidence is recorded per ADR-039 §Decision 3, per validator.** For each of the
   three validators, the PR that performs this BC's registry flip MUST cite: the validator's
   measured fuel consumption against the largest POST-SPLIT shard size for every artifact it
   reads, and confirmation that this measurement falls comfortably under `PRACTICAL_FUEL_CEILING`
   (BC-1.18.005's F4-locked value) — mirroring the calibration-evidence discipline ADR-039 §Decision
   3 already establishes for Cohort A.

4. **Once flipped, `validate-burst-log`'s Edit/Write/MultiEdit arm, `regression-gate`, and
   `convergence-tracker` follow BC-1.18.001/002/003's fail-closed INDETERMINATE contract exactly
   like the three Cohort A validators — this BC does not define new INDETERMINATE-handling
   semantics.** An INDETERMINATE outcome (fuel exhaustion, epoch timeout, output-too-large) on any
   of these three now-fail-closed validators triggers: the `plugin.indeterminate` event, the
   durable marker write, and the next-advance gate (Arm 1 `^Agent$` PreToolUse block + Arm 2
   `git commit`/`git push` Bash PreToolUse block) — the SAME mechanism CAP-041/BC-1.18.001 already
   define for Cohort A. This BC's own scope is the flip and its sequencing gate, not a
   reimplementation of the INDETERMINATE-handling contract itself.

5. **Cohort B, once flipped, becomes a closed set of exactly three registry entries — the same
   "human-confirmed enumeration, no silent expansion" discipline ADR-047 §8a already established
   for Cohort A.** No OTHER currently-fail-open validator is flipped as a side effect of this BC;
   any future expansion of Cohort B (or a Cohort C) requires its own explicit BC and its own
   ADR-039 §Decision 3 calibration-confirmation gate.

## Invariants

1. **The Bash chain-detection arm of `validate-burst-log` is never fail-closed under this BC.**
   No implementation of this BC's flip may set `failure_policy = "fail-closed"` on the
   `tool = "^Bash$"` registry entry for `validate-burst-log` — doing so would be a scope violation
   of this BC's own Postcondition 1 and would fail-closed-gate an unrelated, exec-free,
   non-content-scanning validator that was never part of the fuel-exhaustion problem Layer 2
   solves.

2. **The flip's sequencing gate (Precondition 3, backfill-split completion) is a hard invariant,
   never relaxed for expedience.** No release may deploy this BC's registry change before
   BC-1.18.008's backfill-split has completed and been verified against the live corpus.

3. **Cohort B remains exactly three entries — closed, human-confirmed, enumerable.** Consistent
   with the existing Cohort A precedent (BC-1.18.004 Postcondition 4's "no other validator receives
   a fail-closed assignment... unless explicitly confirmed by the human").

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | The flip is attempted before BC-1.18.008's backfill-split has completed | Precondition 3 violated — this BC's flip MUST NOT be deployed; if attempted anyway, the immediate consequence is the exact self-inflicted-DoS scenario ADR-047 §8a's Cohort A/B partition was designed to prevent (the newly-fail-closed validators INDETERMINATE-block on the still-oversized legacy monolithic files on their very first post-flip invocation) |
| EC-002 | An implementer mistakenly flips `validate-burst-log`'s `^Bash$` arm instead of (or in addition to) its `^(Edit\|Write\|MultiEdit)$` arm | Scope violation of Postcondition 1/Invariant 1 — the `^Bash$` arm's chain-detection logic never scans file content and was never part of Cohort B; flipping it provides zero benefit and adds an unrelated fail-closed surface that could gate legitimate git-commit chain-detection failures unrelated to artifact size |
| EC-003 | `regression-gate` or `convergence-tracker`'s measured fuel consumption against the largest post-split shard is NOT comfortably under `PRACTICAL_FUEL_CEILING` (calibration fails for one of the three) | The flip proceeds for the two validators whose calibration passed; the failing validator's flip is deferred until its own calibration confirms sufficiency — this BC's three-validator enumeration does NOT require all three to flip atomically in lockstep if calibration evidence diverges per-validator |
| EC-004 | A future engine-discipline cycle proposes expanding Cohort B to a fourth validator | Out of this BC's scope — requires its own new BC and its own ADR-039 §Decision 3 calibration-confirmation gate, per Invariant 3's closed-cohort discipline |
| EC-005 | The S-25.02 story's own AC-006 body text still cites the non-existent `validate-burst-log-structure` name at the time story-writer next amends the story | Not fixed by this BC directly (story body is story-writer's domain per CLAUDE.md routing) — flagged as an F3 input for story-writer: correct AC-006's citation from `validate-burst-log-structure` to `validate-burst-log` (Edit/Write/MultiEdit arm) to match this BC's corrected identity |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| `validate-burst-log` Edit/Write/MultiEdit arm, post-backfill-split, fuel consumption measured at 60% of `PRACTICAL_FUEL_CEILING` against the largest post-split `burst-log.md` shard | Calibration passes; `failure_policy = "fail-closed"` assigned | happy-path |
| `validate-burst-log`'s `^Bash$` arm | `failure_policy` remains unchanged (absent/fail-open) — never touched by this BC | happy-path |
| Attempted flip deployment with BC-1.18.008 backfill-split incomplete | Deployment blocked/rejected per Precondition 3 and Invariant 2 | error |
| `regression-gate` fuel measured at 110% of `PRACTICAL_FUEL_CEILING` against a post-split fixture (calibration fails) | Flip deferred for `regression-gate`; the other two validators may still proceed independently (EC-003) | edge-case |
| A fourth validator (`purity-check`) proposed for Cohort B expansion without a new BC | Rejected — out of scope; requires its own BC-authorship and ADR-039 §D3 gate (EC-004) | error |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-129 | Bash-arm-exclusion invariant — `validate-burst-log`'s `^Bash$` registry entry's `failure_policy` is never set to `fail-closed` by this BC's implementation | static-check (config-diff on the `hooks-registry.toml` change: assert the `^Bash$` entry's `failure_policy` field is absent both before and after) |
| VP-130 | Sequencing-gate invariant — the Cohort B flip's registry change commit never lands before (or in the same commit as) BC-1.18.008's backfill-split completion evidence | integration test (CI or PR-template gate requiring the backfill-split PR's merge SHA to be an ancestor of the Cohort B flip PR) |
| VP-129 | Closed-cohort invariant — exactly three `[[hooks]]` entries carry `failure_policy = "fail-closed"` as an attributable consequence of this BC, matching the named set (`validate-burst-log` Edit/Write/MultiEdit arm, `regression-gate`, `convergence-tracker`) | static-check (config audit: grep `hooks-registry.toml` for `failure_policy = "fail-closed"`, cross-reference against BC-1.18.004's Cohort A enumeration plus this BC's Cohort B enumeration — no unattributed entries) |

**Fix-burst note (F-S2502-F2-003):** the closed-cohort row's Proof Method previously read bare
"config audit" without a leading category keyword; normalized to "static-check (config audit...)"
per VP-INDEX v3.02's authoritative VP-129 = static-check assignment, matching the sibling VP-129
row above. No property content changed.

## Related BCs

- BC-1.18.005 — the shard-cap formula whose F4-lock is a hard precondition for this BC's flip (depends on)
- BC-1.18.006 — the roll-before-write gate whose production liveness is a hard precondition (depends on)
- BC-1.18.008 — the one-time backfill-split whose COMPLETION is the hard sequencing gate this BC's own postconditions repeatedly emphasize (depends on)
- BC-1.18.001 — the Cohort A fail-closed INDETERMINATE contract this BC's flipped validators inherit unmodified once flipped (depends on)
- BC-1.18.002 — the next-advance gate (both arms) this BC's flipped validators trigger identically to Cohort A (depends on)
- BC-1.18.004 — the Cohort A backward-compatibility anchor and enumeration discipline this BC's Cohort B enumeration mirrors (related to)

## Architecture Anchors

- `plugins/vsdd-factory/hooks-registry.toml` — the three `[[hooks]]` entries (`validate-burst-log` Edit/Write/MultiEdit arm; `regression-gate`; `convergence-tracker`) this BC assigns `failure_policy = "fail-closed"` to
- `crates/factory-dispatcher/src/executor.rs` — `should_write_marker`/marker-write/gate-arming logic these three validators trigger identically to Cohort A once flipped (no new logic; BC-1.18.001/002 own this)

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -A6 'name = "validate-burst-log"' plugins/vsdd-factory/hooks-registry.toml | grep -E "^tool =|^priority ="
tool = "^(Edit|Write|MultiEdit)$"
priority = 152
tool = "^Bash$"
priority = 152
```

```
$ grep -A5 'name = "convergence-tracker"' plugins/vsdd-factory/hooks-registry.toml | grep -E "^priority =|^plugin ="
plugin = "hook-plugins/legacy-bash-adapter.wasm"
priority = 210
```

```
$ grep -A5 'name = "regression-gate"' plugins/vsdd-factory/hooks-registry.toml | grep -E "^priority =|^on_error ="
priority = 230
on_error = "continue"
```

Confirms the exact registry-entry identities, priorities, and (for `convergence-tracker`) the
`legacy-bash-adapter.wasm` routing this BC's Postcondition 1 cites verbatim, and confirms both
`validate-burst-log` registry entries (Edit/Write/MultiEdit and Bash arms) exist as distinct
entries sharing priority 152 — grounding Invariant 1's "the Bash arm is a SIBLING entry, not the
same entry" claim.

```
$ grep -oE "^pub fn write_indeterminate_marker|^pub fn should_write_marker" crates/factory-dispatcher/src/indeterminate_marker.rs
should_write_marker
write_indeterminate_marker
```

Confirms `should_write_marker`/`write_indeterminate_marker` exist as the marker-write logic this
BC's Postcondition 4 states its flipped validators trigger identically to Cohort A (no new logic
introduced by this BC).

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-129, VP-130 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-129 (static-check; Bash-arm exclusion + closed-cohort config audit), VP-130 (integration; sequencing gate after BC-1.18.008 backfill-split + per-validator calibration evidence). Fuel-ceiling numeric bound PROVISIONAL-until-F4.

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-041 |
| Capability Anchor Justification | CAP-041 ("Validation Integrity: INDETERMINATE Outcome, Durable Mutation Marker, and Next-Advance Gate") per capabilities.md §CAP-041 — this BC extends CAP-041's own text verbatim: "For plugins with `failure_policy = 'fail-closed'`..." to a SECOND human-confirmed cohort (Cohort B), using the exact same durable-marker/next-advance-gate mechanism CAP-041 defines; it does not define new INDETERMINATE semantics, only a second cohort assignment gated on CAP-043's shard-bounding postconditions holding. This BC is intentionally anchored to CAP-041 (the INDETERMINATE/fail-closed-assignment capability), not CAP-043 (the shard-sizing capability), because the registry `failure_policy` flip itself — not the shard-size bounding that makes it safe — is what this BC specifies; the shard-bounding precondition is satisfied by BC-1.18.005/006/008 (CAP-043) and cited here as a dependency, not restated as this BC's own capability anchor. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-07 (Hook Bash Layer — `hooks-registry.toml` routing-table `failure_policy` assignment) |
| ADR | ADR-051 §Decision 9 (Cohort B fail-closed flip sequencing, corrected); ADR-047 §Decision 8a (Cohort B partition, plugin-name corrected) and §Decision 8b (ratified future phase); ADR-039 §Decision 3 (calibration-precedes-fail-closed-flip ordering constraint) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-009 + F-S2502-F2-003 + F-S2502-F2-007): Traceability ADR row's `ADR-047 §Decision 8a (Cohort B partition, plugin-name corrected v1.6)` de-loaded of its volatile `v1.6` version pin per TD-VSDD-091/POLICY 19 — the stable `§Decision 8a` section anchor already resolves the citation without the version token, mirroring the CAP-032/`ADR-026` precedent. VP-129's closed-cohort row Proof Method normalized from bare "config audit" to "static-check (config audit...)" per VP-INDEX v3.02's authoritative method assignment (both VP-129 rows now consistently read "static-check") — no property content changed. Added `## SDK Grounding Evidence` section with literal stable-anchor grep output for the three Cohort B registry entries and the marker-write primitives. No postcondition/invariant/scope change. |
| 1.0 | 2026-09-05 | product-owner | Initial creation. F2 spec-evolution burst, S-25.02 activation. Allocated as BC-7.08.001 (next free SS-07 family slot after the existing BC-7.01–BC-7.07 range; confirmed against BC-INDEX.md and the live `ss-07/` directory listing at authoring time — no collision). Cohort B fail-closed flip for the CORRECTED plugin name `validate-burst-log` (Edit/Write/MultiEdit arm only — explicitly excluding the unrelated `^Bash$` chain-detection arm), `regression-gate`, `convergence-tracker`; hard-gated on BC-1.18.005/006/008's postconditions holding (shard cap enforced, gate live, AND the pre-existing oversized files already backfill-split) per ADR-051 Decision 9's corrected sequencing. CAP-041 capability anchor (extends the INDETERMINATE/fail-closed cohort mechanism to a second, human-confirmed set). ADR-051 §D9 + ADR-047 §D8a/§D8b + ADR-039 §D3 citations. |
