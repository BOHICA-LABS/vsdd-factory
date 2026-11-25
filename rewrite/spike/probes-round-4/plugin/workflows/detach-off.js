export const meta = {
  name: 'detach-off',
  description: 'Round-4 probe Q8 with no sandbox: detached worker vs the timeout',
  phases: [{ title: 'P' }],
}

const PROJ = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project'
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

phase('P')
const r = await agent(
  `Run this Bash command exactly once, setting the Bash tool's timeout parameter to 15000 milliseconds: python3 ${PROJ}/spawncall.py ${PROJ}/detach-off.out 30 40  The call is expected to be killed by its timeout; that is intended. Then return output = exactly what it printed before the timeout, or the exact error text.`,
  { agentType: 'fs4:clerk', schema: TEXT, label: 'Q8OFF', phase: 'P' },
)
return [{ label: 'Q8OFF', result: r }]
