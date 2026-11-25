---
kind: command
id: repos-split
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.11, state.md#2.14, state.md#2.12]
---

# `vsdd repos split`

Creates member repositories after a spec decision and appends their entries to `factory.toml`.

## Syntax

`vsdd repos split` (no arguments).

## What it does

The `split` stage of `multi`: "A `spec` decision chose `topology=split`: the tool creates the member
repositories from the `repo` key of the architect's subsystem files and appends their `[[repo]]` entries.
This is the one case in which code adds to `factory.toml` after `init`, and the next briefing announces it"
([[topic:multi-repo]], [[config:repo]]). "It remains the case in which a `spec` decision creates the
members" — distinct from `init --hub`, which writes the entries when it creates the file.

Appended entries carry "that repository's path, branches and commands", with optional per-repository
`[release]` and `[delivery]` tables "with the same keys as the top level, so the hub release can ship each
repository by its own rules".

## Who may call it

`clerk` (the `split` stage of `multi`).

## Writes

The member repositories; `[[repo]]` entries appended to `factory.toml` — one of the stated cases in which
code writes the configuration file ([[topic:project-configuration]]).

## Output and failure cases

`{ok, repos}`. Requires a `spec` decision that chose `topology=split`.
