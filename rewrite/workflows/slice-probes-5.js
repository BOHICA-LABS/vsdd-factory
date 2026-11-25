export const meta = {
  name: 'vsdd-1-0-slice-probes-5',
  description: 'Platform probes, round 5: what the first headless story run rests on and nobody has observed (plugin loading under the isolation flags, a typed workflow command with no skill in the default mode, bare tool names, step effort in a hook payload, a budget stop mid-workflow, the session folder)',
  whenToUse: 'After the step 6 plans are corrected and before the first story run. Writes rewrite/spike/probes-round-5/, the platform files of rewrite/spec/ and one ledger source file. Needs no step 6 code.',
  phases: [
    { title: 'Probe', detail: 'run each leg headless and record what the platform does', model: 'sonnet' },
    { title: 'Check', detail: 'an independent check of the evidence behind each result', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const LEGS = `The legs are specified in rewrite/plans/step-6/ORDER.md, section 8.2 ("Probes before the run"): read that section first; its table is the authority for what each leg runs, what counts as a pass, and what follows from each answer. In short:

Leg A (one headless session, the gate). A scratch copy of the round-4 probe plugin (rewrite/spike/probes-round-4/plugin, with its logging hook) gains a bin/ tool that prints a marker, a methods/probe.md that holds another marker, an agent "worker" with the tools Bash, Read, Write, Edit, Glob, Grep, and a one-step workflow "one" whose step is typed (agentType the plugin's worker, model sonnet, effort low). The scratch project is a git repository with one commit, a linked worktree .worktrees/x on a branch story/x, and no settings file. The session is started by the typed workflow command itself, with no skill, with exactly these flags:
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 <the logger's variable>=<probe>/logs/<leg>.jsonl claude -p "/<plugin>:<workflow> <absolute plugin directory>" --plugin-dir <probe>/plugin --permission-mode default --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" --settings '{"sandbox":{"enabled":false}}' --setting-sources project,local --strict-mcp-config --model sonnet --effort medium --max-budget-usd 1 --output-format json < /dev/null > <result.json>
  A1. Do the plugin's hooks, workflows and bin/ load under --setting-sources project,local --strict-mcp-config? (Every earlier probe session had the user's settings loaded.) Pass: the hook log holds the step's payloads; a workflow run record exists and ended completed; the step returned the bin/ tool's marker. If it fails, repeat without the two isolation flags and record both.
  A2. Does the typed workflow command work with no skill in the default permission mode? Pass: num_turns at least 1; the run record's script equals the plugin's workflow file; no denial. If the model reaches for the Skill tool and is refused, repeat with Skill added to the allowed tools and record both. If it still fails, stop that leg and say so plainly.
  A3. With bare tool names allowed, can the step Read <plugin>/methods/probe.md (outside the working directory), Edit one file and Write another in .worktrees/x, and commit there with plain Bash? Pass: permission_denials is empty; the marker came back; the commit is on story/x with both files.
  A4. Does the step's hook payload carry the step's effort (low) or the session's (medium)? Record effort.level of every payload of the step.
Leg B (a second session, a few cents). A three-step workflow "three" under --max-budget-usd 0.05, otherwise the same flags. Record: the exit code; whether a result is written, and its subtype, is_error and num_turns; the workflow run record's status; what the step that was running left behind.
Leg C (no session). Describe the platform's session folder from leg A's session and from the kept round-4 folder that ORDER.md names: workflows/wf_*.json with script, scriptPath, status, result, agentCount and workflowProgress (per agent: agentId, agentType, the full model id, state); subagents/workflows/<wf>/agent-*.jsonl with tool_use entries. For a call by name, do script and the file at scriptPath equal the plugin's file byte for byte? Say exactly where the folder is found from a session id and a project directory.
Leg D (recommended, do it last, about 30 cents). The platform record (items 36 to 38 and 40 of rewrite/spec/topics/platform-proven-2.md) was made on Claude Code 2.1.331 and this machine now has a newer version. Run rerun A of the round-4 check again (rewrite/spike/probes-round-4/check-reruns/ has it) and say whether each of those four items still holds, with the version.`

const PROBE_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    claude_version: { type: 'string' },
    scratch_dir: { type: 'string' },
    results: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          leg: { type: 'string' },
          question: { type: 'string' },
          result: { type: 'string', enum: ['pass', 'fail', 'partly', 'recorded', 'not-run'] },
          evidence: { type: 'string' },
          what_follows: { type: 'string' },
        },
        required: ['leg', 'question', 'result', 'evidence', 'what_follows'],
      },
    },
    evidence_dir: { type: 'string' },
    spec_files_changed: { type: 'array', items: { type: 'string' } },
    cost_note: { type: 'string' },
  },
  required: ['summary', 'claude_version', 'scratch_dir', 'results', 'evidence_dir', 'spec_files_changed', 'cost_note'],
}

