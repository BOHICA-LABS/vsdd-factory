---
kind: topic
id: story-admission
sources: [BLUEPRINT.md#4.6, BLUEPRINT.md#5.1, pipeline.md#2.9.1]
---

# Story admission

How a story that does not come from the plan is admitted to a run by the tool.

Rule: every change to the product tree is a story. Steps that only measure are clerk steps and write
nothing but evidence. Plan readiness, verification harnesses and documentation fixes are therefore
chore, harness and docs stories in the standing epic `E-0`, with the same gates as any story
([[table:story-kinds]]).

A story that does not come from the plan (the one story of a fast route, a fix story from a gate, a
readiness chore) is admitted by the tool, not by the script: `vsdd gate plan --story`
([[command:gate-plan]]) and `vsdd story new-fix` ([[command:story-new-fix]]) check the story, record
it in the run and the cycle, create its branch and worktree under the run's lock, and return the
story unit ([[schema:story-unit]]) the child `story` workflow needs
([[workflow:story]]). Admission checks the story, records it in the run and in the cycle's closure
records, creates the branch `story/<id>` from its base ([[topic:worktrees]]) and the worktree. A parent may pass only a unit the tool
returned to this run, and every gate and delivery command re-checks it, so a garbled unit stops at
the first gate and delivers nothing.

Admission fixes every fast route and every fix round: stories written after the survey join the run
without a new survey, and the fast routes also write the readiness chore stories a project needs.
A change that crosses repositories becomes one story per repository, joined by `depends_on`
([[topic:multi-repo]]).
