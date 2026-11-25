export const meta = {
  name: 'vsdd-1-0-slice-layer-2a',
  description: 'Thin slice, layer 2, step 2: bring the foundations into line with the amended design, then build the policy tables (C2) and the roster and lint (B6)',
  whenToUse: 'After the slice amend. Builds under next/, closes ledger findings in code, and records outcomes under rewrite/findings/.',
  phases: [
    { title: 'Build', detail: 'three units in order: catch-up, C2, B6', model: 'sonnet' },
    { title: 'Review', detail: 'one reviewer per unit, tests re-run, outcomes checked', model: 'opus' },
    { title: 'Fix', detail: 'blocker and major findings fixed and re-tested', model: 'sonnet' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const COMMON = `You are building part of vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The repository CLAUDE.md in your context is the rewrite-mode one; follow it.

Where things are:
- The design is rewrite/spec/, one small file per thing. Read rewrite/spec/README.md first for the layout and the link form [[kind:id]]. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it.
- Each build package has a brief, rewrite/spec/packages/<ID>.md, listing the things it implements. Read your brief, every file it lists, and the files those link to. rewrite/spec/conventions.md gives names, paths and tiers; rewrite/spec/platform.md gives the platform behaviour that was proven by experiment; rewrite/spec/packaging/build-plan.md has a section "The thin slice" that says how deep to build each package now.
- The spec was converted from six larger documents kept in rewrite/history/design-v1/. Each spec file's frontmatter lists the sections it came from. If a spec file is silent or seems to contradict itself, read the original section before deciding.
- Working code is under next/. Run its checks before you change anything: cd next && uv run --no-sync python tools/check.py ; uv run --no-sync python -m pytest -q ; node --test tests/workflows/universal.test.mjs. They pass today. If uv cannot reach the package index, add --system-certs or use what is already installed.
- Known problems with the design are in a ledger. rewrite/findings/by-package/<ID>.md lists the ones that concern the files your package implements, each with a status: open, not-fixed (a fix was tried and found incomplete) or left-for-builder (the design was settled and the code must follow).

Fixed constraints: macOS and Linux only. Shell is POSIX sh and must run under sh, bash and dash. Python is standard library only at run time, floor 3.11, started through uv. Nothing is compiled. All product code goes under next/. Do not edit anything under plugins/, crates/, rewrite/history/ or rewrite/tools/. Do not commit, branch or push.

How to work:
- Build to slice depth: what the story path needs, completely and correctly, with tests. Do not build ahead. List what you left for later under deferred.
- Close findings where they can be checked. For each finding in your by-package list, decide: fix it in code with a test that would fail without the fix; or, when it is purely a fault in the text of a spec file your package owns, correct that file (one fact in one place; run python3 rewrite/tools/spec_lint.py afterwards); or say it does not arise at slice depth; or say it is wrong and why. Take the proposed resolution when it is sound.
- The design follows what runs. When the design is wrong, missing or ambiguous for something you must build, choose the reading that best fits the design goals (correct and enforceable; low touch; restartable; lean), build it, and make the spec file that owns the fact say what you built. Report it as a new finding too, so it can be checked.
- Every claim that something works must come from a command you ran.
- Record your work in two files, both valid JSON arrays, written with the Write tool:
  - rewrite/findings/outcomes/<unit>.json: one object per ledger finding you dealt with: {"id": "...", "outcome": "fixed" | "not-fixed" | "left-for-builder" | "rejected", "what_was_done": "...", "where": "file and test"}. Use "left-for-builder" for a finding that does not arise at slice depth and say which later package must take it.
  - rewrite/findings/sources/extra-<unit>.json: one object per new problem you found in the design or in code outside your unit: {"id": "L2-<unit>-<n>", "origin": "slice layer 2, <unit>", "severity": "blocker" | "major" | "minor", "where": "the spec file or code file", "problem": "...", "proposed_resolution": "...", "on_slice_path": true or false}.
  Then run python3 rewrite/tools/findings.py build and confirm it still works.`

const UNITS = {
  catchup: {
    name: 'Catch-up: bring the foundations into line with the amended design (packages B2, B3, B4a, B5, C1)',
    packages: 'B2, B3, B4a, B5 and C1',
    brief: `The foundations under next/ were built before the design was amended. Bring them into line.
- The three findings marked left-for-builder: the vsdd entry in launch.sh must pass the tool's exit 1 through silently (SL1-03); events.py and its schema must admit the lines the launcher writes, a null session_id and whole-second timestamps (SL1-05); every S_* literal in the prelude must be generated from a schema file, and C1 must add the survey, checkpoint-result, delivery-data, story-unit, wave-gate-data and implement-data schema files (SL1-06).
- What the amendment changed: a step result's step identity gains method, and loop and pass on review, fix and arbitrate steps, so the step-result schema, its fixtures and the prelude's step() must follow; judge() tests grounded !== false; converge() passes pass to arbitrate; relay() uses the empty-object fallback; the survey, close and delivery shapes require every field. Read rewrite/findings/sources/slice-amend-result.json (the amend summary and resolutions) and the spec files it names: topics/verdicts.md, topics/prelude-steps.md, topics/story-script.md, schemas/, commands/story-deliver.md.
- Every finding in rewrite/findings/by-package/ for B2, B3, B4a, B5 and C1.
Where the built schemas under next/plugins/vsdd-factory/schemas/ and the spec's rewrite/spec/schemas/ differ, make them agree: the built file is what ships, so when the built one is right, replace the body of the spec file with one sentence pointing at it and keep its frontmatter. rewrite/spec/conflicts/tables-repair.md lists the differences found so far.
Your unit name for the two record files is "catchup".`,
  },
  c2: {
    name: 'Package C2: the policy tables',
    packages: 'C2',
    brief: `Build package C2 to slice depth: the policy tables the plugin ships (the artifact registry with the story kinds, the path classes with their precedence, and the walls with rows, callers and reveals) as files under the plugin, each with policy tests that prove the table says what the design requires. The spec holds these as rewrite/spec/tables/*.toml. They were repaired to match the original documents, but a check found prose copies that still disagree, and the kind name for hidden scenarios was not settled (findings AU-T2-08 and AU-T2-09 among others in your list). Settle them: the shipped table is the single definition. When your tables are built and tested, replace the body of each spec table file you now ship with a short comment pointing at the shipped file, keeping it valid TOML, and make the prose files that restated a table link to it.
For the slice, the rows that must be right and tested are those for the main session, the clerk, the test-writer, the implementer and the adversary, the story artifact kind, and the classes a story touches. Carry the other rows as the design gives them and say which you did not test.
Your unit name for the two record files is "c2".`,
  },
  b6: {
    name: 'Package B6: the roster and the lint',
    packages: 'B6',
    brief: `Build package B6 to slice depth: the roster (tests/lint/roster.toml under next/, which the build step already reads for the tier, role and override tables) and the lint suite. For the slice, the lint must cover the files the slice ships: agent files (the clerk, test-writer, implementer, verifier, adversary, security-reviewer, architect, demo-recorder, technical-writer and devops will be written next), the story workflow, the manifest, and the roster against what is on disk. The build plan says the roster lists the whole design from the start, and that until all packages exist the lint checks only that nothing on disk is absent from the roster. A ledger finding (AU-P2-01) says the spec's roster once had feature, verify, maintain and multi not calling story; check the shipped roster says they do.
When your roster is built and tested, replace the body of rewrite/spec/tables/roster.toml with a short comment pointing at the shipped file, keeping it valid TOML.
Your unit name for the two record files is "b6".`,
  },
}

const BUILD_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    summary: { type: 'string' },
    files_changed: { type: 'number' },
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
    spec_files_changed: { type: 'array', items: { type: 'string' } },
    deferred: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'summary', 'files_changed', 'commands_run', 'tests_passed', 'tests_failed', 'findings_fixed', 'findings_left', 'new_findings', 'spec_files_changed', 'deferred'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    verdict: { type: 'string' },
    tests_rerun: { type: 'string' },
    outcomes_checked: { type: 'number' },
    outcomes_confirmed: { type: 'number' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          file: { type: 'string' },
          problem: { type: 'string' },
          proposed_fix: { type: 'string' },
        },
        required: ['id', 'severity', 'file', 'problem', 'proposed_fix'],
      },
    },
  },
  required: ['unit', 'verdict', 'tests_rerun', 'outcomes_checked', 'outcomes_confirmed', 'findings'],
}

