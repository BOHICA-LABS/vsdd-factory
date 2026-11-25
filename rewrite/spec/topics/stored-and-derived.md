---
kind: topic
id: stored-and-derived
sources: [state.md#2.6]
---

# Stored and derived

Each fact is stored in one place; the derived views are computed on request and never written back. What the
old system stored and what 1.0 does with it:

- **Catalogue of contracts, properties, stories, scenarios, subsystems, research** — old: eleven hand-kept
  index files, four of them over 400 KB. 1.0: `vsdd list <kind>` ([[command:list]]), from a cache keyed by
  path, size and modification time.
- **Counts** — old: stored in `STATE.md`, the requirements document and every index. 1.0: never stored.
- **Titles in other files** — old: copied, with a policy that the heading wins. 1.0: stored in the
  artifact's frontmatter only.
- **Reverse links, traceability matrix, coverage matrix, UI traceability** — old: stored in several files
  and kept in step by policy. 1.0: `vsdd refs <id>` ([[command:refs]]), `vsdd coverage`
  ([[command:coverage]]).
- **Subsystem registry, epic list, dependency graph** — old: `ARCH-INDEX.md`, `epics.md`,
  `dependency-graph.md`. 1.0: derived from the subsystem files, the epic directories and `depends_on`.
- **Wave schedule** — old: `wave-schedule.md`, `sprint-state.yaml`, `wave-state.yaml` and a `wave` key in
  every story. 1.0: computed by `vsdd plan schedule` ([[command:plan-schedule]]) from dependencies and
  limits; stored once in the plan's closure record ([[topic:waves]], [[record:closure]]).
- **Whether a story is in progress** — old: a status an agent set. 1.0: derived — its wave is open and it
  is not done. Its position comes from evidence.
- **Whether a contract is implemented, an epic done, a decision record superseded** — old: status values
  promoted by an agent after merge. 1.0: derived ([[topic:computed-status]]).
- **Version of an artifact, and citations of that version elsewhere** — old: a `version` key, bumped by
  hand, cited in stories and indexes. 1.0: git history. A decision records the digest of what it approved
  ([[topic:digests]]).
- **Whether a file is approved, a story ready or done, a property proven** — old: a status an agent or the
  state-manager wrote into the file. 1.0: computed from the decision or closure record that made it so;
  shown by `vsdd list`.
- **Staleness** — old: `inputs` and `input-hash` in every artifact. 1.0: not tracked per stage. An approval
  is bound to a digest of what was approved, so a changed specification makes its decision stale and the
  stop returns. After approval a contract changes only through an amendment, whose change analysis names
  the stories it affects, and `plan` in delta scope re-traces them. A stage is run again only when a
  person's `revise` or `hold`, or an amendment, marks it not done. There is no `vsdd stale` report and no
  `built_against` field.
- **Product commit ids** — old: cited in `STATE.md` and patched after each commit. 1.0: recorded once in
  evidence and closure records; never updated.
- **Convergence trajectory, cycle manifest, pull request description, debt report** — old: written and
  maintained. 1.0: derived from closure records.
- **Changelogs, amendment histories, burst logs, decision logs** — old: hand-kept files of up to 2 MB.
  1.0: git history of the state branch.

The digest an approval is bound to is `vsdd.state.digest(paths)`: SHA-256 over the sorted list of each
file's path and content hash, cut to 16 hex characters. It is over the whole file: approval changes no file,
so there is nothing to leave out of a file. Decision records use it; which files a `spec` decision covers is
in [[topic:digests]].
