---
kind: package
id: A6b
depends_on: [A1, C1]
sources: [BLUEPRINT.md#9.4]
---

# A6b: Product-facing reviewers
<!-- generated: start -->
## Delivers

`holdout-evaluator`, `ui-reviewer`, `scenario-reviewer` and their methods

## Depends on

- [[package:A1]]
- [[package:C1]]

As written in the build plan: A1, C1

## Implements

- [[agent:holdout-evaluator]]: Reviewer. Scores the running product against hidden scenarios without seeing source, tests or specifications.
- [[agent:scenario-reviewer]]: Reviewer. Reviews hidden scenarios against the contracts; its output never reaches a builder.
- [[agent:ui-reviewer]]: Reviewer. Reviews the interface through a browser.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