const build = (key) =>
  agent(`${COMMON}\n\nYOUR UNIT: ${UNITS[key].name}\n\n${UNITS[key].brief}\n\nWhen you are done, run all the checks once more and return the structured result. Keep the summary under 150 words.`, {
    label: `build:${key}`,
    phase: 'Build',
    model: 'sonnet',
    effort: 'xhigh',
    schema: BUILD_SCHEMA,
  })

const review = (key, built) =>
  agent(
    `You are reviewing newly written code for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The design is rewrite/spec/ (start with rewrite/spec/README.md); each build package has a brief in rewrite/spec/packages/; the spec was converted from the documents in rewrite/history/design-v1/, which settle any doubt about what a spec file should say. Code is under next/. Known problems are in rewrite/findings/ledger.json.

THE UNIT UNDER REVIEW: ${UNITS[key].name}

What the builder was asked to do:
${UNITS[key].brief}

What the builder says it did:
${JSON.stringify(built)}

You did not write this and should not trust the account. Do three things.
1. Run the checks yourself and report what they print: cd next && uv run --no-sync python tools/check.py ; uv run --no-sync python -m pytest -q ; node --test tests/workflows/universal.test.mjs ; and from the repository root python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/findings.py build.
2. Read rewrite/findings/outcomes/${key}.json. For every entry marked fixed, open the finding in the ledger and confirm that the fix is there and that a test would fail without it: break the code or the table briefly in a scratch copy if you need to be sure, never in place. For every entry marked left-for-builder or rejected, judge whether that was right. Count how many you checked and how many you confirmed.
3. Review the code and tables this unit changed under next/, and the spec files it changed. Look for: behaviour that differs from the design; a claim the code does not support; a test that cannot fail, or behaviour with no test; a failure that would be silent (a gate or check that switches off without saying so is the worst defect this project can have); shell that is not portable; anything beyond the Python standard library at run time; a spec file that now says something the code does not do; a fact now stated in two places.

Severity: "blocker" means it does not work or would break later packages; "major" means wrong behaviour, a silent failure, a finding marked fixed that is not, or a missing test for something that matters; "minor" is anything else worth fixing. Give each finding an id (${key.toUpperCase()}-R1, ...), the file, the problem and a proposed fix. Do not modify any file. Do not commit, branch or push. Return the structured result.`,
    { label: `review:${key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
  )

const fix = (keys, findings) =>
  agent(
    `${COMMON}\n\nYOUR TASK: fix review findings in work already done for these units: ${keys.map((k) => UNITS[k].name).join('; ')}.

The findings to fix, every one of them:
${JSON.stringify(findings)}

For each finding, make the fix, add or correct the test that would have caught it, and re-run the tests. If a finding says a ledger entry marked fixed is not fixed, fix it properly and correct its entry in the unit's outcomes file. If you believe a finding is wrong, change nothing for it and say why. Write rewrite/findings/outcomes/${keys.join('-')}-fix.json only for ledger findings whose outcome changed. When all are done, run every check and report what they print. Use "${keys.join('-')}-fix" as your unit name. Return the structured result.`,
    { label: `fix:${keys.join('+')}`, phase: 'Fix', model: 'sonnet', effort: 'xhigh', schema: BUILD_SCHEMA },
  )

const serious = (r) => (r ? r.findings.filter((f) => f.severity !== 'minor') : [])

const catchup = await build('catchup')
let rCatchup = null
let fCatchup = null
if (catchup) {
  rCatchup = await review('catchup', catchup)
  if (rCatchup && rCatchup.findings.length) {
    log(`Catch-up review: ${rCatchup.findings.length} findings (${serious(rCatchup).length} blocker or major)`)
    if (serious(rCatchup).length) fCatchup = await fix(['catchup'], rCatchup.findings)
  }
}

let c2 = null
let b6 = null
let rC2 = null
let rB6 = null
let fRest = null
if (catchup) {
  c2 = await build('c2')
  b6 = await build('b6')
  const reviews = await parallel([() => (c2 ? review('c2', c2) : Promise.resolve(null)), () => (b6 ? review('b6', b6) : Promise.resolve(null))])
  rC2 = reviews[0]
  rB6 = reviews[1]
  const toFix = [...(rC2 ? rC2.findings : []), ...(rB6 ? rB6.findings : [])]
  log(`C2 and B6 review: ${toFix.length} findings (${serious(rC2).length + serious(rB6).length} blocker or major)`)
  if (serious(rC2).length + serious(rB6).length) fRest = await fix(['c2', 'b6'], toFix)
}

const brief = (b) => (b ? { summary: b.summary, files_changed: b.files_changed, tests_passed: b.tests_passed, tests_failed: b.tests_failed, findings_fixed: b.findings_fixed, findings_left: b.findings_left, new_findings: b.new_findings, deferred: b.deferred } : null)
const rbrief = (r) => (r ? { verdict: r.verdict, tests_rerun: r.tests_rerun, outcomes_checked: r.outcomes_checked, outcomes_confirmed: r.outcomes_confirmed, findings: r.findings } : null)

return {
  catchup: { build: brief(catchup), review: rbrief(rCatchup), fix: brief(fCatchup) },
  c2: { build: brief(c2), review: rbrief(rC2) },
  b6: { build: brief(b6), review: rbrief(rB6) },
  fix_c2_b6: brief(fRest),
  unfixed_minor: [...(rCatchup && !fCatchup ? rCatchup.findings : []), ...(!fRest ? [...(rC2 ? rC2.findings : []), ...(rB6 ? rB6.findings : [])] : [])],
}
