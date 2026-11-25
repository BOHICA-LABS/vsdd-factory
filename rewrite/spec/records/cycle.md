---
kind: record
id: cycle
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8, state.md#2.4]
---

# Cycle record

`cycles/<cycle>/cycle.json`. Tracked. Written only by code.

## Fields

- `id`: the cycle's name, a slug of 2 to 40 lower-case characters.
- `kind` (product, feature, hotfix, maintenance)
- `title`
- `route` (`standard` from the opening when a brief opens the cycle; written by `vsdd feature route` when a
  change request opened it; empty for a maintenance cycle)
- `opened`
- `baseline` (a release tag, an ingest or an import; empty for a first release). It is what delta runs
  compare against, not the commit story branches are cut from: each story records its own base
  ([[topic:worktrees]]).
- `request` (`brief`, or the id of the one request file, `CR-<NNN>`)
- once closed: `closed`, `version`, `archive_commit`

Once closed means the close fields the release checkpoint adds: the close date, the release version and the
commit of the state branch just before pruning (totals and pass counts are derived from the closure records,
so there is no separate summary file).

## Written by

Whichever command opens the cycle: `vsdd checkpoint brief`, the `discover` assess checkpoint,
`vsdd survey feature`, `vsdd story new-fix --maintain`. `vsdd feature route` writes `route`. The `release`
checkpoint adds the close fields.

## Cap and lifetime

Cap 2,000 bytes. Kept permanently.

A change request is stored once, as `requests/CR-<NNN>.md`, and `cycle.json` names it (or `brief`); see
[[record:decision]] for the decision records and [[artifact:change-request]] for the request file.

## Team use (decision D33; built when the build widens)

Opening a cycle takes an `open` claim, which is removed in the commit that adds this record, so two
clones cannot each open one ([[table:work-claim-scopes]]). Open cycles are ordered by kind and id,
never by `opened` ([[table:team-checks]], T-ORDER).
