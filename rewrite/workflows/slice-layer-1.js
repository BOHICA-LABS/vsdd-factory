export const meta = {
  name: 'vsdd-1-0-slice-layer-1',
  description: 'Thin slice, layer 1: build the undisputed foundations of vsdd-factory 1.0 under next/ and review them',
  whenToUse: 'Step 3a of the vsdd-factory 1.0 rewrite. Builds under next/ only (plus the root CLAUDE.md). Does not edit the design documents.',
  phases: [
    { title: 'Build', detail: 'four units: skeleton and launcher, core and schemas, prelude and harness, sample project', model: 'sonnet' },
    { title: 'Review', detail: 'one reviewer per unit, tests re-run', model: 'opus' },
    { title: 'Fix', detail: 'blocker and major findings fixed and re-tested', model: 'sonnet' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const COMMON = `You are building part of vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

The design is rewrite/BLUEPRINT.md plus five detail documents in rewrite/design/ (pipeline.md, state.md, agents-skills.md, hooks.md, packaging.md). Section 4 of the blueprint is the authority on every shared name and path; where a detail document differs, the blueprint wins. Section 9 is the build plan: find your packages there by their ids. Section 10 lists platform behaviour that was proven by experiment. rewrite/STATE.md records the user's decisions. A small working plugin that proved the platform shapes (hooks.json, the uv guard script, a Python hook runner with inline script metadata, workflow scripts) is in rewrite/spike/plugin/; use it as a reference for what is known to work.

Fixed constraints: macOS and Linux only. Shell scripts are POSIX sh and must run under sh, bash and dash. Python is standard library only, with a floor of 3.11, and is started through uv. Nothing is compiled. Hook timeouts are in seconds.

All product code goes under next/, the staging root that mirrors the final repository; the plugin's root is the path the blueprint and packaging.md give under next/. Create nothing outside next/ unless your unit says so. Do not edit anything under rewrite/, plugins/ or crates/. Do not commit, branch or push. The repository CLAUDE.md that was loaded into your context describes the old system being replaced; its process rules do not apply to this work.

A review of the design documents is running at the same time, so the documents will not change under you, and your code is not being reviewed by it.

How to work:
- Read the package rows and the sections of the design they point to before writing anything.
- Build what the design specifies for your packages, completely and correctly, with tests. Do not build ahead into other packages. Where a package needs something that belongs to a later package, write the smallest stand-in and list it under deferred.
- Run what you build. Every claim in your result that something works must come from a command you ran; give the command and what it printed.
- Tests: check first whether pytest is available (python3 -m pytest --version, or uv run --with pytest). The sandbox may not reach the package index; if pytest cannot be had, write the tests with unittest and say so.
- When the design is wrong, missing, ambiguous or contradicts itself for something you have to build, do not edit the design. Choose the reading that best fits section 4 of the blueprint and the design goals (correct and enforceable; low touch; restartable; lean), build that, and report it under design_findings with the exact location and what you did. These reports are how the design gets corrected, so be specific.`

const UNITS = {
  u1: {
    name: 'Unit 1: skeleton, manifest, launcher and entries (packages B0, B1, B2)',
    brief: `Build packages B0, B1 and B2.
- B0: the next/ skeleton that mirrors the final repository layout; a tools/check.py that runs the checks that exist so far and passes; and the short rewrite-mode CLAUDE.md the design specifies. For the CLAUDE.md, replace the file at the repository root (${ROOT}/CLAUDE.md) with the short version; this is the one file outside next/ that you may write. The old file stays in git history on the develop branch. Keep it to what a person or agent working on this rewrite needs: where the design and the resume record are, where code goes, how to run the checks, and the user's git rules (no AI attribution in commits, no --no-verify, no force-push to main, commit or push only when asked).
- B1: plugin.json with every field the design gives, the plugin README, the licence, a placeholder icon, the .mcp.json, the empty component folders, and lint for the manifest and the MCP file.
- B2: the launcher and its guard, the root finder, a stub hook entry, the command-line entry with --version, the bin/vsdd wrapper, and shell lint.
Prove with commands: claude plugin validate on the plugin root; the launcher under sh, bash and dash if dash is installed; bin/vsdd --version; a run with uv removed from PATH, which must exit 2 with a message that tells the person how to fix it; and the inert path in a directory that is not a factory project, with its time.`,
  },
  u2: {
    name: 'Unit 2: shared core and schemas (packages B3, C1)',
    brief: `Build packages B3 and C1 on top of unit 1, which is already in next/.
- B3: the shared core modules the design names (project detection, configuration reading, the frontmatter reader, the event log writer, schema validation) with unit tests. The frontmatter reader and the schema validator are standard library only, so state plainly in your result which subset of YAML and of JSON Schema each supports, and test the edges of that subset.
- C1: the result schemas of section 4.10 of the blueprint as JSON files in the plugin, with valid and invalid fixtures for each, and a test that runs every fixture through the validator.
Prove with commands: the whole test suite, and tools/check.py.`,
  },
  u3: {
    name: 'Unit 3: workflow prelude, build step and harness (packages B4a, B5)',
    brief: `Build packages B4a and B5 on top of units 1 and 2, which are already in next/.
- B4a: the shared workflow prelude (workflow scripts cannot import, so the prelude is stamped into each script), including the spike wrapper, the story runner, the argument parser and the result checks the design gives; the shared agent text and severity table; and tools/build.py, which stamps the prelude, the schema literals and the tier and role tables into each workflow script, with a --check mode that fails when a stamped copy has drifted.
- B5: the Node harness that runs a workflow script against stubbed agents under the platform's rules (no imports, no file or shell access, no Date.now or Math.random, one level of nesting, the agent counter shared by parent and child), the universal tests every workflow must pass, and two toy workflows that exercise the prelude.
The platform rules for workflow scripts are in rewrite/STATE.md and in rewrite/spike/plugin/workflows/. Prove with commands: the harness tests under node, build.py and build.py --check, a deliberate drift that --check catches, and tools/check.py.`,
  },
  u4: {
    name: 'Unit 4: the sample project',
    brief: `Build the sample project that the smoke tier and the story tier will use (packages E1 and E2 in section 9 name it; packaging.md describes it). It is a very small product in its own directory under next/ with a test command that writes a JUnit XML report, a lint or format check, and room for one new story: a behaviour that is specified but not yet implemented, with no test for it. It must need nothing from the network, so use only what is on a stock macOS or Linux developer machine with uv. Include the project configuration file the design gives for a factory project, filled in for this sample, and a script that creates a fresh git repository from the sample in a temporary directory, because tests will need a clean copy each time.
Do not build any other part of E1 or E2. Prove with commands: the test command passes and writes the report, the lint check passes, and the fresh-copy script produces a repository with one commit.`,
  },
}

const BUILD_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    summary: { type: 'string' },
    files: { type: 'array', items: { type: 'string' } },
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
    deferred: { type: 'array', items: { type: 'string' } },
    design_findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          where: { type: 'string' },
          problem: { type: 'string' },
          what_i_did: { type: 'string' },
        },
        required: ['severity', 'where', 'problem', 'what_i_did'],
      },
    },
  },
  required: ['unit', 'summary', 'files', 'commands_run', 'tests_passed', 'tests_failed', 'deferred', 'design_findings'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    verdict: { type: 'string' },
    tests_rerun: { type: 'string' },
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
    design_findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          where: { type: 'string' },
          problem: { type: 'string' },
        },
        required: ['severity', 'where', 'problem'],
      },
    },
  },
  required: ['unit', 'verdict', 'tests_rerun', 'findings', 'design_findings'],
}

