export const meta = {
  name: 'vsdd-1-0-design',
  description: 'Design vsdd-factory 1.0: five subsystem designs, three adversarial reviews, one blueprint for approval',
  whenToUse: 'Step 2 of the vsdd-factory 1.0 rewrite. Produces rewrite/design/*.md and rewrite/BLUEPRINT.md.',
  phases: [
    { title: 'Design', detail: 'five designers, one subsystem each', model: 'fable' },
    { title: 'Review', detail: 'three reviewers attack the seams, coverage and operations', model: 'opus' },
    { title: 'Synthesize', detail: 'one blueprint, findings resolved, detail documents amended', model: 'fable' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const AREAS = [
  { key: 'pipeline', file: 'rewrite/design/pipeline.md', title: 'pipeline workflows, human stops, convergence and research spikes' },
  { key: 'state', file: 'rewrite/design/state.md', title: 'project state, artifacts, and resume after compaction or from cold' },
  { key: 'agents-skills', file: 'rewrite/design/agents-skills.md', title: 'the agent roster, the skill roster, and models' },
  { key: 'hooks', file: 'rewrite/design/hooks.md', title: 'the hook runner and its rules' },
  { key: 'packaging', file: 'rewrite/design/packaging.md', title: 'packaging, repository layout, build, tests, release, cutover and supporting tools' },
]

const COMMON = (area) => `You are one of five designers producing the design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Read rewrite/STATE.md in full before anything else. It holds the user's decisions, the platform facts that were verified by experiment, and the requirements your design must meet. User decisions are fixed. Where STATE.md marks something as the assistant's reading, you may improve on it and should say that you did.

The existing code, the old plugin under plugins/vsdd-factory/, the docs, and the repository CLAUDE.md that was loaded into your context all describe the system being replaced. Use them to learn what the system is for and what it does. Their process rules (state-manager bursts, decision-log codification, the policy registry, "the orchestrator never writes files", "no new shell scripts", the release-candidate compatibility promises) do not apply to this task and are not requirements of the new design. Carry a rule over only if you can justify it on its own merits.

Surveys of the old system are in rewrite/reference/. Old pipeline artifacts are on the origin/factory-artifacts branch; read them with git show "origin/factory-artifacts:<path>" (quote the argument). The .factory directory is not mounted in this clone.

Design goals, in priority order:
1. Correct and enforceable: rules are enforced by code, not by asking an agent to behave.
2. Low touch: little human effort to run it and keep it running. A human stop exists only where a person must decide.
3. Restartable: everything resumes from disk after a compaction or in a brand new session.
4. Lean: every agent that starts pays a fixed context cost, and the old system collapsed under its own bookkeeping. Prefer fewer parts, and do not rebuild that bookkeeping.

Shared vocabulary, so the five designs fit together:
- The plugin's root directory is written <plugin>/. The plugin name is vsdd-factory, so agents are vsdd-factory:<name> and workflows are /vsdd-factory:<name>.
- The per-project state directory is written .factory/. The state designer decides its final name, storage and layout; everyone else refers to it this way.
- Model tiers: "planner" is Fable at extra-high effort, "builder" is Sonnet at extra-high effort, "reviewer" is Opus at extra-high effort.
- A "spike" is an isolated, bounded experiment that answers one question.

The other four designers are working in parallel on: ${AREAS.filter((a) => a.key !== area.key).map((a) => a.title).join('; ')}. Do not design their areas. Where your design depends on theirs, state exactly what you need from them under "Interfaces I need", precisely enough that a mismatch can be detected later.

YOUR AREA: ${area.title}.`

const OUTPUT = (area) => `OUTPUT

Write your design to ${area.file} with the Write tool. It will be read by reviewers, by the person who approves the design, and later by builders who have not seen any of this, so write complete sentences, define your terms, and use tables for rosters and mappings. Be specific: real names, real file paths, real schemas. Where you choose between alternatives, say what you chose and why in a line or two. Do not write the product itself; short illustrative skeletons are welcome where the shape matters.

Use these sections, in this order:
1. Summary: what this subsystem is in 1.0, in under 200 words.
2. The design.
3. Mapping from the old system: every old item in your area, marked ported, merged, replaced or dropped, with the reason for anything dropped.
4. Interfaces I provide.
5. Interfaces I need.
6. Open questions for the user: only decisions a person must make, each with your recommendation.
7. Risks.

Do not modify any file other than ${area.file}. Do not commit, branch, or push. When the file is written, return the structured result; keep its summary under 150 words.`

const BRIEFS = {
  pipeline: `Read as well: rewrite/reference/old-workflows-packaging-map.md; the old workflow files under plugins/vsdd-factory/workflows/ (greenfield, feature, brownfield, code-delivery, maintenance, discovery, multi-repo, planning, and the phases/ directory); plugins/vsdd-factory/docs/VSDD.md and CONVERGENCE.md; docs/guide/workflow-modes.md and docs/guide/pipeline-paths.md; plugins/vsdd-factory/agents/orchestrator/ (the sequence files and its per-story playbook, which conflicts with the one under workflows/phases/); and the proven script patterns in rewrite/spike/plugin/workflows/.

Design all of the following.
- The complete set of 1.0 workflows. For each: name, purpose, arguments, the ordered steps (agent type, model tier, what the step is given, the structured result it returns), the gates written as conditions over result fields, loops and their exit rules, what is written to disk at each step boundary, and where it ends for a human decision.
- How each workflow works out what is already done when it starts, so that running the same command in a new session continues the work. Remember that a script cannot read files: an agent step has to do the reading and return what it found.
- Coverage of every old mode and path: greenfield, brownfield, feature (delta phases F1 to F7 and the trivial, bug-fix and critical-bug variants), maintenance sweeps, discovery, planning, multi-repo, per-story delivery, wave gates, holdout evaluation, adversarial refinement, formal hardening, convergence, and release.
- The human stops: which of the old approval steps remain, and why each is a decision only a person can make.
- A convergence rule that can terminate: what counts as a clean pass, how many are needed, the cap, and what happens at the cap. The old rule of three consecutive clean passes was described by its own maintainers as structurally impossible.
- Research spikes at every level: the request a step can return, the shared wrapper that acts on it (give the JavaScript), the limits per step and per run, how the finding returns to the step that asked, and how the slash-command and direct-dispatch entrances relate to it.
- The shared result schemas every workflow uses (step result, review finding, verdict, spike request, spike finding), as JSON Schema.
- Illustrative script skeletons, valid under the platform limits, for story delivery and for a wave.
- For each place where you changed the old flow, one line on what was wrong and what is better. The user asked for the workflows to be improved, not transcribed.`,

  state: `Read as well: plugins/vsdd-factory/docs/FACTORY.md (the sections on layout, identifiers, frontmatter, sharding and lifecycle); plugins/vsdd-factory/rules/factory-protocol.md and spec-format.md; plugins/vsdd-factory/config/artifact-path-registry.yaml; the file list of plugins/vsdd-factory/templates/; section 5 of rewrite/reference/old-workflows-packaging-map.md; and, to see what the old model grew into, the tree and a few large files on origin/factory-artifacts (its STATE.md is 231 KB with single lines of 30,000 characters: look at headings and sizes, not the body).

Design all of the following.
- The state directory: its location, its storage, its layout, and who commits it and when. The old design is an orphan git branch mounted as a worktree; decide whether to keep that and justify the choice against low touch.
- Every artifact type: the specification chain (product brief, domain spec, behavioral contracts, architecture and decision records, verification properties), epics, stories, waves, holdout scenarios, reviews and findings, convergence records, demo evidence, research, the spike ledger, lessons, and deferred work. For each: path, identifier scheme, frontmatter schema, which agent role writes it, size cap, and lifecycle.
- What is stored and what is derived. Indexes, counts and version citations that the old system maintained by hand should be generated or should not exist.
- The pipeline status record: the smallest file that tells a cold session where the project is. Give its schema and its size cap, and say exactly who updates it and when.
- The session briefing printed at session start, on resume and after compaction: its content, how the hook runner computes it cheaply from files, and its length limit.
- What is flushed before compaction, and the single status-and-resume entry point that replaces the old wrap, wave-handoff, wave-reset, rehydrate-wave, recover-state and compact-state skills.
- The project configuration file, in TOML: build and test commands, language, limits for loops and spikes, and anything else workflows and hooks must read.
- What becomes of the artifact templates (136 today).
- Multi-repo projects: where shared state lives.
- Whether projects already running the old plugin get an importer. This is a question for the user; give your recommendation.`,

  'agents-skills': `Read as well: rewrite/reference/old-agents-inventory.md and rewrite/reference/old-skills-inventory.md in full; sections 3 and 4 of rewrite/reference/claude-code-plugin-platform-facts.md; plugins/vsdd-factory/docs/AGENT-SOUL.md; and a sample of the old files to judge them: the agents adversary, implementer, test-writer, product-owner, architect, pr-manager, state-manager, holdout-evaluator and research-agent under plugins/vsdd-factory/agents/, and the skills deliver-story, adversarial-review, wave-gate, brownfield-ingest, create-prd and release under plugins/vsdd-factory/skills/.

Design all of the following.
- The 1.0 agent roster. For each agent: name, role in one line, model tier, tool restrictions, what it is given, the structured result it returns, and which old agents it replaces. Apply the user's model policy: planner for planning, thinking and creative roles; reviewer for the adversary, the security reviewer and every other independent reviewer; builder for everything else.
- Which old agents disappear or merge, and why. Workflows now do the orchestrating, so question every coordinator, bookkeeper and relay agent.
- The standard agent file: a template with its frontmatter and the sections every agent has. Keep it short; the old files carry boilerplate, a dead platform's tool vocabulary, and paths that do not resolve.
- The information walls: which agents must not read which paths, as a table the hook designer can enforce by the calling agent's type.
- The 1.0 skill roster. For each skill: name, purpose, whether a person invokes it, the model may invoke it, or both, its frontmatter, and what it does (starts a workflow, runs in a forked agent, or instructs the main session).
- A mapping of all 127 old skills: kept, merged into a named skill, replaced by a named workflow, or dropped with the reason. The user decided every capability ports; duplicates merge.
- Keep the number of model-visible skills small. Every agent start pays for the skill listing, and descriptions are truncated at about 1,500 characters.
- How research tools (Perplexity, Context7, Tavily) and browser tools reach the agents that need them, given that a plugin agent cannot declare its own MCP servers.
- The rule agents follow for spikes: inside a workflow an agent returns a request, elsewhere it dispatches the spike agent. Name the spike agent and the spike skill.`,

  hooks: `Read as well: rewrite/reference/old-hook-runner-map.md in full; section 5 of rewrite/reference/claude-code-plugin-platform-facts.md; the proven runner in rewrite/spike/plugin/hooks/; and the old bash hooks that carry real rules, under plugins/vsdd-factory/hooks/: destructive-command-guard.sh, protect-secrets.sh, red-gate.sh, factory-branch-guard.sh, verify-git-push.sh, brownfield-discipline.sh, protect-bc.sh, validate-wave-gate-prerequisite.sh, and the validate-*.sh family.

Design all of the following.
- The runner: the entry command and its guard, how the Python is laid out inside the plugin, the interpreter floor, standard library only, how hooks.json wires events and matchers so the runner starts only when it has something to check, and the time budget per call. Hook timeouts are in seconds.
- The rule model: what a rule is, what it receives, what it can return (allow, block with a reason, advise, add context), how rules are registered, and how a project turns a rule off or tunes it.
- Staying inert: how the runner decides that a project is not a factory project and does nothing.
- Failure behaviour: uv missing, a crash inside a rule, a timeout. Say which rules fail closed and why. No gate may switch off silently.
- The 1.0 rule list. For each rule: name, event, what it checks, whether it blocks or advises, and which old hooks it replaces.
- A mapping of all 79 old registry entries: ported, merged, replaced by a workflow gate, or dropped with the reason. Hooks that only police the old bookkeeping formats are dropped.
- Rules the other subsystems need: information walls keyed on the calling agent, spike limits and the exemption for work done inside a spike, the session briefing, the pre-compaction flush, and tests-before-code.
- The event log the runner writes, for the observability stack and for explaining a block to the person who hit it.
- How the rules are tested (pytest with recorded hook payloads), and the old defects listed in STATE.md, so they are not repeated.`,

  packaging: `Read as well: rewrite/reference/old-workflows-packaging-map.md and rewrite/reference/claude-code-plugin-platform-facts.md in full; RELEASING.md; the files under .github/workflows/; docs/guide/plugin-marketplace-architecture.md; the file lists of plugins/vsdd-factory/bin/ and plugins/vsdd-factory/tools/observability/; Cargo.toml; and the "Also observed" notes in rewrite/STATE.md about what claude plugin validate asks for.

Design all of the following.
- The repository layout during the rewrite and after cutover: where the new plugin lives, what is deleted at cutover (the Rust workspace, the bundled binaries, the bats suite) and what is kept.
- plugin.json: every field, including the minimum Claude Code version, user configuration for API keys, and the network hosts declaration.
- The build step, if there is one: stamping the shared workflow helper into each workflow script (scripts cannot import), and the check that fails when the copies drift.
- The test strategy: unit tests for the hook runner, a way to run workflow scripts with stubbed agents, lint for agent and skill files, and an end-to-end run of the plugin against a sample project using headless claude -p --plugin-dir, which the spike proved works.
- Continuous integration: the jobs that replace the cargo and bats pipeline.
- Release: the procedure with no binaries and no bot commit, the marketplace update in drbothen/claude-mp, and the version, 1.0.0.
- Cutover for the eight projects on this machine that have the old release candidate installed.
- The supporting tools: each of the 21 old bin tools and the observability stack, marked ported to Python, replaced, or dropped with the reason. The user decided everything ports.
- The documentation set for 1.0, and the replacement for the repository CLAUDE.md.
- A build plan for the next step: work packages in dependency order, each small enough for one builder and one reviewer, with what each depends on.`,
}

const DESIGN_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    summary: { type: 'string' },
    interfaces_provided: { type: 'array', items: { type: 'string' } },
    interfaces_needed: { type: 'array', items: { type: 'string' } },
    open_questions: {
      type: 'array',
      items: {
        type: 'object',
        properties: { question: { type: 'string' }, recommendation: { type: 'string' } },
        required: ['question', 'recommendation'],
      },
    },
    risks: { type: 'array', items: { type: 'string' } },
  },
  required: ['path', 'summary', 'interfaces_provided', 'interfaces_needed', 'open_questions', 'risks'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict_summary: { type: 'string' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          documents: { type: 'array', items: { type: 'string' } },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'documents', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict_summary', 'findings'],
}

const BLUEPRINT_SCHEMA = {
  type: 'object',
  properties: {
    blueprint_path: { type: 'string' },
    summary: { type: 'string' },
    decisions_for_user: {
      type: 'array',
      items: {
        type: 'object',
        properties: { question: { type: 'string' }, recommendation: { type: 'string' } },
        required: ['question', 'recommendation'],
      },
    },
    findings_fixed: { type: 'number' },
    findings_deferred_to_user: { type: 'array', items: { type: 'string' } },
    findings_rejected: { type: 'array', items: { type: 'string' } },
    build_plan: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          package: { type: 'string' },
          depends_on: { type: 'array', items: { type: 'string' } },
          summary: { type: 'string' },
        },
        required: ['package', 'depends_on', 'summary'],
      },
    },
    not_yet_proven: { type: 'array', items: { type: 'string' } },
  },
  required: [
    'blueprint_path',
    'summary',
    'decisions_for_user',
    'findings_fixed',
    'findings_deferred_to_user',
    'findings_rejected',
    'build_plan',
    'not_yet_proven',
  ],
}

