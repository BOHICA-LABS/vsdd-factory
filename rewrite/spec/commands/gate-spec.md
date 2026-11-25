---
kind: command
id: gate-spec
package: G5
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.3]
---

# `vsdd gate spec`

Checks the specification set, or one part of it with `--only`.

## Syntax

`vsdd gate spec [--full]` and `vsdd gate spec --only <part>`

## What it does

The cross-set check of the specification: one clerk runs `vsdd gate spec` over the whole set, and "fixes by
the owning planner, at most 2 rounds; each fix step ends with `vsdd gate spec --only <part>`, run by the
planner itself, so a failed check is another turn of the agent that has the context". "References between
artifacts are checked for existence by `vsdd gate spec` and at approval, not when a draft is written, so
contracts may name subsystems that the architecture stage has yet to write."

The same cross-set check reappears as the `spec coherence` check inside [[command:gate-sweep]].

## Who may call it

"`clerk` for the cross-set check; the owning planner for `--only <part>` after its own fix."

## Writes

"Evidence" ([[record:evidence]]).

## Output and failure cases

"Gate result whose findings carry `owner`" ([[schema:gate-result]]) — the owning role each fix is routed to,
so a failed check returns to the agent that has the context.

## Feature flags (decision D31; built when the build widens)

One more part, `flags`, checks a proposed flag system: F-CHOICE in [[table:flag-checks]].
