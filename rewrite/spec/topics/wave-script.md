---
kind: topic
id: wave-script
package: W3
sources: [pipeline.md#2.11.3, pipeline.md#2.11]
---

# wave.js skeleton

The wave.js skeleton. The playbook it implements is [[workflow:wave]]; the prelude stamped at its
head is [[topic:prelude]].

```js
export const meta = {
  name: 'wave',
  description: 'Build one wave: deliver its stories in parallel, then run the wave gate to convergence',
  whenToUse: 'Run after the plan is ready. With no argument it takes the next open wave.',
  phases: [{ title: 'Survey' }, { title: 'Stories' }, { title: 'Gate' }, { title: 'Fix' }, { title: 'Close' }],
}

// >>> prelude … <<< prelude   (the stamped prelude, [[topic:prelude]], stamped here)

RUN.wf = 'wave'
const a = parseArgs(args, 'wave', ['repo'])

phase('Survey')
const s = await relay('survey', 'wave', `vsdd survey wave${a.wave ? ` --subject ${safe(a.wave)}` : ''}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --json`, S_SURVEY, 'Survey')
const unmet = s.ok ? s.preconditions.filter((p) => !p.met) : []
if (!s.ok || unmet.length)
  return { workflow: 'wave', subject: a.wave || '', status: 'blocked', summary: s.error || unmet.map((p) => `${p.name}: ${p.detail}`).join('; '), next: s.next }
RUN.id = s.run_id; RUN.dir = s.project_dir; RUN.root = s.plugin_root; RUN.tier = s.config.tier_override || null
RUN.limits = { ...RUN.limits, ...s.limits }
const W = safe(s.subject)

const close = async (status, summary, failed) => {
  phase('Close')
  const c = await relay('close', W, `vsdd checkpoint wave --run ${RUN.id}${failed ? ' --outcome failed' : ''} --json`, S_CLOSE, 'Close')
  return {
    workflow: 'wave', subject: W, status: c.ok ? c.status || status : 'failed', summary, progress: Boolean(c.progress),
    decisions: c.decisions || [], next: c.next, counts: { agents: RUN.agents, spikes: RUN.spikes },
  }
}
let short = false // set when a fix round did not fit beside the reserve for the close

try { // everything after a successful survey is inside try, so the checkpoint always runs (the body is not re-indented here)

phase('Stories')
const todo = s.units.filter((u) => u.position !== 'delivered' && !u.deferred && !(u.blocked && u.blocked.open))
// runStories() is in the prelude: each child gets story_agents, and a batch starts only if it fits beside the reserve.
const first = todo.length ? await runStories(todo, { repo: a.repo }) : { results: [], skipped: [] }
const built = first.results
if (first.skipped.length) {
  log(`${first.skipped.length} stories of ${W} are left for the next run of this command.`)
  return close('partial', `${built.filter((r) => r.status === 'done').length} of ${todo.length} open stories delivered in this run.`)
}

phase('Gate')
// --settle moves stories that wait on a person to a later wave when the plan allows it, and reports what still blocks the gate.
// For a UI wave the same job runs the UI check.
const gateCmd = (settle) => `vsdd gate wave --wave ${W} --run ${RUN.id}${settle ? ' --settle' : ''}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --wait 540 --json`
let g = await relay('integration-r1', W, gateCmd(true), S_WAVE_GATE, 'Gate')
if (!g.ok || g.pending.length)
  return close(g.ok ? 'partial' : 'failed', g.ok ? `Stories not yet delivered: ${g.pending.join(', ')}.` : g.error)

const carried = built.flatMap((r) => r.deferred || [])
const v = await converge({
  perimeter: 'wave', subject: W, cap: RUN.limits.rounds, used: s.rounds_used || 0, waived: s.waived || [],
  review: async (round, claims) => {
    if (round > 1) g = await relay(`integration-r${round}`, W, gateCmd(false), S_WAVE_GATE, 'Gate')
    if (!g.ok) return { ok: false }
    if (!g.passed) return { ok: true, head: g.sha, findings: g.findings, claim_checks: [], coverage_missing: [] }
    const ctx = `Wave ${W}. Integration worktree: ${g.worktree}. Diff patch: ${g.diff_patch} (${g.base}...${g.sha}). Stories: ${g.stories.map((u) => u.id).join(', ')}. Claims to verify: ${JSON.stringify(claims)}.`
    const checks = [
      () => step('adversary', `integration-review-r${round}`, W,
        `${ctx} Review how these stories work together. Findings the story reviews passed up, as scope to check: ${JSON.stringify(carried)}`,
        { phase: 'Gate', method: 'review-wave', data: S_REVIEW_DATA, pass: round }),
      ...(g.needs.security ? [() => step('security-reviewer', `security-r${round}`, W, `${ctx} Review the security of the combined change.`,
        { phase: 'Gate', method: 'security-review', data: S_REVIEW_DATA, pass: round })] : []),
      ...(g.needs.ui ? [() => step('ui-reviewer', `ux-review-r${round}`, W, `${ctx} Review the interface. Start the product with: vsdd -C ${RUN.dir} env start --subject ${W}`,
        { phase: 'Gate', method: 'ui-review', data: S_REVIEW_DATA, pass: round })] : []),
      // The evaluator is given scenario ids and nothing about the code. It returns scores and symptoms, never findings.
      ...g.holdout_batches.map((ids, i) => () => step('holdout-evaluator', `holdout-r${round}-b${i + 1}`, W,
        `Evaluate the hidden scenarios ${ids.join(', ')} against the running product. Start it with: vsdd -C ${RUN.dir} env start --subject ${W}`,
        { phase: 'Gate', method: 'holdout-eval', data: S_HOLDOUT_DATA, pass: round })),
    ]
    const rs = await parallel(checks)
    if (rs.some((r) => r && r.status === 'blocked')) return { ok: false, blocked: true }
    if (rs.some((r) => !r || r.status !== 'done')) return { ok: false }
    return {
      ok: true, head: g.sha, coverage_missing: [],
      // holdoutFindings() builds findings of category holdout from the scores: no location, key holdout:<HS id>.
      findings: rs.flatMap((r) => r.data.findings || []).concat(holdoutFindings(W, rs.flatMap((r) => r.data.scores || []),
        { ...s.config.holdout, ...g.holdout, scope: 'integration', symptoms: rs.flatMap((r) => r.data.symptoms || []) })),
      claim_checks: rs.flatMap((r) => r.data.claim_checks || []),
    }
  },
  fix: async (open, advisory, round) => {
    phase('Fix')
    // When the measured gate failed, its output already holds the admitted fix units. Otherwise the tool recomputes the
    // verdict from the stored pass records, writes one fix story per group of blocking findings and admits each.
    let units = g.fix_units || []
    if (!units.length) {
      const nf = await relay(`new-fix-r${round}`, W, `vsdd story new-fix --wave ${W} --round ${round} --run ${RUN.id} --json`, S_SURVEY, 'Fix')
      if (!nf.ok || !nf.units.length) return { ok: false }
      units = nf.units
    }
    const { results: rs, skipped } = await runStories(units, { repo: a.repo })
    if (skipped.length) { short = true; return { ok: false } } // the round did not fit: return partial, keep the reserve
    phase('Gate')
    const merged = new Map(rs.filter((r) => r.status === 'done').flatMap((r) => ((units.find((u) => u.id === r.subject) || {}).fixes || []).map((k) => [k, r.merged_sha || ''])))
    // Every open finding gets a claim, so none is lost. A fix story that was not delivered is never recorded as a
    // dispute: its claim is checked by the next round and comes back unresolved.
    return { ok: true, resolutions: open.map((f) => ({ key: f.key, resolution: 'fixed', commit: merged.get(f.key) || '', note: '' })) }
  },
  arbitrate: async (open, round, notes) => {
    const r = await step('architect', 'arbitrate', W, `Wave ${W}. Rule on each finding that is still open after ${RUN.limits.rounds} rounds: ${JSON.stringify(open)}`,
      { phase: 'Gate', method: 'arbitrate', data: S_RULING_DATA, pass: round })
    return r.status === 'done' ? r.data : null
  },
})
if (short) return close('partial', `A fix round for ${W} did not fit in this run. Run the same command again.`)
return close(v.outcome === 'converged' ? 'done' : v.outcome === 'failed' ? 'failed' : 'needs_human',
  v.outcome === 'converged' ? `Wave ${W} passed its gate after ${v.passes} round(s).` : `Wave ${W} gate did not converge (${v.reason}); ${v.blocking.length} finding(s) open.`)

} catch (e) {
  return close('failed', `The script stopped on an error: ${String(e).slice(0, 300)}`, true)
}
```

Agent counts from the simulation, for scale: a story with a clean first review used 4 agents as a
child (tests, implement, review, deliver) and 6 standalone; with one finding fixed, 6 and 8. A
two-story wave with a clean gate used 13 agents, and 21 with one integration finding fixed through a
fix story. Those are the floor, with every step succeeding first time. The planning figure for a
realistic story, with a second lens, a fix round and an evidence step, is 8 to 12 agent starts, and 70
to 90 for a six-story wave; the end-to-end tests budget 12 per story until they have measured it.
