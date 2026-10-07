#!/usr/bin/env bats
# validate-cross-site-correspondence.bats — Integration tests for the
# validate-cross-site-correspondence PostToolUse WASM hook plugin.
#
# These tests deliver bats payload-delivery scenarios covering the six blocking
# arms (A1, A2, B1, B2, E1, E2) via fixture files. Class D (arm_d.rs, formerly
# AC-012/013/014) is deferred to S-21.08 and was removed from the
# implementation; it has no coverage in this suite.
#
# This file does NOT enumerate a contiguous "AC-001 through AC-NNN" span —
# that claim decays every time an AC is added, renumbered, or moved to a
# different test layer, and has repeatedly gone stale. The AUTHORITATIVE
# per-AC coverage mapping (which ACs are bats vs Rust-unit vs manual) lives
# in the story's Test Plan (S-21.07, .factory/stories/S-21.07-validate-cross-
# site-correspondence.md, "## Test Plan" section) — consult it, not this
# comment, for AC-to-test-layer traceability. Some ACs are intentionally
# covered outside this bats suite: e.g. AC-028 (BC-5.39.010 precondition 15b
# / postcondition 26, secondary index-file UTF-8 decode advisory) is
# Rust-unit only, in arm_a1.rs and arm_b.rs.
#
# RED GATE (BC-5.38.001): Every payload-driven test MUST FAIL before implementation.
#
# Failure mechanism:
#   - The stub WASM panics on every code path via todo!() bodies.
#   - The dispatcher's on_error="continue" swallows the panic and returns exit 0.
#   - Exit code alone cannot distinguish "plugin ran correctly" from "plugin crashed".
#   - The internal dispatcher log (VSDD_LOG_DIR) records plugin.crashed on every panic.
#
# Discriminating signal: _assert_plugin_ran_not_crashed reads
#   $WORK/.factory/logs/dispatcher-internal-YYYY-MM-DD.jsonl
# and asserts that:
#   (a) a plugin.completed record exists for validate-cross-site-correspondence, AND
#   (b) no plugin.crashed OR plugin.timeout record exists for it.
# Under the stub, condition (a) always fails (stub panics → plugin.crashed, not plugin.completed).
# → ALL 40+ payload tests FAIL in Red Gate.
# After implementation, the stub is replaced with real logic, plugin.completed fires,
# and the tests pass or fail based on actual arm behavior.
#
# NOTE: VSDD_SINK_FILE is not functional in rc.23 (the file is never created).
# The internal log at VSDD_LOG_DIR/dispatcher-internal-YYYY-MM-DD.jsonl IS functional.
#
# AC-019 cap-passing: The constant VALUES (BC_INDEX_MAX_BYTES etc.) are verified by a
# unit test in arm_a1.rs that PASSES in Red Gate. Cap-PASSING (that those constants are
# actually passed to host::read_file) cannot be tested from the pure seam — it requires
# a post-implementation bats integration test with a fixture sized at the cap boundary.
# The implementer must add that test. See arm_a1.rs test_BC_5_39_010_AC019_... for details.
#
# AC-020 registry shape tests: PASS in Red Gate legitimately — they grep the TOML registry
# file directly and do not invoke the dispatcher at all. Their passing is expected and correct.
#
# Governing BC: BC-5.39.010
# Story: S-21.07
#
# Fixture layout: fixtures/validate-cross-site-correspondence/<scenario>/factory/
# The "factory/" subtree is copied to $WORK/.factory/ in each test setup.

FIXTURE_BASE="$(cd "$(dirname "$BATS_TEST_FILENAME")/fixtures/validate-cross-site-correspondence" && pwd)"

# ---------------------------------------------------------------------------
# setup_file: once-per-suite provenance record.
# F-P6-017: emit factory-dispatcher binary provenance to TAP comment stream
# so every run records which binary was exercised, its sha256, and mtime.
# D-693: "N/N GREEN" attestation is only meaningful when the specific binary
# is recorded. Mirrors helpers/dispatcher-provenance.bash from develop.
# ---------------------------------------------------------------------------

setup_file() {
  load "${BATS_TEST_DIRNAME}/helpers/dispatcher-provenance.bash"
  # Auto-resolve: debug preferred over release; emits to >&3 (TAP comments).
  # Under CI_REQUIRE_ARTIFACTS=1, returns non-zero if binary not found.
  emit_dispatcher_provenance
}

# ---------------------------------------------------------------------------
# setup / teardown
# ---------------------------------------------------------------------------

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  PLUGIN_ROOT="$REPO_ROOT/plugins/vsdd-factory"

  # Dispatcher: debug preferred over release; NO operator-cache fallback.
  # F-P6-017 / D-693: the operator-cache binary is only updated on release —
  # silently falling back to it would validate the wrong binary under development.
  # An absent local build produces a loud FATAL rather than a silent stale result.
  DISPATCHER="${REPO_ROOT}/target/debug/factory-dispatcher"
  if [[ ! -x "${DISPATCHER}" ]]; then
    DISPATCHER="${REPO_ROOT}/target/release/factory-dispatcher"
  fi
  if [[ ! -x "${DISPATCHER}" ]]; then
    echo "FATAL: factory-dispatcher not found (tried debug + release builds)." >&2
    echo "  Run: cargo build -p factory-dispatcher" >&2
    return 1
  fi

  GUARD_WASM="$PLUGIN_ROOT/hook-plugins/validate-cross-site-correspondence.wasm"

  WORK="$(mktemp -d)"
  mkdir -p "$WORK/.factory/logs"
  mkdir -p "$WORK/hook-plugins"

  if [ -f "$GUARD_WASM" ]; then
    cp "$GUARD_WASM" "$WORK/hook-plugins/validate-cross-site-correspondence.wasm"
  fi

  export VSDD_LOG_DIR="$WORK/.factory/logs"
}

teardown() {
  [ -n "${WORK:-}" ] && rm -rf "$WORK"
}

# ---------------------------------------------------------------------------
# Preflight helpers
# ---------------------------------------------------------------------------

# Skip if dispatcher binary or WASM is not present.
# Set CI_REQUIRE_ARTIFACTS=1 to convert skips to hard failures.
_require_artifacts() {
  if [ -z "$DISPATCHER" ] || [ ! -x "$DISPATCHER" ]; then
    [ -z "${CI_REQUIRE_ARTIFACTS:-}" ] || {
      echo "FAIL: factory-dispatcher binary not present (CI_REQUIRE_ARTIFACTS=1)."
      echo "  Run: cargo build --release -p factory-dispatcher"
      return 1
    }
    skip "factory-dispatcher binary not found — run: cargo build --release -p factory-dispatcher"
  fi
  if [ ! -f "$WORK/hook-plugins/validate-cross-site-correspondence.wasm" ]; then
    [ -z "${CI_REQUIRE_ARTIFACTS:-}" ] || {
      echo "FAIL: validate-cross-site-correspondence.wasm not staged."
      echo "  Run: cargo build --release --target wasm32-wasip1 -p validate-cross-site-correspondence"
      echo "  Then: cp target/wasm32-wasip1/release/validate-cross-site-correspondence.wasm plugins/vsdd-factory/hook-plugins/"
      return 1
    }
    skip "validate-cross-site-correspondence.wasm not staged — see story S-21.07 AC-021"
  fi
}

# ---------------------------------------------------------------------------
# Fixture loader: copy scenario's factory/ subtree into $WORK/.factory/
# ---------------------------------------------------------------------------

_load_fixture() {
  local scenario="$1"
  local src="$FIXTURE_BASE/$scenario/factory"
  if [ -d "$src" ]; then
    cp -r "$src/." "$WORK/.factory/"
  fi
}

# ---------------------------------------------------------------------------
# Registry writer: write a synthetic hooks-registry.toml with one entry.
# Optional argument overrides the path_allow list (double-quoted TOML strings,
# comma-separated). Default: production path_allow — three prefixes, matching
# hooks-registry.toml for validate-cross-site-correspondence (PG-S-15.11).
# ---------------------------------------------------------------------------

