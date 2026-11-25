# Commands-group repair pass — disagreements and gaps

Written by the fixing pass over `rewrite/spec/commands/` and the `commands`-group topics. One entry per
disagreement between the originals met while repairing, and per gap (a value the originals do not give).
Nothing here is resolved beyond what the precedence rule forces; the older notes live in `commands.md`.
Section ids are the originals (`rewrite/history/section-manifest.json`).

## Disagreements

1. **`spike open --step` has two forms.** `pipeline.md#2.8` line 715: `--step <workflow>/<subject>/<name>`;
   `state.md#2.16` line 1410: `--step <workflow>/<subject>/<name>.<attempt>`. `command:spike-open` keeps the
   state design's form, because that design states it fixes the commands' arguments, and it names the
   attempt that the admission flow keys on. `topic:spikes` (other area) should link, not restate.
2. **Matching of an earlier spike finding, three wordings.** `BLUEPRINT.md#6`: "matched by a hash of the
   normalised question", "only when the earlier runner worked under walls at least as strict as this
   asker's". `pipeline.md#2.8`: only "the same question". `state.md#2.16`: "only if that entry's
   `read_deny` covers this asker's". `command:spike-open` now carries the blueprint's sentence; the
   blueprint outranks the detail documents and both other wordings are narrower statements of the same
   rule.
3. **The green gate pass condition, the red gate pass condition and the behaviour of `vsdd story deliver`
   appear in `workflows/story.md` and again in the command files.** This pass keeps them in
   `command:gate-green`, `command:gate-red` and `command:story-deliver`, which cite `pipeline.md#2.9.1`
   and must each be complete in its own file; `workflows/story.md` (other area) is the copy that should
   shrink to links.
3a. **Spike retention was narrowed to named spikes, and the named-spike branch rule was lost.**
   `state.md#2.17` states one retention rule for spike worktrees ("`vsdd checkpoint` and `vsdd status
   --fix` remove spike worktrees older than `spikes.retain_days` (14) and those whose asking unit is
   delivered; the branch `spike/<name>` of a named spike is kept until the spike is closed");
   `topic:worktrees` and `command:spike-close` had applied it to named-spike worktrees only and dropped
   the branch rule. Restored in `topic:worktrees` as written, with `state.md#2.17` added to its sources;
   `command:spike-close` links. `topics/spikes.md` (other area) restates the same rule and should link.
4. **Commentary between quotations in the command files.** `command:list` carried "a kind is refused, not
   silently shortened" (the choice recorded in `commands.md` item 4) and `command:gate-spec` carried the
   planner-naming note (item 5). Both commentaries are struck from the command files; the disagreements
   they glossed stay recorded in `commands.md` items 4 and 5.
5. **`--repo <name>` on the job gates.** `state.md#2.16` gives one pooled argument cell for `gate wave`,
   `release`, `ingest`, `sweep`, `contracts` and `integration` (`--wave <id> [--settle]`;
   `[--only <check>]`; `[--repo <name>]`; `--run <id>`) without saying which gate takes which.
   `pipeline.md#2.9.11` gives per-repository wording for the wave and release gates only. The files give
   `--repo` to `gate-wave` and `gate-release`; the other four omit it. Unverifiable beyond that.
6. **Caller of the operator commands.** `BLUEPRINT.md#4.9` states the caller ("Main session") only for the
   operator row as a whole; `packaging.md#2.10` names no caller per row. Stated per file in
   `command:events`, `command:obs`, `command:repos-scan`, `command:export-pdf`, `command:render-diagram`
   and `command:doctor`.

## Gaps

7. **`events schema` outside an initialised project is not addressed by the originals.** The shared
   exception list (`init`, `import`, `doctor`, `obs`, `hooks selftest`, `--version`) does not include it,
   so the file must not have claimed it works outside; the claim is removed. Whether the reference
   generators that call it run inside a project is unstated.
8. **`coverage`, `path`, `cycle close`, `idea register`, the deferred-work register store and the storage
   class of `review prepare`'s patch** are unspecified in the originals, as already recorded in
   `commands.md` items 11 to 15; unchanged by this pass.
9. **Driver flag states.** `state.md#2.16` says only "Writes `local/driver.json` with the session id"; the
   three states (`on`, `paused` at a decision, absent) come from `pipeline.md#2.4`'s drive-skill text, not
   from the command table. Recorded because the command files depend on it.
