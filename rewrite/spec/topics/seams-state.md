---
kind: topic
id: seams-state
sources: [state.md#4, state.md#5, state.md#2.18]
---

# Interfaces the state design provides and needs

Every interface the state design stated is now defined by the thing that owns it. This file is the map; it
defines nothing itself.

## What this design provides

- The state directory, its storage and its five conditions — [[topic:state-directory]].
- The layout of `.factory/`, tracked and local — [[topic:state-layout]].
- The registry content, the path classes, the identifier schemes and the flat frontmatter subset —
  [[table:artifacts]], [[table:path-classes]], [[table:identifiers]], [[topic:artifact-rules]].
- The record formats — the status object ([[topic:status-report]], computed and not stored), and the
  machine records: closure, decision, run, step, evidence, job and driver
  ([[record:closure]], [[record:decision]], [[record:run]], [[record:step]], [[record:evidence]],
  [[record:job]], [[record:driver-flag]], [[topic:machine-records]]).
- `factory.toml`: its path, its schema, the sections it hosts for other designs —
  [[topic:project-configuration]] and the `config` files.
- The state commands — `[[command:survey]]`, `[[command:checkpoint]]`, `[[command:status]]` and the rest,
  one file each under `commands/` — and the state functions in `<plugin>/py/vsdd/state/`: `detect`,
  `status`, `briefing`, `flush`, `digest`, `verdict`, `caller`, `lock_seen`, `lock_release`, `store_step`,
  `settle_decision`, `evidence`, `spike_entry`, `spike_admit`, `spike_attach`, `story_kind`,
  `kind_has_red_gate`, `spec_locked`, `artifact_locked`, `guidance_locked`, `unit_tree`. Their signatures
  and contracts are carried by [[topic:state-api]], and their behaviours by
  [[topic:state-directory]], [[topic:status-report]], [[topic:briefing]], [[topic:state-commits]],
  [[topic:digests]], [[topic:verdicts]], [[topic:run-lock]], [[topic:spec-lock]], [[topic:evidence-lookup]],
  [[topic:caller-identity]].
- Skeletons in `<plugin>/templates/<kind>.md` — [[topic:templates]].

## Answers to the needs the sibling designs stated

- N-STATE-1 (agents-skills): the class table and the seven product path keys — [[table:path-classes]],
  [[config:paths]].
- N-STATE-2: one state directory, the same absolute path from every worktree — [[topic:state-directory]].
- N-STATE-3: on-demand review reports and evaluations — [[artifact:review-report]],
  [[record:holdout-evaluation]].
- N-STATE-4: the spike ledger and deferred items — [[artifact:spike-ledger-entry]],
  [[artifact:deferred-item]]; no lessons register, a retrospective is a research report
  ([[artifact:research-report]]).
- N-STATE-5: `.factory/specs/architecture/review-rules.md` — [[artifact:review-rules]].
- N-STATE-6: no artifact asks an agent to keep an index, a count or a version citation —
  [[topic:stored-and-derived]].
- N-STATE-7: `status` behaviour and initialisation — [[command:status]], [[topic:state-directory]];
  templates — [[topic:templates]].
- N-STATE-8: the state directory is excluded from the product checkout — [[topic:state-directory]].
- Hooks S1 to S9: the configuration file ([[topic:project-configuration]]), the local store and its classes
  ([[topic:state-layout]], [[table:path-classes]]), the briefing ([[topic:briefing]]), the registry as data
  ([[table:artifacts]]), the run lock and liveness ([[topic:run-lock]]), `store_step` and
  `settle_decision` ([[record:step]], [[record:decision]]), flush on `PreCompact`
  ([[topic:state-commits]]), and the single state instance ([[topic:state-directory]]).
- Pipeline S1 to S11: the hosted configuration sections, survey and checkpoint, the step, evidence and
  decision stores, the spike ledger, the run lock, `vsdd status --json` and the briefing, the story
  frontmatter keys, and where demo media lives — [[topic:project-configuration]], [[command:survey]],
  [[command:checkpoint]], [[record:step]], [[topic:evidence-lookup]], [[record:decision]],
  [[artifact:spike-ledger-entry]], [[topic:run-lock]], [[topic:status-report]], [[artifact:story]],
  [[artifact:demo-evidence]].
- Packaging S1 to S8: `vsdd.state.detect`, `factory.toml` and the event-log path, the registry as data, the
  run lock, the briefing, the push rule, the importer, and the templates directory —
  [[topic:state-directory]], [[topic:project-configuration]], [[table:artifacts]], [[topic:run-lock]],
  [[topic:briefing]], [[topic:state-commits]], [[topic:importer]], [[topic:templates]].

## Seams that need settling

Places where a sibling design's stated assumption and this design differ:

- Hooks: a story's `wave` is a frontmatter key, and stories match `.factory/stories/*.md` — the wave is in
  the plan's closure record ([[topic:waves]]) and stories are under `stories/E-<N>/` ([[artifact:story]]).
  The example table in the hook design's registry changes; the rule does not.
- Hooks: an earlier draft tested conditions in a status record and read `evidence/<story>/red.json` — there
  is no status record ([[topic:status-report]]); unit and wave state are read from the closure records, a
  settled decision from its decision record, and evidence is one record per gate and commit at
  `local/evidence/<subject>/<gate>.<sha12>.json` ([[topic:evidence-lookup]]).
- Agents-skills: `evidence` assumed at `.factory/evidence/**` — it is `.factory/local/evidence/**`,
  untracked ([[topic:state-layout]]). `holdout` is as assumed.
- Agents-skills: the story-writer writes a wave plan — it writes `depends_on` and `priority`; `vsdd plan
  schedule` computes the waves ([[topic:waves]]).
- Agents-skills: the spike runner writes its ledger entry — it does, by running `vsdd spike open` and
  `vsdd spike record`, not with the Write tool ([[artifact:spike-ledger-entry]]).
- Pipeline: the tool is named `factory` — `vsdd`, per the packaging design ([[topic:command-conventions]]).
- Pipeline: step records decide whether a stage is done — closures decide across sessions and machines;
  step records decide only inside a run ([[record:closure]], [[topic:resume]]).
- Packaging: `vsdd stale` reads stored input hashes — there is no `vsdd stale` and there are no stored
  input hashes ([[topic:stored-and-derived]]). `vsdd lock release` operates the run lock
  ([[topic:run-lock]]). Templates are in `<plugin>/templates/` ([[topic:templates]]). There is no research
  cache: the dated reports under `research/` are what a researcher reads first ([[artifact:research-report]]).

Withdrawn needs: hook need H-1 was withdrawn by the blueprint (flush runs on `PreCompact` and inside `vsdd`
commands only), and pipeline need P-1 was withdrawn (there is no stage registry).
