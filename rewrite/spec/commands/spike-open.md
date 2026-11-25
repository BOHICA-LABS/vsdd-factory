---
kind: command
id: spike-open
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#6, pipeline.md#2.8]
---

# `vsdd spike open`

Opens a spike ledger entry after establishing who asked and checking the limits; with `--name` opens or
attaches to a named spike.

## Syntax

One of: `--run <id> --step <workflow>/<subject>/<name>.<attempt>` (inside a workflow); `--run <id>` (the
`spike` workflow); `--question-file <path>` (a dispatch with the Agent tool). Optional: `--name <name>` for
a named spike, `--ref <commit>`, `--asker-type <type>`.

## What it does

The "Open" step of the spike protocol ([[topic:spikes]]). "Takes the runner's agent id and working directory
from the caller marker. Takes the question and the asker's agent type from a hook-written record: the stored
`needs_spike` step record of the named step; the run record; or the admission the hook recorded for the
dispatch. `--asker-type` is a cross-check only, and a mismatch is refused. Refuses when no asker can be
established" ([[topic:caller-identity]], [[record:spike-admission]]).

Earlier findings. "It returns an earlier finding for the same question in the same unit (matched by a hash
of the normalised question) only when the earlier runner worked under walls at least as strict as this
asker's." Limits: "Enforces `per_run`, `per_day` and `max_open`"
([[config:spikes]]). "Otherwise it writes the ledger entry" ([[artifact:spike-ledger-entry]]).

Named spike. "With `--name`: creates the branch, the worktree and the two ledger files; or, if the named
entry is already open, attaches this runner's agent id to it, with no search for an earlier finding and no
count against `max_open`." Attaching "is never answered from an earlier finding" ([[topic:named-spikes]]).

## Who may call it

`spike-runner` — "how `spike open` knows which runner is calling". "Every input is an option or a file the
runner writes in the temporary directory; none reads standard input, so each is one command the hook can
inspect and approve."

## Writes

The ledger entry; with `--name`, the branch `spike/<name>`, the worktree `.worktrees/spike-<name>/` and the
two ledger files ([[topic:worktrees]]).

## Output and failure cases

The opened entry, or an earlier finding when the `read_deny` rule allows. Failure: no asker establishable,
a `--asker-type` mismatch, a limit reached ("a step that still needs an experiment at the limit becomes a
`blocked` decision with the reason `spike_limit`"). Needs an initialised project: "nothing could bound a
spike" outside one, so hooks are silent there.

## Team use (decision D33; built when the build widens)

A spike's ledger id is taken inside a sync ([[table:work-claim-checks]], T-MINT).
