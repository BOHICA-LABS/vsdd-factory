# Repair record for the rules area (hooks/rules group)

Written by the repairing pass over the rule files, the eight rule tables, `schemas/event.schema.json`
and the 15 hook/rule topics. It records the derivations and the disagreements between originals that
this pass met; the disagreements found at extraction time are in `conflicts/rules.md` (R1 to R10)
and are only referenced here. Nothing here resolves a disagreement by editing the originals.

## What was restored, and from where

- `topics/hook-launcher.md`: the `launch.sh` skeleton as written (byte-identical copy), replacing a
  pointer to the original document. Source: `hooks.md#2.2`.
- `topics/rule-model.md`: the `@rule` declaration with the `def check(ctx: Ctx, cfg: RuleConfig) ->
  Outcome | list[Outcome] | None` line and its comments, and the main-loop skeleton as written.
  Source: `hooks.md#2.5`.
- `topics/state-api.md` (new, registered in `registry.d/hooks.toml`): the state API's signatures and
  refusal behaviour — `store_step(project, result, agent_type, head)`, `settle_decision(project, id,
  choice, values, note, session_id)`, `spike_entry(project, agent_id, worktree)`,
  `spike_admit(project, session_id, dispatcher_type)`, `spike_attach(project, agent_id, cwd)`,
  `story_kind(project, story)`, `kind_has_red_gate(kind)`, `caller(key)`, `evidence(project, gate,
  subject)`, `spec_locked(project)`, `artifact_locked(project, path)`, `guidance_locked(project)`,
  `unit_tree(project, workflow, subject)`, `lock_seen`, `lock_release`, `status(root)`,
  `briefing(root, session_id, source)`, `vsdd.project.path_class(path)`, `vsdd.state.verdict(project,
  subject)` — each returning within one second except `flush`. Sources: `state.md#4`, `state.md#2.8`,
  `hooks.md#5`.
- `tables/walls.toml`: rebuilt in the original format (top-level `[main]`, `[other]`,
  `["vsdd-factory:<agent>]`, with `[callers]` and `[reveals]` in the same file), the holdout-evaluator
  row built from the complete statement instead of the "shortened here" excerpt, and the nine rows
  that `hooks.md#2.7` does not show derived again. Source: `hooks.md#2.7`, `agents-skills.md#2.9`,
  `BLUEPRINT.md#4.3`, with `state.md#2.16` for who runs each gate.
- `tables/event-types.toml`: `runner.call` added, from the `VSDD_FACTORY_HOOKS_DEBUG=1` sentence, so
  the table and `schemas/event.schema.json` list the same types.
- `tables/environment.toml`: every variable the hook design names. Sources: `hooks.md#2.2`,
  `hooks.md#2.6`, `hooks.md#2.11.10`, `hooks.md#2.12`, `hooks.md#2.14`, `hooks.md#4.1`,
  `BLUEPRINT.md#4.2`.
- `schemas/event.schema.json`: the documented line of `hooks.md#2.12`; the field list is exactly the
  example line plus the extra fields the type table names.
- `rules/read-walls.md`, `rules/write-scopes.md`, `rules/tests-first.md`, `rules/merge-guard.md`,
  `rules/spike-bounds.md`, `rules/decisions.md`, `rules/step-results.md`, `rules/session-briefing.md`,
  `rules/compaction.md`, `rules/gate-guard.md`, `rules/auto-allow.md`, `topics/hook-runner.md`,
  `topics/hook-wiring.md`, `topics/hooks-inert.md`, `topics/hook-failure.md`,
  `topics/rule-tuning.md`, `topics/tool-call-approval.md`, `topics/event-log.md`,
  `topics/rule-testing.md`, `topics/caller-identity.md`, `topics/information-walls.md`,
  `topics/hook-platform-facts.md`, `topics/seams-hooks.md`, `topics/risks-hooks.md`: prose copied
  from the sections in each file's `sources`, with change history struck.

## Derivation of the nine wall rows (audit F01)

`hooks.md#2.7` shows rows for 14 of the 22 callers. The nine agent rows it does not show were derived
from `agents-skills.md#2.9` (whose tables are declared to be the content of `walls.toml`), following
the defaults `hooks.md#2.7` states (absent `dispatch` means none; absent `commands` means `"*"`;
absent `vsdd` means the `read` group only), and the cross-roster rules of `hooks.md#2.7`,
`agents-skills.md#2.9` and `BLUEPRINT.md#4.3`:

