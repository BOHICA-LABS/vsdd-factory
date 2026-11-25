---
kind: topic
id: flag-system-choice
package: F4
sources: [decisions.md#D31]
---

# How the flag system is chosen

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D31; the capability
from end to end is [[topic:feature-flags]].

Which flag system the product uses is asked once per project, when the project opts in. The factory uses an
established system, a person confirms which, and it never picks silently.

## The stop that carries the question

The `spec` decision ([[topic:human-stops]], [[table:decision-kinds]]), in each of the three places it
already opens: at the end of `spec` for a new product, at the end of `ingest` for existing code, and at the
end of a `feature` amendment for a project that opts in later.

No decision kind is added, for this reason. The choice is architecture: a decision record, a line of
`dependencies.toml`, four contracts and a table of the toolchain file, all inside the specification set
that a `spec` decision approves as a whole and binds to one digest ([[topic:digests]]). A second decision
over part of that set could be settled the other way from the first. `spec approve` gains one value,
`flags`, and its packet gains the flag lines below.

A project that opts in after its specification is approved does it with a change request for the flag
system. That request adds contracts, so its route is `standard` and it runs as an amendment
([[workflow:feature]], [[topic:amendments]]).

## The proposal

The architect makes one only in the cases its duty names ([[table:flag-duties]]): the code has a flag
system, a person asked for flags, or the project's state is `requested` ([[config:flags]]). It writes, in
the method `architecture`:

- a decision record ([[artifact:adr]]), the record of the choice;
- the `[flags]` table of the toolchain file ([[artifact:toolchain]]), whose keys are
  [[table:flag-config-keys]];
- the system's library in `dependencies.toml` ([[artifact:dependencies]]), so the story that adds it does
  not wait for a `merge` decision.

The product-owner writes the four contracts the table names in `contracts`:

- `fallback`: every read returns the fallback passed at the read when the source cannot be reached or does
  not know the flag, and raises no error into the product;
- `startup`: the product starts, within a stated time, while the source cannot be reached;
- `kill_switch`: a change to a flag's value takes effect in a running product within
  `flags.kill_switch_seconds`, without a restart;
- `offline`: while the override variable is set, even to `{}`, the wiring takes flag values from it and
  opens no connection to the flag source.

`vsdd gate spec` checks the whole proposal (F-CHOICE, [[table:flag-checks]]).

## The order of preference

The architect takes the first that applies and writes it as `source`.

| `source` | Applies when | The record must show |
|---|---|---|
| `existing` | The analysis of existing code found a flag system. The factory proposes keeping it and never adds a second. | The citations from the analysis, and the heading "Existing flag reads". |
| `named` | The brief, the change request or the person's guidance names a system. | The words that name it, quoted. |
| `researched` | Neither of the above. The architect researches which established systems fit the product's stack and constraints and recommends one. | At least two systems considered, the fit of each, and the reasons for the one recommended. |
| `home-built` | No established system fits, for example a product that runs offline or embedded. The last resort. | Why no established system fits, under its own heading. |

Only a person's own words move a proposal down the order. After an ingest the architect always writes
`detected`, and where the code has a flag mechanism it always makes a proposal, so the factory cannot
quietly ignore one it found. Where the person names another system, the next proposal is `named` with
`detected` still true, and its record says how the code moves off the old one.

## What the record must answer

Soundness is the same list for every source, a home-built system included. The decision record has these
headings, each with text under it, and F-CHOICE refuses a record that lacks one.

- **Source of the choice**: the row above and its evidence.
- **Existing flag reads** (only when `detected` is true): every flag mechanism the analysis found, and
  what becomes of each: kept as the system; put behind the interface as a further source; or moved by
  stories. Then the number of reads outside the interface that F-CHOICE counted, and how they move behind
  it: by planned stories, or by the sweep's fix stories ([[topic:flag-lifecycle]]).
- **Interface**: the one interface the product's code reads flags through, the files that hold it and
  its wiring (`flags.interface`), and the shape of a read, with the key as a literal, from which the read
  patterns are written. For each language of the product: whether an OpenFeature SDK exists for it.
- **Where flags are evaluated**: for each component, on a server or in code that runs on a user's device.
  Code behind a flag that is evaluated on a device is shipped to every user, and whoever runs it can
  change the value, so for those flags the record says how the server enforces them.
- **Defaults and failure**: four cases. The source was never reached since the product started (what the
  `startup` contract states). The source was lost after the start: whether a read returns the last value
  the source gave or the fallback, and for how long. The source is slow: the longest one read may wait. The
  product restarts while the source cannot be reached: what a flag that an operator turned off comes back
  as. In no case is an error raised into the product.
- **Tests and local runs**: the test double a test sets flags through, in process and with no network;
  the suite-wide setup that installs it for every test, whatever the environment holds, and gives it the
  values of the override variable; and that variable (`flags.override_env`).
- **Kill switch**: how an operator turns a flag off without a code change, how soon it takes effect
  (`flags.kill_switch_seconds`), and the runbook that documents it (`flags.runbook`).
- **Definitions and keys**: the format and place of the definitions in the repository
  (`flags.definitions`) and the command that validates them; who applies them, a person or the project's
  pipeline, never the factory; which is authoritative, the repository or the service, and how a difference
  between them is found (a dry run in the pipeline's job, or the list of definition changes in the release
  packet); from that, whether a removal waits for a person (`flags.removal`); the variables that hold
  secret keys and the shape of those keys (`flags.key_env`, `flags.key_patterns`).
- **Why the interface is not OpenFeature**: only when `standard` is `partial` or `none`; for each language
  without it, the reason.
- **Why no established system fits**: only when `source = "home-built"`.

## The packet and the confirmation

The checkpoint that opens the `spec` decision prints the flag lines in the packet: the `source`, the
decision record's id and title, the `standard`, `removal`, any `declined` text, the three numbers F-CHOICE
counted, every override a person has written in `[flags]`, and the line to type.

- `approve flags=<source>` confirms the system: the value is the proposal's `source`, so a person who
  approves has said which kind of choice it was, and settling refuses a plain `approve` (F-APPROVE). The
  value is stored in the decision's `values` ([[record:decision]]). The decision record becomes accepted
  and frozen ([[topic:spec-lock]]), and the next survey copies the table into `factory.toml` (F-FILL).
- `approve flags=decline -- <words>`, for an `existing` system only: the specification is approved, the
  code keeps its flags, and the factory does not manage them. Flags stay off.
- `revise -- <guidance>` asks for another proposal. Guidance that names a system makes the next proposal
  `named`. Guidance that says no flags makes the architect take the proposal out and write
  `enabled = false` with the person's words in `declined`.

A later approval of the same specification with the same decision record needs no value.

## Asked once

The accepted decision record that `flags.decision` names is the answer. While it stands, no workflow
asks again and the architect leaves the choice alone; a later story reads the configured keys and nothing
else. A decline stands the same way ([[config:flags]]). A person changes the system with a change
request: an amendment writes a new decision record that supersedes the old one, `spec` approves it with
its own `flags` value, and the next survey replaces the copied keys.

## A home-built system

It needs the explicit approval `flags=home-built`, the reason in the record, and everything the list
above asks of any system. Where an OpenFeature SDK exists for the product's language the home-built source
sits behind it as a provider, so `standard` is still `openfeature` and replacing the source later changes
the wiring and no read. Using OpenFeature is not rolling our own.

## OpenFeature

What the design relies on, from the OpenFeature specification (its flag evaluation and provider sections,
its types, its glossary and its appendix A):

- It is an open specification of a vendor-neutral API for evaluating flags, a project of the Cloud
  Native Computing Foundation. The product's code calls that API; a provider connects it to a flag
  system, which may be a commercial one, an open-source one or one built in-house.
- An evaluation call is typed (boolean, string, number or structure) and takes the flag key, a default
  value and an optional context.
- An evaluation call must return the default value on abnormal execution, for example when the provider
  reports an error or is not ready. With no provider set, the default value is returned.
- That is not a promise of the default whenever the source is unreachable. A provider has the status
  `STALE` when its cached state may no longer match the source, and `CACHED` and `STALE` are among the
  reasons a value can carry, so a provider that has lost its source may answer from what it last held.
  This is why the record states the "lost after the start" case.
- The function that sets a provider and waits returns when the provider is ready or in error, and the
  provider section sets no time limit on a provider's initialisation. This is why `startup` is a contract.
- It separates SDKs for servers, which evaluate for many users, from SDKs for clients, which evaluate for
  one. This is why the record says where flags are evaluated.
- An SDK should ship an in-memory provider, meant for tests. Appendix A also describes a provider that
  wraps several providers behind one client, which is how a second source is put behind the interface.

SDKs are published per language. Whether one exists for each language of the product, and how mature it
is, is looked up by the architect when it writes the proposal and stated under "Interface"; the design
does not carry a list.
