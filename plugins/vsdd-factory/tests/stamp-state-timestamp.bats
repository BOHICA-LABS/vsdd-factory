#!/usr/bin/env bats
# stamp-state-timestamp.bats — S-17.05 bats integration tests (BC-5.38.001 Red Gate).
#
# Covers the 4 bats integration cases from S-17.05 v1.3 Red Gate Test Table:
#
#   AC-006 (e2e): PostToolUse Write, foreign holder present →
#                 expires_at MUST remain byte-identical (identity mismatch → no renewal).
#                 Requires dispatcher + WASM → skip if absent. Red Gate: WASM absent or
#                 stub-renewing unconditionally → expires_at changed assertion fails.
#
#   AC-011 (registry shape): hooks-registry.toml entry for stamp-state-timestamp
#                 MUST NOT include "Bash" or "Agent" in the tool matcher.
#                 Registry absent → fails immediately (no WASM required).
#
#   AC-013 (registry atomicity): stamp-state-timestamp entry PRESENT AND
#                 verify-state-timestamp-refresh entry ABSENT in the same commit.
#                 ADR-046 Decision 3 atomicity constraint.
#                 Either condition unmet → fails immediately (no WASM required).
#
#   AC-014 (e2e mid-burst): PostToolUse Write, self-holder present →
#                 expires_at IS renewed (identity match → renewal fires).
#                 Mandatory real-WASM test per the S-17.04 AC-018 lesson.
#                 Requires dispatcher + WASM → skip if absent.
#
# Story: S-17.05 v1.3 (stamp-state-timestamp PostToolUse WASM hook)
# BC: BC-4.17.001 (stamp-state-timestamp), BC-5.40.001 PC4 (mid-burst keep-alive)
# ADR: ADR-046 (PostToolUse stamp), ADR-019 (async=false)
#
# KEY BEHAVIORAL DIFFERENCE FROM PRETOOLUSE GUARDS:
#   This hook is PostToolUse — the write has ALREADY happened when the hook fires.
#   The hook reads the ON-DISK, post-write file and re-stamps it.
#   There is NO old_string/new_string/content reconstruction step (unlike the retired
#   verify-state-timestamp-refresh guard). The bats tests write the fixture STATE.md
#   first (simulating the agent's already-completed write), then invoke the dispatcher
#   with a PostToolUse payload, then read back STATE.md to assert the hook's output.
#
# File location: plugins/vsdd-factory/tests/stamp-state-timestamp.bats
# (flat path — discovered by run-all.sh `tests/*.bats` glob).
#
# Exit codes (dispatcher stdout/stderr, not hook-specific):
#   0 = Continue (hook ran; PostToolUse hooks cannot block)
#   1 = Error (dispatcher/plugin failure)
#
# Run:
#   bats plugins/vsdd-factory/tests/stamp-state-timestamp.bats

# ---------------------------------------------------------------------------
# setup / teardown
# ---------------------------------------------------------------------------

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  PLUGIN_ROOT="$REPO_ROOT/plugins/vsdd-factory"
  DISPATCHER="$REPO_ROOT/target/release/factory-dispatcher"
  STAMP_WASM="$PLUGIN_ROOT/hook-plugins/stamp-state-timestamp.wasm"

  WORK="$(mktemp -d)"
  mkdir -p "$WORK/.factory/logs"
  mkdir -p "$WORK/hook-plugins"

  # Copy the stamp WASM into the synthetic plugin root if it exists.
  if [ -f "$STAMP_WASM" ]; then
    cp "$STAMP_WASM" "$WORK/hook-plugins/stamp-state-timestamp.wasm"
  fi

  # Hermetic git identity (CI env-independence fix, PR #798).
  #
  # The hook's exec_subprocess capability shells out to `git config user.email`
  # to resolve the writer identity for lock-renewal decisions (BC-4.17.001 PC2).
  # On CI runners with no ambient git identity, this returns empty, causing
  # IdentityResolutionFailed — the self-lock renewal test (AC-014) then fails
  # because the hook cannot confirm a match.
  #
  # Fix: create a throwaway gitconfig in $WORK and export GIT_CONFIG_GLOBAL +
  # GIT_CONFIG_NOSYSTEM + HOME so that (a) _write_state_self_lock captures the
  # same identity as (b) the hook subprocess receives via env_allow.
  #
  # The hook registry declares env_allow = ["HOME", "GIT_CONFIG_GLOBAL",
  # "XDG_CONFIG_HOME"], so both HOME and GIT_CONFIG_GLOBAL propagate into
  # the WASM sandbox's git subprocess automatically.
  #
  # SAFETY: the foreign-lock fixture (AC-006) hardcodes holder@example.com,
  # which will never match ci-hermetic@vsdd-factory.test — that test's
  # no-renewal assertion remains intact.
  git config --file "$WORK/gitconfig" user.email "ci-hermetic@vsdd-factory.test"
  git config --file "$WORK/gitconfig" user.name "vsdd ci"
  export GIT_CONFIG_GLOBAL="$WORK/gitconfig"
  export GIT_CONFIG_NOSYSTEM=1
  export HOME="$WORK"

  export CLAUDE_PROJECT_DIR="$WORK"
  export CLAUDE_PLUGIN_ROOT="$WORK"
}

