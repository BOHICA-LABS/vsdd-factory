#!/usr/bin/env bats
# verify-factory-lock-read-prefix.bats — S-19.07 RED gate tests for BC-4.13.001 Phase-B.
#
# All five tests in this file are RED against Phase-A HEAD (feature/S-19.07 at 9787c056).
# They become GREEN only after the Phase-B implementation migrates
# host::read_file → host::read_prefix in verify-factory-lock.
#
# Test inventory:
#   T-001a  AC-001 Gate A: grep -q "read_prefix" lib.rs exits 0
#           RED today: no read_prefix call in lib.rs (grep exits 1)
#
#   T-001b  AC-001 Gate B: sed block-comment strip + line-comment filter + grep
#           for Phase-A symbols (host::read_file|STATE_MD_MAX_BYTES|TooLarge)
#           must exit NON-ZERO (symbols absent) after Phase-B.
#           RED today: Phase-A symbols present in non-comment code (grep exits 0).
#           Includes block-comment discriminating-fixture mutation-liveness check
#           per S-19.06 T-009g convention.
#
#   T-002-vfl  AC-002: verify-factory-lock registry entry has no [hooks.capabilities.read_file]
#           and has [hooks.capabilities.read_prefix].
#           RED today: read_file count=1, read_prefix count=0.
#
#   T-002-vfl-bash  AC-002: verify-factory-lock-bash registry entry — same gates.
#           RED today: same — both entries still have capabilities.read_file.
#
#   T-005-ec005  EC-005: capabilities.read_prefix absent (only read_file present) + foreign lock.
#           Phase-B guard calls read_prefix → CAPABILITY_DENIED → Continue (fail-open per PC6).
#           Phase-A guard calls read_file (present) → finds lock → BLOCKS (exit 2).
#           Test asserts exit 0 → RED today.
#
# Story: S-19.07
# BC: BC-4.13.001 Phase-B (capabilities.read_prefix, max_bytes=262144, ADR-025 §D15)

# ---------------------------------------------------------------------------
# setup / teardown
# ---------------------------------------------------------------------------

setup_file() {
    # Record which factory-dispatcher binary this suite exercises.
    # D-693 / F-S2107-P6-017: auditable path + sha256 + mtime via TAP comments.
    load "${BATS_TEST_DIRNAME}/helpers/dispatcher-provenance.bash"
    emit_dispatcher_provenance
}

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  PLUGIN_ROOT="$REPO_ROOT/plugins/vsdd-factory"
  DISPATCHER="$REPO_ROOT/target/release/factory-dispatcher"
  GUARD_WASM="$PLUGIN_ROOT/hook-plugins/verify-factory-lock.wasm"
  LIB_RS="$REPO_ROOT/crates/hook-plugins/verify-factory-lock/src/lib.rs"
  REGISTRY_TOML="$REPO_ROOT/plugins/vsdd-factory/hooks-registry.toml"

  WORK="$(mktemp -d)"
  mkdir -p "$WORK/.factory/logs"
  mkdir -p "$WORK/hook-plugins"

  if [ -f "$GUARD_WASM" ]; then
    cp "$GUARD_WASM" "$WORK/hook-plugins/verify-factory-lock.wasm"
  fi

  export CLAUDE_PROJECT_DIR="$WORK"
  export CLAUDE_PLUGIN_ROOT="$WORK"
}

teardown() {
  rm -rf "$WORK"
}

# ---------------------------------------------------------------------------
# Preflight helpers
# ---------------------------------------------------------------------------

# Skip if dispatcher binary or guard WASM is not present.
# SKIP != PASS — tests that skip are still RED at Red Gate time.
_require_artifacts() {
  if [ ! -x "$DISPATCHER" ]; then
    skip "factory-dispatcher binary not built — run: cargo build --release -p factory-dispatcher"
  fi
  if [ ! -f "$WORK/hook-plugins/verify-factory-lock.wasm" ]; then
    skip "verify-factory-lock.wasm not present — run: cargo build --target wasm32-wasip1 -p verify-factory-lock"
  fi
}

# ---------------------------------------------------------------------------
# T-001a  AC-001 Gate A — read_prefix present in verify-factory-lock/src/lib.rs
#
# Phase-A: no host::read_prefix call → grep exits 1 → test FAILS → RED.
# Phase-B: read_prefix call present → grep exits 0 → test PASSES → GREEN.
# ---------------------------------------------------------------------------

