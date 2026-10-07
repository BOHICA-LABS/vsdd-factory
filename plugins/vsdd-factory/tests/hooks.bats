#!/usr/bin/env bats
# hooks.bats — TAP tests for vsdd-factory hooks
#
# Asserts allow and block paths for every hook. Uses bats-core. Install:
#   brew install bats-core
#
# Run from plugin root:
#   bats tests/hooks.bats

setup() {
  PLUGIN_ROOT="$(cd "$BATS_TEST_DIRNAME/.." && pwd)"
  HOOKS="$PLUGIN_ROOT/hooks"
  WORK="$(mktemp -d)"
  cd "$WORK"
  mkdir -p .factory/specs/verification-properties
  mkdir -p .factory/specs/behavioral-contracts
  mkdir -p .reference/someproject
}

teardown() {
  rm -rf "$WORK"
}

# ---------- brownfield-discipline ----------

@test "brownfield-discipline: allows edit outside .reference/" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/brownfield-discipline.sh"'
  [ "$status" -eq 0 ]
}

@test "brownfield-discipline: blocks edit to .reference/" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".reference/foo/bar.md\"}}" | "'"$HOOKS"'/brownfield-discipline.sh"'
  [ "$status" -eq 2 ]
  [[ "$output" == *"read-only"* ]]
}

@test "brownfield-discipline: blocks nested .reference path" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"/abs/path/.reference/x.rs\"}}" | "'"$HOOKS"'/brownfield-discipline.sh"'
  [ "$status" -eq 2 ]
}

@test "brownfield-discipline: allows empty input" {
  run bash -c 'echo "{}" | "'"$HOOKS"'/brownfield-discipline.sh"'
  [ "$status" -eq 0 ]
}

# ---------- protect-vp ----------

@test "protect-vp: allows edit to non-VP file" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/protect-vp.sh"'
  [ "$status" -eq 0 ]
}

@test "protect-vp: allows edit to new VP file (not yet on disk)" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/verification-properties/VP-999.md\"}}" | "'"$HOOKS"'/protect-vp.sh"'
  [ "$status" -eq 0 ]
}

@test "protect-vp: allows edit to draft VP (no green status)" {
  echo "# VP-001" > .factory/specs/verification-properties/VP-001.md
  echo "Status: draft" >> .factory/specs/verification-properties/VP-001.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/verification-properties/VP-001.md\"}}" | "'"$HOOKS"'/protect-vp.sh"'
  [ "$status" -eq 0 ]
}

@test "protect-vp: blocks edit to green VP" {
  echo "# VP-001" > .factory/specs/verification-properties/VP-001.md
  echo "Status: green" >> .factory/specs/verification-properties/VP-001.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/verification-properties/VP-001.md\"}}" | "'"$HOOKS"'/protect-vp.sh"'
  [ "$status" -eq 0 ]
  [[ "$output" == *'"permissionDecision":"deny"'* ]]
  [[ "$output" == *"immutable"* ]]
  [[ "$output" == *"supersede"* ]]
}

# ---------- protect-bc ----------

@test "protect-bc: allows edit to non-BC file" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/protect-bc.sh"'
  [ "$status" -eq 0 ]
}

@test "protect-bc: allows edit to draft BC" {
  echo "# BC-1.01.001" > .factory/specs/behavioral-contracts/BC-1.01.001.md
  echo "Status: draft" >> .factory/specs/behavioral-contracts/BC-1.01.001.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/behavioral-contracts/BC-1.01.001.md\"}}" | "'"$HOOKS"'/protect-bc.sh"'
  [ "$status" -eq 0 ]
}

