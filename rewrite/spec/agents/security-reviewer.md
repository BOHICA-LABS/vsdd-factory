---
kind: agent
id: security-reviewer
package: A6a
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# security-reviewer

Reviewer. Reviews a diff or a system for security defects, triages scanner output, and classifies findings by
CWE and OWASP category.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, REPORT, SPAWN, RESULT, WEB, LOOKUP, DOCS.

What it deliberately lacks: no shell: scanners are run by the gate tool, and it reads their output. The named
sets are defined in [[topic:tool-sets]]; how MCP tools are named in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; the diff patch or module list; scanner output paths; architecture path; claims from a
second pass. On demand only: a report path.

Writes: only what the wall row in [[table:walls]] grants. On demand only: one review report.

Returns in `data`: review data, as `adversary` ([[schema:review-data]]), with a CWE identifier in each
finding's `title`.

Result schema: the common fields of [[schema:step-result]]. Reviewers return raw findings, never a pass or
fail decision.

## Walls

Read walls: `holdout` and `reviews` — independent of the other reviewers. Write scope: the grants of the row
in [[table:walls]]. Refusals in security work are a recorded risk: a step that returns no result must count
as a failed step and reach a person, never as a clean review ([[topic:risks-agents]]).

## Methods

`security-review` (a diff, a system, or the triage of scanner and audit output).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. On a story that builds the product's flag system its review accounts for one
more item, `flag-system`. Its duty and what holds it to the duty: its row of [[table:flag-duties]].
