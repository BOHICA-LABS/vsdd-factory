export const meta = {
  name: 'vsdd-1-0-review-pass-3',
  description: 'Third full review of the vsdd-factory 1.0 design: five independent reviewers, read-only',
  whenToUse: 'Step 2d of the vsdd-factory 1.0 rewrite. Reads rewrite/BLUEPRINT.md and rewrite/design/, writes rewrite/design/pass3-*.md. Does not amend anything.',
  phases: [{ title: 'Review', detail: 'five reviewers, one lens each', model: 'opus' }],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const DESIGNS =
  'rewrite/design/pipeline.md, rewrite/design/state.md, rewrite/design/agents-skills.md, rewrite/design/hooks.md, rewrite/design/packaging.md'

const SLICE = `The first thing to be built is a thin slice: the least that delivers one story on a sample project through the real platform. Its path runs through: the uv launcher and the vsdd entry point; the shared Python core and the schemas; the workflow prelude, build step and test harness; project detection, mount and init; the run, step, evidence and closure records; the survey and checkpoint for a single story, with its worktree; the red and green gates; the review verdict; delivery in local mode with no remote; the hook runner core with the tests-first, merge-guard and step-results rules; the clerk, test-writer, implementer and adversary agents; and the story workflow. A finding that lies on this path matters most, because it will be hit first. Set on_slice_path to true for those.`

const COMMON = `You are an independent reviewer in the third review pass on the design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

The design is rewrite/BLUEPRINT.md plus five detail documents (${DESIGNS}). Section 4 of the blueprint is the authority on shared names. Read rewrite/STATE.md in full first: it holds the user's decisions, the requirements, and the platform facts verified by experiment. Section 10 of the blueprint lists which platform behaviours are proven and which are not. The user adopted every recommendation in section 2 of the blueprint (decisions D1 to D30); treat each as fixed.

History. A first review found 96 problems and a second found 118. Each round of fixes resolved most of them and created a few new mismatches, usually because a fix was made in one place and not carried to the others, or was not read against its neighbours. A close-out run then fixed 26 more. Five minor problems are already known and will be fixed (R1 to R5 in rewrite/design/pass2-closeout-3.md); do not report those again. Earlier reports are in rewrite/design/review-*.md and rewrite/design/pass2-*.md if you want to see what has been raised before, but do your own reading of the design first.

${SLICE}

The repository CLAUDE.md that was loaded into your context, the old plugin under plugins/vsdd-factory/, and the docs describe the system being replaced. Their process rules are not requirements of the new design. Surveys of the old system are in rewrite/reference/. Do not read or review anything under next/: code is being built there at the same time and is not your subject.

This is a fresh, full review, not a check of earlier fixes. Report real problems only. Every finding must give exact locations (file and section), say what breaks if it is left alone, and propose the smallest resolution that would fix it in every place it appears. Do not report style or wording. Severity: "blocker" means the design cannot be built or would not work as written; "major" means it breaks a user decision or a stated requirement, or leaves a real gap; "minor" is anything else worth fixing. Give each finding an id with your prefix and a number.`

const LENSES = [
  {
    key: 'seams',
    prefix: 'P3SEAM',
    brief: `YOUR LENS: the seams between the six documents. Look for a name, path, schema field, identifier, configuration key, command, caller permission or responsibility that differs between two documents, or between a document and section 4 of the blueprint. Check each document's statement of what it needs from the others against what the others provide. Pay particular attention to the places the last two rounds changed: the route and its record, story admission and worktrees, write scopes for manifests and scaffolds, the specification lock and its digest, the re-freeze of tests, read walls for reviewers and for spikes, and the D4 approval scope.`,
  },
  {
    key: 'coverage',
    prefix: 'P3COV',
    brief: `YOUR LENS: coverage and platform truth.
(a) Coverage. The user decided that every capability of the old system ports. Using the four surveys in rewrite/reference/, find a capability that no design carries forward and that was not dropped with a stated reason. Sample the mapping tables against the old files for correctness.
(b) Platform truth. Check every mechanism in the design against section 10 of the blueprint and the probe tables in rewrite/STATE.md. Find anything that relies on behaviour that was disproven, or that is still unproven and has no stated fallback. The probes established, among other things: a workflow script cannot read files or run commands; workflows nest one level; an agent running as a workflow step has no Agent tool; a refused stop does not hold a step that has already returned its result; worktree isolation in an agent file is ignored for a workflow step; a hook sees the whole step result and can make the agent return again; hook timeouts are in seconds; a foreground shell call is capped at ten minutes.`,
  },
  {
    key: 'operations',
    prefix: 'P3OPS',
    brief: `YOUR LENS: how this behaves in use. Walk through each of these against the design as it now stands and report where it fails or is silent:
- a session dies in the middle of a story and a new session starts cold;
- the main session compacts during a wave, and a step agent compacts during a long implementation;
- uv is missing, or the Python floor is not met;
- a hook rule throws an exception, and a hook times out;
- an agent returns a result that fails its schema, returns nothing, or lies about a gate;
- a review loop reaches its cap with a disputed finding;
- an agent asks for spike after spike;
- two sessions work on the same project, on one machine and on two;
- an existing codebase whose tests already fail is ingested, and then a bug fix is requested;
- a person edits an approved specification file by hand.
Also judge: whether any gate can switch off without anyone noticing; whether secrets or destructive commands can get past the gates; whether a person is ever left with no command to run next; and how many agent starts a typical story and a six-story wave now cost.`,
  },
  {
    key: 'requirements',
    prefix: 'P3REQ',
    brief: `YOUR LENS: the user's decisions and requirements. The sources are "Decisions made by the user" and "Requirements the blueprint must meet" in rewrite/STATE.md, and section 2 of the blueprint. For each one, find where the design satisfies it and judge it met, partly met or not met.

Walk these two end to end, as the user would experience them: what they type, what runs, where it stops for them, and what is on disk afterwards.
1. Research spikes, which the user wants "selectable anytime a human or agents thinks a research spike is warranted", from agents, skills and workflows. Walk each entrance in section 6 of the blueprint against what the platform allows.
2. Brownfield, which the user asked about directly. Walk the five starting points in section 3.4 of the blueprint, and for each say whether it now reaches a release without stalling: a codebase the factory did not build; a second feature, a bug fix and a queued third request after a first release; a project imported from the old plugin; a codebase with no tests and no CI; and a change across two repositories.

Also check the model policy, macOS and Linux only, Python through uv with the standard library only, setup once per clone and nothing more, and resume after compaction and from cold. Write a table in your report with one row per requirement. Report as findings only those partly met or not met.`,
  },
  {
    key: 'builder',
    prefix: 'P3BLD',
    brief: `YOUR LENS: read the design as the builder who has to implement the thin slice next week.

1. Walk the slice path named above, step by step, from a person typing the command that delivers one story to the merge. For each step, find the text a builder would implement from: the vsdd command and its arguments and output, the record it reads and writes with its fields, the hook rule and what it checks, the agent and its tool list, the step's prompt and result schema, and the line of the story workflow that calls it. Report every point where that text is missing, ambiguous, or says two different things. Say exactly which build packages in section 9 the slice needs, and to what depth, and whether those packages can be built in the order section 9 gives.
2. Check the build plan in section 9 as a whole: every vsdd command in section 4.9, every agent in 4.3, every skill and workflow in 4.5, every hook rule, schema and policy table is delivered by some package; dependencies are right and not circular; no package is too large for one builder and one reviewer.
3. Note any mechanism on the slice path that could be made simpler without losing an enforcement or a capability. Use severity "minor" for these and say what would be saved.`,
  },
]

const FINDINGS_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict_summary: { type: 'string' },
    checked: { type: 'number' },
    sound: { type: 'number' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          on_slice_path: { type: 'boolean' },
          documents: { type: 'array', items: { type: 'string' } },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'on_slice_path', 'documents', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict_summary', 'checked', 'sound', 'findings'],
}

phase('Review')
const reviews = await parallel(
  LENSES.map((lens) => () =>
    agent(
      `${COMMON}\n\n${lens.brief}\n\nOUTPUT\n\nWrite your report to rewrite/design/pass3-${lens.key}.md with the Write tool: a one-paragraph verdict, then one section per finding (id starting ${lens.prefix}-, severity, whether it is on the slice path, exact locations, the problem, what breaks, the proposed resolution), then a short list of what you checked and found sound. Do not modify any other file. Do not commit, branch or push. Then return the structured result with every finding in it. For "checked" and "sound", give the number of items you examined and the number you found sound.`,
      { label: `review:${lens.key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: FINDINGS_SCHEMA },
    ),
  ),
)

const missing = LENSES.filter((_, i) => !reviews[i]).map((l) => l.key)
if (missing.length) log(`Reviewers that did not return: ${missing.join(', ')}`)
const findings = reviews.filter(Boolean).flatMap((r) => r.findings)
const count = (s) => findings.filter((f) => f.severity === s).length
log(`Review pass 3: ${findings.length} findings (${count('blocker')} blocker, ${count('major')} major, ${count('minor')} minor); ${findings.filter((f) => f.on_slice_path).length} on the slice path`)

return {
  missing,
  reviews: reviews.map((r, i) =>
    r ? { key: LENSES[i].key, path: r.path, verdict: r.verdict_summary, checked: r.checked, sound: r.sound, findings: r.findings.length } : { key: LENSES[i].key, failed: true },
  ),
  findings,
}
