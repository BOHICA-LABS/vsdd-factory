---
kind: packaging
id: cutover-projects
package: CO1
sources: [packaging.md#the-eight-projects, BLUEPRINT.md#2.3]
---

# Cutover of the eight projects

Cutover step X7 ([[packaging:cutover-steps]]). `tools/cutover.py` lives in this repository and is not shipped in the
plugin; it is needed eight times on one machine. It reads
`~/.claude/plugins/installed_plugins.json`, so it finds the installs itself.

`uv run python tools/cutover.py plan` changes nothing and prints one row per project:

- the installed version and scope;
- the old state: branch, size of `STATE.md`, uncommitted changes, unpushed commits;
- story worktrees, and how many have uncommitted work (`vantage` has 98 worktrees, `prism` has 10);
- the stale `agent` setting;
- residue it will not touch: old process terms in the project's `CLAUDE.md`, project hooks that call into
  `.factory/`, and servers in the project's `.mcp.json` that the plugin now bundles;
- whether the project is already done.

`uv run python tools/cutover.py apply <project>` does the mechanical part. It is idempotent, so a failed run is
simply run again:

1. Stop, with a list, if the old state has uncommitted changes or any story worktree has uncommitted work. In-flight
   work is finished or abandoned on rc.25 first. `--force` overrides.
2. Back up `.claude/settings.local.json` beside itself and remove the `agent` key from it.
3. Run `claude plugin update vsdd-factory@claude-mp --scope project` in the project, and confirm 1.0.0 is recorded.
4. Print the residue report and the one command to run next in a session there: `/vsdd-factory:setup`, which shows
   the import dry run and then gives the person the `vsdd import` command ([[topic:importer]]). Where the project's
   `CLAUDE.md` holds old process sections (`prism` and `pregolya` have about 600 lines of them), the report says so
   and points at the offer `setup` makes: with the person's confirmation, those sections move to
   `docs/archive/CLAUDE-pre-1.0.md`, so the session that drives the pipeline is not loaded with rules for agents and
   commands that no longer exist. The cutover tool itself still does not edit a project's `CLAUDE.md`.

`apply <project> --retire` instead uninstalls the plugin for that project and removes the two settings. It leaves
the project's old state alone.

The safety property that makes this low-risk is one the hooks and state designs provide: **a project that has
pre-1.0 state and no 1.0 state is treated as not initialised**. The hooks do nothing there except add one line to
the session briefing that names the migration step ([[topic:hooks-inert]]). Then a project that reaches 1.0 by
accident, for example through a fresh clone, is inert and says why.

Order, from the facts found on disk ([[packaging:packaging-decisions]]): `vantage` first as the pilot, then
`opnrynkr` (14 KB of old state), `vitrand`, `secops-factory`, `prism`, `hylo`, then `flowloom` (281 KB) and
`pregolya` (342 KB) last. All eight should be done within fourteen days of the first, for the reason in risk 6
([[topic:risks-packaging]]).

Things a person will see once and should expect: a prompt for the three optional API keys the first time the plugin
is enabled (they are stored per user, so once covers all eight), and an approval prompt per project for each bundled
server.

Not offered by default: a second marketplace entry, `vsdd-factory-legacy`, pinned to the rc.25 tag, for a project
that must stay on the old line for a long time. The platform allows an entry name that differs from the manifest
name. It is a fallback to add only if decision D27 is answered that way.

Other machines. Anyone else on a release candidate follows `docs/migrating-from-rc.md`: remove the `agent` setting,
update the plugin, run the import or initialise command. The briefing line above points them there.

Delivered by package CO1: `tools/cutover.py` and `docs/migrating-from-rc.md` ([[package:CO1]]).
