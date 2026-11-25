---
kind: packaging
id: tests-workflows
package: B5
sources: [packaging.md#layer-2-workflow-tests-node-built-ins-only, BLUEPRINT.md#5.5, pipeline.md#2.11]
---

# Test layer 2: workflow tests (Node built-ins only)

`tests/workflows/harness.mjs` runs a workflow script the way the platform does, with stubs. It reads the file,
rewrites `export const meta` to `const meta`, wraps the body in an async function, and runs it in a fresh `node:vm`
context that contains only the platform's globals. Skeleton of a scenario test:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { runWorkflow } from './harness.mjs'

test('deliver-story stops at the review cap and escalates', async () => {
  const run = await runWorkflow('deliver-story', {
    args: { story: 'S-1.01' },
    agents: {                                   // looked up by label, then by agentType, then '*'
      'vsdd-factory:adversary': () => ({ verdict: 'findings', findings: [{ severity: 'high', title: 'x' }] }),
      '*': (prompt, opts) => minimal(opts.schema),
    },
    maxAgents: 80,
  })
  assert.equal(run.error, null)
  assert.equal(run.result.outcome, 'escalated')
  assert.ok(run.calls.filter((c) => c.agentType === 'vsdd-factory:adversary').length <= 5)
})
```

`run` carries `result`, `error`, `calls` (label, agent type, model, effort, isolation, phase, prompt, schema, the
stubbed result), `logs` and `phases`.

The harness copies the platform's documented behaviour, and each line below is a place where a script that works in
the harness must also work for real:

| Platform behaviour | In the harness |
|---|---|
| No file, shell, module or network access | The context has no `require`, `process`, `fetch` or `import()`. |
| `Date.now()`, `Math.random()`, argument-less `new Date()` throw | They throw. |
| `agent()` with a schema returns a validated object; without one, a string; `null` on a dead agent | The stub's return value is checked against the schema by a small built-in validator (type, properties, required, enum, items). A stub may return `null`. |
| A schema needs `type: 'object'` and `properties` at the root; `required` is optional, and every name in it must be one of `properties` | Checked on every call; a bad schema throws, as it does on the platform. |
| `parallel()` turns a throwing thunk into `null` and never rejects | Same. |
| `pipeline(items, ...stages)` runs each item through the stages in order, each stage receiving `(prev, item, index)`, and drops an item to `null` when a stage throws | Same. |
| `workflow()` runs a child once, returns the child's value and throws when the child threw; a child that calls `workflow()` throws | The child file is loaded and run for real unless the test overrides it; a second level throws the platform's message. |
| At most 1,000 agents in a run; agents are counted, `workflow()` calls are not | One counter shared by a parent and its children; a test can set a lower `maxAgents` to prove a loop ends. |
| `log(message)` | Collected in `run.logs`. |
| `budget.total` and `budget.remaining()`; no agent starts once the budget is spent | Driven by the test's `budget` option; `remaining()` is infinite when no total is set. |
| `meta` is a pure literal at the top; phase titles match | Phase titles are checked. Purity is checked in layer 3, by the platform's own validator. |

## The B5 test list

In `tests/workflows/` the three blocks are parsed with Node and run against a fake `agent()` through six story
cases (clean first pass, one finding fixed, a spike request, the cap reached, the cap reached with an arbiter
dismissal, a CI failure) and three wave cases (clean gate, one integration finding fixed through a fix story, a
run stopped by the story limit), for twenty checks of the corrected behaviours: a clean story as a child (four
agents, every prompt opening with its method and ending with its closing command, the clerk's command using
`vsdd -C`); an exception in a step, which returns `failed` through the checkpoint path; a research spike
request, which is isolated, runs on the planner tier and is opened from the step record; a clean two-story
wave whose children receive `story_agents`; a thrown child charged its full budget; and the convergence cases
(a coverage gap that calls the fixer, an undisputed finding the arbiter may not dismiss, a disputed one it
may, the cap counted across runs, a blocked reviewer, claims with no title, quote or note, and a `holdout`
finding that blocks without a location). Package B5 turns these and the original nine cases into harness
tests.

## Universal tests

Universal tests need no authoring. They run against every file in `<plugin>/workflows/` automatically. In
tests 4, 5 and 7 a stub does two things besides filling its schema, because a run in which they are missing
ends at its first step and proves nothing: it returns as `step` the identity its prompt gave it (the line
`Return step = {...} unchanged.`), since `step()` rejects any other; and the survey stub passes, with
`ok: true`, every precondition met and the units the test supplies.

1. It loads in the harness (syntax, no forbidden global, no clash with a prelude name).
2. Its file name equals `meta.name`, and every `phase()` title is in `meta.phases`.
3. Every `agentType` it passes is a roster agent (until `<plugin>/agents/` is populated the generated `ROLE` table is the roster), and every `workflow()` target exists.
4. With every agent answered by the smallest object its schema allows (empty strings, `false`, empty arrays, the
   first enum value), the run ends without an error and under the agent cap, and its loops ran.
5. With every agent answered "the other way" (`true`, one-item arrays, the last enum value), which includes every
   agent asking for a spike every time, the run still ends under the cap. This is the mechanical proof that
   convergence loops and spike limits terminate.
6. Every step that is not a clerk step begins its prompt with `Method: <name>`, the method file exists, and the
   step's agent owns that method.
7. With every agent returning `{status: 'done'}` and empty data, and again with one agent throwing, the run still
   ends by calling the checkpoint command.
8. No prompt sent to a reviewer contains a fixer's `note` or summary text, and no claim carries a title or a quote.
9. A survey that returns a limit key the prelude does not know fails the run.
10. No prompt sent to an agent that is denied `holdout` contains text from a walled finding: the harness plants a
    marker string in a scenario-reviewer finding and in a finding located under `.factory/holdout/`, and fails if
    the marker appears in any prompt to another agent type.
11. Every free-text value in a clerk's command went through the percent-encoder: the harness passes a request
    containing spaces, both kinds of quote, a semicolon, parentheses, an asterisk and an exclamation mark, and
    checks that the command is one segment in which none of them appears raw.
12. Every workflow accepts its documented argument text as well as an object, through `parseArgs()`, and a
    `key=value` word whose key is not one of its arguments stays in the main argument.
13. Every child story a workflow starts receives `limits.agents` equal to `story_agents`, and a child that throws
    is charged that amount.

The harness counts agents the way the platform does: a parent and the children it calls share one counter and the
1,000-agent stop, so a wave whose children overspend is caught here.

Delivered by package B5 with the schema validator, the two answer generators, self-tests on two toy workflows
([[package:B5]]). The harness is a second implementation of the platform's rules; the conformance tier pins each
row to the real platform ([[packaging:tests-e2e]]).