teardown() {
  # $WORK contains the hermetic gitconfig and hermetic $HOME; a single rm -rf covers all.
  rm -rf "$WORK"
}

# ---------------------------------------------------------------------------
# Preflight helpers
# ---------------------------------------------------------------------------

# Skip if dispatcher binary or stamp-state-timestamp WASM is not present.
# This is the RED GATE skip — both artifacts are produced by the implementer (T-3/T-4).
#
# P5-M1: CI hard-fail gate.
# If CI_REQUIRE_ARTIFACTS=1 is set, artifact absence is a HARD FAIL rather than
# a graceful skip — `skip` exits 0 in bats, which silently masks a broken build.
_require_artifacts() {
  if [ ! -x "$DISPATCHER" ]; then
    [ -z "${CI_REQUIRE_ARTIFACTS:-}" ] || {
      echo "FAIL: factory-dispatcher binary not present in CI (CI_REQUIRE_ARTIFACTS=1) — run: cargo build --release -p factory-dispatcher"
      false
    }
    skip "factory-dispatcher binary not built — run: cargo build --release -p factory-dispatcher (S-17.05 implementer task T-3)"
  fi
  if [ ! -f "$WORK/hook-plugins/stamp-state-timestamp.wasm" ]; then
    [ -z "${CI_REQUIRE_ARTIFACTS:-}" ] || {
      echo "FAIL: stamp-state-timestamp.wasm not present in CI (CI_REQUIRE_ARTIFACTS=1) — run: cargo build --target wasm32-wasip1 -p stamp-state-timestamp"
      false
    }
    skip "stamp-state-timestamp.wasm not present — run: cargo build --target wasm32-wasip1 -p stamp-state-timestamp (S-17.05 implementer task T-3)"
  fi
}

# ---------------------------------------------------------------------------
# Registry writer (canonical stamp-state-timestamp entry per Registry Entry Spec)
# ---------------------------------------------------------------------------

# Write the canonical stamp-state-timestamp registry entry for e2e tests.
# Per S-17.05 Registry Entry Spec / ADR-046 / BC-4.17.001 Preconditions 1-2.
# async = false REQUIRED (ADR-019 + ADR-046).
# tool = "^(Edit|Write|MultiEdit)$" — NO Bash, NO Agent (PC5/AC-011).
# on_error = "continue" (BC-4.17.001 PC3 / Invariant 4 fail-open).
# priority = 470 (next free PostToolUse slot as of story authoring — reverify before T-4).
# [hooks.capabilities.read_file] AND [hooks.capabilities.write_file] BOTH mandatory.
# [hooks.capabilities.exec_subprocess] required for PC2 identity resolution.
_write_full_registry() {
  cat > "$WORK/hooks-registry.toml" <<'EOF'
schema_version = 2

# ---------- stamp-state-timestamp (PostToolUse, Edit|Write|MultiEdit) ----------
# S-17.05 / ADR-046 / BC-4.17.001: hook-authored STATE.md wall-clock stamping.
# Unconditionally re-stamps timestamp: (PC1). Renews factory_lock.expires_at
# (PC2) ONLY when a lock is held AND this hook's resolved writer identity
# (git config user.email) byte-equals the recorded holder.
# async = false REQUIRED — see ADR-019 + ADR-046

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
env_allow = ["HOME", "GIT_CONFIG_GLOBAL", "XDG_CONFIG_HOME", "GIT_CONFIG_NOSYSTEM"]
EOF
}