- `product-owner` (`vsdd`-only shell per `BLUEPRINT.md#4.3`): `tool_only = true`; `outside` denied;
  `runs` denied; `holdout` not denied (it writes the scenarios); `write = ["spec-product",
  "holdout"]`; `vsdd = ["read", "measure"]` (it finishes with `vsdd gate spec --only <part>`;
  "read and measure commands only").
- `ux-designer` and `story-writer`: as `hooks.md#2.7` shows for the other `vsdd`-only planners, with
  their own write class (`BLUEPRINT.md#4.3`) and `vsdd = ["read", "measure"]`.
- `codebase-analyzer`: a planner with a shell (`BLUEPRINT.md#4.3`), so `outside` stays allowed;
  `write = ["analysis"]`.
- `researcher`: no shell (`BLUEPRINT.md#4.3`), so `outside` is denied.
- `demo-recorder` and `technical-writer`: builders; `story_only = true`; `commands =
  ["git-commit"]` (the command-scope table of `agents-skills.md#2.9` names all six builders for
  `git commit` inside a story worktree); `vsdd = ["read", "measure", "env"]` for the demo-recorder
  (`hooks.md#2.7`: its row holds `env`; `state.md#2.16`: `gate demo` runs by `demo-recorder`),
  `vsdd = ["read", "measure"]` for the technical-writer (a `docs` story ends at `gate green`,
  `state.md#2.16`).
- `code-reviewer`: `agents-skills.md#2.9` "every class in the state directory except `evidence` and
  `guidance`", encoded as the enumerated `read_deny` with both classes kept in `read_allow` (see
  `conflicts/rules.md` R10); `outside` denied (no shell).
- `security-reviewer`: no shell (`BLUEPRINT.md#4.3`), so `outside` denied; `write =
  ["reviews:create", "analysis:create"]` per `BLUEPRINT.md#4.3` (`conflicts/rules.md` R1).
- `ui-reviewer`: `vsdd = ["read", "env"]` (`hooks.md#2.7`: its row holds `env`); `outside` not
  denied (it has a shell and browser tools).

## Disagreements met during the repair

- R1 (security-reviewer's `analysis` grant): `BLUEPRINT.md#4.3` followed in `tables/walls.toml`; the
  narrower `agents-skills.md#2.9`/`state.md#2.4` wording stays in the owning files.
- R2 (the verifier's `commands` key): `hooks.md#2.7` followed — the shown row is quoted without the
  key, which defaults to `"*"`. The six-builder `git-commit` grant of `agents-skills.md#2.9` is
  applied only to the rows this pass derived (`demo-recorder`, `technical-writer`), where
  `agents-skills.md#2.9` is the declared source.
- R6 (computed classes three or four): `tables/path-classes.toml` lists four and quotes both
  sentences' rule.
- R7 (`runs` blanket denial): encoded per-row, with the blanket noted.
- R9, R10: see `conflicts/rules.md`; the wall table follows the choices recorded there.
- Decide event, auto-allow read-only list, event-log location: `conflicts/rules.md` R4, R5, R8; the
  files follow the detail documents as recorded there.

## Gaps (values the originals do not give)

- Whether the `read` group is implied when a row's `vsdd` key is present. `hooks.md#2.7` says only
  that a caller with no `vsdd` key may run `read`. The derived `demo-recorder` and `ui-reviewer` rows
  name `read` explicitly; the shown `holdout-evaluator` row is quoted as written (`vsdd = ["env"]`).
  One sentence in the original would settle it.
- The `purity-advice` side-effect-pattern default is "the old list", which is not reproduced in any
  of the six documents. `rules/purity-advice.md` states it as the original does.
- What, if anything, checks the `Task` tool, which the `PreToolUse` matcher names but no rule file
  claims. Unstated in the originals.
- A fallback for the `SessionEnd` retention prune if the event directory is too large for the 1.5 s
  budget. The originals name no fallback, unlike every other budget.
- The `_unknown` policy of `artifacts.toml` is defined only by the comment in the example. The
  example is carried as written; the key's semantics live in the comment.
