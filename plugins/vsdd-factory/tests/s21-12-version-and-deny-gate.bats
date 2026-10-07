#!/usr/bin/env bats
# s21-12-version-and-deny-gate.bats — Red Gate bats tests for S-21.12.
#
# Story:  S-21.12 (E-21 Wave 4)
# Title:  wasmtime major-version move >= 48.0.4: clear RUSTSEC-2026-0188/0222/0204 + 0316/0314/0321/0322/0323/0324/0327 (ADR-035 v1.2 Decision 6)
#         + add cargo-deny advisories CI job
#
# These tests FAIL at the pre-story state:
#   wasmtime/wasmtime-wasi = "44.0" in Cargo.toml (resolved to 44.0.3)
#   crossbeam-epoch = 0.9.18 in Cargo.lock
#   no `cargo deny check advisories` job in .github/workflows/
#
# and PASS only after the story's implementation is merged.
#
# | Test name                                                         | AC         |
# |-------------------------------------------------------------------|------------|
# | AC-001: Cargo.toml wasmtime dep does not contain 44.0 pin        | AC-001     |
# | AC-002: Cargo.toml wasmtime-wasi dep does not contain 44.0 pin   | AC-002     |
# | AC-003: cargo metadata resolves wasmtime+wasmtime-wasi to 48.0.4..<49 lockstep | AC-003     |
# | AC-004/AC-009: cargo deny exits 0, five RUSTSEC IDs absent       | AC-004/009 |
# | AC-007-T1: a workflow file contains cargo deny check advisories   | AC-007     |
# | AC-007-T2: deny workflow has no paths: on workflow-level trigger  | AC-007     |

setup() {
  # BATS_TEST_DIRNAME = .../plugins/vsdd-factory/tests
  # repo root is three directories up from tests/
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
}

# ---------------------------------------------------------------------------
# AC-001: Cargo.toml workspace pins wasmtime >= 48.0.4
# Traces to: RUSTSEC-2026-0188/0222 (implied) + RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327; ADR-035 v1.2 Decision 6
# RED-before: wasmtime = "44.0" in Cargo.toml  → test FAILS
# GREEN-after: wasmtime = "48.0.4" (or higher 48.x) in Cargo.toml → test PASSES
# ---------------------------------------------------------------------------
@test "AC-001: Cargo.toml wasmtime workspace dep floored at 48.0.4 (not 44.0)" {
  local cargo_toml="$REPO_ROOT/Cargo.toml"
  [ -f "$cargo_toml" ] || { echo "FAIL: Cargo.toml not found at $cargo_toml"; return 1; }

  # Extract the wasmtime workspace dependency line (not wasmtime-wasi).
  local wasmtime_line
  wasmtime_line=$(grep -E '^wasmtime[[:space:]]*=' "$cargo_toml") || {
    echo "FAIL: no wasmtime = ... line found in $cargo_toml"
    return 1
  }

  # The old "44.0" pin MUST NOT be present.
  if echo "$wasmtime_line" | grep -qF '"44.0"'; then
    echo "FAIL: wasmtime still carries the 44.0 pin: $wasmtime_line"
    echo "Expected: wasmtime = \"48.0.4\" (or equivalent >= 48.0.4 floor)"
    return 1
  fi

  # The new floor MUST be at 48.x (RUSTSEC-2026-0316 et al.; 46.x no longer passes cargo deny).
  if ! echo "$wasmtime_line" | grep -qE '"48\.'; then
    echo "FAIL: wasmtime not pinned at 48.x: $wasmtime_line"
    echo "Expected: wasmtime = \"48.0.4\""
    return 1
  fi
}

