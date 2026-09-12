# PR #832 Review — S-25.02 cluster-4 (BC-1.18.009 v1.7 mechanism-B1 rotation)

**Reviewed SHA:** `44d83e993950bc9c8a1f6dbc3ba06acba1b86dba`
**Base:** `develop` (`08ad44b5`)
**Cycle:** 1 (fresh-eyes, different-model cognitive diversity)
**Verdict:** **REQUEST_CHANGES** — 3 BLOCKING, 4 SUGGESTION, 5 NIT

> Posted as a review **comment** rather than a `CHANGES_REQUESTED` review because GitHub rejects
> `addPullRequestReview` with `request_changes` from the PR author's own account
> ("Can not request changes on your own pull request"). The verdict above is the authoritative one:
> **do not merge `44d83e99`.**

> Scope note: this review is pinned to `44d83e99`. Commits `c74c3014` (SEC-001/SEC-002) and
> `265753f5` (B1/B2 fixes) landed on the branch *during* this review and are explicitly
> **not** assessed here — they require a cycle-2 pass.

---

## What I verified (independent, not taken from the PR body)

| Check | Method | Result |
|---|---|---|
| New suites pass | `cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test` / `-p last-amended-migrate --test bc_1_18_009_rotate_at_test` in a clean detached worktree at `44d83e99` | **10/10 and 2/2 green** |
| No regression in the refactored crate | `cargo test -p last-amended-migrate` (full crate, all suites) | **green** — the `rotate_changelog` → `rotate_changelog_at` delegation is behaviour-preserving; `bc_10_13_001_pc5_rotation_test.rs` still pins the `<file-basename>-changelog-archive.md` cycle path |
| Format / lint | `cargo fmt --check --all`; `cargo clippy -p factory-dispatcher -p last-amended-migrate --all-targets -- -D warnings` | **clean** |
| Full workspace, CI-equivalent | `CI_REQUIRE_ARTIFACTS=1 VSDD_CORPUS_ROOT=<mounted .factory> cargo test --workspace --all-targets` at `44d83e99` with `.factory` mounted at `origin/factory-artifacts` (`616daf36`) | **exit 0, 227 suites ok** |
| Operator-surfacing path coherence | Traced `shard_gate_error_outcome` → `extract_block_info` (`wasi_block = exit_code == 2 && on_error == Block`) → `extract_reason_from_outcome` (`message` key) | **coherent**; `plugin_requests_block` and the async `has_block_json` leg are not on this path, so the `"outcome":"error"` marker change does not silently defeat block detection |
| Demo evidence | Read README + all 3 `.tape` files; confirmed `.gif` + `.webm` per clip, real `cargo test` invocations, no hand-typed output | **adequate** — success path and error path both recorded |
| Deferrals | Obs-B → S-25.05, E-SHD Display lint → S-12.13 | **accepted, not re-flagged** |

Overall the mechanism is well-built: the E-SHD-014 guard ordering (`Ok(report) if !report.mutated`
before `Ok(_) => Block`) is correct and load-bearing, the `Error`-vs-`Block` split in `executor.rs`
is a genuine semantic improvement over the previous conflation, and the archive newline
normalization in `rotate_changelog_at` fixes a latent bug that would have made *any* second
rotation into an existing archive fail YAML validation.

---

## BLOCKING

### BLK-1 — Absolute, machine-local archive path is written into a version-controlled spec artifact
**Category:** correctness / portability
**Files:** `crates/factory-dispatcher/src/shard_manager.rs` (`ShardShape::FrontmatterChangelogArray` arm), `crates/last-amended-migrate/src/rotate.rs` (`rewrite_source_after_rotation`)

The gate builds the archive path from `target_path.parent()`, and `target_path` is taken verbatim
from `tool_input.file_path` in `executor::shard_cap_precheck`. Claude Code's `Edit`/`Write`/`MultiEdit`
always supply an **absolute** `file_path`, so `archive_path` is absolute in production.
`rewrite_source_after_rotation` then writes that value straight into the source frontmatter:

```rust
let pointer_line = format!(
    "changelog_archive: \"{}\"\n",
    crate::escape::escape_raw_value(&archive_path.display().to_string())
);
```

Consequence: the first production rotation injects
`changelog_archive: "/Users/<user>/Documents/GITHUB/vsdd-factory/.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md"`
into `BC-INDEX.md`, which is then committed to `factory-artifacts`. That is non-portable across
checkouts and CI, discloses the local filesystem layout into a public repository, and directly
contradicts the **relative** `canonical_path_pattern` this same PR registers in
`artifact-path-registry.yaml`
(`.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`).

This is not a path-traversal issue, so a CWE-22 input guard does not close it. The pointer must be
written relative to the factory root (or relative to the target file).

