# Conflicts and gaps found while writing the state group

Written by the state extractor (the `group = "state"` entries of the registry: the topics, the artifact
kinds and `tables/artifacts.toml`). Nothing here is resolved beyond what was needed to write the files.
Each entry gives both section ids and both texts, and says which one the new files use. The rule applied:
section 4 of the blueprint, then the rest of the blueprint, then the detail document that owns the subject.

## Disagreements

### S1. The contents of `.factory/local/`

- `BLUEPRINT.md#4.7`: "└── local/                           untracked: runs/, evidence/, jobs/, lock.json,
  driver.json, hooks/, events/, media/"
- `state.md#layout`: the same list plus three more names: "├── lock.json, lock.flock           the run lock
  and the file lock that guards it", "├── media/, index.json", "└── flush.lock".
- `BLUEPRINT.md#4.1` (Local store) lists "step records, gate evidence, the run lock, the hook runner's
  records, the event log, caches" and does not name media.
- Used: `state.md#layout` in `topics/state-layout.md`, because `lock.flock` and `flush.lock` are named in
  the prose of `state.md#2.3` as well ("a file lock on `local/lock.flock`", "Takes
  `.factory/local/flush.lock`"), so the blueprint tree is a shorter drawing of the same layout, not a
  different one. The topic also carried a comparative paragraph ("`BLUEPRINT.md#4.7` shows the same tree
  ... the tree above is the definition"); it was moved here when the conversion commentary left the
  design files, and this entry is that comparison. Related gap: `index.json` (below).

### S2. The registry's columns

- `state.md#2.1`: "Registry | `artifacts.toml`: one table per artifact kind giving its path pattern,
  identifier pattern, frontmatter keys, required headings, size cap and frozen statuses."
- `state.md#2.4`: "It needs one column beyond the hook design's list: `template` (the skeleton file under
  `<plugin>/templates/`). The first pass also had `code_values`, for statuses only `vsdd` could set; the
  second pass removed those statuses from the files (below), so the column is gone." The worked example
  also shows `dir` and `lock`.
- Used: `state.md#2.4` in `topics/artifact-rules.md` and `tables/artifacts.toml` (the more specific and
  later text; `code_values` is gone, `template` is added). Already noted in `conflicts/index.md` Part 2.

### S3. `active` — written by the agent or computed?

- `state.md#2.4`: "An agent sets only the statuses its kind lists in section 2.5: `draft`, `retired`,
  `dropped`, `stale`, `proposed`, `rejected`, `withdrawn`, and `active` for the living kinds that have no
  approval step of their own (subsystem, screen, flow, scenario)." The 2.5 rows agree: subsystem `status`
  (`active`, `retired`); UX `status` (`active`, `retired`); scenario `status` (`active`, `stale`,
  `retired`).
- `BLUEPRINT.md#5.2`: "Code no longer promotes files to `approved`, `ready`, `done`, `proven`, `accepted`
  or `active`, so the registry column for code-only values ... is all gone." The sentence lists `active`
  among the values code no longer writes.
- Used: `state.md#2.4` and `state.md#2.5` in `topics/computed-status.md` and the artifact files, because
  the state design owns the rule and `BLUEPRINT.md#4.7` agrees with it ("An agent sets only the statuses
  its artifact kind lists (`draft`, `retired`, `dropped`, `stale` and the like)"). Both senses reconcile
  only this way: an agent writes `active` itself; no code path ever sets it. The topic also carried the
  blueprint's other-side wording ("Whether a specification file is approved, a story ready or done, or a
  property proven is read from the decision or closure record that made it so, and code never rewrites
  tracked files to say it"), moved here when the conversion commentary left the design files; it says what
  this entry records.

### S4. What `CR-NNN` identifies

- `BLUEPRINT.md#4.8`: "`CR-NNN` for a queued change request (new)".
- `BLUEPRINT.md#4.7`: "requests/CR-<NNN>.md             every change request, queued or running (written
  by code)".
- `state.md#2.5`: "Change request | `requests/CR-<NNN>.md` | ... `status` (queued, active, done,
  dropped), ... This is the only copy of a request".
- Used: `BLUEPRINT.md#4.7` with `state.md#2.5` in `artifacts/change-request.md`: the identifier covers the
  request for its whole life (queued, active, done, dropped), and the file is written by code. Both texts
  are inside blueprint section 4, which is why no outside tie-breaker exists; the fuller one is used. Same
  finding as `conflicts/index.md` C8, restated here because the artifact file had to pick one.

### S5. Is the holdout evaluation an agent report or a machine record?

- `BLUEPRINT.md#4.7` counts it among the machine records: "Holdout evaluation, with any walled findings |
  `holdout/evaluations/<cycle>/<unit>-<n>.json` | yes".
