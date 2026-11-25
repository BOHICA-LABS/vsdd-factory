---
kind: topic
id: evidence-lookup
package: S2a
sources: [BLUEPRINT.md#4.13, state.md#2.8]
---

# Evidence lookup

`evidence(project, gate, subject)` depends on the gate. For `red` it returns the latest passing record, or
re-freeze record, on any commit between the story's base and its head; that record lists the frozen test
identities and stays valid as the implementer commits. For `green` and `demo` it returns the record for the
head commit, or one whose patch id equals the head's after a clean rebase. For `review` it returns the
verdict the tool computes from the stored passes, or the one in the closure record.

The lookup rests on the facts the evidence record carries: one record per gate and commit, each with the
`git patch-id` of the subject's diff (see [[record:evidence]]). A review verdict outlives the run that
produced it, because the tool computes it from the stored passes of any run for that unit; after the
checkpoint it is read from the closure record ([[topic:verdicts]]).

`tests-first` uses the per-gate evidence lookup. When a closure names a gate that passed at the current
commit and the evidence record is missing, the survey measures the gate again before deciding; it does not
send the stage back to an agent. It does so through the gate registry: each gate module registers its
measuring function under its gate name with the evidence store, and the survey calls the registered
function. The survey and checkpoint therefore import no gate package, and a gate that is not built yet is
simply not registered.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. For `red`, a clone with no local record reads the tracked copy of the
story's red record, under the same rule ([[record:evidence]]; [[table:work-claim-checks]], T-HANDOFF).