_write_registry() {
  # Build the TOML path_allow list. Default matches the three production
  # path_allow prefixes in plugins/vsdd-factory/hooks-registry.toml.
  # Callers can pass a custom list as the first argument.
  local path_allow_lines
  if [ $# -ge 1 ]; then
    path_allow_lines="$1"
  else
    path_allow_lines='".factory/specs/behavioral-contracts/",
  ".factory/specs/verification-properties/",
  ".factory/stories/"'
  fi

  cat > "$WORK/hooks-registry.toml" <<TOML
schema_version = 2

[[hooks]]
name = "validate-cross-site-correspondence"
event = "PostToolUse"
tool = "^(Edit|Write|MultiEdit)\$"
plugin = "hook-plugins/validate-cross-site-correspondence.wasm"
priority = 460
timeout_ms = 8000
on_error = "continue"
async = false

[hooks.capabilities.read_file]
path_allow = [
  $path_allow_lines
]
TOML
}

# ---------------------------------------------------------------------------
# Envelope builder: PostToolUse Write event with given file_path.
# ---------------------------------------------------------------------------

_post_write_event() {
  local file_path="$1"
  local session="${2:-test-vcsc}"
  local escaped
  escaped="$(printf '%s' "$file_path" | sed 's/\\/\\\\/g; s/"/\\"/g')"
  printf '{"event_name":"PostToolUse","tool_name":"Write","session_id":"%s","dispatcher_trace_id":"%s-trace","tool_input":{"file_path":"%s","content":""},"tool_response":{}}' \
    "$session" "$session" "$escaped"
}

# ---------------------------------------------------------------------------
# Dispatcher runner: writes envelope to file and invokes dispatcher.
# Captures stdout in $output (bats convention); stderr in $_DISP_STDERR.
# ---------------------------------------------------------------------------

_run_dispatcher() {
  local envelope="$1"
  local env_file="$WORK/envelope-$$.json"
  _DISP_STDERR="$WORK/dispatcher-stderr-$$.txt"
  printf '%s' "$envelope" > "$env_file"
  run bash -c "VSDD_LOG_DIR='$WORK/.factory/logs' \
    CLAUDE_PLUGIN_ROOT='$WORK' \
    CLAUDE_PROJECT_DIR='$WORK' \
    '$DISPATCHER' < '$env_file' 2>'$_DISP_STDERR'"
}

# Assert exit code and optionally check combined output for a pattern.
_assert_exit() {
  local expected="$1"
  local pattern="${2:-}"
  local combined
  combined="$(cat "$_DISP_STDERR" 2>/dev/null) $output"

  [ "$status" -eq "$expected" ] || {
    echo "FAIL: expected exit $expected, got $status"
    echo "  Stderr: $(cat "$_DISP_STDERR" 2>/dev/null)"
    echo "  Stdout: $output"
    false
  }

  if [ -n "$pattern" ]; then
    [[ "$combined" == *"$pattern"* ]] || {
      echo "FAIL: expected pattern '$pattern' not found in output"
      echo "  Combined: $combined"
      false
    }
  fi
}

# ---------------------------------------------------------------------------
# Internal log helpers — telemetry-based plugin outcome assertion.
#
# Signal source: $WORK/.factory/logs/dispatcher-internal-YYYY-MM-DD.jsonl
# Each test has its own $WORK (fresh mktemp -d), so the log is test-isolated.
#
# VSDD_SINK_FILE is NOT functional in the current dispatcher build (rc.23):
# the file is never created even when set. The internal log IS functional and
# captures plugin.invoked, plugin.crashed, plugin.completed, etc.
# ---------------------------------------------------------------------------

# Return the path to today's internal dispatcher log in $WORK.
_plugin_log() {
  echo "$WORK/.factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl"
}

# Assert that validate-cross-site-correspondence was invoked AND did NOT crash.
#
# In Red Gate: FAILS because the stub panics → plugin.crashed (not plugin.completed) in log.
# Post-implementation: PASSES when the plugin runs to completion (plugin.completed fires).
#
# RG-006 (T-036): This helper is itself tested by a permanent mutant below that injects
# a plugin.crashed-only log and asserts the helper fails. See T-036 comment block.
#
# This makes ALL 37+ payload-driven tests fail in Red Gate — even controls asserting
# exit 0, which previously passed vacuously because on_error="continue" swallows panics.
# BC-5.38.001 Red Gate: every test must fail for the right reason, not by exit-code coincidence.
_assert_plugin_ran_not_crashed() {
  local log; log="$(_plugin_log)"

  # Must have a plugin.completed record — confirms execution ran to completion.
  # plugin.invoked is written BEFORE execution starts; only plugin.completed proves
  # the plugin ran its full code path without crashing or timing out.
  if ! grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
       | grep -q '"type":"plugin.completed"'; then
    echo "FAIL: no plugin.completed record for validate-cross-site-correspondence"
    echo "  Plugin never ran, crashed before completion, or timed out."
    echo "  Log path: $log"
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null | head -5 \
      || echo "  (no matching records in log)"
    false
    return
  fi

  # Must NOT have a plugin.crashed record — confirms the plugin did not panic.
  if grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
       | grep -q '"type":"plugin.crashed"'; then
    echo "FAIL: plugin.crashed found for validate-cross-site-correspondence"
    echo "  The plugin panicked (todo!() body). Correct Red Gate failure."
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
      | grep '"type":"plugin.crashed"' | head -3
    false
    return
  fi

  # Must NOT have a plugin.timeout record — confirms the plugin did not exceed budget.
  if grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
       | grep -q '"type":"plugin.timeout"'; then
    echo "FAIL: plugin.timeout found for validate-cross-site-correspondence"
    echo "  Plugin exceeded its time budget — check WASM fuel or timeout_ms setting."
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
      | grep '"type":"plugin.timeout"' | head -3
    false
    return
  fi
}

# ---------------------------------------------------------------------------
# T-033 (AC-020): Registry entry present with all required fields.
# These two tests PASS in Red Gate legitimately — they grep the TOML registry
# file directly and do not invoke the dispatcher. No _assert_plugin_ran_not_crashed
# is called here because no plugin execution occurs.
# ---------------------------------------------------------------------------

@test "AC-020: registry entry has all 8 required fields" {
    local registry="$BATS_TEST_DIRNAME/../../../plugins/vsdd-factory/hooks-registry.toml"
    run grep -A 20 'name = "validate-cross-site-correspondence"' "$registry"
    [ "$status" -eq 0 ]
    [[ "$output" == *'event = "PostToolUse"'* ]]
    [[ "$output" == *'tool = "^(Edit|Write|MultiEdit)$"'* ]]
    [[ "$output" == *'plugin = "hook-plugins/validate-cross-site-correspondence.wasm"'* ]]
    [[ "$output" == *'priority = 460'* ]]
    [[ "$output" == *'timeout_ms = 8000'* ]]
    [[ "$output" == *'on_error = "continue"'* ]]
    [[ "$output" == *'async = false'* ]]
}

@test "AC-020: registry entry has no fuel_cap field" {
    local registry="$BATS_TEST_DIRNAME/../../../plugins/vsdd-factory/hooks-registry.toml"
    run awk '/name = "validate-cross-site-correspondence"/,/^\[\[hooks\]\]/' "$registry"
    [ "$status" -eq 0 ]
    # Guard: entry must exist — empty output means the section was never found,
    # which would make the fuel_cap absence check vacuously true.
    [ -n "$output" ] || {
      echo "FAIL: no 'validate-cross-site-correspondence' section found in registry"
      false
    }
    # fuel_cap must NOT be present (BC-5.39.010 §Gate Spec; ADR-035 §Decision 5)
    [[ "$output" != *"fuel_cap"* ]]
}

# ---------------------------------------------------------------------------
# PG-S-15.11: _write_registry default path_allow parity guard.
#
# Asserts that the bats _write_registry helper's default path_allow array is
# byte-identical to the production path_allow in hooks-registry.toml for
# validate-cross-site-correspondence. Catches silent drift so that integration
# tests always run against the same capabilities the production plugin sees.
#
# ALWAYS PASSES (no dispatcher invocation, no WASM execution). The test reads
# two TOML files and compares extracted path entries.
# ---------------------------------------------------------------------------

@test "PG-S-15.11: _write_registry default path_allow matches production hooks-registry.toml" {
  local registry_path
  registry_path="$BATS_TEST_DIRNAME/../../../plugins/vsdd-factory/hooks-registry.toml"

  [ -f "$registry_path" ] || {
    echo "FAIL: production hooks-registry.toml not found at $registry_path"
    false
  }

  # Extract path_allow entries from production registry (under validate-cross-site-correspondence)
  local prod_entries
  prod_entries="$(grep -A 50 '"validate-cross-site-correspondence"' "$registry_path" \
    | awk '/path_allow = \[/{in_arr=1; next} in_arr && /\]/{in_arr=0} in_arr{gsub(/[[:space:]",]/, ""); if(length($0)>0) print}' \
    | sort)"

  # Write default registry and extract its path_allow entries
  _write_registry
  local bats_entries
  bats_entries="$(awk '/path_allow = \[/{in_arr=1; next} in_arr && /\]/{in_arr=0} in_arr{gsub(/[[:space:]",]/, ""); if(length($0)>0) print}' \
    "$WORK/hooks-registry.toml" | sort)"

  [ "$prod_entries" = "$bats_entries" ] || {
    echo "FAIL: bats _write_registry default path_allow does not match production hooks-registry.toml"
    echo "  Production entries (sorted):"
    printf '%s\n' "$prod_entries" | while IFS= read -r line; do echo "    $line"; done
    echo "  Bats default entries (sorted):"
    printf '%s\n' "$bats_entries" | while IFS= read -r line; do echo "    $line"; done
    echo "  PG-S-15.11: update _write_registry default to match production path_allow"
    false
  }
}

# ---------------------------------------------------------------------------
# T-036 (RG-006) PERMANENT MUTANT: _assert_plugin_ran_not_crashed self-test.
#
# This test validates the helper function itself — not a plugin execution.
# It ALWAYS PASSES (both in Red Gate and after implementation) because it tests
# that the helper correctly detects a plugin.crashed-only log as a failure.
#
# RG-006: without this mutant, a bug that makes _assert_plugin_ran_not_crashed
# return success on a crashed-only log would silently invalidate ALL 37+ tests.
# The helper must fail when:
#   (a) only plugin.crashed record exists (no plugin.completed), AND
#   (b) only plugin.timeout record exists (no plugin.completed).
#
# ALWAYS PASSES: This is a meta-test of the Red Gate mechanism, not of plugin
# behavior. It passes both in Red Gate and post-implementation.
# ---------------------------------------------------------------------------

@test "T-036 RG-006 PERMANENT MUTANT: _assert_plugin_ran_not_crashed fails on crashed-only log" {
  # Create a fake internal log with ONLY plugin.crashed (no plugin.completed)
  local fake_log
  fake_log="$WORK/.factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl"
  mkdir -p "$(dirname "$fake_log")"
  printf '{"type":"plugin.crashed","plugin_name":"validate-cross-site-correspondence","message":"todo!() panic"}\n' > "$fake_log"

  # _assert_plugin_ran_not_crashed must FAIL with this log
  # (no plugin.completed record → first check fails)
  if _assert_plugin_ran_not_crashed 2>/dev/null; then
    echo "FAIL: _assert_plugin_ran_not_crashed must fail when only plugin.crashed in log"
    echo "  (no plugin.completed record found — stub panic not detected)"
    false
  fi
  # Test passes: helper correctly returned failure for crashed-only log
}

@test "T-036b RG-006 PERMANENT MUTANT: _assert_plugin_ran_not_crashed fails on timeout-only log" {
  # Create a fake internal log with ONLY plugin.timeout (no plugin.completed)
  local fake_log
  fake_log="$WORK/.factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl"
  mkdir -p "$(dirname "$fake_log")"
  printf '{"type":"plugin.timeout","plugin_name":"validate-cross-site-correspondence","message":"exceeded budget"}\n' > "$fake_log"

  # _assert_plugin_ran_not_crashed must FAIL (no plugin.completed AND has timeout record)
  if _assert_plugin_ran_not_crashed 2>/dev/null; then
    echo "FAIL: _assert_plugin_ran_not_crashed must fail when only plugin.timeout in log"
    false
  fi
}

# ---------------------------------------------------------------------------
# F-S2107-P8-013: Production-scale fuel-budget gate.
#
# F-S2107-P7-010 (fuel axis closure): the WASM sandbox must scan the live-scale
# BC-INDEX.md without exhausting its fuel budget. The largest fixture before this
# test was 2,383 bytes; the live BC-INDEX.md is 576,396 bytes — a 241× gap.
# With on_error="continue", fuel exhaustion converts to HookResult::Continue with
# no violation and no advisory — output-identical to a clean corpus. No gate
# previously distinguished "validated 1,985 rows, found nothing" from "exhausted
# fuel on row 200 and returned Continue."
#
# This test closes the sandbox × production-scale combination:
#   Fixture: a1-production-scale — BC-INDEX.md is a copy of the live file
#     (576,396 bytes, ~1985 rows; comparable in row count to production corpus).
#   Trigger: write BC-5.39.010.md (frontmatter version="1.14", matches INDEX v1.14
#     last-chain-entry) → Arm A1 happy-path scan across the full index.
#   Assertion: plugin.completed present AND plugin.timeout absent
#     (via _assert_plugin_ran_not_crashed, which already checks both conditions;
#     proved non-vacuous by T-036b: a timeout-only log makes the helper fail).
#   Anti-vacuity: exit 0 from _assert_plugin_ran_not_crashed proves the plugin
#     ran to completion; a fuel-exhausted plugin would produce plugin.timeout
#     (timeout-only log) → _assert_plugin_ran_not_crashed FAILS.
#
# Proof the assertion can fail: T-036b (bats line ~410) feeds a synthetic
# timeout-only log and asserts _assert_plugin_ran_not_crashed returns failure.
# That self-test is the permanent mutant demonstrating the assertion has teeth.
# ---------------------------------------------------------------------------

