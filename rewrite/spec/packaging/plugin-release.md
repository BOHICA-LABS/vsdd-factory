---
kind: packaging
id: plugin-release
package: R1
sources: [packaging.md#2.8]
---

# Releasing the plugin

A release has no binaries, no bot commit, no release branch and no back-merge (decision D9 in
[[packaging:packaging-decisions]]). A person does two things: merges a version pull request, and merges the
marketplace pull request.

```
1. uv run python tools/release.py prepare 1.0.1
      edits plugins/vsdd-factory/.claude-plugin/plugin.json  ("version")
      moves the "## [Unreleased]" entries in CHANGELOG.md under "## 1.0.1 — <date>"
   Open a normal pull request with those two files. CI must pass. Merge it.            <- decision 1: cut it

2. release.yml runs on every push to the integration branch. Its first step asks:
   does tag vsdd-factory--v<version in plugin.json> exist?  If yes, it stops in seconds.
   If no:
      verify      tools/check.py static + unit + workflows; the story tier from e2e.yml
      tag         claude plugin tag plugins/vsdd-factory --push      (creates vsdd-factory--v1.0.1)
      release     GitHub Release, notes cut from the CHANGELOG section
      marketplace pull request on bohica-labs/claude-mp setting, for the vsdd-factory entry:
                    "version": "1.0.1"
                    "source.ref": "vsdd-factory--v1.0.1"
                    "source.sha": "<the tag's 40-character commit>"

3. In bohica-labs/claude-mp the pull request runs install-check (below). Review it and merge it.   <- decision 2: publish
```

Properties:

- The marketplace entry is the only switch. Until it merges, no user can receive the release. Reverting it is the
  rollback, and needs no change here.
- The tag and its commit are pinned together, and a tag is never moved. If a release is bad, the next one has a new
  number.
- Every job is idempotent: an existing tag, release or pull request is detected and skipped. If `verify` fails, the
  fix is a normal pull request, and the push that merges it runs the release again at the new tip.
- The story tier is a model-driven gate and can fail for no good reason. A manual run of `release.yml` accepts
  `skip_e2e: true`. That is a decision by a person and is visible in the run's inputs
  ([[packaging:e2e-story-tier]]).
- A missing `CLAUDE_MP_PAT` secret fails the `marketplace` job with the two-line manual edit in its message.

The install check that guards step 3 is [[packaging:install-check]].

The first release under this procedure is `1.0.0`. There are no pre-releases of 1.0: until cutover, the new plugin
is tried with `--plugin-dir`, which needs no publishing ([[packaging:rewrite-layout]]). After 1.0.0 the version
follows semantic versioning over what a project depends on: the state layout and project configuration, the names of
skills and workflows, and the `vsdd` commands.

Delivered by package R1: `tools/release.py` and `release.yml` ([[package:R1]]).