@test "protect-bc: blocks edit to green BC" {
  echo "# BC-1.01.001" > .factory/specs/behavioral-contracts/BC-1.01.001.md
  echo "Status: green" >> .factory/specs/behavioral-contracts/BC-1.01.001.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/behavioral-contracts/BC-1.01.001.md\"}}" | "'"$HOOKS"'/protect-bc.sh"'
  [ "$status" -eq 0 ]
  [[ "$output" == *'"permissionDecision":"deny"'* ]]
  [[ "$output" == *"immutable"* ]]
  [[ "$output" == *"supersede"* ]]
}

# ---------- red-gate ----------

@test "red-gate: allows when state file absent" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

@test "red-gate: allows test files unconditionally" {
  echo '{"mode":"strict","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"tests/foo_test.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

@test "red-gate: allows when mode is off" {
  echo '{"mode":"off","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

@test "red-gate: blocks in strict mode when file not in red list" {
  echo '{"mode":"strict","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 2 ]
  [[ "$output" == *"red list"* ]]
}

@test "red-gate: allows in strict mode when file is in red list" {
  echo '{"mode":"strict","red":["src/lib.rs"]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

@test "red-gate: ignores unknown extensions" {
  echo '{"mode":"strict","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"README.md\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

# ---------- red-gate: absolute paths ----------

@test "red-gate: allows absolute path when in red list (PWD-stripped)" {
  echo '{"mode":"strict","red":["src/lib.rs"]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'"$WORK"'/src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

@test "red-gate: blocks absolute path when not in red list" {
  echo '{"mode":"strict","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'"$WORK"'/src/lib.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 2 ]
}

@test "red-gate: allows absolute test path unconditionally" {
  echo '{"mode":"strict","red":[]}' > .factory/red-gate-state.json
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'"$WORK"'/tests/foo_test.rs\"}}" | "'"$HOOKS"'/red-gate.sh"'
  [ "$status" -eq 0 ]
}

# ---------- purity-check ----------

@test "purity-check: no-op on non-pure path" {
  mkdir -p src
  echo 'fn main() { println!("hi"); }' > src/main.rs
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/main.rs\"}}" | "'"$HOOKS"'/purity-check.sh"'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "purity-check: warns on side effects in pure path" {
  mkdir -p crates/core/src/pure
  echo 'use std::fs;' > crates/core/src/pure/thing.rs
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"crates/core/src/pure/thing.rs\"}}" | "'"$HOOKS"'/purity-check.sh"'
  [ "$status" -eq 0 ]
  [[ "$output" == *"side-effect"* ]]
}

@test "purity-check: clean pure file emits nothing" {
  mkdir -p src/pure
  echo 'pub fn add(a: i32, b: i32) -> i32 { a + b }' > src/pure/math.rs
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/pure/math.rs\"}}" | "'"$HOOKS"'/purity-check.sh"'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

# ---------- verify-git-push ----------

@test "verify-git-push: allows push to feature branch" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin feature/STORY-001\"}}" | "'"$HOOKS"'/verify-git-push.sh"'
  [ "$status" -eq 0 ]
}

@test "verify-git-push: allows push to factory-artifacts" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin factory-artifacts\"}}" | "'"$HOOKS"'/verify-git-push.sh"'
  [ "$status" -eq 0 ]
}

@test "verify-git-push: allows push with -u (set upstream)" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push -u origin my-branch\"}}" | "'"$HOOKS"'/verify-git-push.sh"'
  [ "$status" -eq 0 ]
}

@test "verify-git-push: blocks push to main" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin main\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"BLOCKED"* ]]
  [[ "$output" == *"protected branch"* ]]
}

@test "verify-git-push: blocks push to master" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin master\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"protected branch"* ]]
}

@test "verify-git-push: blocks push to develop" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin develop\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"protected branch"* ]]
}

@test "verify-git-push: blocks force push" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push --force origin feature/x\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"BLOCKED"* ]]
  [[ "$output" == *"Force push"* ]]
}

@test "verify-git-push: blocks force push with -f flag" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin feature/x -f\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
}

@test "verify-git-push: allows non-push git commands" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git status\"}}" | "'"$HOOKS"'/verify-git-push.sh"'
  [ "$status" -eq 0 ]
}

