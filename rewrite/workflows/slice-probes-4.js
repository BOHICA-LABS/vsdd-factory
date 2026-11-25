export const meta = {
  name: 'vsdd-1-0-slice-probes-4',
  description: 'Platform probes, round 4: the behaviours the step 3, 4 and 5 plans rely on and could not prove, tested with a throwaway plugin before the units that depend on them are built',
  whenToUse: 'After the step 4 plans are corrected and before step 4 stage A is built. Writes rewrite/spike/probes-round-4/, the platform files of rewrite/spec/ and one ledger source file.',
  phases: [
    { title: 'Probe', detail: 'run each probe headless and record what the platform does', model: 'sonnet' },
    { title: 'Check', detail: 'an independent check of the evidence behind each result', model: 'opus' },
  ],
}

const ROOT = '/Users/jmagady/dev2/vsdd-factory'

const QUESTIONS = `Q1. Who a hook sees on a workflow step's result. A plugin workflow script dispatches a step with agent(prompt, {agentType: '<plugin>:<agent>', schema: ...}). When that step calls StructuredOutput, what does the PreToolUse payload carry: is agent_type exactly '<plugin>:<agent>', something else (for example 'workflow-subagent'), or absent? Is agent_id present, and is it the same on the step's Bash calls? The design counts a review pass only when the stored agent_type equals 'vsdd-factory:<agent>' (rewrite/plans/step-4/verdicts.md), so this decides whether any delivery can pass. Record the whole payload.
Q2. Rewriting a step's result. Can a PreToolUse hook on StructuredOutput, through hookSpecificOutput.updatedInput, add a key the step's schema does not declare (the design adds "grounded": false inside one object of a findings array)? Does the script receive the added key, is it dropped silently, or is the call refused? Test also with the key declared as optional in the schema.
Q3. Approval with a rewrite outside the auto mode. Item 27 of rewrite/spec/topics/platform-proven.md proved permissionDecision allow together with updatedInput only in the auto permission mode. In the default permission mode, run headless: (a) a Bash call made by an agent inside a workflow step, with a hook that rewrites the command (prefixing an environment assignment) and answers allow; (b) the same on StructuredOutput. Is the rewritten input what runs? Run the control with no hook, to show the call would otherwise wait or be refused.
Q4. A rewrite with no decision. Is updatedInput honoured on a Bash call when the hook output carries no permissionDecision at all? In the default mode and in the auto mode.
Q5. Order of checks on a step's result. When a step returns a result that does not match its schema, does the PreToolUse hook on StructuredOutput run before the platform's own schema check (the hook sees the invalid input), or only after it passes? And can the hook refuse a StructuredOutput call (exit 2 with a message) so that the agent sees the message and answers again?
Q6. The sandbox and git. With the Bash sandbox switched on in the project's settings, can a Bash call inside a workflow step create a branch and a worktree (git worktree add .worktrees/x -b story/x), commit there, and write a ref with git update-ref? These write under .git/. Record exactly what is refused, if anything, and with which message.
Q7. A long foreground call. Does a foreground Bash call inside a workflow step, given an explicit timeout of 600000 ms, run a ten-minute command to its end (sleep 590; echo done)? Four minutes is proven. Run this one in the background of your own work while you do the others.
Q8. A detached job against the timeout. A step starts a double-forked, session-detached worker (python3: fork, setsid, fork; the grandchild sleeps and then writes a file) and then the same Bash call runs past its timeout so that the tool kills it. Does the worker survive and write its file? Repeat with the Bash sandbox on.
Q9. Finding state files. The project keeps state in .factory/, which is a git worktree of another branch and is listed in .gitignore. Can an agent inside a workflow step find a file there with Glob (pattern and path both tried: '.factory/specs/**/*.md' from the project root, and '**/*.md' with path '.factory/specs') and read it with Read? The design has a reviewer with no shell find a contract this way.
Q10. Git hooks in the integration worktree. Not a platform question; plain git in a scratch repository. A project has a pre-commit hook (a script in .git/hooks, and separately core.hooksPath pointing into the tree). The tool makes a merge commit in a second worktree (.worktrees/_integration). Does the hook run there, with which working directory and which GIT_DIR, and does a hook that needs a file from the main checkout fail?
Q11. Only if it is cheap. Does exit 2 from a UserPromptExpansion hook stop a typed plugin command run headless (claude -p "/<plugin>:<command> ...")? And does a workflow that is running survive the end of the session that started it with reason clear? If either cannot be tested headless, say so and do not guess.`

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
  `You are running platform probes for vsdd-factory 1.0, a from-scratch rewrite of a Claude Code plugin. Your working directory is the repository at ${ROOT}, on branch rewrite/v1. The repository CLAUDE.md in your context is the rewrite-mode one; follow it, except that this task names the files under rewrite/ that you write.

Background. The design depends on how Claude Code behaves, and some of that behaviour has never been observed. rewrite/spec/platform.md and rewrite/spec/topics/platform-proven.md list each platform behaviour the design relies on, numbered, with whether it is proven. Three rounds of probes have been run; the evidence and method of the last are in rewrite/spike/probes-round-3/ (read its files and CHECK.md first: they show what worked and what the checker faulted). The build plans for steps 3, 4 and 5 of the thin slice (rewrite/plans/step-3/, step-4/, step-5/; each has an ORDER.md and a RESOLUTIONS.md) rest on the behaviours below, which nobody has observed. Units that depend on them are about to be built, so a wrong assumption found now saves a rebuild.

THE QUESTIONS
${QUESTIONS}

How probes are run. rewrite/spike/plugin/ is a throwaway plugin used for the earlier probes: agents, skills, workflows, a hooks.json, a shell guard and a Python hook runner that logs what each event carries (set FSPIKE_LOG to a file path). The method that worked: copy that plugin to a scratch directory, add what a probe needs (a hook rule in runner.py, an agent file, a small workflow script), create a scratch git project, and run Claude Code headless against it:
  cd <scratch project> && FSPIKE_LOG=<file> claude -p "<prompt>" --plugin-dir <scratch plugin> --model sonnet --allowedTools "Workflow" "Agent" "Bash(<command>:*)" "Read" --output-format json < /dev/null > <result.json>
Then read the result JSON and the hook log. Use --permission-mode default when a probe is about permissions, and say which mode each run used. Give each agent in a probe an explicit tool list and low effort so it costs little, and ask probe agents for short structured answers. Use a scratch directory outside the repository for the plugin copy, the project and the logs (make one with mktemp -d). Put nothing about this user or this repository into a prompt you send. Keep the evidence: when you finish, copy the probe files you added, the exact commands you ran and the logs into rewrite/spike/probes-round-4/, one folder per question, so that each result can be checked.

YOUR TASK.
1. For each question: state it exactly, run it with a control where one makes sense (the case that should be refused, or the run with no hook), and record what you saw. A result must rest on something a command printed: the hook log line, the result JSON, a file that did or did not appear. Do not conclude from what an agent says about itself when a log would show it. If a probe is inconclusive, say so; do not infer. Q1, Q2, Q3 and Q5 matter most: do them first and most carefully.
2. Record the results in the design: add each settled behaviour to rewrite/spec/topics/platform-proven.md as a new numbered item (and to rewrite/spec/platform.md where that file lists the question as unproven), saying what was observed, in which permission mode, and with which platform version (claude --version). Keep each file under 16 KB and lines under 400 characters; if platform-proven.md would pass 16 KB, start rewrite/spec/topics/platform-proven-2.md, register it in rewrite/spec/registry.d/probes.toml the way platform-proven is registered there, and link it. Then run python3 rewrite/tools/spec_lint.py and fix what it reports in your files.
3. For every question where the platform does not behave as a plan assumes, add an entry to a new file rewrite/findings/sources/extra-probes-4.json (a JSON array): {"id": "PR4-<n>", "origin": "platform probes, round 4", "severity": "blocker" | "major" | "minor", "where": "the plan and spec files that rely on the behaviour", "problem": "what was assumed and what was observed", "proposed_resolution": "what must change, in which plan or unit", "on_slice_path": true}. Then run python3 rewrite/tools/findings.py build.

Do not edit anything under next/, plugins/, crates/, rewrite/history/, rewrite/tools/ or rewrite/plans/. Builders are working under next/ right now; your scratch work stays outside the repository. Do not commit, branch, stash or push, and never use git in this repository to discard or restore files. Return the structured result, one entry per question, including those you could not run.`,
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
    consequences: {
      type: 'array',
      items: {
        type: 'object',
        properties: { item: { type: 'string' }, plan_or_unit: { type: 'string' }, what_must_change: { type: 'string' } },
        required: ['item', 'plan_or_unit', 'what_must_change'],
      },
    },
  },
  required: ['verdict', 'checked', 'supported', 'problems', 'consequences'],
}

