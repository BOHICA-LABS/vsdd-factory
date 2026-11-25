---
kind: skill
id: status
package: K1
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, state.md#2.10]
---

# status

Shows the phase, work in flight, open spikes and health problems, the command that fixes each problem, and the
exact next command.

Invoked by: person and model — the one model-visible skill.

Frontmatter beyond `name` and `description`:

```
argument-hint: "[--fix]"
```

## What it does

Instructs the main session to run `vsdd status` with the given arguments and relay the output; when a person
states a standing instruction, to save it with `vsdd note` ([[command:note]]). Its behaviour is the state
design's status-and-resume entry point: `/vsdd-factory:status [--fix]` is safe at any time, in any session, and
running it twice changes nothing. Plain `status` writes nothing; `--fix` repairs (it releases a dead lock,
removes expired worktrees, clones a missing member repository, commits pending state) and is the in-session
form of every repair a message names.

Because `status` is the only skill without `disable-model-invocation: true`, its description may run to at
most 400 characters; the model-visible budget is one skill ([[table:roster]]). The command itself is owned by
the state design ([[command:status]]); this skill only relays it.

## Team use (decision D33; built when the build widens)

With team use on, `--fix` also syncs, and the report gains the work claims and the outcome of the last
sync ([[command:status]], [[topic:team-sync]]).
