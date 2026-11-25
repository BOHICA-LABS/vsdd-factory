---
kind: topic
id: computed-status
sources: [state.md#2.4, BLUEPRINT.md#4.7, BLUEPRINT.md#5.2]
---

# Computed status

Status values live with the record that decides them. An agent sets only the statuses its artifact kind
lists: `draft`, `retired`, `dropped`, `stale`, `proposed`, `rejected`, `withdrawn`, and `active` for the
living kinds that have no approval step of their own (subsystem, screen, flow, scenario).

The values that mean "a gate or a person approved this" are not written into files at all, so there is one
list and it is per kind. They are read from the record that decides them:

| Computed status | Read from |
|---|---|
| A specification file is `approved` | The latest `spec` decision is approved and its digest covers the file as it stands |
| A decision record is `accepted` | An approved `spec` decision lists its id (the decision record keeps that list) |
| A property is `active`, then `proven` | It is in an approved specification set; a `verify` closure lists it as proven |
| A story is `ready`, then `done` | The plan's closure places it in a wave, or a run's closure records its admission; its delivery closure exists |

`vsdd list <kind>` ([[command:list]]) and `vsdd status` ([[command:status]]) show the computed status, so a
person asks the tool instead of opening the frontmatter. Code never rewrites a tracked file to say
`approved`, `ready`, `done`, `proven` or `accepted`.

Removing written approval statuses removes six code write paths into tracked files, the commit after each
promotion, the recovery path for an interrupted promotion, and the case where an amended file kept an
`approved` line over text nobody had approved: an amended file is simply not covered by the old digest
([[topic:digests]]). There is no separate `code_only_value` check in the hook rule, and no `code_values`
column in [[table:artifacts]].
