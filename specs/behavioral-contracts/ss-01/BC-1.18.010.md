---
document_type: behavioral-contract
level: L3
version: "1.3"
status: draft
producer: product-owner
timestamp: 2026-09-05T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md
  - .factory/specs/architecture/ARCH-INDEX.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.009.md
  - .factory/specs/verification-properties/VP-INDEX.md
input-hash: "a5867e7"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
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

# BC-1.18.010: BC-INDEX Per-Subsystem Body-Table Sharding (Mechanism B2) with Zero-Lookup First-Level Addressing and Manifest-Based Second-Level Sub-Sharding

## Description

`BC-INDEX.md`'s body is already partitioned by the ten `### SS-NN` headings that exist today, each
a self-contained BC table for that subsystem — this BC does not invent that partition, it splits it
into individually-addressable files. Each `### SS-NN` section becomes its own file
`.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-NN.md`. First-level addressing (which
subsystem shard holds a given `BC-X.YY.NNN`) is a PURE FUNCTION of the BC ID's numeric prefix via
the already-authoritative BC-S-prefix→SS-NN mapping (ARCH-INDEX Subsystem Registry, POLICY 6) — no
index file read is required for the common single-BC-lookup case. Two subsystems (SS-05, SS-06)
already exceed the provisional cap on their own section size alone and require immediate
second-level sub-sharding, addressed via a per-subsystem manifest (a manifest read IS required at
that second level, since sub-shard boundaries are growth-based, not ID-prefix-deterministic). A NEW
sibling BC (BC-1.18.011, added in this fix-burst per F-S2502-F2-002/ADR-051 v1.1 Decision 10)
governs the ONE-TIME migration from today's monolithic body to this BC's end-state; this BC
specifies the end-state addressing scheme only, not the transition mechanics.

## Preconditions

1. BC-INDEX.md's live body contains exactly ten `### SS-NN` headings (`### SS-01` through
   `### SS-10`), each a self-contained per-subsystem BC table — the pre-existing partition this BC
   shards along (ADR-051 §Decision 7 confirms this partition "already exists in the live file").
2. ARCH-INDEX's Subsystem Registry `BC-S Prefix` column (`BC-1`→`SS-01`, ..., `BC-10`→`SS-10`) is
   the authoritative, already-documented mapping (POLICY 6) this BC's first-level addressing
   reuses without modification.
3. The dispatcher's native shard-cap gate (BC-1.18.005/BC-1.18.006) is extended with the
   "per-subsystem body table" artifact-shape case needed to trigger this BC's second-level
   sub-sharding when a subsystem shard itself exceeds cap.

## Postconditions

1. **First-level split: one file per existing `### SS-NN` section, leaving BC-INDEX.md's body as a
   lean top-level index.** Each of the ten sections is extracted verbatim into
   `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-NN.md`. `BC-INDEX.md`'s OWN body, after
   this split, retains only: `§Summary`, `§Subsystem Shard Manifest` (a new top-level section
   listing all ten shard files and their `sub_sharded` state), and any cross-cutting invariants
   that are not specific to one subsystem — no full per-BC tables remain in `BC-INDEX.md`'s body.
   The TRANSITION mechanics that produce this end-state from today's monolithic body (content-
   preservation, independent census, crash-atomicity, rollback) are BC-1.18.011's scope, not this
   BC's — this BC specifies the end-state shape only.

2. **First-level addressing requires NO index file read.** Any reader wanting `BC-X.YY.NNN`'s row
   computes its shard path directly: `BC-X` → the ARCH-INDEX `BC-S Prefix` mapping → `SS-NN` →
   `shards/BC-INDEX-SS-NN.md`. This computation is a pure function, requiring no read of
   `BC-INDEX.md` itself, no read of the shard manifest, and no read of ARCH-INDEX beyond the
   already-authoritative, rarely-changing Subsystem Registry table.

