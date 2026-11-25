---
kind: packaging
id: proof-brownfield
package: E4
sources: [packaging.md#layer-4-end-to-end-headless, BLUEPRINT.md#9.6, BLUEPRINT.md#5.5, BLUEPRINT.md#3.4]
---

# The brownfield proof (E4)

**Four runs** (on the sample project and its fixture overlays, [[packaging:sample-project]]):

1. `tally` with its tests and one known lint failure and no factory state: `setup`, `ingest`, approval of the
   recovered baseline, a standard-route feature through to release, then a bug fix.
2. The same package with no tests and no CI: the first run schedules the harness and CI chore stories.
3. An import fixture cut from `vantage` or `opnrynkr`: `vsdd import`, then one feature.
4. A two-repository hub with a change that crosses both.

**Asserts:**

- the inherited lint failure does not fail any gate, and a new one does; `verify` runs in delta scope;
- plan coverage asks for no story for an untouched recovered contract;
- a fast-route story is admitted and delivered;
- after the import the route is `feature` with the request the importer queued for an epic in flight (steady state
  if it queued none);
- the cross-repository change yields one story per repository and the wave closes when both repositories' parts
  close.

These are the routes `BLUEPRINT.md#3.4` states for existing code and finished builds — a codebase the factory did
not build, a project whose first build is finished, a project on the old plugin, a codebase with no tests or CI, and
several repositories. Inherited debt does not block new work: on a project with a baseline the measured gates
compare against it; a test that already failed, a lint or scanner finding that already existed, and a module the
cycle did not touch do not fail a gate; anything new does. Plan coverage and traceability apply to the contracts a
cycle added or changed, not to every recovered contract. Package E4 proves these routes end to end before cutover
([[packaging:build-plan]]).

The user asked whether the factory covers adding features to an existing project and to a project whose first build
is finished. E3's last step and E4 are the proof that it does, and they are why the cutover of the eight real
projects, which are all brownfield, is not gated on a greenfield run alone ([[packaging:proof-greenfield]],
[[packaging:cutover-projects]]).

Delivered by package E4, which depends on E3, W9, W12 and S5 ([[package:E4]]). Cutover waits for E3 and E4.

Decision D33, built when the build widens: a repository with a remote also receives commits made by
hand. That case is proved in [[packaging:proof-team]], in its scenarios of commits made by hand, and none
of the four runs above changes.
