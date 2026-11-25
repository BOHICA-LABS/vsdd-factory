---
kind: topic
id: prelude-steps
package: B4a
sources: [pipeline.md#2.11.1]
---

# Prelude steps

The prelude functions that run a step: `brief()`, `step()`, `gated()` and `relay()`. The constants,
parser and schemas around them are [[topic:prelude]]; the spike wrapper is
[[topic:prelude-run-spike]].

```js
// What every step is told. The method comes first. The step identity and the closing command come last, so
// both survive a compaction of the step. o.closing is a vsdd command without the leading "vsdd". With o.each
// (the fast-route story-draft step: 'story you wrote') the command holds <id> and is run once per item, and
// gated() requires every item's entry to pass.
function brief(id, n, task, spikes, o) {
  return [
    `Method: ${o.method}. Read ${RUN.root}/methods/${o.method}.md first.`,
    task,
    `Project directory: ${RUN.dir}.`,
    'You are running as a workflow step. If you cannot finish without an experiment, stop, return status ' +
      '"needs_spike" and fill spike_request with one question. A spike will be run for you in isolation and ' +
      'this step will be started again with the finding.',
    spikes.length ? `Spike findings already obtained for this step: ${JSON.stringify(spikes)}` : '',
    n > 1 || o.dirty ? 'Earlier work on this step may be on disk, some of it uncommitted. Check before redoing it.' : '',
    `Return step = ${JSON.stringify({ ...id, attempt: n })} unchanged.`,
    o.closing
      ? `Finish by running${o.each ? `, once for each ${o.each}` : ''}: vsdd -C ${RUN.dir} ${o.closing} --wait 540 --json  with a Bash timeout of 600000. ` +
        'If it prints state "pending", run the same command again. Put its final output in gates.'
      : '',
  ].filter(Boolean).join('\n')
}

// Run one step. o.method is required. Retries an unusable result, and serves spike requests within the limits.
// o.spikes, when given, is the list gated() carries across its attempts. The identity carries the method, and
// for a review, fix or arbitrate step the pass (o.pass) and the review loop (o.loop, 1 when not given), which
// is how the tool finds a unit's passes among the step records. A name is unique within its run and unit.
async function step(type, name, subject, task, o = {}) {
  const tier = RUN.tier || TIER[OVERRIDE[`${RUN.wf}/${o.stage || name}`] || ROLE[type]]
  const id = { run: RUN.id, workflow: RUN.wf, name, subject, method: o.method, ...(o.pass ? { loop: o.loop || 1, pass: o.pass } : {}) }
  const spikes = o.spikes || []
  let bad = 0
  for (let n = 1; ; n++) {
    if (RUN.agents >= RUN.limits.agents || (budget.total && budget.remaining() < 20000))
      return made(id, 'blocked', 'agent_budget', 'The agent or token budget for this run is spent.')
    RUN.agents++
    const r = await agent(brief(id, n, task, spikes, o), {
      agentType: P + type, model: tier.model, effort: tier.effort, schema: stepSchema(o.data),
      phase: o.phase, label: `${subject} · ${name} #${n}`,
    })
    if (!r || !r.step || r.step.name !== name || r.step.subject !== subject || r.step.method !== o.method) {
      if (++bad >= RUN.limits.attempts) return made(id, 'failed', 'other', 'The step returned no usable result.')
      continue
    }
    if (r.status !== 'needs_spike') return r
    const q = r.spike_request
    if (!q || !q.question) return made(id, 'failed', 'other', 'The step asked for a spike without a question.')
    const over = spikes.length >= RUN.limits.spikes_per_step || RUN.spikes >= RUN.limits.spikes_per_run
    if (over) return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: 'The spike limit for this step or run is reached.', question: q.question } }
    // Refused above before any agent starts. The tool also counts per run id, so parent and children share the limit.
    const f = await runSpike({ ...q, ref: q.ref || r.head || '' }, `${RUN.wf}/${subject}/${name}.${n}`, o.phase)
    if (f.verdict === 'refused') return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: f.answer, question: q.question } }
    spikes.push(f)
  }
}

