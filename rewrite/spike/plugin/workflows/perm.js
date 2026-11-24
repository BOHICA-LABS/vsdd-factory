export const meta = {
  name: 'perm',
  description: 'Probe: is a hook approval honoured inside a workflow step in the default permission mode',
  phases: [{ title: 'Perm' }],
}
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }
const run = (cmd) =>
  `Run this Bash command exactly once: ${cmd}  Then return output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`
phase('Perm')
const out = await parallel([
  () => agent(`Label "allow". ${run('fspike-tool allowme')}`, { agentType: 'fspike:clerk', schema: TEXT, label: 'allow', phase: 'Perm' }),
  () => agent(`Label "control". ${run('fspike-tool denyme')}`, { agentType: 'fspike:clerk', schema: TEXT, label: 'control', phase: 'Perm' }),
])
return out
