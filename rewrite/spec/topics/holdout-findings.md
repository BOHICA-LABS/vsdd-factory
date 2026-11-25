---
kind: topic
id: holdout-findings
sources: [pipeline.md#2.7, pipeline.md#2.9.2, BLUEPRINT.md#4.10]
---

# Holdout findings

How findings are built from hidden-scenario scores so that they block. The convergence rule they
plug into is [[topic:convergence]]; the wall around the scenario text is
[[topic:walled-findings]].

The evaluator returns scores and symptoms, never findings. The tool (and `holdoutFindings()` in the
prelude, held to it by fixtures) builds the findings, and their shape is fixed so that they block.

- One finding per must-pass scenario under its floor: key `holdout:<HS id>`.
- One aggregate finding when the mean or the spread misses its threshold: key `holdout:<unit>:mean`
  or `holdout:<unit>:spread`.

Each has category `holdout`, severity `high`, scope `integration` at a wave and `system` at release,
`spec_ref` set to the contracts the scenario cites, `repro` set to the scenario id and the rotation
seed, the evaluator's symptom as `observed` and `expected`, and no location. `title` is a fixed
sentence built from the key and the score ("Hidden scenario HS-014 scored 0.50, under its floor of
0.60"), never the scenario's own title.

Three points make the two copies agree and the finding fit [[schema:finding]]:

- "No location" is written `location: {"path": ""}`, because the finding schema requires the field.
  `owner` is `product-owner`, the one author who may read the scenario; at a wave or a release the
  finding is answered by a fix story, which is not routed on `owner`.
- Only the tool can read a scenario, so it resolves the contracts each scenario cites and the gate
  hands them to the script: the output of `vsdd gate wave` and `vsdd gate release` carries `holdout`,
  with the rotation `seed` and `contracts`, a map from scenario id to contract ids
  ([[schema:wave-gate-data]]). `holdoutFindings()` takes its `spec_ref` and seed from there.
- The thresholds (`holdout.mean_min`, `must_pass_min`, `stddev_max`, [[config:holdout]]) reach the
  script in the survey's `config.holdout`, not in `limits`, whose keys are limits only.

The prelude function is `holdoutFindings(unit, scores, opts)`, where `opts` holds the three thresholds,
`seed`, `contracts`, `scope` and the evaluator's `symptoms`. A must-pass scenario that did not run
scores 0.

`judge()` and its Python copy exempt category `holdout` from the location condition and from
grounding, because there is no file to point at and the scenario path must not appear. Without that
exemption a failing must-pass scenario would be advisory and the hidden-scenario gate would be
silently off. Two conformance fixtures hold it: a must-pass scenario at 0.5 blocks, and a mean of 0.8
blocks.

Holdout data, as the evaluator returns it: `scores` (`scenario`, `satisfaction`, `must_pass`, `ran`)
and `symptoms` (observed and expected behaviour, no location, no scenario text)
([[schema:holdout-data]]). The evaluator's text for a failing scenario describes the symptom and must
not quote the scenario; the tool checks that before a fix story is written from it.
