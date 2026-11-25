---
kind: package
id: A6a
depends_on: [A1, C1]
sources: [BLUEPRINT.md#9.4]
---

# A6a: Code-facing reviewers
<!-- generated: start -->
## Delivers

`adversary`, `code-reviewer`, `security-reviewer` and their methods

## Depends on

- [[package:A1]]
- [[package:C1]]

As written in the build plan: A1, C1

## Implements

- [[agent:adversary]]: Reviewer. Attacks specifications, stories and implementations against the specification with fresh context.
- [[agent:code-reviewer]]: Reviewer. Reviews a diff with no access to specifications or factory state.
- [[agent:security-reviewer]]: Reviewer. Security review, triage of scanner output.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