@test "verify-git-push: block message suggests PR workflow" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin main\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [[ "$output" == *"gh pr create"* ]]
}

@test "verify-git-push: allows --force-with-lease (safe force push)" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push --force-with-lease origin feature/x\"}}" | "'"$HOOKS"'/verify-git-push.sh"'
  [ "$status" -eq 0 ]
}

@test "verify-git-push: blocks -f at end of command" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git push origin feature/x -f\"}}" | "'"$HOOKS"'/verify-git-push.sh" 2>&1'
  [ "$status" -eq 2 ]
}

# ---------- check-factory-commit ----------

@test "check-factory-commit: allows non-commit commands" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git status\"}}" | "'"$HOOKS"'/check-factory-commit.sh"'
  [ "$status" -eq 0 ]
}

@test "check-factory-commit: allows commit outside .factory" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"git commit -m fix\"}}" | "'"$HOOKS"'/check-factory-commit.sh"'
  [ "$status" -eq 0 ]
}

@test "check-factory-commit: warns on .factory commit without STATE.md" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"cd .factory && git commit -m artifacts\"}}" | "'"$HOOKS"'/check-factory-commit.sh"'
  [ "$status" -eq 0 ]
  [[ "$output" == *"STATE.md"* ]]
}

@test "check-factory-commit: silent when STATE.md is in commit" {
  run bash -c 'echo "{\"tool_input\":{\"command\":\"cd .factory && git add STATE.md && git commit -m update\"}}" | "'"$HOOKS"'/check-factory-commit.sh"'
  [ "$status" -eq 0 ]
  [[ "$output" != *"STATE.md was not updated"* ]]
}

# ---------- factory-branch-guard ----------

@test "factory-branch-guard: allows non-.factory paths" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"src/main.rs\"}}" | "'"$HOOKS"'/factory-branch-guard.sh"'
  [ "$status" -eq 0 ]
}

@test "factory-branch-guard: blocks .factory/ write when no worktree" {
  # .factory/ exists as plain dir (created in setup) but no .git marker
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/specs/prd.md\"}}" | "'"$HOOKS"'/factory-branch-guard.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"BLOCKED"* ]]
  [[ "$output" == *"not mounted as a git worktree"* ]]
}

@test "factory-branch-guard: block message includes recovery command" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/STATE.md\"}}" | "'"$HOOKS"'/factory-branch-guard.sh" 2>&1'
  [[ "$output" == *"git worktree add"* ]]
}

@test "factory-branch-guard: allows .factory/ write when worktree exists" {
  # Simulate worktree by creating .git marker file
  echo "gitdir: /fake/path" > .factory/.git
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/specs/prd.md\"}}" | "'"$HOOKS"'/factory-branch-guard.sh"'
  [ "$status" -eq 0 ]
}

@test "factory-branch-guard: allows empty file path" {
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"\"}}" | "'"$HOOKS"'/factory-branch-guard.sh"'
  [ "$status" -eq 0 ]
}

@test "factory-branch-guard: allows missing file path" {
  run bash -c 'echo "{\"tool_input\":{}}" | "'"$HOOKS"'/factory-branch-guard.sh"'
  [ "$status" -eq 0 ]
}

# ---------- validate-count-propagation (#690 id-drop + #567 historical exclusion) ----------
#
# The hook uses associative arrays (declare -A), which require bash 4+.
# It runs via its #!/bin/bash shebang, so the interpreter that matters is
# /bin/bash — bash 5.x on the ubuntu `validate` CI job, but 3.2 on macOS
# where these cases skip (the hook's own pre-existing requirement, not a
# limitation of the fix).
require_bash4_hook_interp() {
  local maj
  maj=$(/bin/bash -c 'echo ${BASH_VERSINFO[0]}')
  [[ "$maj" -ge 4 ]] || skip "hook requires bash 4+ (declare -A); /bin/bash is ${maj}.x"
}

