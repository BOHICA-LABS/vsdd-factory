---
kind: packaging
id: build-step
package: B4a
sources: [packaging.md#2.5, BLUEPRINT.md#5.5]
---

# The build step

A marketplace install copies the committed tree and runs nothing. So there is no build at install time, and anything
generated is committed (decision D4 in [[packaging:packaging-decisions]]). `tools/build.py` generates six things
inside the plugin, and `tools/gen_reference.py` generates the reference pages
([[packaging:reference-pages]]). `--check` on either fails on drift.

| Generated | From | Where it lands |
|---|---|---|
| The workflow prelude | `workflow-lib/prelude.js` | The stamped region of every workflow file |
| Schema literals (`S_FINDING`, `S_GATE` and the rest) | `<plugin>/schemas/*.schema.json` | A marked block inside the prelude, so scripts, tool and hook runner validate the same shapes |
| The `TIER`, `ROLE` and `OVERRIDE` tables | `tests/lint/roster.toml` (`[tiers]`, `[agents]`, `[overrides]`) | A marked block inside the prelude, so a workflow cannot use a model or an agent name the roster does not have, and the only step that runs off its agent's tier is one the roster lists |
| The agents' shared block | `workflow-lib/agent-shared.md` | A marked region of every agent file (`spike-runner` and `clerk` take the variants the source file defines) |
| The severity table | `workflow-lib/severity.md` | A marked region, under `## Severity`, of every reviewer file |
| `hooks/hooks.json` | `py/vsdd/hooks/registry.py` | The checked-in file, so a rule cannot be registered and left unwired. Built by package B4b, after the rule registry exists |

The build generates nothing for the hook runner from the workflows: step names are built at run time, so the hook
stores the agent type with each record and the tool checks it when a pass is counted. `[reveals]` in `walls.toml` is
generated from the class of each artifact kind in `artifacts.toml`, by the same build, under the same `--check`
([[topic:single-sources]]).

## 1. The workflow prelude

`workflow-lib/prelude.js` is the single source. `tools/build.py` copies it into a marked region of every file in
`<plugin>/workflows/`:

```js
export const meta = {
  name: 'deliver-story',
  description: '...',
  phases: [{ title: 'Recon' }, { title: 'Build' }, { title: 'Review' }],
}

// >>> stamped:prelude
...the prelude, verbatim...
// <<< stamped:prelude // generated from workflow-lib/prelude.js by tools/build.py; do not edit between the markers

// the workflow's own code starts here
```

The provenance note rides on the closing marker, and nothing may follow the opening marker on its line: the
build replaces everything between the two markers, so text after the opening marker would be rewritten away
and reported as drift.
Layout rule, kept simple so no JavaScript parser is needed: line 1 is `export const meta = {`, the object ends at
the first line that is exactly `}`, and the opening marker is the next non-blank line.

`uv run python tools/build.py` rewrites the region in every workflow file, inserting it if absent.
`uv run python tools/build.py --check` changes nothing and exits 1, naming each file, if any of these is false:

- every `.js` file in `<plugin>/workflows/` has exactly one opening and one closing marker, in that order and place;
- the text between them equals `workflow-lib/prelude.js` byte for byte;
- the stamped marker appears in no other file. The exact scan scope is what tools/build.py enforces: the toy workflows under `tests/workflows/toys/` carry stamped regions, and the build's own sources and tests quote the marker strings.

The check runs in `tools/check.py`, in the CI `static` job, and again in the release workflow, so a release cannot
carry a drifted copy ([[packaging:check-command]], [[packaging:plugin-release]]).

Constraints on the prelude, which the pipeline designer writes: declarations only (`const` and `function`); no
`import` or `export`; no `Date.now()`, `Math.random()` or argument-less `new Date()`; no reference to anything a
single workflow defines; a header comment listing every top-level name it introduces; at most 400 lines, measured after the two generated sub-blocks are stripped, so the cap bounds the code the designer writes. The build regenerates the two marked sub-blocks inside `workflow-lib/prelude.js` first, then stamps the assembled file into every workflow. A workflow
that redeclares one of the prelude's top-level names fails to load in the harness with a syntax error, which is how
collisions are caught. Script text is executed by the platform and is not put into any model's context, so stamping
the whole prelude into every workflow costs no tokens. The platform limits a script follows are in
[[topic:prelude]].

Alternative rejected: a separate source tree with an include directive and fully generated output. It leaves two
copies of every workflow in the repository and makes the shipped file something authors must not edit. With marked
regions the shipped file is the file you edit, and only the region is machine-owned.

## Nothing else is built

There is no per-platform variant and no version stamping: the version is edited by hand in one place,
`plugin.json`, in a release pull request ([[packaging:plugin-release]]).

The build is delivered by two packages: B4a (the prelude build: prelude, schema literals, tier, role and override
tables, stamped agent text, `--check`) and B4b (`hooks.json`, after the rule registry H1 exists) — the split breaks
a dependency cycle ([[package:B4a]], [[package:B4b]]).
