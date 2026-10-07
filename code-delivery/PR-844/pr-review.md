## PR Review — #844 (fresh-eyes, covered_sha `2ef47950752cb9be969204f60891ccc8668d630f`)

**Verdict: APPROVE.** No blocking findings. This approval covers head `2ef47950` only. Merge should still wait for the in-progress CI checks, especially `deny-advisories`, `cargo-host` (ubuntu/macos), `bats-full-suite`, and the five `build-dispatcher` targets.

### What I verified independently

1. **The sandbox did not get wider or narrower.** I read the wasmtime-wasi 48.0.5 source. `WasiCtxBuilder::preopened_dir(host, guest, FsPerms)` maps `FsPerms::ReadWrite` to `OpenMode::READ | OpenMode::WRITE` and stores `perms` on the `Dir`. `FsPerms` has two variants, `ReadOnly` and `ReadWrite`, so `ReadWrite` is the direct successor of `DirPerms::all() + FilePerms::all()`. The preopen is still the only one (project cwd mounted as `.`), and the failure path is still non-fatal with a debug log. The posture is unchanged from the existing SEC-001 setup.
2. **The lockfile resolves cleanly.** `wasmtime`, `wasmtime-wasi`, `wasmtime-environ`, `wasmtime-wasi-io` and every `wasmtime-internal-*` crate resolve to exactly one version, 48.0.5. There is no leftover 46.x entry. The lockfile drops `cap-std`, `cap-fs-ext`, `cap-net-ext` and `cap-time-ext`, and adds `io-lifetimes`, `wasm-metadata` and `wit-component`. `wit-component` now shows up at two versions (0.244.0 and 0.254.2). That is harmless for advisories, but you will see it if `bans.multiple-versions` is ever made strict.
3. **There are no leftover API call sites.** A grep of `crates/**/*.rs` finds no remaining `DirPerms`/`FilePerms` code. The only remaining mentions are in a comment and in a historical test message string.
4. **`wasm_component_model(false)` is correct and covered by a test.** The dispatcher only compiles core modules through `Module::new`, so this is defence in depth, not a behavior change. `engine_rejects_component_binaries` exercises the real `Component::new` path against the production `build_engine()`. `wat` is already a dev-dependency.
5. **The floor guard is sound.** `check_wasmtime_lockstep_floor` is a pure function. It rejects these fixtures:
   - below the floor (48.0.3, 46.0.3)
   - at or above the ceiling (49.0.0)
   - versions out of lockstep
   - duplicate entries, in both orders, including two identical entries

   It accepts 48.0.4 and 48.1.0. Because it requires exactly one entry per package, it closes the gap where a first-match parse could hide an older version.
6. **The bats AC-003 rewrite is correct.** It loops over both packages, checks `48.0.4 <= v < 49.0.0`, and then checks lockstep. I hand-checked the boolean: with `major == 48`, the test `minor > 0 || patch >= 4` is right.
7. **Commits:** 4 commits, all in Conventional Commits format with clear scopes. The test commit (`0beb654e`) comes before the fix (`2ef47950`), red then green. That is fine under squash-merge.
8. **Diff size:** about 430 non-lockfile lines. Most of the 317-line lockfile churn is mechanical.
9. **Description:** it matches the diff. Changes 1–4 map one-to-one to the commits.
10. **Demo evidence:** N/A, and I accept that here. This is a dependency and security fix with no user-visible ACs. Behavior is pinned by the tests above, not by a recording.
11. **Dependencies:** none upstream.

### Findings