phase('Check')
let check = null
if (probes) {
  check = await agent(
    `You are checking the evidence behind a set of platform probe results for vsdd-factory 1.0. Your working directory is the repository at ${ROOT}, on branch rewrite/v1.

An agent has just probed how Claude Code behaves and recorded the results in rewrite/spec/topics/platform-proven.md (and possibly rewrite/spec/platform.md and a second proven file) and in rewrite/findings/sources/extra-probes-4.json. Its evidence, meaning the probe files, the commands and the logs, should be in rewrite/spike/probes-round-4/. The questions it was given:
${QUESTIONS}

What it reports:
${JSON.stringify(probes.results)}

For each result marked proven, disproven or partly, open the evidence and decide whether the result follows from what was actually printed. A result is supported only if the log or result file shows the behaviour directly. Look for: a conclusion drawn from what an agent said about itself when a log line would have shown it; a control that was not run (for a permission probe, the case that should be refused); a probe that tested something easier than the question (a top-level agent where the question is about a workflow step; the auto mode where the question is about the default mode); and a result that the recorded evidence contradicts. Rerun the probes for Q1, Q2 and Q3 yourself if their evidence leaves any doubt, and one or two others if cheap, using the same method (a scratch copy outside the repository, headless claude -p with --plugin-dir and low-effort agents, nothing about this user or repository in a prompt). Check that the spec files say what the evidence supports, no more.

Then say what follows for the build: for each result that differs from what a plan assumes, name the plan or unit (rewrite/plans/step-3/, step-4/ and step-5/, and their ORDER.md files) and what must change in it, concretely enough for a planner to act on. If the results agree with the plans, say so.

Do not modify any file except rewrite/spike/probes-round-4/CHECK.md, where you write your report. Do not commit, branch or push. Return the structured result: checked is the number of results you examined and supported is the number the evidence supports.`,
    { label: 'check', phase: 'Check', model: 'opus', effort: 'xhigh', schema: CHECK_SCHEMA },
  )
}

return { probes, check }