phase('Probe')
const probes = await agent(
  `You are running platform probes for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Background. The design depends on how Claude Code behaves. rewrite/spec/platform.md, rewrite/spec/topics/platform-proven.md and rewrite/spec/topics/platform-proven-2.md list each platform behaviour the design relies on, numbered, with whether it is proven. Four rounds of probes have been run; the evidence and method of the last are in rewrite/spike/probes-round-4/ (read COMMANDS.md, RESULTS.md and CHECK.md first: they show what worked and what the checker faulted; the prober of that round misread five of eleven results, each time by concluding more than a log line showed). The thin slice ends with one story delivered by a headless session (rewrite/plans/step-6/first-run.md describes the driver; rewrite/plans/step-6/story-workflow.md the workflow). That first run rests on the behaviours below, which nobody has observed with these flags. Its code is not built yet and you need none of it.

THE LEGS
${LEGS}

How to run them. Make the scratch directory under the home directory, not under a temporary directory (mkdir -p "$HOME/.vsdd-probes" && mktemp -d "$HOME/.vsdd-probes/round-5.XXXXXX"), because a project under Claude Code's own temporary tree is treated differently (a project CLAUDE.md there is not loaded) and the first run's project will live under the home directory. If your own shell refuses to write there, say so in your result, use mktemp -d instead, and record that the legs ran under the temporary tree. Put the plugin copy, the project and the logs in that scratch directory; nothing of the probe goes under next/. Run each session from the scratch project's directory. The headless sessions need the Bash sandbox off, which the --settings flag above asks for; record whether it was honoured (a write under .git/worktrees/ that succeeds shows it). Put nothing about this user or this repository into a prompt you send: the workflow argument is the scratch plugin's own path and nothing else. Record claude --version.

A result must rest on something a command printed: a hook log line, the result JSON, a run record, a file that did or did not appear, git log. Do not conclude from what an agent says about itself when a log would show it. If a leg is inconclusive, say so; do not infer. For a failure, keep the whole result JSON and the log: the denied tool and input are the finding.

When you finish:
1. Copy the probe files you added, the exact commands you ran, the result files and the logs into rewrite/spike/probes-round-5/, one folder per leg, with a COMMANDS.md and a RESULTS.md, so that each result can be checked. Do not copy session transcripts that hold anything but the probe's own text.
2. Record each settled behaviour as a new numbered platform item, continuing the numbering of rewrite/spec/topics/platform-proven-2.md, saying what was observed, in which permission mode, with which flags and with which platform version. Keep every file under 16 KB and lines under 400 characters: if platform-proven-2.md would pass 16 KB, start rewrite/spec/topics/platform-proven-3.md, register it in rewrite/spec/registry.d/probes.toml the way platform-proven-2 is registered there, and link it from platform.md. platform.md is within a hundred bytes of its limit and platform-proven-2.md within three kilobytes, so new items go into the new file and platform.md gains nothing it does not also lose: shorten what the new items make redundant. Update the section of rewrite/spec/platform.md headed "Headless runs: what the first one (step 6) must set" with what these legs showed, and its list of items still open. Then run python3 rewrite/tools/spec_lint.py and python3 rewrite/tools/sections.py coverage and fix what they report in your files.
3. For every leg where the platform does not behave as the step 6 plans assume, add an entry to a new file rewrite/findings/sources/extra-probes-5.json (a JSON array; an empty array if there is none): {"id": "PR5-<n>", "origin": "platform probes, round 5", "severity": "blocker" | "major" | "minor", "where": "the plan section and spec files that rely on the behaviour", "problem": "what was assumed and what was observed", "proposed_resolution": "what must change, in which plan section", "on_slice_path": true}. Then run python3 rewrite/tools/findings.py build.

Do not edit anything under next/, plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Builders are working under next/ right now. Do not commit, branch, stash or push, and never use git in this repository to discard or restore files. Return the structured result, one entry per leg and sub-leg (A1, A2, A3, A4, B, C, D), including any you could not run.`,
  { label: 'probes', phase: 'Probe', model: 'sonnet', effort: 'xhigh', schema: PROBE_SCHEMA },
)

