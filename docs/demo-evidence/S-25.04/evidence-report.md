---
document_type: demo-evidence-report
product: "vsdd-factory / validate-factory-path-staged"
pipeline_run: "2026-09-04"
demo_type: "library"
recording_tool: "vhs"
status: recorded
---

# Demo Evidence Report — S-25.04

## Product: `validate-factory-path-staged` (BC-4.16.002) — Layer-1 zero-enforcement-gap closure
## Pipeline Run: 2026-09-04
## Demo Type: library (test-harness) — backend WASM PostToolUse hook validator, no UI

Story S-25.04 closes the zero-observable-enforcement gap left by `validate-factory-path-staging`'s
(BC-4.16.001) `PreToolUse`-only `failure_policy = "fail-closed"` registration (S-25.01 / ADR-047
§Decision 8a). It adds a new companion crate, `validate-factory-path-staged`
(`crates/hook-plugins/validate-factory-path-staged/`), registered `PostToolUse ^Bash$` — the first
Layer-1 validator whose fail-closed assignment structurally reaches
`write_indeterminate_marker` (BC-1.18.001 Postcondition 4).

There is no UI to record. Evidence below is captured via VHS terminal recordings of `grep`
inspection of the production artifacts (registry, source, CHANGELOG) interleaved with live
`cargo test` runs of the specific Red-Gate/regression tests that pin each acceptance criterion,
per the demo-recorder skill's "library (test harness)" demo form for CLI/backend products.

**Toolchain note:** the installed `vhs` (v0.11.0) `Wait`/`Wait+Line` command timed out
unconditionally in this sandbox regardless of shell (`zsh`/`bash`), quoting, or pattern
(reproduced on a trivial `echo` + `Wait+Line` tape with no repo dependencies) — a pre-existing
environment incompatibility, not specific to this story's commands. Every tape below therefore
follows the same `Sleep`-based timing convention already established in this repo's own
`docs/demo-evidence/S-17.05/AC-008-fail-open.tape`, with generous sleep durations
(verified frame-by-frame below) instead of `Wait+Line`. All commands and their real output were
independently confirmed via direct shell execution before being scripted into each tape, and each
recording was re-inspected frame-by-frame after generation (via `ffmpeg` frame extraction) to
confirm the actual command output — not just the typed command — is visible in the recording.

---

## Per-AC Demo Recordings

| AC | Story | Description | Recording | Format | Status |
|----|-------|-------------|-----------|--------|--------|
| AC-001 | S-25.04 | New PostToolUse companion (`event=PostToolUse`, `tool=^Bash$`, `priority=161`, `failure_policy=fail-closed`) structurally reaches the marker-write path (BC-4.16.002 PC3 / BC-1.18.001 PC4) | [AC-001-posttooluse-marker-reachability.gif](AC-001-posttooluse-marker-reachability.gif) / [.webm](AC-001-posttooluse-marker-reachability.webm) | gif+webm | recorded |
| AC-002 | S-25.04 | Reuse-not-reimplementation: no second marker/gate mechanism, no `write_indeterminate_marker` call site inside the new crate, and no `factory-dispatcher` dependency exists for one to call | [AC-002-reuse-not-reimplementation.gif](AC-002-reuse-not-reimplementation.gif) / [.webm](AC-002-reuse-not-reimplementation.webm) | gif+webm | recorded |
| AC-003 | S-25.04 | `validate-factory-path-staging`'s own `PreToolUse` registration is byte-unchanged by this story (EC-004 construction-level distinctness) | [AC-003-sibling-unchanged.gif](AC-003-sibling-unchanged.gif) / [.webm](AC-003-sibling-unchanged.webm) | gif+webm | recorded |
| AC-004 | S-25.04 | ADR-039 §Decision 2 seventh member: dispatcher's sanctioned fail-closed allowlist names `validate-factory-path-staged` as a distinct second authority, not a fourth Cohort A member | [AC-004-adr039-seventh-member.gif](AC-004-adr039-seventh-member.gif) / [.webm](AC-004-adr039-seventh-member.webm) | gif+webm | recorded |
| AC-005 | S-25.04 | Layer-1 effective fail-closed count moves from ZERO (S-25.01 merge, unchanged historical anchor) to N≥1 at S-25.04 merge | [AC-005-fail-closed-count-zero-to-n.gif](AC-005-fail-closed-count-zero-to-n.gif) / [.webm](AC-005-fail-closed-count-zero-to-n.webm) | gif+webm | recorded |
| AC-006 | S-25.04 | Full crate regression suite (45 tests: 40 unit + 5 proptest) green, including T-4 BROAD-scope detection via a non-`git add`-text wrapper command; DO-NOT-DELETE backward-compat guard (BC-1.18.004 PC5) unmodified and green | [AC-006-regression-suite-green.gif](AC-006-regression-suite-green.gif) / [.webm](AC-006-regression-suite-green.webm) | gif+webm | recorded |
| CORE | S-25.04 | Canonical block message + block path (T-1: `.factory/` staged on `develop`) and clean-pass path (T-3: non-`.factory/` path staged) | [CORE-block-and-pass-path.gif](CORE-block-and-pass-path.gif) / [.webm](CORE-block-and-pass-path.webm) | gif+webm | recorded |

