export const meta = {
  name: 'story',
  description: 'Spike: child workflow called by the wave workflow',
  phases: [{ title: 'Story' }],
}

phase('Story')
const id = args && args.id ? args.id : 'no-args'
const reply = await agent(`Reply with exactly this text and nothing else: STORY-OK ${id}`, {
  effort: 'low',
  label: 'story-step',
  phase: 'Story',
})
return { id, reply }
