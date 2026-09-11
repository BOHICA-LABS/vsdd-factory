---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 9
verdict: NOT CLEAN — 2 findings (2 MEDIUM), both taxonomy doc-drift; CODE independently re-verified spec-conformant
finding_count: 2
finding_breakdown: "2 MEDIUM (F-C3-P9-001, E-SHD-011 form (b) taxonomy cell ≠ shipped MissingBackfillManifest Display; F-C3-P9-002, E-SHD-003 taxonomy stale + undocumented Io emission) — both doc-only, no code defect"
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

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-9

> Fresh-context adversarial review (LOCAL Claude adversary — human directed drive-to-3-CLEAN using
> LOCAL adversary only, no further cross-vendor rotation unless the human specifies) of
> `feature/S-25.02-backfill` @ `8e2a37f4` against BC-1.18.008 (mechanism-A one-time backfill-split)
> v1.8 and BC-1.18.007 (retention/compaction) v1.2, run immediately after pass-8's fix-burst
> (D-1198) landed on the branch.

## Verdict

**NOT CLEAN — 2 findings (2 MEDIUM), both doc-only.** Reviewing `feature/S-25.02-backfill` @
`8e2a37f4` against `shard_manager.rs`, `bc_1_18_008_backfill_split_test.rs`, BC-1.18.008.md v1.8, and
`prd-supplements/error-taxonomy.md` v1.12, the adversary independently re-verified every
`E-SHD-NNN` Display emitted by `ShardRollError`, `MechanismABackfillError`, and `ShardRetentionError`
in `crates/factory-dispatcher/src/shard_manager.rs` against this table's Message Format cells and
found the CODE itself spec-conformant end to end — zero code/behavior/BC defects this pass — but 2
further instances of the cluster's own recurring "taxonomy Message Format cell ≠ shipped `Display`"
drift class, both confined to `error-taxonomy.md`. BC-5.39.001 cluster-3 LOCAL streak: **0/3 → 0/3**
(cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track).

## Part A — Findings

### F-C3-P9-001 (MEDIUM) — `E-SHD-011` form (b) taxonomy cell ≠ shipped `MissingBackfillManifest` `Display`

**Location:** `prd-supplements/error-taxonomy.md` `E-SHD-011` row, form (b); shipped
`MechanismABackfillError::MissingBackfillManifest` `Display` impl
(`crates/factory-dispatcher/src/shard_manager.rs`).

