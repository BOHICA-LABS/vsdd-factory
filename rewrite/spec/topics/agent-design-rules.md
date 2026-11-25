---
kind: topic
id: agent-design-rules
sources: [agents-skills.md#1, agents-skills.md#2.2]
---

# Design rules for agents and skills

In 1.0 this subsystem is 20 agents, 13 skills and about 75 method files, down from 35 agents and 127 skills.
An agent type exists only where the model tier, the tool list, the information wall or the isolation differs.
Coordinators, bookkeepers and relays are gone, because workflows orchestrate and code keeps the books.
Seven agents run on the planner tier, six on the builder tier, six on the reviewer tier and one, the clerk,
on a tier of its own ([[topic:model-tiers]]). Every agent file is short, lists its tools explicitly, and
points to method files that are read only when a task needs them. Walls and write scopes are tables keyed on
the calling agent's type, for the hook runner to enforce ([[table:walls]]). Only one skill is visible to the
model ([[topic:skill-file]]). Research and browser tools reach agents through explicit tool lists that name
both the plugin's and the user's MCP servers ([[topic:research-and-browser-tools]]).

## The rules that decided every entry in the rosters

1. **An agent type must earn its place.** A separate agent exists only where one of four things differs: the
   model tier, the tool list, the wall or write scope, or the isolation. Two old agents that differ only in
   what they are asked to do become one agent with two method files.
2. **No coordinators, bookkeepers or relays.** A workflow script decides what runs next. Status, indexes and
   counts are written or derived by code. An agent that only passes commands along (the old `github-ops`) is
   replaced by giving the tool to the agent that needs it.
3. **Every agent lists its tools.** No agent inherits the session's tool set. This keeps unrelated tools (the
   user's mail, calendar and other MCP servers) out of every agent, and makes the tool list reviewable.
4. **No agent has the Skill tool.** Procedures are method files, and spikes use the Agent tool or a request.
   This removes the skill listing from every agent start if the platform omits the listing when the tool is
   absent (probe V4; nothing depends on it for correctness).
5. **Done means on disk.** An agent writes its artifact (and commits code) before it returns. Its result
   carries paths and facts, not content. No agent writes status, index or log files.
6. **Verdicts are computed, not chosen.** A reviewer reports findings with severities, and the holdout
   evaluator reports one score per scenario. Whether a pass is clean or a gate passes is arithmetic in the
   workflow script.
7. **A skill must need a person or the main session.** Anything that is a sequence of agent steps is a
   workflow. A skill is kept only for conversation with a person, for a one-off specialist job outside the
   pipeline, or for driving a local tool.
8. **Short files, checked by lint.** Limits in the design (line counts, description lengths, the number of
   model-visible skills) are test failures, not advice; the checks are handed to the packaging design
   ([[topic:seams-agents]]).

## Where the line is

`devops`, `verifier` and `researcher` each absorb several old agents. The rule for splitting one again is
mechanical: split when two of its tasks need different tools, a different wall, or a different tier. Until
then, a method file is the cheaper way to carry a different procedure, because it adds nothing to the fixed
cost of every agent start.
