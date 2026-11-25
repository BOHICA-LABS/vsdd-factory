---
kind: topic
id: human-stops
sources: [pipeline.md#2.6, BLUEPRINT.md#4.11]
---

# Human stops

Decisions as a person meets them: the three routine stops, the exceptional ones, queuing, and how
work is reopened. The nine decision kinds, their grammar and their statuses are one table,
[[table:decision-kinds]]; this topic is how a person encounters them.

A human stop exists only where a person must decide. It is always a workflow boundary: the workflow's
checkpoint opens a decision, returns, and nothing waits inside a running script. The decision kinds
and choices are defined by the pipeline design ([[topic:decisions]] carries the store; the statuses
and records are [[record:decision]]).

## Routine stops, three per product cycle

- `brief` — what to build, for whom, and whether it is worth the money. Opened by the brief skill, or
  by `discover` in `assess` mode; either way the checkpoint that opens the decision also opens the
  cycle. Choices: `approve` → `spec`; `revise -- <guidance>` → the brief is reworked; `drop`.
- `spec` — the specification is the contract for what the product will do. Accepting it, choosing
  between design variants, and deciding whether to split into repositories are product and
  organisation decisions. Opened by `spec`; `feature` on the standard route or as an amendment;
  `multi` for a project-level specification. For an empty repository the packet also shows the
  toolchain lines (`[commands]`, `[paths]`) that approval will write into `factory.toml`. Choices:
  `approve [variant=<id>] [topology=single or split]` → the specification set is locked, next is
  `plan`; `revise [stage=<name>] -- <guidance>` → `spec` or `feature` runs again from that stage (by
  default from the review stage) with the guidance; `drop`.
- `release` — shipping to users. The person accepts the residual risk shown in the convergence
  report: waivers, dismissed findings, advisory counts, scenario scores. Opened by `verify`.
  Choices: `approve` → `release`; `hold -- <guidance>` → `verify` runs again with the guidance.

The grammar is `<id> <choice> [key=value ...] [-- note]` for every choice in this section. Values are
named, a key may repeat, and free text follows `--`. There are no positional values and no quoted
strings.

## Conditional stops, present only when configured or when the situation arises

- `ideas` — which opportunities to pursue is strategy. Raised at the end of `discover` in `scan`
  mode. One command settles the list: `select pursue=<ids> research=<ids> defer=<ids> reject=<ids>`.
  Rejected and deferred ideas are remembered so they are not proposed again.
- `merge` — a story touched a protected path (by default `.github/**`, `.env*`, `CODEOWNERS` and the
  files `[release]` names); it adds a dependency or upgrades one beyond what
  `specs/architecture/dependencies.toml` and earlier approved `merge` decisions list; or project
  configuration sets `delivery.merge = "human"`. The packet carries the pull request link. In
  `maintain`, dependency changes are batched into one decision per run. `approve`: the next
  `vsdd story deliver` for that story performs the governed merge, and a dependency the decision
  names is recorded in the decision record as approved, so the next story that uses it does not ask
  again. `reject`: the next survey closes the pull request and the story is descoped.
- `plan` — authorising the spend. Opened when project configuration sets `stops.plan = "always"`, or
  the plan's estimate exceeds `stops.plan_ceiling_agents` (default 600).
- `spec` (the recovered baseline) — whether the recovered contracts, architecture and toolchain
  describe the system well enough to build on. Raised at the end of `ingest` with `as: project`.
  Approval locks them as the specification and, where `[commands]` or `[paths]` are unset, writes the
  recovered toolchain into `factory.toml`. It is one decision, once per ingested project.

## Exceptional stops

These are queued, not blocking: the blocked unit waits, and the route continues with every other unit
that does not depend on it. The one exception is a wave whose own gate waits on a decision, which
holds the waves after it ([[workflow:wave]]).

- `finding` — a review loop reached its cap with an upheld blocking finding or an uncovered item, or
  a critical or security finding that an agent may not dismiss. Choices: `waive key=<key> [key=<key>
  ...] -- <reason>` (risk acceptance), `waive-coverage item=<item> [item=<item> ...] -- <reason>`,
  `direct -- <guidance>` (exactly one more round with the guidance), `descope`.
- `spec-gap` — a reviewer showed that the approved specification is silent or wrong on a point the
  story depends on. Choices: `amend` (the gap joins the amendment run, [[topic:amendments]]),
  `not-a-gap -- <reason>`, `descope`.
- `blocked` — a unit cannot continue. The reason is in the record, the packet and the briefing line:
  `reviewer` (a reviewer returned blocked, for example because it declined the material or an input
  was missing; opened the first time it happens); `spike_limit` (a step still needs an experiment
  after the spike limits, [[topic:spikes]]); `environment` (a credential, access right or tool is
  missing and cannot be supplied by an agent); `stuck` (the unit failed in two runs for no reason a
  person was asked about). Choices: `retry` (also "I have supplied it" for `environment`),
  `direct -- <guidance>`, `descope`. For `spike_limit` also `open-spike name=<name>` (a named,
  human-led spike, [[topic:named-spikes]]). For an optional environment check also `skip`.

The four reasons are one decision kind, because their choices are the same; the identifier is
`blocked.<subject>`.

Which decision a blocked step becomes: the checkpoint maps the stored result, not the script. A
review loop that ended at its cap opens `finding`. A review loop whose verdict reason is
`reviewer_blocked`, or a step blocked with kind `reviewer_blocked`, opens `blocked` with the reason
`reviewer` at once, the first time. A step blocked with kind `spec_gap` opens `spec-gap`;
`spike_limit` opens `blocked` with the reason `spike_limit`; `environment` opens `blocked` with the
reason `environment`. A step blocked with kind `gate`, `conflict` or `other`, and a unit that
`failed`, open nothing the first time: the next run retries, and the second failure opens `blocked`
with the reason `stuck`. Kind `agent_budget` is not a decision; the run returns `partial`.

## How a decision is recorded, and why an agent cannot forge one

- A decision is opened only by `vsdd checkpoint`, which is code. For the brief, the interactive brief
  skill runs `vsdd checkpoint brief` when the person says the draft is ready. The packet is a file a
  person can read: what is being decided, the evidence, the choices, and the recommendation where
  there is one.
- A decision is settled only by a person. They type
  `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, which is the person-only `decide`
  skill ([[skill:decide]]). The hook runner sees the typed command on the prompt-expansion event and
  writes the decision record itself, before any model reads the message. A person can also run
  `vsdd decide` in their own terminal, through the shim that `setup` installs. Every agent is blocked
  from that command and from writing the decision store.
- An approval is bound to a digest of what was approved: the specification file set for `spec`, the
  commit for `release`. The brief, which has its own decision, and the two guidance files,
  `conventions.md` and `review-rules.md`, which have their own lock, are outside that set. Approval
  changes no file, so the digest is over the files exactly as they are. If the content changes
  afterwards the decision reads as `stale` and the stop returns. In a session, settling writes the
  decision record and nothing else; the next survey commits it.
- The approved specification set is locked as a whole: while the latest `spec` decision is approved
  and no live run holds an amendment, no file in the specification classes can be written, whether or
  not it carries a status. It changes only inside a `feature` or `spec` run that holds an amendment,
  and that run ends at decision `spec` again, whose packet lists each changed file. An accepted
  decision record stays frozen even under an amendment. Specification gaps found during a wave are
  batched: the stories wait, the wave finishes, and one amendment run covers them all, inside the
  open cycle ([[topic:amendments]]).
- When a decision opens, the tool notifies through the channel in project configuration, and the
  session briefing lists open decisions.

Reopening work is explicit. `revise` and `hold` mark stages as not done in the unit's closure record,
from the stage named by `stage=` or, by default, from the review stage, whose fix loop then works on
the guidance ([[topic:resume]]).

A second cycle beside the main one is [[topic:hotfix-cycle]].

## Feature flags (decision D31; built when the build widens)

No decision kind is added, and no story stops because of a flag. `spec` carries the choice of the
product's flag system, and approving a specification that proposes one needs the value `flags`
([[topic:flag-system-choice]]). Where the project's setting says a person confirms the removal of a
flag, `merge` carries that question once for each flag, before the removal is built
([[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

No decision kind is added and no stop. A decision opened on one clone is settled by any person on any
clone, the first settlement pushed stands, and the record names the person and the clone
([[topic:team-records]]). A commit made by hand reaches a person through two existing kinds: `finding`,
when its review item is still open at the cap, and `spec-gap`, when the specification has fallen behind
it ([[topic:team-delivery]]).
