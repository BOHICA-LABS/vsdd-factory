# Second pass: close-out check, round 3

Date: 2026-11-24. Branch `rewrite/v1`. Checked: the five items of `rewrite/design/pass2-closeout-2.md` (N1 to
N5) against the fixing agent's stated changes, at every place each change names. For each change I also read
the section around it and the other documents that state the same rule. This checks this round's fixes; it is
not a third review. Line numbers are as of this check.

## Verdict

**Clean of blockers and majors. Five minor problems remain.** Each comes from a fix in this round, and each is
a local edit that needs no decision from the user.

- All five changes are at the places named, and the blueprint and design documents agree on each rule.
- N4 and N5 are fully resolved.
- N1 resolves the major misroute it was about. The route now lives in `cycle.json`, and a fresh clone routes a
  standard-route feature to `plan`. But because the route is now written into `cycle.json`, an amendment run
  can overwrite it (R1). The route is also defined as empty for a maintenance cycle, and the lists that read
  the route have no entry for that (R4). The new fixture also depends on a route row that is not defined (R5).
- N2 resolves the brief's effect on the digest. A side effect: an earlier approval now stays current in a later
  product cycle, and three rules do not tell that approval apart from the cycle's own (R2).
- N3 resolves the reported case. But `gated()` still accepts a retry that leaves out a story an earlier attempt
  admitted (R3).

I ran the new `gated()` as written in Node against eight cases. It behaves as stated for both stories passing,
the second failing once, the second failing twice (blocked, kind `gate`), a passing entry with no unit, no
entries at all, and the single-gate path. It returns `done` when the retry reports only the story that failed
before (R3). The correction proposed under R3 was run against the same eight cases and gives `blocked` there,
with every other case unchanged.

## The five items

| Item | Present | Resolves it | Agrees across documents | Notes |
|---|---|---|---|---|
| N1 | yes | yes, with R1, R4, R5 | yes | `pipeline.md` 2.4 (plan row, verify row, the paragraph after the table, the sixth fixture), 2.5, 2.9.4, 2.9.6; `state.md` 2.7, 2.8, 2.16; `packaging.md` 2.6; blueprint 3.3 step 2, 4.9, 5.1 and 11.4. The run record no longer lists a route. Putting the new row in the 2.7 writers table, not the flush table, is sound: the `feature` checkpoint commits `cycle.json`, and a run that dies before it redoes its delta step anyway. |
| N2 | yes | yes, with R2 | yes | The digest excludes `brief` and `guidance` in `state.md` 2.8, blueprint 4.11 and 5.2, `pipeline.md` 2.6 and `hooks.md` 2.11.3. The `spec` survey's amendment mark is in `pipeline.md` 2.9.3, `state.md` 2.16 and blueprint 4.11. |
| N3 | yes | yes, with R3 | yes | `gated()` `each` mode, the `brief()` comment, the 2.11 intro, the 2.9.4 story-draft row, `state.md` 2.16, blueprint 4.10, and the two-story scenario in `packaging.md` 2.6. |
| N4 | yes | yes | yes | `flaky` is in the gate-result schema (2.10.6). The closure record holds it (`state.md` 2.8), and `report debt` (`state.md` 2.16), the sweep row (2.9.10), blueprint 4.13 and the `packaging.md` fixture read it from there. |
| N5 | yes | yes | yes | Blueprint 3.4 now agrees with `state.md` 2.15, both `packaging.md` 2.6 rows and the blueprint 4.9 `import` row. |

## Minor problems that remain

### R1. An amendment run can overwrite the cycle's route

Caused by: N1.

Where:

- `pipeline.md` 2.9.4, delta and route row (line 982). `vsdd feature route` "derives the route ... writes it
  into `cycle.json`", with no exception.
- `state.md` 2.16, `vsdd feature route` row (line 1432), and `state.md` 2.7, writers table (line 720), say the
  same.
- `state.md` 2.8, Cycle record row (line 736), says it is written "when a change request opened" the cycle.
- `pipeline.md` 2.6, "Amendments inside an open cycle" (lines 505 to 513). The `feature` run with `amend` runs
  inside the open cycle and has a change analysis, so it has a delta step, and that step ends with
  `vsdd feature route`.

The problem. The `feature` run with `amend` runs inside a cycle that some other opener started. That can be a
product cycle opened by a brief (route `standard`), or a fast-route cycle. Three of the four places above say
`vsdd feature route` writes the route it derives, so it overwrites the cycle's route. The fourth limits the
write to cycles a change request opened.

