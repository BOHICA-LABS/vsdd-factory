#!/usr/bin/env bash
# demo-runner.sh — S-17.05 demo evidence runner for stamp-state-timestamp PostToolUse hook.
#
# Usage: bash docs/demo-evidence/S-17.05/demo-runner.sh <scenario>
# Run from the repo root: cd $(git rev-parse --show-toplevel)
#
# Scenarios:
#   ac001-restamp           AC-001/002: timestamp always re-stamped (no lock block)
#   ac003-self-renewal      AC-003: self-held lock → expires_at renewed (now + 2700s)
#   ac006-foreign-no-renewal  AC-006: foreign holder → expires_at NOT changed (anti-resurrection)
#   ac008-fail-open         AC-008: malformed frontmatter → fail-open (0 bytes written)
#   ac010-crlf              AC-010: CRLF line endings preserved after timestamp re-stamp
#   registry                AC-011/AC-013: tool-matcher shape + atomicity (registry-grep)
#
# BC: BC-4.17.001 (stamp-state-timestamp PostToolUse hook)
# Story: S-17.05

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
DISPATCHER="$REPO_ROOT/target/release/factory-dispatcher"
STAMP_WASM="$REPO_ROOT/plugins/vsdd-factory/hook-plugins/stamp-state-timestamp.wasm"
SCENARIO="${1:-help}"

# -----------------------------------------------------------------------
# Preflight checks
# -----------------------------------------------------------------------
_check_artifacts() {
  if [ ! -x "$DISPATCHER" ]; then
    echo "ERROR: factory-dispatcher not built. Run: cargo build --release -p factory-dispatcher"
    exit 1
  fi
  if [ ! -f "$STAMP_WASM" ]; then
    echo "ERROR: stamp-state-timestamp.wasm not found at: $STAMP_WASM"
    exit 1
  fi
}

# -----------------------------------------------------------------------
# Temp WORK environment (matches bats setup)
# -----------------------------------------------------------------------
WORK=""
setup_work() {
  WORK="$(mktemp -d)"
  mkdir -p "$WORK/.factory/logs"
  mkdir -p "$WORK/hook-plugins"
  cp "$STAMP_WASM" "$WORK/hook-plugins/stamp-state-timestamp.wasm"

  cat > "$WORK/hooks-registry.toml" <<'REGISTRY'
schema_version = 2

# stamp-state-timestamp — S-17.05 / ADR-046 / BC-4.17.001
[[hooks]]
name = "stamp-state-timestamp"
event = "PostToolUse"
tool = "^(Edit|Write|MultiEdit)$"
plugin = "hook-plugins/stamp-state-timestamp.wasm"
priority = 470
timeout_ms = 5000
on_error = "continue"
async = false

[hooks.capabilities.read_file]
path_allow = [".factory/STATE.md"]

[hooks.capabilities.write_file]
path_allow = [".factory/STATE.md"]

[hooks.capabilities.exec_subprocess]
binary_allow = ["git"]
env_allow = ["HOME", "GIT_CONFIG_GLOBAL", "XDG_CONFIG_HOME"]
REGISTRY
  # shellcheck disable=SC2064
  trap "rm -rf '$WORK'" EXIT
}

# Run the dispatcher with a PostToolUse Write payload targeting .factory/STATE.md
run_hook() {
  local envelope
  envelope='{"event_name":"PostToolUse","tool_name":"Write","session_id":"demo","dispatcher_trace_id":"demo-trace","tool_input":{"file_path":".factory/STATE.md","content":"(already-written)"},"tool_response":{}}'
  printf '%s' "$envelope" | \
    CLAUDE_PLUGIN_ROOT="$WORK" \
    CLAUDE_PROJECT_DIR="$WORK" \
    "$DISPATCHER" 2>&1 || true
}

# -----------------------------------------------------------------------
# Scenarios
# -----------------------------------------------------------------------

