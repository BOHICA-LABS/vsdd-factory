# Second pass: close-out check

Date: 2026-11-24. Branch `rewrite/v1`. Checked: `rewrite/BLUEPRINT.md` (read in full) and the five design
documents against the 118 second-pass findings and the amender's 118 stated resolutions. This is a check of
the amendments, not a third review.

## Verdict

**Not clean. No blocker remains. Five major problems remain.** Each is a seam that a second-pass fix opened or
left half-closed. None needs a new decision from the user. Each is a small, local edit.

Most resolutions are in place and agree across the six documents:

- All 4 blockers and all 43 majors were opened at the place each resolution names.
- Every applied cut was checked: no status promotion, no status or verdict record, no stage registry, no job
  commands, three policy tables, combined measured checks, `vsdd`-only planner shells, about 48 subcommands,
  and the merged and split packages.
- The minors were checked at their named locations, by search where a fix was a wording change.

Three problems in the five majors are new. They come from combining two second-pass fixes:

- the route table with fast routes (M1);
- the whole-set lock and its digest with the retrospective path (M2);
- the code-reviewer's wall with the new `pattern-sweep` method (M3).

The other two are fixes that are only partly done:

- re-freezing a weak test (M4);
- passing the admitted story unit back to the script (M5).

The structural checks you asked for pass:

- **Section 1 counts match sections 4 and 9.** Section 4.3 lists 20 agents. Section 4.5 lists 13 skills and
  12 workflows. `agents-skills.md` 2.8 lists 77 distinct method files, which fits "about 75". `hooks.md` 2.10
  lists 17 rules. Section 4.9 lists about 48 subcommands and 11 gate names. Section 9 lists exactly 74
  packages: 13 + 17 + 9 + 12 + 11 + 12. The section 1 and 11 totals are also right: 118 findings, made up
  of 4 blockers, 43 majors and 71 minors; 100 marked Fixed; 16 cuts applied (15 in full, 1 in part);
  1 offered and 1 declined.
- **Section 11 lists every finding id, with the right severity.** All 118 ids appear once each, and a
  machine diff against the finding list matched.
- **Section 2 still treats three adopted decisions as open in its wording.** Item m16 below has the
  details.
  - D4's last column still says "This is the one answer two findings wait on (COV-04, OPS-12)".
  - D11 still says "They are listed as unconfirmed in `rewrite/STATE.md`".
  - D14 still says "If you would rather drop either skill, say so".

  The intro of section 2 and the table in 2.4 are correct. Nothing in section 2 reopens a decision.

## Major problems that remain

### M1. A fast-route cycle or a maintenance cycle routes to `plan`, not `verify`

Findings concerned: P2SEAM-06, P2COLD-24, P2REQ-16(b), and the end of the P2COLD-01 and P2REQ-01 fast route.

`pipeline.md` 2.4, route table, row "A cycle is open: the specification is approved and the plan is not
ready, or an amendment was approved and the plan has not been re-traced → `plan`". The row sits above
"all waves closed (or the fast-route story delivered) … → `verify`". "Plan is not ready" is defined nowhere,
and the row is not limited to the standard route.

In a `feature` cycle on a fast route, the specification is approved (from the ingest, the import or the
earlier release) and the cycle has no plan. So when the feature checkpoint computes `next`, it matches the
`plan` row before the `verify` row. The same happens to the maintenance cycle that `maintain` opens
(`pipeline.md` 2.9.10, "the route after the run is `verify`").

Three passages contradict the row:

- `pipeline.md` 2.3 ("Fast routes: nothing, next is `verify`");
- `pipeline.md` 2.9.4 close ("Fast routes: `next` is `verify`");
- blueprint 3.3 step 10 ("returns the next command: `/vsdd-factory:verify`").

This is the brownfield path you asked about. E3's post-release trivial feature and E4's bug fix would both go
to `plan`. A `plan` run on an ingested project would then decompose the whole recovered specification.

The route fixtures in `packaging.md` 2.6, layer 1 have no case for "fast route delivered → `verify`" or
"maintenance cycle with delivered fixes → `verify`".

The fix is to limit the `plan` row to a cycle whose route is `standard`, or a brief, or an approved
amendment, and to add the two fixtures.

### M2. Adopting a retrospective proposal between cycles makes the approved specification stale

Findings concerned: P2LEAN-09, P2COLD-31, P2LEAN-01 and P2COLD-20, and P2REQ-17 for an imported project.

