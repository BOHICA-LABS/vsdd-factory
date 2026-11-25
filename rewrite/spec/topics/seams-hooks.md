---
kind: topic
id: seams-hooks
sources: [hooks.md#4.1, hooks.md#4.2, hooks.md#5]
---

# Hook seams

The interfaces the hook design provides to and needs from the other designs, each seam linked to the
thing that carries it.

## What the hook subsystem provides

| Provided by | Where it lives now |
|---|---|
| The hook entry: `hooks/hooks.json` and the launcher's exit-code contract | [[topic:hook-wiring]], [[topic:hook-launcher]] |
| The rule model: one module exposing `check`, and one `RuleEntry` row in the registry, which is the declaration (HD1); case tables | [[topic:rule-model]] |
| Project configuration keys under `[hooks]` | [[topic:rule-tuning]] |
| Policy table formats (`walls.toml` with `[callers]` and `[reveals]`, `classes.toml` with the product-class precedence, `artifacts.toml` with the story kinds) | [[table:walls]], [[table:path-classes]], [[table:callers]] |
| Agent-keyed enforcement of walls on `Read`, `Grep`, `Glob`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Agent` and, heuristically, `Bash` | [[rule:read-walls]], [[rule:write-scopes]], [[table:walls]] |
| Step records and grounded findings | [[rule:step-results]] |
| Decision capture through the typed command | [[rule:decisions]] |
| Evidence-backed guards: red before source, frozen tests, governed merge | [[rule:tests-first]], [[rule:merge-guard]] |
| Spike enforcement: ledger entry, time box, no nesting, recorded finding, admission, asker's walls, containment, `spike_exempt` | [[rule:spike-bounds]] |
| Caller identity and the hook liveness check (no canary command) | [[topic:caller-identity]] |
| Artifact validation as a module, used before a write and by `vsdd validate` | [[rule:artifacts]] |
| The event log: writer, line format, schema version 1, `vsdd events schema` | [[topic:event-log]], [[schema:event]] |
| Block explanation: the block id in every message, and `vsdd explain` | [[topic:event-log]] |
| Session briefing delivery, with the state design's content and limit | [[rule:session-briefing]] |
| Pre-compaction flush with a 20 second limit; no snapshot file | [[rule:compaction]] |
| `vsdd rules` and `vsdd hooks selftest` | [[command:rules]], [[command:hooks-selftest]] |
| Environment switches | [[table:environment]] |

## What the hook design needs

From the state design: the project configuration file and the legacy-file test (S1); a git-ignored
directory for the runner's own records and the event log, both `write = "code"` (S2); configuration
keys for the seven product classes, `hooks.main_session_wall`, `hooks.rules.auto-allow.scope`, the
integration and release branches, the spike time box and the event log location (S3); the briefing
function and its length limit (S4); the content of `artifacts.toml` as the one copy (S5); the content
of `classes.toml` including stores for step records, evidence, decisions and the spike ledger, and an
answer to N-STATE-8 for `holdout` (S6); the stores as importable functions, each returning within one
second (S7) — the signatures are in [[topic:state-api]]; the pre-compaction flush, idempotent, at
most 20 seconds (S8); whether the state directory is a single instance at the project root (S9).

From the agent and skill designer: the content of `walls.toml` in the format of [[table:walls]] (A1);
every agent with a `read_deny` entry has only tools the runner can inspect (A2); agents that write
artifacts have `Edit` or `Write` (A3); the spike runner's file sets `isolation: worktree` and `decide`
exists as a plugin command a person can type (A4).

From the pipeline designer: the person-only and governed subcommands as constants and the story-kind
table (P1); every pipeline agent step names an agent type (P2); story worktrees and branch patterns
(P3); the decide command's grammar (P4); `spike open` and `spike record` write and read the ledger
entries the hook matches by `agent_id` (P5); no canary step or command (P6); the step-result and
finding field names (P7).

From the packaging designer: the minimum Claude Code version (K1); the launcher contract in place of
interface I3 for the `hook` entry (K2); `hooks.json` regenerated and checked, never edited by hand
(K3); the hook subcommands wired into the tool (K4); the observability stack reads the event log (K5);
`uv` documented as the one prerequisite (K6); no subcommand an agent can run changes `[hooks]` (K7);
the governed merge writes `pr.merged` and gates take the exit code from the command they ran (K8).

Three mismatches between the other designs that touch this one are settled by the blueprint: the tool
is `vsdd` and the governed commands are `story deliver` and `release`; `clerk` and `scenario-reviewer`
are roster agents with rows; and the tool writes the spike ledger. One correction to a test in
`packaging.md`: a `StructuredOutput` call does reach the runner; it passes when it carries no step
identity.
