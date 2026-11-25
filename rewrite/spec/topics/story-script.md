---
kind: topic
id: story-script
package: W2
sources: [pipeline.md#2.11.2, pipeline.md#2.11]
---

# story.js skeleton

The story.js skeleton. The playbook it implements is [[workflow:story]]; the prelude stamped at its
head is [[topic:prelude]]. These skeletons are illustrations of shape, not the product; a builder
ports what is written here.

```js
export const meta = {
  name: 'story',
  description: 'Deliver one story: failing tests, implementation, review to convergence, evidence, merge',
  whenToUse: 'Run for one story id. Normally started by the wave, feature, verify, maintain or multi workflows.',
  phases: [
    { title: 'Survey' }, { title: 'Tests' }, { title: 'Implement' },
    { title: 'Review' }, { title: 'Evidence' }, { title: 'Deliver' },
  ],
}

// >>> prelude … <<< prelude   (the stamped prelude, [[topic:prelude]], stamped here)

RUN.wf = 'story'
const a = parseArgs(args, 'story', ['repo'])
const sid = safe(a.story)
if (!sid) return { workflow: 'story', subject: '', status: 'failed', summary: 'No valid story id was given.' }

phase('Survey')
let u = a.position // set when a parent passes a unit the tool returned: from its survey, or from an admission
let surveyNext = null
if (u) {
  RUN.id = a.run_id; RUN.dir = a.project_dir; RUN.root = a.plugin_root; RUN.tier = a.tier || null
  RUN.limits = { ...RUN.limits, ...(a.limits || {}) } // the parent passes agents = story_agents and this child's share of spikes
} else {
  const s = await relay('survey', sid, `vsdd survey story --subject ${sid}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --json`, S_SURVEY, 'Survey')
  const unmet = s.ok ? s.preconditions.filter((p) => !p.met) : []
  if (!s.ok || unmet.length || !s.units.length)
    return { workflow: 'story', subject: sid, status: 'blocked', summary: s.error || unmet.map((p) => `${p.name}: ${p.detail}`).join('; '), next: s.next }
  RUN.id = s.run_id; RUN.dir = s.project_dir; RUN.root = s.plugin_root; RUN.tier = s.config.tier_override || null
  RUN.limits = { ...RUN.limits, ...s.limits }
  u = s.units[0]
  surveyNext = s.next
}

const ORDER = ['ready', 'red', 'green', 'reviewed', 'evidenced', 'delivered']
const before = (p) => ORDER.indexOf(u.position) < ORDER.indexOf(p)
const out = { workflow: 'story', subject: sid, passes: 0, advisory: 0, deferred: [], pr: '', merged_sha: '' }
const end = async (status, summary, failed) => {
  let next = surveyNext
  if (!a.position) { // standalone run: the parent's checkpoint does this otherwise
    const c = await relay('close', sid, `vsdd checkpoint story --run ${RUN.id}${failed ? ' --outcome failed' : ''} --json`, S_CLOSE, 'Deliver')
    next = c.next || next
  }
  return { ...out, status, summary, next, agents: RUN.agents, spikes: RUN.spikes }
}
const where = `Story ${sid}. Work only in the worktree ${u.worktree}. Story file: ${u.story_path}. Contracts: ${u.bcs.join(', ')}.`
const gate = (name) => `gate ${name} --story ${sid} --run ${RUN.id}` // a closing command; brief() adds "vsdd -C <dir>" and the wait
const pick = (r, name) => (r.gates || []).find((x) => x.gate === name) || {}
// The survey resolves the story's kind row - the shipped story-kind rows of the policy table
// ([[table:story-kinds]]) - into the unit: tests_agent and the resolved tests_method, impl_agent and
// the resolved impl_method, and red_gate for whether the kind has one. A story's tests_method /
// impl_method frontmatter has already been applied, so there is no second kind table here.
const K = u
let head = u.head
let patch = u.diff_patch // the latest diff patch; each green gate result replaces it

try { // everything after a successful survey is inside try, so the checkpoint always runs (the body is not re-indented here)

if (before('red') && K.tests_agent) {
  phase('Tests')
  const r = await gated(K.tests_agent, 'tests', sid,
    `${where} Write a failing test for every acceptance criterion, with the stubs the tests need to build, and commit them.`,
    'red', { phase: 'Tests', method: K.tests_method, closing: gate('red'), dirty: u.dirty })
  if (r.status !== 'done') return end(r.status === 'failed' ? 'failed' : 'blocked', r.summary)
  head = r.head
}

if (before('green')) {
  phase('Implement')
  const impl = (name, more) => gated(K.impl_agent, name, sid,
    `${where} ` + (K.tests_agent
      ? 'Make the failing tests pass one at a time with the least code. The tests frozen at the red gate are read-only for you. ' +
        'If a frozen test is wrong, stop: return status "blocked" and list it in data.test_defects.'
      : 'Build what the story asks for.') + (more || ''),
    'green', { phase: 'Implement', method: K.impl_method, closing: gate('green'), dirty: u.dirty, data: S_IMPLEMENT_DATA })
  let r = await impl('implement')
  const defects = r.data.test_defects || [] // a result the prelude made itself has empty data
  if (K.tests_agent && r.status === 'blocked' && defects.length) {
    // Only the test-writer may change a frozen test. It corrects the ones that are wrong and re-freezes, once;
    // then the implementer runs once more.
    const t = await gated('test-writer', 'test-defects', sid,
      `${where} The implementer reports these frozen tests as wrong: ${JSON.stringify(defects)}. ` +
        'Correct each one that is wrong; for each you leave unchanged, say why in your summary.',
      'red', { phase: 'Implement', method: 'test-fixes', closing: `${gate('red')} --refreeze` })
    if (t.status !== 'done') return end(t.status === 'failed' ? 'failed' : 'blocked', t.summary)
    r = await impl('implement-2', ` The test-writer answered your report of wrong tests: ${t.summary}`)
  }
  if (r.status !== 'done') return end(r.status === 'failed' ? 'failed' : 'blocked', r.summary)
  head = r.head; patch = pick(r, 'green').diff_patch || patch
}

// One review loop, used for the full diff (loop 1) and again for any commits added after it converged
// (loop 2 onward, one per delivery round). The loop and pass go into every step's identity and name, so a
// later loop never overwrites the records of an earlier one and the tool can find each pass ([[topic:verdicts]]).
const lenses = [['adversary', 'review-story-diff']]
  .concat(['critical', 'high'].includes(u.criticality) || u.security ? [['security-reviewer', 'security-review']] : [])
const testOwned = (f) => f.owner === 'test-writer' // findings of category test or coverage
const review = (loop, from, cap, must, used) => converge({
  perimeter: 'story', subject: sid, cap, used, waived: u.waived || [],
  review: async (pass, claims, gaps) => {
    const rs = await parallel(lenses.map(([l, m]) => () => step(l, `review-${l}-l${loop}-p${pass}`, sid,
      `${where} Tree root: ${u.tree}. Review the diff patch ${patch} (${from}...${head}) against the story and its contracts. ` +
        `Test output: ${u.test_output}. Review rules: ${u.review_rules || 'none'}. Account for each of: ${must.join(', ')}. ` +
        `Claims to verify from the last pass: ${JSON.stringify(claims)}. Not yet covered: ${gaps.join(', ') || 'nothing'}.`,
      { phase: 'Review', method: m, data: S_REVIEW_DATA, loop, pass })))
    if (rs.some((r) => r && r.status === 'blocked')) return { ok: false, blocked: true } // a reviewer refused, or lacked an input
    if (rs.some((r) => !r || r.status !== 'done' || r.data.reviewed_sha !== head)) return { ok: false }
    const covered = (item) => rs.some((r) => r.data.coverage.some((c) => c.item === item && c.state !== 'skipped'))
    return {
      ok: true, head, findings: rs.flatMap((r) => r.data.findings), claim_checks: rs.flatMap((r) => r.data.claim_checks),
      coverage_missing: must.filter((item) => !covered(item)),
    }
  },
  fix: async (open, advisory, pass) => {
    // Routed on owner. The test-writer goes first: only it may change a frozen test, and it then records the
    // change with the re-freeze, so the green gate that follows does not fail on "frozen tests unchanged".
    // A strengthened test may fail on the current code. The re-freeze accepts that, and the implementer then
    // runs even with no finding of its own, to make the re-frozen tests pass.
    let resolutions = [], red = false
    for (const who of ['test-writer', 'implementer']) {
      const tw = who === 'test-writer'
      const mine = open.filter((f) => testOwned(f) === tw)
      if (!mine.length && (tw || !red)) continue
      const r = await gated(who, `fix-${who}-l${loop}-p${pass}`, sid,
        `${where} Fix each blocking finding, or dispute it with evidence: ${JSON.stringify(mine)}. ` +
          (red && !tw ? 'Tests the test-writer strengthened and re-froze now fail on this code: make them pass. ' : '') +
          `Fix these too where it is cheap: ${JSON.stringify(advisory.filter((f) => testOwned(f) === tw).slice(0, 10))}.` +
          (tw ? ` After the re-freeze below, also run: vsdd -C ${RUN.dir} ${gate('green')} --wait 540 --json  and add its output to gates.` : ''),
        tw ? 'red' : 'green',
        { phase: 'Review', method: tw ? 'test-fixes' : 'fix-findings', closing: tw ? `${gate('red')} --refreeze` : gate('green'), data: S_FIX_DATA, loop, pass })
      if (r.status !== 'done') return { ok: false }
      head = r.head; patch = pick(r, 'green').diff_patch || patch
      if (tw) red = !pick(r, 'green').passed // no passing green result after the re-freeze: the code must change
      resolutions = resolutions.concat(r.data.resolutions)
    }
    return { ok: true, resolutions }
  },
  arbitrate: async (open, pass, notes) => {
    const disputes = open.map((f) => ({ key: f.key, disputed: notes.get(f.key) || '' }))
    const r = await step('architect', `arbitrate-l${loop}`, sid,
      `${where} A reviewer and a builder still disagree after ${cap} passes. Rule on each finding: ${JSON.stringify(open)} ` +
        `The builder disputed these, with its note: ${JSON.stringify(disputes)}`,
      { phase: 'Review', method: 'arbitrate', data: S_RULING_DATA, loop, pass })
    return r.status === 'done' ? r.data : null
  },
})

if (before('reviewed')) {
  phase('Review')
  const v = await review(1, u.base, RUN.limits.passes, u.acs, u.rounds_used || 0)
  out.passes = v.passes; out.deferred = v.deferred; out.advisory = v.advisory.length
  if (v.outcome !== 'converged') return end(v.outcome === 'failed' ? 'failed' : 'blocked', `Review did not converge (${v.reason}).`)
}

if (before('evidenced') && u.demo_required) {
  phase('Evidence')
  const r = await gated('demo-recorder', 'evidence', sid,
    `${where} Record evidence for each user-visible acceptance criterion.`,
    'demo', { phase: 'Evidence', method: 'demo', closing: gate('demo') })
  if (r.status !== 'done') return end('blocked', r.summary)
}

if (before('delivered')) {
  phase('Deliver')
  for (let round = 1; ; round++) {
    // One clerk covers the whole delivery, the CI wait included: the command re-attaches to its job while it is pending.
    const d = await relay(`deliver-${round}`, sid, `vsdd story deliver --story ${sid} --run ${RUN.id} --wait 540 --json`, S_DELIVERY, 'Deliver')
    if (d.state === 'merged') { out.pr = d.pr; out.merged_sha = d.merged_sha; break }
    if (d.state === 'awaiting_merge') return end('needs_human', `PR ${d.pr} passed every gate and waits for a person's merge decision.`)
    if (!d.ok || round >= RUN.limits.delivery_rounds || !['ci_failed', 'conflict', 'review_stale'].includes(d.state))
      return end('blocked', d.error || `Delivery stopped in state ${d.state}.`)
    patch = d.diff_patch || patch // the patch from the reviewed commit to the head
    if (d.state !== 'review_stale') {
      const r = await gated('implementer', `repair-${round}`, sid, `${where} Delivery failed (${d.state}): ${d.detail} Repair it.`,
        'green', { phase: 'Deliver', method: 'fix-findings', closing: `${gate('green')} --since ${safe(d.reviewed_sha)}` })
      if (r.status !== 'done') return end('blocked', r.summary)
      head = r.head; patch = pick(r, 'green').diff_patch || patch // with --since the gate writes the patch from that commit
    }
    const v = await review(round + 1, d.reviewed_sha, 2, d.changed_files, 0) // only the commits added since the converged review
    if (v.outcome !== 'converged') return end('blocked', `Review of the repair did not converge (${v.reason}).`)
  }
}
return end('done', `Story ${sid} is merged at ${out.merged_sha || u.head}.`)

} catch (e) {
  return end('failed', `The script stopped on an error: ${String(e).slice(0, 300)}`, true)
}
```

A pass with both test findings and code findings runs the test-writer first and the implementer
second, as the `fix` function shows. The implementer also runs when the pass had test findings only
and a re-frozen test fails on the current code.
