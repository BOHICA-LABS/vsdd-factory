---
kind: command
id: rules
package: H9
sources: [BLUEPRINT.md#4.9, hooks.md#2.13]
---

# `vsdd rules`

Prints the rule table with each rule's current strength in this project.

## Syntax

`vsdd rules [--json]`

## What it does

"The rule table: id, events, tools, kind, one-line summary, current strength in this project"
([[table:rules]]). The current strength reflects how a project tuned a rule or turned it off
([[topic:rule-tuning]]); the reference pages and the meta-test are generated from the same table
(packaging H6).

## Who may call it

"People; reference generation and the meta-test" use it; the read-only row of the command table puts it
with the views "any caller, subject to its wall".

## Output and failure cases

The rule table as text, or as one JSON object under `--json`; writes nothing.
