---
document_type: local-adversary-review
story_id: S-25.02
cluster: cluster-3 (mechanism-A backfill, BC-1.18.007 + BC-1.18.008)
pass: 6
verdict: NOT CLEAN — 3 findings (2 HIGH, 1 MEDIUM), CROSS-VENDOR (OpenAI Codex), novelty HIGH
finding_count: 3
finding_breakdown: "2 HIGH (F-C3-P6-001 data-loss, F-C3-P6-002 spec-fidelity/missing disk-read-back); 1 MEDIUM (F-C3-P6-003 h3 word-boundary spec-fidelity)"
streak: 0/3
adversary_model: openai-codex (CROSS-VENDOR — first non-Claude pass on this BC)
review_date: 2026-09-10
diff_base: "26c79f13"
diff_head: "26c79f13"
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.007.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.008.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
input-hash: "62eb09e"
---

# S-25.02 F4 Cluster-3 — LOCAL Adversary Pass-6 (CROSS-VENDOR: OpenAI Codex)

> Fresh-context adversarial review of `feature/S-25.02-backfill` @ `26c79f13` against BC-1.18.008
> (mechanism-A one-time backfill-split) v1.5 and BC-1.18.007 (retention/compaction) v1.2, run after
> pass-5's fix-burst (D-1195) landed on the branch. **This is the FIRST CROSS-VENDOR pass run against
> this BC's cascade** — the adversary role was fulfilled by OpenAI Codex rather than the Claude-family
> model used for passes 1-5, per the human-authorized full grind-to-literal-3-CONSECUTIVE-CLEAN drive
> (D-1195) explicitly bringing cross-vendor review into the mix. Rendered here in VSDD Part-A format
> from the raw Codex review JSON
> (`scratchpad/codex-pass6/codex-review.json`, `summary`/`novelty`/`reviewed`/`findings[]` fields),
> matching pass-1..5's own standalone-artifact convention.

## Verdict

