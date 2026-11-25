# Conflicts and gaps — commands group

One entry per disagreement between the source documents about a thing in the `commands` group, and per gap
noticed while copying. Section ids are the originals (`rewrite/history/section-manifest.json`).

## Disagreements

1. **The shared output sentence, three wordings.** `BLUEPRINT.md#4.9`: "Every command prints one JSON
   object with `ok` under `--json`." `state.md#2.16`: "Every command prints one JSON object with `ok` when
   given `--json`." `packaging.md#2.10`: "`--json` on anything that reads." Used the blueprint's wording in
   `topics/command-conventions.md`; packaging's "anything that reads" is narrower and is not enforced
   anywhere.
2. **Pending-job result shape.** `BLUEPRINT.md#4.9`: "returns either the result or `{state: "pending"}`".
   `state.md#2.16`: "returns either the command's ordinary result or `{ok: true, state: "pending", retry:
   "<the same command>"}`". Used the state design's shape (`topics/long-commands.md`); it owns the tool's
   behaviour and the fuller shape names the `retry` field callers follow.
3. **`validate` signature and ownership.** `state.md#2.16` defines "`vsdd validate [--changed] [paths]`";
   `packaging.md#2.10` prints "`vsdd validate [PATH...]`" with no `--changed`; `hooks.md#2.13` says
   "`vsdd validate` ... [is] defined in `packaging.md`" while `state.md#2.16` says "It is the hook design's
   batch check and `packaging.md`'s `vsdd validate`: one command." Used `state.md#2.16` (the state design
   "owns the whole pipeline-facing tool"); the `--changed` flag appears only there. Packaging's row is
   superseded by the state design's, and `commands/validate.md` states one syntax: the row was the
   conversion commentary the second audit's structure pass moved here.
4. **`list` with an unknown kind.** `BLUEPRINT.md#4.9`: "an unknown kind is treated as revealing every
   walled class". `state.md#2.16`: "an unknown kind is refused for any caller with a wall, as if it revealed
   every walled class." Used the state design's: the unknown kind is refused, not shown.
5. **`gate plan --story` caller.** `BLUEPRINT.md#4.9` says "the planners for `gate spec --only <part>` and
   `gate plan --story`" (unnamed plural); `state.md#2.16` says "`clerk`; the `story-writer` for `--story`",
   and `pipeline.md#2.9.4` has the story-writer running it. Used `state.md#2.16`: the story-writer.
6. **`explain` and `rules` users vs callers.** `hooks.md#2.13` lists "Used by: People, the status skill"
   (`explain`) and "People; reference generation and the meta-test" (`rules`); `BLUEPRINT.md#4.9` places
   both in the read-only row "Any caller, subject to its wall". Used the blueprint for permission and the
   hooks row for intended users; both are recorded in the command files.
7. **`env stop` argument.** `state.md#2.16`'s argument column gives `--subject <id>` for the joint row
   "`vsdd env start`, `env stop`"; `BLUEPRINT.md#4.9` gives "`env start --subject <id>`, `env stop`" with no
   argument on stop. Used the blueprint for stop (no argument); `state.md#2.16` is ambiguous.
8. **"The two governed merge commands" named in one place only.** `BLUEPRINT.md#4.9`: "The commands only a
   person may run, the two governed merge commands and the story kinds exempt from tests before code are
   constants in code", without naming them. `hooks.md#2.7` names all three constants: `HUMAN_ONLY` is
   `decide`, `config set`, `cycle close`, `lock release --force`, `init --fresh` and `import`
   (`import --dry-run` allowed); `GOVERNED` is `story deliver` and `release`, "the only routes onto the
   integration and release branches"; the kinds exempt from tests before code are `facade`, `docs`, `chore`
   and `harness`, read from the story-kind table by `kind_has_red_gate(kind)`. The blueprint's person-only
   row names `decide`, `config set`, `cycle close` and `lock release --force`; `init --fresh` and the real
   `import` it states in the init/import row.
