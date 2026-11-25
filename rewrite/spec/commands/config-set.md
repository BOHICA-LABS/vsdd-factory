---
kind: command
id: config-set
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.12, packaging.md#2.10]
---

# `vsdd config set`

Changes one key of `factory.toml`; person only.

## Syntax

`vsdd config set <key> <value>`

## What it does

"Changes one key of `factory.toml`; person only." It is one of the person-only commands ("the person-only
commands (`vsdd decide`, `vsdd import`, `vsdd init --fresh`, `vsdd config set`, `vsdd lock release --force`)
always run the code of the version this project uses" through the terminal shim). It is the person's way to
edit a key outside the moments code writes the file itself ("`vsdd init` and `vsdd import` when they create
it; `vsdd repos split`; and the survey, when an approved `spec` decision covers a toolchain file and
`[commands]` or `[paths]` keys are still unset").

## Who may call it

The person — "through the typed command for `decide`, otherwise in their own terminal"
([[topic:command-conventions]], [[topic:caller-identity]]).

## Writes

One key of `factory.toml` ([[config:project]] and the section configs it hosts, e.g. [[config:commands]],
[[config:limits]]).

## Failure cases

"Unknown keys and wrong types are errors that the briefing reports" — the same loader validates what the
command writes.
