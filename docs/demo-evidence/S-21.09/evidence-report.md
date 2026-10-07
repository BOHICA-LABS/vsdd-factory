---
document_type: demo-evidence-report
product: "vsdd-factory — validate-factory-path-staging WASM guard (Layer-1 registry parity gate)"
pipeline_run: "2026-08-12"
demo_type: "cli"
recording_tool: "vhs"
status: recorded
---

# Demo Evidence Report

## Product: validate-factory-path-staging.wasm artifact restore + per-name registry parity check
## Pipeline Run: 2026-08-12
## Demo Type: cli

Story S-21.09 is CLI/hook-infrastructure only (a Rust crate + bats gate) — there is no
UI, so all evidence is captured as terminal recordings (VHS, both `.gif` and `.webm`)
against the real worktree tree at `feature/S-21.09` (`HEAD 12d0fe98`, LOCAL 3-CLEAN
converged), plus one supplementary captured-log file for the AC-002 zero-skip claim
(a 36-line full-suite scroll that is more legibly reviewed as text than as a GIF).
No registries or git-tracked artifacts were mutated by any recording — all "orphan
fires" evidence uses the existing fixture-driven Cargo tests, which build isolated
synthetic declared/tracked sets in-memory or in throwaway tempdirs.

---

## Per-AC Demo Recordings

