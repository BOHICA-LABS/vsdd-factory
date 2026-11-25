---
kind: skill
id: setup
package: K1
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, state.md#mounting]
---

# setup

Makes a project a factory project, mounts state on a fresh clone, or prepares the import of pre-1.0 state (it
runs `vsdd import --dry-run` and hands the person the import command, which only a person may run).

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "[--hub] [--repo]"
```

## What it does

Instructs the main session, in this order.

1. Ask the decision D4 question before anything is written: may the plugin approve its own routine tool calls,
   and at which scope (`off`, `core`, or `extended`, each described; `extended` is recommended for unattended
   runs). With `--hub`, also run `vsdd repos scan` and show the member repositories for the person to confirm.
2. Run `vsdd init --auto-allow <scope>` (and `--hub` with the confirmed list). `init` is the one moment the
   plugin writes those keys, which is why the questions come first. On a project that is already initialised,
   show the person the lines to add instead. On pre-1.0 state `init` converts nothing: run
   `vsdd import --dry-run`, show the report, and give the person the `vsdd import --auto-allow <scope>`
   command.
3. Show the detected configuration, any tracked file that falls in no path class, and which lines of
   `factory.toml` to edit if a value is wrong (no tool call can write that file).
4. Run `vsdd doctor` ([[command:doctor]]).
5. Offer, each with the person's confirmation: the terminal shim; the setting that disables Claude Code's
   commit attribution; moving build conventions out of the project's `CLAUDE.md`, by dispatching the
   `architect` with the method `conventions`, because the main session may not write `conventions.md`; and
   moving the sections of `CLAUDE.md` that describe the old process to `docs/archive/CLAUDE-pre-1.0.md`, which
   the main session edits and the person commits.

`vsdd init` (run by this skill) does one of three things at first mount: with no state branch anywhere it
creates the orphan branch with a first commit holding `factory.toml`, `notes.json` and `.gitignore`, and
mounts it; with a state branch on the remote or locally it mounts that; with pre-1.0 state it converts
nothing, reporting `legacy` (the mounting rules are the state design's).

## Team use (decision D33; built when the build widens)

On a clone that joins a project, `setup` shows the shared settings it found, the decision D4 scope among
them, and writes none. When several remotes of the clone hold the state it asks which one is the
repository the team shares, and it offers the machine's host name as the clone's label, which the
person may replace. It shows the notice `init` returns when it mounts state another clone wrote while
team use is off ([[table:team-checks]], T-JOIN). The passage that offers team use and shows the line to
add is delivered by package M5, once the team proof has passed ([[packaging:proof-team]]).
