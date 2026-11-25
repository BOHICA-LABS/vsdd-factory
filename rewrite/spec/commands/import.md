---
kind: command
id: import
package: S5
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.15]
---

# `vsdd import`

Converts pre-1.0 state in place and writes what `feature` needs to start; the real import is person only.

## Syntax

`vsdd import [--dry-run] [--auto-allow <scope>]`

## What it does

"Section 2.15. `--dry-run` may be run by the `setup` skill; the real import is person-only, like
`init --fresh`, because it rewrites the state tree."

The steps, in place on the existing branch and worktree:

1. "Refuses to start if the old state has uncommitted changes or a story worktree has uncommitted work."
2. "Tags the current tip of the state branch `factory-pre-1.0`. That tag is the archive; nothing is deleted
   from history."
3. "Rewrites the tree mechanically and commits once as 'factory: import to the 1.0 layout'."
4. "Writes what a 1.0 project needs in order to take a feature": `factory.toml` by the same detection as
   [[command:init]]; `cycles/_standing/units/import.json`, a baseline closure record with the integration
   head and the latest old release tag, whose measurements "are taken by the first survey that needs them";
   one `spec` decision record, status `approved`, `settled_by: import`, with the digest of the imported
   specification set — "the only approval the importer writes"; and one queued change request
   `requests/CR-<NNN>.md` for each epic that had stories in flight, whose stories are imported as `draft`.
5. "Writes a report: what was converted, what was left out, which files are over their cap, and the requests
   it queued."

"The import does not use a model, so it is deterministic, testable against fixtures of the eight real
layouts, and has a `--dry-run` that writes only the report." `vsdd init` never runs the importer. A project
that prefers a clean start runs `vsdd init --fresh` instead.

## Who may call it

"Main session in `setup` for `init` and `import --dry-run`; `init --fresh` and the real `import` are run by
the person" — in their own terminal, through the shim.

## Writes

Baseline closure record ([[record:closure]] in `cycles/_standing/units/import.json`, carrying
[[record:baseline]]), one approved `spec` decision ([[record:decision]]), queued change requests,
`factory.toml`.

## Failure cases

Uncommitted old state or story worktree (refuses); a file over its cap is imported as it is and listed
under `[caps] waived`.
