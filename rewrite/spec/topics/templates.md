---
kind: topic
id: templates
sources: [state.md#2.13]
---

# Skeleton templates

A template in the old system was a full sample document that an agent copied, and that a hook then compared
the output against. In 1.0 the registry is the schema and code enforces it ([[table:artifacts]]), so what
remains of a template is a skeleton: the frontmatter keys with their starting values and the required
headings, each with one line saying what belongs under it.

Skeletons live in `<plugin>/templates/<kind>.md`, one per agent-written kind, about 25 files of at most 40
lines. A lint test checks each skeleton against its registry entry, so the two cannot drift. Method files
cite them by that path, and `vsdd new <kind>` ([[command:new]]) stamps one with the next free identifier.

## What becomes of the 136 old templates

- A sample of an agent-written artifact (brief, domain spec, contract, property, architecture, decision
  record, UX files, epic, story, scenario, evaluation, research, lesson, deferred item, assessments,
  conventions, release notes) — 38 old templates. In 1.0: about 25 skeletons, because several old templates
  described one kind.
- A report format (adversarial, code, security and consistency reviews, findings, drift, fuzz, formal,
  performance, fidelity, readiness, sweep, session review, and the four UI quality reports) — 25. In 1.0:
  one skeleton, `review.md`. The structure of findings is the pipeline design's result schema
  ([[topic:result-schemas]]).
- An index, a matrix, a trajectory, a manifest, a schedule or a pull request description — 18. In 1.0: not
  stored. Each is a derived view ([[topic:stored-and-derived]]).
- A file code now writes (state, wave state, red-gate log, findings tracker, demo evidence report) — 5. In
  1.0: machine records ([[topic:machine-records]]).
- A configuration file (policies, merge, autonomy, discovery, project manifest, reference manifest) — 6.
  In 1.0: sections of `factory.toml` ([[topic:project-configuration]]), written by `vsdd init`.
- Something copied into the product (design-system starter of 23 files, demo tape, demo browser test, demo
  CI job, justfile) — 27. In 1.0: files under `<plugin>/assets/`: `design-system/`, `demo/` and
  `scaffold/`. No skill copies them. Method files cite them by path, and the lint rule L-TEMPLATE checks
  that every cited path exists.
- A prompt or an authoring guide (three adversary prompts, agent and skill authoring templates) — 7. In
  1.0: method files and contributor documentation, owned by the agents-skills design.
- Bookkeeping with nothing left to describe (burst log, session checkpoints, state-manager checklist,
  resolved blockers, spec changelog, test-vector supplement, property withdrawal, the two multi-repo state
  templates, the commit-id currency script) — 10. In 1.0: dropped.

The lint test that compares each skeleton in `<plugin>/templates/` with its registry entry is a need the
packaging design states (its K-5); see [[topic:seams-state]].
