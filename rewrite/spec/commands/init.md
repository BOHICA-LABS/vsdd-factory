---
kind: command
id: init
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#mounting, state.md#2.12, pipeline.md#2.9.11]
---

# `vsdd init`

Creates or mounts the state branch and writes `factory.toml` by detection; converts nothing on pre-1.0
state.

## Syntax

`vsdd init [--fresh] [--auto-allow <scope>] [--hub]`

## What it does

"Creates or mounts the state branch, writes `factory.toml` by detection (with the D4 answer and, for a hub,
the `[[repo]]` entries that `setup` collected), `notes.json`, `.gitignore` and the two exclusion lines. On
pre-1.0 state it converts nothing: it reports `legacy` and prints the dry-run command and the import
command. `--fresh` needs the person."

Mounting, one of three things: "With no state branch anywhere it creates the orphan branch with a first
commit holding `factory.toml`, `notes.json` and `.gitignore`, and mounts it. With a state branch on the
remote or locally it mounts that. With pre-1.0 state it converts nothing." It writes `.factory/` and
`.worktrees/` into `.git/info/exclude`, and the state branch's own `.gitignore` contains `local/`.

Detection. "Init writes the first version by detection (`Cargo.toml`, `package.json` scripts,
`pyproject.toml`, `go.mod`, a `Makefile` or `justfile`, the current branch names, the release configuration,
whether there is a remote and whether there is CI) and then runs each detected command once to see that it
starts. It detects the globs of the seven product classes and lists any tracked file that falls in none. It
detects a JUnit report for the test command (nextest, `pytest --junitxml`, jest-junit, go-junit-report). It
checks that `commands.run`, when set, takes its port from `{port}` or `$VSDD_PORT`. A command it cannot
detect is left empty and listed in the briefing." Missing commands are a state, not an error. It also lists
every file those trial runs left untracked and not ignored, because a story gate refuses a worktree that
holds one ([[topic:gates]]).

Hub. "Init does not guess a hub. `setup --hub` runs `vsdd repos scan`, shows the repositories it found, and
passes the confirmed list to `vsdd init --hub`, which writes the `[[repo]]` entries when it creates
`factory.toml`."

## Who may call it

"Main session in `setup` for `init` and `import --dry-run`; `init --fresh` and the real `import` are run by
the person." `--auto-allow <scope>` and `--hub` "carry the answers `setup` collected before it ran `init`,
because `init` is the only moment the plugin writes those keys."

## Writes

`factory.toml` ([[config:project]]), `notes.json` ([[record:notes]]), `.gitignore` and the two exclusion
lines; with `--fresh` it renames the old branch to `factory-artifacts-rc` and starts a new one.

## Exit behaviour and failure cases

One of the commands that work outside an initialised project ([[topic:command-conventions]]). On pre-1.0
state it reports `legacy` and prints the dry-run and import commands; it never converts. State exists but
is not mounted is reported by `vsdd doctor` and `setup`, not repaired here.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. `vsdd init` finds the state branch on whatever remote holds it and, when
more than one does, mounts nothing until `--remote <name>` says which is the repository the team
shares. On a clone that did not create the state it joins the team when team use is on there, giving
the clone its record and the label of `--label`; otherwise it mounts as it does today and returns a
notice that another clone wrote this state ([[table:team-checks]], T-JOIN; [[record:clone]]).
