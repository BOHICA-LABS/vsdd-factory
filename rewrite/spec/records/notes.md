---
kind: record
id: notes
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.7]
---

# Notes

`notes.json` at the state root. Tracked. Up to five notes a person asked to keep, each free text of at most
200 characters; the oldest is dropped. Fields in [[schema:notes]].

Written by `vsdd note "<text>"`, run by the main session on request. Notes are the one part of the status
report that is not derived from other records: `vsdd status` reads the file and the briefing lists the notes.