- `state.md#2.5` puts the same file in the table "Reports written by agents during a cycle", with writer
  "`vsdd checkpoint`, from the evaluator's and the scenario-reviewer's step records".
- Used: neither file defines it twice. It is registered once, as `record:holdout-evaluation`, and
  `artifacts/` has no file for it; the rows of `state.md#2.5` that concern it are carried by the records
  group. `artifacts/review-report.md` and `topics/state-layout.md` link to it instead of restating it.

### S6. `dependencies.toml` has no row in the artifact tables

- `state.md#layout` lists it: "overview.md, SS-<NN>.md, modules.toml, dependencies.toml, conventions.md,
  review-rules.md, ..." — `state.md#2.8` describes its content ("`dependencies.toml` is part of the locked
  specification set, so a dependency a person approves in a `merge` decision cannot be written into it
  during a build."), and `BLUEPRINT.md#5.2` says "The architecture gains `dependencies.toml`, the list of
  dependencies a `spec` decision approves."
- `state.md#2.5`: the specification-chain and architecture tables of this section have no
  `dependencies.toml` row, so it has no stated writer, cap or frontmatter there.
- Used: wrote `artifacts/dependencies.md` from `state.md#layout`, `state.md#2.8` and `BLUEPRINT.md#5.2`.
  Gap to fix: the cap and the writer moment for `dependencies.toml` are stated nowhere; the file links
  instead of inventing them.

## Gaps noticed while copying

### S7. "Idea briefs" in `research/`

Both `state.md#layout` ("reports, retrospectives and idea briefs") and `BLUEPRINT.md#4.7` ("reports,
retrospectives and idea briefs") say the research directory holds idea briefs. But `state.md#2.5` puts
ideas at `ideas/ID-<NNN>.md` and gives the research-report kinds as "domain, technical, market,
competitive, feedback, opportunity, retrospective" — there is no idea-brief kind. Not resolved; copied as
written in `topics/state-layout.md`, flagged here.

### S8. `local/index.json` is named but never defined

`state.md#layout` has "├── media/, index.json" in the `local/` tree. The only related text is `vsdd status
--fix` "rebuilds the catalogue cache" (`state.md#2.10`) and the catalogue being "`vsdd list <kind>`, from a
cache keyed by path, size and modification time" (`state.md#2.6`). The path of the catalogue cache is
therefore presumably `local/index.json`, but no section says so. Not fixed; carried into
`topics/state-layout.md` as written and flagged here.

### S9. Writers of `disposition.md` and `port.md` under `analysis/`

`BLUEPRINT.md#4.7` and `state.md#layout` both say `analysis/<scope>/` holds "security.md, disposition.md,
port.md". `state.md#2.4` names a writer only for `security.md` ("`codebase-analyzer`; `security-reviewer`
may create `security.md`"), and `state.md#2.5` gives the analysis kind one frontmatter shape (`title`,
`pass`) for all of them. Nothing says who writes `disposition.md` or `port.md`, or whether they are
separate kinds, nor how the two files relate to the scopes of an analysis. Not fixed; `artifacts/analysis.md`
named this conflicts file for the relation, which was the conversion commentary the second audit's
structure pass moved here.

### S10. The `pass` key of the analysis frontmatter

`state.md#2.5`, analysis row: "Frontmatter keys: `title`, `pass`". No type or value list is given for
`pass` (elsewhere in the old system `pass` was the analysis pass number; here it is undefined). Flagged;
not fixed.

### S11. The `kind` of a brief-opened cycle

`state.md#2.8` gives the cycle record `kind` (`product`, `feature`, `hotfix`, `maintenance`) and says the
route is `standard` "from the opening when a brief opens the cycle". No section states which `kind` a
cycle opened by `vsdd checkpoint brief` receives (the status-object example shows a `feature` cycle). A
first release presumably takes `product`, but no text says it. Flagged in `topics/cycles.md` scope; not
fixed.

## Cross-references to the same findings elsewhere

- `conflicts/index.md` Part 2 already records S1, S2, S5 and S6 with the same conclusion; they are
  restated here because the owning files are this group's.
- History in this group's sources, deliberately not copied (also reported as `history_sections`):
  `state.md#amendments-from-the-blueprint` (the amendment list), `state.md#3` (mapping from the old
  system), `state.md#2.17` (departures from the readings in `rewrite/STATE.md`), `state.md#6` (open
  questions, answered by `BLUEPRINT.md#2`). Inside otherwise-copied sections, the sentences recording how
  large old files grew (the 685 KB decision record, the 480 KB story, the 198 KB analysis pass, the 110 MB
  of demo recordings, the 2,000-contract corpus, the 3,489 state commits) were kept only where they justify
  a cap or a design choice in the copied text.
