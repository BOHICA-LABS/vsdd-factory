---
document_type: behavioral-contract
level: L3
version: "1.17"
status: active
producer: product-owner
timestamp: 2026-09-07T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-047-indeterminate-outcome-model-durable-mutation-marker-next-advance-gate.md
  - .factory/specs/architecture/decisions/ADR-042-validate-cross-site-correspondence-fuel-budget-raise-and-loud-exhaustion-signaling.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f1-delta-analysis.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md
  - .factory/specs/domain-spec/capabilities.md
  - .factory/specs/verification-properties/VP-INDEX.md
input-hash: "8eaa97a"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: active
last_amended: "2026-10-14 (v1.17) — ADR-052 v1.33 (§Decision 11(g)(2)(f) \"Scope of the tracing-field half\" (2); §Decision 11(c) point 7(e) row (11)(ii)): Postcondition 10 gains the scoping sentence for the tracing-field half; NEW Postcondition 11 (a config `artifact_stem` failing `is_safe_path_component` is refused at shard-config decode with `ShardConfigError::ArtifactStemUnsafe`, rendered per slot under Postcondition 10); NEW EC-024 and 4 Canonical Test Vectors; see changelog row 1.17. Prior: 2026-10-13 (v1.16) — ADR-052 §Decision 11(g)(2) sanitization extended to `ShardConfigError` (S-25.09 black-box finding): new Postcondition 10 (every config-derived string slot of every `ShardConfigError` Display escaped and capped at 256; `Toml` renders category plus line/column only, never the toml message or source line; holds on the dispatcher `block_reason`, the `migrate-bc-index` stderr via `ShardCapConfigUnavailable` and any log) and new EC-023. Same version (ADR-052 §Decision 11(g)(2)(f)): Postcondition 10 gains the 64-per-slot rule for InternalLog/tracing event fields with `Display` and event rendering from one exhaustive `render_with_cap` source (no wildcard arm), the `Toml` variant storing only a closed category token plus line and column (never the toml error object; `#[from]` becomes a constructor), and `ShardCapConfigUnavailable.detail` built from the sanitized rendering; EC-023 gains vectors (f)-(h). Prior: 2026-09-08 (v1.15)."
introduced: v1.0-brownfield-backfill
modified: ["2026-10-13 (v1.16)", "2026-10-14 (v1.17)"]
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.005: Byte-Size-Denominated Shard-Cap Formula and Native Deterministic Size-Trigger (Provisional Constants Pending F4 Harness Calibration)

## Description

A native (non-WASM) dispatcher function computes, for every registered sharded artifact, a
`shard_cap_bytes` ceiling from four calibrated inputs (`PRACTICAL_FUEL_CEILING`,
`WORST_CASE_FUEL_PER_BYTE`, `MAX_SINGLE_RECORD_BYTES`, `SAFETY_MARGIN`) and performs a `stat()`-only
byte-size check — never a fuel-budgeted content read — before every `Edit`/`Write`/`MultiEdit`
against a matched artifact. This is the deterministic, mechanistic size-trigger AC-001/AC-002 of
S-25.02 require: no LLM-side awareness of shard size is required or permitted, and the check itself
cannot exhaust a fuel budget because it has none (native code, not a WASM plugin). This BC owns
BOTH trigger shapes the native gate dispatches on: mechanism A's byte-size-denominated `stat()`-only
trigger (Postconditions 1-7) AND mechanism B1's item-count-denominated trigger (Postcondition 8,
added in this fix-burst per ADR-051 v1.1 Decision 1's "Trigger-shape dispatch" amendment,
F-S2502-F2-005).

## Preconditions

1. The dispatcher's PreToolUse handling path for `Edit`/`Write`/`MultiEdit` tool calls is reached.
   **Invocation point (UPDATED, PR #824 cycle-3 MAJOR-3, ADR-051 §Decision 17):** `shard_cap_precheck`
   is invoked from `main::run`, before the `sync_tiers.is_empty() && partition.async_group.is_empty()`
   early-return guard — hoisted from its prior call site inside `executor.rs::execute_tiers` (which
   `main::run` skips entirely whenever no plugin matched, silently defeating this gate for exactly
   that configuration). `shard_cap_precheck` itself remains DEFINED in `executor.rs`; only its call
   site moved. This is the SAME placement rule ADR-051 §Decision 1 already establishes (a native,
   non-registry-gated check must run before any guard that could skip it) — §Decision 17 documents
   the hoist as a new addendum, not a change to §Decision 1's underlying rule. See Architecture
   Anchors below.
2. A `[[shard]]` config table (`hooks-registry.toml` or a sibling `shard-config.toml`, TBD at F4)
   exists naming the target artifact's stem, its cap-formula inputs, its current `shard_cap_bytes`,
   and (per Postcondition 8) a `shape` field declaring the artifact's trigger shape (`"flat"` or
   `"frontmatter-changelog-array"`).
3. The tool call's target path resolves against an entry in the `[[shard]]` config (if it does
   not, this BC's check is a no-op — see Postcondition 1).

## Postconditions

