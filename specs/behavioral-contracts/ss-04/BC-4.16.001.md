---
document_type: behavioral-contract
level: L3
version: "1.10"
status: active
producer: product-owner
timestamp: 2026-07-19T00:00:00Z
phase: brownfield-backfill
cycle: v1.0-brownfield-backfill
inputs:
  - .factory/cycles/v1.0-brownfield-backfill/e-21-arch-delta-analysis.md
  - plugins/vsdd-factory/hooks/factory-branch-guard.sh
  - plugins/vsdd-factory/hooks-registry.toml
  - .factory/specs/behavioral-contracts/ss-04/BC-4.13.001.md
  - .factory/specs/architecture/ARCH-INDEX.md
input-hash: "2e55c4a"
traces_to: .factory/specs/architecture/ARCH-INDEX.md
origin: brownfield
extracted_from: null
subsystem: "SS-04"
capability: "CAP-034"
lifecycle_status: active
introduced: v1.0-brownfield-backfill
modified:
  - "2026-10-08 (v1.10) — Target-directory branch resolution amendment for [D-1254-HOOK-FP-003] (product-owner; GH #840 family; S-26.02): NEW Invariant 7 — the guard resolves the branch of the git TARGET directory (explicit `-C`/`--work-tree`/`core.worktree` value, a preceding `cd`, or the guarded Bash call's working directory), never the hook process cwd; Invariant 3 and PC1 amended; NEW EC-015..EC-018 and T-10..T-13. No change to the detection contract, the conservative path forms, PC2-PC4, fail-open or the `factory-artifacts` pass rule."
  - "2026-08-13 (v1.9) — POL-14 auto-promotion completion + frontmatter status-field parity fix (state-manager; S-21.09 PR #775 merged to develop `2e8087af` 2026-08-13T14:16:26Z; post-merge processing burst D-991): top-level `status:` field draft→active, completing the POL-14 promotion that the v1.8 burst (2026-07-23, S-21.01 PR #759 merge) applied only to `lifecycle_status:` (already `active` since v1.8) — the two fields drifted out of parity since 2026-07-23; this burst closes that gap. No behavioral/content change — Description/Preconditions/Postconditions/Invariants/Edge Cases/Canonical Test Vectors byte-identical to v1.8. BC-INDEX v4.56→v4.57 (BC-4.16.001 row version-chain cell v1.8→v1.9; status column UNCHANGED, already `active`)."
  - "2026-07-23 (v1.8) — POL-14 auto-promotion at merge (state-manager; S-21.01 PR #759 squash-merged 7bb0e797 2026-07-23): lifecycle_status draft→active. BC-INDEX v4.19→v4.20."
  - "2026-07-23 (v1.7) — Accepted Residuals record (product-owner; S-21.01 pass-6/7 adversarial cascade; documentary-only — no behavioral requirement changes): two accepted residuals added inside Invariant 6: (1) trailing-slash -C .factory/ target form not classified factory-class — rare OVER-block via CWD fallback; conservative direction, accepted (S-21.01 pass-6 sub-NITPICK); (2) single-detection-first-target semantics — chained payload with benign-first + dangerous-second factory-class target under-matched; accepted: faithful to Invariant 6 singular-target contract, near-zero realism, outside issue #342 threat model, Precondition 4 neutralizes single-mount variants (S-21.01 pass-7 NITPICK). BC-INDEX v4.18→v4.19."
  - "2026-07-23 (v1.6) — F-P5-001 spec-tightening amendment (product-owner; S-21.01 LOCAL cascade pass 5; human gate decision 2026-07-23): Invariant 6 added (target-aware branch detection for -C and -c core.worktree= forms: when consumed value names a .factory-class path, branch detection runs against target directory via git -C <target> branch --show-current; product branch → BLOCK; factory-artifacts → PASS; failure → fail-open per Invariant 3); Precondition 2 note added (target INSPECTION of already-consumed values, not re-classification as arguments); §Description extended; EC-012/EC-013/EC-014 added; T-7/T-8/T-9 added; sub-NITPICK --super-prefix dedicated test deferred to test-writer. BC-INDEX v4.17→v4.18."
  - "2026-07-22 (v1.5) — F-P4-001 + F-P4-002 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 4): Precondition 2 detection contract made class-complete (value-consuming git global option set enumerated: -C, -c, --git-dir, --work-tree, --namespace, --super-prefix, --exec-path; conservative rule for unknown dash-prefix tokens; F-P4-002: leading shell-punctuation stripping on git candidate tokens). §Description + PC4 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.16→v4.17."
  - "2026-07-19 (v1.1) — CAP-034 backfill + crate name correction (product-owner; ADR-031 §Decision 3): capability frontmatter TBD→CAP-034; all 10 occurrences of wrong crate name `validate-artifact-path` replaced with `validate-factory-path-staging` (TD-VSDD-060 sibling-site sweep: H1 title, §Description, Precondition 3, Invariants 2/4, SDK Grounding Evidence Grep 1/3, §Traceability Architecture Module, §Architecture Anchors ×2); ADR Reference row added; Capability Anchor Justification updated to cite CAP-034/ADR-031; BC-4.11.001 note added to §Related BCs."
  - "2026-07-19 (v1.2) — Research validation precision amendments (product-owner; research validation 2026-07-19): SDK Grounding Evidence Grep 2 expected-output corrected (comment-line hit on line 15 is expected; conclusion derives from absence of intercept pattern, not zero hits); Precondition 4 added (environmental scope: guard's primary protective value is on unmounted checkouts; mounted-worktree silent-no-op behavior documented)."
  - "2026-07-22 (v1.4) — F-P2-002 + F-P2-003 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 2): Precondition 2 detection contract expanded to cover global-option forms (git -C <path> add, git --no-pager add, git -c key=val add); behavioral description 'any token sequence beginning git whose first non-option subcommand token is add or stage, tolerating any number of intervening global options or flags'; minimum form git\\s+(add|stage) retained; chained-form applicability explicit (&&, ;, |). Invariant 4: .factory path detection case-insensitive (conservative blocking on macOS HFS+/Windows NTFS). §Description + PC4 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.15→v4.16."
  - "2026-07-22 (v1.3) — F-P1-003 + F-P1-001 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 1): Precondition 2 matcher git\\s+add extended to git\\s+(add|stage) per ADR-031 §Decision 2 Layer-1 (git stage is a true git synonym; bypassing creates identical INV-E21-001 dual-tracking condition); Invariant 4 conservative-blocking enumeration extended with bare .factory (no trailing slash), ./ (CWD-relative with explicit slash), and :/‑family pathspec-magic arguments. H1 title + §Description + PC4 + EC-009 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.14→v4.15."
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
bc_id: BC-4.16.001
section: "4.16"
last_amended: "2026-10-08 (v1.10) — Target-directory branch resolution amendment for [D-1254-HOOK-FP-003]: NEW Invariant 7, Invariant 3 and PC1 amended, NEW EC-015..EC-018 and T-10..T-13 (product-owner; S-26.02). Prior: (v1.9) — POL-14 auto-promotion completion + frontmatter status-field parity fix (state-manager; S-21.09 PR #775 merged to develop `2e8087af` 2026-08-13T14:16:26Z; post-merge processing burst D-991): top-level `status:` field draft→active, completing the POL-14 promotion the v1.8 burst applied only to `lifecycle_status:` (already active since v1.8) — the two fields had drifted out of parity since 2026-07-23; this burst closes that gap. No behavioral/content change. BC-INDEX v4.56→v4.57. [Prior: (v1.8) — POL-14 auto-promotion at merge (state-manager; S-21.01 PR #759 squash-merged 7bb0e797 2026-07-23): lifecycle_status draft→active. BC-INDEX v4.19→v4.20. [Prior: (v1.7) — Accepted Residuals record (product-owner; S-21.01 pass-6/7 adversarial cascade; documentary-only — no behavioral requirement changes): two accepted residuals added inside Invariant 6: (1) trailing-slash -C .factory/ target form not classified factory-class — rare OVER-block via CWD fallback; conservative direction, accepted (S-21.01 pass-6 sub-NITPICK); (2) single-detection-first-target semantics — chained payload with benign-first + dangerous-second factory-class target under-matched; accepted: faithful to Invariant 6 singular-target contract, near-zero realism, outside issue #342 threat model, Precondition 4 neutralizes single-mount variants (S-21.01 pass-7 NITPICK). BC-INDEX v4.18→v4.19. [Prior: (v1.6) — F-P5-001 spec-tightening amendment (product-owner; S-21.01 LOCAL cascade pass 5; human gate decision 2026-07-23): Invariant 6 added (target-aware branch detection for -C and -c core.worktree= forms); Precondition 2 note added (target INSPECTION of already-consumed values, not re-classification); §Description extended; EC-012/EC-013/EC-014 added; T-7/T-8/T-9 added; sub-NITPICK --super-prefix dedicated test deferred to test-writer. BC-INDEX v4.17→v4.18. [Prior: (v1.5) — F-P4-001 + F-P4-002 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 4): Precondition 2 detection contract made class-complete (value-consuming git global option enumeration: -C, -c, --git-dir, --work-tree, --namespace, --super-prefix, --exec-path; conservative rule for unknown dash-prefix tokens; F-P4-002: leading shell-punctuation stripping on git candidate tokens). §Description + PC4 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.16→v4.17. [Prior: (v1.4) — F-P2-002 + F-P2-003 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 2): Precondition 2 detection contract expanded to cover global-option forms; behavioral description 'any token sequence beginning git whose first non-option subcommand token is add or stage, tolerating intervening global options'; minimum form git\\s+(add|stage) retained; chained-form applicability explicit. Invariant 4: .factory path detection case-insensitive (conservative blocking). §Description + PC4 sibling-site sweep (TD-VSDD-060); BC-INDEX v4.15→v4.16. (v1.3) — F-P1-003 + F-P1-001 spec-tightening amendments; Precondition 2 matcher git\\s+(add|stage); Invariant 4 enumeration extended with bare .factory, ./, :/‑family; H1 + §Description + PC4 + EC-009 sibling-site sweep (TD-VSDD-060); BC-INDEX v4.14→v4.15. (v1.2) — Research validation precision amendments; Precondition 4 environmental scope note added. (v1.1) — CAP-034 backfill + crate name correction; TD-VSDD-060 sibling-site sweep. (v1.0) — Initial authoring; validate-factory-path-staging WASM guard; INV-E21-001 invariant layer. lifecycle_status: draft (POL-14).]]"
---

