export const meta = {
  name: 'main',
  description: 'Round-4 probes Q1 Q2 Q3 Q5, default permission mode',
  phases: [{ title: 'P' }],
}

const PROJ = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project'
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }
const FIND = {
  type: 'object',
  properties: {
    label: { type: 'string' },
    findings: { type: 'array', items: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] } },
  },
  required: ['label', 'findings'],
}
const Q5S = {
  type: 'object',
  properties: { label: { type: 'string' }, attempts: { type: 'integer' }, output: { type: 'string' } },
  required: ['label', 'attempts', 'output'],
}

const run = (cmd) =>
  `Run this Bash command exactly once: ${cmd}  Then return output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`

phase('P')
const results = await parallel([
  safe('Q1', () =>
    agent(`${run('echo Q1MARKER-77')}`, { agentType: 'fs4:clerk', schema: TEXT, label: 'Q1', phase: 'P' }),
  ),
  safe('Q2A', () =>
    agent(`Return label 'Q2A' and findings = [{text: 'Q2ARESULT'}]. Nothing else.`, {
      agentType: 'fs4:clerk',
      schema: FIND,
      label: 'Q2A',
      phase: 'P',
    }),
  ),
  safe('Q2B', () =>
    agent(`Return label 'Q2B' and findings = [{text: 'Q2BRESULT'}]. Nothing else.`, {
      agentType: 'fs4:clerk',
      schema: FIND,
      label: 'Q2B',
      phase: 'P',
    }),
  ),
  safe('Q3A', () => agent(`${run(`sh ${PROJ}/probe.sh RUNME3A`)}`, { agentType: 'fs4:clerk', schema: TEXT, label: 'Q3A', phase: 'P' })),
  safe('Q3ACTRL', () =>
    agent(`${run(`sh ${PROJ}/probe.sh RUNME3ACTRL`)}`, { agentType: 'fs4:clerk', schema: TEXT, label: 'Q3ACTRL', phase: 'P' }),
  ),
  safe('Q3B', () =>
    agent(`Return label 'Q3B' and output 'Q3BMARKER'. Nothing else.`, {
      agentType: 'fs4:clerk',
      schema: TEXT,
      label: 'Q3B',
      phase: 'P',
    }),
  ),
  safe('Q5', () =>
    agent(
      `Follow this sequence exactly. Step 1: return your result with label 'Q5', attempts = the TEXT 'three' (in quotes, not a number), output = ''. Step 2: if the system refuses your result, read the exact refusal text, do what it demands, and return your result again, correcting only what the refusal demands, with output = the exact first refusal text you were shown and attempts = the number of times you tried to return a result.`,
      { agentType: 'fs4:q5returner', schema: Q5S, label: 'Q5', phase: 'P' },
    ),
  ),
])

function safe(label, fn) {
  return async () => {
    try {
      return { label, result: await fn() }
    } catch (e) {
      return { label, error: String(e).slice(0, 400) }
    }
  }
}

return results
