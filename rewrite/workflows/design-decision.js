export const meta = {
  name: 'vsdd-1-0-design-decision',
  description: 'Write one adopted decision into the 1.0 design: one designer (Fable), two reviewers (Opus: fit with the design; soundness in practice), one integrator (Fable), one confirmation (Opus)',
  whenToUse: 'When the user adopts a decision that adds a capability across the design. Pass args {id, slug, decision, read_first, must_answer, soundness_lens, package_prefix, extra?}. Writes rewrite/spec/ and rewrite/findings/ only. Findings the confirmation leaves open go to spec-fix-open.js.',
  phases: [
    { title: 'Design', detail: 'one designer writes the decision into rewrite/spec/', model: 'fable' },
    { title: 'Review', detail: 'two reviewers: fit with the design, and soundness in practice', model: 'opus' },
    { title: 'Integrate', detail: 'every finding decided and the design corrected', model: 'fable' },
    { title: 'Confirm', detail: 'each correction confirmed; what is still open goes to the ledger', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'
const A = args || {}
for (const name of ['id', 'slug', 'decision', 'read_first', 'must_answer', 'soundness_lens', 'package_prefix']) {
  if (!A[name]) throw new Error(`args.${name} is required`)
}
const ID = A.id
const SLUG = A.slug
const LOW = ID.toLowerCase()
const PREFIX = A.package_prefix
const EXTRA = A.extra || ''

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin that builds software from a specification: it takes a brief through specification, stories, tests written first, implementation, adversarial review, delivery and release, for new products and for existing code. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the rules: one fact in one place, every thing in the registry, links written [[kind:id]], no file over 16 KB and no line over 400 characters, machine-readable where a program reads it, no history. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it; each package has a generated brief, rewrite/spec/packages/<ID>.md. rewrite/spec/conventions.md gives names, paths and model tiers. rewrite/spec/decisions.md records the decisions D1 to D30 and rewrite/spec/decisions-later.md those adopted since; rewrite/spec/topics/decisions-built.md says where each is built.
- Two capabilities were added across the design after it was first written, and show how that is done here: research spikes (rewrite/spec/topics/spikes.md and what it links to) and feature flags (decision D31: rewrite/spec/topics/feature-flags.md, its registrations in rewrite/spec/registry.d/d31.toml, and the short linked passages it left in the files it touches).
- Working code is under next/. Some packages are built: for those the shipped file under next/ is the truth and the spec file is a pointer to it. The thin slice (rewrite/spec/packaging/build-plan.md, section "The thin slice") is being built now from the plans in rewrite/plans/; builders and planners are reading rewrite/spec/ as they work.
- Known problems are in a ledger under rewrite/findings/: raw findings in sources/, the packages that must act in triage/, outcomes in outcomes/. python3 rewrite/tools/findings.py build writes ledger.json and one list per package in by-package/.
- The platform facts that were proven by experiment are rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md and platform-proven-2.md.

The checks on the design, run from the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/sections.py coverage ; python3 rewrite/tools/gen_packages.py ; python3 rewrite/tools/findings.py build. They are clean today. A new file takes "sources: [decisions.md#${ID}]" in its frontmatter; the coverage check accepts that once rewrite/spec/decisions-later.md has the heading "### ${ID}".

Do not edit anything under next/, plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Do not edit rewrite/spec/registry.toml, any existing file under rewrite/spec/tables/ or rewrite/spec/schemas/, or a package brief by hand. Do not commit, branch, stash or push, and never use git in this repository to discard or restore files: almost all of the work in this tree is uncommitted and other people are working in it.`

const DECISION = `DECISION ${ID}, adopted by the user on 2026-11-24. The clauses are the user's; do not weaken or widen them.
${A.decision}`

const LOOKUP = `Anything you state as fact about git, a Git host or another outside system must be something you have checked: run it in a scratch repository outside this one (mktemp -d), or look it up with a web or documentation tool (find them with ToolSearch), using generic queries that contain nothing about this user, this repository or this project. If you cannot check a claim, do not make it. Nothing a program reads may depend on the name of a Git host or vendor.`

const DESIGN_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    files_created: { type: 'array', items: { type: 'string' } },
    files_edited: { type: 'array', items: { type: 'string' } },
    packages: { type: 'array', items: { type: 'string' } },
    checks: { type: 'array', items: { type: 'string' } },
    design_choices: { type: 'array', items: { type: 'string' } },
    ledger_entries: { type: 'number' },
    facts_checked: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' } },
    lint: { type: 'string' },
    coverage: { type: 'string' },
  },
  required: ['summary', 'files_created', 'files_edited', 'packages', 'checks', 'design_choices', 'ledger_entries', 'facts_checked', 'open_questions', 'lint', 'coverage'],
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
    verdict: { type: 'string' },
    checks_rerun: { type: 'string' },
    clauses_covered: { type: 'number' },
    clauses_total: { type: 'number' },
    findings: { type: 'array', items: FINDING },
  },
  required: ['verdict', 'checks_rerun', 'clauses_covered', 'clauses_total', 'findings'],
}

phase('Design')
const design = await agent(
  `You are writing one adopted decision into the design of vsdd-factory 1.0. ${WHERE}

${DECISION}

Read first: rewrite/spec/README.md, conventions.md, decisions.md, decisions-later.md, topics/decisions-built.md, packaging/build-plan.md, and then: ${A.read_first} Find the rest through the registry; do not guess.

Write ${ID} into rewrite/spec/ as the smallest set of changes that makes it complete and buildable:
1. rewrite/spec/decisions-later.md: add ${ID} in the file's own form (Question, Decided, What the answer changes). Keep the entry short and link to the topic that explains it. Add ${ID} to rewrite/spec/topics/decisions-built.md.
2. New files for the facts ${ID} adds: at least one topic that explains the capability from end to end and links to everything else, and then whatever records, commands, configuration keys, checks and tables it needs, each written in the form this design already uses for that kind of thing. Each check says what it reads, when it runs and what it refuses. Reuse what exists (a record, a command, a human stop, a decision kind, a gate) where it can carry the need; add a new one only if none can, and say why.
3. Short edits to the existing files of the things ${ID} touches, so that a builder of that thing finds its duties: a line or a short passage under a heading that names decision ${ID}, linking to the file that owns the fact, never a second copy of the fact.
4. One or more new build packages, ids starting with ${PREFIX} (${PREFIX}1, ${PREFIX}2, ...), each sized for one builder and one reviewer. Register them, and every other new thing, in a new file rewrite/spec/registry.d/${LOW}.toml. A package entry is [package.${PREFIX}1] with summary, delivers, depends (text naming package ids) and source = "decisions.md#${ID}". Then run python3 rewrite/tools/gen_packages.py, which writes the briefs. Update rewrite/spec/packaging/build-plan.md: the count of packages, where the new packages come in the order of work, and that none is on the thin slice.
5. Where a shipped table or schema, or code already built or planned for the thin slice, must change for ${ID}, do not edit it. Record each needed change in the ledger with three files: rewrite/findings/sources/extra-${LOW}.json, a JSON array of {"id": "${ID}-<n>", "origin": "decision ${ID}, design", "severity": "major" or "minor", "where": "the spec, plan or code file", "problem": "what must be added or must not be assumed, and why", "proposed_resolution": "...", "on_slice_path": true or false}; rewrite/findings/outcomes/${LOW}.json, a JSON array of {"id": "${ID}-<n>", "outcome": "left-for-builder", "what_was_done": "which package must take it and when", "where": "the file"}; and rewrite/findings/triage/${LOW}.json, a JSON array of {"id": "${ID}-<n>", "files": ["spec paths relative to rewrite/spec/"], "packages": ["<ID>"], "on_slice_path": true or false}. Then run python3 rewrite/tools/findings.py build and check that each entry appears in the by-package list of the package that must act.
${EXTRA}

Design goals, in this order: correct and enforceable; low touch; restartable; lean. Every clause of ${ID} must end as something a program checks, or as a named duty of a named agent with a named reviewer who checks it. A sentence that only says a thing should be handled well is a defect. A check that cannot measure must say so loudly; it must never pass by default.

The design must answer these, and the topic must say what was decided and why, in a few lines each:
${A.must_answer}

${LOOKUP}

Builders of the thin slice are reading these files now. Say near the top of each new file that it is built when the build widens. An edit to a file a slice builder reads must be short, sit under a heading or sentence that says the same, and add nothing to a table the slice uses. Keep the lint clean as you go: run it after each batch of edits.

When you are done, run all four checks and return the structured result: checks as one line each (its name, its form, what it refuses); design_choices as one line per question above; open_questions only for what cannot be decided without the user. Keep the summary under 200 words.`,
  { label: `design:${LOW}`, phase: 'Design', model: 'fable', effort: 'xhigh', schema: DESIGN_SCHEMA },
)

if (!design) {
  log('The designer did not return. Nothing was reviewed.')
  return { design: null, note: `designer did not return; inspect rewrite/spec/ and rewrite/spec/registry.d/${LOW}.toml before relaunching` }
}

const account = JSON.stringify({ files_created: design.files_created, files_edited: design.files_edited, packages: design.packages, checks: design.checks, design_choices: design.design_choices, facts_checked: design.facts_checked, open_questions: design.open_questions })

const REVIEWS = [
  {
    key: 'fit',
    prefix: `${ID}R-F`,
    task: `Your lens is fit: does the design say exactly what the user decided, and does it fit the rest of the design?
- Go through ${ID} clause by clause. For each, find where the design delivers it. Count the clauses covered and the total. A clause that is only restated, with no check and no named duty, is not covered.
- Find anything that goes beyond ${ID} or that it rules out, anything a program reads that depends on a vendor's or host's name, a new workflow or human stop that is not needed, and anything forced on a project that does not use the capability.
- Check the rules of rewrite/spec/README.md: one fact in one place (search for the same fact stated twice, in new files and in the edited ones), every new thing registered with a package, links that resolve, sizes, frontmatter.
- Check it against the design it joins, file by file for every existing thing it touches or relies on, and against the platform facts that were proven. Confirm that nothing a thin-slice builder must build has changed, and that what the slice units must not assume is recorded for them in the ledger.
- Check the packages: each is sized for one builder, depends on packages that exist, creates no cycle, and the build plan says where it comes. Check that every change needed in a shipped table, schema, plan or built code is in the ledger and appears in the by-package list of the package that must act (rewrite/findings/by-package/).`,
  },
  {
    key: 'soundness',
    prefix: `${ID}R-S`,
    task: `Your lens is whether this would hold up in real use. ${A.soundness_lens}
For each failure, say whether the design prevents it by a check a program runs, by a named duty with a named reviewer, or not at all; "not at all" and "by a sentence of advice" are findings. Then test whether each mechanism can work as specified: try the git commands it rests on in scratch repositories outside this one (two or three clones of one bare remote), including the races, and report what git actually does. What does each check do when it cannot measure? Finally check every factual claim about git or an outside system. ${LOOKUP}`,
  },
]

phase('Review')
const reviews = await parallel(
  REVIEWS.map((r) => () =>
    agent(
      `You are reviewing an addition to the design of vsdd-factory 1.0 before anything is built from it. ${WHERE}

${DECISION}

A designer wrote ${ID} into rewrite/spec/. What the designer says it did:
${account}

You did not write this and should not trust the account. Run the four checks yourself and report what they print. Read every file the designer created or edited, in full, and the existing files they link to where you need them.

${r.task}

Severity: "blocker" means the design contradicts the decision or cannot be built; "major" means a clause not delivered, a mechanism that cannot work or a check that passes when it cannot measure, a real conflict with the existing design, or a change on the thin slice path; "minor" is anything else worth fixing. Give each finding an id (${r.prefix}-1, ${r.prefix}-2, ...), the file, the problem and a proposed fix.

Write your findings to rewrite/findings/sources/extra-${LOW}-review-${r.key}.json with the Write tool, as a JSON array with one object per finding: {"id": "...", "origin": "decision ${ID}, review (${r.key})", "severity": "...", "where": "the spec file, as a path relative to rewrite/spec/ where it has one", "problem": "...", "proposed_resolution": "...", "on_slice_path": false}. Write an empty array if you have none. That is the only file in this repository you may create or change. Then return the structured result with the same findings.`,
      { label: `review:${r.key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
    ),
  ),
)
const findings = reviews.filter(Boolean).flatMap((r) => r.findings)
const lost = REVIEWS.filter((_, i) => !reviews[i]).map((r) => r.key)
if (lost.length) log(`Reviewers that did not return: ${lost.join(', ')}`)
log(`${ID} reviews: ${findings.length} findings, ${findings.filter((f) => f.severity !== 'minor').length} blocker or major.`)

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
    lint: { type: 'string' },
    coverage: { type: 'string' },
  },
  required: ['summary', 'resolutions', 'decisions_made', 'files_changed', 'lint', 'coverage'],
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

