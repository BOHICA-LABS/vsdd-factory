export const meta = {
  name: 'vsdd-1-0-slice-build-unit',
  description: 'Thin slice: build one unit from its plan (Sonnet), review it (Opus), fix every finding (Sonnet) and confirm the fixes (Opus), recording every finding and outcome in the ledger',
  whenToUse: 'After a step has corrected plans and an ORDER.md under rewrite/plans/step-<n>/. One run per unit. Pass args {step, key, title, packages, concurrent?, notes?, plan?, order?}. Run units at the same time only where ORDER.md allows it.',
  phases: [
    { title: 'Build', detail: 'one builder writes the unit from its plan', model: 'sonnet' },
    { title: 'Review', detail: 'checks re-run, outcomes checked, findings written to the ledger', model: 'opus' },
    { title: 'Fix', detail: 'every review finding fixed and re-tested, up to two rounds', model: 'sonnet' },
    { title: 'Confirm', detail: 'each fix confirmed against the code and its tests', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const A = args || {}
for (const name of ['step', 'key', 'title', 'packages']) {
  if (!A[name]) throw new Error(`args.${name} is required`)
}
const STEP = A.step
const KEY = A.key
const TITLE = A.title
const PACKAGES = A.packages
const UNIT = `step${STEP}-${KEY}`
const PLAN = A.plan || `rewrite/plans/step-${STEP}/${KEY}.md`
const ORDER = A.order || `rewrite/plans/step-${STEP}/ORDER.md`
const OTHERS = A.concurrent || []
const NOTES = A.notes || ''
const BUILD_ID = `S${STEP}B-${KEY}`
const REVIEW_ID = `S${STEP}R-${KEY}`

const CHECKS = `From next/: uv run --no-sync python tools/check.py ; uv run --no-sync python -m pytest -q ; node --test tests/workflows/universal.test.mjs. From the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/findings.py build. If uv cannot reach the package index, add --system-certs or use what is already installed. Run Python with -B or PYTHONDONTWRITEBYTECODE=1 when you import the plugin's code yourself: a __pycache__ folder under next/plugins/ makes two checks fail for everyone working in this tree. If you find one there that your own command made, delete it.`

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the layout and the link form [[kind:id]]. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it; each package has a brief, rewrite/spec/packages/<ID>.md. rewrite/spec/conventions.md gives names, paths and tiers; rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md give the platform behaviour proven by experiment; rewrite/spec/packaging/build-plan.md has a section "The thin slice" that says how deep each package is built now.
- The spec was converted from six larger documents kept in rewrite/history/design-v1/. Each spec file's frontmatter lists the sections it came from. Read the original section when a spec file is silent or contradicts itself.
- Working code is under next/; the plugin is next/plugins/vsdd-factory/.
- Known problems with the design are in a ledger: rewrite/findings/ledger.json, with one list per package in rewrite/findings/by-package/<ID>.md. Status open means nobody has dealt with it; not-fixed means a fix was tried and found incomplete; left-for-builder means the design was settled and the code must follow.
- The build plans for this step are in rewrite/plans/step-${STEP}/: one per unit, REVIEW.md (what a reviewer found when reading the plans against each other) and ORDER.md (the build order and the files each unit owns).

Fixed constraints: macOS and Linux only. Shell is POSIX sh and must run under sh, bash and dash. Python is standard library only at run time, floor 3.11, started through uv. Nothing is compiled. All product code goes under next/. Do not edit anything under plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Do not commit, branch, stash or push, and never use git to discard or restore files (no checkout, restore, reset, clean or stash): almost all of the work in this tree is uncommitted and other people are working in it.`

const SHARED_TREE = OTHERS.length
  ? `Other builders are building these units in this same working tree at the same time: ${OTHERS.join('; ')}. ${ORDER} says which files each unit owns. Do not create or edit a file it gives to another unit; if you need something from one, use the interface its plan promises and report what was missing. While you work, run only your own test files, because the whole suite can fail on another unit's half-written files. At the end run every check. If a failure is in a file another unit owns, wait a few minutes and run again; if it is still there, do not fix it: list it under foreign_failures.`
  : `No other unit is being built in this working tree while you work. Units built before yours are finished; build on their code as it is.`

const RECORDS = `Record your work in two files, both valid JSON arrays, written with the Write tool:
- rewrite/findings/outcomes/${UNIT}.json: one object per ledger finding you dealt with: {"id": "...", "outcome": "fixed" | "not-fixed" | "left-for-builder" | "rejected", "what_was_done": "...", "where": "file and test"}. Use "left-for-builder" for a finding that does not arise at slice depth and say which later package must take it.
- rewrite/findings/sources/extra-${UNIT}.json: one object per new problem you found in the design, in the plan or in code outside your unit: {"id": "${BUILD_ID}-<n>", "origin": "slice step ${STEP}, ${KEY}, builder", "severity": "blocker" | "major" | "minor", "where": "the spec, plan or code file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true or false}. Write an empty array if there are none.
Then run python3 rewrite/tools/findings.py build once and confirm it still works.`

const BUILD_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    summary: { type: 'string' },
    files_changed: { type: 'array', items: { type: 'string' } },
    commands_run: {
      type: 'array',
      items: {
        type: 'object',
        properties: { command: { type: 'string' }, result: { type: 'string' } },
        required: ['command', 'result'],
      },
    },
    tests_passed: { type: 'number' },
    tests_failed: { type: 'number' },
    findings_fixed: { type: 'number' },
    findings_left: { type: 'array', items: { type: 'string' } },
    new_findings: { type: 'number' },
    departures_from_plan: { type: 'array', items: { type: 'string' } },
    spec_files_changed: { type: 'array', items: { type: 'string' } },
    foreign_failures: { type: 'array', items: { type: 'string' } },
    deferred: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'summary', 'files_changed', 'commands_run', 'tests_passed', 'tests_failed', 'findings_fixed', 'findings_left', 'new_findings', 'departures_from_plan', 'spec_files_changed', 'foreign_failures', 'deferred'],
}

const FINDING = {
  type: 'object',
  properties: {
    id: { type: 'string' },
    severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
    file: { type: 'string' },
    problem: { type: 'string' },
    proposed_fix: { type: 'string' },
  },
  required: ['id', 'severity', 'file', 'problem', 'proposed_fix'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    verdict: { type: 'string' },
    tests_rerun: { type: 'string' },
    interfaces_checked: { type: 'number' },
    interfaces_as_promised: { type: 'number' },
    outcomes_checked: { type: 'number' },
    outcomes_confirmed: { type: 'number' },
    findings: { type: 'array', items: FINDING },
  },
  required: ['unit', 'verdict', 'tests_rerun', 'interfaces_checked', 'interfaces_as_promised', 'outcomes_checked', 'outcomes_confirmed', 'findings'],
}

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
          outcome: { type: 'string', enum: ['fixed', 'rejected', 'not-fixed'] },
          what_was_done: { type: 'string' },
          test: { type: 'string' },
        },
        required: ['id', 'outcome', 'what_was_done', 'test'],
      },
    },
    tests_passed: { type: 'number' },
    tests_failed: { type: 'number' },
    checks: { type: 'string' },
    foreign_failures: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'tests_passed', 'tests_failed', 'checks', 'foreign_failures'],
}

