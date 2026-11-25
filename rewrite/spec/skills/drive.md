---
kind: skill
id: drive
package: K1
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, pipeline.md#2.4]
---

# drive

The low-touch driver: runs the pipeline from where it stands until a person is needed. The one deliberate
exception to "no skill exists to start a workflow": it follows the route from one workflow to the next so a
person does not type each command.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
```

## What it does

Instructs the main session to run `vsdd driver on` ([[command:driver-on]]), read `next` from
`vsdd status --json` ([[command:status]]), start that workflow with the Workflow tool, read `next` from its
result and repeat. At a human decision it runs `vsdd driver pause <decision id>` ([[command:driver-pause]])
and stops; when the person settles that decision, the `decide` hook's context tells the main session to resume
([[skill:decide]]), so the person does not type `/vsdd-factory:drive` again after each approval. At done, or
after two runs with `progress: false`, it runs `vsdd driver off` ([[command:driver-off]]).

It lives in the main session because a workflow may nest only one level. After a compaction the briefing tells
the main session that the driver is active or paused and how to continue. The driver only steers: if the main
session starts the wrong workflow, that workflow's survey refuses, so enforcement does not depend on the
driver behaving. With the driver, a greenfield project needs three human actions after the brief conversation:
approve the brief, approve the specification, authorise the release.