# BC-4.16.001: validate-factory-path-staging WASM PreToolUse guard MUST block any `git add` or `git stage` command that stages a path under `.factory/` on a product branch, and MUST pass all non-`.factory/` staging commands unconditionally

## Description

The `validate-factory-path-staging` native-WASM plugin enforces the nested-worktree path-exclusivity
invariant (INV-E21-001): no path under the `.factory/` physical directory may be tracked on a
product branch (`develop`, `main`, `feature/*`, `release/*`, `maintenance/*`). It fires on every
`PreToolUse` event where the Bash tool payload contains a `git add` or `git stage` command in
any form — bare (`git add`, `git stage`), with global options before the subcommand (e.g.,
`git -C <path> add`, `git --no-pager add`, `git -c key=val add`), or where `git` is glued to
leading shell punctuation (`$(git add ...`, `(git add ...`, or backtick-git forms — the
detector strips leading `$(`, `(`, or a leading backtick from candidate tokens before matching)
— anywhere in the payload including chained forms (`&&`, `;`, `|`). (`git stage` is a true git synonym for `git add` and
creates the identical dual-tracking condition.) On invocation, the plugin inspects the staged
paths in the command: if any path matches `.factory/**` (or `\.factory/` as a component, matched
case-insensitively to cover case-folding filesystems such as macOS HFS+ and Windows NTFS —
e.g., `.Factory/STATE.md` matches) AND the current working branch is a product branch (not
`factory-artifacts`), the plugin emits `block_intent = true` (exit code 2) with an actionable
error. When the command uses a `-C <target>` or `-c core.worktree=<target>` option where
`<target>` is a `.factory`-class path (bare `.factory`, `./.factory`, or any absolute/relative
form ending in `/.factory`, matched case-insensitively), branch detection is performed against
the target directory rather than the CWD (Invariant 6); this preserves the state-manager's
canonical `git -C .factory add ...` workflow on the mounted `factory-artifacts` worktree (target
branch = `factory-artifacts` → PASS) while blocking the same command on an unmounted checkout
(target branch = product branch → BLOCK). All other `git add`/`git stage` commands (in any form)
and all non-`git add`/`git stage` Bash commands pass unconditionally.

This is the **invariant layer** for issue #342. It prevents the dual-tracking condition that makes
product-branch merges dangerous: if no `.factory/` path is ever staged on a product branch, the
git working-tree update triggered by a product-branch merge/checkout cannot delete a path the
nested `.factory/` worktree is serving. The companion **safety-net layer** (BC-5.43.001) intercepts
product-branch merges that would clobber `.factory/` paths even if the invariant layer is bypassed
or preceded by a prior accidental staging.

