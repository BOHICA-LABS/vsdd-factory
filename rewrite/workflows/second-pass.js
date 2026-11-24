export const meta = {
  name: 'vsdd-1-0-second-pass',
  description: 'Second pass on the vsdd-factory 1.0 blueprint: verify the first-pass fixes, look for cuts, trace requirements, then amend and close out',
  whenToUse: 'Step 2b of the vsdd-factory 1.0 rewrite. Reads rewrite/BLUEPRINT.md and rewrite/design/, writes rewrite/design/pass2-*.md and amends the blueprint.',
  phases: [
    { title: 'Review', detail: 'six reviewers: three verify fixes, one looks for cuts, one cold read, one requirements trace', model: 'opus' },
    { title: 'Amend', detail: 'decide every finding and amend the blueprint and design documents', model: 'fable' },
    { title: 'Close out', detail: 'check the amendments landed', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const DESIGNS = 'rewrite/design/pipeline.md, rewrite/design/state.md, rewrite/design/agents-skills.md, rewrite/design/hooks.md, rewrite/design/packaging.md'

const COMMON = `You are an independent reviewer in the second review pass on the design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Background. Five designers wrote one design document each, in parallel (${DESIGNS}). Three reviewers then found 96 problems (rewrite/design/review-seams.md, review-coverage.md, review-operations.md). One agent decided all 96, wrote rewrite/BLUEPRINT.md, and edited the five design documents to match. Section 8 of the blueprint lists each finding and what was done. Nobody has checked that work. That is what this pass is for.

Read rewrite/STATE.md in full first. It holds the user's decisions, the platform facts verified by experiment, and the requirements. Then read rewrite/BLUEPRINT.md in full. Section 4 of the blueprint is the authority on shared names.

On 2026-11-23 the user adopted every recommendation in section 2 of the blueprint (decisions D1 to D30). Treat each recommendation as a fixed decision, not as an open question.

The repository CLAUDE.md that was loaded into your context, the old plugin under plugins/vsdd-factory/, and the docs describe the system being replaced. Their process rules are not requirements of the new design. Surveys of the old system are in rewrite/reference/.

Report real problems only. Every finding must give exact locations (file and section), say what breaks if it is left alone, and propose a resolution. Do not report style or wording. Severity: "blocker" means the design cannot be built or would not work as written; "major" means it breaks a user decision or a stated requirement, or leaves a real gap; "minor" is anything else worth fixing. Give each finding an id with your prefix and a number.`

const VERIFY = (prefix, report, count) => `YOUR TASK: verify the first-pass fixes for the findings in ${report} (${count} findings, ids starting ${prefix}-).

Read that report so you know each original problem. Then, for each finding, take the resolution stated in section 8 of the blueprint and check that it was really carried out: find the text in the blueprint and in every design document where the problem appeared, and judge it.
- verified: the fix is present, complete, and consistent in every place the problem appeared.
- partial: the blueprint has the fix but a design document still says the old thing, or only part of the problem is fixed.
- not fixed: the stated resolution is not in the documents, or does not solve the problem.
- regressed: the fix is present but creates a new problem. Say what.
Work through the blockers first, then the majors, then the minors. Check every one; do not sample.

Report as findings only those that are partial, not fixed, or regressed. Use the original id inside your finding text so it can be traced, and use the id prefix P2${prefix}- for your own findings. Count how many you checked and how many you verified.`

const REVIEWERS = [
  {
    key: 'verify-seams',
    prefix: 'P2SEAM',
    file: 'rewrite/design/pass2-verify-seams.md',
    brief: VERIFY('SEAM', 'rewrite/design/review-seams.md', 44),
  },
  {
    key: 'verify-coverage',
    prefix: 'P2COV',
    file: 'rewrite/design/pass2-verify-coverage.md',
    brief: VERIFY('COV', 'rewrite/design/review-coverage.md', 21),
  },
  {
    key: 'verify-operations',
    prefix: 'P2OPS',
    file: 'rewrite/design/pass2-verify-operations.md',
    brief: VERIFY('OPS', 'rewrite/design/review-operations.md', 31),
  },
  {
    key: 'lean',
    prefix: 'P2LEAN',
    file: 'rewrite/design/pass2-lean.md',
    brief: `YOUR TASK: find what can be cut.

The old system collapsed under its own machinery: about 195,000 lines of Rust for a hook dispatcher, and far more bookkeeping commits than product commits. The first design pass produced 843 KB of design, a vsdd command with dozens of subcommands, many record types, 17 hook rules, about 70 method files and about 70 build packages. The user asked for a design that is low touch and lean. Your job is to find mechanism that can be removed or simplified.

A cut is acceptable only if it does not (a) remove a capability the user asked to keep (the user decided that every capability of the old system ports), (b) weaken an enforcement the design depends on, or (c) contradict a user decision in STATE.md or section 2 of the blueprint.

Look hard at:
- the record types and stores: run, step, evidence, closure, decision, cycle, request, note; which of them are needed to resume or to enforce, and which only describe;
- digests, the run lock and liveness, detached jobs, the semaphore;
- the number of vsdd subcommands, and whether some are one command with a flag;
- clerk steps, which are about a third of all agent starts: where a step that already runs can run the command as its last act instead of a separate agent;
- policy tables, generated copies and the build step;
- the four test layers and the harness;
- anything that exists only to guard against an agent lying, where one check at the point of no return already catches it;
- anything that re-creates the old bookkeeping (the operations review raised this as OPS-28);
- build packages that could merge, and packages that exist for a rare case.

For each proposed cut, state: what is removed; what it saves, in countable terms (commands, record types, rules, packages, agent starts per story or per wave); what is lost or what risk returns; and whether you recommend it. Put what is lost in the "problem" field and the cut itself in "proposed_resolution". Use severity "major" for a cut you strongly recommend and "minor" for one worth considering; do not use "blocker". Rank them by value in your report. Also give your overall judgment in the verdict: is this design lean enough to build as it stands?`,
  },
  {
    key: 'cold-read',
    prefix: 'P2COLD',
    file: 'rewrite/design/pass2-cold-read.md',
    brief: `YOUR TASK: read the blueprint as the builder who has to implement it, then check the design documents against it.

Find:
- contradictions inside the blueprint, between its sections;
- places where a design document still disagrees with section 4 of the blueprint. The amendments were applied by one agent in one pass. Check names of agents, skills, workflows, vsdd commands, state paths, configuration keys, schema fields and model tiers in each design document against section 4;
- names that are used but never defined;
- completeness of the build plan in section 9: every vsdd command in section 4.9, every agent in 4.3, every skill and workflow in 4.5, every hook rule, every schema and every policy table must be delivered by some package. List anything that is not;
- packages whose dependencies are wrong, missing or circular, and packages too large for one builder and one reviewer;
- steps in the story walk-through of section 3.3, the spike protocol of section 6 and the resume table of section 7 that the detailed designs do not actually support;
- anything in sections 3 to 7 that contradicts a user decision.`,
  },
  {
    key: 'requirements',
    prefix: 'P2REQ',
    file: 'rewrite/design/pass2-requirements.md',
    brief: `YOUR TASK: trace every user decision and requirement to where the design satisfies it.

The sources are: "Decisions made by the user" and "Requirements the blueprint must meet" in rewrite/STATE.md; and section 2 of the blueprint, where every recommendation is now a decision the user has adopted.

Two things the user asked about directly must be walked through end to end, as a user would experience them: what they type, what runs, where it stops for them, and what exists on disk afterwards.

1. Research spikes. The user said: "make sure vsdd has the ability to run spikes, from agents, to skills, to workflows, etc. it should be selectable anytime a human or agent thinks a research spike is warranted." Walk each entrance: a person; the main session on its own judgment; an agent dispatched in a normal session; a skill; an agent running as a workflow step; a workflow; a workflow that is already a child. Check each against the spike results in STATE.md, which record what the platform actually allows.

2. Brownfield. The user asked: "did we also include brownfield capabilities? so being able to add features to an existing project or when we finish the initial build specified by our greenfield process?" Walk these cases:
   a. an existing project the factory did not build, with code and tests but no factory state: setup, ingest, then a feature;
   b. the same, but the codebase has no tests and no CI;
   c. a project the factory built from a brief, after its first release: a second feature, then a bug fix, then a third feature requested while the second is still in flight;
   d. a project that used the old plugin (pre-1.0 state), imported, then a feature;
   e. a multi-repository project where a feature touches two repositories.
   For each, say whether the design carries it through without a gap, and where it would stall.

Also check: the model policy (Fable for planning, Opus for every independent reviewer, Sonnet for the rest, the clerk tier); macOS and Linux only; Python launched through uv with the standard library only; no activation step beyond one setup per clone; resume after compaction and from a cold start; every old capability carried forward.

Write a table in your report with one row per requirement: met, partly met or not met, and where. Report as findings only the ones that are partly met or not met.`,
  },
]

const FINDINGS_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict_summary: { type: 'string' },
    checked: { type: 'number' },
    verified: { type: 'number' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          documents: { type: 'array', items: { type: 'string' } },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'documents', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict_summary', 'checked', 'verified', 'findings'],
}

const AMEND_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    resolutions: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          outcome: { type: 'string', enum: ['fixed', 'cut-applied', 'offered-to-user', 'rejected'] },
          what_was_done: { type: 'string' },
          where: { type: 'string' },
        },
        required: ['id', 'outcome', 'what_was_done', 'where'],
      },
    },
    cuts_for_user: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          cut: { type: 'string' },
          saves: { type: 'string' },
          loses: { type: 'string' },
          recommendation: { type: 'string' },
        },
        required: ['cut', 'saves', 'loses', 'recommendation'],
      },
    },
    new_decisions_for_user: {
      type: 'array',
      items: {
        type: 'object',
        properties: { question: { type: 'string' }, recommendation: { type: 'string' } },
        required: ['question', 'recommendation'],
      },
    },
    counts_after: { type: 'string' },
    files_changed: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'cuts_for_user', 'new_decisions_for_user', 'counts_after', 'files_changed'],
}

