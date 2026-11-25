# Second pass: close-out check, round 2

Date: 2026-11-24. Branch `rewrite/v1`. Checked: the 21 items of `rewrite/design/pass2-closeout.md` (M1 to M5,
m1 to m16) against the fixing agent's stated changes, at every place each change names. For each change I
also read the section around it and the other documents that state the same rule. This is a check of this
round's fixes, not a third review. Line numbers are as of this check.

## Verdict

**Not clean. One major problem remains, and four minor ones.** All five are new: each was caused by a fix in
this round. None needs a decision from the user, and each is a local edit.

- All 21 changes are present at the places named.
- 20 resolve their problem as the report described it, and the documents that state the same rule agree.
- M1 resolves the misroute it was about. But its new `plan` row reads the cycle's route, and no tracked record
  holds that route (N1, major).
- The four minor problems come from M2 (N2), M5 (N3), m14 (N4) and m11 (N5).

I also ran two of the changed functions as written. `parseArgs()` keeps `timeout=30` in the request and still
sets `intent=bug-fix`. The routing loop of `fix()` was run for four cases, and the implementer runs exactly
when the item says: with code findings, or when the test-writer's green result did not pass.

## The 21 items

| Item | Present | Resolves it | Agrees across documents | Notes |
|---|---|---|---|---|
| M1 | yes | yes, with N1 | yes | The `plan` row is limited and both fixtures are added (`pipeline.md` 2.4, `packaging.md` 2.6 layer 1). The `verify` row and its precondition, 2.9.4 close, 2.9.10, blueprint 3.3 step 10 and blueprint 5.1 agree. The new row depends on a value with no durable writer: see N1. |
| M2 | yes | yes, with N2 | yes | The digest leaves out `guidance` in `state.md` 2.4, 2.8 and 5.2's summary, blueprint 4.11, `pipeline.md` 2.6, `hooks.md` 2.11.3 and `agents-skills.md` 2.12. The new definition names only `guidance` as an exception, so it now includes the brief: see N2. |
| M3 | yes | yes | yes | The `guidance` class is nested inside `spec-arch`, as `brief` is inside `spec-product`. It fits the existing `read_allow` meaning ("a class named here wins over a containing class that is denied"). The architect's `spec-arch` grant still covers both files, as `spec-product` covers the brief. The holdout-evaluator and scenario-reviewer still cannot read the files, because they deny `spec-arch`. No document states a class count that is now wrong. |
| M4 | yes | yes | yes | Five places say the same rule: `pipeline.md` 2.9.1 ("Fixing a frozen test" and the review row), `fix()` and the sentence after it, `state.md` 2.16, `agents-skills.md` 2.8 `test-fixes`, and `hooks.md` 2.11.4. The `packaging.md` 2.6 scenario test matches. |
| M5 | yes | yes, with N3 | yes | `unit` is in the gate-result schema and in `S_GATE`. "Story-draft data" requires `units`. The 2.9.4 story-draft row, `agents-skills.md` 2.6 and blueprint 4.10 agree. `brief()` takes `o.each`, but `gated()` was not changed to match: see N3. |
| m1 | yes | yes | yes | The walled key is defined the same way in four places: blueprint 4.10 (with a pointer in 4.8), `pipeline.md` 2.7, `state.md` 2.5 and `hooks.md` 2.11.7 step 5. Claims to the scenario-reviewer keep the full key, which is allowed because that reader may read `holdout`. A waiver of a walled key cannot arise: the only loop that produces walled findings runs at the `spec` perimeter, which opens no `finding` decision. |
| m2 | yes | yes | yes | |
| m3 | yes | yes | yes | |
| m4 | yes | yes | yes | Both calls (`story.js`, `wave.js`) pass their argument names. Universal test 12 matches. |
| m5 | yes | yes | yes | |
| m6 | yes | yes | yes | No dependency cycle results. |
| m7 | yes | yes | yes | |
| m8 | yes | yes | yes | It uses the same "a flagged entry wins" matching as `status --fix`. `state.md` 2.16 agrees. |
| m9 | yes | yes | yes | |
| m10 | yes | yes | yes | |
| m11 | yes | yes, with N5 | mostly | `state.md` 2.15 and both `packaging.md` 2.6 rows are corrected. Blueprint 3.4 still says the next command after an import is `feature`: see N5. |
| m12 | yes | yes | yes | |
| m13 | yes | yes | yes | Every method the row names is in its agent's list in `agents-skills.md` 2.8. |
| m14 | yes | yes, with N4 | yes | Blueprint 4.13, `pipeline.md` 2.9.8, `state.md` 2.8 and the `packaging.md` fixture agree. The flaky record is not kept where `vsdd report debt` reads: see N4. |
| m15 | yes | yes | yes | |
| m16 | yes | yes | yes | Every listed passage is corrected, and so are the three the agent found itself (`hooks.md` amendment 10, `state.md` amendment 18, the COV-09 row). The `agents-skills.md` 2.9 write-scopes table is whole again. |

