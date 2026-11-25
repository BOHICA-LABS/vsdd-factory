---
kind: topic
id: result-schemas
package: C1
sources: [BLUEPRINT.md#4.10, pipeline.md#2.10, pipeline.md#2.10.1, pipeline.md#2.10.2, pipeline.md#2.10.4, pipeline.md#2.10.5, pipeline.md#2.10.6, pipeline.md#2.10.7, pipeline.md#2.10.8]
---

# Result schemas

The schemas live as files in `<plugin>/schemas/` and are the single source for the scripts, the tool and
the hook runner. The build generates the schema literals in the prelude from them, and the tool and hook
runner load them. Each one is a file here, starting at [[schema:step-result]].

Every shape is such a file, and package C1 delivers all of them: the step result; the data a step returns
([[schema:review-data]], [[schema:fix-data]], [[schema:ruling-data]], [[schema:holdout-data]],
[[schema:intake-data]], [[schema:implement-data]] and the others registered as schemas); the output of each
tool command a script relays ([[schema:survey]], [[schema:checkpoint-result]], [[schema:delivery-data]],
[[schema:gate-result]], [[schema:wave-gate-data]]); and the records ([[topic:machine-records]]). No shape is
defined only as a field table, and no `S_*` literal in `workflow-lib/prelude.js` is written by hand: the
build generates every one from its file, and `tools/build.py --check` fails on one that is not.

The scripts hold them as object literals composed without `$ref`, because the platform only promises a root
object with `properties`: the build inlines each file's `$ref` to its sibling when it generates the literal,
so the files compose freely and the literals never carry one.

Rules that apply across them:

- `data` in a step result is required (it may be empty). The script never reads a field the schema does not
  require: every field a script reads is required by that step's data schema.
- A tool command a script relays prints every field of its output schema on every exit, with an empty value
  (`""`, `0`, `false`, `[]`, `{}`) where it has nothing to say, an error included. Every field of those
  schemas is therefore required, and a clerk relaying an error invents nothing. When the clerk returns
  nothing twice, `relay()` returns that same empty object for the schema it was given, with `ok: false` and
  `error` set ([[topic:prelude-steps]]).
- The checkpoint's output is the workflow result's `status`, `progress`, `decisions` and `next`, typed as
  [[schema:workflow-result]] types them, plus `ok` and `error`: the script returns it as the workflow's
  result, and the drive loop tests `progress` as a boolean.
- Severity has one definition and one text: the table in `workflow-lib/severity.md`, stamped into every
  reviewer file, with the levels `critical`, `high`, `medium`, `low` ([[table:severity]]).
- Findings are grounded when stored: the tool adds `grounded` when it stores the finding
  ([[schema:finding]], [[topic:verdicts]]).
- A walled finding is carried as its walled key and severity only, except inside the holdout class
  ([[topic:walled-findings]]).
- The hook runner re-validates nothing the platform has already validated: the `step-results` rule reads only
  the fields it uses, whose names come from `<plugin>/schemas/step-result.schema.json` and
  `finding.schema.json`.