# ---------------------------------------------------------------------------
# STATE.md fixture writers (written to WORK/.factory/STATE.md)
# ---------------------------------------------------------------------------

# Write STATE.md with foreign lock (holder != any test identity).
# Used by AC-006 e2e test to verify expires_at is NOT renewed on mismatch.
_write_state_foreign_lock() {
  local expires_at="${1:-2099-01-01T00:00:00Z}"
  mkdir -p "$WORK/.factory"
  cat > "$WORK/.factory/STATE.md" <<EOF
---
document_type: state
version: "0.0.1-bats-test"
timestamp: 2020-01-01T00:00:00Z
phase: test
current_step: "bats-test"
factory_lock:
  holder: "holder@example.com"
  locked_at: "2026-01-01T10:00:00Z"
  expires_at: "${expires_at}"
---

# STATE (bats fixture — foreign lock, expires=${expires_at})
EOF
}

# Write STATE.md with self lock (holder == git config user.email in this machine).
# Used by AC-014 e2e test to verify expires_at IS renewed on identity match.
# The holder must match the actual git config user.email on the test machine.
# This fixture writes the holder as the REAL git user email (runtime-resolved).
_write_state_self_lock() {
  local git_email
  git_email="$(git config user.email 2>/dev/null || echo "testuser@example.com")"
  local expires_at="${1:-2099-01-01T00:45:00Z}"
  mkdir -p "$WORK/.factory"
  cat > "$WORK/.factory/STATE.md" <<EOF
---
document_type: state
version: "0.0.1-bats-test"
timestamp: 2020-01-01T00:00:00Z
phase: test
current_step: "bats-test"
factory_lock:
  holder: "${git_email}"
  locked_at: "2026-01-01T10:00:00Z"
  expires_at: "${expires_at}"
---

# STATE (bats fixture — self lock, holder=${git_email}, expires=${expires_at})
EOF
}

# ---------------------------------------------------------------------------
# Dispatcher invocation helper (PostToolUse variant)
# ---------------------------------------------------------------------------

# Invoke the dispatcher with the given JSON envelope.
# Sets $status and $output (combined stdout+stderr).
#
# PostToolUse payloads: same shape as PreToolUse but event_name = "PostToolUse".
# The hook reads the ON-DISK file via read_file capability — the tool_input.file_path
# just tells the dispatcher (and hook) which file was written; the hook ignores
# tool_input content fields and reads from disk instead.
_run_dispatcher() {
  local envelope="$1"
  run bash -c "printf '%s' '$envelope' | \
    CLAUDE_PLUGIN_ROOT='$WORK' \
    CLAUDE_PROJECT_DIR='$WORK' \
    '$DISPATCHER' 2>&1"
}

# ---------------------------------------------------------------------------
# AC-006 (e2e): Foreign holder write — expires_at MUST remain unchanged
#
# BC-4.17.001 PC2 + Invariant 2 (SAFETY-CRITICAL).
# Fixture: STATE.md with foreign holder (holder@example.com ≠ this machine's git email).
# Hook fires (PostToolUse Write), resolves identity → mismatch → no renewal.
# Expected: STATE.md expires_at byte-identical after hook fires (no resurrection).
#           timestamp: IS re-stamped (PC1 unconditional).
#
# RED GATE: WASM absent → skip. Stub renewing unconditionally → expires_at changed
# → assertion fails.
# ---------------------------------------------------------------------------

