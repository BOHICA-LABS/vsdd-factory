# Conflicts and gaps found while extracting the rules group

Written by the rules extractor (the 17 rule files, the 8 rule tables, `schemas/event.schema.json` and
the 15 rule topics). Nothing here is resolved by editing the originals. Each disagreement gives both
section ids and both texts, and which text the new files use. The rule applied: section 4 of the
blueprint, then the rest of the blueprint, then the detail document that owns the subject.

## Part 1: disagreements between documents

### R1. The security-reviewer's `analysis` grant

- `BLUEPRINT.md#4.3` (roster table, row 16): "`reviews` and `analysis`, create only".
- `agents-skills.md#2.9` (write scopes): "`reviews`, creating new files only, in an on-demand review.
  `security-reviewer` may also create `analysis/<scope>/security.md` during an ingest."
  `state.md#2.4` (class `analysis`) agrees with the narrower reading: "codebase-analyzer;
  security-reviewer may create `security.md`".
- Used: `BLUEPRINT.md#4.3` (section 4 is the authority). Encoded in `tables/walls.toml` as
  `write = ["reviews:create", "analysis:create"]`. If the narrower grant is intended, the walls row
  and the blueprint row need one correction between them.

### R2. The verifier's git command families

- `hooks.md#2.7` (the `walls.toml` excerpt): the `verifier` row carries no `commands` key, and the key
  table says `"*" means any, and is the default when the key is absent` — so the verifier may run
  every family, `git-push` and `git-integrate` included.
- `agents-skills.md#2.9` (command scopes): "`git commit` inside a story worktree | `test-writer`,
  `implementer`, `verifier`, `demo-recorder`, `technical-writer`, `devops`" — the verifier only
  commits.
- Used: `hooks.md#2.7` as written (the row is quoted verbatim, no `commands` key), because hooks.md is
  the owner of the wall-table format and the agents-skills tables are "proposals to the hook design".
  Consequence worth checking: on the hooks.md reading only `merge-guard` stands between the verifier
  and a push.

### R3. Who may run `git push` and friends in Bash

- `agents-skills.md#2.9` (command scopes): "`git push`, `gh pr create`, worktree add and remove,
  tagging | The main session when a person asks. Inside the pipeline no agent runs them: `vsdd story
  deliver`, `vsdd survey` and `vsdd release` do."
- `hooks.md#2.7`: `[main]` has `commands = ["*"]` with the comment "merge-guard still blocks commits
  and pushes on the integration and release branches" — any family, gated by branch, not by family.
  No agent row shown grants `git-push`.
- Used: `hooks.md#2.7` (the format owner). The phrase "when a person asks" is instruction, not
  enforcement, and is not mechanised anywhere.

### R4. The read-only command list of the `extended` auto-allow scope

- `hooks.md#2.11.10`: "`ls`, `cat`, `head`, `tail`, `wc`, `rg`, `grep`, `find` without `-exec`,
  `-delete` or `-ok`, and the `git-read` family."
- `BLUEPRINT.md#5.4` (second pass): "a fixed list of read-only commands (`ls`, `cat`, `head`, `wc`,
  `rg`, `find` without `-exec` or `-delete`, and the reading git commands)".
- Differences: the blueprint omits `tail` and `grep`; the blueprint's `find` qualifier list omits
  `-ok`.
- Used: `hooks.md#2.11.10` (the detail document that owns the rule; it is also the longer list).
  The blueprint's summary should be aligned to it.

### R5. Which event carries the decide command

- `hooks.md#2.11.5`: "I use `UserPromptExpansion`, where `pipeline.md` names `UserPromptSubmit`, for
  one reason: the hooks documentation says `UserPromptSubmit` also fires for scheduled tasks,
  background subagent reports and messages from other sessions... The `UserPromptSubmit` fallback is
  therefore not built."
- `pipeline.md#2.6` names `UserPromptSubmit` for human stops (as reported by hooks.md; not checked
  directly in this extraction).
- Used: `hooks.md#2.11.5` (`UserPromptExpansion`), which the blueprint's `hooks.md#2.16` platform-facts
  row supports as proven headless. Recorded here so `pipeline.md`'s naming is not "fixed" silently.