const CHECK_SCHEMA = {
  type: 'object',
  properties: {
    verdict: { type: 'string' },
    gate_passed: { type: 'boolean' },
    checked: { type: 'number' },
    supported: { type: 'number' },
    problems: {
      type: 'array',
      items: {
        type: 'object',
        properties: { leg: { type: 'string' }, problem: { type: 'string' } },
        required: ['leg', 'problem'],
      },
    },
    consequences: {
      type: 'array',
      items: {
        type: 'object',
        properties: { leg: { type: 'string' }, plan_section: { type: 'string' }, what_must_change: { type: 'string' } },
        required: ['leg', 'plan_section', 'what_must_change'],
      },
    },
    reruns: { type: 'array', items: { type: 'string' } },
  },
  required: ['verdict', 'gate_passed', 'checked', 'supported', 'problems', 'consequences', 'reruns'],
}

phase('Check')
let check = null
if (probes) {
  check = await agent(
    `You are checking the evidence behind a set of platform probe results for vsdd-factory 1.0. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

An agent has just probed how Claude Code behaves, to settle what the first headless story run rests on, and recorded the results in the platform files of rewrite/spec/ (platform.md, topics/platform-proven-2.md and possibly a platform-proven-3.md) and in rewrite/findings/sources/extra-probes-5.json. Its evidence, meaning the probe files, the commands, the result files and the logs, should be in rewrite/spike/probes-round-5/. Its scratch directory was ${probes.scratch_dir}. The legs it was given:
${LEGS}

What it reports (platform version ${probes.claude_version}):
${JSON.stringify(probes.results)}

For each result, open the evidence and decide whether the result follows from what was actually printed. A result is supported only if the log, the result file, the run record or git shows the behaviour directly. Look for: a conclusion drawn from what an agent said about itself when a log line would have shown it; a pass claimed with a flag missing from the command that was actually run (compare each recorded command with the flags of rewrite/plans/step-6/ORDER.md section 8.2, one by one: the permission mode, the allowed tools, the settings flag, the two isolation flags, the effort, the environment variable, the scratch location); a leg that tested something easier than the question (a skill where the question is the typed workflow command with no skill; the auto mode where the question is the default mode; a file inside the working directory where the question is one outside it; a main-session call where the question is a workflow step); and a result the recorded evidence contradicts. Rerun leg A yourself, the whole session with the exact flags, in a fresh scratch directory under the home directory (mkdir -p "$HOME/.vsdd-probes" && mktemp -d "$HOME/.vsdd-probes/round-5-check.XXXXXX"; if your shell cannot write there, say so and use mktemp -d), because the first run is made on its answers; rerun leg B only if its evidence leaves doubt. Put nothing about this user or this repository into a prompt you send. Check that the spec files say what the evidence supports and no more, and that each new platform item names the version, the mode and the flags.

Then say what follows for the first run: for each answer, name the section of rewrite/plans/step-6/first-run.md or story-workflow.md (and ORDER.md) that must change, and the exact change: the isolation default, the allowed-tools list, STEP_EFFORT_PROVEN, the wording of the budget-stop diagnosis, the shape of the session folder the checks read. If the answers agree with the plans, say so. Set gate_passed to true only if legs A1, A2 and A3 pass as specified or pass in the fallback form the table allows, and say which form.

Do not modify any file except rewrite/spike/probes-round-5/CHECK.md, where you write your report, and files in your own scratch directory. Do not commit, branch or push. Return the structured result: checked is the number of results you examined and supported is the number the evidence supports.`,
    { label: 'check', phase: 'Check', model: 'opus', effort: 'xhigh', schema: CHECK_SCHEMA },
  )
}

return { probes, check }
