---
kind: package
id: CO2
depends_on: [E3, E4]
sources: [BLUEPRINT.md#9.6]
---

# CO2: Cutover
<!-- generated: start -->
## Delivers

Steps X0 to X8 of the packaging design: rehearse, pin the marketplace to the old tag, tag the archive, promote `next/`, merge, release 1.0.0, collapse the branches, migrate the eight projects, delete `rewrite/`

## Depends on

- [[package:E3]]
- [[package:E4]]

As written in the build plan: Everything above; E3 and E4 passed

## Implements

- [[packaging:cutover-deletions]]: What is deleted at cutover and what is kept.
- [[packaging:cutover-steps]]: Cutover of the repository and the marketplace: steps X0 to X8.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
