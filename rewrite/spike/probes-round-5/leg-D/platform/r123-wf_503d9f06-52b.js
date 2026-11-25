export const meta = { name: 'r123', description: 'Check probes Q1 Q2 Q3 Q5 in the default mode', phases: [{ title: 'P' }] }
const PROJ = '/Users/jmagady/.vsdd-probes/round-5.QkGwwK/projd'
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }
const item = (extra) => ({ type: 'object', properties: { text: { type: 'string' }, ...extra.props }, required: ['text'], ...extra.more })
const FIND = (it) => ({ type: 'object', properties: { label: { type: 'string' }, findings: { type: 'array', items: it } }, required: ['label', 'findings'] })
const OPEN = FIND(item({ props: {}, more: {} }))
const DECL = FIND(item({ props: { grounded: { type: 'boolean' } }, more: {} }))
const CLOSED = FIND(item({ props: {}, more: { additionalProperties: false } }))
const R5S = { type: 'object', properties: { label: { type: 'string' }, note: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'note', 'output'] }
const run = (cmd) => `Run this Bash command exactly once: ${cmd}  Then return output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`
const safe = (label, fn) => async () => { try { return { label, result: await fn() } } catch (e) { return { label, error: String(e).slice(0, 600) } } }
const ret = (label) => `Return label '${label}' and findings = [{text: '${label}TEXT'}]. Nothing else.`
phase('P')
const results = await parallel([
  safe('R1', () => agent(run('echo R1MARK-55'), { agentType: 'chk5:clerk', schema: TEXT, label: 'R1', phase: 'P' })),
  safe('R2A', () => agent(ret('R2A'), { agentType: 'chk5:clerk', schema: OPEN, label: 'R2A', phase: 'P' })),
  safe('R2B', () => agent(ret('R2B'), { agentType: 'chk5:clerk', schema: DECL, label: 'R2B', phase: 'P' })),
  safe('R2C', () => agent(ret('R2C'), { agentType: 'chk5:clerk', schema: CLOSED, label: 'R2C', phase: 'P' })),
  safe('R3A', () => agent(run(`sh ${PROJ}/probe.sh RUN3A`), { agentType: 'chk5:clerk', schema: TEXT, label: 'R3A', phase: 'P' })),
  safe('R3CTRL', () => agent(run(`sh ${PROJ}/probe.sh RUN3CTRL`), { agentType: 'chk5:clerk', schema: TEXT, label: 'R3CTRL', phase: 'P' })),
  safe('R3B', () => agent("Return label 'R3B' and output 'R3BMARK'. Nothing else.", { agentType: 'chk5:clerk', schema: TEXT, label: 'R3B', phase: 'P' })),
  safe('R5B', () => agent("Follow this sequence exactly. Step 1: return your result with label 'R5B', note = '' (the empty string), output = 'FIRSTTRY'. Step 2: if the system refuses your result, do exactly what the refusal text demands, and return again with note = the full refusal text you were shown, verbatim.", { agentType: 'chk5:returner', schema: R5S, label: 'R5B', phase: 'P' })),
])
return results
