export const meta = {
  name: 'vsdd-1-0-slice-step-5-plan',
  description: 'Plan step 5 of the thin slice: the agent files and method files the story workflow needs, reviewed against the story script and corrected',
  whenToUse: 'Before the step 5 build. Reads rewrite/spec/, rewrite/findings/ and next/; writes rewrite/plans/step-5/ only.',
  phases: [
    { title: 'Plan', detail: 'one planner for all the agents the story script names, so they share one frame', model: 'fable' },
    { title: 'Review', detail: 'one reviewer checks the plan against the story script, the walls and the lint', model: 'opus' },
    { title: 'Integrate', detail: 'the plan is corrected', model: 'fable' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const CONTEXT = `Your working directory is the repository at ${ROOT}, on branch rewrite/v1. This is vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. The repository CLAUDE.md in your context is the rewrite-mode one; follow it.

Where things are:
- The design is rewrite/spec/, one small file per thing. Read rewrite/spec/README.md first. rewrite/spec/registry.toml and rewrite/spec/registry.d/ list every thing and the build package that delivers it. Each build package has a brief, rewrite/spec/packages/<ID>.md. rewrite/spec/conventions.md gives names, paths and model tiers. rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md give the platform behaviour proven by experiment.
- rewrite/spec/packaging/build-plan.md has a section "The thin slice". The first milestone is one story delivered on the sample project through the real platform, in local delivery mode. Step 5 of that list is the agents the story script names, each as a complete agent file with the methods the story script gives it and no others yet: the clerk (package A1); the test-writer, implementer and verifier (A4); the adversary and security-reviewer (A6a); the architect with its arbitrate method (A2b); the demo-recorder, technical-writer and devops (A5).
- The story script is specified in rewrite/spec/topics/story-script.md and rewrite/spec/workflows/story.md, with the shared functions in the prelude topics (rewrite/spec/topics/prelude-*.md). The code for the prelude is next/workflow-lib/prelude.js. Each agent is in rewrite/spec/agents/<name>.md; the agent file template, the shared block and the method files are described in the topics and tables of the agents group (rewrite/spec/topics/, rewrite/spec/tables/methods.toml, rewrite/spec/tables/roster.toml or the shipped next/tests/lint/roster.toml).
- The spec was converted from six larger documents kept in rewrite/history/design-v1/ (agents-skills.md is the one for this subject). Read the original section when a spec file is silent or contradicts itself.
- Working code is under next/. Another workflow is changing it right now: it is building the policy tables (package C2: the read walls, write scopes and path classes) and the roster and agent lint (package B6). The agent files must satisfy that lint and those walls, so plan against what their briefs and spec files say they will be, and say where the plan depends on them. A third workflow is planning the state core, records, survey, hook runner and jobs in rewrite/plans/step-3/; if those plans exist when you read this, use the command names and outputs they settle.
- Known problems with the design are in a ledger: rewrite/findings/by-package/<ID>.md lists the ones that concern a package's files, each with a status.

Facts the platform probes established that shape an agent file: an agent with an explicit tool list starts with about 3,000 tokens of context and one without starts with about 19,000, so every agent carries an explicit tool list; an agent running as a workflow step has no tool for starting other agents even if its file lists one; worktree isolation written in an agent file is ignored for a workflow step; a plugin agent cannot set its own hooks, MCP servers or permission mode; the workflow sets model and effort on each step, so the file's values are the default for a direct dispatch.

Models for the build that follows: a Sonnet builder writes the files from the plan, an Opus reviewer checks them. Do not write or change anything under next/, rewrite/spec/, rewrite/findings/, rewrite/history/ or rewrite/tools/. Do not commit, branch or push.`

const PLAN_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    summary: { type: 'string' },
    agents: {
      type: 'array',
      items: {
        type: 'object',
        properties: { name: { type: 'string' }, tier: { type: 'string' }, tools: { type: 'string' }, methods: { type: 'array', items: { type: 'string' } }, used_by_steps: { type: 'array', items: { type: 'string' } } },
        required: ['name', 'tier', 'tools', 'methods', 'used_by_steps'],
      },
    },
    files: { type: 'array', items: { type: 'string' } },
    design_decisions: { type: 'array', items: { type: 'string' } },
    depends_on: { type: 'array', items: { type: 'string' } },
    findings_planned: { type: 'number' },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
  required: ['path', 'summary', 'agents', 'files', 'design_decisions', 'depends_on', 'findings_planned', 'open_questions'],
}

phase('Plan')
const plan = await agent(
  `You are planning the agent files for the first milestone of a build. ${CONTEXT}

YOUR TASK: plan step 5. Read the story script and its prelude first, and list every step it runs: the agent type, the method it names, what the step is given, and the result schema it must return. Then read the spec file for each of the ten agents, the agent template and shared block, the methods table, and the by-package findings for A1, A2b, A4, A5 and A6a.

Write the plan to rewrite/plans/step-5/agents.md with the Write tool, under 24 KB, with these sections:
1. Scope: the ten agents and exactly which methods each gets now. Everything else waits for the agent's own package.
2. The frame: the agent file template as it will be used (frontmatter keys and the body sections, in order), what is stamped in from the shared block by the build step, and the rules every file must meet (length limit, explicit tool list, no tool for starting agents unless the design gives one for direct dispatch, how the agent learns its method and its closing command). Take this from the spec; decide only what the spec leaves open.
3. One entry per agent: file path under next/plugins/vsdd-factory/; frontmatter values (name, description, model, effort, tools); what it is given; the result it returns, by schema name; its read walls and write scope, by reference to the policy tables; the methods it has now.
4. One entry per method file: path; the agent that owns it; the story step that names it; what the procedure must tell the agent to do, as a short ordered list; the command it ends with and the result fields it fills. Say plainly what a method must not do (for example, an implementer must not edit a test).
5. Design decisions: each place the spec was silent, ambiguous or contradicted itself for something this step must build, with what you decided and why. Goals in order: correct and enforceable; low touch; restartable; lean.
6. Findings: every finding in those by-package lists, with the change that resolves it or the package that must take it later.
7. Tests: what proves these files are right before a real run: the lint checks they must pass, and the harness tests that show each story step names an agent and a method that exist and that the result schema matches.
8. What must exist before the builder starts, and what the builder must not touch.

Return the structured result.`,
  { label: 'plan:agents', phase: 'Plan', model: 'fable', effort: 'xhigh', schema: PLAN_SCHEMA },
)
if (!plan) {
  log('The planner did not return')
  return { failed: 'plan' }
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    path: { type: 'string' },
    verdict: { type: 'string' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
          problem: { type: 'string' },
          proposed_resolution: { type: 'string' },
        },
        required: ['id', 'severity', 'problem', 'proposed_resolution'],
      },
    },
  },
  required: ['path', 'verdict', 'findings'],
}

