---
kind: package
id: B4a
depends_on: [B0, C1, B6]
sources: [BLUEPRINT.md#9.1]
---

# B4a: Prelude build
<!-- generated: start -->
## Delivers

`workflow-lib/prelude.js` (with `runStories()`, the argument parser and the percent-encoder), `agent-shared.md`, `severity.md`; `tools/build.py` for the prelude, schema literals, the tier, role and override tables and the stamped agent text, with `--check`

## Depends on

- [[package:B0]]
- [[package:C1]]
- [[package:B6]]

As written in the build plan: B0, C1, B6

## Implements

- [[packaging:build-step]]: tools/build.py: what it generates, from what, where it lands, and the --check that fails on drift.
- [[topic:prelude]]: The stamped prelude: its markers, generated tables and constants, the argument parser and the platform limits a script follows.
- [[topic:prelude-converge]]: judge() and converge(): the single source of the convergence rule as code.
- [[topic:prelude-run-spike]]: runSpike(): the wrapper that runs a spike as a step of the same workflow and restarts the asking step.
- [[topic:prelude-run-stories]]: runStories(): how child stories are started and how budgets are shared between a parent and its children.
- [[topic:prelude-steps]]: The prelude functions that run a step: brief(), step(), gated() and relay().

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
