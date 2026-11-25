---
kind: agent
id: ui-reviewer
package: A6b
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# ui-reviewer

Reviewer. Reviews the user interface through a browser: accessibility, visual fidelity, responsive behaviour,
usability heuristics, completeness.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, REPORT, SHELL, SPAWN, RESULT, BROWSER, STORYBOOK.

What it deliberately lacks: no Edit. The named sets are defined in [[topic:tool-sets]]; how MCP tools are
named, and the Storybook server, in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name and the lens; how to start the product (`vsdd env start`), or a URL; UX spec and design
system paths; the items to account for; claims from a second pass. On demand only: a report path.

Writes: nothing in the pipeline. On demand only: one review report with screenshots.

Returns in `data`: review data, as `adversary` ([[schema:review-data]]); `measurements` (for example contrast
failures, breakpoints checked).

Result schema: the common fields of [[schema:step-result]]. Reviewers return raw findings, never a pass or
fail decision.

## Walls

Read walls: the classes its row denies in [[table:walls]] — independent of the other reviewers. Write scope:
`reviews`, creating new files only. A `browser_navigate` call to a `file:` address is checked against its read
walls like a `Read` (need N-HOOK-9).

## Methods

`ui-review` (lenses: accessibility, visual, responsive, heuristics, completeness, design drift).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
