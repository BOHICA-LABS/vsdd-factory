---
kind: topic
id: risks-agents
sources: [agents-skills.md#7]
---

# Risks of the agents and skills design

The risks the design names and what mitigates each.

1. **Platform behaviour this design assumes and has not verified.** Each is a one-line test in a throwaway
   plugin, like the first platform spike. They should run before the agent files are written.

   | Probe | Assumption | If it fails |
   |---|---|---|
   | V1 | `${CLAUDE_PLUGIN_ROOT}` is substituted in an agent file's body. | The hook runner adds the plugin root to the agent's context at `SubagentStart`, or the workflow wrapper puts it in every step prompt. |
   | V2 | A workflow step whose agent has an explicit tool list can still return a structured result. | Covered already by listing `StructuredOutput`; if the name differs, lint updates every file. |
   | V3 | An agent that lists an MCP tool can call it as a workflow step, by direct dispatch and as a forked skill; an unresolved name is dropped silently; no tool-search step is needed first. | Add the tool-search tool to the agents that have MCP sets. If that is not enough, those agents, and only those, drop their `tools` list and inherit. |
   | V4 | An agent without the Skill tool is not sent the skill listing. | No change in behaviour; the saving is smaller. |
   | V5 | `isolation: worktree` in a plugin agent's frontmatter is honoured, and the hook can see the isolation argument of an `Agent` call. | Callers pass it; the hook checks the spike's working directory instead. |
   | V6 | A forked skill can name a plugin agent as `vsdd-factory:<name>`. | Use the bare name, or switch the two forked skills to main-session dispatch. |
   | V7 | `omitClaudeMd` works for a plugin agent. | The holdout evaluator's wall is extended to CLAUDE.md by the hook. |

2. **Same-family review.** Opus reviewing Sonnet's work shares more blind spots than a different vendor would.
   The design compensates with walls, fresh context, hidden scenarios and tool-based verification, not with a
   second vendor. If review quality disappoints in the end-to-end proof, a second vendor is a new requirement,
   not a setting.
3. **Walls have edges.** A wall keyed on agent type is only as good as its coverage of `Bash`, of recursive
   search and of browser `file:` addresses, and a workflow author can still leak by writing a builder's
   summary into a reviewer's prompt. Needs N-HOOK-1, N-HOOK-9, N-STATE-8 and N-PIPE-3 exist for this
   ([[topic:seams-agents]]), and the prompt rules need a test in the workflow suite.
4. **Merged agents can become grab-bags.** `devops` has ten methods. The split rule is the control
   ([[topic:agent-design-rules]]); the lint limit on agent file length is the alarm.
5. **A method file may not be read.** An agent could act without reading the method its task names. The
   control is that results carry fields only the method defines (the score scale, `novelty`, `tools_used`), so
   a skipped method shows up as a failed schema or a failed gate.
6. **Explicit tool lists keep project tools out.** A project's own MCP servers do not reach builders in 1.0.
   If this hurts, the fix is a project configuration list of extra tool names stamped into builder files at
   setup, which this design leaves out to keep agent files static.
7. **Untrusted content.** `researcher` and `spike-runner` read web pages. The researcher has no shell and a
   narrow write scope. The spike runner has a shell, confined to its worktree and barred from pushing, and
   depends on the hook design's secret and destructive-command guards.
8. **Refusals in security work.** The current models can decline a request their safety classifiers flag, and
   security review, fuzzing and exploit analysis are near that line. A step that returns no result must count
   as a failed step and reach a person, never as a clean review.
9. **Cost of the planner tier.** Seven agents run on the most expensive model at the highest effort the user
   chose. The tier is one line in each file, so it can be tuned after the end-to-end proof measures it.
10. **Name collisions.** Skills and workflows share the `/vsdd-factory:` namespace. P-9 and the lint rule
    guard it, but only once the pipeline's names are final.
11. **On-demand reviews pass through the main session.** The `review` skill dispatches from the main session,
    which could add its own framing to the prompt. Pipeline reviews are not affected, because scripts compose
    those prompts. If it matters, `review` splits into one forked skill per reviewer at no listing cost.
