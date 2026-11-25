---
kind: packaging
id: packaging-assumptions
sources: [packaging.md#2.13]
---

# Not yet proven

Each item is documented by the platform or assumed in the packaging design, and was not exercised by the first spike.
B8 or R0 settles each. Section 10 of the blueprint is the full list across all five designs; it adds, among others,
the Bash timeout and detached jobs, hook approval of permission prompts inside agents, the prompt-expansion event for
`decide`, a skill starting a workflow, hooks on MCP tools, compaction inside a step, and `! vsdd` in a session.

A second round of probes ran on 2026-11-23, and blueprint section 10 records its results: N1 is proven in the main
session, a dispatched agent and a workflow step, headless (`! vsdd` typed by a person is not, and the shim covers the
person); the others below are still open. Package B8 is limited to the items a toy probe can settle; the rest belong
to R0, the end-to-end packages and T5 ([[package:B8]], [[package:R0]]).

- **N1** — A file in `<plugin>/bin/` is on `PATH` for Bash calls in the main session, in an Agent-tool subagent, in a
  workflow step, and under headless `--plugin-dir`. How to test: conformance probe, `vsdd --version` in each of the
  four. If it fails: every agent file names the tool by `${CLAUDE_PLUGIN_ROOT}/bin/vsdd`, which the platform
  substitutes in agent and skill bodies.
- **N2** — A Claude Code below the `engines` range refuses the plugin with a clear message. How to test: install an
  older version in a clean configuration directory and load the plugin. If it fails: `vsdd doctor` and the session
  briefing check the version themselves ([[command:doctor]]).
- **N3** — `claude plugin validate`, `marketplace add`, `install` and `tag` run in CI with no credentials, and honour
  `CLAUDE_CONFIG_DIR`. How to test: R0 is the test. If it fails: `install-check` runs with a key, or is run by hand
  before each marketplace merge ([[packaging:install-check]]).
- **N4** — A `git-subdir` entry whose `ref` is a tag and which also sets `sha` installs correctly. How to test: R0, on
  the X1 pull request, which pins `v1.0.0-rc.25`. If it fails: pin by `sha` alone.
- **N5** — `claude plugin tag` works for a plugin in a subdirectory and produces `vsdd-factory--v1.0.0`. How to test:
  `--dry-run` on a clean tree; today it was only seen refusing a dirty one. If it fails:
  `git tag -a vsdd-factory--v1.0.0`.
- **N6** — A bundled server whose key is not configured stays quiet. How to test: load the plugin with no keys set
  and read the session's errors. If it fails: remove that server from `.mcp.json` and document a project-level entry
  instead.
- **N7** — A session starts cleanly when the `agent` setting names an agent that no longer exists. How to test: step
  X0. If it fails: the cutover tool removes the setting before the update, which it does anyway
  ([[packaging:cutover-projects]]).
- **N8** — Updating one project does not strand the other seven on a cache directory marked for cleanup. How to test:
  after the first `apply`, look for `.orphaned_at` in the rc.25 cache directory. If it fails: finish all eight inside
  fourteen days. The inert-on-old-state property covers a miss ([[topic:hooks-inert]]).
- **N9** — End-to-end runs can use cheap models. Either `CLAUDE_CODE_SUBAGENT_MODEL` and
  `CLAUDE_CODE_EFFORT_LEVEL` override a step's own `model` and `effort`, or the workflows accept a test switch. How
  to test: conformance probe. If it fails: settled by the blueprint — when `VSDD_TEST_TIER` is set, `vsdd survey`
  returns a tier override and the step wrapper uses it. Without either, the smoke tier costs several times more.
- **N10** — The harness's copy of the platform's behaviour is right. How to test: the conformance tier, one probe per
  row of the harness table. If it fails: fix the harness. A wrong row is a harness bug, not a design problem
  ([[packaging:tests-workflows]]).
