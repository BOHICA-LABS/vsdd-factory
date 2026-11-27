# Releasing vsdd-factory

Canonical procedure for cutting a `v1.0.0-rc.X` (or `v1.0.0`, `v1.0.x`) release of vsdd-factory and getting it onto the [BOHICA-LABS/claude-mp](https://github.com/BOHICA-LABS/claude-mp) marketplace where operators install it.

This file is the **single source of truth**. The release skill (`/vsdd-factory:release`), the orchestrator agent, and any human operator all defer to this document. If you change the release procedure, change it here first.

## TL;DR for impatient operators

```bash
git checkout -b release/v1.0.0-rc.X origin/develop
scripts/bump-version.sh 1.0.0-rc.X "<release title>"
# Edit CHANGELOG.md to replace the stub with the real release notes
git add CHANGELOG.md && git commit -m "chore(release): CHANGELOG entry for v1.0.0-rc.X"
git push -u origin release/v1.0.0-rc.X
gh pr create --base main --head release/v1.0.0-rc.X \
  --title "release: v1.0.0-rc.X — <title>" --body "..."
# Wait for CI green, then:
gh pr merge <N> --merge --delete-branch     # NOT --squash
git fetch origin main
git tag -a v1.0.0-rc.X -m "v1.0.0-rc.X — <title>" origin/main
git push origin v1.0.0-rc.X
# Watch release.yml; verify BOHICA-LABS/claude-mp PR opens; merge it.
```

If anything in that block confuses you, **read the rest of this document** before proceeding. The shortcuts are recoveries from a known-correct mental model, not a substitute for one.

## Why this procedure exists in this exact shape

vsdd-factory has a **two-branch release model**:

- `develop` — active integration branch. Every feature/fix PR targets here.
- `main` — released versions only. Receives one merge per release (the `release/v1.0.0-rc.X` branch) plus one bot commit per release (the binary bundle).

The release pipeline (`.github/workflows/release.yml`) fires on tag push and:
1. Validates against the tag's commit (which sits on `main`)
2. Builds dispatcher binaries on 5 platforms
3. Bot checks out **`main`**, stages binaries, force-moves the tag to a new commit on `main`
4. Creates the GitHub Release as prerelease
5. Auto-opens a pull request at `BOHICA-LABS/claude-mp` that pins the catalog entry to the release tag and its commit

The pipeline assumes the tag's commit is on `main` and that `main` already has every source-file change for the release. If `main` is behind `develop`, the tarball ships with stale source against fresh binaries — exactly what produced the [v1.0.0-rc.14 marketplace failure](CHANGELOG.md#v1.0.0-rc.14--release-ci-unblocker-supersedes-rc11rc13-2026-05-09) (operators saw `E-REG-001` fail-closed on every tool call).

The `release/v1.0.0-rc.X → main` PR is **the** mechanism that brings develop's source onto main. Skip it or mis-target it, and the marketplace publish breaks.

## Mandatory invariants

The following invariants are non-negotiable. Violating any of them is what historically broke our releases.

| Invariant | What goes wrong if violated |
|-----------|----------------------------|
| Release branch is named exactly `release/v<full-semver>` | `.github/workflows/release-branch-guardrail.yml` won't match the pattern; convention drift accelerates |
| Release branch is **branched from develop**, not main | Source-file fixes from develop never reach main → marketplace ships stale source |
| Release branch PR **targets `main`**, not develop | Same outcome — main never gets develop's content. **Enforced by `release-branch-guardrail.yml`.** |
| Release PR is merged with **`--merge`** strategy, not `--squash` | Squash collapses develop's commits; main loses develop ancestry; future releases see false "diverged" warnings and trigger the [TD #68 binary auto-resolve](CHANGELOG.md#v1.0.0-rc.15--marketplace-source-sync-fix-main-backfill-2026-05-10) every time |
| Tag is created **on `main`** after the merge, at main's new tip | Tag at develop's tip works (the bot will retag) but causes bot to retag onto main's existing commit, missing the merge |
| Tag name is exactly `v<full-semver>` (no spaces, with `v` prefix) | `release.yml` only fires on `tags: ["v*"]` |
| `CHANGELOG.md` has a `## <full-semver>` heading (first version heading, directly below `## [Unreleased]`) | Validate job fails — `release.yml` checks for this |
| `develop` is the source branch tracker, not `master` | Hardcoded throughout workflows + scripts |

## Step-by-step: cutting a release

### Step 0 — Pre-flight

```bash
# Be on develop with no local changes
git checkout develop
git pull --ff-only origin develop
git status                          # must be clean

# Verify develop CI is green for the head commit
gh run list --workflow=ci --branch=develop --limit=1 --json conclusion --jq '.[0].conclusion'
# Expected: "success"
```

If develop CI is red, **fix that first**. Cutting a release from a red branch produces a release that fails its own validate job.

### Step 1 — Create the release branch

```bash
git checkout -b release/v1.0.0-rc.X origin/develop
```

Branch naming MUST be exactly `release/v<full-semver>`. The guardrail workflow uses `startsWith(github.head_ref, 'release/')` to detect release branches.

### Step 2 — Bump version + draft CHANGELOG

```bash
scripts/bump-version.sh 1.0.0-rc.X "<release title>"
```

This inserts a CHANGELOG.md stub with today's date, directly below the top-of-file `## [Unreleased]` section. **Do not commit yet** — the stub is a placeholder.

Open `CHANGELOG.md` and replace the stub with real release notes. **Drain `## [Unreleased]` first:** story PRs accumulate their changelog rows under that section between releases — move them down into the new `## <full-semver>` section (they sit directly above the stub), then leave the `## [Unreleased]` heading in place, empty, for the next cycle. Synthesize the rest of the narrative from the drained rows plus the commit log. Required sections (keep-a-changelog format):

- Headline paragraph (2-4 sentences) — what shipped, why it matters
- `### Added` (if applicable)
- `### Fixed` (if applicable)
- `### Refactored` (if applicable)
- `### Operational` — anything operators need to know (breaking? marketplace impact? cache fixes?)
- `### Deferred` — known follow-ups, TD links

Then:

```bash
git add CHANGELOG.md
git commit -m "chore(release): CHANGELOG entry for v1.0.0-rc.X"
```

### Step 3 — Push branch + open PR

```bash
git push -u origin release/v1.0.0-rc.X
gh pr create --base main --head release/v1.0.0-rc.X \
  --title "release: v1.0.0-rc.X — <release title>" \
  --body "$(cat <<'EOF'
## Summary

[2-4 sentences: what this release ships, biggest changes]

## Carried forward

[bullet list of cumulative changes since previous release on main, by source PR if useful]

## Pre-Merge Checklist

- [x] Branched from develop
- [x] Targets main
- [x] CHANGELOG entry on this branch
- [ ] CI passes
- [ ] Squash merge is FORBIDDEN — use --merge to preserve develop ancestry
- [ ] Post-merge: tag at main's new tip, watch release.yml chain
EOF
)"
```

The guardrail workflow validates `head=release/* → base=main` automatically. If you target develop by mistake, it fails the PR and tells you the fix command.

### Step 4 — Wait for CI

Watch the PR's checks. The same checks that gate ordinary feature PRs apply:

- `validate` (bats run-all.sh) — must pass
- `cargo-host` (×2: ubuntu, macos) — fmt + clippy + workspace test + perf-baseline
- `build-dispatcher` (5 platforms) — full release build per platform
- `platforms-drift`, `SAST (Semgrep)` — sanity gates
- `release-branch-guardrail` — confirms target=main

All 10 must be green before merge.

### Step 5 — Merge with `--merge` (NOT `--squash`)

```bash
gh pr merge <N> --merge --delete-branch
```

Why `--merge` matters: it preserves the develop commits (e.g. PR #112, PR #113, PR #114) as ancestors of main. After merge, `git merge-base --is-ancestor origin/main origin/develop` returns true, which means:

- Future releases' `sync-develop` job is a clean no-op
- TD #68's binary auto-resolve doesn't fire on the next release (no spurious conflicts)
- `git log --first-parent main` shows a clean release-by-release history

Squash merge breaks all three.

### Step 6 — Tag at main's new tip

```bash
git fetch origin main
NEW_HEAD=$(git rev-parse origin/main)
git tag -a v1.0.0-rc.X \
  -m "$(cat <<'EOF'
v1.0.0-rc.X — <title>

[1-2 paragraphs from CHANGELOG]

See CHANGELOG.md for full notes.
EOF
)" "$NEW_HEAD"
git push origin v1.0.0-rc.X
```

Tag at `origin/main` — not at the release branch. The PR's merge commit IS main's new tip; tagging there ensures the bot retag happens on main, not on a now-orphaned commit.

### Step 7 — Watch the release pipeline

```bash
gh run list --workflow=Release --limit=1
gh run watch <run-id>
```

The workflow runs 6 jobs in dependency order:

1. `validate` — runs bats run-all.sh against the tag's tree. Must pass.
2. `build-binaries` — 5-platform matrix (~10–50 min total wall-clock; darwin-x64 is the slowest).
3. `commit-binaries` — bot bundles binaries into a chore commit on main, force-moves the tag.
4. `release` — creates GitHub Release as prerelease.
5. `bump-marketplace` — opens a pull request at `BOHICA-LABS/claude-mp` that moves the catalog pin (`ref` = the release tag, `sha` = its commit); the catalog's own checks run on it. Requires `CLAUDE_MP_PAT` secret.
6. `sync-develop` — back-merges main → develop. Should be a clean no-op since `--merge` preserved ancestry. If it fails, see "Recovery" below.

### Step 8 — Verify the marketplace PR + merge it

```bash
gh pr list --repo BOHICA-LABS/claude-mp --state open
# Look for "vsdd-factory: pin to <tag>"
gh pr view <N> --repo BOHICA-LABS/claude-mp
# Review the diff (ref and sha of the vsdd-factory entry in marketplace.json)
gh pr merge <N> --repo BOHICA-LABS/claude-mp --squash --delete-branch
```

After merge, operators on `/plugin update vsdd-factory@claude-mp` resolve to the new version on their next plugin refresh.

### Step 9 — Verify operator install works

```bash
# In a separate Claude Code session or terminal:
/plugin update vsdd-factory@claude-mp
# Verify the cache schema is correct:
grep -n "^schema_version" \
  /Users/$USER/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hooks-registry.toml
# Expected: schema_version = 2 (or whatever the current canonical schema is)
```

If the cache shipped with stale schema (the rc.14 failure mode), see "Recovery — operator-side cache fix" below.

## Recovery procedures

### `validate` fails on the release run

`run-all.sh` is failing on `main`'s tree. Either:

1. The release branch wasn't actually a clean superset of develop — find what's missing and add it on a follow-up PR targeting main.
2. CI runner environment regression — re-run the workflow:
   ```bash
   gh run rerun <run-id> --failed
   ```

### `commit-binaries` fails — binary build issue

Bot couldn't bundle a platform's binary. Check the `build-binaries` matrix output for the failing platform; likely a toolchain or dependency issue. Fix on a follow-up PR, then either:

- Re-run the workflow (`gh run rerun <run-id>`), OR
- Force-delete the tag, fix, re-tag at the new main tip:
  ```bash
  git push origin :refs/tags/v1.0.0-rc.X
  git tag -d v1.0.0-rc.X
  # Wait for the fix PR to merge to main
  git fetch origin main
  git tag -a v1.0.0-rc.X -m "..." origin/main
  git push origin v1.0.0-rc.X
  ```

### `bump-marketplace` skipped — `CLAUDE_MP_PAT` secret missing

Job logs show "CLAUDE_MP_PAT secret is not configured. Skipping marketplace bump." Fix:

1. Generate a fine-grained PAT with `contents: write` + `pull-requests: write` scoped to `BOHICA-LABS/claude-mp`
2. Add as `CLAUDE_MP_PAT` in [vsdd-factory secrets](https://github.com/drbothen/vsdd-factory/settings/secrets/actions)
3. Re-run the workflow

### `sync-develop` fails — binary conflicts

This SHOULD no longer happen if you used `--merge` (TD #68 auto-resolve handles legitimate conflicts). If it does:

```bash
# Manually run the sync as a follow-up PR:
git checkout -b chore/sync-main-to-develop-rc.X origin/develop
git merge origin/main --no-ff -m "chore: sync main → develop (rc.X)"
# If conflicts on bundle paths only, take main's version:
git checkout origin/main -- 'plugins/vsdd-factory/hook-plugins/*.wasm' \
                            'plugins/vsdd-factory/hooks/dispatcher/bin/**'
git add plugins/vsdd-factory/hook-plugins/ plugins/vsdd-factory/hooks/dispatcher/
git commit --no-edit
git push -u origin chore/sync-main-to-develop-rc.X
gh pr create --base develop --head chore/sync-main-to-develop-rc.X \
  --title "chore: sync main → develop (rc.X bundle)"
```

### Operator-side: marketplace cache shipped with stale schema (rc.14-class failure)

The dispatcher binary expects a schema_version that doesn't match the bundled `hooks-registry.toml`. Every tool call fails with `E-REG-001`. Operator workaround until next release:

```bash
# Find the canonical schema_version expected by your dispatcher:
grep "expects" /Users/$USER/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hooks-registry.toml 2>/dev/null
# (or read the dispatcher's own constant from the rc.X CHANGELOG)

# Force the cache to match:
sed -i '' 's/^schema_version = 1$/schema_version = 2/' \
  /Users/$USER/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hooks-registry.toml
```

If you're hitting this systematically, the release was poisoned — file an issue, the fix needs to ship as a new release.

## What gets automated, what stays manual

The release workflow (`.github/workflows/release.yml`) automates everything from tag-push onward. Pre-tag is operator-driven on purpose: the human is the one who knows what's ready to ship and who writes the CHANGELOG narrative.

| Step | Who | Why |
|------|-----|-----|
| Branch from develop | Operator | Choosing the release point is a human decision |
| CHANGELOG narrative | Operator (or AI agent) | Writing release notes requires synthesis |
| PR open + merge | Operator | Code review is human-in-the-loop |
| Tag push | Operator | Tagging is the "I'm sure" gesture |
| Validate, build, retag, GitHub Release | Workflow | Mechanical |
| Marketplace bump PR | Workflow | Mechanical (but the merge of THAT PR is operator) |
| Sync develop | Workflow | Mechanical |

## How agents should consume this document

If you are an AI agent (orchestrator, devops-engineer, release skill, etc.) executing a release:

1. **Read this entire file before any action.** It's the source of truth.
2. **Honor the mandatory invariants table.** They are the wire format of the procedure; deviating breaks the marketplace.
3. **Use the step-by-step section literally.** The shell snippets are tested; substitute `1.0.0-rc.X` and the title.
4. **If you encounter a failure mode not in the recovery section, STOP and surface it to the human.** Do not improvise on the release pipeline.
5. **If you are about to update RELEASING.md itself, dispatch a separate explicit request for human review.** This file is a fast lane to making future releases worse if changed without thought.

## See also

- `.github/workflows/release.yml` — the automated half (run on tag push)
- `.github/workflows/release-branch-guardrail.yml` — enforces release/* → main
- `.factory/release-config.yaml` — pre-release commands, version source paths
- `scripts/bump-version.sh` — CHANGELOG stub generator
- `plugins/vsdd-factory/skills/release/SKILL.md` — the invokable skill (defers here)
- `CHANGELOG.md` — release history and per-release operational notes
- TD #69 — guardrail tracking ticket
