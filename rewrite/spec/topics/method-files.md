---
kind: topic
id: method-files
sources: [agents-skills.md#2.8, BLUEPRINT.md#4.4]
---

# Method files

A method file is the procedure for one kind of task: when to use it, the inputs, the steps, the output and its
template, and the checks to make before returning. It is a plain Markdown file
`<plugin>/methods/<name>.md`, at most 150 lines. A workflow step names it in the step prompt, and a forked
skill names it in the skill body, so each procedure has one source. Method files replace the bodies of the old
skills, their 148 supporting step files, the nine orchestrator sequence files, the ten rule files and most of
each old agent file.

Why files and not skills: a skill the model can load must be model-visible, and every model-visible skill is
paid for by every agent start. A method file is paid for only by the agent that reads it.

Every method file and the agent that owns it is the table [[table:methods]]; the method of every workflow step
is [[table:step-methods]].

## The pipeline design's provisional names

The pipeline design was written with provisional agent names. Every workflow uses roster names, and each step
names a method file. Its table of provisional names, the roster agent that replaces each, and the method that
carries the work now, is the `replaced-name` rows of [[table:methods]].

## One step, one method

`step()` takes exactly one method name, and universal test 6 checks that the method file exists and that the
step's agent file lists it. [[table:step-methods]] gives the method of every step of every workflow, and
it is the table the workflow scripts are written from. There is no `release` method: the release is one
command of the tool, and its notes use `release-notes`.

The scenario-freshness sweep of `maintain` has two halves because of the wall. The evaluator re-runs the
scenarios with `holdout-eval`; it may not read contracts, so it cannot judge whether a scenario is stale or
whether the contracts are still covered. The `scenario-reviewer`, which may read both, judges that with
`review-scenarios`.

The build may merge two method files that are always read together. It may not let one grow past the limit.
