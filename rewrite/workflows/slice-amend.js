export const meta = {
  name: 'vsdd-1-0-slice-amend',
  description: 'Triage every open finding to its owning spec files and packages, fix the faithless tables, and apply the slice-path blockers and majors to the spec',
  whenToUse: 'After the restructure repair, before slice layer 2. Reads rewrite/findings/ledger.json; writes rewrite/findings/triage/, rewrite/findings/outcomes/ and edits rewrite/spec/.',
  phases: [
    { title: 'Triage', detail: 'four clerks assign each finding to its owning files and packages', model: 'sonnet' },
    { title: 'Tables', detail: 'make the machine tables say what the originals say', model: 'sonnet' },
    { title: 'Amend', detail: 'apply the slice-path blockers and majors to their owning files', model: 'fable' },
    { title: 'Verify', detail: 'check the table fixes and the amendments', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const LAYOUT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

The design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin, lives under rewrite/spec/ as one small file per thing. Read rewrite/spec/README.md first: it gives the kinds and folders, the frontmatter, the link form [[kind:id]], and the checks (python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage). rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing with its build package. rewrite/spec/packages/<ID>.md is the brief for each build package and lists the things it implements. The six documents the spec was converted from are kept unchanged in rewrite/history/design-v1/ (BLUEPRINT.md, pipeline.md, state.md, agents-skills.md, hooks.md, packaging.md); rewrite/spec/source-index.json maps each of their sections to the spec files that cite it. Review reports are in rewrite/history/reviews/. rewrite/STATE.md records the user's decisions and the platform facts proven by experiment. Working code for the foundations is under next/, with its own checks (cd next && uv run --no-sync python tools/check.py).

Every open problem with the design is in rewrite/findings/ledger.json: 238 entries, each with an id, an origin, a severity, where it was found, the problem and usually a proposed resolution. Ids starting P3 come from the third design review, CL- from an earlier close-out, SL1- from the builders and reviewers of the foundations (these say what the code under next/ actually does), and AU- from an audit of the conversion into rewrite/spec/ (these say a spec file does not say what its original says).

Do not edit anything under next/, rewrite/history/, rewrite/tools/, rewrite/spec/packages/ or rewrite/spec/README.md. Do not commit, branch or push.`

const SLICE = `The thin slice is the next thing to be built: the least that delivers one story on a sample project through the real platform. Its path runs through: the uv launcher and the vsdd entry point; the shared Python core and the schemas; the workflow prelude, build step and test harness; the lint and roster; the policy tables; project detection, mount and init; the run, step, evidence and closure records; the survey and checkpoint for a single story, with its worktree; the red and green gates; the review verdict; delivery in local mode with no remote; the hook runner core with the tests-first, merge-guard and step-results rules and the hook wiring; the clerk, test-writer, implementer and adversary agents; and the story workflow. Its build packages are B0 to B6, C1, C2, S1, S2a, S2b, S3, S4, G1, G2, G3, G4a, H1, H5, H7, A1, A4, A6a, W2 and E2.`

// ---------------------------------------------------------------- Triage
const PARTS = [
  { key: 'a', which: 'ids that start P3SEAM-, P3COV- or P3OPS-' },
  { key: 'b', which: 'ids that start P3REQ-, P3BLD- or CL-' },
  { key: 'c', which: 'ids that start SL1-' },
  { key: 'd', which: 'ids that start AU-' },
]
const TRIAGE_SCHEMA = {
  type: 'object',
  properties: {
    part: { type: 'string' },
    path: { type: 'string' },
    triaged: { type: 'number' },
    on_slice_path: { type: 'number' },
    could_not_place: { type: 'array', items: { type: 'string' } },
  },
  required: ['part', 'path', 'triaged', 'on_slice_path', 'could_not_place'],
}

phase('Triage')
const triagePromise = parallel(
  PARTS.map((part) => () =>
    agent(
      `You are sorting a list of design findings so that each one reaches the people who must act on it. ${LAYOUT}

${SLICE}

YOUR PART: the findings in rewrite/findings/ledger.json whose ${part.which}. Read each one in full.

For each finding decide three things.
1. files: the spec files where its fix belongs, as paths relative to rewrite/spec/ (for example "commands/gate-red.md" or "tables/walls.toml"). Name the file that owns the fact, which is usually one file and at most four. Do not list every file that mentions the subject. Use the registry to find the owner and open the file to confirm it holds the text in question.
2. packages: the ids of the build packages that must act on it, taken from the package field of those files in the registry. Usually one or two.
3. on_slice_path: true if any of those packages is one of the slice's packages, otherwise false.

Write your answers to rewrite/findings/triage/${part.key}.json with the Write tool, as one JSON array with one object per finding: {"id": "...", "files": ["..."], "packages": ["..."], "on_slice_path": true, "note": "one short line saying what must change"}. Include every finding in your part, exactly once. Then run python3 -c "import json;print(len(json.load(open('rewrite/findings/triage/${part.key}.json'))))" to confirm the file parses, and python3 rewrite/tools/findings.py build to confirm the ledger still builds.

Do not edit any spec file; you are only sorting. Return the structured result. Under could_not_place, list the ids for which you could find no owning file, with a few words on why; give those an empty files list.`,
      { label: `triage:${part.key}`, phase: 'Triage', model: 'sonnet', effort: 'xhigh', schema: TRIAGE_SCHEMA },
    ),
  ),
)

// ---------------------------------------------------------------- Tables (runs alongside triage)
const OUTCOME_ITEM = {
  type: 'object',
  properties: {
    id: { type: 'string' },
    outcome: { type: 'string', enum: ['fixed', 'not-fixed', 'left-for-builder', 'rejected'] },
    what_was_done: { type: 'string' },
    where: { type: 'string' },
  },
  required: ['id', 'outcome', 'what_was_done', 'where'],
}
const WORK_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    outcomes_file: { type: 'string' },
    resolutions: { type: 'array', items: OUTCOME_ITEM },
    lint_result: { type: 'string' },
    coverage_result: { type: 'string' },
  },
  required: ['summary', 'outcomes_file', 'resolutions', 'lint_result', 'coverage_result'],
}

phase('Tables')
const tablesPromise = agent(
  `You are making machine-readable design tables say exactly what their originals say. ${LAYOUT}

An audit compared every TOML table under rewrite/spec/tables/ and every JSON schema under rewrite/spec/schemas/ with the original documents. Its reports are rewrite/spec/conflicts/audit-2-tables.md (ids T2-), audit-2-prose.md (P2-) and audit-2-structure.md (S2-); in the ledger the same findings have ids starting AU-. 33 of the 68 machine files are faithful. The rest are your work.

YOUR TASK: fix every finding in rewrite/spec/conflicts/audit-2-tables.md, and the findings in the other two reports that concern a file under rewrite/spec/tables/ or rewrite/spec/schemas/. The blockers: the roster says feature, verify, maintain and multi do not call story, and the originals say they do; the artifact table's story row is an excerpt the original calls illustrative, while state.md section 2.5 gives the full row; the code-reviewer's wall leaves the local class readable; the limit name ci_minutes survives only in a comment; the holdout class lost its exact block message.

Rules. The six originals are right by definition for this task. Copy values; never invent one. Every row, key, default, enumeration, type and required list must be traceable to original text; where the original gives only an excerpt and the complete data is stated elsewhere in the originals, build from the complete statement. Where the originals disagree, section 4 of the blueprint wins, then the rest of the blueprint, then the document that owns the subject; note the disagreement at the end of rewrite/spec/conflicts/tables-repair.md. Where the same data is defined both in a table and in a prose file, the table is the single definition and the prose file links to it; make that so for the artifact kinds and the configuration keys. Do not fix a design problem that the originals also have: that is someone else's job. One exception, because working code already exists: the files under next/tests/lint/roster.toml and next/plugins/vsdd-factory/schemas/ were built and reviewed from the same originals, so where a spec table or schema and its built counterpart differ, say so in tables-repair.md and do not change the built file.

When you are done, write rewrite/findings/outcomes/tables.json with the Write tool: a JSON array with one object per finding you dealt with, {"id": "AU-<the audit id>", "outcome": "fixed" or "not-fixed", "what_was_done": "...", "where": "the file and place"}. Use the AU- prefix so the ids match the ledger. Run python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage and fix what they report. Return the structured result with the same entries.`,
  { label: 'tables', phase: 'Tables', model: 'sonnet', effort: 'xhigh', schema: WORK_SCHEMA },
)

const triage = await triagePromise
const tables = await tablesPromise
const triagedTotal = triage.filter(Boolean).reduce((n, t) => n + t.triaged, 0)
log(`Triage: ${triagedTotal} findings placed; tables: ${tables ? tables.resolutions.filter((r) => r.outcome === 'fixed').length : 0} table findings fixed`)

// ---------------------------------------------------------------- Amend
phase('Amend')
const amend = await agent(
  `You are applying the most important open findings to a design, so that the next packages can be built from it. ${LAYOUT}

${SLICE}

YOUR WORK LIST: every finding in rewrite/findings/ledger.json that is a blocker or a major, is on the slice path (on_slice_path true), and whose id starts P3 or SL1-. That is about forty findings. Each now has a triage entry in rewrite/findings/triage/*.json naming the spec files where its fix belongs. Read each finding in full, and the report it came from in rewrite/history/reviews/ when the entry is not enough. Leave the minor findings and everything off the slice path: the builders of each package will close those in code.

The findings that matter most, because several reviewers found them independently:
- Local delivery has nowhere to merge. With no remote, the integration branch is the branch checked out in the person's own checkout; the design requires merging into it and also promises never to touch that checkout. Decide how local delivery works, in one place.
- The tool cannot recompute a review verdict: no record stores which method a review pass belonged to, nothing says how passes, disputes and rulings are found among step records, and a scoped re-review overwrites the first review's records.
- Delivery never checks red evidence, and in local mode some git commands can move the integration branch past the merge guard.
- Gate behaviour is underspecified: how a test maps to its file, where the base commit's report comes from, what a stub marker is, and whether a dirty worktree may be measured.
- A story branch is cut from "the cycle's base", which no record defines.

How to work.
- Decide each finding. Take the proposed resolution when it is sound; choose another when it is not, and say why. Choose by these goals in order: correct and enforceable; low touch; restartable; lean. Do not reopen a user decision recorded in rewrite/STATE.md or rewrite/spec/decisions.md.
- Where a finding starts SL1-, working code under next/ already made a choice. Read that code. Make the design say what the code does, unless the code is wrong, in which case say so and mark the finding left-for-builder with exactly what must change.
- One fact, one place. Make each change in the file that owns the fact, and only there. If another file restates the fact, replace the restatement with a link instead of editing two copies. Before finishing an item, search rewrite/spec/ for the names involved and make sure nothing contradicts the change.
- Make the smallest change that resolves the problem. Add no command, record, rule, agent or package unless the problem cannot be resolved without one, and say so when you do; register anything new in rewrite/spec/registry.d/amend.toml.
- Keep files under 16 KB and lines under 400 characters. Write no history into the spec: no finding ids, no "was", no "amended".
- When a decision cannot be made without knowing how the platform behaves, do not guess: mark the finding left-for-builder and say what the builder must test.

When you are done, write rewrite/findings/outcomes/slice-amend.json with the Write tool: a JSON array with one object per finding on your list, {"id": "...", "outcome": "fixed", "left-for-builder", "rejected" or "not-fixed", "what_was_done": "the decision in one or two sentences", "where": "the spec file or files changed"}. Then run python3 rewrite/tools/spec_lint.py, python3 rewrite/tools/sections.py coverage, python3 rewrite/tools/gen_packages.py and python3 rewrite/tools/findings.py build, and fix what the first two report. Return the structured result with the same entries.`,
  { label: 'amend:slice', phase: 'Amend', model: 'fable', effort: 'xhigh', schema: WORK_SCHEMA },
)

// ---------------------------------------------------------------- Verify
const VERIFY_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    clean: { type: 'boolean' },
    summary: { type: 'string' },
    checked: { type: 'number' },
    confirmed: { type: 'number' },
    remaining: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          problem: { type: 'string' },
          where: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'problem', 'where', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'clean', 'summary', 'checked', 'confirmed', 'remaining'],
}

phase('Verify')
const done = [...(tables ? tables.resolutions : []), ...(amend ? amend.resolutions : [])]
const verify = await agent(
  `You are checking two sets of fixes to a design. ${LAYOUT}

${SLICE}

Two agents have just worked on rewrite/spec/. One repaired the machine tables and schemas so that they say what the originals say; its record is rewrite/findings/outcomes/tables.json. The other decided and applied about forty slice-path blocker and major findings; its record is rewrite/findings/outcomes/slice-amend.json. Here is what they say they did:
${JSON.stringify(done)}

For each entry marked fixed: open the finding in rewrite/findings/ledger.json, open the place named, and confirm three things. The change is there. It resolves the problem as the finding described it (for a table finding: the value now matches the original document, which you must read; for an amendment: the decision is sound, buildable on the platform as proven in rewrite/STATE.md, and breaks no user decision). And the fact is stated in one file only, with nothing elsewhere in rewrite/spec/ contradicting it: search for the names involved. For each entry marked left-for-builder or rejected, judge whether that was right.

Then read the files changed for the five subjects several reviewers raised (local delivery, verdict recomputation, red evidence and the merge guard, gate behaviour, the cycle's base) from start to end as the builder who must implement them next, and report anything that still cannot be built from the text.

Report a new problem only if one of these fixes caused it or left it. Run python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage and say what they print.

Write your report to rewrite/findings/verify-slice-amend.md with the Write tool. Do not modify any other file. Return the structured result: checked is the number of entries you examined, confirmed the number you found sound. Set clean to true only if no blocker or major problem remains.`,
  { label: 'verify', phase: 'Verify', model: 'opus', effort: 'xhigh', schema: VERIFY_SCHEMA },
)

return {
  triage: triage.map((t, i) => (t ? t : { part: PARTS[i].key, failed: true })),
  tables: tables ? { summary: tables.summary, fixed: tables.resolutions.filter((r) => r.outcome === 'fixed').length, not_fixed: tables.resolutions.filter((r) => r.outcome !== 'fixed'), lint: tables.lint_result } : null,
  amend: amend ? { summary: amend.summary, resolutions: amend.resolutions, lint: amend.lint_result, coverage: amend.coverage_result } : null,
  verify,
}
