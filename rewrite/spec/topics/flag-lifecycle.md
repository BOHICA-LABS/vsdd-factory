---
kind: topic
id: flag-lifecycle
package: F3
sources: [decisions.md#D31]
---

# The life of a flag

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D31; the capability
from end to end is [[topic:feature-flags]].

What happens to a flag after it is planned: delivery, the product under review, turning it on, the pull
request, the release and removal. The record and its computed states (`planned`, `live`, `on`, `removed`)
are [[artifact:flag]]. The checks named here are rows of [[table:flag-story-checks]] and
[[table:flag-release-checks]].

## Delivered off

A flag's owning story merges with the flag off: its definition serves the default, its fallback is the
default, its tests hold in three states, the suite holds with the flag on, and its review accounted for
the off state (F-USED, F-DEFINITION, F-STATES, F-ON, F-ITEMS). The stories behind the same flag follow
it. What becomes of an existing test that the on state breaks is the test-writer's duty
([[table:flag-duties]], F-STATES). A flag is no reason to wait for a person: `delivery.merge = "auto"`
applies to a story behind a flag as to any other ([[topic:governed-delivery]]).

## The product under review

`vsdd env start` ([[command:env-start]]) starts the product as F-KEY starts every command, so the wiring
takes its flag values from the override variable and connects to no flag source. What the variable holds
is decided here. For a story (a demo, a UI review) it turns that story's flag on. For a wave or a
release (hidden scenarios, a UI review) it turns every live temporary flag on, so the scenarios meet what
the cycle built. Permanent flags keep their defaults. The job record names what was set
([[record:job]]). The state with those flags off is not evaluated a second time there: it is the
behaviour the baseline already has, and the suite is compared against the baseline, with no flag set, at
every green gate, every wave gate and the release baseline check.

## The deferred item

Each temporary flag has one deferred item while it is live ([[artifact:deferred-item]]). The rule is
stateless, so any checkpoint repairs it: `vsdd checkpoint` writes `deferred/flag-<stem>.md`, with the
stem of the flag's record, for every live temporary flag that has none (`title: Remove flag <key>`,
`target: unscheduled`, `source: flag:<key>`, `decided` the day it is written), and retires the item of
every flag that is removed, dropped or made permanent ([[command:checkpoint]], [[command:retire]]). The
item holds no copy of the removal condition; that stays in the record. The register's limit and its
warning in `vsdd status` therefore also show when temporary flags pile up.

## Who turns a flag on

A person, in one of two places, and the factory acts in only one of them.

- **In the outside service** (who sees the flag, how fast it ramps, experiments): the person or the
  project's pipeline. The factory never does this.
- **In the repository's definitions**: a change request, for example
  `/vsdd-factory:feature "turn on checkout-v2"`. The story-writer writes one `feature` story that names
  the flag with `flag_change: enable`; only such a story, the flag's owner or its removal may change the
  flag's entry (F-DEFINITION). Its one new test reads the repository's definitions and fails until the
  entry serves the flag on, which gives the story its red gate. Its green gate applies F-ON and
  F-DEFINITION. Once the story is delivered and its cycle has closed, the flag is `on`.

The fallback in the code is not a way to turn a flag on. It stays the default until the flag is removed,
so the product stays in its old behaviour whenever the flag source cannot be reached.

## The pull request

`vsdd story deliver` renders the body from the story and its evidence ([[command:story-deliver]]).
Package F3 adds two sections to it.

- **Rollback**, in every pull request of every project, whatever its flag state: the revert of the merge
  commit. For a story that names a flag, a step comes before it: turn the flag off as the runbook's
  heading "Kill switch" describes, which takes effect within `flags.kill_switch_seconds`, then check as
  its heading "Verify a rollback" describes. For a story with `flag_irreversible` the section prints that
  line, because turning the flag off does not then restore the earlier state.
