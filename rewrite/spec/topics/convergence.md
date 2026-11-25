---
kind: topic
id: convergence
sources: [pipeline.md#2.7, BLUEPRINT.md#5.1]
---

# Convergence

The review loop and the convergence rule: blocking finding, clean pass, converged, the loop, the cap,
arbitration, and checks that are not reviews. The rule is one function used at every perimeter; as
code it is `judge()` and `converge()` in the prelude ([[topic:prelude-converge]]).

Severity. Reviewers assign one of four levels, defined so that a level can be checked against
evidence. The four definitions are one table, [[table:severity]]; its text is the file
`workflow-lib/severity.md`, which the build stamps into every reviewer file, so the byte-for-byte lint
has one source.

## Blocking finding

A finding blocks only when all of these hold. The script applies the first three and the hook or tool
applies the fourth.

1. Its severity is `critical` or `high`.
2. It is anchored: it cites the contract clause or acceptance criterion it violates (`spec_ref`), or
   gives a command or scenario that shows the defect (`repro`). A severe finding with no anchor is
   advisory.
3. Its scope is within reach of the perimeter. A story review reaches `unit` findings. A wave reaches
   `unit` and `integration`. Release reaches those and `system`. A finding beyond reach is deferred to
   the perimeter that can judge it and does not block here. A finding whose scope is `spec` cannot be
   fixed by a builder and becomes a `spec-gap` decision.
4. It is grounded: the file exists, the line range is inside it and the quoted text is found there. An
   ungrounded finding is rejected when the reviewer returns it.

Category `holdout` findings — built from scenario scores ([[topic:holdout-findings]]) — are exempt
from the location condition and from grounding.

## Clean pass

A pass is clean when every reviewer in it returned a result for the current commit; every item the
reviewers were told to account for (each acceptance criterion at a story, each changed file after a
repair, each contract at the specification) is marked `checked`, or `not_checkable` with a reason when
another evidence route covers it; there is no blocking finding (an item left `skipped` is itself a
finding of category `coverage`, routed to the test-writer, so the fixer is called and the loop can
end); and every claim from the previous pass ("this was fixed", "this is disputed") was checked and
came back resolved or withdrawn.

## Converged

A unit has converged when its latest pass is clean and nothing has changed since. One clean pass is
enough. It must be the last word: any later commit that touches source or tests needs a scoped pass
over the new commits before the unit counts as reviewed.

## The loop

1. Run a pass with fresh reviewers. From the second pass they receive the list of claims to verify
   and nothing else from earlier passes. A claim is structured data only: the finding key, the path
   and line range of its location (never the quote), the resolution (`fixed` or `disputed`) and the
   commit. A claim about a finding of category `holdout` carries the key alone. The fixer's free text
   stays in the step record for the arbiter and is never shown to a reviewer.
2. If the pass is clean, stop: converged.
3. Otherwise hand the blocking findings to the fixer. For each one the fixer either fixes it or
   disputes it with evidence. Advisory findings are handed over once, to fix where cheap, and never
   extend the loop.
4. Repeat until the cap (three passes at story and specification, three rounds at wave and release,
   two at plan and ingest).
5. At the cap, an arbiter rules on each open finding: `defect`, `not_defect` or `spec_gap`. The
   arbiter is the architect at the planner tier, a third role that neither wrote the code nor raised
   the finding. A `not_defect` ruling dismisses the finding only if the builder disputed it, and
   never if it is `critical` or in the security category. Defects that remain get one more fix and
   one more pass. The number of passes and rounds used is stored in the unit's closure record, so
   the cap counts across runs.
6. If findings are still open, the unit is blocked and a `finding` or `spec-gap` decision is opened
   ([[topic:human-stops]]). The pipeline carries on with other units.

At the specification perimeter there is no arbiter, because a person is about to read the packet
anyway: findings still open at the cap are listed in it as open issues with both positions.

Why it terminates. The loop runs at most cap plus one passes, each pass is a fixed number of steps,
and each step is bounded by the attempt and spike limits. A story review therefore uses at most 4
passes, 4 fixes and one arbitration.

Why one clean pass is enough. The design gets at least three independent fresh looks at every change
before release, one at each perimeter, each with a wider view: the story review against its
contracts, the wave review of how stories fit together, and the system review in `verify`. Stories in
critical or high modules get two lenses in every story pass (adversary and security reviewer).
Repeating a look at the same perimeter mostly re-finds the same things; widening the view finds
different ones.

## How a finding can leave the blocking list

By a fix that the next reviewer confirms. By a dispute that the next reviewer accepts (it withdraws
the finding). By an arbiter's `not_defect` ruling, which needs the builder's dispute first, so two
independent agents have disagreed with the reviewer. By a person's waiver. There is no other way, and
every dismissal is listed in the release packet.

## Checks that are not reviews

At the wave and release perimeters a round also contains measured checks (suite, mutation score,
scans, proofs, scenario scores). A failing check produces findings in the same shape, so one loop
handles both. The hidden-scenario thresholds and the mutation thresholds by module criticality are
configuration keys: `holdout.*` and `mutation.kill_rate` in [[table:config-keys]], which holds each
default.

## Walled findings

A finding returned by the `scenario-reviewer`, or one whose location lies under `.factory/holdout/`,
is walled: see [[topic:walled-findings]].
