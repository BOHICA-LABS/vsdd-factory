---
kind: skill
id: author
package: K2
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# author

Has the owning specialist create or change one artifact.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<artifact type> <what>"
```

## What it does

Instructs the main session to dispatch the owner agent with its method.

Artifact types and the agent and method each dispatches:

| Type | Agent | Method |
|---|---|---|
| `decision` | `architect` | `decision-record` |
| `architecture`, `diagram`, `data-model` | `architect` | the method of the same name |
| `conventions`, `review-rules` | `architect` | `conventions` |
| `contract`, `requirements`, `domain` | `product-owner` | `requirements`, `spec-delta` or `domain-spec` |
| `holdout` | `product-owner` | `holdout-scenarios` |
| `story` | `story-writer` | `story` |
| `flag` | `story-writer` | `story`, for a flag record ([[artifact:flag]]; decision D31, built when the build widens) |
| `ux`, `design-system` | `ux-designer` | `ux-spec` or `design-system` |
| `doc` | `technical-writer` | `docs` |
| `conform <path>` | the owner of the path's class | the owner's method, to fix what the lint tool reported |

`conventions` and `review-rules` deserve the why: this is how a retrospective's proposal is adopted, and how
`setup` moves build conventions out of a project's `CLAUDE.md` — the edit lands in the file every builder
(`conventions.md`) or every reviewer (`review-rules.md`) is given. These two guidance files are locked with the
specification while a cycle is open, so nobody changes the rules in the middle of a build, and are writable by
the architect between cycles, which is when a retrospective's proposals arrive. They are outside the `spec`
decision's digest, so adopting one leaves the approval current. Any other specification file that is locked is
changed through `/vsdd-factory:feature`, and the hook's block message says so.

The diagram renderer behind `author diagram` is `vsdd render diagram` (need N-PKG-4).