1. **Zero-cost bypass for unmatched paths — scoped to the TARGET ARTIFACT (scope clarified, S-25.02
   Phase F4 LOCAL adversary cluster-1 pass-3 finding F-C1-P3-003, OBSERVATION, product-owner
   adjudication, v1.9).** If the tool call's target path does not match any `[[shard]]` config
   entry, the native check returns `Continue` immediately, performing no `stat()` call against the
   TARGET ARTIFACT and no arithmetic. This preserves zero added latency for the ~99% of
   `Edit`/`Write`/`MultiEdit` calls Layer 2 does not touch (ADR-051 Decision 1 step 1). This
   guarantee is scoped to the target artifact's own file specifically; it does NOT forbid the
   necessary one-time applicability probe against the FIXED `[[shard]]` config file itself
   (`Path::exists()` against a fixed, small config path — e.g. `.factory/shard-config.toml`) that
   determines WHETHER this BC's gate is even a candidate for the current dispatch at all. That
   config-presence probe is O(1), fixed-size regardless of the target artifact, reads no target-
   artifact content, and — being native dispatcher code — is fuel-free; it is the minimal necessary
   applicability check, not a violation of the zero-cost bypass this postcondition protects
   (Postcondition 2's stat()-only-for-matched-paths and Invariant 3's identically-scoped guarantee
   apply this same target-artifact-specific reading).

   **Blast-radius scoping — entry VALIDATION is scoped to the MATCHED entry only, never eager
   across the whole config (CORRECTED, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding
   F-C1-P6-001, LOW pending-intent, product-owner adjudication, v1.12).** The sanctioned
   implementation's `shard_cap_precheck` (`crates/factory-dispatcher/src/executor.rs`) calls
   `ShardRegistry::load()` — which PARSES and semantically VALIDATES (fail-loud on EC-009/EC-011/
   EC-013/EC-015/EC-016/EC-017) EVERY `[[shard]]` entry in the config — on EVERY `Edit`/`Write`/
   `MultiEdit` dispatch whose config file exists, BEFORE `shard_cap_gate_check` matches the target
   path to a specific entry. Consequence: a SINGLE semantically malformed sibling entry (e.g. a
   typo omitting `shape` on an unrelated `lessons` entry) causes `HookResult::Error` for EVERY
   `Edit`/`Write`/`MultiEdit` dispatch in the repository whose config file exists — INCLUDING
   dispatches whose target path matches NO `[[shard]]` entry at all (e.g. an edit to
   `src/foo.rs`), or matches a DIFFERENT, well-formed entry. This is in tension with this
   Postcondition's own "no arithmetic" / "zero-cost bypass" guarantee for unmatched paths: eager
   whole-config validation performs real arithmetic (finiteness checks, divisions, comparisons) —
   and, worse, can fail-loud-BLOCK — for dispatches this Postcondition promises are entirely
   unaffected by Layer 2. **Two candidate readings were weighed: (A) EAGER FAIL-FAST — a malformed
   config is an operator error that should block ALL matched-tool dispatches until fixed, ops-
   hygiene style; vs. (B) MATCH-FIRST — this Postcondition's zero-cost/zero-blast-radius promise
   should hold per-artifact: an unmatched (or differently-matched) target returns `Continue` (or
   proceeds against its OWN correctly-matched entry) even if a SIBLING entry is malformed; only the
   MATCHED entry is ever validated.** **ADJUDICATED (B) MATCH-FIRST, over (A) EAGER FAIL-FAST:**
   - This Postcondition's own literal text ("no arithmetic" for unmatched paths) already leans B —
     eager validation performs arithmetic (and can outright block) for paths this Postcondition
     says pay zero cost, a strictly stronger violation than the O(1) config-presence probe the
     v1.9 scoping clarification exempted.
   - The blast radius is disproportionate to the actual risk: a malformed entry for artifact Z
     threatens only Z's own fuel-exhaustion protection (Related BCs' BC-1.18.001 concern) — it
     creates no risk whatsoever for an edit to `src/foo.rs` or to a DIFFERENT, well-formed entry's
     artifact. Halting ALL repository editing because ONE sharded artifact's config entry has a
     typo is a basic blast-radius-containment failure, not legitimate ops hygiene — it inverts this
     BC's own purpose (BC-1.18.005 exists to PREVENT a fuel-exhaustion catastrophe from cascading
     across unrelated validators; it must not itself become a mechanism by which one config typo
     cascades across unrelated files).
   - Match-first does NOT weaken any existing fail-loud guarantee: when a dispatch DOES target the
     malformed entry's own artifact, entry validation still runs (now at match-time, immediately
     before `shard_cap_gate_check`'s trigger-boundary logic) and still fails loud — the "before any
     write is ever attempted against the artifact" guarantee each of EC-009/EC-011/EC-012/EC-013/
     EC-015/EC-016/EC-017 and Postcondition 8's N-presence bullet already establish is UNCHANGED;
     only the SCOPE narrows from "every entry in the file, every dispatch" to "the one entry the
     current dispatch targets."
   - **Terminology re-grounding, effective v1.12:** `ShardRegistry::load()` now denotes STRUCTURAL
     TOML deserialization only (`toml::from_str`) — unavoidably whole-file, since a `Vec<ShardEntry>`
     cannot partially deserialize (this is the "some parse per dispatch is unavoidable" cost F-C1-
     P6-001 itself acknowledges: `find_matching_entry` needs every entry's `artifact_stem`, which
     requires a successful structural parse of the whole file). A NEW function, `validate_entry
     (&ShardEntry) -> Result<(), ShardConfigError>`, carries the SEMANTIC checks previously inlined
     in `load()`'s per-entry loop (EC-009's `shape` presence, EC-022's `artifact_path`
     non-emptiness, EC-015/EC-017's `worst_case_fuel_per_
     byte` divisor-door guards, Postcondition 9/EC-013's cap-vs-formula comparison, EC-016's `N`
     presence, EC-011/EC-012's `low_water_mark` range/advisory). `shard_cap_precheck`/
     `shard_cap_gate_check` MUST call `find_matching_entry` FIRST; if it returns `None`, this
     Postcondition's `Continue` fires WITHOUT calling `validate_entry` on ANY entry (not the
     unmatched-for entry, not any sibling). If it returns `Some(entry)`, `validate_entry(entry)`
     runs on THAT entry alone, before `shard_cap_gate_check`'s shape-dispatch/trigger-boundary logic
     — a `validate_entry` failure returns `HookResult::Error` exactly as `load()`'s eager loop did
     for that same entry today; sibling entries are never touched. **Every downstream reference in
     this BC to "`ShardRegistry::load()` MUST... for EVERY `[[shard]]` entry... fail loud" or "at
     load time"** — Postcondition 8's N-presence bullet, Postcondition 9, and EC-009/EC-011/EC-012/
     EC-013/EC-015/EC-016/EC-017 — **is retroactively read, from v1.12 forward, as "for the entry
     matched against the current dispatch's target path, evaluated at entry-match time (immediately
     after `find_matching_entry` succeeds)"** — NOT eagerly for every entry in the file regardless
     of match. This changes the ENTRY-SCOPE of each cited check from all-entries-in-the-file to the
     one-matched-entry; it does NOT change any cited check's substantive fail-loud outcome, its
     numeric thresholds, or its "before any write against that artifact" timing guarantee.
   - **Residual, unavoidable whole-file exception (NOT weakened by this ruling):** a `[[shard]]`
     config file that fails STRUCTURAL TOML deserialization — invalid TOML syntax, or ANY entry
     omitting a non-`Option`-typed field required for `toml::from_str` to succeed at all (i.e.
     `artifact_stem`, `shard_cap_bytes`, or any of the four `cap_formula_inputs` fields —
     `practical_fuel_ceiling`, `worst_case_fuel_per_byte`, `max_single_record_bytes`,
     `safety_margin`) — STILL propagates `HookResult::Error` for EVERY `Edit`/`Write`/`MultiEdit`
     dispatch while the config file exists, matched or not, because `find_matching_entry` cannot
     run at all without a successfully deserialized registry. This is inherent to TOML's whole-file
     grammar, not a validation-eagerness design choice this ruling controls, and is explicitly OUT
     OF SCOPE of the (A)-vs-(B) adjudication above — it is the one case where (A)'s blast radius is
     unavoidable rather than a choice. See EC-019.

   **Empty-`artifact_path` structural defect guard (NEW, PR #818 cycle-4 fresh-context PR reviewer
   finding F-1, product-owner adjudication, v1.14 backfill of an already-shipped code ruling).**
   `path_falls_under_or_equals` (`crates/factory-dispatcher/src/shard_manager.rs`) matches a
   tool-call target path against a `[[shard]]` entry's `artifact_path` by comparing the two paths'
   NORMALIZED component suffixes (after filtering `Component::CurDir` components out of both
   sides). When an entry's `artifact_path` is EMPTY, or NORMALIZES TO EMPTY after that
   `Component::CurDir` filtering (`""`, `"."`, `"./"`), the component-suffix comparison degenerates
   to `[] == []`, which is trivially true against ANY target path's component suffix — the entry
   VACUOUSLY MATCHES every `Edit`/`Write`/`MultiEdit` dispatch in the repository, silently
   mis-applying that entry's shard-cap gate (and, for a `"frontmatter-changelog-array"`-shaped
   entry, its item-count gate) to artifacts the entry was never intended to govern. This is a
   FAIL-OPEN-BY-OVER-MATCH structural defect in the MATCH ITSELF, distinct in kind from every other
   `validate_entry` check (each of which rejects a malformed entry's OWN semantic fields — `shape`,
   `low_water_mark`, `N`, the cap-formula inputs); it must therefore be caught as a structural
   precondition of the match, BEFORE any cap-formula arithmetic (Postcondition 9's cap-vs-formula
   comparison and its EC-015/EC-017 divisor-door guards) is even attempted for that entry.
   `validate_entry` MUST check, for the entry under validation, that `artifact_path`'s component
   sequence — after filtering `Component::CurDir` — is non-empty. If it IS empty, `validate_entry`
   MUST return `Err(ShardConfigError::EmptyArtifactPath { artifact_stem, artifact_path })` (a new,
   per-field-diagnostic variant mirroring `MissingShape`/`InvalidLowWaterMark`/
   `CapExceedsFormulaCeiling`/`FormulaCeilingSaturated`/`MissingN`'s existing convention), which
   `shard_cap_precheck` surfaces as `HookResult::Error` — NEVER a silent vacuous match, and NEVER a
   silent skip of the entry. Consistent with this Postcondition's own v1.12 MATCH-FIRST re-scoping,
   this check runs at ENTRY-MATCH time (immediately after `find_matching_entry` resolves the entry
   that would otherwise vacuously match every path) and validates only the matched entry — it is
   never run eagerly across the whole config. See EC-022 and its Canonical Test Vector.
   See EC-018 (new, the resolved blast-radius scenario), EC-019 (new, the residual structural-
   parse-failure exception), and EC-022 (new, the empty-`artifact_path` structural defect guard)
   below, and the matching Canonical Test Vectors.

2. **`stat()`-only size read — no file content enters WASM memory or a fuel budget.** For a
   matched path, the check reads ONLY the current shard's byte size via filesystem metadata
   (`stat()`/`metadata()`), never reading file content into memory for the size determination.
   This is native dispatcher-process code with no WASM sandbox and no fuel budget of its own —
   the postcondition that must hold is that the size-check itself is structurally incapable of
   producing a `plugin.timeout`/`plugin.indeterminate` event, because it is not a WASM plugin
   invocation at all (ADR-051 §Rationale — "Why native, not WASM"). This postcondition applies to
   the `"flat"` artifact shape only — Postcondition 8 specifies the DIFFERENT, more-than-`stat()`
   read cost for the `"frontmatter-changelog-array"` shape.

3. **CORRECTED (fix-burst pass-2, F-P2-002, HIGH) — projected size is computed PER-TOOL-SEMANTICS,
   never by a single `current_shard_bytes + payload_bytes` formula applied uniformly to every
   tool.** The WITHDRAWN formula (`current_shard_bytes + payload_bytes` for every tool, including
   `Write`) is UNSOUND for `Write`: a `Write` call REPLACES a file's entire content — `content`'s
   own length already IS the file's post-apply size — so adding `current_shard_bytes` on top
   double-counts the shard's own pre-existing bytes on top of a payload that already represents the
   complete post-apply file, over-triggering rotation on ordinary same-size-or-shrinking full-file
   `Write` calls. The corrected, tool-discriminated formula:
   - **`Write`:** `projected_size = len(content)` alone. `current_shard_bytes` is NOT added — the
     `content` parameter is the file's complete post-apply state, not a delta.
   - **`Edit`/`MultiEdit`:** `projected_size = current_shard_bytes + net_delta_bytes`, where
     `net_delta_bytes` is the net length delta — `len(new_string) - len(old_string)` — for `Edit`,
     or the SUM of each individual edit's net delta for `MultiEdit`. This leg is UNCHANGED from the
     original formula — `Edit`/`MultiEdit` mutate existing content in place, so the delta-against-
     current-size model was always correct for these two tools; only the `Write` leg required
     correction.
   Never a re-read of the file after a hypothetical apply, for either tool class. If
   `projected_size <= shard_cap_bytes`, the check returns `Continue` (no roll). If
   `projected_size > shard_cap_bytes`, the check triggers the roll-before-write sequence
   specified in BC-1.18.006 (this BC does not itself perform the roll or the block — it is the
   formula-and-trigger boundary; BC-1.18.006 owns the observable roll/block outcome).

   **Known formula gap — `replace_all: true` occurrence multiplicity, DEFERRED to BC-1.18.006
   (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5 finding F-C1-P5, product-owner
   adjudication).** The `Edit`/`MultiEdit` formula above computes `net_delta_bytes` as
   `len(new_string) - len(old_string)` for a SINGLE occurrence — this is exactly what an ordinary
   `Edit`/`MultiEdit` call (without `replace_all`) produces on disk, and matches
   `net_delta_bytes_for_edit`'s implementation precisely. The Claude Code `Edit` tool also
   supports `replace_all: true`, under which the ACTUAL on-disk delta is
   `occurrence_count(old_string, current_file_content) x (len(new_string) - len(old_string))` — a
   multiplicity this formula does not compute. Left uncorrected, an `Edit{replace_all: true}` with
   many occurrences can under-count `projected_size`, in principle growing a shard past
   `shard_cap_bytes` without the byte-size trigger firing. **Adjudicated DEFER, not fix-now, for
   THIS cluster:** computing the true occurrence count requires reading the current file's full
   content to count non-overlapping matches of `old_string` — a departure from this
   Postcondition's stat()-only-for-`Write` / stat()-only-for-current-size-for-`Edit`/`MultiEdit`
   design and from Postcondition 2's "never reading file content into memory for the size
   determination" guarantee for the `"flat"` shape, with real cost/consistency implications (a
   full-content read on every `replace_all` `Edit`, versus today's O(1) `stat()`) that warrant
   cross-component assessment rather than a same-burst formula patch. Critically, THIS cluster's
   trigger (this Postcondition / `shard_cap_gate_check`) is NON-BLOCKING today: a fired trigger
   only emits a `tracing::warn!` and returns `Continue` (`shard_cap_gate_check`'s
   `ShardShape::Flat` arm) — the roll/block outcome does not exist until BC-1.18.006 lands (this
   Postcondition's own "Ownership" text above). An under-count in THIS cluster therefore has no
   live user-facing consequence beyond a possibly-missing telemetry warning; the consequence
   becomes load-bearing only once BC-1.18.006's block exists and trigger accuracy actually gates
   an observable outcome. **Explicit deferral anchor:** BC-1.18.006 MUST add a precondition or
   postcondition acknowledging this gap and MUST NOT treat its roll/block outcome as
   complete/production-ready against a `replace_all: true` `Edit` until either (a) this
   Postcondition is amended to compute the occurrence-multiplied delta (with Postcondition 2's
   stat-only guarantee explicitly re-scoped, mirroring this BC's own precedent for narrowing an
   invariant's literal wording without weakening its underlying safety property — see the v1.9
   Postcondition-1/Invariant-3 target-artifact scoping clarification), or (b) BC-1.18.006's own
   roll-execution path — which must already read the shard's full content to perform a rotation —
   independently re-validates the post-apply size against `shard_cap_bytes` using the ACTUAL
   applied content length, catching an under-counted trigger before or immediately after write.
   This is a routing obligation to `product-owner`/`architect` at BC-1.18.006's own spec-evolution
   burst (cluster 2), not a deferral to an unattached "later" — BC-1.18.006 is a named,
   already-scheduled BC in this BC's own Related BCs section.

4. **The cap formula is byte-size-denominated, never line-count-denominated.**
   `shard_cap_bytes <= (PRACTICAL_FUEL_CEILING / WORST_CASE_FUEL_PER_BYTE) -
   MAX_SINGLE_RECORD_BYTES - SAFETY_MARGIN`. A line-count proxy (e.g. the D-442(e) STATE.md
   soft/hard line-budget convention) MUST NOT be substituted for this formula on any Layer-2
   sharded artifact: `lessons.md` is a directly-measured counter-example (1,646/3,500 lines —
   comfortably under its own D-442(e) line budget — while carrying 234,731 bytes, per the F1
   delta analysis), and BC-INDEX.md carries a single 16,521-byte physical line (per ADR-051
   direct measurement) that a naive per-line cap would not catch. Every `shard_cap_bytes`,
   `current_shard_bytes`, `payload_bytes`, and `projected_size` value in this BC's postconditions
   is a byte count. This postcondition governs the `"flat"` shape's byte-size trigger; it does NOT
   govern mechanism B1's item-count trigger (Postcondition 8), which is deliberately NOT
   byte-size-denominated for its rotation decision.

5. **Per-artifact cap is the MINIMUM across every Cohort B validator that reads that artifact
   (Cross-Validator Minimum Rule).** `burst-log.md`'s effective `shard_cap_bytes` is
   `MIN(cap_for(validate-burst-log), cap_for(regression-gate), cap_for(convergence-tracker))`
   because ADR-047 §8a's Cohort B table names all three as readers of `burst-log.md`.
   `decision-log.md`, `lessons.md`, and `session-checkpoints.md`'s effective caps are
   `MIN(cap_for(regression-gate), cap_for(convergence-tracker))` only — `validate-burst-log` does
   not read these three artifacts, so its (possibly tighter) per-plugin cap is not applied to
   them. A single global cap across all four mechanism-A artifacts MUST NOT be used, since it
   would be needlessly conservative for artifacts only two of the three Cohort B validators read
   (ADR-051 Decision 2 step 5).

   **Timing clarified (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding
   F-C1-P4-003, LOW pending-intent, product-owner adjudication, v1.10) — this Rule is a
   CONFIG-AUTHORING / F4-CALIBRATION-HARNESS-TIME computation, not a live per-write runtime
   obligation.** ADR-051 §Decision 2 places the Cross-Validator Minimum Rule inside the F4
   calibration harness's own design (harness step 5: "Per-artifact cap = MIN over every Cohort B
   plugin that reads that artifact") — the harness computes each per-plugin cap, takes the MIN
   across an artifact's Cohort B readers, and the RESULT is what gets DECLARED as that entry's
   single `shard_cap_bytes` value in the `[[shard]]` config. `ShardEntry` carries exactly ONE
   `shard_cap_bytes` field (Postcondition 4) — there is no per-validator breakdown for a live gate
   to combine at write time. The live per-write gate (Postcondition 3) therefore consumes
   `entry.shard_cap_bytes` DIRECTLY, as an already-MIN'd value; it does NOT re-derive the MIN
   across Cohort B readers on each write. `effective_shard_cap_bytes(per_validator_caps: &[u64])
   -> u64` (`crates/factory-dispatcher/src/shard_manager.rs`) is this Rule's
   CONFIG-AUTHORING/HARNESS-TIME helper — the F4 harness or an operator calls it once, offline,
   when computing what value to declare for a given artifact's `shard_cap_bytes`; it is not a
   function the live gate invokes per write, and the absence of a `shard_cap_gate_check` call site
   for it is CORRECT, not a wiring gap. This clarification is wording-only: the numeric examples
   already in this Postcondition's own prose and in the Canonical Test Vectors table below
   (`MIN(40000, 49152, 52000) = 40,000`, etc.) already describe exactly this authoring-time
   computation; only the earlier ambiguity about WHEN the MIN is computed is resolved — no
   Postcondition-3 wording, no Canonical Test Vector, and no code change follow from this
   clarification. **Deferred, separate obligation (explicitly out of scope for this BC and for
   cluster-1):** a RUNTIME re-derivation of the Cross-Validator Minimum — e.g. because a Cohort-B
   reader-set for an artifact could change without a config republish — is a fundamentally
   different capability than this Postcondition describes: it would require wiring a
   Cohort-B-reader-set-to-artifact mapping into the live gate, which `ShardEntry`'s current
   single-`shard_cap_bytes` schema does not carry and which this BC does not specify. If that need
   ever materializes, it MUST be raised as a NEW postcondition (with `architect`/ADR-051
   involvement, since it changes the config schema and the gate's runtime data dependencies), not
   assumed to already be satisfied by `effective_shard_cap_bytes`'s existing authoring-time role.

6. **Constants are PROVISIONAL until the F4 synthetic calibration harness runs; the formula shape
   is locked now, the numbers are not.** The current provisional values (ADR-051 Decision 2,
   grounded in ADR-042's measured `fuel = 2,585,970 + 53.18 × payload_bytes` linear model and
   direct 2026-09-05 byte measurements) are:

   | Constant | Provisional value | Locked at F4? |
   |---|---|---|
   | `PRACTICAL_FUEL_CEILING` (today, `DEFAULT_FUEL_CAP=10M`-effective) | 8,000,000 | YES — replace the 80% haircut with the harness's measured "reliably completes" percentile |
   | `PRACTICAL_FUEL_CEILING` (post rc.24, `DEFAULT_FUEL_CAP=20M`, informational only) | 16,000,000 | YES — recompute once rc.24 is confirmed live at the operator level; re-run the harness, do not simply double |
   | `WORST_CASE_FUEL_PER_BYTE` | 106.36 (2× ADR-042's measured 53.18 fuel/byte coefficient, a conservative floor from a comparable legacy-bash-adapter-routed validator) | YES — per-plugin measured value from the harness |
   | `MAX_SINGLE_RECORD_BYTES` | 16,384 (64% margin over the 9,987-byte largest single physical line measured in the active cycle's four append-log files, 2026-09-05) | YES — full-repository sweep at F4 |
   | `SAFETY_MARGIN` | 8,192 (buffer for shard-index-entry + shard-header overhead; not yet measured against the real index schema's per-entry byte cost) | YES — recompute once BC-1.18.006's shard-index schema's real per-entry size is known |

   Illustrative worked example (NOT the locked value): today's ceiling yields
   `(8,000,000 / 106.36) - 16,384 - 8,192 = 50,640`, rounded down to a provisional **49,152 bytes
   (48 KiB)** cap. This BC's implementation MUST treat these five constants as configuration
   inputs to the formula (not hardcoded into the check's logic), so the F4 harness can replace
   them without a code change to the check itself.

7. **Calibration harness ownership and re-run trigger.** The F4 synthetic calibration harness
   (owned by `performance-engineer`, co-run with `implementer`) that produces the final, locked
   constants is executed once before this BC's postconditions are treated as final, and re-run
   whenever `DEFAULT_FUEL_CAP` changes at the operator level (ADR-051 Decision 2). Harness design:
   adversarially-shaped synthetic fixtures at a geometric size series, dispatched directly against
   each of the three Cohort B plugins in isolation, fuel captured via `VSDD_SINK_FILE`, fit for
   linearity/superlinearity exactly as ADR-042 did; if superlinearity is found,
   `WORST_CASE_FUEL_PER_BYTE` MUST be the LOCAL marginal rate at the largest tested size, never the
   global average slope (ADR-051 Decision 2 step 4).

8. **Item-count-denominated trigger for the `"frontmatter-changelog-array"` artifact shape
   (mechanism B1; added F-S2502-F2-005 fix-burst, ADR-051 v1.1 Decision 1 amendment).** This BC
   owns BOTH trigger shapes the SAME native gate dispatches on, distinguished by the `[[shard]]`
   config entry's `shape` field:
   - **Shape declaration.** `shape = "flat"` selects Postconditions 1-7's `stat()`-only byte-size
     trigger. `shape = "frontmatter-changelog-array"` selects THIS postcondition's item-count
     trigger — the gate dispatches to the shape-appropriate check based on this field alone; both
     shapes share the SAME config-match/no-match entry point (Postcondition 1's zero-cost bypass
     applies identically to both).
   - **Read cost — CORRECTED cold/steady-state split (fix-burst pass-2, F-P2-007, MEDIUM).** The
     read-cost claim below is TRUE ONLY IN STEADY STATE (after at least one rotation has occurred)
     and is FALSE at cold start; the two states MUST be characterized separately, never collapsed
     into a single unqualified "bounded" claim:
     - **Cold state (pre-BC-1.18.012 migration).** Direct measurement (2026-09-05) shows
       `BC-INDEX.md`'s `changelog:` sequence, which has never been rotated, holds approximately
       1,997 items across 177,305 bytes of frontmatter. The FIRST trigger evaluation against this
       state must parse and count all ~1,997 items — a finite, single-file read, but NOT bounded
       relative to N in any meaningful sense (it is ~40x the illustrative `N≈50` retention target).
       This is a mischaracterization to correct, not a fuel-budget hazard (the check is native code
       with no fuel budget), but it is genuinely NOT "bounded" in the sense Postcondition 8's
       steady-state claim below uses that word. BC-1.18.012 (the governed one-time B1 changelog
       backfill migration) exists specifically to eliminate this cold state before the ongoing
       per-write gate (BC-1.18.009) is treated as steady-state-bounded.
     - **Steady state (post-BC-1.18.012, and after every subsequent rotation).** The trigger check
       parses the target file's frontmatter far enough to count the existing `changelog:`
       sequence's items — a GENUINELY BOUNDED read: the live sequence is capped at `<= N` items by
       this same mechanism after BC-1.18.012's one-time migration and every prior rotation, per
       BC-1.18.009's corrected contract, so this is never an unbounded-growth read once steady state
       is established. This is MORE than a `stat()` call (it requires reading and lightly
       parsing frontmatter content) but remains native, fuel-budget-free dispatcher code, not a WASM
     plugin invocation — the "why native, not WASM" rationale (Postcondition 2) applies identically
     to this shape.
   - **Missing-file / first-write case (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3
     finding F-C1-P3-001, MEDIUM, product-owner adjudication, v1.9) — GRACEFUL, mirroring EC-004's
     flat-shape precedent, NOT fail-loud.** When the `"frontmatter-changelog-array"`-shaped target
     artifact does not yet exist on disk — a legitimate first-ever `Write` that CREATES the
     artifact (e.g. the first-ever `Write` of a new sharded index file, with valid frontmatter
     present in the `content` payload) — `read_changelog_item_count` MUST map
     `io::ErrorKind::NotFound` to `Ok(0)`, treating the not-yet-existing artifact as holding 0
     existing changelog items, rather than propagating a bare `io::Error` that would surface as
     `HookResult::Error` and hard-block the create (exit 2). This is the SAME missing-file-is-zero
     posture EC-004 already establishes for the `"flat"` shape's `current_shard_bytes_flat` on the
     identical "artifact absent on disk" precondition — there is no principled reason for the two
     trigger shapes to diverge on that precondition, and per CLAUDE.md's production-grade default a
     hard block on a legitimate create is a user-facing wrong behavior, not an acceptable asymmetry.
     Combined with the trigger condition below, `current_item_count + 1 = 0 + 1 = 1 > N` is false
     for any config `N >= 1`, so the create always returns `Continue`. Any OTHER `io::Error` kind
     encountered reading an EXISTING file (a permissions failure, etc.) is UNCHANGED and still
     propagates as `HookResult::Error` (fail-loud) — ONLY `NotFound` is special-cased. **Scope
     note (added v1.13, cross-reference only — no behavior change to THIS bullet):** a
     present-but-fenceless file (no `---`/`---` delimiters found at all) is ALSO treated
     permissively as `Ok(0)`, per the "present-but-fenceless" ruling below (PR #818 fix-burst
     finding B1) — that case is a STRUCTURAL absence of a frontmatter block to parse, not an
     `io::Error` this bullet's "any OTHER io::Error kind... is UNCHANGED" sentence was ever meant to
     cover. A file that HAS a well-formed fence pair but whose YAML content inside fails to parse
     remains fail-loud under the malformed-YAML ruling further below (PR #818 fix-burst finding
     M-1, EC-020) — that IS the "genuinely malformed/unreadable frontmatter" `io::Error` case this
     bullet's fail-loud sentence describes. See EC-014.

   - **Present-but-fenceless case — PERMISSIVE, mirroring this bullet's own missing-file precedent
     (NEW, PR #818 fix-burst finding B1, product-owner adjudication, v1.13 backfill of an
     already-shipped code ruling).** When the `"frontmatter-changelog-array"`-shaped target
     artifact EXISTS on disk but carries NO well-formed `---` frontmatter fence at all — no
     line-anchored opening fence (`raw` does not start with `---\n` or `---\r\n`), OR an opening
     fence is found but no line-anchored closing fence (a line consisting of exactly `---`,
     optionally with a trailing `\r`) is ever found before end-of-file — `read_changelog_item_count`
     MUST return `Ok(0)` rather than propagating an `io::Error`. This mirrors the missing-file
     bullet's rationale exactly: a file with no delimited frontmatter block has no `changelog:`
     sequence to undercount — `Ok(0)` is the CORRECT count, not an approximation of one, because
     zero delimited items exist when no delimited block exists at all. Combined with the trigger
     condition below, `current_item_count + 1 = 0 + 1 = 1 > N` is `false` for any config `N >= 1`,
     so a present-but-fenceless target's edit is never hard-blocked. **This bullet backfills a
     ruling the sanctioned implementation already shipped** (`read_changelog_item_count`'s two
     `Ok(0)` early-returns, one for a missing opening fence and one for a missing closing fence) in
     response to a fresh-context PR reviewer's cycle-1 finding that the pre-fix behavior propagated
     a bare parse-adjacent `io::Error` for a legitimate present-but-fenceless file (e.g. a
     newly-created artifact whose `content` payload had not yet been written with frontmatter, or a
     match against an unrelated file that happens to lack frontmatter entirely) and hard-blocked it
     exactly as EC-014 already forbids for the missing-file case. **Distinguish sharply from the
     malformed-YAML case immediately below (M-1/EC-021):** this bullet applies ONLY when no fence
     pair is found to parse in the first place; it does NOT apply when a fence pair IS found and the
     content between the fences fails to parse — that case is fail-loud, never `Ok(0)`, per the next
     bullet. See EC-020 and its Canonical Test Vector, and EC-021's Canonical Test Vector table for a
     side-by-side contrast of the two cases.

   - **Malformed-YAML case — FAIL-LOUD, with MANDATORY self-deadlock-escape guidance in the error
     message (NEW, PR #818 fix-burst finding M-1, MAJOR, product-owner adjudication, v1.13).** When
     the `"frontmatter-changelog-array"`-shaped target artifact EXISTS on disk AND carries a
     well-formed `---`/`---` fence pair (i.e., neither the missing-file bullet above nor the
     present-but-fenceless bullet above applies), but the YAML content BETWEEN the fences fails to
     parse (`serde_norway::from_str` returns `Err`), `read_changelog_item_count` MUST continue to
     map that failure to `io::ErrorKind::InvalidData` and propagate it as `Err`, which the gate
     (`shard_cap_gate_check`'s item-count arm) MUST continue to surface as `HookResult::Error`
     (fail-loud) — it MUST NEVER be mapped to `Ok(0)` the way `NotFound` (EC-014) or a genuinely
     fenceless file (the bullet immediately above) are. **The question posed by a fresh-context PR
     reviewer (M-1):** because `Edit`, `Write`, AND `MultiEdit` against the target artifact ALL
     route through this same read of the CURRENT on-disk content before the gate decides whether to
     allow the edit, a target file that has already acquired malformed YAML inside an otherwise
     well-formed fence blocks every subsequent gated edit — INCLUDING the repair edit that would fix
     the YAML — with no gated path out; the only escape under the pre-v1.13 spec text was an
     ungated shell-level rewrite of the file, bypassing the gate entirely. Two readings were
     weighed: **(A) FAIL-LOUD** — malformed YAML in a registered, fence-bearing artifact is itself a
     real defect distinct from "no frontmatter to count," and silently coercing it to `Ok(0)` would
     UNDERCOUNT the live `changelog:` sequence's true size (the corruption event does not shrink the
     sequence, only the parser's ability to see it), defeating this BC's own
     fuel-exhaustion-prevention purpose by suppressing the rotation trigger exactly when the
     artifact's true state is least trustworthy; vs. **(B) PERMISSIVE** — extend the
     present-but-fenceless `Ok(0)` posture to this case too, on the theory that any read-path
     failure this gate cannot safely characterize should default to "let the write through" rather
     than deadlock repair. **ADJUDICATED (A) FAIL-LOUD, over (B) PERMISSIVE:**
     - The present-but-fenceless ruling and this case are NOT the same shape of problem. A
       fenceless file has NO frontmatter block at all to fail to parse — there is no YAML content
       whose loss `Ok(0)` could be hiding, so `Ok(0)` there is the CORRECT count, not an
       approximation. THIS case's file HAS a fence pair with actual YAML content inside that failed
       to parse — the `changelog:` sequence the artifact is registered to track may hold any number
       of items, and `Ok(0)` would silently report zero when the truth is "unknown, possibly large."
       Collapsing the two cases into one "any read-path failure is permissive" rule would erase this
       distinction and reintroduce exactly the unbounded-growth-goes-undetected failure mode this
       BC's cap-and-trigger mechanism exists to prevent (Postcondition 9's rationale makes the
       identical point about a silently-inflated ceiling; this is its item-count-shape analogue for
       a silently-deflated count).
     - CLAUDE.md's production-grade default treats "surface-and-defer-via-error" as the correct
       pattern and "silently return a default that hides partial-failure state" as the violation to
       avoid (Standing Rule 3 §2's `Vec::new()` example is the direct analogue: a corrupted count
       silently reported as `0` is the identical anti-pattern applied to a `u64` instead of a
       `Vec`). Malformed YAML in a registered sharded artifact is a real defect an operator should
       be told about, not one this gate should paper over.
     - The self-deadlock is real, but it is a MESSAGING gap, not a reason to weaken the fail-loud
       posture: the fix is to make the escape path DISCOVERABLE from the error itself, not to stop
       erroring. **This is therefore a MANDATORY, load-bearing amendment to the error message
       constructed by `read_changelog_item_count`'s YAML-parse-failure branch (the `map_err` that
       builds the `io::ErrorKind::InvalidData` error from `serde_norway::from_str`'s `Err`):** from
       v1.13 forward, that message MUST, in addition to the underlying parser error text it already
       includes, state PLAINLY (a) that this specific frontmatter block failed YAML parsing (not
       some other cause), and (b) that the gate producing this error only intercepts
       `Edit`/`Write`/`MultiEdit` tool calls against this artifact — the operator can repair the
       malformed frontmatter block directly (fixing the YAML syntax so it parses) using any means
       OUTSIDE those three gated tool calls (e.g., a `Bash`-invoked editor, script, or heredoc
       rewrite of the file), after which this SAME gate will parse successfully and every
       subsequent `Edit`/`Write`/`MultiEdit` — including ones touching only content outside the
       frontmatter block — proceeds normally again. This is NOT a sanctioned exception to
       TD-FACTORY-HOOK-BYPASS-001 (that policy governs bypassing the WASM hook-plugin chain via
       `--no-verify`-class flags; this native dispatcher check has no such flag and none is
       introduced here) — it is pointing the operator at the one degree of freedom that already,
       structurally, exists outside this native check's own gated tool surface. The message
       requirement is normative, not advisory: an unchanged, generic `"{path}: frontmatter YAML
       parse failed: {e}"`-only message is a NON-COMPLIANT implementation of this Postcondition from
       v1.13 forward. See EC-021 and the matching Canonical Test Vector.

   - **Load-time presence requirement for `N` — scope CORRECTED to entry-match time (S-25.02 Phase
     F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, LOW pending-intent, product-owner
     adjudication, v1.12) — see Postcondition 1's "Blast-radius scoping" sub-paragraph below.** The
     "at LOAD time, not merely at gate/dispatch time" framing in the bullet below (unchanged text,
     retained for its historical F-C1-P4-002 rationale) is, from v1.12 forward, read as "at
     entry-match time for the entry matched against the current dispatch's target path" — NOT
     eagerly for every `[[shard]]` entry in the config file regardless of match. See Postcondition
     1 below for the full ruling and its rationale; this cross-reference exists so a reader of this
     bullet in isolation is not misled into the pre-v1.12 eager-validate-all-entries reading.
   - **Trigger condition.** `current_item_count + 1 > N` (a config value, per BC-1.18.009
     Postcondition 1) — NEVER a byte-size comparison for this shape. `shard_cap_bytes`
     (Postcondition 4's byte-size formula) still bounds `BC-INDEX.md`'s TOTAL byte footprint as a
     whole-artifact concern, but the ROTATION decision within this shape is item-count-based only.
   - **Ownership.** Both trigger shapes are specified as THIS BC's postconditions — there is no
     separate, competing trigger-owning BC. BC-1.18.009 owns the observable rotate/block-and-retry
     OUTCOME once this shape's trigger fires (the same division of responsibility Postcondition 3
     already establishes between this BC's trigger and BC-1.18.006's outcome for the `"flat"`
     shape).
   - **Rotation-target config, NEW (fix-burst pass-3, F-P3-005, ADR-051 v1.3 Decision 14).** A
     sibling config value, `low_water_mark`, is declared alongside `N` in the SAME `[[shard]]`
     config entry (`"frontmatter-changelog-array"` shape only) — it is the rotation TARGET
     BC-1.18.009's gate trims the live `changelog:` sequence down to, NEVER `N` itself and NEVER a
     hardcoded `N-1`. **Default:** `floor(N/2)` when the field is omitted. **Fail-loud validation
     constraint:** `0 <= low_water_mark < N`, enforced at config-load time — a malformed value
     (`low_water_mark >= N` — the `== N` boundary included — or a negative value) is NEVER silently
     clamped or silently defaulted around; the config is treated as malformed and the check returns
     `HookResult::Error` (mirroring EC-009's "no silent default for malformed shape" posture,
     extended to this field). `N` remains the UNCHANGED trigger threshold
     (`current_item_count + 1 > N`, above) — this BC owns declaring/validating BOTH `N` and
     `low_water_mark`; BC-1.18.009 owns what its rotation step DOES with `low_water_mark` once the
     trigger fires (the same trigger-vs-outcome division this bullet's own "Ownership" text already
     establishes for `N`). Rationale: trimming to exactly `N-1` on every rotation left the live
     sequence back at the trigger boundary after the very next prepend, causing a block+retry
     round-trip on essentially every subsequent write; `low_water_mark` amortizes rotation to once
     per `N - low_water_mark` writes (ADR-051 §Decision 14).
   - **Load-time presence requirement for `N` (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1
     pass-4 finding F-C1-P4-002, MEDIUM, product-owner adjudication) — a
     `"frontmatter-changelog-array"`-shaped entry that omits `N` MUST fail-loud at
     `ShardRegistry::load()` time, not merely at gate/dispatch time.** `N` is REQUIRED for this
     shape — without it, neither the trigger condition (`current_item_count + 1 > N`) nor the
     `low_water_mark` fail-loud/advisory validation (`0 <= low_water_mark < N`) is even
     well-defined. The sanctioned implementation currently checks `entry.n`'s presence only inside
     the per-write dispatch path (`HookResult::Error` on `None`) — a genuine, load-bearing
     fail-loud check, but at the WRONG time relative to this BC's own established uniform posture:
     `shape` (EC-009), `low_water_mark`'s numeric range (EC-011), and `shard_cap_bytes` vs. its
     formula ceiling (Postcondition 9/EC-013) all fail loud at LOAD time, so an operator learns of
     a malformed `[[shard]]` entry before any write is ever attempted against the artifact, not on
     the first write that happens to target it. Deferring the missing-`N` check to gate time is an
     asymmetry with no principled justification, and — per CLAUDE.md's production-grade default —
     a load-time-checkable malformation that is only caught at first-use is a defer-pattern smell.
     `ShardRegistry::load()` MUST therefore validate, for every `"frontmatter-changelog-array"`-
     shaped entry, that `entry.n` is `Some` — fail-loud `HookResult::Error` (a new
     `ShardConfigError` variant, e.g. `MissingN { artifact_stem }`, mirroring `MissingShape`'s
     convention) on `None`, evaluated BEFORE the entry's `low_water_mark` is examined.
     **Restructuring consequence — EC-011 is never silently bypassed by a missing `N`:** the
     existing `ShardRegistry::load()` logic validates `low_water_mark` only when BOTH `entry.n`
     and `entry.low_water_mark` are `Some` (a destructure-and-short-circuit pattern) — this meant
     an entry that omitted `N` AND declared an out-of-range `low_water_mark` passed load-time
     validation entirely (neither condition was checked), with the error deferred to the first
     gate-time write. Requiring `N`'s presence as an unconditional, load-time, shape-scoped check —
     evaluated first, independent of whether `low_water_mark` is present — closes this gap
     structurally: every `"frontmatter-changelog-array"` entry now either (a) fails loud on a
     missing `N` at load time, or (b) has `N` present, in which case the existing `low_water_mark`
     numeric-range validation (when `low_water_mark` is also present) runs exactly as before. No
     entry can silently load with BOTH a missing `N` and an out-of-range `low_water_mark`
     undetected. See EC-016.
   - **CORRECTED (fix-burst pass-4, F-P4-001, HIGH, ADR-051 v1.4 Decision 14 adjudication Option
     (b)) — a legal-but-poorly-amortizing `low_water_mark` value NEVER fail-louds; it loads
     normally and instead emits a non-fatal amortization advisory.** Any value satisfying
     `0 <= low_water_mark < N` — including every value in `(floor(N/2), N)` up to and including the
     boundary value `N-1` — is LEGAL and loads normally (`Continue`; config load NEVER returns
     `HookResult::Error` for such a value). `N-1` is not a special case: `N-1 < N` holds by
     construction for every `N >= 1`, so it satisfies the fail-loud constraint exactly as any other
     interior value does. To close the latent amortization-churn concern WITHOUT a false fail-loud
     claim, config load additionally emits a NEW, NON-FATAL, WARN-level diagnostic (via
     `tracing::warn!`, never `println!`, never `HookResult::Error`) whenever
     `low_water_mark > floor(N/2)` — i.e., whenever the configured value amortizes rotation worse
     than the recommended default. The advisory cites the configured `(N, low_water_mark)` pair and
     the resulting amortization factor `N - low_water_mark` (writes-per-rotation), compared against
     the default's `N - floor(N/2)` amortization, so an operator can see quantitatively how much
     worse their configured value performs. The advisory fires or does not fire; either way, config
     load ALWAYS succeeds for any value satisfying the numeric constraint — this is the load-time
     equivalent of a lint warning, not a second validation gate.

