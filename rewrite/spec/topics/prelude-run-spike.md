---
kind: topic
id: prelude-run-spike
package: B4a
sources: [pipeline.md#2.11.1, pipeline.md#2.8]
---

# Prelude runSpike()

`runSpike()`: the wrapper that runs a spike as a step of the same workflow and restarts the asking
step. The spike protocol, entrances and limits are [[topic:spikes]]; the person-facing workflow is
[[workflow:spike]].

```js
// One research spike: an isolated, bounded experiment that answers one question. Nothing about the asker is
// passed on the command line: vsdd spike open reads the question and the asking agent's type from the
// needs_spike step record that the hook stored, named by stepKey. A workflow always passes one; the person's
// spike workflow calls runSpike() with an empty step key, which omits --step, and spike open reads the run
// record instead. Inputs are files, never standard input.
async function runSpike(req, stepKey, phaseTitle) {
  RUN.spikes++
  RUN.agents++
  const tier = RUN.tier || TIER[ROLE['spike-runner']] // the spike runner's own tier, for every kind of spike
  const f = await agent(
    `Method: spike. Read ${RUN.root}/methods/spike.md first.\n` +
      `Question: ${req.question}\nWhy it blocks: ${req.why_blocking}\n` +
      `Suggested method: ${req.method_hint || 'your choice'}\nDone when: ${req.done_when || 'the question has an answer with evidence'}\n` +
      `First run: vsdd -C ${RUN.dir} spike open --run ${RUN.id}${stepKey ? ` --step ${stepKey}` : ''} --ref ${safe(req.ref) || 'integration'} --json\n` +
      `If it returns an existing finding, return that finding with cached = true and do nothing else. If it refuses, return ` +
      `a finding with verdict "refused" and its reason. Otherwise run the experiment inside your own worktree only. Write the ` +
      `finding as JSON to a file in the temporary directory, then run: vsdd -C ${RUN.dir} spike record --id <ledger_id> ` +
      `--json-file <that file>  and return the same finding.`,
    {
      agentType: P + 'spike-runner', model: tier.model, effort: tier.effort, schema: S_SPIKE_FINDING, phase: phaseTitle,
      label: `spike ${RUN.spikes}: ${req.question.slice(0, 48)}`,
      isolation: 'worktree', // always, research included: a step does not inherit the agent file's isolation
    },
  )
  return f || { ledger_id: '', question: req.question, verdict: 'inconclusive', answer: 'The spike agent returned nothing.', method: '', evidence: [], confidence: 0, cached: false }
}
```

How `step()` uses it: it runs the agent; if the result asks for a spike and the limits allow, it calls
`runSpike()`, appends the finding to the list the step is given, and runs the step again; if the
limits do not allow, it returns the step as blocked with kind `spike_limit`
([[topic:prelude-steps]]). Spike findings are carried across `gated()` attempts, so a retry does not
ask again. The spike runner's tier is its own, for every kind of spike, and the step passes
`isolation: 'worktree'` itself, because the platform ignores the agent file's setting for a step.
