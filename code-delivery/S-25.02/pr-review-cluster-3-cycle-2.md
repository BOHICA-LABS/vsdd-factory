## PR Review — Cycle 2 (fresh-eyes, PR #831, `feature/S-25.02-backfill` @ `a5a80103`)

> **VERDICT: APPROVE.** Posted as a COMMENTED review only because GitHub refuses a formal
> `--approve` from the PR author's own account (`addPullRequestReview`: "Can not approve your own
> pull request"), exactly as in cycle 1. Treat this as an approval for merge purposes: cycle-1's
> BLOCKING-1 is closed and no blocking findings remain.

**Verdict: APPROVE.** BLOCKING-1 is genuinely closed — structurally, not by assertion. Four
non-blocking findings below, all follow-ups rather than merge blockers.

Scope of this cycle per the triage comment: verify BLOCKING-1's fix and hunt for regressions the
fix itself introduced. SUGGESTION-2/3/4/5 and the cycle-1 NITs are not re-litigated; none of them
was made more severe or mischaracterized by this fix.

---

### BLOCKING-1 — VERIFIED CLOSED

I did not take the "fixed" claim on faith. What I actually checked:

**1. It is a real struct field, not a renamed side-channel.** `ShardIndex` now declares
`pub backfill_manifest: Option<BackfillManifest>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, positioned between the scalar
`retention_count` and the `#[serde(rename = "shard")] shards` array-of-tables. That ordering
matters and is correct: TOML requires scalars before tables, and a `[backfill_manifest]` table
emitted before `[[shard]]` is well-formed. `#[serde(default)]` makes every pre-v1.9 index file
deserialize to `None` (backward compatible); `skip_serializing_if` keeps the table out of the file
entirely when absent, preserving the retired design's on-disk shape.

**2. The side channel is genuinely gone.** `BackfillManifestWrapper`, `BackfillManifestDocument`,
and `read_backfill_manifest` are deleted, not deprecated. `write_shard_index_for_backfill` lost its
fourth parameter and its manual string-append — it is now a single `toml::to_string(index)`.

**3. `publish_shard_index_update` needs no special-case code — verified, not assumed.** I grepped
every index write site in the module. There are exactly **two**: `publish_shard_index_update`
(line 3418) and `write_shard_index_for_backfill` (line 6271). Both serialize the whole `ShardIndex`
struct. `publish_shard_index_update` does `load_shard_index` → `index.shards.push(...)` →
`toml::to_string(&index)` → `write_atomic`, so the manifest round-trips as an ordinary consequence
of struct serialization, with zero manifest-aware code. Both self-heal paths
(`self_heal_resume_from_truncate`, `self_heal_reconcile_missing_index_entries`) route through
`publish_shard_index_update` rather than writing the index themselves, so they inherit the same
property. The synthesized-index `unwrap_or_else` branch correctly sibling-swept to
`backfill_manifest: None`.

**4. The two new tests exercise the real mechanism.** I read both bodies, not the names.
- `..._EC015_run_backfill_split_after_prior_roll_appends_shards_and_publishes_manifest` calls the
  real `execute_roll` against a real temp-dir canonical file, then the real
  `run_mechanism_a_backfill_split`. It asserts the roll's `decision-log.0001.md` is byte-identical
  to what the roll wrote (not merely present), that the backfill's shards land at seq 2 and 3, that
  the published index carries `seqs == [1,2,3]` with the roll's path verbatim, and that
  `original_bytes`/`original_sha256` hash the *post-roll current* content — pinning Composability
  point 1 rather than restating it.
- `..._EC014_backfill_then_roll_then_backfill_manifest_survives_roll` does backfill → real
  `execute_roll` → re-read, and asserts `manifest_after_roll == manifest_before_roll` as a whole
  struct (`PartialEq`), not field-by-field on a subset. This is exactly the assertion that fails on
  the pre-fix code. It additionally asserts the second backfill invocation is **not**
  `MissingBackfillManifest`, which is the precise regression pin.

No mocks, no stubs, no injected seams. Both are genuine end-to-end regression tests.

**5. Spec fidelity (BC-1.18.008 v1.9).** Invariant 3's redefined idempotency basis is implemented
literally: `mechanism_a_backfill_already_migrated` now does `load_shard_index(...)` and returns
`index.backfill_manifest.is_some()`, replacing the `fs::metadata(...).is_ok()` bare-existence check
the spec explicitly supersedes. The Residual `MissingBackfillManifest` clause is real — it exists
in the BC body — so the previously-fabricated "Invariant 3(c)" citation is genuinely corrected, and
`error-taxonomy.md` v1.14's `E-SHD-011` form (b) row now cites the real clause. `E-SHD-002` form (c)
(`ArchivalIndexEntryVanished`, from commit `4febaaa7`) is documented, closing cycle-1's SUGGESTION-1.
The narrowed disposition matches the code: with the idempotency check keyed on manifest presence,
the roll-before-backfill shape never reaches `heal_or_confirm_already_migrated` at all, so the
variant is now only reachable via a TOCTOU race or field-specific corruption, exactly as specified.

**6. Gates, run live in this worktree — not taken from a self-report.**
- `cargo test --workspace --all-targets` → exit 0, zero failures.
- `cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test` → 62 passed, 0 failed;
  both new `FC3P9004` tests and the rewritten/added `INV3` idempotency tests confirmed present and
  passing by name.
- `cargo fmt --check --all` → clean.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.

---

### Findings