// Run a step until the gate it must finish with passes on the commit it left behind. o.closing is that gate's
// command. Spike findings are carried from one attempt to the next, so a retry does not ask again. With o.each
// the step returns one entry for the gate per item: every entry must pass and have its unit in data.units, and
// the retry note names each one that did not.
async function gated(type, name, subject, task, gate, o = {}) {
  let note = ''
  const spikes = []
  for (let i = 1; i <= RUN.limits.attempts; i++) {
    // Keep o.stage for every attempt, so the OVERRIDE lookup resolves the retry's tier.
    const r = await step(type, i === 1 ? name : `${name}-retry${i - 1}`, subject, task + note, { ...o, stage: o.stage || name, spikes })
    if (r.status !== 'done') return r
    const all = (r.gates || []).filter((x) => x.gate === gate)
    const gs = o.each ? all : all.slice(0, 1)
    const units = (r.data && r.data.units) || []
    const ok = (g) => g.passed && g.sha === r.head
    const bad = gs.filter((g) => !ok(g) || (o.each && !units.some((u) => u.id === g.subject)))
    if (gs.length && !bad.length) return r
    const why = bad.map((g) => (o.each ? `${g.subject}: ` : '') + (ok(g) ? 'passed, but its unit is not in data.units' : g.failures.join('; ')))
    note = `\nThe ${gate} gate did not pass last time: ${gs.length ? why.join(' | ').slice(0, 1500) : 'no gate result was returned'}. Deal with that first.` +
      (o.each ? ` Then run the closing command again for every ${o.each}, and return every unit.` : '')
  }
  return made({ run: RUN.id, workflow: RUN.wf, name, subject, method: o.method }, 'blocked', 'gate', `The ${gate} gate still fails after ${RUN.limits.attempts} attempts.`)
}

// Run one vsdd command through the clerk and return its JSON output. The command is a single segment,
// "vsdd -C <dir> ...", with no cd in front and no redirect, so the clerk's wall and the auto-allow rule accept
// it. A long command returns state "pending" while its job runs; the clerk runs the same command again. One
// retry if the clerk returns nothing.
async function relay(name, subject, command, schema, phaseTitle) {
  const full = RUN.dir ? command.replace(/^vsdd /, `vsdd -C ${RUN.dir} `) : command
  for (let n = 1; n <= 2; n++) {
    RUN.agents++
    const r = await agent(
      `Run exactly this command, with a Bash timeout of 600000, and return the JSON it prints as your result. ` +
        `If the JSON has state "pending", run the same command again, and repeat until it does not. ` +
        `Do not interpret, repair or summarise it. If it prints no JSON, return ${JSON.stringify(emptyOf(schema))} with "error" set to what it printed.\n${full}`,
      {
        agentType: P + 'clerk', model: (RUN.tier || TIER.clerk).model, effort: (RUN.tier || TIER.clerk).effort, schema,
        phase: phaseTitle, label: `${subject} · ${name}${n > 1 ? ' (retry)' : ''}`,
      },
    )
    if (r) return r
  }
  return { ...emptyOf(schema), ok: false, error: 'The clerk step returned nothing, twice.' } // every field the script reads exists
}
// The empty object of an output schema: every required field, as '', 0, false, [] or {}, valid
// against the schema it is returned for (CATCHUP-R5): an object fills its own required members the
// same way, and the relayed-output enums admit '' so the empty string is one of their options.
function emptyOf(s) {
  const empty = (p) => (p.type === 'array' ? [] : p.type === 'object' ? emptyOf(p) : p.type === 'boolean' ? false : p.type === 'string' ? '' : 0)
  return Object.fromEntries((s.required || []).map((k) => [k, empty((s.properties || {})[k] || {})]))
}
```