@test "test_stamp_state_timestamp_foreign_holder_write_never_renews_e2e" {
  _require_artifacts
  _write_full_registry

  local original_expires="2099-06-01T00:00:00Z"
  _write_state_foreign_lock "$original_expires"

  # PostToolUse Write envelope: simulates a completed Write to .factory/STATE.md.
  # The hook reads STATE.md from disk; the tool_input content field is not used
  # for decision logic (PostToolUse reads on-disk post-write state).
  # tool_response is required: guard_logic GAP-3 (BC F-013 Precondition-1) returns
  # Continue immediately when tool_response is absent/null — simulating a write that
  # did not complete.  A successful PostToolUse Write carries tool_response with no
  # "error" key.  Unit tests use "tool_response": {} for the same reason.
  local envelope
  envelope="{\"event_name\":\"PostToolUse\",\"tool_name\":\"Write\",\"session_id\":\"t-ac006\",\"dispatcher_trace_id\":\"ac006-trace\",\"tool_input\":{\"file_path\":\".factory/STATE.md\",\"content\":\"(already-written)\"},\"tool_response\":{}}"

  _run_dispatcher "$envelope"

  # PostToolUse hooks do not block; dispatcher must exit 0 (Continue).
  [ "$status" -eq 0 ] || {
    echo "FAIL: expected exit 0 (PostToolUse hooks cannot block) but got status=$status"
    echo "Output: $output"
    return 1
  }

  # Read back STATE.md from disk to assert hook outcome.
  local actual_state
  actual_state="$(cat "$WORK/.factory/STATE.md")"

  # AC-006 PC2+Invariant 2 (SAFETY-CRITICAL): expires_at must be byte-identical.
  # Foreign holder (holder@example.com) MUST NOT be renewed regardless of hook timing.
  echo "$actual_state" | grep -q "expires_at: \"${original_expires}\"" || {
    echo "FAIL (SAFETY-CRITICAL): AC-006 Invariant 2 violated — foreign holder's expires_at was changed."
    echo "Original expires_at: ${original_expires}"
    echo "Actual STATE.md after hook:"
    echo "$actual_state"
    return 1
  }

  # PC1: timestamp MUST be re-stamped (no longer 2020-01-01T00:00:00Z).
  echo "$actual_state" | grep -v "2020-01-01T00:00:00Z" | grep -q "timestamp:" || {
    echo "FAIL: AC-006/PC1 — timestamp must be re-stamped by hook (must not remain 2020-01-01T00:00:00Z)."
    echo "Actual STATE.md after hook:"
    echo "$actual_state"
    return 1
  }
}

# ---------------------------------------------------------------------------
# AC-011 (registry shape): tool matcher MUST NOT include Bash or Agent
#
# BC-4.17.001 PC5: this hook fires ONLY for Edit|Write|MultiEdit.
# Including Bash or Agent would cause the hook to fire on factory-lock-write.sh
# invocations (acquire/release/clear) — violating PC5.
#
# This is a pure registry-grep test — no WASM or dispatcher required.
# RED GATE: registry entry absent → grep assertion fails immediately.
# Entry present with Bash/Agent in matcher → assertion fails.
# ---------------------------------------------------------------------------