// ---------------------------------------------------------------- Design
phase('Design')
const designs = await parallel(
  AREAS.map((area) => () =>
    agent(`${COMMON(area)}\n\n${BRIEFS[area.key]}\n\n${OUTPUT(area)}`, {
      label: `design:${area.key}`,
      phase: 'Design',
      model: 'fable',
      effort: 'xhigh',
      schema: DESIGN_SCHEMA,
    }),
  ),
)
const missingDesigns = AREAS.filter((_, i) => !designs[i]).map((a) => a.key)
if (missingDesigns.length) log(`Designs that did not return: ${missingDesigns.join(', ')}`)
log(`Design phase done: ${AREAS.length - missingDesigns.length} of ${AREAS.length} designs written`)

// ---------------------------------------------------------------- Review
const DESIGN_FILES = AREAS.map((a) => a.file).join(', ')

const REVIEW_COMMON = `You are an independent reviewer of the design for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Read rewrite/STATE.md in full first: it holds the user's decisions, the platform facts verified by experiment, and the requirements the design must meet. Then read every design document: ${DESIGN_FILES}. ${missingDesigns.length ? `These designs were not produced and are themselves a finding: ${missingDesigns.join(', ')}. ` : ''}Five designers wrote them in parallel without seeing each other's work.

The repository CLAUDE.md that was loaded into your context, the old plugin under plugins/vsdd-factory/, and the docs all describe the system being replaced. Their process rules are not requirements of the new design. Surveys of the old system are in rewrite/reference/.

Your job is to find real problems, not to summarise the designs or praise them. Every finding must name the documents and sections involved, say what breaks if it is left alone, and propose a resolution. Do not report style preferences or wording. If you looked at an area and found nothing wrong, say so in one line in your report. Give each finding an id with your prefix and a number.

Severity: "blocker" means the design cannot be built or would not work as written; "major" means it would work but breaks a user decision or a stated requirement, or leaves a real gap; "minor" is anything else worth fixing.`

