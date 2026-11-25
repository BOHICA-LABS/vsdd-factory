---
kind: topic
id: prelude
package: B4a
sources: [pipeline.md#2.11, pipeline.md#2.11.1, packaging.md#2.5, BLUEPRINT.md#4.14]
---

# The stamped prelude

The stamped prelude: its markers, generated tables and constants, the argument parser and the platform
limits a script follows. The functions that run steps are [[topic:prelude-steps]]; the spike wrapper
is [[topic:prelude-run-spike]]; child stories are [[topic:prelude-run-stories]]; the convergence rule
as code is [[topic:prelude-converge]].

Shared workflow code, including `runStories()` and the argument parser, has one source,
`next/workflow-lib/prelude.js` in the repository: package B4a is built, so the file is the truth
and this page is a pointer at it. `tools/build.py` copies that file between the markers
`// >>> stamped:prelude` and `// <<< stamped:prelude` of every workflow, and
`tools/build.py --check` fails the build when any generated copy differs.

Inside the source, two marked sub-blocks are generated, never written by hand: the `TIER`, `ROLE`
and `OVERRIDE` tables from the roster ([[table:roster]], [[topic:model-tiers]]), and every `S_*`
schema literal from the files of [[topic:result-schemas]]. The provenance comment rides on the
closing marker, and nothing may follow the opening marker on its line ([[packaging:build-step]]).

The constraints on the prelude (declarations only; no clocks; the 400-line cap measured with the
generated sub-blocks stripped; the header comment listing every top-level name) are stated once, in
[[packaging:build-step]], which is authoritative for them.
