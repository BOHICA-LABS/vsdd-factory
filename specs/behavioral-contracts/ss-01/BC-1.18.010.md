---
document_type: behavioral-contract
level: L3
version: "1.10"
status: active
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
input-hash: "1684ab3"
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
   one-time migration operation — see Related BCs). **Sub-shard boundaries are computed by the
   deterministic chunk-boundary algorithm specified in ADR-051 §Decision 18: canonical-BC-ID-sorted
   (via `extract_and_sort_bc_rows`), greedy-pack-until-cap, single left-to-right pass over the
   subsystem's rows — using the SAME `shard_cap_bytes` value that triggers first-level splitting
   (per Invariant 4 below), never a separately-calibrated boundary rule.** A fixed preamble (the
   rewritten `### SS-NN` section heading plus the markdown table header and separator rows) is
   replicated verbatim into EVERY sub-shard file — never packed with only the first — and its byte
   cost is counted against `shard_cap_bytes` as that sub-shard's starting `current_bytes` (an empty
   sub-shard is never "free"). Given a fixed row set, preamble, and `shard_cap_bytes`, the resulting
   chunk boundaries are a pure, deterministic function of those three inputs — identical input
   bytes in identical order always produce identical boundaries, on any invocation, any machine, any
   retry (ADR-051 §Decision 18 item 5). Chunk-boundary determinism and correctness are verified by
   **VP-142** (proptest; hosted on BC-1.18.011 Postcondition 6, cross-referenced here because the
   property must hold identically for the one-time migration and the steady-state rebuild path). A
   sub-shard boundary is growth-based, NOT ID-prefix-deterministic like the first level — so, unlike
   first-level addressing, a reader needing a specific `BC-5.YY.NNN` row MUST consult
   `shards/BC-INDEX-SS-05.manifest.toml` to determine which sub-shard (`.a`, `.b`, ...) holds that ID
   range. This second-level manifest read is the genuine, acknowledged asymmetry with mechanism A's
   near-zero-cost addressing (ADR-051 Consequences §Negative item 2).
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
   ARCH-INDEX's Subsystem Registry at CI time and at migration activation time via an explicit
   three-way parity check, never independently hardcoded.** At runtime, `shard_manager.rs` reads
   the mapping from the config entry only (a snapshot embedding the `arch_index_sha` of the
   ARCH-INDEX commit it was generated from, per ADR-052 §Decision 10). Three parity checks
   enforce that the snapshot never diverges from ARCH-INDEX:
   (a) CI test (`arch_index_parity`) diffs the snapshot against HEAD ARCH-INDEX on every commit;
   (b) at migration activation (under exclusion), the binary performs a three-way check:
       `config.arch_index_sha` == `manifest.approved_arch_index_sha` == live ARCH-INDEX SHA;
       fails CLOSED if any pair diverges — including a stale binary (config at revision A)
       invoked with a current manifest (revision B);
   (c) stale-installed-config CI test verifies the binary's embedded snapshot `arch_index_sha`
       matches the expected revision used in the test fixture.
   A future ARCH-INDEX subsystem renumbering requires regenerating the config snapshot from the
   new ARCH-INDEX commit and triggering a CI parity failure as the forcing function.
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

## Reader Integration

During the B2 migration window (after CURRENT.json pointer swap, before completed.json
written), readers accessing BC-INDEX paths MUST use the following protocol:
1. Check `.factory/migration-state/completed.json` — if exists: canonical paths are current.
2. Check `.factory/migration-state/CURRENT.json` — if `status: committing`: for each required
   file, open `gen-<generation_id>/<file>`; on ENOENT, open the canonical path. Rationale (C-1,
   ADR-052 §Decision 7c): the canonical path holds STALE old content for in-place-overwrite
   targets (e.g. BC-INDEX.md) until step 7's specific rename for that file — OPEN-based with
   ENOENT fallback ensures new content is always returned for BOTH net-new shard files AND
   in-place-overwrite targets. A file present at `gen-<uuid>/` has not yet been moved (new
   content); ENOENT on `gen-<uuid>/<file>` means the file has already been renamed to canonical
   (new content at canonical) — open the canonical path instead. `rename(2)` atomicity makes
   this protocol race-free: the canonical path is guaranteed to hold new content the moment
   ENOENT is observable on the gen-path (the rename completed before ENOENT was visible).
