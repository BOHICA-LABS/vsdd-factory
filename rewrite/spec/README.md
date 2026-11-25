# The 1.0 design, one file per thing

This folder is the design for vsdd-factory 1.0. It replaces six large documents (kept unchanged under
`rewrite/history/design-v1/`) in which the same fact was written in several places, so that a correction made
in one place left the others wrong.

## The rules

1. **One fact, one place.** Each thing is defined in exactly one file. Any other file that needs it links to
   it and does not restate it.
2. **Every thing is in `registry.toml`**, once, under its kind, with a one-line summary and the build package
   that delivers it.
3. **Links are written `[[kind:id]]`**, for example `[[command:gate-red]]` or `[[agent:implementer]]`.
   A link must point at something in the registry.
4. **Files are small.** No file over 16 KB and no line over 400 characters. A table row that would be longer
   becomes a short list under a heading.
5. **Machine-readable where a program will read it.** Schemas are JSON Schema files. Rosters, walls, routes,
   limits and configuration keys are TOML tables. Prose explains; it does not duplicate the table.
6. **No history here.** Review findings, amendment notes and mappings from the old system live in
   `rewrite/history/`.
7. **When a package is built, its code, tests and schemas become the truth**, and its files here shrink to a
   pointer.

## Kinds, folders and file names

| Kind | File | What it defines |
|---|---|---|
| `doc` | `<id>.md` in this folder | `overview`, `decisions`, `conventions`, `platform` |
| `topic` | `topics/<id>.md` | An explanation that cuts across things: it links, it does not define |
| `command` | `commands/<id>.md` | One `vsdd` command. The id is the command's words joined by hyphens: `gate-red`, `story-deliver` |
| `record` | `records/<id>.md` | One kind of machine record the tool writes |
| `schema` | `schemas/<id>.schema.json` | One JSON Schema |
| `rule` | `rules/<id>.md` | One hook rule |
| `agent` | `agents/<id>.md` | One agent |
| `skill` | `skills/<id>.md` | One skill |
| `workflow` | `workflows/<id>.md` | One workflow |
| `artifact` | `artifacts/<id>.md` | One kind of project artifact (a contract, a story, a decision record, and so on) |
| `table` | `tables/<id>.toml` | One machine-readable table |
| `config` | `config/<id>.md` | A group of project configuration keys |
| `packaging` | `packaging/<id>.md` | Repository layout, manifest, build, tests, CI, release, cutover, operator tools |
| `package` | `packages/<ID>.md` | One build package: what it delivers and the files it implements |

## Frontmatter

Every Markdown file starts with:

```
---
kind: command
id: gate-red
package: G2
sources: [pipeline.md#2.9.1, state.md#2.16, BLUEPRINT.md#4.9]
---
```

- `package` is required for the kinds that get built (command, record, rule, agent, skill, workflow,
  artifact, config). Use `none` only for a thing that no package builds, and say why in the file.
- `sources` lists the sections of the original documents the file was taken from, written
  `<document>#<section number>`. The ids are in `rewrite/history/section-manifest.json`. This is how the
  restructure is shown to have lost nothing, and how a review finding that names an old section is traced to
  its new file.

A JSON or TOML file has no frontmatter; its registry entry carries the package, and a TOML file starts with a
comment naming its sources.

## Checking

```
python3 rewrite/tools/spec_lint.py            # names, links, packages, sizes
python3 rewrite/tools/sections.py coverage    # every original section is cited or retired
```

`retired.toml` lists original sections that are deliberately not carried over, each with a reason:
`history` (a finding table, an amendment list, a mapping from the old system), or `duplicate` with the id of
the section that holds the same text.
