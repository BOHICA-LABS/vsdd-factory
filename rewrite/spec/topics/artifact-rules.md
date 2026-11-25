---
kind: topic
id: artifact-rules
sources: [state.md#2.4]
---

# Rules every artifact follows

## One registry

`<plugin>/py/vsdd/policy/artifacts.toml` ([[table:artifacts]]) is the single copy. The hook rule `artifacts`
reads it before a write, `vsdd validate` reads it for a whole tree, and `vsdd new` reads it to stamp a
skeleton ([[topic:templates]]). This design owns its content: one row per artifact kind, and the kinds are
the files under [[table:artifacts]] — the rows carry their paths, frontmatter keys, writers, caps and
lifecycles. The registry needs one column beyond the hook design's list: `template` (the skeleton file under
`<plugin>/templates/`). The story kinds, with whether each has a red gate, are a table in this file
too ([[table:story-kinds]]).

## Flat frontmatter

Every agent-written artifact starts with one `---` block of `key: value` lines. A value is a string, an
integer, `true` or `false`, a date, or a flow list such as `[BC-2.03.007, BC-2.03.008]`. There are no nested
maps and no multi-line values, and a line is at most 200 characters. This is inside the subset the hook
design's reader accepts. Only the keys the registry lists for the kind are allowed.

Frontmatter carries an identifier, a title, a status and the links to what the artifact depends on. It
carries nothing else.

## Identifiers

The path of an identified artifact is a function of its identifier. There are no slugs in file names, so
there is no rule about slugs staying fixed when titles change, and a hook can find a referenced artifact
without scanning a directory. The identifier schemes are [[table:identifiers]].

Identifiers are never renumbered and never reused. A retired artifact keeps its file with
`status: retired`, which is what keeps the number occupied. Because there is one state directory
([[topic:state-directory]]), an agent that picks a number already taken is blocked at the write and told the
next free one. `vsdd new <kind>` allocates for callers that have a shell.

## Links point one way

A contract names its capability and subsystem. A property names its contracts. A story names its contracts,
properties and the stories it depends on. An acceptance criterion names one contract in its heading,
`### AC-2 (BC-2.03.007)`. A scenario names its contracts. A decision record names the one it supersedes.
Nothing stores the reverse. "Which stories implement this contract" is a query (`vsdd refs BC-2.03.007`).
This removes the old propagation policies, the five-place contract retirement checklist and the agents that
enforced them.

## Size caps

Caps are in bytes and are enforced before the write (hook rule `artifacts`, reason code `size_cap`). A write
that makes a file smaller is always allowed. Body lines are at most 1,000 characters. A project may raise a
cap in `[caps]` to at most twice the default, and every raised cap is listed in `vsdd status`
([[config:caps]]). The block message says what to do instead, for example: "S-7.03 would be 17,412 bytes;
the cap is 16,000. Split the story."

## Who may write

Two mechanisms, both enforced by the hook runner from data and records.

1. **Path class.** `classes.toml` ([[table:path-classes]]) and the agents-skills design's write-scope table:
   each class has one writer role. A class with `write = "code"` cannot be written by any tool call.
2. **The lock.** The decision and closure records, through `spec_locked` and `artifact_locked`
   ([[topic:spec-lock]]): the specification set is locked as a whole, and individual files are frozen by
   their lifecycle.

## Deletion

Identified specification artifacts, stories and scenarios are never deleted; they are retired. Deferred
items and research may be deleted when they are resolved or superseded, because their names are slugs and
git keeps the history. The hook runner blocks every shell deletion inside the state directory, so deletion
goes through one command, `vsdd retire <path>` ([[command:retire]]), which deletes only in the classes
`deferred` and `research` and refuses anywhere else. The main session runs it in the `defer` and `research`
skills. Inside a workflow nobody has to: a research step returns the paths it superseded in
`data.superseded`, and the checkpoint retires them. Evaluations and demo files are write-once and are
removed only when the cycle closes ([[topic:cycles]]).

## Status values

Which status values an agent may write, and which are computed from records: [[topic:computed-status]].
