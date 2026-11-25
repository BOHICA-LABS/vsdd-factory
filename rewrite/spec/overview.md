---
kind: doc
id: overview
sources: [BLUEPRINT.md#1, BLUEPRINT.md#3.1, BLUEPRINT.md#3.2]
---

# vsdd-factory 1.0: overview

## What 1.0 is

vsdd-factory 1.0 is a Claude Code plugin that takes a product from a brief to a released, verified change with as
little human effort as the work allows. It is rewritten from scratch as plain text: 20 agents, 13 skills,
12 JavaScript workflows, about 75 method files, one `hooks.json`, and one standard-library Python package that is
both the hook runner (17 rules) and a command-line tool named `vsdd` (about 48 subcommands). Nothing is compiled.

Three layers each do one job. Workflow scripts order the work. The `vsdd` tool measures facts (it runs the
project's own build and test commands, inspects git and computes review verdicts) and writes the evidence. Hooks
refuse any action the evidence does not support, and keep each agent inside its information wall.

The three layers: [[topic:trust-layers]]. What the tool measures: [[topic:gates]]. What it writes:
[[record:evidence]]. Walls: [[topic:information-walls]].

Project state lives in one directory, `.factory/`, on its own git branch. Code mounts, validates, commits and
pushes it, and no agent keeps books. Every workflow starts by asking the tool what is already done, so running the
same command again continues the work in any session.

State: [[topic:state-directory]], [[topic:state-commits]]. Asking what is done: [[command:survey]],
[[topic:resume]].

A person makes three routine decisions in a product cycle: approve the brief, approve the specification, and
authorise the release. Each is one typed command. Everything else runs unattended, and anything only a person can
settle is queued while other work continues. A review loop ends on one clean pass by a fresh reviewer, at each of
three widening perimeters (story, wave, release), with a cap of three passes, one arbitration, and then a person.
Any person, agent, skill or workflow can start a bounded research spike.

Decisions: [[topic:human-stops]], [[skill:decide]]. The review loop: [[topic:convergence]]. Spikes:
[[topic:spikes]].

The same pipeline serves existing code. A codebase the factory did not build is taken in once with `ingest`,
which ends with one decision of its own: approving the recovered baseline. After that, and after any release, a
change is one `feature` command. A bug fix or a small change goes straight to a story; a larger change goes
through a specification amendment first ([[topic:brownfield]]).

[[workflow:ingest]], [[workflow:feature]], [[topic:brownfield]].

## The parts

| Part | What it is | Its one job |
|---|---|---|
| Workflows | 12 JavaScript scripts in `<plugin>/workflows/`, run as `/vsdd-factory:<name>` | Order the work: sequence, fan out, loop, branch on result fields. A script cannot read files or run commands, so it is never trusted with truth. |
| Agents | 20 Markdown files in `<plugin>/agents/` | Judgment: write a specification, write code, find defects. An agent never reports whether its own work passed. |
| Method files | About 75 Markdown files in `<plugin>/methods/` | The procedure for one kind of task. An agent reads one only when its task names it, so it costs nothing until used. |
| The `vsdd` tool | One standard-library Python package in `<plugin>/py/vsdd/`, on the Bash tool's `PATH` through `<plugin>/bin/vsdd` | Measure and record: run the project's build and test commands, inspect git, compute verdicts, write evidence, merge, and keep project state. |
| The hook runner | The same Python package, started by `<plugin>/hooks/hooks.json` through a guarded shell launcher and `uv` | Enforce: refuse an action the evidence does not support, keep each agent inside its wall, record each step result, print the session briefing. |
| Project state | `.factory/` in each project, on the branch `factory-artifacts` | Hold specifications, stories, decisions and records. Code commits it. |
| Skills | 13 directories in `<plugin>/skills/` | Conversation with a person, one-off specialist jobs, and local tools. No skill re-implements a workflow. |

Where each part is defined:

- Workflows: [[topic:workflow-shape]], and one file each: [[workflow:ingest]], [[workflow:discover]],
  [[workflow:spec]], [[workflow:feature]], [[workflow:plan]], [[workflow:wave]], [[workflow:story]],
  [[workflow:verify]], [[workflow:release]], [[workflow:maintain]], [[workflow:multi]], [[workflow:spike]].
- Agents: [[table:roster]], [[topic:agent-file]], and one file per agent.
- Method files: [[topic:method-files]], [[table:methods]].
- The `vsdd` tool: [[topic:command-conventions]] and one file per command; [[packaging:python-package]],
  [[packaging:cli-entry]].
- The hook runner: [[topic:hook-runner]], [[topic:hook-launcher]], [[table:rules]] and one file per rule.
- Project state: [[topic:state-directory]], [[topic:state-layout]], [[table:artifacts]],
  [[topic:machine-records]].
- Skills: [[topic:skill-file]], and one file per skill.

The terms the design uses (unit, gate, evidence record, step, step record, clerk step, decision, perimeter,
integration branch, cycle, baseline) are in [[topic:glossary]].

## How the layers check each other

```
person ── types a command ──> main session (thin: holds a few hundred bytes per run)
                                   │ Workflow tool
                                   v
   workflow script ── step ──> agent ── tool calls ──> hook runner ── allow / block ──> files, git
        ^                        │                          │
        │ step result            │ runs `vsdd gate ...`     │ stores the step record
        │                        v                          v
        └────────────── `vsdd` tool: measures, writes evidence, merges, checkpoints ──> .factory/
```

At every point of no return (merge, closing a wave, opening a decision, publishing) the tool checks the evidence
again and ignores what the script believes. A fabricated step result can waste a pass. It cannot merge code.

## Where to read next

- [[topic:story-life]]: how one story travels from a request to a merged change.
- [[topic:brownfield]]: existing code and finished builds.
- [[topic:feature-flags]]: feature flags in the software the factory builds, opt-in per project (decision
  D31; built after the thin slice).
- [[topic:team-use]]: several people, each with their own clone, on one repository (decision D33; built
  after the thin slice).
- [[doc:decisions]] and [[topic:decisions-built]]: what was decided, and where each decision is built.
- [[doc:conventions]]: names, paths, model tiers and identifiers.
- [[doc:platform]]: what has been tested on the platform and what has not.
- [[packaging:build-plan]]: the order of the build, with one brief per package under `packages/`.
