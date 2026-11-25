---
kind: agent
id: adversary
package: A6a
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# adversary

Reviewer. Attacks specs, stories and implementations against the spec with fresh context, and checks
brownfield extraction against the code.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, REPORT, SPAWN, RESULT.

What it deliberately lacks: no shell, no Edit, no web. Read-only by tool list, except for its one report file.
The named sets are defined in [[topic:tool-sets]].

## Is given / writes / returns

Is given: method name (the lens); the tree root and target paths; evidence paths (the latest diff patch, test
output) for code; scope; the items to account for; the project's review rules file; from a second pass,
structured claims to verify. Never an earlier review and never a builder's summary. On demand only: a report
path.

Writes: nothing in the pipeline. On demand only: one review report at the path it was given.

Returns in `data`: review data ([[schema:review-data]]): `reviewed_sha`; `findings` (each with `key`, `title`,
`severity`, `category`, `scope`, `owner`, `location`, `spec_ref`, `repro`, `observed`, `expected`);
`claim_checks` (`key`, `outcome`, `note`); `coverage` (`item`, `state`, `note`). A defect outside the scope it
was given is a finding whose `scope` names the level that should handle it.

Result schema: the common fields of [[schema:step-result]]. Reviewers return raw findings, never a pass or
fail decision: whether a pass is clean is arithmetic in the workflow script.

## Walls

Read walls: the classes its row denies in [[table:walls]] — fresh context: it never sees an earlier pass, its
own or anyone's. Write scope: `reviews`, creating new files only, in an on-demand review. Reviewer files
carry the severity table and the finding key rule, stamped from one source ([[topic:agent-shared-block]]).

## Methods

`review-specs`, `review-stories`, `review-story-diff`, `review-wave`, `review-system`, `review-extraction`,
`score-ideas`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. For a story that names a feature flag its review accounts for more items, the
first of them new code reachable with the flag off. Its duty, the text of each item and what holds it to
the duty: its row of [[table:flag-duties]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Where a review is given items `hand:<12 hex>`, each comes with the path of
that commit's patch, written by the gate or the survey that listed the commit. The adversary accounts
for each: it reads the patch against the contracts of the modules it touches, reports defects as
findings like any other, and returns `blocked` with kind `spec_gap` when the commit changes behaviour a
contract fixes or adds behaviour no contract covers. The verdict holds it to an answer for every item
([[table:team-delivery-checks]], T-ITEMS, T-GAP).
