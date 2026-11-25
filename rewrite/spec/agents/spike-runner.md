---
kind: agent
id: spike-runner
package: A3
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# spike-runner

Planner. Runs one spike in an isolated worktree and records the finding. New in 1.0.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]). One of the three roles
raised as a cost judgment call under the user's policy (open question 1 of the design). Its file alone sets
`isolation: worktree` in its frontmatter ([[topic:agent-file]]), and one agent omits the first bullet of the
second section of the shared block, because a spike cannot start a spike: `spike-runner`
([[topic:agent-shared-block]]).

## Tools

READ, EDIT, SHELL, RESULT, WEB, LOOKUP, DOCS.

What it deliberately lacks: no SPAWN: a spike cannot start a spike. The named sets are defined in
[[topic:tool-sets]]; how MCP tools are named in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: the question, why it blocks the asker, a suggested method, the starting commit, and its opening
command. Not who asked: the tool takes that from a hook-written record.

Writes: experiment files inside its own worktree, and the question or finding file in the temporary directory.
The ledger entry is written by `vsdd spike open` and `vsdd spike record`, not by a tool call.

Returns: the spike-finding schema of [[schema:spike-finding]], as its whole result: `ledger_id`, `question`,
`verdict` (`confirmed`, `refuted`, `answered`, `inconclusive`, `refused`), `answer`, `method`, `evidence`,
`confidence`, `cached`, `artifacts`, `follow_up`.

## Walls

Read walls: `holdout`, `reviews`, everything outside the project, and every class the asking agent may not
read; the runner inherits the asker's walls, recorded by `vsdd spike open` from a hook-written record, never
from the runner ([[table:walls]]). Desk research is a valid spike method: `spike-runner` (which has web and
lookup tools) chooses between an experiment, research, or both. Nothing leaves a spike except through the
pipeline: no push.

## Methods

`spike`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