phase('Review')
const review = await agent(
  `You are reviewing a build plan before any file is written from it. ${CONTEXT}

A planner has written rewrite/plans/step-5/agents.md: the agent files and method files the story workflow needs. Here is its own summary of what it planned:
${JSON.stringify({ agents: plan.agents, design_decisions: plan.design_decisions, depends_on: plan.depends_on, open_questions: plan.open_questions })}

Read the plan in full, then check it against the things it has to fit:
- The story script (rewrite/spec/topics/story-script.md, rewrite/spec/workflows/story.md, the prelude topics and next/workflow-lib/prelude.js). Every step the script runs must have an agent and a method in the plan, with the same names; what the step passes must be what the method expects; the result schema must be the one the script reads. Walk the script from the first step to delivery, and through its repair and review loops.
- The policy tables (rewrite/spec/tables/walls.toml and the path classes, or the shipped copies under next/plugins/vsdd-factory/ if they exist yet). Each agent's tool list, read walls and write scope in the plan must agree with the tables. An agent must be able to do what its method tells it to do, and nothing its role forbids: for example the implementer cannot edit tests, a reviewer cannot write product code, a clerk only runs the one command.
- The roster and the agent lint (next/tests/lint/roster.toml and the lint spec in rewrite/spec/tables/lint-checks.toml). Tiers, tool lists and names must match.
- The proven platform facts in the context above.
- The spec file for each agent, and the original agents-skills.md where the spec is in doubt.
Also look for: a method that ends without a closing command or leaves a required result field unfilled; an agent that would have to judge whether its own work passed; a step that could fail silently; anything built beyond what one story needs.

Write your report to rewrite/plans/step-5/REVIEW.md with the Write tool. Do not modify any other file. Return the structured result. Severity: "blocker" means a story could not run from these files; "major" means a real mismatch that would cost a rebuild or weaken an enforcement; "minor" is anything else. Give each finding an id (PL5-1, ...).`,
  { label: 'review:plan', phase: 'Review', model: 'opus', effort: 'xhigh', schema: REVIEW_SCHEMA },
)

const INTEGRATE_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    resolutions: {
      type: 'array',
      items: {
        type: 'object',
        properties: { id: { type: 'string' }, outcome: { type: 'string', enum: ['fixed', 'rejected', 'needs-a-person'] }, what_was_done: { type: 'string' } },
        required: ['id', 'outcome', 'what_was_done'],
      },
    },
    files_to_build: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'resolutions', 'files_to_build'],
}

phase('Integrate')
let integrated = null
if (review && review.findings.length) {
  integrated = await agent(
    `You are correcting a build plan. ${CONTEXT}

The plan is rewrite/plans/step-5/agents.md. A reviewer checked it against the story script, the policy tables, the roster and lint, and the platform facts, and wrote rewrite/plans/step-5/REVIEW.md. Its findings:
${JSON.stringify(review.findings)}

Decide every finding and correct the plan with the Edit tool. Keep it under 24 KB and in its eight sections. If the step 3 plans now exist in rewrite/plans/step-3/, make the command names and outputs in this plan agree with them. You may edit only rewrite/plans/step-5/agents.md. Return the structured result with one resolution per finding, and the final list of files the builder will write.`,
    { label: 'integrate', phase: 'Integrate', model: 'fable', effort: 'xhigh', schema: INTEGRATE_SCHEMA },
  )
}

return {
  plan: { path: plan.path, summary: plan.summary, agents: plan.agents.map((a) => `${a.name} (${a.tier}): ${a.methods.join(', ')}`), files: plan.files.length, findings_planned: plan.findings_planned, open_questions: plan.open_questions },
  review: review ? { verdict: review.verdict, findings: review.findings.length, blockers: review.findings.filter((f) => f.severity === 'blocker').length, majors: review.findings.filter((f) => f.severity === 'major').length } : null,
  integrated,
}
