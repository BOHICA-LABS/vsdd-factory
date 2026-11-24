export const meta = {
  name: 'spike-request',
  description: 'Spike test: an agent asks for a spike through its result and the workflow runs it in an isolated worktree',
  phases: [{ title: 'Request' }, { title: 'Spike' }, { title: 'Resume' }],
}

const NEED = {
  type: 'object',
  properties: {
    label: { type: 'string' },
    needs_spike: { type: 'boolean' },
    question: { type: 'string' },
    answer: { type: 'string' },
  },
  required: ['label', 'needs_spike', 'question', 'answer'],
}
const FIND = {
  type: 'object',
  properties: {
    question: { type: 'string' },
    verdict: { type: 'string' },
    result: { type: 'string' },
    cwd: { type: 'string' },
    branch: { type: 'string' },
    wrote_file: { type: 'boolean' },
  },
  required: ['question', 'verdict', 'result', 'cwd', 'branch', 'wrote_file'],
}

const task = `You are a builder. Your task: report the exact text stored in the file spike-token.txt in the project root. `
const step = { agentType: 'fspike:builder', model: 'sonnet', effort: 'low', schema: NEED }

phase('Request')
const first = await agent(
  task +
    `In this step you cannot inspect the project: do not use Read, Bash, or any other tool except returning your ` +
    `result, and do not guess. If you cannot complete the task without an experiment that someone else runs for ` +
    `you, return needs_spike = true, put the question to investigate in "question", and "" in "answer". ` +
    `Label "T4-request".`,
  { ...step, phase: 'Request', label: 'T4-request' },
)

let finding = null
let second = null
if (first && first.needs_spike) {
  phase('Spike')
  finding = await agent(
    `Research spike. Question: ${first.question} Experiment, once each: (1) run pwd; (2) run ` +
      `git branch --show-current; (3) use the Read tool on spike-token.txt in your current working directory; ` +
      `(4) use the Write tool to create spike-scratch.txt in your current working directory containing the token. ` +
      `Return the finding: result = the token text, cwd and branch from steps 1 and 2, wrote_file = whether ` +
      `step 4 succeeded.`,
    { agentType: 'fspike:spike-runner', isolation: 'worktree', schema: FIND, phase: 'Spike', label: 'T4-spike' },
  )
  phase('Resume')
  second = await agent(
    task +
      `A research spike was run for you and found: ${JSON.stringify(finding)}. Answer from that finding and set ` +
      `needs_spike = false. Label "T4-after-spike".`,
    { ...step, phase: 'Resume', label: 'T4-after' },
  )
}

return { first, finding, second }