- **Feature flags**, for a story that names a flag: one row per flag with its key, what it controls (the
  record's `title`) and its default; under it the number of passing tests in each of the three states,
  the existing tests the story moved, each by name with its token, and the result of the run with the
  flag on, from the green evidence. A story with `flag_exempt` gets the reason here.

## The release

The checkpoint of `verify` adds a section "Feature flags" to the `release` packet, computed from the
records, the cycle's stories, their closure records and the gates' evidence ([[workflow:verify]]). In the
state requested the section is that state's one line and nothing else, and in the state off there is no
section ([[config:flags]]). In the state on it lists:

- flags that ship new in this release, off;
- flags this release turns on in the repository's definitions, each with how to roll it back: the
  runbook's kill switch, how soon it takes effect, and the value the product then falls back to;
- flags this release removes, each with the behaviour that stays and the commit a rollback would revert;
- the changes this release makes to definitions files, which a person or the pipeline has still to apply;
- stories with `flag_irreversible`, with the line;
- live temporary flags whose removal condition is met, and whether each is on;
- stories that carry `flag_exempt`, and permanent flags, with their reasons;
- the debt figures of the last `flags` check: reads outside the interface, unkeyed reads, keys whose
  fallbacks disagree, and inherited flags whose type or lifetime is unknown;
- every item the cycle's gates reported as not measured (`metrics.flags.unmeasured`).

The change log `vsdd release` renders gets one line for each flag turned on and each flag removed
([[command:release]]). `vsdd report debt` lists the debt figures above and the live temporary flags past
their removal condition ([[command:report-debt]]).

## Reads the code already had

After an ingest the code may read flags around the interface. They are counted, never ignored: the
`spec` packet prints the number, every `flags` check records it, and the sweep returns each one as a
fixable finding (F-INTERFACE), so `maintain` moves them behind the interface a few at a time, each as a
fix story through `story`, within `maintain.fixes_per_run`. What needs a design first, such as a second
source behind the interface, is planned as stories from the decision record
([[topic:flag-system-choice]]).

## Removal

The sweep finds a temporary flag that is due (F-REMOVE, [[command:gate-sweep]]): its removal condition is
met and it is on. A flag that has not been turned on in the repository's definitions is never proposed
for removal; the sweep reports it, and a person turns it on, asks for its removal or changes its
condition.

Whether a flag is on for everyone in production is a fact the factory cannot measure, so the project
says who answers it, in `flags.removal` ([[table:flag-config-keys]]).

- `human`: the question goes to a person once, before anything is written. The checkpoint of `maintain`
  opens a `merge` decision named for the flag, `merge.flag-<stem>` ([[topic:human-stops]]). Its packet
  gives the flag's
  title and lifetime, its removal condition, the story that turned it on and the release that shipped
  that, and asks whether this is a release flag that is now on for everyone. `approve`: the next
  `maintain` run writes the cleanup story. `reject until=<yyyy-mm-dd>`: the flag stays, and it is not due
  again before that day. A flag the product keeps, such as a kill switch, is made permanent with
  `/vsdd-factory:author flag` ([[skill:author]]). A run in report-only mode asks nothing and lists the
  flags that are due.
- `auto`: the cleanup story is written at once.

`vsdd story new-fix --maintain` ([[command:story-new-fix]]) writes the cleanup story, within
`maintain.fixes_per_run`: kind `fix`, `flag: <key>` with `flag_change: remove`, with these criteria: no
read of the key remains, no test sets it or is still named for its off state, its definition is gone, and
a regression test shows the on behaviour with no flag set. It is built through `story` like any fix, its
green gate holds it to the first three (F-DEFINITION), its review accounts for the code that only the off
state reached (F-ITEMS),
and it merges as `delivery.merge` says. When it is delivered the checkpoint retires the deferred item;
the record is `removed` and frozen, and its key is never used again.

A person can ask for a removal at any time with a change request. The request is the answer, so the
story waits for no decision, and it says which behaviour stays: the story-writer writes
`flag_change: remove` or `flag_change: withdraw` from it ([[artifact:flag]]). A withdrawal has the first
three criteria above, with no test still named for the flag at all, and no regression test. Where the
red gate refuses a `remove` because a test of the flag is still `pending` (F-STATES), the stories behind
the flag first provide what the test asserts, which gives the test its own name back, or the flag is
withdrawn. A permanent flag is never proposed for removal. A flag whose lifetime is `unknown` is reported
by every sweep until someone decides.