let fixed = null
let confirm = null
if (findings.length) {
  phase('Integrate')
  fixed = await agent(
    `You are correcting an addition to the design of vsdd-factory 1.0. ${WHERE}

${DECISION}

A designer wrote ${ID} into rewrite/spec/ (the new things are registered in rewrite/spec/registry.d/${LOW}.toml). Two reviewers read it: one for fit with the decision and the rest of the design, one for whether it would hold up in real use. Their findings:
${JSON.stringify(findings)}

Decide every finding, the minor ones included, and correct the design. You may rewrite the files the designer created, the passages it added to existing files, and the ${ID} ledger files (rewrite/findings/sources/extra-${LOW}.json, outcomes/${LOW}.json, triage/${LOW}.json), so that every open ledger entry describes the design as it stands when you finish. Keep to the rules of rewrite/spec/README.md, and to the decision: where a finding asks for more than the user decided, take what makes the decision correct and enforceable and reject the rest, saying why. The design goals in order: correct and enforceable; low touch; restartable; lean. Correct the whole of what a finding describes, not only its example, and after each correction search for the same fact stated elsewhere so that it stays in one place. Outcomes: "fixed" when the design now says the right thing; "rejected" with a reason a reviewer can check; "left-for-builder" only for something that can be settled only in code when a package is built, naming the package; "needs-the-user" only for a choice that is truly the user's, stated as one question. List under decisions_made each design choice you made, one line each.

${LOOKUP}

Nothing on the thin slice path may change. Keep the lint clean as you go.

Write rewrite/findings/outcomes/${LOW}-fix.json with the Write tool: a JSON array with one object per finding, {"id": "...", "outcome": "fixed" | "rejected" | "left-for-builder" | "not-fixed", "what_was_done": "...", "where": "the file"} (record "needs-the-user" as "not-fixed" and put the question in what_was_done). For each left-for-builder, add an entry to a new file rewrite/findings/triage/${LOW}-review.json: {"id": "...", "files": ["spec paths relative to rewrite/spec/"], "packages": ["<ID>"], "on_slice_path": false}. Then run all four checks and return the structured result with one resolution per finding.`,
    { label: `integrate:${LOW}`, phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (fixed) {
    phase('Confirm')
    confirm = await agent(
      `You are confirming corrections to an addition to the design of vsdd-factory 1.0. ${WHERE}

${DECISION}

Reviewers found these problems with the way ${ID} was written into rewrite/spec/:
${JSON.stringify(findings)}

An integrator says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

and made these design choices: ${JSON.stringify(fixed.decisions_made)}

You did not make these corrections and should not trust the account. Run the four checks yourself and report what they print. Then, for every finding and every part of it: read the files and confirm the problem is gone and that the correction did not state a fact twice, contradict another file, go beyond the decision or change anything on the thin slice path. Judge each design choice: would it work as specified (try the git commands it rests on in a scratch repository outside this one where that settles it), and is it enforceable? For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed. For one marked left-for-builder, confirm that it can only be settled in code and that it appears in the by-package list of the package named (rewrite/findings/by-package/).

List under still_open every finding that is not properly dealt with, keeping its id, with what is still wrong and what to do. If a correction introduced a new problem, add it with a new id (${ID}R-C-1, ...). Write rewrite/findings/outcomes/${LOW}-confirm.json with the Write tool: a JSON array with one object per finding in still_open that kept its id, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are dealt with. If you added new problems, also write them to rewrite/findings/sources/extra-${LOW}-confirm.json as a JSON array of {"id": "...", "origin": "decision ${ID}, confirmation", "severity": "...", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": false}. Those are the only files in this repository you may create or change. Return the structured result.`,
      { label: `confirm:${LOW}`, phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
    )
  } else {
    log('The integrator did not return. The review findings are in the ledger and still open.')
  }
}

return {
  id: ID,
  slug: SLUG,
  design: {
    summary: design.summary,
    files_created: design.files_created,
    files_edited: design.files_edited,
    packages: design.packages,
    checks: design.checks,
    design_choices: design.design_choices,
    ledger_entries: design.ledger_entries,
    open_questions: design.open_questions,
  },
  reviews: REVIEWS.map((r, i) => (reviews[i] ? { lens: r.key, verdict: reviews[i].verdict, clauses_covered: reviews[i].clauses_covered, clauses_total: reviews[i].clauses_total, findings: reviews[i].findings.length, blockers: reviews[i].findings.filter((f) => f.severity === 'blocker').length, majors: reviews[i].findings.filter((f) => f.severity === 'major').length } : { lens: r.key, verdict: 'did not return' })),
  integrate: fixed ? { summary: fixed.summary, decisions_made: fixed.decisions_made, not_fixed: fixed.resolutions.filter((x) => x.outcome !== 'fixed'), lint: fixed.lint, coverage: fixed.coverage } : null,
  confirm: confirm ? { checks_rerun: confirm.checks_rerun, checked: confirm.checked, confirmed: confirm.confirmed, still_open: confirm.still_open } : null,
}
