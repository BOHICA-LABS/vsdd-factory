---
kind: skill
id: brief
package: K2
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# brief

Guided creation of the product brief, with brainstorming techniques and a final check, saved as a draft after
each section.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "[idea, or a path to notes]"
```

## What it does

Instructs the main session. It is a conversation, so it cannot be a workflow. It saves `brief.md` as a draft
after each section, so a compaction or a crash in the middle of the conversation loses nothing, and "the brief
is in draft" is a route state whose next command is this skill. When the person says the draft is ready it
dispatches the `product-owner` with the method `brief` to write the final text on the planner tier
([[agent:product-owner]]), and then runs `vsdd checkpoint brief` ([[command:checkpoint-brief]]), which opens
the cycle and the `brief` decision.

The main session may write the `brief` class inside the state directory (the draft) and nothing else there;
the final text is written by the dispatched agent.