**NOT CLEAN — 3 findings (2 HIGH, 1 MEDIUM), novelty HIGH.** Reviewing `.worktrees/S-25.02-backfill`
at commit `26c79f1373eb0a47b71ae72a92c2a7a28c11d59a` against `shard_manager.rs`,
`bc_1_18_008_backfill_split_test.rs`, `last-amended-migrate/atomic_write.rs`, BC-1.18.007.md,
BC-1.18.008.md, and `policies.yaml` (read-only; Rust tests were not executed by the reviewing model),
OpenAI Codex found **two HIGH findings and one MEDIUM finding — all THREE novel: none were raised, in
this shape, by any of the five prior same-vendor (Claude) adversary passes.** Most significantly,
**F-C3-P6-001 is a genuine data-loss bug in the recovery/idempotency path that pass-1's own equivalent
concern (the recovery-heuristic shape) was examined and NOT flagged as exploitable across passes 2 and
3**, and F-C3-P6-002 identifies that the PC6(c)/Invariant 4 "actual bytes written to disk" language —
present in the spec since pass-3 (D-1193) and read by 3 subsequent same-vendor passes (4, 5, and this
cascade's own now-superseded confidence) — was never actually implemented as a **disk** read-back; the
production gate checks only in-memory `partition.bytes.len()`. BC-5.39.001 cluster-3 LOCAL streak:
**0/3 → 0/3** (cycle-level BC-5.39.001 3/3 CONVERGED streak UNCHANGED, separate track). All 3 findings
are FIXED this burst (see Disposition Summary).

## Part A — Findings

### F-C3-P6-001 (HIGH) — Recovery byte-prefix heuristic destroys data on repeated-prefix content

**Location:** `crates/factory-dispatcher/src/shard_manager.rs:5602` (recovery/idempotency arm of the
mechanism-A backfill entry point, historically named `heal_or_confirm_already_migrated` /
`mechanism_a_backfill_already_migrated` in this cascade's own narrative).

**Finding:** recovery classifies an already-published shard-index re-invocation as "interrupted, needs
resuming" using ONLY a structural byte-prefix comparison —
`canonical_bytes.len() > sealed_concat.len() && canonical_bytes[..sealed_concat.len()] ==
sealed_concat[..]` — then unconditionally overwrites the canonical file with
`canonical_bytes[sealed_concat.len()..]`. Codex constructed a concrete counterexample: a
`session-checkpoints.md`-shaped input `A + A + "more\n"`, where `A = "## Checkpoint\nx\n"` and
`cap = 16`. The first successful migration seals `A` as shard 1 and correctly leaves `A + "more\n"` as
the legitimate, intact, oversized final record (BC-1.18.008 explicitly permits oversized single
records, EC-002). A **second** invocation reads the canonical file, sees its bytes begin with the
already-sealed `A` (the SAME bytes as shard 1, because the second checkpoint record is byte-identical
to the first), concludes "interrupted, resume by stripping the sealed prefix," and **overwrites the
canonical file with only `"more\n"` — silently destroying the second checkpoint record's own heading
and body**, which were never actually sealed and are not recoverable from the shard files. BC-1.18.008
PC2 permits any well-formed h2 checkpoint heading; it does not require headings to be unique, so
repeated-prefix content of exactly this shape is a legitimate, reachable input. The existing
idempotency test suite (`concat_records(&[1, 2, 3, 4, 5])`) only ever constructs records with distinct
synthetic bodies, so this exact-repeat collision was never exercised. **This is the class of bug pass-1
through pass-5's own narrative repeatedly asserted was "structurally sound" for the recovery arm
(O-C3-P3-001, observation, "not reachable today... deferred to T-12") without independently
constructing a repeated-prefix counterexample — the cross-vendor pass is the first to falsify that
assessment with a concrete, reachable, data-destroying input.**

**Impact:** HIGH — silent, unrecoverable data loss on a legitimate, non-adversarial input shape
(duplicate checkpoint content is a realistic occurrence, not a crafted attack). No production caller
is wired yet (F-C3-P1-006/T-12 deferral, still in effect), so not reachable in TODAY's production
traffic, but this is the exact defect class BC-1.18.008's own fail-loud philosophy (Postcondition 6,
"fail loud rather than silently mis-split or silently destroy data") exists to prevent, and it would
fire the moment T-12 wires a production caller against real, non-synthetic content.

**Disposition:** FIXED — product-owner amended **BC-1.18.008 v1.5→v1.6**: added a **Backfill Recovery
Manifest** (`[backfill_manifest]` table in the shard-index, recording `original_bytes`/
`original_sha256` and `final_bytes`/`final_sha256` — the pre-split original and intended-final-partition
whole-file identity, written durably at split time) to Postcondition 3; added a **Recovery-Confirmation
Rule** to Postcondition 5 replacing the byte-prefix heuristic outright: a re-invocation's disposition
(SAFE no-op / DANGEROUS complete-the-interrupted-truncate / AMBIGUOUS fail loud) is determined SOLELY
by exact whole-file `(length, SHA-256)` comparison of the on-disk canonical file against the Manifest's
two recorded pairs — never by re-concatenating sealed shards and comparing prefixes; rewrote Invariant 3
to require this exact-manifest basis and explicitly forbid the byte-prefix heuristic; added EC-009
(legitimate repeated-prefix content, e.g. this exact counterexample, correctly classified SAFE) and
EC-010 (on-disk state matching neither manifest pair fails loud, new error `E-SHD-011`, rather than
guessing). implementer rebuilt `heal_or_confirm_already_migrated` (renamed to reflect the new contract)
around whole-file `(length, SHA-256)` identity against the Manifest instead of byte-prefix comparison;
product-owner added `E-SHD-011` to `error-taxonomy.md` v1.9→v1.10. test-writer replaced the pre-v1.6
self-heal test that asserted the now-forbidden byte-prefix behavior (obsolete, retired) and added a
genuine disk-corruption-race fault-injection test plus a repeated-prefix (this exact counterexample)
regression test. Feature branch `feature/S-25.02-backfill`, culminating at HEAD `b1134954`.

### F-C3-P6-002 (HIGH) — PC6(c)/Invariant 4 gate checks in-memory bytes, never reads back the disk

**Location:** `crates/factory-dispatcher/src/shard_manager.rs:5097` (the per-shard-cap verification
gate, `mechanism_a_verify_backfill_per_shard_cap_preserved`, added at pass-3/D-1193 to close
F-C3-P3-001) and the sealed-file write loop / index-publication sequence immediately following it in
`run_mechanism_a_backfill_split`.

**Finding:** the cap gate checks only `partition.oversized_record || partition.bytes.len() as u64 <=
shard_cap_bytes` — a pure in-memory length check against the `Vec<u8>` the partitioner produced, BEFORE
any bytes are written to disk. All three invocations of this gate in
`run_mechanism_a_backfill_split` precede the sealed-file write loop. That loop calls
`write_atomic_bytes(&sealed_path, &partition.bytes, ...)` for each shard, derives
`bytes_at_seal` from `partition.bytes.len()` (the SAME in-memory value already gated, not a
post-write measurement), then publishes the shard-index and rewrites the canonical file — **without
ever reading any sealed shard file back off disk to confirm what was actually written matches what was
intended.** BC-1.18.008 PC6(c) is explicit: "checked explicitly against **the actual bytes written to
each sealed shard file**"; Invariant 4 repeats "post-hoc, against **the actual bytes written to
disk**." Both clauses' plain-language claim — verification against disk state — is not implemented;
the gate verifies only the in-memory candidate before any write occurs, which cannot detect a
truncated, corrupted, or partially-written sealed shard file (e.g. an interrupted `write_atomic_bytes`,
a filesystem full mid-write, or any other write-time corruption between the in-memory check and the
durable on-disk result). The negative cap test
(`tests/bc_1_18_008_backfill_split_test.rs:2537`) only feeds a pre-constructed oversized `Vec<u8>` to
the pure helper function directly — it never exercises the production write-then-verify path at all,
so it cannot and does not catch this gap.

**Impact:** HIGH — a spec-fidelity gap between BC-1.18.008's own explicit "actual bytes written to
disk" language (present since pass-3/D-1193, re-read and NOT flagged by passes 4 and 5) and the shipped
implementation, which never performs the disk read-back the spec names as the mechanism. This is
precisely the defect class Postcondition 6(c)/Invariant 4 were written to close (silent shard-cap
violation surviving to disk) — the gate as shipped provides no additional assurance beyond the
in-memory partitioner already having been correct, which defeats the purpose of a POST-write hard
gate.

**Disposition:** FIXED — product-owner ruled (adjudicating this finding) that PC6(c)/Invariant 4's
"actual bytes written to disk" REQUIRES a genuine post-hoc disk read-back after each shard write, not
an in-memory-only length check — no BC text amendment needed beyond this ruling, since the existing
v1.5 language already specified disk-verification; the code was non-compliant with already-correct
spec text. implementer extracted a new function,
`mechanism_a_write_and_verify_sealed_shard`, that performs the write via `write_atomic_bytes`, THEN
opens and reads the resulting file back off disk (`std::fs::read` against the sealed path, not the
in-memory buffer), and verifies the actual on-disk byte length against `shard_cap_bytes` (or the
`oversized_record`/`is_preamble_shard` exception) before the shard is considered sealed and before
index publication proceeds — a disk-corruption or truncation between write and read-back now aborts
the migration fail-loud instead of silently publishing a corrupted index. test-writer added a genuine
disk-corruption-race fault-injection test (writes a sealed shard, corrupts/truncates it on disk between
write and the verification read-back via a controlled test seam, asserts fail-loud abort — not merely
a pure-function-level oversized-`Vec` unit test). Feature branch `feature/S-25.02-backfill`, HEAD
`b1134954`.

### F-C3-P6-003 (MEDIUM) — Lesson h3 marker heading predicate missing word boundary

**Location:** `crates/factory-dispatcher/src/shard_manager.rs:4710`
(`is_id_tagged_lesson_heading`, delegated to by `is_lesson_record_heading` at line 4741).

**Finding:** `is_id_tagged_lesson_heading` accepts the numeric ID suffix using only
`after_dash.starts_with(|c: char| c.is_ascii_digit())` — it checks that the character immediately
after the tag's trailing dash IS a digit, but does not require the digit run to be followed by a word
boundary. BC-1.18.008 PC2 specifies the h3 lesson-heading exception as the regex
`^### L-<tag>-[0-9]+\b` — an explicit `\b` word-boundary anchor after the digit run. Consequently
`### L-EDP1-050details` and `### L-EDP1-050_extra` are BOTH incorrectly accepted as record-boundary
starts by the shipped predicate (the digit-run `050` is present and immediately follows a dash; the
predicate never checks what comes after the digits), when the spec's `\b`-anchored regex would reject
both — `050details` has no boundary before `details` (word character abuts word character) and
`050_extra` likewise (`_` is a word character in most regex `\b` semantics). A nested sub-heading of
this shape inside an existing lesson entry can therefore be mis-detected as an independent
record-boundary, splitting a single lesson across two shards. Re-deriving the record-boundary oracle
inside the migration itself does not catch this, because the oracle uses the exact same detector
function. The existing negative lesson-marker fixture
(`tests/bc_1_18_008_backfill_split_test.rs:2659`) covers malformed `## LESSON(...)` parenthetical
forms, not these h3 digit-suffix boundary cases — a different, unrelated negative-fixture family.

**Impact:** MEDIUM — this is the SAME general defect shape as F-C3-P3-003 (pass-3, over-matching
marker-table sibling headings) and F-C3-P1-002 (pass-1, a different marker-heading predicate bug) —
the mechanism-A marker-detection predicate family has now had 3 distinct word-boundary/heading-match
precision bugs across 6 passes. Not immediately reachable (no production caller yet, per T-12), but a
genuine spec-vs-code divergence that would incorrectly fragment a lesson entry the moment real
`lessons.md` content containing a numerically-suffixed sub-heading (a realistic authoring pattern — see
this very file's own `L-BB-D1194`/`L-BB-D1195` naming convention, which is exactly `L-<tag>-<digits>`
with no suffix today, but a future author appending descriptive suffix text to an h3 sub-heading is not
implausible) is migrated.

**Disposition:** FIXED — implementer tightened `is_id_tagged_lesson_heading` to consume the COMPLETE
digit sequence via `after_dash.find(|c: char| !c.is_ascii_digit())` (or equivalent full-run consumption)
and then require the documented word boundary immediately afterward (end-of-string, or a non-word
character), rejecting both `050details` and `050_extra` while continuing to accept the documented
`^### L-<tag>-[0-9]+\b` shape (e.g. `### L-BB-D1195-hook-bypass-...` — a dash immediately after the
digit run is itself a non-word character, satisfying `\b`, and correctly continues to match). The
separately-specified h2 prefix semantics (`is_lesson_h2_record_heading`, tightened at pass-3/D-1193)
are untouched. test-writer added negative alphabetic-suffix and underscore-suffix fixtures
(`050details`, `050_extra`) plus an end-to-end packing test proving such headings remain correctly
nested inside their enclosing lesson's shard rather than becoming a spurious record boundary. Feature
branch `feature/S-25.02-backfill`, HEAD `b1134954`.

## Disposition Summary

| ID | Severity | Disposition | Landing |
|----|----------|--------------|---------|
| F-C3-P6-001 | HIGH | FIXED — Backfill Recovery Manifest (BC-1.18.008 v1.6 PC3) + Recovery-Confirmation Rule (PC5, exact whole-file length+SHA-256 vs. byte-prefix heuristic) + Invariant 3 rewrite + EC-009/EC-010 + `E-SHD-011` fail-loud; `heal_or_confirm_already_migrated` rebuilt around manifest identity; obsolete byte-prefix test retired, repeated-prefix + disk-corruption-race regression tests added | `feature/S-25.02-backfill` @ `b1134954` |
| F-C3-P6-002 | HIGH | FIXED — product-owner ruled PC6(c)/Invariant 4 requires genuine disk read-back (spec text already correct, code non-compliant); implementer extracted `mechanism_a_write_and_verify_sealed_shard` performing a real post-hoc disk read after each shard write, before index publication; disk-corruption-race fault-injection test added | `feature/S-25.02-backfill` @ `b1134954` |
| F-C3-P6-003 | MEDIUM | FIXED — `is_id_tagged_lesson_heading` now consumes the full digit run and requires the documented `\b` word boundary afterward, rejecting alphabetic/underscore-suffixed h3 sub-headings; sibling-swept against `is_lesson_record_heading`; negative fixtures + packing test added | `feature/S-25.02-backfill` @ `b1134954` |

## Code Gate (this burst)

Full `cargo test --workspace --all-targets` suite: green; `cargo fmt --check --all`: clean; `cargo
clippy --workspace --all-targets -- -D warnings`: clean; 784 tests total on `feature/S-25.02-backfill`.
Feature branch HEAD: `feature/S-25.02-backfill` @ `b1134954` (implementer's F-C3-P6-001/002/003 fixes +
test-writer's new/retired test set, immediately after `26c79f13`), pushed to `origin`.

## Cross-Vendor Process Note

This is the first pass in cluster-3's cascade — and the first in this cycle's broader
history — reviewed by a model OUTSIDE the Claude family (OpenAI Codex). It surfaced 1 genuine
data-loss bug and 2 genuine spec-fidelity gaps that 5 consecutive same-vendor (Claude) adversary
passes either missed outright or, in F-C3-P6-001's case, examined the SAME underlying mechanism and
explicitly characterized as non-blocking (O-C3-P3-001, pass-3: "not reachable today... deferred to
T-12") without constructing the repeated-prefix counterexample that falsifies that characterization,
and in F-C3-P6-002's case, read the exact "actual bytes written to disk" spec language across 3
subsequent passes (4, 5, and this cascade) without flagging that the implementation never performs a
disk read-back. See `cycles/v1.0-brownfield-backfill/lessons.md` for the `[codified]` lesson this
finding pattern is promoted to (routed to a new draft follow-up story, engine self-improvement / E-12
Engine Governance epic — cross-vendor adversary pass as a required BC-5.39.001 convergence-protocol
step, not an optional/occasional practice).

## Next

Because findings were present (2 HIGH + 1 MEDIUM, all fixed this burst), pass-6 does **NOT** satisfy
BC-5.39.001's CLEAN bar — the human-authorized full grind-to-literal-3-CONSECUTIVE-CLEAN drive (started
at this pass, per D-1195) has NOT yet begun accumulating a streak: **cluster-3 LOCAL streak stays
0/3.** The substantive-CODE-defect-surface-EXHAUSTED assessment reached after passes 4/5 is hereby
**REOPENED** — the cross-vendor pass demonstrates the surface was not, in fact, exhausted; it was
exhausted only with respect to the same-vendor review perspective that had examined it 5 times.
**NEXT = cluster-3 LOCAL adversary pass-7**, fresh context, against BC-1.18.008 v1.6 / BC-1.18.007
v1.2 / code `feature/S-25.02-backfill` @ `b1134954` — continuing the 3-CLEAN drive, with cross-vendor
passes now an explicit part of the rotation per this pass's own codified process lesson.
