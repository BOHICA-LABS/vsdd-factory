---
kind: rule
id: decisions
package: H7
sources: [hooks.md#2.10, hooks.md#2.11.5, hooks.md#2.3, BLUEPRINT.md#4.11, BLUEPRINT.md#5.4]
---

# decisions

Together with [[rule:gate-guard]], what makes "a decision is settled only by a person" true
(pipeline.md need H5). `pipeline.md` section 2.6 defines human stops as decisions. A decision is opened
only by the tool's checkpoint and settled only by a person.

| Registration | |
|---|---|
| Kind | gate (nothing to refuse except a malformed or stale decision) |
| Runs on | `UserPromptExpansion` with matcher `^vsdd-factory:decide$`, `onFailure: "block"`, timeout 10 s (hooks.md 2.3) |
| Result | `Block` when the function refuses; `Context` (with `decision.recorded` logged) when it succeeds |
| Fails | closed — "a decision that could not be recorded must not look recorded; the person sees the reason and retypes" |
| Relax | locked (decision D12) |

## The flow

1. A person types `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`. `decide` is a
   person-only skill in the agents-skills roster, so the command is registered. `UserPromptExpansion`
   fires before any model reads the message, with `command_name` and `command_args` in the payload.
2. The rule parses the arguments by one grammar (the blueprint's, BLUEPRINT.md#4.11): the id (which may
   be qualified, `release@hotfix-1.2.1`), the choice, then `key=value` pairs, then the note after `--`.
   Every value is named and a key may repeat; nothing is positional and nothing is quoted.
3. The rule calls the state design's settle function (need S7) with the id, the choice, the pairs, the
   note and the session id. That function checks that the decision is open, that the id is not
   ambiguous, that the choice and its keys are valid for the kind, and that the digest of what is being
   approved still matches. Then it writes the decision record, and nothing else: it promotes no file
   and runs no git, so the call takes milliseconds on a specification of any size and stays far inside
   the rule's four-second deadline. The next survey commits the record.
4. If the function refuses (no such decision, an ambiguous id, already settled, stale digest, unknown
   choice or key), the rule blocks. The person sees the reason, with the choices where the id was
   ambiguous, and nothing reaches the model.
5. If it succeeds, the rule returns context for the model: the decision id, the choice, and the next
   command from the status function. If `local/driver.json` says the driver is paused at this decision
   in this session, the context adds: "The driver was paused at this decision. Resume the driver loop:
   start the command in `next` with the Workflow tool and continue." That is what lets a person approve
   and walk away without typing `/vsdd-factory:drive` again. It logs `decision.recorded`.

## Why an agent cannot do any of this

The decision store is a code-only path class. [[rule:gate-guard]] refuses every tool call that runs a
subcommand on the `human_only` list (`decide`), whoever the caller is, including the main session's
model. A person can still run it in their own terminal, where no hook applies, through the
`~/.local/bin/vsdd` shim that `setup` installs.

Parser fixtures (hooks.md 2.11.5): `spec approve variant=B topology=split`; `spec revise
stage=contracts -- cover the offline case`; `ideas select pursue=ID-003,ID-007 reject=ID-004`;
`finding.S-7.04 waive key=code:src/auth.rs:lockout key=test:tests/auth.rs:lockout_race -- accepted for
the beta`; `finding.W-02 waive-coverage item=AC-3 -- covered by the manual check`; `blocked.S-7.04
open-spike name=lockout-timing`; `release hold -- wait for the security fix`. A finding key contains
colons and slashes but no spaces and no `=`, so it is a legal value.

## Event choice

The design uses `UserPromptExpansion`, where `pipeline.md` names `UserPromptSubmit`, for one reason:
the hooks documentation says `UserPromptSubmit` also fires for scheduled tasks, background subagent
reports and messages from other sessions, so it does not prove that a person typed anything.
`UserPromptExpansion` fires when a typed command expands. It requires `decide` to be a registered
plugin command (need P4). The platform probe of 2026-11-23 proved this headless: the event fires with
`command_name` and `command_args`, a hook can add context, and a block ends the prompt with no model
turn. The `UserPromptSubmit` fallback is therefore not built.

The decision kinds and their choices are [[table:decision-kinds]]; human stops are explained in
[[topic:human-stops]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The rule still settles on the clone and runs no git. Settling lists the
decision as unpublished in the clone record, under the lock it settles under, and until the remote
holds the settlement every reader treats the decision as open. After it has settled, the rule starts
a sync as a detached job of its own, keyed by the decision ([[topic:long-commands]]), and returns; the
context it gives the model says whether the settlement is published yet
([[table:work-claim-checks]], T-SETTLE). Whether a hook may leave a detached job
behind is not proven. If it may not, the next sync publishes the settlement.
