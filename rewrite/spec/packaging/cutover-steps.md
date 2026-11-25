---
kind: packaging
id: cutover-steps
package: CO2
sources: [packaging.md#the-repository-and-the-marketplace, BLUEPRINT.md#2.3, BLUEPRINT.md#9.6]
---

# Cutover of the repository and the marketplace

Steps X0 to X8. Package CO2 depends on everything above and on E3 and E4 passed
([[package:CO2]], [[packaging:proof-greenfield]], [[packaging:proof-brownfield]]).

| Step | Who | What |
|---|---|---|
| X0 | builder | Rehearse. In `~/Dev/vantage` (its old state file is 109 bytes, so it is effectively empty), run `claude --plugin-dir <repo>/next/plugins/vsdd-factory`. Confirm the hooks stay inert on old state and the session starts cleanly with the stale `agent` setting present. Nothing is installed or changed. |
| X1 | person | Merge a pull request in `claude-mp` that changes the entry's `ref` from `main` to `v1.0.0-rc.25`. This is the same commit, so nothing changes for anyone, but installs no longer follow `main`. |
| X2 | person | Tag `archive/rc-line` at the tip of `origin/develop` and push it. |
| X3 | builder | The promotion commit on `rewrite/v1`: delete the old tree, `git mv next/* .`, delete `.github/workflows/next.yml`. Then `uv run python tools/check.py` must pass from the root ([[packaging:check-command]]). `git diff --stat -M` should show renames and deletions only. |
| X4 | person | Open the pull request to the integration branch. Update the required checks ([[packaging:ci]]). Merge. |
| X5 | person | Release 1.0.0 ([[packaging:plugin-release]]). The marketplace pull request also rewrites the entry's description. Merge it after `install-check` passes ([[packaging:install-check]]). |
| X6 | person | Collapse the branches (decision D25 in `BLUEPRINT.md#2.3`: fast-forward `main`, make it the default, delete `develop`), and close the open dependency pull requests and feature branches that target Rust code. |
| X7 | person, with the tool | Migrate the projects ([[packaging:cutover-projects]]). |
| X8 | builder | When the last project is done and `rewrite/STATE.md` says so, delete `rewrite/` in one commit. |

What the promotion commit deletes is listed in [[packaging:cutover-deletions]]; the archive tag of step X2 and the
marketplace pin of step X1 are what make it safe; neither may be skipped.

Decision D27 in `BLUEPRINT.md#2.3` settles the last step: run `tools/cutover.py plan`, pilot on `vantage`, migrate
all eight within two weeks, and add no legacy marketplace entry.