3. **Shard-manifest schema (needed for whole-corpus scans and second-level addressing):**
   ```toml
   # .factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml
   schema_version = 1

   [[subsystem_shard]]
   ss_id = "SS-01"
   bc_prefix = "BC-1"
   path = "shards/BC-INDEX-SS-01.md"
   sub_sharded = false

   [[subsystem_shard]]
   ss_id = "SS-05"
   bc_prefix = "BC-5"
   path = "shards/BC-INDEX-SS-05.md"          # becomes a stub pointer once sub_sharded=true
   sub_sharded = true
   sub_manifest = "shards/BC-INDEX-SS-05.manifest.toml"
   ```
   A whole-corpus reader (needing every BC across every subsystem) iterates this manifest's
   `[[subsystem_shard]]` entries rather than globbing or full-text-scanning `BC-INDEX.md`.

4. **Second-level sub-sharding, keyed by a per-subsystem manifest recording BC-ID-range
   boundaries, for any subsystem shard that itself exceeds `shard_cap_bytes`.** SS-05 (Pipeline
   Orchestration, 661 BCs, ~88,695 bytes measured 2026-09-05) and SS-06 (Skill Catalog, 592 BCs,
   ~85,407 bytes) both already exceed the provisional 48 KiB today-cap on their own section size
   and require immediate second-level sub-sharding at F4 activation (covered by BC-1.18.011's
   one-time migration operation — see Related BCs). A sub-shard boundary is growth-based (e.g.,
   `shards/BC-INDEX-SS-05.a.md` covering `BC-5.01.001`..`BC-5.30.099`), NOT ID-prefix-deterministic
   like the first level — so, unlike first-level addressing, a reader needing a specific
   `BC-5.YY.NNN` row MUST consult `shards/BC-INDEX-SS-05.manifest.toml` to determine which
   sub-shard (`.a`, `.b`, ...) holds that ID range. This second-level manifest read is the genuine,
   acknowledged asymmetry with mechanism A's near-zero-cost addressing (ADR-051 Consequences
   §Negative item 2).
   The SAME size-check gate that triggers first-level splitting also triggers second-level
   sub-sharding for every subsystem — SS-05/SS-06 are not hardcoded as the only subsystems that can
   ever need a second level; the other eight subsystems (SS-07 measured at ~39,072 bytes,
   comfortably under cap) are re-verified empirically once actual post-split per-subsystem file
   sizes are known, not assumed permanently exempt.

5. **The migration surface is a bounded, enumerable set of touchpoints — not open-ended — and this
   BC's postconditions name them explicitly:** (a) product-owner's BC authorship/amendment
   workflow: write target becomes the per-subsystem shard file, not `BC-INDEX.md`'s body; (b)
   state-manager's POLICY 7/8 title-sync and count-propagation bursts: `§Summary` count
   aggregation MUST sum across shard files' actual row counts rather than scanning one file
   in-place, and `validate-count-propagation.sh`'s `_extract_counts` needs a companion pass across
   the shard set; (c) consistency-validator's cross-reference checks: any check that currently
   globs or full-text-scans `BC-INDEX.md` for a specific ID MUST instead consult the shard manifest
   or glob `shards/BC-INDEX-SS-*.md`. The adversarial-review skill's POLICY auto-load
   (`.factory/policies.yaml`) is verified NOT to require migration — it is a small, independent
   file, not a scan of `BC-INDEX.md`.

6. **POLICY 7's title-authority invariant is preserved unchanged; only the row's file path
   changes.** BC-INDEX's H1-per-BC-row remains the authoritative title source per POLICY 7 — this
   BC changes WHERE that authoritative row physically lives (from `BC-INDEX.md`'s body to
   `shards/BC-INDEX-SS-NN.md`), not WHICH file's row is authoritative relative to the BC's own H1.
   No amendment to POLICY 7's text in `.factory/policies.yaml` is required by this BC (a
   metadata/addressing change, not a semantic-authority change).

## Invariants

1. **First-level addressing never requires a shard-manifest read.** Any implementation of this
   BC's first-level lookup that reads `BC-INDEX.shard-manifest.toml` before computing a
   single-BC's shard path (when that BC's subsystem is NOT sub-sharded) is a defect relative to
   this BC's own "zero-lookup" postcondition — the manifest is needed ONLY for whole-corpus scans
   and second-level (sub-sharded subsystem) lookups.
