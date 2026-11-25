export const meta = {
  name: 'vsdd-1-0-slice-fix-open',
  description: 'Thin slice: fix a list of findings that earlier rounds left open (Sonnet), confirm each fix (Opus), and repeat once if anything is still open',
  whenToUse: 'When a build or confirmation run reports findings still open. Pass args {name, about, findings: [{id, severity, file, problem, proposed_fix}], others?}. name is the unit name for the record files.',
  phases: [
    { title: 'Fix', detail: 'every finding fixed with a test that fails without the fix', model: 'sonnet' },
    { title: 'Confirm', detail: 'each fix confirmed against the code and its tests', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const A = args || {}
for (const name of ['name', 'about', 'findings']) {
  if (!A[name]) throw new Error(`args.${name} is required`)
}
if (!Array.isArray(A.findings) || !A.findings.length) throw new Error('args.findings must be a non-empty array')
const NAME = A.name
const ABOUT = A.about
const OTHERS = A.others || ''

const CHECKS = `From next/: uv run --no-sync python tools/check.py ; uv run --no-sync python -m pytest -q ; node --test tests/workflows/universal.test.mjs ; uv run --no-sync python tools/build.py --check. From the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/findings.py build. If uv cannot reach the package index, add --system-certs or use what is already installed. Run Python with -B or PYTHONDONTWRITEBYTECODE=1 when you import the plugin's code yourself: a __pycache__ folder under next/plugins/ makes two checks fail for everyone. If you find one there that your own command made, delete it.`

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the layout and the link form [[kind:id]]. Each build package has a brief, rewrite/spec/packages/<ID>.md. The spec was converted from six larger documents kept in rewrite/history/design-v1/, which settle any doubt about what a spec file should say.
- Working code is under next/; the plugin is next/plugins/vsdd-factory/.
- Known problems are in a ledger under rewrite/findings/: raw findings in sources/, outcomes in outcomes/, and ledger.json built from them by python3 rewrite/tools/findings.py build. Look a finding up there by its id to read its history.

Other people are working in this tree. ${OTHERS} rewrite/plans/step-3/ORDER.md lists the files the step 3 units own; never change one of those. A failing check or test in a file that is not yours is not yours to fix: say that you saw it.

Fixed constraints: macOS and Linux only. Shell is POSIX sh and must run under sh, bash and dash. Python is standard library only at run time, floor 3.11, started through uv. Nothing is compiled. Product code goes under next/. Do not edit anything under plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Do not commit, branch, stash or push, and never use git to discard or restore files (no checkout, restore, reset, clean or stash): almost all of the work in this tree is uncommitted.`

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
    checks_rerun: { type: 'string' },
    checked: { type: 'number' },
    confirmed: { type: 'number' },
    still_open: { type: 'array', items: FINDING },
  },
  required: ['checks_rerun', 'checked', 'confirmed', 'still_open'],
}

const rounds = []
let open = A.findings
for (let round = 1; round <= 2 && open.length; round++) {
  const suffix = round === 1 ? '' : `-${round}`
  const fixed = await agent(
    `You are fixing problems in code already written for vsdd-factory 1.0. ${WHERE}

THE WORK: ${ABOUT}

${round === 1 ? 'These findings were given to a fixer before, and an independent check found each still open, or found that a fix broke something.' : 'These findings were given to you or another fixer in the last round, and an independent check found them still open.'} Read exactly what the check says is still wrong, and fix every one of them properly, the minor ones included:
${JSON.stringify(open)}

For each: read the finding's history in the ledger; make the fix; add or correct the test that would have caught it, so that the test fails without the fix; re-run that test. Fix the whole of what the finding describes, each numbered part of it: a fix that covers the example and not the rule is why these are still open. Change only the files the findings concern and their tests. If a step 3 unit owns a file you would need to change, do not change it: mark the finding not-fixed and say what that unit must do. If you are sure a finding is wrong, change nothing for it, mark it rejected and say why in a way a reviewer can check. Where the design must follow what you built, make the spec file that owns the fact say it, and run python3 rewrite/tools/spec_lint.py.

Write rewrite/findings/outcomes/${NAME}${suffix}.json with the Write tool: a JSON array with one object per finding above, {"id": "...", "outcome": "fixed" | "rejected" | "not-fixed", "what_was_done": "...", "where": "file and test"}. When all are done, run every check and report what they print. ${CHECKS} Return the structured result with one resolution per finding.`,
    { label: `fix${suffix}`, phase: 'Fix', model: 'sonnet', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (!fixed) {
    log(`The fixer (round ${round}) did not return. ${open.length} findings stay open.`)
    rounds.push({ round, fix: null, confirm: null })
    break
  }
  const confirm = await agent(
    `You are confirming fixes in code already written for vsdd-factory 1.0. ${WHERE}

THE WORK: ${ABOUT}

An earlier check found these problems still open:
${JSON.stringify(open)}

A fixer says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

You did not make these fixes and should not trust the account. Run the checks yourself and report what they print. ${CHECKS} Then, for every finding and every numbered part of it: open the code and confirm the problem is gone; confirm there is a test that fails without the fix (copy next/ to a scratch folder outside the repository, leaving out .venv, and break the code there if you need to be sure; never break it in place); and confirm the fix did not break anything next to it. For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed.

List under still_open every finding that is not properly fixed, keeping its id, with what is still wrong and what to do. If a fix introduced a new problem, add it with a new id (${NAME}-C${round}-1, ...). Write rewrite/findings/outcomes/${NAME}-confirm${suffix}.json with the Write tool: a JSON array with one object per finding in still_open that kept its id, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are fixed. If you added new problems, also write them to rewrite/findings/sources/extra-${NAME}-confirm${suffix}.json as a JSON array of {"id": "...", "origin": "${NAME}, confirmation", "severity": "blocker" | "major" | "minor", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true}. Those are the only files you may create or change. Return the structured result.`,
    { label: `confirm${suffix}`, phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
  )
  rounds.push({
    round,
    fix: { summary: fixed.summary, tests_passed: fixed.tests_passed, tests_failed: fixed.tests_failed, checks: fixed.checks, foreign_failures: fixed.foreign_failures, not_fixed: fixed.resolutions.filter((r) => r.outcome !== 'fixed') },
    confirm: confirm ? { checks_rerun: confirm.checks_rerun, checked: confirm.checked, confirmed: confirm.confirmed, still_open: confirm.still_open.length } : null,
  })
  if (!confirm) {
    log(`The confirmation (round ${round}) did not return. The fixes are unconfirmed.`)
    break
  }
  log(`${NAME}, round ${round}: ${confirm.confirmed} of ${confirm.checked} fixes confirmed, ${confirm.still_open.length} still open.`)
  open = confirm.still_open
}

return { name: NAME, given: A.findings.length, rounds, still_open: open }