| # | Severity | Category | Finding | Suggestion |
|---|----------|----------|---------|------------|
| S-1 | suggestion | coherence | **EC-014's "typically SAFE" claim is factually wrong, and the v1.9 changelog contradicts its own EC row.** EC-014 says a post-roll recovery-confirmation "resolves correctly (typically SAFE...)". It cannot. `execute_roll` truncates the canonical file to **empty** (step (c)), so after *any* ordinary roll the canonical is 0 bytes and matches neither `final_*` nor `original_*` — the disposition is **always** AMBIGUOUS, never SAFE. The delivered test proves this: it asserts `second.is_err()` with `E-SHD-011`. Separately, the v1.9 changelog's implementer-scope item (4) instructs the test to assert the second invocation "correctly reports `AlreadyMigrated` via the SAFE disposition" — which contradicts EC-014's own body and the shipped test. The implementation followed the normative EC row, which is the right call; the spec is what is internally inconsistent. | Route to `product-owner`: amend EC-014 to state the post-roll disposition is *always* AMBIGUOUS (not "typically SAFE"), and correct the v1.9 changelog's item (4) to match. No code change. |
| S-2 | suggestion | missing | **Residual liveness: a healthy, fully-migrated artifact hard-fails once it has been rolled.** This is BLOCKING-1's own failure *family*, narrowed but not eliminated. Post-fix, the manifest survives the roll — but the canonical content has moved past it, so re-invoking `run_mechanism_a_backfill_split` returns `E-SHD-011` ("recovery-confirmation ambiguous ... pending operator investigation") for an artifact that is correctly and completely migrated. v1.9 consciously adjudicates this as expected, and Invariant 3's three-way list technically permits it, so it is **not** blocking here. But once F4 wiring makes this function reachable from a production caller, every re-run against a rolled artifact becomes a false-alarm operator escalation. | Before the F4 activation wiring lands, add a terminal "migration complete" signal the recovery check can short-circuit on (e.g. a `migrated_at` marker alongside the manifest, or a `completed: bool` on `BackfillManifest`) so a benign post-roll state resolves `AlreadyMigrated` instead of escalating. Track against the S-12.12-class follow-up. |
| S-3 | suggestion | coherence | **Stale doc comment now contradicted by the very clause this commit implements, plus a silent field overwrite.** `run_mechanism_a_backfill_split`'s doc comment still reads: *"`retention_count` is threaded in explicitly rather than re-derived, since no shard-index (and therefore no `ShardIndex::retention_count`) exists yet for an artifact that has never been backfilled."* Under the Composability clause an index demonstrably **can** exist. The code consequence is real: `fresh_backfill_shard_index` rebuilds the index from `entry` + the caller-supplied `retention_count`, so a pre-existing rolled index's persisted `retention_count`, `current_shard`, `shard_cap_bytes`, `max_single_record_bytes`, `safety_margin_bytes`, `practical_fuel_ceiling`, and `worst_case_fuel_per_byte` are all silently discarded and replaced. Today the values coincide, so nothing misbehaves — but nothing enforces that, and the doc comment now actively misleads the next reader. | Update the doc comment to reflect the Composability clause, and either (a) carry forward the pre-existing index's `retention_count` explicitly, or (b) state in the comment that the caller's value deliberately wins, with the BC-1.18.007 EC-002 ("newly-lowered retention") rationale. |
| S-4 | suggestion | coverage | **EC-016 branch skips retention, contrary to Postcondition 4.** The `partitions.len() <= 1` branch now carries `existing_shards` forward (correctly) but, unlike the main path, never calls `archive_overflow_shards`. With a pre-existing rolled shard set exceeding `retention_count`, it publishes an over-retention active set — precisely what Postcondition 4 forbids ("does not first produce an over-retention active set and defer archival to a later event"). This is reachable rather than theoretical: `archive_overflow_shards` has **no production caller outside this function**, so BC-1.18.006's roll path does not apply retention itself and `existing_shards` is not bounded by `retention_count`. The branch was safe before this commit only because `shards` was hardcoded `Vec::new()`. No test covers EC-016-with-pre-existing-shards at all. | Call `archive_overflow_shards` in the EC-016 branch as the main path does, and add an EC-016 + prior-roll test. |
| N-1 | nit | coherence | Two comments in `bc_1_18_008_backfill_split_test.rs` (lines 1184, 2886) still cite `read_backfill_manifest` and `BackfillManifestWrapper` — symbols this commit deleted. | Sweep the two comments to reference `load_shard_index(...).backfill_manifest`. |

---

### Checklist

| Item | Result |
|------|--------|
| 1. Diff coherence | Pass — all three files trace to BLOCKING-1's adjudicated fix. |
| 2. Description accuracy | Pass — behavior matches the amended BC and the commit messages. |
| 3. Test coverage | Pass for the fix (two real end-to-end regression tests + a rewritten and a new INV3 idempotency test). One gap noted at S-4. |
| 4. Demo evidence | Recorded at `a1328c3f` for cluster-3; this fix-burst is a spec-driven correction with regression-test evidence, consistent with prior fix-burst practice on this PR. |
| 5. Commit quality | Pass — conventional format, story ID, BC version cited, test-before-fix ordering (`1cce59b7` RED, then `a5a80103` GREEN). |
| 6. Diff size | 589 insertions / 145 deletions across 3 files — well within range; the bulk is doc comments and test bodies. |
| 7. Missing changes | None for BLOCKING-1. S-2 and S-4 are forward-looking gaps, not omissions from this scope. |
| 8. Dependency status | `#818` and `#824` merged; branch targets `develop`. |

**No blocking findings.** The fix is structural: BLOCKING-1's failure class is eliminated by
construction rather than patched at each call site, which is the stronger closure. The four
suggestions are follow-ups, and S-2 and S-4 should be closed before the F4 activation wiring makes
this code path reachable from production.