@test "validate-count-propagation: epic-id token is not parsed as a count (#690)" {
  require_bash4_hook_interp
  # STATE.md references an epic by id next to a countable noun ("5 E-11 stories").
  # The "11" belongs to the E-11 identifier, not a claimed count of 11 stories.
  # STORY-INDEX carries the genuine, mutually-consistent story count.
  printf '# STATE\nPhase 3 complete -> 5 E-11 stories delivered.\n' > .factory/STATE.md
  printf '# STORY-INDEX\nThis corpus has 42 stories total.\n' > .factory/STORY-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
}

@test "validate-count-propagation: genuine count drift still blocks" {
  require_bash4_hook_interp
  printf '# STATE\nDelivered 13 stories this phase.\n' > .factory/STATE.md
  printf '# STORY-INDEX\nThis corpus has 42 stories total.\n' > .factory/STORY-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"COUNT DRIFT DETECTED"* ]]
}

@test "validate-count-propagation: mixed line keeps genuine count after dropping id" {
  require_bash4_hook_interp
  # A genuine count (13) shares a line with an epic id (E-11); dropping the id
  # must not swallow the real quantity, so drift against the index still fires.
  printf '# STATE\nPhase 3: E-11 delivered 13 stories.\n' > .factory/STATE.md
  printf '# STORY-INDEX\nThis corpus has 42 stories total.\n' > .factory/STORY-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"13 stories"* ]]
}