@test "test_stamp_state_timestamp_registry_tool_matcher_excludes_bash_and_agent" {
  local registry="$REPO_ROOT/plugins/vsdd-factory/hooks-registry.toml"

  # Confirm the production registry file exists.
  [ -f "$registry" ] || {
    echo "FAIL: production hooks-registry.toml not found at: $registry"
    return 1
  }

  # RED GATE: stamp-state-timestamp entry MUST be present.
  grep -q 'name = "stamp-state-timestamp"' "$registry" || {
    echo "FAIL: stamp-state-timestamp entry not found in production hooks-registry.toml."
    echo "Implementer: add the [[hooks]] entry per S-17.05 T-4 / Registry Entry Spec."
    echo "Registry: $registry"
    return 1
  }

  # Extract the stamp-state-timestamp section using awk (bounded by next [[hooks]] or EOF).
  local section
  section=$(awk '
    /name = "stamp-state-timestamp"/ { in_section=1 }
    /^\[\[hooks\]\]/ && in_section && !/name = "stamp-state-timestamp"/ { in_section=0 }
    in_section { print }
  ' "$registry")

  # The tool matcher line must be present and MUST NOT contain Bash or Agent.
  # Correct: tool = "^(Edit|Write|MultiEdit)$"
  # Incorrect: tool = "Edit|Write|MultiEdit|Bash" or "Edit|Write|MultiEdit|Agent" etc.
  echo "$section" | grep -q '^tool = ' || {
    echo "FAIL: stamp-state-timestamp registry entry has no 'tool = ...' line."
    echo "Required: tool = \"^(Edit|Write|MultiEdit)$\" (AC-011 / BC-4.17.001 PC5)."
    return 1
  }

  # Assert Bash is absent from the tool matcher (PC5: no fire on factory-lock-write.sh).
  echo "$section" | grep '^tool = ' | grep -q 'Bash' && {
    echo "FAIL: stamp-state-timestamp tool matcher includes 'Bash' — MUST NOT."
    echo "Including Bash causes the hook to fire on factory-lock-write.sh acquire/release/clear,"
    echo "violating BC-4.17.001 PC5 (no lock-lifecycle involvement)."
    echo "Tool line: $(echo "$section" | grep '^tool = ')"
    return 1
  }

  # Assert Agent is absent from the tool matcher.
  echo "$section" | grep '^tool = ' | grep -q 'Agent' && {
    echo "FAIL: stamp-state-timestamp tool matcher includes 'Agent' — MUST NOT."
    echo "BC-4.17.001 PC5: hook must never observe CAS-push or Agent-invoked operations."
    echo "Tool line: $(echo "$section" | grep '^tool = ')"
    return 1
  }

  # Assert the correct tool matcher is present.
  # Use -F (fixed-string) to match the exact literal value including parentheses and
  # pipe characters. BRE grep treats \( / \) as grouping metacharacters and | as a
  # literal — neither form reliably matches the canonical tool = "^(Edit|Write|MultiEdit)$"
  # value from the registry (AC-011 / BC-4.17.001 PC5).
  echo "$section" | grep -qF 'tool = "^(Edit|Write|MultiEdit)$"' || {
    echo "FAIL: stamp-state-timestamp tool matcher is not the canonical value."
    echo "Required: tool = \"^(Edit|Write|MultiEdit)\$\""
    echo "Actual tool line: $(echo "$section" | grep '^tool = ')"
    return 1
  }
}

# ---------------------------------------------------------------------------
# AC-013 (registry atomicity): stamper PRESENT AND old guard ABSENT
#
# BC-4.17.001 Preconditions 1-2 / ADR-046 Decision 3:
#   stamp-state-timestamp and verify-state-timestamp-refresh MUST be toggled
#   atomically in the same commit (T-4 + T-5 are one commit).
#   Either condition unmet → assertion fails immediately.
#   - stamper absent → assertion fails
#   - old guard still present → assertion fails
#
# Pure registry-grep test — no WASM or dispatcher required.
# RED GATE: stamper absent → fails immediately.
# ---------------------------------------------------------------------------

@test "test_hooks_registry_stamper_present_and_old_guard_absent_atomically" {
  local registry="$REPO_ROOT/plugins/vsdd-factory/hooks-registry.toml"

  # Confirm the production registry file exists.
  [ -f "$registry" ] || {
    echo "FAIL: production hooks-registry.toml not found at: $registry"
    return 1
  }

  # Condition 1: stamp-state-timestamp MUST be present.
  local stamper_count
  stamper_count=$(grep -c 'name = "stamp-state-timestamp"' "$registry" || true)
  [ "$stamper_count" -ge 1 ] || {
    echo "FAIL: stamp-state-timestamp entry not found in production hooks-registry.toml."
    echo "AC-013 / ADR-046 Decision 3: stamp-state-timestamp MUST be present before"
    echo "the verify-state-timestamp-refresh entry is removed (atomicity constraint)."
    echo "Implementer: add the [[hooks]] entry per S-17.05 T-4."
    echo "Registry: $registry"
    return 1
  }

  # Condition 2: verify-state-timestamp-refresh MUST be absent (T-5 deregistration).
  local old_guard_count
  old_guard_count=$(grep -c 'name = "verify-state-timestamp-refresh"' "$registry" || true)
  [ "$old_guard_count" -eq 0 ] || {
    echo "FAIL: verify-state-timestamp-refresh entry is STILL present in production hooks-registry.toml."
    echo "AC-013 / ADR-046 Decision 3: both T-4 (add stamp-state-timestamp) AND T-5"
    echo "(remove verify-state-timestamp-refresh) MUST land in the SAME commit."
    echo "Count of old-guard occurrences: ${old_guard_count} (must be 0)."
    echo "Implementer: delete the [[hooks]] entry for verify-state-timestamp-refresh (S-17.05 T-5)."
    echo "Registry: $registry"
    return 1
  }

  # Validation: confirm the count of each for belt-and-suspenders.
  echo "AC-013 atomicity check: stamp-state-timestamp present (${stamper_count} occurrence(s));"
  echo "verify-state-timestamp-refresh absent (0 occurrences). PASS."
}

# ---------------------------------------------------------------------------
# AC-014 (e2e mid-burst): Self-holder write — expires_at MUST be renewed
#
# BC-4.17.001 PC2 + BC-5.40.001 PC4 (mid-burst keep-alive).
# Fixture: STATE.md with self lock (holder == git config user.email on this machine).
# Hook fires (PostToolUse Write), resolves identity → match → renewal fires.
# Expected: STATE.md expires_at IS advanced (> original fixture value) after hook fires.
#           Mandatory real-WASM-runtime test — mirrors the S-17.04 AC-018 lesson:
#           native-env unit tests do not validate the deployed WASM trigger/registry-wiring.
#
# RED GATE: WASM absent → skip. Stub not renewing → expires_at unchanged → assertion fails.
# ---------------------------------------------------------------------------

@test "test_stamp_state_timestamp_mid_burst_renewal_e2e" {
  _require_artifacts
  _write_full_registry

  # Fixture expires_at: FAR-FUTURE value — valid (unexpired) at any invocation time.
  # The lock must be valid so Case-5 (identity-match + not-expired → renewal fires) applies.
  # An expired fixture would trigger Case-2 (AlreadyExpired → NoOp), which is the opposite
  # of AC-014's intent and is tested separately by test_expired_self_held_lock_never_renewed.
  local fixture_expires="2099-01-01T00:45:00Z"
  _write_state_self_lock "$fixture_expires"

  # Record the timestamp of the initial STATE.md for later comparison.
  local initial_state
  initial_state="$(cat "$WORK/.factory/STATE.md")"

  # PostToolUse Write envelope: simulates a completed Write to .factory/STATE.md.
  # tool_response required — same rationale as AC-006 (GAP-3 / BC F-013 gate).
  local envelope
  envelope="{\"event_name\":\"PostToolUse\",\"tool_name\":\"Write\",\"session_id\":\"t-ac014\",\"dispatcher_trace_id\":\"ac014-trace\",\"tool_input\":{\"file_path\":\".factory/STATE.md\",\"content\":\"(already-written)\"},\"tool_response\":{}}"

  # Bracket the dispatcher call to compute the expected post-renewal expires_at window.
  local before_epoch
  before_epoch="$(date -u +%s)"
  _run_dispatcher "$envelope"
  local after_epoch
  after_epoch="$(date -u +%s)"

  # PostToolUse hooks do not block; dispatcher must exit 0.
  [ "$status" -eq 0 ] || {
    echo "FAIL: expected exit 0 (PostToolUse hooks cannot block) but got status=$status"
    echo "Output: $output"
    return 1
  }

  # Read back STATE.md from disk.
  local actual_state
  actual_state="$(cat "$WORK/.factory/STATE.md")"

  # AC-014 PC2 / BC-5.40.001 PC4: fixture value must be gone (renewal fired).
  # The far-future fixture (${fixture_expires}) must be replaced by now + TTL_SECONDS (2700s).
  echo "$actual_state" | grep -q "expires_at: \"${fixture_expires}\"" && {
    echo "FAIL: AC-014 PC2 / BC-5.40.001 PC4 — expires_at was NOT renewed."
    echo "Fixture value ${fixture_expires} is still present after hook invocation."
    echo "Expected: expires_at renewed to approximately NOW + 2700s."
    echo "This means either the hook did not fire (WASM not loaded), identity resolution"
    echo "failed, or renewal logic is not implemented."
    echo "Actual STATE.md after hook:"
    echo "$actual_state"
    return 1
  }

  # AC-014 PC1: timestamp MUST also have been re-stamped.
  echo "$actual_state" | grep -q "timestamp: 2020-01-01T00:00:00Z" && {
    echo "FAIL: AC-014 PC1 — timestamp was NOT re-stamped."
    echo "Stale fixture timestamp is still present. PC1 is unconditional."
    echo "Actual STATE.md after hook:"
    echo "$actual_state"
    return 1
  }

  # Belt-and-suspenders: confirm expires_at line is present and non-empty.
  echo "$actual_state" | grep -q "expires_at:" || {
    echo "FAIL: AC-014 — expires_at line missing from STATE.md after hook invocation."
    echo "The factory_lock block must retain all three fields (holder, locked_at, expires_at)."
    echo "Actual STATE.md after hook:"
    echo "$actual_state"
    return 1
  }

  # O-S1705-P1-001 (positive coverage): verify the NEW expires_at is approximately now + 2700s.
  # This assertion prevents false-green if expires_at were accidentally dropped or left
  # with an incidentally different value rather than properly renewed to now + TTL_SECONDS.
  # Renewed value must fall in [before_epoch+2700, after_epoch+2700+5] (5s fudge for CI).
  local new_expires
  new_expires="$(echo "$actual_state" | grep 'expires_at:' | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z')"

  # Validate ISO-8601 shape with bash regex — platform-portable; avoids GNU/BSD date
  # divergence for the format-validity gate.  grep -oE above already strips YAML quotes,
  # but the regex provides a load-bearing assertion and works identically on Linux and macOS.
  [[ "$new_expires" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] || {
    echo "FAIL: AC-014 O-S1705-P1-001 — cannot parse renewed expires_at as UTC ISO-8601: '${new_expires}'"
    echo "Expected a value approximately equal to now + 2700s (before_epoch=${before_epoch})."
    return 1
  }

  # Convert ISO-8601 string to epoch for window comparison.
  # Platform-portable: detect GNU vs BSD date and use the correct invocation for each.
  #   GNU date (Linux/CI): date -u -d "YYYY-MM-DDTHH:MM:SSZ" +%s  (-d parses the string;
  #                         GNU date understands the Z suffix as UTC natively)
  #   BSD date (macOS):    date -juf "%Y-%m-%dT%H:%M:%SZ" "..." +%s  (-j = don't set date,
  #                         -u = UTC, -f = input format)
  # Detection: `date --version` succeeds on GNU (prints to stdout), exits non-zero on BSD.
  local new_expires_epoch
  if date --version >/dev/null 2>&1; then
    # GNU date (Linux/CI)
    new_expires_epoch="$(date -u -d "$new_expires" +%s)"
  else
    # BSD date (macOS)
    new_expires_epoch="$(date -juf "%Y-%m-%dT%H:%M:%SZ" "$new_expires" +%s)"
  fi
  local expected_min=$(( before_epoch + 2700 ))
  local expected_max=$(( after_epoch + 2700 + 5 ))
  [ "$new_expires_epoch" -ge "$expected_min" ] || {
    echo "FAIL: AC-014 O-S1705-P1-001 — renewed expires_at (${new_expires}, epoch=${new_expires_epoch})"
    echo "  is earlier than expected minimum before_epoch(${before_epoch})+2700=${expected_min}."
    echo "  Renewal must set expires_at = now + TTL_SECONDS (2700s)."
    return 1
  }
  [ "$new_expires_epoch" -le "$expected_max" ] || {
    echo "FAIL: AC-014 O-S1705-P1-001 — renewed expires_at (${new_expires}, epoch=${new_expires_epoch})"
    echo "  exceeds expected maximum after_epoch(${after_epoch})+2700+5=${expected_max}."
    echo "  Clock skew or systematic TTL offset exceeds tolerance."
    return 1
  }
}
