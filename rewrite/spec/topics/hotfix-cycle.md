---
kind: topic
id: hotfix-cycle
sources: [BLUEPRINT.md#4.11, BLUEPRINT.md#4.5, pipeline.md#2.4, pipeline.md#2.5, pipeline.md#2.6]
---

# Hotfix cycle

A critical bug fix as a second cycle beside the main one: its branch, its lock, its next and the
merge back.

A critical bug fix may open a second cycle while the main one is open. Its `next` carries `cycle` in
`args`, and `verify`, `release` and `feature` accept `cycle`. Where both cycles hold a decision with
the same identifier, each is written `<id>@<cycle>` (for example `release@hotfix-1.2.1`); the
briefing prints that form, and a bare identifier that is ambiguous is refused with the list of
choices ([[topic:decisions]]).

Its stories are cut from the release tag and delivered to the branch `hotfix/<version>`. When it is
released, the merge back into the integration branch waits for the integration lock; a conflict opens
a `merge` decision.

Concurrency: a critical bug fix takes a separate lock because it works from the release tag, on the
branch `hotfix/<version>` ([[topic:run-lock]]). The route table row for a second cycle that has work
that can run names that cycle's next workflow, with `cycle` in `args` ([[table:routes]]). Required
`verify` checks for the `hotfix` route: baseline and one adversarial pass ([[workflow:verify]]).