**Finding:** pass-8 (F-C3-P8-001) completed `E-SHD-011`'s row to document its second real emission
(form (b), the missing-manifest precondition-failure case), but the wording it recorded diverges from
the shipped `Display` text in three places: it reads "backfill recovery cannot proceed for" where the
shipped text reads "backfill recovery-confirmation ambiguous for"; it omits the shipped
"([backfill_manifest]) to compare the canonical file's current bytes against" clause entirely; and it
reads "refusing to guess whether the canonical file's current state is SAFE, DANGEROUS, or AMBIGUOUS
without it;" where the shipped text reads the bare "refusing to guess;". This is the same
"documented-at-creation-time before the code existed, never re-diffed once implemented" shape as the
cluster's prior taxonomy-drift findings — form (b) was authored as a taxonomy-first contract
(pass-8's own report noted "not yet implemented... this taxonomy row defines the contract those
functions must implement") and the implementation that landed did not reproduce the row's prose
verbatim.

**Impact:** an operator or downstream tooling consulting the taxonomy row for `E-SHD-011` form (b) to
recognize or parse the emitted error text would fail to match against the actual shipped string.

**Disposition:** FIXED, DOC-ONLY (no code change). product-owner corrected form (b)'s Message Format
cell to the shipped text verbatim. `prd-supplements/error-taxonomy.md` v1.12→v1.13. No BC/AC/EC/VP
change — pure taxonomy-accuracy correction; `MissingBackfillManifest`'s shipped `Display` was already
correct.

### F-C3-P9-002 (MEDIUM) — `E-SHD-003` taxonomy cell stale + undocumented second (`Io`) real emission

**Location:** `prd-supplements/error-taxonomy.md` `E-SHD-003` row; shipped
`MechanismABackfillError::ContentPreservationFailed` and `MechanismABackfillError::Io` `Display` impls
(`crates/factory-dispatcher/src/shard_manager.rs`).

**Finding:** the `E-SHD-003` row documented only `ContentPreservationFailed`'s emission, and in a form
that omits the shipped `E-SHD-003:` code prefix and uses bare `for <artifact>:` instead of the shipped
`for artifact_stem "<artifact>":` quoting convention this table uses elsewhere. `Io` — a genuine I/O
failure while staging the split's shard files and index during Postcondition 5's stage-then-verify-
then-atomically-replace sequence, emitted under the SAME `E-SHD-003` code — has a real, reachable
emission in the shipped code with no taxonomy row documenting it at all.

**Impact:** identical in kind to F-C3-P9-001 — a taxonomy consumer would neither recognize the correct
code-prefixed/quoted form of `ContentPreservationFailed`'s message nor recognize `Io`'s emission under
this code at all.

**Disposition:** FIXED, DOC-ONLY (no code change). product-owner corrected the row to document BOTH
real emissions verbatim as forms (a) (`ContentPreservationFailed`) and (b) (`Io`), matching the
two-form documentation convention this table already uses for `E-SHD-002`/`E-SHD-011`.
`prd-supplements/error-taxonomy.md` v1.12→v1.13 (same burst as F-C3-P9-001). No BC/AC/EC/VP change.

## Companion sweep (product-owner, beyond this pass's 2 named findings)

Per the human's direction to stop fixing this drift class one code at a time, product-owner's fix
burst performed a complete mechanical re-diff of every `#[error("E-SHD-...")]` `Display` in
`shard_manager.rs` — covering `ShardRollError`, `MechanismABackfillError`, AND, for the first time
this cluster, the previously-unswept `ShardRetentionError` — against this table's Message Format
column, 13 codes / 17 real emissions total. This surfaced a THIRD drift instance beyond this pass's
own 2 findings: **`E-SHD-002`** (`ShardRetentionError`, two real emissions) documented only
`IndexUnavailable`, in a form missing the `E-SHD-002:` prefix, using bare `for <artifact>:` quoting,
and mislabeling the `#[source] io::Error` field `<parse-error>`; `ArchivalMoveFailed`'s real emission
was entirely undocumented. Corrected to document both forms verbatim in the same burst. Every other
`E-SHD-NNN` row (`001`, `004`/`005` not-yet-implemented anywhere in the workspace, `006`-`010`,
`012`, `013`) was re-verified MATCH, no change — this is the first attestation covering all three
error enums (`ShardRollError`, `MechanismABackfillError`, `ShardRetentionError`) in one pass; the
prior v1.8 changelog's "all nine rows source-verified" attestation covered only `ShardRollError`.
`error-taxonomy.md` v1.12→v1.13 carries F-C3-P9-001, F-C3-P9-002, and this companion `E-SHD-002` fix
as one changelog entry.

## Observations (non-blocking)

- **[process-gap] Recurring drift class, 6th+ occurrence, no mechanical gate exists.** The taxonomy
  Message-Format-cell-≠-shipped-Display drift class has now recurred across `E-SHD-001` (cluster-2
  pass-10), `E-SHD-006`/`E-SHD-008`/`E-SHD-009` (cluster-2 pass-8/9), and — within cluster-3 alone —
  `E-SHD-011`/`E-SHD-012` (pass-8) and now `E-SHD-002`/`E-SHD-003`/`E-SHD-011` form (b) (this pass):
  6 or more distinct occurrences of the identical root shape. Root cause: no gate mechanically
  re-diffs the FULL emitted-`Display` set against this table on every change to either side; every
  prior fix reconciled only the one code the triggering finding named, letting a different code drift
  by the next pass. Routed to state-manager for codification (see lessons.md L-entry and follow-up
  story, this burst's bookkeeping).
- **Code independently re-verified spec-conformant.** Zero CODE/behavior/BC defects this pass — the
  shipped `Display` impls were correct in every one of the 13 codes / 17 emissions checked; only this
  taxonomy's own Message Format cells were stale. BC-1.18.008 v1.8 already governs the semantics for
  the `MechanismABackfillError` codes and needs no amendment.

## Disposition Summary

Both findings FIXED same-burst, doc-only. F-C3-P9-001 (MEDIUM): `E-SHD-011` form (b) corrected to the
shipped `MissingBackfillManifest` text verbatim. F-C3-P9-002 (MEDIUM): `E-SHD-003` corrected to
document both `ContentPreservationFailed` and `Io` real emissions verbatim. Companion sweep also
corrected `E-SHD-002` (both `ShardRetentionError` emissions) in the same burst, beyond this pass's 2
named findings. `prd-supplements/error-taxonomy.md` v1.12→v1.13. No BC/AC/EC/VP change — CODE was
already correct in all cases; `feature/S-25.02-backfill` stays UNCHANGED at `8e2a37f4`. Because
findings were present, pass-9 is NOT CLEAN — BC-5.39.001 cluster-3 LOCAL streak stays **0/3**.

## Code Gate (this burst)

No code change this burst (doc-only fix). Feature branch `feature/S-25.02-backfill` remains at
`8e2a37f4` (pass-8's already-verified state): full `cargo test --workspace --all-targets` suite
green; `cargo fmt --check --all` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean
(re-confirmed current at `8e2a37f4`, no new commit required).

## Next

**NEXT = pass-10 — the VSDD 10-pass guardrail.** Per the project's convergence protocol, the
orchestrator assesses cluster-3's 3-CLEAN convergence status with the human at this guardrail rather
than automatically dispatching a fresh pass-10 adversary review. 9 passes have now run without
reaching 3 consecutive CLEAN; streak remains **0/3**.
