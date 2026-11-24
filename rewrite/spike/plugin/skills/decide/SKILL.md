---
name: decide
description: Probe skill standing in for the decision command a person types.
disable-model-invocation: true
argument-hint: "<id> <choice>"
---

Reply with exactly one line: DECIDE-SKILL-BODY-RAN args=$ARGUMENTS
Then quote verbatim any line in your context that starts with FSPIKE-DECISION, or say NONE.
