---
kind: command
id: lock-release
package: S2b
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.11, packaging.md#2.10]
---

# `vsdd lock release`

Releases the run lock; `--force` is person only.

## Syntax

`vsdd lock release [--force]`

## What it does

"Releases the run lock (2.11). `vsdd status` shows the lock" ([[topic:run-lock]], [[record:lock]]).
Packaging's row: "Breaks the project lock that stops two sessions changing pipeline state at once. `vsdd
status` shows the lock, and `vsdd status --fix` releases one whose holder is dead, so there is no `lock
status`. Taking the lock is automatic and belongs to the state design."

Taking and releasing otherwise belong to the workflow: "vsdd survey takes it once its preconditions pass",
"vsdd checkpoint releases it. So does the hook runner when the holder's session ends".

What each form releases:

- `vsdd lock release` releases a lock that the caller's own session holds (the session is read from the
  caller marker, [[topic:caller-identity]]) and a lock that is stale. It refuses any other lock with
  `ok: false` and the holder.
- `vsdd lock release --force` releases any lock. Only a person may run it, in their own terminal.

Releasing a lock ends the run that held it ([[topic:run-lock]]).

## Who may call it

The command table puts `lock release` in the main-session row and `lock release --force` in the
person-only row ("the person: ... otherwise in their own terminal"); through the shim it is one of the
person-only commands that "always run the code of the version this project uses". The status design sends a
person who knows the holding session is gone to "`vsdd lock release` in a terminal once the shim is
installed".

## Writes

Removes or replaces `.factory/local/lock.json` ([[record:lock]]).

## Output and failure cases

Released, or `ok: false` with the holder when another session's live run holds the lock; `--force` is for
the case where a person knows that session is gone and will not wait for the lock to go stale. While a
flush is running, the release cannot end the runs: `ok: false`, a kept entry named as the holder, and an
error that says a flush is running. While the run lock itself is held by another release or a survey, the
error says the run lock is busy instead — no flush is involved and no entry was matched
(finding step3-records-fix-3-C1-3).
