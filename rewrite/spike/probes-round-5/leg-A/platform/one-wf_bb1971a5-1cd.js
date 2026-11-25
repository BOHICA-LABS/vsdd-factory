export const meta = {
  name: 'one',
  description: 'Round-5 leg A: one typed step over bin/, methods/ and the story worktree',
  phases: [{ title: 'P' }],
}

const PLUGIN = typeof args === 'string' ? args.trim() : String(args)

const RESULT = {
  type: 'object',
  properties: {
    bin_bare: { type: 'string' },
    bin_abs: { type: 'string' },
    method: { type: 'string' },
    wrote: { type: 'string' },
    edited: { type: 'string' },
    commit: { type: 'string' },
    problems: { type: 'string' },
  },
  required: ['bin_bare', 'bin_abs', 'method', 'wrote', 'edited', 'commit', 'problems'],
}

const PROMPT = `Do these steps in order, each exactly once. The current working directory is a git project with a linked worktree at .worktrees/x.
1. Bash: run \`probe-marker\` (the bare command name). Remember its exact stdout, or the exact error text.
2. Bash: run \`${PLUGIN}/bin/probe-marker\`. Remember its exact stdout, or the exact error text.
3. Read the file ${PLUGIN}/methods/probe.md. Remember its exact full text.
4. Write the file .worktrees/x/leg5-write.md with exactly one line: WRITEMARKER-R5
5. Edit the file .worktrees/x/notes.md: change the string TODO-5 into DONE-5.
6. Bash: run \`git -C .worktrees/x add -A\`
7. Bash: run \`git -C .worktrees/x commit -m leg5\`
Return exactly: bin_bare = step 1's stdout or its exact error text; bin_abs = step 2's stdout or its exact error text; method = step 3's text; wrote = the exact content of .worktrees/x/leg5-write.md as you wrote it; edited = the exact line of .worktrees/x/notes.md that now holds DONE-5; commit = the first line of step 7's output or its exact error text; problems = anything else that went wrong, else the empty string.`

phase('P')
const result = await agent(PROMPT, {
  agentType: 'p5:worker',
  model: 'sonnet',
  effort: 'low',
  schema: RESULT,
  label: 'ONE',
  phase: 'P',
})
return result
