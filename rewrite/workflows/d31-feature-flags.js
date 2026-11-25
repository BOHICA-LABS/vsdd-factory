export const meta = {
  name: 'vsdd-1-0-d31-feature-flags',
  description: 'Write decision D31 (feature flag support in the products the factory builds) into the 1.0 design: one designer, two reviewers, one integrator, one confirmation',
  whenToUse: 'Once, after the user adopted D31 on 2026-11-24. Writes rewrite/spec/ and rewrite/findings/ only; nothing under next/.',
  phases: [
    { title: 'Design', detail: 'one designer writes D31 into rewrite/spec/', model: 'fable' },
    { title: 'Review', detail: 'two reviewers: fit with the design, and whether it yields sound flags in a product', model: 'opus' },
    { title: 'Integrate', detail: 'every finding decided and the design corrected', model: 'fable' },
    { title: 'Confirm', detail: 'each correction confirmed; what is still open goes to the ledger', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const WHERE = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin that builds software from a specification: it takes a brief through specification, stories, tests written first, implementation, adversarial review, delivery and release, for new products and for existing code. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Where things are:
- The design is rewrite/spec/, one small file per thing. rewrite/spec/README.md gives the rules: one fact in one place, every thing in the registry, links written [[kind:id]], no file over 16 KB and no line over 400 characters, machine-readable where a program reads it, no history. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it; each package has a generated brief, rewrite/spec/packages/<ID>.md. rewrite/spec/conventions.md gives names, paths and model tiers. rewrite/spec/decisions.md records the decisions the user adopted, D1 to D30; rewrite/spec/topics/decisions-built.md says where each is built.
- Research spikes were added to the design as a capability that cuts across agents, skills and workflows: rewrite/spec/topics/spikes.md and the things it links to show how such a capability is written here.
- The old system this rewrite replaces is under plugins/vsdd-factory/. It is reference only. It touched feature flags in two places: plugins/vsdd-factory/skills/code-delivery/SKILL.md (a line recommending a flag for high-risk changes) and plugins/vsdd-factory/templates/pr-description-template.md (a rollback section and a flags section). The user decided that every capability of the old system ports.
- Working code is under next/. Some packages are built: for those the shipped file under next/ is the truth and the spec file is a pointer to it.
- Known problems are in a ledger under rewrite/findings/: raw findings in sources/, the packages that must act in triage/, outcomes in outcomes/. python3 rewrite/tools/findings.py build writes ledger.json and one list per package in by-package/.

The checks on the design, run from the repository root: python3 rewrite/tools/spec_lint.py ; python3 rewrite/tools/sections.py coverage ; python3 rewrite/tools/gen_packages.py ; python3 rewrite/tools/findings.py build. They are clean today. A new file takes "sources: [decisions.md#D31]" in its frontmatter; the coverage check accepts that once decisions.md has the heading "### D31".

Other workflows are reading rewrite/spec/ right now, and one is building under next/. Do not edit anything under next/, plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Do not edit rewrite/spec/registry.toml, any existing file under rewrite/spec/tables/ or rewrite/spec/schemas/, or a package brief by hand. Do not commit, branch, stash or push, and never use git to discard or restore files: almost all of the work in this tree is uncommitted and other people are working in it.`

const DECISION = `DECISION D31, adopted by the user on 2026-11-24. The clauses are the user's; do not weaken or widen them.
1. vsdd-factory treats feature flags as a first-class concern of the software it builds. This is not about flags inside vsdd-factory itself.
2. Two halves, both in scope. (a) The product gets a sound flag system: one flag interface, a record of every flag, safe defaults when the flag source cannot be reached, and a documented kill switch. (b) Features are delivered through it: a story can ship behind a flag, default off, so that risky or half-finished work can merge without being exposed. This matters most for features added to existing code.
3. It is opt-in per project. A small command-line tool or library is not forced to carry flags.
4. Which flag system. The factory uses an established system and the person confirms which one; it never picks silently. In order of preference: (i) existing code: if ingest finds a flag system already in the codebase, the factory proposes keeping it and asks the person to confirm; (ii) named by the person or the brief (LaunchDarkly, Unleash and so on); (iii) otherwise the architect researches which established systems fit the product's stack and constraints, recommends one with reasons, and waits for the person's confirmation; (iv) rolling our own is the last resort, only when no established system fits (an offline or embedded product, for example), and it needs the person's explicit approval with the reason written into the decision record. The question is asked once per project, when the project opts in, and recorded so that later stories do not ask again.
5. The product's code reaches the flag system through one interface, so the vendor can be changed later. Where the product's language supports it, that interface is OpenFeature, the vendor-neutral standard. Using it is not rolling our own.
6. The factory writes flag definitions into the repository. It does not create or change flags in the outside service itself; applying them is left to the person or the project's pipeline. Keys for the service never go in the repository. Tests run without reaching the live service.
7. No new workflow. It threads through what the design already has:
   - a project setting: whether the project uses flags, and when a story must be flagged;
   - the mechanism, chosen by the architect in a decision record; ingest of existing code detects a flag system and adopts it instead of adding a second;
   - a flag record: each flag has a type, a default, an owning story, an intended lifetime and a removal condition;
   - story and tests: a flagged story's contract covers both states. Flag off leaves existing behaviour unchanged, which fits the green gate's comparison against the baseline; flag on is the new behaviour;
   - checks: every flag in code is registered and every registered flag is used; no code reads a flag outside the one interface; reviewers look for new code reachable with the flag off;
   - release and maintenance: release lists which flags turn on and how to roll back; each temporary flag creates a deferred item; maintenance opens a cleanup story when the removal condition is met.
8. Out of scope: operating flags in production (gradual ramping, targeting rules, experiment analysis). The factory builds and verifies. It can write the rollout plan and the runbook; it does not run the service.
9. Timing: written into the design now; built when the build widens past the thin slice (rewrite/spec/packaging/build-plan.md, section "The thin slice"). Nothing on the thin slice path changes.`

const LOOKUP = `Anything you state as fact about OpenFeature or about an outside flag system must be checked first with a web or documentation lookup (find the tools with ToolSearch). Use generic queries that contain nothing about this user, this repository or this project. If you cannot check a claim, do not make it. Name outside systems only as examples: nothing a program reads may depend on a vendor's name.`

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
    facts_looked_up: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' } },
    lint: { type: 'string' },
    coverage: { type: 'string' },
  },
  required: ['summary', 'files_created', 'files_edited', 'packages', 'checks', 'design_choices', 'ledger_entries', 'facts_looked_up', 'open_questions', 'lint', 'coverage'],
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

Read first: rewrite/spec/README.md, conventions.md, decisions.md, topics/decisions-built.md, topics/human-stops.md and the decision kinds it links to, topics/brownfield.md, topics/spikes.md (as the example of a capability added across the design), packaging/build-plan.md, and then the files of every thing D31 touches: the agents, workflows, artifacts, configuration groups, gates, lints and hook rules. Find them through the registry; do not guess. Read the two places the old system touched flags.

Write D31 into rewrite/spec/ as the smallest set of changes that makes it complete and buildable:
1. rewrite/spec/decisions.md: add D31 in the file's own form (Question, Decided, What the answer changes), under a heading for decisions adopted after 2026-11-23, and correct the opening lines so they no longer say every decision is D1 to D30 adopted on one day. Keep the entry short and link to the topic; the file must stay under 16 KB. Add D31 to rewrite/spec/topics/decisions-built.md.
2. New files for the facts D31 adds. Expect at least: a topic that explains the capability from end to end and links to everything else; a group of project configuration keys; the flag record (what is recorded about each flag, where it lives in a project, who writes it, who reads it); each check, written in the form this design already uses for something a program enforces (a lint, a gate, a hook rule, a command, a step of the survey), with what it reads, when it runs and what it refuses; and how the choice of flag system is asked, confirmed and recorded. Reuse an existing human stop and decision kind where one can carry the question; add a new one only if none can, and say why.
3. Short edits to the existing files of the things D31 touches, so that a builder of that thing finds its flag duties: a line or a short passage that links to the file that owns the fact, never a second copy of the fact.
4. One or more new build packages, ids starting with F (F1, F2, ...), each sized for one builder and one reviewer. Register them, and every other new thing, in a new file rewrite/spec/registry.d/d31.toml. A package entry is [package.F1] with summary, delivers, depends (text naming package ids) and source = "decisions.md#D31". Then run python3 rewrite/tools/gen_packages.py, which writes the briefs. Update rewrite/spec/packaging/build-plan.md: the count of packages, where the F packages come in the order of work, and that none is on the thin slice.
5. Where a shipped table or schema, or code already built under next/, must change for D31 (an artifact kind, a path class, a wall, the roster, a lint check, the configuration schema), do not edit it. Record each needed change in the ledger with three files: rewrite/findings/sources/extra-d31.json, a JSON array of {"id": "D31-<n>", "origin": "decision D31, design", "severity": "major" or "minor", "where": "the spec or code file", "problem": "what must be added and why", "proposed_resolution": "...", "on_slice_path": false}; rewrite/findings/outcomes/d31.json, a JSON array of {"id": "D31-<n>", "outcome": "left-for-builder", "what_was_done": "which package must take it and when", "where": "the file"}; and rewrite/findings/triage/d31.json, a JSON array of {"id": "D31-<n>", "files": ["spec paths relative to rewrite/spec/"], "packages": ["<ID>"], "on_slice_path": false}. Then run python3 rewrite/tools/findings.py build and check that each entry appears in the by-package list of the package that must act.
6. Port the old system's two touches: say where the recommendation and the rollback and flags sections of the pull request description now live.

Design goals, in this order: correct and enforceable; low touch; restartable; lean. Every clause of D31 must end as something a program checks, or as a named duty of a named agent with a named reviewer who checks it. A sentence that only says flags should be handled well is a defect. A check that cannot measure must say so loudly; it must never pass by default.

The design must answer these, and the topic must say what was decided and why, in a few lines each:
- how a project opts in, who may propose it, and what happens when a story needs a flag in a project that has not opted in;
- the setting for when a story must be flagged: its values, its default, and who applies it to a story;
- where flag records and flag definitions live, for a new product and for existing code, given that project state lives on its own branch and product files live in the product tree;
- how the checks find flag reads in code without a parser for every language, given that the design is neutral about language and the toolchain artifact holds a project's commands and path patterns; be honest about the limits of what a pattern can find;
- how both states are tested and what evidence shows it: flag off against the baseline, flag on as new behaviour, and how a test sets a flag without the live service;
- the default when the flag source cannot be reached, and how that is tested;
- lifetime: temporary and permanent flags, the removal condition, the deferred item, the maintenance sweep that opens the cleanup story, and who turns a flag on after its story is delivered with the flag off;
- flags in a project with more than one component;
- keys and secrets: how the design keeps them out of the repository, using what it already has;
- exactly what the factory writes for an outside service and what it never does;
- rolling our own: what approval and what record it needs, and what the design requires of a home-built system so that it is still sound.

${LOOKUP}

None of this is on the thin slice path, and builders of the thin slice are reading these files now. Say near the top of each new file that it is built when the build widens. An edit to a file a slice builder reads (the story artifact, the story workflow, the clerk, test-writer, implementer, verifier, adversary, security-reviewer, architect, demo-recorder, technical-writer, devops) must be short, sit under a heading or sentence that says the same, and add nothing to a table the slice uses. Keep the lint clean as you go: run it after each batch of edits.

When you are done, run all four checks and return the structured result: checks as one line each (its name, its form, what it refuses); design_choices as one line per question above; open_questions only for what cannot be decided without the user. Keep the summary under 200 words.`,
  { label: 'design:d31', phase: 'Design', model: 'fable', effort: 'xhigh', schema: DESIGN_SCHEMA },
)

if (!design) {
  log('The designer did not return. Nothing was reviewed.')
  return { design: null, note: 'designer did not return; inspect rewrite/spec/ and rewrite/spec/registry.d/d31.toml before relaunching' }
}

const account = JSON.stringify({ files_created: design.files_created, files_edited: design.files_edited, packages: design.packages, checks: design.checks, design_choices: design.design_choices, facts_looked_up: design.facts_looked_up, open_questions: design.open_questions })

const REVIEWS = [
  {
    key: 'fit',
    prefix: 'D31R-F',
    task: `Your lens is fit: does the design say exactly what the user decided, and does it fit the rest of the design?
- Go through D31 clause by clause, including every item of clause 7. For each, find where the design delivers it. Count the clauses covered and the total. A clause that is only restated, with no check and no named duty, is not covered.
- Find anything that goes beyond D31: anything that operates flags in production, anything a program reads that depends on a vendor's name, anything forced on a project that has not opted in, a new workflow, a human stop that is not needed.
- Check the rules of rewrite/spec/README.md: one fact in one place (search for the same fact stated twice, in new files and in the edited ones), every new thing registered with a package, links that resolve, sizes, frontmatter.
- Check it against the design it joins: the human stops and decision kinds (low touch: the question is asked once), who may read and write the flag record under the walls and path classes, the state branch against the product tree, the gates and their evidence, ingest of existing code, release, maintenance, deferred items, and the story script. Confirm that nothing a thin-slice builder must build has changed.
- Check the packages: each is sized for one builder, depends on packages that exist, creates no cycle, and the build plan says where it comes. Check that every change needed in a shipped table, schema or built code is in the ledger and appears in the by-package list of the package that must act (rewrite/findings/by-package/).
- Check that the old system's two touches are ported.`,
  },
  {
    key: 'domain',
    prefix: 'D31R-D',
    task: `Your lens is whether this would produce sound feature flags in a real product. Read it as an engineer who has run flag systems in production and seen them fail. For each failure below, say whether the design prevents it by a check a program runs, by a named duty with a named reviewer, or not at all; "not at all" and "by a sentence of advice" are findings.
- a temporary flag that is never removed; removal that leaves dead code behind; a permanent operational flag treated as a temporary release flag, or the reverse;
- a flag read in two places with different defaults; a flag key mistyped; a flag in code that is not recorded, or recorded and not used; a flag renamed in one place only;
- new code reachable with the flag off; the off path untested, or the baseline changed with the flag off; the on path untested; two flags that interact and are never tested together;
- tests that reach the live service; behaviour when the flag source is slow or cannot be reached, at start-up and later; a default that fails open for something that should fail closed;
- a flag wrapped around a change that flipping cannot undo (a data migration, a schema change, a message already sent);
- a flag evaluated in a client that exposes an unreleased feature to anyone who looks;
- keys committed to the repository; the factory calling the outside service; flag definitions in the repository that drift from the service;
- a project with more than one component, or more than one flag system; existing code with a flag system that ingest does not notice;
- a home-built flag system chosen without the person's approval, or built without the properties the design requires of an established one.
Then test whether each check can work as specified: can it find what it claims to find in any language, with only a project's commands and path patterns to go on? What does it do when it cannot measure? Would it produce so many false alarms that people switch it off? Finally check every factual claim about OpenFeature and outside systems. ${LOOKUP}`,
  },
]

