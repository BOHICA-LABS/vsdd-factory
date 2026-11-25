---
kind: skill
id: defer
package: K2
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# defer

Adds, lists or resolves an entry in the deferred-work register.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<add or list or resolve> ..."
```

## What it does

Instructs the main session: `add` runs `vsdd defer add` ([[command:defer-add]]) and writes the entry; `list`
runs `vsdd defer list` ([[command:defer-list]]); `resolve` runs `vsdd retire <path>` ([[command:retire]]), the
one command that deletes a file in a class that allows deletion.

Because `defer` cannot be invoked by the model, the old rule that only a person may defer work is enforced by
the platform and not by a paragraph of prose. Inside the state directory the main session writes only `brief`
and `deferred`; this skill is the `deferred` half of that grant.
