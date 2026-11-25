export const meta = {
  name: 'vsdd-1-0-slice-step-6-plan',
  description: 'Plan step 6 of the thin slice: the story workflow with its scenario tests, and the story-ready fixture with the first headless run, reviewed against the step 4 and 5 plans and the code, and corrected',
  whenToUse: 'While step 3 finishes and before steps 4 and 5 are built. Reads rewrite/spec/, rewrite/findings/, rewrite/plans/ and next/; writes rewrite/plans/step-6/ only.',
  phases: [
    { title: 'Plan', detail: 'two planners: the story workflow; the fixture and the first run', model: 'fable' },
    { title: 'Review', detail: 'one reviewer walks the story from the first survey to the merge, across both plans, the step 4 and 5 plans and the code', model: 'opus' },
    { title: 'Integrate', detail: 'one planner corrects the plans and writes the build order', model: 'fable' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. Read rewrite/spec/README.md first. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it. Each build package has a brief, rewrite/spec/packages/<ID>.md, listing the things it implements. rewrite/spec/conventions.md gives names, paths and model tiers.
- rewrite/spec/packaging/build-plan.md has a section "The thin slice": the first milestone is one story delivered on the sample project through the real platform, in local delivery mode. Step 6, the last, is: W2 (the story workflow), then the story-ready fixture and its loader, then E2's first run: one story, headless. A package named there is finished for the slice when the files the story path touches pass their tests; the rest waits for when the build widens. No feature-flag package (F) and no team-use package (M) is on the slice.
- The platform behaviour proven by experiment is in rewrite/spec/platform.md, rewrite/spec/topics/platform-proven.md and rewrite/spec/topics/platform-proven-2.md (items 36 to 49 are the latest round). Read the section of platform.md headed "Headless runs: what the first one (step 6) must set": the variable CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS, the Bash sandbox off, a block is not an error, and the check that the named workflow is what ran. Nothing may be assumed of the platform that those files do not record as proven; a thing you need that is not proven is an open question for a probe, with the smallest probe that would settle it.
- The spec was converted from six larger documents kept in rewrite/history/design-v1/. Each spec file's frontmatter lists the sections it came from; read the original section when a spec file is silent or contradicts itself.
- Steps 1 to 3 are built under next/: the launcher and entries, the shared Python core (plugins/vsdd-factory/py/vsdd/), the result schemas, the workflow prelude (workflow-lib/prelude.js) and the build step that stamps it into a script (tools/build.py), the workflow harness (tests/workflows/harness.mjs, with toy scripts in tests/workflows/toys/), the policy tables, the roster and lint, the sample project (tests/e2e/sample-project), the state core, the record stores, the lock, caller identity and decisions, the hook runner, jobs, and the first part of the survey and checkpoint (its last fixes are being made while you plan; rewrite/plans/step-3/survey.md and ORDER.md say what it provides). Read the code: where the code and a step 3 plan differ, the code is what your unit will meet.
- Steps 4 and 5 are planned, corrected, and not built yet. Step 4 (rewrite/plans/step-4/: gates.md, verdicts.md, rules.md, survey-2.md, delivery.md, and ORDER.md) is the story gates, the verdicts, the hook rules, story positions with the checkpoint mapping, and local delivery. Step 5 (rewrite/plans/step-5/agents.md) is the ten agent files and fifteen method files the story script dispatches. Plan against the interfaces those plans promise, cite the plan and the interface, and never plan to change a file one of their units owns: say what you need from it instead. rewrite/plans/step-4/ORDER.md section 7 has a list headed "Inputs to W2's plan (step 6)" and a list of corrections to the step 5 plan; both are inputs here.
- Known problems with the design are in a ledger. rewrite/findings/by-package/<ID>.md lists the ones that concern the files a package implements, each with a status: open, not-fixed (a fix was tried and found incomplete) or left-for-builder (the design was settled and the code must follow). These entries of rewrite/findings/ledger.json were set aside for this step: S3P-1, S5P-N1, S5P-N9, S5P-N10, S5P-N11, probes-4-apply-C2-1 and layer-2a-fix-3-C1-1 (the scenario tests assert every dispatched step's model and effort against the roster).
- rewrite/STATE.md is the record of the whole rewrite: the user's standing decisions, and under "Waiting on the user" the defaults that stand until the user says otherwise (the sandbox is required off; local delivery refuses a story that touches a dependency manifest or CHANGELOG.md; a decision is settled with vsdd decide in a terminal).

Fixed constraints: macOS and Linux only. Python is standard library only at run time, floor 3.11, started through uv. Shell is POSIX sh. Workflow scripts are plain JavaScript with no file or process access: everything a script learns or changes goes through a step. Nothing is compiled. Product code goes under next/. Models for the build that follows: a Sonnet builder writes the code from your plan, an Opus reviewer checks it. The builder has not read the design the way you have, so the plan is what it works from.

Do not write or change anything under next/, rewrite/spec/, rewrite/findings/, rewrite/history/, rewrite/tools/ or the plans of steps 3 to 5. Do not commit, branch, stash or push, and never use git to discard or restore files: almost all of the work in this tree is uncommitted and other people are working in it. Run Python with -B so that no __pycache__ appears under next/plugins/.`

const UNITS = [
  {
    key: 'story-workflow',
    packages: 'W2',
    title: 'the story workflow script and its scenario tests (package W2)',
    note: `Read rewrite/spec/workflows/story.md, rewrite/spec/topics/story-script.md, the prelude topics (rewrite/spec/topics/prelude-*.md) and next/workflow-lib/prelude.js, rewrite/spec/packaging/tests-workflows.md, and the harness and toy scripts under next/tests/workflows/. Plan the story script as it will ship (the build step stamps the prelude in; say which regions are hand-written and which generated) and the scenario tests that run it in the harness with scripted step results.

The script must be right on every path, not only the one where each step succeeds first time. Walk and plan each of these, and give each a scenario test: a fresh story from the first survey to delivery; a start at every position the survey can report (the run was killed after any step); a red gate that fails and its repair; a green gate that fails and its repair; a review that finds something, the fix, and convergence; a review that does not converge within its rounds; a disputed finding sent to the architect's arbitrate method; a repair after review whose changed files touch a frozen test; a story with demo set and one without; delivery refused, with and without needs_person; a step that returns blocked; a relayed command that printed no JSON or was moved to the background (S3P-1, probes-4-apply-C2-1); a result the step-results rule blocks. For each say what the script passes to the step, what it reads from the result, and what it does next. The script judges nothing itself that the tool can measure: a gate, a verdict and delivery are the tool's answers, relayed.

Take in the list "Inputs to W2's plan (step 6)" of rewrite/plans/step-4/ORDER.md section 7 item by item, and the ledger entries named in the context. Every dispatched step's agent, method, model and effort must be those of the roster, and a scenario test asserts it for every step (layer-2a-fix-3-C1-1).

Say exactly how a run is started: what a person types or what a headless driver puts in its prompt, what the main session then does (which tool, with which name and arguments), and how the run is known to be the shipped script and not one the model wrote (platform item 49). The second planner is planning the fixture and the first headless run and depends on this; state it as an interface. If starting a run needs a file no package on the slice builds (a skill or a command file), plan the smallest one and name the package that owns its full form.`,
  },
  {
    key: 'first-run',
    packages: 'E1 (the story-ready fixture and its loader only) and E2 (the first run only)',
    title: 'the story-ready fixture with its loader, and the first headless run of one story (the fixture part of package E1 and the first run of package E2)',
    note: `Read rewrite/spec/packaging/sample-project.md (the section "The story-ready fixture"), rewrite/spec/packaging/tests-e2e.md, rewrite/spec/packaging/e2e-story-tier.md, the sample project under next/tests/e2e/, and next/tests/unit/story_ready.py (the builder of a ready project that the unit tests of steps 3 and 4 use; decide and say how the fixture loader and it relate, so that there are not two descriptions of a ready project that can drift apart).

Plan three things.
(a) The fixture overlay as data and the loader tests/e2e/fixtures/load.py, with --local: every file of the overlay with its content rule, the story "add median()" to the tally sample written so that it has a contract, acceptance criteria a test-writer can turn into failing tests, and touches no dependency manifest and no CHANGELOG.md; the order in which the loader calls the tool and writes files; how the spec decision's digest is computed; and the unit test that validates the overlay so that it cannot rot.
(b) The driver of the first run: one command a person runs from a terminal that loads the fixture into a fresh temporary directory outside the repository, starts claude -p with the plugin directory and exactly the settings the platform record proves are needed (the wait-ceiling variable, the sandbox off and how the driver makes sure of it or refuses to start, the allowed tools, the permission mode, the budget cap, the output format), waits, and then makes its assertions. Say which models the first run uses and why (the roster's own tiers, or the cheap override through VSDD_TEST_TIER), with the cost you expect for each; the default must be the cheaper one that still proves the path, with a switch for the other.
(c) The assertions, on files, git and the event log only, never on model prose: the named workflow is what ran; the failing-test commit precedes the implementation commit; the merged commit is the one the review approved; no builder-tier agent read a holdout path; the story's closure records it delivered; the integration branch holds the change and its tests pass; the run reached its end with no input; every state file is under its cap; agent starts and cost are recorded against the budget of 12 starts. And the diagnosis: when the run fails, what the driver prints and keeps (the session result, the event log, the run record, the worktree) so that the cause can be read without running it again, and how the same directory is resumed with the same command.

Leave for when the build widens, and list as such: the three kills, the eleven-minute gate, the pull-request path and the gh stand-in, e2e.yml, the smoke probes and the general probe runner of E1. But shape the driver so that those become more probes of the same form, not a second harness.

The first planner is planning the story workflow, including how a run is started and how the run is known to be the shipped script. You cannot see that plan: state what your driver assumes of it as an interface, taking it from rewrite/spec/workflows/story.md and the platform record.`,
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
    needs_from_earlier_steps: { type: 'array', items: { type: 'string' } },
    differs_from_step_4_or_5_plans: { type: 'array', items: { type: 'string' } },
    findings_planned: { type: 'number' },
    findings_deferred: { type: 'number' },
    design_decisions: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'path', 'summary', 'files_owned', 'shared_files', 'provides', 'needs', 'needs_from_earlier_steps', 'differs_from_step_4_or_5_plans', 'findings_planned', 'findings_deferred', 'design_decisions', 'open_questions'],
}

const PLAN_FORMAT = `Write the plan as a Markdown file with these sections, in this order. Keep it under 28 KB: it is a plan, not a second design. Link to spec files by path instead of copying them.
1. Scope. What this unit builds now, in one paragraph, and a list of what it deliberately leaves for when the build widens.
2. Files. Every file the builder creates or changes under next/, one line each saying what goes in it. Mark each as owned (only this unit writes it) or shared (another unit also writes it); for a shared file say exactly what this unit adds.
3. Interfaces this unit provides, exactly: names, arguments, outputs, refusals.
4. Interfaces this unit needs: in the code already under next/ (by module and function, as built), in the step 4 plans and the step 5 plan (cite the plan and the interface), and in the other unit of this step.
5. Design decisions. Each place where the spec was silent, ambiguous or contradicted itself for something this unit must build: what you decided and why, in two or three lines. Choose by these goals in order: correct and enforceable; low touch; restartable; lean.
6. Findings. Every finding in this unit's by-package lists and every ledger entry the context names: its id, then either the change that resolves it and the test that proves it, or "not at slice depth" with the package that must take it later.
7. Tests. The tests the builder writes, by name, with one line each on what it proves. Include one that shows a failure is loud.
8. Order of work, what must exist before the builder starts, and what the builder must not touch.`

phase('Plan')
const plans = await parallel(
  UNITS.map((u) => () =>
    agent(
      `You are planning one unit of a build. ${CONTEXT}

YOUR UNIT: ${u.title}. Read the brief or briefs for ${u.packages} in rewrite/spec/packages/, every spec file they list for the part you plan, the files those link to, and the by-package findings lists for those packages.

${u.note}

${PLAN_FORMAT}

Write the plan to rewrite/plans/step-6/${u.key}.md with the Write tool. Then return the structured result: files_owned and shared_files as paths under next/; provides, needs and needs_from_earlier_steps as one line per interface; differs_from_step_4_or_5_plans as one line for each place where what you need is not what those plans promise (so that they can be corrected before they are built); design_decisions as one line each; open_questions only for things that cannot be decided without a person or a probe.`,
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
  required: ['path', 'verdict', 'findings'],
}

phase('Review')
const review = await agent(
  `You are reviewing two build plans before any code is written from them. ${CONTEXT}

Two planners, working at the same time and unable to see each other's work, each wrote a plan for one unit of step 6 of the thin slice. The plans are in rewrite/plans/step-6/: ${done.map((p) => p.path).join(', ')}.${missing.length ? ` These units have no plan because their planner did not return: ${missing.join(', ')}; report that as a blocker.` : ''} A Sonnet builder will write each unit from its plan alone, and an Opus reviewer will check the code. After these two units are built, one story is run for the first time on the real platform, so a mismatch that the plans carry will be paid for in a failed run.

What the planners reported:
${JSON.stringify(done.map((p) => ({ unit: p.unit, files_owned: p.files_owned, shared_files: p.shared_files, provides: p.provides, needs: p.needs, needs_from_earlier_steps: p.needs_from_earlier_steps, differs_from_step_4_or_5_plans: p.differs_from_step_4_or_5_plans, design_decisions: p.design_decisions, open_questions: p.open_questions })))}

Read both plans in full, the step 4 plans with rewrite/plans/step-4/ORDER.md, the step 5 plan, the spec files they cite where a seam is in doubt, the platform record, and the code under next/ they build on. Then walk one story from the driver's command to the merge, step by step, as the two plans and the plans of steps 3 to 5 would have it run: the fixture loaded, the session started, the workflow started, the first survey, each dispatched step with what it is given and what it returns, each gate, the review loop, delivery, the checkpoint, the driver's assertions. At every hand-over check that the name, the arguments, the fields and the refusal cases are the same on both sides. Find:
- an interface one plan needs that nothing provides, or that is provided with a different name, shape or refusal case: between the two plans; against a step 4 or step 5 plan; against the code as built;
- a field the script passes to a step that the step 5 method does not expect, or a result field the script reads that the method or the schema does not fill;
- a path of the script on which the run cannot end: a loop with no bound, a state from which every run does the same thing again, a step whose failure is read as success, a blocked or refused result that is dropped;
- a start position the survey can report that the script does not handle, or handles by repeating work that evidence shows is done;
- a place where the script decides something that the tool should measure, or trusts a model's word for a gate;
- a scenario the plan lists with no test, or a test that would pass with the script wrong (for example one that checks only that a step ran, not what it was given);
- a step whose model or effort is not the roster's, or is not asserted;
- something the fixture gives the story that the survey, the gates or delivery will refuse (a missing record, a digest that does not match, a path the configuration does not name, a story that touches a protected file);
- something the driver assumes of the platform that the platform record does not prove, or a setting the record says is needed that the driver leaves out;
- an assertion that reads model prose, or that would pass on a run that did not deliver the story;
- anything built beyond what one story on one machine needs.

Write your report to rewrite/plans/step-6/REVIEW.md with the Write tool. Do not modify any other file. Return the structured result. Severity: "blocker" means the first run could not deliver a story from these plans, or a unit cannot be built from its plan; "major" means a real mismatch or gap that would cost a rebuild or a failed run; "minor" is anything else. Give each finding an id (PL6-1, ...). In "plans" name every plan a finding touches, including a step 4 or step 5 plan when the correction belongs there.`,
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
    needs_changed_in_step_4_or_5_plans: { type: 'array', items: { type: 'string' } },
    probes_needed_before_the_run: { type: 'array', items: { type: 'string' } },
    questions_for_the_person: { type: 'array', items: { type: 'string' } },
    plan_files: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'build_order', 'needs_changed_in_step_4_or_5_plans', 'probes_needed_before_the_run', 'questions_for_the_person', 'plan_files'],
}

phase('Integrate')
let integrated = null
if (review) {
  integrated = await agent(
    `You are correcting two build plans so that the units they describe fit together and fit the steps before them. ${CONTEXT}

The plans are in rewrite/plans/step-6/. A reviewer walked one story through them, the plans of steps 3 to 5 and the code, and wrote rewrite/plans/step-6/REVIEW.md. Its findings:
${JSON.stringify(review.findings)}

What the planners said differs from the step 4 and step 5 plans, and what they left open:
${JSON.stringify(done.map((p) => ({ unit: p.unit, differs_from_step_4_or_5_plans: p.differs_from_step_4_or_5_plans, open_questions: p.open_questions })))}

Decide every finding and correct the two plans with the Edit tool. Where the two plans disagree about an interface, choose one form, taking it from the spec where the spec gives it, and make both state it identically. Where a step 6 plan needs something a step 4 or step 5 plan does not promise, do not edit those plans: either change the step 6 plan to use what is promised, or list the need under needs_changed_in_step_4_or_5_plans, with the plan file, the section and the exact change, so that the orchestrating session can put it into that unit's build. Keep each plan under 28 KB and in its eight sections.

Then write two files in rewrite/plans/step-6/:
- ORDER.md: the order in which the two units are built and whether they may be built at the same time; the files each owns; the rules that cross the two units; what must be true of steps 3 to 5 before each starts; the checks to run; then a section "The first run" that gives the exact command, what it costs, what a pass looks like, and what to look at first when it fails; and a section "For the orchestrating session" listing the changes asked of the step 4 and step 5 builds, the probes needed before the run, and the questions for the person, each with the default that stands if the person does not answer.
- RESOLUTIONS.md: one line per review finding: its id, fixed, rejected or needs-a-person, and what was done.

You may create or edit only files in rewrite/plans/step-6/. Return the structured result with one resolution per finding.`,
    { label: 'integrate', phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: INTEGRATE_SCHEMA },
  )
}

return {
  missing,
  plans: done.map((p) => ({ unit: p.unit, path: p.path, summary: p.summary, files_owned: p.files_owned, shared_files: p.shared_files, needs_from_earlier_steps: p.needs_from_earlier_steps, differs_from_step_4_or_5_plans: p.differs_from_step_4_or_5_plans, findings_planned: p.findings_planned, findings_deferred: p.findings_deferred, design_decisions: p.design_decisions, open_questions: p.open_questions })),
  review: review ? { path: review.path, verdict: review.verdict, findings: review.findings, blockers: review.findings.filter((f) => f.severity === 'blocker').length, majors: review.findings.filter((f) => f.severity === 'major').length } : null,
  integrated,
}
