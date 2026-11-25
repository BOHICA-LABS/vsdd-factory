export const meta = {
  name: 'vsdd-1-0-slice-step-3-plan',
  description: 'Plan step 3 of the thin slice: one build plan per unit (state core, records and decisions, survey and checkpoint, hook runner, jobs), reviewed for seams and corrected',
  whenToUse: 'Before the step 3 build workflows. Reads rewrite/spec/, rewrite/findings/ and next/; writes rewrite/plans/step-3/ only.',
  phases: [
    { title: 'Plan', detail: 'five planners, one unit each', model: 'fable' },
    { title: 'Review', detail: 'one reviewer checks the seams between the plans and against the code', model: 'opus' },
    { title: 'Integrate', detail: 'one planner corrects the plans', model: 'fable' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it.

Where things are:
- The design is rewrite/spec/, one small file per thing. Read rewrite/spec/README.md first. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it. Each build package has a brief, rewrite/spec/packages/<ID>.md, listing the things it implements. rewrite/spec/conventions.md gives names, paths and model tiers; rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md give the platform behaviour proven by experiment.
- rewrite/spec/packaging/build-plan.md has a section "The thin slice": the first milestone is one story delivered on the sample project through the real platform, in local delivery mode, and it says how deep each package is built for that. Step 3 of that list is: S1, S2a, S2b, S4 and the first part of S3; H1, then B4b; G1.
- The spec was converted from six larger documents kept in rewrite/history/design-v1/. Each spec file's frontmatter lists the sections it came from; read the original section when a spec file is silent or contradicts itself.
- Working code is under next/: the launcher and entries, the shared Python core (py/vsdd/), the result schemas, the workflow prelude, the build step, the workflow harness, and a sample project. Read it; the plans must fit it. Another workflow is changing next/ right now: it is bringing those foundations into line with the amended design, then building the policy tables (package C2) and the roster and lint (package B6). Plan against what those packages will deliver as their briefs describe, and say where your plan depends on them.
- Known problems with the design are in a ledger. rewrite/findings/by-package/<ID>.md lists the ones that concern the files a package implements, each with a status: open, not-fixed (a fix was tried and found incomplete) or left-for-builder (the design was settled and the code must follow). rewrite/findings/sources/slice-amend-result.json records the design decisions made for the slice, and rewrite/findings/verify-slice-amend.md what a check found wrong with them.

Fixed constraints: macOS and Linux only. Python is standard library only at run time, floor 3.11, started through uv. Shell is POSIX sh. Nothing is compiled. Product code goes under next/. Models for the build: a Sonnet builder writes the code from your plan, an Opus reviewer checks it. The builder has not read the design the way you have, so the plan is what it works from.

Do not write or change anything under next/, rewrite/spec/, rewrite/findings/, rewrite/history/ or rewrite/tools/. Do not commit, branch or push.`

const UNITS = [
  {
    key: 'state-core',
    packages: 'S1',
    title: 'the state core (package S1)',
    note: 'S1 is large (its brief lists about forty things). For the slice, plan what a single story needs: detecting a factory project, mounting the state directory and init on the sample project, the configuration schema and reading it, the artifact registry validation that a story and its contract need, creating and listing artifacts, and the global -C option. Say exactly which of its commands are built now and which wait. S1 owns the command-line dispatch that the other units register their commands with: define that registration so that the other units can add commands without editing the same file.',
  },
  {
    key: 'records',
    packages: 'S2a, S2b and S4',
    title: 'the record stores, the lock and caller identity, and decisions (packages S2a, S2b, S4)',
    note: 'For the slice: the run, step, evidence and closure stores a story writes and reads, with their schemas; the digest only if a story needs it; the run lock, caller identity and flush as the hook runner and the survey need them; and of decisions only what the story path touches. The platform probes settled how a hook and the tool can tell who is calling: read rewrite/spec/topics/platform-proven.md items 27 to 29 and 35. These stores are the shared state API that the hook runner, the gates and delivery all call, so give every function its exact signature, what it returns, and when it refuses.',
  },
  {
    key: 'survey',
    packages: 'the first part of S3',
    title: 'the survey, checkpoint and story admission for a single story (the first part of package S3)',
    note: 'S3 is too large for one builder: it holds the survey and checkpoint of all twelve workflows and the whole route. Plan only what delivering one story needs: the survey for the story workflow (what is done, the story unit and its position, its worktree), the worktree library, story admission, and the checkpoint that closes a delivered story. Say what the second part (needed after the gates exist) will add. The survey and checkpoint outputs are what the story script reads, so their fields must match the schemas the prelude uses.',
  },
  {
    key: 'hook-runner',
    packages: 'H1, then B4b',
    title: 'the hook runner core and the hook wiring (packages H1 and B4b)',
    note: 'H1: dispatch, the context a rule receives, output, failure behaviour, the inert path, the caller marker and the command rewrite, the event log, and the self-guarding command. B4b: generating hooks/hooks.json from the rule registry, with a check for drift. No rules are built in this unit beyond what proves the core works; the rules the slice needs (tests-first, merge-guard, step-results) come in step 4 and plug into what you define, so the rule interface must be exact. A probe showed that a missing launcher does not block anything, so the self-guarding command is load-bearing. Another showed that a hook can rewrite a tool call through the nested hookSpecificOutput.updatedInput.',
  },
  {
    key: 'jobs',
    packages: 'G1',
    title: 'jobs and the product environment (package G1)',
    note: 'Detached jobs that re-attach, keyed by command, subject and commit; the semaphore; and starting and stopping the product environment with ports. For the slice, the red and green gates will run the sample project test command through this. A probe showed a foreground shell call in a step runs for at least four minutes with an explicit timeout and that a detached job outlives the call that started it; a ten-minute call is not yet proven. Plan so that a gate never depends on one long foreground call.',
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
    findings_planned: { type: 'number' },
    findings_deferred: { type: 'number' },
    design_decisions: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'path', 'summary', 'files_owned', 'shared_files', 'provides', 'needs', 'findings_planned', 'findings_deferred', 'design_decisions', 'open_questions'],
}

const PLAN_FORMAT = `Write the plan as a Markdown file with these sections, in this order. Keep it under 24 KB: it is a plan, not a second design. Link to spec files by path instead of copying them.
1. Scope. What this unit builds now, in one paragraph, and a list of what it deliberately leaves for when the build widens.
2. Files. Every file the builder creates or changes under next/, one line each saying what goes in it. Mark each as owned (only this unit writes it) or shared (another unit also writes it); for a shared file say exactly what this unit adds and how the units avoid colliding.
3. Interfaces this unit provides. Every function another unit will call, with its module, exact signature, return value and the cases in which it refuses or raises; every command with its arguments, output fields and exit behaviour. Where the spec gives this, cite the spec file and state only what the spec leaves open. These are promises to the other units.
4. Interfaces this unit needs. What it calls in other units and in the code already under next/, by name.
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

Four other planners are planning the other units of step 3 at the same time: ${UNITS.filter((x) => x.key !== u.key).map((x) => x.title).join('; ')}. You cannot see their plans. Where your unit meets theirs, state the interface exactly, taking it from the spec where the spec gives it, so that a reviewer can check the plans against each other.

${PLAN_FORMAT}

Write the plan to rewrite/plans/step-3/${u.key}.md with the Write tool. Then return the structured result: files_owned and shared_files as paths under next/; provides and needs as one line per interface; design_decisions as one line each; open_questions only for things that cannot be decided without a person or a probe.`,
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

Five planners, working at the same time and unable to see each other's work, each wrote a plan for one unit of step 3 of the thin slice. The plans are in rewrite/plans/step-3/: ${done.map((p) => p.path).join(', ')}.${missing.length ? ` These units have no plan because their planner did not return: ${missing.join(', ')}; report that as a blocker.` : ''} A Sonnet builder will write each unit from its plan alone, and an Opus reviewer will check the code.

What the planners reported:
${JSON.stringify(done.map((p) => ({ unit: p.unit, files_owned: p.files_owned, shared_files: p.shared_files, provides: p.provides, needs: p.needs, design_decisions: p.design_decisions, open_questions: p.open_questions })))}

Read every plan in full, the spec files they cite where a seam is in doubt, and the code under next/ they build on. Find the problems that would only show up when the units are put together:
- an interface one plan needs that no plan provides, or that is provided with a different name, signature, return value or refusal case;
- two plans that write the same file without saying how they avoid colliding, or that each define the same function, record or command;
- a design decision in one plan that contradicts a decision in another, a spec file, a user decision in rewrite/STATE.md, or a platform fact that was proven;
- a plan that does not fit the code already under next/ (a module, function or schema it assumes that is not there, or is there with a different shape);
- a plan that builds more than the slice needs, or leaves out something the story path will hit;
- a finding from the ledger that a plan says it resolves, where the stated change would not resolve it;
- a failure that would be silent.
Then say which units can be built at the same time without touching the same files, and which must be built in order and why.

Write your report to rewrite/plans/step-3/REVIEW.md with the Write tool. Do not modify any other file. Return the structured result. Severity: "blocker" means the units would not fit together or a unit cannot be built from its plan; "major" means a real mismatch or gap that would cost a rebuild; "minor" is anything else. Give each finding an id (PL3-1, ...).`,
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
    plan_files: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'build_order', 'parallel_groups', 'plan_files'],
}

phase('Integrate')
let integrated = null
if (review) {
  integrated = await agent(
    `You are correcting five build plans so that the units they describe fit together. ${CONTEXT}

The plans are in rewrite/plans/step-3/. A reviewer read them against each other and against the code and wrote rewrite/plans/step-3/REVIEW.md. Its findings:
${JSON.stringify(review.findings)}

Its view of the build order: can be built at the same time: ${JSON.stringify(review.can_build_in_parallel)}; must be built in order: ${JSON.stringify(review.must_build_in_order)}.

Decide every finding and correct the plans with the Edit tool. Where two plans disagree about an interface, choose one form, taking it from the spec where the spec gives it, and make both plans state it identically. Where two plans write the same file, give the file one owner and say what the other unit hands to it. Keep each plan under 24 KB and in its eight sections. Then write rewrite/plans/step-3/ORDER.md: the order in which the units are built, which may be built at the same time, and for each unit the files it owns, so that a workflow can run the builds safely.

You may edit only the files in rewrite/plans/step-3/. Return the structured result with one resolution per finding.`,
    { label: 'integrate', phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: INTEGRATE_SCHEMA },
  )
}

return {
  missing,
  plans: done.map((p) => ({ unit: p.unit, path: p.path, summary: p.summary, files_owned: p.files_owned.length, shared_files: p.shared_files, findings_planned: p.findings_planned, findings_deferred: p.findings_deferred, open_questions: p.open_questions })),
  review: review ? { path: review.path, verdict: review.verdict, findings: review.findings.length, blockers: review.findings.filter((f) => f.severity === 'blocker').length, majors: review.findings.filter((f) => f.severity === 'major').length } : null,
  integrated,
}