const CONFIRM_SCHEMA = {
  type: 'object',
  properties: {
    tests_rerun: { type: 'string' },
    checked: { type: 'number' },
    confirmed: { type: 'number' },
    still_open: { type: 'array', items: FINDING },
  },
  required: ['tests_rerun', 'checked', 'confirmed', 'still_open'],
}

phase('Build')
const built = await agent(
  `You are building one unit of vsdd-factory 1.0. ${WHERE}

YOUR UNIT: ${TITLE} (${PACKAGES}).

Your plan is ${PLAN}. A planner who read the whole design wrote it; a reviewer read it against the other units' plans and against the code; and it was corrected. Read it in full first, then ${ORDER}, then the spec files and the code it cites.${NOTES ? `\n\n${NOTES}` : ''}

How to work:
- Build what the plan says, in the order its last section gives, to the depth its first section gives. Do not build ahead; list what you left for later under deferred.
- The interfaces in the plan's section 3 are promises to the other units, which are built against them. Build each with exactly the name, signature, return value and refusals stated, and give each a test.
- Write the tests the plan's section 7 names, and at least one that shows a failure is loud. A gate or check that switches off without saying so is the worst defect this project can have.
- Close the ledger findings the plan's section 6 lists, each with a test that would fail without the fix.
- If the plan is wrong (it does not fit the code, it contradicts the design where that matters, or it cannot work), do what is correct, keep the promised interfaces unless they are themselves the fault, and report each departure under departures_from_plan and as a new finding.
- The design follows what runs. Where you built something the spec file that owns the fact does not say, make that spec file say it (one fact in one place), and run python3 rewrite/tools/spec_lint.py afterwards.
- Every claim that something works must come from a command you ran.

${SHARED_TREE}

The checks. ${CHECKS} They pass today; run them before you change anything so you know your starting point.

${RECORDS}

When you are done, run every check once more and return the structured result. Use "${UNIT}" as the unit. Keep the summary under 150 words.`,
  { label: `build:${KEY}`, phase: 'Build', model: 'sonnet', effort: 'xhigh', schema: BUILD_SCHEMA },
)