This BC governs the `validate-factory-path-staging` WASM crate at
`crates/hook-plugins/validate-factory-path-staging/` (new crate to be created by S-21.01; named
per ADR-031 §Decision 3 to avoid collision with `crates/hook-plugins/validate-artifact-path/`
which serves BC-4.11.001 since S-13.01). POLICY 21 (no new shell scripts) is satisfied: the guard
is a native WASM plugin, not a `.sh` file.

## Preconditions

1. A `PreToolUse` event has fired for the `Bash` tool.

2. The Bash tool payload contains a `git add` or `git stage` command. Detection contract: any
   token sequence in the payload beginning with a `git` candidate token — where a candidate
   token is any token that equals `git` after stripping leading shell punctuation (`$(`, `(`,
   or a leading backtick; F-P4-002 — conservative strip) — whose first non-option subcommand
   token is `add` or `stage`. To determine the subcommand token, the detector MUST consume the
   value token of every value-consuming git global option; the canonical value-consuming options
   whose value is a SPACE-SEPARATED subsequent token are: `-C <path>`, `-c <name=value>`,
   `--git-dir <path>`, `--work-tree <path>`, `--namespace <path>`, `--super-prefix <path>`,
   and `--exec-path <path>` (when space-separated; `=`-joined forms such as
   `--git-dir=/foo` are self-contained and do NOT consume the next token as their value). For
   any UNKNOWN dash-prefixed token (a token beginning with `-` that is not in the canonical
   set above and not a known non-value-consuming flag), the detector MUST apply the conservative
   rule: treat it as non-value-consuming and let any subsequent standalone `add` or `stage` token
   in the same command segment trigger detection. Under-match is forbidden; conservative
   over-match is acceptable. The minimum form `git\s+(add|stage)` (substring match,
   case-insensitive) covers the common bare invocation and MUST be retained as a fast path.
   This matcher applies anywhere in the payload, including in chained and sequential command
   forms (`&&`, `;`, `|`) — a `git add` or `git stage` appearing after a semicolon,
   ampersand-pair, or pipe is in scope. `git stage` is a true git synonym for `git add` and
   presents the identical INV-E21-001 dual-tracking vector per ADR-031 §Decision 2 Layer-1. All
   other Bash payloads — including `git commit`, `git push`, `git checkout`, `git merge`, and
   all non-git commands — are NOT in scope for this plugin and pass unconditionally.

   **Precondition 2 note — target-aware inspection (Invariant 6):** The value-consuming detection
   above (v1.5: consuming the token after `-C`, `-c`, `--work-tree`, etc.) continues to apply for
   subcommand determination. When the consumed value for `-C` or the value component of a
   `-c core.worktree=<path>` option names a `.factory`-class path, the guard additionally
   INSPECTS that target directory to determine its branch (Invariant 6). This is target INSPECTION
   of already-consumed values — not a re-classification of those values as staging-path arguments.

   **Sub-NITPICK (F-P5-001):** A dedicated test vector for the `--super-prefix` value-consuming
   option is deferred to test-writer (AC-006 proptest expansion scope).

3. The dispatcher has invoked the `validate-factory-path-staging` WASM plugin. The plugin is
   registered in `hooks-registry.toml` as a PreToolUse handler for `^Bash$` with
   `on_error = "continue"` (fail-open: a crashed or timed-out guard never blocks the session).

4. **Environmental scope:** On checkouts where `.factory/` IS mounted as a live git worktree
   (the standard developer machine layout), `git add .factory/<path>` (plain and `-f`) is typically
   a silent no-op — git declines to stage paths that belong to another worktree. Dual-tracking can
   therefore only ORIGINATE on checkouts where the `.factory/` worktree is NOT mounted: fresh CI
   checkouts, bare or shallow clones, or commits made before the initial `git worktree add` call.
   This plugin's primary protective value is on those unmounted checkout environments; it converts
   a silent failure into an explicit, actionable block. On mounted checkouts it provides defense-in-depth
   against edge cases (detached HEAD, partial teardown, misconfigured worktree).

## Postconditions

### PC1 — Blocked: `git add`/`git stage` of `.factory/` path on a product branch

When the Bash payload contains `git add` or `git stage` AND at least one argument (expanded path
or glob) resolves to a path matching `^\.factory/` (or containing `/.factory/` as a path
component, capturing the unnested form), AND the guard's branch detection reports a product branch
(any branch other than `factory-artifacts`):

1. The plugin returns `block_intent = true` (exit code 2).
2. The plugin emits a blocking error message:
   ```
   BLOCKED: git add/stage of .factory/ path on product branch '<branch>'.
   .factory/ paths are exclusively owned by the factory-artifacts worktree.
   Staging .factory/ content on a product branch creates the dual-tracking condition
   that allows product-branch merges to silently delete factory artifact files.
   If you intended to commit factory artifacts: switch to the .factory/ worktree and
   commit from there on the factory-artifacts branch.
   ```
3. The `git add`/`git stage` command is NOT executed.

(v1.10) `<branch>` in the message is the branch of the git TARGET directory resolved per Invariant 7,
never the branch of the hook process's working directory; the branch the guard evaluates and the
branch it reports are the same.

**Error variant:** `FactoryPathOnProductBranch`

### PC2 — Passed: `git add`/`git stage` of non-`.factory/` paths

When the Bash payload contains `git add` or `git stage` AND no argument resolves to a
`.factory/`-rooted path, the plugin returns `block_intent = false` (exit code 0). The command
proceeds normally.

### PC3 — Passed: `git add`/`git stage` on `factory-artifacts` branch

When the Bash payload contains `git add` or `git stage` AND the current branch is `factory-artifacts`, the plugin
MUST pass unconditionally regardless of the paths being staged. Factory artifact commits require
staging `.factory/` paths on the `factory-artifacts` branch — that is correct behavior. The plugin
MUST NOT block legitimate factory-artifacts branch staging.

### PC4 — Passed: non-`git add`/`git stage` Bash command

When the Bash payload does NOT contain a `git add` or `git stage` command in any form — bare,
with global options (e.g., `git -C <path> add`, `git --no-pager add`), or with leading shell
punctuation stripped (`$(git add ...`, `(git add ...`, backtick-git forms) — anywhere in the
payload including chained forms — (regardless of paths), the plugin returns `block_intent =
false` (exit code 0) immediately without path inspection.

## Invariants