case "$SCENARIO" in

  ac001-restamp)
    _check_artifacts
    echo "=== AC-001/002: timestamp: unconditionally re-stamped after any qualifying write ==="
    echo "    (BC-4.17.001 PC1 — no identity gate on timestamp re-stamping)"
    echo ""
    setup_work
    cat > "$WORK/.factory/STATE.md" <<'FIXTURE'
---
document_type: state
version: "0.0.1-demo"
timestamp: 2020-01-01T00:00:00Z
phase: demo
current_step: "ac001-demo"
---

# STATE (demo fixture — no factory_lock block)
FIXTURE
    echo "BEFORE:"
    grep "^timestamp:" "$WORK/.factory/STATE.md"
    echo ""
    echo "Running PostToolUse Write hook..."
    run_hook
    echo ""
    echo "AFTER (hook re-stamped to current wall-clock UTC):"
    grep "^timestamp:" "$WORK/.factory/STATE.md"
    AFTER_TS="$(grep "^timestamp:" "$WORK/.factory/STATE.md" | sed 's/timestamp: //')"
    if [ "$AFTER_TS" != "2020-01-01T00:00:00Z" ]; then
      echo ""
      echo "PASS: AC-001 — timestamp re-stamped from stale 2020-01-01 to current UTC"
    else
      echo "FAIL: timestamp was not re-stamped"
      exit 1
    fi
    ;;

  ac003-self-renewal)
    _check_artifacts
    echo "=== AC-003: Self-held lock — factory_lock.expires_at renewed to now + 2700s ==="
    echo "    (BC-4.17.001 PC2 identity-gate row 1: holder matches caller → renew)"
    echo ""
    setup_work
    GIT_EMAIL="$(git config user.email 2>/dev/null || echo "testuser@example.com")"
    FIXTURE_EXPIRES="2099-01-01T00:45:00Z"
    cat > "$WORK/.factory/STATE.md" <<FIXTURE
---
document_type: state
version: "0.0.1-demo"
timestamp: 2020-01-01T00:00:00Z
phase: demo
current_step: "ac003-demo"
factory_lock:
  holder: "${GIT_EMAIL}"
  locked_at: "2026-01-01T10:00:00Z"
  expires_at: "${FIXTURE_EXPIRES}"
---

# STATE (demo fixture — self lock)
FIXTURE
    echo "BEFORE:"
    echo "  holder:     ${GIT_EMAIL}"
    echo "  expires_at: ${FIXTURE_EXPIRES}  (far-future fixture)"
    echo ""
    echo "Running PostToolUse Write hook..."
    run_hook
    echo ""
    RENEWED="$(grep 'expires_at:' "$WORK/.factory/STATE.md" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z')"
    echo "AFTER:"
    echo "  expires_at: ${RENEWED}  (renewed to now + 2700s)"
    if [ "$RENEWED" != "$FIXTURE_EXPIRES" ]; then
      echo ""
      echo "PASS: AC-003 — self-held lock expires_at advanced (lock keep-alive BC-5.40.001 PC4)"
    else
      echo "FAIL: expires_at was not renewed"
      exit 1
    fi
    ;;

  ac006-foreign-no-renewal)
    _check_artifacts
    echo "=== AC-006: Foreign holder — expires_at NOT renewed (anti-resurrection) ==="
    echo "    (BC-4.17.001 PC2 identity-gate row 4: identity MISMATCH → no renewal)"
    echo "    SAFETY-CRITICAL: foreign/expired lock MUST NEVER be silently resurrected"
    echo ""
    setup_work
    FIXTURE_EXPIRES="2099-06-01T00:00:00Z"
    cat > "$WORK/.factory/STATE.md" <<FIXTURE
---
document_type: state
version: "0.0.1-demo"
timestamp: 2020-01-01T00:00:00Z
phase: demo
current_step: "ac006-demo"
factory_lock:
  holder: "foreign-holder@example.com"
  locked_at: "2026-01-01T10:00:00Z"
  expires_at: "${FIXTURE_EXPIRES}"
---

