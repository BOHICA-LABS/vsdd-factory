export const meta = {
  name: 'vsdd-1-0-slice-step-4-plan',
  description: 'Plan step 4 of the thin slice: one build plan per unit (story gates, verdicts, the second part of the survey, delivery, the delivery-order and step-result rules), reviewed for seams and corrected',
  whenToUse: 'After the step 3 plans are corrected. Reads rewrite/spec/, rewrite/findings/, rewrite/plans/step-3/ and next/; writes rewrite/plans/step-4/ only.',
  phases: [
    { title: 'Plan', detail: 'five planners, one unit each', model: 'fable' },
    { title: 'Review', detail: 'one reviewer checks the seams between the plans, against step 3 and against the code', model: 'opus' },
    { title: 'Integrate', detail: 'one planner corrects the plans and writes the build order', model: 'fable' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. Read rewrite/spec/README.md first. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it. Each build package has a brief, rewrite/spec/packages/<ID>.md, listing the things it implements. rewrite/spec/conventions.md gives names, paths and model tiers; rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md give the platform behaviour proven by experiment.
- rewrite/spec/packaging/build-plan.md has a section "The thin slice": the first milestone is one story delivered on the sample project through the real platform, in local delivery mode, and it says how deep each package is built for that. Step 4 of that list is: G2 and G3; the second part of S3; G4a; H5 and H7. rewrite/spec/topics/story-script.md and rewrite/spec/workflows/story.md say what the story workflow calls, in what order; a thing the story path never reaches is not built now.
- The spec was converted from six larger documents kept in rewrite/history/design-v1/. Each spec file's frontmatter lists the sections it came from; read the original section when a spec file is silent or contradicts itself.
- Step 3 comes before this step: the state core (S1), the record stores, lock, caller identity and decisions (S2a, S2b, S4), the first part of the survey and checkpoint (S3), the hook runner core and hook wiring (H1, B4b), and jobs with the product environment (G1). Its seven plans are in rewrite/plans/step-3/, already reviewed against each other and corrected. Read ORDER.md there first: section 3 gives the files each unit owns, section 4 the rules that cross units (how a command is written and registered, flat names, who is calling, person only, git, globs, path classes, test projects, schema names), and section 7 ends with a list headed "Step 4 planners build on these". Step 3 is being built from those plans while you plan, so its code under next/ is incomplete and changing. Plan against the interfaces the step 3 plans promise in their section 3, cite the plan file and the interface you use, and never plan to change a file a step 3 unit owns: say what you need from it instead.
- The agents the story workflow dispatches are planned as step 5, in rewrite/plans/step-5/agents.md. Its sections 6 and 8 say what the agent and method files assume of the gates, the survey and the story unit: the name a gate result carries, what gate red --refreeze does when no frozen test changed, and the paths that must reach a step. rewrite/plans/step-5/RESOLUTIONS.md lists the questions it left open. Meet those assumptions, or say exactly where your plan differs so that the step 5 plan can be corrected. The ledger findings S5P-N1 to S5P-N12 (rewrite/findings/ledger.json) record the gaps that planning found; the ones triaged to your packages are yours.
- Working code is under next/: the launcher and entries, the shared Python core (py/vsdd/), the result schemas, the workflow prelude, the build step, the workflow harness, the policy tables, the roster and lint, and a sample project. Read it; the plans must fit it.
- Known problems with the design are in a ledger. rewrite/findings/by-package/<ID>.md lists the ones that concern the files a package implements, each with a status: open, not-fixed (a fix was tried and found incomplete) or left-for-builder (the design was settled and the code must follow). rewrite/findings/sources/slice-amend-result.json records the design decisions made for the slice, and rewrite/findings/verify-slice-amend.md what a check found wrong with them.

Fixed constraints: macOS and Linux only. Python is standard library only at run time, floor 3.11, started through uv. Shell is POSIX sh. Nothing is compiled. Product code goes under next/. Models for the build: a Sonnet builder writes the code from your plan, an Opus reviewer checks it. The builder has not read the design the way you have, so the plan is what it works from.

Do not write or change anything under next/, rewrite/spec/, rewrite/findings/, rewrite/history/, rewrite/tools/ or rewrite/plans/step-3/. Do not commit, branch, stash or push, and never use git to discard or restore files: almost all of the work in this tree is uncommitted and other people are working in it.`

const UNITS = [
  {
    key: 'gates',
    packages: 'G2',
    title: 'the story gates (package G2)',
    note: 'The red gate (JUnit results, freezing the tests, --refreeze), the green gate with its comparison against the baseline, the demo gate, and the diff patch writer. For the slice these run the sample project test command through the jobs unit of step 3 (rewrite/plans/step-3/jobs.md) and write evidence through the record stores (rewrite/plans/step-3/records.md). A gate must never depend on one long foreground call, and a gate that cannot measure must say so loudly: it must never pass by default. The survey reaches the gates through the gate registry (rewrite/spec/topics/evidence-lookup.md), so define how a gate registers itself, without the survey importing a gate package.',
  },
  {
    key: 'verdicts',
    packages: 'G3',
    title: 'verdicts (package G3)',
    note: 'The verdict function and grounding, review show with redaction of walled findings, review prepare, and the conformance fixtures that prove the Python verdict function and the judge() in the workflow prelude (next/workflow-lib/prelude.js) give the same answer for the same input. Read rewrite/spec/topics/verdicts.md. For the slice: what the review, fix and arbitrate loop of one story needs. The fixtures are shared by the Node and Python tests, so say exactly where they live and what shape they have.',
  },
  {
    key: 'survey-2',
    packages: 'the second part of S3',
    title: 'story positions, measuring a missing gate again, and the checkpoint mapping of results to closures and decisions (the second part of package S3)',
    note: 'The first part of S3 is planned in rewrite/plans/step-3/survey.md, which says what it leaves for this part. Plan exactly that remainder for one story: the position of a story unit worked out from evidence, measuring a gate again when its evidence is missing or stale, and the checkpoint mapping a step result to closures and decisions. You extend files the first part owns: say for each which functions you add and which you change, and why that is safe once step 3 is built. The survey and checkpoint outputs are what the story script reads, so their fields must match the schemas the prelude uses.',
  },
  {
    key: 'delivery',
    packages: 'G4a',
    title: 'delivery (package G4a)',
    note: 'story deliver. The slice delivers one story in local delivery mode, so plan local mode completely: the verdict check, the patch id, the merge with the head-commit match, adoption, and what is recorded. Plan the remote path (push, pull request with the story trailer, CI wait, delivery.ci, the gh test double) only as far as local mode shares code with it, and list the rest as left for when the build widens. Delivery is the last thing between a story and the main branch: every refusal must be exact and tested, and a kill at any point must leave a state the next run can finish from.',
  },
  {
    key: 'rules',
    packages: 'H5 and H7',
    title: 'the delivery-order rules and the step-result rules (packages H5 and H7)',
    note: 'tests-first and merge-guard (H5); decisions and step-results, with the agent type stored and walled findings (H7). They plug into the hook runner core of step 3: read rewrite/plans/step-3/hook-runner.md for the rule interface, the context a rule receives, how a rule is registered and how hooks/hooks.json is generated from the registry. A probe showed that a hook can rewrite a tool call through the nested hookSpecificOutput.updatedInput, and that a missing launcher does not block. For the slice: what one story on the sample project exercises. A rule that fails to load or crashes must not silently allow what it exists to stop; say what happens in each such case and test it.',
  },
]

const PLAN_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    path: { type: 'string' },
    summary: { type: 'string' },
    files_owned: { type: 'array', items: { type: 'string' } },
    shared_files: { type: 'array', items: { type: 'string' } },
    provides: { type: 'array', items: { type: 'string' } },
    needs: { type: 'array', items: { type: 'string' } },
    needs_from_step_3: { type: 'array', items: { type: 'string' } },
    findings_planned: { type: 'number' },
    findings_deferred: { type: 'number' },
    design_decisions: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'path', 'summary', 'files_owned', 'shared_files', 'provides', 'needs', 'needs_from_step_3', 'findings_planned', 'findings_deferred', 'design_decisions', 'open_questions'],
}

const PLAN_FORMAT = `Write the plan as a Markdown file with these sections, in this order. Keep it under 24 KB: it is a plan, not a second design. Link to spec files by path instead of copying them.
1. Scope. What this unit builds now, in one paragraph, and a list of what it deliberately leaves for when the build widens.
2. Files. Every file the builder creates or changes under next/, one line each saying what goes in it. Mark each as owned (only this unit writes it) or shared (another unit, of this step or of step 3, also writes it); for a shared file say exactly what this unit adds and how the units avoid colliding.
3. Interfaces this unit provides. Every function another unit will call, with its module, exact signature, return value and the cases in which it refuses or raises; every command with its arguments, output fields and exit behaviour. Where the spec gives this, cite the spec file and state only what the spec leaves open. These are promises to the other units.
4. Interfaces this unit needs. What it calls in other units of this step, in step 3 (cite the step 3 plan and the interface) and in the code already under next/, by name.
5. Design decisions. Each place where the spec was silent, ambiguous or contradicted itself for something this unit must build: what you decided and why, in two or three lines. Choose by these goals in order: correct and enforceable; low touch; restartable; lean.
6. Findings. Every finding in this unit's by-package lists: its id, then either the change that resolves it and the test that proves it, or "not at slice depth" with the package that must take it later.
7. Tests. The tests the builder writes, by name, with one line each on what it proves. Include at least one test per interface this unit provides and one that shows a failure is loud.
8. Order of work, and what must exist before the builder starts.`

phase('Plan')
const plans = await parallel(
  UNITS.map((u) => () =>
    agent(
      `You are planning one unit of a build. ${CONTEXT}

YOUR UNIT: ${u.title}. Read the brief or briefs for ${u.packages} in rewrite/spec/packages/, every spec file they list, the files those link to, and the by-package findings lists for those packages.

${u.note}

Four other planners are planning the other units of step 4 at the same time: ${UNITS.filter((x) => x.key !== u.key).map((x) => x.title).join('; ')}. You cannot see their plans. Where your unit meets theirs, state the interface exactly, taking it from the spec where the spec gives it, so that a reviewer can check the plans against each other.

${PLAN_FORMAT}

Write the plan to rewrite/plans/step-4/${u.key}.md with the Write tool. Then return the structured result: files_owned and shared_files as paths under next/; provides, needs and needs_from_step_3 as one line per interface; design_decisions as one line each; open_questions only for things that cannot be decided without a person or a probe.`,
      { label: `plan:${u.key}`, phase: 'Plan', model: 'fable', effort: 'xhigh', schema: PLAN_SCHEMA },
    ),
  ),
)
const missing = UNITS.filter((_, i) => !plans[i]).map((u) => u.key)
if (missing.length) log(`Plans that did not return: ${missing.join(', ')}`)
const done = plans.filter(Boolean)

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict: { type: 'string' },
    can_build_in_parallel: { type: 'array', items: { type: 'string' } },
    must_build_in_order: { type: 'array', items: { type: 'string' } },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          plans: { type: 'array', items: { type: 'string' } },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'plans', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict', 'can_build_in_parallel', 'must_build_in_order', 'findings'],
}

phase('Review')
const review = await agent(
  `You are reviewing five build plans before any code is written from them. ${CONTEXT}

Five planners, working at the same time and unable to see each other's work, each wrote a plan for one unit of step 4 of the thin slice. The plans are in rewrite/plans/step-4/: ${done.map((p) => p.path).join(', ')}.${missing.length ? ` These units have no plan because their planner did not return: ${missing.join(', ')}; report that as a blocker.` : ''} A Sonnet builder will write each unit from its plan alone, and an Opus reviewer will check the code.

What the planners reported:
${JSON.stringify(done.map((p) => ({ unit: p.unit, files_owned: p.files_owned, shared_files: p.shared_files, provides: p.provides, needs: p.needs, needs_from_step_3: p.needs_from_step_3, design_decisions: p.design_decisions, open_questions: p.open_questions })))}

Read every plan in full, the step 3 plans and rewrite/plans/step-3/ORDER.md, the spec files they cite where a seam is in doubt, and the code under next/ they build on (step 3 may be partly built by now; where the code and a step 3 plan differ, the code is what the step 4 units will meet, so say so). Find the problems that would only show up when the units are put together:
- an interface one plan needs that no plan of step 3 or step 4 provides, or that is provided with a different name, signature, return value or refusal case;
- two plans that write the same file without saying how they avoid colliding, or that each define the same function, record or command; a step 4 plan that changes a file a step 3 unit owns without saying exactly what it adds;
- a design decision in one plan that contradicts a decision in another plan of either step, a spec file, a user decision in rewrite/STATE.md, or a platform fact that was proven;
- a plan that does not fit the code already under next/ (a module, function or schema it assumes that is not there, or is there with a different shape);
- a plan that builds more than the slice needs, or leaves out something the story path will hit: walk the story script (rewrite/spec/topics/story-script.md) step by step and check that every command, gate, rule and record it reaches is planned in step 3 or step 4;
- a finding from the ledger that a plan says it resolves, where the stated change would not resolve it;
- a failure that would be silent, above all a gate, a rule or a delivery check that would pass or allow when it could not measure.
Then say which units can be built at the same time without touching the same files, and which must be built in order and why.

Write your report to rewrite/plans/step-4/REVIEW.md with the Write tool. Do not modify any other file. Return the structured result. Severity: "blocker" means the units would not fit together or a unit cannot be built from its plan; "major" means a real mismatch or gap that would cost a rebuild; "minor" is anything else. Give each finding an id (PL4-1, ...).`,
  { label: 'review:plans', phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
)

const INTEGRATE_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    resolutions: {
      type: 'array',
      items: {
        type: 'object',
        properties: { id: { type: 'string' }, outcome: { type: 'string', enum: ['fixed', 'rejected', 'needs-a-person'] }, what_was_done: { type: 'string' } },
        required: ['id', 'outcome', 'what_was_done'],
      },
    },
    build_order: { type: 'array', items: { type: 'string' } },
    parallel_groups: { type: 'array', items: { type: 'string' } },
    needs_changed_in_step_3: { type: 'array', items: { type: 'string' } },
    plan_files: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'build_order', 'parallel_groups', 'needs_changed_in_step_3', 'plan_files'],
}

phase('Integrate')
let integrated = null
if (review) {
  integrated = await agent(
    `You are correcting five build plans so that the units they describe fit together. ${CONTEXT}

The plans are in rewrite/plans/step-4/. A reviewer read them against each other, against the step 3 plans and against the code, and wrote rewrite/plans/step-4/REVIEW.md. Its findings:
${JSON.stringify(review.findings)}

Its view of the build order: can be built at the same time: ${JSON.stringify(review.can_build_in_parallel)}; must be built in order: ${JSON.stringify(review.must_build_in_order)}.

Decide every finding and correct the plans with the Edit tool. Where two plans disagree about an interface, choose one form, taking it from the spec where the spec gives it, and make both plans state it identically. Where two plans write the same file, give the file one owner and say what the other unit hands to it. Where a step 4 plan needs something step 3 does not provide, do not edit the step 3 plans: put the need in the step 4 plan that must build it, or list it under needs_changed_in_step_3 if only a step 3 unit can provide it. Keep each plan under 24 KB and in its eight sections.

Then write two files in rewrite/plans/step-4/:
- ORDER.md: the order in which the units are built, which may be built at the same time, and for each unit the files it owns, so that a workflow can run the builds safely.
- RESOLUTIONS.md: one line per review finding: its id, fixed, rejected or needs-a-person, and what was done.

You may create or edit only files in rewrite/plans/step-4/. Return the structured result with one resolution per finding.`,
    { label: 'integrate', phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: INTEGRATE_SCHEMA },
  )
}

return {
  missing,
  plans: done.map((p) => ({ unit: p.unit, path: p.path, summary: p.summary, files_owned: p.files_owned.length, shared_files: p.shared_files, needs_from_step_3: p.needs_from_step_3.length, findings_planned: p.findings_planned, findings_deferred: p.findings_deferred, open_questions: p.open_questions })),
  review: review ? { path: review.path, verdict: review.verdict, findings: review.findings.length, blockers: review.findings.filter((f) => f.severity === 'blocker').length, majors: review.findings.filter((f) => f.severity === 'major').length } : null,
  integrated,
}