2. **The BC-S-prefix→SS-NN mapping used for first-level addressing is validated against
   ARCH-INDEX's Subsystem Registry at CI time and at migration activation time, never
   independently hardcoded.** At runtime, `shard_manager.rs` reads the mapping from the config
   entry only (a snapshot embedding the ARCH-INDEX commit SHA it was generated from, per
   ADR-052 §Decision 10). Two parity checks enforce that the snapshot never diverges from
   ARCH-INDEX: (a) a CI test (`arch_index_parity`) diffs the snapshot against HEAD ARCH-INDEX
   on every commit; (b) at migration activation, the binary verifies the live ARCH-INDEX matches
   the `approved_arch_index_sha` in the armed-activation manifest and fails CLOSED on mismatch.
   A future ARCH-INDEX subsystem renumbering propagates to this BC's addressing logic by
   regenerating the config snapshot from the new ARCH-INDEX commit, triggering a CI parity
   failure and requiring an explicit config update with the new SHA. This satisfies the 'never
   independently hardcoded' constraint: the snapshot is CI-validated, not independent. The CI
   parity check is NECESSARY-BUT-NOT-SUFFICIENT for live correctness; the activation-time check
   against `approved_arch_index_sha` is the sufficient condition, closing the gap between
   "CI-validated at commit N" and "live at activation time."
3. **No BC row is ever present in both `BC-INDEX.md`'s body AND a `shards/BC-INDEX-SS-NN.md` file
   simultaneously.** After the first-level split (Postcondition 1), `BC-INDEX.md`'s body contains
   zero per-BC table rows; every row lives in exactly one shard file (or, post-second-level-split,
   exactly one sub-shard file). BC-1.18.011 governs the one-time transition that establishes this
   state; this invariant is the STEADY-STATE property BC-1.18.011's own migration-integrity checks
   verify at transition time.
