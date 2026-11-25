---
kind: topic
id: resume
sources: [BLUEPRINT.md#4.14, pipeline.md#2.5, state.md#2.8]
---

# Resume

How workflows resume: when a stage is done, what the survey does on a new session or a fresh clone,
and how work is reopened. The shape that makes this possible is [[topic:workflow-shape]]; the stores
are [[record:closure]] and [[record:step]].

The script never reads state. The survey command does, and it is code. A stage is `done` when three
things hold: the unit's closure record says so, or, inside a run that has not reached its checkpoint,
a step record with status `done` exists for the stage and unit; the artifacts that result listed still
exist and pass their structural check; and any gate the stage must pass has a passing evidence record
for the current commit (the survey measures the gate again when only the evidence record is missing).

Refinement for a machine that has no local store:

- A stage is done when its closure entry says `done`, or, within a run that has not yet reached its
  checkpoint, when a done step record exists; and the artifacts that entry lists still exist and
  validate; and the gate it must pass has passing evidence for the current commit. There is no input
  digest: nothing reopens because an upstream file changed.
- When a closure names a gate that passed at the current commit and the evidence record is missing,
  the survey measures the gate again before deciding. It does not send the stage back to an agent.
- A stage is marked not done only explicitly. Settling `revise` or `hold` clears the entries from the
  stage the person named with `stage=` or, by default, from the entry named `review` onward. Opening
  an amendment clears the specification unit's entries the same way. An intake step's `covers` list is
  recorded as done entries for the stages it names, once their artifacts validate.
- The tool keeps no list of stages. It records the stage names the scripts report and returns the
  names that are done; the script decides what to skip.

Nothing is re-run because a digest of inputs changed: stages reopen only when a person's `revise` or
`hold`, or an amendment, says so. After approval a contract changes only through an amendment, whose
change analysis names the affected stories.

So a fresh clone resumes from the last checkpoint, and loses only the steps of a run that never
reached its checkpoint. In the same session the platform's own resume replays finished steps from its
cache, which is faster but is not relied on. A remote story branch with its pull request can be
adopted by any clone, which is what lets a fresh clone or a crashed session continue a delivery;
a remote story branch without one is owned elsewhere, and the unit is reported as blocked with the
reason `conflict` ([[topic:governed-delivery]]).

After any session clear or compaction, the session briefing and `vsdd status` restate the next
command ([[topic:compaction-and-cold-start]], [[topic:status-report]]); running the same workflow
command continues the work, because the survey starts from disk.

## Team use (decision D33; built when the build widens)

On a team the adoption rule of the paragraph on fresh clones gives way to the claim: a clone continues
a story it holds, from the pushed branch and the closure, and reads the tracked copy of the red record
where it has no local one ([[topic:team-records]]).