@test "validate-count-propagation: file with no genuine counts exits clean" {
  require_bash4_hook_interp
  # After identifier tokens are dropped, an all-phantom line leaves no counts.
  # This path must exit 0, not trip set -u on the empty associative array.
  printf '# STATE\nPhase 3 complete -> 5 E-11 stories delivered.\n' > .factory/STATE.md
  printf '# STORY-INDEX\nNo countable nouns here.\n' > .factory/STORY-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "validate-count-propagation: long dotted-id line stays fast and keeps trailing count" {
  require_bash4_hook_interp
  # Dense dotted-id lines are normal in index tables. The id-drop pattern must
  # stay linear on them: a nested-extglob variant of the same drop takes >30s
  # on this 550-char line (per-line, under PostToolUse). Also asserts the ids
  # are dropped whole and the genuine trailing count survives extraction.
  local ids
  ids="$(printf 'BC-1.11.11 %.0s' $(seq 1 50))"
  printf '# STATE\n%s41 BCs\n' "$ids" > .factory/STATE.md
  printf '# BC-INDEX\ntotal_bcs: 41\n' > .factory/BC-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
}

# ---------- validate-bc-title ----------

@test "validate-bc-title: title lookup scopes to the Title-header table (#566)" {
  # The BC id appears first in a capability-satisfaction table (adjacent cell
  # "CAP-001") before the §2 navigation table. The nav table title agrees with
  # the H1, so this edit must pass — the matcher must not grab the CAP-001 cell.
  local bc_dir=".factory/specs/behavioral-contracts"
  cat > "$bc_dir/BC-2.01.001.md" <<'EOF'
# BC-2.01.001: Drop-In SSH_AUTH_SOCK Replacement
EOF
  cat > "$bc_dir/BC-INDEX.md" <<'EOF'
# BC-INDEX

## 1. Capability Satisfaction

| BC | Satisfies | Notes |
|----|-----------|-------|
| BC-2.01.001 | CAP-001 | primary |

## 2. Navigation

| BC | Title | Subsystem | Priority |
|----|-------|-----------|----------|
| BC-2.01.001 | Drop-In SSH_AUTH_SOCK Replacement | SS-02 | P0 |
EOF
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/behavioral-contracts/BC-2.01.001.md\"}}" | "'"$HOOKS"'/validate-bc-title.sh" 2>&1'
  [ "$status" -eq 0 ]
}

@test "validate-bc-title: genuine H1/index drift in the nav table still blocks" {
  local bc_dir=".factory/specs/behavioral-contracts"
  cat > "$bc_dir/BC-2.01.001.md" <<'EOF'
# BC-2.01.001: Drop-In SSH_AUTH_SOCK Replacement
EOF
  cat > "$bc_dir/BC-INDEX.md" <<'EOF'
# BC-INDEX

## 1. Capability Satisfaction

| BC | Satisfies | Notes |
|----|-----------|-------|
| BC-2.01.001 | CAP-001 | primary |

## 2. Navigation

| BC | Title | Subsystem | Priority |
|----|-------|-----------|----------|
| BC-2.01.001 | Stale Divergent Title | SS-02 | P0 |
EOF
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/behavioral-contracts/BC-2.01.001.md\"}}" | "'"$HOOKS"'/validate-bc-title.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"bc_h1_index_drift"* ]]
  [[ "$output" == *"Stale Divergent Title"* ]]
}

@test "validate-bc-title: headerless single-row index falls back and still blocks" {
  # No Title-header table present — preserve prior behavior (first-occurrence
  # fallback) so a genuine drift in a minimal index is still caught.
  local bc_dir=".factory/specs/behavioral-contracts"
  cat > "$bc_dir/BC-1.01.001.md" <<'EOF'
# BC-1.01.001: Correct Title
EOF
  printf '| BC-1.01.001 | Wrong Title | SS-01 |\n' > "$bc_dir/BC-INDEX.md"
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\".factory/specs/behavioral-contracts/BC-1.01.001.md\"}}" | "'"$HOOKS"'/validate-bc-title.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"Wrong Title"* ]]
}

@test "validate-count-propagation: historical count in sibling is not drift (#567)" {
  require_bash4_hook_interp
  # Editing BC-INDEX (current total_bcs: 41). The only BC count in the STATE.md
  # sibling is a frozen Phase Progress row, "PRD (38 BCs)". Pre-fix, the sibling's
  # historical 38 was compared against the current 41 and fired drift; post-fix the
  # historical section is skipped, the sibling has no current count, and absence is
  # not drift.
  printf '%s\n' '---' 'total_bcs: 41' '---' '# BC-INDEX' > .factory/BC-INDEX.md
  printf '# STATE\n## Phase Progress\n| PRD | done | PRD (38 BCs) at phase-1 close |\n' > .factory/STATE.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/BC-INDEX.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
}

@test "validate-count-propagation: historical count in source before current is not drift (#567)" {
  require_bash4_hook_interp
  # Editing STATE.md. Its frozen Phase Progress row ("PRD (38 BCs)") appears before
  # the live Count Verification row ("41 BCs"). Pre-fix, first-match picked the
  # historical 38 as the source count and fired drift against BC-INDEX's 41; post-fix
  # the historical section is skipped and the live 41 is the source count — no drift.
  printf '%s\n' '---' 'total_bcs: 41' '---' '# BC-INDEX' > .factory/BC-INDEX.md
  printf '# STATE\n## Phase Progress\n| PRD | done | PRD (38 BCs) at phase-1 close |\n## Count Verification\nCurrent corpus: 41 BCs verified.\n' > .factory/STATE.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
}

@test "validate-count-propagation: genuine current-count drift still blocks (#567)" {
  require_bash4_hook_interp
  # STATE's LIVE Count Verification section says 39 while BC-INDEX frontmatter says
  # 41 — a real current-site disagreement that must still fire even though the same
  # file carries a frozen Phase Progress row (38) that is correctly ignored.
  printf '%s\n' '---' 'total_bcs: 41' '---' '# BC-INDEX' > .factory/BC-INDEX.md
  printf '# STATE\n## Phase Progress\n| PRD | done | PRD (38 BCs) at phase-1 close |\n## Count Verification\nCurrent corpus: 39 BCs verified.\n' > .factory/STATE.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"COUNT DRIFT DETECTED"* ]]
  [[ "$output" == *"39 BCs"* ]]
}

