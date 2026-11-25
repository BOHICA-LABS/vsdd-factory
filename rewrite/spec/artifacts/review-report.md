---
kind: artifact
id: review-report
package: C2
sources: [state.md#2.5]
---

# Review report
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

A review inside the pipeline writes no file: its structured result is the review (a step record), and only
that feeds a verdict. A person reads findings with `vsdd review show <unit>` ([[command:review-show]]). The
exception is a walled finding, whose full form stays in the evaluation file
([[topic:walled-findings]]).

The `-pass[N]`/`-final` naming of the old reports becomes `<lens>-<n>.md`; there is no `-final`
([[table:identifiers]]).