## Major problem that remains

### N1. The new `plan` row reads a cycle route that no tracked record holds

Caused by: M1.

Where:

- `pipeline.md` 2.4, route table, `plan` row (line 299): "A cycle is open that needs a plan (it began with a
  brief, or its route is `standard`)".
- `pipeline.md` 2.4, `verify` row (line 302) and 2.5, the `verify` precondition (line 381): "in a cycle with no
  plan, which is a fast route or a maintenance cycle".
- `state.md` 2.8, Cycle record row (line 735): `cycle.json` has a `route` field. Its writers are only "whichever
  command opens the cycle" (`vsdd survey feature` for a feature cycle), and the release checkpoint adds the
  close fields.
- `state.md` 2.16, `vsdd feature route` row (line 1427): it writes "the route and the amendment flag in the run
  record".
- `pipeline.md` 2.9.4, delta and route row (line 976): the route is derived after the survey has opened the
  cycle.

The problem. For a cycle that began with a brief, "began with a brief" can be read from `cycle.json` (`request:
brief`). A feature cycle needs the route. The route is derived by `vsdd feature route` after the cycle is
opened, and it is written only to the run record.

The run record is local and untracked. It is deleted after 30 days or when the cycle closes, and a fresh clone
does not have it. `cycle.json` has a `route` field, but it is written only when the cycle opens, before the
route exists, and no command later fills it. `state.md` 2.8 says a fresh clone resumes from the last
checkpoint, so the route table must be computable from tracked records.

What fails. A standard-route feature whose delta specification is approved in another clone, or more than 30
days later:

- The `plan` row misses, because the route cannot be read.
- No wave exists. The `verify` row's "every admitted story delivered" is then vacuously true for a cycle that
  has admitted no story, so `next` is `verify` instead of `plan`.

This is the reverse of the misroute M1 fixed. The same value also decides which checks `verify` runs
("Required checks by route", 2.9.6), so that survey has no durable source either. That part predates this
round.

Proposed resolution:

1. `vsdd feature route` also writes `route` into `cycle.json`, in the same command and under the run's lock.
   - In `state.md` 2.8, add "`vsdd feature route` (the `route` field)" to the Cycle record's writers.
   - In the "Who commits state" table, add a row "the route is derived".
   - In 2.16, add `cycle.json` `route` to the Writes column of the `vsdd feature route` row.
   - A cycle opened by `checkpoint brief` or the `discover` assess checkpoint writes `route = standard` when it
     opens. A maintenance cycle needs no route, because the rows test its kind.
2. Define "a cycle with no plan" positively in the `verify` row and precondition: "a cycle whose route is
   `trivial`, `bugfix` or `hotfix`, or whose kind is `maintenance`". Then an unknown route can never match
   `verify` vacuously.
3. Add a route fixture: a standard-route feature whose delta specification is approved, with no run records on
   disk (a fresh clone), gives `plan`.

## Minor problems that remain

### N2. The new digest definition includes the brief, which the lock leaves out

Caused by: M2.

Where:

- `state.md` 2.8, "How a decision is protected" (lines 815 to 817): "The specification file set is every file in
  the three specification classes except the class `guidance`".
- `state.md` 2.4, the nesting sentence (line 519): "`brief` lies inside `spec-product`".
- `state.md` 2.4, lock row: "The `brief` class is outside the lock; it has its own decision".
- The same rule in blueprint 4.11 (line 656): "without the two guidance files".

The problem. By the stated nesting, `specs/product/brief.md` is now explicitly in the `spec` digest. Drafting a
brief between cycles therefore makes the last `spec` approval stale. The brief skill saves after every section.
Drafting a brief is a documented path: "write a brief" in steady state, and "a brief and then the greenfield
route" after an ingest.

