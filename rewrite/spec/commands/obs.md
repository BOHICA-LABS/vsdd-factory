---
kind: command
id: obs
package: T3
sources: [BLUEPRINT.md#4.9, packaging.md#2.10, packaging.md#the-observability-stack]
---

# `vsdd obs`

Operator tool that runs the observability stack ([[packaging:observability-stack]]):

- `vsdd obs up`
- `vsdd obs down`
- `vsdd obs status`
- `vsdd obs logs`
- `vsdd obs dashboard`
- `vsdd obs register`
- `vsdd obs unregister`
- `vsdd obs list`
- `vsdd obs reset`
- `vsdd obs telemetry` — writes the five environment settings that turn on Claude Code's own telemetry for a
  project into the project's `.claude/settings.local.json`.

`vsdd obs up` inside an initialised project registers the project automatically, which removes a manual step. The
stack itself is ported whole and stays opt-in; the event names it reads are the 1.0 event log's
([[topic:event-log]]). `vsdd obs` always passes `-p observability`, the compose project name.

## Who may call it

Run by the main session.

Built by package T3 ([[package:T3]]). Warns of nothing
itself: `vsdd doctor` warns about Docker, which only the observability stack needs.