3. If neither exists: legacy BC-INDEX.md path is current (migration not started).
In steady state (completed.json present), canonical paths are always authoritative. The
'COMMITTED absent → read legacy BC-INDEX.md' heuristic from v1.2 is eliminated: completed.json
is permanent and its presence is unambiguous in all states including after cleanup.

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
- VP-142 (cross-reference; hosted on BC-1.18.011 Postcondition 6, not owned by this BC) — proptest; chunk-boundary determinism and correctness for the ADR-051 §Decision 18 `chunk_subsystem_rows_into_sub_shards` algorithm this BC's Postcondition 4 cites. Cited here because sub-shard boundary correctness is a property of the shared chunking algorithm that must hold identically for BC-1.18.011's one-time migration and the future steady-state rebuild path (ADR-051 §Decision 18 item 7) — not a property owned or independently verified by this BC's own VP-127/VP-128.

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
| 1.10 | 2026-09-22 | product-owner | ADR-051 §Decision 18 addendum encoding (spec-closure chain step 2 of 2: architect → product-owner; human-approved 2026-09-22 design proposal). Postcondition 4 amended: replaced the worked-example-only "growth-based (e.g., ... covering `BC-5.01.001`..`BC-5.30.099`)" boundary text with the actual deterministic chunk-boundary rule — sub-shard boundaries are computed by ADR-051 §Decision 18's `chunk_subsystem_rows_into_sub_shards` algorithm (canonical-BC-ID-sorted via `extract_and_sort_bc_rows`, greedy-pack-until-cap, single left-to-right pass), reusing the SAME `shard_cap_bytes` value as first-level splitting per Invariant 4 (no separately-calibrated migration-time cap), with a fixed preamble replicated verbatim into every sub-shard and counted against cap as that sub-shard's starting size. Determinism (same row set + preamble + cap always yields identical boundaries) and chunk-boundary correctness now cited to **VP-142** (proptest; hosted on BC-1.18.011 Postcondition 6, cross-referenced here since the property holds identically for migration-time and the steady-state rebuild path). VP Anchors section updated with the VP-142 cross-reference. No change to Postcondition 4's SS-05/SS-06 measured-size facts or its migration-coverage claim (still covered by BC-1.18.011). input-hash recompute owed to state-manager. |
| 1.9 | 2026-09-13 | product-owner | ADR-052 v1.11 pass-8 reader-protocol mirror (MED-1). §Reader Integration step 2: replaced existence-check-then-read form ("try gen path FIRST; if absent, fall back to the canonical path") with the OPEN-based with ENOENT fallback form per the canonical reader protocol (ADR-052 §Decision 7c): open `gen-<generation_id>/<file>`; on ENOENT, open the canonical path. Updated "ENOENT is impossible" rationale to reference the open-with-fallback implementation: ENOENT on the gen-path signals the file was already renamed to canonical; `rename(2)` atomicity makes the protocol race-free (the canonical path is guaranteed to hold new content the moment ENOENT is observable on the gen-path). "generation-first ensures new content is always returned" rationale rephrased to "OPEN-based with ENOENT fallback ensures new content is always returned." input-hash recompute owed to state-manager. |
| 1.8 | 2026-09-13 | product-owner | ADR-052 v1.7 re-hardening (F3 casing sweep). Corrected all path-bearing COMPLETED.json occurrences in §Reader Integration to lowercase completed.json per ADR-052 §Decision 7c step 8: (1) §Reader Integration preamble "before COMPLETED.json written" → "before completed.json written"; (2) steady-state prose "In steady state (COMPLETED.json present)" → "In steady state (completed.json present)"; (3) "COMPLETED.json is permanent and its presence is unambiguous" → "completed.json is permanent and its presence is unambiguous". Step 1's `completed.json` check (line 220) was already lowercase — confirmed correct and unchanged. |
| 1.7 | 2026-09-13 | product-owner | ADR-052 v1.5 re-hardening (C-1 mirror). §Reader Integration step 2 protocol inverted to generation-first/canonical-fallback: for each required file, try `gen-<generation_id>/` path FIRST; if absent, fall back to canonical path. Rationale (C-1, ADR-052 §Decision 7c): the canonical path holds STALE old content for in-place-overwrite targets (e.g. BC-INDEX.md itself) until step 7's specific rename for that file — canonical-first (the v1.6 protocol) returns stale content for BC-INDEX.md before its rename; generation-first is correct for BOTH net-new shard files AND in-place-overwrite targets. A file present at `gen-<uuid>/` is not yet moved (new content); a file absent from `gen-<uuid>/` has already been renamed to canonical (new content at canonical); `rename(2)` atomicity ensures ENOENT is impossible for any new-generation file during the COMMITTING window. |
| 1.6 | 2026-09-13 | product-owner | ADR-052 v1.4 re-hardening (§Reader Integration step 2 C1 fix). Step 2 changed from "use `gen-<generation_id>/` paths for reads" to canonical-first / generation-fallback: for each required file, try canonical path first; fall back to `gen-<generation_id>/` equivalent if canonical absent. Rationale (C1): step 7 of the publication sequence (ADR-052 §Decision 7c) moves files from `gen-<uuid>/` to canonical paths one by one; `gen-<uuid>/` paths become unavailable for already-moved files; canonical-first fallback ensures readers never see ENOENT for any new-generation file during the committing window, regardless of step 7 progress, because `rename(2)` is atomic — a file is at canonical OR at `gen-<uuid>/`, never at neither. |
| 1.5 | 2026-09-13 | product-owner | ADR-052 v1.3 re-hardening. §Reader Integration section replaced: the v1.2 COMMITTED-absent heuristic ("COMMITTED present → canonical; COMMITTED absent → legacy; COMMITTED archived at CLEANED") is replaced with the v1.3 three-step protocol keyed on two permanent pointer files — (1) check `completed.json` first (presence means canonical paths current); (2) check `CURRENT.json` with `status: committing` (use generation staging paths); (3) if neither exists: legacy BC-INDEX.md path is current (migration not started). COMMITTED/CLEANED terminology eliminated throughout. COMPLETED.json is permanent and never archived; its absence in steady state is not an error. Closes F1/F9 alignment gap: BC-1.18.010's reader protocol now matches ADR-052 v1.3 §Decision 7c's atomic-pointer model. |
| 1.4 | 2026-09-13 | product-owner | ADR-052 v1.2 hardening (2 amendments): (1) Invariant 2 amended from two-parity to three-way parity: replaced "Two parity checks" with "Three parity checks" plus explicit three-way formula `config.arch_index_sha` == `manifest.approved_arch_index_sha` == live ARCH-INDEX SHA; added stale-binary detection description ("including a stale binary (config at revision A) invoked with a current manifest (revision B)"); added (c) stale-installed-config CI test; removed "NECESSARY-BUT-NOT-SUFFICIENT / sufficient condition" framing in favour of the new three-way check structure; `arch_index_sha` embedding field now cited as `arch_index_sha` (not generic "the ARCH-INDEX commit SHA"); updated closing sentence to "triggering a CI parity failure as the forcing function." (2) New §Reader Integration section added (2nd Codex F2 — not in v1.1): specifies that during the B2 migration window (after first target rename, before CLEANED), readers MUST consult the COMMITTED marker at `.factory/migration-state/migrate-bc-index-state.json` to determine read path — COMMITTED present: read from canonical shard paths; COMMITTED absent: read from legacy BC-INDEX.md; in steady state (after CLEANED), shard paths are always canonical; COMMITTED archived to `.factory/migration-audit/` at CLEANED time, its absence in steady state is not an error. |
| 1.3 | 2026-09-12 | product-owner | ADR-052 v1.1 hardening: replaced Invariant 2 with the full config-snapshot-with-revision-binding specification. The prior text ("READ from ARCH-INDEX's Subsystem Registry, never independently hardcoded or duplicated") only stated what NOT to do; the replacement specifies the complete positive mechanism: the mapping is embedded as a `subsystem_prefixes` TOML snapshot in the `[[shard]]` config entry with a mandatory `arch_index_sha` binding (ADR-052 §Decision 10); two parity checks enforce the snapshot never diverges from ARCH-INDEX — (a) CI test `arch_index_parity` diffs snapshot vs. HEAD ARCH-INDEX on every commit (necessary-but-not-sufficient), and (b) at migration activation the binary verifies the live ARCH-INDEX matches `approved_arch_index_sha` in the armed-activation manifest and fails CLOSED on mismatch (the sufficient condition); a future subsystem renumbering propagates by regenerating the config snapshot and updating the SHA, triggering CI parity failure first; this satisfies the 'never independently hardcoded' constraint because the snapshot is CI-validated, not independent. ADR-052 added to inputs. |
| 1.2 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-2 finding F-P2-006, MEDIUM, POLICY 5 v1.3.6 HEAD-reproducibility mandate): re-grounded the `## SDK Grounding Evidence` §Summary-row grep to a STRUCTURAL-FORM assertion (the `BC-S Prefix`→`SS-NN`→count→shard-directory row shape, with the volatile count field redacted to `<N>` via a `sed` pass) instead of pasting a literal count digit that had already drifted from `133` (v1.1's citation) to `134` (live at authoring time) and drifts again to `135` within this SAME burst (BC-1.18.012's addition below) — closes the drift CLASS, not just this instance; future readers re-execute the grep at HEAD for the current count. No postcondition/invariant/VP content change. |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-002 + F-S2502-F2-003 + F-S2502-F2-007): Description/Postcondition 1/Invariant 3 amended to cross-reference the new BC-1.18.011 (governed one-time B2 migration BC) — this BC now explicitly states it specifies the END-STATE addressing scheme only, deferring transition mechanics (content-preservation, census, atomicity, rollback) to BC-1.18.011. Added a Related BCs row and an ADR Traceability citation for ADR-051 §Decision 10. VP-128's single-authoritative-row row Proof Method normalized from bare "consistency-validator scan" to "integration test (consistency-validator scan...)" per VP-INDEX v3.02's authoritative method assignment — no property content change. Added `## SDK Grounding Evidence` section. |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, not in the original F1 enumeration — mechanism B2, added per D-1166 human widest-scope decision). Per-subsystem body-table sharding with zero-lookup first-level addressing (BC-S-prefix→SS-NN, reusing ARCH-INDEX's authoritative mapping) and manifest-based second-level sub-sharding for SS-05/SS-06 (both already over cap on section size alone). Enumerated the bounded reader/writer migration surface. CAP-043 capability anchor. ADR-051 §D7/§D8 citations. |
