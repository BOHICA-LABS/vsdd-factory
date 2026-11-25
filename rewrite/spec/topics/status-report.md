---
kind: topic
id: status-report
sources: [state.md#2.7, BLUEPRINT.md#5.2]
---

# The status report: computed, not stored

There is no status file. `vsdd.state.status(root)` computes the object
below from the records each time it is asked: `factory.toml`, the lock, the driver flag, the open cycle's
`cycle.json`, its closure records and open decisions, the spike ledger's open named entries, `notes.json`,
and then the route ([[table:routes]]). That is tens of small file reads and no subprocess, a few
milliseconds, well inside the eight seconds the briefing is allowed. `vsdd status --json` ([[command:status]])
prints it, and the briefing ([[topic:briefing]]) renders it, so the two cannot disagree, and nothing can be
stale or missing. Notes, the one part that is not derived, live in the tracked `.factory/notes.json`
([[record:notes]]). `next` is the object `{kind, command, args}` of the pipeline design's workflow result.
The object holds only what the briefing prints and what the hook rules test.

```json
{
  "schema": 1,
  "updated": "2026-11-26T16:40:00Z",
  "project": "acme-api",
  "cycle": {
    "id": "auth-sso",
    "kind": "feature",
    "title": "Single sign-on for the admin console",
    "opened": "2026-11-23",
    "units": { "spec": "closed", "plan": "closed", "verify": "open" },
    "waves": { "W-01": "passed", "W-02": "open" },
    "wave": { "id": "W-02", "stories": 5, "delivered": 3, "blocked": 1 }
  },
  "decisions": [
    { "id": "finding.S-7.04", "kind": "finding", "question": "Waive or direct: lockout counter race",
      "packet": ".factory/cycles/auth-sso/decisions/finding.S-7.04.md" }
  ],
  "spikes": ["uv-startup"],
  "others": [],
  "requests_queued": 0,
  "driver": { "state": "on", "at": null },
  "next": { "kind": "workflow", "command": "/vsdd-factory:wave", "args": {} },
  "notes": [ { "at": "2026-11-26T16:40:00Z", "text": "PR 41 needs the staging secret before it can merge." } ]
}
```

| Field | Meaning | Limit |
|---|---|---|
| `schema`, `updated`, `project` | Format version, when it was computed, project name | |
| `cycle` | The active cycle, or `null` | One |
| `cycle.units` | State of each non-story unit: `open`, `closed`, `blocked`, `failed` | One entry per workflow |
| `cycle.waves` | Gate status of each wave: `open`, `gating`, `passed`, `failed`. `passed` is the only passing value. | 60 waves |
| `cycle.wave` | Counts for the open wave | |
| `decisions` | Open decisions: identifier, kind, one line, packet path | 12 listed, then a count |
| `spikes` | Open named spikes | 8 |
| `others` | One line each for a second cycle in flight (a critical fix), with its qualified decision identifiers | 3 |
| `requests_queued` | How many change requests wait for the cycle to close | |
| `driver` | Whether the driver is `on`, `paused` (and at which decision) or `off`, and in which session | |
| `next` | The object `{kind, command, args}` from the route table. `kind` is `workflow`, `human` or `done`. For the second of two open cycles `args` carries `cycle`. The briefing renders it as text | The rendered text is at most 200 characters |
| `notes` | Free text a person asked to keep, through `vsdd note` ([[command:note]]) | 5 entries of 200 characters; the oldest is dropped |

The lists are shortened so that the rendered object stays under 8,192 bytes.

## Who writes what the report is computed from

Nothing writes the report. The records it reads are written only by `vsdd`, at these points.

| Moment | Command | What it writes | Run by |
|---|---|---|---|
| A workflow starts | `vsdd survey` | The run record and the lock; a cycle record when a request starts a cycle | The clerk step that opens every workflow |
| A workflow ends | `vsdd checkpoint` | Closure records, decision records and packets | The clerk step that closes every workflow |
| A story merges, a wave gate is measured | `vsdd story deliver`, `vsdd gate wave` | The story's or the wave's closure record | A clerk step |
| A person settles a decision | `vsdd decide`, called by the hook runner when the person types `/vsdd-factory:decide`, or by the person in a terminal | The decision record, and nothing else | The hook runner or the person; never an agent |
| A cycle opens | `vsdd checkpoint brief`; the checkpoint of `discover` in assess mode (the same code); `vsdd survey feature`; `vsdd story new-fix --maintain` when it is about to write fix stories and no cycle is open | `cycle.json` | A clerk step, or the main session for the brief |
| A change request's route is derived | `vsdd feature route` | The `route` field of `cycle.json` | The architect, as the last act of its delta step |
| A cycle closes | The `release` checkpoint | The close fields of `cycle.json` | A clerk step |
| A named spike opens or closes | `vsdd spike open --name`, `vsdd spike close` | The ledger entry | The spike runner |
| A person leaves a note | `vsdd note "<text>"` | `notes.json` | The main session, on request |

The pipeline design runs one cycle at a time; a critical fix is the only second line of work, and the report
has room for it in `others` ([[topic:cycles]], [[topic:hotfix-cycle]]).

## Team use (decision D33; built when the build widens)

The status object gains `work_claims` and `sync`; what they hold is in [[command:status]].