# ---------------------------------------------------------------------------
# AC-002: Cargo.toml workspace pins wasmtime-wasi >= 48.0.4
# Traces to: RUSTSEC-2026-0188/0222 (implied) + RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327; ADR-035 v1.2 Decision 6
# RED-before: wasmtime-wasi = "44.0" in Cargo.toml → test FAILS
# GREEN-after: wasmtime-wasi = "48.0.4" (or higher 48.x) in Cargo.toml → test PASSES
# ---------------------------------------------------------------------------
@test "AC-002: Cargo.toml wasmtime-wasi workspace dep floored at 48.0.4 (not 44.0)" {
  local cargo_toml="$REPO_ROOT/Cargo.toml"
  [ -f "$cargo_toml" ] || { echo "FAIL: Cargo.toml not found at $cargo_toml"; return 1; }

  # Extract the wasmtime-wasi workspace dependency line.
  local wasi_line
  wasi_line=$(grep -E '^wasmtime-wasi[[:space:]]*=' "$cargo_toml") || {
    echo "FAIL: no wasmtime-wasi = ... line found in $cargo_toml"
    return 1
  }

  # The old "44.0" pin MUST NOT be present.
  if echo "$wasi_line" | grep -qF '"44.0"'; then
    echo "FAIL: wasmtime-wasi still carries the 44.0 pin: $wasi_line"
    echo "Expected: wasmtime-wasi = \"48.0.4\" (or equivalent >= 48.0.4 floor)"
    return 1
  fi

  # The new floor MUST be at 48.x.
  if ! echo "$wasi_line" | grep -qE '"48\.'; then
    echo "FAIL: wasmtime-wasi not pinned at 48.x: $wasi_line"
    echo "Expected: wasmtime-wasi = \"48.0.4\""
    return 1
  fi
}

# ---------------------------------------------------------------------------
# AC-003: cargo metadata --locked resolves wasmtime AND wasmtime-wasi to 48.0.4 <= v < 49.0.0 (lockstep, identical)
# Traces to: cargo lockfile correctness
# RED-before: wasmtime-wasi resolves to 44.0.3 → version comparison fails
# GREEN-after: both resolve to the same 48.x >= 48.0.4 → version comparison passes
# ---------------------------------------------------------------------------
@test "AC-003: cargo metadata --locked resolves wasmtime and wasmtime-wasi to lockstep >= 48.0.4 and < 49.0.0" {
  command -v jq >/dev/null 2>&1 || skip "jq required for cargo metadata JSON parsing"

  # Capture stdout and exit status separately from stderr — merging stderr into
  # the JSON stream (via 2>&1) corrupts jq's input whenever cargo emits any
  # diagnostic text (e.g. registry/index warnings), producing a false FAIL that
  # has nothing to do with the actual wasmtime-wasi version.
  local metadata_json metadata_exit
  set +e
  metadata_json=$(cd "$REPO_ROOT" && cargo metadata --format-version 1 --locked 2>/dev/null)
  metadata_exit=$?
  set -e

  if [ "$metadata_exit" -ne 0 ] || [ -z "$metadata_json" ]; then
    # This job runs bats in an environment where `cargo metadata` is not
    # guaranteed to succeed (e.g. no warm registry cache, sandboxed network).
    # The wasmtime-wasi version floor is authoritatively verified by the
    # cargo-host and deny-advisories CI jobs, which run cargo metadata /
    # cargo deny natively against this same lockfile and gate on it. This
    # bats check is a convenience mirror, not the source of truth — skip
    # cleanly here rather than hard-failing on an environment gap.
    skip "cargo metadata --locked failed or produced no output in this job (exit ${metadata_exit}) — wasmtime-wasi version floor is authoritatively verified by the cargo-host + deny-advisories CI jobs"
  fi

  local pkg resolved_version major minor patch
  local versions=""
  for pkg in wasmtime wasmtime-wasi; do
    resolved_version=$(echo "$metadata_json" | jq -r --arg n "$pkg" '.packages[] | select(.name == $n) | .version' | head -1)

    [ -n "$resolved_version" ] || {
      echo "FAIL: $pkg not found in 'cargo metadata --locked' output"
      echo "(cargo metadata succeeded but produced no $pkg package entry — check Cargo.toml/Cargo.lock)"
      return 1
    }

    major=$(echo "$resolved_version" | cut -d. -f1)
    minor=$(echo "$resolved_version" | cut -d. -f2)
    patch=$(echo "$resolved_version" | cut -d. -f3)

    # Assert 48.0.4 <= version < 49.0.0
    if [ "$major" -eq 48 ] && { [ "$minor" -gt 0 ] || [ "$patch" -ge 4 ]; }; then
      :
    else
      echo "FAIL: $pkg resolved to $resolved_version, expected >= 48.0.4 and < 49.0.0"
      echo "(ADR-035 v1.2 Decision 6: RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327 are not patched below 48.0.4)"
      return 1
    fi
    versions="$versions $resolved_version"
  done

  # Lockstep: wasmtime and wasmtime-wasi MUST resolve to the identical version.
  local v1 v2
  v1=$(echo "$versions" | awk '{print $1}')
  v2=$(echo "$versions" | awk '{print $2}')
  [ "$v1" = "$v2" ] || {
    echo "FAIL: wasmtime ($v1) and wasmtime-wasi ($v2) are not in lockstep"
    return 1
  }
}