### R6. The count of computed path classes

- `hooks.md#2.7`: "Three classes are computed, not listed: `own-worktree` is the git worktree that
  contains the caller's `cwd`; `project` is every path outside the state directory; `tmp` is the
  system temporary directory and is always writable."
- Later in the same section: "`outside` is a computed class: every path outside the project, the
  plugin directory and the temporary directory."
- Used: both sentences as written; `tables/path-classes.toml` lists four computed classes
  (`own-worktree`, `project`, `tmp`, `outside`). The "Three" sentence is understated by one; the fix
  belongs in hooks.md's successor or in the table, not both.

### R7. The `runs` class and the blanket denial

- `hooks.md#2.7`: the `runs` class comment reads "and every agent row denies reading it", and each
  shown row lists `runs` in `read_deny` explicitly; `[main]` too.
- `agents-skills.md#2.9` (read walls prose): "In addition to its row, every caller is denied `runs`,
  the main session included."
- Not a factual disagreement (both deny `runs` to everyone), but the mechanism differs: a per-row
  column in one, a blanket rule over the rows in the other. `tables/walls.toml` encodes the per-row
  form and notes the blanket. Whoever fixes one document should fix the other.

### R8. Event log location: working name vs fixed path

Already recorded in `conflicts/index.md` as C6 (`BLUEPRINT.md#4.1` fixed path vs `hooks.md#2.12`
"the directory the project configuration names... `VSDD_EVENTS_DIR` overrides it"). Restated here
because it binds the rules: the event-log directory must be in a `write = "code"` class (hooks.md
need S2), so if the configurability is kept, `tables/path-classes.toml` needs a movable class.
Used in `topics/event-log.md`: both texts, side by side.

### R9. Where the wall-table row content comes from for nine agents

`hooks.md#2.7` shows the `walls.toml` format and fourteen rows (`main`, `other`, `implementer`,
`test-writer`, `verifier`, `devops`, `architect`, `story-writer`, `adversary`, `holdout-evaluator`,
`scenario-reviewer`, `clerk`, `spike-runner`, and the `[callers]`/`[reveals]` tables), but no rows for
`product-owner`, `ux-designer`, `codebase-analyzer`, `researcher`, `demo-recorder`, `technical-writer`,
`code-reviewer`, `security-reviewer`, `ui-reviewer`.

- `agents-skills.md#2.9` carries those agents' walls in prose tables, and says "the two tables above
  are the content of `walls.toml`, one row per agent type, in the format of `hooks.md` section 2.7".
- Used: hooks.md rows where shown; for the other nine, `tables/walls.toml` translates the
  `agents-skills.md#2.9` tables (with `BLUEPRINT.md#4.3` for disagreements per R1), following
  hooks.md's statement of the defaults ("Absent means none" for `dispatch`; `"*"` for `commands` when
  absent; "Absent means `read` only" for `vsdd`). Recorded as a gap below because the encoded rows for
  those nine agents are a translation, not a quotation.

### R10. Code-reviewer's wall expressed two ways

- `agents-skills.md#2.9`: "every class in the state directory except `evidence` and `guidance`...
  In `walls.toml` the two exceptions are the row's `read_allow`."
- Correction (second audit, T2-03): an earlier version of this record said `evidence` and `guidance`
  were "siblings" of no denied class. `state.md#2.4` says they are contained: `guidance` inside
  `spec-arch`, and `runs` and `evidence` inside `local`. The deny list must therefore name `local`
  (and the containing classes) as well, or `local` — locks, jobs, caller markers, events, media — is
  readable.
- Used in `tables/walls.toml`: `read_deny` lists every state-directory class except `evidence` and
  `guidance` — `local` included — and `read_allow = ["evidence", "guidance"]` is kept as belt and
  braces, as `agents-skills.md` describes. One of the two documents should be corrected to describe
  the enumerated-deny form.

## Part 2: gaps noticed while copying

- **Wall rows nine agents wide are nowhere in the format owner.** (Part 1, R9.) Until `walls.toml`'s
  full row set exists in hooks.md's successor, the translated rows in `tables/walls.toml` are the only
  machine form.