---

## Per-AC evidence detail

### AC-001 — PostToolUse companion reaches the marker-write path

- **Registry stanza** (`plugins/vsdd-factory/hooks-registry.toml:1453-1470`): `name =
  "validate-factory-path-staged"`, `event = "PostToolUse"`, `tool = "^Bash$"`, `priority = 161`,
  `failure_policy = "fail-closed"`, `on_error = "continue"`, `timeout_ms = 5000` —
  `[hooks.capabilities.exec_subprocess]` `binary_allow = ["git"]`.
- **Tests** (`crates/hook-plugins/validate-factory-path-staged/src/tests.rs`):
  `test_bc4_16_002_ac001_registry_entry_exists`,
  `test_bc4_16_002_ac001_pc3_registry_entry_is_post_tool_use_bash` — both green
  (`cargo test -p validate-factory-path-staged ac001`).
- Contrast with the sibling: `validate-factory-path-staging` is registered `PreToolUse` (line
  1024), which per BC-1.18.001 Invariant 4 can never reach `write_indeterminate_marker` — this is
  the exact structural gap this story closes.

### AC-002 — Reuse, not reimplementation

- `grep -rn 'write_indeterminate_marker(' crates/hook-plugins/validate-factory-path-staged/src/`
  returns **no call site** inside the new crate — the two textual mentions in `lib.rs`/`tests.rs`
  are doc-comments/assertion-message prose citing the (dispatcher-owned, unchanged) function name,
  not a call.
- `crates/hook-plugins/validate-factory-path-staged/Cargo.toml` declares only `vsdd-hook-sdk`,
  `serde`, `serde_json` as dependencies — **zero dependency on `factory-dispatcher`**, where
  `write_indeterminate_marker` / `should_write_marker` / `classify_outcome` live
  (`crates/factory-dispatcher/src/indeterminate_marker.rs`, `executor.rs`). Reuse here is
  architecturally enforced, not merely a convention the implementer followed.

### AC-003 — Sibling unchanged

- `git diff $(git merge-base HEAD origin/develop) HEAD -- plugins/vsdd-factory/hooks-registry.toml`
  touches **zero** lines containing `validate-factory-path-staging"` — the sibling's own
  `[[hooks]]` stanza (lines 1022-1041) is byte-identical to the pre-S-25.04 merge-base. This story
  only inserts the new `validate-factory-path-staged` stanza (verified diff hunk headers:
  `@@ -1028,11 +1028,16 @@` for a comment-only annotation add above the sibling's unchanged body,
  and `@@ -1426,6 +1431,44 @@` for the wholly new stanza further down the file).
