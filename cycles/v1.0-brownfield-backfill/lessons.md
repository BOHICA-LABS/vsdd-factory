---
document_type: lessons-codification
level: ops
cycle: v1.0-brownfield-backfill
producer: session-reviewer
timestamp: 2026-05-04T00:00:00Z
---

# Lessons Codified — v1.0-brownfield-backfill

> Lessons from this cycle that have been promoted to follow-up artifacts (stories or
> STATE.md Drift Items). Each entry records the gap, the evidence, and the disposition.

---

## RECURRENCE NOTE (D-1063) — D-1044(g)/D-995 class recurs one layer further out: BC-INDEX Stories-column + VP-INDEX Story-Anchors-row propagation legs

**Category:** process-gap (recurrence, not a new class)

**Session evidence** (2026-08-21, S-21.25 pre-TDD pass-5, F-S2125-P5-001/002): the D-1059 burst
that registered BC-3.08.001's Event 7 (`plugin.fuel_headroom_warning`) and bumped VP-079 to v1.20
correctly propagated the version-cite legs and the §Full Index row's six→seven event-type
enumeration, but self-deferred two further propagation legs that surfaced two clean-looking bursts
later (D-1060 through D-1062, three intervening passes, before the wider cross-index audit at pass
5 caught it): (1) BC-INDEX's own BC-3.08.001 Stories column never gained `S-21.25`, even though the
BC file's own §Traceability Stories row and §Story Anchor section were updated at the time Event 7
was registered to name S-21.25 as the anchor; (2) VP-INDEX's §Story Anchors VP-079 row (a
DIFFERENT row than the already-correctly-swept §Full Index row) retained the pre-registration
"six event types" enumeration and a now-doubly-stale `per BC-3.08.001 v1.19` version pin. Both are
the SAME underlying failure shape already codified at D-1044(g) (BC-version-bump-mid-cascade lacks
same-burst story-propagation-dispatch discipline) and D-995
([[L-BB-story-propagation-obligation-on-governing-bc-normative-prose-amendment]]) — a governing
artifact's version bump obligates a sweep of every downstream/sibling site citing it, and no
existing convention enumerates ALL such sites in one pass. This recurrence demonstrates the class
extends one layer further than either prior instance covered: D-995 was BC→story propagation;
D-1044(g) was BC-bump→story-frontmatter-and-body propagation; this recurrence is
BC-event-registration→INDEX-file-Stories-column-and-Story-Anchors-row propagation — a THIRD
distinct sweep-target surface within the same underlying obligation.

**Disposition (per explicit dispatch instruction):** NOT a new codification, NOT a new follow-up
story. Recorded as a recurrence anchored to the existing S-15.03 PRIORITY-A candidate — a
POLICY-14-leg-5 same-burst index-Stories-column sweep gate: whenever a BC-registration burst adds
a new anchor story to a BC's §Traceability/§Story Anchor sections, the SAME burst must sweep (a)
that BC's own BC-INDEX Stories column, and (b) every VP-INDEX row (both §Full Index AND
§Story Anchors, which are DISTINCT rows that can independently drift) whose event/property
enumeration references the same BC clause. Both findings (F-S2125-P5-001, F-S2125-P5-002) fixed
in scope at D-1063; no story-body defect — the S-21.25 story itself independently re-derived CLEAN
across all 7 previously-named risk areas at the same pass.

**Cites:** recurrence of [[L-BB-story-propagation-obligation-on-governing-bc-normative-prose-amendment]]
(D-995) and D-1044(g); companion to the D-996/D-998/D-1000/D-1004/D-1006 "fix/attest scoped to the
named site, not the class" family — this is the fourth generation of that family's underlying
insight applied to a NEW surface (cross-index Stories-column/Story-Anchors-row propagation, as
distinct from BC-body prose, story-body prose, index-aggregation-cell arithmetic, or
attestation-predicate adequacy). `[process-gap; recurrence; POLICY-8; POLICY-9; POLICY-14; BC-INDEX; VP-INDEX; Stories-column; Story-Anchors-row; story-propagation; state-manager; F-S2125-P5-001; F-S2125-P5-002; D-1044; D-995; D-1063; S-21.25; S-15.03-PRIORITY-A-anchor]`

## RECURRENCE NOTE (D-1064) — POLICY 19 (adr_version_cite_volatile_pin_prohibition) never applied to BC-1.03.019's own Traceability row across v1.0-v1.2 authoring + 5 prior adversary passes

**Category:** process-gap (recurrence-class candidate, not a new codification)

**Session evidence** (2026-08-21, S-21.25 pre-TDD pass-6, F-S2125-P6-001): a corpus-wide grep sweep
for POLICY 19 compliance (every BC Traceability row citing an ADR) found BC-1.03.019's own
Traceability ADR row was the SOLE outlier in the entire BC corpus, carrying a load-bearing
`ADR-039 v1.15 §Decision 5 Mitigation 1` version pin since its v1.1 authoring (D-1059) — a pattern
POLICY 19 explicitly prohibits precisely because ADR-039 continues amending (it already had, twice,
through §AMD-003 and multiple Erratum entries, by the time this pin went stale). The pin survived
five prior adversary passes (pass-1 through pass-5) of the S-21.25 LOCAL cascade, none of which
caught it, because each pass's rubric review focused on the story's own 7 named risk areas plus
whatever finding classes the immediately-prior pass had surfaced — no pass before pass-6 ran a
corpus-wide POLICY-19-specific grep against BC-1.03.019's own file. Sibling BC-3.08.001 carried the
identical pattern in its own Traceability ADR row for the same Event-7 provenance clause, introduced
at the same v1.25 registration burst (D-1059) and also un-caught through 5 subsequent bursts,
D-1060..D-1063 (sample) — not an exhaustive burst-by-burst enumeration.

**Disposition (per explicit dispatch instruction):** NOT a new codification, NOT a new follow-up
story. Recorded as a recurrence-class candidate anchored to S-15.03 PRIORITY-A — a tree-wide
POLICY-19 Traceability-row sweep gate applied at BC-authoring time (i.e., a lint/validator that
greps every new or amended BC's Traceability ADR row for a bare `ADR-NNN vX.Y` token and fails
closed if found, rather than relying on adversarial cascades to eventually catch it via corpus-grep).
This is a DIFFERENT failure shape than the D-1044(g)/D-995/D-1063 family (which concerns
propagation of a version bump OUTWARD to sibling sites) — this is a gap in APPLYING an existing
POLICY to a site AT AUTHORING TIME, only caught by a much-later corpus-wide audit. Both findings
(F-S2125-P6-001 HIGH, F-S2125-P6-002 LOW) fixed in scope at D-1064 by product-owner (BC-1.03.019
v1.2→v1.3, BC-3.08.001 v1.26→v1.27) with story-writer downstream cite-propagation (S-21.25
v1.4→v1.5); no story-body defect — the S-21.25 story itself independently re-derived CLEAN across
all 7 previously-named risk areas at the same pass.

**Cites:** distinct governance-discipline gap from the D-995/D-1044(g)/D-1063 propagation-sweep
family — this is a POLICY-application-at-authoring-time gap, not a propagation-sweep gap. Companion
observation: F-S2125-P6-003 (VP-079 internal six/seven event-type inconsistency, deferred to
architect) is itself a further instance of the SAME general shape (a correction applied at one site
— the Property Statement — not swept to a sibling site — the header comments — within the same
file), reinforcing that within-file sibling-sweep discipline (TD-VSDD-060) remains incompletely
internalized even at single-document scope. `[process-gap; recurrence-class-candidate; POLICY-19; adr_version_cite_volatile_pin_prohibition; BC-1.03.019; BC-3.08.001; Traceability-row; corpus-grep; state-manager; product-owner; story-writer; F-S2125-P6-001; F-S2125-P6-002; F-S2125-P6-003; D-1064; S-21.25; S-15.03-PRIORITY-A-anchor]`

## LESSON (D-1065) — S-21.19 is the first of the 7 split stories to converge its pre-TDD cascade; the fresh-context loop's value in surfacing a governance defect AFTER substantive convergence

**Category:** positive-signal / methodology note (not a process-gap)

**Session evidence** (2026-08-21, S-21.19 pre-TDD pass-7): S-21.19 reached BC-5.39.001 3-CLEAN
convergence (passes 5, 6, 7 all CLEAN) after 4 substantive remediation passes (1-4). Of the seven
split stories spawned from the D-1057 sizing-override decomposition of the converged S-21.11 v2.11
(S-21.19, S-21.20, S-21.21, S-21.22, S-21.23, S-21.24, S-21.25), S-21.19 is the FIRST to close its
own independent pre-TDD LOCAL adversarial cascade — 7 passes total, an asymptotic hygiene tail
(substantive findings at passes 1-4, zero substantive findings at passes 5-7). This establishes an
empirical baseline for the remaining six seams: S-21.25 is mid-cascade (7 passes, streak 1/3,
distinct 1 HIGH POLICY-19 governance finding at pass 6 after its own body had already been CLEAN
since pass 5); S-21.20/S-21.21/S-21.22/S-21.23 (Wave 7) and S-21.24 (Wave 8) have not yet started.

**Methodology observation:** S-21.19's own pass-6 finding pattern (zero findings, second consecutive
CLEAN pass) illustrates that a story's own body can be substantively converged well before its
governing BCs' cross-cutting governance properties (like POLICY 19 Traceability-row hygiene) are
independently audited — S-21.25's pass-6 HIGH was discovered by a corpus-wide grep sweep against
the GOVERNING BC (BC-1.03.019), not by re-reviewing the story body, which had already been CLEAN
since pass 5. This confirms the fresh-context adversarial loop's value extends beyond finding
story-body defects: periodic corpus-wide governance sweeps (POLICY-N compliance across ALL BCs, not
just the ones a given story cites) surface defects that per-story rubric review alone would miss,
because no single story's cascade has a natural trigger to audit BCs it does not itself amend.

**Disposition:** Not a process-gap requiring codification — this is a positive validation that the
BC-5.39.001 3-CLEAN protocol, applied independently per split seam, correctly distinguishes
"this seam's own perimeter is stable" from "the seam is ready for TDD in isolation from its
siblings" (the latter remains an orchestrator/human wave-sequencing decision, not an adversary
verdict). Recorded as a methodology note for the remaining six seams' own cascades, and as
context for the S-15.03 PRIORITY-A tree-wide POLICY-19 authoring-time sweep gate candidate
(D-1064) — such a gate would have caught F-S2125-P6-001 at BC-authoring time rather than five
adversary passes later. `[positive-signal; methodology; BC-5.39.001; 3-CLEAN-convergence; S-21.19; first-of-seven; split-seam; D-1057; D-1065; asymptotic-hygiene-tail; corpus-wide-governance-sweep; S-15.03-PRIORITY-A-context]`

## LESSON (D-1066) — WAVE 6 COMPLETE: both split seams with zero blocking DAG dependencies independently reached BC-5.39.001 3-CLEAN convergence; asymmetric pass-counts (7 vs 9) validate per-seam independence

**Category:** positive-signal / methodology note (not a process-gap)

**Session evidence** (2026-08-21, S-21.25 pre-TDD passes 8-9): S-21.25 reached BC-5.39.001 3-CLEAN
convergence (passes 7, 8, 9 all CLEAN) after a 9-pass cascade — 4 substantive remediation passes
(1-4), 2 index-propagation-residue passes (5-6, including a HIGH POLICY-19 governance finding at
pass 6 discovered AFTER the story body itself had already re-derived CLEAN at pass 5), then 3
consecutive CLEAN passes (7-9). Combined with S-21.19's own 7-pass convergence at D-1065, **both
Wave-6 seams (S-21.19 and S-21.25 — the two of the seven D-1057 split stories with zero DAG
dependencies blocking their own convergence work, `depends_on: [S-21.10]` and `depends_on: []`
respectively) have now independently reached full BC-5.39.001 3-CLEAN convergence, completing
Wave 6.** Fourth dispatch attempt for this closing burst — three prior state-manager delegates died
to API connection loss before any commit landed; this is the first successful commit for D-1066,
not a recovery from a partial or backfill state (TD-VSDD-053 single-commit discipline held: no
partial artifact was ever pushed by the three failed attempts).

**Methodology observation:** the two seams needed different pass-counts to converge (S-21.19: 7
passes; S-21.25: 9 passes) despite starting from the same D-1057 split-decomposition event and
running under the identical `.factory/policies.yaml` rubric. This asymmetry is expected and
correct, not a process anomaly: S-21.25's own governing BC (BC-1.03.019) had a POLICY-19
Traceability-row defect dating back to its v1.0 authoring (5 passes before it was caught by a
corpus-wide grep at pass 6), while S-21.19's governing BCs did not carry an equivalent latent
defect. This confirms the BC-5.39.001 3-CLEAN protocol correctly measures each seam's OWN
convergence state rather than assuming split seams inherit a shared clock from their common
parent — the D-1057 sizing-override decision explicitly anticipated this (per D-1057(k): "splitting
a converged spec does not inherit convergence for the split parts").

**Wave-sequencing implication:** with Wave 6 fully closed, the pipeline's next action is Wave 7
(S-21.20, S-21.21, S-21.22, S-21.23 — each starts its own pre-TDD cascade from pass-1, none has run
yet) followed by Wave 8 (S-21.24 capstone, STRICTLY LAST, gated on all five prior seams). Wave 7's
first three stories (S-21.20/21/22) each carry a known pre-existing drift item — a stale
`BC-1.03.017 v1.18` cite that must be re-anchored to `v1.19` before or during their own pass-1,
per the D-1060 deferral (extended at D-1064/F-S2119-P6-001 to also cover the decomposition-plan.md
§1 per-story detail and the STORY-INDEX sibling rows). S-21.23 cites only `BC-1.03.018` (confirmed
D-1062) and needs no re-anchor before its own pass-1. This front-loads the re-anchor work into the
FIRST burst of each affected story's cascade rather than deferring it further, avoiding a repeat of
the same class of index-propagation residue that cost S-21.25 two non-substantive passes (5-6) in
its own cascade.

**Disposition:** Not a process-gap requiring codification — this is a positive milestone marking
the completion of Wave 6 and validating that the per-seam independent-convergence discipline
(D-1057(k)) produces correct, if asymmetric, outcomes. Recorded as context for Wave 7's dispatch
planning — front-load the known BC-1.03.017 re-anchor rather than letting it surface as a
mid-cascade finding. `[positive-signal; methodology; milestone; BC-5.39.001; 3-CLEAN-convergence; wave-6-complete; S-21.19; S-21.25; split-seam; D-1057; D-1065; D-1066; asymmetric-pass-counts; wave-7-planning; wave-sequencing]`

## LESSON (D-1067) — Cycle-wide logs have no automated trim cadence, and the only related tool (`/compact-state`) feeds them rather than trimming them, so they grew unbounded until they broke state-manager burst reliability

**Category:** process-gap

**Session evidence** (2026-08-21, cycle-log-trim burst, root-caused from the six consecutive
D-1066 dispatch deaths): `decision-log.md`, `burst-log.md`, and `lessons.md` for
`v1.0-brownfield-backfill` grew to 21,539 / 29,806 / 11,330 lines respectively across the
continuous brownfield-onboarding cascade with no trim ever applied. The only existing tool with
"compact" in its name — the `/compact-state` skill (`plugins/vsdd-factory/skills/compact-state/`)
— does the OPPOSITE of what these files needed: it extracts historical content OUT OF STATE.md
and FEEDS it INTO these same cycle files (burst logs, adversary passes, session checkpoints,
lessons) to keep STATE.md itself under its own 200-line/415-line budget. There has never been a
tool or scheduled discipline that trims the cycle-file destination once content lands there. As a
direct consequence, the three files eventually exceeded the WASM-sandboxed PostToolUse validators'
`DEFAULT_FUEL_CAP` on nearly every Edit/Write/MultiEdit against them, each state-manager burst
touching them ballooned to roughly 40 minutes of wall-clock time, and six consecutive D-1066
seal-burst dispatch attempts died to "API connection lost mid-response" before any commit could
land — D-1066 was only rescued via a fourth-attempt direct commit of already-completed work.

**Root cause:** growth is asymmetric and directional. `/compact-state` (STATE.md → cycle files)
has an explicit trigger (STATE.md approaching its own size budget) and a well-defined target
(<200 lines). The cycle files themselves (decision-log.md, burst-log.md, lessons.md) have no
matching trigger or target — they are treated as unbounded historical logs by every existing
skill and hook, and nothing in the pipeline ever asks "is THIS file now too big to safely edit?"
until a validator starts failing closed on it (as `[D-954]` and `[D-442(e)]` both independently
recorded, months apart, without either resulting in a fix at the time).

**Disposition:** Section-aware archival at a named D-NNN cutoff boundary (active file retains the
current cascade forward from the boundary; everything before moves verbatim to a
`<log>-archive-through-D<NNN>.md` sibling file, with heading-count conservation independently
re-verified) is now the established remediation pattern for this class of drift item — see
`[D-954]`/`[D-442(e)]` RESOLVED this cycle via `decision-log-archive-through-D1056.md` /
`burst-log-archive-through-D1056.md` / `lessons-archive-pre-D1057.md`. This is anchored
**S-15.03 PRIORITY-A**: cycle logs must be trimmed at wave/epic boundaries, or proactively when a
file approaches the WASM validators' effective line-count budget (empirically, low thousands of
lines per file), not reactively after bursts start dying. A future S-15.03 PRIORITY-A story should
automate the trim trigger (e.g., a maintenance-sweep check that flags any cycle file above a
threshold) rather than relying on an AI agent noticing the file is large during an unrelated burst.
`[process-gap; cycle-log-bloat; wasm-fuel-exhaustion; burst-fragility; state-manager;
D-954; D-442(e); D-1057; D-1066; D-1067; S-15.03-PRIORITY-A; compact-state-asymmetry;
section-aware-archival]`

---

**Wave-7 floor-break (D-1077): consolidated full-perimeter consistency audit broke the asymptotic
floor in a single pass — process technique, not a defect.**

At D-1076 HEAD, a consolidated full-perimeter consistency-validator audit (all 10 audit classes,
all 6 stories simultaneously) surfaced all five remaining residual findings (C-W7-001..C-W7-005)
in one pass rather than the one-or-two-per-pass asymptotic pattern observed in prior Wave-7
rounds. This is a process-technique effect: the full-perimeter audit eliminates the partial-scan
selection bias that would cause some cross-story sibling-sweep misses to go undetected until a
later pass. The findings themselves were consistent with already-codified TD-VSDD-059/060
patterns (sibling-sweep misses, missing durable-gate task, missing FSR entry) — no new failure
mode or new codification was required. References: TD-VSDD-059 (paper-fix detection), TD-VSDD-060
(sibling-site sweep). `[process-technique; full-perimeter-audit; floor-breaking; consistency-validator;
TD-VSDD-059; TD-VSDD-060; D-1077; asymptotic-convergence; wave-7]`

---

**[codified] POLICY 19 must cover story-bodies in addition to behavioral-contracts-traceability-rows — omitting story-bodies leaves a sweep gap that re-accumulates forbidden ADR version pins in story bodies across each re-anchor pass.**

At D-1079 pass-7/R6, finding F-S2123-P7-P19-001 MED surfaced 6 ADR-039 version-pin cites in S-21.23's story body (`ADR-039 §Decision 3 v1.10` and `ADR-039 §Decision 3 v1.9` at 6 sites). POLICY 19, which prohibits load-bearing ADR version pins, was written to cover `behavioral-contracts-traceability-rows` — the class where the pin first appeared. Story body text was not explicitly in scope, so story-writer sweeps at earlier passes did not catch the story-body pins. The root cause is the scope enumeration in POLICY 19's `applies_to` field: it named only the initially-affected artifact type, not the broader logical class. The fix: extend POLICY 19 scope to `story-bodies` (policies.yaml v1.4.25) so that all future story-writer re-anchor sweeps are instructed to check story bodies as well as traceability rows. Changelog entries remain exempt (they record historical provenance; stripping version info from changelogs destroys audit trail).

**Disposition:** POLICY scope enumerations must explicitly cover every artifact class that contains the prohibited pattern — not just the class where the pattern was first detected. When a new finding class emerges in a previously unguarded artifact type (story bodies, ADR body narrative, architecture docs), the remediation should simultaneously: (1) fix the current instance, and (2) extend the POLICY scope to prevent silent re-accumulation in the same artifact type on the next re-anchor pass. `[codified; POLICY-19; story-bodies; ADR-version-pin; scope-enumeration; F-S2123-P7-P19-001; D-1079; wave-7]`

---

**[codified] Per-physical-line grep misses wrapped version-pin cites — multiline-normalized `tr '\n' ' ' | grep` detector is mandatory for all re-anchor sweeps.**

At D-1079 pass-7/R6, finding F-S2119-R6-001 MED revealed that S-21.19's Task 2 contained `ADR-044 v1.3` split across two physical lines ("`ADR-044`" on one line, "`v1.3`" on the next). The D-1078 pass-6/R5 sweep claimed 14 corrected sites but missed the 15th because it used single-line `grep 'ADR-044 v1.3'`, which cannot match tokens that cross a physical line boundary. This is a recurrence of F-S2119-P3-001, which first identified the line-wrap mechanism. The root cause: D-1078 documented the correct `tr '\n' ' ' | grep -oE 'ADR-[0-9]+ v[0-9.]+'` fix in its sweep log but the fix was NOT carried forward as a process discipline in POLICY 5 — meaning the next re-anchor sweep was free to revert to per-line grep.

**Disposition:** POLICY 5 (re-anchor sweep discipline) codified the mandatory `tr '\n' ' ' | grep` normalized detector at D-1079 (policies.yaml v1.4.25). Any version-pin sweep that uses `grep 'ADR-NNN v1.X'` directly on a file without the `tr` normalization stage is non-compliant with POLICY 5 and will silently miss wrapped cites. The `tr '\n' ' ' | grep -oE 'ADR-[0-9]+[[:space:]]+v[0-9.]+'` form is the canonical detector; sed-or-awk alternatives are acceptable provided they also normalize across physical lines before matching. `[codified; POLICY-5; multiline-sweep; tr-normalized; re-anchor-discipline; F-S2119-R6-001; F-S2119-P3-001-regression; D-1079; wave-7]`

---

**[codified] A "POLICY 8 full propagation" claim for a BC version pin re-anchor is only valid if it includes a table-cell-aware grep that isolates the pipe-delimited BC-table Version column cell — free-text body sweep is insufficient.**

At D-1080 pass-8/R7, findings F-S2120-R7-001/F-S2121-P8-001/F-S2122-P8-001 (MED×3) revealed that three stories (S-21.20/21/22) had their BC-table Version column cells stuck at v1.25 even though D-1079 claimed "POLICY 8 full propagation" for BC-1.03.017 v1.25→v1.26. The D-1079 re-anchor sweep checked frontmatter `behavioral_contracts:`, H1 cite, and body narrative cites using free-text grep, but did not run a table-cell-aware grep (`grep -nE '\| *BC-1\.03\.017 *\| *v1\.[0-9]+'`) to isolate the Version column cell in the pipe-delimited BC table. The pipe-delimited table format (`| BC-1.03.017 | v1.25 |`) is structurally distinct from narrative prose and is not reliably captured by a free-text search for `BC-1.03.017 v1.25`. A sweep can find ALL narrative occurrences and declare "full propagation" while the table cell silently stays at the old version.

The consequence was significant: S-21.20 had reached 3/3 CONVERGED PROVISIONAL at D-1079 (streak 2/3→3/3 ADVANCE). Pass-8 re-confirmation found the BC table cell had NOT been propagated, which constitutes a genuine POLICY 8 violation. Streak reset 3/3→0/3.

**Disposition:** POLICY 8 TABLE-CELL-AWARE PARITY GATE codified at D-1080 (policies.yaml v1.4.26): any burst claiming "POLICY 8 full propagation" for a BC version pin re-anchor MUST run and capture a table-cell-aware grep (`grep -nE '\| *<BC-ID> *\| *v[0-9]'`) to isolate the BC-table Version column cell BEFORE attesting propagation. An unbacked "full propagation" claim is itself a POLICY 8 finding. The captured stdout MUST appear in burst-log Dim-2/Dim-6 evidence per D-449(a) literal-shell discipline. `[codified; POLICY-8; table-cell-aware-parity-gate; BC-version-pin; re-anchor-propagation; F-S2120-R7-001; F-S2121-P8-001; F-S2122-P8-001; straggler-class; D-1080; wave-7]`

---

**A field-identical sibling-struct type name is still a spec-vs-code defect the adversary must catch — structural similarity is not a substitute for verifying the actual call graph.**

At D-1082 (ADR-046 pass-25), finding F-P25-001 (MED) revealed that ADR-046 §Decision 1 and BC-7.07.001 Invariant 3b had annotated the value `renew_lock_if_holder` resolves at its holder-present step as `lock_state: FactoryLock`. Ground truth: `renew_lock_if_holder` performs its own independent `flp::parse_factory_lock(content)` parse at that step, which returns `LockState` (crate `factory-lock-parse`) — a distinct type from `FactoryLock` (crate `factory-lock`, produced only by the different function `factory_lock::parse_lock`, never called on this path). The two structs are field-identical (same field names, same field types), which is precisely why the misattribution survived 24 prior adversarial passes without producing any observable behavioral test failure or internal contradiction — nothing in the spec's own text was inconsistent with itself; the defect was only visible by tracing the actual function call and its actual return type against `crates/factory-lock/src/lib.rs` + `crates/factory-lock-parse/src/lib.rs`, not by reading the spec in isolation. This is the same underlying pattern as TD-VSDD-059 (paper-fix detection: a rename or doc-comment can look like a fix without being one) applied in reverse — here, a type name can look correct (same shape, same usage pattern) without being the actual type the code produces. The finding was originally caught only as a LOW "type-provenance imprecision" nit (O-P24-001) at pass-24 and required a second, more skeptical look to escalate it to a genuine MED spec-vs-code mismatch at pass-25.

**Disposition:** When a spec annotates a value's type by name (not just by shape/usage), the adversary MUST independently trace the actual function that PRODUCES that value in the referenced source file and confirm the type name matches — field-identical shape between two distinct types in sibling crates is not sufficient grounds to treat a type-name citation as low-severity or cosmetic. This generalizes beyond this specific `FactoryLock`/`LockState` pair: any spec citing a concrete Rust type name for a value sourced from a shared/reused function (not a fresh local computation) should have that citation checked against the function's actual return type, not just against whether the cited type "would work" given identical fields. Not a [process-gap] — this is a content defect in the spec artifact itself, correctly caught by the adversarial review process operating as designed (the process caught it; the spec authors introduced it). `[content-defect; type-provenance; spec-vs-code; sibling-struct; field-identical-types; F-P25-001; O-P24-001-escalation; D-1082; adr-046-gate; not-process-gap]`

---

**ADR-vs-BC implementing-story anchor drift: when a spec cites a story ID in narrative prose before that story is registered in every companion artifact's Traceability section, the citation itself is not a substitute for actually closing the cross-reference.**

At D-1082 (ADR-046 pass-25), finding F-P25-002 (MED) revealed that ADR-046 named S-17.05 as the implementing story in its own narrative text, while all three companion BCs (BC-4.17.001, BC-7.07.001, BC-5.40.001) still carried `[pending]` placeholders in their Traceability §Stories rows and §Story Anchor fields — and ADR-046's own "referenced in the File-Change Plan" cross-reference to S-17.05 did not actually resolve, because S-17.05 was not yet listed in the File-Change Plan itself. The root cause is a variant of TD-VSDD-060 (sibling-site sweep): when a story anchor is decided (here, during S-17.05's drafting), the decision needs to propagate to EVERY sibling artifact that references "the implementing story" — the ADR's own File-Change Plan, and each companion BC's Traceability section — in the same burst, not left as narrative-only in one location while formal cross-reference fields elsewhere still say `[pending]`. A reader trusting only the ADR's prose would believe the anchor was already closed; a reader (or validator) checking the BCs' formal Traceability fields would find it open.

**Disposition:** Whenever a spec artifact's narrative prose asserts a fact (e.g., "the implementing story is S-N.NN") that should also be reflected in a structured/formal field elsewhere (a Traceability table row, a File-Change Plan entry, a §Story Anchor field), the two MUST be updated in the same burst — narrative-prose-only assertions are a distinct drift class from the formal field they describe, and a sibling-site sweep across all artifacts sharing that cross-reference is required before declaring the anchor resolved. Not a [process-gap] — per adversary disposition, this is a content/traceability defect in the artifacts themselves, not a gap in the adversarial-review process (which correctly caught it). `[content-defect; traceability; story-anchor; sibling-site-sweep; TD-VSDD-060-variant; F-P25-002; D-1082; adr-046-gate; not-process-gap]`

---

**A self-referential version-bump DIRECTIVE inside an ADR's own File-Change Plan is itself a parity leg — TD-VSDD-060 sibling-sweep discipline extends to instruction rows, not just content rows.**

At D-1083 (ADR-046 pass-26), finding F-P26-001 (MED) revealed that ADR-046's File-Change Plan carries a row that does not describe the ADR's own content — it INSTRUCTS a downstream artifact (ARCH-INDEX) what version to cite for this ADR. That row had drifted stale: it still directed "bump to v1.10," leading with the pass-21/F-P21-001 disposition, even after the pass-25 architect edit (same session, immediately prior burst) had already advanced the ADR to v1.11 and added this same table's own new S-17.05 row (F-P25-002). The pass-25 edit swept every locus stating the ADR's substantive content (§Decision 1(b), the type-provenance loci, the Changelog/frontmatter) but did not sweep this SIBLING instruction row, because the row is not itself an assertion ABOUT the ADR — it is a downstream-facing directive whose own correctness depends on staying pinned to whatever version the CURRENT revision produces, a target that moves every single time the ADR's version bumps, including the very revision that just fixed something else entirely.

**Disposition:** TD-VSDD-060 (sibling-site sweep on value changes) already required sweeping every site that CITES a changed value. This generalizes it one layer further: a version-bump obligates a sweep not just of every site citing the ADR's OLD version, but of every site whose own text INSTRUCTS what the ADR's NEW version should be cited as elsewhere — a self-referential directive is itself a parity leg, and it goes stale on every single-version bump the ADR undergoes, not just on bumps that change its substantive content. Practically: any architect edit that changes an ADR's `version:` frontmatter field must include a check of the File-Change Plan (or any other section) for rows of the form "artifact X's row should cite version vN" and update N to match the version this very edit produces — a `grep -n "version bump to\|bump to v[0-9]"` sweep against the ADR's own body, run as part of every version-bump edit, is sufficient to catch this class before adversarial review does. Not a [process-gap] — per adversary disposition, this is a content defect in the ADR artifact itself (the row's own text was wrong), correctly caught by the adversarial review process operating as designed. `[content-defect; self-referential-directive; sibling-instruction-row; TD-VSDD-060-variant; version-bump-obligation; F-P26-001; D-1083; adr-046-gate; not-process-gap]`

