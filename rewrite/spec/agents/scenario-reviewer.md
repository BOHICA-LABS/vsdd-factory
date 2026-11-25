---
kind: agent
id: scenario-reviewer
package: A6b
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# scenario-reviewer

Reviewer. Reviews the hidden scenarios against the contracts they cite. It never reviews code, and its findings
go only to the product owner. It exists because its wall differs from every other agent's: it may read
`holdout`.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, SPAWN, RESULT.

What it deliberately lacks: no shell, no Write, no web. It returns findings only. The named sets are defined in
[[topic:tool-sets]].

## Is given / writes / returns

Is given: method `review-scenarios`; the scenario paths and the contract paths they cite; the items to account
for; claims from its own earlier pass.

Writes: nothing.

Returns in `data`: review data, as `adversary` ([[schema:review-data]]). Its findings quote scenarios, so they
are walled: they go only to the `product-owner`, in a loop of their own, and are stored in the holdout class.

Result schema: the common fields of [[schema:step-result]]. Reviewers return raw findings, never a pass or
fail decision.

## Walls

Read walls: `source`, `tests`, `ci`, `reviews`, `stories`, `analysis`, `spec-arch`, `spec-ux` are denied. It
reads `holdout` and `spec-product` to check scenarios against contracts, and nothing it could carry to a
builder ([[table:walls]]). A walled finding — one it returned, or one located under `.factory/holdout/` — has
its full form stored only in the `holdout-evaluations` class and is named outside it by its walled key.

## Methods

`review-scenarios` — the judging half of the scenario-freshness sweep; the sweep's two halves follow from the
wall, and are described with [[topic:method-files]].

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
