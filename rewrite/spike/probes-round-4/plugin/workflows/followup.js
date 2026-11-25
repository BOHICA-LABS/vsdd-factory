export const meta = {
  name: 'followup',
  description: 'Round-4 follow-up probes: Q3 permission control leg, Q5 refusal and ordering',
  phases: [{ title: 'P' }],
}

const PROJ = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project'
const Q5S = {
  type: 'object',
  properties: { label: { type: 'string' }, note: { type: 'string' }, output: { type: 'string' } },
  required: ['label', 'note', 'output'],
}
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

const safe = (label, fn) => async () => {
  try {
    return { label, result: await fn() }
  } catch (e) {
    return { label, error: String(e).slice(0, 400) }
  }
}

phase('P')
const results = await parallel([
  safe('Q3ACTRL2', () =>
    agent(
      `Run this Bash command exactly once: sh ${PROJ}/probe.sh RUNME3ACTRL2  Then return label 'Q3ACTRL2' and output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`,
      { agentType: 'fs4:clerk', schema: TEXT, label: 'Q3ACTRL2', phase: 'P' },
    ),
  ),
  safe('Q5B', () =>
    agent(
      `Follow this sequence exactly. Step 1: return your result with label 'Q5B', note = '' (the empty string), output = 'FIRSTTRY'. Step 2: if the system refuses your result, read the exact refusal text, do what it demands, and return again with note = the first 40 characters of the exact refusal text you were shown, output = 'SECONDTRY'.`,
      { agentType: 'fs4:q5returner', schema: Q5S, label: 'Q5B', phase: 'P' },
    ),
  ),
  safe('Q5C', () =>
    agent(
      `Follow this sequence exactly. Step 1: call your result tool with label 'Q5C', output = 'NOCANDO', omitting the note field entirely from the input. Step 2: if the system refuses your result, read the exact refusal text and return again WITH note = the first 40 characters of the exact refusal text, output = 'FIXED'.`,
      { agentType: 'fs4:q5returner', schema: Q5S, label: 'Q5C', phase: 'P' },
    ),
  ),
])

return results
