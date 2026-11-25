# Second audit: the machine-readable files

Scope: every TOML file under `rewrite/spec/tables/` (21) and every JSON file under `rewrite/spec/schemas/` (47),
checked row by row, key by key, against the six originals. Line numbers are those of the files on `rewrite/v1`
at the time of this audit. Nothing was fixed.

## Verdict

Not yet faithful. 68 files checked, 33 fully faithful: 6 tables (`callers`, `methods`, `preconditions`,
`routes`, `severity`, `story-kinds`) and 27 schemas (the seven of `pipeline.md#2.10.1`-`#2.10.7`, byte-equal,
plus `review-data`, `fix-data`, `ruling-data`, `holdout-data`, `claim`, `pending-output`, `outline-data`,
`module-map-data`, `score-data`, `cycle-record`, `baseline`, `decision-record`, `run-record`, `job-record`,
`lock`, `driver-flag`, `config-fingerprint`, `spike-admission`, `env-result`, `release-result`). Every schema
passes Draft 2020-12 `check_schema`. 26 findings: 5 blockers, 12 major, 9 minor.

## T2-01. The roster's `[workflows]` drops four call edges (blocker, altered)

- Originals: `agents-skills.md#2.12` line 901: `wave = { calls = ["story"] }    # and so on for feature, verify,
  maintain and multi`. `pipeline.md#2.3` line 251: "`wave`, `feature`, `verify`, `maintain` and `multi` may call
  `story`." `BLUEPRINT.md#5.1` line 867: "shared by the five workflows that call `story`".
- New: `tables/roster.toml` lines 64, 68, 70, 71 give `feature`, `verify`, `maintain`, `multi` `calls = []`; the
  comment at line 58 reads `BLUEPRINT.md#4.5` ("Only `story` is ever called") as "no other call edges".
  `conflicts/people-repair.md` P-8 records the same misreading.
- Resolution: `calls = ["story"]` for those four rows; correct P-8.

## T2-02. The artifact registry's `[story]` row is the illustrative excerpt (blocker, lost, defined twice)

- Originals: `hooks.md#2.11.3` line 1057: `[story]  # illustrative; the content is state.md section 2.5`.
  `state.md#2.5` line 565 gives the frontmatter `cycle`, `method`, `depends_on`, `properties`, `points`,
  `priority`, `criticality`, `security`, `ui`, `demo`, `screens`, `repo`, `source`, the writer and the
  lifecycle. `state.md#2.4` line 404: "Only the keys the registry lists for the kind are allowed."
- New: `tables/artifacts.toml` line 48 lists only `id`, `title`, `status`, `kind`, `contracts`; no row from
  `state.md#2.5` follows. `artifacts/story.md` lists all keys, so the two statements disagree; a story with
  `depends_on` would be refused `frontmatter_invalid`. The `[contract]` row (line 29) also lacks writer and
  lifecycle.
- Resolution: build `[story]` from `state.md#2.5` in the row format, keeping the excerpt's `forbid`/`tables`.

## T2-03. The code-reviewer's wall leaves `local` readable (blocker, altered)

- Originals: `agents-skills.md#2.9` line 578: "every class in the state directory except `evidence` and
  `guidance`"; `BLUEPRINT.md#4.3` lines 327-328. `state.md#2.4` lines 519-520: "`guidance` inside `spec-arch`
  ... and `runs` and `evidence` inside `local`".
- New: `tables/walls.toml` line 152 enumerates every state class except `guidance`, `evidence` and `local`.
  `.factory/local/` (lock, jobs, caller markers, events, media) is readable. `conflicts/rules.md` R10 says the
  two allowed classes are "siblings" of no denied class; `state.md#2.4` says they are contained.
- Resolution: add `local` to `read_deny`; correct R10.

## T2-04. The limits table lost the `ci_minutes` mapping (blocker, lost)

- Originals: `BLUEPRINT.md#4.12` line 697 and `state.md#2.12` line 1217 map prelude name `ci_minutes` to
  `delivery.ci_wait_minutes` (30); the survey "returns limits under the prelude's names".
- New: `tables/limits.toml` has `ci_minutes` only in the header comment (line 3); the delivery row (line 63)
  carries `name = "delivery_rounds"`. `conflicts/records-repair.md` RR7 says the row was given the
  blueprint's names; it was not. Line 86-87 pack two names and three keys into one string each, so no row
  maps `spikes.per_day` or a single name.
