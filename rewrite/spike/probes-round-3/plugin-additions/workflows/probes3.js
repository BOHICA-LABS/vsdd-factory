export const meta = {
  name: 'probes3',
  description: 'Platform probes round 3: rewrite, session ids, ancestry, concurrency, encoding',
  phases: [{ title: 'Probes' }],
}

const PROJECT = '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/probe3/project'
const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

const safe = (label, fn) => async () => {
  try {
    const r = await fn()
    return { label, result: r }
  } catch (e) {
    return { label, error: String(e).slice(0, 300) }
  }
}

phase('Probes')

const results = await parallel([
  // 27: hook rewrites a Bash call inside a workflow step (updatedInput); probe.sh prints the env.
  safe('P27-step-rewrite', () =>
    agent(`Run this Bash command exactly once: bash ${PROJECT}/probe.sh PROBE_REWRITE_MARKER  Return output = exactly what it printed.`, {
      agentType: 'fspike:clerk',
      schema: TEXT,
      label: 'P27',
      phase: 'Probes',
    }),
  ),
  // 27 tail: hook rewrites one field of a StructuredOutput step result.
  safe('P27-so-rewrite', () =>
    agent(`Return label "P27SO" and output exactly ORIGVAL. If the system changes your result, keep the changed value.`, {
      schema: TEXT,
      label: 'P27SO',
      phase: 'Probes',
    }),
  ),
  // 29: process-tree ancestry as seen from inside a workflow step.
  safe('P29-ancestry', () =>
    agent(`Run this Bash command exactly once: python3 ${PROJECT}/ancestry.py  Return output = exactly what it printed.`, {
      agentType: 'fspike:clerk',
      schema: TEXT,
      label: 'P29',
      phase: 'Probes',
    }),
  ),
  // 6: a long call with an explicit ten-minute timeout parameter.
  safe('P6-long-call', () =>
    agent(
      `Run this Bash command exactly once, setting the Bash tool timeout parameter to 600000 milliseconds: sleep 240 && echo LONGDONE  Return output = exactly what it printed, or the exact error or timeout text.`,
      { agentType: 'fspike:clerk', schema: TEXT, label: 'P6', phase: 'Probes' },
    ),
  ),
  // 35: two concurrent steps of different agent types, for SubagentStart pairing.
  safe('P35-plain', () => agent(`Reply with exactly PLAIN-RUNNER-OK`, { agentType: 'fspike:plain', label: 'P35a', phase: 'Probes' })),
  safe('P35-clerk', () =>
    agent(`Run this Bash command exactly once: echo PAIR-CHECK-done  Return output = exactly what it printed.`, {
      agentType: 'fspike:clerk',
      schema: TEXT,
      label: 'P35b',
      phase: 'Probes',
    }),
  ),
])

return { enc: (() => { try { return encodeURIComponent('a b&c/d') } catch (e) { return 'ERR:' + String(e) } })(), results }
