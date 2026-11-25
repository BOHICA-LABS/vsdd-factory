---
kind: workflow
id: spike
package: W1
sources: [pipeline.md#2.3, pipeline.md#2.8, pipeline.md#2.9.12, BLUEPRINT.md#4.5, BLUEPRINT.md#6]
---

# `spike`

Runs one research spike for a person, or opens, advances or closes a named spike.

Arguments: `question`, `kind`, `ref`, or `open` / `close` with a name; `repo`. Preconditions: none
beyond the common one, and it takes no lock. It does need an initialised project: hooks are silent
before setup, so no bound could be enforced there, and the workflow answers "run
`/vsdd-factory:setup` first" ([[skill:setup]]). Because it takes no lock, it is available while any
other workflow runs. The spike protocol, its limits and the walls the runner inherits are defined in
[[topic:spikes]]; the wrapper that serves spike requests inside a workflow is
[[topic:prelude-run-spike]].

## Calls

| Call | Steps | Returns |
|---|---|---|
| `{question, kind?, ref?}`, or the text `<question>` | A survey that stores the question in the run record (it takes no lock), then one `runSpike()` call | The spike finding |
| `{open: name, question}`, or the text `open <name> -- <mission>` | The same survey; then one spike-runner step whose first command is `vsdd spike open --name <name> --run <id>` and whose last is `vsdd spike record --increment`. No clerk opens the entry | The ledger's current state and next action |
| `{close: name}`, or the text `close <name>` | One spike-runner step that writes the final finding and runs `vsdd spike close --name <name>` | The final finding |

The workflow needs an initialised project and says so when there is none.

## Notes

The asker recorded for the `spike` workflow is the main session; the survey stores the question it
was given as `--question-uri` ([[command:spike-open]]). The user's own `resume-spike` convention keeps
working in any repository and can resume a named spike, because the branch, worktree and ledger file
names are the same ([[topic:named-spikes]]). Named spikes are listed in the session briefing.

There is no spike skill: the slash command is this workflow, which calls the same `runSpike()`
function as the wrapper. The main session on its own judgment dispatches the spike agent directly —
in the platform probe the model never started a workflow unprompted.
