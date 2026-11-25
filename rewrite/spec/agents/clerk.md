---
kind: agent
id: clerk
package: A1
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# clerk

Clerk. Runs one `vsdd` command and returns its JSON output unchanged. While the output says `pending` it runs
the same command again.

## Tier and effort

clerk tier — `model: sonnet`, `effort: low` ([[topic:model-tiers]], [[table:roster]]). The only agent on its
tier; the lint allows low effort for the clerk tier only.

## Tools

SHELL, RESULT.

What it deliberately lacks: everything else. The hook runner lets it run nothing but `vsdd`. The named sets are
defined in [[topic:tool-sets]]. Its file carries a two-line shared block of its own
([[topic:agent-shared-block]]).

## Is given / writes / returns

Is given: one `vsdd` command.

Writes: nothing.

Returns: the command's JSON, unchanged.

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls: the classes its row denies in [[table:walls]]; it runs commands and reads nothing. Like every
agent with no shell or a `vsdd`-only shell, it may not read outside the project, the plugin and the temporary
directory.
It may not dispatch anything. It runs no method: its task is the command.

## Methods

None: its task is the command.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