The documents say that `conventions.md` and `review-rules.md` are "writable by the architect between
cycles", and that this is how a retrospective's proposal is adopted (`guidance_locked`). The places are:

- `state.md` 2.4, the lock row;
- `hooks.md` 2.11.3, `artifact_frozen`;
- `agents-skills.md` 2.12, the `author` table.

Both files are in `spec-arch`, inside the specification file set. The `spec` decision's digest is taken over
that whole set (`state.md` 2.6, last paragraph; 2.8, "How a decision is protected": `files` are "the
specification file set"). No document leaves the two guidance files out of the digest.

So the first adoption makes the latest `spec` decision read as `stale`. Three things then fail:

- "The specification is approved" means a current digest (`pipeline.md` 2.5; blueprint 5.1). It fails, so
  `plan` and a standalone `story` are refused.
- `spec_locked` is "true when the latest `spec` decision is approved", so the whole specification set is
  also unlocked.
- No route row covers a stale approval outside a `spec` run.

The same happens when `setup` dispatches the architect to move conventions on a project whose state was
just imported, because the import writes an approved `spec` decision.

The fix is to state that the decision digest leaves out the two guidance files, which have their own lock.

### M3. The code-reviewer's wall blocks the inputs of its own steps

Findings concerned: P2COV-03 and P2SEAM-08 (the pattern-consistency sweep).

Two documents give the `code-reviewer` files in the state directory:

- `pipeline.md` 2.9.10 and 2.9.13 give it the sweep `pattern-sweep`, "which reads the code against
  `conventions.md`";
- `agents-skills.md` 2.6 gives it "the review rules file" and, for `pattern-sweep`, "`conventions.md`".

Both files are under `.factory/specs/architecture/`. But the code-reviewer's read wall denies "every class in
the state directory except `evidence`" (`agents-skills.md` 2.9, read-walls row; blueprint 4.3, "Read walls").

Read walls are locked rules, so the hook blocks the read and the sweep cannot do its job. `pattern-sweep` is
the code-reviewer's only pipeline step. The fix is either of these:

- give the row a `read_allow` for the two guidance files;
- give the sweep to an agent that may read `spec-arch`.

### M4. A strengthened test that fails cannot be re-frozen

Finding concerned: P2COLD-04.

`pipeline.md` 2.9.1, "Fixing a frozen test", and `state.md` 2.16 (the `vsdd gate red` row) both say that
`gate red --refreeze` "requires every changed frozen test to still exist and to pass".

A finding of the kind "this test cannot fail" exists because a weak test can hide a defect. When the
strengthened test then fails on the current code, three things happen:

- The re-freeze is refused.
- The green gate still compares against the old freeze, so it fails on "frozen tests unchanged".
- The implementer is never asked to fix the code. In `story.js` (`pipeline.md` 2.11.2, `fix()`), the
  test-writer runs first, its `gated(..., 'red')` step returns blocked, and `fix()` returns `ok: false`.

The story then fails, and later becomes a `stuck` decision. This is the case that matters most for
P2COLD-04.

The fix is to let a re-freeze record a changed frozen test that fails on an assertion, as the red gate does.
The test-writer then hands over to the implementer for green in the same pass, and the `fix()` ordering
follows from that. The same correction applies to `agents-skills.md` 2.8, `test-fixes` ("it ends with
`vsdd gate red --refreeze` and then the green gate").

### M5. The admitted story unit has no field in any schema

Findings concerned: P2COLD-01 and P2REQ-01 (the fast route). Neither blocker is reopened, but the mechanism
is not yet specified.

The fast route hands the admitted story unit back to the script by way of the story-writer's result. Three
places say so:

- `pipeline.md` 2.9.4, story-draft: "the same command admits it and returns its unit";
- `pipeline.md` 2.10.8, Story unit row: "`vsdd gate plan --story` (in the gate result's `unit`)";
- `agents-skills.md` 2.6, story-writer row: "the plan gate result in `gates`, with the admitted `unit`".

No schema defines that field. The gate-result schema in `pipeline.md` 2.10.6 has no `unit` property, and
neither does the prelude's `S_GATE` in 2.11.1. Blueprint 4.10 also says "The script never reads a field the
schema does not require".

A schema-shaped `StructuredOutput` return will drop, or may refuse, a property the schema does not declare.
In that case the script has no `position` to pass to the child `story`, and every fast route stops after
admission.

A second gap is in the same stage. The story-writer may write several stories (the readiness chores, and
one story per repository). Each has its own closing command, but `brief()` carries one closing command.

The fix is to define `unit` in the plan-gate result, or in the story-writer's data schema as a list, mark it
required for `--story`, and let the step take a closing command per story.

## Minor problems that remain

- **m1. A walled finding's key carries the scenario path.**
  - `state.md` 2.5 (findings paragraph) has a policy test: "no record outside the holdout class and `runs`
    contains a scenario path".
  - The same paragraph, and blueprint 4.10 ("Walled finding"), keep the walled finding's `key` in packets
    and closure records.
  - By the key rule (blueprint 4.8), a scenario-reviewer finding's key is
    `<category>:<path>:<symbol or clause>`. Its path is under `.factory/holdout/`, and its clause can carry
    what the scenario tests.
  - So the test fails on every walled finding, or the key needs a defined redaction.
- **m2. The title of a scenario-score finding is unspecified.** `pipeline.md` 2.7 ("Findings built from
  scenario scores") and blueprint 4.10 define the key, scope, `spec_ref`, `repro` and `observed`. They do not
  define the required `title`. The scenario's own title must not be used.
- **m3. The main-session scenario check refuses only once.** `hooks.md` 2.11.2, `SubagentStop` row: a
  message that quotes a scenario "is refused once … and logged if it is repeated". On the second attempt the
  text reaches the main session, so P2REQ-14 is only partly closed.
- **m4. `parseArgs()` swallows `key=value` words inside free text.** `pipeline.md` 2.3 and 2.11.1: a request
  such as "make timeout=30 configurable" loses `timeout=30` to a named argument.
- **m5. Two conditions for report-only `maintain`.**
  - The `pipeline.md` 2.5 preconditions table and the first paragraph of 2.9.10 say "while a product cycle
    holds the lock".
  - The second paragraph of 2.9.10 says "when a product cycle is open".
  - An open cycle with no lock (one waiting at its release decision) is not covered.
- **m6. Missing package dependencies.**
  - E1 (blueprint 9.6) has probes that need builder and reviewer agents and a gate. Probe 4 needs a
    builder-tier and a reviewer-tier agent; probe 7 needs a builder step that runs its gate. E1 does not
    depend on A4, A6 or G2.
  - W4 and W5 (blueprint 9.5) do not reach B5, the workflow harness, even indirectly.
- **m7. Two commands have no package.** `vsdd note` and `vsdd cycle close` are not delivered by any package
  in blueprint 9.2. This is left over from P2COLD-17.
- **m8. The re-freeze limit is not written into the callers table.**
  - `hooks.md` 2.11.4 and `state.md` 2.16 say `gate red --refreeze` is for the test-writer only.
  - In `walls.toml` (`hooks.md` 2.7) `measure = ["gate"]`, and every builder and planner row holds it.
  - No `[callers]` entry or stated tool check limits the flag.
- **m9. The `env` group does not match the command table.** In `hooks.md` 2.7 the `main` row holds `env`
  and the `clerk` row does not. Blueprint 4.9 and `state.md` 2.16 list `vsdd env start` for the clerk and not
  for the main session.
- **m10. Example rows lack `dispatch`.**
  - In `hooks.md` 2.7 the `verifier`, `devops` and `story-writer` rows have no `dispatch` key, and the key
    table states no default.
  - `agents-skills.md` 2.9 and the L-AGENT lint say that every agent except the spike runner and the clerk
    may dispatch the spike runner.
- **m11. The importer contradicts itself in two places.**
  - `state.md` 2.15 says over-cap imported files are made smaller by `/vsdd-factory:author conform`. But the
    import writes an approved `spec` decision, which locks the specification set, so those files cannot be
    edited without an amendment.
  - `packaging.md` 2.6 has the layer 1 fixture "Import, then one fast-route feature", and E4 asserts that
    after the import the route is "steady state". But the importer queues a request for each epic in flight,
    so the route is then `feature`.
- **m12. In-source proof harnesses stay `source`.**
  - In `state.md` 2.12 the default `harness` globs are `fuzz/**`, `benches/**` and `proofs/**`.
  - Kani-style proofs inside `src/` remain `source`, which the verifier cannot write.
  - This is left over from P2SEAM-01.
- **m13. The `feature` evolve stage names one method per agent.**
  - The `feature` evolve row of `pipeline.md` 2.9.13 gives the product-owner `spec-delta` and the architect
    `architecture`.
  - But 2.9.4 also has evolve produce new hidden scenarios, property deltas and new assessments.
  - Those need `holdout-scenarios`, `verification-properties` and the assessment methods, one step each.
- **m14. Inherited flaky tests are not handled.** The baseline comparison (`pipeline.md` 2.9.8; `state.md`
  2.8, "The baseline record") treats an inherited test that passed at the baseline and now fails as new. A
  flaky legacy test therefore fails unrelated stories. P2REQ-02 named flakes.
- **m15. The Playwright option is unnamed.** `packaging.md` 2.4 `.mcp.json` carries the placeholder
  `"<the option that refuses file: addresses>"`. Name the option, or say that probe 33 picks it.
- **m16. Stale text not covered by the amendment lists.**
  - Blueprint 2.1:
    - D4 last column: "This is the one answer two findings wait on (COV-04, OPS-12)";
    - D11: "They are listed as unconfirmed in `rewrite/STATE.md`";
    - D14: "If you would rather drop either skill, say so".
  - Blueprint 8: the list of rows whose mechanism the second pass replaced leaves out several rows:
    - SEAM-09 (`release publish`);
    - SEAM-22, OPS-07 and OPS-14 (the per-session hook marker);
    - SEAM-24 (`driver on/off`, now also `pause`).
  - `pipeline.md` amendments 5 ("the hook runner's session marker") and 16 ("the request carries the asker's
    agent type"). Both are still as written, although the list's header says the items the second pass
    changed were corrected.
  - `agents-skills.md` 3.2, row 16: it still cites `vsdd review verdict`, which is now a function, not a
    command.
  - `hooks.md` 5, K7: "No subcommand of the tool changes the `[hooks]` table" contradicts `vsdd init
    --auto-allow` and `vsdd import --auto-allow`.
  - `state.md` 2.4, class table, the product-class writer cell: it gives the test-writer `tests` and
    `manifests` only, and leaves out `source` before red, which blueprint 4.3, `hooks.md` 2.7 and
    `agents-skills.md` 2.9 all grant.
  - `agents-skills.md` 2.9: the write-scopes table is broken by a paragraph inserted after the `devops` row.
    The rows for the reviewers, the evaluator, the main session and other callers now fall outside the
    table.
  - `packaging.md` 2.10: "The agents that research have no shell" is no longer true of the spike runner.

## Resolutions confirmed

The blockers:

- **P2SEAM-01 and P2COLD-02.** The seven product classes have one precedence, the same in blueprint 4.7,
  `state.md` 2.4 and 2.12, and `hooks.md` 2.7. The role grants include `source:create@repo-setup` for devops.
  The toolchain reaches `factory.toml` through the `spec` approval. The scaffold story runs alone in wave 1.
  L-KIND and the write-scope cases are in place.
- **P2COLD-01 and P2REQ-01.** Admission by `gate plan --story` and `story new-fix` is in place. So is the
  membership check at every gate, and readiness computed from the admission closure. The schema gap is M5.

The majors that hold:

- P2SEAM-02 to 05 and P2SEAM-07.
- P2COV-01 and 02.
- P2OPS-01 to 04.
- P2COLD-03, 05 to 16 and 20.
- P2REQ-02 to 09.
- All seven major cuts: P2LEAN-01 to 07.

Some of these were checked in detail:

- **Caller markers and probes 27 to 29.** They agree in blueprint 4.9, `state.md` 2.11 and `hooks.md`
  2.11.10.
- **Spike asker records.** Admission records, `spike open` reading the asker from records, and the
  cached-finding wall check agree across blueprint 6, `pipeline.md` 2.8, `state.md` 2.16 and `hooks.md`
  2.11.6.
- **Auto-allow.** The `core` and `extended` scopes are written by `init --auto-allow`, and setup asks first.
- **The baseline record and first-release rule.** They agree in `pipeline.md` 2.9.6 and 2.9.8 and in
  `state.md` 2.8.
- **The amendment run inside an open cycle.** It agrees in `pipeline.md` 2.6.
- **The second cycle and qualified ids.** They are in place.
- **Hub per-repository records.** They agree in `pipeline.md` 2.9.11 and `state.md` 2.14.
- **E4.**
- **The dependency-cycle break.** B4a and B4b break it, and the listed edges are present.
- **The corrected skeletons.** In `pipeline.md` 2.11 they now show the reserve check in `runStories()`, the
  arbiter dismissing only disputed findings, coverage gaps that call the fixer, `rounds_used` counted across
  runs, `reviewer_blocked`, isolated spikes and the relay retry.
- **The severity text.** It is byte-identical in the blueprint, `pipeline.md` and `agents-skills.md`.
