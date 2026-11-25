---
kind: packaging
id: build-plan
sources: [BLUEPRINT.md#9, BLUEPRINT.md#9.1, BLUEPRINT.md#9.2, BLUEPRINT.md#9.3, BLUEPRINT.md#9.4, BLUEPRINT.md#9.5, BLUEPRINT.md#9.6, decisions.md#D31, decisions.md#D33]
---

# How the build is ordered

Each package is sized for one builder and one reviewer. Builders work only under `next/`, the staging root that
mirrors the final repository ([[packaging:rewrite-layout]]). "Design" names the document a package is built from; the
blueprint's section 4 overrides it where they differ.

One file per package — what it delivers and what it depends on — is in `rewrite/spec/packages/<ID>.md`; the table of
contents is the registry's `package` entries. The rule that the per-package briefs are generated from the build plan
and the registry stands: this file holds the ordering, not the package contents.

There are 86 packages: the 74 of the blueprint's build plan, six that decision D31 added for feature flags,
F1 to F6 ([[package:F1]], [[package:F2]], [[package:F3]], [[package:F4]], [[package:F5]], [[package:F6]];
registered in `registry.d/d31.toml`), and six that decision D33 added for team use, M1 to M6
([[package:M1]], [[package:M2]], [[package:M3]], [[package:M4]], [[package:M5]], [[package:M6]];
registered in `registry.d/d33.toml`).
Each package is sized for one builder and one reviewer. B4 is two packages, to break a dependency cycle with
H1 ([[package:B4a]], [[package:B4b]], [[package:H1]]). There is no package C3 (stages and routes): there is no
stage registry, and the route function and the preconditions are part of S3 ([[package:S3]]).

## Order of work

1. First: B0 to B3, B6, B8, R0, C1 and C2 ([[package:B0]] to [[package:B3]], [[package:B6]], [[package:B8]],
   [[package:R0]], [[package:C1]], [[package:C2]]). B8's answers can change H, S and K packages, so it runs early.
2. Then B4a and B5 ([[package:B4a]], [[package:B5]]).
3. Then the S, G, H and A packages run in parallel; B4b follows H1.
4. Workflows follow the packages they call.
5. Then the flag packages, in the order F1, F5, F2, F6, F3, F4 ([[topic:feature-flags]]); the ids say when
   a package was added, not when it is built. Each extends packages that must be complete first: F1 the
   state core, the policy tables, the survey, the status report and the artifact rule; F5 the planning
   gates; F2 the story gates, the verdict and the rule `secrets`; F6 the wave, release and sweep jobs; F3
   decisions, delivery and the registers; F4 the agent packages, to whose method files it adds a section,
   and the workflows its scenarios run. F4 needs only the first part of E1 and the story tier of E2, which
   the slice builds. They come before the proofs so that E3 and E4 run on the tool as it will ship; neither
   proof depends on them.
6. Then the team packages, in the order M6, M1, M2, M3, M4, M5 ([[topic:team-use]]); here too the ids
   say when a package was added. Each extends packages that must be complete first: M6 the state core,
   the record stores, the loader, the status report and the configuration fingerprint; M1 the flush, the
   survey and the jobs; M2 the survey and admission, decisions, the wall table and the gate guard; M3
   decisions, the registers, the story gates, the verdict and the `decisions` rule; M4 delivery in `pr`
   mode, governed delivery, the wave and release gates, the release command, the reviewers and the
   `story`, `wave` and `verify` workflows; M5 the harness, the `setup` skill, the first part of E1 and
   the story tier of E2. None of them needs a package of item 7: `vsdd doctor` (T1) prints its team
   lines from a function M6 delivers, and takes them when it is built. They come before the proofs for
   the same reason as the flag packages, and neither E3 nor E4 depends on them: both proofs run on one
   clone. M5 is the proof of team use itself, and until it passes neither `setup` nor the user documents
   of item 7 offer team use ([[packaging:proof-team]]).
7. Operator tools, proofs, documents, release and cutover: T1, T3, T5, E1 to E4, D2, D3, R1, CO1, CO2
   ([[package:T1]], [[package:T3]], [[package:T5]], [[package:E1]], [[package:E2]], [[package:E3]],
   [[package:E4]], [[package:D2]], [[package:D3]], [[package:R1]], [[package:CO1]], [[package:CO2]]).
   The blueprint gives no order among these beyond each package's own dependencies, and a package here that
   an earlier one depends on is built before it: T1 before K1 ([[package:K1]]).

E3 and E4 are the gate for cutover; the cutover steps are X0 to X8 ([[packaging:cutover-steps]]).

## Rules the order follows

The package briefs give each package's dependencies as the plan's tables wrote them. Four rules add to
them, and where a brief and a rule differ the rule holds:

- **A workflow needs every agent its script names.** So W2 and W3 also need A2b (the architect arbitrates),
  W7 also needs A2b and A6b, and W11 and W12 also need A6b.
- **The runner core needs the probes.** H1 and S2b are built after B8 has run the items that decide their
  code, and the result of each is in `rewrite/STATE.md`: 6 and 10, 27 to 31, and 34 ([[doc:platform]]).
- **S3 is built in two parts, in this order.** First the records it writes, the preconditions, the route
  with a fixture per row, the worktree library, story admission and cycles, which need S2a, S2b and C1.
  Then story positions, measuring a missing gate again and the checkpoint's mapping of results to
  closures and decisions, which also need G2 and G3. G5 and the other gate packages need only the first
  part; G4a needs both. The survey reaches the gates through the gate registry
  ([[topic:evidence-lookup]]), so S3 imports no gate package and there is no cycle.
- **E1 is built in two parts.** The sample project, its JUnit runner, the fixture overlays and their
  loader come first and need only B3 and S1 ([[packaging:sample-project]]). The probe runner and the
  smoke probes follow when H1 to H9, W1 and A6b exist. E2 needs only the first part.

While packages are still missing, the roster lint checks that nothing on disk is absent from the roster;
its other half, that nothing in the roster is absent from disk, is switched on when E1's second part is
built ([[table:lint-checks]], L-ROSTER). The roster itself always lists the whole design
([[table:roster]]), so the prelude's tier tables are complete from the start.

## The thin slice

The first milestone is one story delivered on the sample project through the real platform, in local
delivery mode. It is built in this order:

1. Foundations: B0 to B3, C1, B4a, B5 and the sample project with its runner.
2. B8 for the probe items above; C2; and B6 for the roster and the lints the slice's files need.
3. S1, S2a, S2b, S4 and the first part of S3; H1, then B4b; G1.
4. G2 and G3; the second part of S3; G4a; H5 and H7.
5. The agents `story.js` names, each as a complete agent file with the methods the story script gives
   it and no others yet: the clerk (A1); the test-writer, implementer and verifier (A4); the adversary and
   security-reviewer (A6a); the architect with `arbitrate` (A2b); the demo-recorder, technical-writer and
   devops (A5). Their remaining methods come with their packages.
6. W2, then the `story-ready` fixture and its loader, then E2's first run: one story, headless.

A package named here is finished for the slice when the files the story path touches pass their tests; the
rest of the package is built when the build widens.

No F package is on the slice. Decision D31 changes nothing the slice path touches: where a file the slice
reads has a section headed "Feature flags", that section is built with F1 to F6 and not before.

No M package is on the slice either. Decision D33 changes nothing the slice does: it stays one story on
one clone, in local delivery mode, and where a file the slice reads has a section headed "Team use", that
section is built with M1 to M6 and not before. What decision D33 asks of the slice's own units is only
that they do not build in the assumption of one clone. Each such point is a ledger entry `D33-<n>` in
the list of the package that owns it (`rewrite/findings/by-package/`); those marked as on the slice path
are for the unit to build now, and none of them changes what the slice does on one machine.

Foundation packages of the packaging area: B0 (rewrite-mode root), B1 (manifest and shell), B2 (launcher and
entries), B3 (shared core), B4a and B4b (the two halves of the build step, [[packaging:build-step]]), B5 (workflow
harness), B6 (lint and roster), B7 (CI), B8 (platform probes), R0 (marketplace install check,
[[packaging:install-check]]).
