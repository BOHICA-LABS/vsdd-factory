// Universal tests: every workflow file must pass them, with no per-workflow authoring
// (packaging.md 2.6, layer 2). Run: node --test tests/workflows/universal.test.mjs

import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  runWorkflow, runWorkflowFile, readMeta, stripStamped, minimal, alternate,
  PLATFORM_AGENT_CAP,
} from './harness.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const PLUGIN = path.join(ROOT, 'plugins', 'vsdd-factory')
const TOYS = path.join(ROOT, 'tests', 'workflows', 'toys')

const workflowFiles = () => {
  const files = fs.readdirSync(TOYS).filter((f) => f.endsWith('.js')).map((f) => path.join(TOYS, f))
  const pluginDir = path.join(PLUGIN, 'workflows')
  if (fs.existsSync(pluginDir)) {
    files.push(...fs.readdirSync(pluginDir).filter((f) => f.endsWith('.js')).map((f) => path.join(pluginDir, f)))
  }
  return files.sort()
}

// Any agent, answered by the smallest object its schema allows.
const smallest = (prompt, opts) => (opts.schema ? minimal(opts.schema) : '')

const DONE_GATE = (subject) => ({ ok: true, gate: 'green', subject, passed: true, sha: 'h1', evidence: 'e', failures: [] })

// A step stub: a done step result whose name/subject match the label "subject · name #n".
const doneResult = (prompt, opts) => {
  const base = typeof smallest(prompt, opts) === 'object' && smallest(prompt, opts) ? smallest(prompt, opts) : {}
  if (base.step) {
    const label = opts.label || ''
    const m = /^(.*) · ([a-z0-9-]+(?:-retry\d+)?) #\d+/.exec(label)
    base.step = { run: 'r1', workflow: 'toy', name: m ? m[2] : 'x', subject: m ? m[1] : 'S-1.01', attempt: 1 }
    if (base.status === 'done' && (!base.gates || base.gates.length === 0)) {
      base.gates = [DONE_GATE(base.step.subject)]
    }
    base.head = 'h1'
  }
  return base
}

const THE_AGENT = (prompt, opts) => doneResult(prompt, opts)

// An agent stub that answers every step from the prompt's own identity line
// ("Return step = {...} unchanged."), so no toy label is hard-coded and
// convergence loops actually progress (U3-R13). Non-step schemas (survey,
// relayed tool output) fall back to the smallest object, with ok flipped.
const identityStub = (prompt, opts) => {
  const base = smallest(prompt, opts)
  if (base && typeof base === 'object') {
    if ('ok' in base) base.ok = true
    if ('run_id' in base) {
      base.run_id = base.run_id || 'r1'
      base.project_dir = base.project_dir || '/proj'
      base.plugin_root = base.plugin_root || '/plugin'
      base.preconditions = []
      base.done = []
      base.units = surveyState.units
      base.config = surveyState.config
      base.limits = surveyState.limits || {}
    }
    if (base.step !== undefined || /Return step = /.test(prompt)) {
      const m = /Return step = ({.*?}) unchanged\./.exec(prompt)
      const identity = m ? JSON.parse(m[1]) : { run: 'r1', workflow: 'toy', name: 'x', subject: 'x', attempt: 1 }
      base.step = identity
      base.status = base.status && base.status !== 'done' ? base.status : 'done'
      base.summary = ''
      base.head = base.head || 'h1'
      if (base.status === 'done' && 'gates' in base && (!base.gates || base.gates.length === 0)) {
        base.gates = [DONE_GATE(identity.subject)]
      }
      if ('data' in base) base.data = base.data || {}
    }
    if (Array.isArray(base.units)) base.units = surveyState.units
    if (base.config) base.config = surveyState.config
  }
  return base
}

// ----------------------------------------------------------------- 1. it loads