- Resolution: one entry per prelude name (`ci_minutes` with key and default only, nothing invented); split
  the spike row's names and keys into lists.

## T2-05. The holdout class lost its block-message route (blocker, lost)

- Original: `hooks.md#2.7` line 579: `route = "Holdout scenarios are written by the product owner:
  /vsdd-factory:author holdout"`.
- New: `tables/path-classes.toml` line 68 has no `route`; the sentence occurs nowhere under `rewrite/spec/`.
- Resolution: copy the key into the row.

## T2-06. `path-classes.toml` changed the `classes.toml` format (major, altered)

- Original: `hooks.md#2.7` lines 576-621: top-level `[holdout]`, `[config]`, ... and `[_product]`; lines 636 and
  756: the computed classes are "computed, not listed".
- New: `tables/path-classes.toml` uses `[path-classes.<class>]` (line 17 on), `[product]` (line 165), and adds a
  `[computed]` list (line 172). The header claims "the content of classes.toml and nothing else".
- Resolution: the original layout (`[<class>]`, `[_product]`); computed classes as a comment, R6 kept.

## T2-07. `artifacts.toml` has two row structures (major, altered)

- Original: `state.md#2.4` line 376: the registry is "in the format of `hooks.md` section 2.11.3" (`glob`, `id`,
  `dir`, `frontmatter = {...}`, `lock`, `headings`, `max_bytes`, `template`).
- New: the two worked rows use that format; the 29 rows from line 61 use `path`, a prose `frontmatter`
  string, `writer`, `cap`, `cap_note`, `lifecycle`. Multi-valued cap cells are folded into one number:
  `[demo-evidence]` line 230 `cap = 1500000` is the per-story total, not the 8,000-byte script cap.
- Resolution: one structure for every row; where a cell has several caps keep all, not one.

## T2-08. Every artifact kind is still defined twice (major, earlier finding not fixed)

- Audit F25 asked for every row in the table "and have the artifact files link to it". 30 of the 31
  `artifacts/*.md` files still restate path, frontmatter, writer, cap and lifecycle (`- Cap:` lines). T2-02
  shows the copies already disagree.
- Resolution: the artifact files keep their prose and link to their `[[table:artifacts]]` row.

## T2-09. Kind ids that disagree with the originals' kind vocabulary (major, altered)

- Originals: `hooks.md#2.7` lines 747 and 810 use the list kind `scenario` (`"list scenario"`, `vsdd list
  scenario`); line 769: `[reveals]` is "generated from the kind's class in `artifacts.toml`, so the kind
  vocabulary is the registry's". `/vsdd-factory:author conventions` (`state.md#2.5` line 605).
- New: `tables/artifacts.toml` names the kinds `holdout-scenario` (line 204) and `project-conventions`
  (line 110); the `[reveals]` key `"list scenario"` in `tables/walls.toml` matches no kind.
- Resolution: record the disagreement for the registry owner; one vocabulary for kinds and `[reveals]`.

## T2-10. Invented identifier examples (major, invented)

- Original: `state.md#2.4` lines 416-433 give one example per row (`CAP-012`, `E-NET-001`, `SCR-004`, `AC-2`,
  `SP-007`, `reviews/2026-11-26-security-2.md`, none for Finding).
- New: `tables/identifiers.toml` splits rows and adds `ENT-004`, `DI-002`, `EVT-007`, `DEC-003`, `ASM-005`,
  `R-009`, `FM-001`, `NFR-011`, `FLOW-002`, `EC-1`, `ID-006`, `CR-003`,
  `holdout/evaluations/auth-sso/story.S-7.03-1.json`, `code:src/parser/parse.rs:null-check` and
  `walled:9c1d2e3a4b5f`; none occurs in any original. `merge.S-7.04` and `blocked.S-7.04`
  (`BLUEPRINT.md#4.8` line 530) are dropped.
- Resolution: keep only the originals' examples; leave split rows without one.

## T2-11. List fields that are not arrays (major, altered)

