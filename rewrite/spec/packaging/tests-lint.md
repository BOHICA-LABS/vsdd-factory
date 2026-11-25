---
kind: packaging
id: tests-lint
package: B6
sources: [packaging.md#layer-3-lint-over-every-shipped-file-pytest-test, BLUEPRINT.md#5.5]
---

# Test layer 3: lint over every shipped file

Lint is ordinary pytest (`tests/lint/`) so that there is one runner and one report format. A file that fails a check
cannot merge.

The checks, by id and what each verifies, are the lint-checks table [[table:lint-checks]]: L-MAN, L-MCP, L-HOOK,
L-AGENT, L-METHOD, L-TEMPLATE, L-KIND, L-POLICY, L-SKILL, L-ROSTER, L-BUDGET, L-XREF, L-DEAD, L-PY, L-SH, L-SIZE,
L-DOC and L-OBS.

## The roster

`tests/lint/roster.toml` is the roster the lint compares against ([[table:roster]]): `[platform]` (the Claude Code
floor and the Python floor), `[tiers]`, `[agents]`, `[skills]`, `[workflows]`, `[overrides]` (the only steps that
run off their agent's tier; lint rejects any other) and `[budgets]`.

It is an expectation, not a registry: nothing reads it at run time, and it changes only when the roster changes.
Its job is to make the user's model policy and the context budget into failing tests. It is the one place where the
packaging design asks for a hand-kept list: each agent that starts pays for every listed skill.

The roster is the single source for the generated `TIER`, `ROLE` and `OVERRIDE` tables in the prelude and for agent
frontmatter ([[topic:single-sources]], [[topic:model-tiers]]).

Delivered by package B6, including the rule that every story kind's implement agent can write what its method
produces — L-KIND ([[package:B6]]).