9. **Command count.** `BLUEPRINT.md#4.9`: "The tool has about 48 subcommands, down from about 60, and 11
   gate names, down from 15." The registry lists 55 command ids in this group plus 7 operator commands
   (doctor, events, obs, repos scan, export pdf, render diagram, version). Registry used as the catalog;
   the blueprint's "about 48" is stale relative to it.

## Gaps

10. **Caller column promised, not present.** `state.md#2.16` intro: "The last column of the second table
    says who may run a command" — but the two-column command table it introduces has no caller column;
    callers for those commands appear only in `BLUEPRINT.md#4.9`.
11. **`review prepare`'s patch is not in the evidence-writers list.** `state.md#2.16` ("writes the diff
    patch") vs `state.md#2.8`'s evidence-record row, whose writers are "`vsdd gate`, `vsdd survey` (diff
    patches), `vsdd story deliver`" — `review prepare` is absent, and the patch's storage path is stated
    nowhere.
12. **`coverage` and `path` are one-liners.** `BLUEPRINT.md#4.9` and `state.md#2.16` each give a single
    sentence; no arguments, output fields or wall specifics beyond the shared `list` rules anywhere.
13. **`cycle close` has no definition of the repair.** `state.md#2.16` and `BLUEPRINT.md#4.9` give one
    sentence each ("Repairs a cycle that did not close; person only"); what the repair writes besides the
    close fields is unstated.
14. **`idea register` has no write target or schema.** `pipeline.md#2.9.9`: "Duplicates and ideas in
    cool-down removed; the rest registered"; `BLUEPRINT.md#4.9`: "Idea registry". No path or record kind for
    the registry is named in these sections (the artifact kind `idea` exists in the registry), and its
    argument detail is stated nowhere beyond the discover stage that calls it.
15. **Deferred-work register store unnamed.** `vsdd defer add` writes "the deferred-work register"
    (`state.md#2.16`, `agents-skills.md#2.12`), and `validate` warns when the register is over its limit,
    but no file path or record kind for the register itself is given in these sections; the register's
    store and limit keys are named only through `validate`'s warning.
16. **"Three stated cases" lists four.** `state.md#2.12`: "Code writes it in three stated cases" — then
    names `init` and `import` (creation), `repos split`, and the survey's toolchain application, i.e. four
    clauses (init/import counted together).
17. **History embedded in cited sections, not copied.** The following passages are statements of what
    changed from an earlier version or mappings from the old system; they sit inside otherwise-design
    sections, so their containers cannot be retired whole: the "Removed in the first pass ... Removed in
    the second pass" paragraph of `BLUEPRINT.md#4.9`; the "Not built, and why" paragraph of `state.md#2.16`;
    the "It replaces six old skills" table of `state.md#2.10`; the "Old system" column of `state.md#2.6`;
    the old-template mapping table of `state.md#2.13`; the "Old content | Import" mapping table of
    `state.md#2.15`. None of it was copied into the new files.
18. **The re-freeze record's clause and its tail.** `state.md#2.16`'s gate-red row reads: "a re-freeze
    record with the new hashes of frozen tests the test-writer changed, written only if each still exists
    and either passes or fails on an assertion, as at the red gate (the implementer then makes a failing
    one pass)". `commands/gate-red.md` states the clause up to "on an assertion". The framing sentence
    around the clause was the conversion commentary the second audit's structure pass moved here; the
    tail — "as at the red gate (the implementer then makes a failing one pass)" — is the state design's
    own text, which `gate-red.md` leaves out, and the fact it states also stands in
    `topics/story-script.md`.
19. **The green gate's row in `state.md#2.16` against the pipeline's pass condition.** The state design's
    row: "Frozen tests are compared with the latest red or re-freeze record. On a project with a baseline
    it fails only on what the baseline does not list." `pipeline.md#2.9.1` states the same two facts in
    its pass condition ("the frozen tests are unchanged since the latest red or re-freeze record"; on a
    baseline, "a test, lint finding or format finding that the baseline lists does not fail the gate").
    Used the pipeline's pass condition in `commands/gate-green.md`; the row's wording stands only here.
