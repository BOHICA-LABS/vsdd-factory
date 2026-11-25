export const meta = {
  name: 'sandbox',
  description: 'Round-4 probes Q6 and Q8 with the Bash sandbox on',
  phases: [{ title: 'P' }],
}

const PROJ = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/pr4/project'
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
  safe('Q6', () =>
    agent(
      `Run this Bash command exactly once: sh ${PROJ}/sbx.sh  Then return output = exactly what it printed (every line), or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`,
      { agentType: 'fs4:clerk', schema: TEXT, label: 'Q6', phase: 'P' },
    ),
  ),
  safe('Q8ON', () =>
    agent(
      `Run this Bash command exactly once, setting the Bash tool's timeout parameter to 15000 milliseconds: python3 ${PROJ}/spawn.py ${PROJ}/detach-on.out 30; sleep 40; echo CALL-END  The call is expected to be killed by its timeout; that is intended. Then return output = exactly what it printed before the timeout, or the exact error text.`,
      { agentType: 'fs4:clerk', schema: TEXT, label: 'Q8ON', phase: 'P' },
    ),
  ),
])

return results