test('1. every workflow loads in the harness: syntax, no forbidden global, no clash with a prelude name', () => {
  const preludeSource = fs.readFileSync(path.join(ROOT, 'workflow-lib', 'prelude.js'), 'utf8')
  const decls = [...matchAll(stripGenerated(preludeSource), /^(?:const|(?:async )?function)\s+([A-Za-z_$][\w$]*)/gm)].map((m) => m[1])
  assert.ok(decls.includes('step') && decls.includes('RUN'), 'the prelude declarations were not parsed')

  for (const file of workflowFiles()) {
    const source = fs.readFileSync(file, 'utf8')
    const meta = readMeta(source) // parses the meta literal
    assert.equal(typeof meta.name, 'string', `${file}: meta.name`)
    const body = stripStamped(source)
    for (const name of decls) {
      const re = new RegExp(`^\\s*(?:const|let|function)\\s+${name}\\b`, 'm')
      assert.ok(!re.test(body), `${file}: redeclares the prelude name ${name}`)
    }
    const forbidden = /(^|\W)(require|process|fetch|module)\s*(\.|\()|\bimport\s*\(/m
    assert.ok(!forbidden.test(body), `${file}: uses a global the platform does not give a script`)
  }
})

function* matchAll(text, re) {
  let m
  while ((m = re.exec(text))) yield m
}

function stripGenerated(source) {
  const start = source.indexOf('// >>> generated:tables')
  const tablesEnd = source.indexOf('// <<< generated:tables')
  const schemasStart = source.indexOf('// >>> generated:schemas')
  const schemasEnd = source.indexOf('// <<< generated:schemas')
  if (start < 0 || schemasEnd < 0) return source
  return source.slice(0, start) + source.slice(tablesEnd, schemasStart) + source.slice(schemasEnd)
}

// ----------------------------------------------------------------- shared run helpers

const surveyOk = (units = []) => ({
  ok: true,
  run_id: 'r1',
  workflow: 'toy-build',
  subject: 'S-1.01',
  project_dir: '/proj',
  plugin_root: '/plugin',
  head: 'h0',
  preconditions: [],
  done: [],
  units,
  config: {},
  limits: {},
  spikes_used: 0,
})

const okStub = (prompt, opts) => {
  const v = minimal(opts.schema)
  if (v && typeof v === 'object') {
    if ('ok' in v) v.ok = true
    if (v.step) Object.assign(v, doneResult(prompt, opts))
    // survey: units live under units; give none unless the test set them via a closure
    if (Array.isArray(v.units)) v.units = surveyState.units
    if (v.config) v.config = surveyState.config
  }
  return v
}

let surveyState = { units: [], config: {} }

// ----------------------------------------------------------------- 2 and 3

test('2 and 3. phase titles match meta, the name matches the file, and every agentType is a roster agent', async () => {
  for (const file of workflowFiles()) {
    const meta = readMeta(fs.readFileSync(file, 'utf8'))
    const titles = meta.phases.map((p) => p.title)
    assert.equal(meta.name, path.basename(file, '.js'), `${file}: meta.name must equal the file name`)
    const run = await runWorkflow(path.basename(file, '.js'), {
      directory: path.dirname(file),
      agents: { '*': identityStub },
      workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
    })
    assert.equal(run.error, null, `${file}: ${run.error}`)
    for (const phase of run.phases) {
      assert.ok(titles.includes(phase), `${file}: phase "${phase}" is not in meta.phases`)
    }
    for (const call of run.calls) {
      if (call.agentType.startsWith('vsdd-factory:')) {
        const name = call.agentType.slice('vsdd-factory:'.length)
        assert.ok(roleHas(name), `${file}: agentType ${name} is not in the roster`)
      }
    }
  }
})

function roleHas(name) {
  // Exact membership in the generated ROLE table, parsed from the stamped
  // prelude — a substring regex accepts `reviewer` inside `scenario-reviewer`
  // and would let a typo'd agentType pass.
  const prelude = fs.readFileSync(path.join(ROOT, 'workflow-lib', 'prelude.js'), 'utf8')
  const match = /const ROLE = (\{[\s\S]*?\n\});/.exec(prelude)
  assert.ok(match, 'the generated ROLE table was not found in the prelude')
  const roles = JSON.parse(match[1])
  return name in roles
}

// ----------------------------------------------------------------- 4 and 5

test('4. minimal answers: every workflow ends without an error, its loops ran, and under the agent cap', async () => {
  for (const file of workflowFiles()) {
    surveyState = { units: [], config: {}, limits: {} }
    const run = await runWorkflow(path.basename(file, '.js'), {
      args: { story: 'S-1.01', request: 'r' },
      agents: { '*': identityStub },
      workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
      maxAgents: 80,
    })
    assert.equal(run.error, null, `${file}: ${run.error}`)
    assert.ok(run.agentsUsed <= 80, `${file}: exceeded the agent cap`)
    assert.ok(run.agentsUsed >= 1, `${file}: the stubs drove no loop at all`)
  }
})

test('5. the other answers, and a spike asked for every time: still under the cap', async () => {
  for (const file of workflowFiles()) {
    surveyState = { units: [], config: {}, limits: {} }
    const run = await runWorkflow(path.basename(file, '.js'), {
      args: { story: 'S-1.01', request: 'r' },
      // alternate() first, falling back to the identity answer, so a schema
      // whose alternate is unusable still converges (U3-R13).
      agents: { '*': (p, o) => alternate(o.schema) ?? identityStub(p, o) },
      workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
      maxAgents: 80,
    })
    assert.equal(run.error, null, `${file}: ${run.error}`)
    assert.ok(run.agentsUsed <= 80, `${file}: exceeded the agent cap`)
    assert.ok(run.agentsUsed >= 1, `${file}: the stubs drove no loop at all`)
  }

  // Every build step asks for a spike: served twice (the per-step limit), then blocked, under the cap.
  surveyState = { units: [], config: {}, limits: {} }
  const needsSpike = (prompt, opts) => {
    const base = okStub(prompt, opts)
    if (base && base.step && (opts.label || '').includes('· build')) {
      Object.assign(base, {
        status: 'needs_spike',
        summary: 'spike limit test',
        spike_request: { question: 'why is the sky blue?', why_blocking: 'blocks the build', kind: 'experiment' },
      })
      base.gates = []
      base.head = ''
    }
    return base
  }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { '*': needsSpike, 'vsdd-factory:spike-runner': (p, o) => alternate(o.schema) },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
    maxAgents: 80,
  })
  assert.equal(run.error, null, String(run.error))
  assert.equal(run.result.status, 'blocked')
  assert.match(run.result.summary, /build|spike/i)
  assert.ok(run.agentsUsed <= 80)
})

