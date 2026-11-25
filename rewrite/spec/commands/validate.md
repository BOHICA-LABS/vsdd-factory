---
kind: command
id: validate
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, packaging.md#2.10, hooks.md#2.13]
---

# `vsdd validate`

Checks artifacts against the registry: path, identifier, frontmatter, references, caps and the lock.

## Syntax

`vsdd validate [--changed] [PATH...]`. `--changed` and the paths are alternatives, used
together they are a usage error.

## What it does

"Checks artifacts against the registry: path, identifier, frontmatter, references, caps, the lock,
acceptance criteria against contracts. For workflows, CI and `vsdd status`. It also reports register limits
(deferred items, on-demand reports), always as warnings with exit 0, so a 41st deferred item never turns a
project's CI red. It is the hook design's batch check and `packaging.md`'s `vsdd validate`: one command."
"It checks artifact frontmatter and size caps against the state design's schemas. The same module backs the
hook rule on writes" ([[rule:artifacts]]), "and it can run in a project's own CI."

## Who may call it

Read-only: "Any caller, subject to its wall." It is the hook design's batch check, usable in a project's
own CI and called by [[command:status]] for files changed since the last flush.

## Output, exit behaviour and failure cases

One JSON object under `--json` ([[topic:command-conventions]]). A register over its limit is "a warning
with exit 0, in every mode, so it can never turn a project's CI red". A structural defect is the negative
answer (exit 1); a usage or environment error exits 2.
