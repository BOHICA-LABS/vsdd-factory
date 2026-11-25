export const meta = {
  name: 'q4',
  description: 'Round-4 probe Q4: updatedInput with no permissionDecision',
  phases: [{ title: 'P' }],
}

const PROJ = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project'
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

phase('P')
const r = await agent(`Run this Bash command exactly once: sh ${PROJ}/probe.sh RUNME4  Then return output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`, {
  agentType: 'fs4:clerk',
  schema: TEXT,
  label: 'Q4',
  phase: 'P',
})
return [{ label: 'Q4', result: r }]