---

**Resolving a `[pending]` implementing-story anchor to a real story ID must sweep ALL sibling loci in the SAME burst — §Story Anchor (incl. any cardinality quantifier), §Traceability §Stories, status/lifecycle_status parity, `inputs:` completeness, and every prose mention, not just the Traceability rows the initial fix touched.**

At D-1084 (ADR-046 pass-27), findings F-P27-001 (HIGH), F-P27-002 (MED), and F-P27-003 (MED) all trace back to the SAME root event: the pass-25 (D-1082) fix that resolved BC-5.40.001's and BC-7.07.001's Traceability §Stories rows from `[pending]` to the confirmed implementing story S-17.05. That fix correctly updated the Traceability §Stories rows in both files, but left three sibling loci un-swept for two full passes (25 and 26): (1) each BC's own §Story Anchor section — a DIFFERENT heading than §Traceability §Stories, which had never been touched and in BC-5.40.001's case carried a now-incorrect cardinality quantifier ("Dual-story anchor" when the story count had grown to three); (2) BC-7.07.001's `status: draft`/`lifecycle_status: active` frontmatter contradiction, pre-existing and unrelated in root cause to the S-17.05 anchor but caught by the same pass; (3) BC-7.07.001's `inputs:` frontmatter completeness relative to its own body's code/BC citations. A partial anchor-resolution that touches only the primary Traceability row leaves HIGH-severity contradictions in sibling sections — §Story Anchor is a formally-distinct field from §Traceability §Stories even though both describe "the implementing story," and a reader (or validator) trusting one while the other still says something different (or omits the story entirely) has no way to know which is authoritative.

**Disposition:** This generalizes TD-VSDD-060 (sibling-site sweep on value changes) to a specific, recurring high-value checklist for the "resolve a `[pending]` story-anchor placeholder" class of fix, distinct from — but structurally identical in shape to — the D-1082/F-P25-002 lesson and the D-1083/F-P26-001 self-referential-directive lesson already codified above: whenever a burst resolves an implementing-story `[pending]` placeholder to a real story ID anywhere in a spec artifact, the SAME burst must sweep, at minimum: (a) §Story Anchor (including any cardinality-quantifier word like "Dual"/"Tri" that encodes the story count), (b) §Traceability §Stories (the row this class of fix most often touches first), (c) status/lifecycle_status parity across the file and its BC-INDEX row (a pending amendment does not itself imply draft status, but any existing status contradiction surfaced during the same review should be resolved in the same burst rather than left for a later pass), (d) `inputs:` frontmatter completeness for any newly-cited code/BC dependency the resolved story's implementation touches, and (e) every other prose mention of "the implementing story" or "pending story" in the file. Not a [process-gap] — per adversary disposition, these are content defects in the BC artifacts themselves (the sibling sections' own text was stale or incomplete), correctly caught by the adversarial review process operating as designed; the process is not missing a capability, the BC-authoring/fix bursts have twice now under-swept the sibling-loci set for this specific placeholder-resolution class. `[content-defect; story-anchor; sibling-sweep; TD-VSDD-060-variant; cardinality-quantifier; status-lifecycle-parity; inputs-completeness; F-P27-001; F-P27-002; F-P27-003; O-P27-001; D-1084; adr-046-gate; not-process-gap]`

---

**A fix landing on 1 of N siblings carrying an identical claim doesn't just leave the other N-1 siblings' gap open — it can inject a FALSE cross-reference/premise into the fix's OWN disposition text, because the fixer asserted something about a sibling it never actually checked.**

At D-1085 (ADR-046 pass-28), findings F-P28-001 (HIGH) and F-P28-002 (MED) both trace to the SAME root event: the pass-27 (D-1084) fix that resolved BC-7.07.001's `inputs:` completeness and `status:` contradiction. That fix's own disposition prose made two comparative claims about sibling BC-4.17.001 — "mirroring sibling BC-4.17.001's input set" (F-P27-003) and "sibling BC-4.17.001/BC-5.40.001 both carry `status: active`" (F-P27-002) — and BOTH were false: BC-4.17.001 did not yet contain the cited crate in its own `inputs:` at that time, and BC-4.17.001 is correctly `status: draft` (its own base deliverable, story S-17.05, has not shipped), not `active`. The pass-27 fixer never actually opened BC-4.17.001 to verify either claim; it asserted a sibling's state as supporting rationale for its own disposition without checking it. This is a distinct failure mode from ordinary sibling-sweep omission (D-1084's lesson, immediately above): there, sibling loci were simply left untouched; here, the fix's own justifying prose made an affirmative, checkable, and false claim about a sibling it did not verify.

**Disposition:** When fixing a claim shared across N artifacts, the correct discipline is not just "enumerate and fix all N" (per TD-VSDD-060 and the sibling-sweep lesson above) but also: NEVER write a disposition asserting a sibling artifact's current state ("mirrors X", "same as sibling Y", "parity with Z") without opening and verifying that sibling's actual current content at fix time. A comparative claim used as supporting rationale is itself a checkable fact, and an adversary (or a careful self-review) can and should verify it independently of whether the primary fix itself was correct — the primary fix values (here, BC-7.07.001's `inputs:`/`status:` VALUES) can be entirely correct while the PROSE justifying them is false. Tag as content-defect discipline; relates to but is distinct from TD-VSDD-060 (sibling-site sweep is about propagating a VALUE change to all sites; this is about not fabricating an unverified claim about ANOTHER site's value while fixing THIS site). Not a `[process-gap]` — the adversarial-review process caught it correctly; the pass-27 fix burst asserted unverified facts. `[content-defect; false-premise; unverified-sibling-claim; comparative-disposition-prose; TD-VSDD-060-adjacent; F-P28-001; F-P28-002; D-1085; adr-046-gate; not-process-gap]`

---

**[codified] A self-referential version-bump DIRECTIVE that hard-codes its target as a literal number will recur indefinitely — the only structural fix is to make the directive read the artifact's own live `version:` field, not to keep patching the literal each time it's caught stale.**

At D-1085 (ADR-046 pass-28), observation O-P28-002 (`[process-gap]`, LOW) caught ADR-046's own File-Change Plan ARCH-INDEX sync-instruction row stale for the THIRD time: F-P25-002 (D-1082) added a new File-Change Plan row without sweeping the directive forward; F-P26-001 (D-1083) caught and rewrote a stale "v1.10" straggler to "v1.12"; pass-28 caught it again — and, had it been patched the same way a third time (bump the literal to "v1.13"), it would have gone stale a FOURTH time the moment this very burst's fix landed, since the directive's own correctness depends on staying pinned to whatever version the CURRENT revision produces, a target that moves on every single version bump the ADR undergoes. Patching the literal number is a treadmill, not a fix — the D-1083 lesson (already codified) correctly identified that a self-referential directive is itself a parity leg requiring sweep, but did not resolve the recurrence, because sweeping a literal-valued directive still leaves a literal-valued directive that will go stale again on the very next bump.

