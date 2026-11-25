---
kind: topic
id: hooks-inert
sources: [hooks.md#2.8, BLUEPRINT.md#2.2]
---

# Staying inert

How the hooks stay silent in a project that is not initialised, and what they do on pre-1.0 state.

`find-root.sh` decides, in shell and without starting any process:

1. Start at `CLAUDE_PROJECT_DIR` (the project root where the session started), or the working directory
   if that is unset.
2. At each directory, test for `.factory/factory.toml`. If it exists, that directory is the project
   root.
3. If the directory's `.git` is a file that points into `<main>/.git/worktrees/`, also test the main
   checkout. This covers a session started inside a linked worktree.
4. Otherwise move to the parent. Stop at the user's home directory or the filesystem root, so a stray
   file there can never turn every project into a factory project.

If no root is found the launcher exits 0 immediately: no uv, no Python, no output, no file created.
The old dispatcher needed a "mount gate" to avoid creating `.factory/logs` in unrelated projects; here
nothing is written before the root is known. The Python side receives the root in `VSDD_FACTORY_ROOT`
as the session's project. For each call it then resolves the governing project from the path being
touched (see [[topic:rule-model]]). A multi-repository project has one state directory, in its hub
([[topic:multi-repo]]): a member repository cloned under the hub has no state of its own, and walking
up from a path inside it reaches the hub's state, which governs it. The one case not covered is a
session started outside any factory project that reaches into one: the launcher has already exited.

## The five conditions

A project is in one of the five conditions that `vsdd.state.detect` returns; [[topic:state-directory]]
is the one definition, and this table is the same five as the hook runner meets them. The first three
reconcile two requirements in `rewrite/STATE.md` that pull against each other ("hooks do nothing in a
project that has not been initialised" and "the briefing says whether the project is initialised");
decision D24 confirmed that reading.

| Condition (`detect` value) | How it is recognised | Behaviour |
|---|---|---|
| Not a factory project (`none`) | Neither `.factory/factory.toml` nor `.factory/STATE.md` exists | Completely inert and silent. |
| Pre-1.0 state (`legacy`) | `.factory/STATE.md` exists and `.factory/factory.toml` does not | Inert: no gate runs. On `SessionStart` only, the launcher prints one fixed line naming the migration step. No Python starts. |
| State exists but is not mounted (`unmounted`) | `.factory/` is absent and the state branch exists locally or on the remote | Inert and silent, because there is no configuration file to find. `vsdd doctor` and `setup` report it. |
| Partial (`partial`) | `.factory/factory.toml` exists and the mount test fails: `.factory/` is not a worktree on the state branch. An invalid file is the loader's report, not a condition | All gates run. State writes are blocked ([[rule:write-scopes]] check 7). The briefing says so and names `/vsdd-factory:status --fix`. |
| Ready (`ready`) | `.factory/factory.toml` exists and the mount test passes. An invalid file is reported by the loader; the mount test itself reads no configuration | Normal operation. |

An unknown `schema` value in the configuration is not a sixth condition: it is an invalid
configuration, and the rules run on their built-in defaults (see [[topic:hook-failure]]).