| # | Severity | Category | File | Finding | Suggestion |
|---|----------|----------|------|---------|------------|
| 1 | SUGGESTION | coverage | `plugins/vsdd-factory/tests/s21-12-version-and-deny-gate.bats` (AC-004, line 195) | The AC-004 explicit-absence loop still lists only the five original advisories. The seven new ones are not listed. The `exit 0` assertion already covers them, but the header and AC-001/002 comments now point to 0316 and the others, and the Rust guard documents them. The per-ID check is the part that tells a reviewer *which* advisory came back. | Add `RUSTSEC-2026-0316 RUSTSEC-2026-0314 RUSTSEC-2026-0321 RUSTSEC-2026-0322 RUSTSEC-2026-0323 RUSTSEC-2026-0324 RUSTSEC-2026-0327` to the loop and update the AC-004 header comment and test name. |
| 2 | NIT | coverage | `plugins/vsdd-factory/tests/s21-12-version-and-deny-gate.bats` line 127 | AC-003 still takes the first match (`\| head -1`). If there are duplicate `wasmtime` entries, the bats gate checks only the first one. The Rust guard now rejects duplicates, so the two gates disagree on this case. | Mirror the single-entry rule, e.g. `count=$(... \| sort -u \| wc -l)` and fail if `count != 1`. |
| 3 | SUGGESTION | description | `crates/hook-sdk/HOST_ABI.md` line 528 | This PR edits a sentence saying plugins get a preopen for both `CLAUDE_PROJECT_DIR` **and the `FACTORY_STATE_FILE` parent directory**. `invoke_plugin` makes exactly one `preopened_dir` call (`host_ctx.cwd` as `.`), and nothing in `factory-dispatcher/src` mentions `FACTORY_STATE_FILE`. The inaccuracy was already there, but this is the security-model doc and this PR is rewriting this exact sentence. | Fix the sentence to describe the single project-root preopen. If a second preopen really exists somewhere else, cite where. |

None of these block merge.

---

## Cycle 2 re-review: covered_sha `dc38a9d75ef6ab94417b7bd3ada508b4053a155c`

**Verdict: APPROVE.** No blocking findings. I reviewed `2ef47950..dc38a9d7`, which is commits `6dc2dd52` (bats) and `dc38a9d7` (docs). Only two files changed, +28/-12. Nothing outside the three cycle-1 findings changed.

GitHub refused `--approve` because the author and reviewer are the same account, so this review was posted with `--comment`. A duplicate COMMENTED review was posted by mistake: 5438802669 (from github-ops) and 5438802786 (my repost) have the same body.

| # | Cycle-1 finding | Status | Verification |
|---|---|---|---|
| 1 | SUGGESTION: add the 7 new advisory IDs to the AC-004 loop | Resolved | The loop now checks all 12 IDs with `grep -qF`. The header comment, the GREEN-after line and the `@test` name are consistent with the loop. |
| 2 | NIT: AC-003 `head -1` duplicates | Resolved | It now counts entries with jq `length`. A count below 1 fails as "not found", and a count other than 1 fails and lists every version. The version is read only after the count is confirmed as exactly 1. |
| 3 | NIT: HOST_ABI.md `FACTORY_STATE_FILE` preopen | Resolved | The sentence now matches `invoke.rs`: a single `preopened_dir(&host_ctx.cwd, ".", FsPerms::ReadWrite)` call, and nothing in the dispatcher source preopens `FACTORY_STATE_FILE`. |

**Bash 3.2:** compatible. The new code uses only `local`, `$(...)`, `[ -ge/-eq ]`, `{ ; }` groups and a backslash continuation inside a `for` word list.

### Cycle-2 findings

| # | Severity | Category | File | Finding | Suggestion |
|---|----------|----------|------|---------|------------|
| 1 | NIT | coverage | `plugins/vsdd-factory/tests/s21-12-version-and-deny-gate.bats` (AC-003) | The duplicate-rejection branch has no negative test. This does not block: the guard is three lines and can be read directly, the Rust `check_wasmtime_lockstep_floor` handles duplicates authoritatively, and a fixture would need a `cargo metadata` stub. | Add a two-entry fixture if this file ever gets a stub seam. |
| 2 | NIT | description | `crates/hook-sdk/HOST_ABI.md` "Relationship" section | It still says "the preopened **directories**" (plural). | Make it singular. |
| 3 | NIT | description | `crates/hook-sdk/HOST_ABI.md` "WASI preopened directories" | "All plugins receive" is slightly absolute. The preopen is skipped when `cwd` is empty, and a failed preopen is non-fatal. | Add "when a project directory is available". |

CI on `dc38a9d7` was mostly pending at review time. Merge should wait for it to go green.