@test "T-001a S-19.07 AC-001 Gate A: host::read_prefix called in verify-factory-lock/src/lib.rs" {
  [ -f "$LIB_RS" ] || {
    echo "FAIL: $LIB_RS not found"
    false
  }

  # Scope to the production-code region only (before #[cfg(test)]).
  # Test code mentions "read_prefix" in error message strings, so a full-file grep
  # would give a false positive on Phase-A. The production call is what Gate A checks.
  local prod_output
  prod_output=$(
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$LIB_RS" \
      | grep -oE 'host::read_prefix' \
      || true
  )

  if [ -z "$prod_output" ]; then
    echo "FAIL: AC-001 Gate A — host::read_prefix not found in production code of lib.rs."
    echo "  Expected: Phase-B migration complete — on_pre_tool_use must call"
    echo "  host::read_prefix(path, max_bytes=262144, timeout_ms) instead of host::read_file."
    echo "  Current Phase-A state: only host::read_file is called in production code."
    echo "  (Test module mentions of read_prefix are excluded from this gate.)"
    false
  fi
}

# ---------------------------------------------------------------------------
# T-001b  AC-001 Gate B — Phase-A symbols absent from non-comment production code
#
# Gate pipeline (from BC-4.13.001 AC-001):
#   sed -E -e ':a' -e 's:/\*[^*]*\*+([^/*][^*]*\*+)*/::' -e 'ta' LIB_RS
#     | grep -vE '^\s*(//|//!|///)'
#     | grep -qE 'host::read_file|STATE_MD_MAX_BYTES|TooLarge'
#
# Phase-A: symbols found → gate exits 0 → test FAILS → RED.
# Phase-B: symbols absent → gate exits non-zero → test PASSES → GREEN.
#
# Mutation-liveness check (T-009g convention):
#   Discriminating fixture contains Phase-A symbols ONLY inside a block comment.
#   OLD gate (line-comment filter only): exits 0 — block comment not stripped, symbols
#     wrongly counted as production code.
#   NEW gate (with sed block-comment strip): exits non-zero — block comment stripped,
#     symbols correctly absent from non-comment code.
#   If the NEW gate exits 0 on the discriminating fixture, the sed chain is broken
#   (or absent) and the test fails — this proves the sed chain is load-bearing.
# ---------------------------------------------------------------------------