const LENSES = [
  {
    key: 'seams',
    prefix: 'SEAM',
    file: 'rewrite/design/review-seams.md',
    brief: `YOUR LENS: the seams between the documents. Look for a name, path, schema, identifier or responsibility that differs between two designs; an interface one design needs that no other design provides; and jobs that two designs both claim or both disclaim. Check every item under "Interfaces I need" in each document against "Interfaces I provide" in the others, one by one. Check that the shared result schemas, the agent names, the skill names, the workflow names, the state paths and the configuration keys are used identically everywhere.`,
  },
  {
    key: 'coverage',
    prefix: 'COV',
    file: 'rewrite/design/review-coverage.md',
    brief: `YOUR LENS: coverage and platform truth.
(a) Coverage. The user decided that every capability of the old system ports. Using rewrite/reference/old-agents-inventory.md, old-skills-inventory.md, old-hook-runner-map.md and old-workflows-packaging-map.md, find capabilities that no design carries forward and that were not explicitly dropped with a reason. Check the mapping tables in each design for completeness, and sample them for correctness against the old files.
(b) Platform truth. Check each design against the "Verified platform facts" and the spike results in rewrite/STATE.md. Find anything that relies on behaviour the platform does not have or that has not been verified. Examples: a workflow script that reads a file, calls the shell or imports a module; a workflow nested two levels deep; an agent inside a workflow that spawns another agent; user input in the middle of a workflow; resuming a workflow run from a new session; a plugin agent that declares its own hooks, MCP servers or permission mode; a hook timeout given in milliseconds.`,
  },
  {
    key: 'operations',
    prefix: 'OPS',
    file: 'rewrite/design/review-operations.md',
    brief: `YOUR LENS: how this behaves in use. Walk through each of these and report where the design fails or is silent:
- a session dies in the middle of a wave and a new session starts cold;
- the context compacts during story delivery;
- uv is not installed;
- a hook rule throws an exception;
- an agent returns a result that fails its schema, or returns nothing;
- a convergence loop never produces a clean pass;
- an agent asks for spike after spike;
- two sessions work on the same project at once.
Also judge: whether each human stop is necessary, and whether a needed one is missing; how many agents a typical story and a typical wave start, and what that costs given that every agent start has a fixed cost of roughly fifteen cents before it does anything; whether any part is rebuilding the bookkeeping that sank the old system; whether any gate can switch off without anyone noticing; and whether secrets or destructive commands can get past the gates.`,
  },
]