const build = (key) =>
  agent(
    `${COMMON}\n\nYOUR UNIT: ${UNITS[key].name}\n\n${UNITS[key].brief}\n\nWhen you are done, return the structured result. List every file you created or changed, the commands you ran with what they printed (shortened), the test counts, what you deferred to a later package, and your design findings. Keep the summary under 150 words.`,
    { label: `build:${key}`, phase: 'Build', model: 'sonnet', effort: 'xhigh', schema: BUILD_SCHEMA },
  )

const review = (key, built) =>
  agent(
    `You are reviewing newly written code for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The design is rewrite/BLUEPRINT.md (section 4 is the authority on names and paths, section 9 is the build plan, section 10 lists proven platform behaviour) plus the detail documents in rewrite/design/. rewrite/STATE.md records the user's decisions. The repository CLAUDE.md that was loaded into your context may describe the old system; its process rules do not apply.

THE UNIT UNDER REVIEW: ${UNITS[key].name}

What the builder was asked to do:
${UNITS[key].brief}

What the builder says it did:
${JSON.stringify({ summary: built.summary, files: built.files, tests_passed: built.tests_passed, tests_failed: built.tests_failed, deferred: built.deferred, design_findings: built.design_findings })}

Review the code under next/ for this unit. You did not write it and you should not trust the builder's account: run the tests and the proof commands yourself and report what they print.

Look for: behaviour that differs from the design or from section 4 of the blueprint; a claim in the builder's result that the code does not support; a test that cannot fail, or a behaviour with no test; a failure that would be silent (a gate or check that switches off without saying so is the worst defect this project can have); shell that is not portable across sh, bash and dash, or across macOS and Linux; anything beyond the Python standard library; a missing or unhelpful message when something a person must fix is wrong; and code built ahead of its package. Do not report style.

Severity: "blocker" means it does not work or would break later packages; "major" means wrong behaviour, a silent failure, or a missing test for something that matters; "minor" is anything else worth fixing. Give each finding an id (${key.toUpperCase()}-R1, ${key.toUpperCase()}-R2, ...), the file, the problem and a proposed fix. Separately, under design_findings, report anything in the design documents themselves that you found wrong, missing or contradictory while checking the code against them; do not edit the design.

Do not modify any file. Do not commit, branch or push. Return the structured result.`,
    { label: `review:${key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
  )

const fix = (keys, findings) =>
  agent(
    `${COMMON}\n\nYOUR TASK: fix review findings in code that is already built under next/ for these units: ${keys.map((k) => UNITS[k].name).join('; ')}.

The findings to fix, every one of them:
${JSON.stringify(findings)}

For each finding, make the fix, add or correct the test that would have caught it, and re-run the tests. If you believe a finding is wrong, do not change the code for it; say why in your result. When all are done, run the whole test suite and tools/check.py and report what they print. Return the structured result; put one line per finding in commands_run or summary saying what was done.`,
    { label: `fix:${keys.join('+')}`, phase: 'Fix', model: 'sonnet', effort: 'xhigh', schema: BUILD_SCHEMA },
  )

const serious = (r) => (r ? r.findings.filter((f) => f.severity !== 'minor') : [])

// The sample project depends on nothing else, so it builds alongside the chain.
const u4Promise = build('u4')

const u1 = await build('u1')
let r1 = null
let u1fix = null
if (u1) {
  r1 = await review('u1', u1)
  if (r1 && r1.findings.length) {
    log(`Unit 1 review: ${r1.findings.length} findings (${serious(r1).length} blocker or major)`)
    if (serious(r1).length) u1fix = await fix(['u1'], r1.findings)
  }
}

let u2 = null
let u3 = null
let r2 = null
let r3 = null
let fix23 = null
if (u1) {
  u2 = await build('u2')
  if (u2) u3 = await build('u3')
  const reviews = await parallel([
    () => (u2 ? review('u2', u2) : Promise.resolve(null)),
    () => (u3 ? review('u3', u3) : Promise.resolve(null)),
  ])
  r2 = reviews[0]
  r3 = reviews[1]
  const toFix = [...(r2 ? r2.findings : []), ...(r3 ? r3.findings : [])]
  log(`Units 2 and 3 review: ${toFix.length} findings (${serious(r2).length + serious(r3).length} blocker or major)`)
  if (serious(r2).length + serious(r3).length) fix23 = await fix(['u2', 'u3'], toFix)
}

const u4 = await u4Promise

const designFindings = []
const add = (source, list) => (list || []).forEach((f) => designFindings.push({ source, ...f }))
add('build:u1', u1 && u1.design_findings)
add('review:u1', r1 && r1.design_findings)
add('fix:u1', u1fix && u1fix.design_findings)
add('build:u2', u2 && u2.design_findings)
add('review:u2', r2 && r2.design_findings)
add('build:u3', u3 && u3.design_findings)
add('review:u3', r3 && r3.design_findings)
add('fix:u2+u3', fix23 && fix23.design_findings)
add('build:u4', u4 && u4.design_findings)

const brief = (b) => (b ? { summary: b.summary, files: b.files.length, tests_passed: b.tests_passed, tests_failed: b.tests_failed, deferred: b.deferred } : null)
const rbrief = (r) => (r ? { verdict: r.verdict, tests_rerun: r.tests_rerun, findings: r.findings } : null)

return {
  u1: { build: brief(u1), review: rbrief(r1), fix: brief(u1fix) },
  u2: { build: brief(u2), review: rbrief(r2) },
  u3: { build: brief(u3), review: rbrief(r3) },
  fix_u2_u3: brief(fix23),
  u4: { build: brief(u4) },
  unfixed_minor_findings: [
    ...(r1 && !u1fix ? r1.findings : []),
    ...(!fix23 ? [...(r2 ? r2.findings : []), ...(r3 ? r3.findings : [])] : []),
  ],
  design_findings: designFindings,
}
