export const meta = {
  name: 'probes',
  description: 'Platform probes for the vsdd-factory 1.0 blueprint, section 10',
  phases: [{ title: 'Probes' }],
}

const DETACH_FILE =
  '/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/spike/out/detached.txt'

const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }
const QUOTE = {
  type: 'object',
  properties: { quote: { type: 'string' }, attempts: { type: 'number' }, refusal_text: { type: 'string' } },
  required: ['quote', 'attempts', 'refusal_text'],
}

const safe = (label, fn) => async () => {
  try {
    const r = await fn()
    return { label, result: r }
  } catch (e) {
    return { label, error: String(e).slice(0, 400) }
  }
}
const run = (cmd) =>
  `Run this Bash command exactly once: ${cmd}  Then return output = exactly what it printed, or the exact error text if the call was refused or failed. Do not retry and do not run anything else.`

phase('Probes')
const results = await parallel([
  // 1. What the hook sees of a step result, and whether a block makes the agent return again.
  safe('P1-structured-output-block', () =>
    agent(
      `Return quote = the exact text BADQUOTE. If the system refuses your result, read the reason it gives, follow it, and return again. Set attempts to the number of times you tried to return a result, and refusal_text to the exact refusal you were shown, or "" if there was none.`,
      { schema: QUOTE, model: 'sonnet', effort: 'low', label: 'P1', phase: 'Probes' },
    ),
  ),
  // 3. A refused stop keeps a step's agent running.
  safe('P3a-refused-stop-text', () =>
    agent(`Reply with the single word DONE. Do nothing else unless the system tells you to.`, {
      agentType: 'fspike:closer',
      label: 'P3a',
      phase: 'Probes',
    }),
  ),
  safe('P3b-refused-stop-schema', () =>
    agent(`Return label "P3b" and output "DONE". Do nothing else unless the system tells you to; if it does, add what you did to output.`, {
      agentType: 'fspike:closer',
      schema: TEXT,
      label: 'P3b',
      phase: 'Probes',
    }),
  ),
  // 5. Is <plugin>/bin on PATH inside a workflow step?
  safe('P5-bin-on-path', () =>
    agent(`Label "P5". ${run('fspike-tool path')}`, { agentType: 'fspike:plain', schema: TEXT, label: 'P5', phase: 'Probes' }),
  ),
  // 6. A long Bash call and a detached job.
  safe('P6a-long-bash', () =>
    agent(
      `Label "P6a". Run this Bash command exactly once, setting the Bash tool's timeout parameter to 200000 milliseconds: fspike-tool long 150  Then return output = exactly what it printed, or the exact error text if it was refused, failed or timed out. Do not retry.`,
      { agentType: 'fspike:plain', schema: TEXT, label: 'P6a', phase: 'Probes' },
    ),
  ),
  safe('P6b-detached-job', () =>
    agent(`Label "P6b". ${run(`fspike-tool detach ${DETACH_FILE} 25`)}`, {
      agentType: 'fspike:plain',
      schema: TEXT,
      label: 'P6b',
      phase: 'Probes',
    }),
  ),
  // 7. A hook approval is honoured for a call no permission rule allows; the control is refused.
  safe('P7a-hook-allow', () =>
    agent(`Label "P7a". ${run('fspike-tool allowme')}`, { agentType: 'fspike:plain', schema: TEXT, label: 'P7a', phase: 'Probes' }),
  ),
  safe('P7b-control-no-allow', () =>
    agent(`Label "P7b". ${run('fspike-tool denyme')}`, { agentType: 'fspike:plain', schema: TEXT, label: 'P7b', phase: 'Probes' }),
  ),
  // 8. Tool lists.
  safe('P8a-agent-tool-listed', () =>
    agent(
      `Label "P8a". List the exact names of every tool you have. Then say whether you have a tool that starts another agent (Agent or Task). If you do, use it once to run the agent type fspike:spike-runner with the prompt "Reply OK" and report what came back. Put all of this in output.`,
      { agentType: 'fspike:lister', schema: TEXT, label: 'P8a', phase: 'Probes' },
    ),
  ),
  safe('P8b-bash-only-schema', () =>
    agent(`Label "P8b". ${run('fspike-tool path')}`, { agentType: 'fspike:clerk', schema: TEXT, label: 'P8b', phase: 'Probes' }),
  ),
  safe('P8c-bash-structured-schema', () =>
    agent(`Label "P8c". ${run('fspike-tool path')}`, { agentType: 'fspike:clerk2', schema: TEXT, label: 'P8c', phase: 'Probes' }),
  ),
  // 11. Frontmatter isolation and omitClaudeMd.
  safe('P11a-frontmatter-isolation', () =>
    agent(
      `Label "P11a". Run pwd, then run git branch --show-current, each once. Return output = both outputs, each on its own line.`,
      { agentType: 'fspike:isolated', schema: TEXT, label: 'P11a', phase: 'Probes' },
    ),
  ),
  safe('P11b-omit-claude-md', () =>
    agent(
      `Label "P11b". Without using any tool: does your context contain a line with the word CLAUDEMD-MARKER-77? Return output = that line quoted exactly, or the word ABSENT.`,
      { agentType: 'fspike:nomd', schema: TEXT, label: 'P11b', phase: 'Probes' },
    ),
  ),
  safe('P11b-control-claude-md', () =>
    agent(
      `Label "P11b-control". Without using any tool: does your context contain a line with the word CLAUDEMD-MARKER-77? Return output = that line quoted exactly, or the word ABSENT.`,
      { agentType: 'fspike:plain', schema: TEXT, label: 'P11b-control', phase: 'Probes' },
    ),
  ),
  // 14. Search tools.
  safe('P14-search-tools', () =>
    agent(
      `Label "P14". First list the exact names of the tools you have for searching file contents and for finding files by name, such as Grep and Glob; say NONE if you have no such dedicated tool. Then, only if you have a dedicated content-search tool, use it once (not Bash) to search the current project for the text NEEDLE-123 and report which files matched. Put all of this in output.`,
      { agentType: 'fspike:plain', schema: TEXT, label: 'P14', phase: 'Probes' },
    ),
  ),
  // 17. Fixed cost of an agent start with and without a tool restriction.
  safe('P17-cost-unrestricted', () => agent(`Reply with the single word OK.`, { agentType: 'fspike:plain', label: 'P17-plain', phase: 'Probes' })),
  safe('P17-cost-bash-only', () => agent(`Reply with the single word OK.`, { agentType: 'fspike:clerk', label: 'P17-clerk', phase: 'Probes' })),
])

return results
