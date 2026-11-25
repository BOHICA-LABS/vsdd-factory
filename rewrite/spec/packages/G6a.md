---
kind: package
id: G6a
depends_on: [G2, G5]
sources: [BLUEPRINT.md#9.2]
---

# G6a: Wave gate
<!-- generated: start -->
## Delivers

`gate wave`: the integration worktree per repository, mutation, clone fidelity, settle, rotation, the UI check, fix units

## Depends on

- [[package:G2]]
- [[package:G5]]

As written in the build plan: G2, G5

## Implements

- [[command:gate-wave]]: Runs every due wave check on the integration head as one job: suite, lint, build, mutation, clone fidelity and UI.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
