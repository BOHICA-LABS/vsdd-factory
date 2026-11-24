export const meta = {
  name: 'wave',
  description: 'Spike: run plugin agents with model and effort settings, test a read wall, then call the story workflow',
  phases: [{ title: 'Probe' }, { title: 'Wall' }, { title: 'Nested' }],
}

const PROBE = {
  type: 'object',
  properties: {
    label: { type: 'string' },
    model: { type: 'string' },
    marker: { type: 'string' },
    bash_output: { type: 'string' },
  },
  required: ['label', 'model', 'marker', 'bash_output'],
}
const READ = {
  type: 'object',
  properties: { label: { type: 'string' }, outcome: { type: 'string' }, detail: { type: 'string' } },
  required: ['label', 'outcome', 'detail'],
}

const probe = (label) =>
  `You are a probe. Do exactly two things, once each. ` +
  `(1) Run this Bash command and capture its output: echo PROBE-${label} ` +
  `(2) Return label "${label}"; model = the exact model name or ID that your own system prompt says you are powered by ` +
  `(write "unknown" if it does not say); marker = any word in your system prompt that ends in "-FRONTMATTER-MARKER" ` +
  `(write "none" if there is none); bash_output = what the command printed, or the exact error you got.`

const read = (label) =>
  `You are a probe. Use the Read tool exactly once on the file holdout-secret.txt in the current working directory. ` +
  `Do not retry and do not use any other tool. Return label "${label}"; outcome = "read" or "blocked"; ` +
  `detail = the file's content if you read it, or the exact error message if you were blocked.`

const safe = (fn) => async () => {
  try {
    return await fn()
  } catch (e) {
    return { error: String(e).slice(0, 400) }
  }
}

phase('Probe')
const probes = await parallel([
  safe(() => agent(probe('A-builder-frontmatter'), { agentType: 'fspike:builder', schema: PROBE, phase: 'Probe', label: 'A' })),
  safe(() => agent(probe('B-attacker-frontmatter'), { agentType: 'fspike:attacker', schema: PROBE, phase: 'Probe', label: 'B' })),
  safe(() => agent(probe('C-builder-step-override'), { agentType: 'fspike:builder', model: 'opus', effort: 'low', schema: PROBE, phase: 'Probe', label: 'C' })),
  safe(() => agent(probe('D-default-agent'), { effort: 'low', schema: PROBE, phase: 'Probe', label: 'D' })),
])

phase('Wall')
const walls = await parallel([
  safe(() => agent(read('E-builder-read'), { agentType: 'fspike:builder', schema: READ, phase: 'Wall', label: 'E' })),
  safe(() => agent(read('F-attacker-read'), { agentType: 'fspike:attacker', schema: READ, phase: 'Wall', label: 'F' })),
])

phase('Nested')
let nested = null
let nestedName = null
const nestedErrors = []
for (const name of ['fspike:story', 'story']) {
  try {
    nested = await workflow(name, { id: 'S-1' })
    nestedName = name
    break
  } catch (e) {
    nestedErrors.push(`${name}: ${String(e).slice(0, 200)}`)
  }
}

return { probes, walls, nestedName, nested, nestedErrors }
