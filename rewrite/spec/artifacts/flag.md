---
kind: artifact
id: flag
package: F1
sources: [decisions.md#D31]
---

# Flag record

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D31; the capability
from end to end is [[topic:feature-flags]].

One file per flag: what is recorded about it, for every flag the product reads. The files together are the
flag register; there is no index. The kind is not yet a row of the shipped registry
([[table:artifacts]]). Package F1 adds this row, its skeleton `flag.md`, its `[reveals]` entry, and the two
things the registry's format lacks today: the `stem` cell, and the lock value `removed`.

```toml
[flag]
glob        = ".factory/stories/flags/*.md"
id          = '^\S(.*\S)?$'  # the flag key, as the code reads it; equals the frontmatter id
stem        = "the id with `/`, `%` and whitespace percent-encoded, so the path is a function of the key"
writer      = "story-writer; vsdd survey, for the flags the code already reads"
max_bytes   = 2000
template    = "flag.md"
lock        = "removed"  # frozen once the flag's computed state is removed
lifecycle   = "Living until removed; then frozen. The file is never deleted, so a key is never reused."

[flag.frontmatter]
id = "str"                                 # the key the code reads
title = "str"                              # what the flag controls, in one line
status = ["active", "dropped"]             # dropped: its owning story was dropped before delivery
type = ["boolean", "string", "number", "structure", "unknown"]
default = "str?"                           # the value every read passes as its fallback
origin = ["new", "inherited"]              # inherited: the code read it before a story introduced it
story = "ref:story?"                       # the owning story: the one that introduces the flag
lifetime = ["temporary", "permanent", "unknown"]
remove_when = "str?"                       # date:<yyyy-mm-dd> or after:<story or epic id>
permanent_because = "str?"                 # why the product keeps a permanent flag
default_because = "str?"                   # why a permanent flag's default is the safe value
repo = "str?"                              # the member repository, in a hub
```

## Where it lives, and why there

On the state branch, under `stories/`, in the path class `stories` ([[table:path-classes]]). A flag is
named when its story is planned, before any code exists, because the test-writer needs the key and the
default to write tests for both states. It has the writer of a story and the readers of a story, so it
needs no class and no wall of its own. The flag's definition is a product file and lives in the product
tree (`flags.definitions`, [[config:flags]]); the key ties the two, and the checks compare them at every
gate (F-USED and F-DEFINITION in [[table:flag-story-checks]]).

## The fields

- **Type.** One of the four kinds of value a flag can have. `unknown` is allowed only on an inherited
  record.
- **Default.** The value every read passes as its fallback, which is what the product gets when the flag
  source cannot be reached.
- **Owning story.** `story`, the story that introduces the flag: its diff adds the first read and the
  definition. Other stories ship behind the same flag by naming it in their own `flag` key, so a
  feature that takes several stories has one flag. An inherited record has no owning story.
- **Lifetime.** `temporary`: a release flag, which guards work until the work is on for everyone and is
  then removed. It is boolean and its default is `false`, so that off, unset and unreachable all mean the
  behaviour that exists. `permanent`: a kill switch or an operational setting the product keeps. It says
  why it is kept (`permanent_because`) and why its default is the value the product should run with when
  the source cannot be reached, a restart during an outage included (`default_because`). A new flag is
  temporary unless its record gives those two reasons. `unknown` is allowed only on an inherited record,
  and the sweep reports it until someone decides.
- **Removal condition.** `remove_when`, required for a temporary flag and refused for a permanent one.
  Two forms, both of which a program can evaluate: `date:<yyyy-mm-dd>`, met on or after that day; and
  `after:<story or epic id>`, met when that story, or every story of that epic that is not dropped, is
  delivered and the cycle that delivered the last of them has closed. It says when the work is complete,
  not that the flag is on: what else a removal waits for is in [[topic:flag-lifecycle]].

The plan gate holds a new record to these rules (F-RECORD, [[table:flag-checks]]).

## Computed, not written

Nothing writes a flag's state ([[topic:computed-status]]). It is `planned` while its owning story is not
delivered; `live` once that story's delivery closure exists, and from the start for an inherited record;
and `removed` once a delivered story removes it, as `flag_change` below says ([[record:closure]]). A live
flag is also `on` once a delivered story names it with `flag_change: enable` and the cycle that delivered
that story has closed: on in the repository's definitions, which is all the factory can know.

## The story keys

Package F1 adds four optional keys to the story's frontmatter ([[artifact:story]]):

- `flag`: the key of the flag the story concerns. With no `flag_change` the story ships behind it: its
  new behaviour is reachable only with that flag on. Allowed for a boolean flag, on a story whose kind
  has a red gate ([[table:story-kinds]]), because both states are evidenced by tests.
- `flag_change`: `enable`, `remove` or `withdraw`. The story does not ship behind the flag. `enable` turns
  the flag on in the repository's definitions (a `feature` story). The other two remove the flag, each in
  a `fix` story, and differ in the behaviour that stays. `withdraw` keeps what the product does with no
  flag set, the behaviour of the flag's default: for a release flag the off behaviour, so the feature is
  given up. `remove` keeps the behaviour of another value: for a boolean flag the other one, so a release
  flag's on behaviour, and for any other flag the value the story's criteria name
  ([[topic:flag-lifecycle]]).
- `flag_exempt`: one line saying why a story that `flags.require` asks to be behind a flag is not.
- `flag_irreversible`: one line naming what stays changed after the flag is turned off again (a
  migration, a new stored format, a message sent), and what restores the earlier state.

"A story behind a flag" in this design is a story with `flag` and no `flag_change`, and "a story that
removes a flag" is one with `flag_change: remove` or `withdraw`. The design keeps the word "flagged" for a
story's own boolean keys, such as `security`.

## Who writes and who reads

The story-writer writes a record with the story that owns it, and changes `title`, `type`, `default`,
`lifetime`, `remove_when` or the two reasons later when a person asks through
`/vsdd-factory:author flag` ([[skill:author]]). The survey writes the inherited records (F-FILL). The
hook rule `artifacts` checks every write against the row ([[rule:artifacts]]). Readers: every agent that
may read a story; the gates; `vsdd list flag`; the pull request body and the release packet.

The row accepts any key existing code may already read. A new flag's key is held to the tidy form
`^[a-z0-9][a-z0-9._-]*$` by the plan gate, so nothing the factory introduces needs encoding.