@test "F-S2107-P8-013: production-scale BC-INDEX (576KB) — plugin.completed present, plugin.timeout absent" {
  _require_artifacts
  _load_fixture "a1-production-scale"
  _write_registry

  local envelope
  # Trigger: write BC-5.39.010.md (version="1.14") against the 576 KB BC-INDEX.md.
  # BC-5.39.010 v1.14 is the last chain entry in the live index (confirmed at fixture
  # creation: "v1.10 \| v1.11 \| v1.12 \| v1.13 \| v1.14").
  # Happy path: frontmatter matches last-chain-entry → no block, exit 0.
  # The assertion is not about the exit code but about plugin.completed vs plugin.timeout.
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  # plugin.completed must be present AND plugin.timeout must be absent.
  # _assert_plugin_ran_not_crashed checks both conditions (see T-036 / T-036b for
  # proof that both checks have teeth against crashed-only and timeout-only logs).
  _assert_plugin_ran_not_crashed

  # Also assert exit 0 (happy-path: version matches, no block).
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-001: Class A Arm1 — index-newer-than-primary blocks (traces to BC-5.39.010 postcondition 2b)
# Fixture: a1-stale-index (BC v1.5, INDEX v1.6 — index NEWER than primary)
# BC-5.39.010 §PC2b: index_version > fm_version → anomalous → Block [Class A Arm1]
# Message must contain "index is newer than primary" and "POLICY 14 leg 5".
# (AC-022 / T-P6A covers the opposite direction: primary-newer → advisory.)
# ---------------------------------------------------------------------------

@test "AC-001 MUTANT: index-newer-than-primary BC-INDEX row blocks with prescribed text" {
  _require_artifacts
  _load_fixture "a1-stale-index"
  _write_registry

  local envelope
  # BC frontmatter version="1.5"; INDEX row cites v1.6.
  # index (1.6) > primary (1.5) → PC2b → anomalous → BLOCK.
  # RED GATE: v1.10 impl blocks but with wrong message (no "index is newer than primary").
  # After v1.11 implementation: PC2b block with normative text → PASSES.
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class A Arm1]"

  # AC-001 normative assertions: full-string equality on the complete PC2b block message.
  # BC-5.39.010 §PC4a: test-writer MUST assert COMPLETE formatted string by equality
  # check; .contains()-only is NON-CONFORMING. The exit-code check above confirms detection;
  # these assertions confirm the normative text is reproduced verbatim.
  # Fixture a1-stale-index: BC frontmatter v1.5; BC-INDEX row cites v1.6.
  local actual_block_reason
  actual_block_reason=$(python3 -c "
import sys
data = open(sys.argv[1]).read()
for line in data.splitlines():
    key = 'block_reason=\"'
    if key in line:
        idx = line.find(key) + len(key)
        val = line[idx:]
        if val.endswith('\"'):
            val = val[:-1]
        val = val.replace('\\\\n', '\\n').replace('\\\\r', '\\r')
        print(val, end='')
        break
" "$_DISP_STDERR" 2>/dev/null || true)

  local expected_decoded
  # Full block_reason as surfaced by dispatcher stderr (includes BLOCKED by / [N] / Fix / Code
  # wrapper from HookResult::block_with_fix + combine_violations_into_block; trailing period
  # on description stripped by trim_end_matches('.') in block_with_fix implementation).
  expected_decoded=$(python3 -c "print('BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md body-table row for BC-5.39.010 cites v1.6 but frontmatter version: is \"1.5\" — index is newer than primary. This is anomalous: the index cannot legitimately advance ahead of the BC it cites. Verify no index row was updated out-of-burst or under the wrong BC path. Update per POLICY 14 leg 5. Fix: review and fix all cross-site correspondence issues listed above, then retry the write. Code: POLICY 14/18.', end='')" 2>/dev/null || true)

  [ "$actual_block_reason" = "$expected_decoded" ] || {
    echo "FAIL: AC-001 block message does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_block_reason"
    echo "  (empty actual = block_reason not found in dispatcher stderr; mismatch = wrong message)"
    false
  }

  # Anti-vacuity: a message containing only the old substrings but wrong format fails equality.
  local wrong_substr_msg
  wrong_substr_msg="(WRONG FORMAT) index is newer than primary. POLICY 14 leg 5."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

@test "AC-001 CONTROL: current BC-INDEX row produces exit code 0" {
  _require_artifacts
  _load_fixture "a1-current-index"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# T-039: Class A Arm1 — escaped-pipe version chain CONTROL
# Fixture: a1-escaped-pipe-current (BC-1.13.001 v1.12; INDEX row v1.3 \| ... \| v1.12)
# F-S2107-P1B-006: escaped-pipe chain → extract_bc_index_version returns FIRST token "1.3"
# F-S2107-P1B-007: frontmatter changelog line with pipe also false-matches before body row
# BC-5.39.010 PC5/PC6: LAST version token in escaped-pipe chain is authoritative.
# ---------------------------------------------------------------------------

@test "T-039 CONTROL: escaped-pipe version chain last token matches — exit code 0" {
  _require_artifacts
  _load_fixture "a1-escaped-pipe-current"
  _write_registry

  local envelope
  # BC-1.13.001 at v1.12; INDEX row v1.3 \| ... \| v1.12
  # After fix: last token "1.12" matched → no violation → exit 0
  # F-S2107-P1B-006 RED GATE: split('|') returns first token "1.3" → "1.3"≠"1.12" → exit 2 → FAILS
  # F-S2107-P1B-007 RED GATE: frontmatter changelog line matches first → "4.43"≠"1.12" → exit 2 → FAILS
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-01/BC-1.13.001.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-002: v1.0 not in INDEX → advisory; v1.1 not in INDEX → block
# BC-5.39.010 PC3 (advisory) + PC4 (block)
# ---------------------------------------------------------------------------

@test "AC-002: v1.0 BC not in INDEX produces exit code 0 (advisory)" {
  _require_artifacts
  _load_fixture "a1-v1-0-not-in-index"
  _write_registry

  local envelope
  # BC-9.99.001 at version "1.0", not in INDEX → advisory only (no block)
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

@test "AC-002: v1.1 BC not in INDEX produces exit code 2 (block)" {
  _require_artifacts
  _load_fixture "a1-v1-1-not-in-index"
  _write_registry

  local envelope
  # BC-9.99.001 at version "1.1", not in INDEX → block (dropped registration)
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2
}

# ---------------------------------------------------------------------------
# AC-003: BC-INDEX.md CapabilityDenied blocks; NotFound is advisory
# BC-5.39.010 invariant 5 (CapabilityDenied) + precondition 8 (NotFound)
# ---------------------------------------------------------------------------

@test "AC-003: BC-INDEX CapabilityDenied produces exit code 2" {
  _require_artifacts
  _load_fixture "a1-stale-index"
  # Restrict path_allow to ss-05/ only — BC-INDEX.md at parent level → CapabilityDenied
  _write_registry '".factory/specs/behavioral-contracts/ss-05/"'

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2
}

@test "AC-003: BC-INDEX NotFound produces exit code 0 (advisory)" {
  _require_artifacts
  # Use a1-no-bc-index fixture: BC file present, BC-INDEX.md absent on disk
  _load_fixture "a1-no-bc-index"
  # Full path_allow so read is attempted; file absent → NotFound
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-004: primary target CapabilityDenied blocks
# BC-5.39.010 invariant 4: primary target read failure → Block always
# ---------------------------------------------------------------------------

@test "AC-004: primary BC file CapabilityDenied produces exit code 2" {
  _require_artifacts
  _load_fixture "a1-stale-index"
  # path_allow omits behavioral-contracts/ entirely → primary BC file read → CapabilityDenied
  _write_registry '".factory/stories/"'

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2
}

# ---------------------------------------------------------------------------
# ADV-RECON-007 (BC-5.39.010 v1.20 precondition 15a / postcondition 25 /
# invariant 4 extension): a primary target that reads successfully as bytes
# but fails UTF-8 decoding MUST block — it is NOT eligible for silent
# Continue. The shipped `on_post_tool_use` (lib.rs Step 4) currently decodes
# with `std::str::from_utf8(&primary_bytes)` and, on `Err(_)`, logs a warning
# and returns `HookResult::Continue` ("fail-open, invariant 9") — invariant 9
# governs `is_char_boundary()` slicing safety on already-decoded strings only
# and does NOT authorize this fail-open disposition (BC-5.39.010 v1.20
# precondition 15a). This is the invariant-9-misattribution the amendment
# names NON-CONFORMING (EC-038).
#
# Fixture: utf8-decode-failure/factory/specs/behavioral-contracts/ss-09/
# BC-9.99.012.md — a raw byte sequence containing an invalid UTF-8 start byte
# (0xFF) that `host::read_file` reads successfully (the read itself succeeds;
# only the subsequent UTF-8 decode fails).
#
# Fixed behavior: on decode failure the shipped wasm returns
# HookResult::block_with_fix per postcondition 25 (exit 2), not
# HookResult::Continue. `_assert_exit 2` below asserts this against the
# committed release wasm.
# ---------------------------------------------------------------------------

@test "ADV-RECON-007: primary target UTF-8 decode failure produces exit code 2 (block), not Continue" {
  _require_artifacts
  _load_fixture "utf8-decode-failure"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-09/BC-9.99.012.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2

  # ADV-RECON2-002 (MEDIUM): BC-5.39.010 v1.20 postcondition 25 prescribes an exact
  # message; the exit-code-only assertion above is necessary but not sufficient
  # (F-P6-002 message-equality discipline; matches the AC-001/PC4a full-string
  # equality pattern used elsewhere in this suite — .contains()-only is
  # NON-CONFORMING). Full-string equality on the complete block_reason as surfaced
  # by dispatcher stderr (BLOCKED by / [N] / Fix / Code wrapper from
  # HookResult::block_with_fix + combine_violations_into_block; trailing period on
  # the postcondition-25 description stripped by trim_end_matches('.') in
  # block_with_fix). Fixture utf8-decode-failure/.../BC-9.99.012.md: invalid byte
  # 0xFF at byte offset 196 → std::str::from_utf8 Display = "invalid utf-8
  # sequence of 1 bytes from index 196" (confirmed via direct decode of the
  # fixture bytes with std::str::from_utf8).
  local actual_block_reason
  actual_block_reason=$(python3 -c "
import sys
data = open(sys.argv[1]).read()
for line in data.splitlines():
    key = 'block_reason=\"'
    if key in line:
        idx = line.find(key) + len(key)
        val = line[idx:]
        if val.endswith('\"'):
            val = val[:-1]
        val = val.replace('\\\\n', '\\n').replace('\\\\r', '\\r')
        print(val, end='')
        break
" "$_DISP_STDERR" 2>/dev/null || true)

  local expected_decoded
  expected_decoded=$(python3 -c "print('BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence [primary-read]: cannot decode primary target \'.factory/specs/behavioral-contracts/ss-09/BC-9.99.012.md\' as UTF-8: invalid utf-8 sequence of 1 bytes from index 196. Fail-closed per BC-5.39.010 invariant 4 (extended, v1.20) — invariant 9 governs slicing safety only and does not authorize Continue here. Fix: verify the file\'s encoding and re-save as UTF-8, then retry the write. Fix: review and fix all cross-site correspondence issues listed above, then retry the write. Code: POLICY 14/18.', end='')" 2>/dev/null || true)

  [ "$actual_block_reason" = "$expected_decoded" ] || {
    echo "FAIL: ADV-RECON2-002 postcondition-25 block message does not match BC-5.39.010 v1.20 normative verbatim text."
    echo "  This asserts full-string equality of the block_reason against the committed release wasm."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_block_reason"
    echo "  (empty actual = block_reason not found in dispatcher stderr; mismatch = wrong message or stale wasm)"
    false
  }

  # Anti-vacuity: a message containing only the old substrings but wrong format fails equality.
  local wrong_substr_msg
  wrong_substr_msg="(WRONG FORMAT) cannot decode primary target as UTF-8. invariant 4."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

# ADV-RECON-007 CONTROL: a primary target that DOES decode as valid UTF-8 must
# NOT be affected by the v1.20 fail-closed decode-failure path — it proceeds to
# normal Step 5 dispatch (Arm A1 for a BC file). Regression guard proving the
# UTF-8 fail-closed change does not over-block valid content.
#
# Fixture: utf8-valid-target/factory/specs/behavioral-contracts/ss-09/
# BC-9.99.012.md — same path/shape as the mutant fixture above, but containing
# a genuinely valid multi-byte UTF-8 character (an em dash) instead of the
# invalid 0xFF/0xFE byte. No BC-INDEX.md is present, mirroring the
# a1-no-bc-index fixture's NotFound-is-advisory-only shape (BC-5.39.010
# precondition 8) so Arm A1 does not itself produce a block.
#
# This control PASSES against the committed release wasm — it is a
# coverage-fill regression guard, not a red-gate test.
@test "ADV-RECON-007 CONTROL: primary target valid UTF-8 dispatches normally (exit code 0)" {
  _require_artifacts
  _load_fixture "utf8-valid-target"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-09/BC-9.99.012.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-005: Class A Arm2 — stale story BC-table version citation blocks
# Fixture: a2-stale-citation (story cites v1.17, BC fm v1.18)
# BC-5.39.010 PC10-15
# ---------------------------------------------------------------------------

@test "AC-005 MUTANT: stale Token Budget row produces exit code 2 with [Class A Arm2]" {
  _require_artifacts
  _load_fixture "a2-stale-citation"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class A Arm2]"
}

@test "AC-005 CONTROL: current citation produces exit code 0" {
  _require_artifacts
  _load_fixture "a2-current-citation"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-006: cascade — two stale BCs combined block
# Fixture: a2-two-stale (two stale BC citations in Token Budget)
# BC-5.39.010 postcondition 23: violations combined into one block message
# ---------------------------------------------------------------------------

@test "AC-006: two stale BC citations produce single combined block referencing both IDs" {
  _require_artifacts
  _load_fixture "a2-two-stale"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class A Arm2]"
  # The combined block must reference both stale BCs in its output
  local combined
  combined="$(cat "$_DISP_STDERR" 2>/dev/null) $output"
  [[ "$combined" == *"BC-6.26.001"* ]] || {
    echo "FAIL: combined block must cite BC-6.26.001"
    echo "  Output: $combined"
    false
  }
  [[ "$combined" == *"BC-7.27.002"* ]] || {
    echo "FAIL: combined block must cite BC-7.27.002"
    echo "  Output: $combined"
    false
  }
}

# ---------------------------------------------------------------------------
# AC-007: empty behavioral_contracts skips Arm A2
# Fixture: a2-empty-bcs (behavioral_contracts: [])
# BC-5.39.010 PC14
# ---------------------------------------------------------------------------

@test "AC-007: empty behavioral_contracts produces exit code 0" {
  _require_artifacts
  _load_fixture "a2-empty-bcs"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-008: no version-citing table row skips that BC
# Fixture: a2-prose-only-citation (BC-6.26.001 in prose, no Token Budget row)
# BC-5.39.010 PC13
# ---------------------------------------------------------------------------

@test "AC-008: BC ID in prose only (no version table row) produces exit code 0" {
  _require_artifacts
  _load_fixture "a2-prose-only-citation"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-009: Class B Arm1 — B2≠B3 STORY-INDEX internal inconsistency blocks (traces to BC-5.39.010 postcondition 13b)
# Fixture: b1-hash-mismatch (B1=47a65c9, B2=4be9d21, B3=c3f9811 — catalog≠blockquote)
# BC-5.39.010 §PC13b: B2 ≠ B3 → STORY-INDEX internally inconsistent → Block [Class B]
# Message must cite POLICY 18 (D-923) and enumerate all three provenance categories
# (stale/fabricated/algorithm-divergent) WITHOUT asserting which applies.
# (AC-023 / T-P6C covers the opposite direction: B2==B3, B1≠B2 → advisory.)
# ---------------------------------------------------------------------------

@test "AC-009 MUTANT: B2≠B3 STORY-INDEX internal inconsistency blocks with provenance categories" {
  _require_artifacts
  _load_fixture "b1-hash-mismatch"
  _write_registry

  local envelope
  # B1=47a65c9 (frontmatter); B2=4be9d21 (catalog); B3=c3f9811 (blockquote).
  # B2 ≠ B3 → STORY-INDEX internally inconsistent → PC13b → BLOCK.
  # RED GATE: v1.10 impl blocks on B1≠B2 with wrong message (no provenance categories).
  # After v1.11 implementation: PC13b block with normative provenance text → PASSES.
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class B]"

  # AC-009 normative assertions: full-string equality on the complete PC13b block message.
  # BC-5.39.010 §PC4a: test-writer MUST assert COMPLETE formatted string by equality
  # check; .contains()-only (including loop-over-categories) is NON-CONFORMING. The exit-code
  # check above confirms detection; these assertions confirm the normative text (including all
  # three provenance categories per invariant 11, and the POLICY 18 (D-923) anchor) verbatim.
  # Fixture b1-hash-mismatch: B1=47a65c9 (frontmatter); B2=4be9d21 (catalog); B3=c3f9811 (blockquote).
  # Story ID S-21.07 extracted from basename S-21.07-test via PC17 ^(S-[0-9]+\.[0-9]+) regex.
  local actual_block_reason
  actual_block_reason=$(python3 -c "
import sys
data = open(sys.argv[1]).read()
for line in data.splitlines():
    key = 'block_reason=\"'
    if key in line:
        idx = line.find(key) + len(key)
        val = line[idx:]
        if val.endswith('\"'):
            val = val[:-1]
        val = val.replace('\\\\n', '\\n').replace('\\\\r', '\\r')
        print(val, end='')
        break
" "$_DISP_STDERR" 2>/dev/null || true)

  local expected_decoded
  # Full block_reason as surfaced by dispatcher stderr (includes BLOCKED by / [N] / Fix / Code
  # wrapper from HookResult::block_with_fix + combine_violations_into_block; trailing period
  # on description stripped by trim_end_matches('.') in block_with_fix implementation).
  expected_decoded=$(python3 -c "print('BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence [Class B]: Story S-21.07 input-hash three-way mismatch: frontmatter=47a65c9 STORY-INDEX-catalog=4be9d21 STORY-INDEX-blockquote=c3f9811. STORY-INDEX catalog and blockquote disagree — this is anomalous and has no burst-ordering explanation. Update per POLICY 18 (D-923). This hook detects inconsistency only — operator MUST determine which of the following applies before remediating: (a) STALE: previously valid hash; inputs changed after authoring; remedy: rerun \`compute-input-hash --update\` on the story. (b) FABRICATED: hash was never output of \`compute-input-hash --update\` at any revision (POLICY 18 violation); remedy: acknowledge PROVENANCE-BREAK in burst-log before recomputing. (c) ALGORITHM-DIVERGENT: hash produced by prior binary version per ADR-036 §Decision 4; NOT fabricated; remedy: recompute with current authoritative binary, no PROVENANCE-BREAK annotation required. Fix: review and fix all cross-site correspondence issues listed above, then retry the write. Code: POLICY 14/18.', end='')" 2>/dev/null || true)

  [ "$actual_block_reason" = "$expected_decoded" ] || {
    echo "FAIL: AC-009 block message does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_block_reason"
    echo "  (empty actual = block_reason not found in dispatcher stderr; mismatch = wrong message)"
    false
  }

  # Anti-vacuity: a message containing only the old substrings but wrong format fails equality.
  local wrong_substr_msg
  wrong_substr_msg="(WRONG FORMAT) POLICY 18 (D-923) STALE FABRICATED ALGORITHM-DIVERGENT."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

@test "AC-009 CONTROL: three-way hash match produces exit code 0" {
  _require_artifacts
  _load_fixture "b1-hash-match"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# T-037: Class B — B1=B2 agree but B3 mismatch blocks (B3-only mismatch)
# Fixture: b1-b3-only-mismatch (B1=47a65c9, B2=47a65c9, B3=deadbee)
# F-S2107-P1B-003: parse_story_index_blockquote_hash uses starts_with("> S-21.07=")
#   → production prose blockquote shape never matches → B3=None → inert (no block)
# BC-5.39.010 PC13b: B3≠B1 must block (blockquote hash mismatch).
# ---------------------------------------------------------------------------

@test "T-037 MUTANT: B1=B2 agree but B3 mismatch in production blockquote blocks" {
  _require_artifacts
  _load_fixture "b1-b3-only-mismatch"
  _write_registry

  local envelope
  # story frontmatter B1=47a65c9, STORY-INDEX catalog B2=47a65c9, blockquote B3=deadbee
  # After fix: B3 extracted → B3≠B1 → exit 2 [Class B]
  # F-S2107-P1B-003 RED GATE: blockquote parser inert on production shape → B3=None
  # → no three-way comparison → exit 0 → test expects 2 → FAILS
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class B]"
}

# ---------------------------------------------------------------------------
# T-038: Class B — cross-story catalog row correct lookup
# Fixture: b1-cross-story-catalog (S-18.00 row mentions S-18.01 in blocks column)
# F-S2107-P1B-008: parse_story_index_catalog_hash naive contains(story_id) matches
#   S-18.00 row first (because it mentions "S-18.01" in blocks/depends column)
# BC-5.39.010 §PC16: catalog lookup must return hash from CANONICAL story row only.
# ---------------------------------------------------------------------------

@test "T-038 CONTROL: cross-story catalog row lookup returns own-story hash (exit code 0)" {
  _require_artifacts
  _load_fixture "b1-cross-story-catalog"
  _write_registry

  local envelope
  # S-18.01-test.md B1=1b4ea21; STORY-INDEX: S-18.00 row mentions S-18.01 (hash e5bc551)
  # before S-18.01 row (hash 1b4ea21). After fix: S-18.01 row matched → B2=1b4ea21=B1 → exit 0
  # F-S2107-P1B-008 RED GATE: naive contains("S-18.01") hits S-18.00 row → "e5bc551"≠"1b4ea21" → exit 2
  envelope="$(_post_write_event '.factory/stories/S-18.01-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-010: absent secondary sites are advisory-only
# Fixtures: b1-no-story-index (input-hash present, STORY-INDEX absent)
#           b1-no-input-hash (no input-hash field)
# BC-5.39.010 PC18 (no input-hash → skip), PC20 (NotFound → advisory)
# ---------------------------------------------------------------------------

@test "AC-010: absent STORY-INDEX sites produce exit code 0 (advisory)" {
  _require_artifacts
  _load_fixture "b1-no-story-index"
  _write_registry

  local envelope
  # Story has input-hash but STORY-INDEX.md is absent → NotFound → advisory, exit 0
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

@test "AC-010: no input-hash field skips Arm B1 entirely (exit code 0)" {
  _require_artifacts
  _load_fixture "b1-no-input-hash"
  _write_registry

  local envelope
  # Story has no input-hash field → B1 skips entirely
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-011: Class B Arm2 — STORY-INDEX internal catalog vs blockquote mismatch
# Fixtures: b2-catalog-mismatch (catalog 47a65c9, blockquote 4be9d21)
#           b2-catalog-agree (both 47a65c9)
# BC-5.39.010 PC22-27
# ---------------------------------------------------------------------------

@test "AC-011 MUTANT: catalog/blockquote mismatch produces exit code 2 with [Class B]" {
  _require_artifacts
  _load_fixture "b2-catalog-mismatch"
  _write_registry

  local envelope
  # STORY-INDEX.md is the primary trigger for Arm B2
  envelope="$(_post_write_event '.factory/stories/STORY-INDEX.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class B]"
}

@test "AC-011 CONTROL: catalog/blockquote agree produces exit code 0" {
  _require_artifacts
  _load_fixture "b2-catalog-agree"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/stories/STORY-INDEX.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-012/AC-013/AC-014 (Class D — excluded-namespace / non-F- advisory /
# historical-positional-exclusion) integration tests REMOVED.
#
# Class D (arm_d.rs) was removed from the implementation per v1.14 spec +
# S-21.07 reconciliation (commit e6ce015f): the story's File Structure section
# and Task 12 explicitly mark Class D "DEFERRED v1.6 — do NOT create," and the
# module's dispatch hook (is_cycle_artifact) always returned None, i.e. the
# arm was already runtime-inert. The five tests that previously lived here
# (AC-012 x2, AC-013 MUTANT/CONTROL, AC-014) exercised the deleted arm_d
# module end-to-end and had been left `skip`-marked as "preserved for future
# re-activation." That skip rationale cited POLICY 1 (append-only ID
# numbering for BC/VP/DI/CAP/STORY/HS artifacts, scope: .factory/policies.yaml
# id 1) — POLICY 1 does not apply to bats test function bodies, so the
# citation did not hold. Per the production-grade default (CLAUDE.md), an
# orphaned integration test for code that no longer exists is deleted, not
# preserved in a permanently-skipped state; Class D re-implementation in
# S-21.08 will need fresh tests written against whatever the reinstated arm's
# actual contract is, not a resurrection of these stale bodies. One of the
# removed tests (AC-013 MUTANT) asserted a Class-D advisory is emitted while
# runtime dispatch was already inert -- exactly the kind of test that must not
# ship. The four Class-D-only fixtures (d-clean-tokens, d-non-f-token,
# d-all-f-token, d-historical-excluded) were deleted alongside these tests as
# they had no other referents in the repo.
# ---------------------------------------------------------------------------
# AC-015: Class E1 — version vs last_amended mismatch blocks
# Fixtures: e1-version-mismatch / e1-version-match / e1-unparseable
# BC-5.39.010 PC35-37 + postconditions 19-20
# ---------------------------------------------------------------------------

@test "AC-015 MUTANT: version 1.33 vs last_amended (v1.31) produces exit code 2 with [Class E1]" {
  _require_artifacts
  _load_fixture "e1-version-mismatch"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class E1]"
}

@test "AC-015 CONTROL: version 1.6 matches last_amended (v1.6) produces exit code 0" {
  _require_artifacts
  _load_fixture "e1-version-match"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

@test "AC-015: unparseable last_amended produces exit code 0 (advisory, not block)" {
  _require_artifacts
  # e1-unparseable: last_amended has no YYYY-MM-DD (vX.X) pattern
  _load_fixture "e1-unparseable"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # BC-5.39.010 PC37 last sentence: unparseable format → advisory, not block
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-015 over-broad exclusion prevention (F-S2104-P29-H02 lesson):
# When [Prior:] chain is present in last_amended but the OUTERMOST version is
# wrong, E1 must still fire. The [Prior:] annotation must NOT suppress the check.
# Fixture: e1-prior-chain-wrong-outermost (version "1.6", last_amended "(v1.5) [Prior: (v1.4)]")
# BC-5.39.010 EC-018: extract_last_amended_outer_version reads from char 0 → (v1.5) ≠ 1.6 → block.
# ---------------------------------------------------------------------------

@test "AC-015: [Prior:] chain present but outermost version wrong still produces exit code 2 with [Class E1]" {
  _require_artifacts
  _load_fixture "e1-prior-chain-wrong-outermost"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class E1]"
}

# ---------------------------------------------------------------------------
# AC-016: Class E2 — non-ascending modified[] blocks
# Fixtures: e2-non-monotonic / e2-ascending
# BC-5.39.010 PC38-39 + postcondition 22
# ---------------------------------------------------------------------------

@test "AC-016 MUTANT: non-ascending modified[] produces exit code 2 with [Class E2]" {
  _require_artifacts
  _load_fixture "e2-non-monotonic"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class E2]"
}

@test "AC-016 CONTROL: ascending modified[] produces exit code 0" {
  _require_artifacts
  _load_fixture "e2-ascending"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-017: Class E1 — Prior-chain version positionally excluded
# Fixture: e1-prior-chain-correct (version "1.6", last_amended "(v1.6) ... [Prior: (v1.5) ...]")
# BC-5.39.010 EC-018: Prior-chain exclusion by positional anchor
# ---------------------------------------------------------------------------

@test "AC-017: Prior-chain (v1.5) does not cause E1 mismatch when version is 1.6" {
  _require_artifacts
  _load_fixture "e1-prior-chain-correct"
  _write_registry

  local envelope
  # [Prior: (v1.5)] in last_amended — positional anchor ensures (v1.5) is NOT at position 0
  # extract_last_amended_outer_version reads from char 0 → returns "1.6" → matches version → pass
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# AC-018: combined violations from multiple arms → single block
# Fixture: combined-a1-e1 (BC v1.5, INDEX v1.6 — index-newer PC2b; last_amended "(v1.33)")
# BC-5.39.010: A1 fires via PC2b (index-newer → block); E1 fires via version≠last_amended.
# BC-5.39.010 postcondition 23: all violations combined into ONE block_with_fix.
# v1.11 rationale: prior v1.10 fixture (primary-newer) now produces only PC2a advisory from A1,
# so combined-block path requires index-newer direction to make both arms block independently.
# ---------------------------------------------------------------------------

@test "AC-018: A1 index-newer block + E1 mismatch produces single combined block message" {
  _require_artifacts
  _load_fixture "combined-a1-e1"
  _write_registry

  local envelope
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class A Arm1]"
  # Combined block must also cite [Class E1]
  local combined
  combined="$(cat "$_DISP_STDERR" 2>/dev/null) $output"
  [[ "$combined" == *"[Class E1]"* ]] || {
    echo "FAIL: combined block must also contain [Class E1]"
    echo "  Output: $combined"
    false
  }
}

# ---------------------------------------------------------------------------
# T-035: Class A Arm1 — BC-INDEX.md must NOT be classified as a BC file
# Fixture: a1-current-index (reuse — BC-INDEX.md exists in this fixture)
# F-S2107-P1B-005: dispatch::is_bc_file uses starts_with("BC-") && ends_with(".md")
#   → "BC-INDEX.md" matches → A1 runs with bc_id="BC-INDEX" → spurious violations.
# BC-5.39.010 §Classification invariant: index files excluded from BC classification.
# ---------------------------------------------------------------------------

@test "T-035 CONTROL: BC-INDEX.md write event produces exit code 0 (not classified as BC file)" {
  _require_artifacts
  _load_fixture "a1-current-index"
  _write_registry

  local envelope
  # Writing BC-INDEX.md must NOT trigger Arm A1 (not a behavioral contract file).
  # After fix: dispatch skips BC-INDEX.md → Continue → exit 0
  # F-S2107-P1B-005 RED GATE: is_bc_file returns true for BC-INDEX.md → A1 runs →
  #   bc_id="BC-INDEX", bc_version=index-version → no row for "BC-INDEX" or wrong version
  #   → exit 2 → FAILS
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/BC-INDEX.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# T-048: Class B Arm1 — cross-story catalog row correct lookup
# Fixture: b1-cross-story-catalog (S-18.00 row mentions S-18.01 in blocks column)
# F-S2107-P1B-008: parse_story_index_catalog_hash naive contains("S-18.01") matches
#   S-18.00 row first (mentions "S-18.01" in blocks/depends column; hash e5bc551)
#   → "e5bc551" ≠ S-18.01 story frontmatter "1b4ea21" → spurious exit 2.
# BC-5.39.010 PC16: catalog lookup must match CANONICAL story row (first cell).
# ---------------------------------------------------------------------------

@test "T-048 CONTROL: cross-story catalog lookup returns own-story hash (exit code 0)" {
  # F-S2107-P3-018: duplicate T-038 ID de-duplicated → T-048. Original T-038 (line 668)
  # tests the same scenario from fixture b1-cross-story-catalog but with different
  # envelope path. Both must exist; IDs must be unique per BC-5.39.010 bats discipline.
  _require_artifacts
  _load_fixture "b1-cross-story-catalog"
  _write_registry

  local envelope
  # S-18.01-test.md B1=1b4ea21; S-18.00 row mentions "S-18.01" in blocks column (hash e5bc551)
  # S-18.01 catalog row hash = 1b4ea21. After fix: S-18.01 row matched → B2=1b4ea21=B1 → exit 0
  # F-S2107-P1B-008 RED GATE: naive contains("S-18.01") hits S-18.00 first → "e5bc551"≠"1b4ea21" → exit 2
  envelope="$(_post_write_event '.factory/stories/S-18.01-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0
}

# ---------------------------------------------------------------------------
# T-045: Class E1 — 15-byte last_amended format accepted (no spurious advisory)
# Fixture: e1-15-byte-last-amended (VP-9999; version "2"; last_amended "2026-07-30 (v2)")
# Isolation strategy: VP file write is used (NOT a BC file write) because VP files do
# NOT trigger Arm A1 (A1 is BC-file-only). Arm A1 can't parse single-integer versions
# like "v2" in BC-INDEX (extract_version_token requires vN.N format). Using a VP file
# isolates E1 so the test is genuinely discriminating.
# F-S2107-P1C-014: extract_last_amended_outer_version `if len < 17 { return None }`
#   → 15-byte "2026-07-30 (v2)" → None → advisory "unparseable format" fires.
# BC-5.39.010 §E1: YYYY-MM-DD (vN) with single-digit outer version is valid.
# Fixture renamed: VP-039.md → VP-9999-test.md → VP-9999.md (PC34: is_canonical_vp_filename
# requires all-digit inner part; VP-9999-test.md had non-digit in inner, so E1 never ran).
# ---------------------------------------------------------------------------

@test "T-045 CONTROL: 15-byte last_amended 2026-07-30 (v2) produces no advisory (exit 0)" {
  _require_artifacts
  _load_fixture "e1-15-byte-last-amended"
  _write_registry

  local envelope
  # VP-9999; version "2"; last_amended "2026-07-30 (v2)" = 15 bytes.
  # VP files skip Arm A1 entirely — only Class E runs. This isolates the 15-byte E1 check.
  # After fix: len threshold lowered → Some("2") extracted → "2"=="2" → no E1 advisory → exit 0
  # F-S2107-P1C-014 RED GATE: len < 17 → None → advisory fires → log has warn record → FAILS
  # Fixture uses VP-9999.md (non-live all-digit ID) to satisfy is_canonical_vp_filename.
  envelope="$(_post_write_event '.factory/specs/verification-properties/VP-9999.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 0

  # No Class E1 advisory must appear in the dispatcher log.
  # The 15-byte format is valid; no "unparseable" advisory expected.
  local log; log="$(_plugin_log)"
  if grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
       | grep '"type":"plugin.log"' | grep '"level":"warn"' | grep -qi 'last.amended\|unparseable\|unrecognized'; then
    echo "FAIL: unexpected E1 advisory about last_amended format for valid 15-byte string"
    echo "  '2026-07-30 (v2)' is a valid format per BC-5.39.010 §E1 (F-S2107-P1C-014)"
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" \
      | grep '"type":"plugin.log"' | grep '"level":"warn"' | head -3
    false
  fi
}

# ---------------------------------------------------------------------------
# T-046: Class E1 positive-coverage mutant — VP file version/last_amended mismatch
# Fixture: e1-vp-version-mismatch (VP-9999; version "1.7"; last_amended "2026-07-30 (v1.6)")
# F-P2-013: T-045 only asserts exit 0 + absence of warn advisory. A mutation where
# Class E never ran (or E1 was disabled) would still pass T-045. T-046 adds a MUTANT
# that asserts exit 2 when there IS a version/last_amended mismatch.
# Isolation: VP file → no Arm A1 trigger → pure Class E1 isolation.
# RED GATE: before fix (Class E1 not yet implemented): plugin.crashed → T-046 FAILS.
# Post-fix: E1 detects "1.7" ≠ "1.6" → violation → exit 2 → PASSES.
# ---------------------------------------------------------------------------

@test "T-046 MUTANT (F-P2-013): VP file version/last_amended mismatch produces exit 2" {
  _require_artifacts
  _load_fixture "e1-vp-version-mismatch"
  _write_registry

  local envelope
  # VP-9999: version "1.7", last_amended "2026-07-30 (v1.6)".
  # extract_last_amended_outer_version("2026-07-30 (v1.6)") → "1.6".
  # "1.7" ≠ "1.6" → Class E1 violation → exit 2.
  # VP file → no Arm A1 (A1 is BC-file-only). Class E1 runs in isolation.
  envelope="$(_post_write_event '.factory/specs/verification-properties/VP-9999.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class E1]"
}

# ---------------------------------------------------------------------------
# T-047: PC40 — story with volatile input-hash inputs emits advisory, exits 0
# Fixture: b1-volatile-input (S-21.07-test: input-hash + inputs: [.factory/STATE.md];
#   STORY-INDEX.md with MISMATCHED hash to prove PC40 suppresses the comparison)
# BC-5.39.010 §PC40: volatile inputs → advisory + Continue, NOT three-way comparison.
# RED GATE: without PC40 fix, arm B1 runs three-way comparison → finds mismatch → exit 2.
# Post-fix: volatile inputs detected → advisory → exit 0.
# ---------------------------------------------------------------------------

@test "T-047 (PC40): story with volatile input path emits advisory and exits 0 not 2" {
  _require_artifacts
  _load_fixture "b1-volatile-input"
  _write_registry

  local envelope
  # S-21.07-test.md: input-hash abc123; inputs: [".factory/STATE.md"] (volatile).
  # STORY-INDEX.md: catalog=def4567, blockquote=deadb00 (B2≠B3 → PC13b without PC40).
  # F-P6-003: fixture was "def456" (6-char); {7,40} extractor returned None → test
  # passed vacuously via (None,None) arm. Now "def4567"/"deadb00" (7-char valid hex) →
  # extractors succeed → without PC40: PC13b block (exit 2); with PC40: advisory exit 0.
  # Discrimination is genuine: PC40 is what prevents the block, not None-arms.
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # PC40: volatile input path → advisory + Continue, NOT block.
  # Without fix: three-way comparison runs → B1≠B2 → exit 2. With fix: exit 0.
  _assert_exit 0

  # Advisory mentioning volatile must be present in the dispatcher log.
  local log; log="$(_plugin_log)"
  grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
    | grep '"type":"plugin.log"' \
    | grep '"level":"warn"' \
    | grep -qi 'volatile' || {
      echo "FAIL: expected advisory mentioning 'volatile' not found in dispatcher log"
      echo "  PC40: story with volatile inputs must emit advisory + Continue (BC-5.39.010 §PC40)"
      grep '"plugin_name":"validate-cross-site-correspondence"' "$log" \
        | grep '"type":"plugin.log"' | head -5 || echo "  (no plugin.log records)"
      false
    }
}

# ---------------------------------------------------------------------------
# F-P4-003: RowMalformed — locator-matched <5-field BC-INDEX line emits advisory
# Fixture: a1-row-malformed (BC-INDEX has a 2-field notes-table row carrying the BC link)
# BC-5.39.010 §PC5 postcondition 4a: RowMalformed → advisory + Continue (exit 0).
# Advisory message (NORMATIVE — BC-5.39.010 §postcondition 4a verbatim, bc_id=BC-5.39.010,
# field_count=2):
#   "validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md contains a malformed
#    candidate line for BC-5.39.010 (2 fields found; expected ≥5 for a valid body-table row).
#    This line is structurally not a BC-INDEX body-table row (likely a Changelog entry or
#    notes table). Registration status cannot be determined from this line. Verify BC-INDEX
#    body-table registration manually."
# F-P6-002 RED GATE: prior test used two .contains() substrings — insufficient to gate the
# COMPLETE verbatim message (BC-5.39.010 §postcondition 4a MUST-verbatim clause).
# An equality check on the COMPLETE string is required. The current impl uses a DIFFERENT
# message format (different prefix, different sentence structure) → full-string equality
# FAILS → RED GATE.
# After v1.11 implementation: advisory uses the exact normative text → PASSES.
# ---------------------------------------------------------------------------

@test "F-P4-003 (RowMalformed): 2-field BC-INDEX candidate line emits verbatim advisory, exits 0" {
  _require_artifacts
  _load_fixture "a1-row-malformed"
  _write_registry

  local envelope
  # Trigger Arm A1 by writing the BC file — hook reads BC-INDEX.md (from fixture),
  # finds the 2-field locator-matched line → RowMalformed(2) → advisory + Continue.
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # RowMalformed MUST NOT block — advisory-only path (postcondition 4a).
  _assert_exit 0

  local log; log="$(_plugin_log)"

  # F-P6-002: full-string equality on the COMPLETE v1.12 normative RowMalformed message.
  # BC-5.39.010 §PC5 postcondition 4a (NORMATIVE — MUST reproduce verbatim):
  #   bc_id=BC-5.39.010, field_count=2.
  # This check gates:
  #   (a) correct prefix: "[Class A Arm1]:" not "[Class A Arm1] advisory:"
  #   (b) preamble: "BC-INDEX.md contains a malformed candidate line for BC-5.39.010"
  #   (c) field count: "(2 fields found; ...)" not "(2 non-empty fields found; ...)"
  #   (d) sentence 2: "This line is structurally not a BC-INDEX body-table row..."
  #   (e) both operator-actionable clauses present in the prescribed order
  #   (f) no trailing v1.10 annotation or "Not blocking — ..." tail
  # Two independent .contains() checks cannot detect (a), (d), or (f): the prior test
  # passed against a non-conforming message that had both substrings but wrong surrounding
  # text. BC-5.39.010 §postcondition 4a prohibits .contains()-only assertions.
  local expected_msg
  expected_msg='validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md contains a malformed candidate line for BC-5.39.010 (2 fields found; expected ≥5 for a valid body-table row). This line is structurally not a BC-INDEX body-table row (likely a Changelog entry or notes table). Registration status cannot be determined from this line. Verify BC-INDEX body-table registration manually.'

  local actual_msg
  # Extract the message field from the JSONL log record using python3 JSON decode.
  # This handles Unicode escapes (≥ for ≥) and JSON-escaped characters correctly.
  actual_msg=$(python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except Exception:
        continue
    if (obj.get('plugin_name') == 'validate-cross-site-correspondence' and
            obj.get('type') == 'plugin.log' and
            obj.get('level') == 'warn'):
        print(obj.get('message', ''), end='')
        sys.exit(0)
" < "$log" 2>/dev/null || true)

  # Construct expected with Python to decode ≥ → ≥ (Unicode).
  local expected_decoded
  expected_decoded=$(python3 -c "print('validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md contains a malformed candidate line for BC-5.39.010 (2 fields found; expected ≥5 for a valid body-table row). This line is structurally not a BC-INDEX body-table row (likely a Changelog entry or notes table). Registration status cannot be determined from this line. Verify BC-INDEX body-table registration manually.', end='')" 2>/dev/null || true)

  [ "$actual_msg" = "$expected_decoded" ] || {
    echo "FAIL: RowMalformed advisory does not match v1.12 normative verbatim text."
    echo "  F-P6-002 RED GATE: BC-5.39.010 §postcondition 4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_msg"
    echo "  (empty actual_msg = no warn advisory in log; wrong message format = mismatch)"
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
      | grep '"type":"plugin.log"' | grep '"level":"warn"' | head -3 \
      || echo "  (no warn plugin.log records)"
    false
  }
}

# ---------------------------------------------------------------------------
# AC-022 / T-P6A (F-P6-001): PC2a — primary newer than index → advisory + exit 0
# Fixture: a1-index-behind-primary (BC-5.39.010 frontmatter v1.6; INDEX row v1.5)
# BC-5.39.010 §PC2a directional carve-out: when primary is NEWER than index,
# POLICY 3 (state_manager_runs_last) guarantees this is the correct burst-ordering
# intermediate state (BC written before INDEX update). Advisory + Continue, NOT block.
#
# Advisory MUST match (verbatim, <id>/<index_version>/<fm_version> substituted):
# "validate-cross-site-correspondence [Class A Arm1] advisory: BC-INDEX.md body-table row
# for <id> cites v<index_version> but frontmatter version: is \"<fm_version>\" — primary
# newer than index; state-manager index update pending; Class A BLOCK suspended."
#
# RED GATE (v1.10 implementation): any version mismatch → block (exit 2).
# Test expects exit 0 → FAILS under current implementation.
# After v1.11 implementation: PC2a carve-out → advisory + exit 0 → PASSES.
# ---------------------------------------------------------------------------

@test "AC-022 / T-P6A (PC2a): primary-newer-than-index emits advisory, exits 0" {
  _require_artifacts
  _load_fixture "a1-index-behind-primary"
  _write_registry

  local envelope
  # BC-5.39.010 frontmatter version="1.6"; BC-INDEX row cites v1.5.
  # primary (1.6) > index (1.5) → PC2a → advisory + Continue.
  # RED GATE: v1.10 impl blocks on any mismatch (exit 2). Test expects exit 0 → FAILS.
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # PC2a: primary newer → advisory + Continue, NOT block.
  _assert_exit 0

  # AC-022: full-string equality on the complete PC2a normative advisory text.
  # BC-5.39.010 §PC4a forbids .contains()-only assertions on normative messages.
  # Fixture: BC-5.39.010 frontmatter v1.6; BC-INDEX row cites v1.5.
  local log; log="$(_plugin_log)"

  # Extract the message field from the JSONL log record using python3 JSON decode.
  # Handles Unicode (em dash: —) and JSON-escaped characters correctly.
  local actual_msg
  actual_msg=$(python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except Exception:
        continue
    if (obj.get('plugin_name') == 'validate-cross-site-correspondence' and
            obj.get('type') == 'plugin.log' and
            obj.get('level') == 'warn'):
        print(obj.get('message', ''), end='')
        sys.exit(0)
" < "$log" 2>/dev/null || true)

  # Decode expected text through Python to handle Unicode (em dash —) correctly.
  local expected_decoded
  expected_decoded=$(python3 -c "print('validate-cross-site-correspondence [Class A Arm1] advisory: BC-INDEX.md body-table row for BC-5.39.010 cites v1.5 but frontmatter version: is \"1.6\" — primary newer than index; state-manager index update pending; Class A BLOCK suspended.', end='')" 2>/dev/null || true)

  [ "$actual_msg" = "$expected_decoded" ] || {
    echo "FAIL: PC2a advisory does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_msg"
    echo "  (empty actual_msg = no warn advisory in log; mismatch = wrong message format)"
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
      | grep '"type":"plugin.log"' | grep '"level":"warn"' | head -3 \
      || echo "  (no warn plugin.log records)"
    false
  }

  # Anti-vacuity: a message that passes all three old grep-q substring checks but differs
  # from the normative text must be rejected by full-string equality. Proves PC4a has teeth.
  local wrong_substr_msg
  wrong_substr_msg="validate-cross-site-correspondence [Class A Arm1] advisory: (WRONG FORMAT) primary newer than index; Class A BLOCK suspended."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

# ---------------------------------------------------------------------------
# T-P6B (F-P6-001): PC2b — index newer than primary → block with prescribed message
# Fixture: a1-index-ahead-of-primary (BC-5.39.010 frontmatter v1.10; INDEX row cites v1.11)
# BC-5.39.010 §PC2b: index NEWER than primary is anomalous (no POLICY 3 explanation).
# Block message (normative): "...index is newer than primary. This is anomalous..."
#
# RED GATE: current impl exits 2 with a DIFFERENT message format that lacks
# "index is newer than primary" and "This is anomalous" — test asserts the v1.11
# prescribed substring → FAILS under current implementation.
# After v1.11 implementation: block with normative text → PASSES.
# ---------------------------------------------------------------------------

@test "T-P6B (PC2b): BC-5.39.010 index-newer-than-primary blocks with prescribed text" {
  _require_artifacts
  _load_fixture "a1-index-ahead-of-primary"
  _write_registry

  local envelope
  # BC-5.39.010 frontmatter version="1.10"; BC-INDEX row last cites v1.11.
  # index (1.11) > primary (1.10) → PC2b → BLOCK.
  # Block message must contain "index is newer than primary" and "This is anomalous".
  # RED GATE: current block message is "...cites version v1.11 but BC frontmatter says
  # version 1.10. Update the BC-INDEX.md row to v1.10. POLICY 14 leg 5." — no "anomalous".
  envelope="$(_post_write_event '.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # PC2b: index newer → anomalous → BLOCK (exit 2).
  _assert_exit 2 "[Class A Arm1]"

  # T-P6B: full-string equality on the complete PC2b normative block text.
  # BC-5.39.010 §PC4a forbids .contains()-only assertions on normative messages.
  # Fixture: BC-5.39.010 frontmatter v1.10; BC-INDEX row last cites v1.11 (index newer → PC2b).
  local actual_block_reason
  actual_block_reason=$(python3 -c "
import sys
data = open(sys.argv[1]).read()
for line in data.splitlines():
    key = 'block_reason=\"'
    if key in line:
        idx = line.find(key) + len(key)
        val = line[idx:]
        if val.endswith('\"'):
            val = val[:-1]
        val = val.replace('\\\\n', '\\n').replace('\\\\r', '\\r')
        print(val, end='')
        break
" "$_DISP_STDERR" 2>/dev/null || true)

  local expected_decoded
  # Full block_reason as surfaced by dispatcher stderr (includes BLOCKED by / [N] / Fix / Code
  # wrapper from HookResult::block_with_fix + combine_violations_into_block; trailing period
  # on description stripped by trim_end_matches('.') in block_with_fix implementation).
  expected_decoded=$(python3 -c "print('BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md body-table row for BC-5.39.010 cites v1.11 but frontmatter version: is \"1.10\" — index is newer than primary. This is anomalous: the index cannot legitimately advance ahead of the BC it cites. Verify no index row was updated out-of-burst or under the wrong BC path. Update per POLICY 14 leg 5. Fix: review and fix all cross-site correspondence issues listed above, then retry the write. Code: POLICY 14/18.', end='')" 2>/dev/null || true)

  [ "$actual_block_reason" = "$expected_decoded" ] || {
    echo "FAIL: PC2b block message does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_block_reason"
    echo "  (empty actual = block_reason not found in dispatcher stderr; mismatch = wrong message)"
    false
  }

  # Anti-vacuity: a message containing both old substring targets but wrong format fails equality.
  local wrong_substr_msg
  wrong_substr_msg="BLOCKED by validate-cross-site-correspondence: [1] (WRONG FORMAT) index is newer than primary. This is anomalous."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

# ---------------------------------------------------------------------------
# AC-023 / T-P6C (F-P6-001): PC13a — B2==B3, B1≠B2 → advisory + exit 0
# Fixture: b1-story-index-consistent-stale (B1=47a65c9, B2=4be9d21, B3=4be9d21)
# BC-5.39.010 §PC13a: B2==B3 (STORY-INDEX internally consistent) AND B1≠B2
# (story just rewritten, STORY-INDEX not yet updated by state-manager per POLICY 3).
# Advisory + Continue, NOT block.
#
# Advisory MUST match (verbatim, <id>/<h1>/<h2>/<h3> substituted):
# "validate-cross-site-correspondence [Class B] advisory: Story <id> input-hash mismatch —
# frontmatter=<h1>; STORY-INDEX-catalog=<h2>; STORY-INDEX-blockquote=<h3>. STORY-INDEX
# sites agree with each other; story frontmatter differs. State-manager STORY-INDEX update
# pending; Class B BLOCK suspended."
#
# RED GATE (v1.10 implementation): B2 and B3 present, any mismatch with B1 → block (exit 2).
# Test expects exit 0 → FAILS under current implementation.
# After v1.11 implementation: PC13a carve-out → advisory + exit 0 → PASSES.
# ---------------------------------------------------------------------------

@test "AC-023 / T-P6C (PC13a): B2==B3 story-index-consistent-stale emits advisory, exits 0" {
  _require_artifacts
  _load_fixture "b1-story-index-consistent-stale"
  _write_registry

  local envelope
  # S-21.07-test: B1=47a65c9; STORY-INDEX: B2=4be9d21, B3=4be9d21 (B2==B3, B1 differs).
  # PC13a: B2==B3 → STORY-INDEX internally consistent; B1 differs (story just written).
  # RED GATE: v1.10 impl: B2 and B3 present, !b2_match || !b3_match → block (exit 2).
  # Test expects exit 0 → FAILS.
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # PC13a: B2==B3, B1 differs → advisory + Continue, NOT block.
  _assert_exit 0

  # AC-023: full-string equality on the complete PC13a normative advisory text.
  # BC-5.39.010 §PC4a forbids .contains()-only assertions on normative messages.
  # Fixture: S-21.07-test B1=47a65c9; STORY-INDEX B2=4be9d21, B3=4be9d21 (B2==B3).
  local log; log="$(_plugin_log)"

  # Extract the message field from the JSONL log record using python3 JSON decode.
  # Handles Unicode (em dash: —) and JSON-escaped characters correctly.
  local actual_msg
  actual_msg=$(python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except Exception:
        continue
    if (obj.get('plugin_name') == 'validate-cross-site-correspondence' and
            obj.get('type') == 'plugin.log' and
            obj.get('level') == 'warn'):
        print(obj.get('message', ''), end='')
        sys.exit(0)
" < "$log" 2>/dev/null || true)

  # Decode expected text through Python to handle Unicode (em dash —) correctly.
  local expected_decoded
  expected_decoded=$(python3 -c "print('validate-cross-site-correspondence [Class B] advisory: Story S-21.07 input-hash mismatch — frontmatter=47a65c9; STORY-INDEX-catalog=4be9d21; STORY-INDEX-blockquote=4be9d21. STORY-INDEX sites agree with each other; story frontmatter differs. State-manager STORY-INDEX update pending; Class B BLOCK suspended.', end='')" 2>/dev/null || true)

  [ "$actual_msg" = "$expected_decoded" ] || {
    echo "FAIL: PC13a advisory does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_msg"
    echo "  (empty actual_msg = no warn advisory in log; mismatch = wrong message format)"
    grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
      | grep '"type":"plugin.log"' | grep '"level":"warn"' | head -3 \
      || echo "  (no warn plugin.log records)"
    false
  }

  # Anti-vacuity: a message that passes all three old grep-q substring checks but differs
  # from the normative text must be rejected by full-string equality. Proves PC4a has teeth.
  local wrong_substr_msg
  wrong_substr_msg="validate-cross-site-correspondence [Class B] advisory: (WRONG FORMAT) STORY-INDEX sites agree with each other. State-manager STORY-INDEX update pending; Class B BLOCK suspended."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

# ---------------------------------------------------------------------------
# T-P6D (F-P6-001): PC13b — B2≠B3 → block with three-provenance message
# Fixture: b1-story-index-inconsistent (B1=abc1234, B2=def4567, B3=deadb00, B2≠B3)
# BC-5.39.010 §PC13b: B2≠B3 → STORY-INDEX internally inconsistent → BLOCK.
# Block message must contain "catalog and blockquote disagree" (PC13b normative phrasing).
# No burst-ordering argument explains B2≠B3 — both sites written in same state-manager commit.
#
# RED GATE: current impl blocks on any B1 mismatch (exit 2) but with a DIFFERENT message:
# "All three present sites must agree" — does NOT contain "catalog and blockquote disagree"
# or "has no burst-ordering explanation". Test asserts PC13b normative substring → FAILS.
# After v1.11 implementation: block with normative three-provenance text → PASSES.
# ---------------------------------------------------------------------------

@test "T-P6D (PC13b): B2!=B3 story-index-inconsistent blocks with three-provenance message" {
  _require_artifacts
  _load_fixture "b1-story-index-inconsistent"
  _write_registry

  local envelope
  # S-21.07-test: B1=abc1234; STORY-INDEX: B2=def4567, B3=deadb00 (B2 != B3).
  # PC13b: B2 != B3 → STORY-INDEX internally inconsistent → BLOCK.
  # Block message must contain "catalog and blockquote disagree" (normative) and enumerate
  # all three provenance categories: STALE, FABRICATED, ALGORITHM-DIVERGENT.
  # RED GATE: current message says "All three present sites must agree" — normative
  # "catalog and blockquote disagree" substring is ABSENT → FAILS.
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  # PC13b: B2 != B3 → BLOCK (exit 2).
  _assert_exit 2 "[Class B]"

  # T-P6D normative assertion: full-string equality on the complete PC13b block message.
  # BC-5.39.010 §PC4a: test-writer MUST assert COMPLETE formatted string by equality
  # check; .contains()-only (including compound &&-checks) is NON-CONFORMING. The exit-code
  # check above confirms detection; these assertions confirm the normative text (including
  # "catalog and blockquote disagree", all three provenance categories per invariant 11,
  # and the POLICY 18 (D-923) anchor) is reproduced verbatim.
  # Fixture b1-story-index-inconsistent: B1=abc1234; B2=def4567 (catalog); B3=deadb00 (blockquote).
  # Story ID S-21.07 extracted from basename S-21.07-test via PC17 ^(S-[0-9]+\.[0-9]+) regex.
  local actual_block_reason
  actual_block_reason=$(python3 -c "
import sys
data = open(sys.argv[1]).read()
for line in data.splitlines():
    key = 'block_reason=\"'
    if key in line:
        idx = line.find(key) + len(key)
        val = line[idx:]
        if val.endswith('\"'):
            val = val[:-1]
        val = val.replace('\\\\n', '\\n').replace('\\\\r', '\\r')
        print(val, end='')
        break
" "$_DISP_STDERR" 2>/dev/null || true)

  local expected_decoded
  # Full block_reason as surfaced by dispatcher stderr (includes BLOCKED by / [N] / Fix / Code
  # wrapper from HookResult::block_with_fix + combine_violations_into_block; trailing period
  # on description stripped by trim_end_matches('.') in block_with_fix implementation).
  expected_decoded=$(python3 -c "print('BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence [Class B]: Story S-21.07 input-hash three-way mismatch: frontmatter=abc1234 STORY-INDEX-catalog=def4567 STORY-INDEX-blockquote=deadb00. STORY-INDEX catalog and blockquote disagree — this is anomalous and has no burst-ordering explanation. Update per POLICY 18 (D-923). This hook detects inconsistency only — operator MUST determine which of the following applies before remediating: (a) STALE: previously valid hash; inputs changed after authoring; remedy: rerun \`compute-input-hash --update\` on the story. (b) FABRICATED: hash was never output of \`compute-input-hash --update\` at any revision (POLICY 18 violation); remedy: acknowledge PROVENANCE-BREAK in burst-log before recomputing. (c) ALGORITHM-DIVERGENT: hash produced by prior binary version per ADR-036 §Decision 4; NOT fabricated; remedy: recompute with current authoritative binary, no PROVENANCE-BREAK annotation required. Fix: review and fix all cross-site correspondence issues listed above, then retry the write. Code: POLICY 14/18.', end='')" 2>/dev/null || true)

  [ "$actual_block_reason" = "$expected_decoded" ] || {
    echo "FAIL: T-P6D block message does not match v1.13 normative verbatim text."
    echo "  F-S2107-P7-013 gate: BC-5.39.010 §PC4a requires full-string equality."
    echo "  Expected: $expected_decoded"
    echo "  Actual:   $actual_block_reason"
    echo "  (empty actual = block_reason not found in dispatcher stderr; mismatch = wrong message)"
    false
  }

  # Anti-vacuity: a message containing only the old substrings but wrong format fails equality.
  local wrong_substr_msg
  wrong_substr_msg="(WRONG FORMAT) catalog and blockquote disagree. STALE FABRICATED ALGORITHM-DIVERGENT."
  [ "$wrong_substr_msg" != "$expected_decoded" ] || {
    echo "FAIL: ANTI-VACUITY: substring-passing wrong message must not equal normative text."
    echo "  The equality gate failed to discriminate. expected_decoded may be incorrect."
    false
  }
}

# ---------------------------------------------------------------------------
# T-047-CONTROL: without volatile input, B2≠B3 blocks (proves T-047 discrimination)
# Fixture: b1-story-index-inconsistent (B1=abc1234, B2=def4567, B3=deadb00; no volatile)
# F-P6-003 control: complements T-047 (with volatile → exit 0). This control proves
# that the same B2≠B3 mismatch blocks when volatile inputs are NOT declared.
# Without volatile: no PC40 → PC13b (B2≠B3) → BLOCK (exit 2).
# With volatile (T-047): PC40 fires → advisory + exit 0.
# DISCRIMINATION: PC40 is what prevents the block — not (None,None) arms (old bug).
#
# GREEN once T-P6D passes: this uses the same fixture as T-P6D. Named as "CONTROL"
# to document the T-047 discrimination invariant per F-P6-003.
# ---------------------------------------------------------------------------

@test "T-047-CONTROL: without volatile inputs B2!=B3 blocks (PC13b; proves T-047 discrimination)" {
  _require_artifacts
  _load_fixture "b1-story-index-inconsistent"
  _write_registry

  local envelope
  # No volatile inputs in S-21.07-test.md (b1-story-index-inconsistent fixture).
  # B1=abc1234, B2=def4567, B3=deadb00 (B2 != B3).
  # PC40 does NOT fire (no volatile inputs) → PC13b fires → BLOCK (exit 2).
  # CONTROL for T-047: same B2 != B3 data, but no volatile → proves PC40 is what prevents
  # the block in T-047, not the (None,None) arms from invalid-length hashes (F-P6-003).
  envelope="$(_post_write_event '.factory/stories/S-21.07-test.md')"
  _run_dispatcher "$envelope"

  _assert_plugin_ran_not_crashed
  _assert_exit 2 "[Class B]"
}

# ---------------------------------------------------------------------------
# F-P6-016: Coverage gate — structural counts only (F-S2107-P8-007 fix;
# S-21.07 reconciliation: Class-D-DEFERRED tests removed, not skip-preserved).
# Verifies structural invariants of the bats file itself by counting declaration
# and skip-marker sites.
#   (a) Zero tests carry the "[DEFERRED v1.6 — Class D]" skip marker.
#       The five Class D tests that previously carried this marker (AC-012 x2,
#       AC-013 MUTANT/CONTROL, AC-014) were DELETED alongside arm_d.rs removal
#       (commit e6ce015f) rather than left permanently skipped — a skipped
#       integration test for a deleted module is not preserved coverage, and
#       the "preserved per POLICY 1" rationale that had justified the skip
#       marker misapplied POLICY 1 (append-only ID numbering; scope is
#       BC/VP/DI/CAP/STORY/HS artifacts, not bats test bodies). Class D
#       re-implementation in S-21.08 gets fresh tests against its actual
#       reinstated contract. This assertion guards against the marker being
#       silently reintroduced as a way to leave future orphaned tests in a
#       permanently-skipped state instead of removing them.
#   (b) Exactly 43 _require_artifacts call sites.
#       Each site becomes a skip when the factory-dispatcher binary or WASM is absent.
#       Bounding this explicitly prevents silent growth in dispatcher-gated test count.
#       (was 46; -5 for the removed Class D AC-012/013/014 tests, S-21.07 reconciliation;
#       +1 for ADV-RECON-007 primary-target UTF-8 decode failure red-gate test, v1.20;
#       +1 for ADV-RECON-007 CONTROL valid-UTF-8 regression guard, AC-026 coverage-fill)
#   (c) Total @test declarations >=40 (sanity floor; not the primary execution measure).
#       (was 52 before Class D removal; 47 after; 48 after ADV-RECON-007 mutant added;
#       now 49 — ADV-RECON-007 CONTROL added, AC-026 coverage-fill)
#
# F-S2107-P8-007: removed the "N passed / M skipped / 0 failed" echo that was derived
# from grep counts and hardcoded "0 failed". A test cannot observe its own run's results;
# the echo was fabricating runtime truth from static declarations. run-all.sh TAP
# accounting is the single source of truth for runtime pass/skip/fail figures.
#
# ALWAYS PASSES: no dispatcher invocation. Pure file-structure analysis.
# ---------------------------------------------------------------------------

@test "F-P6-016: coverage gate — 0 deferred, 43 dispatcher-gated, structural counts only" {
  local bats_file="$BATS_TEST_FILENAME"

  # (a) Class-D-DEFERRED skip count: must be exactly 0 (removed, not skip-preserved).
  local deferred_count
  deferred_count=$(grep -c 'skip "\[DEFERRED v1\.6 — Class D\]' "$bats_file" 2>/dev/null || true)
  [ "$deferred_count" -eq 0 ] || {
    echo "FAIL: expected zero Class-D-DEFERRED skip lines, got $deferred_count."
    echo "  S-21.07 reconciliation removed arm_d.rs and its orphaned bats tests"
    echo "  (AC-012 x2, AC-013 MUTANT/CONTROL, AC-014) rather than skip-preserving them."
    echo "  If Class D is being re-implemented, write fresh tests for its real contract"
    echo "  instead of reintroducing this skip marker."
    false
  }

  # (b) _require_artifacts call sites: must be exactly 43.
  # Each site becomes a skip when the factory-dispatcher binary or WASM is absent.
  # Bounding this explicitly prevents silent inflation: a new dispatcher-gated test added
  # without updating this count will fail the gate, not silently grow the skip count.
  # (was 46; -5 for the removed Class D AC-012/013/014 tests, S-21.07 reconciliation;
  # +1 ADV-RECON-007 mutant; +1 ADV-RECON-007 control, AC-026 coverage-fill)
  local req_count
  req_count=$(grep -c '^\s*_require_artifacts$' "$bats_file" 2>/dev/null || true)
  [ "$req_count" -eq 43 ] || {
    echo "FAIL: expected exactly 43 _require_artifacts call sites, got $req_count."
    echo "  This bounds the number of tests that skip when the factory-dispatcher binary"
    echo "  or WASM artifact is absent. Update this count when adding or removing a"
    echo "  dispatcher-gated test."
    false
  }

  # (c) Total @test declarations: sanity floor >=40.
  local total_count
  total_count=$(grep -c '^@test ' "$bats_file" 2>/dev/null || true)
  [ "$total_count" -ge 40 ] || {
    echo "FAIL: expected >=40 @test declarations, got $total_count."
    echo "  Sanity floor. If this fails, tests may have been silently removed."
    false
  }

  # Structural count summary (F-S2107-P8-007 fix: no pass/failed/skipped words here;
  # a test cannot observe its own run's results — run-all.sh TAP accounting is the
  # single source of truth for runtime pass/skip/fail counts).
  # - deferred: always skip regardless of artifact presence.
  # - req_count: dispatcher-gated (skip when artifacts absent).
  # - non-deferred = total - deferred: tests that CAN run when artifacts present.
  # - non-gated = total - req_count: tests that always run (no _require_artifacts).
  local non_deferred=$(( total_count - deferred_count ))
  local non_gated=$(( total_count - req_count ))
  echo "Coverage structural counts (F-P6-016):"
  echo "  Total @test declarations: $total_count"
  echo "  Class-D-DEFERRED (always skip): $deferred_count"
  echo "  _require_artifacts call sites (dispatcher-gated): $req_count"
  echo "  Non-deferred tests (require dispatcher + WASM): $non_deferred"
  echo "  Non-gated tests (no _require_artifacts, always run): $non_gated"
  echo "  Runtime pass/skip/fail: see run-all.sh TAP output (not derived here)"
}