- **Whether `read` is implied when a row's `vsdd` key is present.** hooks.md#2.7: "A caller with no
  `vsdd` key may run the `read` group only" — but for `holdout-evaluator`, whose row is
  `vsdd = ["env"]`, it is unstated whether `vsdd status` (read group) is allowed. The
  `demo-recorder`/`ui-reviewer` rows are encoded as `vsdd = ["read", "env"]` on the reading that read
  must be named; the `holdout-evaluator` row is quoted as written, `vsdd = ["env"]`. One sentence in
  hooks.md settles it.
- **The `purity-advice` side-effect pattern default is "the old list"**, which is a reference to the
  old system and is not reproduced anywhere in the six documents. The new file (`rules/purity-advice.md`)
  says so; the list needs a home in the packaged defaults.
- **The `read-walls` rule-table row understates its tools.** `hooks.md#2.10` row 3 says "Read, Grep,
  Glob, Bash, Agent", but the detail (`hooks.md#2.11.2`) adds the browser navigation/tab/upload tools
  and the `SubagentStop` final-message check. `rules/read-walls.md` carries the fuller set;
  `tables/rules.toml` reflects the table as written. The two should agree.
- **No definition of the `Task` tool's presence in the `PreToolUse` matcher.** The matcher in
  `hooks.md#2.3` names `Task`, but no rule file says what it checks on `Task`; `Agent` is the checked
  dispatch path everywhere else. Possibly intentional (alias), but unstated.
- **`_unknown` policy kind.** The `artifacts.toml` example in `hooks.md#2.11.3` defines an `[_unknown]`
  table (`policy = "block"`, "a write under the state directory that matches no kind") whose semantics
  live only in that example comment. `rules/artifacts.md` carries the example as written; the state
  design's table should state the key.
- **SessionEnd pruning deadline.** `hooks.md#2.11.10` says the retention prune happens on
  `SessionEnd` within Claude Code's 1.5-second budget, calling it easy; no fallback is named if a
  large event directory makes it untrue (unlike every other budget in the design).
- **`decisions` rule and `blueprint 4.11` digests.** The rule's settle function checks "that the
  digest of what is being approved still matches", which binds it to the digest definitions of
  BLUEPRINT.md#5.2/state.md#2.8 (which classes are inside or outside). The rule file links the
  decision kinds but not a digest definition, because none of my sources defines the digest; a
  `[[topic:digests]]`-style link should be added when the state group's file exists.
- **History inside my sources, deliberately not copied** (for the retired record): the "Old hooks it
  replaces" column of the rule table (`hooks.md#2.10`) is a mapping from the old system and is not
  carried into `tables/rules.toml`; `hooks.md#2.1`'s path-class sentence defers to
  `agents-skills.md#2.9` and is carried as a link instead.

## Part 3: sections in my sources that are history and are not copied

- `hooks.md#amendments-from-the-blueprint` — amendment list (also listed in `conflicts/index.md`).
- `hooks.md#2.15` — old defects and what prevents each one; mapping from the old system (also in
  `conflicts/index.md`).
- `hooks.md#3`, `hooks.md#3.1`, `hooks.md#3.2` — mapping from the old system (also in
  `conflicts/index.md`).
- `hooks.md#2.17` — departures from the readings in `rewrite/STATE.md` (also in
  `conflicts/index.md`).
- Inside `hooks.md#2.10`: the "Old hooks it replaces" column (see above).
- Inside `hooks.md#2.11.1`: the sentences naming which old hooks each rule carries over are kept only
  where they carry reason-code continuity information; the naming of old hook files is otherwise not
  carried.
- Inside `BLUEPRINT.md#5.4`: the two amendment lists ("Amendments made here", "Amendments made by the
  second pass") are history; their content is carried in the rule and topic files it amends. The
  section is a registry source, so it is not retired wholesale.
- Inside `hooks.md#1`: the sentence counting the replaced dispatcher, plugins and bash hooks (79
  registry entries) is old-system mapping; the new-layout statement (17 rules, one program) is kept
  in `topics/hook-runner.md`.
