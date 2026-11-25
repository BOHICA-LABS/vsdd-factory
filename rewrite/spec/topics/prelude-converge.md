---
kind: topic
id: prelude-converge
package: B4a
sources: [pipeline.md#2.11.1, pipeline.md#2.7, BLUEPRINT.md#4.14]
---

# Prelude judge() and converge()

`judge()` and `converge()`: the single source of the convergence rule as code. The rule in words is
[[topic:convergence]]; the tool's Python copy is held to this code by conformance fixtures, and the
tool's result is the one guards read.

```js
// The convergence rule.
const HARD = ['critical', 'high']
const REACH = {
  spec: ['unit', 'integration', 'system', 'spec'], plan: ['unit', 'integration', 'system', 'spec'],
  story: ['unit'], wave: ['unit', 'integration'], release: ['unit', 'integration', 'system'],
}
function judge(findings, perimeter, waived) {
  const v = { blocking: [], advisory: [], deferred: [], spec_gaps: [] }
  for (const f of findings) {
    // A holdout finding is built from scenario scores and has no file to point at, so it needs no location.
    const placed = f.category === 'holdout' || Boolean(f.location && f.location.path)
    // grounded is false only on a finding whose citation failed the grounding check twice ([[topic:verdicts]]).
    // A holdout finding is exempt from the grounding condition too: it cites its scenario, not a code path.
    const hard = HARD.includes(f.severity) && (f.category === 'holdout' || Boolean(f.spec_ref || f.repro)) && placed && f.grounded !== false
    if (waived.includes(f.key)) v.advisory.push(f)
    else if (REACH[perimeter].includes(f.scope)) (hard ? v.blocking : v.advisory).push(f)
    else if (f.scope === 'spec') (hard ? v.spec_gaps : v.advisory).push(f)
    else v.deferred.push(f)
  }
  return v
}
// c.review(pass, claims, gaps) -> { ok, head, findings, claim_checks, coverage_missing },
//                                 or { ok: false, blocked: true } when a reviewer returned blocked
// c.fix(open, advisory, pass)  -> { ok, resolutions: [{ key, resolution: 'fixed' | 'disputed', commit, note }] }
// c.arbitrate(open, pass, notes) -> { rulings: [{ key, ruling: 'defect' | 'not_defect' | 'spec_gap', reason }] } (optional)
//                 notes: Map key -> the disputing builder's note, for the arbitration prompt only (P3SEAM-14)
// c.used                       -> passes or rounds this unit already used in earlier runs (from the survey)
async function converge(c) {
  const raised = new Map()
  const waived = (c.waived || []).slice()
  const dismissed = []
  const disputed = new Set()
  const notes = new Map() // the fixer's dispute notes, by key: the arbiter sees them, no one else does
  const used = c.used || 0
  let claims = [], open = [], advisory = [], deferred = [], gaps = [], head = '', pass = 0, extra = 0
  const verdict = (outcome, reason) => ({
    perimeter: c.perimeter, subject: c.subject, head, passes: pass, rounds_used: used + pass, outcome, reason,
    blocking: open, advisory, deferred, dismissed, coverage_missing: gaps,
  })
  // An item that no reviewer marked checked is itself a finding, so the fixer is called and the loop can end.
  const uncovered = (item) => ({
    key: `coverage:${c.subject}:${item}`, title: `Not accounted for: ${item}`, severity: 'high', category: 'coverage',
    scope: REACH[c.perimeter][0], owner: 'test-writer', location: { path: '' }, spec_ref: item, repro: '',
    observed: 'No reviewer marked this item checked.', expected: 'A test or another evidence route covers it.',
  })
  while (used + pass < c.cap + extra) { // the cap counts across runs
    pass++
    const r = await c.review(pass, claims, gaps)
    if (r && r.blocked) return verdict('blocked', 'reviewer_blocked') // a decision opens at once; not a failed run
    if (!r || !r.ok) return verdict('failed', 'no_result')
    head = r.head
    const j = judge(r.findings, c.perimeter, waived)
    j.blocking.forEach((f) => raised.set(f.key, f))
    const checked = new Map((r.claim_checks || []).map((k) => [k.key, k.outcome]))
    for (const k of claims) if (checked.get(k.key) === 'withdrawn') dismissed.push({ key: k.key, by: 'reviewer', reason: 'Withdrawn on re-review.' })
    const still = claims.filter((k) => !['resolved', 'withdrawn'].includes(checked.get(k.key))).map((k) => raised.get(k.key))
    gaps = (r.coverage_missing || []).filter((item) => !waived.includes(`coverage:${c.subject}:${item}`))
    const cov = gaps.map(uncovered)
    cov.forEach((f) => raised.set(f.key, f))
    open = j.blocking.concat(still.filter((f) => f && f.category !== 'coverage' && !j.blocking.some((b) => b.key === f.key)), cov)
    advisory = j.advisory
    deferred = deferred.concat(j.deferred)
    if (j.spec_gaps.length) { open = j.spec_gaps; return verdict('blocked', 'spec_gap') }
    if (!open.length) return verdict('converged', 'clean')
    if (used + pass >= c.cap + extra) {
      if (extra || !c.arbitrate) break
      const a = await c.arbitrate(open, pass, notes)
      const ruled = new Map(((a && a.rulings) || []).map((x) => [x.key, x]))
      const upheld = []
      for (const f of open) {
        const x = ruled.get(f.key)
        if (x && x.ruling === 'spec_gap') { open = [f]; return verdict('blocked', 'spec_gap') }
        // Only a finding the builder disputed can be dismissed, and never a critical or security finding.
        if (x && x.ruling === 'not_defect' && disputed.has(f.key) && f.severity !== 'critical' && f.category !== 'security') {
          dismissed.push({ key: f.key, by: 'arbiter', reason: x.reason })
          waived.push(f.key)
        } else upheld.push(f)
      }
      open = upheld
      if (!open.length) return verdict('converged', 'clean')
      extra = 1
    }
    const fx = await c.fix(open, advisory, pass)
    if (!fx || !fx.ok) return verdict('failed', 'no_result')
    const how = new Map((fx.resolutions || []).map((x) => [x.key, x]))
    // A claim is structured data: no title, no quote, none of the fixer's text. A holdout claim is the key alone.
    claims = open.map((f) => {
      const h = how.get(f.key) || {}
      if (h.resolution === 'disputed') { disputed.add(f.key); notes.set(f.key, h.note || '') }
      const where = f.category === 'holdout' ? {} : { path: f.location.path, line_start: f.location.line_start, line_end: f.location.line_end }
      return { key: f.key, ...where, resolution: h.resolution || 'fixed', commit: h.commit || '' }
    })
  }
  return verdict('blocked', 'cap')
}
```

The verdict is not a command and is not stored as a record of its own: after the checkpoint the verdict
is in the unit's closure record ([[record:closure]]). The tool computes the same object from the stored
passes, as a function that `vsdd gate demo`, `vsdd story deliver`, the survey and the checkpoint call,
from the records and by the rules of [[topic:verdicts]].
