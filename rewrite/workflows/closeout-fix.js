export const meta = {
  name: 'vsdd-1-0-closeout-fix',
  description: 'Fix the 21 problems the second-pass close-out left in the vsdd-factory 1.0 blueprint, then verify',
  whenToUse: 'Step 2c of the vsdd-factory 1.0 rewrite. Works from rewrite/design/pass2-closeout.md.',
  phases: [
    { title: 'Fix', detail: 'smallest change that resolves each remaining problem', model: 'fable' },
    { title: 'Verify', detail: 'check each fix and its neighbours', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const DOCS =
  'rewrite/BLUEPRINT.md and the five design documents rewrite/design/pipeline.md, rewrite/design/state.md, rewrite/design/agents-skills.md, rewrite/design/hooks.md, rewrite/design/packaging.md'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin, is in ${DOCS}. Section 4 of the blueprint is the authority on shared names. rewrite/STATE.md holds the user's decisions, the platform facts verified by experiment, and the requirements. The user adopted every recommendation in section 2 of the blueprint (decisions D1 to D30); none is open. The repository CLAUDE.md that was loaded into your context and the old plugin under plugins/vsdd-factory/ describe the system being replaced; their process rules are not requirements.

History you need: two review passes found 96 and then 118 problems. Each round of fixes resolved most of them and created a few new mismatches between documents, and each round made the documents longer. This is the last paper pass before building starts.`

const FIX_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    resolutions: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          outcome: { type: 'string', enum: ['fixed', 'not-fixed'] },
          what_was_done: { type: 'string' },
          where: { type: 'string' },
        },
        required: ['id', 'outcome', 'what_was_done', 'where'],
      },
    },
    net_size_change: { type: 'string' },
  },
  required: ['summary', 'resolutions', 'net_size_change'],
}

const VERIFY_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    clean: { type: 'boolean' },
    summary: { type: 'string' },
    checked: { type: 'number' },
    remaining: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          problem: { type: 'string' },
          where: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'problem', 'where', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'clean', 'summary', 'checked', 'remaining'],
}

const RULES = `Rules for every fix:
- Make the smallest change that resolves the problem. Prefer correcting or deleting text over adding it. Do not add a new command, record type, rule, agent, skill or build package unless the problem cannot be resolved without one, and say so if you do.
- Apply the change in every place the problem appears: the blueprint and each design document that states the same thing. Before you finish an item, search all six documents for the names involved and make them agree.
- Read the section around each edit, so that the fix does not contradict its neighbours. Most problems left at this stage were created by an earlier fix that did not look sideways.
- Do not reopen an adopted decision. Do not change anything that is not part of an item on the list.
- Keep the counts in section 1 of the blueprint true if a count changes.`

let workList = `the report rewrite/design/pass2-closeout.md. It lists 5 major problems (M1 to M5) and 16 minor ones (m1 to m16), each with its exact location. Read it in full; it is your work list`
let round = 0
let fix = null
let verify = null
const history = []

while (round < 2) {
  round += 1
  phase('Fix')
  fix = await agent(
    `You are fixing the last known problems in a design. ${CONTEXT}

YOUR WORK LIST: ${workList}.

${RULES}

When every item is done, add or update a short subsection at the end of section 11 of the blueprint titled "11.4 Close-out", with one table row per item: id, severity, what was done. Do not modify any file other than the blueprint and the five design documents. Do not commit, branch or push.

Return the structured result with one entry for every item on the work list. In net_size_change, say roughly how many lines you added and removed across the six documents.`,
    { label: `fix:round-${round}`, phase: 'Fix', model: 'fable', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (!fix) {
    log(`Fix round ${round} did not return`)
    break
  }

  phase('Verify')
  verify = await agent(
    `You are checking a set of fixes to a design. ${CONTEXT}

The problems that were to be fixed in this round: ${round === 1 ? 'the 21 items (M1 to M5, m1 to m16) in rewrite/design/pass2-closeout.md; read that report in full' : JSON.stringify(history[history.length - 1].remaining)}.

What the fixing agent says it did:
${JSON.stringify(fix.resolutions)}

For each item, open the place it names and confirm three things: the change is there; it resolves the problem as the report described it; and the blueprint and every design document that states the same thing now agree. Then read the section around each change and check that the fix has not created a new contradiction with its neighbours. Check every item, majors first. Report a new problem only if a fix from this round caused it; this is not another review of the whole design.

Write your report to rewrite/design/pass2-closeout-${round + 1}.md with the Write tool: a verdict, then anything still wrong with its exact location and a proposed resolution. Do not modify any other file. Do not commit, branch or push. Return the structured result. Set clean to true only if no blocker or major problem remains.`,
    { label: `verify:round-${round}`, phase: 'Verify', model: 'opus', effort: 'xhigh', schema: VERIFY_SCHEMA },
  )
  if (!verify) {
    log(`Verify round ${round} did not return`)
    break
  }
  history.push({ round, fixed: fix.resolutions.filter((r) => r.outcome === 'fixed').length, remaining: verify.remaining, clean: verify.clean })
  const serious = verify.remaining.filter((r) => r.severity !== 'minor')
  log(`Round ${round}: ${verify.remaining.length} remaining (${serious.length} blocker or major)`)
  if (verify.remaining.length === 0 || (verify.clean && round >= 1 && serious.length === 0 && verify.remaining.length <= 3)) break
  workList = `these problems, found by the check on the previous round of fixes: ${JSON.stringify(verify.remaining)}`
}

return {
  rounds: history.map((h) => ({ round: h.round, fixed: h.fixed, clean: h.clean, remaining: h.remaining.length })),
  last_fix: fix ? { summary: fix.summary, net_size_change: fix.net_size_change, not_fixed: fix.resolutions.filter((r) => r.outcome !== 'fixed') } : null,
  final: verify,
}
