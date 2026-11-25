---
kind: record
id: clone
package: M6
sources: [decisions.md#D33]
---

# Clone record

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the explanation
is [[topic:team-sync]].

`.factory/local/clone.json`. Not tracked: it is the one record that says which clone this is, so it never
leaves the clone. Fields in [[schema:clone]].

```json
{ "schema": 1, "id": "5f0c2a9d81b3e746", "label": "ada-laptop", "created": "2026-11-26T09:12:40Z",
  "how": "joined",
  "sync": { "state": "synced", "at": "2026-11-26T14:02:09Z", "remote_head": "9c4888d0…", "ahead": 0,
            "unpublished": [], "unknown": [], "error": "" },
  "aside": [], "settled": [] }
```

## What it holds

- **`id`**: sixteen random hex characters, made once. Claims, decision records, deliveries and the
  `Clone:` trailer of every state commit name it ([[topic:team-records]]). A person is named by git,
  never by this record (T-WHO in [[table:team-checks]]).
- **`label`**: a short name people read in place of the id, so that a person with two machines can tell
  which one holds a claim. It is the machine's host name unless the person gives another, with
  `vsdd init --label` when the clone joins or `vsdd sync --label` later. It is copied into the claims
  this clone takes and so is shared; a person who does not want the host name shared gives another. It
  is shown only, and decides nothing.
- **`how`**: `switched` on the clone that turned team use on, `joined` on one that mounted a team's
  state or found a team formed around it, `remade` where the record was lost (T-JOIN, T-SOLO, T-SWITCH).
- **`sync`**: the outcome of the last sync, which the briefing and `vsdd status` print from here, so
  neither fetches. `remote_head` is the last head of the remote's state branch whose arrivals this
  clone checked and took into its mount; when it is written is T-HISTORY's to say, and a fetch that
  only judges a push does not change it.
  `unpublished` lists the settlements the remote does not hold yet, which every reader treats as open
  (T-SETTLE in [[table:work-claim-checks]]). `unknown` lists commits whose push got no answer, until a
  sync measures whether the remote took them (T-PUSH in [[table:sync-checks]]); one that minted an id
  is listed with its `request`.
- **`aside`**: each thing a sync did not take from this clone, with where the clone's version was saved
  under `.factory/local/aside/` and whose version stands (T-COMBINE, T-HOLD, T-SETTLE). An entry is
  listed in `vsdd status` and pruned with the run records, after `state.retain_runs_days`.
- **`settled`**: each commit that left `sync.unknown`, with the claim or the minted file it carried,
  its `request` and whether the remote had taken it. An entry is listed in `vsdd status` and the
  briefing and pruned as `aside` is; one for a minted file also leaves when a command with the same
  request is run again (T-MINT).
- **`request`**, in an `unknown` or a `settled` entry whose commit minted an id: the SHA-256 of the kind
  of id and of every value the caller gave for the new file, in the order of the file's fields. Nothing
  the tool adds is in it (the id, a time, the run, the clone, the person), so the same request has the
  same digest on any day and under any id, and another title or text has another. One function of
  package M3 computes it for every command that mints, and a test holds it to both halves.

## Written by

The sync; `vsdd init` when a clone joins; and the function that settles a decision, which adds the
decision's id to `sync.unpublished` under the lock it settles under. It does not exist while team use is
off, so a project with one clone has nothing new on disk.

## When it is lost

A clone whose record is gone (a deleted `.factory/`, remounted) is a new clone to everyone else. With
team use on, every command that syncs refuses there and names `/vsdd-factory:status --fix`, which makes
a new record with `how = "remade"`, says so, and lists the claims the old id held. A person takes those
over with `vsdd claim take`, as from any clone that is gone.

## Cap and lifetime

Cap 8,000 bytes; when `aside` would pass it, the oldest entries past retention go first and the write is
refused if none can. Kept as long as the clone.
