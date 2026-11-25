---
kind: agent
id: verifier
package: A4
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# verifier

Builder. Builds and runs verification harnesses (proofs, fuzzing, mutation testing, security scans,
benchmarks, clone fidelity) and reports measurements.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]). Raised as a cost judgment
call: its tools, not its judgment, give the verdict (open question 1 of the design).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, DOCS.

The named sets are defined in [[topic:tool-sets]]; how MCP tools are named in
[[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; the story and worktree paths of the harness story; property identifiers; thresholds from
project configuration.

Writes: harness files and tests; commits. It measures nothing itself: the checks run as `vsdd gate` jobs.

Returns in `data`: the green gate result in `gates`; `harnesses` (property or target, path); `not_buildable`
(with the reason).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `verifier` row of [[table:walls]] (`holdout` and `reviews` are
denied: it verifies independently, not led by what a reviewer already found).

## Methods

`proofs`, `fuzzing`, `mutation`, `security-scan`, `performance`, `clone-fidelity`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
