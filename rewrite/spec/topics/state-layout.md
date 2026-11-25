---
kind: topic
id: state-layout
sources: [state.md#layout, BLUEPRINT.md#4.7]
---

# The layout of .factory/

Tracked on the state branch, except `local/`:

```
.factory/
├── factory.toml                        project configuration (2.12); a person edits it, no tool call can
├── notes.json                          up to five notes a person asked to keep (written by vsdd note)
├── .gitignore                          one line: local/
├── specs/
│   ├── product/                        brief.md, prd.md, domain/<section>.md, supplements/<name>.md,
│   │                                   contracts/<S>.<SS>/BC-<S>.<SS>.<NNN>.md
│   ├── architecture/                   overview.md, SS-<NN>.md, modules.toml, dependencies.toml, conventions.md, review-rules.md,
│   │                                   decisions/ADR-<NNN>.md, properties/VP-<NNN>.md, assessments/<slug>.md
│   └── ux/                             overview.md, screens/SCR-<NNN>.md, flows/FLOW-<NNN>.md, design-system/**
├── stories/E-<N>/                      epic.md, S-<N>.<NN>.md
├── holdout/                            HS-<NNN>.md, evaluations/<cycle>/<unit>-<n>.json (written by code;
│                                       scores, symptoms, and the full form of walled findings)
├── reviews/<yyyy-mm-dd>-<lens>-<n>.md  on-demand review reports only; pipeline reviews write no file
├── demo/<cycle>/<story>/               AC-<k>.<ext>, AC-<k>.txt, AC-<k>.png, report.md
├── analysis/<scope>/<slug>.md          brownfield and port analysis; security.md, disposition.md, port.md
├── research/<yyyy-mm-dd>-<slug>.md     reports, retrospectives and idea briefs
├── research/inputs/<source>/<date>.*   collected feedback and analytics (written by vsdd discover collect)
├── requests/CR-<NNN>.md                every change request, queued or running (written by code)
├── ideas/ID-<NNN>.md                   the idea registry
├── spikes/SP-<NNN>.md                  the spike ledger
├── deferred/<slug>.md
├── cycles/<cycle>/
│   ├── cycle.json                      what the cycle is: kind, title, route, baseline, request; and, once
│   │                                   closed, the close date, the version and the state commit before pruning
│   ├── units/<workflow>[.<subject>].json     closure records
│   └── decisions/<id>.json, <id>.md    decision records and their packets
├── cycles/_standing/                   the same shape, for work outside a product cycle: ingest, discover,
│                                       maintain and spikes, and the ingest or import baseline. It never closes;
│                                       it keeps the latest closure per workflow and subject, the baseline, the
│                                       latest `spec` decision, and other settled decisions for 90 days
└── local/                              untracked
    ├── runs/<run-id>/run.json, steps/<workflow>/<subject>/<name>.<attempt>.json
    ├── evidence/<subject>/<gate>.<sha12>.json   (and the raw output, JUnit report or diff patch beside it)
    ├── jobs/<key>.json, <key>.log      detached gate and delivery jobs, keyed by command, subject and commit;
    │                                   products started by vsdd env start
    ├── lock.json, lock.flock           the run lock and the file lock that guards it
    ├── driver.json                     whether the driver skill is active or paused, and in which session
    ├── hooks/                          the hook runner's own records (caller markers, spike admissions, the
    │                                   configuration fingerprint)
    ├── events/                         the event log
    ├── media/, index.json
    └── flush.lock
```

Every top-level directory is one path class with one writer role; the classes and their writers are
[[table:path-classes]], which is what lets the hook runner check write scopes by path prefix.

The machine records that live in the tracked part — the cycle, closure and decision records, the notes,
the holdout evaluations — are defined in [[topic:machine-records]]. The artifact files under `specs/`,
`stories/`, `holdout/`, `reviews/`, `demo/`, `analysis/`, `research/`, `requests/`, `ideas/`, `spikes/`
and `deferred/` are the artifact kinds; each has its own file ([[table:artifacts]] lists them).

Decision D31, built when the build widens: `stories/flags/<stem>.md` holds the flag register, in the class
`stories`; the stem is the flag's key, encoded ([[artifact:flag]]). The tree above is unchanged until then.

Decision D33, built when the build widens: `cycles/<cycle>/claims/<scope>.json` holds the work claims
([[record:work-claim]]) and `cycles/<cycle>/red/<story>.json` the red record that travels with a story
([[record:evidence]]), both in the class `control`; `local/clone.json`, `local/aside/` and
`local/sync.lock` are one clone's own ([[record:clone]], [[command:sync]]). The tree above is unchanged
until then.
