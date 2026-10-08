# Troubleshooting

Common issues and their fixes when working with the VSDD factory plugin.

---

## Factory worktree not found

**Symptom:** Skills report that `.factory/` does not exist or is not a git worktree.

**Fix:** Run `/vsdd-factory:factory-health`. It will auto-create the `factory-artifacts` orphan branch
and mount the worktree. If auto-repair fails, mount manually:

```bash
# Check if the branch exists
git branch --list factory-artifacts

# If missing, create it
git checkout --orphan factory-artifacts
git rm -rf --cached . 2>/dev/null || true
git commit --allow-empty -m "chore: initialize factory-artifacts orphan branch"
git checkout -

# Mount the worktree
git worktree add .factory factory-artifacts
```

If the worktree exists but points to the wrong branch:

```bash
git worktree remove .factory --force
git worktree add .factory factory-artifacts
```

---

## Sandbox permission denied on Bash commands

**Symptom:** A hook or bin helper fails with a permission error when Claude Code's sandbox
blocks the command.

**Cause:** Claude Code sandboxes Bash commands by default. The plugin's hooks and bin helpers
use only standard tools (`git`, `jq`, `yq`, `grep`, `find`, `wc`, `bash`).

**Fix:** Approve the command when Claude Code prompts for permission. If a specific tool
is consistently blocked, check that it is installed and available on your PATH. The
`/vsdd-factory:setup-env` command reports missing tools.

---

## Template not found

**Symptom:** A skill reports it cannot find a template file.

**Cause:** Template paths use `${CLAUDE_PLUGIN_ROOT}/templates/<name>.md`. If this variable
is not set or resolves incorrectly, templates will not be found.

**Fix:**
1. Verify the plugin is installed: check that `plugins/vsdd-factory/templates/` exists
   in the expected location.
2. If running in local development mode, ensure you are using
   `claude --plugin-dir ./plugins/vsdd-factory` with the correct relative path.
3. Check that the template file actually exists. Run the plugin's template portability test:
   ```bash
   bats plugins/vsdd-factory/tests/skills.bats --filter "template actually exists"
   ```

---

## Hook blocking unexpectedly

**Symptom:** An edit or write operation is blocked by a hook when you believe it should
be allowed.

**Diagnosis:**

- **protect-vp.sh blocking:** The target file is a verification property with `Status: green`.
  Green VPs are immutable by design. To change the property, create a new VP that supersedes
  it and retire the old one.

- **protect-bc.sh blocking:** Same pattern as VPs. Green behavioral contracts are immutable.

- **brownfield-discipline.sh blocking:** You are trying to edit a file under `.reference/`.
  Reference codebases are read-only. Copy the content elsewhere if you need to modify it.

- **red-gate.sh blocking:** The Red Gate is active (`.factory/red-gate-state.json` exists
  with strict mode). You must have failing tests before writing implementation code. If
  you need to bypass this temporarily, remove or rename the state file.

**Hook output:** The `protect-vp.sh` and `protect-bc.sh` hooks emit JSON with
`permissionDecision` and `permissionDecisionReason` fields. Check the hook output for
the specific reason the edit was denied.

---

## Subagent output is empty or truncated

**Symptom:** The `handoff-validator.sh` hook warns that a subagent returned empty or
truncated output.

**Cause:** The subagent may have hit a context limit, encountered a sandbox denial on
Write, or failed silently.

**Fix:**
1. Check the subagent's task description for overly broad context. The deliver-story skill
   uses a context discipline table to limit what each specialist receives.
2. For Write denials in subagents, the plugin uses an inline delivery protocol with
   `=== FILE: <name> ===` delimiters. The parent agent extracts and writes the files.
   Check that the subagent prompt includes the inline delivery instructions.
3. Re-dispatch the subagent with a more focused task scope.

---

## Tests failing

**Symptom:** `bats plugins/vsdd-factory/tests/*.bats` reports failures.

**Prerequisites:**
- `bats-core` installed (`brew install bats-core` on macOS)
- `jq` installed and on PATH
- `yq` installed and on PATH (the Go version from mikefarah/yq)
- Run from the repository root directory

**Common issues:**

- **"command not found: yq"** -- Install yq: `brew install yq` on macOS, or download from
  the [yq releases page](https://github.com/mikefarah/yq/vsdd-factory:releases).
- **Hook tests expecting specific exit codes** -- The `protect-vp.sh` and `protect-bc.sh`
  hooks use `permissionDecision:deny` JSON output (exit 0), not exit code 2. Tests assert
  on the JSON output, not the exit code.
- **Template portability test failures** -- A skill or agent references
  `.claude/templates/` instead of `${CLAUDE_PLUGIN_ROOT}/templates/`. This is a real bug.
  Fix the reference in the skill/agent file.

---

## STATE.md out of sync

**Symptom:** STATE.md reports a different phase or status than what you observe in the
actual artifacts.

**Cause:** A skill may have been interrupted before updating STATE.md, or STATE.md was
manually edited incorrectly.

**Fix:**
1. Read the actual artifacts in `.factory/specs/`, `.factory/stories/`, and
   `.factory/cycles/` to determine the true pipeline state.
2. Edit `.factory/STATE.md` to match reality.
3. Commit the fix:
   ```bash
   cd .factory
   git add STATE.md
   git commit -m "factory(recovery): correct STATE.md to match actual pipeline state"
   ```
4. Run `/vsdd-factory:factory-health` to verify the directory structure is intact.

---

## `migrate-bc-index` exits 2 with "operator investigation required"

**Symptom:** `factory-dispatcher migrate-bc-index` exits 2 with
`COMPLETION_RECORD_MISMATCH_ABORT ... no verification was performed in this build; txn and gate unchanged; operator investigation required`,
and/or writes to BC-INDEX are blocked with `E-MAINTENANCE-001 ... (completion-record mismatch — operator investigation required)`.

**Fix:** This is a human-only recovery, performed in a terminal outside any agent session. Follow
[Recovering the `migrate-bc-index` interim block](migration-interim-block-recovery.md). Agents must
not run it.

---

## Brownfield ingest produces hallucinated findings

**Symptom:** Analysis passes contain incorrect counts, misattributed patterns, or
fabricated entity names.

**Cause:** This is a known failure mode, especially in round 1 outputs. The brownfield-ingest
skill documents 5 Known Round-1 Hallucination Classes: over-extrapolated token lists,
miscounted enumerations, named pattern conflation, same-basename artifact conflation, and
inflated/deflated metrics.

**Fix:** The protocol handles this automatically through:
1. Phase B convergence deepening audits round 1 against the known hallucination classes.
2. Phase B.6 extraction validation recounts every numeric claim with `find` + `wc -l`.
3. If you spot a hallucinated finding, note it and let the convergence rounds correct it.
   Do not manually edit pass output files.
