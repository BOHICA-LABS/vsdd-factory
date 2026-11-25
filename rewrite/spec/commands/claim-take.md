---
kind: command
id: claim-take
package: M2
sources: [decisions.md#D33]
---

# `vsdd claim take`

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33.

Takes over a work claim another clone holds. Person only, in a terminal.

## Syntax

`vsdd claim take <scope> [--cycle <id>] [-- note]`, for example `vsdd claim take story.S-7.03 -- Ada is
away until Monday`. The scopes are [[table:work-claim-scopes]].

## What it does

It is how a person says that a holder is gone (T-TAKEOVER in [[table:work-claim-checks]]; why it takes
a person, in [[topic:team-claims]]).

1. It syncs, then prints the claim: the holder's person, clone and label, the time the holder wrote, and
   what that clone last pushed for the unit (the story branch's head and the closure's last stage).
2. It rewrites the claim with this clone as holder, the old holder in `taken_from` and the note, and
   publishes it as every claim is published (T-PUBLISH). If the claim changed since step 1, it shows the
   new holder and stops.
3. It prints what the next run will continue from: the story branch as the remote has it and the
   closure's stages, or "nothing was pushed: the story starts again from its base"
   ([[topic:team-records]]).

It starts no run, and the claim it writes names none. The next survey of that unit on this clone enters
the claim as its own; until then no sync removes it, the opening sync of that survey included
(T-RELEASE).

## Who may call it

A person, through the terminal shim. It is on the person-only list, so `gate-guard` refuses it from any
tool call, the main session's included ([[rule:gate-guard]]), and the command itself refuses a call that
carries a caller marker or has no terminal. Any person may take any claim.

## Writes

The claim file ([[record:work-claim]]), through one commit and push.

## Output and failure cases

The new claim, the old holder and what was pushed. `ok: false` when the scope is not claimed (start the
workflow instead), when this clone already holds it, when the sync or the push does not end well (with
its state), and when the claim changed under the command. What the old holder's clone is told, and what
it can still do, is T-HOLD.
