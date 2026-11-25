export const meta = {
  name: 'vsdd-1-0-slice-probes',
  description: 'Platform probes the thin slice depends on (build package B8): test the unproven platform behaviour with a throwaway plugin and record the results',
  whenToUse: 'Alongside slice layer 2, before the hook runner core, the lock and the story workflow are built.',
  phases: [
    { title: 'Probe', detail: 'run each probe headless and record what the platform does', model: 'sonnet' },
    { title: 'Check', detail: 'an independent check of the evidence behind each result', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const PROBE_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    results: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          item: { type: 'string' },
          question: { type: 'string' },
          result: { type: 'string', enum: ['proven', 'disproven', 'partly', 'not-testable-headless', 'not-run'] },
          evidence: { type: 'string' },
          consequence_for_the_design: { type: 'string' },
        },
        required: ['item', 'question', 'result', 'evidence', 'consequence_for_the_design'],
      },
    },
    evidence_dir: { type: 'string' },
    spec_files_changed: { type: 'array', items: { type: 'string' } },
    cost_note: { type: 'string' },
  },
  required: ['summary', 'results', 'evidence_dir', 'spec_files_changed', 'cost_note'],
}

phase('Probe')
const probes = await agent(
  `You are running platform probes for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The repository CLAUDE.md in your context is the rewrite-mode one; follow it.

Background. The design depends on how Claude Code behaves, and some of that behaviour has never been observed. rewrite/spec/platform.md lists each platform behaviour the design relies on, numbered, with whether it is proven, how to test it, and what the design does if it fails. An earlier round of probes settled items 1, 2, 4, 7, 8, 11, 14, 15 and 17, and partly settled 3, 5 and 6; rewrite/STATE.md has those results in its tables. The build package for the rest is B8: read rewrite/spec/packages/B8.md and the section "The thin slice" in rewrite/spec/packaging/build-plan.md, which says which probe items the slice needs before its hook runner, lock and story workflow are built. Ledger finding P3COV-03 (in rewrite/findings/ledger.json) names them: items 27 to 29 (how a hook or the tool can tell which agent or session is calling: the command rewrite, the caller marker, the working directory of a step that is not isolated), and 31 and 34, among others. One more arose during the last design fix: whether a hook's updatedInput is honoured on a StructuredOutput call.

How probes are run. rewrite/spike/plugin/ is a throwaway plugin that was used for the earlier probes: agents, skills, workflows, a hooks.json, a shell guard and a Python hook runner that logs what each event carries (set FSPIKE_LOG to a file path). The method that worked: copy that plugin to a scratch directory, add what a probe needs (a hook rule in runner.py, an agent file, a small workflow script), create a scratch git project, and run Claude Code headless against it:
  cd <scratch project> && FSPIKE_LOG=<file> claude -p "<prompt>" --plugin-dir <scratch plugin> --model sonnet --allowedTools "Workflow" "Agent" "Bash(<command>:*)" "Read" --output-format json < /dev/null > <result.json>
Then read the result JSON and the hook log. Use --permission-mode default when a probe is about permissions. Give each agent in a probe an explicit tool list and low effort so it costs little, and ask probe agents for short structured answers. Use a scratch directory outside the repository for the plugin copy, the project and the logs (make one with mktemp -d), and keep the evidence: when you finish, copy the probe files you added and the logs into rewrite/spike/probes-round-3/ so the results can be checked.

YOUR TASK.
1. Read rewrite/spec/platform.md in full and list the items that are not yet proven. Decide which can be settled by a headless probe. Do every one the thin slice depends on first, then as many of the others as you can. Skip items that need an interactive person, an MCP server that is not installed, the marketplace, CI, or the finished product, and say so for each.
2. For each probe: state the question exactly, run it, and record what you saw. A result must rest on something a command printed: the hook log line, the result JSON, a file that did or did not appear. If a probe is inconclusive, say so; do not infer.
3. Update rewrite/spec/platform.md: for each item you settled, record the result and, where the behaviour is not what the design assumed, what the design's stated fallback is. Keep the file under 16 KB and lines under 400 characters; if it would pass 16 KB, move the settled items into a new file rewrite/spec/topics/platform-proven.md, register it by adding a [topic.platform-proven] entry (summary, group = "index", sources = ["BLUEPRINT.md#10"]) to a new file rewrite/spec/registry.d/probes.toml, and link it. Then run python3 rewrite/tools/spec_lint.py and fix what it reports.
4. For every item where the platform does not behave as the design assumed, add an entry to rewrite/findings/sources/extra-probes.json (a JSON array): {"id": "PR3-<n>", "origin": "platform probes, round 3", "severity": "blocker" | "major" | "minor", "where": "the spec files that rely on the behaviour", "problem": "...", "proposed_resolution": "the fallback the design names, or what must change", "on_slice_path": true or false}. Then run python3 rewrite/tools/findings.py build.

Do not edit anything under next/, plugins/, crates/, rewrite/history/ or rewrite/tools/. Do not commit, branch or push. Return the structured result, one entry per platform item you considered, including those you skipped.`,
  { label: 'probes', phase: 'Probe', model: 'sonnet', effort: 'xhigh', schema: PROBE_SCHEMA },
)

const CHECK_SCHEMA = {
  type: 'object',
  properties: {
    verdict: { type: 'string' },
    checked: { type: 'number' },
    supported: { type: 'number' },
    problems: {
      type: 'array',
      items: {
        type: 'object',
        properties: { item: { type: 'string' }, problem: { type: 'string' } },
        required: ['item', 'problem'],
      },
    },
  },
  required: ['verdict', 'checked', 'supported', 'problems'],
}

phase('Check')
let check = null
if (probes) {
  check = await agent(
    `You are checking the evidence behind a set of platform probe results for vsdd-factory 1.0. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

An agent has just probed how Claude Code behaves and recorded the results in rewrite/spec/platform.md (and possibly rewrite/spec/topics/platform-proven.md) and in rewrite/findings/sources/extra-probes.json. Its evidence, meaning the probe files and the logs, should be in rewrite/spike/probes-round-3/. Here is what it reports:
${JSON.stringify(probes.results)}

For each result marked proven, disproven or partly, open the evidence and decide whether the result follows from what was actually printed. A result is supported only if the log or result file shows the behaviour directly. Look for: a conclusion drawn from what an agent said about itself when a log line would have shown it; a control that was not run (for a permission probe, the case that should be refused); a probe that tested something easier than the question; and a result that the recorded evidence contradicts. You may rerun one or two of the cheapest probes if the evidence is unclear, using the same method (a scratch copy outside the repository, headless claude -p with --plugin-dir and low-effort agents). Check that rewrite/spec/platform.md says what the evidence supports, no more.

Do not modify any file except rewrite/spike/probes-round-3/CHECK.md, where you write your report. Do not commit, branch or push. Return the structured result: checked is the number of results you examined and supported is the number the evidence supports.`,
    { label: 'check', phase: 'Check', model: 'opus', effort: 'xhigh', schema: CHECK_SCHEMA },
  )
}

return { probes, check }