const CLOSEOUT_SCHEMA = {
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
        },
        required: ['id', 'severity', 'problem', 'where'],
      },
    },
  },
  required: ['path', 'clean', 'summary', 'checked', 'remaining'],
}

// ---------------------------------------------------------------- Review
phase('Review')
const reviews = await parallel(
  REVIEWERS.map((r) => () =>
    agent(
      `${COMMON}\n\n${r.brief}\n\nOUTPUT\n\nWrite your report to ${r.file} with the Write tool: a one-paragraph verdict, then one section per finding (id starting ${r.prefix}-, severity, exact locations, the problem, what breaks, the proposed resolution), then a short list of what you checked and found sound. Do not modify any other file. Do not commit, branch or push. Then return the structured result with every finding in it. For "checked" and "verified", give the number of items you examined and the number you found sound.`,
      { label: `review:${r.key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: FINDINGS_SCHEMA },
    ),
  ),
)
const missing = REVIEWERS.filter((_, i) => !reviews[i]).map((r) => r.key)
if (missing.length) log(`Reviewers that did not return: ${missing.join(', ')}`)
const findings = reviews.filter(Boolean).flatMap((r) => r.findings)
const bySeverity = (s) => findings.filter((f) => f.severity === s).length
log(`Review done: ${findings.length} findings (${bySeverity('blocker')} blocker, ${bySeverity('major')} major, ${bySeverity('minor')} minor)`)
reviews.forEach((r, i) => {
  if (r) log(`${REVIEWERS[i].key}: checked ${r.checked}, sound ${r.verified}, findings ${r.findings.length}`)
})

// ---------------------------------------------------------------- Amend
phase('Amend')
const amended = await agent(
  `You are amending the blueprint for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Read rewrite/STATE.md in full, then rewrite/BLUEPRINT.md in full. The detail designs are ${DESIGNS}. Six reviewers have just completed a second pass and their reports are in rewrite/design/pass2-*.md${missing.length ? ` (these reviewers did not return: ${missing.join(', ')}; say so in the blueprint)` : ''}. The repository CLAUDE.md that was loaded into your context and the old plugin describe the system being replaced; their process rules are not requirements.

On 2026-11-23 the user adopted every recommendation in section 2 of the blueprint (decisions D1 to D30). Each recommendation is now fixed.

The second-pass findings, in full:
${JSON.stringify(findings)}

Do this in order.

1. Decide every finding. Each is fixed (say how and where), rejected (say why it is wrong), or, for a proposed cut, applied or offered to the user.
   - Findings from the three verification reviewers (ids P2SEAM-, P2COV-, P2OPS-) say a first-pass fix is partial, missing or has regressed. Finish the fix in every document where the problem appears.
   - Findings from the cold read (P2COLD-) and the requirements trace (P2REQ-): fix them. If a requirement cannot be met on the platform as verified in STATE.md, say so plainly in the blueprint and give the nearest thing that can be built.
   - Proposed cuts (P2LEAN-): apply a cut when it removes mechanism without removing a capability, weakening an enforcement the design depends on, or contradicting a user decision, and the reviewer's case is sound. Offer the rest to the user with what each saves and loses. Prefer the leaner design when the case is close: the user asked for low touch and lean, and the first pass grew large.
   When findings conflict, choose by the design goals in this order: correct and enforceable; low touch; restartable; lean. User decisions are fixed.

2. Amend rewrite/BLUEPRINT.md with the Edit tool.
   - Section 2 becomes a record: state at its top that the user adopted every recommendation on 2026-11-23, keep the tables as the record of what was decided, and fold each decision into the sections it affects so that no section still treats it as open. The two findings that were waiting on decision D4 (COV-04, OPS-12) are now resolved as yes; update section 8.
   - Apply every fix and every applied cut to the sections they touch, including the rosters and counts in sections 1, 3 and 4 and the build plan in section 9. Keep the counts in the summary true.
   - Add a section 11, "Second pass", with a table of every second-pass finding: id, severity, outcome, what was done. Then a short list of cuts offered to the user, and any new decision only a person can make. Keep new decisions to the minimum; do not reopen an adopted one.
   - Keep the blueprint a document a person can read from the top: complete sentences, terms defined, tables for rosters.

3. Apply the same changes to the five design documents with the Edit tool so they agree with the blueprint, and extend the amendment note at the top of each.

Do not modify anything other than rewrite/BLUEPRINT.md and the five design documents. Do not commit, branch or push. Then return the structured result: one resolution entry for every finding id, and in counts_after a one-line statement of the new counts (agents, skills, workflows, hook rules, vsdd commands, record types, build packages). Keep the summary under 200 words.`,
  { label: 'amend:blueprint', phase: 'Amend', model: 'fable', effort: 'xhigh', schema: AMEND_SCHEMA },
)

// ---------------------------------------------------------------- Close out
phase('Close out')
let closeout = null
if (amended) {
  const toCheck = amended.resolutions.filter((r) => r.outcome === 'fixed' || r.outcome === 'cut-applied')
  closeout = await agent(
    `You are doing the close-out check on the design for vsdd-factory 1.0. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Read rewrite/STATE.md, then rewrite/BLUEPRINT.md in full. A second review pass produced ${findings.length} findings, and one agent has just amended the blueprint and the five design documents (${DESIGNS}) to resolve them. Your job is to check that work, not to review the design again.

The findings:
${JSON.stringify(findings.map((f) => ({ id: f.id, severity: f.severity, problem: f.problem })))}

What the amending agent says it did:
${JSON.stringify(toCheck)}

For each resolution marked fixed or cut-applied, open the place it names and confirm that the change is there, that it does what the finding needed, and that the blueprint and the design documents now agree on it. Check blockers and majors first, and check every one of them. Then check that the counts in section 1 of the blueprint match the rosters in section 4 and the packages in section 9, that section 2 no longer treats any adopted decision as open, and that section 11 lists every finding id.

Write your report to rewrite/design/pass2-closeout.md with the Write tool: a verdict, then anything that is still wrong with its exact location. Do not modify any other file. Do not commit, branch or push. Return the structured result. Set clean to true only if no blocker or major problem remains.`,
    { label: 'closeout', phase: 'Close out', model: 'opus', effort: 'xhigh', schema: CLOSEOUT_SCHEMA },
  )
}

return {
  reviews: reviews.map((r, i) =>
    r
      ? { key: REVIEWERS[i].key, path: r.path, verdict: r.verdict_summary, checked: r.checked, verified: r.verified, findings: r.findings.length }
      : { key: REVIEWERS[i].key, failed: true },
  ),
  findings: findings.map((f) => ({ id: f.id, severity: f.severity, problem: f.problem.slice(0, 260), proposed: f.proposed_resolution.slice(0, 200) })),
  amended,
  closeout,
}