@test "validate-count-propagation: source with only historical counts exits clean, no crash (#567)" {
  require_bash4_hook_interp
  # Editing STATE.md whose only count is a frozen changelog row, with no current
  # count anywhere. After the historical section is skipped the source-count map is
  # empty; that path must exit 0, not trip set -u on the empty associative array
  # (the reason the ${arr[*]:-} guard replaces ${#arr[@]}). Sibling carries no count.
  printf '%s\n' '---' 'document_type: index' '---' '# BC-INDEX' 'Prose only, no counts.' > .factory/BC-INDEX.md
  printf '# STATE\n## Changelog\n| 1.0 | 2026-06-27 | Lists all 36 BCs |\n' > .factory/STATE.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/STATE.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

# ---------- validate-count-propagation (narrowed blockquote guard — cycle-4 BLOCKING-C) ----------
#
# Cycle-3 added a BLANKET skip for every line starting with ">", to suppress a
# false positive from a "> **...batch authoring:** ... 22 BCs anchored ..."
# narrative note in STORY-INDEX.md. But BC-INDEX.md's SOLE carrier of the live
# "subsystems" count is itself a blockquote line ("> Master index of all N
# behavioral contracts across M subsystems."), so the blanket skip silently
# killed subsystems drift detection for that file. The fix narrows the guard
# to skip only two genuinely documentary/historical blockquote FORMS
# ("> Updated YYYY-MM-DD: ..." and "> **...**" bold-prefixed narrative), so a
# live count that happens to be blockquote-formatted is still scanned.

@test "validate-count-propagation: blockquote-formatted live count (subsystems banner) is still checked for drift (BLOCKING-C)" {
  require_bash4_hook_interp
  # BC-INDEX.md's master-index banner is a PLAIN blockquote (no bold prefix,
  # no "Updated" date-stamp) carrying the live subsystems count. It must NOT
  # be skipped by the narrowed guard: ARCH-INDEX.md disagrees (11 vs 10), so
  # this must still fire drift. (Both counts use 2+ digits: Pattern A's
  # "[0-9][0-9,]+" requires at least two characters, so a single-digit count
  # would not match at all — that would test the regex's digit-count floor,
  # not the blockquote guard.)
  printf '# BC-INDEX\n> Master index of all 1,967 behavioral contracts across 10 subsystems.\n> Source of truth for BC count, status, and subsystem assignment.\n' > .factory/BC-INDEX.md
  printf '# ARCH-INDEX\nSubsystem registry: 11 subsystems total.\n' > .factory/ARCH-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/BC-INDEX.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 2 ]
  [[ "$output" == *"COUNT DRIFT DETECTED"* ]]
  [[ "$output" == *"subsystems"* ]]
}

