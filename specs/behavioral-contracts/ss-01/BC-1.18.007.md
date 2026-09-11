---
document_type: behavioral-contract
level: L3
version: "1.2"
status: active
producer: product-owner
timestamp: 2026-09-06T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/architecture/decisions/ADR-047-indeterminate-outcome-model-durable-mutation-marker-next-advance-gate.md
  - .factory/specs/domain-spec/capabilities.md
input-hash: "63e1688"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: active
introduced: v1.0-brownfield-backfill
modified: []
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.007: Shard Retention/Compaction — Configurable Retention Count, Archive Relocation, and Honest O(Active-Shards) Whole-Corpus Accounting

## Description

Shard count per artifact is unbounded absent compaction. This BC specifies the mandatory
retention/compaction companion policy (AC-005): sealed shards beyond a configurable retention
count are relocated to an `archive/` subdirectory — remaining `path_allow`-visible under the
existing directory-scoped globs but excluded from the default whole-corpus glob scope — so that
default whole-corpus validators are honestly `O(active shards)`, not `O(1)` and not
`O(all shards ever)`.

## Preconditions

1. A seal event has occurred for a mechanism-A sharded artifact (BC-1.18.006 Postcondition 1),
   producing a new `[[shard]]` entry in that artifact's shard-index TOML.
2. The shard-index's `[[shard]]` sequence, after the new entry is appended, contains more entries
   than the configured `retention_count`.

## Postconditions

1. **Retention count is a config value, not a hardcoded constant.** The shard-index TOML
   (BC-1.18.006 Postcondition 5 schema) gains a `retention_count` field in its top-level table
   (default value: 10 most-recent shards — a round, human-adjustable number per ADR-051 Decision
   6). This BC's implementation MUST read `retention_count` from the shard-index, never
   hardcoding `10` into `shard_manager.rs`'s compaction logic.

2. **Archival relocation, triggered in the same native-gate invocation as the seal that crossed
   the threshold.** When the `[[shard]]` sequence (after a new seal) exceeds `retention_count`,
   the OLDEST sealed shard(s) beyond the retention window are moved (not deleted) from
   `.factory/cycles/<cycle>/<sealed-filename>` to
   `.factory/cycles/<cycle>/archive/<artifact-stem>/<sealed-filename>`, in the SAME native-gate
   invocation that performed the triggering seal (composing with BC-1.18.006 Postcondition 4's
   same-invocation atomicity guarantee — the archive move, the new seal, and the index update all
   land in the same subsequent factory-artifacts commit).

3. **Archived shards remain `path_allow`-visible; excluded from default whole-corpus glob scope.**
   Because `archive/<artifact-stem>/` is still a subdirectory of `.factory/cycles/<cycle>/`, every
   existing directory-scoped `path_allow` glob (e.g. `.factory/cycles`) continues to match archived
   shards transparently — no validator's capability declaration needs amendment (composing with
   the F1 delta analysis's own confirmed LOW-risk finding for directory-scoped globs). However,
   the DEFAULT whole-corpus glob pattern (`<stem>*.md` at the cycle root, per BC-1.18.006
   Postcondition 6) does NOT match `archive/<artifact-stem>/<stem>*.md` — a whole-corpus reader
   that wants archived history MUST explicitly opt in with a second glob pass
   (`archive/<artifact-stem>/<stem>*.md`) or consult the shard-index's `[[shard]]` entries
   directly (every entry, active or archived, remains listed in the index — archival is a file
   relocation, not an index-record deletion).

4. **Whole-corpus validators operating without the archive opt-in are honestly
   `O(active shards)`.** The default (non-opted-in) whole-corpus glob scope is bounded by
   `retention_count` per artifact, not unbounded. This is the honest accounting ADR-047 §8b
   requires: a whole-corpus validator's cost does not grow without bound as an artifact's total
   historical shard count grows, so long as it does not opt into `archive/` inclusion.

