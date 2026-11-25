---
kind: topic
id: digests
package: S2a
sources: [state.md#2.6, state.md#2.8, BLUEPRINT.md#4.11]
---

# Digests

The digest an approval is bound to: `vsdd.state.digest(paths)`: SHA-256 over the sorted list of each file's
path and content hash, cut to 16 hex characters. It is over the whole file: approval changes no file, so
there is nothing to leave out of a file. Decision records use it.

## Which files a `spec` decision covers

The specification file set is every file in the three specification classes (`spec-product`, `spec-arch`,
`spec-ux`) except the classes `brief` and `guidance`: the brief has its own decision, and the two guidance
files (`conventions.md` and `review-rules.md`) have their own lock and may change between cycles, so
drafting a brief or editing the guidance never makes a `spec` approval stale. For `release`, the digest is
the commit.

## Staleness

`vsdd checkpoint` computes the digest of what is being decided, over the files exactly as they are. The
survey recomputes the digest, and a decision whose digest no longer matches reads as `stale` and cannot be
settled. If the content changes any other way the decision reads as stale and the stop returns.

The specification is locked from the moment the latest `spec` decision is approved, and it stays locked
until a run holds an amendment; an accepted decision record stays frozen even then. See
[[topic:spec-lock]]. The record that carries the digest is [[record:decision]].
