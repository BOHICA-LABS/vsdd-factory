---
kind: command
id: discover-collect
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.9]
---

# `vsdd discover collect`

Runs each configured feedback source and stores its output under `research/inputs/`.

## Syntax

`vsdd discover collect`

## What it does

The `collect` stage of the `discover` scan mode: "the tool runs each command configured under
`[[discover.source]]` (issue trackers, feedback, analytics exports) and stores the output". It is also the
collection inside [[command:gate-sweep]]'s job ("and the collection of configured feedback sources").

The stored files are the researcher's only input: "Files under `.factory/research/inputs/<source>/<date>.*`,
which the researcher reads. The researcher has no shell and never holds a credential"
([[config:discover]], [[artifact:research-report]]).

## Who may call it

`clerk` (the `collect` stage; "clerk running `vsdd discover collect`").

## Writes

Files under `.factory/research/inputs/<source>/<date>.*`.

## Output and failure cases

The stored file paths; a configured source command that fails is a sweep or collect failure reported like
any other check result.
