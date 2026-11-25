---
kind: package
id: G6b
depends_on: [G6a]
sources: [BLUEPRINT.md#9.2]
---

# G6b: Release gate
<!-- generated: start -->
## Delivers

`gate release` with its checks (`baseline`, `formal`, `perf`, `ui`) and `--only`

## Depends on

- [[package:G6a]]

As written in the build plan: G6a

## Implements

- [[command:gate-release]]: Runs the release checks as one job: baseline, formal, perf and ui.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
