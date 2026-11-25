---
kind: command
id: hooks-selftest
package: H9
sources: [BLUEPRINT.md#4.9, hooks.md#2.13, hooks.md#2.14]
---

# `vsdd hooks selftest`

Runs every gate's blocking case through the installed launcher and confirms it blocks.

## Syntax

`vsdd hooks selftest`

## What it does

"Run every gate's blocking case through the installed launcher and confirm exit 2." Used by "`vsdd doctor`,
CI". The launcher cases are the recorded payloads replayed through `sh py/launch.sh hook <mode>`, asserting
on "the exit code, stdout and stderr, which is exactly what Claude Code sees"; "the meta-test fails if any
gate lacks one that blocks and one that allows" ([[topic:rule-testing]]). "`vsdd doctor` ... should run
`vsdd hooks selftest` and the exact launch command, instead of comparing uv version numbers."

"There is no `vsdd hooks canary` and no exit code 3": [[command:doctor]] instead confirms liveness through
its own caller marker ([[topic:caller-identity]]).

## Who may call it

Operator tool (Main session); one of the commands that work outside an initialised project
([[topic:command-conventions]]).

## Output, exit behaviour and failure cases

Per-gate PASS/confirm lines; exit 0 when every blocking case blocks through the launcher. A case that does
not block is a failure of the installation, which `vsdd doctor` surfaces.