- Originals name plural lists in `pipeline.md#2.10.8` (lines 1531-1541), `state.md#2.5`, `#2.7`, `#2.16`.
- New, with no `"type": "array"`/`items`: `story-draft-data.units`, `intake-data.covers`, `route-data.stories`
  and `.readiness`, `sweep-data.findings` and `.fixable`, `survey.done`, `.units`, `.decisions`,
  `story-unit.bcs` and `.acs`, `wave-gate-data.stories` and `.fix_units`, `delivery-data.changed_files`,
  `checkpoint-result.decisions`, `repos-split-result.repos`, `holdout-evaluation.scores`, `.symptoms`,
  `.walled_findings`, `delta-data.cross_repo`, and `status.decisions`, `.spikes`, `.others`, `.notes` (which
  also drop the stated limits 12, 8, 3 and 5 that `notes.schema.json` encodes). `review-data.findings` is an
  array of `finding.schema.json`; the same field in `sweep-data` is untyped.
- Resolution: wrap each as an array of what the original names, as F24 did for the record lists.

## T2-12. Two schemas say "X plus" but do not include X (major, lost)

- Originals: `pipeline.md#2.10.8` line 1539, wave gate data: "The gate result fields, and ...";
  `state.md#2.8` line 740 and `#4` S7 `store_step(project, result, agent_type, head)`.
- New: `schemas/wave-gate-data.schema.json` and `schemas/step-record.schema.json` list only the additions,
  so the gate result's and step result's required fields are not checked.
- Resolution: `allOf` with `gate-result.schema.json` and `step-result.schema.json`.

## T2-13. The configuration key list is defined twice, and the copies differ (major, defined twice)

- `tables/config-keys.toml` and `schemas/config.schema.json` both enumerate every key. The schema (line 298)
  lets `attribution` and `allow_pr_close` appear under every rule; the table ties them to `bash-git` and
  `bash-destructive`. `config/holdout.md`, `config/commands.md`, `config/state.md`, `config/evidence.md` and
  `config/maintain.md` restate meanings and one default that the table holds.
- Resolution: one key list; the schema generated from it or reduced to structure; config files link.

## T2-14. The configuration schema accepts what the originals call errors (major, altered)

- Originals: `state.md#2.12` line 1040: "Unknown keys and wrong types are errors"; `hooks.md#2.6` line 556:
  "An unknown rule id, an unknown key or a bad value under `[hooks]`" logs `config.invalid`.
- New: `schemas/config.schema.json` lines 286-305: any rule id is accepted and a rule table has no
  `additionalProperties: false`; no leaf key has a type, though the table states each one.
- Resolution: rule ids from `table:rules`, closed rule tables, and the table's types on the leaves.

## T2-15. `delivery.protected_paths` ignores the precedence rule, unrecorded (major, altered)

- Originals: `BLUEPRINT.md#4.12` line 721 and `pipeline.md#2.6` line 479: "`.github/**`, `.env*`,
  `CODEOWNERS`, and the files `[release]` names". `state.md#2.12` line 1095 adds the literal `CHANGELOG.md`.
- New: `tables/config-keys.toml` line 233 follows `state.md#2.12`, while `delivery.mode` and `delivery.ci`
  follow section 4.12 (RR2). No conflicts file records this one.
- Resolution: the blueprint's default; record both texts in `conflicts/records-repair.md`.

## T2-16. The environment table is incomplete and labels its own "who" (major, lost, invented)

- Originals: `$VSDD_PORT` (`state.md#2.12` line 1066, `#2.16` line 1437, `BLUEPRINT.md#4.12` line 715);
  `CLAUDE_PLUGIN_OPTION_*` exported to hook processes (`packaging.md#2.4` line 361); `CLAUDE_PLUGIN_ROOT`
  and `CLAUDE_PROJECT_DIR`, read by the guard command (`hooks.md#2.3` line 321).
- New: `tables/environment.toml`, whose summary is "every environment variable the plugin reads or sets"
  and whose sources include `state.md#2.12`, has none of them. Lines 17, 21, 25, 37 give "Debug switch.",
  "Test capture.", "Test override.", "Test switch.", which no original says.
- Resolution: add the rows from the cited text; leave `who` empty where unstated.

## T2-17. Key names invented against the recorded gaps (major, invented)

- `conflicts/records.md` part 2 (lines 99-113) records that the originals give no key spelling for the
  evidence record's "git patch-id", the caller marker's "time" and the shape of `notes.json`. Yet
  `evidence-record.schema.json` names `patch_id`, `caller-marker.schema.json` names `time`, and
  `notes.schema.json` wraps the entries in an invented top-level `notes` key.
- Resolution: leave the unnamed fields out (description only), as `run-record` does.

## T2-18. History and pointers in the limits header (minor, history leaked)