4. **Second-level sub-sharding is triggered by the identical size-check mechanism as first-level
   splitting — no separate, differently-calibrated trigger exists for sub-shards.**

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | A reader looks up `BC-5.39.006` (SS-05, currently sub-sharded) | First checks the top-level shard-manifest to discover SS-05's `sub_sharded=true` and `sub_manifest` path, then consults `shards/BC-INDEX-SS-05.manifest.toml` to find which `.a`/`.b`/... sub-shard's BC-ID range contains `BC-5.39.006` — TWO manifest reads for a sub-sharded subsystem, versus ZERO for a non-sub-sharded one |
| EC-002 | A reader looks up `BC-1.18.005` (SS-01, not sub-sharded) | Computes `shards/BC-INDEX-SS-01.md` directly from `BC-1` → `SS-01` via the ARCH-INDEX mapping — zero manifest reads |
| EC-003 | An eighth subsystem (currently comfortably under cap) grows over time and eventually exceeds `shard_cap_bytes` on its own section size | The SAME size-check gate that triggers SS-05/SS-06's sub-sharding fires for that subsystem too — sub-sharding is not hardcoded to SS-05/SS-06 specifically (Postcondition 4) |
| EC-004 | A whole-corpus consistency-validator scan needs every BC across all ten (sub-)shards | Iterates the top-level shard-manifest's ten `[[subsystem_shard]]` entries; for any entry with `sub_sharded=true`, additionally iterates that subsystem's own sub-manifest |
| EC-005 | A BC is renumbered from one subsystem's prefix range to another's (a genuinely rare event — POLICY 1 append-only numbering generally forbids renumbering) | Out of this BC's normal-path scope; if it occurs, it is a manual cross-shard-file move, not an automated consequence of any size-trigger this BC defines |
| EC-006 | ARCH-INDEX's Subsystem Registry itself is being amended in the same burst that a BC-INDEX shard lookup occurs | This BC's addressing reads ARCH-INDEX's CURRENT committed state; a mid-burst race between an ARCH-INDEX amendment and a BC-INDEX shard lookup is bounded by the same TD-VSDD-053 single-commit-per-burst and factory-lock discipline (ADR-025) that bounds all other concurrent `.factory/` mutation races in this project |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| Lookup `BC-1.18.005` | Path `shards/BC-INDEX-SS-01.md`, computed with zero manifest reads | happy-path |
| Lookup `BC-5.39.006` (SS-05, sub-sharded) | Top-level manifest → `sub_sharded=true` + `sub_manifest="shards/BC-INDEX-SS-05.manifest.toml"` → sub-manifest lookup → correct sub-shard file (e.g. `shards/BC-INDEX-SS-05.b.md`) | edge-case |
| Whole-corpus scan for every `BC-7.*` row | Iterate top-level manifest, find `ss_id="SS-07"`, `sub_sharded=false` → read `shards/BC-INDEX-SS-07.md` directly (no sub-manifest) | happy-path |
| `SS-05` section grows large enough that even sub-shard `.b` itself would exceed cap | Third-level splitting is NOT specified by this BC — flagged as an out-of-scope future extension if it ever occurs (no current measured subsystem approaches this) | error |
| BC-INDEX.md body after first-level split | Contains only `§Summary`, `§Subsystem Shard Manifest`, cross-cutting invariants — zero per-BC table rows | happy-path |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-127 | Zero-lookup invariant — first-level shard-path computation for a non-sub-sharded subsystem never reads the shard-manifest file | unit test (mock filesystem read-call counter, assert zero manifest reads for SS-01/02/03/04/07/08/09/10 lookups) |
| VP-128 | Single-authoritative-row invariant — after the split, no BC ID's row appears in both `BC-INDEX.md`'s body and a shard file | integration test (consistency-validator scan: post-split full-corpus scan asserting exactly one row per BC ID across all shard files, zero in `BC-INDEX.md`'s body) |
| VP-128 | Mapping-source-of-truth invariant — the BC-S-prefix→SS-NN mapping used by this BC's addressing logic is byte-identical to ARCH-INDEX's own Subsystem Registry `BC-S Prefix` column | integration test (cross-reference `shard_manager.rs`'s mapping table against a parsed ARCH-INDEX Subsystem Registry) |

**Fix-burst note (F-S2502-F2-003):** the single-authoritative-row row's Proof Method previously
read "consistency-validator scan" without a leading category keyword; normalized to "integration
test (consistency-validator scan...)" to match VP-INDEX v3.02's authoritative VP-128 = integration
assignment, consistent with the sibling VP-128 row above. No property content changed.

## Related BCs

- BC-1.18.006 — this BC's size-check trigger for second-level sub-sharding reuses the SAME native gate (depends on)
- BC-1.18.009 — sibling mechanism B1 (changelog rotation) targets BC-INDEX's OTHER size driver; both triggered by the same gate (composes with)
- BC-1.18.005 — the cap formula this BC's sub-sharding trigger reuses unmodified (depends on)
- BC-1.18.011 — governs the ONE-TIME migration from today's monolithic BC-INDEX body to this BC's end-state addressing scheme; this BC specifies the end-state, BC-1.18.011 specifies the transition (depended on by)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — the "per-subsystem body table" artifact-shape handler for B2's second-level sub-sharding trigger
- `.factory/specs/architecture/ARCH-INDEX.md` §Subsystem Registry — the authoritative `BC-S Prefix`→`SS-NN` mapping this BC's first-level addressing reuses (POLICY 6)

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^\| SS-01 Hook Dispatcher Core \| BC-1 \| [0-9]+ \| ss-01/ \|" .factory/specs/behavioral-contracts/BC-INDEX.md | sed -E 's/\| [0-9]+ \|/| <N> |/'
| SS-01 Hook Dispatcher Core | BC-1 | <N> | ss-01/ |
```

**CORRECTED (fix-burst pass-2, F-P2-006, MEDIUM) — structural-form re-grounding, count column
redacted.** The prior grep (v1.1) pasted the LITERAL count digit (`133`) into this BC's grounding
evidence — a volatile value that had already drifted to `134` by the time of this fix-burst (and
drifts to `135` again within this SAME burst, when BC-1.18.012 is added below), silently
invalidating the citation on every subsequent SS-01 BC addition. Per POLICY 5 v1.3.6's
HEAD-reproducibility mandate, this BC's grounding now asserts ONLY the STRUCTURAL claim this BC's
Postcondition 1 actually depends on — that the `BC-S Prefix`→`SS-NN`→count→shard-directory ROW
SHAPE exists in `BC-INDEX.md`'s §Summary table (the `[0-9]+` match-and-redact above proves the
count FIELD is present and numeric, without pinning its volatile value). This BC's postconditions
never depend on the SPECIFIC count — only on the mapping/row structure being present and
well-formed — so this closes the drift CLASS, not just this instance: **any future reader
verifying this claim MUST re-execute the grep above at HEAD to obtain the CURRENT count**, rather
than trusting a pasted literal that decays on the very next SS-01 BC addition.

```
$ grep -oE "^pub enum HookResult" crates/hook-sdk/src/result.rs
pub enum HookResult
```

Confirms the shared SDK contract this BC's second-level sub-sharding trigger (reusing the same
native gate as BC-1.18.006/009) is bound by.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-127, VP-128 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-127 (unit-test; zero-lookup first-level addressing), VP-128 (integration; manifest-keyed second-level + single-authoritative-row integrity + ARCH-INDEX-sourced prefix mapping).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies mechanism B2 exactly as CAP-043 describes it: "B2 splits the file's ten already-existing `### SS-NN` per-subsystem body sections into individually-addressable shard files, keyed by the already-authoritative BC-S-prefix→SS-NN mapping... for zero-lookup first-level addressing." |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` B2 artifact-shape handler) |
| ADR | ADR-051 §Decision 7 (B2 per-subsystem sharding design); §Decision 8 (shard-manifest schema, reader/writer migration surface); §Decision 10 (governed one-time migration, BC-1.18.011); §Rationale ("Why the per-subsystem BC-INDEX partition is not a new invention") |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.3 | 2026-09-12 | product-owner | ADR-052 v1.1 hardening: replaced Invariant 2 with the full config-snapshot-with-revision-binding specification. The prior text ("READ from ARCH-INDEX's Subsystem Registry, never independently hardcoded or duplicated") only stated what NOT to do; the replacement specifies the complete positive mechanism: the mapping is embedded as a `subsystem_prefixes` TOML snapshot in the `[[shard]]` config entry with a mandatory `arch_index_sha` binding (ADR-052 §Decision 10); two parity checks enforce the snapshot never diverges from ARCH-INDEX — (a) CI test `arch_index_parity` diffs snapshot vs. HEAD ARCH-INDEX on every commit (necessary-but-not-sufficient), and (b) at migration activation the binary verifies the live ARCH-INDEX matches `approved_arch_index_sha` in the armed-activation manifest and fails CLOSED on mismatch (the sufficient condition); a future subsystem renumbering propagates by regenerating the config snapshot and updating the SHA, triggering CI parity failure first; this satisfies the 'never independently hardcoded' constraint because the snapshot is CI-validated, not independent. ADR-052 added to inputs. |
| 1.2 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-2 finding F-P2-006, MEDIUM, POLICY 5 v1.3.6 HEAD-reproducibility mandate): re-grounded the `## SDK Grounding Evidence` §Summary-row grep to a STRUCTURAL-FORM assertion (the `BC-S Prefix`→`SS-NN`→count→shard-directory row shape, with the volatile count field redacted to `<N>` via a `sed` pass) instead of pasting a literal count digit that had already drifted from `133` (v1.1's citation) to `134` (live at authoring time) and drifts again to `135` within this SAME burst (BC-1.18.012's addition below) — closes the drift CLASS, not just this instance; future readers re-execute the grep at HEAD for the current count. No postcondition/invariant/VP content change. |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-002 + F-S2502-F2-003 + F-S2502-F2-007): Description/Postcondition 1/Invariant 3 amended to cross-reference the new BC-1.18.011 (governed one-time B2 migration BC) — this BC now explicitly states it specifies the END-STATE addressing scheme only, deferring transition mechanics (content-preservation, census, atomicity, rollback) to BC-1.18.011. Added a Related BCs row and an ADR Traceability citation for ADR-051 §Decision 10. VP-128's single-authoritative-row row Proof Method normalized from bare "consistency-validator scan" to "integration test (consistency-validator scan...)" per VP-INDEX v3.02's authoritative method assignment — no property content change. Added `## SDK Grounding Evidence` section. |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, not in the original F1 enumeration — mechanism B2, added per D-1166 human widest-scope decision). Per-subsystem body-table sharding with zero-lookup first-level addressing (BC-S-prefix→SS-NN, reusing ARCH-INDEX's authoritative mapping) and manifest-based second-level sub-sharding for SS-05/SS-06 (both already over cap on section size alone). Enumerated the bounded reader/writer migration surface. CAP-043 capability anchor. ADR-051 §D7/§D8 citations. |
