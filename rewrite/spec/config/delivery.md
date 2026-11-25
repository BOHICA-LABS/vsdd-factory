---
kind: config
id: delivery
package: S1
sources: [state.md#2.12, BLUEPRINT.md#4.12]
---
# [delivery]

Keys and defaults belong to the pipeline design; the file hosts and validates them. A `[[repo]]` entry may
carry its own `[delivery]`. With `delivery.ci = "none"`, "no checks" counts as a pass and a CI chore story
is scheduled. `delivery.protected_paths` always wait for a person. Keys, types and defaults:
[[table:config-keys]].
