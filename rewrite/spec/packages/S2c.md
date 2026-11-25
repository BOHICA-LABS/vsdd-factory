---
kind: package
id: S2c
depends_on: [S3]
sources: [BLUEPRINT.md#9.2]
---

# S2c: Status and briefing
<!-- generated: start -->
## Delivers

The one function behind the briefing and `vsdd status`; the repairs of `status --fix`

## Depends on

- [[package:S3]]

As written in the build plan: S3

## Implements

- [[command:status]]: The read-only status report with the exact next command; with --fix, the safe repairs and a flush.
- [[schema:status]]: The object vsdd status --json prints and the briefing renders.
- [[topic:briefing]]: The session briefing: when it is printed, what it contains and its size limit.
- [[topic:status-report]]: The status report: computed from the records each time, its fields and limits, and what writes the records it reads.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
