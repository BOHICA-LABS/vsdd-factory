export const meta = {
  name: 'spike-child',
  description: 'Spike test: what a workflow that is already a child can do to run a spike',
  phases: [{ title: 'Child spike' }],
}

phase('Child spike')
const question = args && args.question ? args.question : 'no question'

// A child cannot call a third workflow, so it runs the spike as a step of its own.
let inline = null
try {
  inline = await agent(
    `Research spike. Question: ${question} Experiment: run the Bash command echo SPIKE-EXPERIMENT-OK once and ` +
      `reply with one line: CHILD-INLINE-SPIKE followed by what it printed.`,
    { agentType: 'fspike:spike-runner', label: 'child-inline-spike', phase: 'Child spike' },
  )
} catch (e) {
  inline = `error: ${String(e).slice(0, 300)}`
}

let third = null
try {
  third = await workflow('fspike:story', { id: 'third-level' })
} catch (e) {
  third = `error: ${String(e).slice(0, 300)}`
}

return { inline, third }