@test "T-001b S-19.07 AC-001 Gate B: Phase-A symbols absent from non-comment code in verify-factory-lock/src/lib.rs" {
  [ -f "$LIB_RS" ] || {
    echo "FAIL: $LIB_RS not found"
    false
  }

  # --- Mutation-liveness: block-comment discriminating fixture ---
  #
  # The fixture has Phase-A symbols ONLY inside a block comment. The NEW gate
  # (with sed block-comment strip) must NOT find them (output empty). If it does
  # find them, the sed chain is not stripping block comments → gate is broken.
  local mut_file
  mut_file="${WORK}/mutant.rs"
  printf 'fn read_data() -> i32 { /* host::read_file(path, STATE_MD_MAX_BYTES) TooLarge check */ 0 }\n' \
    > "$mut_file"

  local mutant_output
  mutant_output=$(
    sed -E -e ':a' -e 's:/\*[^*]*\*+([^/*][^*]*\*+)*/::' -e 'ta' "$mut_file" \
      | grep -vE '^\s*(//|//!|///)' \
      | grep -oE 'host::read_file|STATE_MD_MAX_BYTES|TooLarge' \
      || true
  )
  rm -f "$mut_file"

  if [ -n "$mutant_output" ]; then
    echo "FAIL: mutation-liveness (block-comment discriminating fixture) —"
    echo "  NEW gate found symbols in a file where they exist ONLY inside a block comment."
    echo "  sed block-comment strip chain is not functioning: found '$mutant_output'."
    echo "  Expected: empty output (symbols absent from non-comment code in fixture)."
    false
  fi
  echo "PASS mutation-liveness: sed chain correctly strips block comments (no symbols found in fixture)"

  # --- Primary check: Gate B on actual lib.rs (production code only) ---
  #
  # Scope: awk strips the test module (stops at #[cfg(test)]). Test-module error
  # message strings contain "host::read_file" and "STATE_MD_MAX_BYTES" as literal
  # text; without scoping, Gate B would never go GREEN after the test code is added.
  #
  # Phase-A: production code has host::read_file / STATE_MD_MAX_BYTES / TooLarge
  #          → output non-empty → FAIL → RED.
  # Phase-B: production code migrated → output empty → PASS → GREEN.
  local actual_output
  actual_output=$(
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$LIB_RS" \
      | sed -E -e ':a' -e 's:/\*[^*]*\*+([^/*][^*]*\*+)*/::' -e 'ta' \
      | grep -vE '^\s*(//|//!|///)' \
      | grep -oE 'host::read_file|STATE_MD_MAX_BYTES|TooLarge' \
      || true
  )

  if [ -n "$actual_output" ]; then
    echo "FAIL: AC-001 Gate B — Phase-A symbols still present in non-comment production code:"
    echo "  Found: $(printf '%s' "$actual_output" | tr '\n' ' ')"
    echo "  After Phase-B migration, host::read_file, STATE_MD_MAX_BYTES, and TooLarge"
    echo "  must be absent from non-comment production code in lib.rs."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-002-vfl  AC-002 — verify-factory-lock registry entry migrated to read_prefix
#
# Per-entry awk scoping (from BC-4.13.001 AC-002 gate spec):
#   Isolate lines belonging to the 'verify-factory-lock' entry only.
#   The entry starts at 'name = "verify-factory-lock"' and resets at '[[hooks]]'.
#
# Phase-A: read_file count=1, read_prefix count=0 → both assertions FAIL → RED.
# Phase-B: read_file count=0, read_prefix count>=1 → both PASS → GREEN.
# ---------------------------------------------------------------------------

@test "T-002-vfl S-19.07 AC-002: verify-factory-lock registry entry has capabilities.read_prefix not capabilities.read_file" {
  [ -f "$REGISTRY_TOML" ] || {
    echo "FAIL: $REGISTRY_TOML not found"
    false
  }

  local read_file_count
  read_file_count=$(
    awk 'BEGIN{p=0} /^\[\[hooks\]\]/{p=0} /^name = "verify-factory-lock"$/{p=1} p' \
      "$REGISTRY_TOML" | grep -c '\[hooks\.capabilities\.read_file\]' || true
  )
  local read_prefix_count
  read_prefix_count=$(
    awk 'BEGIN{p=0} /^\[\[hooks\]\]/{p=0} /^name = "verify-factory-lock"$/{p=1} p' \
      "$REGISTRY_TOML" | grep -c '\[hooks\.capabilities\.read_prefix\]' || true
  )

  local failed=0

  if [ "$read_file_count" -ne 0 ]; then
    echo "FAIL: verify-factory-lock entry still has [hooks.capabilities.read_file] (count=$read_file_count)."
    echo "  After Phase-B migration, capabilities.read_file must be replaced by capabilities.read_prefix."
    failed=1
  fi

  if [ "$read_prefix_count" -lt 1 ]; then
    echo "FAIL: verify-factory-lock entry has no [hooks.capabilities.read_prefix] (count=$read_prefix_count)."
    echo "  After Phase-B migration, the entry must declare [hooks.capabilities.read_prefix]"
    echo "  with path_allow = [\".factory/STATE.md\"] (BC-4.13.001 Phase-B §Precondition 3)."
    failed=1
  fi

  [ "$failed" -eq 0 ]
}

# ---------------------------------------------------------------------------
# T-002-vfl-bash  AC-002 — verify-factory-lock-bash registry entry migrated to read_prefix
#
# Same gates as T-002-vfl but scoped to the 'verify-factory-lock-bash' entry.
# Phase-A: read_file count=1, read_prefix count=0 → FAIL → RED.
# Phase-B: read_file count=0, read_prefix count>=1 → PASS → GREEN.
# ---------------------------------------------------------------------------

@test "T-002-vfl-bash S-19.07 AC-002: verify-factory-lock-bash registry entry has capabilities.read_prefix not capabilities.read_file" {
  [ -f "$REGISTRY_TOML" ] || {
    echo "FAIL: $REGISTRY_TOML not found"
    false
  }

  local read_file_count
  read_file_count=$(
    awk 'BEGIN{p=0} /^\[\[hooks\]\]/{p=0} /^name = "verify-factory-lock-bash"$/{p=1} p' \
      "$REGISTRY_TOML" | grep -c '\[hooks\.capabilities\.read_file\]' || true
  )
  local read_prefix_count
  read_prefix_count=$(
    awk 'BEGIN{p=0} /^\[\[hooks\]\]/{p=0} /^name = "verify-factory-lock-bash"$/{p=1} p' \
      "$REGISTRY_TOML" | grep -c '\[hooks\.capabilities\.read_prefix\]' || true
  )

  local failed=0

  if [ "$read_file_count" -ne 0 ]; then
    echo "FAIL: verify-factory-lock-bash entry still has [hooks.capabilities.read_file] (count=$read_file_count)."
    echo "  After Phase-B migration, capabilities.read_file must be replaced by capabilities.read_prefix."
    failed=1
  fi

  if [ "$read_prefix_count" -lt 1 ]; then
    echo "FAIL: verify-factory-lock-bash entry has no [hooks.capabilities.read_prefix] (count=$read_prefix_count)."
    echo "  After Phase-B migration, the Bash-tool entry must also declare capabilities.read_prefix."
    failed=1
  fi

  [ "$failed" -eq 0 ]
}

# ---------------------------------------------------------------------------
# Registry writer for T-005-ec005 (EC-005 misconfiguration scenario)
#
# Writes a registry where verify-factory-lock has capabilities.read_file
# and capabilities.exec_subprocess but NOT capabilities.read_prefix.
#
# EC-005: after Phase-B migration, if a deployer forgets to add capabilities.read_prefix
# (only copies the old capabilities.read_file), the Phase-B guard calls read_prefix
# which is denied → CAPABILITY_DENIED → Continue (fail-open per PC6).
#
# Phase-A guard calls read_file (present in this registry) → succeeds → finds lock
# → BLOCKS (exit 2). This makes T-005-ec005 RED against Phase-A.
# ---------------------------------------------------------------------------

_write_ec005_read_file_only_registry() {
  cat > "$WORK/hooks-registry.toml" <<'EOF'
schema_version = 2

# EC-005 misconfiguration fixture: capabilities.read_file present, read_prefix absent.
# Phase-B guard calls read_prefix → CAPABILITY_DENIED → Continue (fail-open per PC6).
# Phase-A guard calls read_file (present) → finds lock → BLOCKS.
[[hooks]]
name = "verify-factory-lock"
plugin = "hook-plugins/verify-factory-lock.wasm"
event = "PreToolUse"
tool = "Edit|Write|MultiEdit|Agent"
async = false
on_error = "continue"
timeout_ms = 5000

[hooks.capabilities.read_file]
path_allow = [".factory/STATE.md"]

[hooks.capabilities.exec_subprocess]
binary_allow = ["git"]
env_allow = ["HOME", "GIT_CONFIG_GLOBAL", "XDG_CONFIG_HOME"]
EOF
}

# ---------------------------------------------------------------------------
# T-005-ec005  EC-005 — capabilities.read_prefix absent → graceful degrade to Continue
#
# Setup:
#   - Registry has capabilities.read_file (NOT read_prefix) + exec_subprocess.
#   - STATE.md has a foreign unexpired lock.
#
# Phase-A behavior (today, RED gate):
#   guard calls host::read_file (capability present) → reads STATE.md → finds foreign lock
#   → BLOCKS (exit 2) → test asserts exit 0 → FAILS → RED.
#
# Phase-B behavior (GREEN after migration):
#   guard calls host::read_prefix (capability absent) → CAPABILITY_DENIED (-1)
#   → StateReadError → Continue (fail-open per BC-4.13.001 PC6) → exit 0 → PASSES.
# ---------------------------------------------------------------------------

@test "T-005-ec005 S-19.07 EC-005: capabilities.read_prefix absent → graceful degrade to Continue" {
  _require_artifacts

  _write_ec005_read_file_only_registry

  mkdir -p "$WORK/.factory"
  cat > "$WORK/.factory/STATE.md" <<'EOF'
---
document_type: state
version: "0.0.1-ec005-test"
phase: test
current_step: "bats-test"
factory_lock:
  holder: "other@example.com"
  locked_at: "2026-06-10T14:00:00Z"
  expires_at: "2099-01-01T00:00:00Z"
---

# STATE (EC-005 fixture — foreign unexpired lock + capabilities.read_prefix absent)
EOF

  local envelope
  envelope='{"event_name":"PreToolUse","tool_name":"Edit","session_id":"ec005","dispatcher_trace_id":"ec005-trace","tool_input":{"file_path":".factory/STATE.md"}}'

  run bash -c "printf '%s' '$envelope' | \
    CLAUDE_PLUGIN_ROOT='$WORK' \
    CLAUDE_PROJECT_DIR='$WORK' \
    '$DISPATCHER' 2>&1 >/dev/null"

  # Phase-B: exits 0 (Continue — read_prefix denied, fail-open per PC6).
  # Phase-A: exits 2 (Block — read_file present, lock found) → RED gate.
  [ "$status" -eq 0 ]
}
