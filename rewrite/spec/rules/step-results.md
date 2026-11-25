---
kind: rule
id: step-results
package: H7
sources: [hooks.md#2.10, hooks.md#2.11.7, BLUEPRINT.md#5.4, BLUEPRINT.md#4.10]
---

# step-results

Stores the step result when a workflow step returns. A workflow script cannot write a file, so the
hook does it (`pipeline.md` needs H1 and H2).

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse` for `StructuredOutput`. A call that carries no step identity passes untouched. |
| Result | `Block` (once per step on ungrounded findings; always on a gate result with no matching evidence), then stores |
| Fails | closed — "a step result that is not on disk breaks resume, and an ungrounded finding must go back to its author. The platform's retry limit bounds the loop." |
| Relax | locked (decision D12) |

## What it does

1. If the result carries no step identity (`step.run`), the rule returns nothing and the call passes.
2. **Sender.** A record is accepted only when `step.run` names a live run ([[topic:run-lock]]). The
   rule stores the payload's `agent_type`, and the worktree head (`worktree_head`, the commit the
   worktree's head was at when the result was stored, distinct from the result's `head`), with every
   record. It does not refuse by step name or method; the check sits where the record is used: the
   tool counts a review pass only when the stored agent type owns the method in the record's identity
   ([[topic:verdicts]]).
3. **Gate results.** For each entry of `gates` with `passed: true`, the rule looks the evidence record
   up in the evidence store by the entry's subject, gate and commit — never by the path the entry
   carries, which an agent could forge — and blocks (`gate_result_mismatch`) unless the record exists
   and its gate, subject, commit and `passed` equal the entry's. The fix line says to run the step's
   closing command and return its output unchanged. A gate result an agent wrote itself is therefore
   never stored.
4. **Grounding.** For each finding in a review result, the rule checks `location`. The path is relative
   to the root of the reviewed tree, which the rule gets from the state design's
   `unit_tree(project, workflow, subject)` ([[topic:state-api]]): the story's worktree, the
   integration worktree, or the project root for specification files. The path must exist; `line_start` and `line_end` must lie
   inside the file, and a finding that gives only one of the two is read as that one line; and the `quote`, if given, must occur within three lines of that range, comparing
   with whitespace normalised. Holdout symptoms have no location and are not checked, and neither is a
   finding of category `holdout`, which the tool builds from scenario scores with no location
   (`pipeline.md` 2.7). If any finding fails, the rule blocks the call once per step, with one line per
   finding (`finding <key>: quote not found in <path>`), so the reviewer can correct it. On the next
   return it does not block again: it stores the result with each still-ungrounded finding marked
   `grounded: false`, which makes it advisory. One bad citation therefore cannot use up the platform's
   retries and lose the pass. On that second return the rule also returns `updatedInput` for the call,
   the same result with the mark on those findings, so the script judges what the tool stored
   ([[topic:verdicts]]). The platform honours the rewrite on a `StructuredOutput` call in a step, in the
   default mode, while the finding object stays open ([[topic:platform-proven-2]], items 37 and 38).
   "Once per step" is a marker file, `<hooks_dir>/steps/<32 hex of sha256("run/workflow/subject/name.attempt")>`,
   written before the block and removed at the store; a stale marker is `vsdd explain`'s (H9) to prune.
   An agent-supplied `grounded` key is never stored: it is stripped from every finding, and only the
   failing findings carry `grounded: false` in the stored record; the `updatedInput` is the agent's own
   result with those keys removed and the failing findings marked — not the stored record, which carries
   the runner's own fields (`agent_type`, `worktree_head`, `agent_id`) beside what the agent sent — and
   the rule returns it whenever it differs from the input, so an agent cannot declare its own finding
   grounded.
5. **Storing.** The rule writes the step record through the state design's store function
   ([[topic:state-api]]), keyed by run, workflow, subject, name and attempt, before the call
   completes. It logs `step.recorded`. The store validates the record against the step-result schema
   before it is written, and the tool honours the record's stored `grounded` mark when it counts the
   record toward a verdict — it does not ground the findings again ([[topic:verdicts]]).
6. **Walled findings.** A finding in a result from the `scenario-reviewer`, and any finding whose
   `location.path` lies under `.factory/holdout/`, quotes or names a hidden scenario. The store
   function writes its full form to the evaluation file in the `holdout-evaluations` class. The step
   record keeps it too, in `runs`, which no caller can read. Everything derived later (a decision
   packet, a closure record, a claim, the output of `vsdd review show`) carries its walled key (a hash
   of the key, `pipeline.md` 2.7) and severity only, unless the reader may read `holdout`. See
   [[topic:walled-findings]].
7. **Spike runners.** When the caller is a spike runner and the result is its spike finding,
   [[rule:spike-bounds]] runs on this same call.

Cases in `tests/hooks/cases/step-results.toml` hold the gate check: a passing entry that names no
evidence record (blocked); one whose commit differs from its record's (blocked); one that matches
(stored); a failing entry with no record (stored).

The rule does not re-validate the whole result against its JSON Schema: the store validates the step
record it writes, and the rule surfaces the store's `schema_invalid` refusal when the result is
invalid — the hook sees the result before the platform's own schema check. It reads only the fields
it uses, whose names come from `<plugin>/schemas/step-result.schema.json` and
`finding.schema.json`.

The two platform facts under this rule were proven by the probe of 2026-11-23: the payload carries the
whole result as `tool_input`, and a block makes the agent correct the result and return again. Nine of
the twenty agents have no shell, every pipeline reviewer among them, so no fallback that needs a tool
in the agent can stand in. If a later platform release changes the behaviour, the fallback is one that
needs no tool in the agent: the runner stores the result at `SubagentStop` from the subagent's
transcript (`agent_transcript_path`), and ungrounded findings are marked advisory when the tool
computes the verdict. The weekly conformance tier re-checks the two facts.
