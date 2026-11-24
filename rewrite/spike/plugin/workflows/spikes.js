export const meta = {
  name: 'spikes',
  description: 'Spike test: every way an agent or a workflow can start a research spike',
  phases: [{ title: 'Agent-led' }, { title: 'Workflow-led' }, { title: 'Child' }],
}

const OUT = {
  type: 'object',
  properties: {
    label: { type: 'string' },
    worked: { type: 'boolean' },
    how: { type: 'string' },
    finding: { type: 'string' },
    error: { type: 'string' },
  },
  required: ['label', 'worked', 'how', 'finding', 'error'],
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

const safe = (fn) => async () => {
  try {
    return await fn()
  } catch (e) {
    return { error: String(e).slice(0, 400) }
  }
}

const TAIL =
  `worked = whether you started the spike and got a finding back; how = the exact tool you used; ` +
  `finding = the finding text you received; error = the exact error text, or a statement that the tool is not ` +
  `available to you, if it did not work, otherwise "". Do not run the experiment yourself and do not retry.`

phase('Agent-led')
const agentLed = await parallel([
  safe(() =>
    agent(
      `You are a builder in the middle of a task and you have hit a question that needs an experiment: ` +
        `"does echo print its argument?". Start a research spike yourself. Use the Agent tool (it may be called ` +
        `Agent or Task) to run the agent type "fspike:spike-runner", asking it to run the Bash command ` +
        `echo SPIKE-EXPERIMENT-OK and report the output. Return label "T2-agent-spawns-agent"; ` + TAIL,
      { agentType: 'fspike:builder', model: 'sonnet', effort: 'low', schema: OUT, phase: 'Agent-led', label: 'T2' },
    ),
  ),
  safe(() =>
    agent(
      `You are a builder in the middle of a task and you have hit a question that needs an experiment: ` +
        `"does echo print its argument?". Start a research spike yourself by invoking the skill "fspike:spike" ` +
        `with the Skill tool, passing the question as its argument, then follow the skill's steps. ` +
        `Return label "T3-agent-invokes-skill"; ` + TAIL,
      { agentType: 'fspike:builder', model: 'sonnet', effort: 'low', schema: OUT, phase: 'Agent-led', label: 'T3' },
    ),
  ),
])

phase('Workflow-led')
const task =
  `You are a builder. Your task: state exactly what the shell command echo SPIKE-EXPERIMENT-OK prints. `
const first = await safe(() =>
  agent(
    task +
      `You may not run any command and you may not guess. If you cannot answer with certainty without an ` +
      `experiment, return needs_spike = true, put the question to investigate in "question", and "" in "answer". ` +
      `Label "T4-request".`,
    { agentType: 'fspike:builder', model: 'sonnet', effort: 'low', schema: NEED, phase: 'Workflow-led', label: 'T4-request' },
  ),
)()
let finding = null
let second = null
if (first && first.needs_spike) {
  finding = await safe(() =>
    agent(
      `Research spike. Question: ${first.question} Experiment, once each: (1) run pwd and note the output; ` +
        `(2) run git branch --show-current and note the output; (3) run the Bash command echo SPIKE-EXPERIMENT-OK ; ` +
        `(4) use the Write tool to create a file named spike-scratch.txt in your current working directory ` +
        `containing that output. Return the finding, with cwd and branch from steps 1 and 2.`,
      { agentType: 'fspike:spike-runner', isolation: 'worktree', schema: FIND, phase: 'Workflow-led', label: 'T4-spike' },
    ),
  )()
  second = await safe(() =>
    agent(
      task +
        `A research spike was run for you and found: ${JSON.stringify(finding)}. Answer from that finding. ` +
        `Set needs_spike = false. Label "T4-after-spike".`,
      { agentType: 'fspike:builder', model: 'sonnet', effort: 'low', schema: NEED, phase: 'Workflow-led', label: 'T4-after' },
    ),
  )()
}

phase('Child')
const child = await safe(() => workflow('fspike:spike-child', { question: 'does echo print its argument?' }))()

return { agentLed, workflowLed: { first, finding, second }, child }
