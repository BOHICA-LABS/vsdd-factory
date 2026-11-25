---
kind: rule
id: spike-bounds
package: H8
sources: [hooks.md#2.10, hooks.md#2.11.6, hooks.md#2.3, BLUEPRINT.md#6, BLUEPRINT.md#5.4]
---

# spike-bounds

Enforces what only a hook can about the spike protocol ([[topic:spikes]]; the spike agent is
[[agent:spike-runner]]). The runner's first command is the tool's `spike open`, which checks the ledger
and the count limits and writes a ledger entry; its last is `spike record`. Counts per step, per run and
per day are enforced by the workflow wrapper and by `spike open`, in code.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | any guarded tool called by a spike runner, including its `StructuredOutput` call; `Agent` (to record who dispatched a spike runner); `SubagentStart`; `SubagentStop` |
| Result | `Block` |
| Fails | closed, except at stop — "a spike with no ledger entry or past its time box must not continue. A finish without a record is refused once; the second attempt is allowed and logged, so an agent is never trapped." |
| Relax | time box tunable (project configuration) |

`SubagentStart` cannot block, which is why the first two bounds act on the runner's tool calls and not
on its start. Every check matches the runner by `agent_id`; the worktree is recorded in the entry and
is used for containment, not for identity.

## The bounds

### No work without an open ledger entry

Acts on every guarded tool call by a spike runner. Until the ledger holds an open entry for the
runner, every call is blocked except `spike open`, a `Write` of the question or finding file in the
temporary directory, and a `Read` of a file under the plugin's `methods/` or `templates/` (the agent
file tells it to read its method first). A runner that was refused by `spike open` can therefore do
nothing but return its refusal. The entry is matched by the runner's `agent_id`. The tool learns that
id from the caller marker the hook writes for the runner's `spike open` call, so two runners that open
at the same moment cannot be swapped, and `spike open` writes the id into the entry. For a named
spike, whose entry already exists, the runner's first `spike open --name` attaches its id to that
entry. Every spike is isolated, research spikes included, so no two runners share a working directory
either.

### Time box (default 20 minutes, from project configuration)

Acts on every guarded tool call by a spike runner. The rule notes when it first saw each spike
runner's `agent_id`. After the time box, every call is blocked except `spike record`, with the message
"time box over: record what you have".

### No nesting

Acts on `PreToolUse` for `Agent`. The spike runner's `dispatch` list is empty.

### Who asked

Acts on `PreToolUse` for `Agent` when the target is the spike runner, and on `SubagentStart` for a
spike runner. The rule records an admission through the state design's `spike_admit`: the session and
the dispatching caller's agent type (empty for the main session). When a spike runner starts,
`spike_attach` pairs it with the oldest unclaimed admission of that session. If more than one
admission is pending and the platform does not say which dispatch started which runner
([[doc:platform]], item 35), the runner is given the union of their walls, which is the stricter
result.
Inside a workflow there is no dispatch: `spike open` reads the asker from the stored `needs_spike`
step record instead. Either way the asker's type is a fact the hook wrote, and `spike open` refuses
when it finds none.

### No finish without a finding

Acts on the spike runner's `StructuredOutput` call when it is a workflow step, and on `SubagentStop`
when it was dispatched with the Agent tool. If the ledger entry for the runner's `agent_id` has no
verdict (or, for a named spike, no increment recorded in this run), the finish is refused once with
"run `spike record` first". The platform probe showed why there are two places: a refused stop holds
an agent that returns text, and does not hold a workflow step that has already returned its schema
result, so for a step the rule acts on the result call, which the probe showed can be blocked and
retried. If a finish was already refused once (`stop_hook_active`, or a second result call), it is
allowed and `spike.stop_unrecorded` is logged, so a runner is never trapped.

## Exemption

`ctx.in_spike` is defined in [[topic:rule-model]]. Rules declared `spike_exempt` are skipped when it is
true: [[rule:tests-first]], [[rule:artifacts]] and [[rule:purity-advice]]. The safety rules, the walls
and [[rule:gate-guard]] still apply inside a spike.

## Containment

So that nothing leaves a spike except through the pipeline:

- **[[rule:write-scopes]]**: the spike runner may write only its own worktree and `tmp`. The ledger is
  written by the tool, not by a tool call. Which worktrees those are is the definition of the computed
  `own-worktree` class in [[table:path-classes]] — the worktree that holds the caller's working
  directory and, for the runner, also the worktree its open ledger entry records, never the main
  checkout — so a spike runner writing into a story worktree or the integration worktree is blocked,
  however it spells the path. This is how need N-HOOK-3 of `agents-skills.md` is met. The hook cannot
  see at dispatch time whether isolation will apply, so it enforces the effect instead. The agent
  file's `isolation: worktree` makes the normal case work.
  Of that definition, the hook runner built in step 3 answers only the worktree that holds the
  caller's working directory (`containing_worktree` recognises worktrees under `.worktrees/`
  alone, so the platform's throwaway worktree is not yet `own-worktree`); the union with the
  ledger-recorded worktree, and the platform worktree, are built and tested here, with
  spike-bounds (S3R-hook-runner-b-11).
- **[[rule:bash-git]]**: the spike runner's row grants no push, no merge and no pull request. Nobody
  may merge, cherry-pick or rebase a `spike/*` ref onto another branch, or open a pull request whose
  head is one.
- **[[rule:read-walls]]**: the spike runner is denied `holdout`, `reviews`, `runs` and `outside`, and
  also every class denied to the agent type that asked. The ledger entry records that type
  (`asker_type`) and the resulting deny set (`read_deny`), and both come from hook-written records,
  never from an argument the runner types. Before the entry exists the runner can read only its method
  file, so there is no window in which it is unwalled. A spike therefore cannot be used to read around
  a wall, whoever asks, and the `code-reviewer`, the `holdout-evaluator` and the `scenario-reviewer`
  can ask safely. The cached-finding rule completes it: `spike open` returns an earlier finding only to
  an asker whose deny set the earlier entry's `read_deny` covers, so a finding produced for the
  adversary is never handed to the holdout evaluator.
- **No relay.** Every agent except the spike runner and the clerk has the Agent tool, limited to the
  spike runner, and asks for itself; nobody runs a spike "on behalf of" a walled reviewer, which
  would apply the asker's walls wrongly.

The spike protocol and its limits are [[topic:spikes]].