- `tables/limits.toml` lines 5-6: "the pipeline design left to its sections 2.7 and 2." (a truncated pointer
  into an original) and "the three spike rows" (there are two). Cells gain periods unevenly.
- Resolution: strike the sentence; copy cells as written.

## T2-19. Configuration meanings and defaults paraphrased (minor, altered)

- `tables/config-keys.toml` rewords its sources: `paths.harness` (line 168), `holdout.*` (line 361 on, from
  `pipeline.md#2.7` lines 636-637), the `state.*` keys, `caps.<kind>`, `hooks.log_retention_days`.
  `auto-allow.scope` (line 617) paraphrases "set at setup"; `allow_pr_close` (line 638) has `default = ""`
  where RR4 says none, yet `bash-git.attribution` takes the example's `"block"`; line 597 joins five keys.
- Resolution: quote or leave empty; one key per row.

## T2-20. The event schema lost `cwd` (minor, lost)

- Original: `hooks.md#2.12` line 1476: `agent.start`, `agent.stop` add `agent_type`, `agent_id`, `cwd`.
- New: `schemas/event.schema.json` has no `cwd`; the `id` description names `rule.block` and `rule.advise` but
  not `rule.would_block`, which line 1477 gives the same field.
- Resolution: add the property; complete the description.

## T2-21. Prose tokens in the hook tables (minor, altered)

- `tables/events.toml`: `"all gates"` as a rule id, "the advisor" (`hooks.md#2.3` line 305) as `purity-advice`,
  `timeout_seconds = "platform budget of 1.5"` in an integer column. `tables/rules.toml`: invented tokens
  `any-guarded`, `web-and-research`, `any-guarded-for-lock-liveness`. `tables/event-types.toml`: `runner.call`
  given "(extra: ms)", which `hooks.md#2.12` line 1492 does not state.
- Resolution: copy the cells; record any normalisation in `conflicts/hooks-repair.md`.

## T2-22. Closure-record descriptions say more than the original (minor, altered)

- `schemas/closure-record.schema.json`: `verdict` is "the fields of verdict.schema.json", though R1 records
  counts; "Per-worktree additions"; no `baseline` property, though `state.md#2.8` line 779 gives the ingest
  closure record one; the wave and `verify.json` additions (line 772) are not mentioned.
- Resolution: quote `state.md#2.8`; add `baseline`, referring to `baseline.schema.json`.

## T2-23. An unrecorded disagreement behind the claim schema (minor, other)

- `pipeline.md#2.10.8` line 1524: a holdout claim has "`key`, `resolution` and `commit` only";
  `BLUEPRINT.md#4.10` line 615: "carries the key alone". `schemas/claim.schema.json` follows the blueprint
  correctly, but no conflicts file records the two texts.
- Resolution: record it.

## T2-24. Decision kinds: paraphrased notes (minor, altered)

- `tables/decision-kinds.toml`: the notes of `spec`, `release`, `spec-gap` and `blocked` reword
  `BLUEPRINT.md#4.11` and `state.md#2.8`; `spec-gap.opened_when` takes `pipeline.md#2.6` line 490 over the
  blueprint's line 651 without a record.
- Resolution: quote one source per cell; record the choice.

## T2-25. Step-methods rows break their own pairing rule (minor, other)

- `tables/step-methods.toml` header: "positional" pairs `agents[n]` with `methods[n]`. The `ingest`
  disposition row and the `maintain` risks row are "positional" with one agent and two methods; the
  `discover` research row lists `"market-intel or discovery"` as a method.
- Resolution: repeat the agent, or drop "positional"; keep the alternative in the note.

## T2-26. Small cells invented or repointed (minor, invented)

- `tables/artifacts.toml` line 152: a `[dependencies]` frontmatter cell no original gives (S6 says none);
  line 256 turns "the fields of `pipeline.md` S6" (the ledger) into "the spike-finding schema"; cells keep
  pointers such as "(`pipeline.md` S11)".
- `tables/path-classes.toml` line 23 "appends member repositories" for "appends `[[repo]]` entries"; the
  `holdout-evaluations` and `runs` writers lose their last sentences.
- `tables/command-families.toml`: "in no family" for "in no git family"; `[callers]` linked to
  `[[table:callers]]`, not `[[table:walls]]`. `tables/lint-checks.toml` L-METHOD keeps "pipeline.md 2.9.13".
- Resolution: copy the cells; replace pointers with links.
