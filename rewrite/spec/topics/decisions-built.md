---
kind: topic
id: decisions-built
sources: [BLUEPRINT.md#2.4]
---

# Where each decision is built

The decisions themselves are in [[doc:decisions]]. This file says where each one is built.

## D1, D2

State on its own branch, committed and pushed by code: [[topic:state-directory]], [[topic:state-commits]].
`state.push = false` gives manual pushes.

## D3

`vsdd import`, package [[package:S5]]; the importer also writes the baseline and an approved `spec` decision so
that `feature` can start ([[topic:brownfield]], [[command:import]]).

## D4

The `auto-allow` hook rule is the one mechanism ([[rule:auto-allow]], [[topic:tool-call-approval]]). The
person's answer is stored as `[hooks.rules.auto-allow] scope` in `factory.toml`, written by
`vsdd init --auto-allow <scope>` from what `setup` asks. The platform probe passed in the default permission
mode ([[doc:platform]], item 7). The scope you adopted is the value `core`. Setup also offers `extended`,
described in [[rule:auto-allow]].

## D5

Four tiers and aliases in `tests/lint/roster.toml` ([[doc:conventions]], [[table:roster]]). The builder-tier
override for ingestion's inventory passes is `[overrides]` in the same file. The weekly conformance job reruns
the story tier when a model alias moves.

## D6

The `main` row of the wall table ([[table:walls]], [[topic:information-walls]]). The switch is
`[hooks] main_session_wall` in `factory.toml`, which only a person can edit. It relaxes the `main` row only and
is announced in every briefing while it is off.

## D7

The `decide` skill and the `decisions` hook rule ([[skill:decide]], [[rule:decisions]]).

## D8

`/vsdd-factory:spike` is a workflow; there is no spike skill ([[workflow:spike]], [[topic:spikes]]).

## D9

The convergence rule ([[topic:convergence]]); the proof measures the escape rate ([[doc:platform]], item 21).

## D10

One `verify` workflow ([[workflow:verify]]).

## D11, D12

Default gates and which may be relaxed ([[topic:rule-tuning]], [[table:rules]]). The walls stay locked for every
agent row; the one stated exception is the `main` row under D6.

## D13

Four bundled servers, both tool spellings ([[topic:research-and-browser-tools]]).

## D14

`vsdd export pdf` and `vsdd render diagram`, package [[package:T5]].

## D15 to D24

Keys and defaults in [[table:config-keys]]; decisions `merge`, `plan` and `release` in [[table:decision-kinds]];
spike limits in [[topic:spikes]]. Release configuration files are among the protected paths (D15), and the plan
ceiling is `stops.plan_ceiling_agents` (D16).

## D25 to D30

Cutover steps, package [[package:CO2]] ([[packaging:cutover-steps]]).

## D31

Feature flags in the software the factory builds, end to end: [[topic:feature-flags]]. The setting is
[[config:flags]]; the choice of flag system, confirmed in the `spec` decision, is
[[topic:flag-system-choice]]; the record is [[artifact:flag]]; the checks are [[table:flag-checks]],
[[table:flag-story-checks]] and [[table:flag-release-checks]]; each agent's duty is
[[table:flag-duties]]; delivery, release and removal are [[topic:flag-lifecycle]]. Packages
[[package:F1]] to [[package:F6]], built when the build widens past the thin slice
([[packaging:build-plan]]).

## D32

The hook runner core, package [[package:H1]]: an agent's single `vsdd` call is approved together with the
rewrite that carries the caller marker, whatever `hooks.rules.auto-allow.scope` says
([[topic:caller-identity]], [[config:hooks]]). A test gives the approval with the key `off`.

## D33

Team use, several clones of one repository, end to end: [[topic:team-use]]. The setting is
[[config:team]], with [[record:clone]]; how a clone meets the remote is [[topic:team-sync]], with
[[command:sync]] and the git behaviour it rests on, [[topic:team-git-facts]]; the work claim is
[[topic:team-claims]], with [[record:work-claim]], [[table:work-claim-scopes]], [[command:claim-take]]
and [[command:claim-release]]; who is recorded, decisions across clones and hand-off are
[[topic:team-records]]; delivery and commits made by hand are [[topic:team-delivery]]; the checks are
[[table:team-checks]], [[table:sync-checks]], [[table:work-claim-checks]] and
[[table:team-delivery-checks]]; the proof is [[packaging:proof-team]]. Packages [[package:M1]] to
[[package:M6]], built when the build widens past the thin slice ([[packaging:build-plan]]). What the
slice's own units must not assume is in the ledger as `D33-<n>`.
