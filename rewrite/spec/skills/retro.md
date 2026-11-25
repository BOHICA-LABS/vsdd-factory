---
kind: skill
id: retro
package: K3b
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# retro

A retrospective of a run: cost, timing, gate outcomes, wall integrity, what was learned, improvement proposals.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
context: fork
agent: vsdd-factory:researcher
argument-hint: "[last run or wave N or release]"
```

## What it does

Runs in a forked `researcher` with the `retrospective` method ([[agent:researcher]]). The report is a research
report of kind `retrospective`; there is no lessons register. Each proposal names the line it would add to
`conventions.md` or `review-rules.md`, and the reply tells the person the command that adopts it:
`/vsdd-factory:author conventions <proposal>` or `author review-rules <proposal>` ([[skill:author]]).