- `test_bc4_16_002_ec004_registry_entry_name_distinguishes_from_sibling` — green — asserts the
  registered name is the exact, distinct `"validate-factory-path-staged"` (past participle) and
  that the sibling's own entry is still separately present.

### AC-004 — ADR-039 §Decision 2 seventh member

- `crates/factory-dispatcher/src/registry.rs` (`sanctioned_fail_closed` HashSet, in
  `test_BC_1_01_016_production_registry_all_entries_default_to_fail_open`): four entries —
  `validate-factory-path-staging`, `validate-pr-merge-prerequisites`,
  `validate-wave-gate-prerequisite` (Cohort A, ADR-047 §D8a, unchanged) **plus**
  `validate-factory-path-staged` (ADR-039 §Decision 2 seventh member — explicitly documented in
  the surrounding code comment as a **second, distinct authority**, not a fourth Cohort A member).
- `cargo test -p factory-dispatcher --lib
  test_BC_1_01_016_production_registry_all_entries_default_to_fail_open` — green. This is the
  dispatcher's own regression sentinel: it asserts the production registry's fail-closed entry set
  equals exactly this 4-member allowlist, no more, no less.

### AC-005 — Fail-closed count ZERO → N≥1

- `CHANGELOG.md` `[Unreleased] > Added` carries the S-25.04 bullet documenting the new
  enforcement trigger and the count-increase claim.
- Live count in the production registry: `grep -c '^failure_policy = "fail-closed"'
  plugins/vsdd-factory/hooks-registry.toml` → **4** (excludes 1 additional textual match that is
  inside a `#`-prefixed comment, not a live TOML key — verified: `grep -n 'failure_policy =
  "fail-closed"'` shows 5 raw matches, one at line 1442 is `# failure_policy = "fail-closed":
  this is the first Layer-1 validator...`, a comment).
- The same `test_BC_1_01_016_production_registry_all_entries_default_to_fail_open` gate (AC-004)
  pins this count structurally — a regression that silently added or removed a fail-closed entry
  would fail this test.
- `validate-factory-path-staging` itself remains at its own unchanged ZERO observable-enforcement
  registration (AC-003) — the N≥1 increase is attributable entirely to
  `validate-factory-path-staged`.

### AC-006 — Full regression suite green

- `cargo test -p validate-factory-path-staged`: **45/45 tests pass** — 40 unit tests (`src/lib.rs`
  via `mod tests`) + 5 proptest cases (`tests/proptest_path_variants.rs`). Includes
  `test_bc4_16_002_t4_broad_scope_detects_staging_via_non_git_add_text_command` (T-4): a
  `.factory/` path staged via a simulated non-`git-add`-text wrapper command (`python3
  fix_index.py`) on branch `develop` is still detected and blocked — proving the BROAD,
  unconditional-scope design (Precondition 2 / Invariant 7) closes the exact residual-risk class a
  text-gated pre-filter would have missed.
- `cargo test -p factory-dispatcher --lib
  test_BC_1_18_004_fail_open_default_preserves_advisory_behavior` (the S-25.01 DO-NOT-DELETE
  backward-compat guard, BC-1.18.004 PC5) — green, present, unmodified.