phase('Review')
const reviews = await parallel(
  LENSES.map((lens) => () =>
    agent(
      `${REVIEW_COMMON}\n\n${lens.brief}\n\nOUTPUT\n\nWrite your report to ${lens.file} with the Write tool: a one-paragraph verdict, then one section per finding (id starting ${lens.prefix}-, severity, documents and sections, the problem, what breaks, the proposed resolution), then a short list of areas you checked and found sound. Do not modify any other file. Do not commit, branch, or push. Then return the structured result with every finding in it.`,
      {
        label: `review:${lens.key}`,
        phase: 'Review',
        model: 'opus',
        effort: 'xhigh',
        schema: REVIEW_SCHEMA,
      },
    ),
  ),
)
const allFindings = reviews.filter(Boolean).flatMap((r) => r.findings)
const missingReviews = LENSES.filter((_, i) => !reviews[i]).map((l) => l.key)
if (missingReviews.length) log(`Reviews that did not return: ${missingReviews.join(', ')}`)
log(
  `Review phase done: ${allFindings.length} findings (${allFindings.filter((f) => f.severity === 'blocker').length} blocker, ${allFindings.filter((f) => f.severity === 'major').length} major, ${allFindings.filter((f) => f.severity === 'minor').length} minor)`,
)

// ---------------------------------------------------------------- Synthesize
phase('Synthesize')
const openQuestions = designs
  .filter(Boolean)
  .flatMap((d, i) => d.open_questions.map((q) => ({ from: AREAS[i].key, ...q })))