phase('Review')
const reviews = await parallel(
  REVIEWS.map((r) => () =>
    agent(
      `You are reviewing an addition to the design of vsdd-factory 1.0 before anything is built from it. ${WHERE}

${DECISION}

A designer wrote D31 into rewrite/spec/. What the designer says it did:
${account}

You did not write this and should not trust the account. Run the four checks yourself and report what they print. Read every file the designer created or edited, in full, and the existing files they link to where you need them.

${r.task}

Severity: "blocker" means the design contradicts the decision or cannot be built; "major" means a clause not delivered, a check that cannot work or passes when it cannot measure, a real conflict with the existing design, or a change on the thin slice path; "minor" is anything else worth fixing. Give each finding an id (${r.prefix}-1, ${r.prefix}-2, ...), the file, the problem and a proposed fix.

Write your findings to rewrite/findings/sources/extra-d31-review-${r.key}.json with the Write tool, as a JSON array with one object per finding: {"id": "...", "origin": "decision D31, review (${r.key})", "severity": "...", "where": "the spec file, as a path relative to rewrite/spec/ where it has one", "problem": "...", "proposed_resolution": "...", "on_slice_path": false}. Write an empty array if you have none. That is the only file you may create or change. Then return the structured result with the same findings.`,
      { label: `review:${r.key}`, phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
    ),
  ),
)
const findings = reviews.filter(Boolean).flatMap((r) => r.findings)
const lost = REVIEWS.filter((_, i) => !reviews[i]).map((r) => r.key)
if (lost.length) log(`Reviewers that did not return: ${lost.join(', ')}`)
log(`D31 reviews: ${findings.length} findings, ${findings.filter((f) => f.severity !== 'minor').length} blocker or major.`)

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
    files_changed: { type: 'array', items: { type: 'string' } },
    lint: { type: 'string' },
    coverage: { type: 'string' },
  },
  required: ['summary', 'resolutions', 'files_changed', 'lint', 'coverage'],
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