**Why the tests miss it:** `grep -n 'changelog_archive' ` over both new test files returns **zero
hits** — no test asserts on the pointer line at all, and every fixture uses an absolute
`tempfile::tempdir()` path, so the suite exercises the defective behaviour without observing it.

**Suggestion:** make the pointer relative before writing, and add an assertion that the written
`changelog_archive:` value is `!Path::new(v).is_absolute()` for a gate-driven rotation.

---

### BLK-2 — `BC-INDEX-changelog-archive.md` filename is hardcoded inside a shape-generic handler
**Category:** correctness / spec-fidelity
**File:** `crates/factory-dispatcher/src/shard_manager.rs` (`ShardShape::FrontmatterChangelogArray` arm)

```rust
let archive_path = target_path
    .parent()
    .unwrap_or_else(|| std::path::Path::new("."))
    .join("BC-INDEX-changelog-archive.md");
```

The basename ignores `entry.artifact_stem`. Nothing in `validate_entry` restricts
`shape = "frontmatter-changelog-array"` to `BC-INDEX` — the shape-scoped block only enforces
`n`-presence, `n != 0`, and the `low_water_mark` range. A perfectly legal operator entry for
`VP-INDEX`, `ARCH-INDEX`, or `STORY-INDEX` would therefore rotate into
`<that artifact's own directory>/BC-INDEX-changelog-archive.md`: a misnamed archive, an
**unregistered** write path (`artifact-path-registry.yaml` `enforcement_level: block` only covers
the behavioral-contracts location), and a `Block` reason that names the wrong file to the agent.

This also diverges from the two established conventions in the same codebase:
`resolve_archive_path` in `rotate.rs` derives `format!("{basename}-changelog-archive.md")`, and the
flat shape derives `format!("{artifact_stem}.shard-index.toml")`.

**Suggestion:** `format!("{}-changelog-archive.md", entry.artifact_stem)`. This is safe without
extra sanitization because SEC-002 in `validate_entry` already rejects `/`, `\`, `..`, and NUL in
`artifact_stem`. Register the sibling INDEX archive paths in `artifact-path-registry.yaml` at the
same time, or add a shape-scoped config check that fails loud for an unregistered stem.

---

### BLK-3 — Required check `cargo-host` is RED at the reviewed SHA, and the PR body asserts the opposite
**Category:** merge gate / description accuracy

`cargo-host (ubuntu-latest)` and `cargo-host (macos-latest)` both **fail** at `44d83e99`, failing
step `cargo test (workspace, all targets)`:

```
test tests::test_BC_5_39_005_f_p1_001_real_state_md_banner_wc_passes ... FAILED
test tests::test_BC_5_39_005_full_validation_against_real_state_md ... FAILED
assertion `left == right` failed: real STATE.md banner claims 421 lines but actual count is 423
  — the banner wc-l is stale; update STATE.md banner before committing