What fails. Suppose a spec-gap amendment in a product cycle changes no contract, for example a gap in the UX
specification only. By the rules in 2.9.4, its route derives as `trivial`. Then:

- The product cycle's route becomes `trivial`.
- `verify` runs "baseline only" (2.9.6, line 1051) for the whole product cycle.
- The release version is computed from that route (line 1063).

Before this round the route was kept only in the run record, so an amendment run could not change the
cycle's route.

Proposed resolution:

- In 2.9.4, the 2.6 amendment paragraph, `state.md` 2.7 and 2.16, and blueprint 4.9, state that a run with
  `amend` is always on the standard route.
- State in the same places that `vsdd feature route` writes `route` only in the run that opened the cycle, so
  an amendment run leaves `cycle.json` unchanged.
- The `plan` row's second clause, "an amendment was approved and the plan has not been re-traced", already
  sends an amended fast-route cycle to `plan`.

### R2. An earlier approval now stays current in a later product cycle, and three rules cannot tell it from the cycle's own

Caused by: N2. Before N2, drafting the new brief made the earlier approval stale. That staleness was the only
thing separating it from the cycle's own approval.

Where:

- The `spec` survey's amendment mark. `state.md` 2.16, `vsdd survey` row (line 1379): "when the latest `spec`
  decision is approved". Blueprint 4.11 (lines 656 to 660) and its 11.4 N2 row (line 1733): "a `spec` run that
  starts while an earlier approval stands". `pipeline.md` 2.9.3 survey row (line 950): the same, with the
  sources in a parenthesis ("an earlier cycle's, an ingest's or an import's").
- `pipeline.md` 2.5, `plan` precondition (line 383): "A cycle is open; the specification is approved".
- `pipeline.md` 2.4, `spec` row (line 297): "the specification is not converged", a phrase defined nowhere.
- What these contradict:
  - `pipeline.md` 2.4 (line 336): "If the main session starts the wrong workflow, that workflow's survey
    refuses."
  - `state.md` 2.5, staleness row (line 644), and `packaging.md` (line 1006): after approval, a contract
    changes only through an amendment "whose change analysis names the stories it affects".

The problem. There are three separate effects.

1. The mark is not limited to the start of a later product cycle. Take a product cycle whose own `spec`
   decision is approved, in the middle of its waves. A `spec` run started by hand, or by a main session that
   chose wrongly, passes its preconditions (a cycle is open, the brief is approved). It now holds an amendment
   and unlocks the whole set. It has no change analysis, so when its decision is approved, `plan` in delta
   scope has no stories to re-trace. Before N2 the lock refused that run's writes.
2. In a later product cycle, before `spec` has run, the earlier approval satisfies the `plan` precondition. A
   `plan` started before `spec` would plan the new brief against the old specification, and the survey does
   not refuse it. The fixing agent noted this and left it unchanged.
3. The route's `spec` row now does all the work of sending a later product cycle to `spec`, and it depends on
   "not converged", which is undefined. If a builder reads "converged" as "the latest `spec` decision is
   approved", that cycle skips `spec` and goes to `plan`.

Proposed resolution. Use one predicate, "the cycle's own `spec` decision": a `spec` decision in
`cycles/<cycle>/decisions/`, not in `_standing/` or in an earlier cycle. Apply it in three places.

1. The `spec` survey marks the amendment only when the cycle began with a brief and has no `spec` decision of
   its own, so the approval that stands is an earlier cycle's, an ingest's or an import's. Change `state.md`
   2.16, `pipeline.md` 2.9.3 (drop the parenthesis), and blueprint 4.11 and its N2 row.
2. The `spec` row reads: "the brief is approved and the cycle has no `spec` decision of its own, or its latest
   one asked for revision".
3. The `plan` precondition adds: "in a cycle that began with a brief, the approved `spec` decision is the
   cycle's own".

Also state that the survey's mark is not "opening an amendment" for the clearing rule of `state.md` 2.8 (lines
803 to 805). Otherwise every resumed `spec` run would clear its own review entries.

### R3. `gated()` in `each` mode accepts a retry that leaves out a story an earlier attempt admitted

Caused by: N3.

Where:

- `pipeline.md` 2.11.1, `gated()` (lines 1726 to 1743). Each attempt checks only the entries that attempt
  returned.
- `pipeline.md` 2.9.4, story-draft row (line 983): "no story is delivered while one is missing".
- Blueprint 4.10, Gate output row (line 618).
- `state.md` 2.16, `vsdd gate plan` row (line 1430): "Run again for a story it has admitted, it changes nothing
  and returns the same unit."

