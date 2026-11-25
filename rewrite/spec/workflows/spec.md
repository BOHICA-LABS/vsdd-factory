---
kind: workflow
id: spec
package: W4
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.3, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `spec`

Turns an approved brief into the specification set and reviews it to convergence.

Arguments: `sources` (paths of existing documents to take in), `guidance`, `variant`, `only` (stage
names), `repo`. Preconditions: a cycle is open; decision `brief` is approved
([[table:preconditions]]). Stages run in the order listed, except that assessments, scenarios and ux
run in parallel once the architecture is settled.

## Stages in order

### survey

- Step and agent: clerk.
- Produces: survey; preconditions: a cycle is open and decision `brief` is approved. When the latest
  `spec` decision is approved (an earlier cycle's, an ingest's or an import's), the survey marks the
  run as holding an amendment, as `vsdd feature route` does for the standard route, so the planners
  can write the set that approval locks.

### intake (only with `sources`)

- Step and agent: product-owner, and architect for architecture documents (planner), method `intake`.
- Produces: existing documents named by the `sources` argument, converted into registry artifacts.
  The step returns `data.covers`, the names of the later stages whose output it filled.
- Done when: artifacts valid. The checkpoint, and within the run the survey's done test, record each
  stage in `covers` as done once its artifacts validate, so those stages are skipped and nothing that
  was converted is regenerated.

Intake of existing documents. A person who has a requirements document or an architecture document
elsewhere passes its path in the `sources` argument, and the `intake` stage converts it into registry
artifacts and reports which later stages it covered. Documents a person has placed by hand in the 1.0
layout are not picked up silently: nothing records them as done, so they are passed through `sources`
like any other document, and `intake` validates them where they sit. References between artifacts are
checked for existence by `vsdd gate spec` and at approval, not when a draft is written, so contracts
may name subsystems that the architecture stage has yet to write. CI setup, toolchain preflight and
service-clone creation move to `plan` and to ordinary stories.

### domain

- Step and agent: product-owner (planner), method `domain-spec`.
- Produces: domain specification — capabilities, entities, invariants, edge cases, assumptions,
  risks, failure modes.
- Done when: artifacts valid.

### contracts

- Step and agent: product-owner (planner).
- Produces: requirements document and behavioural contracts, non-functional requirements, error
  taxonomy.
- Done when: as above.

### architecture

- Step and agent: architect (planner), two steps: `architecture`, then `properties`.
- Produces: architecture, decision records, the dependency list, the toolchain file
  `specs/architecture/toolchain.toml` (the `[commands]`, test report and `[paths]` lines the project
  will use; for an empty repository this is where they are chosen), purity boundary map, module
  criticality, verification tooling, deployment topology, and `data.feasibility` (`ok`, `issues`);
  then the verification properties.
- Done when: as above, and feasibility settled.

### feasibility

- Step and agent: product-owner then architect (planner), at most 2 rounds, only when
  `feasibility.ok` is false.
- Produces: revised contracts.
- Done when: `feasibility.ok`, or the open issues are recorded for the packet.

### assessments

- Step and agent: architect (planner).
- Produces: service-clone assessment, reference-implementation assessment.
- Done when: artifacts valid.

### scenarios

- Step and agent: product-owner (planner).
- Produces: hidden scenarios, each citing the contracts it exercises.
- Done when: artifacts valid. Only product-owner, scenario-reviewer and holdout-evaluator can read
  them.

### ux (UI products)

- Step and agent: ux-designer (planner).
- Produces: UX specification, design system (bootstrapped, or extracted after an ingest), two or
  three variants for complex screens with a scored recommendation.
- Done when: artifacts valid.

### checks

- Step and agent: one clerk running the cross-set `vsdd gate spec` ([[command:gate-spec]]). Fixes by
  the owning planner, at most 2 rounds; each fix step ends with `vsdd gate spec --only <part>`, run
  by the planner itself, so a failed check is another turn of the agent that has the context.
- Produces: gate result whose findings carry the owning role.
- Done when: `gate.passed`.

### review

- Step and agent: two separate loops. The specification loop: adversary once per review unit, and
  ui-reviewer on the UX specification (reviewer); fixes by the planner each finding names in `owner`.
  The scenario loop: scenario-reviewer on the hidden scenarios; its findings and claims go only to
  the product-owner and never into the specification loop.
- Produces: review data.
- Done when: `converge()` at the `spec` perimeter, cap 3, once for each loop
  ([[topic:convergence]]). At the cap, open findings go to the packet; a walled finding appears there
  as its walled key and severity ([[topic:walled-findings]]).

### close

- Step and agent: clerk.
- Produces: decision `spec` with the packet — summary, digest, open issues, recommended variant and
  topology, and the toolchain lines that approval will write where `[commands]` or `[paths]` are
  unset.

## Where it stops for a person, and what it returns

Ends: decision `spec` — a routine human stop ([[topic:human-stops]]). The checkpoint opens the
decision and returns the workflow result ([[schema:workflow-result]]); the next survey commits the
settled record. `revise [stage=<name>]` runs `spec` again from that stage (by default from the review
stage) with the guidance.

## Feature flags (decision D31; built when the build widens)

The architecture stage is where the architect may propose the product's flag system; the checks stage
checks the proposal (F-CHOICE in [[table:flag-checks]]); and the `spec` packet shows the flag lines a
person confirms ([[topic:flag-system-choice]]).