**Disposition:** ROOT-CAUSE FIXED this burst, not symptomatically patched a third time. The directive is restructured to a VERSION-STABLE instruction: instead of "bump ARCH-INDEX's ADR-046 row to vX.Y" (a literal that must be re-edited every single version bump), it now reads "bump ARCH-INDEX's ADR-046 row to match this ADR's current frontmatter `version:` field" — a construction that is correct by definition regardless of what the ADR's version number is at any future bump, because it delegates to the live field rather than embedding a snapshot of it. This generalizes beyond ADR-046: any spec artifact containing a directive of the form "the downstream artifact should cite version vN" is structurally guaranteed to drift on every subsequent version bump unless N is expressed as a reference to the artifact's own live version field rather than a literal number. When authoring or fixing such a directive, prefer the version-stable form from the start; when found in literal form during review, restructure it (don't just re-literal-patch it) — the recurrence count (3+ occurrences here) is itself the signal that literal-patching is the wrong fix class. `[codified; process-gap; self-referential-directive; version-stable-directive; TD-VSDD-060-generalization; recurrence-class; O-P25-002; F-P26-001; O-P28-002; D-1083; D-1085; adr-046-gate; root-cause-fix]`

---

**A cross-language home-crate mis-attribution can survive many passes of "attribution audit" scrutiny undetected, because every prior audit only re-checked SYMBOLS that had ALREADY been flagged once — the audit's own scope was implicitly bounded by its own history, not by an exhaustive symbol inventory.**

At D-1086 (ADR-046 pass-29), finding F-P29-001 (HIGH) revealed that ADR-046 self-contradicted on the private Rust fn `rewrite_expires_at`'s home crate: one locus (F-P10-001's own v1.8 citation) correctly placed it in `crates/factory-lock/src/lib.rs`, while two OTHER loci (the Companion Amendment 2 PC4-reconciliation bullet and the v1.8 Changelog entry's own closing sentence) wrongly described it as living inside the bash script `factory-lock-write.sh`. This ADR had already been the subject of a dedicated, successful, ADR-wide "home-crate/hedge audit" at pass-7 (F-P7-001), which decisively pinned `trim_git_email`'s shared home to `crates/factory-lock` across every hedge locus that mentioned it — but that audit, by construction, only swept loci that CITED `trim_git_email` by name; it never established a general inventory of every cross-language symbol this ADR makes claims about and checked each one's file-of-record. `rewrite_expires_at` was never on that list, so the pass-7 audit's success created a false sense that "the home-crate attribution class" was closed, when in fact only the ONE symbol that pass-7 investigated had been verified — `TTL_SECONDS`, `LockState`, `STATE_MD_MAX_BYTES`, and `rewrite_expires_at` each required (or, in `rewrite_expires_at`'s case, still require as of this pass) their OWN independent verification pass.

**Disposition:** When an ADR or BC makes a claim of the form "symbol X lives in crate/file Y" for a cross-language or cross-crate implementation detail (a function name, a constant, a type), do not treat a prior successful "home-crate audit" for a DIFFERENT symbol as evidence the ADR's home-crate attributions are generally sound — each symbol is an independent claim requiring independent verification against the actual source file. The correct discipline is a FULL-DOCUMENT symbol inventory (every named function, constant, and type the artifact makes a concrete "lives at path P" claim about), each one checked by inspection against the cited file, not a spot-check triggered only by a symbol some prior pass happened to flag. This generalizes the D-1082/F-P25-001 lesson (spec-vs-code type-provenance) from "verify the type a function RETURNS" to "verify the crate/file a symbol IS DEFINED IN" — both require tracing the actual source, not trusting a prior pass's audit scope as if it were exhaustive. Not a `[process-gap]` — per adversary disposition, this is a content defect in the ADR/BC artifacts themselves (a genuinely new defect class, not a regression of a prior fix), correctly caught by the adversarial review process operating as designed. `[content-defect; home-crate-mis-attribution; cross-language-symbol; spec-vs-code; audit-scope-not-exhaustive; F-P29-001; D-1086; adr-046-gate; not-process-gap]`

---

**[process-observation] Three consecutive passes (27, 28, 29) each shed a partial-fix regression of the immediately-prior pass's own fix — the behavioral core has converged, but the metadata/hygiene layer (`inputs:` completeness, array-ordering, cross-reference accuracy) is not reaching literal 3-CLEAN under manual sweep discipline alone; this is an ASYMPTOTIC-FLOOR pattern, not a random-noise pattern.**

At D-1086 (ADR-046 pass-29), the adversary's own Part B assessment explicitly separated two classes: the BEHAVIORAL core (write-composition table, five-outcome table, identity-gating logic, event-sourcing struct-variant text) has been VERIFIED CLEAN across three consecutive passes (27, 28, 29) with zero regressions — the substantive design is sound and stable. But the spec has NOT reached literal 3-CLEAN, because each of the last three passes has found a METADATA-layer defect that traces directly to the immediately-prior pass's OWN fix: pass-28's F-P28-002 (de-scoped BC-5.40.001 from a sibling POLICY 18 sweep) produced pass-29's F-P29-002; pass-27's O-P27-001 `modified:`-array reorder fix was re-regressed by pass-28's own v1.30 edit, producing pass-29's F-P29-003. This is structurally different from ordinary "adversary keeps finding new bugs" — it is a fix-introduces-its-own-next-finding cascade confined entirely to the bookkeeping/metadata layer (inputs arrays, ordering arrays, cross-reference prose), never the behavioral content. Three of the last three passes fit this pattern exactly.

**Disposition:** This is recorded as a `[process-observation]`, not resolved this burst per explicit task instruction (a convergence-strategy question anchored for human decision, not a spec content defect this state-manager burst can fix). The pattern suggests that fully-manual metadata-consistency sweeps (recompute every sibling's `inputs:`/`modified:`/cross-reference-prose state by hand, every burst) have an inherent per-burst error rate for LARGE multi-artifact bursts (3-4 files touched simultaneously) that is not converging to zero through additional manual passes — the same class of observation S-15.03 PRIORITY-A automation is meant to address for OTHER artifact classes (BC-INDEX/ARCH-INDEX `last_amended` growth, cycle-log trim cadence). Whether this warrants (a) continuing literal 3-CLEAN under manual discipline (accepting the current per-pass floor as noise that will eventually clear), (b) accepting D-386 Option C asymptotic-acceptance for THIS gate specifically (as already applied to the E-10 sub-cycle and the F5 cycle), or (c) building a structural cross-artifact consistency checker (mirroring S-15.03's planned automation) before continuing the manual cascade, is a human decision — not resolved here. `[process-observation; asymptotic-floor; partial-fix-regression-cascade; convergence-strategy; metadata-layer-vs-behavioral-core; F-P28-002; F-P27-003-vs-F-P29-003; D-1086; adr-046-gate; anchored-human-decision; not-resolved-this-burst]`

---

**[convergence-strategy][codified] Spot-fixing only the single locus the adversary explicitly flags perpetuates the exact partial-fix-regression cascade the D-1086 process-observation identified — switching to a COMPREHENSIVE per-dimension sweep (fix the flagged locus AND every sibling that shares the same defect class, in the same burst) is what closed the metadata layer.**

At D-1087 (ADR-046 pass-30), findings F-P30-001 (HIGH) and F-P30-002 (MED) were BOTH instances of defect classes the immediately-prior three passes (27, 28, 29) had each already fixed on ONE artifact but never swept to every sibling that shared the identical defect: F-P30-001 is the F-P29-003 `modified:`/Changelog array-ordering-parity class, previously fixed only on BC-7.07.001, left un-swept on BC-4.17.001 and BC-5.40.001 (which carried the identical ascending-vs-descending mismatch the whole time); F-P30-002 is the F-P28-001/F-P29-002 `inputs:`-completeness class, previously swept across the three companion BCs but never applied to ADR-046 itself via a MANDATORY complete-document audit (only a spot-check, which pass-28 had already tried and missed 2 of the 6 omissions pass-30 found). This confirms the D-1086 process-observation's asymptotic-floor pattern was NOT random noise — it was the direct, predictable consequence of remediation bursts fixing only the specific locus an adversary pass named, rather than exhaustively fixing every artifact that shares the same defect class.

**Disposition:** This burst's remediation deliberately switched technique: (1) for F-P30-001, product-owner did not just reorder the two BCs the adversary explicitly named — it ran a full 3-BC cluster parity audit (version / Changelog-head / modified-head / last_amended-prefix parity, `inputs:` completeness, §Story-Anchor cardinality) across ALL THREE companion BCs, confirming BC-7.07.001 already clean rather than assuming it based on the pass-29 fix; (2) for F-P30-002, architect ran a MANDATORY complete inputs-completeness audit of ADR-046's ENTIRE document body (every load-bearing current-state file citation, not just the two files the adversary flagged), finding and fixing 6 omissions instead of the 2 initially named, while explicitly documenting the ones REJECTED as non-load-bearing padding (avoiding the opposite failure mode of over-adding). Pass-30 returned ZERO spec-vs-code contradictions — both findings were pure metadata parity — which is consistent with the behavioral core having been genuinely stable since pass-27 (per the D-1086 observation) and suggests the metadata layer itself may now be closing under this comprehensive-sweep technique where single-locus spot-fixes had not. **This is a hypothesis pending confirmation, not a closed result**: if pass-31 returns CLEAN, this comprehensive-per-dimension-sweep technique is the mechanism that closed the metadata layer and should be the DEFAULT remediation technique for all future spec-convergence findings on multi-artifact clusters, not merely a one-off response to this specific finding pair. If pass-31 finds further metadata stragglers despite the comprehensive sweep, the D-1086 process-observation's harder question (structural cross-artifact consistency automation, per S-15.03 PRIORITY-A) remains open and the technique alone is insufficient. `[convergence-strategy; codified; comprehensive-sweep; per-dimension-audit; anti-spot-fix; F-P30-001; F-P30-002; D-1087; adr-046-gate; asymptotic-floor-followup; pending-pass-31-confirmation]`

---

**[convergence-strategy][codified] Comprehensive audits yield stragglers a spot-fix of the flagged findings alone would leave for future passes — extending the D-1087 sweep technique from "every sibling BC sharing the flagged defect class" to "every cross-anchor citation and every spec-inputs claim inside the SAME BC a flagged finding already touched" is a further convergence accelerant, confirmed (not refuted) by this pass.**

At D-1088 (ADR-046 pass-31), the two flagged findings — F-P31-001 (MED, BC-5.40.001 `inputs:` completeness) and F-P31-002 (MED, BC-7.07.001 cross-reference retarget) — were fixed by product-owner exactly as named. But per the D-1087 convergence-strategy lesson's own discipline (fix the flagged locus AND sweep every related surface, not just the named one), product-owner additionally ran (a) a full cross-anchor semantic audit — open every `BC-X.YY.ZZZ §Section`/`PCn`/`Invariant-N` citation in BC-5.40.001's and BC-7.07.001's bodies and verify it against the cited BC's actual section content, not just the citation F-P31-002 already flagged — and (b) a full spec-inputs completeness audit — check every load-bearing claim BC-7.07.001's body makes against files absent from its own `inputs:`, not just the fact that F-P31-001 had already found a gap on the sibling BC-5.40.001. These two audits caught 3 GENUINE additional defects the 2 flagged findings alone would not have surfaced: BC-5.40.001's own PC1/PC2 mis-cite of "BC-6.23.001 PC3/PC4" (should be PC4-only — PC3 is an unrelated acquire-path refusal), and 5 missing load-bearing spec inputs on BC-7.07.001 (BC-5.40.001, BC-5.41.003, BC-1.15.001, BC-2.02.011, domain-spec/invariants.md). Left unaudited, these would have surfaced as separate findings on pass-32 or pass-33, each resetting nothing (streak already 0/3) but each costing a full pass-cycle before discovery.

**Disposition:** This CONFIRMS the D-1087 convergence-strategy hypothesis was on the right track, and generalizes it one dimension further: the useful unit of "comprehensiveness" for spec-convergence remediation is not just "every sibling artifact sharing the flagged defect's CLASS" (D-1087's scope: modified:-array ordering swept across all 3 BCs; inputs:-completeness swept across the whole ADR document) but ALSO "every claim of the SAME KIND as the flagged claim, inside the SAME artifact the fix already opened" — if a finding flags one wrong cross-anchor citation, audit every cross-anchor citation in that file while it's already open; if a finding flags one `inputs:` gap, audit the whole file's body for other load-bearing citations absent from `inputs:` while it's already open. This pass did NOT reach a literal-CLEAN verdict (2 findings were flagged, meaning genuine defects existed BEFORE the audit ran) — so the technique does not, by itself, guarantee a CLEAN pass on first application to a newly-audited dimension. But its YIELD (3 extra genuine defects caught same-burst, at zero marginal pass-cycle cost, because the file was already open for editing) is exactly the kind of accelerant the D-1086 asymptotic-floor process-observation called for. Practically: whenever a remediation burst opens a BC file to fix one flagged cross-reference or one flagged `inputs:` gap, the fixer should default to running the FULL cross-anchor-citation audit or FULL spec-inputs-completeness audit on that file in the same pass, not just the one flagged locus — the marginal cost of auditing the rest of an already-open file is far lower than the cost of a dedicated future pass discovering the same defect piecemeal. Not a `[process-gap]` — per adversary disposition, these are content defects in the BC artifacts themselves, correctly caught by comprehensive-audit discipline operating as designed. `[convergence-strategy; codified; comprehensive-audit-yield; cross-anchor-audit; spec-inputs-audit; same-file-broader-scope; F-P31-001; F-P31-002; D-1088; adr-046-gate; convergence-accelerant; confirms-D-1087-hypothesis]`

---

**[codified][process-gap] The `modified:`-array-head-omission-on-version-bump defect has now recurred THREE times (F-P29-003, F-P30-001, F-P32-001) — a version bump that correctly updates `version:` + the `## Changelog` table + `last_amended:` but forgets to prepend the corresponding `modified:`-array head entry is a recurring SHAPE, not three independent one-off slips, and manual per-burst discipline alone is not preventing it.**

At D-1089 (ADR-046 pass-32), finding F-P32-001 (HIGH) found BC-7.07.001's `modified:` array missing its own v1.32 entry: the Pass-31 remediation burst (D-1088) had correctly bumped `version:` to 1.32, added the `## Changelog` v1.32 row, and updated the `last_amended:` prefix to `(v1.32)` — three of the four in-file parity legs agreed — but never prepended the corresponding `modified:`-array entry, leaving the array's head at v1.31. This is the THIRD occurrence of this exact shape on this gate: F-P29-003 (pass-29, D-1086) found the SAME omission on BC-7.07.001 itself (a re-regression of an even earlier ordering fix, O-P27-001); F-P30-001 (pass-30, D-1087) found the same class on BOTH BC-4.17.001 and BC-5.40.001. Each prior occurrence was fixed as a spot-fix on the specific artifact the adversary flagged — the D-1087 comprehensive-sweep technique (codified two lessons above) was applied to `inputs:`-completeness and cross-anchor-citation classes, but was never retroactively applied to this specific `modified:`-array-head-parity class as a standing PRE-BURST discipline, only as a POST-HOC fix each time the adversary caught it fresh. A pattern is a codification signal at 2 occurrences and a MANDATORY-fix signal at 3+; this is the 3rd.

**Disposition:** CODIFIED this burst as a MANDATORY pre-declare-done check, not merely a further spot-fix: every BC/artifact version bump MUST run a 4-leg head==version self-check — `version:` == `modified:`-array-head == `## Changelog`-table-head == `last_amended:`-prefix, with NO gap in the `modified:` array (i.e., the array must contain a contiguous, strictly-descending run from the new head down to v1.1, not merely have SOME entry matching the new version anywhere in the array) — BEFORE the burst that performs the bump is declared done. This is a `[process-gap]`, distinct from the `[content-defect]`-tagged sibling-sweep lessons above (D-1084, D-1085, D-1086): those were defects IN the spec artifacts' substantive claims, discoverable only by domain review; this is a MECHANICAL, purely-structural self-consistency property of a single frontmatter block, checkable by grep/regex with no domain knowledge required, and its 3+ recurrence under manual discipline is itself evidence that manual discipline is not the right layer to enforce it at. Per the S-7.02 cycle-closing checklist, a follow-up anchor is recorded (not fixed this burst — this is `.factory/` artifact-authoring/process discipline, not factory-artifacts scope) for a MECHANICAL `validate-modified-head-parity` validator hook: a PreToolUse or PostToolUse WASM guard on Edit/Write to any BC/ADR file that extracts `version:`, the `modified:`-array's first element, the `## Changelog` table's first row, and the `last_amended:` prefix, and fail-closed-blocks (or fail-open-advises, pending human policy call on enforcement strength) when they disagree. The existing `validate-changelog-monotonicity` hook is the natural sibling to extend or the precedent to model the new hook on — it already checks Changelog-table date/version monotonicity but does NOT check `modified:`-array-head==`version:` parity, which is the specific gap this 3rd recurrence exposes. Anchored to the same S-15.03 PRIORITY-A automation tranche already carrying this gate's other mechanical-consistency-checker follow-ups (e.g., the `[D-1082]` cyclic-hash structural fix, the BC-INDEX/ARCH-INDEX `last_amended` unbounded-growth compaction). `[codified; process-gap; modified-head-parity; version-bump-self-check; 4-leg-parity; mechanical-validator-anchor; validate-modified-head-parity; F-P29-003; F-P30-001; F-P32-001; D-1089; adr-046-gate; recurrence-class; S-15.03-priority-a-anchor]`

---

**[codified][process-gap] An "inputs-completeness audit" performed as a human read-through is NOT the same discipline as a GREP-COMPLETE mechanical audit — three consecutive prose-narrative audits (passes 28, 30, 31) each believed themselves complete and each still shed exactly one straggler on the NEXT pass; only switching to mechanical file-path-token enumeration finally drained the class.**

At D-1090 (ADR-046 pass-33), finding F-P33-001 (MED) found ADR-046's own `inputs:` array still missing `crates/hook-sdk/src/result.rs` — despite THREE prior passes (28, 30, 31) each having run what its own authoring agent described as a "MANDATORY complete inputs-completeness audit" or "MANDATORY COMPLETE inputs-completeness audit (not a spot sweep)" on this exact ADR and its companion BCs. Each of those three audits found and fixed real omissions (pass-28: 1 file; pass-30: 6 files; pass-31: 2 files on one BC plus 5 on another), and each believed its own sweep to be exhaustive — yet each was followed by a subsequent pass finding at least one MORE straggler of the identical parity-gap character. This burst's remediation changed METHOD, not just effort: instead of a human reading the document body section-by-section and noting every file mention encountered (a read-through — thorough, but bounded by what the reader's attention actually catches on a single pass), the architect ran a MECHANICAL `grep -noE` sweep across every file-path-shaped token class in the document (`crates/[...]\.rs`, `plugins/[...]\.(sh|toml)`, `.factory/[...]\.(md|yaml)`, bare `[...]\.(toml|md|yaml|bats)` basenames, backtick-quoted path literals, `(BC|ADR|VP|DI)-[...]` identifiers), producing an exhaustive candidate list independent of reading-attention, then checked EACH hit against `inputs:` and against whether it is a genuine load-bearing current-state claim (vs. a forward-only sync instruction or pure locator, explicitly rejected with a stated reason). This single grep-complete pass caught the flagged item (`result.rs`) PLUS one further genuine omission (`BC-4.17.001.md`, cited ~20 times) PLUS a latent bracket-balance defect in the same field's `last_amended` history — three distinct catches in one sweep, where the prior three read-through audits combined caught only their own respective single-digit counts and still left stragglers behind each time.

**Disposition:** CODIFIED this burst as a MANDATORY discipline, not merely a one-off technique choice: a claimed "inputs-completeness audit" is only valid if it is GREP-COMPLETE — i.e., performed via mechanical file-path-token enumeration across the pattern classes above, with the resulting per-path disposition (found in `inputs:` already / added / explicitly rejected with a stated reason) recorded in an auditable table, as done in ADR-046's own v1.16 Changelog entry — NOT a human read-through, however careful or however many times repeated. A read-through's completeness is bounded by what the reader's attention happens to catch during one linear pass through prose; a grep sweep's completeness is bounded only by whether the pattern-class set itself is exhaustive (itself checkable, and expandable if a future pass finds a token shape the existing classes miss). This generalizes beyond ADR-046: any spec artifact's `inputs:`-completeness claim, for ANY BC/ADR/VP, should default to the grep-complete method from the first authoring pass, not escalate to it only after 3+ read-through misses. This is the THIRD distinct convergence-technique discipline this gate's history has produced, alongside the version-stable-directive fix (O-P28-002, D-1085 — restructure a self-referential literal into a live-field reference) and the 4-leg head==version parity check (D-1089 — a mechanical frontmatter self-consistency gate). Like the D-1089 lesson, this is a `[process-gap]`, not a `[content-defect]`: the underlying omissions were genuine content gaps, but the RECURRENCE across three nominally-complete audits is a defect in the AUDIT METHOD itself, not in the spec content each audit was checking. No new mechanical validator is anchored by this lesson (unlike D-1089's `validate-modified-head-parity` follow-up) — the fix here is a PROCESS discipline (grep-complete audit method), applicable by any agent performing this class of audit, not a candidate for automated enforcement via a WASM guard, since "is this file a load-bearing current-state citation vs. a forward-only sync instruction" requires judgment a mechanical hook cannot exercise; the grep step (candidate enumeration) is mechanical, but the disposition step (load-bearing or not) remains a human/agent judgment call, faithfully recorded in the audit table. `[codified; process-gap; inputs-completeness; grep-complete-audit; read-through-insufficient; mechanical-enumeration; F-P33-001; D-1090; adr-046-gate; recurrence-class; third-convergence-technique; audit-method-not-content]`

---

**[convergence-confirmation][codified] The three convergence-technique disciplines codified across this gate's history (version-stable directive O-P28-002/D-1085, 4-leg `modified:`-array head==version parity D-1089, GREP-COMPLETE mechanical inputs-completeness audit D-1090), applied together and proactively from the start of a pass, are now empirically confirmed — not merely hypothesized — to be sufficient to drain the asymptotic metadata floor: pass-34 is the FIRST literal zero-finding CLEAN result this gate has produced across its 34-pass history.**

At D-1091 (ADR-046 pass-34), fresh-context adversary review against the unchanged frozen set (ADR-046 v1.16 + BC-4.17.001 v1.15 + BC-5.40.001 v1.14 + BC-7.07.001 v1.33) returned **VERDICT: CLEAN — zero findings at any severity**, the first such result in this gate's history. This is significant because the metadata/hygiene layer — as distinct from the behavioral core, which had already been stable since pass-27 — had produced a genuine finding on EVERY SINGLE prior pass from 27 through 33 (7 consecutive passes: F-P27-001/002/003, F-P28-001/002, F-P29-001/002/003, F-P30-001/002, F-P31-001/002, F-P32-001, F-P33-001), each traced to one of three recurring shapes: (1) a sibling-sweep gap (a fix applied to one artifact never propagated to siblings sharing the identical claim), (2) a `modified:`-array-head omission on version bump, or (3) an `inputs:`-completeness gap surviving a nominally "complete" human read-through. Each of these three shapes was, in turn, closed by a specific codified discipline: sibling-sweep gaps by the D-1087/D-1088 comprehensive-per-dimension-sweep technique; `modified:`-array-head omissions by the D-1089 mandatory 4-leg self-check; `inputs:`-completeness gaps by the D-1090 GREP-COMPLETE mechanical audit method. Pass-34 is the first pass to apply all three disciplines PROACTIVELY — as the DEFAULT review method from the start, not as a reactive technique-change triggered by a fresh finding mid-pass — and it is the first pass to return CLEAN.

**Disposition:** This is recorded as `[convergence-confirmation]`, distinct from the `[codified]`-only entries above: those entries codified a NEW discipline in response to a fresh finding; this entry confirms an ALREADY-codified set of disciplines, taken together, actually closes the class they were each designed to close, rather than merely reducing its recurrence rate. Per BC-5.39.001, one CLEAN pass is not itself convergence — the streak requires 3 CONSECUTIVE clean passes (34, 35, 36), and any finding on pass 35 or 36 would reset the streak to 0/3 and reopen the question of whether a fourth, still-undiscovered defect class exists in this metadata layer. The correct disposition for passes 35 and 36 is therefore to CONTINUE applying all three disciplines proactively (not to relax review rigor on the theory that the layer is "probably clean now") — the value of this confirmation is that it validates the TECHNIQUE, not that it renders further verification unnecessary. If passes 35 and 36 also return CLEAN, this generalizes into a standing recommendation: any future ADR/BC spec-convergence gate on this or other subsystems should apply comprehensive-sibling-sweep, 4-leg-parity, and grep-complete-inputs-audit as its DEFAULT review posture from pass 1, rather than discovering the need for each incrementally through a multi-pass recurrence-then-codify cycle as this gate did. `[convergence-confirmation; codified; empirical-confirmation; three-disciplines; comprehensive-sweep; 4-leg-parity; grep-complete-audit; first-clean-pass; D-1091; adr-046-gate; pending-pass-35-36-confirmation; not-yet-converged]`

---

**[codified][process-gap] Comprehensive cross-anchor audits on this gate had only ever validated `BC→BC §Section` citations, never `ADR §Decision`/`§N.M` citations against the cited ADR's own section content — these are two structurally distinct citation classes, and a clean result on one does NOT imply the other is clean.**

At D-1092 (ADR-046 pass-35), fresh-context adversary review against the pass-34 CLEAN frozen set found F-P35-001 (HIGH): 3 loci across 2 companion BCs (BC-4.17.001 §Precondition 4, BC-5.40.001 §Precondition 6, BC-5.40.001 §Architecture Anchors) cited `ADR-025 §Decision 12 §12.5` for the 256 KiB `STATE_MD_MAX_BYTES` cap — verified against ADR-025's own text: §Decision 12 §12.5 is "Shared parse logic — no duplication," stating no byte-cap value at all; the decision that actually raised the cap is §Decision 14. Every prior comprehensive cross-anchor audit this gate has run — the D-1088 cross-anchor semantic audit (BC-5.40.001 PC1/PC2's "BC-6.23.001 PC3/PC4" mis-cite, corrected to PC4-only) and the D-1090/D-1091 grep-complete `inputs:`-completeness audits — checked BC-to-BC `§Section`/`PCn`/`Invariant-N` references and `inputs:`-array completeness exhaustively, and correctly found those clean. None of them independently re-derived, from the cited ADR's own section content, whether an `ADR-NNN §Decision N`/`§N.M` citation names the CORRECT decision number — they verified the BC's paraphrase of the cited decision's CONTENT was accurate (and it was, for the 262144 figure itself: the VALUE was right at all 3 loci throughout, only the decision NUMBER attributing it was wrong), which is a different check than opening ADR-025 and confirming `§Decision 14`, not `§Decision 12 §12.5`, is the decision that says so.

**Disposition:** CODIFIED this burst as a MANDATORY discipline: comprehensive cross-anchor audits MUST validate BOTH citation classes — (1) `BC→BC §Section`/`PCn`/`Invariant-N` cross-references (the class every prior audit on this gate already covers) AND (2) `ADR §Decision N`/`§N.M` citations against the cited ADR's own section content (the class this pass reveals as previously unaudited) — as two DISTINCT checks, not one check assumed to cover the other. A `BC→BC` citation audit confirms the CITING BC's characterization of another BC's section is accurate; an `ADR §Decision` anchor audit confirms the citing artifact named the correct decision NUMBER within the cited ADR — these can diverge independently, as this pass demonstrates (the 262144 figure's characterization was accurate at all 3 loci; only the source decision-number attribution was wrong). Architect additionally ran this new dimension against ADR-046 itself (its sole cross-ADR anchor, ADR-025 §Decision 12 §12.2, confirmed correct) and product-owner ran it against BC-7.07.001 (clean) — both confirmed no further mis-anchors exist across the frozen set on this dimension. This is the FOURTH distinct convergence-technique discipline this gate's history has produced, alongside the version-stable-directive fix (O-P28-002, D-1085), the 4-leg head==version parity check (D-1089), and the GREP-COMPLETE mechanical inputs-completeness audit (D-1090) — like D-1090, this is a `[process-gap]` in the AUDIT METHOD, not a `[content-defect]` in what any single audit checked; the underlying citation gap was a genuine content defect, but the fact that 8+ prior comprehensive audits never covered this citation CLASS at all is the process gap. No mechanical validator hook is anchored by this lesson (unlike D-1089's `validate-modified-head-parity` follow-up) — confirming "does `§Decision N` in the citing artifact match the actual decision-number that states this fact in the cited ADR" requires opening and semantically reading the cited ADR's section, which is a judgment call no WASM guard can exercise without full-document natural-language comprehension. `[codified; process-gap; adr-decision-anchor-audit; semantic-anchoring-integrity; two-distinct-citation-classes; F-P35-001; D-1092; adr-046-gate; fourth-convergence-technique; audit-dimension-not-content]`

---

**[process-observation] The gate reached its first literal-CLEAN result at pass-34 (streak 1/3) and RESET at pass-35 on a newly-revealed audit dimension — this is empirical confirmation, not merely a hypothesis, that the asymptotic-floor reality described at D-1091 holds even immediately following a zero-finding CLEAN pass, and is decision-relevant for the human's continue-vs-accept-provisional choice.**

D-1091 recorded pass-34 as the first literal zero-finding CLEAN result this gate has produced across 34 passes, and explicitly flagged the confirmation as "provisional pending passes 35/36 also returning CLEAN" — not a closure. Pass-35 (D-1092) is exactly the scenario that provisional framing anticipated: a finding on the very next pass, on a dimension no prior pass (including pass-34's own thorough re-verification of "every dimension this gate's history has ever found a defect in") had ever audited. The substance stayed clean throughout — both F-P35-001 and F-P35-002 are the same cap-migration-lineage citation-cluster defect, not a behavioral or write-composition regression, and pass-34's zero-finding result on every dimension it DID check remains valid and unregressed. What changed is the SCOPE of what gets checked, not the quality of what was previously verified.

**Disposition:** This is recorded as `[process-observation]`, not a fix and not itself a codification (the codification is the preceding `[codified][process-gap]` entry) — it is a meta-level observation about the SHAPE of convergence on this gate, offered as decision-relevant context for the human's standing choice between continuing to loop toward literal 3-CLEAN (BC-5.39.001's strict discipline) versus accepting D-386 Option C asymptotic acceptance for this gate. The empirical pattern now on record: 34 passes to reach the first zero-finding result, immediately followed by a reset on a dimension that a full 34-pass history — including three previously-codified comprehensive-audit disciplines — had never covered. This does not prove convergence is unreachable; it demonstrates that each codified discipline closes the SPECIFIC class it targets, and that "zero findings against every dimension a discipline currently checks" is not equivalent to "zero findings against every dimension that could exist" — new dimensions can and do surface even after codified disciplines are applied proactively and a clean result is achieved. The correct disposition remains to CONTINUE looping under manual discipline (per the standing human decision recorded at D-1087 and reaffirmed through D-1091), now applying FOUR convergence-technique disciplines proactively (comprehensive-sibling-sweep, 4-leg-parity, grep-complete-inputs-audit, ADR-§Decision-anchor-audit) rather than three — but the human should weigh this pattern explicitly rather than assume the next clean-streak attempt is categorically more likely to reach literal 3-CLEAN than the last one was. `[process-observation; asymptotic-floor; empirical-confirmation; streak-reset; clean-then-reset; D-1091; D-1092; adr-046-gate; human-decision-relevant; not-a-fix; not-itself-a-codification]`

---

**[convergence-progress][codified] Pass-36 re-applied all FOUR now-codified convergence-technique disciplines proactively — including the ADR-anchor audit that reset the streak just one pass prior — and found the fourth dimension fully drained across the whole frozen set, not merely the 3 loci pass-35 explicitly fixed; streak ADVANCES 0/3 → 1/3, the SECOND clean pass this gate has produced.**

At D-1093 (ADR-046 pass-36), fresh-context adversary review against the newly-frozen set (ADR-046 v1.16 + BC-4.17.001 v1.16 + BC-5.40.001 v1.15 + BC-7.07.001 v1.33) — the set produced by the pass-35 fix burst — returned **VERDICT: CLEAN — zero findings at any severity**. This directly follows the pass-35 reset that revealed the ADR §Decision/§N.M anchor dimension as a previously-unaudited citation class (D-1092). Pass-36 independently re-derived, from each cited ADR's own section content, every `ADR-NNN §Decision N`/`§N.M` citation across the frozen set — not just the 3 loci pass-35 corrected — including ADR-046's own sole cross-ADR anchor and BC-5.40.001's separate `§Decision 7` citation, and found zero mis-anchors anywhere. All three previously-confirmed disciplines (version-stable directive, 4-leg parity, grep-complete inputs audit) were also re-verified holding, with no regression.

**Disposition:** This is recorded as `[convergence-progress]`, distinct from D-1091's `[convergence-confirmation]` (which confirmed three disciplines together sufficient against the dimensions then known) and from D-1092's `[codified][process-gap]` (which introduced the fourth discipline in response to a fresh finding): this entry is the first direct EVIDENCE that the fourth discipline, applied proactively, closes the class it targets the way the first three closed theirs — evidence, not yet proof, since a single clean pass is not itself a confirmation the way three consecutive passes would be. Per BC-5.39.001, this is 1 of 3 required clean passes counting from the pass-35 reset — the streak requires 2 further CONSECUTIVE clean passes (37, 38), and any finding on either would reset the streak to 0/3 again and reopen the question of whether a fifth, still-undiscovered defect class exists. The correct disposition for passes 37 and 38 is to CONTINUE applying all four disciplines proactively without relaxing rigor on the theory that "the anchor dimension is now proven safe" — one clean pass on a freshly-codified discipline is progress, not closure. `[convergence-progress; codified; four-disciplines; adr-decision-anchor-audit; second-clean-pass; D-1093; adr-046-gate; pending-pass-37-38-confirmation; not-yet-converged]`

---

**[codified][process-gap] Fix-burst disposition prose that makes a sweeping self-attested completeness claim ("read in full, all correct") is itself falsifiable attack surface for a fresh-context adversary — pass-37 found the pass-35 remediation's OWN audit-narrative miscounted ADR-046's Decision list (asserted 1–5, actual 1–6); MITIGATION now in force: disposition prose must be MINIMAL and factual, and self-attested audits need mechanical (greppable) backing, not a bare completeness assertion.**

At D-1094 (ADR-046 pass-37), fresh-context adversary review against the SAME unchanged frozen set (ADR-046 v1.16 + BC-4.17.001 v1.16 + BC-5.40.001 v1.15 + BC-7.07.001 v1.33) found F-P37-001 (MED): both BC-4.17.001's and BC-5.40.001's own `modified:`/`last_amended`/Changelog prose from the pass-35 remediation (D-1092) asserted ADR-046's `## Decision` section is "a flat list, 1–5, ... read in full, all correct" — but ADR-046 actually has 6 numbered decisions; item 6 (same-release ship + CI-gating registry-invariant XOR check) was silently dropped from the count. Every ACTUAL `ADR-046 Decision N` citation in both BCs' live body text remained correctly numbered and unaffected — this defect lives entirely inside the remediation's own summary/bookkeeping narrative, not in any operative spec content. O-P37-001 ([process-gap], LOW) additionally observes that nothing mechanically verifies a "read in full" claim against the cited artifact's actual section-list cardinality before the claim is written down — the gap that let this miscount stand for two full passes (36's CLEAN re-verification checked citation NUMBERS against ADR-046's text but did not independently re-derive the cited artifact's own item COUNT from its narrative summary).

**Disposition:** CODIFIED as a mandatory discipline for all future ADR-046 gate remediation bursts (and, by extension, any fix-burst disposition prose on this project that asserts audit completeness): (1) disposition prose describing a "read in full" or "comprehensive audit" claim MUST be MINIMAL and factual — state what was checked and what was found, without a sweeping certification of exhaustiveness beyond what was actually mechanically verified; (2) any prose that states a cardinality (a count of items, decisions, sections, citations) as part of an audit claim MUST be independently greppable/countable at write-time (e.g., `grep -cE '^### Decision [0-9]+' ADR-046.md` before asserting "1–5" or "1–6") rather than trusted from memory or from a prior pass's own narrative. This is distinct from D-1090's `[codified][process-gap]` (GREP-COMPLETE inputs-completeness audits) and D-1092's (ADR §Decision anchor citation-NUMBER correctness) — this lesson targets the CARDINALITY/COUNT claim inside disposition prose itself, a third structurally-distinct failure mode alongside wrong-citation-target and incomplete-inputs-array. `[codified; process-gap; self-attested-completeness-claim; falsifiable-narrative; mechanical-backing-required; F-P37-001; O-P37-001; D-1094; adr-046-gate; disposition-prose-discipline]`

---

**[process-observation] ASYMPTOTIC-FLOOR meta-observation STRENGTHENED: the gate has now reached 1/3 twice (pass-34, pass-36) and RESET twice (pass-35 on the ADR-anchor dimension, pass-37 on that same dimension's OWN remediation-prose bookkeeping) — the second reset came from the remediation's own bookkeeping rather than a fresh spec-vs-code defect, empirically confirming that prose-only codification leaves literal 3-CLEAN structurally fragile even when the underlying discipline is sound.**

D-1091 (pass-34) and D-1093 (pass-36) each reached the gate's zero-finding CLEAN result, both explicitly flagged provisional. D-1092 (pass-35) reset the streak on a genuinely NEW audit dimension (ADR §Decision/§N.M anchor correctness) no prior pass had covered. D-1094 (pass-37) reset the streak AGAIN — but this time not on a new dimension: the finding is a miscount INSIDE the pass-35 remediation's own narrative describing that same fourth dimension. This is a qualitatively different failure mode from pass-35's: pass-35 demonstrated "a codified discipline can miss a class it doesn't yet check"; pass-37 demonstrates "even after a discipline is codified and correctly applied to the underlying substance, the REMEDIATION'S OWN bookkeeping narrative describing that application is itself a fresh source of falsifiable claims a later adversary can catch." Cf. the F5-cycle's own META-LEVEL taxonomy (L-EDP1-007/051/061): "prose-only codification → literal 3-CLEAN structurally fragile" is not a hypothesis specific to the F5 cycle's dimension-count — it recurs here, on an entirely different gate (ADR-046 vs. F5's engine-discipline cycle), via an entirely different concrete mechanism (decision-count miscitation vs. F5's meta-level-ply taxonomy gaps). This is now empirically observed on TWO separate gates in this project, strengthening the claim from gate-specific anecdote toward a general property of the BC-5.39.001 3-CLEAN discipline under prose-only (non-mechanically-enforced) codification.

**Disposition:** Recorded as `[process-observation]`, not a fix — decision-relevant context for the human's standing continue-vs-accept-provisional choice, re-surfaced at this decision point. **The human RE-AFFIRMED "CONTINUE looping toward literal 3-CLEAN" at this decision point** — accept-provisional under D-386 Option C was offered and declined again, the second such explicit reaffirmation this session (the first at the pass-35 reset). The correct disposition going forward is unchanged in kind but now reinforced by the D-1094/lessons `[codified][process-gap]` mitigation above (minimal, factual, mechanically-backed disposition prose) — applying it should reduce (though not provably eliminate) the rate at which a remediation's OWN narrative becomes the next pass's finding. `[process-observation; asymptotic-floor; strengthened; second-gate-confirmation; streak-reset; clean-then-reset-twice; D-1091; D-1092; D-1093; D-1094; adr-046-gate; f5-cycle-parallel; human-reaffirmed-continue; human-decision-relevant; not-a-fix]`

---

**[convergence-progress][codified] Pass-38 re-applied every now-codified convergence-technique discipline proactively — including BOTH dimensions whose discovery caused this session's two resets (ADR-anchor correctness at pass-35, self-attested cardinality claims at pass-37) — and found the entire frozen set clean on every dimension; streak ADVANCES 0/3 → 1/3, the THIRD clean pass this gate has produced this session.**

At D-1095 (ADR-046 pass-38), fresh-context adversary review against the SAME unchanged frozen set (ADR-046 v1.16 + BC-4.17.001 v1.17 + BC-5.40.001 v1.16 + BC-7.07.001 v1.33) — the set produced by the pass-37 fix burst — returned **VERDICT: CLEAN — zero findings at any severity**. This is the first pass to directly re-verify BOTH previously-reset dimensions in the same review: the ADR §Decision/§N.M anchor-correctness class (D-1092/pass-35's dimension) and the self-attested cardinality/completeness-claim class (D-1094/pass-37's dimension, including an independent recount of ADR-046's own `## Decision` section confirming 6 items against both BCs' now-corrected "1–6" prose). Every other previously-codified dimension (version-stable directive, 4-leg parity, grep-complete inputs audit) was also re-verified holding, with no regression.

**Disposition:** Recorded as `[convergence-progress]`, the same tag used for D-1093's pass-36 confirmation, but distinguished from it by scope: D-1093 confirmed the ADR-anchor dimension alone (immediately after its own reset); this entry confirms BOTH reset dimensions simultaneously, on the same pass, against the same frozen set that triggered the second reset. This is the first direct EVIDENCE that the D-1094 mitigation (minimal, factual, mechanically-backed disposition prose) holds under independent fresh-context re-derivation — evidence, not yet proof, since a single clean pass is not itself a confirmation the way three consecutive passes would be. Per BC-5.39.001, this is 1 of 3 required CONSECUTIVE clean passes — the streak requires 2 further consecutive clean passes (39, 40), and any finding on either would reset the streak to 0/3 again. The correct disposition for passes 39 and 40 is to CONTINUE applying all convergence-technique disciplines proactively without relaxing rigor on the theory that "both reset dimensions are now proven safe" — one clean pass covering two previously-reset dimensions is progress, not closure. `[convergence-progress; codified; both-reset-dimensions-reconfirmed; third-clean-pass; D-1095; adr-046-gate; pending-pass-39-40-confirmation; not-yet-converged]`

---

**[codified][process-gap][convergence-observation] Arm-parity sibling-sweep + substantive-vs-metadata reset distinction — pass-39's finding was a genuine data-destructive internal contradiction in BC-4.17.001's own operative Precondition/Invariant text, the FIRST substantive (not metadata/prose) reset this gate has produced, surfacing a sixth distinct convergence-technique discipline: what-vs-how reconciliations must sweep to every analogous sibling arm/case in the same burst.**

At D-1096 (ADR-046 pass-39), fresh-context adversary review against the frozen set produced by
pass-38 (ADR-046 v1.16 + BC-4.17.001 v1.17 + BC-5.40.001 v1.16 + BC-7.07.001 v1.33) found F-P39-001
(MED): BC-4.17.001's Precondition 4 and Invariant 7 mandated `extract_frontmatter`-slice confinement
for BOTH the `timestamp:` re-stamp arm AND the `expires_at` renewal arm, directly contradicting
Precondition 2's/Invariant 9's own requirement that `renew_lock_if_holder` be fed the FULL
`content_after_pc1` for the single composed `host::write_file`. A literal reading of the `expires_at`
arm's slice-exclusivity directive would have truncated `RenewOutcome::Renewed(new_content)` to the
frontmatter region on write — since that truncated value becomes the entire composed write, this
would have DESTROYED STATE.md's body content on a live write. This defect survived 37 prior passes
(including the Pass-16/O-P16-001 fix that reconciled the analogous what-vs-how tension for PC4's
OTHER case — the `timestamp:` arm) because no prior pass's audit checklist included "when a
what-vs-how reconciliation is applied to one arm/case, check whether every sibling arm/case with
analogous language received the same reconciliation" as a discrete check.

**Part (a) — `[codified][process-gap]`:** when a what-vs-how (semantic-scope vs. mechanism)
reconciliation is applied to ONE arm/case of a contract, ALL sibling arms/cases carrying analogous
language MUST receive the same reconciliation in the SAME burst. This is the arm-parity variant of
the sibling-sweep discipline already codified for callsites (TD-VSDD-060) and for BC clusters
(D-1087/D-1088's comprehensive-sweep convergence-strategy lessons) — but operating at a FINER
granularity: not "every BC that shares this defect class," but "every CLAUSE-ARM within the SAME BC
that uses analogous language to the clause just reconciled." The Pass-16 fix reconciled PC4's
`timestamp:`-arm what-vs-how tension (frontmatter-slice-directive vs. actual write-mechanism scope)
but never checked whether Invariant 7's PARALLEL `expires_at`-arm language carried the identical
tension — it did, and it went unreconciled for 23 further passes (16 through 38) until a
fresh-context adversary traced the full write-composition data flow from first principles rather
than re-running any previously-codified checklist item.

**Part (b) — `[process-observation][convergence-observation]`:** this is the gate's THIRD reset this
session (after pass-35 and pass-37), but the FIRST that is SUBSTANTIVE rather than metadata/prose.
Pass-35's reset was a citation-accuracy gap (wrong ADR §Decision NUMBER, correct underlying value).
Pass-37's reset was a bookkeeping miscount INSIDE a prior remediation's own narrative prose (no
operative-content risk at all — the actual `ADR-046 Decision N` citations in live body text were
never wrong). **Pass-39's finding is neither of those shapes** — it is a genuine unreconciled
contradiction in the BC's own live Precondition/Invariant text that, if shipped into S-17.05's TDD
implementation, would have produced a real data-truncation bug on the very first `expires_at`
renewal write. This is decision-relevant evidence for the human's standing continue-vs-accept-
provisional choice (§5 Pending Human Decision): the two prior resets could be read as the gate
"finding diminishing returns" on prose-level polish; this reset demonstrates the OPPOSITE — the gate
is still finding genuine, consequential defects a fresh adversary's first-principles trace can
surface that no accumulated checklist of previously-codified disciplines would have caught, because
each codified discipline targets the SPECIFIC failure shape its own discovery pass revealed, not the
general class "any two Preconditions/Invariants governing the same composed write may silently
diverge in scope."

**Disposition:** Recorded as `[codified][process-gap]` for part (a) — the arm-parity sibling-sweep
discipline is now in force for all future ADR-046 gate remediation bursts and any other BC exhibiting
a what-vs-how reconciliation pattern across multiple arms/cases — and `[process-observation]
[convergence-observation]` for part (b) — recorded for the human's decision-relevance, not itself a
fix. The human's standing choice (CONTINUE looping toward literal 3-CLEAN, re-affirmed twice already
this session) is not altered by this entry; this entry supplies additional evidence FOR that choice's
continued soundness, since a substantive defect just surfaced at pass-39, the same discipline that
found and fixed 30 prior genuine findings across the gate's history. `[codified; process-gap;
convergence-observation; arm-parity; sibling-sweep; what-vs-how; substantive-reset;
metadata-vs-substantive; D-1096; adr-046-gate; f-p39-001; third-reset; sixth-discipline]`

---

**[codified][process-gap] Pass-40's F-P40-001 empirically VALIDATED the D-1096 arm-parity
sibling-sweep codification — and simultaneously revealed the codification's own scope was
under-specified: the pass-39 fix swept Preconditions/Invariants but missed the sibling
VERIFICATION PROPERTY carrying the identical guarantee**

At D-1097 (ADR-046 pass-40), fresh-context adversary review against the frozen set produced by the
pass-39 fix (ADR-046 v1.16 + BC-4.17.001 v1.18 + BC-5.40.001 v1.16 + BC-7.07.001 v1.33) found
F-P40-001 (MED): BC-4.17.001's VP-TBD-8 — the §Verification Properties table row covering the
identical `extract_frontmatter`-use guarantee Precondition 4/Invariant 7 govern — still lumped the
`timestamp:` and `expires_at` arms under one joint frontmatter-slice confinement, re-encoding the
exact pre-F-P39-001 framing the Precondition/Invariant pair was corrected away from one pass
earlier. This is not a new class of defect independent of pass-39 — it is the SAME defect,
un-swept at a sibling locus, and it is the first direct empirical test of whether the D-1096
arm-parity sibling-sweep codification (recorded one pass ago) actually holds under fresh-context
re-derivation. The answer is: PARTIALLY. The codification's own text ("every analogous sibling
arm/case") was interpreted narrowly at v1.18 to mean the Precondition/Invariant pair alone — the
locus class the pass-39 finding itself was discovered in — and did not extend the sweep to other
locus CLASSES (§Verification Properties rows, Architecture Anchors, SDK-grounding blocks) that
carry the same guarantee in different prose form.

**Extension to the D-1096 codification (this entry's substantive content):** arm-scope/what-vs-how
reconciliations MUST sweep to ALL loci carrying the guarantee, not just the locus class where the
reconciliation was first discovered. Concretely, for this BC-cluster's recurring pattern, the sweep
checklist now explicitly enumerates: Preconditions, Postconditions, Invariants, §Verification
Properties rows, Architecture Anchors, and SDK-grounding blocks. The pass-40 comprehensive 8-locus
sweep (Precondition 4, Invariant 7, PC1's rewrite-mechanism paragraph, PC3a, PC4, Invariant 5, Edge
Cases, Canonical Test Vectors, Architecture Anchors, Description, and VP-TBD-8 itself) is the model
going forward: when in doubt about how far a sibling-sweep must reach, grep the artifact for every
mention of the load-bearing term/mechanism name (here, `extract_frontmatter`) and classify each hit
as ALREADY-CORRECTED, GENUINELY-UNAFFECTED, or STRAGGLER — do not stop at the locus class where the
original finding was discovered.

**Disposition:** Recorded as `[codified][process-gap]`. This is the SEVENTH distinct
convergence-technique discipline this gate has produced (after: (1) comprehensive-sibling-sweep
D-1087/D-1088; (2) 4-leg `modified:`-array head==version parity D-1089; (3) GREP-COMPLETE
`inputs:`-completeness audit D-1090; (4) ADR `§Decision`/`§N.M` anchor audit D-1092; (5)
minimal-prose + mechanical-audit-backing discipline D-1094; (6) arm-parity what-vs-how
reconciliation sweep D-1096; (7) this entry's locus-class extension of (6)). It is also
decision-relevant evidence of a DIFFERENT shape than D-1096's part (b): rather than showing the
gate finds NEW classes of defect, pass-40 shows the gate catches INCOMPLETE APPLICATION of its own
most-recently-codified discipline — meta-level evidence that codified disciplines need their own
follow-up verification pass, not just a one-time application, before being trusted as fully closed.
`[codified; process-gap; arm-parity; sibling-sweep; verification-properties; locus-class-extension;
what-vs-how; D-1097; adr-046-gate; f-p40-001; seventh-discipline; meta-verification]`

---

**[convergence-progress][codified] Pass-41 re-applied every now-codified convergence-technique
discipline proactively — including BOTH the sixth (arm-parity sweep, D-1096) and seventh
(locus-class extension, D-1097) disciplines together, against the exact frozen set those two fixes
themselves produced — and found the entire frozen set clean on every dimension; streak ADVANCES
0/3 → 1/3, the FOURTH clean pass this gate has produced this session**

At D-1098 (ADR-046 pass-41), fresh-context adversary review against the newly-frozen set (ADR-046
v1.16 + BC-4.17.001 v1.19 + BC-5.40.001 v1.16 + BC-7.07.001 v1.33) — the set produced by the pass-40
fix burst — returned **VERDICT: CLEAN — zero findings at any severity**. This is the first pass to
directly re-verify BOTH the arm-parity what-vs-how reconciliation dimension (D-1096/pass-39's
dimension) and its locus-class extension (D-1097/pass-40's dimension) in the same review, including
an independent re-derivation of all eleven `extract_frontmatter`-guarantee loci across BC-4.17.001
(Precondition 4, Invariant 7, VP-TBD-8, PC1's rewrite-mechanism paragraph, PC3a, PC4, Invariant 5,
Edge Cases, Canonical Test Vectors, Architecture Anchors, Description) — all confirmed consistently
arm-split, with VP-TBD-8 specifically re-opened and confirmed correct. Every other previously-codified
dimension (ADR §Decision anchor correctness, self-attested cardinality claims, version-stable
directive, 4-leg parity, grep-complete inputs audit) was also re-verified holding, with no
regression.

**Disposition:** Recorded as `[convergence-progress]`, the same tag used for D-1095's pass-38
confirmation, but distinguished from it by scope: D-1095 confirmed the pass-35/pass-37 dimensions
(citation-accuracy and bookkeeping-cardinality) together; this entry confirms the pass-39/pass-40
dimensions (arm-parity and its locus-class extension) together, the two MOST RECENTLY codified
disciplines, on the same pass, against the frozen set those two fixes themselves produced. This is
the first direct EVIDENCE that the D-1096/D-1097 pair — arguably the most consequential disciplines
this gate has produced, since D-1096 closed a genuine data-destructive hazard — holds under
independent fresh-context re-derivation. Evidence, not yet proof, since a single clean pass is not
itself a confirmation the way three consecutive passes would be. Per BC-5.39.001, this is 1 of 3
required CONSECUTIVE clean passes — the streak requires 2 further consecutive clean passes (42, 43),
and any finding on either would reset (or, if already zero, hold) the streak. The correct
disposition for passes 42 and 43 is to CONTINUE applying all seven convergence-technique disciplines
proactively without relaxing rigor on the theory that "the two most recent dimensions are now proven
safe" — one clean pass covering two previously-found dimensions is progress, not closure.
`[convergence-progress; codified; sixth-and-seventh-dimensions-reconfirmed; fourth-clean-pass;
D-1098; adr-046-gate; pending-pass-42-43-confirmation; not-yet-converged]`

---

**[convergence-governance] At 2/3, a pre-existing dated-historical cosmetic observation is accepted
as documentary-historical-deferred rather than fixed, because touching the frozen set to fix a
non-defect out of the feature perimeter would reset a live convergence streak — the correct
governance call is accept-and-track, not fix-and-reset**

At D-1099 (ADR-046 pass-42), fresh-context adversary review against the SAME unchanged frozen set
(ADR-046 v1.16 + BC-4.17.001 v1.19 + BC-5.40.001 v1.16 + BC-7.07.001 v1.33) returned **VERDICT:
CLEAN — zero BLOCKING findings**, the SECOND CONSECUTIVE clean pass (following pass-41), advancing
the BC-5.39.001 streak 1/3 → 2/3. The pass also surfaced ONE non-blocking observation, O-P42-001: a
pre-existing cosmetic asymmetry in BC-5.40.001's oldest `modified:` frontmatter array entries
(v1.1–v1.4 bare version/date strings vs. v1.5–v1.16 full disposition prose) — a defect that predates
ADR-046 entirely, breaks no head-parity check, and carries no operative risk.

The disposition question this pass raised: should O-P42-001 be fixed in-scope (per the project's
general production-grade default), or accepted as a tracked non-blocking item? The answer is
accept-and-track, for a reason specific to this gate's mechanics: fixing O-P42-001 requires editing
BC-5.40.001 — one of the four artifacts in the FROZEN set this streak's byte-unchanged invariant
depends on. Editing ANY of the four frozen artifacts, for ANY reason, resets the convergence context
this streak is measuring (the next pass would then be reviewing a DIFFERENT frozen set, and the
2/3 progress toward literal 3-CLEAN would no longer be "3 consecutive clean passes against the same
set" — it would be a new streak against a new set). The observation itself is out of the feature
perimeter (a pre-existing dated-historical row, not something ADR-046 introduced or is responsible
for) and non-blocking (no head-parity violation, no propagation gap, no operative risk). Under these
specific conditions — pre-existing, out-of-perimeter, non-blocking, and fixing it would cost a live
convergence streak for no correctness gain — the correct call is to accept it as a tracked item
(recorded in STATE.md Drift Items, anchored to a concrete future touch-point: the next maintenance
sweep or the S-15.03 PRIORITY-A historical-row backfill automation) rather than fix it now.

**Disposition:** Recorded as `[convergence-governance]`, a NEW tag distinguishing this class of
decision (a disposition call about WHETHER to fix something found mid-streak) from
`[convergence-progress]` (evidence a discipline holds) and `[codified][process-gap]` (a new
convergence-technique discipline). This is not a general license to defer fixable defects found
during this gate's passes — the default remains fix-in-scope. The narrow condition that justifies
deferral here is the CONJUNCTION of all four factors above (pre-existing, out-of-perimeter,
non-blocking, and touching the frozen set would cost the streak) — any BLOCKING finding, or any
observation that IS caused by or contemporaneous with ADR-046's own content, must still be fixed
same-burst per this gate's established practice (F-P37-001 through F-P40-001 were all fixed
same-burst, none deferred). The correct disposition for pass-43 is to continue applying all seven
convergence-technique disciplines proactively, treating O-P42-001 as an already-accepted item (not
re-litigating it as a fresh finding), and to fix in-scope any NEW blocking finding pass-43 might
surface (which would reset the streak, as any genuine finding always has).
`[convergence-governance; accept-and-track; documentary-historical-deferred; O-P42-001; streak-2of3;
D-1099; adr-046-gate; frozen-set-preservation; out-of-perimeter]`

---

**[codified][process-gap] Grep-complete mechanical inputs audits (D-1090) must be applied to
EVERY artifact in a cluster, not just the anchor/ADR document — the BCs' own piecemeal inputs
fixes left capabilities.md (+3 more on BC-5.40.001) missing until pass-43, the FIRST pass to
scope the audit to the BCs themselves**

D-1090 (pass-33) codified that inputs-completeness audits must be GREP-COMPLETE mechanical
file-path-token enumeration, not human read-throughs — and every subsequent inputs-completeness
finding through pass-40 (F-P33-001, F-P30-002, F-P29-002, F-P31-001, F-P35-002, F-P40-001, etc.)
was found by running that discipline against ADR-046's own `inputs:` array, or against a single
BC in response to a specific triggering claim. What no prior pass had done was run the
grep-complete audit against EACH of the three companion BCs' own `inputs:` arrays as a routine,
comprehensive sweep — the audits were always triggered reactively (an adversary spotting one
specific missing citation) rather than run proactively and exhaustively per-artifact the way
D-1090 itself was.

At pass-43, product-owner ran the FIRST such comprehensive per-BC audit (prompted by F-P43-001's
discovery that all three BCs were missing `capabilities.md` from `inputs:` despite quoting their
capability's description verbatim) and immediately found 3 MORE genuinely-missing files on
BC-5.40.001 alone — files that had been load-bearing citations since as early as pass-29
(`inputs:` was last touched on BC-5.40.001 at v1.14/pass-31, nine passes before this gap was
caught). This is a structural gap in how the D-1090 discipline had been APPLIED, not a weakness in
the discipline's own definition: D-1090's text never restricted itself to "the ADR only," but every
prior application of it happened to be ADR-scoped or single-BC-triggered, and no burst had ever
explicitly run it as "audit ALL FOUR frozen-set artifacts' own `inputs:` arrays, mechanically,
every time."

**Disposition:** Extend the D-1090 codification's PRACTICE (not its definition — the mechanical
grep-complete method itself is unchanged) to explicitly require: when this gate's grep-complete
inputs audit runs, it MUST run against every artifact in the frozen/reviewed set — the ADR AND
every companion BC — not just the artifact a specific triggering finding happens to implicate.
This closes a class of gap where a defect in one artifact (missing `capabilities.md`) is fixed
locally without checking whether sibling artifacts in the same cluster carry the identical gap.
`[codified][process-gap][inputs-audit-scope-extension][D-1100; adr-046-gate; F-P43-001;
grep-complete; all-cluster-artifacts]`

---

**[content-defect-discipline] A cross-reference to an acceptance criterion must name the OWNING
artifact (story vs. BC) and anchor to the BC's actual normative locus, not assume the AC belongs
to whichever artifact happens to be citing it**

At pass-43, F-P43-002 found that ADR-046's Companion Amendment 3 (and BC-7.07.001's own v1.19
Changelog row, independently) both referred to "AC-018" as though it were a normative
acceptance-criterion section of BC-7.07.001 itself — "The BC's existing AC-018 ('If `Renewed`:
call `host::write_file`') is otherwise unchanged." A full section-heading sweep of BC-7.07.001
confirms it has no Acceptance Criteria section and no AC-NNN numbering scheme at all: this BC's
sections are Description / Preconditions / Postconditions / Invariants / Edge Cases / Canonical
Test Vectors / Related BCs / Architecture Anchors / Story Anchor / VP Anchors / Verification
Properties / Traceability / Changelog. AC-018 is in fact a STORY-level acceptance criterion,
belonging to `.factory/stories/S-18.04a-precompact-flush-sh-core.md`'s own `### AC-018` section —
which itself traces down INTO BC-7.07.001 at a real, citable, section-anchored locus: Postcondition
3 case 5 / Invariant 3 step 4. The label "AC-018" was not fabricated (it genuinely exists, at the
story level, and genuinely traces to the cited BC locus) — it was mis-scoped, attributed to the
wrong owning artifact, by BOTH the ADR and the BC that echo it.

**Disposition:** When a cross-reference names an acceptance criterion (AC-NNN) or any other
artifact-scoped identifier, the citing text must (a) verify which artifact actually OWNS that
identifier's numbering scheme (a story's AC-NNN list is distinct from a BC's own PC/Invariant/EC
numbering — a BC citing "AC-NNN" as if it were its own section is a category error unless that BC
demonstrably has its own AC-NNN scheme), and (b) anchor the citation at the OWNING artifact's
actual locus while ALSO naming the concrete downstream section (PC/Invariant/EC) in the cited BC
that the story-level AC traces to — both halves are required for the cross-reference to resolve to
something real on both ends. This is a distinct class from the AC-018 mis-scoping bug's own
symptom (a plausible-sounding identifier a reader would not think to double-check) and from
ADR-025 §Decision anchor mis-cites (D-1092) — this class is specifically about IDENTIFIER-OWNERSHIP
attribution across the story/BC boundary, not section-number accuracy within one document.
`[content-defect-discipline; ac-owning-artifact; cross-reference-integrity; D-1100; adr-046-gate;
F-P43-002; story-vs-bc-boundary]`

---

**[process-observation][convergence-observation] META: the 4th reset (at the convergence pass
itself) was SUBSTANTIVE/provenance, not gaming or noise — the BC inputs perimeter had never been
grep-audited before this pass, so finding it now is convergence PROGRESS, not a regression**

Pass-43 was dispatched specifically to CLOSE the gate at literal 3-CLEAN (streak was 2/3 entering
this pass). Instead it found 2 MED findings and reset the streak to 0/3 — the 4th reset this
session. It would be tempting to read a reset at exactly the convergence pass as evidence the gate
is not actually converging, or that findings are being manufactured to avoid closure. Neither
reading holds up: F-P43-001 is the FIRST-EVER application of the mandatory grep-complete inputs
audit to the three companion BCs' own `inputs:` arrays (as opposed to ADR-046's) — a genuinely new
DIMENSION this gate's 42 prior passes never checked, not a re-discovery of an already-checked
dimension regressing. F-P43-002 is a genuine cross-reference defect (AC-018 mis-scoping) that
existed since BC-7.07.001's v1.19 (pass-2-era) and was never independently caught because both the
ADR and the BC echoing each other created a false consensus — exactly the kind of two-way
mutual-reinforcement defect fresh-context adversarial review with information asymmetry exists to
catch. Both findings sit at the provenance/cross-reference perimeter, not the behavioral core,
which remains independently re-verified clean for 17 consecutive passes (since pass-27). **The
correct reading: this reset represents the gate finding a previously-unaudited dimension and
draining it — the BC inputs perimeter is now drained (all three BCs grep-complete-audited for the
first time), which is convergence PROGRESS, structurally identical to the D-1096/D-1097 pattern
(a new dimension found, fixed, then re-confirmed clean on subsequent passes) rather than evidence
against the gate's validity.** Fresh pass-44 starts a new streak with one more dimension now
provably clean than pass-43 started with.
`[process-observation; convergence-observation; substantive-reset; not-gaming; D-1100; adr-046-gate;
fourth-reset; bc-inputs-perimeter-drained; convergence-progress]`

---

**[content-defect-discipline] An illustrative "verbatim quote" attached to a fix's own
disposition prose must cite the ACTUAL text of the cited source, not the fix's own paraphrase or
justification sentence — check sibling-parity across every disposition narrative touched in the
same burst**

At pass-44, O-P44-001 found that BC-5.40.001's v1.17 `last_amended` disposition (the entry
recording pass-43's F-P43-001 `capabilities.md` `inputs:` fix) illustrated the newly-added
citation with a parenthetical claiming to quote CAP-031's verbatim description — but the quoted
text, `'this BC defines the authoritative lock state data structure...'`, was in fact this BC's
own Capability Anchor Justification sentence (this BC's argument for why CAP-031 is its correct
anchor), not any text appearing in `capabilities.md`'s CAP-031 entry itself. The underlying
`inputs:` fix was correct throughout — `capabilities.md` genuinely is load-bearing and CAP-031
genuinely is this BC's correct capability anchor — the defect is narrowly that the illustrative
quote-snippet attached to the disposition prose cited the wrong source.

What makes this catchable in-burst rather than only in hindsight: BC-4.17.001 v1.20 and
BC-7.07.001 v1.34 received the IDENTICAL class of fix in the SAME pass-43 burst (each adding
`capabilities.md` to `inputs:` with an illustrative quote of their own respective CAP's
description/title), and BOTH siblings' equivalent illustrative quotes were verbatim-accurate
against their cited capability's actual text. A sibling-parity check — comparing all three BCs'
analogous disposition paragraphs for the same fix class, in the same burst — would have caught
BC-5.40.001's misattribution immediately, since the correct pattern was directly available two
files over.

**Disposition:** When any fix's own disposition prose illustrates a claim with a "verbatim
quote" of a cited source (a capability description, an ADR §Decision text, another BC's PC/
Invariant text, etc.), the quote MUST be checked against the actual cited source's text, not
assumed correct because the surrounding reasoning (which source to cite, why it applies) is
sound. When the same fix class is applied to multiple sibling artifacts in the same burst, their
analogous illustrative quotes must be cross-checked against EACH OTHER as well as against the
source — a correct sibling pattern sitting two files away, unconsulted, is itself evidence the
check was skipped. This is the ninth distinct convergence-technique discipline this gate has
produced, and a specific sub-class of citation-accuracy discipline distinct from ADR §Decision
anchor correctness (D-1092, which concerns *which section* is cited) — this concerns whether the
*quoted text itself* is genuinely present in the cited section.
`[content-defect-discipline; illustrative-quote-verbatim-accuracy; sibling-parity-check; D-1101;
adr-046-gate; O-P44-001; ninth-discipline]`

