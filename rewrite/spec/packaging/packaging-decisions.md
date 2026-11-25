---
kind: packaging
id: packaging-decisions
sources: [packaging.md#2.2, packaging.md#2.1]
---

# The packaging design's decisions

Each with its reason in one line, as the packaging design states them:

| # | Decision | Why, in one line |
|---|---|---|
| D1 | The plugin stays at `plugins/vsdd-factory/` and stays a `git-subdir` source. | The marketplace path does not change, and tests, tools and docs are not copied to users' machines. |
| D2 | During the rewrite the whole new tree is built under a staging root, `next/`, that mirrors the final repository root. | Every new top-level name (`tests/`, `docs/`, `README.md`, the CI files) collides with an old one. One staging root means one mechanical promotion commit and no path edits. |
| D3 | The root `CLAUDE.md` is replaced on the rewrite branch first, before any building. | It is 47 KB of superseded process rules, and it is loaded into every builder and reviewer the rewrite starts ([[packaging:repo-claude-md]]). |
| D4 | Generated content is committed and checked for drift. There is no install-time build. | A marketplace install copies the git tree. Nothing runs at install. |
| D5 | All shipped Python is one package, launched through one guarded `sh` launcher. All supporting tools are one command, `vsdd`. | One guard, one interpreter pin, one name on `PATH`, one place to look. |
| D6 | Test helpers are Python (pytest, via `uv`) and Node built-ins only. No npm packages, no `package.json`. | Workflow scripts are JavaScript and need a JavaScript engine. Nothing else justifies a third toolchain. |
| D7 | A release is an immutable tag, and the marketplace entry pins that tag and its commit. | The marketplace pull request becomes the one switch that publishes, and reverting it is the rollback. |
| D8 | The tag is `vsdd-factory--v<version>`, the platform's own format. | `claude plugin tag` validates it, and plugin dependency resolution looks for it. |
| D9 | No release branch, no bot commit, no back-merge, no guardrail workflow. | With nothing compiled there is nothing to commit after the tag. |
| D10 | The minimum Claude Code version is the version the platform spike ran on, 2.1.331. | Every platform fact the design relies on was verified there and nowhere lower. |
| D11 | `userConfig` holds only per-user secrets (three API keys). Everything per-project lives in the project's own configuration file. | One home for each kind of setting. Keys go to the system credential store instead of plaintext `.mcp.json` files. |
| D12 | The eight projects keep project scope and are migrated one at a time by an idempotent tool that plans before it acts. | No change to which projects load the plugin, and a failed step can simply be re-run. |

These are the packaging design's own D-numbers, distinct from the blueprint's decisions in `BLUEPRINT.md#2`.

## Facts found on disk that change the brief

Checked on 2026-11-23. They are the grounding for the decisions above and for the cutover
([[packaging:cutover-projects]], [[packaging:plugin-manifest]]):

- The marketplace repository is `bohica-labs/claude-mp`, not `drbothen/claude-mp`. The plugin remote is
  `BOHICA-LABS/vsdd-factory`. A rebrand commit moved both. The marketplace entry's URL already says `bohica-labs`.
  **Consequence:** all URLs in 1.0 use `bohica-labs`. GitHub redirects the old names.
- Tag `v1.0.0-rc.25`, `origin/main` and the installed commit (`51023185`) are the same commit. **Consequence:** the
  marketplace can be pinned to the tag with no change in content (cutover step X1).
- `origin/main` is an ancestor of `origin/develop` (17 commits behind, 0 ahead). **Consequence:** the two branches
  can be collapsed into one by a fast-forward.
- The marketplace entry is `git-subdir`, `path: plugins/vsdd-factory`, `ref: main`. `plugin.json`'s `version` takes
  precedence over the entry's. **Consequence:** today, whatever is on `main` is what `claude plugin update`
  installs, and the marketplace pull request only changes the number shown in the catalog. 1.0 reverses that
  ([[packaging:plugin-release]]).
- All eight installs are project-scoped and share one cache directory, `…/vsdd-factory/1.0.0-rc.25/`. `claude-mp`
  has auto-update off (the default for third-party marketplaces). **Consequence:** nothing updates until someone
  runs `claude plugin update` in a project; but see risk 6 on cache cleanup ([[topic:risks-packaging]]).
- Every one of the eight has `"agent": "vsdd-factory:orchestrator:orchestrator"` in `.claude/settings.local.json`,
  written by the old `activate` skill. **Consequence:** if 1.0 has no agent of that name, the setting dangles. The
  cutover removes it.
- Every one of the eight has its own project `.mcp.json`. `prism` and `pregolya` have `CLAUDE.md` files of about 600
  lines with 67 to 81 mentions of old process terms. `pregolya` has a project `SessionStart` hook that calls
  `.factory/hooks/ensure-heartbeat.sh`. **Consequence:** the cutover tool reports this residue. It does not edit a
  project's `CLAUDE.md`.
- `~/.config/vsdd-factory/watched-factories` exists and lists five project paths for the observability stack.
  **Consequence:** the 1.0 observability command keeps this file and its format, so nothing has to be re-registered
  ([[packaging:observability-stack]]).
- `claude plugin validate --strict --profile directory` on the spike plugin asks for: an `author`, a
  `network.hosts` declaration (it cannot read hosts out of hooks or skills), and an icon (square PNG or JPEG, 512 to
  2048 px, under 2 MB, not SVG). It takes the README and licence from the repository root when the plugin folder has
  none. **Consequence:** the plugin folder gets its own README and licence because only that folder is installed
  ([[packaging:plugin-manifest]], [[packaging:repo-layout]]).
- The platform documents `engines` (needs Claude Code 2.1.323 or later), `network` (2.1.328 or later), a `bin/`
  folder that is added to the Bash tool's `PATH`, `claude plugin tag` (creates a `<name>--v<version>` tag after
  checking the manifest), `claude plugin eval`, and a 14-day cleanup of superseded cache directories.
  **Consequence:** used throughout the design; each one that the spike did not exercise is listed in
  [[packaging:packaging-assumptions]].