1. **INV-E21-001 (Nested Worktree Path Exclusivity) — instantiation:** No `git add` or `git stage`
   command executed via the Bash tool on a product branch may stage a path under `.factory/`. This
   invariant prevents the dual-tracking condition: `.factory/` paths tracked on a product branch
   are a P0 data-loss landmine (issue #342 root cause).

2. **Fail-open on crash:** A crashed or timed-out `validate-factory-path-staging` plugin MUST
   return `block_intent = false` (pass-through). The guard MUST be registered with
   `on_error = "continue"`. A broken guard is disruptive but never wedges the session.

3. **Branch detection source:** The plugin detects the product branch OF THE GIT TARGET DIRECTORY
   (Invariant 7; v1.10 — formerly "the current product branch", which the implementation read from the
   hook process cwd) via `host::exec_subprocess` (`git -C <target> branch --show-current` or
   equivalent). If branch detection fails (git unavailable, detached
   HEAD, non-zero exit), the plugin MUST fail-open (pass-through). Uncertain branch state is NOT
   a blocking condition.

4. **Path matching is conservative and case-insensitive:** The path pattern `.factory/` is
   matched as a literal path prefix or path component, **case-insensitively** (e.g.,
   `.Factory/STATE.md` and `.FACTORY/STATE.md` are treated as matching `.factory/`; macOS HFS+
   and Windows NTFS are case-folding filesystems where `git add .Factory/STATE.md` targets the
   same file as `git add .factory/STATE.md` and MUST be blocked conservatively). Glob expansions
   in the `git add`/`git stage` argument are NOT
   evaluated by the `validate-factory-path-staging` plugin at PreToolUse time (git has not yet
   run); the plugin inspects only the literal argument text. The following argument forms are
   treated as conservatively blocking when the detected branch is a product branch:
   - `-A` / `--all` / `-u` / `--update` flags (may stage tracked `.factory/` paths)
   - `.` (CWD-relative; may expand to `.factory/**` when CWD is the project root or `.factory/`)
   - `./` (CWD-relative with explicit slash; semantically identical to `.` for staging purposes)
   - bare `.factory` without trailing slash (git expands this to `.factory/**`, identical staging scope)
   - glob patterns (e.g., `*.md`) that may expand to include `.factory/**` paths
   - `:/`-family pathspec-magic arguments (e.g., `:/`, `:/path`) which anchor from the repository
     root and can include `.factory/` paths regardless of CWD
   Any of the above forms blocks with a conservative `FactoryPathOnProductBranch` message. (Per
   ADR-031 §Decision 2 Layer-1: scope is narrow — `git add`/`git stage` only; `git pull`/`git
   merge` interception is Layer-2's responsibility and MUST NOT be added to this guard.)

5. **No overlap with BC-4.13.001:** `verify-factory-lock` (BC-4.13.001) guards against concurrent
   write races on Edit/Write/MultiEdit/Agent tool calls using the lock mechanism. This plugin
   guards against a different surface (Bash-tool `git add`/`git stage` of `.factory/` paths on
   product branches). Both plugins may fire on the same dispatch; they are additive guards, not
   substitutes.

6. **Target-aware branch detection for `-C` and `-c core.worktree=` forms:** When Precondition 2
   identifies a `git add`/`git stage` command that contains a `-C <target>` option or a
   `-c core.worktree=<target>` option (via the `-c name=value` global option form) where `<target>`
   names a `.factory`-class path (bare `.factory`, `./.factory`, absolute or relative forms ending
   in `/.factory`, matched case-insensitively), the plugin MUST perform branch detection against
   the TARGET directory by executing `git -C <target> branch --show-current` (via
   `host::exec_subprocess`), rather than relying solely on CWD-based detection:
   - Target branch is a product branch → **BLOCK** with `FactoryPathOnProductBranch` (PC1).
   - Target branch is `factory-artifacts` → **PASS** unconditionally (PC3). This is the
     state-manager's canonical mounted-worktree workflow: `git -C .factory add ...` on the
     `factory-artifacts` worktree MUST NOT be blocked.
   - Target branch detection fails (non-zero exit, empty output, or git unavailable) →
     **fail-open** with advisory warning, per Invariant 3 semantics.

   **Human authorization:** human gate decision 2026-07-23 (S-21.01 pass-5 F-P5-001
   "Fix: target-aware detection").

   **Accepted Residuals (S-21.01 pass-6/7; documentary-only; no behavioral requirement
   changes):**
   1. **Trailing-slash `-C .factory/` target form not classified factory-class.** Invariant 6's
      enumeration of `.factory`-class paths (bare `.factory`, `./.factory`, absolute/relative
      forms ending in `/.factory`) deliberately omits the trailing-slash variant
      (e.g., `-C .factory/`). Consequence: a command using the trailing-slash form falls through
      to CWD-based detection rather than target-aware detection, producing a rare OVER-block
      (conservative direction) on a mounted `factory-artifacts` worktree when the operator uses
      a non-canonical trailing-slash path. Conservative direction — accepted. (S-21.01 pass-6
      sub-NITPICK.)
   2. **Single-detection-first-target semantics (chained payload under-match).** The guard
      performs at most ONE `host::exec_subprocess` invocation for target branch detection per
      payload (single-invocation constraint). A chained payload with a benign-first and a
      dangerous-second factory-class target — e.g.,
      `git -C .factory add x && git -C /other/.factory add y` — is under-matched on the second
      target. Accepted: faithful to Invariant 6's singular-target contract; near-zero realism
      (requires two distinct `.factory` mounts in the same payload); outside issue #342's threat
      model; Precondition 4 neutralizes single-mount variants. (S-21.01 pass-7 NITPICK.)

7. **Target-directory branch resolution (v1.10; closes the BC leg of [D-1254-HOOK-FP-003]; GH #840
   family; S-26.02).** The branch that PC1, PC3 and the block message are evaluated against is the
   branch of the git TARGET directory T of the `git add`/`git stage` invocation — the work tree the
   command stages in — NOT the branch of the hook process's working directory. (Defect record: the
   plugin's `exec_subprocess` runs in the dispatcher's session project root, so before v1.10 only the
   Invariant 6 forms looked at any other directory; a `git add` aimed at a feature-branch worktree from
   a session rooted on `develop` was judged on `develop`, and a `git add` aimed at the
   `factory-artifacts` worktree through a non-`.factory`-class path was judged on the session branch.)
   T is resolved from the payload, in this order, and the FIRST that applies wins:
   1. the value of the explicit `-C <path>` global option of the (first) `git add`/`git stage`
      invocation — ANY path, not only a `.factory`-class one (Invariant 6 is the special case of this
      rule where the value is `.factory`-class); successive `-C` options compose as git does (each
      relative to the previous);
   2. otherwise the value of `--work-tree <path>` / `--work-tree=<path>` or of `-c core.worktree=<path>`;
   3. otherwise the directory named by the last `cd <dir>` / `pushd <dir>` that precedes the git
      invocation in the same payload (`cd <worktree> && git add …`), resolved relative to the working
      directory of the next rule;
   4. otherwise the working directory of the guarded Bash call itself (the directory the shell will run
      the command in — the worktree being staged) when the hook input carries it;
   5. only when none of 1-4 is available: the hook process's own working directory (the pre-v1.10
      behavior, retained solely as the last-resort fallback).
   Branch detection is `git -C <T> branch --show-current` through `host::exec_subprocess`, EXACTLY ONE
   invocation per payload (the Invariant 6 residual 2 single-invocation constraint is kept; T is taken
   from the first `git add`/`git stage` invocation). If detection on the resolved T fails (T is not a git
   work tree, does not exist, git exits non-zero, empty output / detached HEAD, git unavailable) the
   plugin FAILS OPEN with an advisory warning per Invariant 3 and MUST NOT fall back to the hook-process
   branch (that fallback is the defect). The branch named in the PC1 block message is the branch of T.
   What Invariant 7 does NOT change: the Precondition 2 detection contract; the Invariant 4 conservative
   path, bulk-flag (`-A`, `--all`, `-u`, `.`, glob) and `.factory/`-argument forms and their EC-004 /
   EC-008 / EC-010 verdicts (the staged-path predicate that would relax them is a separate amendment owed
   to S-26.02 AC-001..AC-003); PC2-PC4; a `.factory/` path staged on a product-branch T is blocked
   exactly as before; a T on `factory-artifacts` passes unconditionally (PC3).

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | `git add .factory/STATE.md` on `develop` branch | BLOCKED: PC1 fires; `FactoryPathOnProductBranch` error emitted |
| EC-002 | `git add .factory/STATE.md` on `factory-artifacts` branch | PASSED: PC3 fires unconditionally |
| EC-003 | `git add src/main.rs` on `develop` branch | PASSED: PC2 fires; no `.factory/` path |
| EC-004 | `git add -A` run from inside `.factory/` directory (CWD = `.factory/`) | BLOCKED: plugin detects `-A` or `.` as potentially staging CWD-relative paths under `.factory/`; conservative block with PC1 error |
| EC-005 | `git add src/main.rs .factory/STATE.md` (mixed) on `develop` | BLOCKED: ANY `.factory/` match is sufficient to block the whole command |
| EC-006 | Branch detection fails (detached HEAD) | PASSED: fail-open per Invariant 3; staging command proceeds |
| EC-007 | Plugin crashes or times out | PASSED: fail-open per `on_error = "continue"` (Invariant 2) |
| EC-008 | `git add` with glob `*.md` from project root on `develop` | BLOCKED: conservative — glob may expand to `.factory/**/*.md`; plugin blocks; user must re-run with explicit non-`.factory/` paths |
| EC-009 | `git commit --amend` (no `git add`/`git stage`) | PASSED: PC4 fires; non-`git add`/`git stage` command |
| EC-010 | `git add -u` on `feature/S-21.01` branch | BLOCKED: `-u` stages all tracked modifications; conservative block since CWD or tracked files may include `.factory/` paths |
| EC-011 | `$(git add .factory/STATE.md)` on `develop` branch (shell-subst with `git` glued to `$(`) | BLOCKED: detection strips leading `$(` from candidate token; `git` token recognized; PC1 fires (F-P4-002 conservative strip) |
| EC-012 | `git -C .factory add STATE.md` on `develop` where `.factory/` is NOT mounted as a worktree (unmounted checkout — CI, fresh clone) | BLOCKED: Invariant 6 fires — `-C .factory` targets a `.factory`-class path; guard runs `git -C .factory branch --show-current` → `develop` (product branch); PC1 fires; `FactoryPathOnProductBranch` error |
| EC-013 | `git -C .factory add STATE.md` where `.factory/` IS mounted as the `factory-artifacts` worktree | PASSED: Invariant 6 fires — guard runs `git -C .factory branch --show-current` → `factory-artifacts`; PC3 passes unconditionally |
| EC-014 | `git -c core.worktree=.factory add STATE.md` on `develop` | BLOCKED: Invariant 6 fires — `core.worktree=.factory` value names a `.factory`-class path; target branch detection → product branch; PC1 fires |
| EC-015 | (v1.10; Invariant 7; [D-1254-HOOK-FP-003]) FEATURE-BRANCH WORKTREE staged from a session whose hook-process cwd (project root) is on `develop`: the staging target T is a worktree `W` (e.g. `/repo/.worktrees/S-25.09`) whose branch is `feature/S-25.09`, reached by (a) `git -C W add crates/hook-plugins/x/src/lib.rs`, (b) `cd W && git add crates/hook-plugins/x/src/lib.rs`, (c) a Bash call whose own working directory is `W` running `git add crates/hook-plugins/x/src/lib.rs`; and the same three forms with `.factory/STATE.md` as the path | (a)(b)(c) with the `crates/…` path: PASSED — PC2 (no `.factory/` argument), never blocked and never reported against `develop`. The three forms with `.factory/STATE.md`: BLOCKED by PC1, and the message names `product branch 'feature/S-25.09'` — NOT `develop` (the branch of the target, Invariant 7). The verdict never depends on the branch the SESSION is rooted on |
| EC-016 | (v1.10; Invariant 7 + PC3) FACTORY-ARTIFACTS WORKTREE staged through a non-`.factory`-class path from a session rooted on `develop`: (a) `cd /repo/.factory && git add -A` (T = the `factory-artifacts` worktree via the `cd`); (b) a Bash call whose working directory is the `factory-artifacts` worktree running `git add -A`; (c) `git -C /repo/.factory-wt add STATE.md` where `/repo/.factory-wt` is a mounted `factory-artifacts` worktree whose path is not `.factory`-class | PASSED: Invariant 7 resolves T to the `factory-artifacts` worktree; PC3 passes unconditionally (the conservative `-A` of (a)(b) is moot on `factory-artifacts`). Before v1.10 these were judged on the session's `develop` and wrongly blocked |
| EC-017 | (v1.10; Invariant 7) GENUINE `develop` STAGING of `.factory/` paths, no regression: (a) `git add .factory/STATE.md` with T = the project root on `develop` (no `-C`, no `cd`, Bash working directory = project root); (b) `git -C /repo add .factory/STATE.md` with `/repo` on `develop`; (c) `cd /repo && git add .factory/stories/S-26.02.md`; (d) the same as (a) with T on `release/v1.0.0-rc.26` | (a)–(d): BLOCKED by PC1 with `product branch 'develop'` (`'release/v1.0.0-rc.26'` for (d)) — the target IS the product branch, so the guard's true-positive case is intact (CAP-034 / INV-E21-001 enforcement) |
| EC-018 | (v1.10; Invariant 7 fail-open and single-invocation) (a) `git -C /nonexistent add .factory/STATE.md`; (b) `git -C /tmp/not-a-repo add .factory/STATE.md`; (c) T detached HEAD; (d) a chained payload `git -C W1 add a.rs && git -C W2 add .factory/x.md` (W1 on `feature/a`, W2 on `develop`); (e) `-C` values composed: `git -C /repo -C .worktrees/S-25.09 add .factory/STATE.md` | (a)(b)(c): PASSED — fail-open per Invariant 3 with an advisory warning; the guard does NOT fall back to the session branch. (d): ONE branch detection, on the FIRST invocation's T (`W1`, `feature/a`); the second target is under-matched exactly as Invariant 6 residual 2 accepts (the block, if any, is judged against W1). (e): T = `/repo/.worktrees/S-25.09` (composition as git does) ⇒ BLOCKED with `product branch 'feature/S-25.09'` |

