---
kind: package
id: G4a
depends_on: [G2, G3, S3]
sources: [BLUEPRINT.md#9.2]
---

# G4a: Delivery
<!-- generated: start -->
## Delivers

`story deliver`: the verdict check, push, pull request with the story trailer, CI wait, patch id, merge with the head-commit match, adoption, local mode, `delivery.ci`; the `gh` test double

## Depends on

- [[package:G2]]
- [[package:G3]]
- [[package:S3]]

As written in the build plan: G2, G3, S3

## Implements

- [[command:story-deliver]]: The only merge onto the integration branch: verdict check, push, pull request, CI wait, merge and worktree removal.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
