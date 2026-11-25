---
kind: package
id: S2b
depends_on: [S2a]
sources: [BLUEPRINT.md#9.2]
---

# S2b: Lock, caller identity and flush
<!-- generated: start -->
## Delivers

The run lock; reading caller markers; flush; the driver flag (`on`, `pause`, `off`)

## Depends on

- [[package:S2a]]

As written in the build plan: S2a

## Implements

- [[command:driver-off]]: Clears the driver flag.
- [[command:driver-on]]: Marks the driver active for this session in local/driver.json.
- [[command:driver-pause]]: Marks the driver paused at a decision.
- [[command:lock-release]]: Releases the run lock; --force is person only.
- [[record:driver-flag]]: Whether the driver is on or paused at a decision, and in which session.
- [[record:lock]]: The run lock: one per repository and branch a run can write.
- [[topic:run-lock]]: The run lock and liveness: its scope, how it is taken and released, when it is stale, and two sessions at once.
- [[topic:state-commits]]: Who commits state and when: flush, its five points, what it does, and the push.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
