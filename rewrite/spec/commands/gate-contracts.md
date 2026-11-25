---
kind: command
id: gate-contracts
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.11]
---

# `vsdd gate contracts`

The multi-repository contracts gate, run when a provider's published interface changed since its consumers
were last built.

## Syntax

`vsdd gate contracts --run <id>`. A job command ([[topic:long-commands]]).

## What it does

A stage of the `multi` workflow: "clerk running `vsdd gate contracts`; generated client libraries are
rebuilt through fix stories in their repositories. Runs when: A provider's published interface changed
since its consumers were last built." It is one of the four checks-as-one-job gates, and `--only <check>`
selects a single check ([[topic:gates]]).

## Who may call it

`clerk` (the `contracts` stage of `multi`).

## Writes

Evidence ([[record:evidence]]). Generated client libraries are not fixed in place: the rebuilds go through
fix stories in their own repositories.

## Output and failure cases

"Gate result" ([[schema:gate-result]]); consumer repositories whose clients no longer match are reported
and repaired through fix stories.
