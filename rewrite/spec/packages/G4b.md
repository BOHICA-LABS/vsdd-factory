---
kind: package
id: G4b
depends_on: [G4a]
sources: [BLUEPRINT.md#9.2]
---

# G4b: Governed delivery and fix stories
<!-- generated: start -->
## Delivers

Protected paths, dependency detection and approved dependencies, `merge` decisions, the hotfix branch, `story new-fix` with admission

## Depends on

- [[package:G4a]]

As written in the build plan: G4a

## Implements

- [[command:story-new-fix]]: Writes fix stories in E-0 from blocking findings and admits each with its worktree.
- [[topic:governed-delivery]]: When a delivered story waits for a person: protected paths, dependency changes, the merge decision, and the hotfix branch.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