---

**[convergence-governance] Fix-vs-accept disposition for a LOW non-blocking observation: FIX
when the streak is at 0/3 (zero cost) AND the item is a fresh, in-session, sibling-confirmed-
correctable defect; reserve accept-and-track for genuinely pre-existing, out-of-perimeter,
dated-historical items**

At D-1099 (pass-42), O-P42-001 (a pre-existing, pre-ADR-046 `modified:`-array documentary
asymmetry in BC-5.40.001's oldest historical rows) was correctly ACCEPTED-and-tracked rather
than fixed: the streak was at 2/3, fixing would have required editing one of the four frozen-set
artifacts, breaking the byte-unchanged invariant the 2/3 streak depended on and forfeiting 2
already-banked clean passes for a cosmetic item that predated this gate's entire history and
carried zero operative risk. That was the correct call under BC-5.39.001's discipline.

At D-1101 (pass-44), O-P44-001 presented superficially the same shape — a LOW, non-blocking
observation on BC-5.40.001 found by a CLEAN-otherwise adversary pass — but the governance facts
were materially different: (a) the streak was ALREADY at 0/3 entering this pass (pass-43's own
FINDINGS verdict had already reset it), so editing the frozen set here costs ZERO additional
streak — there was no banked-clean-passes value to protect; (b) O-P44-001 is a FRESH defect
introduced in THIS SESSION'S OWN pass-43 remediation prose, not inherited pre-existing history
predating the gate; and (c) the two sibling BCs' equivalent disposition prose, edited in the
SAME pass-43 burst, got the analogous quote right — proving the correct pattern was known and
simply unapplied to this one BC, not a genuinely ambiguous or hard-to-verify claim. Under
CLAUDE.md's Canonical Principle Rule 4 (AI-built defects are the AI's responsibility to fix; the
default action is to fix in scope, not defer), and given the fix cost nothing in streak terms,
the correct call here was FIX, not accept-and-track.

