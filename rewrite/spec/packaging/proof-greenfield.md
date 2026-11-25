---
kind: packaging
id: proof-greenfield
package: E3
sources: [packaging.md#layer-4-end-to-end-headless, BLUEPRINT.md#9.6, BLUEPRINT.md#5.5]
---

# The greenfield proof (E3)

Step 4 of the rewrite plan, the proof on a small project, is the story tier ([[packaging:e2e-story-tier]]) plus two
proofs; cutover waits for both ([[packaging:cutover-steps]]).

**Runs:** from an empty repository and `tests/e2e/briefs/tally.md`, started by a person and driven by
`/vsdd-factory:drive`: brief, `spec`, `plan`, the waves, `verify`, `release`. Then one trivial `feature` after the
release.

**Asserts:**

- three typed decisions and no edit of `factory.toml` by a person (the toolchain arrives with the `spec` approval);
- the scaffold story runs alone in the first wave;
- the driver resumes after each decision without being typed again;
- after the release the route is "done", and the later feature runs in delta scope;
- the prompt count for a wave in the default permission mode ([[doc:platform]], item 7);
- the share of findings first caught at a later perimeter — the escape rate ([[doc:platform]], item 21).

The proofs are also why the cutover of the eight real projects, which are all brownfield, is not gated on a
greenfield run alone ([[packaging:proof-brownfield]]).

Delivered by package E3, which depends on the workflows W1 to W7 and the skills K1 and K2 ([[package:E3]]).

Decision D33, built when the build widens: this proof runs on one clone. Team use has its own proof
([[packaging:proof-team]]), which cutover does not wait for.
