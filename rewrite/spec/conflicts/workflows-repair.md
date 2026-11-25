# Workflows repair — disagreements and gaps

Written by the fixing pass over `rewrite/spec/workflows/` and the `workflows`-group topics and tables
(`routes`, `preconditions`, `story-kinds`, `step-methods`). It records every disagreement between the
original documents met during the repair, and every gap (a value the originals do not give). Nothing
here is resolved; each entry names what the repaired files carry.

## Disagreements between originals

1. **Same-question match** — `pipeline.md#2.8` says the ledger check returns an earlier finding when
   "the same question already has a finding in the same unit"; `BLUEPRINT.md#6` says the earlier
   finding is "matched by a hash of the normalised question". Carried: the blueprint's parenthetical,
   in `topics/spikes.md` step 2. (Audit F09.)
2. **Experiment containment** — `pipeline.md#2.8`: "It cannot push, merge or write anywhere else."
   `BLUEPRINT.md#6`: "It cannot push, merge, dispatch agents or write anywhere else." Carried: the
   blueprint's form, in `topics/spikes.md` step 3. (Audit F28.)
3. **Spike retention enforcers** — `pipeline.md#2.8` names only `vsdd checkpoint` as remover of spike
   worktrees; `BLUEPRINT.md#6` names "`vsdd checkpoint` and `vsdd status --fix`". The retention rule
   (14 days, or until the asking unit is delivered; a named spike's `spike/<name>` branch kept until
   the spike is closed, per `state.md#2.17`) is carried in `topic:worktrees` only, per audit F10;
   `topics/spikes.md` links. Whoever owns `topic:worktrees` (group `commands`) must restore both
   rules there as written. (Audits F10, F28.)
4. **`spike open --step` syntax** — `pipeline.md#2.8` gives `--step <workflow>/<subject>/<name>`;
   `state.md#2.16` gives `--step <workflow>/<subject>/<name>.<attempt>`. Carried: the syntax is
   defined only in `command:spike-open`; `topics/spikes.md` links to it. (Audit F17.)
5. **docs, chore, harness "start at red"** — `pipeline.md#2.9.1` remarks these kinds start at red;
   `BLUEPRINT.md#4.6` gives them no tests stage and no red gate. Already recorded in
   `conflicts/workflows.md`; `tables/story-kinds.toml` follows the blueprint. Unchanged by this pass.
6. **Spike-limit granularity** — `pipeline.md#2.5` and `pipeline.md#2.8` carry two limit tables
   (per step/per run/per day; plus time box, nesting, questions per spike); `BLUEPRINT.md#6` carries
   one table whose rows additionally give "named spikes open | 3". The three tables agree in
   substance. Carried: `topics/spikes.md` holds the `pipeline.md#2.8` table; `table:limits` holds the
   `pipeline.md#2.5` rows; the named-spikes cap (`spikes.max_open`, 3) is in `topics/named-spikes.md`
   from the blueprint. `config/spikes.md` (group `records`) should carry only the keys.
7. **Route-table scoping** — `BLUEPRINT.md#5.1`'s amendment prose describes the scoping that
   `pipeline.md#2.4` states as rules; they agree. No action.

## Gaps (values the originals do not give)

1. The common precondition's text ("the project is ready and the hooks are alive for this call")
   exists only as the column header of the `pipeline.md#2.5` precondition table; no original states
   it as a first-class definition. Carried verbatim in the `tables/preconditions.toml` header with
   that source.
2. The per-step "positional" encoding of agent/method pairs in `tables/step-methods.toml` is this
   table's own format for the source table's semicolon lists; the original gives no encoding. It is
   documented in the header comment, not invented as data.
3. No gap was found in `tables/routes.toml`, `tables/story-kinds.toml`, the twelve workflow files or
   the remaining topics: every row, key, default and limit traces to `pipeline.md#2.3`–`#2.9.13`,
   `BLUEPRINT.md#3.3`–`#4.6`, `BLUEPRINT.md#5.1` and `BLUEPRINT.md#6`.

## Ownership decisions made for de-duplication (audit F22)

- **Hub model**: owned by `workflow:multi` (the source is `pipeline.md#2.9.11`); the state-layout
  side is `topic:multi-repo-state` (group `state`). `topics/multi-repo.md` now explains and links.
- **Spike retention**: owned by `topic:worktrees` (group `commands`), per audit F10.
- **Spike `--step` syntax**: owned by `command:spike-open` (group `commands`).
- **Ambiguous decision id**: owned by `topic:decisions` (group `records`, "Qualified identifiers");
  `topics/hotfix-cycle.md` links there instead of restating. `conventions.md`, `commands/decide.md`,
  `skills/decide.md` and `rules/decisions.md` also restate it; their owners should link.
- **Spike limit numbers**: `topics/spikes.md` (protocol enforcement) and `table:limits`
  (`pipeline.md#2.5` rows) mirror the originals' two tables; `config/spikes.md` should not restate
  the defaults.
- **Green/red gate pass conditions and `vsdd story deliver` behaviour**: carried in
  `workflows/story.md` and restated in `commands/gate-red.md`, `commands/gate-green.md` and
  `commands/story-deliver.md` (group `commands`); the commands should link to the workflow file or
  vice versa, one statement in the end.

## Sentences restored from small losses (audit F33)

- The `brief`/`author` skill method sentences and the `stubs`-into-`failing-tests` merge are restored
  in the `tables/step-methods.toml` header. The skill files (`skill:brief`, `skill:author`, group
  `agents`) may also want them.
- The wave/decision interplay sentence ("a story waiting on its own decision does not stop later
  waves...") was already carried in `workflows/wave.md`; the route header links to it.
