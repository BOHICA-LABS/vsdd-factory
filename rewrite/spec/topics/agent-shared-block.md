---
kind: topic
id: agent-shared-block
sources: [agents-skills.md#2.7, BLUEPRINT.md#4.1]
---

# The shared agent block

The shared block is the same text in every agent file, checked byte for byte by lint. Agent files cannot
import, so the build stamps it: the one source is `workflow-lib/agent-shared.md`, and `tools/build.py` copies
it into every agent file between markers, as it stamps the prelude into workflows. One agent omits the first
bullet of the second section, because a spike cannot start a spike: `spike-runner`. The `clerk` carries a
two-line block of its own.

Reviewer files also carry the severity table and the finding key rule
(`<category>:<path>:<symbol or clause>`, the same in every pass). The severity table has one source too,
`workflow-lib/severity.md`, which holds the severity table of [[table:severity]] and is stamped into
every reviewer file under the heading `## Severity`; the table itself lives in [[table:severity]].

## The block

```markdown
## Ground rules
- The spec is the authority. If it is wrong or silent, say so in your result. Do not work around it.
- Report only what you checked. Name the command you ran or the file you read. If a check did not run, say so.
- Done means on disk. Write your work, and commit code, before you return. Your result points to it.
- Stay in your role. If a fix belongs to another role, describe it in your result and leave it.
- Use the paths you were given, as given. Do not build a path into the state directory yourself.
- If a hook blocks a tool call, read its reason and follow it. Do not reach the same effect another way.

## When you cannot proceed
- A question that only an experiment or outside research can settle: do not guess. If you have the Agent tool,
  dispatch `vsdd-factory:spike-runner` with the question, why it blocks you and a time box, then continue with
  its finding. If you do not have the Agent tool, you are a workflow step: stop and return the spike request
  your result schema provides.
- A missing input, or a problem you could not fix in two attempts: stop and return blocked, with what you tried.
```

