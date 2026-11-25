---
kind: skill
id: review
package: K2
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# review

An on-demand independent review outside the pipeline.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<lens> [path or story]"
```

## What it does

Instructs the main session to dispatch the reviewer for the lens with a fixed prompt. The skill first runs
`vsdd review prepare <lens> [base]` ([[command:review-prepare]]), one command that writes the diff patch (for
`code` and `diff`) and returns the next free report path to hand the reviewer. The reviewer's type decides
which walls apply; when it needs a spike it dispatches the spike runner itself.

Lenses:

| Lens | Agent | Method |
|---|---|---|
| `specs` | `adversary` | `review-specs` |
| `stories` | `adversary` | `review-stories` |
| `code <story>` | `adversary` | `review-story-diff` |
| `system` (includes spec drift) | `adversary` | `review-system` |
| `extraction` | `adversary` | `review-extraction` |
| `diff [base]` | `code-reviewer` | `review-diff` |
| `security` | `security-reviewer` | `security-review` |
| `ui` | `ui-reviewer` | `ui-review` |

A report path appears only in an on-demand review like this one: a pipeline reviewer is given no report path
and writes no file. Being dispatched from the main session is a recorded risk: the main session could add its
own framing to the prompt; pipeline reviews are not affected, because scripts compose those prompts, and if it
matters, `review` splits into one forked skill per reviewer at no listing cost
([[topic:risks-agents]]).
