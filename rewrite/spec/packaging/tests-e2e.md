---
kind: packaging
id: tests-e2e
package: E1
sources: [packaging.md#layer-4-end-to-end-headless, BLUEPRINT.md#5.5]
---

# Test layer 4: end to end, headless

Each probe starts a real session:

```
claude -p "<prompt>" \
  --plugin-dir <repo>/plugins/vsdd-factory \
  --setting-sources project,local --strict-mcp-config \
  --allowedTools "Workflow Agent Skill Read Write Edit Bash Glob Grep" \
  --output-format json --max-budget-usd <cap> --no-session-persistence
```

It runs in a temporary copy of `tests/e2e/sample-project/` ([[packaging:sample-project]]) with a scenario overlay
from `tests/e2e/fixtures/` copied on top. `--setting-sources project,local` keeps the developer's own plugins, hooks
and servers out of the run. Assertions are made on the exit code, the JSON result (`is_error`, cost, turns), files
on disk, the sample project's git log and the hook runner's event log. No assertion reads model prose, except for a
fixed token that a prompt asks for.

A probe is a prompt, a fixture name, the allowed tools, a budget and a Python assertion over the result, the files,
the git log and the event log. Other subsystems add probes by adding files.

Results (cost, turns, agent starts per probe) are uploaded as a JSON artifact on every run. A probe that fails is
retried once; a second failure fails the job.

All probes run on cheap models through `VSDD_TEST_TIER`, which `vsdd survey` turns into a tier override
([[doc:conventions]]).

## The tiers

**Smoke** (package E1) — runs on every pull request that touches `plugins/`, `workflow-lib/` or `tests/e2e/`, and
weekly on macOS:

1. Inert: in a project that is not initialised, a command the gates would block is allowed, and nothing is written.
2. Guard: with `uv` hidden, a tool call is blocked with the `uv` message.
3. Briefing: in an initialised fixture, the session can repeat the exact next command its briefing gave.
4. Wall: a builder-tier agent is blocked from a holdout file and a reviewer-tier agent reads it.
5. Workflow: the smallest workflow runs with `Workflow` allowed and returns its structured result.
6. Tool: `vsdd --version` runs from the main session and from a workflow step ([[command:version]]).
7. Prompts: in the default permission mode, with `[hooks.rules.auto-allow] scope = "extended"` and no pre-allowed
   tools, one builder step (it edits a file, runs one test by name, runs `ls` and `git status`, commits and runs its
   gate) and one experiment spike requested by a step raise no permission prompt; with `scope = "core"` the count is
   recorded, not asserted. The count over a whole wave is taken in the greenfield proof
   ([[packaging:proof-greenfield]]).
8. Liveness: with hooks disabled, `vsdd survey` refuses to start a run, because its call has no caller marker.
9. Inert with a broken install: with the launcher file removed, a guarded tool call in a project that is not
   initialised is allowed and prints nothing, and the same call in an initialised project is blocked with the
   reinstall message.
10. Caller identity: two sessions in one project; each session's `vsdd survey` reads its own marker, and the lock
    records the session that called ([[topic:caller-identity]]).

**Story** (package E2, on demand, weekly and in the release workflow) — the story tier, with its kills, its
eleven-minute gate and its budgets: [[packaging:e2e-story-tier]].

**Conformance** (built from the spike plugin; weekly, against the newest Claude Code) — tiny workflows that assert,
on the real platform, each row of the harness table ([[packaging:tests-workflows]]), plus the items not yet proven
([[packaging:packaging-assumptions]]) and the platform facts the design rests on (the hook sees and can block a
step's result; the decide event; the hook permit; the command rewrite that carries the caller marker). It also
records the model ID behind each alias (`fable`, `sonnet`, `opus`) and triggers the story tier when one has changed
since the last run.

Delivered by package E1: the sample project, fixtures, probe runner, the smoke probes and the `e2e-smoke` job
([[package:E1]]). Which tiers CI runs where is [[packaging:ci]].

Alternative considered: `claude plugin eval`, the platform's eval runner. It gives isolation and graders for free,
but it caps a run at one hour, hides project configuration from the run, and has not been tried with workflows. The
spike proved plain `claude -p --plugin-dir`, so that is the harness. Evals are a reasonable later addition for one
narrow question, whether each model-invocable skill triggers on the prompts it should, and are not part of 1.0.

## Team use (decision D33; built when the build widens)

The team proof adds one run to the story tier, with two sessions in two clones; its scenarios on the tool
alone are a Python suite ([[packaging:proof-team]]).