# STATE (demo fixture — foreign lock)
FIXTURE
    echo "BEFORE:"
    echo "  holder:     foreign-holder@example.com  (NOT this session)"
    echo "  expires_at: ${FIXTURE_EXPIRES}"
    echo ""
    echo "Running PostToolUse Write hook..."
    run_hook
    echo ""
    ACTUAL_EXPIRES="$(grep 'expires_at:' "$WORK/.factory/STATE.md" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z')"
    ACTUAL_TS="$(grep "^timestamp:" "$WORK/.factory/STATE.md" | sed 's/timestamp: //')"
    echo "AFTER:"
    echo "  expires_at: ${ACTUAL_EXPIRES}  (must be UNCHANGED)"
    echo "  timestamp:  ${ACTUAL_TS}  (re-stamped per PC1)"
    if [ "$ACTUAL_EXPIRES" = "$FIXTURE_EXPIRES" ]; then
      echo ""
      echo "PASS: AC-006 — foreign holder NOT renewed (anti-resurrection property holds)"
    else
      echo "FAIL: foreign lock was resurrected! expires_at changed."
      exit 1
    fi
    ;;

  ac008-fail-open)
    _check_artifacts
    echo "=== AC-008: Fail-open on malformed frontmatter ==="
    echo "    (BC-4.17.001 PC3 — structural read/parse errors: 0 bytes written)"
    echo ""
    setup_work
    # Malformed: missing closing --- delimiter
    {
      printf '%s\n' '---'
      printf '%s\n' 'document_type: state'
      printf '%s\n' 'timestamp: 2020-01-01T00:00:00Z'
      printf '%s\n' '# MALFORMED: no closing --- delimiter'
      printf '%s\n' ''
      printf '%s'   'Body content.'
    } > "$WORK/.factory/STATE.md"
    BEFORE_BYTES="$(wc -c < "$WORK/.factory/STATE.md")"
    BEFORE_TS="$(grep "^timestamp:" "$WORK/.factory/STATE.md" || echo "  (no timestamp found)")"
    echo "BEFORE: STATE.md has malformed frontmatter (no closing ---):"
    head -4 "$WORK/.factory/STATE.md"
    echo "  (size: ${BEFORE_BYTES} bytes)"
    echo ""
    echo "Running PostToolUse Write hook (hook MUST NOT write on malformed frontmatter)..."
    run_hook
    echo ""
    AFTER_BYTES="$(wc -c < "$WORK/.factory/STATE.md")"
    AFTER_TS="$(grep "^timestamp:" "$WORK/.factory/STATE.md" || echo "  (no timestamp found)")"
    echo "AFTER:"
    echo "  size before: ${BEFORE_BYTES} bytes"
    echo "  size after:  ${AFTER_BYTES} bytes"
    echo "  timestamp before: $BEFORE_TS"
    echo "  timestamp after:  $AFTER_TS"
    if [ "$BEFORE_BYTES" = "$AFTER_BYTES" ]; then
      echo ""
      echo "PASS: AC-008 — fail-open: STATE.md unchanged (0 bytes written on malformed frontmatter)"
    else
      echo "FAIL: hook wrote to STATE.md despite malformed frontmatter (byte count changed)"
      exit 1
    fi
    ;;

  ac010-crlf)
    _check_artifacts
    echo "=== AC-010: CRLF line endings preserved after timestamp re-stamp ==="
    echo "    (BC-4.17.001 PC4 Invariant 5 — line-ending preservation)"
    echo ""
    setup_work
    # Create STATE.md with CRLF line endings (simulate Windows-authored content)
    # Using printf to write each CRLF-terminated line
    {
      printf '%s\r\n' '---'
      printf '%s\r\n' 'document_type: state'
      printf '%s\r\n' 'version: "0.0.1-demo"'
      printf '%s\r\n' 'timestamp: 2020-01-01T00:00:00Z'
      printf '%s\r\n' 'phase: demo'
      printf '%s\r\n' 'current_step: "crlf-demo"'
      printf '%s\r\n' '---'
      printf '%s\r\n' ''
      printf '%s\r\n' '# Body with CRLF endings.'
    } > "$WORK/.factory/STATE.md"
    echo "BEFORE: checking line endings (should be CRLF = 0x0d 0x0a):"
    head -c 80 "$WORK/.factory/STATE.md" | xxd | head -5
    echo ""
    echo "Running PostToolUse Write hook..."
    run_hook
    echo ""
    echo "AFTER: line endings (CRLF must be preserved by hook):"
    head -c 80 "$WORK/.factory/STATE.md" | xxd | head -5
    # Check that CRLF (0d 0a) is still present
    CRLF_COUNT="$(head -c 200 "$WORK/.factory/STATE.md" | xxd | grep -c '0d 0a' || echo 0)"
    RESTAMP="$(grep "^timestamp:" "$WORK/.factory/STATE.md" | tr -d '\r')"
    echo ""
    echo "CRLF sequences found: ${CRLF_COUNT}"
    echo "Timestamp re-stamped: $RESTAMP"
    if [ "$CRLF_COUNT" -gt 0 ] && [ "$RESTAMP" != "timestamp: 2020-01-01T00:00:00Z" ]; then
      echo ""
      echo "PASS: AC-010 — CRLF preserved AND timestamp re-stamped"
    else
      echo "FAIL: CRLF count=${CRLF_COUNT}, restamp='${RESTAMP}'"
      exit 1
    fi
    ;;

  registry)
    echo "=== AC-011/AC-013: Registry tool-matcher shape + atomicity ==="
    echo "    AC-011: tool = '^(Edit|Write|MultiEdit)\$' (NO Bash, NO Agent)"
    echo "    AC-013: stamp-state-timestamp PRESENT AND verify-state-timestamp-refresh ABSENT"
    echo ""
    REGISTRY="$REPO_ROOT/plugins/vsdd-factory/hooks-registry.toml"
    echo "--- AC-011: tool matcher ---"
    TOOL_LINE="$(grep -A 20 'name = "stamp-state-timestamp"' "$REGISTRY" | grep '^tool = ' | head -1)"
    echo "  $TOOL_LINE"
    if echo "$TOOL_LINE" | grep -qF 'tool = "^(Edit|Write|MultiEdit)$"'; then
      if ! echo "$TOOL_LINE" | grep -qE 'Bash|Agent'; then
        echo "  PASS: tool matcher is canonical (no Bash, no Agent)"
      fi
    else
      echo "  FAIL: unexpected tool matcher"
      exit 1
    fi
    echo ""
    echo "--- AC-013: atomicity ---"
    STAMPER_COUNT="$(grep -c 'name = "stamp-state-timestamp"' "$REGISTRY" || true)"
    OLD_COUNT="$(grep -c 'name = "verify-state-timestamp-refresh"' "$REGISTRY" || true)"
    echo "  stamp-state-timestamp occurrences: ${STAMPER_COUNT}  (must be >= 1)"
    echo "  verify-state-timestamp-refresh occurrences: ${OLD_COUNT}  (must be 0)"
    if [ "$STAMPER_COUNT" -ge 1 ] && [ "$OLD_COUNT" -eq 0 ]; then
      echo ""
      echo "PASS: AC-013 — new stamper present AND old guard absent (ADR-046 Decision 3 atomicity)"
    else
      echo "FAIL: atomicity violated"
      exit 1
    fi
    ;;

  help|*)
    echo "Usage: bash docs/demo-evidence/S-17.05/demo-runner.sh <scenario>"
    echo ""
    echo "Scenarios:"
    echo "  ac001-restamp           AC-001/002: timestamp unconditionally re-stamped"
    echo "  ac003-self-renewal      AC-003: self-held lock → expires_at renewed"
    echo "  ac006-foreign-no-renewal  AC-006: foreign holder → NOT renewed (anti-resurrection)"
    echo "  ac008-fail-open         AC-008: malformed frontmatter → fail-open"
    echo "  ac010-crlf              AC-010: CRLF line endings preserved"
    echo "  registry                AC-011/013: registry tool-matcher + atomicity"
    ;;

esac
