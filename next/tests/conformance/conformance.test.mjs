// Conformance: the judge and loop cases run through the prelude in Node
// (packaging/tests-conformance.md, plan step-4 verdicts.md 3.5, decision D11).
// Run: node --test tests/conformance/conformance.test.mjs
//
// The runner writes a temporary workflow script — meta, the stamped prelude,
// and a driver that turns each case into judge() or converge() calls — and
// runs it once through the harness, the way universal.test.mjs runs a toy.
// The pytest runner (test_conformance.py) runs the same files through
// vsdd.state.convergence and starts this runner too.

import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { runWorkflowFile } from '../workflows/harness.mjs'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(HERE, '..', '..')
const PRELUDE = path.join(ROOT, 'workflow-lib', 'prelude.js')
const CASES = path.join(HERE, 'cases')

const cases = fs.readdirSync(CASES).filter((f) => f.endsWith('.json')).sort().map((f) => {
  const parsed = JSON.parse(fs.readFileSync(path.join(CASES, f), 'utf8'))
  if (parsed.name !== f.slice(0, -5)) throw new Error(`case ${f} does not name itself`)
  return parsed
})

const roundTrip = (value) => JSON.parse(JSON.stringify(value))

// The driver body: one result per case, computed inside the harness's VM
// with the prelude's own judge() and converge().
const DRIVER = `
const copy = (value) => JSON.parse(JSON.stringify(value))
const results = {}
for (const c of args.cases) {
  if (c.kind === 'judge') {
    results[c.name] = { result: judge(c.input.findings, c.input.perimeter, c.input.waived) }
  } else if (c.kind === 'loop') {
    const passes = c.input.passes || []
    const trace = []
    const each = { entry: null }
    const ctx = {
      perimeter: c.input.perimeter,
      subject: c.input.subject,
      cap: c.input.cap,
      used: c.input.used || 0,
      waived: (c.input.waived || []).slice(),
    }
    if (c.input.arbiter) {
      ctx.arbitrate = (open, pass, notes) => {
        if (each.entry) {
          each.entry.arbitrate = {
            keys: (open || []).map((f) => f.key),
            notes: Array.from(notes || [], (pair) => [pair[0], pair[1]]),
          }
        }
        return copy(c.input.arbitration || null)
      }
    }
    ctx.review = (p, claims, gaps) => {
      const holder = passes[p - 1]
      if (!holder) throw new Error('the case runs out of passes: ' + p)
      const entry = { pass: p, claims: copy(claims), gaps: copy(gaps) }
      trace.push(entry)
      each.entry = entry
      return copy(holder.review)
    }
    ctx.fix = (open, advisory, p) => {
      if (each.entry) each.entry.fix = (open || []).map((f) => f.key)
      const holder = passes[p - 1]
      return copy(holder ? holder.fix : null)
    }
    const verdict = await converge(ctx)
    results[c.name] = { verdict }
    if (c.expect.trace !== undefined) results[c.name].trace = trace
  } else {
    throw new Error('unknown case kind ' + c.kind)
  }
}
return results
`

async function runCases(selected) {
  const prelude = fs.readFileSync(PRELUDE, 'utf8')
  const script = [
    'export const meta = {',
    "  name: 'conformance-driver',",
    "  description: 'Temporary driver: the conformance judge and loop cases against the prelude',",
    "  phases: [{ title: 'Cases' }],",
    '}',
    '// the prelude (workflow-lib/prelude.js) follows, verbatim',
    prelude,
    '// end of the prelude',
    "phase('Cases')",
    DRIVER,
  ].join('\n')
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'conformance-'))
  const file = path.join(dir, 'driver.js')
  try {
    fs.writeFileSync(file, script, 'utf8')
    const run = await runWorkflowFile(file, { args: { cases: selected } })
    if (run.error) throw run.error
    return run.result
  } finally {
    fs.rmSync(dir, { recursive: true, force: true })
  }
}

const judgeAndLoop = cases.filter((c) => c.kind === 'judge' || c.kind === 'loop')

// The results are computed once; one test per case, named by its file stem,
// so a failing case never hides a later disagreement
// (plan step-4 verdicts.md 7; S4R-verdicts-14).
let resultsPromise = null
const resultsOnce = () => (resultsPromise ??= runCases(judgeAndLoop))

for (const c of judgeAndLoop) {
  test(c.name, async () => {
    const results = await resultsOnce()
    const got = results[c.name]
    assert.ok(got, `case ${c.name} produced no result`)
    assert.deepEqual(
      roundTrip(got.verdict ?? got.result),
      roundTrip(c.expect.verdict ?? c.expect.result),
      `case ${c.name}`,
    )
    if (c.expect.trace !== undefined) {
      assert.deepEqual(roundTrip(got.trace), roundTrip(c.expect.trace), `case ${c.name} trace`)
    }
  })
}

test('a case that runs out of passes fails', async () => {
  const openFinding = {
    key: 'code:a', title: 't', severity: 'high', category: 'code', scope: 'unit',
    owner: 'implementer', location: { path: 'x.py' }, spec_ref: 'BC-1', repro: '',
    observed: 'o', expected: 'e',
  }
  const stretched = {
    name: 'zz-runs-out', about: 'probe', kind: 'loop',
    input: {
      perimeter: 'story', subject: 'S-9.01', cap: 5, used: 0, waived: [], arbiter: true,
      passes: [{ review: { ok: true, head: 'h1', findings: [openFinding], claim_checks: [], coverage_missing: [] }, fix: { ok: true, resolutions: [] } }],
      arbitration: null,
    },
    expect: {},
  }
  await assert.rejects(() => runCases([stretched]), /runs out of passes/)
})