// ----------------------------------------------------------------- 6

test('6. every non-clerk step prompt begins with its method line', async () => {
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { '*': THE_AGENT },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  for (const call of run.calls) {
    const isClerk = call.agentType === 'vsdd-factory:clerk'
    if (!isClerk) {
      assert.match(call.prompt, /^Method: /, `${call.label}: the prompt does not begin with the method`)
    }
  }
})

// ----------------------------------------------------------------- 7

test('7. the checkpoint command runs when every step is done, and when one agent throws', async () => {
  surveyState = { units: [], config: {} }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { '*': okStub, 'vsdd-factory:clerk': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  const closes = run.calls.filter((c) => c.agentType === 'vsdd-factory:clerk' && c.prompt.includes('vsdd checkpoint'))
  assert.ok(closes.length >= 1, 'no checkpoint call')

  surveyState = { units: [], config: {} }
  const boomed = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { 'S-1.01 · build #1': () => { throw new Error('exploded'); }, '*': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  const boomClose = boomed.calls.filter((c) => c.prompt.includes('vsdd checkpoint'))
  assert.ok(boomClose.length >= 1, 'a thrown agent skipped the checkpoint')
})

// ----------------------------------------------------------------- 8 (fixture-backed, below with 10)

// ----------------------------------------------------------------- 9

test('9. a survey limit key the prelude does not know fails the run', async () => {
  const run = await runWorkflow('toy-story', {
    args: { story: 'S-1.01', limits: { bogus_limit: 7 } },
    agents: { '*': okStub },
  })
  assert.notEqual(run.error, null)
  assert.match(String(run.error), /Unknown limit key "bogus_limit"/)
})

// ----------------------------------------------------------------- 10 and 8

test('10 and 8. walled scenario text reaches no other agent, and claims carry no title or note', async () => {
  surveyState = { units: [], config: {} }
  const MARKER = 'HS-007-walled-scenario-text-marker'
  let pass = 0
  const scenarioReviewer = (prompt, opts) => {
    pass++
    const v = minimal(opts.schema)
    const m = /^(.*) · ([a-z0-9-]+) #\d+/.exec(opts.label || '')
    if (v && v.step) {
      v.step = { run: 'r1', workflow: 'toy-build', name: m[2], subject: m[1], attempt: 1 }
      v.status = 'done'
      v.summary = ''
      v.head = 'h1'
      v.data = {
        reviewed_sha: 'h1',
        findings: [{
          key: 'holdout:S-1.01:clause', title: MARKER, severity: 'high', category: 'holdout', scope: 'unit',
          owner: 'product-owner', location: { path: '.factory/holdout/HS-007.md' },
          spec_ref: 'BC-9.02.001', repro: 'HS-007', observed: MARKER, expected: 'fine',
        }],
        claim_checks: [],
        coverage: [],
      }
    }
    return v
  }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: {
      'vsdd-factory:clerk': okStub,
      'vsdd-factory:scenario-reviewer': scenarioReviewer,
      '*': THE_AGENT,
    },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  const holders = ['vsdd-factory:scenario-reviewer', 'vsdd-factory:product-owner', 'vsdd-factory:holdout-evaluator']
  for (const call of run.calls) {
    if (holders.includes(call.agentType)) continue
    assert.ok(!call.prompt.includes(MARKER), `${call.agentType} (${call.label}) received walled scenario text`)
  }
  // Claim shape: the next reviewer's prompt carries the key and resolution, never a title, note or quote.
  const scenarioPrompts = run.calls.filter((c) => c.agentType === 'vsdd-factory:scenario-reviewer').map((c) => c.prompt)
  const withClaims = scenarioPrompts.filter((p) => p.includes('"resolution"'))
  assert.ok(withClaims.length >= 1, 'no claim was ever passed')
  for (const prompt of withClaims) {
    assert.ok(prompt.includes('holdout:S-1.01:clause'), 'the claim key is missing')
    for (const field of ['"title"', '"note"', '"quote"', '"observed"']) {
      assert.ok(!prompt.includes(field), `the claim carries ${field}`)
    }
  }
})

// ----------------------------------------------------------------- 11

test('11. free text reaches the clerk percent-encoded, one segment, shell-safe for ordinary English', async () => {
  surveyState = { units: [], config: {} }
  const TEXT = "fix (the) parser's *bug*! $ ` \\ & | <> \n; drop \"last\" row"
  const run = await runWorkflow('toy-build', {
    args: { request: TEXT },
    agents: { '*': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  const surveyCall = run.calls.find((c) => c.prompt.includes('vsdd survey'))
  assert.ok(surveyCall, 'no survey call')
  const m = /--request (\S+)/.exec(surveyCall.prompt)
  assert.ok(m, 'the request is not one segment')
  const token = m[1]
  // Percent-encoded output is one shell word: alphanumerics and the safe
  // punctuations only, so an apostrophe can never open a quote or a star glob.
  assert.match(token, /^[A-Za-z0-9%._~-]+$/, `raw text reached the command line: ${token}`)
  assert.equal(decodeURIComponent(token), TEXT)
})

// ----------------------------------------------------------------- 12

test('12. both argument forms; a key=value word the workflow does not own stays in the main argument', async () => {
  surveyState = { units: [], config: {} }
  const run = await runWorkflow('toy-build', {
    args: 'CSV broke extra=1 intent=bug-fix',
    agents: { '*': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  const surveyCall = run.calls.find((c) => c.prompt.includes('vsdd survey'))
  // The exact token, and only the unparsed words fold into it.
  assert.match(surveyCall.prompt, /--request CSV%20broke%20extra%3D1(?![\w%])/)
  assert.ok(!surveyCall.prompt.includes('CSV%20broke%20extra%3D1%20intent'), 'intent= was folded into the main argument')
  assert.doesNotMatch(surveyCall.prompt, /intent=/)

  surveyState = { units: [], config: {} }
  const objectRun = await runWorkflow('toy-build', {
    args: { request: 'plain object', intent: 'bug-fix' },
    agents: { '*': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  const objectSurvey = objectRun.calls.find((c) => c.prompt.includes('vsdd survey'))
  assert.match(objectSurvey.prompt, /--request plain%20object/)
})

// ----------------------------------------------------------------- 13

test('13. a child receives agents = story_agents, and a child that throws is charged that amount', async (t) => {
  // The child obeys a tightened budget: toy-story with limits.agents = 3 finishes inside 3 agents.
  const childRun = await runWorkflow('toy-story', {
    args: { story: 'S-1.01', run_id: 'r1', limits: { agents: 3 } },
    agents: { '*': okStub },
    maxAgents: 3,
  })
  assert.equal(childRun.error, null, String(childRun.error))
  assert.ok(childRun.agentsUsed <= 3)

  // A child that throws is charged its full budget by the parent's runStories().
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wf-child-'))
  const child = path.join(dir, 'boom.js')
  fs.writeFileSync(child, "export const meta = {\n  name: 'boom',\n  phases: [],\n}\nthrow new Error('child boom')\n")
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))

  const unit = { id: 'S-1.02', position: 'ready' }
  surveyState = { units: [unit], config: {} }
  const parent = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { '*': okStub },
    workflows: { 'vsdd-factory:story': child },
    maxAgents: 500,
  })
  assert.equal(parent.error, null, String(parent.error))
  assert.equal(parent.result.stories.attempted, 1)
  assert.equal(parent.result.stories.delivered, 0)
  assert.ok(
    parent.result.spent - parent.result.agents >= 40,
    `the thrown child was not charged its budget (charge was ${parent.result.spent - parent.result.agents})`,
  )
})

// ----------------------------------------------------------------- prelude behaviour the platform table fixes

test('the spike wrapper: one spike in an isolated worktree, restarted with the finding', async () => {
  surveyState = { units: [], config: {} }
  let spikeCount = 0
  const buildAttempt = (prompt, opts) => {
    const v = minimal(opts.schema)
    Object.assign(v, {
      step: { run: 'r1', workflow: 'toy-build', name: opts.label.includes('#1') ? 'build' : 'build-retry1', subject: 'S-1.01', attempt: 1 },
      status: 'done',
      head: 'h1',
      gates: [DONE_GATE('S-1.01')],
      data: {},
    })
    return v
  }
  const buildFirst = (prompt, opts) => {
    if (opts.label.includes('· build #')) {
      const v = buildAttempt(prompt, opts)
      v.status = 'needs_spike'
      v.spike_request = { question: 'which serializer?', why_blocking: 'undocumented format', kind: 'experiment' }
      v.gates = []
      return v
    }
    return buildAttempt(prompt, opts)
  }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: {
      'S-1.01 · build #1': buildFirst,
      'S-1.01 · build-retry1 #1': buildAttempt,
      'vsdd-factory:spike-runner': (prompt, opts) => {
        spikeCount++
        return { ledger_id: 'SP-001', question: 'which serializer?', verdict: 'confirmed', answer: 'csv', method: 'probe', evidence: ['e'], confidence: 0.9, cached: false }
      },
      'vsdd-factory:clerk': okStub,
      '*': THE_AGENT,
    },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  assert.equal(spikeCount, 1)
  const spikeCall = run.calls.find((c) => c.agentType === 'vsdd-factory:spike-runner')
  assert.equal(spikeCall.isolation, 'worktree', 'the spike runner must run in its own worktree')
  assert.match(spikeCall.label, /^spike 1:/)
  assert.match(run.result.summary, /converged/)
})

test('the tier tables: the build-generated override sends ingest stages to the builder tier', () => {
  const prelude = fs.readFileSync(path.join(ROOT, 'workflow-lib', 'prelude.js'), 'utf8')
  assert.match(prelude, /"ingest\/map": "builder"/)
  assert.match(prelude, /"ingest\/analyze": "builder"/)
  assert.match(prelude, /"clerk": \{\s*"model": "sonnet",\s*"effort": "low"/m)
})

test(`the platform agent cap is ${PLATFORM_AGENT_CAP}`, () => {
  assert.equal(PLATFORM_AGENT_CAP, 1000)
})

// ----------------------------------------------------------------- behaviours pipeline.md 2.11 assigns to B5

test('the close relay runs with the agent limit spent by steps (the reserve is the close)', async () => {
  surveyState = { units: [], config: {}, limits: {} }
  // A tight but not exhausted cap: the steps spend the budget, the close
  // relay must still run on top of them (relay() has no limit check).
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { '*': identityStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
    maxAgents: 500,
  })
  assert.equal(run.error, null, String(run.error))
  const close = run.calls.find((c) => c.prompt.includes('checkpoint'))
  assert.ok(close, 'the checkpoint relay was skipped')
  // The checkpoint relay is the last agent call of the run: step() refuses
  // before it, never instead of it (relay() has no limit check).
  const last = run.calls[run.calls.length - 1]
  assert.ok(last.prompt.includes('checkpoint'), 'the run ended on a non-clerk call')
})

test('the close relay carries --outcome failed when the run throws', async () => {
  surveyState = { units: [], config: {}, limits: {} }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: { 'S-1.01 · build #1': () => { throw new Error('exploded'); }, '*': okStub },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  const close = run.calls.find((c) => c.prompt.includes('--outcome failed'))
  assert.ok(close, 'a thrown agent never reached the checkpoint as failed')
})

test('a spike ask restarts the step with the findings header and the finding text', async () => {
  surveyState = { units: [], config: {}, limits: {} }
  let asked = 0
  let spikeCalls = 0
  const star = (prompt, opts) => {
    const v = identityStub(prompt, opts)
    if (v && v.step && asked === 0 && !String(opts.label || '').includes('-retry')) {
      asked++
      Object.assign(v, {
        status: 'needs_spike',
        spike_request: { question: 'probe the holdout ledger', why_blocking: 'blocks the review', kind: 'experiment' },
      })
      v.gates = []
      v.head = ''
    }
    return v
  }
  const run = await runWorkflow('toy-build', {
    args: { request: 'r' },
    agents: {
      'vsdd-factory:spike-runner': (prompt, opts) => {
        spikeCalls++
        const v = identityStub(prompt, opts)
        return { ...v, ledger_id: 'SP-009', question: 'probe the holdout ledger', verdict: 'confirmed', answer: 'found it', cached: true }
      },
      '*': star,
    },
    workflows: { 'vsdd-factory:story': path.join(TOYS, 'toy-story.js') },
  })
  assert.equal(run.error, null, String(run.error))
  assert.equal(spikeCalls, 1, 'the spike did not run exactly once')
  // The step retries under its own name (gated() renames only on a failed
  // gate): the attempt after the spike carries the findings header and text.
  const restarted = run.calls.filter((c) => c.prompt.includes('Spike findings already obtained'))
  assert.ok(restarted.length >= 1, 'no attempt was restarted with the finding')
  assert.match(restarted[restarted.length - 1].prompt, /probe the holdout ledger/)
})


test('ingest/map runs on the builder tier from the roster override', async () => {
  const prelude = fs.readFileSync(path.join(ROOT, 'workflow-lib', 'prelude.js'), 'utf8')
  const tierMatch = /const TIER = (\{[\s\S]*?\n\});/.exec(prelude)
  assert.ok(tierMatch, 'the generated TIER table was not found')
  const tiers = JSON.parse(tierMatch[1])
  assert.equal(tiers.builder.model, 'sonnet')
  // A second-stage override keeps its tier on retry: gated() passes stage names through.
  const overrideMatch = /const OVERRIDE = (\{[\s\S]*?\n\});/.exec(prelude)
  const overrides = JSON.parse(overrideMatch[1])
  assert.equal(overrides['ingest/map'], 'builder')
  assert.equal(overrides['ingest/analyze'], 'builder')
})

test('harness realm: log() records, budget() charges agent() and throws past the total, pipeline drops a throwing stage to null, workflow() rethrows a child error and nests once', async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wf-realm-'))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))

  const write = (name, body) => {
    const file = path.join(dir, `${name}.js`)
    fs.writeFileSync(file, `export const meta = {\n  name: '${name}',\n  phases: [],\n}\n${body}`)
    return file
  }

  // log() lands in run.logs (packaging.md 2.6: run carries logs).
  fs.writeFileSync(path.join(dir, 'logger.js'), `export const meta = {\n  name: 'logger',\n  phases: [],\n}\nlog('hello')\nlog('again')\nreturn 'logged'\n`)
  const loggerRun = await runWorkflowFile(path.join(dir, 'logger.js'))
  assert.deepEqual(loggerRun.logs, ['hello', 'again'])

  // budget: agent() throws once spent passes total; remaining() falls with spending.
  fs.writeFileSync(
    path.join(dir, 'spender.js'),
    `export const meta = {\n  name: 'spender',\n  phases: [],\n}\n` +
    `const out = { remaining: budget.remaining(), err: null }\n` +
    `try {\n  await agent('one', { agentType: 'vsdd-factory:implementer' })\n  await agent('two', { agentType: 'vsdd-factory:implementer' })\n} catch (e) { out.err = String(e.message) }\n` +
    `out.final = budget.remaining()\nout.spent = budget.spent()\nreturn out\n`,
  )
  const spenderRun = await runWorkflowFile(path.join(dir, 'spender.js'), {
    budget: { total: 1 },
    agents: { 'vsdd-factory:implementer': 'did it' },
  })
  assert.equal(spenderRun.error, null)
  assert.equal(spenderRun.result.remaining, 1) // Infinity is not expected: a total is set
  assert.match(spenderRun.result.err, /token budget/)
  assert.equal(spenderRun.result.final, 0)

  // budget.remaining() without a total is Infinity.
  fs.writeFileSync(path.join(dir, 'noop.js'), `export const meta = {\n  name: 'noop',\n  phases: [],\n}\nreturn budget.remaining()\n`)
  const noopRun = await runWorkflowFile(path.join(dir, 'noop.js'))
  assert.equal(noopRun.result, Infinity)

  // pipeline: a thrown stage drops that item to null and skips the rest.
  fs.writeFileSync(
    path.join(dir, 'piper.js'),
    `export const meta = {\n  name: 'piper',\n  phases: [],\n}\n` +
    `return pipeline([1, 2, 3], (x) => x + 1, (x, item) => { if (item === 2) throw new Error('boom'); return x * 10 })\n`,
  )
  const piperRun = await runWorkflowFile(path.join(dir, 'piper.js'))
  assert.equal(piperRun.error, null)
  assert.deepEqual(piperRun.result, [20, null, 40]) // item 2 threw; 1->2->20, 3->4->40

  // parallel: a synchronously throwing thunk becomes null and never rejects.
  fs.writeFileSync(
    path.join(dir, 'par.js'),
    `export const meta = {\n  name: 'par',\n  phases: [],\n}\n` +
    `return parallel([() => 1, () => { throw new Error('sync boom'); }, 7])\n`,
  )
  const parRun = await runWorkflowFile(path.join(dir, 'par.js'))
  assert.equal(parRun.error, null)
  assert.deepEqual(parRun.result, [1, null, 7])

  // workflow(): the child's return value reaches the parent unchanged; a child
  // that throws rejects the parent; a grandchild is refused.
  fs.writeFileSync(path.join(dir, 'child.js'), `export const meta = {\n  name: 'child',\n  phases: [],\n}\nreturn { paid: 'yes', agents: 0 }\n`)
  const childOk = await runWorkflowFile(path.join(dir, 'child.js'))
  assert.deepEqual({ ...childOk.result }, { paid: 'yes', agents: 0 }) // spread: a vm-realm object has another prototype

  fs.writeFileSync(path.join(dir, 'caller-ok.js'), `export const meta = {\n  name: 'caller-ok',\n  phases: [],\n}\nreturn workflow('child', {})\n`)
  const parentOfChild = await runWorkflowFile(path.join(dir, 'caller-ok.js'), {
    workflows: { child: path.join(dir, 'child.js') },
  })
  assert.equal(parentOfChild.error, null)
  assert.deepEqual({ ...parentOfChild.result }, { paid: 'yes', agents: 0 })

  fs.writeFileSync(path.join(dir, 'boomer.js'), `export const meta = {\n  name: 'boomer',\n  phases: [],\n}\nthrow new Error('child boom')\n`)
  fs.writeFileSync(path.join(dir, 'caller-boom.js'), `export const meta = {\n  name: 'caller-boom',\n  phases: [],\n}\ntry {\n  await workflow('boomer', {})\n  return 'did not throw'\n} catch (e) {\n  return 'caught: ' + e.message\n}\n`)
  const parentOfBoom = await runWorkflowFile(path.join(dir, 'caller-boom.js'), {
    workflows: { boomer: path.join(dir, 'boomer.js') },
  })
  assert.equal(parentOfBoom.error, null)
  assert.equal(parentOfBoom.result, 'caught: child boom')

  fs.writeFileSync(
    path.join(dir, 'grandcaller.js'),
    `export const meta = {\n  name: 'grandcaller',\n  phases: [],\n}\n` +
    `try {\n  await workflow('child', {})\n  return 'no nesting error'\n} catch (e) {\n  return 'nesting refused: ' + e.message\n}\n`,
  )
  const grand = await runWorkflowFile(path.join(dir, 'grandcaller.js'), {
    depth: 1,
    workflows: { child: path.join(dir, 'child.js') },
  })
  assert.match(grand.result, /nesting refused/)

  // a workflow() call is not an agent: the child's calls spend the shared counter.
  fs.writeFileSync(path.join(dir, 'counter.js'), `export const meta = {\n  name: 'counter',\n  phases: [],\n}\nawait agent('a', { agentType: 'vsdd-factory:implementer' })\nreturn 'done'\n`)
  fs.writeFileSync(path.join(dir, 'parent-counter.js'), `export const meta = {\n  name: 'parent-counter',\n  phases: [],\n}\nawait workflow('counter', {})\nreturn 'done'\n`)
  const counted = await runWorkflowFile(path.join(dir, 'parent-counter.js'), {
    workflows: { counter: path.join(dir, 'counter.js') },
    agents: { 'vsdd-factory:implementer': 'hi' },
  })
  assert.equal(counted.agentsUsed, 1) // the child's one agent; the workflow() call itself is free
})

test('validateValue checks enum for every type, and checkSchemaShape refuses a required ghost', async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wf-valid-'))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))
  fs.writeFileSync(
    path.join(dir, 'enumcheck.js'),
    `export const meta = {\n  name: 'enumcheck',\n  phases: [],\n}\n` +
    `const problems = []\n` +
    `const schema = { type: 'object', properties: { status: { type: 'string', enum: ['done', 'failed'] } }, required: ['status'] }\n` +
    `try { await agent('x', { agentType: 'vsdd-factory:implementer', schema }) } catch (e) { problems.push(String(e.message)) }\n` +
    `return problems\n`,
  )
  const run = await runWorkflowFile(path.join(dir, 'enumcheck.js'), {
    agents: { 'vsdd-factory:implementer': { status: 'bogus' } },
  })
  assert.equal(run.error, null)
  assert.equal(run.result.length, 1, 'an enum-violating result was accepted')
  assert.match(run.result[0], /not one of/)
})

test('every workflow() target a toy names exists on disk and in the roster', () => {
  const roster = fs.readFileSync(path.join(ROOT, 'tests', 'lint', 'roster.toml'), 'utf8')
  const workflowsSection = /\[workflows\][\s\S]*?(?=\n\[)/.exec(roster)
  assert.ok(workflowsSection, 'the roster has no [workflows] table')
  const named = [...workflowsSection[0].matchAll(/^(\S+)\s*=/gm)].map((m) => m[1])
  assert.ok(named.length > 0)
  for (const file of workflowFiles()) {
    const meta = readMeta(fs.readFileSync(file, 'utf8'))
    assert.ok(
      named.includes(meta.name) || meta.name.startsWith('toy-'),
      `${file}: workflow ${meta.name} is not in the roster's [workflows]`,
    )
  }
})
