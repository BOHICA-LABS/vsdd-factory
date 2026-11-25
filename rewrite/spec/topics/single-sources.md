---
kind: topic
id: single-sources
sources: [BLUEPRINT.md#4.14, packaging.md#2.5]
---

# Single sources and generated copies

Section 4.14 of the blueprint is the authority on which copy of a shared fact is the single source. Its table:

| Fact | Single source | Generated or checked copies |
|---|---|---|
| Agents, skills, workflows, tiers, overrides, budgets | `tests/lint/roster.toml` | Agent frontmatter (lint); `TIER`, `ROLE` and `OVERRIDE` in the prelude (build) |
| Result schemas | `<plugin>/schemas/*.json` | Schema literals in the prelude (build); loaded by the tool and the hook runner |
| Hook wiring | `py/vsdd/hooks/registry.py` | `hooks/hooks.json` (build, checked in) |
| Shared workflow code, including `runStories()` and the argument parser | `workflow-lib/prelude.js` | The stamped region of every workflow (build) |
| Stage structure | The workflow scripts | None. The tool keeps no list of stages: the survey returns which stage names are recorded as done, and the script skips those. A harness test checks that every step names a method that its agent's file lists. The method of every step is [[table:step-methods]]. |
| Routes and workflow preconditions | An ordered list of (condition, next) and a list of named preconditions per workflow, in Python beside their condition functions | None; scripts receive `next`. Each row and each precondition has a fixture. |
| Artifact kinds and story kinds, path classes, walls with callers and reveals | The three policy tables | Read by the hook rules and `vsdd validate` |
| Convergence rule | `judge()` and `converge()` in the prelude | The tool's Python copy, held to it by shared fixtures run in Node and Python |
| The agents' shared block; the severity table | `workflow-lib/agent-shared.md`; `workflow-lib/severity.md` | Every agent file (build, checked byte for byte by lint) |

One row the packaging design adds (it calls the same build under the same `--check`):

- What each read command reveals: the class of each artifact kind in `artifacts.toml`. Generated copy: `[reveals]`
  in `walls.toml`.

`tools/build.py --check` fails the build when any generated copy differs ([[packaging:build-step]]).

When a stage counts as done, and what reopens one, is in [[topic:resume]].
