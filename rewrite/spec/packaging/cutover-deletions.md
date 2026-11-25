---
kind: packaging
id: cutover-deletions
package: CO2
sources: [packaging.md#deleted-at-cutover-and-kept, BLUEPRINT.md#2.3]
---

# Deleted at cutover, and kept

Executed by the promotion commit at cutover step X3 ([[packaging:cutover-steps]]). Decision D30 in
`BLUEPRINT.md#2.3` settles the two largest deletions: `docs/demo-evidence` (110 MB) and the 364 KB changelog are
deleted from the tree; both remain in history and under the tag `archive/rc-line`.

| Item | Size | Action |
|---|---|---|
| `crates/` (55 crates), `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `Cross.toml`, `deny.toml` | about 195,000 lines of Rust | Deleted. Nothing is compiled in 1.0. |
| Old `plugins/vsdd-factory/` in full, including `hooks/dispatcher/bin/` and `hook-plugins/` | 102 MB, 79 MB of it binaries | Deleted and replaced by `next/plugins/vsdd-factory/`. Content the other designs keep is ported, not moved. |
| `plugins/vsdd-factory/tests/` and root `tests/` | 2,279 bats tests | Deleted. The cases for rules and tools that port are harvested first ([[packaging:tests-python]]). |
| `scripts/`, `ci/`, `benches/` | 7 scripts, 1 YAML, 1 script | Deleted. What each became is in the old-system mapping of the packaging design, kept under `rewrite/history/`. |
| `.github/workflows/` `ci.yml`, `kani.yml`, `release.yml`, `release-branch-guardrail.yml`; `.github/release-notes/` | 2,100 lines | Deleted and replaced. `semgrep.yml` is kept with its branch names updated. |
| `docs/demo-evidence/` | 110 MB, 1,313 files | Deleted (D30). |
| `docs/guide/` (30 files), `docs/lessons-learned/`, three top-level docs | 476 KB | Deleted and replaced by the 1.0 set ([[packaging:documentation]]). |
| `CHANGELOG.md` | 364 KB | Replaced by a new file that starts at 1.0.0 with one line pointing at tag `v1.0.0-rc.25` for history. |
| `README.md`, `CONTRIBUTING.md`, `RELEASING.md`, `CLAUDE.md` | | Rewritten. |
| `.claude/settings.json`, `.claude/scheduled_tasks.lock`, `.claude/pr-reviews/`, `.lazyclaude/` (a gitlink with no `.gitmodules`) | | Deleted. They enable the old plugin for this repository or are stray session files. |
| `.mcp.json` at the root (untracked, holds live keys) | | Not in git. Delete it locally once the keys are in the plugin configuration. Never copy it into the plugin. |
| `LICENSE`, `.semgrep.yml`, git history, all `v1.0.0-rc.*` tags | | Kept. |
| `origin/factory-artifacts` branch | | Kept as a read-only archive. No job mounts it. |
| `rewrite/` | | Kept until the last project is migrated, because it holds the resume record. Then deleted in one commit (step X8). |

Before the promotion commit, an annotated tag `archive/rc-line` is put on the tip of `origin/develop`, so the last
state of the old source has a permanent name.