A designer wrote D31 into rewrite/spec/ (the new things are registered in rewrite/spec/registry.d/d31.toml). Two reviewers read it: one for fit with the decision and the rest of the design, one for whether it would produce sound feature flags in a real product. Their findings:
${JSON.stringify(findings)}

Decide every finding, the minor ones included, and correct the design. Keep to the rules of rewrite/spec/README.md, and to the decision: where a finding asks for more than the user decided, take what makes the decision correct and enforceable and reject the rest, saying why. The design goals in order: correct and enforceable; low touch; restartable; lean. Outcomes: "fixed" when the design now says the right thing; "rejected" with a reason a reviewer can check; "left-for-builder" only for something that can be settled only in code when a package is built, naming the package; "needs-the-user" only for a choice that is truly the user's, stated as one question.

${LOOKUP}

Nothing on the thin slice path may change. Keep the lint clean as you go.

Write rewrite/findings/outcomes/d31-fix.json with the Write tool: a JSON array with one object per finding, {"id": "...", "outcome": "fixed" | "rejected" | "left-for-builder" | "not-fixed", "what_was_done": "...", "where": "the file"} (record "needs-the-user" as "not-fixed" and put the question in what_was_done). For each left-for-builder, add an entry to a new file rewrite/findings/triage/d31-review.json: {"id": "...", "files": ["spec paths relative to rewrite/spec/"], "packages": ["<ID>"], "on_slice_path": false}. Then run all four checks and return the structured result with one resolution per finding.`,
    { label: 'integrate:d31', phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: FIX_SCHEMA },
  )
  if (fixed) {
    phase('Confirm')
    confirm = await agent(
      `You are confirming corrections to an addition to the design of vsdd-factory 1.0. ${WHERE}