**Disposition:** When this gate (or any BC-5.39.001-governed convergence gate) finds a LOW
non-blocking observation, the governance call is FIX (not accept-and-track) when ALL of: (i) the
streak is already at 0/3 (so the fix costs no banked clean passes), AND (ii) the item is a
fresh, in-session-introduced, correctable inaccuracy rather than pre-existing dated-historical
content, AND (iii) — where applicable — a correct sibling pattern in the same burst demonstrates
the fix is mechanical and low-risk, not a judgment call requiring a new design decision. Reserve
accept-and-track (the O-P42-001/D-1099 pattern) for observations that are genuinely
PRE-EXISTING, OUT-OF-PERIMETER (predating the current feature's own history), and whose fix
would cost banked streak progress for no operative gain. This is a governance-DECISION lesson,
not a mechanical validator — the state-manager applies this disposition rule at each future LOW
observation, consulting the streak position and the item's provenance (fresh vs. inherited)
before defaulting to either accept or fix.
`[convergence-governance; fix-vs-accept; zero-streak-cost; fresh-vs-inherited; D-1101;
adr-046-gate; O-P44-001; O-P42-001-contrast]`

---

**[convergence-progress] Pass-45 re-applied every now-codified convergence-technique
discipline proactively — including the ninth (illustrative-quote verbatim-source-accuracy +
sibling-parity-check, D-1101) whose discovery caused the immediately preceding burst's
governance-fix disposition — against the pass-44-corrected set, and found zero findings, zero
observations, of any severity; streak ADVANCES 0/3 → 1/3, the cleanest pass this gate has
produced across all 45 passes.**

At D-1101 (pass-44), the ninth discipline was freshly codified in response to O-P44-001 — a
misattributed illustrative quote confined to BC-5.40.001's own disposition prose, fixed
same-burst. Pass-45 is the first fresh-context re-derivation to run against the exact
post-fix bytes, re-checking not only BC-5.40.001's newly-corrected CAP-031 quote but both
sibling BCs' equivalent CAP-031/CAP-032 quotes as well — all three confirmed verbatim-correct
against `capabilities.md`. Every other dimension this gate has ever codified (arm-parity
sweep, locus-class extension, ADR-anchor correctness, minimal-prose-plus-mechanical-backing,
grep-complete cluster-wide `inputs:` completeness, AC-owning-artifact cross-reference
discipline) was also re-verified clean, with the arm-parity and locus-class-extension pair now
at its FIFTH consecutive confirmation.

**Disposition:** Recorded as `[convergence-progress]`, the same tag used at D-1093/D-1095/D-1098
for prior single-pass confirmations following a fresh codification or a reset. This is the
first direct EVIDENCE that the ninth discipline, applied proactively from the start of a
fresh-context pass, closes the class it targets — evidence, not yet proof, since a single clean
pass is not itself a confirmation the way three consecutive passes would be. Per BC-5.39.001,
this is 1 of 3 required CONSECUTIVE clean passes counting from the pass-44 governance-fix — the
streak requires 2 further consecutive clean passes (46, 47), and any finding on either would
reset the streak to 0/3 again. The correct disposition for passes 46 and 47 is to CONTINUE
applying all nine disciplines proactively without relaxing rigor on the theory that "the
illustrative-quote dimension is now proven safe" — one clean pass covering a freshly-codified
discipline is progress, not closure.
`[convergence-progress; ninth-discipline-reconfirmed; fifth-clean-pass; cleanest-set-yet;
D-1102; adr-046-gate; pending-pass-46-47-confirmation; not-yet-converged]`

---

**[codified][process-gap] A sweep that judges a locus "clean" must RECORD WHY (arm-split-
correct / semantic-region-true-both-arms / needs-caveat), not just assert "checked" —
self-attested-but-unexplained checks can hide a locus still carrying stale framing**

At D-1097 (pass-40), the sibling-sweep-includes-VPs discipline swept every
`extract_frontmatter`/frontmatter-slice/byte-range locus in BC-4.17.001 and listed Invariant 5
as "confirmed correct, arm-split since v1.18, not re-broken." That verdict was WRONG: Invariant
5's headline ("body never read or parsed") was still carrying the pre-F-P39-001 un-caveated
byte-range framing, and its "Mirrors BC-4.13.001 Invariant 9" citation imported BC-4.13.001's
read-only `read_prefix` semantics without the reader-vs-writer caveat this hook's whole-file
`host::read_file`/`host::write_file` call site requires. The "checked" label recorded that the
locus was inspected, but not the REASONING that produced the "correct" verdict — so a
one-word self-attestation survived six further passes (41 through 45) without being
independently re-derived from first principles, because each subsequent pass trusted the
prior pass's "checked" label rather than re-deriving the arm-split from the locus's own text.
This is a distinct failure mode from O-P37-001's "self-attested read-in-full has no mechanical
backing" — that lesson concerns claiming a document was READ; this one concerns claiming a
VERDICT was correct without recording the reasoning that makes it checkable by a future pass.

**Disposition:** When any convergence-technique sweep (arm-parity, locus-class-extension,
byte-range/body-confinement, or any future class) judges a locus "clean" / "correct" /
"unaffected," the record MUST capture WHY — the specific classification the locus falls into
(e.g., "arm-split-correct: byte-range-confined on PC1, semantic-region-guarantee on PC2" or
"semantic-region-true-under-both-arms" or "unrelated to this claim class entirely") — not a
bare "checked" or "confirmed correct" verdict with no reasoning attached. This makes a future
fresh-context pass able to verify the PRIOR pass's reasoning against the locus's actual current
text, rather than trusting an opaque pass/fail label. Applied this burst: the mandatory
exhaustive byte-range/body-confinement locus audit (F-P46-001's remediation) records, for every
locus enumerated, which of the three classifications it falls into and why — the model for all
future locus-class audits under this discipline.
`[codified; process-gap; record-why-not-just-checked; byte-range-class-drained; D-1103;
adr-046-gate; F-P46-001; pass-40-sweep-gap]`

---

**[codified][process-gap] The grep-complete-all-cluster-artifacts audit discipline (D-1100,
eighth discipline) extends to AC-NNN cross-references and their attached verbatim quotes, not
only to `inputs:`-array completeness — a fix scoped to ONE cited AC-NNN locus does not by
itself confirm every other AC-NNN reference across the cluster is correctly scoped**

At D-1100 (pass-43), F-P43-002 fixed ADR-046's/BC-7.07.001's mis-scoped "AC-018" cross-
reference (implying it was BC-7.07.001's own acceptance criterion, when AC-018 is in fact
S-18.04a's story-level AC). That fix was scoped to the ONE locus the pass-43 finding named. It
did not trigger a grep-complete sweep of every OTHER `AC-[0-9]+` token across the frozen set —
so ADR-046's own separate "BC-5.40.001 Invariant 2/AC-007" cross-reference, carrying the
identical mis-scoping pattern (AC-007 is S-17.01's story-level AC, not BC-5.40.001's own),
survived three further passes (44, 45, unfound-at-43) unaudited, plus a fabricated "verbatim
quote" attached to it that was never checked against either candidate source text. The
underlying discipline (D-1100) was correctly grep-complete for `inputs:`-array
citations — it had not yet been extended to treat AC-NNN cross-references (and any illustrative
quote attached to them) as their own grep-complete class.

**Disposition:** Any burst that fixes a mis-scoped AC-NNN cross-reference MUST, in the same
pass, run a grep-complete sweep of every `AC-[0-9]+` token across the full frozen set's live
body (not just the one finding's own locus) and verify: (a) the citation names the actual
OWNING artifact (story vs. BC — extending D-1100's existing owning-artifact discipline), and
(b) any quote presented as verbatim is checked against the actual cited source text via a
repo-wide grep, not assumed accurate because it "sounds right." Applied this burst: F-P46-002's
mandatory exhaustive AC-reference audit swept every `AC-[0-9]+` token in ADR-046's body (found
AC-007 ×2, both now fixed; AC-018 ×2, both already correct from pass-43) — the model for all
future AC-reference-scoped fixes at this gate. This audit class is now confirmed DRAINED for
the two-instance cluster identified across passes 43 and 46 (AC-018, AC-007); a future AC-NNN
citation added to any of the four frozen-set artifacts must re-trigger this same grep-complete
sweep before the burst closes.
`[codified; process-gap; ac-reference-grep-complete; verbatim-quote-source-check; D-1103;
adr-046-gate; F-P46-002; D-1100-extension]`

---

**[codified][process-gap] Class-draining audits must sweep ALL cluster artifacts in the SAME
burst — unifying meta-lesson spanning passes 43, 46, and 47's AC-attribution stragglers**

The AC-attribution defect class (a live-body citation mis-scoping a story-level AC-NNN as if it
belonged to the BC it cites) recurred THREE times across three passes before it was genuinely
drained: pass-43 (F-P43-002) fixed BC-7.07.001's own "AC-018" mis-scoping, but the fix's audit
was scoped only to BC-7.07.001 — it did not sweep ADR-046 or the other companion BCs for the
same pattern. Pass-46 (F-P46-002) then found ADR-046's own separate "AC-007" mis-scoping — a
DIFFERENT artifact, the SAME pattern — and fixed it, but again the audit was scoped only to
ADR-046, not to the two companion BCs that also cite BC-5.40.001/AC-007. Pass-47 (F-P47-001)
then found BC-4.17.001's own Invariant 3 carrying the identical AC-007 mis-scoping — the THIRD
instance, the direct cluster-sibling of pass-46's fix, surviving because pass-46's audit never
swept BC-4.17.001 in the same burst. Each individual audit (pass-43, pass-46) was itself
correctly grep-complete WITHIN its own artifact's scope — the gap was never in the grep
mechanics, but in the SCOPE BOUNDARY: each audit treated "the artifact the finding named" as the
sweep's perimeter, when the actual perimeter should always have been "every artifact in the
frozen cluster" (ADR-046 + BC-4.17.001 + BC-5.40.001 + BC-7.07.001). Pass-47's own audit broke
this pattern deliberately — it swept all three companion BCs in one pass BEFORE any further
finding could surface the gap piecemeal — and confirmed the class is now genuinely DRAINED
cluster-wide: zero remaining live-body AC-NNN mis-anchors across any of the four frozen-set
artifacts.

This is not limited to the AC-attribution class. The SAME single-artifact-scoping failure mode
could recur for any other class-draining audit this gate runs: `inputs:`-completeness
(D-1090/D-1100), byte-range/body-confinement arm-scope (D-1096/D-1097), BC-to-BC/ADR
cross-anchor citations (D-1092/F-P35-001 class), or illustrative verbatim-quote accuracy
(D-1101). Each of those disciplines was ALREADY extended to be grep-complete WITHIN a single
artifact by its originating pass — but none of them had an explicit, standing requirement that
the sweep itself default to cluster-wide scope regardless of which one artifact triggered it.

**Disposition:** ANY class-draining grep audit run at this gate — whether triggered by an
`inputs:`-completeness finding, an AC-NNN cross-reference finding, a byte-range/arm-scope
finding, a BC↔BC/ADR cross-anchor finding, or a verbatim-quote-accuracy finding — MUST, as a
standing default, sweep ALL FOUR cluster artifacts (ADR-046, BC-4.17.001, BC-5.40.001,
BC-7.07.001) in the SAME burst that produces the fix, not just the one artifact where the
finding happened to surface. This eleventh discipline supersedes the "sweep the artifact this
finding names" default that pass-43 and pass-46 each implicitly applied. Applied this burst:
F-P47-001's own audit swept BC-4.17.001, BC-5.40.001, and BC-7.07.001 in a single pass —
confirming the AC-attribution class fully drained, with zero remaining live-body mis-anchors in
any of the three BCs and (from pass-46) ADR-046. Future class-draining audits at this gate
should apply this same cluster-wide-by-default scope from the outset, rather than waiting for a
cluster-sibling straggler to surface it pass-by-pass.
`[codified; process-gap; cluster-wide-audit-scope; ac-reference-grep-complete; D-1104;
adr-046-gate; F-P47-001; D-1100-extension; unifying-meta-lesson; pass-43-46-47-root-cause]`

---

**[codified][process-gap] VERBATIM-ABSENCE claims require a TRUE repository-wide grep, with
scope stated explicitly — twelfth distinct convergence-technique discipline**

At pass-46 (F-P46-002), the architect's fix corrected ADR-046's fabricated "verbatim quote"
claim by asserting the quoted phrase ("MUST NOT be overridden via environment or arguments") was
"not present anywhere in this repository," backed by a `grep -rn` sweep. That sweep was, in
fact, rooted at `.factory/` — it never searched `plugins/` or `crates/`. At pass-48 (F-P48-001),
a TRUE repository-wide grep (executed this pass, spanning the whole repository, not
`.factory/`-scoped) found the phrase verbatim-present in
`plugins/vsdd-factory/bin/factory-lock-write.sh`'s `TTL_SECONDS` header comment — an
`inputs:`-listed file this ADR already cites for the same TTL constant. The phrase was INHERITED
from that file, not fabricated; the pass-46 "fabricated" claim was itself false, caused by an
under-scoped grep presented with unqualified "anywhere in this repository" language.

**Disposition:** ANY assertion in disposition/changelog prose that a phrase, quote, or pattern is
"fabricated," "not present anywhere," or "verbatim-absent" MUST be backed by a grep that
genuinely covers the full repository — every `inputs:`-listed file class, including `plugins/`
and `crates/`, not just `.factory/` — and the disposition prose recording the claim MUST STATE
the scope of the grep performed (e.g., "repo-wide, not `.factory/`-scoped") so a later pass can
verify the claim's provenance without re-deriving it from scratch. An absence claim that does not
state its own scope is not verifiable and should be treated as suspect. Applied this burst:
F-P48-001's own fix explicitly states "a TRUE repo-wide `grep -rn` for the phrase, across the
whole repository, not `.factory/`-scoped" as the basis for the corrected claim.
`[codified; process-gap; verbatim-absence-claim; repo-wide-grep-scope; D-1105; adr-046-gate;
F-P48-001; twelfth-discipline]`

---

**[content-defect-discipline] SUMMARY-ENUMERATION claims must match the normative body exactly —
thirteenth distinct convergence-technique discipline**

At pass-48 (O-P48-001), BC-7.07.001's Description used "only" to enumerate the hook's exit-0
conditions as success/no-op/STATE.md-unreadable — omitting two normatively-documented exit-0
paths (Precondition 4's worktree-discovery-failure/split-tree-mismatch, and Postcondition 9's
hook-crash-under-`on_error=continue`). The mandatory within-artifact sweep this finding triggered
(grepping the BC's Description plus all other summary/overview prose for further "only"/
"exclusively"/"exhaustively" claims, cross-checked against the normative Preconditions/
Postconditions/Invariants) found a SIBLING straggler of the identical defect class inside the
NORMATIVE body itself: Postcondition 8's own closing sentence restated the same under-inclusive
exit-0 list ("The ONLY exit codes this hook ever produces are 0 ... and 2 ...") — a more serious
instance than the Description summary, since it purports to be part of the authoritative
enumeration rather than an informal overview.

**Disposition:** any Description/overview sentence using "only," "exclusively," "exhaustively,"
or equivalent totalizing language to enumerate a set of outcomes, conditions, or exit codes MUST
be cross-checked against the FULL set of Preconditions/Postconditions/Invariants that can produce
that outcome — not just the ones the summary sentence's author had in mind at drafting time — and
any such claim found in one location REQUIRES a within-artifact sweep for sibling instances of the
same pattern before the burst closes, since (as this pass demonstrated) an under-inclusive claim
in an informal summary section often has a normative-section sibling carrying the identical
defect. Applied this burst: the Description fix and the Postcondition 8 sibling fix were both
applied same-burst, from the single triggering finding's mandated sweep.
`[content-defect-discipline; summary-enumeration-accuracy; sibling-sweep; D-1105; adr-046-gate;
O-P48-001; thirteenth-discipline]`

---

**[process-observation][convergence-observation] META: a remediation's OWN disposition/changelog
prose is itself attack surface — now confirmed across FOUR instances (pass-37, pass-44, and TWO
at pass-48)**

Four separate findings at this gate now share the same recurring root pattern: the prose a fix
writes to DESCRIBE its own correction contains a factual error independent of the substantive
fix itself. Pass-37 (F-P37-001): ADR-046's own decision-count mis-statement in disposition prose.
Pass-44 (O-P44-001): BC-5.40.001's illustrative "verbatim quote" misattributed to the wrong
source. Pass-48 (F-P48-001): ADR-046's own "fabricated" provenance claim, itself false, caused by
an under-scoped grep. Pass-48 (O-P48-001): BC-7.07.001's own Description/Postcondition-8
under-inclusive exhaustive-enumeration claim. In every instance, the LIVE-BODY substantive
correction being described was itself accurate — only the NARRATIVE describing/justifying it
carried the defect. This confirms disposition/changelog/`last_amended` prose is not inert
documentation of a fix; it is itself a surface an adversarial pass will find defects in, at a
rate comparable to the live body.

**Disposition (reaffirmed, no change from pass-44's original framing):** keep disposition prose
MINIMAL, and verify every factual claim it makes — every quote, every count, every "fabricated"/
"absent"/"only" assertion — via direct evidence (grep, count, cross-reference) BEFORE writing it,
rather than composing persuasive narrative first and treating its claims as self-evidently true
because they read plausibly. Applied this burst: F-P48-001's own fix narrative states the exact
grep command scope used to verify the corrected claim; O-P48-001's own fix narrative states the
exact sweep performed and its one additional finding (Postcondition 8) — both narratives are
falsifiable, not merely assertive.
`[process-observation; convergence-observation; meta-lesson; disposition-prose-attack-surface;
D-1105; adr-046-gate; F-P37-001; O-P44-001; F-P48-001; O-P48-001; four-instance-pattern]`

---

**[codified][process-gap] CITATION→INPUT PARITY — a body edit that adds a verbatim citation must
add its source to `inputs:` in the SAME burst — fourteenth distinct convergence-technique
discipline**

At pass-49 (F-P49-001), ADR-046's own v1.19 disposition prose — itself the pass-48 fix that
re-attributed AC-007 to S-17.01 and quoted BC-5.40.001 Invariant 2 verbatim — quoted S-17.01's
AC-007 verbatim without adding S-17.01 to ADR-046's own `inputs:` array. This is distinct from
every prior `inputs:`-completeness finding at this gate (F-P28-002, F-P29-002, F-P30-002,
F-P31-001, F-P33-001, F-P35-002, F-P43-001): those were all PRE-EXISTING omissions a
not-yet-performed audit would have caught. F-P49-001 is the FIRST instance where a PREVIOUSLY-RUN
grep-complete audit (pass-43's, extended cluster-wide at pass-44) was subsequently invalidated by
a LATER body edit (pass-46/48's AC-007 re-attribution) that added a brand-new citation without a
same-burst `inputs:` update — the audit was correct when it ran; the ground shifted under it
afterward. The mandatory re-audit this finding triggered found 6 more instances of the identical
mechanism, cluster-wide: ADR-046's own §Companion Amendment 3 (S-18.04a, cited since pass-43);
BC-4.17.001's Invariant 3 (S-17.01, cited since pass-47) and Invariant 5 (BC-1.17.001); BC-5.40.001's
§VP Anchors (S-19.08); BC-7.07.001's PC4/Architecture Anchors (`factory-lock-write.sh`) and
Related BCs (BC-7.07.002).

**Disposition:** (i) any body edit that ADDS a verbatim citation or quote of a source file or
story — not merely a passing mention, but content the artifact's own claims depend on — MUST add
that source to `inputs:` in the SAME burst as the edit that introduces the citation; this is a
point-of-edit obligation, not deferred to the next scheduled audit. (ii) Because the
grep-complete inputs audit (D-1090/D-1100, eighth discipline) is inherently POINT-IN-TIME, it
does not stay valid across subsequent body-evolving edits — after any RUN of body-evolving
bursts (such as this gate's own passes 43 through 48, each adding or correcting a live citation),
a fresh CLUSTER-WIDE grep-complete inputs re-audit must be performed, not merely trusted because
an earlier pass's audit reported clean. Applied this burst: state-manager ran the compute-input-
hash `--check`/`--update` recompute across all four cluster artifacts, and product-owner/architect
each ran the triggered re-audit as a cluster-wide sweep rather than scoping to the single artifact
the initiating finding named — directly applying the D-1104 eleventh-discipline standing default
to this fourteenth discipline's own re-audit.
`[codified; process-gap; citation-input-parity; cluster-wide-reaudit; D-1106; adr-046-gate;
F-P49-001; fourteenth-discipline]`

---

**[codified][process-gap] "Verified present in STORY-INDEX" claims require a mechanical
membership check, not a BC-prose-only Traceability fix**

At pass-50 (F-P50-001), BC-4.17.001's own §Story Anchor and ADR-046's own §File-Change Plan both
asserted that S-17.05 (the ADR-046 implementing story) was "verified present in STORY-INDEX.md" /
"is its catalog entry." This claim was FALSE: STORY-INDEX's E-17 roster ended at S-17.04, marked
"COMPLETE"; S-17.05 existed as a drafted story FILE (authored by story-writer earlier this
session) but was never REGISTERED as a STORY-INDEX catalog row. The false "verified present"
language traces to the pass-25 F-P25-002 remediation (2026-08-26), which resolved the
Traceability `[pending]`→S-17.05 anchor in the BC's own prose but never performed — nor claimed to
perform — the actual STORY-INDEX membership check the "verified present" wording asserts. The gap
survived ~24 further passes (pass-26 through pass-49) because every intervening audit type
(grep-complete `inputs:`-completeness, AC-attribution cross-reference) checks CITATION
correctness, not INDEX-ROW EXISTENCE — a claim can cite a story by exact path, quote its content
correctly, and still be lying about whether that story is catalogued anywhere.

**Disposition:** any spec-prose assertion of the form "X is verified present in <INDEX>" or "X is
<INDEX>'s catalog entry" is NOT self-certifying — it MUST be backed by an actual mechanical
index-membership check (e.g. `grep -c "^| X |" <INDEX>.md`) performed at or before the time the
assertion is written, not inferred from a sibling Traceability-row edit in a DIFFERENT artifact.
Standing mandate: (i) any newly-drafted implementing story MUST be registered in STORY-INDEX at
draft time, not deferred until its own PR; (ii) any BC/ADR/spec §Story Anchor or similar
"present-in-index" assertion about a story, BC, VP, or ADR requires the citing agent (or the
state-manager burst that reconciles it) to run the membership check and record that it was run,
not merely assert the conclusion. This is distinct from CITATION→INPUT PARITY (fourteenth
discipline, D-1106): that discipline governs whether a citation's SOURCE is listed in `inputs:`;
this discipline governs whether an "present-in-<INDEX>" CLAIM about that source is actually true.
`[codified][process-gap][catalog-membership-verification; D-1107; adr-046-gate; F-P50-001;
fifteenth-discipline]`

---

**[codified][process-gap] CITATION→INPUT PARITY (fourteenth discipline) extends to exact-path
STORY citations, not merely file/BC/ADR citations**

At pass-50 (F-P50-002), all three ADR-046 companion BCs (BC-4.17.001, BC-5.40.001, BC-7.07.001)
cited S-17.05 by exact file path and content claims (§Story Anchor: E-17 Wave 5, 8pts, `tdd_mode:
strict`) in their live bodies, but S-17.05 was absent from all three BCs' `inputs:` arrays. This is
a direct instance of the CITATION→INPUT PARITY discipline (fourteenth, D-1106) — a body edit that
adds a verbatim or exact-path citation of a source MUST add that source to `inputs:` in the SAME
burst — applied to a citation TYPE the discipline's initial codification did not explicitly
enumerate: exact-path STORY citations, as distinct from the file/BC/ADR citations D-1106's own
finding set covered. BC-5.40.001's own cross-check (product-owner, in-scope this pass) additionally
found and fixed a sibling gap of the identical class: S-17.01, cited with content claims since
PR #181/D-544 (2026-06-11), was ALSO missing from BC-5.40.001's `inputs:`.

**Disposition:** the fourteenth discipline's perimeter is CONFIRMED to cover every citation TYPE
that makes a load-bearing claim about a source's content — file paths, BC/ADR references, AND
story IDs cited by exact path — not merely the file/BC/ADR subset D-1106's own finding happened to
instantiate. Future grep-complete `inputs:` audits at this gate must explicitly include a
story-ID-shaped token sweep (`S-[0-9]+\.[0-9]+`) alongside the existing file-path and BC/ADR-ID
sweeps, not treat story citations as a separate, lower-priority class.
`[codified][process-gap][citation-input-parity-extension; D-1107; adr-046-gate; F-P50-002;
fourteenth-discipline-extension]`

---

**[content-defect-discipline] Illustrative "analogous to T-NNN" enumerations must match the
authoritative implementing-BC's own basis — an extension of the ninth discipline (D-1101) from
verbatim quotes to illustrative example-lists**

At pass-51 (O-P51-001), ADR-046's own §Decision 5 per-element reconciliation table illustratively
enumerated BC-4.17.001's migrated VP-row analogs as "analogous to T-001/T-002/T-003/T-004/T-007."
This was IMPRECISE: T-002/T-003 are BC-5.40.001's staleness-BLOCK tests with no stamper analog
(the always-allow-and-correct stamper never blocks, so they were never migrated), and T-005
(`extract_frontmatter`/no-delimiter fail-open) was omitted despite being migrated. BC-4.17.001's
own §Verification Properties note cites the exact set "T-001/T-004/T-005/T-007" as the
authoritative migrated-analog basis (VP-TBD-7/8/9) — the sibling BC had it right; only ADR-046's
own illustrative parenthetical carried the imprecise enumeration.

**Disposition:** this is NOT a new standalone discipline — it is a content-defect instance of the
existing NINTH discipline (D-1101), which required illustrative "verbatim quote" content attached
to a fix's own disposition prose to cite the ACTUAL text of the cited source, cross-checked via
sibling-parity. O-P51-001 demonstrates the same underlying obligation applies to a DIFFERENT
illustrative-content shape: an "analogous to <ID-list>" example enumeration, not only a quoted
string. Standing mandate (extended): any illustrative content in an ADR/BC that references another
artifact's own basis for a claim — whether a verbatim quote OR an enumerated ID-list — MUST be
cross-checked against that other artifact's own authoritative statement of the same claim before
being asserted independently, and any within-artifact sibling instances of the same illustrative
reference MUST be swept for the identical defect (this pass's T-NNN sweep found none, confirming
the defect was confined to this single locus).
`[codified][process-gap][content-defect-discipline][ninth-discipline-extension; D-1108;
adr-046-gate; O-P51-001]`

---

**[process-observation][convergence-observation][meta] Pass-51 zero-BLOCKER/HIGH/MED result
confirms the substance and all fourteen prior metadata-layer disciplines continue holding**

Pass-51 is the cleanest pass this gate has produced since the last clean streak (pass-45): zero
BLOCKER/HIGH/MED findings, only a single LOW illustrative-enumeration observation (O-P51-001),
itself an instance of an already-codified discipline (the ninth, D-1101) rather than a new failure
mode. The behavioral core (write-composition table, five-outcome table, identity-gating logic,
event-sourcing struct-variant text) remains independently re-verified CLEAN for the 25th
consecutive pass (since pass-27). None of the fourteen previously-codified convergence-technique
disciplines (comprehensive-sibling-sweep, 4-leg parity, grep-complete inputs, ADR anchor audit,
minimal-prose discipline, arm-parity sweep, sibling-sweep-includes-VPs, AC-attribution
cross-reference, illustrative-quote accuracy, record-why-not-just-checked, cluster-wide-audit
scope, verbatim-absence grep, summary-enumeration accuracy, CITATION→INPUT PARITY) regressed this
pass. **Disposition:** because ADR-046 itself was edited this burst (O-P51-001's fix), the
BC-5.39.001 3-CLEAN streak does NOT advance on the strength of this clean-of-blockers result — the
literal-3-CLEAN count restarts fresh at pass-52 against the newly-edited set, per BC-5.39.001's
own discipline (any spec edit, even a fix, requires re-review of the edited artifact before it can
count toward a clean streak).
`[process-observation][convergence-observation][meta; D-1108; adr-046-gate; pass-51]`

---

**[convergence-progress] Pass-52 re-applied all fifteen now-codified convergence-technique
disciplines proactively — including the ninth discipline's D-1108 extension to illustrative
example-enumerations, the exact dimension pass-51's own finding (O-P51-001) targeted — and found
the entire O-P51-001-corrected frozen set clean on every dimension; streak ADVANCES 0/3 → 1/3, the
first clean pass against the corrected set.**

Pass-52 dispatched fresh-context, information-asymmetric per the Iron Law, against the frozen set
ADR-046 v1.21 / BC-4.17.001 v1.24 / BC-5.40.001 v1.20 / BC-7.07.001 v1.37 — the set produced by the
pass-51 fix burst. Zero findings at any severity. Most directly, ADR-046 §Decision 5's illustrative
"analogous to T-NNN" enumeration (fixed by architect at D-1108 to read "T-001/T-004/T-005/T-007")
was independently re-derived and confirmed matching BC-4.17.001's own authoritative basis, with a
fresh within-artifact sweep finding no sibling recurrence anywhere else. All fourteen prior
disciplines (comprehensive-sibling-sweep, 4-leg parity, grep-complete inputs, ADR anchor audit,
minimal-prose discipline, arm-parity sweep, sibling-sweep-includes-VPs, AC-attribution
cross-reference, illustrative-quote verbatim accuracy, record-why-not-just-checked,
cluster-wide-audit scope, verbatim-absence grep, summary-enumeration accuracy, CITATION→INPUT
PARITY, catalog-membership-verification) also re-verified holding with zero regression. The
behavioral core (write-composition, five-outcome table, identity-gating, event-sourcing) remains
independently re-verified CLEAN for the 26th consecutive pass (since pass-27).

**Disposition:** Recorded as `[convergence-progress]`, the same tag used for D-1093's pass-36 and
D-1095's pass-38 confirmations, distinguished by scope: this entry is the first direct EVIDENCE
that the ninth discipline's D-1108 extension (illustrative example-enumerations, not only verbatim
quotes) holds under a fresh adversary's independent re-derivation — evidence, not yet proof, since
a single clean pass is not itself a confirmation the way three consecutive passes would be. Per
BC-5.39.001, this is 1 of 3 required CONSECUTIVE clean passes — the streak requires 2 further
consecutive clean passes (53, 54), and any finding OR spec edit on either would reset the streak to
0/3 again. The correct disposition for passes 53 and 54 is to CONTINUE applying all fifteen
disciplines proactively without relaxing rigor on the theory that "the illustrative-enumeration
extension is now proven safe" — one clean pass on a freshly-extended discipline is progress, not
closure. `[convergence-progress; fifteen-disciplines; ninth-discipline-extension-reconfirmed;
first-clean-pass-post-O-P51-001; D-1109; adr-046-gate; pending-pass-53-54-confirmation;
not-yet-converged]`

---

**[convergence-progress] Pass-53 is the second CONSECUTIVE clean pass against the unchanged
O-P51-001-corrected set — streak ADVANCES 1/3 → 2/3, one clean pass short of literal 3-CLEAN.**

Pass-53 dispatched fresh-context, information-asymmetric per the Iron Law, against the SAME frozen
set pass-52 reviewed (ADR-046 v1.21 / BC-4.17.001 v1.24 / BC-5.40.001 v1.20 / BC-7.07.001 v1.37).
Zero findings at any severity. All fifteen previously-codified convergence-technique disciplines
re-verified holding with zero regression, including a THIRD consecutive independent re-derivation
of the ninth discipline's D-1108 illustrative-enumeration extension (pass-51 fix, pass-52
re-derivation, pass-53 re-derivation). The behavioral core remains independently re-verified CLEAN
for the 27th consecutive pass (since pass-27).

**Disposition:** Recorded as `[convergence-progress]`, distinguished from D-1109's entry by being
the SECOND consecutive confirmation rather than the first — this is materially stronger evidence
that the ninth-discipline extension and all fourteen other disciplines are stable under repeated
independent scrutiny, not a one-off. Per BC-5.39.001, this is 2 of 3 required CONSECUTIVE clean
passes — exactly 1 further consecutive clean pass (54) is required for literal 3-CLEAN convergence.
Any finding OR spec edit on pass 54 resets the streak to 0/3 again. The correct disposition for
pass 54 remains to CONTINUE applying all fifteen disciplines proactively without relaxing rigor —
being one pass from convergence is not itself a reason to review less carefully.
`[convergence-progress; fifteen-disciplines; ninth-discipline-extension-triple-confirmed;
second-consecutive-clean-pass; D-1110; adr-046-gate; pending-pass-54-confirmation;
not-yet-converged]`

---

**[convergence-governance] At streak 2/3, an adversary-adjudicated-defensible LOW descriptive item
is accepted-and-tracked rather than fixed — fixing a defensible non-defect would reset a live
convergence streak for no substantive gain.**

Pass-53's adversary considered BC-7.07.001 §Description's "Renewal is a no-op when: … or
`expires_at` is malformed (never repaired)" phrasing against the normative body's actual
`Err(LockError::Malformed(msg))` return (Postcondition 3 case 1 / Invariant 3b table row 1 /
EC-004), and correctly distinguished a plain-English observable-effect summary ("no-op" describing
the fact that nothing gets written) from an assertion of the `RenewOutcome::NoOp` Rust enum variant
(which this case explicitly is NOT). The Description's substantive claim is true; only its
colloquial label differs from the normative body's precise type vocabulary. **Disposition:**
tracked as O-P53-DESC-NOOP, ACCEPTED non-blocking, NOT fixed this pass — applying the same
`[convergence-governance]` fix-vs-accept discipline the O-P42-001 disposition (D-1099) established:
a defensible non-defect found while the streak is live (2/3, one pass from convergence) is accepted
and tracked rather than fixed in-place, because the fix costs a live streak for a purely
editorial/precision gain, not a correctness gain. This differs from the D-1101 fix-vs-accept
precedent (which fixed O-P44-001/O-P48-001/O-P51-001 at streak-floor 0/3, where a fix costs
nothing) — the disposition RULE is the same (weigh fix-cost against streak-state), but the
STREAK-STATE input differs, correctly producing the opposite action (accept, not fix) at non-floor
streak. Anchor: optional Description-precision tightening at a future non-gating touch.
`[convergence-governance][accept-vs-fix][streak-state-dependent; O-P53-DESC-NOOP; D-1110;
adr-046-gate; O-P42-001-precedent-consistent; O-P51-001-precedent-distinguished]`

---

**[codified][process-gap] STEP-NUMBER CITATION discipline — any "Step N"/"Steps N-M" citation of a
module's own internal enumeration MUST be cross-checked against that module's actual
`//!`/doc-comment step numbering, not merely checked for functional/arm correctness.**

Pass-54's adversary found F-P54-001: ADR-046 mis-cited `verify-state-timestamp-refresh`'s own
module-doc step numbers at four loci — the lock-expiry (`factory_lock.expires_at`) staleness arm
was labeled "Step 7" (correct: "Step 8"), and the timestamp staleness arm was labeled "Steps 4-6"
(correct: "Steps 4-7"), confirmed against the module's own doc-comment enumeration ("skip Steps
4-7; proceed to Step 8"). This defect survived 53 prior passes because every prior pass verified
the FUNCTIONAL correctness of the two-arms description (which was always accurate — both arms
correctly attributed to the right behavior) without independently re-deriving the LITERAL step
numbers from the cited module's own source. Functional correctness and numeric-label correctness
are separable failure modes: a citation can correctly describe WHAT a step does while citing the
WRONG NUMBER for it, and no prior discipline's grep/audit pattern (inputs-completeness,
AC-attribution, illustrative-content-accuracy, etc.) was scoped to catch numeric step-citation
drift specifically. **Disposition:** CODIFIED as the SIXTEENTH convergence-technique discipline —
any "Step N"/"Steps N-M" citation of a module's own enumeration must be cross-checked, via direct
inspection of that module's actual `//!`/doc-comment numbering, before being asserted in a spec
document; a within-artifact grep-complete sweep of every `Step[s] [0-9]` token plus a cross-BC check
for the same class must accompany any such fix, mirroring the now-established pattern (fourteenth,
fifteenth disciplines) of pairing a point fix with a mechanical corpus-wide re-audit.
`[codified][process-gap][step-number-citation; sixteenth-discipline; F-P54-001; D-1111;
adr-046-gate; functional-vs-numeric-correctness-distinct]`

---

**[META] Sixth streak reset this session, SECOND at the convergence pass itself (2/3→0/3 at both
pass-43 and pass-54) — both convergence-pass resets came from a fresh-context adversary finding
ONE genuine-but-narrow defect via a lens no prior pass had used; empirical asymptotic-floor
confirmation.**

The BC-5.39.001 3-CLEAN streak has now reset six times this session (passes 35, 37, 39, 43, 46,
54), and twice specifically AT the convergence pass itself — i.e., with the streak already at 2/3
and only one more consecutive clean pass required for literal 3-CLEAN. Pass-43 (D-1101 range)
reset 2/3→0/3 via a `capabilities.md` inputs-completeness lens no prior pass had applied at that
depth; pass-54 (D-1111) reset 2/3→0/3 via an exact module-doc step-number citation lens no prior
pass had applied. In both cases the finding was a real, narrow, single-locus-class defect (not a
regression of a previously-fixed item, not a false positive), and in both cases the behavioral core
itself remained untouched and independently re-verified CLEAN. This is a second empirical data
point supporting the same conclusion the pass-43 reset first suggested: a 50+-pass-deep gate can
still be genuinely one fresh cross-check dimension away from a real (if narrow) defect, which is
the strongest argument yet for the D-386 Option C asymptotic-acceptance alternative to chasing
literal 3-CLEAN indefinitely — the marginal defects being found are increasingly confined to the
citation/provenance/numbering perimeter, never the behavioral core (CLEAN for 28 consecutive
passes). **Disposition:** recorded as a META observation for the human's ongoing convergence-
strategy decision (STATE.md §5 Pending Human Decision). The human was RE-OFFERED accept-provisional
under D-386 Option C this burst and has again DECLINED, choosing to CONTINUE looping toward literal
3-CLEAN. `[META][convergence-strategy][sixth-reset; second-convergence-pass-reset; pass-43-parallel;
D-1111; adr-046-gate; asymptotic-floor-empirical-confirmation; human-reaffirmed-continue]`

---

**[convergence-progress] Pass-55 is the first clean pass against the pass-54-corrected set, and the
FIRST independent re-derivation of the newly-codified sixteenth discipline (STEP-NUMBER CITATION)
since its D-1111 codification — streak ADVANCES 0/3 → 1/3.**

Pass-55 dispatched fresh-context, information-asymmetric per the Iron Law, against the frozen set
produced by the pass-54 fix burst (ADR-046 v1.22 / BC-4.17.001 v1.24 / BC-5.40.001 v1.20 /
BC-7.07.001 v1.37). Zero findings at any severity. Most directly, the adversary independently
re-derived every "Step N"/"Steps N-M" citation of `verify-state-timestamp-refresh`'s own module-doc
enumeration across ADR-046's body (§Context item 2, §Rationale, §Decision 3, §Decision 5) and
confirmed all four F-P54-001 loci now correctly read "Step 8" (lock-expiry arm) and "Steps 4–7"
(timestamp arm), cross-checked directly against the module's own source doc-comment — the fix landed
completely and correctly, with no partial fix and no sibling recurrence found anywhere in the frozen
set (within-artifact sweep plus cross-BC check, both clean). All fifteen other previously-codified
disciplines also re-verified holding with zero regression, including a fourth consecutive
independent re-derivation of the ninth discipline's D-1108 illustrative-enumeration extension. The
behavioral core remains independently re-verified CLEAN for the 29th consecutive pass (since
pass-27).

**Disposition:** Recorded as `[convergence-progress]`, the same tag used for D-1093/D-1095/D-1098/
D-1109/D-1110's confirmations, distinguished by being the first evidence that a freshly-codified
discipline (the sixteenth, STEP-NUMBER CITATION) holds under independent fresh-context
re-derivation immediately following its own codifying fix — evidence, not yet proof, since a single
clean pass is not itself a confirmation the way three consecutive passes would be. Per BC-5.39.001,
this is 1 of 3 required CONSECUTIVE clean passes — the streak requires 2 further consecutive clean
passes (56, 57), and any finding OR spec edit on either would reset the streak to 0/3 again (already
at floor). The correct disposition for passes 56 and 57 is to CONTINUE applying all sixteen
disciplines proactively without relaxing rigor on the theory that "the step-number dimension is now
proven safe" — one clean pass on a freshly-codified discipline is progress, not closure.
`[convergence-progress; sixteen-disciplines; sixteenth-discipline-first-confirmation;
first-clean-pass-post-F-P54-001; D-1112; adr-046-gate; pending-pass-56-57-confirmation;
not-yet-converged]`

---

**[codified][process-gap] 0TH-CASE/NO-OP CLAIM VERIFICATION — any "0th case" / "no lock held" / `NoOp`
claim about a degenerate or missing field value MUST be verified against the actual parser's
`Ok`/`Err` partition, with every degenerate sub-case (empty-string, absent-with-siblings-present,
explicit `null` token, and whitespace) traced individually, not inferred from the field's ABSENCE
alone.**

Pass-56's adversary found F-P56-001: ADR-046, BC-4.17.001, and BC-7.07.001 all mischaracterized an
empty-string or absent `holder` sub-field as equivalent to the pre-existing 0th case
(`factory_lock:` fully absent/null — silent `NoOp`), grounding the claim in "inherited from
`renew_lock`'s existing presence-precheck." Direct inspection of
`crates/factory-lock-parse/src/lib.rs`'s `parse_factory_lock` (architect) proved this FALSE:
`Ok(None)` is returned ONLY when `factory_lock:` is fully absent or present-but-fully-null with NO
sub-fields at all; once ANY sub-field is present, an empty-string or absent `holder` returns
`Err(MalformedLockBlock(..))` — mapped by `renew_lock_with_now` to `Err(LockError::Malformed)`, case
1, never `NoOp`. `has_factory_lock_key`'s presence pre-check tests only the literal `factory_lock:`
key line, never `holder`'s value, so the "inherited from... presence-precheck" grounding was itself
fabricated-by-inference, not verified. A THIRD degenerate sub-case — an explicit `holder: null` YAML
token — was missed by this burst's own round-1 sweep (BC-7.07.001 v1.38) and required a round-2
straggler fix (v1.39, EC-009 correction + new EC-011) once discovered that
`extract_yaml_string_value` has no special-case for the bare `null` token (it parses as the literal
string `"null"`, never absence). This defect survived 55 prior passes because no prior adversary had
independently re-derived the parser's actual `Ok`/`Err` partition for degenerate `holder` values from
its own match arms — every prior pass accepted the spec's own "0th case, no lock held" framing of the
empty/absent-holder condition at face value, the same failure mode the sixteenth discipline
(STEP-NUMBER CITATION, D-1111) named for step-number citations: accepting a spec's SELF-DESCRIPTION
of a mapping instead of independently re-deriving that mapping from the actual source. **Disposition:**
CODIFIED as the SEVENTEENTH convergence-technique discipline — any "0th case"/"no lock held"/`NoOp`
claim about a degenerate or missing field value must be verified against the actual parser's
`Ok`/`Err` partition, and EVERY degenerate sub-case of that field (empty-string, absent-with-siblings,
explicit `null` token, and — per this burst's own round-1 miss — an audit for whitespace-only values)
must be traced individually rather than assumed to share the same outcome; a fix addressing one
sub-case does not, by itself, establish coverage of sibling sub-cases, as this burst's own 2-round
sweep demonstrates.
`[codified][process-gap][0th-case-no-op-verification; seventeenth-discipline; F-P56-001; D-1113;
adr-046-gate; degenerate-sub-case-exhaustive-enumeration; two-round-sweep-required]`

---

**[META] Seventh streak reset this session, but the MOST SUBSTANTIVE finding of the entire
56-pass convergence effort — the first genuine spec-vs-code BEHAVIORAL divergence since the
behavioral core stabilized at pass-27.**

The BC-5.39.001 3-CLEAN streak has now reset seven times this session (passes 35, 37, 39, 43, 46,
54, 56). Unlike every one of the prior six resets — each confined to the
provenance/citation/traceability/metadata perimeter (cross-reference accuracy, `inputs:`
completeness, catalog membership, step-number citation, illustrative-enumeration accuracy) — pass-56's
F-P56-001 is a genuine BEHAVIORAL divergence: a claim about WHAT THE CODE ACTUALLY DOES in a specific
input state (empty/absent/null `holder`), and the claim was FALSE, verified false by direct inspection
of the parser's match arms and independently corroborated by a pre-existing unit test
(`test_BC_5_40_001_parse_factory_lock_errors_on_empty_holder`). This breaks a 29-consecutive-pass
clean streak on the specific dimension of "does the spec accurately describe the code's behavior in
every input state," the first crack in what had otherwise been the single most stable dimension of
this gate's entire history. Had this shipped uncorrected into S-17.05's TDD implementation, the
implementer would have built test coverage against a spec that contradicted the very code it
describes — a defect class the BC-5.39.001 literal-3-CLEAN discipline exists specifically to catch,
and did. **Disposition:** recorded as a META observation distinguishing this reset sharply from the
prior six metadata-perimeter resets — this is the concrete PAYOFF of continuing to grind toward
literal 3-CLEAN rather than accepting D-386 Option C asymptotic acceptance at an earlier streak-peak:
a fresh-context adversary, applying an edge-case lens no prior pass had used, caught a real defect at
pass 56 that 55 prior passes — including two each at streak 2/3, one pass from convergence — had all
missed. The human's standing decision to CONTINUE looping toward literal 3-CLEAN (declined
accept-provisional at every prior reset) is thereby validated by this pass's outcome specifically,
independent of whether the human continues declining accept-provisional going forward.
`[META][convergence-strategy][seventh-reset; first-behavioral-core-finding-since-pass-27;
F-P56-001; D-1113; adr-046-gate; literal-3-clean-payoff-demonstrated]`

---

**[convergence-governance] A fresh-context adversary explicitly adjudicating an item a NON-DEFECT
(correct-as-is, authorial-intent-optional) is accepted-and-tracked, not fixed — distinct from a
correctable inaccuracy, which IS fixed.**

Pass-57's adversary considered whether BC-4.17.001's EC-011 (`holder: ""`) lacking a parallel
`holder: null` edge case — while its sibling BC-7.07.001 v1.39 has one, added at the F-P56-001
round-2 straggler fix — constitutes a defect. It explicitly ruled NOT: BC-4.17.001 asserts nothing
FALSE about `holder: null` (that sub-case flows correctly through its general 0th-case/case-1..5
analysis; it is simply not called out as its own illustrative EC row). This is the governing
distinction this lesson codifies: **O-P51-001 (D-1108) found an illustrative "analogous to T-NNN"
enumeration that was factually WRONG** — a correctable inaccuracy, and it WAS fixed, at zero streak
cost (streak was at floor). **O-P57-001 (D-1114) found an illustrative-coverage asymmetry where
nothing asserted is false** — only an optional elaboration is absent, and whether to add it is an
authorial-intent/documentation-style call, not a content-correctness call an adversary can
adjudicate. **Disposition:** tracked as O-P57-001, ACCEPTED non-blocking, NOT fixed this pass — the
same `[convergence-governance]` fix-vs-accept discipline (D-1101, extended streak-state-dependent at
D-1110) applies: at streak 1/3, fixing an optional non-defect would cost a live streak for no
correctness gain. The TEST for this class going forward: does the finding assert the artifact
contains a FALSE claim (fix it, streak-cost permitting per D-1101/D-1110's weighing), or does it
observe an OPTIONAL absence with nothing false asserted (accept-and-track, never fix mid-streak)?
Anchor: optional `holder: null` EC mirror into BC-4.17.001 at a future non-gating touch.
`[convergence-governance][accept-vs-fix][non-defect-adjudication; O-P57-001; D-1114; adr-046-gate;
O-P51-001-precedent-distinguished; O-P42-001-and-O-P53-DESC-NOOP-precedent-consistent]`

---

**[codified][process-gap] A BC's §Description/§Traceability ADR-coverage enumeration MUST include
EVERY ADR Decision the BC is a migration TARGET of — verify against the BC's own "MIGRATED … §Decision
N" annotations, not just against the Decisions the BC was originally authored to cover.**

Pass-58's adversary found F-P58-001: BC-4.17.001's §Description ADR-046-coverage sentence and
§Traceability ADR Reference row enumerated "Decisions 1, 2, and 4" only, omitting **Decision 5** —
despite this BC's own Precondition 4, Invariant 7, Invariant 8, EC-015, and VP-TBD-7/8/9 all carrying
explicit "MIGRATED … per ADR-046 §Decision 5" annotations (originally sourced at F-P4-002, v1.4), and
despite ADR-046's own File-Change Plan + Companion Amendment 1 item (vi) directing that migration
explicitly into this BC. The BC was silently under-counting its own documented scope: a coverage
summary written once at initial authoring (v1.0, Decisions 1/2/4 only) was never revisited when a
LATER amendment (v1.4, F-P4-002) added an entire Decision's worth of migrated content to the BC's own
body. **CODIFIED test going forward:** whenever a BC's §Description/§Traceability enumerates its
ADR-coverage as a fixed Decision-number list, cross-check that list against every "MIGRATED …
§Decision N" / "per ADR-NNN §Decision N" annotation actually present in the BC's OWN live body
(Preconditions/Invariants/EC/VP rows) — the coverage summary must be a superset of every Decision the
body cites itself as implementing, not merely the Decisions cited at initial authoring. **Same defect
CLASS as O-P48-001** (a prior under-inclusive-coverage-enumeration gap, fixed under governance
election) — this is a re-surfacing of an EXISTING discipline at a different BC/Decision pairing, not a
new discipline. **Disposition:** FIXED by product-owner (BC-4.17.001 v1.25→v1.26). Anchor: apply this
cross-check proactively to ADR-046's OTHER companion BCs (BC-5.40.001, BC-7.07.001) and to ADR-046
itself at the next pass, per standard proactive-application-of-all-codified-disciplines practice.
`[codified][process-gap][ADR-decision-coverage-enumeration; F-P58-001; D-1115; adr-046-gate;
O-P48-001-same-class; eighth-reset]`

---

**[convergence-governance] O-P58-001 provenance-ID split adjudicated NON-DEFECT (origin-pass
F-P25-002 vs sibling-sweep-pass F-P27-001 correctly cited per locus) — track accepted.**

Pass-58's adversary independently re-derived why BC-4.17.001 cites F-P25-002 at BOTH its
§Traceability §Stories row AND its §Story Anchor section, while sibling BCs BC-5.40.001 and
BC-7.07.001 cite F-P25-002 at §Traceability but F-P27-001 at §Story Anchor — and confirmed this
asymmetry is CORRECT PROVENANCE, not a defect. F-P25-002 (pass-25) is the origin fix that first
resolved the `[pending]` S-17.05 placeholder at §Traceability across all three cluster BCs.
F-P27-001 (pass-27) was a §Story-Anchor-specific sibling-sweep fix applied only to
BC-5.40.001/BC-7.07.001, because BC-4.17.001's own pass-25 fix had ALREADY touched both its
§Traceability row and its §Story Anchor section in the same burst — leaving no separate §Story-Anchor
gap in BC-4.17.001 for a pass-27 fix to close. Citing the fix that actually closed each locus, rather
than forcing uniform citation across sibling BCs regardless of each BC's own distinct fix history, is
the correct provenance discipline. **Disposition:** CONFIRMED NON-DEFECT, tracked as O-P58-001,
ACCEPTED, no edit — so pass-59 (and any future pass) does not re-raise this cross-BC citation
asymmetry as a fresh finding.
`[convergence-governance][provenance-verification][non-defect-adjudication; O-P58-001; D-1115;
adr-046-gate; F-P25-002-vs-F-P27-001-provenance-confirmed]`

---

**[codified][process-gap] SWEEP-BOTH-MIGRATION-PARTIES-AT-FIX-TIME — fixing a migration-coverage
finding on one artifact MUST sweep the migration counterpart AND run the cluster-wide audit in the
SAME burst, not defer it to "next pass" (reinforces D-1104).**

Pass-59's adversary found F-P59-001 (MED, POLICY 4): BC-5.40.001's §Traceability ADR Reference row
and §Description omitted ADR-046 §Decision 5 despite this same BC's own Precondition 6, Invariant 7,
Invariant 8, EC-010, and §VP Anchors T-001..T-007 all carrying explicit MIGRATED/RETAINED-AS-
HISTORICAL annotations under §Decision 5. This is the EXACT mirror-image gap of BC-4.17.001's own
F-P58-001, found and fixed just one pass earlier (v1.25→v1.26) — but the pass-58 fix-burst touched
only BC-4.17.001 (the migration TARGET), not BC-5.40.001 (the migration SOURCE, where the contract
originated before being migrated out). The pass-58 disposition prose itself correctly IDENTIFIED the
sibling risk (Drift Items table: "apply proactively to BC-5.40.001/BC-7.07.001/ADR-046 at the next
pass") but did not act on it in the same burst — deferring the sweep cost a full adversary cycle
(pass-59) before the gap was actually closed, and re-triggered the streak-blocking mechanism a second
time for what is structurally the SAME underlying defect discovered at pass-58. **Root-cause class:**
this is the SAME single-artifact-scoped-fix pattern as the AC-attribution class drained at D-1104
(eleventh discipline) — a fix confined to the literal artifact named in the finding, without also
sweeping the artifact's structural counterpart (here: the OTHER party to a migration relationship),
reliably reproduces the same finding CLASS at the counterpart on the very next pass. **CODIFIED rule
going forward (extends D-1104):** any fix-burst that closes an inputs/AC-ref/ADR-Decision-coverage/
cross-anchor/byte-range class of finding MUST, in the SAME burst, sweep ALL cluster artifacts party to
that finding's class — for a migration-class finding specifically, this means BOTH the migration
SOURCE and the migration TARGET, plus the mandatory cluster-wide audit already codified at D-1115 —
not just the single artifact literally named in the finding text, and not "anchor for the next pass."
Pass-59's own fix (BC-5.40.001 v1.20→v1.21) additionally re-ran the cluster-wide audit in the SAME
burst and confirmed BC-4.17.001 (COMPLETE) and BC-7.07.001 (CLEAN, not a §Decision-5 participant) —
this is the pattern the codification requires; had pass-58's fix done the same, pass-59 would have
found zero findings instead of one. **Disposition:** FIXED by product-owner (BC-5.40.001 v1.20→v1.21)
plus in-burst cluster audit. Anchor: apply this same-burst-both-parties discipline to every future
migration-class or cross-artifact-coverage finding, not only ADR-Decision-coverage-enumeration.
`[codified][process-gap][sweep-both-migration-parties-at-fix-time; F-P59-001; D-1116; adr-046-gate;
reinforces-D-1104; mirror-of-F-P58-001]`

**L-BB-D1117-pass60-clean** — First clean pass (pass-60 CLEAN, streak 0/3→1/3) after the
pass-58/59 double-reset confirms the frozen set (ADR-046 v1.23 + BC-4.17.001 v1.26 + BC-5.40.001
v1.21 + BC-7.07.001 v1.39) is substantively clean: all seventeen codified disciplines hold, all
eight spec-vs-code ground-truth checks MATCH. Two non-defect observations (O-P60-001
opening-fence assumption, O-P60-002 trim_git_email cross-ref), both accepted-tracked. Needs 2 more
consecutive clean passes (61, 62) for literal 3-CLEAN.
`[convergence-progress][D-1117; adr-046-gate; pass-60-CLEAN; streak-1/3]`

**L-BB-D1118-pass61-clean** — Second consecutive clean pass (pass-61 CLEAN, streak 1/3→2/3):
frozen set passes nine extended spec-vs-code ground-truth checks (added `parse_factory_lock`
lines 207-227, `extract_yaml_string_value` no null-special-casing, `renew_lock_with_now`
Duration::seconds(2700)/byte-compare, `is_expired` now>=expires_at boundary, `trim_git_email`,
Step-4 identity-blind renew, plus confirmed-absent design-only symbols). All seventeen convergence
disciplines still hold; no regression. O-P61-001 reveals F-P56-001 defect class has an unswept
locus in implementation crate doc-comments — TRACKED DEFECT-TO-FIX, candidate anchor S-17.05.
One more consecutive clean pass (pass-62) reaches literal 3-CLEAN.
`[convergence-progress][D-1118; adr-046-gate; pass-61-CLEAN; streak-2/3]`

**L-BB-D1119-pass62-findings-arch-index-headline** — ARCH-INDEX output-cell embedded version literals go stale independently of ADR instruction-row directives. The O-P28-002 fix (pass-28) made the ADR File-Change-Plan instruction row version-stable by replacing "bump to v1.12" with "read ADR frontmatter version at bump time." But it left the ARCH-INDEX output cell's own headline marker (`**RATIFIED 2026-08-25; ADR-046 v1.18 as of this row.**`) as a hard-coded literal that must be manually swept on every ADR touch. F-P62-001 found this 5 revisions stale — a NEW LOCUS of the same staleness class. Durable close: any ARCH-INDEX cell that cites an artifact's current version MUST use a stable reference form ("current version per [artifact] frontmatter") rather than a hard-coded literal number. Hard-coded version literals in cell headlines are a recurrence trap; structural restatement (TD-VSDD-059) is the only correct fix.
`[codified][process-gap][D-1119; adr-046-gate; pass-62-FINDINGS; arch-index-output-cell-literal-staleness; O-P28-002-falsification]`

**L-BB-D1121-pass63-clean** — Post-reset restart at pass-63 CLEAN confirms the F-P62-001 structural fix
(ARCH-INDEX ADR-046 row headline marker → stable reference form) held under independent fresh-lens
re-derivation. The recurrence mechanism (sweep-every-touch failure on hard-coded version literals in
index cell headlines) is durably eliminated. All seventeen codified convergence-technique disciplines
re-verified holding with zero regression. BC-5.39.001 streak ADVANCES 0/3 → 1/3; two more consecutive
clean passes needed for literal 3-CLEAN.
`[convergence-progress][D-1121; adr-046-gate; pass-63-CLEAN; streak-1/3; F-P62-001-retired-confirmed]`

**L-BB-D1122-pass64-clean** — Pass-64 CLEAN advances the BC-5.39.001 streak to 2/3 — one pass from
literal 3-CLEAN. All seventeen spec-vs-code checks independently re-derived with zero divergence;
behavioral core confirmed stable. O-P64-001 (BC-4.17.001 holder:null EC asymmetry) is the same
documentation-symmetry class as O-P57-001 (D-1114) — adjudicated NON-DEFECT twice now; not a defect,
not fixed. O-P64-002 (stale factory-lock crate doc-comments) is already captured in S-17.05 v1.1 T-8.
The converged spec set has now passed 64 adversary reviews with 46 genuine findings fixed total.
`[convergence-progress][D-1122; adr-046-gate; pass-64-CLEAN; streak-2/3; one-pass-from-3-CLEAN]`

**L-BB-D1123-pass65-3clean-achieved** — Pass-65 CLEAN is the THIRD consecutive clean pass (63/64/65), achieving literal BC-5.39.001 3-CLEAN after 65 adversary passes and 9 streak resets this session. The final clean run (63/64/65) followed the F-P62-001 ARCH-INDEX structural fix (headline marker replaced with stable reference form per TD-VSDD-059). Novelty is ZERO across all 14 ground-truth verification checks — the spec-vs-code behavioral core has been substantively converged for many passes. Gate closure is PENDING: (a) fresh-context consistency-validator perimeter audit and (b) explicit human gate approval. S-17.05 TDD implementation remains gated until both steps complete.
`[convergence-progress][D-1123; adr-046-gate; pass-65-CLEAN; streak-3/3; LITERAL-3-CLEAN-ACHIEVED-63-64-65; closure-pending-consistency-audit+human-approval]`

**L-BB-D1124-perimeter-audit-caught-story-scope-gaps-post-3clean** — The fresh-context perimeter
audit (consistency-validator) caught 3 BLOCKS-CLOSURE story-scope gaps AFTER the adversary
declared 3-CLEAN (passes 63/64/65, D-1123). This validates the two-axis model: the adversary
verifies the spec set is internally consistent (closed-world behavioral correctness within the
frozen artifact cluster); the consistency-validator checks whether the surrounding perimeter — the
implementing story, BC citations, and index parity — is also correct (open-world structural
completeness). The adversary structurally cannot see story under-scoping because the story is
outside the frozen spec set; the consistency-validator is the correct agent for perimeter checks.
Without this perimeter gate, three implementation gaps (factory-lock shared functions, precompact-
flush identity-gate amendment, BC-7.07.001 re-anchoring) would not have been caught until TDD
failure, costing substantially more rework.
`[convergence-governance][perimeter-audit; two-axis-gate; adversary-vs-consistency-validator-scope; D-1124; S-17.05-under-scoping; wave-decomposition]`

The perimeter-audit-driven wave decomposition (D-1124) was closed by a 4-phase coordinated
cascade (D-1125): the 3 BLOCKS-CLOSURE gaps (Gap A factory-lock shared fns, Gap B precompact-
flush identity gate, Gap C BC-7.07.001 re-anchoring) were each remediated in dedicated
sub-commits (Phase B spec amendments, Phase C story authoring, Phase D index registration).
The POLICY 18 three-way parity discipline (frontmatter=catalog-row=blockquote) ensured hash
consistency across all three new story files simultaneously at cascade close.
`[convergence-governance][cascade-closure; 4-phase-burst; POLICY-18-parity; D-1125; wave-decomposition-complete]`

**L-BB-D1126-governance-pol14-co-impl-exception** — When a behavioral contract is co-implemented across multiple stories in the same wave (as BC-4.17.001 is split across S-17.06 PC2 and S-17.05 PC1/PC3/PC4/PC5), POL-14 auto-promotion on any single story merge would produce a false "active" status misrepresenting partial delivery as complete. The correct pattern is to hold the BC at `draft` until all co-implementing stories merge and the wave-integration gate passes. This exception must be explicitly codified at delivery time (D-1126 Part 2) with: (a) the BCs being held; (b) which stories co-implement them; (c) the specific trigger for promotion. Without explicit codification, a future session may auto-promote on the wrong merge boundary.
`[governance][POL-14; co-implemented-BC; wave-5; BC-4.17.001; S-17.06-S-17.05; D-1126]`

**L-BB-D1126-governance-autonomous-merge-policy-explicit** — The orchestrator had been implicitly assuming autonomous-merge authority for story PRs meeting BC-5.39.001 3-CLEAN + CI-green + diverse-model review criteria. When the harness surfaced a self-approval flag on PR #787, the human formally ratified the merge AND explicitly granted autonomous-merge authority for this session. Lesson: governance assumptions that are implicit become audit liabilities. Make the authority grant explicit and on-the-record at the moment the assumption surfaces as a flag. The flag itself is the prompt to codify.
`[governance][autonomous-merge-policy; self-approval-flag; explicit-ratification; D-1126; PR-787]`

**L-BB-S1705-P11-policy14-leg2-changelog-row-discipline** — When the pass-10 seal (S1705-P10-CLEAN-BURST) bumped story S-17.05 from v1.5 to v1.6 and updated the STORY-INDEX catalog row and blockquote, it did NOT add the corresponding `v1.6` row to the story's `## Changelog` table. The `validate-changelog-monotonicity` hook correctly blocked the story-writer on the next pass when attempting to add a v1.7 row without a preceding v1.6 row — requiring a backfill of both rows in the same burst. **Going-forward discipline:** Every seal burst that bumps a story version MUST add the Changelog table row in the same commit as the STORY-INDEX catalog row update. The STORY-INDEX update is not complete without the corresponding story-file Changelog row. No follow-up story required at this pre-convergence stage.
`[process-discipline][POLICY-14-leg-2; changelog-row; story-version-bump; seal-discipline; S-17.05; S1705-P11-FINDINGS-BURST; validate-changelog-monotonicity]`

**L-BB-D1127-batch-low-doc-findings-during-3clean** — During the S-17.05 local BC-5.39.001 3-CLEAN cascade, pass 12 returned CLEAN (zero MEDIUM+) with one LOW/documentary observation (F-P12-001: stale test-count tally sentence in §Red Gate prose). The human governance ruling (D-1127, 2026-08-28) establishes: LOW-only documentary findings during a local 3-CLEAN run are BATCHED and swept in a single finalization doc-sweep after 3-CLEAN is reached — NOT fixed mid-run. The rationale is the frozen-artifact-reset trap (L-EDP1-007/051/061): fixing a LOW doc item mid-run bumps the story version and input-hash, causing pass N+1 to be a fresh-context review of a newly-modified artifact rather than the converged implementation, producing spurious streak resets that prolong convergence indefinitely. The correct pattern: reach 3-CLEAN on the converged (frozen) artifact, then sweep all batched LOW doc items in one post-convergence pass with story-writer, then submit the PR. MEDIUM+ findings continue to reset the streak and require immediate in-scope fixes (unchanged). The finalization backlog MUST be anchored to a concrete future step with specific items listed (`finalization-doc-sweep.md`) to satisfy production-grade Rule 3's "attached to concrete future step" requirement.
`[convergence-governance][D-1127; batch-low-doc; frozen-artifact-reset-trap; L-EDP1-007/051/061; 3-CLEAN-discipline; finalization-doc-sweep; S-17.05]`

**L-BB-BC539005-banner-seal-discipline** — Every STATE.md edit that changes the line count MUST update the `N lines (wc-l)` claim in the SIZE BUDGET banner to match the actual newline count (BC-5.39.005 seal discipline). The `validate-state-structure` cargo-host tests `test_BC_5_39_005_f_p1_001_real_state_md_banner_wc_passes` and `test_BC_5_39_005_full_validation_against_real_state_md` read the live `.factory/STATE.md` from `origin/factory-artifacts` (mounted by CI via `git worktree add .factory origin/factory-artifacts` in the "Mount factory artifacts" step) and panic if the banner claim diverges from the actual newline count. The regression was introduced during the 2026-08-28 STATE.md rewrites v9.18→v9.25 when the pre-frontmatter SIZE BUDGET HTML comment was relocated to the document body but the dual-margin phrases (`margin from soft-target`, `margin from actual`) and the `N lines (wc-l)` claim were omitted from the new location. Remediated by commit `bab12dbc`. **Going-forward seal discipline (4 invariants):** (1) Before committing any STATE.md edit, run `wc -l .factory/STATE.md` and update the banner `N lines (wc-l...)` claim to match. (2) The banner MUST be inside an HTML comment block containing the literal string `STATE.md SIZE BUDGET`. (3) Both `margin from soft-target` and `margin from actual` are REQUIRED in the banner (D-446(c) dual-margin form). (4) The terminator character immediately after `wc-l` must NOT be an ASCII letter or digit — `)`, `;`, `.`, space are all valid terminators. The WASM hook (`validate-state-structure`) enforces invariants (2)-(4) at write-time; invariant (1) requires manual discipline by the state-manager.
`[seal-discipline][BC-5.39.005; banner-wc-l; dual-margin; D-446(c); CI-regression; bab12dbc; validate-state-structure; cargo-host-test]`

**L-BB-D1129-PG-CI-1-github-workflow-sibling-sweep** — S-17.05 delivery surfaced a CI-only failure: `ci.yml` still referenced the deleted `verify-state-timestamp-refresh.bats` test file after it was removed from the bats test suite during story implementation. The adversary (passes 12/13/14) and the TD-VSDD-060 sibling-sweep both missed this because neither exercise `.github/` workflow files as part of the standard sweep scope. **Going-forward discipline:** When a story deletes or renames a test file, the adversary rubric AND TD-VSDD-060 sibling-sweep MUST explicitly include `.github/workflows/ci.yml` (and any other workflow files that reference test paths) in the sweep. The workflow file is a load-bearing dependency of the test: removing the test without removing its CI reference causes a CI failure that cannot be caught locally. Rule: delete-test → grep `.github/` for the test path → remove/update the reference in the same commit.
`[process-gap][PG-CI-1; D-1129; ci.yml; deleted-test-file; TD-VSDD-060-sibling-sweep; github-workflow-reference; S-17.05]`

**L-BB-D1129-PG-CI-2-cross-platform-portability-discipline** — S-17.05 delivery surfaced multiple CI-only failures caused by platform-specific behavior not reproducible on the local macOS development environment: GNU-vs-BSD `date` flag differences (GNU `date -d` vs BSD `date -v`); Windows CRLF line endings causing self-match failures in output comparison tests; platform-specific path separators. The local adversary (passes 12/13/14) ran exclusively in the local macOS environment and never exercised the linux/windows matrix that CI uses. **Going-forward discipline for test authoring:** (1) Avoid GNU-only `date` flags — use POSIX `date` or Rust's `chrono` for timestamp arithmetic. (2) Use `str::lines()` (not `split('\n')`) to handle CRLF self-match in Rust tests. (3) When a test exercises platform-specific behavior (paths, line endings, date formats), add a platform-detect guard or use cross-platform abstractions. **Going-forward discipline for adversary rubric:** Add an explicit "portability" check pass: does the implementation use any GNU-only, macOS-only, or CRLF-sensitive patterns that the CI matrix would exercise differently?
`[process-gap][PG-CI-2; D-1129; cross-platform; GNU-date; CRLF; windows-ci; portability; str-lines; S-17.05; adversary-rubric]`

**L-BB-D1129-PG-CI-3-pr-manager-all-checks-completed-gate** — S-17.05 delivery surfaced a POLICY 22 violation: the pr-manager declared "all green" and proceeded with merge while some CI jobs were still PENDING (not yet completed). The pr-manager was monitoring a subset of the CI jobs (those visible in the immediate checks list) rather than waiting for ALL checks reported by `gh pr checks` / the `statusCheckRollup` to reach a terminal state (pass/fail/skip). **Going-forward gate discipline:** Before any pr-manager merge decision, the authoritative check is: `gh pr checks <PR-NUMBER>` — ALL checks must show terminal status (`pass`, `fail`, or `skipped`). Any check still `pending` or `in_progress` means the gate has NOT passed. The term "CI-green" means ALL checks terminal AND all required checks passing. Watching a subset (e.g., only the "Rust CI" job) is insufficient when the PR has other required checks (e.g., bats, clippy, Windows cross-compile). This discipline is a direct application of POLICY 22 and must be enforced mechanically, not by judgment.
`[process-gap][PG-CI-3; D-1129; POLICY-22; gh-pr-checks; statusCheckRollup; all-checks-completed; pr-manager-gate; S-17.05]`

**L-BB-D1142-TD-VSDD-060-symmetric-emission-sibling-sweep-miss** — S-25.01 adversary pass 6 (D-1141) fixed F-P6-001 by establishing "emit `marker.written` only after `write_indeterminate_marker` returns `Ok(())`, never before, never on `Err(_)`" for the marker-write callsite's POSITIVE creation-record event. The SAME callsite already had a second write-tied audit event — `emit_superseded_if_cross_pair`'s `marker.cleared(SUPERSEDED)` emission for the pair being overwritten — that was structurally identical in every way that mattered (same callsite, same write, same fabrication-on-failure risk) but was NOT swept into the v1.4 fix. It surfaced one full adversary pass later, at pass 9, as F-P9-001 (D-1142), fixed by the symmetric correction: `emit_superseded_if_cross_pair`'s call also moves inside the `Ok(())` arm, alongside `marker.written`. **Root cause of the miss:** TD-VSDD-060 sibling-sweep discipline is typically applied by searching for other CALLSITES of the same function or other USES of the same constant/identifier being changed — a textual/structural search. This miss was NOT a callsite-search gap; `emit_superseded_if_cross_pair` and `emit_marker_written` are different functions with different call signatures, so a grep-based sibling-sweep would not have connected them. The actual relationship was BEHAVIORAL: both are audit events tied to the outcome of the exact same fallible operation (`write_indeterminate_marker`), and both share the identical fabrication-on-write-failure defect class (an event asserting "the write happened" gets emitted even when the write did not happen). **Going-forward discipline (TD-VSDD-060 extension):** when a fix establishes an "emit only after operation X returns `Ok`" rule for one audit event tied to operation X, the adversary rubric and the implementer's own sibling-sweep MUST explicitly enumerate ALL audit events emitted at or near the SAME fallible operation's callsite — not just events sharing a function name or constant — and verify each one independently satisfies the same emit-after-success rule. A callsite that produces N audit events on a single fallible operation requires N independent applications of the rule, not one applied to the event the original finding happened to name.
`[process-gap][TD-VSDD-060; D-1141; D-1142; F-P6-001; F-P9-001; emit-after-success; sibling-sweep-miss; behavioral-not-textual; marker.written; marker.cleared-SUPERSEDED; ADR-048; S-25.01; write_indeterminate_marker]`

**L-BB-D1143-TD-VSDD-060-plus-TD-VSDD-059-timestamp-field-sibling-and-paper-coverage-miss** — S-25.01 adversary pass 10 (D-1143) found a two-layer defect that survived 9 prior adversary passes. Layer 1 (TD-VSDD-060 sibling-parity miss, code): `emit_indeterminate` (Event 8 `plugin.indeterminate`) and `emit_marker_cleared` (Event 9 `marker.cleared`) — the two newest dispatcher-native audit emitters added across the ADR-047/ADR-048 cascade — omitted the distinct `timestamp` wire field that all 7 PRE-EXISTING sibling emitters in `host/emit_event.rs` already carry, despite BC-3.08.001's wire-format table and VP-108's Property Statement both declaring `timestamp` mandatory since v1.0. Layer 2 (TD-VSDD-059 paper-coverage miss, verification-gap — the reason Layer 1 survived 9 passes): VP-108's own proof harness never asserted the `timestamp` field in any of its four `marker.cleared` emission-positive tests (Postconditions 1/2/3/5), even though the harness's own Property Statement declared it mandatory — a harness that "proves" 7 of 8 mandatory wire fields but silently never checks the 8th provides false confidence that is textually indistinguishable from full coverage. **Root cause of the miss (compounding of two independent disciplines):** the sibling-sweep gap (TD-VSDD-060) meant the code never got the field; the paper-coverage gap (TD-VSDD-059) meant nothing detected the code never got the field, because the harness that should have caught it was itself incomplete on the exact same dimension. Neither discipline alone would have caught this for 9 passes if the other had been sound — TD-VSDD-060 sibling-sweep would have caught the missing field at authoring time if performed; TD-VSDD-059 paper-coverage verification would have caught it on pass 1 if the harness asserted every field its own Property Statement declared mandatory. **Going-forward discipline:** (1) TD-VSDD-060 sibling-sweep for a NEW dispatcher-native audit emitter must diff its field-set against ALL existing emitters routing through the same host function family (`HostContext::emit_internal`/`emit_event`), not just emitters sharing a function name; (2) TD-VSDD-059 paper-coverage discipline requires the adversary and the spec author to cross-check the harness's actual assertions against the Property Statement's own claimed-mandatory field list every time a new mandatory field is added anywhere in the wire-format table — a harness silently passing while omitting one assertion out of eight is the specific failure mode this class targets.
`[process-gap][TD-VSDD-060][TD-VSDD-059][D-1143; F-P10-001; F-P10-002; timestamp; emit_indeterminate; emit_marker_cleared; VP-108; proof-harness-gap; sibling-parity; paper-coverage; S-25.01]`

**L-BB-D1144-POLICY9-VP-title-scope-change-must-sweep-both-arch-derived-views** — S-25.01 adversary pass 11 (D-1144) found F-P11-001 HIGH: VP-108's title/scope grew across passes 6 (write-path added), 9 (SUPERSEDED emission-point), and 10 (timestamp field) into "Marker Lifecycle Audited Events — Write and Clear Path Emission Correctness." VP-INDEX.md's own §Full Index and §Story Anchors rows — VP-108's authoring location and thus the first propagation target any state-manager burst reaches for — were updated correctly at each of those passes. But `verification-architecture.md` (§SS-01 Provable Properties Catalog) and `verification-coverage-matrix.md` (SS-01 module table) — the two ARCH-INDEX-registered architecture derived-view documents that ALSO carry a VP-108 row, independently of VP-INDEX — were never swept, and sat stale at the pre-write-path title ("marker.cleared Audited-Clear Event — Clear Path Emission Correctness") with an incomplete BC-anchor (missing BC-1.18.001 §PC4 and BC-3.08.001 Event 10) for three full passes' worth of scope growth before an adversary caught it. **Root cause:** the state-manager per-burst propagation habit (established across passes 2/3/6/9/10) treats "VP-INDEX §Full Index + §Story Anchors both updated same-burst" as the complete POLICY 9 sweep — and for a VP with no ARCH-INDEX-registered derived-view mirror, it would be. VP-108 is NOT such a VP: it has two additional independent mirror copies of its title/BC-anchor living outside VP-INDEX entirely, in files owned by a different POLICY (ARCH-INDEX §Document Map lists both as "Derived from VP-INDEX," but derivation is manual/architect-driven, not automatic — staleness is possible and, here, occurred). Pass 10's own burst-log Block 5 literal-shell attestation explicitly checked and confirmed "verification-architecture.md/verification-coverage-matrix.md confirmed NO textual change needed" — a real check that was CORRECT for pass 10's specific delta (a proof-harness-only change, no title/scope change) but whose passing result was then over-generalized by the NEXT burst's author as "these two files never need checking for VP-108," rather than "these two files needed checking THIS time and didn't need edits THIS time." **Going-forward discipline (POLICY 9 extension):** for any VP whose ARCH-INDEX §Document Map entry states "Derived from VP-INDEX" (currently `verification-architecture.md` and `verification-coverage-matrix.md`), a title/scope/BC-anchor change to that VP's VP-INDEX row is NOT complete until the identical title/BC-anchor string is verified (via literal grep, D-449(a)) to match in BOTH derived-view files — every single burst that changes the VP's title or BC-anchor, not just the burst that first adds the VP. A burst that confirms "no textual change needed" this time does not exempt future title/scope-changing bursts from re-checking; each title/BC-anchor-changing burst is its own independent propagation obligation. Secondary finding (this same burst): the ARCH-INDEX §Document Map version-pointer cells for these two files (a SEPARATE tracking mechanism from the row content itself) had independently drifted stale since v1.13/v1.11 while the actual files advanced to v1.16/v1.14 — a second, compounding propagation gap on top of F-P11-001's row-content staleness, showing that even the META-level "which version is this file at" pointer is subject to the same sibling-sweep discipline gap.
`[process-gap][POLICY-9][D-1144; F-P11-001; VP-108; verification-architecture.md; verification-coverage-matrix.md; ARCH-INDEX; Document-Map; derived-view; title-scope-propagation; sibling-sweep-miss; S-25.01]`

**L-BB-D1145-VP-postcondition-without-test-plus-phantom-proof-harness-anchors** [codified][process-gap] — S-25.01 adversary pass 12 (D-1145) found F-P12-001 MEDIUM: two coupled roots surviving multiple prior passes. Root (a): VP-108's Postcondition 8 — the F-P9-001 negative-control regression requirement that a cross-pair marker overwrite whose write fails must emit NEITHER `marker.cleared(SUPERSEDED)` NOR `marker.written` — was MANDATED by the spec since v1.4 but had NO implementing test anywhere in the crate; the proof-harness skeleton's cited test name was never authored, a TD-VSDD-059-class paper-coverage gap (same family as L-BB-D1143). This gap was compounded by the emission-decision logic itself being duplicated verbatim at two callsites (`execute_tier` and `spawn_async_plugin`), a TD-VSDD-060-class sibling-duplication risk: a future edit fixing one callsite's emission rule could silently miss the other, re-introducing the exact F-P6-001/F-P9-001 defect class ADR-048 exists to close. Root (b): the VP-108 proof-harness skeleton's cited test fn names for PC1 through PC8 were, on grep-verification against the real crate, almost entirely PHANTOM — plausible-sounding invented names (e.g. `test_revalidated_clear_emits_marker_cleared`) that do not exist anywhere in `indeterminate_marker.rs`, `executor.rs`, or `tests/marker_integration.rs`; the SAME class of defect an earlier pass already found and fixed for a single phantom FILE reference (v1.6), now found to be systemic across the skeleton's individual test-name anchors (v1.7 fix). **Going-forward discipline:** (1) When a VP mandates a postcondition via a proof-harness skeleton, the skeleton's cited test name is not evidence the test exists — every skeleton anchor must be grep-verified against the real crate at the time it is written AND re-verified whenever the postcondition is amended; a plausible name is not a real one. (2) When a fix extracts shared emission/decision logic to close a sibling-duplication risk (TD-VSDD-060), the extraction itself becomes the natural place to attach the previously-missing regression test — pass a fallible operation's `Result` as a parameter (rather than having the helper perform the operation itself) so failure-path tests do not require forcing a real failure condition. (3) A single mis-anchor finding for one postcondition (PC8, v1.6) should trigger an immediate CLASS-WIDE grep-audit of every OTHER postcondition's proof-harness anchor in the same VP, not a postcondition-by-postcondition trickle — v1.7 closed PC1-PC5 in one pass specifically because the fix was scoped as "the SAME defect class, applied class-wide" rather than "just fix PC8."
`[process-gap][TD-VSDD-059][TD-VSDD-060][D-1145; F-P12-001; VP-108; PC8; phantom-test-names; emit_write_tied_audit_events; sibling-duplication; proof-harness-anchor-accuracy; class-wide-audit; S-25.01]`

**L-BB-D1147-emitter-conformance-tests-must-assert-excluded-field-absence-not-only-mandatory-field-presence** [codified][process-gap] — S-25.01 adversary pass 14 (D-1147) found F-P14-001 MEDIUM: `emit_indeterminate` (Event 8 `plugin.indeterminate`, `executor.rs`) called `.with_plugin_version(&base_ctx.plugin_version)`, but BC-3.08.001 §Common Fields explicitly states `plugin_version` is NOT emitted by Events 1, 4, 5, 7, and 8 — the sibling emitters `emit_marker_cleared`/`emit_marker_written` correctly omit the call. This is the MIRROR-IMAGE defect class to F-P10-001 (D-1143, L-BB-D1143): that pass found a MISSING mandatory field (`timestamp`) on the same emitter family; this pass finds an EXTRA excluded field on the same family. **Root cause:** the existing Event 8 conformance test asserted only that the 8 mandatory fields were PRESENT — it never asserted that fields the spec explicitly EXCLUDES were ABSENT. A wire-format contract with both a mandatory-field list AND an excluded-field list is only fully characterized by a test that checks BOTH directions; a presence-only test cannot detect an over-emission defect no matter how thorough its mandatory-field coverage is, because "field X is present" and "field X is absent" are logically independent assertions — proving one says nothing about the other. This is structurally the SAME root-cause shape as TD-VSDD-059 paper-coverage gaps (a proof that is incomplete relative to what the spec actually claims), but on the OPPOSITE side of the field list: TD-VSDD-059/L-BB-D1143 was about an incomplete MANDATORY-field assertion; this is about a MISSING EXCLUDED-field assertion. The secondary root is a TD-VSDD-060 sibling-emitter angle: `emit_indeterminate` is the newest emitter in the ADR-047/ADR-048 dispatcher-native-events lineage (added same burst as the very `timestamp` fix that closed F-P10-001), and it was authored by extending an OLDER emitter's field-building call chain without checking which of those calls the wire-format table for THIS SPECIFIC event actually sanctions — `plugin_version` is a common builder-chain call used by other events, but Event 8's contract explicitly excludes it. **Going-forward discipline:** (1) every emitter conformance test for an event with BOTH a mandatory-field list and an explicit excluded-field list must assert BOTH directions — mandatory fields present AND excluded fields absent — a full-closure characterization, not a presence-only spot-check; asserting presence of N mandatory fields provides zero information about whether M excluded fields leaked onto the wire. (2) When authoring a NEW dispatcher-native emitter by extending/copying an existing builder-chain pattern, the wire-format table's excluded-field list (not just its mandatory-field list) must be checked line-by-line against every builder call inherited from the pattern being extended — copying a working emitter's structure copies its field-set too, mandatory and excluded fields alike, unless each call is individually re-verified against the NEW event's own contract row.
`[process-gap][TD-VSDD-060][D-1143; D-1147; F-P10-001; F-P14-001; emit_indeterminate; plugin_version; excluded-field; full-closure-characterization; mandatory-vs-excluded; emitter-conformance-test; BC-3.08.001; Common-Fields; S-25.01]`

**L-BB-D1148-VP-postcondition-class-wide-sweep-must-cover-ALL-postconditions-not-only-the-named-ones** [codified][process-gap][regression] — S-25.01 adversary pass 15 (D-1148) found F-P15-001 HIGH: VP-108 Postcondition 1 (REVALIDATED clear)'s Property Statement paragraph still described the PRE-CORRECTION trace_id-source ("`<current trace_id>`"), emission-locus ("the gate plugin deletes...and emits"), and trigger ("PASS outcome via `evaluate_gate`") — all three of which were corrected for PC2/PC3 at pass 2 (F-P2-002, D-1139) and for PC5 at pass 3 (F-P3-001, D-1140), but never applied to PC1. The stale wording survived thirteen subsequent adversary passes (3 through 14 inclusive) undetected because none of those passes happened to re-read PC1's Property Statement paragraph against the corrected mechanism — the code and PC1's own implementing test were ALREADY CORRECT throughout, so no test failure or code-behavior symptom ever surfaced the drift; only a fresh-context adversary directly comparing PC1's prose against BC-3.08.001 Event 9 and the sibling postconditions' prose caught it. **Root cause:** F-P2-002 and F-P3-001 were each scoped and fixed as "correct the postcondition(s) the finding named" (PC2/PC3 at pass 2; PC5, newly added, at pass 3) rather than "this VP's property-statement error class — trace_id source, emission locus, trigger — applies uniformly across every clear-mode postcondition describing a `marker.cleared` emission, so re-audit ALL of them." PC1 was not named by either finding because PC1 pre-dated both corrections and was not the one being edited in either burst; a same-VP sibling postcondition was invisible to a fix scoped by "which postcondition did the finding cite," rather than by "which postconditions share this VP's underlying wire-emission mechanism." This is a specific instance of the general TD-VSDD-060 sibling-sweep discipline, but one level higher than callsite or emitter siblings (L-BB-D1142, L-BB-D1147): here the siblings are POSTCONDITIONS WITHIN THE SAME SPEC DOCUMENT describing the same underlying mechanism from different clear-mode angles, not distinct call sites or distinct emitter functions in code. **Going-forward discipline (TD-VSDD-060 extension, spec-postcondition class):** when a fix corrects a property-statement error (a wrong source, wrong locus, or wrong trigger for an emitted event) in ANY postcondition of a VP, the SAME burst must re-read EVERY OTHER postcondition of that VP that describes an emission of the same event family (here, all `marker.cleared` clear-mode postconditions: PC1 REVALIDATED, PC2 TTL_EXPIRED, PC3 OPERATOR_OVERRIDE, PC5 SUPERSEDED) for the identical error class — not only the postcondition(s) the triggering finding happened to cite. A fix that corrects PC2/PC3/PC5 but silently leaves PC1 unaudited has not actually closed the error class; it has narrowed it by three-quarters and left the remainder to be found by chance on a later pass. This burst's own sibling-sweep of PC2–PC8 (architect commit `90675c7d`) executed exactly this discipline retroactively — the going-forward obligation is to perform it PROACTIVELY, in the SAME burst as the first instance of the error class, not reactively three passes later when the un-swept sibling happens to be the one an adversary reads next.
`[process-gap][codified][regression][TD-VSDD-060][D-1139; D-1140; D-1148; F-P2-002; F-P3-001; F-P15-001; VP-108; PC1; PC2; PC3; PC5; REVALIDATED; trace_id-source; emission-locus; trigger; partial-fix-propagation; postcondition-sibling-sweep; class-wide-audit; S-25.01]`

**L-BB-D1149-last-amended-write-path-grows-unbounded-sidecar-split-is-mitigation-not-cure** [codified][process-gap][one-time-exception] — D-1149's one-time POL-3 exception surgery split the `last_amended:` mega-line in 5 files (STORY-INDEX.md, BC-INDEX.md, ARCH-INDEX.md, VP-INDEX.md, STATE.md itself) into a slim current-entry-plus-pointer, moving each removed tail verbatim into a sibling `*-amendment-history.md` sidecar, because the largest line (STORY-INDEX.md, 323,499 chars) was exhausting the bash-adapter validators' WASM fuel budget and causing hook hangs — the previously-DEFERRED TASK-2 from the 2026-08-31 `BC-INDEX-STORY-INDEX-LAST-AMENDED-COMPACTION-AND-D-PARITY` burst, deferred at the time because Edit/Write cannot perform a targeted in-place replacement inside a single physical line of that size. **Root cause:** the `last_amended` write-path is architecturally append-only-by-wrapping — every burst PREPENDS a new dated entry and wraps the ENTIRE prior value as a `` [Prior: ...] `` tail, so the line's length is monotonically non-decreasing across a long-running cycle with no natural cap; a mega-line is not a one-off accident but the GUARANTEED eventual state of any sufficiently long-running cycle under this write-path. **This is a mitigation, not a cure:** the sidecar split resets the line to near-zero length at the moment of the surgery, but the SAME unbounded-growth write-path resumes immediately afterward — the very next burst's new dated entry, wrapping the just-slimmed current entry as its own `[Prior: ...]`, begins the identical growth curve again from a smaller base. Left unaddressed, this surgery will need to be repeated indefinitely (per-cycle rotation), each time consuming a state-manager burst and a one-time POL-3 exception request, which does not scale and re-opens the hook-fuel-exhaustion risk between rotations. **Going-forward discipline:** the durable fix is a structured, non-string-concatenation `changelog:` write-path (an actual YAML/array field appended to, not a scalar string re-wrapped) — already anchored as a follow-up at S-15.03 PRIORITY-A per D-448(b)/D-449(d)(iii)'s structured-changelog-migration deferral for STORY-INDEX; this surgery's finding extends that same anchor to cover BC-INDEX/ARCH-INDEX/VP-INDEX/STATE.md's `last_amended` fields as well, since all 5 share the identical wrap-and-prepend defect. Until that lands, expect these 5 lines to regrow steadily; do not treat a future large `last_amended` line as a fresh, unexplained defect — it is the SAME root cause recurring, and `git -C .factory log -p` remains the authoritative full historical archive regardless of the live line's length. Incidental, orthogonal finding surfaced during verification (not part of this lesson's root cause, and not fixed by this surgery): 3 of the 5 files' CURRENT `last_amended` entries independently fail strict-YAML parsing due to unescaped embedded quotes in narrative prose — this is the pre-existing, already-tracked `[D-1144]` Drift Item, not a consequence of the wrap-and-prepend growth pattern this lesson addresses.
`[process-gap][codified][one-time-exception][architectural][D-1149; D-1136; last_amended; write-path; unbounded-growth; wrap-and-prepend; sidecar-rotation; mitigation-not-cure; S-15.03; structured-changelog; POL-3-exception; STORY-INDEX; BC-INDEX; ARCH-INDEX; VP-INDEX; STATE.md]`

**L-BB-D1152-pr-manager-runaway-subagent-spawning-and-shared-worktree-clobber** [process-gap] — S-15.03's PR #805 delivery surfaced a coordination-discipline gap in pr-manager's CI-monitoring and fix-dispatch behavior: during the CI-green push, pr-manager spawned roughly a dozen `github-ops` CI-watcher agents (rather than one), each independently polling the same PR's check-run status, and each poll that observed a still-red or newly-red check re-notified the orchestrator — producing repeated, redundant status noise rather than a single coherent CI-outcome signal. Concurrently with this watcher sprawl, pr-manager ALSO spawned a second fix-agent (a `test-writer` dispatched to `fix-windows-path-assertion`) into the SAME shared worktree an existing implementer fix-agent already owned, without checking for an in-flight owner first; the two agents' concurrent edits collided, and the second agent's write clobbered the first agent's in-progress fix. **Root cause:** pr-manager's CI-monitoring loop has no single-watcher invariant — each polling cycle (or each newly-surfaced red check) was treated as independently dispatchable, rather than routed through one long-lived watcher that owns the full monitoring lifecycle for a given PR. Separately, pr-manager's fix-dispatch logic has no worktree-ownership check — it can dispatch a new fix-agent into a worktree without first verifying no other agent already holds it, which is the identical single-writer violation TD-FACTORY-HOOK-BYPASS-001/POL-3 guards against for `.factory/`, just unenforced for product-branch worktrees. **Going-forward discipline:** (1) bound pr-manager's CI-monitoring to exactly ONE watcher per PR for the lifetime of that PR's CI-green wait — the watcher owns polling cadence and is the sole source of CI-status notifications to the orchestrator, collapsing what would otherwise be N notifications into 1 coherent state transition per check-run change; (2) before dispatching any fix-agent into a worktree, pr-manager MUST check whether that worktree already has an in-flight owning agent (a single-writer-per-worktree invariant) and MUST NOT dispatch a second fix-agent into an already-owned worktree — if a second fix is needed concurrently, it must target a DIFFERENT worktree or wait for the first agent to complete/release. Anchored to E-12 (Engine Governance — the self-improvement epic S-15.03 itself belongs to) for a follow-up pr-manager hardening story; no story ID allocated yet, this lesson is the intake trigger. Not fixed this burst (process/prompt-level fix, outside state-manager's domain — routes to pr-manager agent-prompt authorship, not `.factory/` bookkeeping).
`[process-gap][pr-manager][ci-watcher-sprawl][single-writer-per-worktree][worktree-clobber][S-15.03][PR-805][E-12][fix-dispatch]`

**L-BB-D1152-validate-factory-path-staging-branch-detection-uses-session-cwd-not-bash-tool-cwd** [process-gap] — During S-15.03's PR #805 delivery, `validate-factory-path-staging` (the PreToolUse guard that blocks `.factory/`-path `git add`/`git stage` on product branches) produced a false-positive `FactoryPathOnProductBranch` block on a glob `git add` issued from INSIDE the `feature/S-15.03` worktree, while the orchestrating session's own cwd was still the main repo checked out on `release/v1.0.0-rc.24`. The hook's own doc comments (`crates/hook-plugins/validate-factory-path-staging/src/lib.rs`, `find_factory_class_target`) already implement TARGET-AWARE branch detection (BC-4.16.001 v1.7 Invariant 6, closing an earlier finding F-P5-001) for the case where the command's own tokens name an explicit `.factory`-class path argument or a `-C <dir>`/`--work-tree=<dir>` git option — in that case it correctly runs `git -C <target> branch --show-current` against the named target. But the fallback arm (no explicit target found in the command's tokens, e.g. a bare `git add -A` with no literal path argument) falls back to a plain `git branch --show-current` with no `-C`, which resolves against whatever the WASM host process considers its OWN working directory — and that is the session's ORIGINAL cwd at dispatcher-process start, not the Bash tool's actual PER-CALL `cwd` (which changes every time an agent's shell operates inside a different worktree, e.g. `.worktrees/S-15.03`). **Root cause:** the hook has two branch-detection code paths with different fidelity — the explicit-target path is worktree-aware, the fallback path is NOT, and the fallback path's notion of "current directory" is the dispatcher process's own cwd rather than the specific Bash tool invocation's cwd, which the dispatcher payload does carry (precedent: BC-5.41.003's `git_context` injected host-side into `payload.extra` for commit-SHA corroboration — the same host-side plumbing pattern could carry the invoking Bash call's cwd). **Going-forward discipline:** extend `find_factory_class_target` (or add a parallel fallback path) to consult the Bash tool's own per-call cwd from `payload.extra`/`git_context` BEFORE falling back to a bare `git branch --show-current`, so that a bare `git add -A` issued from inside a feature worktree resolves that worktree's OWN branch rather than the session's original checkout. Anchored to `crates/hook-plugins/validate-factory-path-staging/src/lib.rs` for a devops-engineer/architect fix; not fixed this burst (source-code hook-plugin change, outside state-manager's `.factory/`-only tool access — routed per the Agent Routing Table to devops-engineer, with architect adjudication on the payload-plumbing extension if `git_context` needs a new field).
`[process-gap][validate-factory-path-staging][find_factory_class_target][branch-detection][cwd-fallback][worktree-false-positive][BC-4.16.001][S-15.03][PR-805]`

**L-BB-D1156-finalization-doc-sweep-batched-deferrals-need-a-terminal-disposition-not-a-carry-forward** [codified][process-gap] — S-25.01's post-3-CLEAN finalization-doc-sweep (D-1156) closed out 11 batched non-blocking items accumulated across passes 1, 13, 16, 17, and 18 of the LOCAL adversary cascade. Two of those items (`[process-gap]` registry-comment-lint and O-P16-1) and two more surfaced only at the final convergence pass (O-P17-001, O-P18-001) were not fixable in-scope — they were either genuine process/tooling gaps outside `.factory/`-only state-manager scope, or (for O-P18-001) a project-wide spec-wording question requiring human Direction selection before any spec amendment could be authored. **Root cause of the pattern this lesson closes:** a batched-deferral discipline (D-1127) that only specifies WHEN to sweep (after 3-CLEAN, before the PR) without also specifying WHAT a swept-but-unfixable item's terminal state must look like risks silently re-deferring the same item indefinitely — "sweep after 3-CLEAN" is not itself a disposition, and a sweep that merely re-lists an item as "still open" produces no forward progress and no audit trail of WHY it wasn't fixed. **Going-forward discipline (extends production-grade Canonical Principle Rule 3):** every item that survives a finalization-doc-sweep without an in-scope code/spec fix MUST leave the sweep with exactly one of four terminal states, never a fifth "still open, sweep again later": (1) RESOLVED (an actual commit fixed it — cite the SHA); (2) RESOLVED-already-fixed (a grep/read confirms a prior unrelated commit already closed it — cite that commit); (3) ACCEPTED won't-fix (a concrete spec/design justification is recorded, not just "seems fine"); (4) DEFERRED to a NAMED future story or STATE.md Drift Item with a CONCRETE anchor (an epic, a devops/architect/product-owner routing, and — for anything with a precondition, like O-P18-001's human Direction selection — the precondition itself stated explicitly) — never a bare "OPEN, sweep later" with no anchor. This burst applied states (1)-(4) to all 11 items and additionally verified, via a literal `git diff --stat` against the actual finalization commits, that two OLDER pre-existing Drift Items ([D-1146] F-P13-001, [D-1141] the AC-021-025 Red Gate stub gap) — themselves anchored to "the S-25.01 finalization-doc-sweep" by earlier passes — were NOT touched by this sweep's actual commits, and recorded them as explicitly, honestly OPEN rather than silently claiming "sweep complete" implied their resolution too. **Secondary discipline:** when a deferred item's follow-up work requires a human decision before it can even be scoped (Direction A vs B vs hybrid for O-P18-001), the deferral's anchor text must name the precondition itself, not just "route to architect" — otherwise a future session re-reading the Drift Item cannot tell whether it is actionable now or is blocked on an upstream human decision.
`[process-gap][codified][D-1156][D-1127][D-1146][D-1141][finalization-doc-sweep][batched-deferral][terminal-disposition][concrete-anchor][S-25.01][production-grade-rule-3][O-P18-001][precondition]`

**L-BB-D1161-f1-delta-analysis-must-verify-cited-bc-anchors-not-trust-draft-framing** [codified][process-gap] — S-25.04's original draft (registered under D-1157, PENDING-BC) inherited its technical framing verbatim from ADR-047 §Decision 8a's "Known Gap" note, which loosely described the recommended companion validator as covering "artifact write content." That loose framing implicitly pointed toward the wrong BC family and the wrong validator model: it read as though the companion validator would extend or pair with the BC-1.18.xxx/SS-01 marker-write-path contracts, when the actual authoritative behavior it needed to mirror was `validate-factory-path-staging`'s own contract, **BC-4.16.001** (SS-04, CAP-034) — a narrow git-staging exclusivity guard, not a content validator. Because the draft's Context/Narrative sections were authored by carrying that ADR framing forward rather than re-deriving it from BC-4.16.001's actual text, the story's BC anchors were headed toward the wrong family before any BC authorship had occurred. **Root cause:** F1 delta analysis is specified to determine "impact boundary, affected specs/stories/tests, and regression risk" from the triggering artifact (here, ADR-047's Known Gap note) — but a note like this is itself narrative summary, one hop removed from the actual behavioral contract it describes; treating the summary as ground truth propagates any imprecision in the summary directly into the new story's spec anchors. This is the SAME class of error TD-VSDD-059 (paper-fix detection) and TD-VSDD-060 (sibling-site sweep) guard against for CODE changes, applied one layer earlier — at SPEC-anchor-selection time, before any code or even BC text exists. **Going-forward discipline:** F1 delta analysis MUST verify a story's cited BC anchors against the authoritative BC's ACTUAL behavior (read the real BC-S.SS.NNN file's Description/Preconditions/Postconditions, not just the narrative note that recommended it) BEFORE any BC authorship or story-context drafting proceeds from that citation. When the F1 analysis surfaces a premise correction of this kind (as it did here — re-anchoring from the implied BC-1.18.xxx/SS-01 family to the real BC-4.16.001/SS-04 family), the correction must propagate to every downstream artifact in the SAME activation burst: the story's Narrative/Context, the newly-authored companion BC's `subsystem`/`capability` fields, and the originating ADR's own "Known Gap" note (fixed here as ADR-047 v1.5's erratum) — not just the new BC's own text, or the premise-error simply moves one hop further downstream instead of being closed. Caught and corrected in-scope this burst (D-1161); no code or already-shipped spec content was affected (S-25.01 remains fully unaffected — this was a pre-BC-authorship correction on a still-draft follow-up story).
`[process-gap][codified][D-1161][D-1157][ADR-047][BC-4.16.001][BC-4.16.002][S-25.04][F1-delta-analysis][premise-verification][TD-VSDD-059][TD-VSDD-060][spec-anchor-selection]`

**L-BB-D1163-scan-max-d-nnn-word-boundary-false-positive** [codified] — `validate-dispatch-advance`'s `scan_max_d_nnn` helper (a `D-<digits>` reference scanner used to detect stale `current_step` decision cites) matched the bare digit sequence "2026" inside the unrelated literal string "RC25-RELEASED-2026-09-04" as if it were decision `D-2026`, and then flagged `current_step`'s genuine, correct `D-1162` cite as stale because `2026 > 1162`. The same unguarded-substring class also mis-read `TD-VSDD-053` as `D-053`. Surfaced when branch-protection enablement on `develop` was attempted (PR #813) and made this pre-existing false positive a REAL, blocking CI red across `cargo-host` + `build-dispatcher` for the first time — it had previously been silently tolerated via the project's own historical merge-over-red pattern (the same class of gap D-1130's PR-MANAGER-MERGE-OVER-RED item records). **Root cause:** the scanner matched `D-` (or a bare 4-digit run) anywhere in a line without requiring a LEFT WORD BOUNDARY — i.e., without checking that the character immediately preceding the match is not itself alphanumeric — so any date string, semantic-version fragment, or unrelated ID scheme sharing the digit shape of a decision number was indistinguishable from a real `D-NNN` citation. **Going-forward discipline:** every `D-<digits>` reference scanner in this project (and by extension any similarly-shaped ID scanner — `TD-VSDD-NNN`, `BC-S.SS.NNN`, `VP-NNN`) MUST require a left word boundary (reject a match whose preceding character is alphanumeric) before treating a digit run as a decision-number citation; this was fixed in `validate-dispatch-advance/src/lib.rs` with 3 new Red Gate tests (PR #813, `5e009dc0`). This lesson generalizes: a required-status-checks gate (branch protection) surfaces pre-existing masked defects that a merge-over-red culture had been silently absorbing — enabling the gate is itself a forcing function for real fixes, not merely bureaucratic overhead.
`[codified][D-1163][scan_max_d_nnn][validate-dispatch-advance][word-boundary][false-positive][branch-protection][PR-813][merge-over-red][D-1130]`

**L-BB-D1163-stale-committed-wasm-must-be-rebuilt-from-source** [codified] — During PR #814's review, pr-reviewer's fresh-eyes diff review caught that the committed `plugins/vsdd-factory/hook-plugins/validate-factory-path-staged.wasm` binary in the feature branch had gone stale relative to its own crate's Rust source — 3 commits' worth of source changes (including the PC6 fail-open test additions) had landed without a corresponding wasm rebuild, so the bundled binary a hook consumer would actually load did not reflect the latest source behavior. Fixed via a dedicated rebuild-and-recommit commit (`79252d38`, "build(S-25.04): rebuild validate-factory-path-staged.wasm from current source (was 3 commits stale)") as the LAST commit before merge. **Root cause:** this project's hook-plugin crates are dual-artifact (Rust source in `crates/hook-plugins/*/` + a committed compiled `.wasm` binary in `plugins/vsdd-factory/hook-plugins/`), and nothing in the implementer workflow or CI enforces that the committed binary is rebuilt whenever its source changes — an implementer can edit `src/lib.rs`, run `cargo test` (which exercises the Rust source directly, not the compiled wasm), and never notice the bundled artifact drifted. This is the SAME class of defect TD-VSDD-059 (paper-fix detection) guards against for narrative claims, applied to a BINARY artifact instead of prose: the source said one thing, the shipped artifact said another, and nothing but a human fresh-eyes diff review caught it. **Going-forward discipline:** (1) rebuild + recommit the `.wasm` binary as the LAST implementer step before opening any PR that touches a hook-plugin crate's Rust source (`cargo build --target wasm32-wasip1 --release -p <crate>` or the project's equivalent build invocation, then copy the output into `plugins/vsdd-factory/hook-plugins/<name>.wasm`); (2) the durable fix is a CI gate that rebuilds each hook-plugin crate from source and asserts the freshly-built wasm is byte-identical (or at minimum, hash-identical modulo non-deterministic build metadata) to the committed one — anchored as a candidate follow-up (no story ID allocated yet; batches naturally with the existing `bundle_orphan_check`/`T-009` CI-hardening lineage from PR #813).
`[codified][D-1163][stale-wasm][hook-plugin][dual-artifact][rebuild-discipline][pr-reviewer][PR-814][TD-VSDD-059][ci-gate-candidate]`

**L-BB-D1163-pr-manager-nested-subagent-sprawl-and-hang** [codified][process-gap] — During PR #814's delivery, pr-manager (itself a subagent dispatched by the orchestrator) spawned approximately 7 further teammates one level deeper — 2 github-ops CI-watchers, 2 security-reviewer instances, and multiple pr-reviewer review cycles — several of which hung and never returned control, stalling the delivery until the orchestrator intervened directly. This is a RECURRENCE of the identical coordination-discipline gap D-1152 already recorded for S-15.03's PR #805 (CI-watcher sprawl + shared-worktree clobber) — the same root cause (pr-manager treating each new need as independently dispatchable rather than routed through a bounded, long-lived set of owned watchers) resurfaced on a completely different story, confirming it is systemic to pr-manager's dispatch behavior rather than a one-off. **New observation this occurrence:** the failure mode this time was not just redundant notification noise (D-1152's original finding) but outright non-completion — nested sub-agent dispatch (a subagent spawning further subagents, 2 levels deep from the orchestrator) does not reliably complete in this environment; teammates spawned by a spawned agent are more prone to silently hanging than teammates spawned directly by the orchestrator. **Going-forward discipline (extends D-1152's remediation with a NEW structural constraint):** the ORCHESTRATOR — not pr-manager — should dispatch security-reviewer, pr-reviewer, and the CI-watch DIRECTLY (one level deep from the orchestrator), rather than nesting them under pr-manager as a second level of spawning; pr-manager's role narrows to coordinating findings/triage/merge-readiness rather than itself spawning the reviewer agents. CI-monitoring stays bounded to exactly ONE watcher per PR (D-1152's original invariant, re-affirmed, still correct). This is a process/prompt-level change to the pr-manager and orchestrator agent definitions, outside state-manager's `.factory/`-only scope — not fixed this burst; anchored to the same E-12 follow-up story class D-1152 already opened (no story ID allocated yet), with this occurrence as reinforcing evidence that the fix should land in the orchestrator's own dispatch pattern, not solely in pr-manager's prompt.
`[codified][process-gap][D-1163][D-1152][pr-manager][orchestrator][nested-subagent][ci-watcher-sprawl][subagent-hang][recurrence][PR-814]`

**L-BB-D1164-self-referential-narrative-literal-defect-class** [codified] — `validate-dispatch-advance`'s `scan_max_d_nnn` false-positive (D-1163, fixed on `develop` for the word-INTERNAL-embedding sub-class only) had a second, adjacent sub-class that PR #813 did not close: a well-formed, correctly word-bounded quoted/prose citation of a decision number, appearing inside STATE.md's OWN descriptive text about a DIFFERENT, unrelated decision, was still mis-read as a real decision cite. This became concretely dangerous the moment STATE.md's own narrative needed to DESCRIBE the D-1163 bug — the description itself, written in prose, contained the exact decision-number-shaped substring the scanner was designed to detect, and the scanner could not distinguish "this text IS a decision citation" from "this text is DESCRIBING a decision citation as an example." The bug's own bug-report text could re-trigger the bug. **Root cause:** a lexical scanner operating over unstructured narrative prose has no notion of quotation, exemplification, or reported speech — every scanner of this shape (any `ID-<digits>`-pattern detector reading free-text commentary rather than a structured field) is structurally exposed to this class whenever the artifact it scans is also the artifact used to document defects in that same scanner. **Going-forward discipline:** any scanner whose job is "find the latest/highest ID actually recorded" should prefer a STRUCTURAL source (a table's first column, a frontmatter field, a dedicated index) over a LEXICAL source (grep across free-text narrative) wherever a structural source exists — `scan_max_decision_log_id`'s structured `## Decisions Log` first-cell scan (BC-5.39.006 v1.8, PR #815) is the durable fix; a purely lexical scanner, however well word-boundary-guarded, can always be defeated by a sufficiently well-formed narrative-literal citation of the exact shape it is looking for, because well-formedness is precisely what a word-boundary guard checks for.
`[codified][D-1164][self-referential][narrative-literal][scan_max_d_nnn][scan_max_decision_log_id][validate-dispatch-advance][BC-5.39.006][PR-815][PR-813][D-1163][structural-vs-lexical]`

**L-BB-D1164-word-boundary-fix-incomplete-for-adjacent-narrative-literal-subclass** [codified][process-gap] — PR #813/D-1163's word-boundary guard (require a non-alphanumeric character immediately before a `D-<digits>` match) was validated against the specific failure it was written to fix (a bare digit run embedded inside another token, e.g. a date fragment embedded in an all-caps status label) and closed CI red for that vector. It did NOT close the narrative-literal vector — a correctly word-bounded, well-formed quoted citation of a decision number inside unrelated prose — because that vector was never a WORD-BOUNDARY failure in the first place; it satisfies the word-boundary check perfectly and is textually indistinguishable from a genuine citation. The two sub-classes look similar (both present as "a `D-<digits>`-shaped string that isn't really the latest decision") but have different, non-overlapping root causes, and a fix validated against one does not imply coverage of the other. **This recurred to put `develop` CI red a second time** (the narrative-literal sub-class), on the very artifact (STATE.md) whose own bookkeeping about the FIRST fix supplied the triggering text. **Going-forward discipline (extends TD-VSDD-059 paper-fix detection to fix-COMPLETENESS, not just fix-EXISTENCE):** when a defect report describes a scanner/matcher bug by giving one concrete failing example, the fix author and the adversary reviewing it must independently enumerate the DISTINCT root-cause sub-classes that could produce a superficially similar symptom (not just verify the one given example now passes) — a single passing regression test for the reported example is necessary but not sufficient evidence the defect CLASS is closed. This generalizes TD-VSDD-060 (sibling-site sweep, for code-structure siblings) to symptom-siblings: report-siblings that share a surface presentation but differ in mechanism.
`[codified][process-gap][D-1164][D-1163][PR-815][PR-813][word-boundary][narrative-literal][TD-VSDD-059][TD-VSDD-060][fix-completeness][symptom-sibling][scan_max_d_nnn]`

**L-BB-D1164-structural-position-beats-lexical-heuristics-for-latest-decision** [codified] — The durable fix PR #815 shipped (`scan_max_decision_log_id`, BC-5.39.006 v1.8) works not by patching the lexical scanner's heuristics further, but by changing WHERE the answer is read from: the `## Decisions Log` table's first-column cell of its bottommost/most-recent row IS the authoritative structural record of "what is the latest codified decision" — state-manager's own append protocol guarantees this position is kept current on every burst. A lexical `D-<digits>` grep over free-text narrative, no matter how many word-boundary/emphasis/separator-row special cases it accumulates, is only ever approximating that same fact from evidence the narrative happens to leak, and remains permanently vulnerable to the next unanticipated narrative shape (this cycle alone: word-internal embedding, then narrative-literal citation, then emphasis-decorated cells and GFM separator rows as adjacent hardening). **Going-forward discipline:** when a "find the current/latest/highest recorded X" scanner exists over an artifact this project itself governs the structure of (a table, a frontmatter field, an index), prefer reading the STRUCTURAL position over inferring it lexically — this is a general instance of "verify structural invariants directly rather than reconstructing them from narrative" that generalizes beyond this specific scanner to any future STATE.md/INDEX extractor design choice.
`[codified][D-1164][structural-position][lexical-heuristic][scan_max_decision_log_id][BC-5.39.006][PR-815][Decisions-Log][design-principle]`

**L-BB-D1167-vp-body-same-burst-sweep-discipline** [process-gap] — During S-25.02 F2's 12-pass adversarial cascade, VP-116..140 body content (and VP-INDEX descriptions) that BC/ADR contract changes at passes 1–3 superseded (the B1 double-prepend blocker fix, the `rotate_changelog` layout/`Write`-semantics/rename-vs-truncate corrections) were NOT swept in the same burst as those corrections — the VP re-review was deferred across passes 1–3 and only actioned comprehensively at pass 4 (a full VP-116..140 sweep), by which point the superseding BC/ADR content had already moved through 3 further amendment rounds. No genuine VP-vs-BC divergence escaped to convergence (pass 4's comprehensive sweep closed the gap before the 3-CLEAN streak began at pass 10), but the gap was a live risk window across 3 passes. **Going-forward discipline (generalizes POLICY 9's same-burst VP-INDEX-change propagation obligation to the INVERSE direction — BC/ADR change → VP sweep — and extends TD-VSDD-060's sibling-site-sweep discipline from code callsites to spec cross-references):** when a BC/ADR contract change during an adversarial cascade supersedes prose, examples, or Property Statements a VP body cites, the VP-body + VP-INDEX-description sweep for every VP touching that contract MUST happen in the SAME fix-burst as the BC/ADR correction, not deferred to a later pass "when convenient." Standing-rule codification (a mechanical check or rubric addition enforcing this) is a JUSTIFIED DEFERRAL per Canonical Principle Rule 3 — anchored to the next self-improvement epic / engine-discipline pass (E-12 Engine Governance follow-up story, no ID allocated yet) — because the in-cycle instance is already fixed and codifying the standing rule requires a governance-policy amendment outside this burst's `.factory/`-bookkeeping scope.
`[codified][process-gap][D-1167][S-25.02][POLICY-9][TD-VSDD-060][VP-sweep][same-burst-discipline][E-12]`

**L-BB-D1169-story-ac-grounding-catches-what-spec-cascades-cannot** [codified] — S-25.02's F2 adversarial cascade (12 passes, 3-consecutive-clean) verified BC↔ADR↔VP consistency exhaustively but could not have caught F-1: story AC-012 cited `BC-1.18.007 postcondition 3` (the general opt-in-required default) for an obligation ADR-051 §Decision 6 actually ratifies (the archive-INCLUSIVE POLICY-1 whole-corpus-scan carve-out), which had no BC postcondition capturing it at all. F2's rubric never compares story ACs against BC content, because ACs do not exist yet at F2 — they are authored at F3. The F3→F4 consistency-gate audit (a fresh-context pass run specifically to verify story-AC↔BC grounding before delta-implementation begins) found and closed F-1 in a single 5-commit cascade. **Going-forward discipline:** a spec-evolution phase's adversarial cascade (F2-equivalent) and a story-population phase's consistency-gate audit (F3-equivalent) are NOT redundant — they check different axes (spec-internal consistency vs. story-to-spec grounding) and a clean F2 cascade provides zero evidence about F3-axis correctness. Feature Mode's F3→F4 gate should always include an explicit AC-to-BC-postcondition citation-and-content audit, not just a frontmatter-population check.
`[codified][D-1169][S-25.02][F-1][ADR-051][BC-1.18.007][consistency-gate][AC-to-BC-grounding][F2-vs-F3-axis][feature-mode]`

**L-BB-D1172-partial-fix-propagation-stale-comment-sibling-sweep** [codified][process-gap][TD-VSDD-060-extension] — Promotes the pass-5 `[codified-pending]` observation (D-1171) to a standing rule now that it is FULLY EVIDENCED: across cluster-1's 12-pass LOCAL adversary cascade on BC-1.18.005 (cap+trigger), the PARTIAL-FIX PROPAGATION / STALE-COMMENT-AFTER-CHANGE finding class recurred **7 times** (F-P1-003 → F-P2-002 → F-P3-002 → F-P5-001 → F-P7-001/002/003/004/005 → F-P8-001/002 → F-P9-001). **Root cause:** every code change — especially a restructure — that renames symbols/tests, moves behavior between functions, or rewrites a doc comment leaves behind stale sibling references the change's own author did not think to check, because "sibling" extends well past the narrow TD-VSDD-060 callsite/signature grep: field-docs, test comments, negative-control tests (which can become VACUOUS when the behavior they probe moves to a different function), renamed-test cross-references in OTHER files, quoted doc-comment excerpts, and crate-root re-exports are all sibling surfaces a symbol/behavior rename or restructure can silently stale. Pass-6's MATCH-FIRST `ShardRegistry::load()`/`validate_entry` split was the single highest-yield trigger for this class (F-P7-001..005) precisely because it moved behavior between functions without a corresponding sweep of every site describing the OLD behavior. **Standing rule (codified):** every code change that renames symbols/tests, moves behavior between functions, or rewrites a doc comment MUST, in the SAME burst, exhaustively sweep ALL sibling references — field-docs, test comments, negative-control tests, renamed-test cross-references in other files, quoted doc-comment excerpts, and crate-root re-exports — not just the adversary-enumerated ones. This is a comment-domain + test-vacuity EXTENSION of TD-VSDD-060 sibling-sweep (which historically covered only callsite/signature sweeps). **Mitigation that worked (cluster-1):** a dedicated comprehensive grep-audit sweep across the ENTIRE delta (all touched files, not just adversary-flagged ones) after any restructure, plus a rule that tests describe BEHAVIOR/INVARIANTS rather than transient RED/GREEN state — so a moved behavior cannot silently vacate the test that once covered it without the test's own wording flagging the mismatch. **Anchor:** E-12 Engine Governance follow-up story (no ID allocated yet) OR the next engine-discipline pass — whichever comes first generalizes this into a project-wide CLAUDE.md/CI-lint discipline. Satisfies cluster-1's Cycle-Closing Checklist S-7.02 obligation for this finding class (D-1172).
`[codified][process-gap][D-1172][D-1171][S-25.02][BC-1.18.005][TD-VSDD-060][sibling-sweep][stale-comment][test-vacuity][partial-fix-propagation][E-12]`

**L-BB-D1181-nclean-passes-not-proof-of-correctness-enshrined-wrong-test** [codified][process-gap] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary passes 5 AND 6 both independently re-derived the ENTIRE data-loss/correctness surface CLEAN and explicitly certified "only text-consistency + test-quality findings remained, NOT correctness." Pass-7 FALSIFIED that certification: F-C2-P7-001 (MAJOR) found that a first-ever over-cap `Write` against a MISSING canonical returned a hard `E-SHD-001` Error instead of the sanctioned empty-canonical `Block`, violating BC-1.18.006 v1.8 Invariant 1 (no Error for a normal over-cap condition) and Precondition 2 (a missing canonical MUST be treated as zero-byte, mirroring BC-1.18.005 EC-004's precedent). **Root cause of the masking:** a pre-existing test, `test_BC_1_18_006_P1a_read_canonical_content_missing_file_is_io_error`, had ENSHRINED the wrong behavior under a plausible-sounding rationale — a comment reading Precondition 2 as governing only the byte-COUNT comparison, not the READ-RESULT itself. Because the test asserted the (wrong) behavior and passed, neither pass 5 nor pass 6's re-derivation of "the entire correctness surface" actually exercised the missing-canonical branch as a live question — the enshrined test had already answered it, incorrectly, and a pass that re-verifies against a codebase's own test suite inherits that suite's blind spots. **Going-forward discipline (generalizes TD-VSDD-059 paper-fix detection one layer earlier — to test-RATIONALE, not just test-presence/pass-fail):** an adversary pass claiming to have re-derived a surface "CLEAN" must independently evaluate whether existing PASSING tests are asserting the CORRECT behavior per the BC's actual Preconditions/Postconditions/Invariants, not merely note that tests exist and pass; a test's inline rationale comment is not authoritative just because it is plausible-sounding — it must be checked against the BC text itself. This is direct evidence that "N consecutive clean passes" is NOT proof a surface is actually correct, and validates the BC-5.39.001 3-CLEAN protocol's insistence on FRESH-CONTEXT passes (each pass re-deriving correctness from the BC text, not inheriting a prior pass's or the codebase's own certification) over trusting any single pass's — or the shipped test suite's own — certification.
`[codified][process-gap][D-1181][D-1180][S-25.02][BC-1.18.006][F-C2-P7-001][TD-VSDD-059][test-rationale][enshrined-wrong-behavior][3-clean-protocol][convergence-economics]`

**L-BB-D1181-commit-attribution-resolved-claude-md-governs** [codified] — Session Resume Checkpoint §4 item 1 had carried a live, unresolved policy conflict since at least D-1176: a session-level instruction to append a `Claude-Session:` commit trailer to every `.factory/` commit, directly contradicting CLAUDE.md's explicit "NEVER add AI attribution to commits — no `Co-Authored-By: Claude`, no robot emojis. The user has explicitly directed this for vsdd-factory" rule (Git Workflow § Non-negotiable git rules). `git -C .factory log` confirmed every recent `factory-artifacts` commit through D-1180 (`671637b9`, `be92d9fc`, `833f1267`, and earlier) had in practice followed the session-level instruction, not CLAUDE.md's rule — a live, uncorrected conflict, not a merely-documented one. **Resolution (explicit human direction, this burst):** CLAUDE.md governs. No `Claude-Session:` trailer, no `Co-Authored-By: Claude`, no emoji, on this or any future `.factory/` commit. This burst's own commit is the first to apply the resolution. **Going-forward discipline:** when a session-level instruction and a checked-in project-governance file (CLAUDE.md) conflict, and the conflict has been observed to resolve in the session-level instruction's favor in actual commit history across multiple bursts, that is itself evidence the conflict needs explicit human adjudication rather than continued default-to-whichever-instruction-was-most-recently-seen — the conflict should be SURFACED (as this session's checkpoint correctly did) rather than silently decided by instruction-recency, and the adjudication, once given, should be recorded as a first-class decision (D-1181, this entry) so it is discoverable rather than merely inferred from a change in subsequent commit messages.
`[codified][D-1181][commit-attribution][CLAUDE.md][governance-conflict][session-resume-checkpoint][§4.1]`

**L-BB-D1182-second-order-fix-interaction-post-fix-rereview-load-bearing** [codified][process-gap] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary pass-8 found F-C2-P8-002 (MEDIUM, correctness/liveness): a 0-byte orphan at a shard's next-seal-sequence destination path could permanently deadlock ALL future rolls, because pass-4's Postcondition 8 write-once guard (`E-SHD-009` on any `write_exclusive` collision) and pass-7's self-heal 0-byte-candidate skip (F-C2-P7-004) — each independently correct and each independently verified safe by its own fix-burst's tests — interact to produce a NEW, unrecoverable deadlock that neither fix's own review pass exercised. This is the SECOND consecutive pass demonstrating the same phenomenon `L-BB-D1181` codified one pass earlier (pass-7's own fix falsified passes 5-6's "correctness surface CLEAN" certification). **Going-forward discipline:** a fix verified safe in isolation — even a fix whose own regression test passes and whose own fix-burst's full workspace gate is green — is NOT thereby proven safe against the CUMULATIVE, evolving state of the spec; every fix-burst's review scope must explicitly cross-check the fix against recently-added SIBLING invariants/postconditions/guards it could plausibly interact with (not merely the finding it was written to close), and a fresh-context adversary pass reviewing the cumulative state — not just the individual diff — is precisely the mechanism that catches this class of defect. This is direct, repeated evidence that the BC-5.39.001 3-CLEAN fresh-context protocol's insistence on re-deriving correctness from the current cumulative spec state (not trusting any prior pass's or any individual fix's own certification) is load-bearing, not redundant process overhead.
`[codified][process-gap][D-1182][second-order-interaction][pass-4][pass-7][write-once-guard][self-heal-skip][3-CLEAN-protocol]`

**L-BB-D1182-sibling-clause-sweep-applies-within-bc-not-only-frontmatter-body** [codified] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary pass-8 found F-C2-P8-001 (MEDIUM, spec-internal contradiction): when pass-4 ADDED BC-1.18.006 Postcondition 8 (a NEW, more-specific `write_exclusive`/exclusive-create seal mechanism), it did NOT sweep the BC's own pre-existing, now-stale sibling clauses — Postcondition 1(b), the adjacent CORRECTED note, Invariant 2, and the `E-SHD-006` self-heal recovery narrative — which still described the OLD `write_atomic`-rename seal mechanism as canonical. The contradiction survived FOUR further passes (5, 6, 7, and the start of 8) undetected, because each of those passes' review scope treated the older clauses as already-settled background rather than re-deriving them against the newest, most-specific clause. **Going-forward discipline:** TD-VSDD-060's sibling-sweep discipline (and its S-7.01 code-comment-domain extension, `L-BB-D1172`) is NOT limited to frontmatter↔body propagation or code-comment↔code propagation — it applies with equal force WITHIN a single BC's own body text: ratifying or amending a newer, more-specific clause (a Postcondition, Invariant, or Edge Case) obligates a same-burst sweep of every OTHER clause in the SAME document that describes the same mechanism, not just the clause being directly edited. A fresh-context adversary pass that re-derives the BC's INTERNAL consistency (not only its consistency against sibling documents) is the correct backstop for this class, as pass-8 demonstrated here.
`[codified][D-1182][sibling-clause-sweep][S-7.01][TD-VSDD-060][BC-internal-consistency][seal-mechanism]`

**L-BB-D1183-weak-substring-error-assertion-recurring-3x-process-gap** [codified][process-gap] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary pass-9 found F-C2-P9-002 (MINOR, doc-vs-code text drift): Postcondition 8's quoted `E-SHD-009` message plus error-taxonomy.md's `E-SHD-008`/`E-SHD-009` Message-Format cells had all diverged from the actual emitted `Display` text, and a prior weak assertion (`.contains("E-SHD-009")`-shaped, not a verbatim-text assertion) had let the divergence ship undetected. This is the THIRD occurrence of the SAME underlying defect class in this one cluster: F-C2-P5-002 (pass-5 — a routed "pin verbatim" test obligation was silently down-scoped to a weak substring assertion, masking a MAJOR spec-vs-code divergence for a full pass), F-C2-P8-003 (pass-8 — `E-SHD-006`/`E-SHD-007` Message-Format `(seq=<N>)` token drift, again undetected by non-verbatim test coverage), and now F-C2-P9-002. Per the Cycle-Closing Checklist's "same defect recurs 3+ times" rule, a THIRD occurrence of an identical defect CLASS is a signal that the underlying root cause is a missing PROCESS-LEVEL guard, not merely three independent instances each fixable by its own one-off test-strengthening. **Going-forward discipline — PROCESS-LEVEL fix, not a fourth manual catch:** the recommended remedy is either (a) a lint/hook that flags `.contains("E-SHD` / `.contains("<ERROR-CODE`-shaped substring-only assertions against an error `Display` impl in test code and requires a verbatim/table-driven assertion instead, or (b) a policy amendment mandating verbatim `Display`-text assertions for every error-message test as a structural rule (not a per-finding adjudication). **Anchor (Canonical Principle Rule 3 — concrete future dependency + specific story/wave, not an un-anchored observation):** routed to the E-12 Engine Governance follow-up story (no ID allocated yet), the SAME anchor this cluster has already used for its other codified cross-cutting process-gap lessons (`L-BB-D1172-partial-fix-propagation-stale-comment-sibling-sweep`, D-1167's VP-body-same-burst-sweep discipline) — this is a testing-discipline defect class general to any error-taxonomy-bearing BC in the codebase, not specific to BC-1.18.006, so a dedicated hook/lint change (option (a), preferred) or governance-policy amendment (option (b)) belongs in that engine-wide governance follow-up rather than a further BC-1.18.006-scoped fix.
`[codified][process-gap][D-1183][weak-substring-assertion][3x-recurrence][E-12][cycle-closing-checklist][TD-VSDD-059]`

**L-BB-D1183-first-clean-correctness-surface-convergence-signal** [codified] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary pass-9 is the FIRST pass in this 9-pass cascade whose fresh-context adversary certified the CORRECTNESS surface CLEAN — all 3 findings (F-C2-P9-001/002/003) are documentation/text-drift or test-coverage classes, zero functional, data-loss, or liveness defects. Contrast with passes 5 and 6, which also certified the correctness surface CLEAN but were subsequently FALSIFIED at passes 7 and 8 respectively (`L-BB-D1181`, `L-BB-D1182`) — pass-9's certification has not yet been re-derived by a subsequent fresh-context pass, so per those same lessons it should NOT be treated as proof of correctness on its own. **Convergence-trajectory signal (not a streak advance):** the remaining defect surface across passes 5-9 has visibly narrowed from correctness-affecting findings (passes 1-4: data-loss, corruption, fail-open-vs-fail-loud, a falsified invariant) toward doc/text/test-coverage findings only (passes 5-6 partial, 8-9 the last two), which is the expected SHAPE of a converging cascade even though the literal 3-CLEAN streak has not started (pass-9 itself carried 3 non-zero MINOR findings, so the streak stays 0/3). This is recorded as a trajectory data point for whoever next evaluates whether to continue toward literal 3-CLEAN or accept D-386 Option C asymptotic convergence for this LOCAL cluster cascade — not as a claim that convergence has been reached.
`[codified][D-1183][convergence-trajectory][correctness-surface-clean][pass-9][not-a-streak-advance]`

**L-BB-D1184-asymptotic-acceptance-is-a-legitimate-human-authorized-convergence-economics-call** [codified] — S-25.02 cluster-2 (roll, BC-1.18.006) LOCAL adversary pass-10 was the SECOND consecutive correctness-surface-CLEAN pass (following pass-9, `L-BB-D1183`) — 0 BLOCKER/MAJOR/MEDIUM, 1 MINOR (an `error-taxonomy.md` text drift) + 3 ADVISORY (a race-wording narrowing, an untested-arm process-gap, and a gloss-annotation reconciliation), none correctness-affecting. `L-BB-D1183`'s own trajectory-data-point note explicitly flagged this as "a data point for whoever next evaluates whether to continue toward literal 3-CLEAN or accept D-386 Option C asymptotic convergence" — pass-10 is that evaluation, and the human explicitly authorized converging the cluster-2 LOCAL BC-5.39.001 cascade to PR via asymptotic acceptance rather than continuing to grind toward a literal 3-CONSECUTIVE-CLEAN streak. **This closes the cascade at streak 0/3 (D-1184), NOT literal 3/3 — a materially different closure basis than cluster-1's (BC-1.18.005, D-1172), which reached literal 3-CLEAN via 12 passes under an explicit human-directed grind-to-literal-3-CLEAN instruction.** The evidence basis supporting the asymptotic-acceptance decision is the full 10-pass finding-severity trajectory: P1-4 data-loss/corruption-class MAJORs and a falsified invariant → P5-6 correctness-surface-CLEAN (later falsified at P7, `L-BB-D1181`) → P7 MAJOR (falsification) → P8 2×MEDIUM (second-order interaction, `L-BB-D1182`) → P9/P10 MINOR-only, correctness CLEAN both times. **Going-forward discipline:** converging a LOCAL adversarial cascade to PR at an asymptotic minor/advisory floor (correctness surface independently re-derived CLEAN by 2 consecutive fresh-context passes) is a legitimate human-authorized economics call under D-386 Option C — it is NOT a shortcut around the 3-CLEAN protocol's rigor, because the protocol's remaining review layers (PR-level pr-reviewer cascade within pr-manager's 9-step, CI's fmt/clippy/full-workspace gate, and Phase F6 targeted-hardening for the 2 items this decision explicitly defers — F-C2-P10-002's `O_EXCL` reclaim hardening and F-C2-P10-003's EC-025 fault-injection test) remain fully in force and are the mechanism by which any residual risk this bounded LOCAL cascade might have missed gets caught. A state-manager or product-owner burst recording such a decision MUST (as this burst does) record the trajectory evidence, the explicit distinction from any sibling cluster that reached literal 3/3, and the concrete F6 anchors for whatever the decision defers — an asymptotic-acceptance record that omits any of these three is a documentation gap, not merely a style preference.
`[codified][D-1184][D-1183][D-1172][D-386-option-c][asymptotic-acceptance][convergence-economics][cluster-2][cluster-1-contrast][human-authorized]`

**L-BB-D1184-exhaustive-sibling-sweep-closes-recurring-message-drift-class** [codified] — F-C2-P10-001 (MINOR) found `error-taxonomy.md`'s `E-SHD-001` Message Format cell had drifted from the shipped `Display` text — the SAME underlying defect class (`error-taxonomy.md` cell vs. shipped `Display` text divergence) that passes 6, 7, 8, and 9 had each independently found and fixed ONE ROW AT A TIME (`E-SHD-006`/`E-SHD-007` at pass-6/8, `E-SHD-008`/`E-SHD-009` at pass-9) without ever performing a single exhaustive sweep across ALL nine `E-SHD-NNN` rows in the same burst. Pass-10 broke this one-at-a-time pattern by reading and diffing EVERY `ShardRollError` variant's `#[error(...)]` `Display` impl in `shard_manager.rs` against the taxonomy table in a single pass, confirming `E-SHD-001` was the sole remaining drift and that `E-SHD-006`/`007`/`008`/`009` (already corrected at prior passes) still match verbatim. **This directly reinforces `L-BB-D1183`'s TD-VSDD-060 lesson from the immediately-prior pass** ("sweep ALL siblings when the first is found") — but demonstrates the SAME cluster needed a second, explicit instance of the discipline before the sweep was actually performed exhaustively rather than reactively: a lesson codified after finding N does not retroactively close finding N+1 of the identical class if the NEXT burst that touches the same file family reverts to one-at-a-time fixing. **Going-forward discipline:** when a burst is about to fix ONE row of a multi-row, multi-cell taxonomy/reference table against its own authoritative source (here, `Display` impls in source code), the fix-burst SHOULD default to sweeping ALL rows of the SAME family against the SAME source in the SAME burst — not just the row the triggering finding named — even when the lesson that would have prompted this was only codified one pass earlier; codifying a lesson is necessary but not sufficient, the NEXT applicable burst must actually apply it exhaustively rather than narrowly.
`[codified][D-1184][D-1183][TD-VSDD-060][exhaustive-sweep][error-taxonomy][E-SHD][sibling-sweep][recurring-pattern][reinforces-prior-lesson]`

**L-BB-D1194-transient-status-test-doc-comment-recurring-3x-process-gap** [codified][process-gap] — S-25.02 cluster-3 (mechanism-A backfill, BC-1.18.007+008) LOCAL adversary pass-4 found O-2 (LOW, non-blocking-severity but process-gap-class): the test module's doc comment describing pass-3's `mechanism_a_verify_backfill_per_shard_cap_preserved` hard-gate fixtures used transient "expected to fail pre-fix" / "does not yet exist" framing that went stale the moment pass-3's own implementer fix landed in the SAME burst — a reader encountering the module in isolation post-fix would be misled about current test status/intent. This is the THIRD occurrence of the exact same transient-status-header defect shape in this one cluster's own cascade: F-C3-P1-005 (pass-1 — a stale doc comment describing a withdrawn exact-`ceil()` framing that had gone stale after the same-pass BC-1.18.008 v1.2→v1.3 amendment), F-C3-P2-004 (pass-2 — a test-module header still describing F-001/F-002 in "expected to fail pre-fix" framing, stale since pass-1 landed), and now this O-2/pass-4 instance. All three share the identical root shape: a test doc-comment header phrased as a transient, commit-relative pass/fail claim ("expected to fail", "does not yet exist pre-fix") rather than a status-neutral description of what the fixture verifies by stable BC/EC/finding-ID citation — the header is written to be true only until the very fix landing in the same burst makes it false. Per the project's established 3+-recurrence rule (the same threshold that triggered `[process-gap]` codification for the weak-substring-error-assertion class at D-1183/cluster-2 pass-9), a THIRD occurrence of an identical defect CLASS is a signal that the underlying root cause is a missing PROCESS-LEVEL guard on the `vsdd-factory:test-writer` agent's own doc-comment authoring convention, not three independent instances each fixable by its own one-off rewrite. This instance was tracked as a `[D-1192] [process-watch]` Drift Item at 2/3 recurrences following pass-2 — this occurrence crosses the codification threshold. **Going-forward discipline — PROCESS-LEVEL fix, not a fourth manual catch:** the recommended remedy is a `vsdd-factory:test-writer` agent-prompt amendment forbidding transient-status prose ("expected to fail", "should fail pre-fix", "does not yet exist", "RED surface", or similar commit-relative framing) in test doc-comment headers, requiring status-neutral "this test pins `<BC/EC/finding-ID>`" framing instead — a header phrased this way remains true regardless of whether the fix it verifies has landed yet, closing the recurrence at its source rather than catching each instance after the fact. Optionally paired with a lint/hook flagging the forbidden phrase set in test-file doc comments. **Anchor (Canonical Principle Rule 3 — concrete future dependency + specific story/wave, not an un-anchored observation):** routed to a NEW draft follow-up story **S-12.09** (E-12 Engine Governance, registered this burst in `STORY-INDEX.md`), the SAME epic this cluster has already used for its other codified cross-cutting process-gap lessons (`L-BB-D1172-partial-fix-propagation-stale-comment-sibling-sweep`, `L-BB-D1183-weak-substring-error-assertion-recurring-3x-process-gap`) — this is an agent-authoring-discipline defect class general to any BC-driven TDD story, not specific to BC-1.18.008, so the fix belongs in that engine-wide governance follow-up rather than a further BC-1.18.008-scoped fix. Distinct from `L-BB-D1183`'s weak-substring-assertion class: that lesson is about TEST-ASSERTION content (what a test checks); this lesson is about TEST DOC-COMMENT framing (what a test's header claims about its own status) — related agent-discipline gaps, not the same underlying defect.
`[codified][process-gap][D-1194][transient-status-header][3x-recurrence][test-doc-comment][S-12.09][E-12][cycle-closing-checklist][process-watch-escalation]`
