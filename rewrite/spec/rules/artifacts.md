---
kind: rule
id: artifacts
package: H4
sources: [hooks.md#2.10, hooks.md#2.11.3, hooks.md#2.4, BLUEPRINT.md#5.4]
---

# artifacts

Validates a write whose result is a state artifact before the write lands: the content after the call
must be a valid artifact of its kind.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tools `Edit`, `Write`, `MultiEdit` |
| Result | `Block` |
| Fails | closed — "the write would land; the agent can simply retry once the fault is fixed" |
| Relax | yes |
| Spike exemption | yes — a spike writes no pipeline artifacts |

## How it works

The rule looks up the target path in `artifacts.toml` and validates `ctx.new_content`, the text the
file would contain after the call. Because it runs on `PreToolUse`, an invalid artifact never reaches
the disk. The old validators ran after the write, could only complain, and needed a marker file and two
more hooks to cope when they timed out.

The rows of `artifacts.toml`, including the worked rows and the `[_unknown]` policy, are
[[table:artifacts]]; there is one copy, which the state design owns and this rule reads.

## Checks and reason codes

| Check | Reason code |
|---|---|
| Path matches no known kind | `unknown_artifact_path` |
| The file cannot be read or decoded when the tree is checked as a whole (`vsdd validate`, `flush`) — a write-time check always holds the text, so this code arises only there | `unreadable` |
| The file is locked (below) | `artifact_frozen` |
| Frontmatter missing, outside the supported subset, or a required key absent or of the wrong type | `frontmatter_invalid` |
| Identifier does not match the pattern, the file name, or the directory `dir` derives from it; or the identifier is already taken (the message names the next free one) | `id_mismatch` |
| The status value is not one the kind's frontmatter lists. The lists hold only values an agent may set (`draft`, `retired`, `dropped`, and so on); `approved`, `ready`, `done`, `proven` and `accepted` are computed from records and are in no list, so an agent that writes one is refused by the ordinary frontmatter check. There is no separate `code_only_value` check | `frontmatter_invalid` |
| A reference is not a well-formed identifier of the referenced kind. Whether the artifact exists is not checked at the write, because stages write in an order (contracts before subsystems) and in parallel (stories by epic); `vsdd gate spec`, `vsdd gate plan`, `vsdd validate` and approval check existence | `reference_malformed` at the write; `dangling_reference` at a gate |
| A required heading is missing | `heading_missing` |
| Content exceeds the size cap (a write that makes the file smaller is always allowed) | `size_cap` |
| A forbidden pattern is present | `forbidden_pattern` |
| A table row has the wrong number of cells | `table_malformed` |

The `artifact_frozen` check, in full: either the file is in a specification class (`spec-product`,
`spec-arch`, `spec-ux`), new or existing, while the specification set is locked — the latest `spec`
decision is approved and no live run holds an amendment (`spec_locked(project)`;
[[topic:spec-lock]]). The lock is over the whole set and does not look at the file's status, so it also covers the files
that never had one. The `brief` class is outside it, and outside the `spec` decision's digest, because
a brief has its own decision. The two guidance files, `conventions.md` and `review-rules.md`, are
locked with the set while a cycle is open and writable by the architect between cycles
(`guidance_locked(project)`), so a retrospective's proposal can be adopted without an amendment run and
the rules cannot change in the middle of a build; they are outside the `spec` decision's digest, so
that edit leaves the approval current. Or the file is individually frozen whatever the amendment state:
a decision record an approved `spec` decision accepted, a proven property, a done story
(`artifact_locked(project, path)`). One record read per guarded write.

## Backstop

The same module backs `vsdd validate` ([[command:validate]]), which workflows and CI call at gates.
That is the backstop for anything that reached the disk some other way.

The frontmatter reader this rule uses accepts one `---` fenced block at the top of a file containing
scalars (string, integer, boolean, null), flow lists (`[a, b]`), block lists of scalars, and one level
of nested mapping. Anything else raises an error with a line number, which this rule reports as a
block (the reader is the one described in [[topic:hook-runner]]).
