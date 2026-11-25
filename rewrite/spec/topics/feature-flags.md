---
kind: topic
id: feature-flags
sources: [decisions.md#D31]
---

# Feature flags

Built when the build widens past the thin slice ([[packaging:build-plan]]). Nothing on the slice path reads
this file or changes because of it.

Decision D31 ([[doc:decisions]]) makes feature flags a first-class concern of the software the factory
builds. It is not about flags inside vsdd-factory. It has two halves: the product gets a sound flag system
(one flag interface, a record of every flag, safe defaults when the flag source cannot be reached, a
documented kill switch), and features are delivered through it (a story can ship behind a flag, default
off, so that risky or half-finished work can merge without being exposed).

It adds no workflow, agent, skill, command, gate name, hook rule or decision kind. This file links the
parts and gives the reason for each choice; each fact is in the file the table names, and nowhere else.

| Part | Defined in | Built by |
|---|---|---|
| The setting, its two tables in `factory.toml`, the three states, patterns, keys | [[config:flags]], [[table:flag-config-keys]] | [[package:F1]] |
| The flag record and the story keys | [[artifact:flag]] | [[package:F1]] |
| The rules of every check; the checks of choosing and planning | [[table:flag-checks]] | [[package:F5]] |
| The checks of a story's gates and review | [[table:flag-story-checks]] | [[package:F2]] |
| The checks of release and maintenance | [[table:flag-release-checks]] | [[package:F6]] |
| Delivery, turning on, release and removal | [[topic:flag-lifecycle]] | [[package:F3]] |
| How the flag system is chosen, confirmed and recorded | [[topic:flag-system-choice]] | [[package:F4]] |
| Each agent's duty and what holds it to the duty | [[table:flag-duties]] | [[package:F4]] |

## From opt-in to removal

1. **Ask.** A person asks for flags in the brief, in a request or in `factory.toml`; or `ingest` finds
   that the code already has them. Only then does the architect propose a flag system.
2. **Confirm.** The `spec` packet shows the flag lines, and `approve flags=<source>` is the opt-in and
   the confirmation of the system, once per project (F-APPROVE).
3. **Configure.** The next survey copies the approved table into `factory.toml` and records the flags
   the code already reads (F-FILL).
4. **Build the system.** Its behaviour is four contracts, so `plan` covers them with stories, and a
   story behind a flag waits for the interface and the runbook (F-READY).
5. **Plan.** The story-writer applies `flags.require`, names a flag and writes its record (F-REQUIRE,
   F-RECORD).
6. **Test and implement.** Tests in three states at the red gate; at the green gate registered reads
   through the interface, the definition, and the whole suite once more with the flag on (F-STATES,
   F-REGISTERED, F-INTERFACE, F-USED, F-DEFINITION, F-DEFAULT, F-KEY, F-ON).
7. **Review.** The verdict stays open until a reviewer has checked the off state (F-ITEMS).
8. **Deliver and release.** The pull request and the release packet list the flags and how to roll
   back; each temporary flag has a deferred item.
9. **Turn on.** A person, in the service or by a change request for the repository's definitions.
10. **Remove.** The sweep finds a flag that is due, a person confirms where the project says so, and
    `maintain` builds the cleanup story (F-REMOVE).

## Why it is this way

- **Opting in is a person's act, and nothing stops for it.** A project that never asks never meets
  flags, and the architect does not propose them unasked. A person who writes `enabled = true` before a
  system is chosen gets a reported state, not a failing gate, because the remedy cannot run inside an open
  cycle. A "no" stands until a person changes it. ([[config:flags]])
- **The confirmation is typed.** Approving a large specification set must not opt a project in by the
  way, so the approval names the source, and a home-built system is approved in those words.
  ([[topic:flag-system-choice]])
- **One copy of the settings.** Code keeps the approved settings in a table of its own, and a person's
  override is a separate, visible line. ([[config:flags]])
- **The default of `require` is the narrowest rule a program can apply.** It needs two story keys, and
  it asks nothing during a first release, when nothing is exposed yet; a wider value is one line away.
  (The default: [[table:flag-config-keys]]; the values: [[config:flags]])
- **The record and the definition are separate files,** because one is planned with the story and the
  other merges with the code. ([[artifact:flag]])
- **Patterns, not parsers.** The tool is neutral about language, so the architect gives regular
  expressions for a read, a bypass, a test's setting and a definition's entry, and a person approves
  them. Each check states what its pattern cannot see, and a read the tool can match but cannot key is
  refused when new and counted when old. ([[config:flags]], [[table:flag-story-checks]])
- **A story ships behind a boolean flag whose default is false.** Off, unset and unreachable are then
  one behaviour, the one that exists, which the baseline already measures; on is the new behaviour, and
  the suite runs once more with it. ([[artifact:flag]], F-STATES, F-ON)
- **A flag hides behaviour, not effects.** What turning a flag off cannot undo ships earlier with no
  flag, or the story names it and the rollback text says so. A flag read on a user's device is enforced
  by the server. ([[table:flag-duties]])
- **The tool never needs the service or its key.** Every command a gate runs gets its flag values from
  the tool and no key. (F-KEY in [[table:flag-story-checks]]; the keys: [[config:flags]])
- **An existing test of what a flag changes is moved, never edited or dropped.** A feature added to
  existing code breaks the tests of what it replaces as soon as its flag is on. The run with the flag on
  shows which tests those are and whether each move was needed; whether the behaviour was meant to go is
  a judgment, so a reviewer accounts for every move. A test of behaviour the product must keep is marked
  `pending`, and while one is the flag cannot be turned on, or removed with its new behaviour kept. A
  moved test keeps its own name inside the new one, so a program can tell which test it was and when it
  has come back. (F-STATES, F-ON, [[table:flag-duties]])
- **The factory writes files and nothing else.** Definitions, the runbook, the test double's wiring, a
  rollout plan when asked, and the pipeline job where the pipeline applies definitions. Applying them,
  ramping, targeting and experiments are the person's or the pipeline's. ([[topic:flag-lifecycle]])
- **A removal is proposed only for a flag that is on,** and the project says whether a person confirms
  it, because production is not something the factory can see. ([[topic:flag-lifecycle]])
- **A removal names the behaviour that stays.** A feature that is given up must not leave its flag and
  its code behind, so a flag can also be withdrawn, and the two are told apart in the story because the
  gates treat them differently. ([[artifact:flag]], F-STATES)
- **Rolling our own is the last resort,** and is held to the same list as any system.
  ([[topic:flag-system-choice]])

## What nothing here measures

Each check states its own limit, once, in its `unmeasured` cell. The widest are those of F-REMOVE (what
production runs, which is why a removal can wait for a person) and of F-KEY (what an agent's own shell
and the session's connected tools can reach).

## Where each clause of D31 ends

| Clause | Ends as |
|---|---|
| 1. Flags of the product, not of the factory | Every key, record and check is about the project's product tree; none is in the plugin's own configuration |
| 2a. One interface, a record of every flag, safe defaults, a kill switch | F-INTERFACE; F-REGISTERED, F-USED, F-FILL; F-DEFAULT, the `fallback` and `startup` contracts and the `default` tests; F-RUNBOOK and the `kill_switch` contract |
| 2b. A story ships behind a flag, default off | F-REQUIRE, F-RECORD, F-STATES, F-ON, F-DEFINITION, the item `flag-off` |
| 3. Opt-in per project | The state off, which every project starts in; F-REQUIRE's first refusal; the architect's duty not to propose unasked |
| 4. An established system, confirmed, asked once | F-CHOICE, F-APPROVE; the architect's duty, checked by the adversary |
| 5. One interface, OpenFeature where the language supports it | `flags.standard` and its headings (F-CHOICE), checked per language by the adversary; F-INTERFACE |
| 6. Definitions in the repository, nothing done in the service, no keys, no live tests | F-USED, F-DEFINITION, F-KEY, the `offline` contract, the rule `secrets`, the items `flag-off` and `flag-system` |
| 7. No new workflow | The roster is unchanged, and L-ROSTER fails on a workflow, agent or skill outside it ([[table:lint-checks]]); L-FLAG holds each duty to a section of an existing method file |
| 8. Operating flags is out of scope | "Who turns a flag on" and the removal rule in [[topic:flag-lifecycle]] |
| 9. Designed now, built later | Packages F1 to F6, none on the thin slice ([[packaging:build-plan]]) |
