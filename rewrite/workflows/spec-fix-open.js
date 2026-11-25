export const meta = {
  name: 'vsdd-1-0-spec-fix-open',
  description: 'Design: correct a list of findings that an earlier round left open in rewrite/spec/ (Fable), confirm each correction (Opus), and repeat once if anything is still open',
  whenToUse: 'When a design workflow reports findings still open. Pass args {name, about, findings: [{id, severity, file, problem, proposed_fix}], may_write?, guidance?}. name is the unit name for the record files.',
  phases: [
    { title: 'Correct', detail: 'every finding decided and the design corrected', model: 'fable' },
    { title: 'Confirm', detail: 'each correction confirmed against the files', model: 'opus' },
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
const MAY_WRITE = A.may_write || 'Only files under rewrite/spec/ that the findings concern.'
const GUIDANCE = A.guidance || ''
const FORBID = A.forbid || 'next/, plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/'

const CHECKS = `From the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/sections.py coverage ; python3 rewrite/tools/gen_packages.py ; python3 rewrite/tools/findings.py build.`

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin that builds software from a specification. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the rules: one fact in one place, every thing in the registry, links written [[kind:id]], no file over 16 KB and no line over 400 characters, machine-readable where a program reads it, no history. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it; each package has a generated brief, rewrite/spec/packages/<ID>.md. rewrite/spec/decisions.md records the decisions the user adopted.
- Known problems are in a ledger under rewrite/findings/: raw findings in sources/, the packages that must act in triage/, outcomes in outcomes/ (applied in the order listed in outcomes/ORDER, so a later file wins), and ledger.json and by-package/ built from them by python3 rewrite/tools/findings.py build. Look a finding up by its id to read its history.
- Working code is under next/. Builders are building the thin slice there now (rewrite/spec/packaging/build-plan.md, section "The thin slice"), and they read rewrite/spec/ as they work.

Do not edit anything under ${FORBID}, or a package brief by hand. Do not commit, branch, stash or push, and never use git to discard or restore files: almost all of the work in this tree is uncommitted and other people are working in it. Nothing on the thin slice path may change: in a file a slice builder reads, change only the passage the findings concern. Keep the lint clean as you go.`

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
          outcome: { type: 'string', enum: ['fixed', 'rejected', 'left-for-builder', 'needs-the-user'] },
          what_was_done: { type: 'string' },
        },
        required: ['id', 'outcome', 'what_was_done'],
      },
    },
    decisions_made: { type: 'array', items: { type: 'string' } },
    files_changed: { type: 'array', items: { type: 'string' } },
    checks: { type: 'string' },
  },
  required: ['summary', 'resolutions', 'decisions_made', 'files_changed', 'checks'],
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
    `You are correcting the design of vsdd-factory 1.0. ${WHERE}

THE WORK: ${ABOUT}

An independent check found these still open after an earlier round of corrections. Decide every one, the minor ones included, and correct the design:
${JSON.stringify(open)}

${GUIDANCE}

Design goals, in this order: correct and enforceable; low touch; restartable; lean. Every requirement must end as something a program checks, or as a named duty of a named agent with a named reviewer who checks it. A check that cannot measure says so; it never passes by default. Correct the whole of what a finding describes, not only its example, and after each correction search for the same fact stated elsewhere so that it stays in one place. Where a finding asks for more than the user decided, take what makes the decision correct and enforceable and reject the rest, saying why. Outcomes: "fixed" when the design now says the right thing; "rejected" with a reason a reviewer can check; "left-for-builder" only for something that can be settled only in code when a package is built, naming the package; "needs-the-user" only for a choice that is truly the user's, stated as one question. List under decisions_made each design choice you made that a person reading the design later would want to know, one line each.

You may write: ${MAY_WRITE}

Write rewrite/findings/outcomes/${NAME}${suffix}.json with the Write tool: a JSON array with one object per finding above, {"id": "...", "outcome": "fixed" | "rejected" | "left-for-builder" | "not-fixed", "what_was_done": "...", "where": "the file"} (record "needs-the-user" as "not-fixed" and put the question in what_was_done). For each left-for-builder, add an entry to rewrite/findings/triage/${NAME}.json: {"id": "...", "files": ["spec paths relative to rewrite/spec/"], "packages": ["<ID>"], "on_slice_path": false}. Then run the checks and report what they print. ${CHECKS} Return the structured result with one resolution per finding.`,
    { label: `correct${suffix}`, phase: 'Correct', model: 'fable', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (!fixed) {
    log(`The corrector (round ${round}) did not return. ${open.length} findings stay open.`)
    rounds.push({ round, fix: null, confirm: null })
    break
  }
  const confirm = await agent(
    `You are confirming corrections to the design of vsdd-factory 1.0. ${WHERE}

THE WORK: ${ABOUT}

An earlier check found these problems still open:
${JSON.stringify(open)}

A corrector says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

and made these design choices: ${JSON.stringify(fixed.decisions_made)}

You did not make these corrections and should not trust the account. Run the checks yourself and report what they print. ${CHECKS} Then, for every finding and every part of it: read the files and confirm the problem is gone, and that the correction did not state a fact twice, contradict another file, go beyond what the user decided, or change anything on the thin slice path. Judge each design choice: would it work as specified, and is it enforceable? For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed. For one marked left-for-builder, confirm it can only be settled in code and that it appears in the by-package list of the package named (rewrite/findings/by-package/).

List under still_open every finding that is not properly dealt with, keeping its id, with what is still wrong and what to do. If a correction introduced a new problem, add it with a new id (${NAME}-C${round}-1, ...). Write rewrite/findings/outcomes/${NAME}-confirm${suffix}.json with the Write tool: a JSON array with one object per finding in still_open that kept its id, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are dealt with. If you added new problems, also write them to rewrite/findings/sources/extra-${NAME}-confirm${suffix}.json as a JSON array of {"id": "...", "origin": "${NAME}, confirmation", "severity": "blocker" | "major" | "minor", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": false}. Those are the only files you may create or change. Return the structured result.`,
    { label: `confirm${suffix}`, phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
  )
  rounds.push({
    round,
    fix: { summary: fixed.summary, decisions_made: fixed.decisions_made, checks: fixed.checks, not_fixed: fixed.resolutions.filter((r) => r.outcome !== 'fixed') },
    confirm: confirm ? { checks_rerun: confirm.checks_rerun, checked: confirm.checked, confirmed: confirm.confirmed, still_open: confirm.still_open.length } : null,
  })
  if (!confirm) {
    log(`The confirmation (round ${round}) did not return. The corrections are unconfirmed.`)
    break
  }
  log(`${NAME}, round ${round}: ${confirm.confirmed} of ${confirm.checked} corrections confirmed, ${confirm.still_open.length} still open.`)
  open = confirm.still_open
}

return { name: NAME, given: A.findings.length, rounds, still_open: open }
