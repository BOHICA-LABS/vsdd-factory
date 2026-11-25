export const meta = {
  name: 'three',
  description: 'Round-5 leg B: three typed steps in a row under a tight budget cap',
  phases: [{ title: 'P' }],
}

const S = {
  type: 'object',
  properties: { n: { type: 'integer' }, t: { type: 'string' } },
  required: ['n', 't'],
}

phase('P')
const step = (n, t) =>
  agent(`Return n = ${n} and t = '${t}'. Nothing else.`, {
    agentType: 'p5:worker',
    model: 'sonnet',
    effort: 'low',
    schema: S,
    label: t,
    phase: 'P',
  })

const a = await step(1, 'T1')
const b = await step(2, 'T2')
const c = await step(3, 'T3')
return { a, b, c }