if (!built) {
  log(`The builder for ${KEY} did not return. Nothing was reviewed.`)
  return { unit: UNIT, build: null, review: null, rounds: [], still_open: [], note: 'builder did not return; inspect the tree before relaunching' }
}

phase('Review')
const review = await agent(
  `You are reviewing newly written code for vsdd-factory 1.0. ${WHERE}

THE UNIT UNDER REVIEW: ${TITLE} (${PACKAGES}). Its plan is ${PLAN}; ${ORDER} says which files it owns.

What the builder says it did:
${JSON.stringify(built)}

You did not write this and should not trust the account. Do four things.
1. Run the checks yourself and report what they print. ${CHECKS}${OTHERS.length ? ` Other units are being built in this tree at the same time (${OTHERS.join('; ')}); a failure in a file another unit owns is not a finding against this unit, but say that you saw it.` : ''}
2. Check every interface the plan's section 3 promises against the code: name, module, signature, return value, each refusal. Other units are built against these promises, so a difference is at least major unless the builder reported the departure and it is right. Count how many you checked and how many are as promised.
3. Read rewrite/findings/outcomes/${UNIT}.json. For every entry marked fixed, open the finding in the ledger and confirm that the fix is there and that a test would fail without it: break the code briefly in a scratch copy outside the repository if you need to be sure, never in place. For every entry marked left-for-builder or rejected, judge whether that was right. Count how many you checked and how many you confirmed.
4. Review the code, tests and tables this unit changed under next/, and the spec files it changed. Look for: behaviour that differs from the plan or the design; a claim the code does not support; a test that cannot fail, or behaviour with no test; a failure that would be silent; a race or a partial write that a kill at the wrong moment would leave behind (this tool must be restartable); shell that is not portable; anything beyond the Python standard library at run time; a spec file that now says something the code does not do; a fact now stated in two places; code built beyond what the slice needs.

Severity: "blocker" means it does not work or would break the units built on it; "major" means wrong behaviour, a silent failure, a promised interface that differs, a finding marked fixed that is not, or a missing test for something that matters; "minor" is anything else worth fixing. Give each finding an id (${REVIEW_ID}-1, ${REVIEW_ID}-2, ...), the file, the problem and a proposed fix.

Write your findings to rewrite/findings/sources/extra-${UNIT}-review.json with the Write tool, as a JSON array with one object per finding: {"id": "...", "origin": "slice step ${STEP}, ${KEY}, review", "severity": "...", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true}. Write an empty array if you have none. That is the only file you may create or change. Then return the structured result with the same findings.`,
  { label: `review:${KEY}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
)

const rounds = []
let open = review ? review.findings : []
if (!review) log(`The reviewer for ${KEY} did not return. The unit is built but unreviewed.`)
else log(`Review of ${KEY}: ${review.verdict}. ${open.length} findings, ${open.filter((f) => f.severity !== 'minor').length} blocker or major.`)

for (let round = 1; round <= 2 && open.length; round++) {
  const suffix = round === 1 ? '' : `-${round}`
  const fixed = await agent(
    `You are fixing review findings in one unit of vsdd-factory 1.0. ${WHERE}

THE UNIT: ${TITLE} (${PACKAGES}). Its plan is ${PLAN}; ${ORDER} says which files it owns.

${round === 1 ? 'An independent reviewer read the unit after it was built and found these problems.' : 'These findings were given to a fixer once already, and an independent check found them still open or found that the fix broke something. Read what the check says and fix them properly this time.'} Fix every one of them, the minor ones included:
${JSON.stringify(open)}

For each finding: make the fix; add or correct the test that would have caught it, so that the test fails without the fix; re-run that test. Keep the interfaces the plan's section 3 promises unless the finding is about one of them, and if you change one, say so in what_was_done, because other units are built against it. If a finding says a ledger entry marked fixed is not fixed, fix it properly. If you are sure a finding is wrong, change nothing for it, mark it rejected and say why in a way the reviewer can check.

${SHARED_TREE}

Write rewrite/findings/outcomes/${UNIT}-fix${suffix}.json with the Write tool: a JSON array with one object per finding above, {"id": "...", "outcome": "fixed" | "rejected" | "not-fixed", "what_was_done": "...", "where": "file and test"}, plus one object for any other ledger finding whose outcome your work changed. When all are done, run every check and report what they print. ${CHECKS} Return the structured result with one resolution per finding.`,
    { label: `fix${suffix}:${KEY}`, phase: 'Fix', model: 'sonnet', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (!fixed) {
    log(`The fixer for ${KEY} (round ${round}) did not return. ${open.length} findings stay open.`)
    rounds.push({ round, fix: null, confirm: null })
    break
  }
  const confirm = await agent(
    `You are confirming fixes in one unit of vsdd-factory 1.0. ${WHERE}

THE UNIT: ${TITLE} (${PACKAGES}). Its plan is ${PLAN}.

A reviewer found these problems:
${JSON.stringify(open)}

A fixer says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

You did not make these fixes and should not trust the account. Run the checks yourself and report what they print. ${CHECKS} Then, for every finding: open the code and confirm the problem is gone; confirm there is a test that fails without the fix (break the code briefly in a scratch copy outside the repository if you need to be sure, never in place); and confirm the fix did not break a promised interface or anything next to it. For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed; if not, the finding is still open.

List under still_open every finding that is not properly fixed, keeping its id, with what is still wrong and what to do. If a fix introduced a new problem, add it with a new id (${REVIEW_ID}-C${round}-1, ...). Write rewrite/findings/outcomes/${UNIT}-confirm${suffix}.json with the Write tool: a JSON array with one object per finding in still_open that kept its id, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are fixed. If you added new problems, also write them to rewrite/findings/sources/extra-${UNIT}-confirm${suffix}.json as a JSON array of {"id": "...", "origin": "slice step ${STEP}, ${KEY}, confirmation", "severity": "...", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true}. Those are the only files you may create or change. Return the structured result.`,
    { label: `confirm${suffix}:${KEY}`, phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
  )
  rounds.push({
    round,
    fix: { summary: fixed.summary, tests_passed: fixed.tests_passed, tests_failed: fixed.tests_failed, checks: fixed.checks, foreign_failures: fixed.foreign_failures, rejected: fixed.resolutions.filter((r) => r.outcome !== 'fixed') },
    confirm: confirm ? { tests_rerun: confirm.tests_rerun, checked: confirm.checked, confirmed: confirm.confirmed, still_open: confirm.still_open.length } : null,
  })
  if (!confirm) {
    log(`The confirmation for ${KEY} (round ${round}) did not return. The fixes are unconfirmed.`)
    break
  }
  log(`Round ${round} for ${KEY}: ${confirm.confirmed} of ${confirm.checked} fixes confirmed, ${confirm.still_open.length} still open.`)
  open = confirm.still_open
}

return {
  unit: UNIT,
  build: {
    summary: built.summary,
    files_changed: built.files_changed.length,
    tests_passed: built.tests_passed,
    tests_failed: built.tests_failed,
    findings_fixed: built.findings_fixed,
    findings_left: built.findings_left,
    new_findings: built.new_findings,
    departures_from_plan: built.departures_from_plan,
    spec_files_changed: built.spec_files_changed,
    foreign_failures: built.foreign_failures,
    deferred: built.deferred,
  },
  review: review
    ? {
        verdict: review.verdict,
        tests_rerun: review.tests_rerun,
        interfaces_checked: review.interfaces_checked,
        interfaces_as_promised: review.interfaces_as_promised,
        outcomes_checked: review.outcomes_checked,
        outcomes_confirmed: review.outcomes_confirmed,
        findings: review.findings.length,
        blockers: review.findings.filter((f) => f.severity === 'blocker').length,
        majors: review.findings.filter((f) => f.severity === 'major').length,
      }
    : null,
  rounds,
  still_open: review ? open : [],
}