${DECISION}

Reviewers found these problems with the way D31 was written into rewrite/spec/:
${JSON.stringify(findings)}

An integrator says it dealt with them like this:
${JSON.stringify(fixed.resolutions)}

You did not make these corrections and should not trust the account. Run the four checks yourself and report what they print. Then, for every finding: read the files and confirm the problem is gone and that the correction did not state a fact twice, contradict another file, go beyond the decision or change anything on the thin slice path. For a finding marked rejected, judge the reason: if it holds, count the finding as confirmed. For one marked left-for-builder, confirm that it can only be settled in code and that it appears in the by-package list of the package named (rewrite/findings/by-package/).

List under still_open every finding that is not properly dealt with, keeping its id, with what is still wrong and what to do. If a correction introduced a new problem, add it with a new id (D31R-C-1, ...). Write rewrite/findings/outcomes/d31-confirm.json with the Write tool: a JSON array with one object per finding in still_open that kept its id, {"id": "...", "outcome": "not-fixed", "what_was_done": "what the check found still wrong", "where": "the file"}; write an empty array if all are dealt with. If you added new problems, also write them to rewrite/findings/sources/extra-d31-confirm.json as a JSON array of {"id": "...", "origin": "decision D31, confirmation", "severity": "...", "where": "the file", "problem": "...", "proposed_resolution": "...", "on_slice_path": false}. Those are the only files you may create or change. Return the structured result.`,
      { label: 'confirm:d31', phase: 'Confirm', model: 'opus', effort: 'xhigh', schema: CONFIRM_SCHEMA },
    )
  } else {
    log('The integrator did not return. The review findings are in the ledger and still open.')
  }
}

return {
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
  integrate: fixed ? { summary: fixed.summary, not_fixed: fixed.resolutions.filter((x) => x.outcome !== 'fixed'), lint: fixed.lint, coverage: fixed.coverage } : null,
  confirm: confirm ? { checks_rerun: confirm.checks_rerun, checked: confirm.checked, confirmed: confirm.confirmed, still_open: confirm.still_open } : null,
}