9. **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 finding, PC4 load-enforcement-intent
   question, product-owner adjudication) — load-time fail-loud enforcement of the cap-vs-formula
   inequality.** `ShardRegistry::load()` MUST, for EVERY `[[shard]]` config entry — regardless of
   `shape` (Postcondition 8 already establishes that `shard_cap_bytes` bounds an artifact's TOTAL
   byte footprint as a whole-artifact concern even for the `"frontmatter-changelog-array"` shape,
   so this check is NOT `"flat"`-shape-only) — independently recompute
   `compute_shard_cap_bytes(entry.cap_formula_inputs())` from that SAME entry's own four parsed
   formula inputs (`practical_fuel_ceiling`, `worst_case_fuel_per_byte`, `max_single_record_bytes`,
   `safety_margin` — Precondition 2, Postcondition 4/6) and compare it against that entry's own
   declared `shard_cap_bytes`. If `shard_cap_bytes > compute_shard_cap_bytes(entry.cap_formula_inputs())`
   — the declared cap exceeds what the entry's OWN stated formula inputs justify — `load()` MUST
   return `HookResult::Error`; it MUST NEVER silently accept the oversized value and MUST NEVER
   silently clamp `shard_cap_bytes` down to the computed ceiling. This mirrors the fail-loud-at-
   load-time posture EC-009 (`shape` omission) and EC-011 (`low_water_mark` out of range) already
   establish for this SAME config surface, extended to the cap-vs-formula relationship. **Adjudication
   rationale (Reading (B) adopted over Reading (A)):** without this check, a `[[shard]]` entry whose
   declared `shard_cap_bytes` is set far above its own declared formula ceiling loads silently, and
   the live per-write gate (Postcondition 3) then honors the too-large cap — reintroducing the exact
   fuel-exhaustion failure mode BC-1.18.001's sibling Layer-1 fail-closed INDETERMINATE contract
   exists to prevent (see Related BCs). The four formula inputs are therefore load-bearing
   configuration at load time, not merely documentary/harness-facing metadata carried for the F4
   calibration harness's offline use — Postcondition 6's "the formula shape is locked now, the
   numbers are not" already states that the INEQUALITY itself (not just its eventual F4-locked
   numeric inputs) is a live constraint of the system today, under the current provisional
   constants. This is a load-time misconfiguration guard: it protects against a human/config-authoring
   error in a `[[shard]]` entry's own internally-inconsistent inputs; it does NOT require or perform
   any re-derivation of the F4 harness's calibration itself (Postcondition 6/7 continue to own
   HOW the constants are measured; this postcondition only owns verifying INTERNAL CONSISTENCY
   between a given entry's own declared `shard_cap_bytes` and its own declared four inputs).

   **Formula-input validity precondition to the cap-vs-formula comparison — "divisor-door" closure
   (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-001, MEDIUM/HIGH,
   product-owner adjudication).** `compute_shard_cap_bytes` divides `practical_fuel_ceiling` by
   `worst_case_fuel_per_byte`. `practical_fuel_ceiling` is `u64`-typed (Precondition 2), so TOML
   deserialization already guarantees it is a finite, non-negative integer — no additional check
   is needed for it. `worst_case_fuel_per_byte` is `f64`-typed and therefore NOT type-guaranteed
   finite or positive — TOML's float grammar accepts `nan`, `inf`, `-inf`, `0.0`, and negative
   literals. If `worst_case_fuel_per_byte = 0.0` (with `practical_fuel_ceiling > 0`), the division
   yields `+inf`; `(+inf).floor() as u64` saturates to `u64::MAX` (Rust's saturating
   float-to-integer cast), so `compute_shard_cap_bytes` returns a ceiling of effectively
   `u64::MAX - MAX_SINGLE_RECORD_BYTES - SAFETY_MARGIN` — a value no realistic `shard_cap_bytes`
   declaration will ever exceed. This DEFEATS the cap-vs-formula comparison immediately above: an
   arbitrarily-large `shard_cap_bytes` passes the `shard_cap_bytes > computed_ceiling` check
   silently, the entry loads, the roll never triggers at the resulting oversized cap, and the
   exact fuel-exhaustion failure mode this Postcondition exists to prevent (see the rationale
   paragraph above and BC-1.18.001 in Related BCs) is reintroduced through a divisor-door rather
   than a directly-oversized `shard_cap_bytes`. `ShardRegistry::load()` MUST therefore validate,
   for EVERY `[[shard]]` entry and BEFORE computing `compute_shard_cap_bytes(entry.cap_formula_
   inputs())` for that entry, that `worst_case_fuel_per_byte.is_finite() && worst_case_fuel_per_byte
   > 0.0`. If this validation fails, `load()` MUST return `HookResult::Error` (a new
   `ShardConfigError` variant — e.g. `InvalidWorstCaseFuelPerByte { artifact_stem,
   worst_case_fuel_per_byte }` — mirroring `MissingShape`/`InvalidLowWaterMark`/
   `CapExceedsFormulaCeiling`'s existing per-field-diagnostic convention); it MUST NEVER silently
   substitute a default rate and MUST NEVER silently skip the cap-vs-formula comparison for that
   entry. **Scope — no analogous check needed for the other three inputs.**
   `max_single_record_bytes` and `safety_margin` are likewise `u64`-typed (Precondition 2), so
   TOML deserialization already excludes NaN/±infinite/negative values for them; and a degenerate
   `0` value for any of `practical_fuel_ceiling`, `max_single_record_bytes`, or `safety_margin`
   drives `compute_shard_cap_bytes`'s result toward its floor of `0` (via `saturating_sub`) — the
   SAFE, maximally-conservative direction, never the exploitable-widening-toward-`u64::MAX`
   direction this clause closes. Only `worst_case_fuel_per_byte`'s `f64` type creates that hazard.
   See EC-015.

   **Residual divisor-door closure — saturated computed ceiling, NOT merely a divisor lower-bound
   (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5 finding F-C1-P5, MEDIUM/HIGH,
   product-owner adjudication).** The `worst_case_fuel_per_byte.is_finite() && > 0.0` guard
   immediately above is NECESSARY but INSUFFICIENT to close the divisor-door: a legal
   tiny-positive divisor (e.g. `worst_case_fuel_per_byte = 1e-300`) satisfies that guard yet still
   drives `compute_shard_cap_bytes`'s internal division (`practical_fuel_ceiling as f64 /
   worst_case_fuel_per_byte`) to a value so large that the subsequent `.floor() as u64` cast
   saturates to `u64::MAX` — reproducing the IDENTICAL failure mode the guard above exists to
   close: an oversized `shard_cap_bytes` sails past the `CapExceedsFormulaCeiling` comparison
   because the computed ceiling itself is astronomically, meaninglessly large. Picking an
   arbitrarily larger lower bound on `worst_case_fuel_per_byte` (e.g. requiring `> 1.0` instead of
   `> 0.0`) would only move this failure mode's threshold, never close it — any positive lower
   bound admits a divisor small enough, relative to a large enough `practical_fuel_ceiling`, to
   re-saturate the cast. The robust, production-grade close is to detect the SATURATED/non-finite
   COMPUTED CEILING directly, not to bound the divisor. `ShardRegistry::load()` MUST, for every
   entry that passes the `worst_case_fuel_per_byte.is_finite() && > 0.0` guard and BEFORE the
   entry's `CapExceedsFormulaCeiling` comparison is evaluated, independently evaluate the RAW,
   pre-cast division result `practical_fuel_ceiling as f64 / worst_case_fuel_per_byte` (the same
   quantity `compute_shard_cap_bytes` computes internally, before its own `.floor() as u64` cast)
   and reject the entry — `HookResult::Error` — if that raw f64 value is non-finite
   (`!result.is_finite()`) OR is `>= u64::MAX as f64` (the saturation boundary of the subsequent
   lossy cast). This MUST use a new `ShardConfigError` variant (POLICY 1 append-only — e.g.
   `FormulaCeilingSaturated { artifact_stem, practical_fuel_ceiling, worst_case_fuel_per_byte }`),
   distinct from `InvalidWorstCaseFuelPerByte` (which catches an illegal `<= 0.0`/non-finite
   DIVISOR itself, not a saturated RESULT arising from an otherwise-legal tiny-positive divisor).
   `load()` MUST NEVER silently accept a saturated ceiling and MUST NEVER silently substitute a
   fallback ceiling — a saturated ceiling cannot meaningfully bound any declared `shard_cap_bytes`,
   so the entry is a config error, not a value to work around. Together, the
   `worst_case_fuel_per_byte.is_finite() && > 0.0` guard (rejecting an illegal divisor) and this
   saturated-ceiling guard (rejecting a legal-but-degenerate divisor whose RESULT still saturates)
   close the divisor-door completely. See EC-017.

10. **(v1.16; ADR-052 §Decision 11(g)(2) sanitization rule extended to this BC's error type; closes the S-25.09
    black-box finding that a hostile `artifact_stem` reached the dispatcher `block_reason` with a raw ESC)
    Every `ShardConfigError` Display renders each config-derived string slot escaped and capped.** Every
    config-derived string slot of every `ShardConfigError` variant (`artifact_stem`, `artifact_path`, and any
    other string field read from `.factory/shard-config.toml`; the rule is variant-exhaustive, including
    `MissingShape`, `MissingN`, `EmptyArtifactPath`, `InvalidLowWaterMark`, `InvalidWorstCaseFuelPerByte`,
    `CapExceedsFormulaCeiling` and `FormulaCeilingSaturated`) is rendered through the same `sanitize_diagnostic`
    rule as ADR-052 §Decision 11(g)(2): control characters (ESC, LF, CR, NUL, other C0/C1) are escaped so the
    rendering is ONE line, and each data-derived substring is capped at 256 characters; the fixed message text and
    numeric values are emitted verbatim and never truncated. The `ShardConfigError::Toml` variant renders the
    error category plus line and column ONLY: it NEVER carries the toml parser's message or the offending source
    line (which echoes on-disk content and can include a secret); a wrong-typed value in `shard-config.toml` is
    never echoed. The rule holds on EVERY surface that renders the error: the dispatcher `block_reason` of the
    PreToolUse shard-cap gate (`HookResult::Error`, exit 2), the `migrate-bc-index` stderr via
    `ShardCapConfigUnavailable`, and any log or event field. **Per-slot cap by surface, one rendering source, and
    the `Toml` representation (ADR-052 §Decision 11(g)(2)(f)):** (a) every InternalLog or tracing event field
    renders each data slot independently capped at 64 characters (operator stderr and `block_reason` stay at 256);
    `Display` and the event rendering are ONE `render_with_cap(cap)` source (`Display` = 256) implemented as an
    exhaustive `match` over every `ShardConfigError` variant with NO wildcard arm, so a future variant cannot
    compile without a rendering. (b) The `Toml` variant STORES only a closed category token plus the 1-based line and
    column, never the `toml::de::Error` object: the `#[from] toml::de::Error` conversion is replaced by a
    constructor that stores only (category, line, column), so the parser message cannot be rendered by accident.
    (c) `BcIndexMigrationError::ShardCapConfigUnavailable.detail` is built from the already-sanitized
    `ShardConfigError` rendering and never from a raw error. **Scope of the tracing-field half (v1.17;
    ADR-052 §Decision 11(g)(2)(f) "Scope of the tracing-field half", (2)):** the tracing-field rule in (a) is
    scoped to the migration and shard surface, NOT dispatcher-wide: it governs `crates/factory-dispatcher/src/
    shard_manager.rs`, every file under `crates/factory-dispatcher/src/shard_manager/` (including `admission.rs`)
    and, by provenance, any other file under `crates/factory-dispatcher/src` that passes a value of the
    shard/migration error family (`ShardConfigError`, `ShardRollError`, `ShardRetentionError`,
    `MechanismABackfillError`, `BcIndexAddressingError`, `BcIndexMigrationError`, `AppendLogMigrationError`) or text
    read from `shard-config.toml`, a shard index, a BC-INDEX manifest or a `migration-state` record to a tracing
    macro, while `executor.rs`, `indeterminate_marker.rs`, `invoke.rs`, `sinks/`, `vsdd_sink.rs` and every other
    file not so matched are outside it (their diagnostics are governed by BC-3.08.001's no-subscriber invariant).
    Vectors: EC-023.

11. **(v1.17; ADR-052 §Decision 11(c) point 7(e) row (11)(ii)) A config `artifact_stem` that is not a safe path
    component is refused at shard-config decode, never reaching a path operation.** Every `[[shard]]` entry's
    `artifact_stem` is parsed at `shard-config.toml` DECODE time by the `ArtifactStem` constructor, which accepts
    only a value for which `is_safe_path_component` holds (point 7(a)); a violating value fails the configuration
    load with `ShardConfigError::ArtifactStemUnsafe`, so no `ShardEntry` carrying an unsafe stem can exist and the
    stem never reaches a path join, `rename`, `remove` or `read_dir` argument on any surface (the PreToolUse
    shard-cap gate, the roll and retention paths, `migrate-bc-index`). The variant is rendered under Postcondition
    10: the stem is a config-derived string slot, escaped to ONE line and capped (256 for `Display`, 64 for an event
    field), fixed text and numerics verbatim, as one more arm of the exhaustive `render_with_cap` match (no wildcard
    arm). The refusal is fail-loud (`HookResult::Error`, exit 2 on the gate; `ShardCapConfigUnavailable.detail`
    built from the sanitized rendering on `migrate-bc-index`) and never substitutes a sanitized or default stem.
    Vectors: EC-024.

## Invariants

1. **The size check is native code, never a WASM plugin.** No `[[hooks]]` registry entry, no
   `.wasm` binary, and no `HOST_ABI_VERSION` bump implements this BC's check. It is invoked from
   inside `crates/factory-dispatcher/src/shard_manager.rs` (new module), called from the
   dispatcher's own PreToolUse handling path, architecturally analogous to the existing native
   `block_if_marker_check` precedent (`indeterminate_marker.rs`, consulted from `executor.rs`).

2. **The cap value is always expressed and compared in bytes.** No code path in this BC's
   implementation may substitute a line count, a character count under a non-byte encoding
   assumption, or any other non-byte unit for `shard_cap_bytes`, `current_shard_bytes`,
   `payload_bytes`, or `projected_size`. This invariant governs the `"flat"` shape only;
   Postcondition 8's item-count trigger is explicitly exempt (it is item-count-denominated by
   design, not a violation of byte-denomination).

3. **Unmatched paths never pay a `stat()` cost against the TARGET ARTIFACT (scope clarified,
   F-C1-P3-003, v1.9), NOR a validation-fail cost from a malformed SIBLING entry (scope extended,
   F-C1-P6-001, v1.12).** The config-match check (Postcondition 1) MUST occur before any filesystem
   `stat()` call against the target artifact's own file, so that Layer 2 imposes zero measurable
   per-call overhead, on the target artifact, for the ~99% of `Edit`/`Write`/`MultiEdit` calls that
   do not target a sharded artifact. This invariant is scoped to the target artifact specifically;
   it does NOT forbid the one-time, fixed-size, fuel-free `[[shard]]` config-presence probe
   (`Path::exists()` against the fixed config path) that gates whether this BC's check runs at all
   — that probe is the necessary applicability check this BC's own gate placement requires, is O(1)
   and fixed-size regardless of the target artifact, and is not a per-target-path filesystem read of
   the artifact itself. **Extended (v1.12, F-C1-P6-001):** the config-match check (`find_matching_
   entry`) MUST likewise occur before ANY entry's semantic VALIDATION (`validate_entry` — EC-009/
   EC-011/EC-012/EC-013/EC-015/EC-016/EC-017/EC-022) is attempted, and validation MUST run against the
   MATCHED entry only — never against sibling entries the current dispatch does not target. A
   dispatch whose target path does not match the malformed entry (whether it matches nothing, or
   matches a DIFFERENT well-formed entry) MUST NEVER observe `HookResult::Error` as a consequence
   of that sibling's malformation (see Postcondition 1's "Blast-radius scoping" sub-paragraph for
   the full ruling, and EC-019 for the one unavoidable residual exception — whole-file TOML
   structural parse failure).

