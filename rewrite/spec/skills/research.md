---
kind: skill
id: research
package: K3b
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, agents-skills.md#2.10]
---

# research

Researches a topic with cited sources and saves a dated report.

Invoked by: person.

Frontmatter beyond `name` and `description` (the whole file, from the skeleton in the design):

```
---
name: research
description: Research a topic with cited sources and save a dated report.
argument-hint: "<domain or general or market> <topic>"
disable-model-invocation: true
context: fork
agent: vsdd-factory:researcher
---

Method: `research`, or `market-intel` when the first word is `market`.
Topic: $ARGUMENTS
Write the report to the research directory. Return its path and a five-line summary.
```

## What it does

Runs in a forked `researcher` ([[agent:researcher]]). The researcher has no shell, so it cannot delete a report
its new one replaces: its reply ends with the superseded paths, and the main session runs `vsdd retire <path>`
for each. Inside a workflow the checkpoint does that from the step's `superseded` list.

When a research server is missing, nothing fails: the `research` method tells the researcher to prefer
Perplexity, Context7 and Tavily, to fall back to `WebSearch` and `WebFetch`, and to record which tools it used;
its result carries `sources_count` and `tools_used`, so the workflow can refuse a report with no sources.
Which tools reach it is [[topic:research-and-browser-tools]].