@test "validate-count-propagation: bold-prefixed batch-authoring blockquote note is still skipped (BLOCKING-C)" {
  require_bash4_hook_interp
  # STORY-INDEX.md carries a genuinely documentary bold-prefixed blockquote
  # note citing a per-wave local tally ("22 BCs anchored") that disagrees with
  # the file's own live current-count line ("41 BCs total") and with the
  # sibling's authoritative total_bcs: 41. If the note were NOT skipped, its
  # "22 BCs" (first-seen, same rank) would win over the live "41 BCs" and
  # falsely disagree with the sibling — firing drift. With the note correctly
  # skipped, only the live "41 BCs" is extracted, which matches the sibling,
  # so this must exit clean.
  printf '%s\n' \
    '# STORY-INDEX' \
    '> **E-8 Tier 1 batch authoring (2026-04-30):** 9 Tier 1 hook port stories authored across 2 bursts; 22 BCs anchored (BC-7.03.* + BC-7.04.*); 33 base story points.' \
    'Current corpus: 41 BCs total.' \
    > .factory/STORY-INDEX.md
  printf '%s\n' '---' 'total_bcs: 41' '---' '# BC-INDEX' > .factory/BC-INDEX.md
  run bash -c 'echo "{\"tool_input\":{\"file_path\":\"'$WORK'/.factory/STORY-INDEX.md\"}}" | "'"$HOOKS"'/validate-count-propagation.sh" 2>&1'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

# ---------- validate-count-propagation (CPU-runaway fix — S-25.01) ----------
#
# Regression + correctness tests for the catastrophic-backtracking fix:
#   - length guard: skip lines > 8192 chars before any regex/extglob processing
#   - linear sed: replace extglob ID-strip with sed -E 's/[A-Za-z]+-[0-9.]+//g'
#
# Root cause: BC-INDEX.md contains a single ~195KB "last_amended:" blob line;
# the extglob substitution line="${line//+([A-Za-z])-+([0-9.])/}" spins a full
# CPU core for >12s on that input (PostToolUse → orphaned hook at PPID 1).

@test "validate-count-propagation: 200KB ID-dense single line completes in <3s (cpu-runaway fix)" {
  require_bash4_hook_interp
  # BC-INDEX.md with a single ~200KB line of ID tokens reproduces the CPU-runaway
  # bug. On unfixed code the extglob substitution hangs >12s; with the length guard
  # the line is skipped in O(1) and the hook exits cleanly in <1s.
  #
  # A STATE.md sibling is required: without a sibling the hook exits early at the
  # "no siblings" guard before calling _extract_counts, so the bug is never reached.
  # SUGGESTION-3: use awk instead of perl (perl may be unavailable; awk is universal).
  awk 'BEGIN { s=""; for(i=0;i<8000;i++) s=s "BC-1.18.003 VP-105 DI-007 "; print s }' \
    > .factory/BC-INDEX.md
  printf '# STATE — no count keywords\n' > .factory/STATE.md

  # Verify the fixture is large enough to reproduce the bug.
  local size
  size=$(wc -c < .factory/BC-INDEX.md | tr -d ' ')
  [ "$size" -gt 150000 ] || skip "Fixture too small (${size}B); generation failed"

  # Portable timeout: background process + polling kill.
  # macOS/BSD has no `timeout` command; GNU/Linux has it but we avoid the
  # dependency. The fork+poll pattern works on both platforms.
  # Budget: 3s. Unfixed code hangs >12s on this input (confirmed per bug report).
  # NIT-1: invoke hook directly (shebang #!/bin/bash controls interpreter,
  # consistent with require_bash4_hook_interp which checks /bin/bash).
  # SUGGESTION-2: capture exit status in a file for post-timeout assertion.
  {
    echo '{"tool_input":{"file_path":".factory/BC-INDEX.md"}}' \
      | "$HOOKS/validate-count-propagation.sh" && echo 0 > "$BATS_TEST_TMPDIR/exit_status.txt" || echo $? > "$BATS_TEST_TMPDIR/exit_status.txt"
  } &
  local pid=$!

  local timed_out=0
  local ticks=0
  while kill -0 "$pid" 2>/dev/null && [ "$ticks" -lt 30 ]; do
    sleep 0.1
    ticks=$((ticks + 1))
  done

  if kill -0 "$pid" 2>/dev/null; then
    # SUGGESTION-1: reap bash/awk/sed descendants before killing parent.
    pkill -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    timed_out=1
  else
    wait "$pid" 2>/dev/null || true
  fi

  [ "$timed_out" -eq 0 ] \
    || { echo "FAIL: Hook still running after 3s on 200KB line — length guard or linear-sed fix not applied" >&2; return 1; }
  # SUGGESTION-2: assert hook exited 0 (oversized all-ID line: no counts after awk
  # filter, SOURCE_COUNTS empty, hook takes the early-exit-0 path).
  [ -f "$BATS_TEST_TMPDIR/exit_status.txt" ] \
    && [ "$(cat "$BATS_TEST_TMPDIR/exit_status.txt")" -eq 0 ] \
    || { echo "FAIL: hook exited non-zero: $(cat "$BATS_TEST_TMPDIR/exit_status.txt" 2>/dev/null)" >&2; return 1; }
}

@test "validate-count-propagation: sed ID-strip removes BC/VP/DI tokens, genuine BCs count survives drift check" {
  require_bash4_hook_interp
  # Line: "BC-1.18.003 VP-105 42 BCs DI-007" — three ID tokens stripped by the
  # linear sed; genuine count "42 BCs" preserved.  Sibling (BC-INDEX) disagrees
  # with total_bcs: 38 → drift must still fire.  If the sed over-aggressively
  # stripped digits from "42 BCs" the count would be lost and the hook would exit
  # 0 (empty SOURCE_COUNTS) — a false negative caught by requiring exit 2 here.
  printf '# STATE\nBC-1.18.003 VP-105 42 BCs DI-007\n' > .factory/STATE.md
  printf '%s\n' '---' 'total_bcs: 38' '---' '# BC-INDEX' > .factory/BC-INDEX.md
  run bash -c "echo '{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}' | \"$HOOKS/validate-count-propagation.sh\" 2>&1"
  [ "$status" -eq 2 ]
  [[ "$output" == *"42 BCs"* ]]
}

@test "validate-count-propagation: preprocessing failure exits 2, not 0 (fail-closed)" {
  require_bash4_hook_interp
  # Install a broken sed shim at the front of PATH that always exits 1.
  # This simulates a preprocessing pipeline failure (awk | sed) inside
  # _extract_counts.  Before BLOCKING-A fix, the process substitution
  # `done < <(_extract_counts ...)` swallowed the failure and the hook
  # returned exit 0 (false-pass: "no drift found").  After the fix,
  # the caller-managed temp file + direct function call gates on the exit
  # status and exits 2 (fail-closed).
  _broken_sed_dir="$(mktemp -d)"
  printf '#!/bin/bash\nexit 1\n' > "$_broken_sed_dir/sed"
  chmod +x "$_broken_sed_dir/sed"

  # Create a file with genuine drift so the hook would normally fire exit 2
  # for drift — but with the broken sed shim it must STILL exit 2, just for
  # the preprocessing-failure path rather than the drift-detection path.
  printf '# STATE\n42 BCs\n' > .factory/STATE.md
  printf '%s\n' '---' 'total_bcs: 38' '---' '# BC-INDEX' > .factory/BC-INDEX.md

  run bash -c "PATH=\"$_broken_sed_dir:\$PATH\" bash \"$HOOKS/validate-count-propagation.sh\" 2>&1 <<< '{\"tool_input\":{\"file_path\":\".factory/STATE.md\"}}'"
  rm -rf "$_broken_sed_dir"
  [ "$status" -eq 2 ]
  # S-2 test-rigor fix: asserting on exit status alone is vacuous — a
  # genuine COUNT DRIFT DETECTED exit (the fixture above is deliberately
  # drift-shaped: source "42 BCs" vs sibling total_bcs: 38) also exits 2, so
  # this test would pass even if fail-closed were wired to the wrong path
  # and preprocessing failure were silently swallowed. Discriminate on the
  # actual stderr message instead: the preprocessing-failure path emits
  # "preprocessing pipeline failed for ..." from inside _extract_counts,
  # which is then reported to the caller as "count extraction failed for
  # ...". The drift-detection path never emits either string — it emits
  # "COUNT DRIFT DETECTED" instead. Assert BOTH: the preprocessing-failure
  # message is present, AND the drift message is absent, so this test can
  # only pass via the preprocessing-failure branch.
  [[ "$output" == *"preprocessing pipeline failed for"* ]]
  [[ "$output" != *"COUNT DRIFT DETECTED"* ]]
}
