---
kind: topic
id: prelude-run-stories
package: B4a
sources: [pipeline.md#2.11.1, pipeline.md#2.5, BLUEPRINT.md#4.12]
---

# Prelude runStories()

`runStories()`: how child stories are started and how budgets are shared between a parent and its
children. The workflows that call it: [[workflow:wave]], [[workflow:verify]], [[workflow:feature]],
[[workflow:maintain]] and [[workflow:multi]], which call `story` ([[workflow:story]]) as a child.

Before it starts a batch it checks
`RUN.agents + units.length * story_agents + CLOSE_RESERVE <= limits.agents` and, if the batch does not
fit, starts the units that do and reports the rest, so the caller returns `partial`. Each child
receives `limits.agents = story_agents` and its share of the run's remaining spikes (at least one)
with the count already used. A child that throws or returns nothing is charged its full
`story_agents`, because it may have spent them. The platform's 1,000-agent counter is shared and hard,
so this check is what keeps the reserve for the checkpoint, which releases the lock and pushes state.
The survey returns limits under the prelude's names ([[table:limits]]); a test fails on any unknown
limit key.

```js
// Start child stories. Used by wave, verify, feature, maintain and multi. A batch starts only if it fits
// beside the reserve for the close. Each child gets its own agent budget and its share of the run's spikes.
// A child that throws or returns nothing is charged its full budget, because it may have spent it.
async function runStories(units, extra = {}) {
  const per = RUN.limits.story_agents
  const fit = Math.max(0, Math.floor((RUN.limits.agents - RUN.agents - CLOSE_RESERVE) / per))
  const batch = units.slice(0, Math.min(fit, RUN.limits.stories_per_run))
  const share = Math.max(1, Math.floor((RUN.limits.spikes_per_run - RUN.spikes) / Math.max(1, batch.length)))
  const lost = (u, why) => ({ subject: u.id, status: 'failed', summary: why, deferred: [], agents: per, spikes: 0 })
  const rs = await parallel(batch.map((u) => async () => {
    try {
      return await workflow(`${P}story`, {
        ...extra, story: u.id, position: u, run_id: RUN.id, project_dir: RUN.dir, plugin_root: RUN.root, tier: RUN.tier,
        limits: { ...RUN.limits, agents: per, spikes_per_run: share },
      })
    } catch (e) {
      return lost(u, String(e).slice(0, 300))
    }
  }))
  const results = rs.map((r, i) => r || lost(batch[i], 'The story run returned nothing.'))
  results.forEach((r) => { RUN.agents += r.agents || per; RUN.spikes += r.spikes || 0 })
  return { results, skipped: units.slice(batch.length) }
}
```

`verify`, `feature`, `maintain` and `multi` treat the `skipped` list the same way `wave` does, which
is how the reserve check reaches every workflow that runs child stories.