| AC | Story | Description | Recording | Format | Duration | Size | Status |
|----|-------|-------------|-----------|--------|----------|------|--------|
| AC-001 | S-21.09 | `validate-factory-path-staging.wasm` is present in the git INDEX (`git ls-files`) under gitignored `hook-plugins/`, AND declared in production `hooks-registry.toml` (`plugin = "hook-plugins/validate-factory-path-staging.wasm"`, line 947) | [AC-001-artifact-tracked-and-declared](AC-001-artifact-tracked-and-declared.webm) / [.gif](AC-001-artifact-tracked-and-declared.gif) | webm+gif | 15.3s | 72KB / 120KB | recorded |
| AC-002 | S-21.09 | Positive claim: with both required artifacts present (`factory-dispatcher` release binary + committed WASM) and `CI_REQUIRE_ARTIFACTS=1` set, zero tests in `validate-factory-path-staging.bats` skip — full 36-test suite, all `ok`, zero "not present"/"not built" skip lines | [AC-002-full-suite-zero-skips.txt](AC-002-full-suite-zero-skips.txt) | captured log | n/a | 3.7KB | recorded |
| AC-003 | S-21.09 | Guard BLOCKS `git add .factory/STATE.md` on `develop` (product branch) — T-001, `✓` in bats pretty output, `3 tests, 0 failures` | [AC-003-AC-004-AC-005-bats-gate](AC-003-AC-004-AC-005-bats-gate.webm) / [.gif](AC-003-AC-004-AC-005-bats-gate.gif) | webm+gif | 9.5s | 55KB / 101KB | recorded |
| AC-004 | S-21.09 | Guard PASSES `git add src/main.rs` on `develop` (non-`.factory/` path) — T-004, same recording as AC-003 | [AC-003-AC-004-AC-005-bats-gate](AC-003-AC-004-AC-005-bats-gate.webm) / [.gif](AC-003-AC-004-AC-005-bats-gate.gif) | webm+gif | 9.5s | 55KB / 101KB | recorded |
| AC-005 | S-21.09 | Guard PASSES `git add .factory/STATE.md` on `factory-artifacts` branch (state-manager's canonical workflow) — T-002, same recording as AC-003 | [AC-003-AC-004-AC-005-bats-gate](AC-003-AC-004-AC-005-bats-gate.webm) / [.gif](AC-003-AC-004-AC-005-bats-gate.gif) | webm+gif | 9.5s | 55KB / 101KB | recorded |
| AC-006 (happy path) | S-21.09 | `run_t012_gate(&workspace_root())` executed against the REAL production registries + real git-tracked WASM set — T-012 highlighted among `51 passed; 0 failed` in `bundle_orphan_check.rs` | [AC-006-T012-declared-tracked-parity-gate](AC-006-T012-declared-tracked-parity-gate.webm) / [.gif](AC-006-T012-declared-tracked-parity-gate.gif) | webm+gif | 9.4s | 50KB / 88KB | recorded |
| AC-006 (orphan fires) | S-21.09 | Core "gate fires on an orphan" behavior — T-015 fixture-driven negative control (isolated synthetic declared/tracked set, NOT the real repo) proves `check_declared_subset_tracked()` returns `Err` with a `MISSING: hook-plugins/hooks-only.wasm` identifier; source assertion (line 1941) shown first, then the test run live | [AC-006-T015-orphan-missing-classification](AC-006-T015-orphan-missing-classification.webm) / [.gif](AC-006-T015-orphan-missing-classification.gif) | webm+gif | 18.8s | 129KB / 216KB | recorded |
| AC-007 | S-21.09 | Pre-fix audit: satisfied by AC-006's T-012 `declared − tracked = ∅` subset assertion — same recording as AC-006 happy path (the subset check IS the "no other gaps" proof; no separate assertion needed) | [AC-006-T012-declared-tracked-parity-gate](AC-006-T012-declared-tracked-parity-gate.webm) / [.gif](AC-006-T012-declared-tracked-parity-gate.gif) | webm+gif | 9.4s | 50KB / 88KB | recorded |

---

## Recording Notes

- **AC-001** (`AC-001-artifact-tracked-and-declared`): two read-only commands against the
  real worktree — `git ls-files plugins/vsdd-factory/hook-plugins/validate-factory-path-staging.wasm`
  (confirms git-INDEX tracking, not just disk presence) then a `grep -n` of the production
  `hooks-registry.toml` (confirms the `plugin = "hook-plugins/validate-factory-path-staging.wasm"`
  declaration at line 947). Both piped through `sed 's/^/PREFIX: /'` so the VHS `Wait+Screen`
  match target is guaranteed to be command-output-only (VHS's `Wait` begins polling immediately
  after `Enter`, and the fully-typed command line — including any trailing text typed before
  Enter — is already on-screen; an un-prefixed marker risks a premature match against the
  as-yet-unexecuted command text rather than genuine output. All four tapes in this story use
  this output-only-marker discipline).
- **AC-002** (`AC-002-full-suite-zero-skips.txt`): captured log, not VHS — the full 36-test
  suite scrolling past is more legible as text than as a compressed GIF, and the meaningful
  claim (zero skip lines) is best verified with an explicit `grep -i skip` pass at the end of
  the capture, which the log includes. `CI_REQUIRE_ARTIFACTS=1` set; both required artifacts
  (`target/release/factory-dispatcher`, built via `cargo build --release -p factory-dispatcher`,
  and the now-committed WASM) present.
- **AC-003/AC-004/AC-005** (`AC-003-AC-004-AC-005-bats-gate`): one recording covers all three
  ACs — `bats` auto-detects the VHS pty as a TTY and uses its pretty/checkmark formatter
  (not TAP), so the recording shows human-readable `✓ T-001 ...` / `✓ T-002 ...` / `✓ T-004 ...`
  lines and a `3 tests, 0 failures` summary. `--filter 'T-001 |T-002 |T-004 '` narrows to
  exactly the three tests that map to these three ACs.
- **AC-006 happy path** (`AC-006-T012-declared-tracked-parity-gate`): runs the FULL
  `bundle_orphan_check.rs` suite (all 51 tests, release profile) piped through
  `grep -E 'T012_declared_set_subset_of_tracked_set|test result'` so the recording highlights
  the T-012 end-to-end gate specifically while still surfacing the whole-suite
  `test result: ok. 51 passed; 0 failed` line.
- **AC-006 orphan-fires** (`AC-006-T015-orphan-missing-classification`): per the recording
  brief, no destructive mutation of the real registries or git-tracked tree — T-015 builds
  its declared/tracked sets as in-memory `HashSet`s (29 filler hooks + `hooks-only.wasm`
  declared; `hooks-only.wasm` absent from the tracked set) and calls
  `check_declared_subset_tracked()` directly. The recording first `grep -n`'s the exact
  load-bearing assertion (`msg.contains("MISSING: hook-plugins/hooks-only.wasm")`, line 1941)
  out of the test source, then runs the test live to confirm the classification fires as
  asserted. This is the same `MISSING:`/`UNGATED-DECLARATION`/`OUTSIDE-REPO-DECLARATION`
  classification machinery `run_t012_gate` uses against the real tree in the AC-006 happy-path
  recording — T-015 exercises the `MISSING:` arm specifically (the T-012/T-015/T-050/T-051
  family referenced in the recording brief).
- All four `.tape` scripts are committed alongside their outputs for reproducibility.

---

## Toolchain

| Tool | Version | Status |
|------|---------|--------|
| VHS | 0.11.0 | installed |
| ffmpeg | 8.1.2 | installed |
| bats | 1.13.0 | installed |
| cargo/rustc | workspace-pinned toolchain | installed |

---

## PR Embedding Snippet

```markdown
### Demo Evidence — S-21.09

| AC | What it proves | Recording |
|----|-----------------|-----------|
| AC-001 | WASM tracked in git INDEX + declared in hooks-registry.toml | ![AC-001](docs/demo-evidence/S-21.09/AC-001-artifact-tracked-and-declared.gif) |
| AC-002 | Zero WASM/dispatcher-absent skips (36/36 tests execute) | [AC-002-full-suite-zero-skips.txt](docs/demo-evidence/S-21.09/AC-002-full-suite-zero-skips.txt) |
| AC-003/004/005 | Guard blocks `.factory/` on product branch, passes non-`.factory/` paths, passes `.factory/` on factory-artifacts | ![bats gate](docs/demo-evidence/S-21.09/AC-003-AC-004-AC-005-bats-gate.gif) |
| AC-006/AC-007 (happy path) | T-012 declared⊆tracked gate passes on real tree (51/51) | ![T-012](docs/demo-evidence/S-21.09/AC-006-T012-declared-tracked-parity-gate.gif) |
| AC-006 (orphan fires) | T-015 proves MISSING: classification fires on a declared-but-untracked artifact | ![T-015](docs/demo-evidence/S-21.09/AC-006-T015-orphan-missing-classification.gif) |
```

---

## Notes

- WebM is the primary format (best compression, GitHub supports playback); GIF is the
  PR-embeddable fallback. All recordings are well under the 2MB WebM / 5MB GIF targets
  (largest artifact: 216KB).
- Story remains code-unchanged at `12d0fe98` — this is an additive, evidence-only commit.