## Canonical Test Vectors

| Test # | Precondition | Command (Bash payload) | Expected Result |
|--------|-------------|------------------------|----------------|
| T-1 | Branch = `develop`; path = `.factory/STATE.md` | `git add .factory/STATE.md` | BLOCKED: `FactoryPathOnProductBranch` error |
| T-2 | Branch = `factory-artifacts`; path = `.factory/STATE.md` | `git add .factory/STATE.md` | PASSED: PC3 |
| T-3 | Branch = `feature/S-21.01`; path = `src/lib.rs` | `git add src/lib.rs` | PASSED: PC2 |
| T-4 | Branch = `develop`; payload = `git add -A` | `git add -A` | BLOCKED: conservative PC1 |
| T-5 | Branch = `release/v1.0.0-rc.24`; path = `.factory/stories/S-21.01.md` | `git add .factory/stories/S-21.01.md` | BLOCKED: PC1 (release branch is a product branch) |
| T-6 | Plugin crashes mid-invocation | `git add src/main.rs` | PASSED: fail-open |
| T-7 | Branch = `develop`; `.factory/` unmounted; `-C` targeting `.factory/` | `git -C .factory add STATE.md` | BLOCKED: Invariant 6; target branch = `develop`; `FactoryPathOnProductBranch` |
| T-8 | `.factory/` mounted as `factory-artifacts`; `-C` targeting `.factory/` | `git -C .factory add STATE.md` | PASSED: Invariant 6; target branch = `factory-artifacts`; PC3 |
| T-9 | Branch = `develop`; `-c core.worktree=.factory` targeting `.factory/` | `git -c core.worktree=.factory add STATE.md` | BLOCKED: Invariant 6; target branch = `develop`; `FactoryPathOnProductBranch` |
| T-10 | (v1.10, EC-015) Hook-process cwd (project root) on `develop`; target worktree `/repo/.worktrees/S-25.09` on `feature/S-25.09` | `git -C /repo/.worktrees/S-25.09 add crates/hook-plugins/x/src/lib.rs` (also `cd /repo/.worktrees/S-25.09 && git add crates/hook-plugins/x/src/lib.rs`, and the same bare command with the Bash call's working directory = the worktree) | PASSED: PC2 — not blocked, never judged on `develop` |
| T-11 | (v1.10, EC-015) Same setup as T-10 | `git -C /repo/.worktrees/S-25.09 add .factory/STATE.md` (and the `cd`/working-directory forms) | BLOCKED: PC1; message names `product branch 'feature/S-25.09'` — NOT `develop` |
| T-12 | (v1.10, EC-016) Hook-process cwd on `develop`; Bash working directory (or `cd` target) = the mounted `factory-artifacts` worktree `/repo/.factory` | `cd /repo/.factory && git add -A` (and `git add -A` with that working directory) | PASSED: Invariant 7 resolves T to `factory-artifacts`; PC3 |
| T-13 | (v1.10, EC-017) Genuine `develop` staging, hook-process cwd and target both `develop` (no `-C`, no `cd`) | `git add .factory/STATE.md` | BLOCKED: PC1; `product branch 'develop'` (the true-positive case, identical to T-1) |

## SDK Grounding Evidence

**Grep 1 — WASM crate stub location (to be created by S-21.01):**
```
ls crates/hook-plugins/validate-factory-path-staging/ 2>/dev/null || echo "PLANNED — not yet created"
```
Expected: "PLANNED — not yet created" at authoring time; crate created by S-21.01 implementation.

**Grep 2 — factory-branch-guard.sh scope (does NOT guard Bash `git add` or `git stage`):**
```
grep -n "git add" plugins/vsdd-factory/hooks/factory-branch-guard.sh
```
Expected: one comment-line hit — line 15: `#   state-manager commits: Bash tool for git add/commit in .factory/`.
This is a descriptive comment, not an interception pattern. The conclusion holds: factory-branch-guard.sh
contains no `git add` or `git stage` interception logic. The gap this BC closes is confirmed by the
absence of any blocking pattern (conditional branch, exit call, or intercept logic) keyed on a
`git add` or `git stage` match — NOT by zero grep hits. The comment's presence confirms the author
was aware of the `git add` surface; the absence of a guard pattern confirms it was left unguarded.
(`git stage` is an alias pointing to the same git plumbing — the same analysis applies.)

**Grep 3 — hooks-registry.toml entry for validate-factory-path-staging:**
```
grep -n "validate-factory-path-staging" plugins/vsdd-factory/hooks-registry.toml
```
Expected: entry present (added by S-21.01) or absent before implementation.

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| (TBD — to be assigned by state-manager after VP authoring pass) | Block fires on `git add`/`git stage` of `.factory/X` when branch != `factory-artifacts` | bats: invoke guard with mocked branch=develop; payload=`git add .factory/STATE.md`; assert exit 2 + `FactoryPathOnProductBranch` |
| (TBD) | Pass unconditionally when branch == `factory-artifacts` | bats: invoke guard with branch=factory-artifacts; payload=`git add .factory/STATE.md`; assert exit 0 |
| (TBD) | Fail-open on crash | unit: force panic in guard body; assert dispatcher continues with `block_intent=false` |
| (TBD) | Path-pattern coverage: proptest | proptest: fuzz .factory/ path variants (with components, root-relative, abs path); assert block fires on all |

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-034 |
| Capability Anchor Justification | CAP-034 (Nested Worktree Path Exclusivity) per ARCH-INDEX v3.07 / ADR-031 §Decision 3. This BC is the primary enforcement mechanism for CAP-034's invariant layer: the `validate-factory-path-staging` WASM guard blocks the `git add`/`git stage` surface that creates the dual-tracking condition. BC-5.43.001 is the safety-net layer of the same CAP-034 capability. |
| L2 Domain Invariants | none (operational infrastructure) |
| Architecture Module | `crates/hook-plugins/validate-factory-path-staging/` (new crate; to be created by S-21.01; ADR-031 §Decision 3 naming authority) |
| Stories | S-21.01 (E-21 Wave 1); S-26.02 (E-26; v1.10 Invariant 7 target-directory branch resolution, EC-015..EC-018, T-10..T-13 — [D-1254-HOOK-FP-003]; the staged-path predicate AC-001..AC-003 remain "pending BC-4.16.001 amendment") |
| Source Issues | #342 (product-branch merge silently rm's a `.factory/` file) |
| ADR Reference | ADR-031 §Decision 3 (authoritative crate naming: `validate-factory-path-staging.wasm`; `crates/hook-plugins/validate-artifact-path/` serves BC-4.11.001 Edit/Write/MultiEdit path validation since S-13.01 and cannot be reused — registry filename + cargo output collisions) |

## Related BCs

- BC-4.13.001 — sibling guard on same SS-04 surface; governs Edit/Write/MultiEdit/Agent lock enforcement; orthogonal (different tool surface, different invariant)
- BC-4.11.001 — the existing BC that owns `crates/hook-plugins/validate-artifact-path/` (Edit/Write/MultiEdit path validation since S-13.01); NOT the same crate as this BC; naming distinction enforced by ADR-031 §Decision 3
- BC-5.43.001 — safety-net layer companion; governs orchestrator pre-merge intersection check that defends against dual-tracking even if this guard was bypassed (issue #342 defense-in-depth)

## Architecture Anchors

- `crates/hook-plugins/validate-factory-path-staging/` — WASM plugin source (new crate; to be created by S-21.01; ADR-031 §Decision 3 naming authority)
- `plugins/vsdd-factory/hooks-registry.toml` — registry entry for `validate-factory-path-staging` (to be added by S-21.01)
- `plugins/vsdd-factory/hooks/factory-branch-guard.sh` — existing guard that covers Edit/Write surface but NOT Bash `git add`/`git stage`; grandfathered per E-20 scope; this BC closes the `git add`/`git stage` gap without touching that script

## Story Anchor

S-21.01 (E-21 Wave 1 — factory artifact path guard: prevent dual-tracking and intercept product-branch merges that would clobber `.factory/` paths)

S-26.02 (E-26 — validate-factory-path-staging false positives, GH #840; v1.10): delivers Invariant 7
(target-directory branch resolution), the Invariant 3 and PC1 wording, EC-015..EC-018 and T-10..T-13
([D-1254-HOOK-FP-003]). Note for the architect/implementer: the SDK `HookPayload` currently carries no
working-directory field, and `host::exec_subprocess` takes no working-directory argument (it runs in the
dispatcher's session project root); Invariant 7 rule 4 (the Bash call's working directory) therefore needs
the host input `cwd` forwarded to the plugin, while rules 1-3 and the `git -C <T>` form need no host change.

## VP Anchors

TBD — VP IDs to be assigned after VP authoring pass.

## Changelog

| Version | Date | Description |
|---------|------|-------------|
| 1.10 | 2026-10-08 | **Target-directory branch resolution — closes the BC leg of [D-1254-HOOK-FP-003] (GH #840 family; attached to S-26.02; product-owner).** Defect (source read, `crates/hook-plugins/validate-factory-path-staging/src/lib.rs` `hook_logic` Step 3): the branch is detected by `git branch --show-current` run through `host::exec_subprocess`, whose working directory is the dispatcher's session project root (`resolve_project_cwd`), NOT the directory the guarded `git add` acts on; only the Invariant 6 forms (`-C`/`-c core.worktree=` naming a `.factory`-class path) use `git -C <target> branch --show-current`. Consequence: a `git add` run in or aimed at a feature-branch worktree (`git -C <worktree> add …`, `cd <worktree> && git add …`, or a Bash call whose own working directory is the worktree) from a session rooted on `develop` was judged on `develop` (the block message reported `develop` while the staging target was on `feature/S-25.09`); symmetrically a `git add` aimed at the `factory-artifacts` worktree through a non-`.factory`-class path was judged on the session branch instead of passing PC3. Amendment: (1) NEW Invariant 7 "Target-directory branch resolution": the branch is that of the git TARGET directory T, resolved in the order explicit `-C <path>` (any path, not only `.factory`-class; successive `-C` compose), `--work-tree`/`-c core.worktree=` value, a `cd`/`pushd <dir>` preceding the git invocation in the same payload, the guarded Bash call's working directory when the hook input carries it, and only then the hook process cwd as the last-resort fallback; detection is `git -C <T> branch --show-current`, exactly ONE `host::exec_subprocess` per payload (the Invariant 6 residual 2 constraint is kept); a failure to detect on the resolved T fails OPEN per Invariant 3 (never falling back silently to the session branch, which was the defect). (2) Invariant 3 amended: its "current product branch" is the branch of T. (3) PC1 message `<branch>` is the branch of T. (4) NEW EC-015..EC-018 and canonical test vectors T-10..T-13. Unchanged: the Precondition 2 detection contract, Invariant 4 conservative path/bulk-flag forms (so `git add -A` etc. keep their EC-004/EC-008/EC-010 verdicts until the separate staged-path predicate amendment owed to S-26.02 AC-001..AC-003), PC2-PC4, Invariants 1, 2, 5, 6 and their Accepted Residuals, fail-open, the `factory-artifacts` pass rule. No VP-count change. **Stories affected by BC changes:** S-26.02 (story-writer must add an AC for target-directory branch resolution and cite BC-4.16.001 under `bc_array_changes_propagate_to_body_and_acs`; the plugin source is not changed by this BC edit). **VP citations changed in:** none. |
| 1.9 | 2026-08-13 | POL-14 auto-promotion completion + frontmatter status-field parity fix (state-manager; S-21.09 PR #775 merged to develop `2e8087af` 2026-08-13T14:16:26Z; post-merge processing burst D-991). Top-level `status:` field draft→active, completing the POL-14 promotion the v1.8 burst applied only to `lifecycle_status:` (already active since v1.8) — the two fields had drifted out of parity since 2026-07-23. No behavioral/content change. BC-INDEX v4.56→v4.57. |
| 1.8 | 2026-07-23 | POL-14 auto-promotion at merge (state-manager; S-21.01 PR #759 squash-merged 7bb0e797 2026-07-23). lifecycle_status draft→active. BC-INDEX v4.19→v4.20. |
| 1.7 | 2026-07-23 | Accepted Residuals record (product-owner; S-21.01 pass-6/7 adversarial cascade; documentary-only — no behavioral requirement changes). Two accepted residuals recorded in Invariant 6: (1) trailing-slash `-C .factory/` target forms not classified factory-class (Invariant 6 enumeration omits trailing-slash variant) — consequence is rare OVER-block via CWD fallback on mounted worktrees with non-canonical trailing-slash form; conservative direction, accepted (S-21.01 pass-6 sub-NITPICK); (2) single-detection-first-target semantics: guard performs at most ONE target branch detection per payload (`exec_subprocess` single-invocation constraint); chained payload with benign-first + dangerous-second factory-class target (`git -C .factory add x && git -C /other/.factory add y`) under-matched on second target; accepted: faithful to Invariant 6 singular-target contract, near-zero realism (requires two distinct `.factory` mounts), outside issue #342 threat model, Precondition 4 neutralizes single-mount variants (S-21.01 pass-7 NITPICK). BC-INDEX v4.18→v4.19. |
| 1.6 | 2026-07-23 | F-P5-001 spec-tightening amendment (product-owner; S-21.01 LOCAL cascade pass 5; human gate decision 2026-07-23). F-P5-001 [LOW][spec-gap]: Target-aware branch detection for `-C` and `-c core.worktree=` forms — when a consumed `-C <target>` or `-c core.worktree=<target>` value names a `.factory`-class path, branch detection MUST run against the target directory (`git -C <target> branch --show-current`); product branch → BLOCK; `factory-artifacts` → PASS (state-manager mounted-worktree canonical); detection failure → fail-open per Invariant 3. Invariant 6 added. Precondition 2 note added (target INSPECTION of already-consumed values, not re-classification as arguments). §Description extended with target-aware detection behavior. EC-012 (`git -C .factory add` on unmounted develop → BLOCKED), EC-013 (mounted factory-artifacts → PASSED), EC-014 (`-c core.worktree=.factory` on develop → BLOCKED) added. T-7/T-8/T-9 added. Sub-NITPICK: dedicated test for `--super-prefix` value-consuming option deferred to test-writer. BC-INDEX v4.17→v4.18. |
| 1.5 | 2026-07-22 | F-P4-001 + F-P4-002 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 4). F-P4-001 [MEDIUM]: Precondition 2 detection contract made class-complete — detector MUST consume the value token of every value-consuming git global option before selecting the subcommand token; canonical value-consuming set enumerated (`-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace`, `--super-prefix`, `--exec-path` when space-separated; `=`-joined forms self-contained); conservative rule for UNKNOWN dash-prefix tokens: treat as non-value-consuming, any subsequent standalone `add`/`stage` in same command segment triggers detection (under-match forbidden; conservative over-match acceptable). F-P4-002 [LOW]: candidate `git` tokens glued to leading shell punctuation (`$(git`, `(git`, backtick-git) — detection strips leading shell punctuation from candidate tokens (conservative). §Description + PC4 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.16→v4.17. |
| 1.4 | 2026-07-22 | F-P2-002 + F-P2-003 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 2). Precondition 2 detection contract expanded: behavioral contract stated as "any token sequence beginning `git` whose first non-option subcommand token is `add` or `stage`, tolerating any number of intervening global options or flags"; covers `git -C <path> add`, `git --no-pager add`, `git -c key=val add` (Layer-1 bypasses via common invocations); minimum form `git\s+(add\|stage)` retained; chained-command-form applicability made explicit (`&&`, `;`, pipe). Invariant 4: `.factory` path detection is case-insensitive (conservative blocking on case-folding filesystems: macOS HFS+, Windows NTFS — e.g., `.Factory/STATE.md` matches). §Description + PC4 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.15→v4.16. |
| 1.3 | 2026-07-22 | F-P1-003 + F-P1-001 spec-tightening amendments (product-owner; S-21.01 LOCAL cascade pass 1). Precondition 2 matcher extended `git\s+add` to `git\s+(add\|stage)` to cover `git stage` (true git synonym; same dual-tracking vector) per ADR-031 §Decision 2 Layer-1. Invariant 4 conservative-blocking enumeration extended with bare `.factory` (no trailing slash), `./` (CWD-relative with explicit slash), and `:/`-family pathspec-magic arguments alongside existing `-A`/`--all`/`-u`/`--update`/`.`/glob forms. H1 title + §Description + PC4 + EC-009 sibling-site sweep (TD-VSDD-060). BC-INDEX v4.14→v4.15. |
| 1.2 | 2026-07-19 | Research validation precision amendments (product-owner; research validation 2026-07-19). SDK Grounding Evidence Grep 2 expected-output corrected: grep hits line 15 comment (not zero hits); conclusion reworded to derive from absence of intercept pattern, not zero hits. Precondition 4 added: environmental scope note (mounted-worktree silent-no-op behavior; guard primary protective value on unmounted checkouts — CI, clone, pre-mount commits). |
| 1.1 | 2026-07-19 | CAP-034 backfill + crate name correction (product-owner; ADR-031 §Decision 3). All 10 occurrences of wrong crate name `validate-artifact-path` replaced with `validate-factory-path-staging` (TD-VSDD-060 sibling-site sweep: H1 title, §Description ×2, Precondition 3, Invariant 2/4 ×2, SDK Grounding Evidence Grep 1/3, §Traceability Architecture Module, §Architecture Anchors ×2). §Traceability ADR Reference row added. §Traceability Capability Anchor Justification updated to cite CAP-034 / ARCH-INDEX v3.07 / ADR-031 §Decision 3. §Related BCs: BC-4.11.001 note added clarifying `validate-artifact-path` crate ownership. |
| 1.0 | 2026-07-19 | Initial authoring (product-owner; E-21 factory-state data-loss hardening; issue #342). validate-factory-path-staging WASM PreToolUse Bash guard — block `git add` of `.factory/` paths on product branches (PC1), pass all other commands (PC2/PC3/PC4). INV-E21-001 instantiation (invariant layer). 4 error variants: `FactoryPathOnProductBranch` (PC1), fail-open (Invariants 2/3). 10 edge cases EC-001..EC-010. 6 canonical test vectors T-1..T-6. lifecycle_status: draft (POL-14 auto-promotion on S-21.01 PR merge). |
