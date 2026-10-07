#!/bin/bash
# run-all.sh — run the full vsdd-factory plugin test suite.
#
# Requirements: bats-core, jq, yq.

set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PLUGIN_ROOT"

die() { echo "run-all: $*" >&2; exit 1; }

for tool in bats jq yq; do
  command -v "$tool" &>/dev/null || die "$tool is required. Install: brew install $tool"
done

echo "== Syntax checks =="
fail=0
for f in hooks/*.sh hooks/dim2-gates/*.sh bin/*; do
  # Skip glob patterns that matched nothing (nullglob not set for this loop)
  [ -e "$f" ] || continue
  if ! bash -n "$f" 2>&1; then
    echo "FAIL: $f"
    fail=$((fail+1))
  fi
done
[ "$fail" -eq 0 ] || die "$fail syntax errors"
echo "all scripts ok"

echo
echo "== Running all bats test suites =="

# SKIP_SUITES — bats suites excluded from the release-validation gate.
# Currently empty after the TD-020 sweep (2026-05-04) resolved the four
# previously-skipped suites (codify-lessons, generate-registry,
# novelty-assessment, state-health). See CHANGELOG entry "TD-020 sweep —
# bats SKIP_SUITES cleanup" for per-suite outcomes.
#
# Do NOT add new entries without an accompanying tech-debt-register
# ticket and an inline rationale.
SKIP_SUITES=()

is_skipped() {
  local target="$1"
  local s
  # Guard for empty array under `set -u` on older bash (3.2 on macOS).
  [ "${#SKIP_SUITES[@]}" -eq 0 ] && return 1
  for s in "${SKIP_SUITES[@]}"; do
    [ "$s" = "$target" ] && return 0
  done
  return 1
}

shopt -s nullglob
set +e   # allow individual bats suites to fail without aborting the loop
fail_count=0
failed_suites=()
skipped_suites=()
# F-S2107-P1C-018: track pass/skip counts so the final summary line distinguishes
# "34 executed" from "34 skipped" — a suite that cannot make this distinction is a
# false-green generator (blast radius: any dispatcher-dependent suite reaching
# _require_artifacts() without CI_REQUIRE_ARTIFACTS set).
total_suites=0
total_tests=0
total_skips=0
for f in tests/*.bats tests/dim2-gates/*.bats tests/docs-completeness/*.bats tests/validate-index-cite-refresh/*.bats tests/validate-burst-log/*.bats tests/validate-closes-completeness/*.bats tests/validate-state-structure/*.bats tests/validate-state-size/*.bats tests/validate-dispatch-advance/*.bats tests/validate-policies-schema/*.bats tests/validate-trajectory-tail-cell-completeness/*.bats; do
  name=$(basename "$f" .bats)
  if is_skipped "$name"; then
    skipped_suites+=("$name")
    continue
  fi
  echo
  echo "-- $name --"
  # Capture TAP output so we can count passes and skips without running each suite twice.
  tap_out=$(bats --tap "$f" 2>&1)
  bats_exit=$?
  echo "$tap_out"
  # grep -c always outputs a count (even "0") and exits 1 when zero matches.
  # In set+e context that exit 1 is benign; do NOT add "|| echo 0" which would
  # produce "0\n0" and break the arithmetic expansion on the next line.
  suite_total=$(echo "$tap_out" | grep -cE '^(ok|not ok) [0-9]+'; true)
  suite_skips=$(echo "$tap_out" | grep -cE '^ok [0-9]+.*# skip'; true)
  total_suites=$((total_suites + 1))
  total_tests=$((total_tests + suite_total))
  total_skips=$((total_skips + suite_skips))
  if [ "$bats_exit" -ne 0 ]; then
    fail_count=$((fail_count + 1))
    failed_suites+=("$name")
  fi
done
set -e

if [ "${#skipped_suites[@]}" -gt 0 ]; then
  echo
  echo "== Skipped suites (TD-020 — pre-existing failures) =="
  for name in "${skipped_suites[@]}"; do
    echo "  - $name"
  done
fi

echo
# F-S2107-P1C-018: positive-coverage summary — distinguishes executed vs skipped.
# Fail if every collected test was skipped (total_executed == 0 and total_tests > 0)
# so a broken staging step that converts all payload tests to silent skips does not
# produce a false-green "All tests passed." with zero active coverage.
total_executed=$((total_tests - total_skips))
echo "Coverage: $total_executed executed, $total_skips skipped ($total_tests total across $total_suites suites)."

if [ "$fail_count" -gt 0 ]; then
  echo "FAIL: $fail_count suite(s) had failures:"
  for name in "${failed_suites[@]}"; do
    echo "  - $name"
  done
  exit 1
fi

if [ "$total_executed" -eq 0 ] && [ "$total_tests" -gt 0 ]; then
  echo "FAIL: All $total_tests tests were skipped — suite produced no active coverage."
  exit 1
fi

echo "All tests passed."
