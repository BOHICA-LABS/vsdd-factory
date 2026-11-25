---
kind: skill
id: decide
package: K1
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, BLUEPRINT.md#4.11, hooks.md#2.11.5]
---

# decide

Settles a decision: the one act only a person may perform.

Invoked by: person.

Frontmatter beyond `name` and `description` (the whole file, from the skeleton in the design):

```
---
name: decide
description: Settle a factory decision that is waiting on you.
argument-hint: "<id> <choice> [key=value ...] [-- note]"
disable-model-invocation: true
---

The hook runner has already recorded this decision, or refused it and told the person why.
Do not run `vsdd decide`. Relay the recorded decision and the next command from the context you were given,
in at most three lines.
```

## What it does

The hook runner records the decision from the typed command before any model reads it. The skill body tells
the main session to relay the context the hook returned (the decision, the choice, the next command) and never
to run `vsdd decide` itself. If that context says the driver was paused at this decision, the main session
resumes the driver loop.

The recording rule is the hook design's decisions rule (need: the rule [[rule:decisions]]): a person types
`/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, `UserPromptExpansion` fires before any model
reads the message, the rule parses the arguments by one grammar and calls the state design's settle function,
which checks that the decision is open, that the id is not ambiguous, that the choice and its keys are valid
for the kind, and that the digest of what is being approved still matches. Nine decision kinds and their
choices are in [[table:decision-kinds]]. If the settle
function refuses, the rule blocks and nothing reaches the model; if it succeeds, the rule returns context for
the model: the decision id, the choice, and the next command from the status function. An agent cannot do any
of this: `gate-guard` refuses every tool call that runs a subcommand on the `human_only` list (`decide`),
whoever the caller is, including the main session's model. A person can still run it in their own terminal
through the `~/.local/bin/vsdd` shim that `setup` installs.