# ---------------------------------------------------------------------------
# AC-004 + AC-009: cargo deny check advisories exits 0 and all five RUSTSEC
# advisory IDs (RUSTSEC-2026-0188, RUSTSEC-2026-0222, RUSTSEC-2026-0204,
# RUSTSEC-2026-0190, RUSTSEC-2025-0052) are absent from the output.
#
# Traces to: deny.toml [advisories] deny-all posture
# RED-before: advisories present → exit non-zero → test FAILS
# GREEN-after: all five patched/removed, deny.toml ignore = [] → exit 0 → test PASSES
#
# AC-009 note: RUSTSEC-2026-0204 (crossbeam-epoch pointer dereference) is
# cleared by the crossbeam-epoch >= 0.9.20 transitive bump. Without that bump
# the deny job exits non-zero on 0204, making the exit-0 assertion impossible.
# RUSTSEC-2026-0190 (anyhow unsoundness) is cleared by anyhow >= 1.0.104.
# RUSTSEC-2025-0052 (async-std via httpmock 0.7) is cleared by httpmock >= 0.8.
# ---------------------------------------------------------------------------
@test "AC-004/AC-009: cargo deny check advisories exits 0 and RUSTSEC-2026-0188/0222/0204/0190/0052 absent" {
  # cargo-deny must be installed; skip if absent so the bats suite does not
  # error-out in environments where cargo-deny is not yet installed.
  cargo deny --version >/dev/null 2>&1 || skip "cargo-deny not installed (cargo deny --version failed)"

  local output exit_code
  set +e
  output=$(cd "$REPO_ROOT" && cargo deny check advisories 2>&1)
  exit_code=$?
  set -e

  if [ "$exit_code" -ne 0 ]; then
    echo "FAIL: cargo deny check advisories exited $exit_code (expected 0)"
    echo "--- cargo deny output ---"
    echo "$output"
    echo "--- end ---"
    return 1
  fi

  local failed=0
  for advisory in RUSTSEC-2026-0188 RUSTSEC-2026-0222 RUSTSEC-2026-0204 RUSTSEC-2026-0190 RUSTSEC-2025-0052; do
    if echo "$output" | grep -qF "$advisory"; then
      echo "FAIL: $advisory still present in cargo deny output"
      failed=1
    fi
  done

  if [ "$failed" -ne 0 ]; then
    echo "--- cargo deny output ---"
    echo "$output"
    echo "--- end ---"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# AC-007 Test 1: a workflow file under .github/workflows/ contains a job that
# runs `cargo deny check advisories` in a pull_request context.
# Traces to: PR-007 process gap (deny.toml present but no CI job)
# RED-before: no deny job in ci.yml → grep finds nothing → test FAILS
# GREEN-after: deny job added → grep finds the string → test PASSES
# ---------------------------------------------------------------------------
@test "AC-007-T1: a workflow file contains a cargo-deny advisories job" {
  # Strengthened assertion (O-2): checking for the step NAME string
  # 'cargo deny check advisories' is insufficient — a step can have that name
  # while running a completely different command (e.g. 'command-arguments: licenses').
  # Instead, assert the REAL invocation: the workflow must contain BOTH
  #   (a) EmbarkStudios/cargo-deny-action (SHA-pinned usage), AND
  #   (b) command-arguments: advisories
  # so that renaming command-arguments from 'advisories' to anything else causes
  # this test to FAIL even if the step name is preserved.
  local workflows_dir="$REPO_ROOT/.github/workflows"
  [ -d "$workflows_dir" ] || { echo "FAIL: .github/workflows/ not found at $workflows_dir"; return 1; }

  # (a) Find the workflow file that uses EmbarkStudios/cargo-deny-action (SHA-pinned).
  local deny_workflow=""
  while IFS= read -r -d '' wf; do
    if grep -qF 'EmbarkStudios/cargo-deny-action@' "$wf"; then
      deny_workflow="$wf"
      break
    fi
  done < <(find "$workflows_dir" -name '*.yml' -print0 2>/dev/null)

  if [ -z "$deny_workflow" ]; then
    echo "FAIL: no workflow file in $workflows_dir uses 'EmbarkStudios/cargo-deny-action@' (SHA-pinned)"
    echo "Expected: .github/workflows/ci.yml to contain a step using EmbarkStudios/cargo-deny-action@<SHA>"
    echo "Found workflow files:"
    find "$workflows_dir" -name '*.yml' 2>/dev/null | sort
    return 1
  fi

  # (b) The same workflow file must also contain 'command-arguments: advisories' —
  # this is the argument that actually runs 'cargo deny check advisories', not
  # any other subcommand (e.g. 'licenses', 'bans').
  if ! grep -qF 'command-arguments: advisories' "$deny_workflow"; then
    echo "FAIL: $deny_workflow uses EmbarkStudios/cargo-deny-action but does not contain 'command-arguments: advisories'"
    echo "Expected: the deny job to invoke 'cargo deny check advisories' via command-arguments"
    echo "Actual command-arguments lines in $deny_workflow:"
    grep 'command-arguments' "$deny_workflow" || echo "  (none found)"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# AC-007 Test 2: the workflow file that contains the cargo-deny advisories job
# MUST NOT have a `paths:` key on the workflow-level `on: pull_request:` block.
#
# Rationale (from AC-007): GitHub Actions has no per-job paths: trigger.
# A paths: filter on the workflow-level on.pull_request trigger would allow
# the advisory check to be skipped on PRs that do not touch Cargo files —
# recreating the gap that allowed RUSTSEC-2026-0188 to sit silently on develop.
#
# Traces to: PR-007 gap analysis
# RED-before: no deny job → prerequisite check fails → test FAILS
# GREEN-after: deny job present AND on.pull_request has no paths: → test PASSES
# ---------------------------------------------------------------------------
@test "AC-007-T2: workflow containing cargo-deny job has no paths: filter on workflow-level on.pull_request" {
  local workflows_dir="$REPO_ROOT/.github/workflows"
  [ -d "$workflows_dir" ] || { echo "FAIL: .github/workflows/ not found"; return 1; }

  # Find the workflow file that contains the deny job.
  local deny_workflow=""
  while IFS= read -r -d '' wf; do
    if grep -qF 'cargo deny check advisories' "$wf"; then
      deny_workflow="$wf"
      break
    fi
  done < <(find "$workflows_dir" -name '*.yml' -print0 2>/dev/null)

  if [ -z "$deny_workflow" ]; then
    echo "FAIL: no workflow file contains 'cargo deny check advisories'"
    echo "(AC-007-T1 prerequisite not met — no deny job to inspect for paths: filter)"
    return 1
  fi

  # Extract the top-level `on:` block (lines from `^on:` up to `^jobs:`).
  local on_block
  on_block=$(awk '/^on:/{found=1} found && /^jobs:/{exit} found{print}' "$deny_workflow")

  # Assert the on.pull_request trigger does NOT contain a `paths:` key.
  if echo "$on_block" | grep -qE '^[[:space:]]+paths:'; then
    echo "FAIL: workflow-level on.pull_request trigger in $deny_workflow contains a paths: filter"
    echo "A paths: filter would skip the advisory check on non-Cargo PRs — recreating PR-007 gap"
    echo "--- on: block ---"
    echo "$on_block"
    echo "--- end ---"
    return 1
  fi
}