5. **Archive format is identical to the active-shard format — no compression, no schema change.**
   An archived shard file is byte-for-byte identical to its pre-archival sealed form; only its
   directory changes. This keeps `grep`/`awk`-based tooling (including the F4 calibration
   harness's own full-repository line-length sweep, BC-1.18.005 Postcondition 7) working
   unmodified against archived shards when explicitly globbed.

6. **POLICY-1 (`append_only_numbering`) enforcement carve-out: an archive-INCLUSIVE whole-corpus
   scan is MANDATORY, not opt-in (S-25.02 F3→F4 consistency gate finding F-1, MAJOR; ADR-051
   §Decision 6, subsection "Whole-corpus history-scanning validator enumeration and POLICY-1
   reconciliation").** Notwithstanding Postcondition 3's general "opt-in required for `archive/`"
   default, any D-NNN/BC/VP/story-ID append-only, gap, or uniqueness audit performed against a
   Layer-2-sharded artifact under `.factory/policies.yaml` POLICY-1 (`append_only_numbering`) MUST
   use the ARCHIVE-INCLUSIVE glob — `<stem>*.md` UNION `archive/<artifact-stem>/<stem>*.md` — never
   Postcondition 3's default-exclusion glob. This is an explicit, named carve-out, justified because
   append-only-numbering integrity is inherently a whole-HISTORY property: an ID whose sole prior
   occurrence has aged into `archive/<artifact-stem>/` MUST remain visible to reuse/gap/uniqueness
   detection, unlike the general whole-corpus validators covered by Postconditions 3/4, whose
   correctness concern is bounded to current/recent state (ADR-051's own distinction between
   "the current/recent state" validators and "the complete historical ID space" audit).
   **Audited surface (enumerated, not assumed):** the four SS-04-owned WASM crates
   (`validate-dispatch-advance`, `validate-state-structure`, `validate-closes-completeness`,
   `validate-cross-site-correspondence`) were each individually confirmed NOT to require
   amendment — each is either STATE.md-scoped (never reading the sharded cycle artifact at all) or
   already correctly current-shard-scoped by design (its correctness concern is the shard being
   written, not historical enumeration). POLICY-1's own `consistency-validator`/adversary-prompt
   enforcement path is the ONE genuine archive-caused whole-corpus gap this carve-out closes.
   **Present-day enforcement mechanism.** Because POLICY-1 has `lint_hook: null` in
   `.factory/policies.yaml` today (no automated WASM/native validator implements it), this
   Postcondition is currently satisfied at the agent-instruction level: the `consistency-validator`
   and adversary-prompt agent definitions, and POLICY-1's own `verification_steps` entry in
   `.factory/policies.yaml`, MUST instruct "scan MUST include `archive/<artifact-stem>/` for any
   Layer-2-sharded artifact, not just active shards." If POLICY-1 is ever automated into a
   WASM/native hook, that hook's design MUST inherit this archive-inclusive default from day one —
   this Postcondition binds both the present agent-level enforcement and any future automated
   enforcement identically.

## Invariants

1. **Archival never deletes data.** The retention/compaction mechanism moves files; it never
   truncates, compresses (lossily), or deletes a sealed shard's content. Full history remains
   recoverable from git history even if a future policy change were to actually delete archived
   shards (out of this BC's scope — this BC's mechanism is move-only).

2. **`retention_count` is per-artifact, not global.** Each of the four mechanism-A artifacts
   (`decision-log.md`, `burst-log.md`, `lessons.md`, `session-checkpoints.md`) maintains its own
   independent shard-index and its own independent `retention_count` value and archival state —
   archiving one artifact's oldest shards has no effect on another artifact's active/archived
   partition.

3. **The shard-index is the single source of truth for "how many total shards has this artifact
   ever had," regardless of archival state.** Archiving a shard does not remove its `[[shard]]`
   entry from the index; it only updates the entry to reflect the new archived path (or adds an
   `archived: true` boolean alongside the unchanged `path` field — implementation detail, but the
   entry MUST remain enumerable). A count derived purely from the shard-index's `[[shard]]`
   sequence length is always the artifact's true lifetime shard count.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Shard count reaches retention limit exactly (story's own EC-002) | Oldest shard archived per this BC's retention/compaction policy — the archival move happens in the SAME invocation as the seal that crossed the threshold (Postcondition 2) |
| EC-002 | `retention_count` is decreased by a human editing the shard-index (e.g., 10 → 5) after several artifacts already have 8 active shards | The NEXT seal event that reads the shard-index observes `active_count (8) > new retention_count (5)` and archives enough of the oldest shards to bring the active count back within the new limit (potentially archiving more than one shard in a single invocation) |
| EC-003 | A whole-corpus validator globs `<stem>*.md` without the archive opt-in, on an artifact with archived history | Only active (non-archived) shards plus the current file are matched — archived shards under `archive/<artifact-stem>/` are silently excluded, per Postcondition 3's explicit-opt-in design (not a defect; this is the intended honest-`O(active)` behavior) |
| EC-004 | An operator manually moves a shard file out of `archive/` back to the cycle root | Not a supported operation under this BC's contract; the shard-index's recorded path would then diverge from the file's actual location — flagged as a manual-intervention risk, not a mechanism defect (consistent with the project's existing operator-escape-hatch posture, e.g. BC-1.18.003's manual marker deletion, which is explicitly supported and audited; unarchival is NOT similarly audited by this BC and is out of scope) |
| EC-005 | The shard-index is missing or corrupt at the moment a retention check would run (companion to the story's own EC-005) | Fail-loud: the native gate returns `HookResult::Error`; the triggering write is blocked; dispatch is halted for that artifact until the operator restores the index from git history — retention/compaction MUST NOT silently skip its check and proceed as if no archival were needed |
| EC-006 | A POLICY-1 (`append_only_numbering`) audit runs against an artifact with archived history, where a D-NNN/BC/VP/story-ID's SOLE prior occurrence has aged into `archive/<artifact-stem>/` | The audit MUST use the archive-inclusive glob (Postcondition 6) and detect the ID's prior occurrence there; a subsequent attempt to reuse that same ID IS flagged as a POLICY-1 violation, never silently missed — contrast with EC-003, where a generic non-POLICY-1 whole-corpus validator correctly stays opt-in-excluded from the same archived shard |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| Artifact with 10 active shards (`retention_count=10`), 11th seal occurs | Shard #1 (oldest) moved to `archive/<stem>/`; shard-index `[[shard]]` entry for #1 updated to reflect the new path; active count returns to 10 | happy-path |
| Artifact with 5 active shards, `retention_count=10` | No archival triggered; all 5 remain in the cycle root | happy-path |
| `retention_count` manually lowered from 10 to 3 while 8 shards are active | Next seal archives 6 shards (bringing active count to 3, including the new seal) in one invocation | edge-case |
| Whole-corpus `grep -c "D-" decision-log*.md` on an artifact with 3 archived + 2 active shards | Matches only the 2 active shards + current file (3 files total); archived shards NOT matched | edge-case |
| Whole-corpus `grep -c "D-" decision-log*.md archive/decision-log/decision-log*.md` (explicit opt-in) | Matches all 5 sealed shards + current file (6 files total) | happy-path |
| Shard-index file corrupt (malformed TOML) at retention-check time | `HookResult::Error`; write blocked; no archival attempted | error |
| POLICY-1 uniqueness audit on an artifact where `D-042` exists ONLY inside `archive/decision-log/decision-log.0003.md`; a new write attempts to reuse `D-042` | Archive-inclusive glob (Postcondition 6) finds the existing `D-042` occurrence in the archived shard; the reuse attempt is flagged as a POLICY-1 (`append_only_numbering`) violation | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-121 | Bounded-active-count invariant — after any sequence of seals, the number of NON-archived `[[shard]]` entries for a given artifact never exceeds `retention_count` | proptest (arbitrary seal sequences; property: active count `<= retention_count` after every seal) |
| VP-121 | No-data-loss invariant — every `[[shard]]` entry ever created remains present (active or archived) in the shard-index; none are ever removed | proptest (arbitrary seal + archival sequences; property: `len(shard_index.shards)` is monotonically non-decreasing) |
| VP-122 | Default-glob exclusion invariant — the default whole-corpus glob pattern never matches a path under `archive/<artifact-stem>/` | unit test (glob-matching assertion against a fixture directory tree with both active and archived shards) |
| VP-141 | Archive-inclusive POLICY-1 scan-mode invariant — a POLICY-1 (`append_only_numbering`) append-only/gap/uniqueness audit's effective glob scope ALWAYS includes `archive/<artifact-stem>/` UNION active shards for a Layer-2-sharded artifact; it never silently excludes an ID whose sole occurrence has been archived (Postcondition 6) | integration + static-check (dual-leg VP-141: integration/fixture-based assertion over a synthetic active+archive shard tree — archived-only ID + attempted reuse → POLICY-1 violation detected, per this BC's Canonical Test Vector — plus a static/compliance check that the `consistency-validator`/adversary-prompt agent definitions and POLICY-1's `verification_steps` in `.factory/policies.yaml` specify archive-inclusive scanning, since no WASM/native hook implements POLICY-1 today) |

## Related BCs

- BC-1.18.006 — this BC's archival trigger fires in the same invocation as BC-1.18.006's seal (depends on)
- BC-1.18.005 — the shard cap that determines seal frequency, which in turn determines archival frequency (related to)
- BC-1.18.008 — the one-time backfill-split's initial shard set is subject to this BC's SAME retention policy once it exceeds `retention_count` (composes with)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — archival-move logic, `retention_count` read from shard-index config
- `crates/last-amended-migrate/src/atomic_write.rs` / `crates/factory-dispatcher/src/indeterminate_marker.rs` — atomic-write primitives reused for the shard-index update accompanying an archival move
- `.factory/policies.yaml` (POLICY-1, `append_only_numbering`, `lint_hook: null`) and the
  `consistency-validator`/adversary-prompt agent definitions — Postcondition 6's present-day
  enforcement surface (agent-instruction level; no automated WASM/native hook implements POLICY-1
  today). The four audited-and-cleared SS-04 crates (`crates/hook-plugins/validate-dispatch-advance`,
  `validate-state-structure`, `validate-closes-completeness`, `validate-cross-site-correspondence`)
  are NOT Architecture Anchors for this Postcondition — they were confirmed out of scope, not
  amended.

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

```
$ grep -oE "^pub fn write_indeterminate_marker" crates/factory-dispatcher/src/indeterminate_marker.rs
pub fn write_indeterminate_marker
```

Confirms both existing atomic-write primitives this BC's Architecture Anchors name as reuse
candidates for the shard-index update that accompanies an archival move.

```
$ grep -oE "^pub enum HookResult" crates/hook-sdk/src/result.rs
pub enum HookResult
```

Confirms `HookResult::Error` (EC-005's fail-loud outcome on a missing/corrupt shard-index) is a
real variant of the SDK contract this BC's error-path edge case relies on.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-121, VP-122 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-121 (proptest; bounded active count + no-data-loss / honest O(active)), VP-122 (unit-test; default-glob archive exclusion + fail-loud missing/corrupt index E-SHD-002).
- VP-141 — allocated by formal-verifier (S-25.02 F3→F4 consistency-gate finding F-1 closure; VP-INDEX v3.07). VP-141 (integration + static-check; Postcondition 6/EC-006 — archive-inclusive POLICY-1 (`append_only_numbering`) scan-mode invariant: a POLICY-1 append-only/gap/uniqueness audit's effective glob scope ALWAYS includes `archive/<artifact-stem>/` UNION active shards for a Layer-2-sharded artifact, never silently excluding an ID whose sole occurrence has been archived, per ADR-051 §Decision 6's "Whole-corpus history-scanning validator enumeration and POLICY-1 reconciliation" subsection). Back-reference added here (formal-verifier's VP-141 body already cited this BC in `source_bc`; this BC's own §Verification Properties table already cited VP-141, but this §VP Anchors list did not yet cite it back — gap closed here, reference-only, no behavior change).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies the retention/compaction companion policy the capability's own outcome statement presupposes (bounded, honestly-accounted shard growth), consistent with the ADR-047 §8b "honest shard count accounting" mandate CAP-043 cites as source. |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` retention/archival logic) |
| ADR | ADR-051 §Decision 6 (retention/compaction companion policy: retention count, archive path, opt-in whole-corpus scope; subsection "Whole-corpus history-scanning validator enumeration and POLICY-1 reconciliation" grounds Postcondition 6's archive-inclusive POLICY-1 carve-out) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.2 | 2026-09-06 | product-owner | Fix-burst amendment (S-25.02 F3→F4 consistency gate finding F-1, MAJOR): added Postcondition 6 — POLICY-1 (`append_only_numbering`) enforcement MUST use an archive-INCLUSIVE whole-corpus scan, an explicit carve-out from Postcondition 3's opt-in-required default, naming the four audited-and-cleared SS-04 crates and the genuine `consistency-validator`/adversary-prompt-level gap, per ADR-051 §Decision 6's "Whole-corpus history-scanning validator enumeration and POLICY-1 reconciliation" subsection. Added EC-006 (archive-inclusive POLICY-1 detection) + 1 new Canonical Test Vector + 1 new Verification Properties row (VP-NNN, pending formal-verifier allocation). Architecture Anchors gained the POLICY-1/agent-definition enforcement surface. Traceability ADR row extended to cite the specific ADR-051 §Decision 6 subsection. Closes the S-25.02 story AC-012 mis-citation gap (AC-012 previously cited Postcondition 3, which does NOT establish the archive-inclusive obligation — story-writer re-points AC-012 to Postcondition 6 next). |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-007, POLICY 5): added `## SDK Grounding Evidence` section with literal stable-anchor grep output for `write_atomic`, `write_indeterminate_marker`, and `HookResult`. No postcondition/invariant/VP content change. |
| 1.0 | 2026-09-05 | product-owner | Initial creation. F2 spec-evolution burst, S-25.02 activation. Configurable `retention_count`, same-invocation archival move, `path_allow`-preserved-but-default-glob-excluded archive scope, honest O(active-shards) accounting per ADR-047 §8b mandate. CAP-043 capability anchor. ADR-051 §D6 citation. |
