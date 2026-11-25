export const meta = {
  name: 'q9',
  description: 'Round-4 probe Q9: Glob and Read on a gitignored worktree mount',
  phases: [{ title: 'P' }],
}

const TEXT = { type: 'object', properties: { label: { type: 'string' }, output: { type: 'string' } }, required: ['label', 'output'] }

phase('P')
const r = await agent(
  `Do exactly this, in order, and nothing else.
1. Call Glob with pattern '.factory/specs/**/*.md' and no path argument.
2. Call Glob with pattern '**/*.md' and path '.factory/specs'.
3. If either call returned at least one file, Read the first file it returned.
Return output = a report with exactly three lines:
LINE1: everything the first Glob call returned, verbatim, or the word EMPTY.
LINE2: everything the second Glob call returned, verbatim, or the word EMPTY.
LINE3: the single most distinctive marker string inside the file you Read, or the word NOREAD.
Do not use Bash. Do not use any other tool.`,
  { agentType: 'fs4:glober', schema: TEXT, label: 'Q9', phase: 'P' },
)
return [{ label: 'Q9', result: r }]
