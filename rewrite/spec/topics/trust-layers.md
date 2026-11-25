---
kind: topic
id: trust-layers
sources: [pipeline.md#2.2, BLUEPRINT.md#3.2]
---

# Trust layers

Scripts steer, tools measure, hooks enforce: what each layer does and what it must not be trusted
with.

A workflow script cannot read a file or run a command. Everything it knows arrives in a step result,
and a step result is something an agent said. The old system had the same weakness everywhere: an
agent announced that a gate had passed, and another agent believed it. The design therefore gives each
layer one job.

| Layer | Its job | What it must not be trusted with |
|---|---|---|
| Script | Order the work: sequence, fan out, loop, branch on result fields. | Truth. A script gate is flow control only. |
| Agent | Judgment: write a spec, write code, find defects. | Reporting whether its own work passed. |
| Tool (`vsdd`) | Measure: run the project's build and test commands, inspect git, check artifacts, compute verdicts, and write the evidence record. | Judging quality. |
| Hook runner | Refuse an action the evidence does not support, and keep agents inside their information walls. | Starting work. |

```
person ── types a command ──> main session (thin: holds a few hundred bytes per run)
                                   │ Workflow tool
                                   v
   workflow script ── step ──> agent ── tool calls ──> hook runner ── allow / block ──> files, git
        ^                        │                          │
        │ step result            │ runs `vsdd gate ...`     │ stores the step record
        │                        v                          v
        └────────────── `vsdd` tool: measures, writes evidence, merges, checkpoints ──> .factory/
```

Two consequences follow, and the rest of the design relies on them.

1. An agent that produces something finishes by running the gate command for it and returns the
   gate's JSON in its result. The script branches on that. At the point of no return (merge, wave
   close, opening a decision, publish) the tool or a hook checks the evidence record again and ignores
   what the script believes. A fabricated result can waste a pass. It cannot merge code.
2. Build, test, lint, format, run, mutation, scan, benchmark and proof commands come from project
   configuration ([[config:commands]]). No workflow, agent prompt or gate names `cargo` or any other
   toolchain.

The gates the tool runs are catalogued in [[topic:gates]]; the wrapper rules the scripts follow are
[[topic:prelude-steps]].
