---
kind: topic
id: brownfield
sources: [BLUEPRINT.md#3.4, BLUEPRINT.md#4.13, pipeline.md#2.4, pipeline.md#2.9.8]
---

# Brownfield

Existing code and finished builds: the routes for each starting point and how inherited debt is kept
from blocking new work. The workflow that reads an existing codebase is [[workflow:ingest]]; the
baseline record is [[record:baseline]].

### A codebase the factory did not build

What runs: `/vsdd-factory:setup`, then `/vsdd-factory:ingest`. Ingest maps and analyses the code;
recovers contracts, architecture, conventions, the dependency list and the toolchain; measures a
baseline (which tests fail today, lint and format findings, mutation scores, scanner findings); and
ends at the decision `spec` on that recovered baseline.
Human decisions: one, approve the baseline.

### Any project with a baseline: after an ingest, an import or a release

What runs: `/vsdd-factory:feature "<request>"`. A bug fix, or a small change that needs no contract
change, goes straight to one story and then to `verify` in delta scope. A larger change goes through
a specification amendment, `plan` and waves. A request typed while a cycle is open is queued and
starts when that cycle closes.
Human decisions: the release; and the specification for a larger change.

### A project on the old plugin

What runs: `/vsdd-factory:setup` shows the import dry run and the person runs `vsdd import`. The
importer writes the baseline and an approved `spec` decision marked as imported, so `feature` can
start: the route names it at once when the importer queued a request for an epic in flight, and
otherwise the person starts it.
Human decisions: the import command.

### A codebase with no tests, no JUnit report or no CI

What runs: the same routes. The first run schedules chore stories in the standing epic `E-0` for the
missing test harness and CI before any other story. Delivery treats "this repository has no CI" as a
recorded fact (`delivery.ci = "none"`), so a missing check is never a pass by accident.
Human decisions: the first CI story touches `.github/**`, so it waits once for a merge approval.

### Several repositories

What runs: `/vsdd-factory:setup --hub` proposes the member list. Work runs from the hub with the
`repo` argument. A change that crosses repositories becomes one story per repository, joined by
dependencies.
Human decisions: as for one repository.

Inherited debt does not block new work. On a project with a baseline the measured gates compare
against it. A test that already failed, a lint or scanner finding that already existed, and a module
the cycle did not touch do not fail a gate; anything new does. `vsdd report debt`
([[command:report-debt]]) lists what was inherited, and `maintain` pays it down
([[workflow:maintain]]). Plan coverage and traceability apply to the contracts a cycle added or
changed, not to every recovered contract. Package E4 proves these routes end to end before cutover
([[package:E4]]).

The routes above are rows of the route table ([[table:routes]]); a project imported from the old
plugin starts at `feature`: the importer writes the baseline and an approved `spec` decision.

### Feature flags in existing code (decision D31; built when the build widens)

Ingest says whether the code already uses feature flags. Where it does, the factory proposes keeping
that system and never adds a second, and once a person confirms it the survey records the flags the code
already reads ([[topic:flag-system-choice]], [[topic:feature-flags]]).

### Teammates who commit by hand (decision D33; built when the build widens)

A team's repository also receives commits the factory did not make. They are noticed, listed and
reviewed, and never rewritten ([[topic:team-delivery]]).