4. **The formula's four inputs are configuration, not embedded constants.** `PRACTICAL_FUEL_CEILING`,
   `WORST_CASE_FUEL_PER_BYTE`, `MAX_SINGLE_RECORD_BYTES`, and `SAFETY_MARGIN` are read from the
   `[[shard]]` config (or the shard-index TOML per BC-1.18.006 Decision 4's schema), never
   hardcoded as Rust constants inside `shard_manager.rs` — this is what makes the F4 harness's
   recalibration a config change, not a code change. **Extended (fix-burst pass-3, F-P3-005):** `N`
   and `low_water_mark` (Postcondition 8's item-count-shape config pair) are likewise always
   configuration, never embedded constants or a derived-and-hardcoded value. **Timing clarified
   (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-2 finding F-C1-P2-003, product-owner
   adjudication, v1.8):** `low_water_mark`'s default (`floor(N/2)`) MUST always be DERIVED FROM the
   entry's own config `N` — it MUST NEVER be a hardcoded fallback constant baked into
   `shard_manager.rs`. The derivation's TIMING is not itself load-bearing: an implementation MAY
   materialize the default eagerly while parsing the `[[shard]]` config, OR resolve it lazily at
   first use/consumption (e.g. a pure `resolved_low_water_mark(n, low_water_mark)` function called
   at rotation-target resolution, per BC-1.18.009) — either shape satisfies this invariant, because
   both derive the value from config `N` at the moment it is needed and neither ever substitutes a
   constant. What this invariant forbids is a *value*, not a *when*: e.g. a Rust `const
   DEFAULT_LOW_WATER_MARK_FALLBACK: u64 = 25` (or any other numeral not derived from that entry's
   own `N`) would violate this invariant regardless of when it were read.

5. **The `shape` field is read once per config entry, never inferred from the target path's
   content or extension.** Shape dispatch (Postcondition 8) is a config-declared property of the
   `[[shard]]` entry, not a runtime heuristic over the file's actual structure — this keeps the
   dispatch itself a cheap, deterministic config lookup rather than a content-sniffing operation.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Tool call target path does not match any `[[shard]]` config entry | `Continue` immediately; no `stat()` call; zero overhead (Postcondition 1) |
| EC-002 | `projected_size` exactly equals `shard_cap_bytes` (boundary, not exceeding) | `Continue` — no roll (Postcondition 3's `<=` comparison is inclusive of the boundary) |
| EC-003 | `projected_size` exceeds `shard_cap_bytes` by exactly 1 byte | Roll triggers (Postcondition 3's `>` comparison; see BC-1.18.006 for the observable roll/block sequence) |
| EC-004 | The matched artifact's current shard does not yet exist on disk (first write ever) | `current_shard_bytes` treated as 0; `projected_size = payload_bytes`; compared normally against `shard_cap_bytes` |
| EC-005 | `MultiEdit` with multiple edit blocks, some net-negative (deletions), some net-positive | `payload_bytes` is the SUM of each edit's net length delta (may be negative overall — a net-shrinking `MultiEdit` never triggers a roll even if individual edit blocks are large) |
| EC-006 | The harness (F4) finds a Cohort B plugin's fuel-per-byte relationship is superlinear, not linear | `WORST_CASE_FUEL_PER_BYTE` for that plugin MUST use the local marginal rate at the largest tested fixture size, not the global average slope (Postcondition 7) |
| EC-007 | `DEFAULT_FUEL_CAP` changes at the operator level after F4 lock (e.g., rc.24 ships 20M) | `PRACTICAL_FUEL_CEILING` and all downstream `shard_cap_bytes` values MUST be recomputed via a harness re-run, not a naive proportional scale (Postcondition 7) |
| EC-008 | A `[[shard]]` config entry for `BC-INDEX.md` declares `shape = "frontmatter-changelog-array"` and the live `changelog:` sequence is at exactly N items | Postcondition 8's item-count trigger fires (`current_item_count + 1 = N+1 > N`); BC-1.18.009's rotate-then-block-and-retry outcome applies, NOT this BC's own byte-size roll path |
| EC-009 | A `[[shard]]` config entry omits the `shape` field entirely (malformed config) | Fail-loud: this BC's implementation MUST NOT default silently to either shape; the dispatch is treated as a config error (`HookResult::Error`), never a silent `Continue` that would leave an oversized artifact unguarded |
| EC-010 (fix-burst pass-3, F-P3-005; timing clarified S-25.02 Phase F4 LOCAL adversary cluster-1 pass-2, F-C1-P2-003, v1.8) | A `"frontmatter-changelog-array"`-shaped config entry omits `low_water_mark` | Defaults to `floor(N/2)`, DERIVED FROM the entry's own config `N` (Postcondition 8's rotation-target-config bullet) — NEVER a hardcoded fallback constant in `shard_manager.rs`. Derivation MAY be materialized eagerly at config-load time OR deferred to first-use/resolution (e.g. `resolved_low_water_mark`) — timing is an implementation choice, not a contract obligation; the obligation is that the value is always computed from `N`, never a baked-in numeral |
| EC-011 (fix-burst pass-3, F-P3-005; scope CORRECTED fix-burst pass-4, F-P4-001, HIGH) | A `"frontmatter-changelog-array"`-shaped config entry declares `low_water_mark >= N` (the `== N` boundary included) or a negative `low_water_mark` | Fail-loud: `HookResult::Error` — the config is treated as malformed and NEVER silently clamped or defaulted around (mirrors EC-009's posture for the `shape` field, extended to this field). Scope is EXACTLY `>= N` or negative — `low_water_mark = N-1` is NOT in this scope; see EC-012 |
| EC-012 (fix-burst pass-4, F-P4-001, HIGH, ADR-051 v1.4 Decision 14 adjudication Option (b)) | A `"frontmatter-changelog-array"`-shaped config entry declares a legal-but-poorly-amortizing `low_water_mark` in `(floor(N/2), N)`, up to and including the boundary value `N-1` | Loads normally — `Continue`, config load NEVER returns `HookResult::Error` (the value satisfies `0 <= low_water_mark < N`) — but config-load emits a non-fatal `tracing::warn!` amortization advisory citing the configured `(N, low_water_mark)` pair and the resulting amortization factor `N - low_water_mark`, compared against the default's `N - floor(N/2)` amortization |
| EC-013 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 finding, product-owner adjudication) | A `[[shard]]` config entry declares `shard_cap_bytes` GREATER than `compute_shard_cap_bytes(entry.cap_formula_inputs())` — i.e., the declared cap exceeds its own formula-derived ceiling (applies to ANY `shape`, not `"flat"`-only) | Fail-loud: `ShardRegistry::load()` returns `HookResult::Error` — the misconfigured entry is REJECTED; NEVER silently accepted, NEVER silently clamped down to the computed ceiling (Postcondition 9) |
| EC-014 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3 finding F-C1-P3-001, MEDIUM, product-owner adjudication) | A `"frontmatter-changelog-array"`-shaped target artifact does not yet exist on disk (a legitimate first-ever `Write` CREATES it) | `read_changelog_item_count` maps `io::ErrorKind::NotFound` to `Ok(0)` — treated as 0 existing items, mirroring EC-004's flat-shape missing-file precedent; the item-count trigger (`current_item_count + 1 > N`) evaluates `0 + 1 > N`, which is `false` for any `N >= 1`, so `Continue` — the legitimate create is NEVER hard-blocked as `HookResult::Error`. Any OTHER `io::Error` kind reading an EXISTING file (malformed frontmatter fence, permissions failure, etc.) is UNCHANGED and still propagates as `HookResult::Error` — only `NotFound` is special-cased |
| EC-015 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-001, MEDIUM/HIGH, product-owner adjudication) | A `[[shard]]` config entry declares `worst_case_fuel_per_byte` as non-finite (`NaN`, `inf`, or `-inf`) or `<= 0.0` (including `0.0` itself — the "divisor-door" case that saturates `compute_shard_cap_bytes` to near-`u64::MAX`) | Fail-loud: `ShardRegistry::load()` returns `HookResult::Error` — the entry is REJECTED BEFORE the cap-vs-formula comparison (Postcondition 9) is even attempted; `practical_fuel_ceiling`, `max_single_record_bytes`, and `safety_margin` need no analogous check (all `u64`-typed; a degenerate `0` drives the ceiling toward the safe direction, never away from it). See also EC-017 for the residual closure covering a LEGAL tiny-positive divisor whose RESULT still saturates |
| EC-016 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-002, MEDIUM, product-owner adjudication) | A `"frontmatter-changelog-array"`-shaped `[[shard]]` config entry omits the required `N` item-count trigger threshold | Fail-loud: `ShardRegistry::load()` returns `HookResult::Error` — evaluated at LOAD time (not merely gate/dispatch time), and BEFORE `low_water_mark` is examined, so an entry that also declares an invalid `low_water_mark` is never silently accepted merely because `N` happened to be absent |
| EC-017 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5 finding F-C1-P5, MEDIUM/HIGH, product-owner adjudication) | A `[[shard]]` config entry declares a `worst_case_fuel_per_byte` that is finite and `> 0.0` (passes EC-015's guard) but is so small (e.g. `1e-300`) that the RAW division `practical_fuel_ceiling as f64 / worst_case_fuel_per_byte` is non-finite or `>= u64::MAX as f64` — i.e. the result saturates the subsequent `.floor() as u64` cast to (or beyond) `u64::MAX`, even though the divisor itself is legal under EC-015 | Fail-loud: `ShardRegistry::load()` returns `HookResult::Error` (new `ShardConfigError::FormulaCeilingSaturated` variant) — evaluated BEFORE the `CapExceedsFormulaCeiling` comparison (Postcondition 9); the entry is REJECTED, never silently accepted with a saturated (and therefore meaningless) computed ceiling |
| EC-018 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, LOW pending-intent, product-owner adjudication) | A `[[shard]]` config entry for an artifact OTHER than the current dispatch's target is semantically malformed (fails any of EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017's checks), and the target path does not match that malformed entry — it matches a DIFFERENT, well-formed entry, or matches no entry at all | `Continue` (unmatched target) or normal gate logic against the correctly-matched, well-formed entry (matched-but-different target) — the malformed sibling entry is NEVER validated (`validate_entry` is called only on the MATCHED entry, per-dispatch) and NEVER causes `HookResult::Error` for this dispatch (Postcondition 1's "Blast-radius scoping" ruling, Invariant 3 extension) |
| EC-019 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, LOW pending-intent, product-owner adjudication) | The `[[shard]]` config FILE itself fails STRUCTURAL TOML deserialization — invalid TOML syntax, or any entry omits a non-`Option`-typed field required for `toml::from_str` to succeed (`artifact_stem`, `shard_cap_bytes`, or any of the four `cap_formula_inputs` fields) | Fail-loud: `HookResult::Error` for EVERY `Edit`/`Write`/`MultiEdit` dispatch while the config file exists, matched or not — `find_matching_entry` cannot run without a successfully deserialized registry. This is the ONE residual case where a config defect retains whole-file blast radius; it is inherent to TOML's grammar, not the eager-semantic-validation pattern EC-018 closes, and is NOT weakened by this BC's v1.12 ruling |
| EC-020 (NEW, PR #818 fix-burst finding B1, product-owner adjudication, v1.13 backfill of an already-shipped code ruling) | A `"frontmatter-changelog-array"`-shaped target artifact EXISTS on disk but carries NO well-formed `---` frontmatter fence at all (no line-anchored opening fence, or an opening fence with no line-anchored closing fence found before end-of-file) | Permissive: `read_changelog_item_count` returns `Ok(0)` — mirroring EC-014's `NotFound`-is-zero precedent, because a file with no delimited frontmatter block genuinely has zero delimited items, not an undercounted unknown quantity. `current_item_count + 1 = 1 > N` is `false` for any `N >= 1`, so the target is never hard-blocked. Distinguish sharply from EC-021's malformed-YAML case below, which fails loud |
| EC-021 (NEW, PR #818 fix-burst finding M-1, MAJOR, product-owner adjudication, v1.13) | A `"frontmatter-changelog-array"`-shaped target artifact EXISTS on disk with a well-formed `---`/`---` fence pair, but the YAML content between the fences fails to parse (`serde_norway::from_str` returns `Err`) | Fail-loud: `read_changelog_item_count` maps the parse failure to `io::ErrorKind::InvalidData` and propagates `Err`, which the gate surfaces as `HookResult::Error` — NEVER coerced to `Ok(0)` (distinct from EC-014's `NotFound` and EC-020's fenceless cases, neither of which involves actual unparsed YAML content whose count could be silently lost). The error message MUST name the YAML-parse cause specifically and MUST state that the gate only intercepts `Edit`/`Write`/`MultiEdit`, so the malformed block can be repaired via any other means (e.g., a `Bash`-invoked edit) to escape the resulting self-deadlock |
| EC-022 (NEW, PR #818 cycle-4 fresh-context PR reviewer finding F-1, product-owner adjudication, v1.14 backfill of an already-shipped code ruling) | A `[[shard]]` config entry (of ANY `shape`) declares an `artifact_path` that is EMPTY, or normalizes to empty after `Component::CurDir` filtering (`""`, `"."`, `"./"`) | Fail-loud: `validate_entry` returns `Err(ShardConfigError::EmptyArtifactPath { artifact_stem, artifact_path })`, surfaced as `HookResult::Error` at entry-match time (Postcondition 1's v1.12 MATCH-FIRST scoping — only the MATCHED entry is checked, evaluated BEFORE any cap-formula arithmetic) — NEVER a silent vacuous `[] == []` component-suffix match that would otherwise cause the entry to mis-apply its shard-cap (and item-count, for a `"frontmatter-changelog-array"`-shaped entry) gate to every unrelated `Edit`/`Write`/`MultiEdit` dispatch in the repository |
| EC-023 (NEW, v1.16; S-25.09 black-box finding; ADR-052 §Decision 11(g)(2); Postcondition 10) | `ShardConfigError` rendering of config-derived data on every surface: (a) a `[[shard]]` entry with NO `shape` and a hostile `artifact_stem` containing ESC (`\x1b[31m`) and LF, driven through the PreToolUse shard-cap gate; (b) a `[[shard]]` entry with a 400-character `artifact_stem`; (c) `shard-config.toml` with a wrong-typed value carrying a secret-looking string (e.g. `shard_cap_bytes = "sk-live-SECRET"`) so the `Toml` variant is produced; (d) the same inputs reached through `migrate-bc-index` (`ShardCapConfigUnavailable`); (e) a benign `artifact_stem` (`lessons`) | (a) `MissingShape` renders on ONE line with the ESC and LF escaped (no raw control byte in `block_reason`, stderr or any log), exit 2, `blocking_plugins=shard-cap-gate`; (b) the `artifact_stem` slot is capped at 256 characters while the fixed message text is intact and untruncated; (c) the rendering carries error category plus line/column only: the secret string, the toml message and the offending source line never appear; (d) identical escaped, capped, category-only rendering on the `migrate-bc-index` stderr; (e) control: rendered verbatim, byte-identical to the pre-change message; further vectors (ADR-052 §Decision 11(g)(2)(f)): (f) the same hostile and 400-character stems rendered into an InternalLog/tracing event field ⇒ each data slot capped at 64 with the fixed text intact, from the same `render_with_cap` source as `Display` (no divergence between the two renderings for any variant; the match has no wildcard arm); (g) the wrong-typed secret value ⇒ the stored `Toml` variant holds only (category, line, column), so neither `Display`, `Debug` nor any event can contain the secret or the toml message; (h) `ShardCapConfigUnavailable.detail` equals the sanitized `ShardConfigError` rendering (escaped, one line, capped, no double escape) for the hostile-stem and secret-bearing inputs |
| EC-024 (NEW, v1.17; ADR-052 §Decision 11(c) point 7(e) row (11)(ii); Postcondition 11) | A `[[shard]]` entry in `shard-config.toml` whose `artifact_stem` is not a safe path component: `../../specs/x`, an absolute path, a stem containing a separator or `..`, an empty stem, or an ESC- or LF-bearing stem | The config load fails at decode with `ShardConfigError::ArtifactStemUnsafe`; no `ShardEntry` is built; the stem reaches no path join, `rename`, `remove` or `read_dir`; the message is rendered under Postcondition 10 (one line, the stem slot escaped and capped at 256 on `Display` / 64 in an event, fixed text intact); `HookResult::Error` exit 2 on the gate, and the same sanitized rendering inside `ShardCapConfigUnavailable.detail` on `migrate-bc-index`; no fallback stem is substituted |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| **CORRECTED (fix-burst pass-2, F-P2-002).** `Write` to `decision-log.md`, current shard 40,000 bytes (irrelevant to the `Write` formula), `content` length 5,000 bytes, `shard_cap_bytes=49,152` | `projected_size = len(content) = 5,000 <= 49,152` → `Continue` — `current_shard_bytes` is NOT added for `Write` | happy-path |
| **CORRECTED (fix-burst pass-2, F-P2-002).** `Write` to `decision-log.md`, current shard 45,000 bytes (irrelevant), `content` length 50,000 bytes, `shard_cap_bytes=49,152` | `projected_size = len(content) = 50,000 > 49,152` → roll triggers (BC-1.18.006) | edge-case |
| **NEW regression vector (fix-burst pass-2, F-P2-002) — demonstrates the WITHDRAWN formula's over-trigger bug.** `Write` to `decision-log.md`, current shard 40,000 bytes, `content` length 40,000 bytes (a same-size full-file rewrite), `shard_cap_bytes=49,152` | CORRECTED: `projected_size = len(content) = 40,000 <= 49,152` → `Continue`. The WITHDRAWN formula would have wrongly computed `40,000 + 40,000 = 80,000 > 49,152` and over-triggered a roll on an ordinary same-size `Write` | edge-case |
| `Edit` to `decision-log.md`, current shard 45,000 bytes, `old_string`/`new_string` netting +5,000 bytes, `shard_cap_bytes=49,152` | `Edit`/`MultiEdit` formula UNCHANGED: `projected_size = current_shard_bytes + net_delta_bytes = 45,000 + 5,000 = 50,000 > 49,152` → roll triggers (BC-1.18.006) | edge-case |
| `Edit` to `some/unrelated/file.md` (no `[[shard]]` match) | `Continue`, zero `stat()` calls | happy-path |
| `Write` to `burst-log.md`, `cap_for(validate-burst-log)=40,000`, `cap_for(regression-gate)=49,152`, `cap_for(convergence-tracker)=52,000` | Effective `shard_cap_bytes = MIN(40000, 49152, 52000) = 40,000` (Cross-Validator Minimum Rule) | edge-case |
| `Write` to `decision-log.md` (not read by `validate-burst-log`), `cap_for(regression-gate)=49,152`, `cap_for(convergence-tracker)=52,000` | Effective `shard_cap_bytes = MIN(49152, 52000) = 49,152` — `validate-burst-log`'s cap is NOT applied | edge-case |
| `MultiEdit` on `lessons.md` with edits netting +2,000 / -500 / +100 bytes, current shard 48,000, `shard_cap_bytes=49,152` | `net_delta_bytes = 2000-500+100 = 1,600`; `projected_size = current_shard_bytes + net_delta_bytes = 49,600 > 49,152` → roll triggers (`Edit`/`MultiEdit` formula, unchanged) | edge-case |
| `projected_size` == `shard_cap_bytes` exactly (boundary) | `Continue` — inclusive boundary, no roll | error |
| `Edit` to `BC-INDEX.md`'s frontmatter, `shape="frontmatter-changelog-array"`, `N=50`, live `changelog:` at 50 items | `current_item_count + 1 = 51 > 50` → item-count trigger fires (Postcondition 8); BC-1.18.009's rotation-then-retry outcome applies | edge-case |
| `Edit` to `BC-INDEX.md`'s frontmatter, `shape="frontmatter-changelog-array"`, `N=50`, live `changelog:` at 10 items | `current_item_count + 1 = 11 <= 50` → `Continue`, no rotation | happy-path |
| **NEW (fix-burst pass-3, F-P3-005).** `shape="frontmatter-changelog-array"` config entry declares `N=50`, omits `low_water_mark` | `low_water_mark` defaults to `floor(50/2) = 25` at config-load time (EC-010) | edge-case |
| **NEW (fix-burst pass-3, F-P3-005).** `shape="frontmatter-changelog-array"` config entry declares `N=50`, `low_water_mark=50` (the degenerate `== N` case) or `low_water_mark=-1` (negative) | Fail-loud: `HookResult::Error` in both cases — both values violate `0 <= low_water_mark < N` and are NEVER silently clamped or defaulted around (EC-011). (`low_water_mark=49`, i.e. `N-1`, is a VALID boundary value — `49 < 50` satisfies the constraint — and does NOT fail-loud.) | error |
| **NEW (fix-burst pass-4, F-P4-001, ADR-051 v1.4 Decision 14 adjudication Option (b)).** `shape="frontmatter-changelog-array"` config entry declares `N=50`, `low_water_mark=49` (i.e. `N-1`) | Loads normally — `Continue`, no `HookResult::Error` (EC-012; `49 < 50` satisfies `0 <= low_water_mark < N`) — config-load emits a non-fatal `tracing::warn!` amortization advisory: amortization factor `N - low_water_mark = 50 - 49 = 1` (rotation fires roughly every 1 write), versus the recommended default `low_water_mark=25`'s amortization factor `50 - 25 = 25` | edge-case |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 finding, product-owner adjudication).** `[[shard]]` entry declares `practical_fuel_ceiling=8,000,000`, `worst_case_fuel_per_byte=106.36`, `max_single_record_bytes=16,384`, `safety_margin=8,192` (`compute_shard_cap_bytes(inputs) = 49,152`), and declares `shard_cap_bytes=100,000` (far above its own formula ceiling) | `ShardRegistry::load()` returns `HookResult::Error` (Postcondition 9 / EC-013) — the misconfigured entry is REJECTED; the too-large cap is NEVER loaded and NEVER honored by the live per-write gate | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 finding, product-owner adjudication).** `[[shard]]` entry declares the SAME four inputs as above (`compute_shard_cap_bytes(inputs) = 49,152`) and `shard_cap_bytes=49,152` (exactly equal to its own formula ceiling) | `ShardRegistry::load()` succeeds (`Ok`) — Postcondition 4's `<=` comparison is inclusive of the boundary (Postcondition 9), mirroring EC-002's inclusive-boundary precedent for the per-write trigger | happy-path |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3 finding F-C1-P3-001, product-owner adjudication).** `Write` to a NOT-YET-EXISTING `"frontmatter-changelog-array"`-shaped target artifact (e.g. the first-ever `Write` of a new sharded index file), `shape="frontmatter-changelog-array"`, `N=50`, target file absent on disk, valid frontmatter with a `changelog:` array present in the `content` payload | `read_changelog_item_count` maps `io::ErrorKind::NotFound` to `Ok(0)`; `current_item_count + 1 = 0 + 1 = 1 <= 50` → `Continue` — the create is NEVER hard-blocked as `HookResult::Error` (EC-014; mirrors EC-004's flat-shape missing-file precedent) | edge-case |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-001, product-owner adjudication).** `[[shard]]` entry declares `practical_fuel_ceiling=8,000,000`, `worst_case_fuel_per_byte=0.0` (divisor-door), `max_single_record_bytes=16,384`, `safety_margin=8,192`, `shard_cap_bytes=999,999,999` (arbitrarily large) | `ShardRegistry::load()` returns `HookResult::Error` (EC-015) BEFORE the cap-vs-formula comparison runs — WITHOUT this check, `compute_shard_cap_bytes` would compute `(8,000,000 / 0.0).floor() as u64 = u64::MAX` (saturating cast of `+inf`), the resulting near-`u64::MAX` ceiling would pass ANY declared `shard_cap_bytes` including this vector's `999,999,999`, and Postcondition 9's own protection would be silently defeated | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-001, product-owner adjudication).** `[[shard]]` entry declares the SAME four inputs as the Postcondition-9/EC-013 happy-path vector above except `worst_case_fuel_per_byte=NaN` | `ShardRegistry::load()` returns `HookResult::Error` (EC-015) — `NaN` is non-finite and fails the `worst_case_fuel_per_byte.is_finite() && worst_case_fuel_per_byte > 0.0` guard identically to the `0.0` case above, even though a bare `NaN` division would itself saturate to `0` (the safe direction) rather than `u64::MAX` — the config is still malformed and MUST be rejected regardless of which direction the resulting arithmetic degenerates | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 finding F-C1-P4-002, product-owner adjudication).** `shape="frontmatter-changelog-array"` config entry omits `n` entirely and declares `low_water_mark=999` (which would itself be invalid against any realistic `N`) | `ShardRegistry::load()` returns `HookResult::Error` (EC-016, missing `N`) — caught AT LOAD TIME, not deferred to the first gate-time write against the artifact; the entry's ALSO-invalid `low_water_mark` is superseded by the load-time missing-`N` failure, but the entry is rejected either way, never silently accepted | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5 finding F-C1-P5, product-owner adjudication).** `[[shard]]` entry declares `practical_fuel_ceiling=8,000,000`, `worst_case_fuel_per_byte=1e-300` (legal under EC-015's `is_finite() && > 0.0` guard), `max_single_record_bytes=16,384`, `safety_margin=8,192`, `shard_cap_bytes=999,999,999` (arbitrarily large) | `ShardRegistry::load()` returns `HookResult::Error` (`ShardConfigError::FormulaCeilingSaturated`, EC-017) — the raw division `8,000,000 / 1e-300` is `~8e306`, which is `>= u64::MAX as f64`, so the entry is REJECTED BEFORE `CapExceedsFormulaCeiling` runs; WITHOUT this check, `compute_shard_cap_bytes` would saturate its `.floor() as u64` cast to `u64::MAX`, the resulting near-`u64::MAX` ceiling would pass ANY declared `shard_cap_bytes` including this vector's `999,999,999`, and EC-015's own divisor-door closure would be defeated by a legal-but-degenerate divisor | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, product-owner adjudication, EC-018 pinning vector — THE F-C1-P6-001 scenario).** `Edit` to `src/foo.rs` (matches NO `[[shard]]` config entry). The SAME config's `lessons` entry omits `shape` entirely (would fail EC-009 if matched) | `Continue` — `find_matching_entry` returns `None` for `src/foo.rs` BEFORE `validate_entry` is ever called on ANY entry, so the malformed `lessons` entry's `shape` omission is NEVER evaluated for this dispatch. Contrast with the pre-v1.12 implementation, which called the eager, whole-config `ShardRegistry::load()` validation loop BEFORE matching and returned `HookResult::Error` for this exact dispatch | edge-case |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, product-owner adjudication).** `Edit` to `lessons.md` — matches the malformed `lessons` entry itself (omits `shape`) | `HookResult::Error` (EC-009) — UNCHANGED from pre-v1.12 behavior: the MATCHED entry is still validated and still fails loud, now at entry-match time via `validate_entry` rather than inside `load()`'s eager loop | error |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, product-owner adjudication).** `Write` to `decision-log.md` — matches a DIFFERENT, well-formed entry (`shard_cap_bytes=49,152`, `content` length 5,000 bytes), while the SAME config's sibling `lessons` entry omits `shape` (malformed) | `projected_size = len(content) = 5,000 <= 49,152` → `Continue` — normal Postcondition 3 gate logic against `decision-log.md`'s own well-formed entry proceeds entirely unaffected by the `lessons` entry's malformation (EC-018) | edge-case |
| **NEW (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, product-owner adjudication, EC-019 residual-exception vector).** `Edit` to `src/foo.rs` (matches NO `[[shard]]` entry); the config FILE has invalid TOML syntax (or an entry omits `shard_cap_bytes`, a non-`Option` field) | `HookResult::Error` for this dispatch too — `find_matching_entry` cannot run without a successfully deserialized registry (EC-019). Distinguishes the residual, unavoidable whole-file structural-parse failure from EC-018's semantic-malformation scenario, which this ruling scopes to the matched entry only | error |
| **NEW (PR #818 fix-burst finding B1, product-owner adjudication, v1.13).** `Edit` to a `"frontmatter-changelog-array"`-shaped target artifact that EXISTS on disk with NO `---` frontmatter fence at all (e.g. a plain-text file, or a file whose `content` has not yet had frontmatter added) | `read_changelog_item_count` returns `Ok(0)` (no opening fence found); `current_item_count + 1 = 1 <= N` for any `N >= 1` → `Continue` — the edit is NEVER hard-blocked (EC-020) | edge-case |
| **NEW (PR #818 fix-burst finding B1, product-owner adjudication, v1.13).** `Edit` to a `"frontmatter-changelog-array"`-shaped target artifact with an opening `---` fence but NO line-anchored closing `---` fence before end-of-file | `read_changelog_item_count` returns `Ok(0)` (opening fence found, closing fence never found); `current_item_count + 1 = 1 <= N` for any `N >= 1` → `Continue` — the edit is NEVER hard-blocked (EC-020) | edge-case |
| **NEW (PR #818 fix-burst finding M-1, product-owner adjudication, v1.13) — side-by-side contrast with the two EC-020 vectors immediately above.** `Edit` to a `"frontmatter-changelog-array"`-shaped target artifact with a well-formed `---`/`---` fence pair whose YAML content is malformed (e.g. unbalanced quotes, invalid indentation, a duplicate mapping key) | `read_changelog_item_count` returns `Err(io::Error { kind: InvalidData, .. })` — NEVER `Ok(0)`, unlike the fenceless case above; the gate returns `HookResult::Error` with a message that (a) names the YAML-parse failure specifically and (b) states the gate only intercepts `Edit`/`Write`/`MultiEdit`, directing the operator to repair the frontmatter via any other means to escape the resulting self-deadlock (EC-021) | error |
| **NEW (PR #818 cycle-4 fresh-context PR reviewer finding F-1, product-owner adjudication, v1.14).** `[[shard]]` entry declares `artifact_path = ""` (or `"."`, or `"./"`) — every other field well-formed | `validate_entry` returns `Err(ShardConfigError::EmptyArtifactPath { artifact_stem, artifact_path })` at entry-match time — `HookResult::Error` for any dispatch whose target matches this entry. WITHOUT this check, `path_falls_under_or_equals`'s component-suffix comparison degenerates to `[] == []`, vacuously matching EVERY target path and silently mis-applying this entry's shard-cap (and item-count, for a `"frontmatter-changelog-array"`-shaped entry) gate to unrelated artifacts across the whole repository (EC-022) | error |
| `shard-config.toml` with `[[shard]] artifact_stem = "../../specs/x"` (otherwise valid), loaded by the PreToolUse shard-cap gate (v1.17, EC-024) | Config load fails at decode: `ShardConfigError::ArtifactStemUnsafe`; `HookResult::Error`, exit 2, `blocking_plugins=shard-cap-gate`; the message is one line with the stem slot escaped and capped; no path operation was attempted (directory byte-identical) | error |
| `[[shard]] artifact_stem = "decision-log\x1b[31m\nX"` (ESC and LF) | `ArtifactStemUnsafe` at decode; rendered on ONE line, ESC and LF escaped, no raw control byte in `block_reason`; stem slot capped (256 `Display`, 64 event) | error |
| Same unsafe stem reached through `migrate-bc-index` | `ShardCapConfigUnavailable.detail` equals the sanitized `ArtifactStemUnsafe` rendering (no double escape); exit 2; nothing written | error |
| `[[shard]] artifact_stem = "decision-log"` (and `lessons`, `burst-log`, `session-checkpoints`) | Decodes unchanged; behaviour byte-identical to the pre-change gate | happy-path |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-117 | Unmatched-path zero-cost invariant (no `stat()` call issued when target path does not match any `[[shard]]` config entry); Cross-Validator Minimum Rule (effective cap for a multi-reader artifact equals the MIN of all applicable per-plugin caps); Byte-denomination invariant (no code path compares a non-byte-denominated quantity against `shard_cap_bytes`) | unit test — three facets: mock filesystem call counter; table-driven over the 4 mechanism-A artifacts × 3 Cohort B plugins; arbitrary payload/current-shard sizes compared against a byte-for-byte oracle |
| VP-116 | Boundary inclusivity — `projected_size == shard_cap_bytes` never triggers a roll; `projected_size == shard_cap_bytes + 1` always triggers a roll | kani-proof (exact-boundary + overflow/underflow-safety over symbolic inputs) |
| VP-140 | Postcondition 8 item-count trigger boundary AND `low_water_mark` rotation-target config validation — shape-dispatch read-once; item-count off-by-one (`current_item_count + 1 > N`: `N-1` → Continue, exactly `N`/`N+1` → fire, EC-008); `low_water_mark` default `floor(N/2)` when omitted (EC-010); fail-loud `HookResult::Error` on `low_water_mark >= N` (incl. degenerate `== N`) or negative, `N-1` valid (EC-011); **amortization-advisory biconditional (added VP-140 v1.1, fix-burst pass-4, F-P4-001, ADR-051 v1.4 Decision 14 Option (b)):** a `tracing::warn!` non-fatal amortization advisory fires IFF `low_water_mark > floor(N/2)`, while config-load ALWAYS succeeds (never `HookResult::Error`) for any `0 <= low_water_mark < N`, including `N-1` (EC-012) | unit test — five facets: mock-config shape-dispatch + content-read counter; `{N-1, N, N+1}` trigger-boundary table; even/odd-`N` default table; `{N, N-1, 0, -1}` config-validation table; `low_water_mark ∈ {floor(N/2), floor(N/2)+1, N-1}` amortization-advisory table asserting the advisory fires iff `low_water_mark > floor(N/2)` and config-load always succeeds |

**Fix-burst note (F-S2502-F2-003):** the two rows above that previously read "unit test" (for
VP-116) and "proptest" (for VP-117's byte-denomination row) were reconciled to the authoritative
`VP-INDEX.md` v3.02 catalog assignment — VP-116 = kani-proof, VP-117 = unit-test — in this
fix-burst. No property content changed, only the Proof Method column.

**Fix-burst note (fix-burst pass-3, F-P3-006):** VP-117's three previously-separate rows are
collapsed to ONE row listing its three facets (multi-facet convention — VP-117 is a single
allocated VP covering all three properties, not three separate VPs). No property content or
coverage change, only table presentation.

**Adjudication note (S-25.02 Phase F4 LOCAL adversary cluster-1, product-owner, v1.7):** new
Postcondition 9 / EC-013 (load-time fail-loud enforcement of `shard_cap_bytes <=
compute_shard_cap_bytes(inputs)`) is a NEW verification obligation not yet covered by VP-116,
VP-117, or VP-140. Per this BC's own precedent for Postcondition 8 (added at v1.1 without a VP,
with VP-140 allocated by formal-verifier in a later burst — see the §VP Anchors note below),
product-owner does NOT allocate a VP number here. **Routed to architect/formal-verifier:** allocate
a new VP (or extend VP-117's multi-facet unit-test row) covering the Postcondition 9 / EC-013
config-validation property — `compute_shard_cap_bytes(inputs) >= shard_cap_bytes` boundary-inclusive
check at `ShardRegistry::load()` time — and propagate to VP-INDEX.md, verification-architecture.md,
and verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Adjudication note (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3, product-owner, v1.9):** new
EC-014 (item-count shape's `NotFound`-is-zero missing-file graceful-degradation behavior, mirroring
EC-004's flat-shape precedent) is a NEW verification obligation not yet covered by VP-140's existing
five facets (shape-dispatch, item-count off-by-one, `low_water_mark` default, fail-loud validation,
amortization advisory). Per this BC's own Postcondition-9/EC-013 precedent immediately above (where
product-owner did not invent a new VP number for a load-time-validation extension), product-owner
does NOT allocate a VP number or facet here either. **Routed to architect/formal-verifier:** extend
VP-140 with a sixth facet (or allocate a new VP) covering `read_changelog_item_count`'s
`NotFound -> Ok(0)` mapping and the resulting `0 + 1 <= N` non-trigger for a not-yet-existing target
artifact, and propagate to VP-INDEX.md, verification-architecture.md, and
verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Adjudication note (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4, product-owner, v1.10):** new
EC-015 (non-finite/non-positive `worst_case_fuel_per_byte` load-time fail-loud "divisor-door" guard)
and EC-016 (missing-`N` load-time fail-loud guard for the `"frontmatter-changelog-array"` shape) are
NEW verification obligations not yet covered by VP-116, VP-117, or VP-140. Per this BC's own
Postcondition-9/EC-013 and EC-014 precedent immediately above (product-owner does not self-allocate
a VP number for a load-time-validation extension), product-owner does NOT allocate a VP number here
either. **Routed to architect/formal-verifier; VP allocation DEFERRED TO F6 (per the PC9/EC-014
precedent above):** extend VP-117's multi-facet unit-test row (or allocate a new VP) covering (a)
EC-015's `worst_case_fuel_per_byte` finiteness/positivity guard at `ShardRegistry::load()` time —
including the `0.0` divisor-door case and the `NaN` non-finite case — and (b) EC-016's missing-`N`
load-time guard for the `"frontmatter-changelog-array"` shape, including the restructured ordering
that keeps EC-011 from being silently bypassed on an entry that also omits `N`; propagate to
VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md per
`vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Adjudication note (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5, product-owner, v1.11):** new
EC-017 (residual divisor-door closure — saturated/non-finite RAW computed-ceiling guard, distinct
from EC-015's divisor-legality guard) is a NEW verification obligation not yet covered by VP-116,
VP-117, or VP-140. Per this BC's own EC-013/EC-014/EC-015/EC-016 precedent (product-owner does not
self-allocate a VP number for a load-time-validation extension), product-owner does NOT allocate a
VP number here either. **Routed to architect/formal-verifier; VP allocation DEFERRED TO F6 (per the
precedent above):** extend VP-117's multi-facet unit-test row (or allocate a new VP) covering
EC-017's saturated-ceiling guard — including the `1e-300` tiny-positive-divisor case that passes
EC-015 yet still saturates the `.floor() as u64` cast — and propagate to VP-INDEX.md,
verification-architecture.md, and verification-coverage-matrix.md per
`vp_index_is_vp_catalog_source_of_truth` (POLICY 9). (Item 2's `replace_all` multiplicity gap is
DEFERRED to BC-1.18.006, per Postcondition 3's new sub-paragraph — no VP obligation arises in THIS
BC from Item 2, since no code/test change follows here; BC-1.18.006's own spec-evolution burst owns
allocating any VP needed for its closure.)

**Adjudication note (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6, product-owner, v1.12):** new
EC-018 (malformed-sibling-entry blast-radius containment — entry validation scoped to the MATCHED
entry only) and EC-019 (residual whole-file structural-parse-failure exception) are NEW verification
obligations not yet covered by VP-116, VP-117, or VP-140. Per this BC's own EC-013 through EC-017
precedent (product-owner does not self-allocate a VP number for a load/match-time-validation
extension), product-owner does NOT allocate a VP number here either. **Routed to architect/
formal-verifier; VP allocation DEFERRED TO F6 (per the EC-013..EC-017 precedent above):** extend
VP-117's multi-facet unit-test row (or allocate a new VP) covering (a) EC-018's blast-radius
containment — a mock-registry test asserting `validate_entry` is invoked ZERO times for entries
other than the one `find_matching_entry` resolves, across the three sub-cases (unmatched target,
matched-to-a-different-well-formed-entry target, and matched-to-the-malformed-entry target), and
(b) EC-019's residual whole-file structural-parse-failure exception (invalid TOML syntax, or a
missing non-`Option` field) still propagating `HookResult::Error` for every dispatch regardless of
match; propagate to VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md
per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Routing note (v1.12, F-C1-P6-001) — code restructuring required (Reading (B) adopted):** unlike
the v1.7-v1.11 amendments (wording clarifications or new load-time checks layered onto the existing
eager-validate-all-entries design), this ruling requires an ACTUAL code restructuring of
`crates/factory-dispatcher/src/shard_manager.rs` and `crates/factory-dispatcher/src/executor.rs` —
splitting `ShardRegistry::load()` into structural-parse-only and a new `validate_entry` function
invoked AFTER `find_matching_entry`, per Postcondition 1's "Blast-radius scoping" sub-paragraph.
**Routed to implementer** (the restructuring itself) **and test-writer** (the EC-018/EC-019 pinning
tests specified in the new Canonical Test Vectors above, plus a regression test asserting the
existing EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017 fail-loud outcomes are preserved for a
MATCHED malformed entry post-restructuring).

**Adjudication note (PR #818 fix-burst findings B1 + M-1, product-owner, v1.13):** new EC-020
(present-but-fenceless permissive `Ok(0)`, backfilling an already-shipped code ruling) and EC-021
(malformed-YAML fail-loud, with a MANDATORY error-message escape-hatch requirement) are NEW
verification obligations not yet covered by VP-116, VP-117, or VP-140. Per this BC's own EC-013
through EC-019 precedent (product-owner does not self-allocate a VP number for a
load/read-path-validation extension), product-owner does NOT allocate a VP number here either.
**Routed to architect/formal-verifier; VP allocation DEFERRED TO F6 (per the EC-013..EC-019
precedent above):** extend VP-140's multi-facet unit-test row (or allocate a new VP) covering (a)
EC-020's permissive `Ok(0)` outcome for both the missing-opening-fence and missing-closing-fence
sub-cases, (b) EC-021's fail-loud outcome for malformed-but-fenced YAML (asserting `Ok(0)` is NEVER
returned for this case, in contrast to EC-014's `NotFound` and EC-020's fenceless cases at the same
call site), and (c) a message-content assertion that the resulting `HookResult::Error`'s message
text names the YAML-parse cause AND states the gate's `Edit`/`Write`/`MultiEdit`-only scope as the
self-deadlock escape guidance; propagate to VP-INDEX.md, verification-architecture.md, and
verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Routing note (v1.13, PR #818 fix-burst finding M-1) — code change required (error-message
enhancement, Reading (A) FAIL-LOUD adopted over Reading (B) PERMISSIVE):** unlike EC-020 (which
backfills spec text for an already-shipped, already-compliant code ruling — no code change), this
ruling requires `read_changelog_item_count`'s YAML-parse `map_err` branch
(`crates/factory-dispatcher/src/shard_manager.rs`) to be EXTENDED with the escape-hatch guidance
text specified in Postcondition 8's malformed-YAML bullet above — the current generic `"{path}:
frontmatter YAML parse failed: {e}"` message does not name the gate's `Edit`/`Write`/`MultiEdit`-only
scope or point the operator at an out-of-band repair path, and is therefore NON-COMPLIANT with this
Postcondition from v1.13 forward. **Routed to implementer** (the message-text amendment) **and
test-writer** (a new EC-021 pinning test asserting (i) `Err` propagation with `InvalidData` kind is
preserved for malformed-but-fenced YAML — i.e. that FAIL-LOUD behavior is unchanged — and (ii) the
resulting message text contains both a YAML-parse-cause substring and an escape-hatch-guidance
substring naming the `Edit`/`Write`/`MultiEdit` gated-tool scope; a substring assertion, not a full
string match, so future message wording refinements do not spuriously break the test as long as
both required elements remain present). No code change is required for EC-020 (B1) — the sanctioned
implementation's two `Ok(0)` early-returns already satisfy this Postcondition's fenceless-case
bullet as written.

**Adjudication note (PR #818 cycle-4 fresh-context PR reviewer finding F-1, product-owner, v1.14):**
new EC-022 (empty-`artifact_path` structural defect guard, backfilling an already-shipped code
ruling) is a NEW verification obligation not yet covered by VP-116, VP-117, or VP-140. Per this BC's
own EC-013 through EC-021 precedent (product-owner does not self-allocate a VP number for a
load/match-time-validation extension), product-owner does NOT allocate a VP number here either.
**Routed to architect/formal-verifier; VP allocation DEFERRED TO F6 (per the EC-013..EC-021
precedent above):** extend VP-117's multi-facet unit-test row (or allocate a new VP) covering
EC-022's empty-`artifact_path` guard — asserting `validate_entry` rejects `artifact_path ∈ {"", ".",
"./"}` (and any other sequence that filters down to empty after `Component::CurDir` removal) with
`ShardConfigError::EmptyArtifactPath` BEFORE the component-suffix match is ever evaluated against a
target path, and that the guard runs at entry-match time (the matched entry only, per Postcondition
1's v1.12 MATCH-FIRST scoping) — and propagate to VP-INDEX.md, verification-architecture.md, and
verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

**Routing note (v1.14, PR #818 cycle-4 finding F-1) — NO further code or test change required (spec
backfill for already-shipped, already-tested code, mirroring EC-020's v1.13 precedent):** the
implementer's fix — a new `ShardConfigError::EmptyArtifactPath { artifact_stem, artifact_path }`
variant enforced in `validate_entry` — was already shipped in the SAME PR #818 cycle-4 fix-burst, at
develop squash-commit `fff5e4cc`, with 7 covering tests (`test_BC_1_18_005_F1_*`) landed in the same
commit. This BC amendment closes the spec-anchor gap the fresh-context PR reviewer identified —
shipped, tested code with no prior BC text describing it — it does not itself require a further code
or test change. If the F6 VP-allocation review above finds the existing `test_BC_1_18_005_F1_*`
tests do not already satisfy the VP-140/VP-117 facet extension's coverage bar, closing that gap is
formal-verifier's task at F6, not a reopening of this spec backfill.

## Related BCs

- BC-1.18.006 — owns the observable roll/block outcome once this BC's mechanism-A trigger fires (depends on); ALSO owns closing the `replace_all: true` occurrence-multiplicity gap in this BC's Postcondition 3 formula before its own roll/block outcome is treated as production-ready (NEW, v1.11, F-C1-P5 explicit deferral anchor — see Postcondition 3's "Known formula gap" sub-paragraph)
- BC-1.18.007 — retention/compaction policy for shards this BC's cap produces (composes with)
- BC-1.18.008 — one-time backfill-split applies this BC's SAME formula retroactively (depends on)
- BC-1.18.009 — owns the observable rotate/block-and-retry outcome once this BC's item-count trigger (Postcondition 8) fires (depended on by)
- BC-1.18.012 — the governed one-time B1 changelog backfill migration whose successful completion is what MAKES Postcondition 8's steady-state "bounded" characterization true (the cold-state characterization above holds only before this BC's migration runs) (depended on by)
- BC-7.08.001 — Cohort B fail-closed flip is gated on this BC's postconditions holding at F4-lock (depended on by)
- BC-1.18.001 — sibling Layer-1 fail-closed INDETERMINATE contract this BC's cap prevents from firing (related to)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` (new module) — cap-formula evaluation, `[[shard]]` config resolution, `stat()`-only size check, item-count trigger read
- `crates/factory-dispatcher/src/executor.rs` — defines `shard_cap_precheck` (the native gate itself); `execute_tiers` is where it was invoked PRIOR to PR #824 cycle-3 MAJOR-3
- `crates/factory-dispatcher/src/main.rs` — **UPDATED (PR #824 cycle-3 MAJOR-3, ADR-051 §Decision 17):** `main::run` is now the PreToolUse dispatch path invocation point for `shard_cap_precheck`, called before the `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return guard (hoisted out of the registry-driven `execute_tiers` loop, which `main::run` would otherwise skip entirely on an empty matched-plugin set, silently defeating the gate). ADR-051 §Decision 1's original placement rule (a native, non-registry-gated check must run before any guard that could skip it) is unchanged; §Decision 17 documents the hoisted call site as an addendum.
- `crates/factory-dispatcher/src/indeterminate_marker.rs` — `block_if_marker_check` native-check precedent this BC's placement pattern follows
- `plugins/vsdd-factory/hooks-registry.toml` — `[[shard]]` config table (or sibling `shard-config.toml`, TBD at F4), including the `shape` field this BC's Postcondition 8 dispatches on

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
`grep -n` and file:line citations are forbidden per TD-VSDD-091 — anchors below are grepped by
stable pattern only):

```
$ grep -oE "^pub enum HookResult" crates/hook-sdk/src/result.rs
pub enum HookResult
```

```
$ grep -oE "^\s*(Continue|Block \{[^}]*\}|Error \{[^}]*\})" crates/hook-sdk/src/result.rs | sed -E 's/^\s+//' | sort -u
Block { reason: String }
Continue
Error { message: String }
```

Confirms `HookResult`'s three-variant contract this BC's Postcondition 3 hands off to (roll
triggered → BC-1.18.006's `Block`; this BC itself never constructs a `HookResult` other than
`Continue` for the trigger-boundary role it owns).

```
$ grep -oE "^pub fn write_indeterminate_marker|^pub fn should_write_marker|^pub fn block_if_marker_check" crates/factory-dispatcher/src/indeterminate_marker.rs
pub fn block_if_marker_check
pub fn should_write_marker
pub fn write_indeterminate_marker
```

Confirms the native, non-WASM `block_if_marker_check` precedent this BC's Invariant 1 cites as the
architectural analogue for a fuel-budget-free dispatcher-native check.

```
$ grep -A6 'name = "validate-burst-log"' plugins/vsdd-factory/hooks-registry.toml | grep -E "^tool =|^priority ="
tool = "^(Edit|Write|MultiEdit)$"
priority = 152
tool = "^Bash$"
priority = 152
```

```
$ grep -A4 'name = "convergence-tracker"' plugins/vsdd-factory/hooks-registry.toml | grep -E "priority|async"
priority = 210
```

```
$ grep -A4 'name = "regression-gate"' plugins/vsdd-factory/hooks-registry.toml | grep "priority"
priority = 230
```

Confirms the three Cohort B validator identities and priorities the Cross-Validator Minimum Rule
(Postcondition 5) is keyed against.

```
$ grep -oE "^\*\*CAP-04[123] " .factory/specs/domain-spec/capabilities.md
**CAP-041 
**CAP-042 
**CAP-043 
```

Confirms CAP-041/CAP-042/CAP-043 all exist as registered capabilities, grounding this BC's
capability anchor (CAP-043) and its distinction from CAP-041/CAP-042 cited in the Description.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-116, VP-117, VP-140 — allocated by formal-verifier (VP-116/VP-117: S-25.02 F2 verification-property extension burst, VP-INDEX v3.02; VP-140: S-25.02 F2 verification-property fix-burst PASS-3, VP-INDEX v3.05, F-P3-004). VP-116 (kani-proof; boundary inclusivity + cap-comparison arithmetic overflow-safety, `"flat"` byte-size shape), VP-117 (unit-test; unmatched-path zero-cost bypass, Cross-Validator Minimum Rule, byte denomination, `"flat"` byte-size shape). Cap-constant numeric bounds PROVISIONAL-until-F4 per ADR-051 §Decision 2. **VP-140 (unit-test; the dedicated Postcondition 8 item-count-trigger VP the prior forward reference called for, F-P3-004):** covers the shape-dispatch (read-once, no content-sniff), the `current_item_count + 1 > N` item-count trigger off-by-one boundary (`N-1` → Continue, exactly `N`/`N+1` → fire, EC-008), and the `low_water_mark` rotation-target config validation added in the pass-3 BC fix-burst — default `floor(N/2)` when omitted (EC-010), fail-loud `HookResult::Error` on `low_water_mark >= N` or negative with `N-1` valid (EC-011). **Also covers the EC-012 amortization-advisory facet added at VP-140 v1.1 (pass-4, F-P4-001, ADR-051 v1.4 Decision 14 Option (b)):** a non-fatal `tracing::warn!` advisory fires IFF `low_water_mark > floor(N/2)`, while config-load ALWAYS succeeds (never `HookResult::Error`) for any `0 <= low_water_mark < N`, including `N-1` (EC-012). VP-140 is the item-count-shape analogue of VP-116's role for the byte boundary; it carries NO F4-provisional numeric dependency (the item-count trigger is deliberately not byte-denominated).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies CAP-043's cap-formula-and-trigger boundary: "a native... dispatcher function computes, for every registered sharded artifact, a `shard_cap_bytes` ceiling from four calibrated inputs... and performs a `stat()`-only byte-size check... before every `Edit`/`Write`/`MultiEdit`." |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN — consistent with BC-1.18.004's own precedent for this class of dispatcher-mechanics contract) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` native PreToolUse check) |
| ADR | ADR-051 §Decision 1 (native gate placement + algorithm + v1.1 trigger-shape dispatch amendment), §Decision 2 (cap formula, calibration method, provisional constants); ADR-047 §Decision 8b (ratified future phase this BC elaborates); ADR-042 (measured fuel/byte linear model grounding `WORST_CASE_FUEL_PER_BYTE`'s provisional floor) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.17 | 2026-10-14 | product-owner | Application of ADR-052 v1.33 (NEW version; S-25.09 pass 5 default-deny source gates). (1) Postcondition 10 gains the sentence scoping the tracing-field half of the sanitization rule to the migration and shard surface per ADR-052 §Decision 11(g)(2)(f) "Scope of the tracing-field half", (2) (named files plus the provenance extension over the shard/migration error family; `executor.rs`, `indeterminate_marker.rs`, `invoke.rs`, `sinks/`, `vsdd_sink.rs` and all other files outside; non-shard diagnostics governed by BC-3.08.001's no-subscriber invariant). (2) NEW Postcondition 11: a `[[shard]]` `artifact_stem` is parsed at `shard-config.toml` decode by the `ArtifactStem` constructor (`is_safe_path_component`); a violation is `ShardConfigError::ArtifactStemUnsafe`, rendered per slot under Postcondition 10, fail-loud, no fallback stem (ADR-052 §Decision 11(c) point 7(e) row (11)(ii)). (3) NEW EC-024 and 4 Canonical Test Vectors. No existing Postcondition other than the one added sentence in 10, no Invariant, EC or vector changed. |
| 1.16 | 2026-10-13 | product-owner | Extension of the ADR-052 §Decision 11(g)(2) diagnostic-sanitization rule to this BC's `ShardConfigError` (S-25.09 test-writer black-box finding: a `[[shard]]` entry with no `shape` and a hostile `artifact_stem` reached the dispatcher `block_reason` of the shard-cap gate with a raw ESC; eleven variants render config-derived `{artifact_stem}`/`{artifact_path}` raw, and `Toml` carries the multi-line toml message that echoes the offending source line and can include a secret). New Postcondition 10: every config-derived string slot of every `ShardConfigError` Display is escaped (one line) and capped at 256, fixed text and numerics verbatim; `Toml` renders category plus line/column only; holds on the dispatcher `block_reason`, the `migrate-bc-index` stderr via `ShardCapConfigUnavailable`, and any log. New EC-023 (hostile ESC/LF stem through the gate, over-long stem, secret in a wrong-typed value, `migrate-bc-index` surface, benign control). Same-version extension (ADR-052 §Decision 11(g)(2)(f)): Postcondition 10 adds the 64-per-slot rule for InternalLog/tracing event fields with `Display` and event rendering from ONE exhaustive `render_with_cap` source (no wildcard arm), the `Toml` variant storing only a closed category token plus line and column (never the toml error object; the `#[from]` conversion becomes a constructor), and `ShardCapConfigUnavailable.detail` built from the sanitized rendering; EC-023 gains vectors (f)-(h). No formula, constant, trigger or verdict change; no H1 change. Frontmatter gains `last_amended`; `modified[]` records this version. |
| 1.15 | 2026-09-08 | product-owner | Citation refresh only (PR #824 cycle-3 MAJOR-3 gate-hoist, ADR-051 §Decision 17; architect adjudication, human-directed). **No postcondition/invariant/EC/CTV content change.** The architect's MAJOR-3 fix (feature/S-25.02-roll tip `6d3d96e7`) hoisted `shard_cap_precheck`'s INVOCATION from inside `crates/factory-dispatcher/src/executor.rs::execute_tiers` to `crates/factory-dispatcher/src/main.rs::main::run`, called before the `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return guard — closing the gap where `main::run` skipped `execute_tiers` (and therefore the gate) entirely whenever no plugin matched. `shard_cap_precheck` itself remains DEFINED in `executor.rs`; only its call site moved. UPDATED Precondition 1 (previously cited only "ADR-051 Decision 1 placement: before the registry-driven plugin loop," which described the now-superseded `execute_tiers` call site) to name `main::run` as the invocation point and cite ADR-051 §Decision 17 alongside §Decision 1. UPDATED the Architecture Anchors section: the `executor.rs` line now describes it as `shard_cap_precheck`'s DEFINITION site (and `execute_tiers`'s PRIOR invocation site); ADDED a new `main.rs` line naming `main::run` as the CURRENT PreToolUse dispatch path invocation point, citing ADR-051 §Decision 17. No code change, no test change, no story-body propagation required (frontmatter `capability`/`subsystem`/postcondition scope unchanged) — this is a pure spec-citation-accuracy correction following an already-adjudicated, already-shipped architecture change. `status` remains `active`. |
| 1.14 | 2026-09-07 | product-owner | Spec-anchor backfill for an already-shipped fail-open-gap fix (PR #818 cycle-4 fresh-context PR reviewer finding F-1, product-owner adjudication). **The gap:** `path_falls_under_or_equals` (`crates/factory-dispatcher/src/shard_manager.rs`) matches a tool-call target path against a `[[shard]]` entry's `artifact_path` by comparing the two paths' normalized component suffixes (after filtering `Component::CurDir`); when an entry's `artifact_path` is EMPTY or normalizes to empty after that filtering (`""`, `"."`, `"./"`), the comparison degenerates to `[] == []`, which is vacuously true against ANY target path — the entry silently mis-applies its shard-cap gate (and, for a `"frontmatter-changelog-array"`-shaped entry, its item-count gate) to every unrelated `Edit`/`Write`/`MultiEdit` dispatch in the repository, a FAIL-OPEN-BY-OVER-MATCH structural defect with no prior BC text describing it. The implementer already fixed this in the SAME PR #818 cycle-4 fix-burst, shipped at develop squash-commit `fff5e4cc`: a new `ShardConfigError::EmptyArtifactPath { artifact_stem, artifact_path }` variant, enforced in `validate_entry`, with 7 covering tests (`test_BC_1_18_005_F1_*`) landed in the same commit. ADDED a new "Empty-`artifact_path` structural defect guard" sub-paragraph to Postcondition 1 (immediately before its closing EC cross-reference sentence) specifying the guard: `validate_entry` MUST check, for the entry under validation and BEFORE any cap-formula arithmetic (Postcondition 9 and its EC-015/EC-017 divisor-door sub-paragraphs), that `artifact_path`'s component sequence — after filtering `Component::CurDir` — is non-empty, returning `Err(ShardConfigError::EmptyArtifactPath)` (surfaced as `HookResult::Error`) if it is, evaluated at ENTRY-MATCH time consistent with this Postcondition's own v1.12 MATCH-FIRST re-scoping (the matched entry only, never eagerly across the config). Updated Postcondition 1's closing EC cross-reference sentence and its "Terminology re-grounding" `validate_entry` checklist enumeration to cite EC-022, and swept the parallel enumeration in Invariant 3 to add EC-022 (sibling-site sweep, TD-VSDD-060). ADDED EC-022 (+ matching Canonical Test Vector) to the Edge Case Catalog and Canonical Test Vectors table. Added an adjudication note under §Verification Properties routing EC-022 VP allocation to architect/formal-verifier, DEFERRED TO F6 per the EC-013..EC-021 precedent (product-owner does not self-allocate a VP number). Added a routing note: this ruling requires NO further code or test change — the sanctioned implementation and its 7 `test_BC_1_18_005_F1_*` tests already ship the described behavior at `fff5e4cc`; this amendment is spec-catchup only, mirroring EC-020's v1.13 precedent for an already-shipped, already-compliant ruling. No change to Postconditions 2-9, other bullets of Postcondition 1, other Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP numeric content. |
| 1.13 | 2026-09-07 | product-owner | Self-deadlock adjudication for a present-and-fenced-but-malformed-YAML target artifact (fresh-context PR reviewer finding M-1, MAJOR, PR #818 cycle-2 re-review), plus a spec backfill for an already-shipped, related B1 ruling. **B1 backfill (present-but-fenceless, no code change — spec catch-up only):** the cycle-1 fix-burst already shipped `read_changelog_item_count` returning `Ok(0)` when a `"frontmatter-changelog-array"`-shaped target EXISTS but has no `---` fence at all (missing opening fence, or opening fence with no closing fence) — mirroring EC-014's `NotFound`-is-zero precedent — but this BC's text had not yet been updated to describe it; Postcondition 8's "any OTHER io::Error kind... is UNCHANGED" sentence (from v1.9) was, on its literal wording, in tension with this already-shipped behavior. ADDED a new "Present-but-fenceless case" sub-bullet to Postcondition 8 and EC-020 (+ two Canonical Test Vectors) documenting the shipped `Ok(0)` behavior; no code or test change follows from this half of the ruling. **M-1 adjudication (substantive, FAIL-LOUD ruling — the important one):** a target file that HAS a well-formed fence pair but whose YAML content inside fails to parse produces `io::ErrorKind::InvalidData`, surfaced as `HookResult::Error` for every `Edit`/`Write`/`MultiEdit` against that file — including the repair edit that would fix the YAML — a genuine self-deadlock the reviewer flagged as sitting in tension with the B1 permissive ruling landed in the same fix-burst. Two readings were weighed: (A) FAIL-LOUD (malformed YAML is a real defect distinct from "no frontmatter to count"; silently returning `Ok(0)` would undercount a possibly-large `changelog:` sequence, defeating this BC's fuel-exhaustion-prevention purpose exactly when the artifact's true state is least trustworthy) vs. (B) PERMISSIVE (extend B1's `Ok(0)` posture to this case too, to eliminate the deadlock). **ADJUDICATED (A) FAIL-LOUD, over (B) PERMISSIVE:** B1's fenceless case has NO YAML content to lose — `Ok(0)` there is the CORRECT count, not an approximation — whereas THIS case's file has actual (corrupted) YAML content whose true item count is unknown and possibly large; collapsing the two into one permissive rule would reintroduce the unbounded-growth-goes-undetected failure mode this BC exists to prevent (the item-count-shape analogue of Postcondition 9's silently-inflated-ceiling concern, now for a silently-deflated count). Per CLAUDE.md's production-grade default, the self-deadlock is a MESSAGING gap to close, not a reason to weaken the fail-loud posture (Standing Rule 3 §2's `Vec::new()`-hiding-partial-failure anti-pattern is the direct analogue for a corrupted-count-as-`0`). ADDED a "Malformed-YAML case" sub-bullet to Postcondition 8 imposing a MANDATORY, load-bearing requirement on the error message `read_changelog_item_count`'s YAML-parse `map_err` branch constructs: it MUST name the YAML-parse failure specifically AND state that the gate only intercepts `Edit`/`Write`/`MultiEdit`, directing the operator to repair the malformed frontmatter via any other means (e.g., a `Bash`-invoked edit) — an unchanged generic message is explicitly declared NON-COMPLIANT from v1.13 forward. Clarified this is NOT a TD-FACTORY-HOOK-BYPASS-001 exception (that policy governs the WASM hook-plugin chain; this native check has no bypass flag and none is introduced). ADDED EC-021 (+ a contrasting Canonical Test Vector alongside the two new EC-020 vectors). Corrected Postcondition 8's pre-existing v1.9 "any OTHER io::Error kind... (a genuinely malformed/unreadable frontmatter fence...)... is UNCHANGED" sentence to remove the now-superseded fenceless framing and instead cross-reference EC-020 (fenceless, permissive) and EC-021 (malformed-YAML, fail-loud) precisely. Added adjudication notes under §Verification Properties routing EC-020/EC-021 VP allocation to architect/formal-verifier, DEFERRED TO F6 per the EC-013..EC-019 precedent (product-owner does not self-allocate a VP number). Added a routing note: EC-020 (B1) requires NO code change (already shipped and compliant); EC-021 (M-1) requires an ACTUAL code change — routed to implementer (the error-message amendment) and test-writer (an EC-021 pinning test asserting both the preserved fail-loud outcome and the message's escape-hatch substring content). No change to Postconditions 1-7, 9, other bullets of Postcondition 8, Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP numeric content. |
| 1.12 | 2026-09-06 | product-owner | Blast-radius adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-6 finding F-C1-P6-001, LOW pending-intent). **The question:** `shard_cap_precheck` (`executor.rs`) calls `ShardRegistry::load()` — which parses AND semantically validates (fail-loud on EC-009/EC-011/EC-013/EC-015/EC-016/EC-017 for ANY entry) EVERY `[[shard]]` entry — on EVERY `Edit`/`Write`/`MultiEdit` dispatch whose config file exists, BEFORE `shard_cap_gate_check` matches the target path to a specific entry. A single malformed sibling entry therefore caused `HookResult::Error` for EVERY matched-tool dispatch in the repo, INCLUDING dispatches targeting paths that match NO entry at all or match a DIFFERENT, well-formed entry. Two readings were weighed: (A) EAGER FAIL-FAST (malformed config should block all matched-tool dispatches, ops-hygiene style — no code change, Postcondition 1 clarified to scope the zero-cost guarantee to well-formed configs) vs. (B) MATCH-FIRST (Postcondition 1's zero-cost-bypass promise holds per-artifact — an unmatched or differently-matched target returns `Continue`/proceeds normally even with a malformed sibling; only the MATCHED entry is validated — requires a code restructuring). **ADJUDICATED (B) MATCH-FIRST over (A):** Postcondition 1's own literal "no arithmetic" text for unmatched paths already leans B (eager validation performs arithmetic, and can outright block, for paths this Postcondition promises pay zero cost — a stronger violation than the O(1) config-presence probe the v1.9 scoping clarification exempted); the blast radius under (A) is disproportionate to the actual risk (a malformed entry for artifact Z threatens only Z's own protection, never an edit to `src/foo.rs` or to a different well-formed entry's artifact — halting ALL repository editing over one artifact's config typo inverts this BC's own fuel-exhaustion-containment purpose); and match-first does not weaken any existing fail-loud guarantee (a dispatch that DOES target the malformed entry's own artifact still fails loud, now at entry-match time instead of inside `load()`'s eager loop — the "before any write against the artifact" timing guarantee EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017 and Postcondition 8's N-presence bullet already establish is UNCHANGED, only the entry-SCOPE narrows from all-entries-in-the-file to the one matched entry). ADDED a "Blast-radius scoping" sub-paragraph to Postcondition 1 specifying the required restructuring: `ShardRegistry::load()` becomes structural-TOML-deserialization-only (unavoidably whole-file — `find_matching_entry` needs every entry's `artifact_stem`, so some parse per dispatch is unavoidable); a NEW `validate_entry(&ShardEntry) -> Result<(), ShardConfigError>` function carries the semantic checks (EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017's logic) previously inlined in `load()`'s per-entry loop; `shard_cap_precheck`/`shard_cap_gate_check` MUST call `find_matching_entry` FIRST and invoke `validate_entry` ONLY on the matched entry (zero validation calls when unmatched). Every existing downstream reference to "`ShardRegistry::load()` MUST... for EVERY entry... at load time" (Postcondition 8's N-presence bullet, Postcondition 9, EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017) is retroactively read, from v1.12 forward, as "for the entry matched against the current dispatch's target, at entry-match time" — an entry-scope narrowing, not a change to any cited check's substantive outcome or timing guarantee. Carved out an explicit, UNAVOIDABLE residual exception: a config file failing STRUCTURAL TOML deserialization (invalid syntax, or a missing non-`Option` field such as `artifact_stem`/`shard_cap_bytes`/a formula input) still fails `HookResult::Error` for every dispatch regardless of match, since `find_matching_entry` cannot run without a successfully deserialized registry — this is inherent to TOML's whole-file grammar, not a validation-eagerness choice this ruling controls. Extended Invariant 3 to state the config-match check must precede entry validation, and validation must be scoped to the matched entry only. ADDED EC-018 (the resolved blast-radius scenario: malformed sibling + unmatched-or-differently-matched target → `Continue`/normal gate logic, sibling never validated) and EC-019 (the residual structural-parse-failure exception: malformed TOML syntax or missing required field → `HookResult::Error` regardless of match). ADDED four Canonical Test Vectors (EC-018 pinning: unmatched target with malformed sibling → `Continue`; matched-to-the-malformed-entry target → unchanged `HookResult::Error`; matched-to-a-different-well-formed-entry target with malformed sibling → normal gate logic; EC-019 pinning: unmatched target with structurally-invalid config file → `HookResult::Error`). Added an adjudication note under §Verification Properties routing EC-018/EC-019 VP allocation to architect/formal-verifier, DEFERRED TO F6 per the EC-013..EC-017 precedent (product-owner does not self-allocate a VP number). Added a routing note flagging that — UNLIKE v1.7-v1.11's wording-only or additive-check amendments — this ruling requires an ACTUAL code restructuring, routed to implementer (the restructuring) and test-writer (the EC-018/EC-019 pinning tests plus a regression test preserving EC-009/EC-011/EC-012/EC-013/EC-015/EC-016/EC-017's fail-loud outcomes for a matched malformed entry). No change to Postconditions 2-8's substantive formulas/thresholds, other Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP's numeric content — additive/clarifying at the spec level, but NOT a no-code-change amendment (code restructuring is required, per Reading (B)). |
| 1.11 | 2026-09-06 | product-owner | Residual divisor-door closure + `replace_all` multiplicity adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5 finding F-C1-P5, two items). **Item 1 (substantive, FAIL-LOUD ruling — "the important one," divisor-door RESIDUAL):** EC-015's `worst_case_fuel_per_byte.is_finite() && > 0.0` guard is NECESSARY but INSUFFICIENT — a legal tiny-positive divisor (e.g. `1e-300`) passes that guard yet still drives `compute_shard_cap_bytes`'s internal division to a value so large its `.floor() as u64` cast saturates to `u64::MAX`, reproducing the IDENTICAL failure mode EC-015 exists to close (an oversized `shard_cap_bytes` sails past `CapExceedsFormulaCeiling` because the computed ceiling itself is meaninglessly large). Picking a larger arbitrary lower bound on the divisor would only move the threshold, never close it. ADJUDICATED: the robust close is to detect a SATURATED/non-finite COMPUTED CEILING directly, not to bound the divisor. ADDED a new "Residual divisor-door closure" sub-paragraph to Postcondition 9 requiring `ShardRegistry::load()` to independently evaluate the RAW pre-cast division result (`practical_fuel_ceiling as f64 / worst_case_fuel_per_byte`) for every entry that passes the EC-015 guard, and reject the entry (`HookResult::Error`, new `ShardConfigError::FormulaCeilingSaturated` variant — POLICY 1 append-only, distinct from `InvalidWorstCaseFuelPerByte`) if that raw value is non-finite OR `>= u64::MAX as f64`, evaluated BEFORE the `CapExceedsFormulaCeiling` comparison. ADDED EC-017 and a matching Canonical Test Vector (`worst_case_fuel_per_byte = 1e-300` → fail-loud). Together with EC-015's existing divisor-legality guard, this closes the divisor-door completely. VP allocation deferred to F6 per the EC-013..EC-016 precedent (adjudication note added under §Verification Properties). **Item 2 (DEFERRED, explicit anchor to BC-1.18.006):** `net_delta_bytes_for_edit` computes a SINGLE-occurrence `len(new) - len(old)` delta, matching Postcondition 3's formula exactly — but the `Edit` tool's `replace_all: true` mode applies the delta `occurrence_count × (len(new) - len(old))` times, which this formula does not account for, potentially under-counting `projected_size` for a bulk `replace_all` edit. ADJUDICATED DEFER (not fix-now) for this cluster: computing the true occurrence count requires reading the current file's full content to count matches of `old_string` — a departure from Postcondition 2's stat()-only guarantee for the `"flat"` shape, with real cost/consistency implications warranting cross-component assessment rather than a same-burst patch; AND this cluster's trigger is non-blocking today (`shard_cap_gate_check`'s `ShardShape::Flat` arm only `tracing::warn!`s and returns `Continue` on a fired trigger — BC-1.18.006's roll/block outcome does not exist yet), so an under-count has zero live user-facing consequence until BC-1.18.006 lands and trigger accuracy becomes load-bearing. ADDED a "Known formula gap — `replace_all: true` occurrence multiplicity, DEFERRED to BC-1.18.006" sub-paragraph to Postcondition 3 recording the gap, the non-blocking-today rationale, and an EXPLICIT deferral anchor: BC-1.18.006 MUST add a precondition/postcondition acknowledging this gap and MUST NOT treat its roll/block outcome as production-ready against a `replace_all: true` `Edit` until either this Postcondition is amended (with Postcondition 2 explicitly re-scoped) or BC-1.18.006's own roll-execution path (which must already read full content to perform a rotation) independently re-validates post-apply size. UPDATED the BC-1.18.006 Related-BCs bullet to cite this dependency. No code/test change follows from Item 2 in this burst — anchor only. No change to Postconditions 1-2, 4-8, Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP — additive/clarifying only. |
| 1.10 | 2026-09-06 | product-owner | Load-time fail-loud gap-closure + PC5 wiring-intent adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4 findings F-C1-P4-001 MEDIUM/HIGH + F-C1-P4-002 MEDIUM + F-C1-P4-003 LOW pending-intent). **F-C1-P4-001 (substantive, FAIL-LOUD ruling):** `compute_shard_cap_bytes` divides `practical_fuel_ceiling` by `worst_case_fuel_per_byte` with no validation that the latter is finite and strictly positive — a `[[shard]]` entry declaring `worst_case_fuel_per_byte = 0.0` makes the division yield `+inf`, whose saturating `as u64` cast is `u64::MAX`, so Postcondition 9's cap-vs-formula check (`shard_cap_bytes > computed_ceiling`) is defeated: ANY declared `shard_cap_bytes` passes, the oversized cap loads silently, and the roll never triggers — reintroducing the exact fuel-exhaustion failure mode this BC exists to prevent. ADJUDICATED FAIL-LOUD (this is squarely this cluster's own load-time-validation job, per this BC's own EC-009/EC-011/EC-013 precedent and CLAUDE.md's production-grade default): ADDED a new sub-bullet to Postcondition 9 requiring `ShardRegistry::load()` to validate `worst_case_fuel_per_byte.is_finite() && worst_case_fuel_per_byte > 0.0` for every entry BEFORE computing `compute_shard_cap_bytes`, fail-loud `HookResult::Error` (new `ShardConfigError` variant, e.g. `InvalidWorstCaseFuelPerByte`) on violation. Explicitly scoped OUT the other three formula inputs: `practical_fuel_ceiling`/`max_single_record_bytes`/`safety_margin` are `u64`-typed (TOML already excludes non-finite/negative values for them) and a degenerate `0` on any of the three drives the ceiling toward the SAFE direction (0), not the exploitable-widening direction — no analogous check needed for them. ADDED EC-015 (non-finite/`<=0.0` `worst_case_fuel_per_byte` → `HookResult::Error`) and two matching Canonical Test Vectors (the `0.0` divisor-door case and a `NaN` case). **F-C1-P4-002 (substantive, FAIL-LOUD ruling):** `ShardRegistry::load()` only validates a `"frontmatter-changelog-array"` entry's `low_water_mark` when BOTH `entry.n` and `entry.low_water_mark` are `Some` — the destructure short-circuits when `n` is `None`, so an entry that omits `n` AND declares an out-of-range `low_water_mark` is never validated at load time (EC-011 silently bypassed for that entry); the current implementation only fail-louds on missing `n` at gate/dispatch time (`shard_cap_gate_check`), not at load time, an asymmetry with EC-009/EC-011/EC-013's uniform load-time posture. ADJUDICATED YES — missing `n` MUST fail-loud AT LOAD: ADDED a new sub-bullet to Postcondition 8's rotation-target-config bullet requiring `ShardRegistry::load()` to validate `entry.n.is_some()` for every `"frontmatter-changelog-array"`-shaped entry, evaluated BEFORE `low_water_mark` is examined, fail-loud `HookResult::Error` (new `ShardConfigError` variant, e.g. `MissingN`) on `None` — this restructuring ensures no entry can silently load with BOTH a missing `N` and an out-of-range `low_water_mark` undetected. ADDED EC-016 and a matching Canonical Test Vector. **F-C1-P4-003 (wording-only, Reading (A) CONFIG-TIME/HARNESS-HELPER adjudicated over Reading (B) RUNTIME):** `effective_shard_cap_bytes` (implementing Postcondition 5's Cross-Validator Minimum Rule) is implemented and unit-tested but has no live caller in `shard_cap_gate_check`, which compares against the raw single `entry.shard_cap_bytes` instead. ADJUDICATED Reading (A): ADR-051 §Decision 2 places the Cross-Validator Minimum Rule inside the F4 calibration harness's own design (harness step 5), `ShardEntry` carries exactly one `shard_cap_bytes` field (no per-validator breakdown for a live gate to combine), and the existing Postcondition-5 prose/Canonical Test Vectors already describe an authoring-time MIN computation — REWORDED Postcondition 5 (additive "Timing clarified" paragraph) to state explicitly that the effective cap is DECLARED as `shard_cap_bytes`, computed via the MIN rule at config-authoring/F4-harness time, and consumed directly (not re-derived per-write) by the live gate; `effective_shard_cap_bytes` is correctly implemented as this authoring-time helper and needs no live caller wired into `shard_cap_gate_check` — NO code change follows from this ruling. Recorded an explicit deferral: a hypothetical future RUNTIME re-derivation of the Cross-Validator Minimum is a separate, out-of-scope obligation requiring a new postcondition and `architect`/ADR-051 involvement (a Cohort-B-reader-set-to-artifact mapping the current schema does not carry) — not assumed to already exist. VP allocation for EC-015/EC-016 deferred to F6 per the PC9/EC-014 precedent (adjudication note added under §Verification Properties). No change to Postconditions 1-4, 6-7, Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP — additive/clarifying only. |
| 1.9 | 2026-09-06 | product-owner | Spec/implementation asymmetry + wording-precision adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3 findings F-C1-P3-001 MEDIUM + F-C1-P3-003 OBSERVATION). **F-C1-P3-001 (substantive, GRACEFUL ruling):** the item-count shape's `read_changelog_item_count` did NOT special-case `io::ErrorKind::NotFound` the way the flat shape's `current_shard_bytes_flat` already does per EC-004 — a bare `read_to_string(target_path)?` propagated `NotFound` as an `io::Error` -> `HookResult::Error`, hard-blocking a legitimate FIRST-EVER `Write` that CREATES a `"frontmatter-changelog-array"` artifact (e.g. the first-ever `Write` of a new sharded index file). ADJUDICATED GRACEFUL over FAIL-LOUD: this is the identical "artifact absent on disk" precondition EC-004 already treats as size-0, not an error, for the sibling flat shape; there is no principled reason for the two trigger shapes to diverge on that precondition, and per CLAUDE.md's production-grade default a hard block on a legitimate create is a user-facing wrong behavior, not an acceptable, unexamined asymmetry. Postcondition 8's read-cost bullet was SILENT on the missing-file case (it characterized only cold/steady state of an EXISTING file) — this was the gap. ADDED a new missing-file/first-write sub-bullet to Postcondition 8 specifying `read_changelog_item_count` MUST map `NotFound -> Ok(0)` (any OTHER `io::Error` kind on an existing file is UNCHANGED, still fail-loud). ADDED EC-014 (missing target artifact -> `Ok(0)` -> `Continue` for any `N >= 1`, mirroring EC-004) and a matching Canonical Test Vector. Added an adjudication note routing VP-140 sixth-facet extension (or new VP) to architect/formal-verifier, per this BC's own Postcondition-9/EC-013 precedent for not self-allocating a VP number. **F-C1-P3-003 (cheap permanent close, OBSERVATION):** Postcondition 1 / Invariant 3 said the applicability check performs "no `stat()` call" / "zero measurable per-call overhead" for unmatched paths — but the implementation's applicability check does a small `Path::exists()` probe on a FIXED, small `[[shard]]` config file (not the target artifact) to decide whether the gate is even a candidate for the dispatch. REWORDED Postcondition 1 and Invariant 3 (minimally, additively) to scope the "no stat() / zero-cost" guarantee to the TARGET ARTIFACT specifically, explicitly exempting the one-time, O(1), fixed-size, fuel-free config-presence probe as the necessary applicability check — the load-bearing guarantee (no target-artifact stat, no fuel-budgeted content read on unmatched paths) is unchanged; only the literal wording's scope is clarified, preventing a future adversary pass from re-flagging the same literal-wording tension. No change to Postconditions 2-9, other Invariants, or any pre-existing Edge Case/Canonical Test Vector/VP — additive/clarifying only. |
| 1.8 | 2026-09-06 | product-owner | Spec/implementation wording-tension adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-2 finding F-C1-P2-003, LOW). Invariant 4 and EC-010 said `low_water_mark`'s `floor(N/2)` default is "computed at config-load time when the field is omitted" — but the sanctioned cluster-1 implementation (`crates/factory-dispatcher/src/shard_manager.rs`) leaves `low_water_mark = None` after `ShardRegistry::load()` when the field is omitted (`load()`'s own EC-010 branch only runs `validate_low_water_mark` when an EXPLICIT value is present) and derives `floor(N/2)` LAZILY in the pure function `resolved_low_water_mark(n, low_water_mark)`, called by the caller at first use/consumption (BC-1.18.009's rotation-target resolution). ADJUDICATED: the BINDING invariant this BC actually needs — grounded in ADR-051 §Decision 14's own framing ("This is a config DEFAULT, not a hardcoded Rust constant inside `shard_manager.rs` — consistent with BC-1.18.005 Invariant 4's 'formula inputs are configuration, not embedded constants' discipline") — is that the default is ALWAYS DERIVED FROM config `N`, NEVER a hardcoded fallback constant; ADR-051 never states derivation TIMING (load-time materialization vs. lazy first-use resolution) as a load-bearing constraint, and the cluster-1 lazy design has zero behavioral consequence in cluster 1 (nothing consumes `low_water_mark` until BC-1.18.009/cluster 4) and is a legitimate, pure, engineering choice (single source of truth is `N`; no derived-value staleness risk since config loads once per process; avoids mutating parsed config state). Per CLAUDE.md's production-grade default (over-specifying an implementation-timing detail that carries no safety value, then treating the resulting non-conformance as a defect, is itself the anti-pattern — the mechanical question "is timing load-bearing" is answerable now from ADR-051, not a TODO), REWORDED Invariant 4's `low_water_mark` clause and EC-010 to state the constraint as "derived from config `N`, never a hardcoded fallback constant; derivation MAY be materialized at config-load time OR deferred to first-use/resolution — timing is an implementation choice, not a contract obligation." No change to the numeric default (`floor(N/2)`), to the fail-loud validation constraint (`0 <= low_water_mark < N`, still validated at config-load time when an explicit value is declared — EC-011/EC-012 unaffected), to any Postcondition, or to any other Edge Case/Canonical Test Vector/VP. The sanctioned implementation was already conformant under the corrected wording; no code or test change is required. |
| 1.7 | 2026-09-06 | product-owner | BC-semantic intent adjudication (S-25.02 Phase F4 LOCAL adversary cluster-1 finding — PC4 load-enforcement-intent question). The cluster-1 implementation (`crates/factory-dispatcher/src/shard_manager.rs`) parses the four cap-formula inputs into `ShardEntry` and computes `compute_shard_cap_bytes()`, but `ShardRegistry::load()` never compared a declared `shard_cap_bytes` against its own entry's formula-derived ceiling — a misconfigured `[[shard]]` entry declaring `shard_cap_bytes` far ABOVE its own formula ceiling would load silently and the live per-write gate would honor the too-large cap, reintroducing the exact fuel-exhaustion failure this BC exists to prevent. ADJUDICATED Reading (B) RUNTIME-ENFORCED over Reading (A) HARNESS-CONSUMED-ONLY: the four inputs are parsed into the SAME `ShardEntry` the live per-write gate consumes (not a separate harness-only config surface), Postcondition 6 already states "the formula shape is locked now" (a live constraint, not merely a future-harness reference), and this BC's own Related BCs section identifies BC-1.18.005 as the mechanism that prevents BC-1.18.001's sibling Layer-1 fail-closed INDETERMINATE contract from firing — an unenforced cap-vs-formula inequality would defeat that stated purpose. Per CLAUDE.md's production-grade default (Rule 1 — no MVP-driven deferrals; a gate that lets an over-cap misconfiguration through silently is a defer-pattern smell), ADDED new Postcondition 9 (load-time fail-loud enforcement of `shard_cap_bytes <= compute_shard_cap_bytes(entry.cap_formula_inputs())` at `ShardRegistry::load()` time, for every `[[shard]]` entry regardless of `shape`, mirroring EC-009/EC-011's established fail-loud-at-load-time posture for this same config surface). ADDED EC-013 (cap exceeds formula ceiling → `HookResult::Error`). ADDED two Canonical Test Vectors (error case: `shard_cap_bytes=100,000` against a `49,152`-byte formula ceiling → `HookResult::Error`; boundary happy-path: `shard_cap_bytes` exactly equal to the formula ceiling → `Ok`, inclusive boundary mirroring EC-002). ADDED an adjudication note under §Verification Properties routing new-VP allocation for Postcondition 9/EC-013 to architect/formal-verifier (no VP number invented by product-owner, per this BC's own Postcondition-8-then-VP-140 precedent). No change to Postconditions 1-8, Invariants, or existing Edge Cases/Canonical Test Vectors — additive only. |
| 1.6 | 2026-09-06 | product-owner | Surgical parity tidy-up (adversary pass-11 LOW observation — §VP Anchors/§Verification Properties/VP-INDEX sibling parity): the §VP Anchors VP-140 bullet enumerated VP-140's facets (shape-dispatch, item-count off-by-one/EC-008, `low_water_mark` default/EC-010, fail-loud/EC-011) but OMITTED the EC-012 amortization-advisory facet that the v1.5 fix-burst already reconciled into the sibling §Verification Properties table row and that VP-INDEX's own VP-140 entry already carried — a §VP Anchors-only lag (the pass-7 fix swept the table row but not this bullet). APPENDED the EC-012 amortization-advisory facet (`tracing::warn!` fires IFF `low_water_mark > floor(N/2)`; config-load always succeeds for any `0 <= low_water_mark < N` incl. `N-1`) to the §VP Anchors VP-140 bullet, bringing it to parity with the table row and VP-INDEX at five facets. No postcondition, invariant, or contract-behavior change; reference-completeness only. |
| 1.5 | 2026-09-06 | product-owner | Surgical residual-sweep fix-burst (adversary pass-7 observation — VP-140 row completeness): the §Verification Properties VP-140 row description listed the shape-dispatch, item-count off-by-one (EC-008), and `low_water_mark` default/fail-loud (EC-010/EC-011) facets but OMITTED the EC-012 amortization-advisory facet VP-140 gained at v1.1 (pass-4, F-P4-001, ADR-051 v1.4 Decision 14 Option (b)) — a row-description completeness lag only; the VP↔BC linkage itself was already correct (VP-INDEX.md's own VP-140 row already carries all five facets). APPENDED the amortization-advisory biconditional facet (`tracing::warn!` fires IFF `low_water_mark > floor(N/2)`; config-load always succeeds for any `0 <= low_water_mark < N` incl. `N-1`, EC-012) to the VP-140 row's Property column and Proof Method column (four facets → five facets, matching VP-140 v1.1). No postcondition, invariant, or contract-behavior change; row-description reconciliation only. |
| 1.4 | 2026-09-06 | product-owner | Fix-burst amendment (adversary pass-4 finding F-P4-001 HIGH, ADR-051 v1.4 Decision 14 ADJUDICATION Option (b), per `.factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md` §4d): resolved the live, mutually-unsatisfiable contradiction between Postcondition 8/EC-011 (which folded `low_water_mark = N-1` into the fail-loud `>= N` bucket) and this BC's own Canonical Test Vectors table + VP-140 + VP-125 (which already stated `N-1` is a VALID, non-fail-loud boundary value). Postcondition 8's rotation-target-config bullet: STRUCK the parenthetical `"(including the degenerate N-1)"` from the fail-loud sentence — the fail-loud scope is now stated as EXACTLY `low_water_mark >= N` (the `== N` boundary included) or negative. ADDED a new sentence/bullet: any legal value in `(floor(N/2), N)`, up to and including `N-1`, loads normally (`Continue`, NEVER `HookResult::Error`) but config-load emits a NEW non-fatal `tracing::warn!` amortization advisory citing `(N, low_water_mark)` and the amortization factor `N - low_water_mark`, compared against the default's `N - floor(N/2)`. REWROTE EC-011: struck `"(including the degenerate low_water_mark = N-1)"` from its description (scope narrows to exactly `>= N`/negative; `HookResult::Error` outcome unchanged for that narrowed scope) and added a cross-reference to EC-012. ADDED NEW EC-012 (legal-but-poorly-amortizing `low_water_mark` in `(floor(N/2), N)` incl. `N-1` → loads normally + `tracing::warn!` amortization advisory, never `HookResult::Error`). Canonical Test Vectors: the existing `N=50`/`low_water_mark=49` parenthetical was already correct (no change); ADDED one new row for EC-012 (`N=50`, `low_water_mark=49` → loads, advisory fires with amortization factor 1). The numeric constraint `0 <= low_water_mark < N` itself is UNCHANGED — it was never wrong; only the erroneous "fail-loud on `N-1`" prose is withdrawn. No other BC-1.18.005 wording (Postconditions 1-7, other Edge Cases, VP-140/VP-125 rows, which were already correct) required amendment for this finding. |
| 1.3 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-3 finding F-P3-005 MEDIUM, ADR-051 v1.3 Decision 14): ADDED a `low_water_mark` rotation-target config field to Postcondition 8 (sibling to `N`, `"frontmatter-changelog-array"` shape only) — default `floor(N/2)` when omitted, fail-loud-validated `0 <= low_water_mark < N` (never silently clamped or defaulted around a violation); `N` remains the unchanged trigger threshold, this BC owns declaring/validating both `N` and `low_water_mark`, BC-1.18.009 owns what its rotation step does with `low_water_mark`. Extended Invariant 4 to cover the new field. Added EC-010 (omitted → default) and EC-011 (fail-loud on `>= N` or negative) plus two matching Canonical Test Vectors. Replaced the stale §VP Anchors note (which referenced an unauthored PC8 item-count VP) with a clean forward reference to formal-verifier's follow-on PC8 VP authorship. Collapsed the §Verification Properties table's three separate VP-117 rows into one multi-facet row (F-P3-006; no coverage change, presentation only). |
| 1.2 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-2 findings F-P2-002 HIGH + F-P2-007 MEDIUM, ADR-051 v1.2 Decisions 1/13): REWROTE Postcondition 3's `projected_size` formula from the WITHDRAWN uniform `current_shard_bytes + payload_bytes` (unsound for `Write` — double-counted a `Write`'s own already-complete content on top of the current shard's size) to the CORRECTED tool-discriminated formula: `Write` → `projected_size = len(content)` alone; `Edit`/`MultiEdit` → `projected_size = current_shard_bytes + net_delta_bytes` (unchanged — this leg was never wrong). Updated Canonical Test Vectors: corrected the two `Write` vectors, added a regression vector demonstrating the withdrawn formula's over-trigger bug on a same-size full-file `Write`, and added an explicit `Edit` vector to preserve coverage of the unchanged current+delta formula. Corrected Postcondition 8's read-cost claim into an explicit COLD-STATE (pre-BC-1.18.012 migration: ~1,997-item, non-N-relative-bounded, one-time read) vs. STEADY-STATE (post-migration: genuinely `<= N`-item-bounded) split — the prior text's unqualified "bounded" claim was true only in steady state. Added BC-1.18.012 to Related BCs (the new governed one-time B1 changelog backfill migration BC that makes the steady-state characterization true). |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-1 findings F-S2502-F2-005 + F-S2502-F2-003 + F-S2502-F2-007, ADR-051 v1.1 Decision 1 amendment): NEW Postcondition 8 + Invariant 5 + EC-008/EC-009 adding the item-count-denominated trigger for the `"frontmatter-changelog-array"` artifact shape (mechanism B1, BC-INDEX's `changelog:` array) — this BC now owns BOTH trigger shapes the native gate dispatches on, distinguished by a `[[shard]]` config `shape` field; does not replace or weaken the existing byte-size postconditions. VP table reconciled to VP-INDEX v3.02 authoritative methods: VP-116 unit-test→kani-proof, VP-117 byte-denomination row proptest→unit-test (no property content change). Added `## SDK Grounding Evidence` section with literal stable-anchor grep output for `HookResult`, `block_if_marker_check`, Cohort B validator registry entries, and CAP-041/042/043 existence. |
| 1.0 | 2026-09-05 | product-owner | Initial creation. F2 spec-evolution burst, S-25.02 activation. Byte-size-denominated shard-cap formula, native `stat()`-only deterministic size-trigger, Cross-Validator Minimum Rule, provisional-constants table (explicitly marked PROVISIONAL-until-F4-harness-calibration per ADR-051 Decision 2). CAP-043 capability anchor. ADR-051 §D1/§D2 + ADR-047 §D8b + ADR-042 citations. |