test result: FAILED. 63 passed; 2 failed
```
(`crates/hook-plugins/validate-state-structure`)

**Root cause is external to this diff.** The `cargo-host` job mounts `origin/factory-artifacts` and
runs under `CI_REQUIRE_ARTIFACTS=1`, so these two tests validate the live `STATE.md`; that banner
was stale on `factory-artifacts` when CI ran. It has since been fixed on that branch by `616daf36`.
I confirmed this by reproducing the exact CI invocation at `44d83e99` against
`origin/factory-artifacts@616daf36` — all green.

It is nonetheless blocking as a **merge gate**, and the PR body's claims are inaccurate as written:
"`cargo test --workspace --all-targets` green at HEAD 44d83e99", "Regressions: 0", and the
"All CI status checks passing" checklist line. A reviewer relying on the body would have merged red.

**Suggestion:** re-run `cargo-host`, confirm green, and correct the Test Evidence table to state the
SHA the gate was actually verified against. No implementer code work required.

---

## SUGGESTION

### SUG-1 — `E-SHD-014` message omits the self-deadlock escape hatch its sibling error is required to carry
**File:** `crates/factory-dispatcher/src/shard_manager.rs`

`E-SHD-014` is a **permanent** hard block: every `Edit`/`Write`/`MultiEdit` against `BC-INDEX.md`
routes through this same read, so once counter divergence exists the artifact is unwritable through
any gated tool — including a would-be repair edit. The message ends at
"manual inspection of {path} required", which does not tell the operator how to escape.

The EC-021 message in `read_changelog_item_count`, a few hundred lines up, exists for exactly this
failure class and its doc comment states the escape-hatch text is "MANDATORY, load-bearing content,
not cosmetic": it must state that the gate only intercepts `Edit`/`Write`/`MultiEdit` so the
operator can repair by other means (e.g. a Bash-invoked edit). `E-SHD-014` should carry the same
clause, plus the concrete remedy for its actual cause — converting the inline flow sequence
`changelog: [{...}]` to block-sequence form.

### SUG-2 — Reimplements `shard_sibling_path` inline instead of reusing it
`shard_sibling_path(canonical_path, filename)` already encapsulates
`parent().map(|d| d.join(f)).unwrap_or_else(...)` and is used by the flat shape. The B1 arm open-codes
the same logic with a `Path::new(".")` fallback. Reusing the helper keeps the no-parent policy in one
place (TD-VSDD-060 sibling-site discipline) — relevant because the silent `"."` fallback would put
the archive in the process CWD, wholly outside the registered path.

### SUG-3 — B1 pre-empts the deferred BC-1.18.012 one-time backfill, undocumented
`BC-INDEX.md` currently holds 297 `changelog:` items. With a typical `N = 50` / `low_water_mark = 25`
entry, the very first gated edit after an operator adds `.factory/shard-config.toml` performs a
single ~272-item mass rotation — the work BC-1.18.012 was scoped to do as a controlled one-time
migration. Benign today only because no `.factory/shard-config.toml` exists in the repo, so the gate
is dormant. Worth stating explicitly in the PR body or a module comment so the cold-state ordering
between B1 and BC-1.18.012 is a decision rather than an accident.

### SUG-4 — Latent `rotate_changelog` bug fix is unannounced
The new `if !archive_content.is_empty() && !archive_content.ends_with('\n')` normalization also
changes the pre-existing CLI-facing `rotate_changelog` path: before it, a second rotation into an
existing cycle archive would concatenate mid-line and fail
`validate_changelog_sequence_yaml`. That is a real shipped-behaviour fix and should be called out in
the PR body / CHANGELOG rather than landing silently inside a refactor.

---

## NIT

| # | Finding | File |
|---|---|---|
| NIT-1 | `build_b1_block_reason`'s doc comment still calls itself a "stub" and says "The test for this function's message format passes against this stub immediately" / "Implementer MUST still write a test" — leftover Red-Gate narrative that the O-1 sweep (commits `78269cef`/`2886ff5f`/`32350e2c`) missed, in the one function the sweep's own commits touched. | `shard_manager.rs` |
| NIT-2 | Error text reads `"E-SHD-004: rotate_changelog invocation failed"` but the function invoked is `rotate_changelog_at`. Operator-facing; makes grepping the source from the message misleading. | `shard_manager.rs` |
| NIT-3 | `rotate_changelog_at`'s doc says "Identical behaviour to [`rotate_changelog`] in all other respects" — the relationship is now inverted (`rotate_changelog` delegates to it), and the newline normalization is not identical to prior behaviour. | `rotate.rs` |
| NIT-4 | Demo README cites "BC-1.18.009 **v1.6**" while the PR title/body claim v1.7, and its AC table cites "INV-5 v1.6 hardening" / "Inv-6 withdrawn in v1.7". One of the two is stale. | `docs/demo-evidence/S-25.02/cluster-4-b1-rotation/README.md` |
| NIT-5 | `resolved_low_water_mark(n, ..) as usize` truncates on 32-bit targets. Unreachable for realistic `n`, but `usize::try_from(...)` with a fail-loud arm matches the module's stated "never an unwrap/expect, always fail-loud defensive" posture. | `shard_manager.rs` |

---

## Checklist result

| # | Item | Result |
|---|---|---|
| 1 | Diff coherence | PASS — all changes trace to BC-1.18.009 / the Red-Gate narrative sweep |
| 2 | Description accuracy | **FAIL** — CI claims false (BLK-3); Changed Files table and mermaid are otherwise accurate |
| 3 | Test coverage | PARTIAL — happy path, EC-008 divergence (real inline-sequence fixture), E-SHD-004 (EISDIR), below-threshold, amortized cadence, correct retry, and double-rotation accumulation are all covered; **no** coverage of the `changelog_archive:` pointer value (BLK-1), the per-stem archive filename (BLK-2), `MigrationMode::Check` on the new API, or the `create_dir_all` branch |
| 4 | Demo evidence | PASS — `README.md` + `.gif`/`.webm` per AC, success and error paths both recorded, real test invocations |
| 5 | Commit quality | PASS — conventional format, story ID on every commit, no AI attribution |
| 6 | Diff size | PASS in substance — 1,913 additions but 1,327 are tests and ~350 are demo tapes/README; production delta is ~210 lines |
| 7 | Missing changes | See BLK-1/BLK-2 |
| 8 | Dependency status | PASS — clusters 1/2/3 (#818/#824/#831) merged; base `08ad44b5` contains cluster 3 |
