---
kind: topic
id: importer
sources: [state.md#2.15, BLUEPRINT.md#5.2, BLUEPRINT.md#2.1]
---

# Importing pre-1.0 state

An importer is provided (decisions D1 and D3 of the blueprint). The eight projects on this machine hold real
specifications: five of them have between 86 and 278 contracts; `prism` has 352 story files, 95 decision
records and about 100 hidden scenarios across nine repositories; `pregolya` and `flowloom` each carry a
`STATE.md` of about 300 KB. Without an importer each project starts again from nothing or stays on rc.25.

Because storage is unchanged ([[topic:state-directory]]), the import is a transformation in place. When
`setup` finds pre-1.0 state it runs `vsdd import --dry-run` ([[command:import]]), shows the report, and gives
the person the command `vsdd import`, which only a person may run (in their own terminal, through the shim).
The import works in the existing `.factory` worktree on the existing branch:

1. Refuses to start if the old state has uncommitted changes or a story worktree has uncommitted work.
2. Tags the current tip of the state branch `factory-pre-1.0`. That tag is the archive; nothing is deleted
   from history.
3. Rewrites the tree mechanically and commits once as "factory: import to the 1.0 layout".
4. Writes what a 1.0 project needs in order to take a feature. Without these an imported project could not
   start: `feature` would refuse for lack of a baseline, and the route would send a project with hundreds of
   approved contracts back to `ingest`.
   - `factory.toml` ([[topic:project-configuration]]), by the same detection as `vsdd init` (and with
     `--auto-allow`, the answer `setup` collected).
   - `cycles/_standing/units/import.json`, a baseline closure record ([[record:baseline]]) with the
     integration head and the latest old release tag. Its measurements are taken by the first survey that
     needs them.
   - One `spec` decision record in `cycles/_standing/decisions/`, status `approved`, `settled_by: import`,
     with the digest of the imported specification set. This is what makes the imported contracts the
     approved, locked specification ([[topic:spec-lock]]), and it is the only approval the importer writes.
   - One queued change request ([[artifact:change-request]]), `requests/CR-<NNN>.md`, for each epic that had
     stories in flight, naming those stories. The stories are imported as `draft`; the request is what
     brings them back through `feature` and `plan`, so no story is "ready" with no plan or wave behind it.
5. Writes a report: what was converted, what was left out, which files are over their cap, and the requests
   it queued.

`vsdd init` never runs the importer ([[command:init]]). On pre-1.0 state it reports `legacy` and prints the
dry-run command and the import command. A fixture test holds the whole path: import, then one fast-route
`feature`.

## What is converted

- **Product brief, domain spec sections, requirements document.** Moved under `specs/product/`. Index files
  dropped. Lists of contracts stripped from the requirements document.
- **Contracts, properties, decision records, subsystem documents.** Moved to the new paths. Frontmatter
  reduced to the registry's keys. Old statuses mapped to the statuses a file may hold: active contracts and
  properties become `draft` files covered by the imported `spec` decision, which is what makes them
  approved; deprecated, retired and removed become `retired`; accepted decision records are listed in the
  imported decision's `accepted`; properties verified with a lock are listed as proven in the import
  closure. Reverse-link sections and changelogs removed. Identifiers kept.
- **Stories and epics.** Moved under `stories/E-<N>/`. A merged story gets a delivery closure in the import
  record, which is what makes it done. A story in flight is imported as `draft` and named in a queued
  change request for its epic. An identifier that does not fit `S-N.NN` gets a new one, and the old one
  goes into `aliases`, so references in code and commit messages still resolve through `vsdd refs`.
- **Hidden scenarios.** Moved to `holdout/`. Wave scenarios become ordinary scenarios.
- **Research, ingestion and port analysis.** Moved to `research/` and `analysis/`.
- **Technical debt register.** Open entries become deferred items ([[artifact:deferred-item]]) with
  `target: unscheduled`.
- **`STATE.md`, decision logs, burst logs, session checkpoints, lessons, indexes, amendment histories,
  cycle directories, wave and sprint state, policies, pull request and review archives, demo evidence,
  logs.** Not imported. They remain at the tag.

## Files over their cap

A file over its cap is imported as it is and listed under `[caps] waived` ([[config:caps]]) with its size. A
waived file can be made smaller but not larger, and `vsdd status` counts them. Bringing one under the cap is
ordinary work for the owning agent: `/vsdd-factory:author conform <path>` for a file that is not locked (a
draft story, research, analysis), and the next amendment run for a file in the specification set, which the
imported `spec` decision locks. A frozen file (an accepted decision record, a done story) stays waived. The
import does not use a model, so it is deterministic, testable against fixtures of the eight real layouts,
and has a `--dry-run` that writes only the report.

A project that prefers a clean start runs `vsdd init --fresh`, which renames the old branch to
`factory-artifacts-rc` and starts a new one.