While the approval is stale, the whole specification set is unlocked (`spec_locked` needs an approved decision).
A queued fast-route `feature` started during the draft is also refused: the plan gate requires criteria that
cite the approved set, and there is none. No text says whether including the brief is intended.

The inclusion also hides a gap that predates this round, and the resolution has to deal with it. A `spec` run in
a new product cycle has to write a specification set that an earlier `spec` decision locks. Only a `feature` run
on the standard route and the `amend` run are said to hold an amendment. Today the new brief makes the old
approval stale, which unlocks the set as a side effect.

Proposed resolution (recommended):

1. In `state.md` 2.8 and blueprint 4.11, define the set as "every file in the three specification classes except
   the classes `brief` and `guidance`, which have their own decision and their own lock".
2. In `pipeline.md` 2.9.3, state that the `spec` survey marks its run as holding an amendment when the latest
   `spec` decision is approved, as `vsdd feature route` does for the standard route.

The alternative is to keep the brief in the set and say so in `state.md` 2.8, together with the effects above.

### N3. `gated()` checks one gate entry, but the story-draft step now returns one per story

Caused by: M5.

Where:

- `pipeline.md` 2.11.1: `brief()` with `o.each` (lines 1636 to 1655), and `gated()` (lines 1715 to 1726), which
  takes `(r.gates || []).find((x) => x.gate === gate)`.
- `pipeline.md` 2.9.4, story-draft row (line 977): "at most 2 attempts", and the gate "`gate.passed` for every
  story, and a unit in `data.units` for each".

The problem. The story-draft step is the one that needs `gated()` (two attempts, with the gate's failure text
fed back). But `gated()` accepts the step as soon as the first `plan` entry passes. When the story-writer wrote
a readiness chore and a story, and the second fails its plan gate, two things go wrong:

- The step is accepted with no retry.
- The failed story has no unit in `data.units`, so it is never delivered, and nothing reports it.

Proposed resolution. Give `gated()` the same `o.each` option. With it, the step passes only when every entry for
that gate passed on the step's head, and the retry note lists each failing entry. Then the script checks that
each passing `plan` entry has a matching unit in `data.units`. Add a harness case to the fast-route scenario of
`packaging.md` 2.6: two stories, the second fails once.

### N4. Flaky inherited tests are recorded only in evidence, which `vsdd report debt` does not keep

Caused by: m14.

Where:

- `pipeline.md` 2.9.8 (lines 1094 to 1098): "recorded in the evidence as flaky, listed by `vsdd report debt`".
- `state.md` 2.8, "The baseline record" (lines 783 to 785): "recorded in the evidence as flaky".
- `state.md` 2.8, Evidence record row (line 740): untracked, kept "until the subject's unit closes".
- `state.md` 2.16, `vsdd report debt` row (line 1383), and the Deferred item row of 2.5: the report is derived
  from closure records and the baseline.

The problem. The flaky record is gone once the story closes, and on any other clone. The debt report's stated
sources do not include it, so "listed by `vsdd report debt`" has no persistent source.

Proposed resolution:

1. The gate result lists the flaky test identities, and the checkpoint copies them into the unit's closure
   record, for example a `flaky` list beside the verdict.
2. The `state.md` 2.16 `report debt` row names "flaky inherited tests, from closure records" among its sources.
3. `pipeline.md` 2.9.8 and `state.md` 2.8 say "recorded as flaky in the gate's evidence and the unit's closure
   record".

### N5. Blueprint 3.4 still says the next command after an import is `feature`

Caused by: m11 (its resolution changed the stated route after an import, but not this row).

Where:

- Blueprint 3.4, row "A project on the old plugin" (line 224): "so the next command is `feature`".
- m11's resolution, in `packaging.md` 2.6 (both the fixture and E4) and blueprint 11.4: the route is `feature`
  with the queued request when an epic was in flight, and steady state (done) when none was queued.

`pipeline.md` 2.4's Brownfield row ("starts at `feature`") describes the entry point, not `next`, and can stay.

Proposed resolution. Reword the blueprint 3.4 cell: "so `feature` can start: the route names it at once when the
importer queued a request for an epic in flight, and otherwise the person starts it."
