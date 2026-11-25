export const meta = {
  name: 'vsdd-1-0-slice-layer-2a-confirm',
  description: 'Thin slice, step 2: confirm that the 52 review findings on the catch-up, C2 and B6 units are fixed, fix what is still open, and confirm again',
  whenToUse: 'Once, after slice-layer-2a reported. Its fixers said every review finding was fixed, and nobody had checked. Reads the findings from rewrite/findings/sources/extra-layer-2a-review.json.',
  phases: [
    { title: 'Confirm', detail: 'two reviewers check each fix against the code and its tests', model: 'opus' },
    { title: 'Fix', detail: 'whatever is still open is fixed and re-tested', model: 'sonnet' },
    { title: 'Reconfirm', detail: 'the second fixes are checked', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const CHECKS = `From next/: uv run --no-sync python tools/check.py ; uv run --no-sync python -m pytest -q ; node --test tests/workflows/universal.test.mjs ; uv run --no-sync python tools/build.py --check. From the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/findings.py build. If uv cannot reach the package index, add --system-certs or use what is already installed.`

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the layout and the link form [[kind:id]]. Each build package has a brief, rewrite/spec/packages/<ID>.md. The spec was converted from six larger documents kept in rewrite/history/design-v1/, which settle any doubt about what a spec file should say.
- Working code is under next/; the plugin is next/plugins/vsdd-factory/.
- Known problems are in a ledger under rewrite/findings/: raw findings in sources/, outcomes in outcomes/, and ledger.json built from them by python3 rewrite/tools/findings.py build.
- Step 2 of the thin slice built three units: "catch-up" (packages B2, B3, B4a, B5 and C1 brought into line with the amended design), package C2 (the policy tables) and package B6 (the roster and the lint). An independent reviewer read each and found 52 problems, which are in rewrite/findings/sources/extra-layer-2a-review.json (ids CATCHUP-R<n>, C2-R<n>, B6-R<n>). Two fixers then reported every one of them fixed. Their accounts are in rewrite/findings/sources/slice-layer-2a-result.json, under result.catchup.fix and result.fix_c2_b6; the reviewers' full reports are in the same file under result.<unit>.review.

Other people are working in this tree. Builders may be adding new files under next/ for step 3 (the state core, the record stores, the survey, the hook runner, jobs); rewrite/plans/step-3/ORDER.md, once it exists, lists the files they own. A failure in one of their files is not a finding against these units, but say that you saw it. Another workflow is adding feature flag support to rewrite/spec/ (decision D31); its files are not yours either.

Do not edit anything under plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Do not commit, branch, stash or push, and never use git to discard or restore files (no checkout, restore, reset, clean or stash): almost all of the work in this tree is uncommitted.`

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

const CONFIRM_SCHEMA = {
  type: 'object',
  properties: {
    checks_rerun: { type: 'string' },
    checked: { type: 'number' },
    confirmed: { type: 'number' },
    left_for_builder: { type: 'number' },
    still_open: { type: 'array', items: FINDING },
  },
  required: ['checks_rerun', 'checked', 'confirmed', 'left_for_builder', 'still_open'],
}

const GROUPS = [
  { key: 'catchup', ids: 'CATCHUP-R1 to CATCHUP-R22 (22 findings)', account: 'result.catchup.fix', units: 'the catch-up unit (packages B2, B3, B4a, B5 and C1)' },
  { key: 'c2-b6', ids: 'every C2-R<n> and B6-R<n> (30 findings)', account: 'result.fix_c2_b6', units: 'package C2 (the policy tables) and package B6 (the roster and the lint)' },
]

phase('Confirm')
const confirms = await parallel(
  GROUPS.map((g) => () =>
    agent(
      `You are confirming fixes in code already written for vsdd-factory 1.0. ${WHERE}

YOUR FINDINGS: ${g.ids}, on ${g.units}. Read them in rewrite/findings/sources/extra-layer-2a-review.json, and the fixer's account under ${g.account} in rewrite/findings/sources/slice-layer-2a-result.json.

You did not make these fixes and should not trust the account. Run the checks yourself and report what they print. ${CHECKS}

Then, for every finding: open the code, the tables and the spec files it names and confirm the problem is gone; confirm there is a test that fails without the fix (copy next/ to a scratch folder outside the repository, leaving out .venv, and break the code there if you need to be sure; never break it in place); and confirm the fix did not break something next to it. Where the reviewer's report lists a mutation that survived every suite, repeat that mutation in the scratch copy and see whether a test now catches it. A finding that the fixer deliberately left to a later package is acceptable only if it truly cannot be finished now; name the package.

Write rewrite/findings/outcomes/layer-2a-confirm-${g.key}.json with the Write tool: a JSON array with one object for every one of your findings, {"id": "...", "outcome": "fixed" | "not-fixed" | "left-for-builder", "what_was_done": "for fixed: the fix and the test that pins it; for not-fixed: what is still wrong; for left-for-builder: the package that must finish it and why it cannot be finished now", "where": "file and test"}. If you find a new problem that the fixes introduced, write it to rewrite/findings/sources/extra-layer-2a-confirm-${g.key}.json as a JSON array of {"id": "L2C-${g.key}-<n>", "origin": "slice layer 2, confirmation", "severity": "blocker" | "major" | "minor", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true}, and list it under still_open too. Those are the only files you may create or change.

Return the structured result. still_open holds every finding that is not properly fixed, keeping its id, with what is still wrong and what to do, plus any new problem.`,
      { label: `confirm:${g.key}`, phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
    ),
  ),
)

const lost = GROUPS.filter((_, i) => !confirms[i]).map((g) => g.key)
if (lost.length) log(`Confirmations that did not return: ${lost.join(', ')}. Their findings stay open in the ledger.`)
const open = confirms.filter(Boolean).flatMap((c) => c.still_open)
log(`Confirmation: ${confirms.filter(Boolean).reduce((n, c) => n + c.confirmed, 0)} of ${confirms.filter(Boolean).reduce((n, c) => n + c.checked, 0)} fixes confirmed, ${open.length} still open (${open.filter((f) => f.severity !== 'minor').length} blocker or major).`)

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

let fixed = null
let reconfirm = null
if (open.length) {
  phase('Fix')
  fixed = await agent(
    `You are fixing problems in code already written for vsdd-factory 1.0. ${WHERE}

Fixed constraints: macOS and Linux only. Shell is POSIX sh and must run under sh, bash and dash. Python is standard library only at run time, floor 3.11, started through uv. Nothing is compiled. Product code goes under next/.

These findings were given to a fixer once already, and an independent check found them still open, or found that a fix broke something. Read what the check says and fix every one of them properly, the minor ones included:
${JSON.stringify(open)}

For each: make the fix; add or correct the test that would have caught it, so that the test fails without the fix; re-run that test. Change only the files the findings concern and their tests. If a step 3 builder owns a file you would need to change (rewrite/plans/step-3/ORDER.md), do not change it: mark the finding not-fixed and say what that unit must do. If you are sure a finding is wrong, change nothing for it, mark it rejected and say why in a way a reviewer can check. Where the design must follow what you built, make the spec file that owns the fact say it, and run python3 rewrite/tools/spec_lint.py.

Write rewrite/findings/outcomes/layer-2a-fix-2.json with the Write tool: a JSON array with one object per finding above, {"id": "...", "outcome": "fixed" | "rejected" | "not-fixed", "what_was_done": "...", "where": "file and test"}. When all are done, run every check and report what they print. ${CHECKS} If a failure is in a file a step 3 builder owns, do not fix it: list it under foreign_failures. Return the structured result with one resolution per finding.`,
    { label: 'fix-2', phase: 'Fix', model: 'sonnet', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (fixed) {
    phase('Reconfirm')
    reconfirm = await agent(
      `You are confirming fixes in code already written for vsdd-factory 1.0. ${WHERE}

A check found these problems still open after a first round of fixes:
${JSON.stringify(open)}

A second fixer says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

You did not make these fixes and should not trust the account. Run the checks yourself and report what they print. ${CHECKS} Then, for every finding: open the code and confirm the problem is gone; confirm there is a test that fails without the fix (break the code in a scratch copy outside the repository if you need to be sure, never in place); and confirm the fix did not break anything next to it. For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed.

Write rewrite/findings/outcomes/layer-2a-confirm-2.json with the Write tool: a JSON array with one object per finding that is still not properly fixed, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are fixed. That is the only file you may create or change. Return the structured result; left_for_builder is 0 unless a finding truly cannot be finished before a later package exists.`,
      { label: 'reconfirm', phase: 'Reconfirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
    )
  } else {
    log('The second fixer did not return. The findings stay open in the ledger.')
  }
}

return {
  confirm: GROUPS.map((g, i) => (confirms[i] ? { group: g.key, checks_rerun: confirms[i].checks_rerun, checked: confirms[i].checked, confirmed: confirms[i].confirmed, left_for_builder: confirms[i].left_for_builder, still_open: confirms[i].still_open.length } : { group: g.key, note: 'did not return' })),
  open_after_first_confirmation: open.map((f) => ({ id: f.id, severity: f.severity, file: f.file })),
  fix: fixed ? { summary: fixed.summary, tests_passed: fixed.tests_passed, tests_failed: fixed.tests_failed, checks: fixed.checks, foreign_failures: fixed.foreign_failures, not_fixed: fixed.resolutions.filter((r) => r.outcome !== 'fixed') } : null,
  reconfirm: reconfirm ? { checks_rerun: reconfirm.checks_rerun, checked: reconfirm.checked, confirmed: reconfirm.confirmed, still_open: reconfirm.still_open } : null,
}
