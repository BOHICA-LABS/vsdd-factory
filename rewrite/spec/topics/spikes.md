---
kind: topic
id: spikes
sources: [BLUEPRINT.md#6, pipeline.md#2.8, agents-skills.md#2.11]
---

# Spikes

Research spikes: the protocol, who may ask and how each entrance starts one, what bounds a spike, and
when one can run. Named, human-led spikes that span sessions are [[topic:named-spikes]]; the wrapper
that serves them inside a workflow is [[topic:prelude-run-spike]]; the workflow a person invokes is
[[workflow:spike]].

A spike is an isolated, bounded experiment that answers one question. There are two sizes of the same
protocol.

- A quick spike is one run of the `spike-runner` agent ([[agent:spike-runner]]) in a throwaway
  worktree. Agents, workflows and people can all start one, and no person has to approve it.
- A named spike is a human-led effort that lasts across sessions: [[topic:named-spikes]].

The protocol, the same from every entrance.

1. Ask. Someone states one question, why it blocks, and optionally a method and a commit to start
   from.
2. Open. The spike runner's first command is `vsdd spike open` ([[command:spike-open]]). The tool
   first establishes who asked, from a record the hook wrote and never from the runner's word, and
   refuses if it cannot. It then checks the ledger. If the same question already has a finding in the
   same unit — matched by a hash of the normalised question — and the runner that produced it worked
   under a read-deny set that covers this asker's, it returns that finding and the spike ends. If a
   limit is reached it refuses. Otherwise it writes a ledger entry
   ([[artifact:spike-ledger-entry]]): question, the asker's agent type, the effective read-deny set,
   the runner's agent id and working directory, the run, the starting commit, the time box.
3. Experiment. The runner works only inside its own worktree. Pipeline gates such as
   tests-before-code do not apply there. It cannot push, merge, dispatch agents or write anywhere
   else.
4. Record. The runner's last command is `vsdd spike record` ([[command:spike-record]]), which
   validates the finding against the schema and appends method, result, verdict and the worktree path
   to the ledger entry. The hook runner refuses to let a spike runner finish without a ledger record:
   for a workflow step it blocks the `StructuredOutput` call, and for a runner dispatched with the
   Agent tool it refuses the stop.
5. Return. The finding goes back to whoever asked. Nothing from a spike worktree reaches the product
   branch except by being rebuilt through the story playbook, tests first.

## Who asked

The asker's agent type decides which read walls the runner inherits, so it must be a fact and not a
claim.

| Entrance | Where the asker comes from | The runner's opening command |
|---|---|---|
| A workflow step returned `needs_spike` | The stored step record of the asking step. The hook wrote it and stored the calling agent's type with it; it also holds the question | `vsdd spike open --run <id> --step <step>`; the `--step` argument's syntax is [[command:spike-open]]'s |
| The `spike` workflow, started by a person or a skill | The run record, where the survey stored the question it was given as `--question-uri`. The asker is the main session | `vsdd spike open --run <id>` |
| A dispatch with the Agent tool | An admission the hook records at `PreToolUse` on `Agent` when the target is the spike runner (the dispatcher's agent type), and attaches to the runner when it starts | `vsdd spike open --question-file <path>`, a file the runner writes in the temporary directory |

`--asker-type <type>` is accepted as a cross-check only: a value that disagrees with the record is
refused. `vsdd spike record --json-file <path>` and `vsdd spike close --name <name> --json-file <path>`
take the finding as a file the runner writes in the temporary directory. No spike command reads
standard input, so each is a single command the hook can approve.

The full enumeration across callers, from the blueprint:

| Who decides a spike is needed | How it starts |
|---|---|
| A person | Types `/vsdd-factory:spike <question>`, which runs the `spike` workflow |
| The main session, on its own judgment | Dispatches the spike agent with the Agent tool. In the platform probe the model never started a workflow unprompted, and did dispatch the agent, so this is the entrance it uses |
| An agent dispatched in a normal session | Dispatches `vsdd-factory:spike-runner` with the Agent tool. The agent file sets worktree isolation, which the platform honours for a dispatch |
| A skill | A skill that instructs the main session starts the workflow. A skill that runs in a forked agent dispatches the spike agent like any agent |
| An agent running as a workflow step | Returns `needs_spike` with a spike request. It has no Agent tool. The wrapper stamped into every workflow runs the spike as a step of the same workflow and starts the asking step again with the finding |
| A workflow | Calls the same `runSpike()` function as a step. This works in a parent and in a child, because the spike is never a third workflow |

The test an agent applies is "do I have the Agent tool": if yes, dispatch; if no, return the request.
Every agent except the spike runner and the clerk has the Agent tool, limited by the hook to the
spike runner, so in a normal session a walled reviewer asks by dispatching the runner itself. Nobody
runs a spike "on behalf of" a walled agent, because that would record the wrong asker. Every agent
type may ask; the hook runner gives the spike runner the asker's read walls in addition to its own
(`holdout`, `reviews`, `runs` and everything outside the project, plus the asker's walls), which is
what makes a request from the `code-reviewer`, the `holdout-evaluator` or the `scenario-reviewer`
safe. Desk research is a valid spike method: `spike-runner` (which has web and lookup tools) chooses
between an experiment, research, or both.

## The shared wrapper

The JavaScript is `brief()`, `runSpike()` and `step()` in the prelude ([[topic:prelude-steps]],
[[topic:prelude-run-spike]]). Scripts cannot import, so the build stamps that block into every
workflow file, and a test fails if any copy differs from the source. In short: `step()` runs the
agent; if the result asks for a spike and the limits allow, it calls `runSpike()`, appends the finding
to the list the step is given, and runs the step again; if the limits do not allow, it returns the
step as blocked with kind `spike_limit`.

## Limits

The numeric limits - spikes per step, per run and per day, and the time box - are the spike rows of
[[table:limits]], with their keys, defaults and what enforces each. What the table does not hold:

| Limit | Enforced by | When exceeded |
|---|---|---|
| Nesting: none | The spike finding schema has no spike request, and the hook runner refuses a spike runner that dispatches agents | Not possible |
| Questions per spike: 1 | Schema | Not possible |

## How the finding returns to the step that asked

The asking agent stops when it returns `needs_spike`; any partial work stays on disk. The wrapper
starts a new agent for the same step with the same task, plus the findings, plus a line saying
earlier work may exist. A finding is small: verdict, answer, method, up to ten evidence lines,
confidence, and the paths of anything the runner built ([[schema:spike-finding]]). The asker may read
those paths as reference.

Ledger and retention. One ledger per project holds every spike from every entrance. A worktree the
platform kept because it has changes is listed in its ledger entry. When spike worktrees are removed,
and how long a named spike's branch is kept, is defined in [[topic:worktrees]].

## Where a spike can run

The `spike` workflow takes no run lock, so it is available while any other workflow runs. It does need
an initialised project: hooks are silent before setup, so no bound could be enforced there, and the
workflow answers "run `/vsdd-factory:setup` first". The user's own `resume-spike` convention keeps
working in any repository and can resume a named spike, because the branch, worktree and ledger file
names are the same.
