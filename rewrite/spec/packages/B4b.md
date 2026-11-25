---
kind: package
id: B4b
depends_on: [B4a, H1]
sources: [BLUEPRINT.md#9.1]
---

# B4b: Hook wiring build
<!-- generated: start -->
## Delivers

`tools/build.py` generating `hooks/hooks.json` from the rule registry, with `--check`

## Depends on

- [[package:B4a]]
- [[package:H1]]

As written in the build plan: B4a, H1

## Implements

- [[topic:hook-wiring]]: How hooks.json is generated and what it wires: the self-guarding command, the matcher, and the events deliberately left unwired.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
