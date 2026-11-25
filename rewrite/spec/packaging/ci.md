---
kind: packaging
id: ci
package: B7
sources: [packaging.md#2.7, BLUEPRINT.md#2.3]
---

# Continuous integration

Decision D26 in `BLUEPRINT.md#2.3`: pay for end-to-end tests in CI — the smoke tier on pull requests that touch the
plugin, the story tier at each release and weekly.

`ci.yml` runs on pushes to the integration branch and on pull requests. All jobs except the last need no secret.

| Job | Runner | Does | Time |
|---|---|---|---|
| `static` | Ubuntu | `uv sync --frozen`, then `tools/check.py static`. | under 1 min |
| `unit` | Ubuntu and macOS, each on the Python floor and the newest stable CPython | `uv run --python <v> pytest tests/unit tests/hooks`. | 1 to 2 min |
| `workflows` | Ubuntu, Node from `.nvmrc` | `node --test tests/workflows/`. | under 1 min |
| `validate` | Ubuntu, Claude Code at the floor and at latest | `claude plugin validate --strict plugins/vsdd-factory`, and a comparison of what the validator says the plugin provides (`--json`) against the roster ([[table:roster]]). `--profile directory` runs as a non-blocking report. | under 1 min |
| `e2e-smoke` | Ubuntu, after the four above, only when the changed paths call for it | The smoke tier ([[packaging:tests-e2e]]). Needs the `ANTHROPIC_API_KEY` secret. On a fork, where there is no secret, it is skipped with a visible notice. In this repository a missing secret fails the job. | 5 to 10 min |

`e2e.yml` holds the story and conformance tiers, on a weekly schedule and on manual dispatch, and is callable from
the release workflow ([[packaging:e2e-story-tier]], [[packaging:plugin-release]]).

`semgrep.yml` is kept as it is, with the branch names updated; it now scans Python, JavaScript and shell.

The macOS leg of `unit` matters: the two `sh` files run under a different shell there.

A human step at cutover: if branch protection names the old jobs as required checks, the required list must be
changed to `static`, `unit`, `workflows` and `validate` in the repository settings (risk 13,
[[topic:risks-packaging]]).

Delivered by package B7 with `static`, `unit`, `workflows` and `validate` jobs ([[package:B7]]). During the rewrite
there is no CI; if the branch is pushed, one `next.yml` runs the check command
([[packaging:rewrite-layout]]).
