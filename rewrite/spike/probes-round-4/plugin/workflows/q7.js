export const meta = {
  name: 'q7',
  description: 'Round-4 probe Q7: ten-minute foreground call with explicit 600000 ms timeout',
  phases: [{ title: 'P' }],
}

const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

phase('P')
const r = await agent(
  `Run this Bash command exactly once, setting the Bash tool's timeout parameter to 600000 milliseconds. The command takes about 10 minutes; set the timeout parameter and let it run: python3 /private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project/wait590.py  Then return output = exactly what it printed, or the exact error text if the call was refused, failed or timed out. Do not retry and do not run anything else.`,
  { agentType: 'fs4:clerk', schema: TEXT, label: 'Q7', phase: 'P' },
)
return [{ label: 'Q7', result: r }]
