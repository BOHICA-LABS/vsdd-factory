---
kind: topic
id: amendments
sources: [BLUEPRINT.md#4.11, pipeline.md#2.6, pipeline.md#2.9.4]
---

# Amendments

A specification gap found during a wave: the amendment request, the amendment run and what follows
its approval. The `spec-gap` decision kind itself is in [[topic:human-stops]].

A specification gap is always found while a cycle is open, so the amendment cannot be a queued
request. When a person settles a `spec-gap` decision with `amend`, the settle function writes an
amendment request for the open cycle (`requests/CR-NNN.md` with `cycle` and `amendment = true`,
[[artifact:change-request]]). The stories that wait on the gap stay blocked. When nothing else in the
open wave can run, the route returns `feature` with `amend: true` ([[table:routes]]).

That run is not queued: its survey takes every `spec-gap` decision settled `amend` in the cycle as
its request, so one run covers the wave's gaps, holds the amendment that unlocks the specification
set for the planners, and ends at decision `spec`. When the amended specification is approved, `plan`
runs in delta scope: the story-writer re-traces the stories the change analysis names, the plan gate
runs again, and the waiting stories are released into the open or a trailing wave.

Inside the `feature` run with `amend: true`, the survey opens nothing: it runs inside the open cycle,
takes every `spec-gap` decision settled `amend` as its request, and marks the run as holding the
amendment ([[workflow:feature]]). An amendment run unlocks the whole specification set for the
planner roles, and the next `spec` decision approves the amended set. The approved specification set
is locked again from that approval ([[topic:human-stops]]).
