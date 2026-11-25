---
kind: record
id: work-claim
package: M2
sources: [decisions.md#D33]
---

# Work claim

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the explanation
is [[topic:team-claims]].

`cycles/<cycle>/claims/<scope>.json`. Tracked: it is committed to the state branch, which is where every
clone sees it. Written only by code, in the path class `control`, which already covers `cycles/**`. Fields
in [[schema:work-claim]]. The name says work because the design already has a claim of another kind, the
fixer's claim about a finding ([[schema:claim]]).

```json
{ "schema": 1, "scope": "story.S-7.03", "cycle": "auth-sso", "workflow": "story", "subject": "S-7.03",
  "holder": { "name": "Ada Example", "email": "ada@example.invalid", "clone": "5f0c2a9d81b3e746",
              "label": "ada-laptop" },
  "run": "r-20261126-1402-7q", "at": "2026-11-26T14:02:11Z", "plugin": "1.0.0" }
```

One file per scope ([[table:work-claim-scopes]]). The file exists while the claim is held and is removed
when it ends; who held a scope, and when, stays in the state branch's history.

## Written by

One function of the claim store, which the survey, the two admission commands, `vsdd feature route`,
`vsdd claim take` and the sync in which a clone turns team use on or joins a team call to take or
change a claim: it holds nothing until the remote has accepted the push (T-PUBLISH in
[[table:work-claim-checks]]). The checkpoint, a full sync and `vsdd claim release` remove one
(T-RELEASE). `run` is empty where no run took the claim; T-RELEASE reads it.

## What it is not

It is not the run lock ([[record:lock]]), which stays on the clone and keeps one clone to one run.
`at` is shown to people and decides nothing; how a claim ends is T-TAKEOVER's and T-RELEASE's to say.
It gives no person a right: `holder` says which clone has the work.

## Cap and lifetime

Cap 1,000 bytes. Kept until the claim ends.