- **Full workspace** (`cargo test --workspace --all-targets`, captured in full): 592 tests passed;
  **1 pre-existing failure**, unrelated to this story:
  `test_S_19_04_ac006_T009_hermetic_tracked_bundle_zero_orphans`
  (`crates/factory-dispatcher/tests/bundle_orphan_check.rs`) — fails because two git-tracked WASM
  binaries (`hook-plugins/last-amended-migrate.wasm`,
  `hook-plugins/verify-state-timestamp-refresh.wasm`) are orphaned (tracked but not referenced by
  any registry entry). **Verified pre-existing and out of this story's scope**: both files are
  present, byte-for-byte, at `git merge-base HEAD origin/develop` — i.e. already on `develop`
  before any S-25.04 commit — and no S-25.04 commit touches either file
  (`git log --oneline $(git merge-base HEAD origin/develop)..HEAD -- <both paths>` returns empty).
  This finding is **out of scope for the demo-recorder role** (recording-only, no source/test
  edits) and is flagged here for orchestrator routing rather than silently omitted or fixed
  out-of-band. **AC-006, scoped to this story's own changes, is satisfied**: nothing in S-25.04's
  diff caused or touches this failure, and the story's own specified regression targets
  (`cargo test -p validate-factory-path-staged`, the BC-1.18.004 PC5 guard, and this story's own
  registry/count sentinels) are all green.
- `plugins/vsdd-factory/tests/run-all.sh` (bats integration suite): **completed, all green** —
  `Coverage: 2233 executed, 29 skipped (2262 total across 256 suites). All tests passed.` (exit
  code 0). Zero `not ok` lines across the full run.

### CORE behavior — block path and clean-pass path

- **Block message** (`crates/hook-plugins/validate-factory-path-staged/src/lib.rs`, the literal
  string passed to `HookResult::block_with_fix`): `"DETECTED: .factory/ path staged on product
  branch '<branch>' (post-hoc check). .factory/ paths are exclusively owned by the
  factory-artifacts worktree. A staging operation reached the git index without being intercepted
  by validate-factory-path-staging's PreToolUse guard (git plumbing, alias, wrapper script, or
  under-matched invocation text). Staged path: '<path>'"` with fix guidance `"Unstage immediately:
  git restore --staged <path> (or equivalent), or switch to the .factory/ worktree and commit from
  there on the factory-artifacts branch"`.
- **Block path** (T-1, `test_bc4_16_002_t1_blocks_factory_path_staged_on_develop`): mocked `git
  diff --cached --name-only` → `.factory/STATE.md`, mocked `git branch --show-current` →
  `develop` → `hook_logic` returns `HookResult::Block` with `FactoryPathStagedOnProductBranch`,
  exit code 2. Green.
- **Clean-pass path** (T-3, `test_bc4_16_002_t3_passes_no_factory_path_staged_on_feature_branch`):
  mocked `git diff --cached --name-only` → `src/lib.rs` (no `.factory/` path), branch
  `feature/S-25.04` → `hook_logic` returns `HookResult::Continue`. Green.

---

## Toolchain

| Tool | Version | Status |
|------|---------|--------|
| VHS | 0.11.0 | installed (`Wait`/`Wait+Line` non-functional in this sandbox; `Sleep`-based timing used instead, per-recording frame-verified) |
| cargo | workspace-pinned | installed |
| ffmpeg | (Homebrew) | installed (used only for post-hoc frame verification of recordings, not part of the recorded demos themselves) |
| bats-core | 1.13.0 | installed |

---

## Notes

- All 7 recordings were frame-inspected after generation (via `ffmpeg` frame extraction, not part
  of the recorded artifact) to confirm real command output — not just the typed command line — is
  visible before the recording ends. File sizes: 150 KB–420 KB per GIF, 120 KB–365 KB per WebM,
  all well under the <2 MB WebM / <5 MB GIF optimization targets.
- No source code or test files were modified to produce this evidence. All commands shown are
  read-only inspections (`grep`, `git diff`) or existing `cargo test` invocations against the
  story's own committed test suite.
- **Flag for orchestrator routing** (production-grade default, not silently omitted): the
  pre-existing, S-25.04-unrelated `test_S_19_04_ac006_T009_hermetic_tracked_bundle_zero_orphans`
  failure (two orphaned tracked WASM binaries already on `develop`) should be routed to
  `devops-engineer` or `state-manager` for a follow-up fix-burst — it is out of scope for both this
  demo-recorder task and this story's own diff.
