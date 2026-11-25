---
kind: topic
id: state-api
sources: [state.md#4, state.md#2.8, hooks.md#5]
---

# The state API

The functions the state design provides in `<plugin>/py/vsdd/state/` and the hook design imports
(need S7). The two designs build to one signature each, and each returns within one second except
`flush`: `detect`, `status`, `briefing`, `flush`, `digest`, `verdict`, `caller`, `lock_seen`,
`lock_release`, and the store functions below.

Two of them resolve the shared names other files define: `vsdd.project.path_class(path)` resolves a
path to one of the seven product classes with the precedence of [[table:path-classes]]; the stores'
paths and caps are [[record:step]], [[record:evidence]], [[record:decision]] and the spike ledger,
and `vsdd.state.verdict(project, subject)` is computed from the stored passes of any run for the
unit, so a converged review still outlives the run that produced it; it always returns an object —
`failed` with `no_result` when no pass is stored, never None — and "no pass is stored" is tested as
`summary(project, subject)` being None.

`store_step(project, result, agent_type, head)` writes the step record at the path of [[record:step]]
with the calling agent's type and the worktree head, refuses one over its cap, and writes the full
form of any walled finding to the evaluation file. `settle_decision(project, id, choice, values,
note, session_id)` writes the decision record and nothing else (and, for `spec-gap amend`, the
amendment request); it raises with a reason when the decision is not open, the id is ambiguous, the
choice is not one of its kind's, or the digest of what is being approved no longer matches.
`evidence(project, gate, subject)` returns the passing record by the per-gate rule of
[[record:evidence]] (for `red`, the latest passing or re-freeze record on any commit of the story
branch since its base), or nothing. `spike_entry(project, agent_id, worktree)` returns the open ledger
entry for that runner, matched by agent id first, with `opened`, `verdict`, `asker_type` and
`read_deny`. `spike_admit(project, session_id, dispatcher_type)` records an admission, and
`spike_attach(project, agent_id, cwd)` pairs a starting runner with it. `story_kind(project, story)`
reads the story's `kind`, and `kind_has_red_gate(kind)` reads the story-kind table.
`spec_locked(project)` is true when the latest `spec` decision is approved and no live run record
holds an amendment; it takes no path, because the lock covers the whole specification set.
`artifact_locked(project, path)` is true for an accepted decision record, a proven property and a done
story. `guidance_locked(project)` is true while a cycle is open and no live run holds an amendment; it
governs `conventions.md` and `review-rules.md`. `unit_tree(project, workflow, subject)` returns the
root that a unit's finding paths are relative to. `caller(key)` reads the caller marker for a `vsdd`
call ([[record:caller-marker]], [[topic:caller-identity]]). `lock_seen` and `lock_release` serve the
hook runner's liveness duty ([[topic:run-lock]]). When `lock_release` cannot end a run in time it
keeps the entry and raises `StoreBusy` whose `released` attribute carries the entries it did release
and whose `kept` carries the ones it held back; these attribute names are the seam's contract, and
the session-end caller logs the `released` entries as `lock.released`
(finding step3-records-fix-3-C1-2). The story's wave is in
`cycles/<cycle>/units/plan.json`, and a wave's gate status is in its closure record,
`units/wave.<W-NN>.json`.

Used by [[rule:step-results]] (`store_step`, `unit_tree`), [[rule:decisions]] (`settle_decision`),
[[rule:tests-first]] (`evidence`, `story_kind`, `kind_has_red_gate`), [[rule:artifacts]]
(`spec_locked`, `artifact_locked`, `guidance_locked`) and [[rule:spike-bounds]] (`spike_entry`,
`spike_admit`, `spike_attach`).