const blueprint = await agent(
  `You are writing the blueprint for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

Read rewrite/STATE.md in full first: it holds the user's decisions, the verified platform facts, and the requirements. Then read the five design documents (${DESIGN_FILES}) and the three review reports (${LENSES.map((l) => l.file).join(', ')}). ${missingDesigns.length ? `These designs were not produced: ${missingDesigns.join(', ')}; say so prominently in the blueprint. ` : ''}${missingReviews.length ? `These reviews were not produced: ${missingReviews.join(', ')}; say so prominently in the blueprint. ` : ''}The repository CLAUDE.md that was loaded into your context and the old plugin describe the system being replaced; their process rules are not requirements.

The designers worked in parallel without seeing each other's work, and the reviewers found ${allFindings.length} problems. Your job is to turn this into one coherent design that the user can approve and builders can work from.

The reviewers' findings, in full:
${JSON.stringify(allFindings)}

The designers' open questions:
${JSON.stringify(openQuestions)}

Do this in order.

1. Decide every review finding. Each one is fixed (say how), deferred to the user (because only a person can decide), or rejected (say why it is wrong). No finding may be dropped. When two designs conflict, choose by the design goals in this order: correct and enforceable; low touch; restartable; lean. User decisions in STATE.md are fixed.

2. Write rewrite/BLUEPRINT.md with the Write tool. It is the single document the user approves. The user has not read the design documents, so it must stand on its own: complete sentences, terms defined, tables for rosters and mappings. Sections, in this order:
   1. Summary: what 1.0 is, in under 300 words.
   2. Decisions for you: each question a person must answer before or during the build, with your recommendation and what the answer changes. Include the designers' open questions that survive and the findings you deferred. Put the ones that block the build first.
   3. Architecture at a glance: the parts, and how one story travels from a request to a merged change.
   4. Shared conventions: the names, paths, schemas, identifiers and model tiers that settle every conflict between the documents. This section is authoritative.
   5. The subsystems: for each of the five, a summary of the final design, the amendments you made to it, and a pointer to its design document.
   6. Research spikes: how a person, an agent, a skill and a workflow each start one, and what bounds them.
   7. Compaction, warm-up and cold start: what happens in each case.
   8. Review findings and their resolution: a table of id, severity, and what was done.
   9. Build plan: work packages in dependency order, each small enough for one builder and one reviewer, with dependencies.
   10. Not yet proven: anything the design relies on that has not been tested, and how to test it.

3. Apply your resolutions to the design documents with the Edit tool, so that the detail documents agree with the blueprint. Add one line at the top of each amended design document saying it was amended by the blueprint.

Do not modify anything other than rewrite/BLUEPRINT.md and the files in rewrite/design/. Do not commit, branch, or push. Then return the structured result; keep its summary under 200 words.`,
  {
    label: 'synthesize:blueprint',
    phase: 'Synthesize',
    model: 'fable',
    effort: 'xhigh',
    schema: BLUEPRINT_SCHEMA,
  },
)

return {
  designs: designs.map((d, i) => (d ? { area: AREAS[i].key, path: d.path, summary: d.summary, open_questions: d.open_questions.length, risks: d.risks } : { area: AREAS[i].key, failed: true })),
  reviews: reviews.map((r, i) => (r ? { lens: LENSES[i].key, path: r.path, verdict: r.verdict_summary, findings: r.findings.length } : { lens: LENSES[i].key, failed: true })),
  findings: allFindings.map((f) => ({ id: f.id, severity: f.severity, problem: f.problem.slice(0, 240) })),
  blueprint,
}