The problem. Attempt 1 admits S-1, and S-2 fails. The retry note names S-2. If the story-writer then reruns
only S-2 and returns only S-2's entry and unit, `gated()` returns `done`. S-1 is admitted but not in
`data.units`, and S-2 is delivered while S-1 is missing. That contradicts the 2.9.4 row and blueprint 4.10.

The fixing agent left out a check against the route's story list. That check would also catch this case.

There are two smaller points:

- When the agent fixes a story within one attempt, its earlier failing entry for that story still counts.
  That wastes one of the two attempts.
- "changes nothing" can be read as "does not measure again". An admitted story that the story-writer edits
  on the retry so that it fails must still be caught.

Proposed resolution:

1. In `gated()`, keep the set of subjects seen across attempts. In `each` mode:
   - only the last entry for each subject counts;
   - every subject seen in an earlier attempt must have a passing entry and its unit in the final result;
   - a missing subject is named in the retry note ("no result this time").

   I checked this with the eight cases above. It blocks the omission case and changes no other case.
2. The `feature` story-draft row also states that the units cover the route data's `stories` and
   `readiness`.
3. Reword the `state.md` 2.16 sentence: "Run again for a story it has admitted, it measures the gate again,
   admits nothing new, and returns the same unit when the story still passes."

### R4. A maintenance cycle's route is now defined as empty, and the lists that read the route have no entry for it

Caused by: N1. The original problem included "the verify survey's per-route check list reads the same missing
value", and for a maintenance cycle it still does.

Where:

- `state.md` 2.8, Cycle record row (line 736): the route is "empty for a maintenance cycle".
- `pipeline.md` 2.9.6 (line 1051): "Required checks by the cycle's route". It lists standard, first release,
  `trivial`, `bugfix` and `hotfix`.
- `pipeline.md` 2.9.7 (line 1063): the version is "computed from the route".
- `pipeline.md` 2.9.10 (lines 1158 to 1161): the fixes go through `verify` "in delta scope and a `release`
  decision like any other change".

The problem. A maintenance cycle reaches `verify` and `release` with no defined check list and no version
rule.

Proposed resolution. In 2.9.6, add "a maintenance cycle (no route): all that apply, in delta scope". In 2.9.7,
state the version rule for a maintenance cycle, for example a patch release, as for `bugfix`.

### R5. The new `plan` fixture depends on a route row whose test is not defined

Caused by: N1. It added the fixture "a standard-route feature whose delta specification is approved, with no
run records on disk, gives `plan`".

Where:

- `pipeline.md` 2.4, the fixture (line 325) and `packaging.md` 2.6 (line 516).
- `pipeline.md` 2.4, the row above the `plan` row (line 296): "a change request is in flight and neither
  delivered nor awaiting a decision" gives `feature`.

The problem. After the standard-route `spec` decision is approved, the request is still active. Its stories are
not written, so it is not delivered, and it awaits no decision. Read plainly, line 296 matches first and gives
`feature`, not `plan`. Nothing defines "delivered" for a request.

The fast-route fixture passes because its story is delivered. The new fixture passes only if "delivered" is
read as "the request's `feature` run has finished".

Proposed resolution. Reword the row: "a change request is in flight, its `feature` unit is not closed, and it
awaits no decision". A unit can be closed while its decision is open, as the `plan` unit is (line 313). The
`feature` checkpoint at decision `spec` then closes the unit, and both fixtures hold.

## Seen while checking, not caused by this round

These were not counted. They touch the same rules, and the first is now easy to fix because the route is in
`cycle.json`.

- **A resumed standard-route `feature` run holds no amendment.** Only `vsdd feature route` marks the run, in
  the delta step, which a resumed run skips as done. The `evolve` writes of the second run are therefore
  refused by the lock.
  - Fix: the `feature` survey marks the run when the cycle's `route` is `standard` and the cycle has no
    approved `spec` decision of its own. This is the same predicate as R2.
  - The Run record row of `state.md` 2.8 (line 739) also does not list `vsdd feature route` among its writers,
    although 2.16 says it writes the amendment flag there.
- **The first half of the `verify` precondition is also vacuous before a plan exists.** That is "every wave is
  closed" in `pipeline.md` 2.5 (line 386) and the `verify` row (line 302).
  - In the route table the `plan` row's position protects it. But a `verify` started by hand in a
    standard-route cycle before `plan` is not refused.
  - Blueprint 11.4's N1 row claims "a missing route cannot send a standard-route feature to `verify`". That
    holds only because of the row order.
  - Fix: "the plan unit is closed and every wave is closed".
