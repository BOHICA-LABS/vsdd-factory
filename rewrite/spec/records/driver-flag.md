---
kind: record
id: driver-flag
package: S2b
sources: [BLUEPRINT.md#4.7, state.md#2.8, state.md#2.7]
---

# Driver flag

`local/driver.json`: `on`, `paused` at a decision id, or absent; and the session. Not tracked. Fields in
[[schema:driver-flag]].

Written by `vsdd driver on`, `vsdd driver pause <decision>`, `vsdd driver off`, for the `drive` skill. Kept
until it is turned off. Cap 500 bytes.

The status report derives `driver` from it: whether the driver is `on`, `paused` (and at which decision) or
`off`, and in which session.
