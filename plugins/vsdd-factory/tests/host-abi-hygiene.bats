#!/usr/bin/env bats
# host-abi-hygiene.bats — S-19.09 D20/D21 static source hygiene assertions.
#
# Tests T-004 through T-012 verify the documentation and bare-literal
# hygiene requirements of story S-19.09:
#
#   D20 (timeout_ms framing):
#     T-004: stale "epoch interruption" text absent from read_file.rs
#     T-005: stale "epoch interruption" text absent from read_prefix.rs
#     T-006: corrected "structurally unenforced" text present in read_file.rs
#     T-007: corrected "structurally unenforced" text present in read_prefix.rs
#     T-008: two-linker duality comment (Linker<StoreData>/setup_host_on_store_data)
#            present in read_file.rs adjacent to the test-path register()
#
#   D21 (named constants for telemetry literals):
#     T-009: pub const INTERNAL_FILE_NOT_FOUND exported from internal_log.rs
#            with value "internal.file_not_found" (grep-gate; with mutation-liveness)
#     T-010: pub const PLUGIN_ABANDONED exported from internal_log.rs
#            with value "plugin.abandoned" (grep-gate; with mutation-liveness)
#     T-011: zero bare "internal.file_not_found"/"plugin.abandoned"/
#            "plugin.completed"/"plugin.timeout" literals in production
#            code (before #[cfg(test)]) of read_file.rs, read_prefix.rs,
#            emit_event.rs (extended to four classes per story v1.1/AC-008)
#     T-012: cargo test -p factory-dispatcher --lib -- s19_09 exits 0 (bidirectional
#            regression gate, scoped to the D19/D22 read_prefix + timestamp unit tests;
#            narrowed from --all-targets per rc.24 flake fix — see T-012 comment below)
#
# RED gate status (pre-D20/D21 at develop 9787c056):
#   T-004: FAILS — "enforced in S-1.5 via epoch interruption" present in read_file.rs
#   T-005: FAILS — "enforced via epoch interruption" present in read_prefix.rs
#   T-006: FAILS — "structurally unenforced" absent from read_file.rs
#   T-007: FAILS — "structurally unenforced" absent from read_prefix.rs
#   T-008: FAILS — "Linker<StoreData>"/"setup_host_on_store_data" absent from read_file.rs
#   T-009: FAILS — pub const INTERNAL_FILE_NOT_FOUND absent from internal_log.rs
#   T-010: FAILS — pub const PLUGIN_ABANDONED absent from internal_log.rs
#   T-011: FAILS — bare literals present in production code of all three files
#   T-012: FAILS — T-001/T-002/T-003 (production linker) and T-013 (timestamp) fail at runtime
#
# VP trace: VP-101
# Story: S-19.09
# BC: AC-004, AC-005, AC-006, AC-007, AC-008, AC-009, AC-010

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  READ_FILE_RS="$REPO_ROOT/crates/factory-dispatcher/src/host/read_file.rs"
  READ_PREFIX_RS="$REPO_ROOT/crates/factory-dispatcher/src/host/read_prefix.rs"
  EMIT_EVENT_RS="$REPO_ROOT/crates/factory-dispatcher/src/host/emit_event.rs"
  INTERNAL_LOG_RS="$REPO_ROOT/crates/factory-dispatcher/src/internal_log.rs"
  EXECUTOR_RS="$REPO_ROOT/crates/factory-dispatcher/src/executor.rs"
  VSDD_SINK_RS="$REPO_ROOT/crates/factory-dispatcher/src/vsdd_sink.rs"
  TEST_TMP="$(mktemp -d "${BATS_TMPDIR}/host-abi-hygiene-XXXXXX")"
}

teardown() {
  rm -rf "${TEST_TMP}"
}

# ---------------------------------------------------------------------------
# T-004  AC-004 Gate A — stale "epoch interruption" text absent from read_file.rs
#
# The comment on `let _ = timeout_ms;` in read_file.rs before D20 reads:
#   "accepted for ABI stability; enforced in S-1.5 via epoch interruption"
# This text is technically incorrect (ADR-025 §Decision 18): epoch interruption
# fires only at WASM yield points, not during synchronous func_wrap host closures.
# After D20, the corrected form is required; the stale text must be absent.
#
# Gate A (negative): `! grep -q "epoch interruption" read_file.rs` exits 0.
#
# RED today: stale text is present; `grep -q "epoch interruption"` exits 0,
# so `!` makes the bats assertion exit 1 (FAIL).
# ---------------------------------------------------------------------------
@test "T-004 AC-004 Gate A: stale 'epoch interruption' text absent from read_file.rs" {
  [ -f "$READ_FILE_RS" ] || {
    echo "FAIL: read_file.rs not found at $READ_FILE_RS"
    false
  }
  if grep -q "epoch interruption" "$READ_FILE_RS"; then
    echo "FAIL: stale 'epoch interruption' text still present in $READ_FILE_RS"
    echo "D20 requires replacing it with the corrected ADR-025 §Decision 18 form."
    grep -n "epoch interruption" "$READ_FILE_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-005  AC-004 Gate A — stale "epoch interruption" text absent from read_prefix.rs
#
# The comment in read_prefix.rs reads:
#   "accepted for ABI stability; enforced via epoch interruption"
# Same rationale as T-004; incorrect for the same structural reason.
# After D20, this stale text must be absent.
#
# RED today: stale text present in read_prefix.rs.
# ---------------------------------------------------------------------------
@test "T-005 AC-004 Gate A: stale 'epoch interruption' text absent from read_prefix.rs" {
  [ -f "$READ_PREFIX_RS" ] || {
    echo "FAIL: read_prefix.rs not found at $READ_PREFIX_RS"
    false
  }
  if grep -q "epoch interruption" "$READ_PREFIX_RS"; then
    echo "FAIL: stale 'epoch interruption' text still present in $READ_PREFIX_RS"
    echo "D20 requires replacing it with the corrected ADR-025 §Decision 18 form."
    grep -n "epoch interruption" "$READ_PREFIX_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-006  AC-004 Gate B — corrected "structurally unenforced" text present in read_file.rs
#
# After D20, the corrected comment form (per ADR-025 §Decision 18) must contain
# "structurally unenforced" as its anchor phrase:
#   "per-host-function timeout is structurally unenforced in the current
#    synchronous func_wrap dispatch path"
#
# Gate B (positive): `grep -q "structurally unenforced" read_file.rs` exits 0.
#
# RED today: corrected text absent; `grep -q` exits 1 (not found).
# ---------------------------------------------------------------------------
@test "T-006 AC-004 Gate B: corrected 'structurally unenforced' text present in read_file.rs" {
  [ -f "$READ_FILE_RS" ] || {
    echo "FAIL: read_file.rs not found at $READ_FILE_RS"
    false
  }
  if ! grep -q "structurally unenforced" "$READ_FILE_RS"; then
    echo "FAIL: corrected timeout_ms comment not found in $READ_FILE_RS"
    echo "D20 requires the corrected ADR-025 §Decision 18 form containing 'structurally unenforced'."
    echo "Expected text (per ADR-025 §Decision 18):"
    echo "  // accepted for ABI forward-compatibility; per-host-function timeout is structurally"
    echo "  // unenforced in the current synchronous func_wrap dispatch path; ..."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-007  AC-004 Gate B — corrected "structurally unenforced" text present in read_prefix.rs
#
# Same corrected-form requirement as T-006 but for read_prefix.rs.
#
# RED today: corrected text absent from read_prefix.rs.
# ---------------------------------------------------------------------------
@test "T-007 AC-004 Gate B: corrected 'structurally unenforced' text present in read_prefix.rs" {
  [ -f "$READ_PREFIX_RS" ] || {
    echo "FAIL: read_prefix.rs not found at $READ_PREFIX_RS"
    false
  }
  if ! grep -q "structurally unenforced" "$READ_PREFIX_RS"; then
    echo "FAIL: corrected timeout_ms comment not found in $READ_PREFIX_RS"
    echo "D20 requires the corrected ADR-025 §Decision 18 form containing 'structurally unenforced'."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-008  AC-005 — two-linker out_ptr=0 protocol comment present in read_file.rs
#
# D20 requires a comment in read_file.rs's register() function (adjacent to
# the prepare() call or the Ok((bytes, 0)) return) that documents the
# test-path vs production-path duality:
#
#   test-path  (Linker<HostContext> / setup_linker in host/mod.rs):
#     register() always returns out_ptr=0; SDK ptr==0 guard → Vec::new()
#
#   production-path (Linker<StoreData> / setup_host_on_store_data in invoke.rs):
#     grows WASM memory and writes at current_bytes > 0
#
# Gate: `grep -qE "Linker.*StoreData|setup_host_on_store_data" read_file.rs`
# exits 0. This anchor is unique to the new two-linker duality comment and is
# absent from the current file (no reference to Linker<StoreData> or
# setup_host_on_store_data exists in read_file.rs today).
#
# RED today: anchor absent from read_file.rs.
# ---------------------------------------------------------------------------
@test "T-008 AC-005: two-linker duality comment (Linker<StoreData>/setup_host_on_store_data) present in read_file.rs" {
  [ -f "$READ_FILE_RS" ] || {
    echo "FAIL: read_file.rs not found at $READ_FILE_RS"
    false
  }
  if ! grep -qE "Linker.*StoreData|setup_host_on_store_data" "$READ_FILE_RS"; then
    echo "FAIL: two-linker duality comment absent from $READ_FILE_RS"
    echo "D20 AC-005 requires a comment in register() distinguishing:"
    echo "  test-path (Linker<HostContext>/setup_linker) → out_ptr=0 constant"
    echo "  production-path (Linker<StoreData>/setup_host_on_store_data) → out_ptr>0"
    echo "Anchor: 'Linker<StoreData>' or 'setup_host_on_store_data' must appear in the file."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009  AC-006 — pub const INTERNAL_FILE_NOT_FOUND exported from internal_log.rs
#
# After D21, internal_log.rs must export a pub const named INTERNAL_FILE_NOT_FOUND
# with the value "internal.file_not_found".  This constant replaces all bare
# occurrences of that string in production code (read_file.rs, read_prefix.rs).
#
# Gate: grep -qE 'pub[[:space:]]+const[[:space:]]+INTERNAL_FILE_NOT_FOUND'
#        internal_log.rs exits 0.
#
# Mutation-liveness: inject the expected declaration into a temp copy and assert
# the pattern fires, confirming the grep expression is not vacuously correct.
#
# RED today: constant absent from internal_log.rs; grep exits 1 → test FAILS.
# ---------------------------------------------------------------------------
@test "T-009 AC-006: pub const INTERNAL_FILE_NOT_FOUND exported from internal_log.rs" {
  [ -f "$INTERNAL_LOG_RS" ] || {
    echo "FAIL: internal_log.rs not found at $INTERNAL_LOG_RS"
    false
  }

  # Mutation-liveness: verify the grep pattern matches the expected declaration form.
  local mut_file
  mut_file="${TEST_TMP}/mutant.rs"
  printf 'pub const INTERNAL_FILE_NOT_FOUND: &str = "internal.file_not_found";\n' > "$mut_file"
  if ! grep -qE 'pub[[:space:]]+const[[:space:]]+INTERNAL_FILE_NOT_FOUND' "$mut_file"; then
    rm -f "$mut_file"
    echo "FAIL: mutation-liveness — grep pattern did not match injected INTERNAL_FILE_NOT_FOUND declaration."
    false
  fi
  rm -f "$mut_file"
  echo "PASS mutation-liveness: pattern correctly identifies INTERNAL_FILE_NOT_FOUND declaration."

  if ! grep -qE 'pub[[:space:]]+const[[:space:]]+INTERNAL_FILE_NOT_FOUND' "$INTERNAL_LOG_RS"; then
    echo "FAIL: pub const INTERNAL_FILE_NOT_FOUND not found in $INTERNAL_LOG_RS"
    echo "D21 requires: pub const INTERNAL_FILE_NOT_FOUND: &str = \"internal.file_not_found\";"
    echo "exported from internal_log.rs to replace bare \"internal.file_not_found\" literals."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-010  AC-007 — pub const PLUGIN_ABANDONED exported from internal_log.rs
#
# After D21, internal_log.rs must export a pub const named PLUGIN_ABANDONED
# with the value "plugin.abandoned".  This constant replaces all bare
# occurrences of that string in production code (emit_event.rs).
#
# Gate: grep -qE 'pub[[:space:]]+const[[:space:]]+PLUGIN_ABANDONED'
#        internal_log.rs exits 0.
#
# Mutation-liveness: same pattern as T-009.
#
# RED today: constant absent from internal_log.rs; grep exits 1 → test FAILS.
# ---------------------------------------------------------------------------
@test "T-010 AC-007: pub const PLUGIN_ABANDONED exported from internal_log.rs" {
  [ -f "$INTERNAL_LOG_RS" ] || {
    echo "FAIL: internal_log.rs not found at $INTERNAL_LOG_RS"
    false
  }

  # Mutation-liveness: verify the grep pattern matches the expected declaration form.
  local mut_file
  mut_file="${TEST_TMP}/mutant.rs"
  printf 'pub const PLUGIN_ABANDONED: &str = "plugin.abandoned";\n' > "$mut_file"
  if ! grep -qE 'pub[[:space:]]+const[[:space:]]+PLUGIN_ABANDONED' "$mut_file"; then
    rm -f "$mut_file"
    echo "FAIL: mutation-liveness — grep pattern did not match injected PLUGIN_ABANDONED declaration."
    false
  fi
  rm -f "$mut_file"
  echo "PASS mutation-liveness: pattern correctly identifies PLUGIN_ABANDONED declaration."

  if ! grep -qE 'pub[[:space:]]+const[[:space:]]+PLUGIN_ABANDONED' "$INTERNAL_LOG_RS"; then
    echo "FAIL: pub const PLUGIN_ABANDONED not found in $INTERNAL_LOG_RS"
    echo "D21 requires: pub const PLUGIN_ABANDONED: &str = \"plugin.abandoned\";"
    echo "exported from internal_log.rs to replace bare \"plugin.abandoned\" literals."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-011  AC-008 — zero bare literals in production code of read_file.rs,
#         read_prefix.rs, emit_event.rs, executor.rs, vsdd_sink.rs
#         (extended to four classes, story v1.1; perimeter widened F-P2-001)
#
# After D21 (original sweep) + D22 sweep (8913c2e9), all bare occurrences of
# all four telemetry literal classes in production code (the region before
# the #[cfg(test)] boundary) must be replaced by named constants:
#   "internal.file_not_found" → INTERNAL_FILE_NOT_FOUND
#   "plugin.abandoned"        → PLUGIN_ABANDONED
#   "plugin.completed"        → PLUGIN_COMPLETED
#   "plugin.timeout"          → PLUGIN_TIMEOUT
#
# Pipeline (per AC-008):
#   Stage 1: awk '/^#\[cfg\(test\)\]/{exit} {print}'
#            Scopes output to the production region only.
#   Stage 2: stateful awk block-comment stripper (strips /* ... */ spans).
#   Stage 3: sed 's://.*::'
#            Strips // line comments.
#   Stage 4: grep -oE '"internal\.file_not_found"|"plugin\.abandoned"|
#                      "plugin\.completed"|"plugin\.timeout"'
#            Emits literal matches; empty output → gate passes.
#
# Mutation-liveness checks per TD-VSDD-059:
#   (a) original: inject "internal.file_not_found" and assert gate fires.
#   (b) extension: inject "plugin.completed" and assert gate fires — proves
#       the two new D22 classes ("plugin.completed"/"plugin.timeout") are
#       live in the gate, not merely subsumed by the original two patterns.
#   (c) executor.rs extension: inject "plugin.completed" into executor.rs
#       and assert gate fires — proves the F-P2-001 perimeter widening is live.
#
# RED today (pre-sweep): production code of all three files contains bare
# literals; the gate returns non-empty output for each file.
# GREEN post-D22 (8913c2e9): all four classes swept to named constants.
# executor.rs/vsdd_sink.rs: already const-clean; perimeter widened F-P2-001.
# ---------------------------------------------------------------------------

# Helper: run the 4-stage bare-literal scan on a file path (arg $1).
# Prints any found bare literals; exit 0 regardless (caller checks output).
_scan_bare_literals() {
  local target="$1"
  awk '/^#\[cfg\(test\)\]/{exit} {print}' "$target" \
    | awk 'BEGIN{b=0}{
        if(b){
          if(sub(/.*\*\//,""))b=0
          else next
        }
        while(match($0,/\/\*/)){
          s=substr($0,1,RSTART-1)
          r=substr($0,RSTART+2)
          if(match(r,/\*\//)){
            $0=s substr(r,RSTART+RLENGTH)
          } else {
            $0=s
            b=1
            break
          }
        }
        print
      }' \
    | sed 's://.*::' \
    | grep -oE '"internal\.file_not_found"|"plugin\.abandoned"|"plugin\.completed"|"plugin\.timeout"' || true
}

@test "T-011 AC-008: zero bare literals in production code of read_file.rs, read_prefix.rs, emit_event.rs, executor.rs, vsdd_sink.rs" {
  local files_with_literals=""
  local failed=0

  for target in "$READ_FILE_RS" "$READ_PREFIX_RS" "$EMIT_EVENT_RS" "$EXECUTOR_RS" "$VSDD_SINK_RS"; do
    [ -f "$target" ] || {
      echo "FAIL: $target not found"
      failed=1
      continue
    }

    local actual_output
    actual_output=$(_scan_bare_literals "$target")
    if [ -n "$actual_output" ]; then
      echo "FAIL: bare literals found in production code of $target:"
      echo "  $actual_output"
      files_with_literals="$files_with_literals $target"
      failed=1
    fi
  done

  # Mutation-liveness check: inject a bare literal into a temp copy of
  # read_file.rs's production region and assert the gate fires.
  local mut_file
  mut_file="${TEST_TMP}/mutant.rs"
  # Insert the mutation line immediately before the #[cfg(test)] boundary.
  awk '{
    if (/^#\[cfg\(test\)\]/) {
      print "    let _ = \"internal.file_not_found\"; // mutation-liveness injection"
    }
    print
  }' "$READ_FILE_RS" > "$mut_file"

  local mutant_output
  mutant_output=$(_scan_bare_literals "$mut_file")
  rm -f "$mut_file"

  if [ -z "$mutant_output" ]; then
    echo "FAIL: mutation-liveness (a) — gate did NOT fire on a temp copy of read_file.rs"
    echo "  with 'internal.file_not_found' injected into the production region."
    echo "  The gate is not live and cannot detect the violation it is designed to catch."
    failed=1
  else
    echo "PASS mutation-liveness (a): gate output on mutant = '$mutant_output'"
  fi

  # Mutation-liveness check (b): verify the D22 extension classes are live.
  # Inject "plugin.completed" (one of the two new D22 classes) immediately
  # before the #[cfg(test)] boundary and assert the gate fires on it.
  local mut_file2
  mut_file2="${TEST_TMP}/mutant2.rs"
  awk '{
    if (/^#\[cfg\(test\)\]/) {
      print "    let _ = \"plugin.completed\"; // mutation-liveness injection (D22 extension)"
    }
    print
  }' "$READ_FILE_RS" > "$mut_file2"

  local mutant_output2
  mutant_output2=$(_scan_bare_literals "$mut_file2")
  rm -f "$mut_file2"

  if [ -z "$mutant_output2" ]; then
    echo "FAIL: mutation-liveness (b) — gate did NOT fire on a temp copy of read_file.rs"
    echo "  with 'plugin.completed' injected into the production region."
    echo "  The D22-class extension ('plugin.completed'/'plugin.timeout') is not live."
    failed=1
  else
    echo "PASS mutation-liveness (b): gate output on mutant = '$mutant_output2'"
  fi

  # Mutation-liveness check (c): verify executor.rs perimeter extension is live.
  # Inject "plugin.completed" before the #[cfg(test)] boundary in executor.rs
  # and assert the 4-stage scan fires — proves the widened perimeter detects
  # a future bare literal in executor.rs production code (F-P2-001).
  local mut_file3
  mut_file3="${TEST_TMP}/mutant3.rs"
  awk '{
    if (/^#\[cfg\(test\)\]/) {
      print "    let _ = \"plugin.completed\"; // mutation-liveness injection (executor.rs extension)"
    }
    print
  }' "$EXECUTOR_RS" > "$mut_file3"

  local mutant_output3
  mutant_output3=$(_scan_bare_literals "$mut_file3")
  rm -f "$mut_file3"

  if [ -z "$mutant_output3" ]; then
    echo "FAIL: mutation-liveness (c) — gate did NOT fire on a temp copy of executor.rs"
    echo "  with 'plugin.completed' injected before the #[cfg(test)] boundary."
    echo "  The executor.rs perimeter extension (F-P2-001) is not live in T-011."
    failed=1
  else
    echo "PASS mutation-liveness (c): gate output on mutant = '$mutant_output3'"
  fi

  if [ "$failed" -ne 0 ]; then
    echo "ACTION: replace bare literals with named constants (D21+D22):"
    echo "  INTERNAL_FILE_NOT_FOUND (from crate::internal_log)"
    echo "  PLUGIN_ABANDONED (from crate::internal_log)"
    echo "  PLUGIN_COMPLETED (from crate::internal_log)"
    echo "  PLUGIN_TIMEOUT (from crate::internal_log)"
    false
  fi
}

# ---------------------------------------------------------------------------
# T-012  AC-008/AC-009 bidirectional regression gate
#
# Gate: `cargo test -p factory-dispatcher --lib -- s19_09` exits 0.
#
# T-012's actual intent (per the original RED/GREEN narrative below) is
# narrower than "the entire factory-dispatcher test suite": it verifies that
# the D19 (read_prefix wired into setup_host_on_store_data, the production
# linker) and D22 (timestamp field on plugin.completed) unit tests pass, and
# that the D21 literal→named-constant sweep did not change the event-type
# string *values* those tests assert on. All six tests carrying that intent
# live as `#[cfg(test)]` unit tests inside src/ (invoke.rs, emit_event.rs) and
# share the "s19_09" name fragment:
#   t001_s19_09_read_prefix_instantiates_without_link_error_via_production_linker
#   t002_s19_09_read_prefix_round_trip_bytes_correct_and_out_ptr_nonzero_via_production_path
#   t002b_s19_09_read_prefix_head_c_bound_clamps_out_len_to_max_bytes
#   t003_s19_09_read_prefix_capability_absent_returns_capability_denied_via_production_path
#   t015_s19_09_read_prefix_empty_file_returns_ok_with_zero_ptr_len_no_grow
#   test_s19_09_t013_emit_plugin_completed_async_has_timestamp_field
#
# Narrowed from `cargo test -p factory-dispatcher --all-targets` (rc.24 flake
# fix): `--all-targets` re-runs every integration-test binary in
# crates/factory-dispatcher/tests/, including the BC-1.14.001 async e2e tests
# in full_stack_plugin_invocation.rs. Those tests spawn real debug-build WASM
# plugins and wait on wall-clock deadlines; ~2200 lines into this bats suite
# the CI runner is under enough load that those deadlines occasionally blow
# out (confirmed flake — the SAME tests pass under the `cargo-host` CI job's
# `cargo test --workspace --all-targets`, which duplicates this gate's
# coverage anyway). T-012 does not need that coverage to prove its own
# intent, and re-paying debug-WASM cold-start here only adds risk with no
# unique signal. `--lib` runs unit tests only (no tests/ integration
# binaries, no WASM-plugin-file cold start) and `-- s19_09` scopes to the
# tests the RED/GREEN narrative below actually documents.
#
# RED today: T-001/T-002/T-003 (invoke.rs production-linker) fail at runtime
# because read_prefix is absent from setup_host_on_store_data (D19 unimplemented);
# T-013 (emit_event.rs) fails because the "timestamp" field is absent from
# plugin.completed (D22 unimplemented).  T-009/T-010 are now compile-safe
# bats grep-gates and no longer cause compile errors here.
#
# GREEN after D19+D20+D21+D22: T-001..T-003 (+T-002b/T-015) pass (read_prefix
# wired), T-013 passes (timestamp field present); exits 0.
#
# Bidirectional: RED pre-fix (runtime failures in T-001/T-002/T-003/T-013);
# GREEN post-fix (the D19/D22 unit test set above passes).
# ---------------------------------------------------------------------------
@test "T-012 AC-008 regression: cargo test -p factory-dispatcher --lib -- s19_09 passes after D21 sweep" {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"

  # Select platform-compatible timeout command.
  local timeout_cmd=""
  if command -v timeout &>/dev/null; then
    timeout_cmd="timeout 120"
  elif command -v gtimeout &>/dev/null; then
    timeout_cmd="gtimeout 120"
  fi

  local output
  local exit_code=0
  cd "$REPO_ROOT"
  # shellcheck disable=SC2086
  output=$(${timeout_cmd} cargo test -p factory-dispatcher --lib -- s19_09 2>&1) \
    || exit_code=$?

  if [ "$exit_code" -ne 0 ]; then
    echo "FAIL: cargo test -p factory-dispatcher --lib -- s19_09 exited $exit_code"
    echo "--- output (last 40 lines) ---"
    echo "$output" | tail -40
    echo "---"
    echo "T-012: existing test assertions on event type strings must pass after D21 sweep."
    false
  fi

  # Sanity gate: the name filter must actually select the six documented
  # tests (not zero tests via a silently-broken filter, and not fewer than
  # expected via a partial name collision). "running N tests" is libtest's
  # own summary line.
  if ! echo "$output" | grep -qE "running 6 tests?"; then
    echo "FAIL: expected the 's19_09' filter to select exactly 6 unit tests; got:"
    echo "$output" | grep -E "running [0-9]+ tests?" || echo "(no 'running N tests' line found)"
    echo "T-012: filter scope drifted — update either the filter or this count."
    false
  fi

  # Identity gate: a count-only check ("running 6 tests") would still pass
  # if a filter typo swapped in six DIFFERENT tests that happen to also
  # match "s19_09" (e.g. a future rename collision). Assert each of the six
  # documented test names individually appears in the output, not just the
  # aggregate count.
  local expected_tests=(
    "t001_s19_09_read_prefix_instantiates_without_link_error_via_production_linker"
    "t002_s19_09_read_prefix_round_trip_bytes_correct_and_out_ptr_nonzero_via_production_path"
    "t002b_s19_09_read_prefix_head_c_bound_clamps_out_len_to_max_bytes"
    "t003_s19_09_read_prefix_capability_absent_returns_capability_denied_via_production_path"
    "t015_s19_09_read_prefix_empty_file_returns_ok_with_zero_ptr_len_no_grow"
    "test_s19_09_t013_emit_plugin_completed_async_has_timestamp_field"
  )
  local missing_tests=()
  for t in "${expected_tests[@]}"; do
    echo "$output" | grep -qF "$t" || missing_tests+=("$t")
  done
  if [ "${#missing_tests[@]}" -gt 0 ]; then
    echo "FAIL: the 's19_09' filter matched 6 tests by count, but by NAME the"
    echo "following documented test(s) are missing from the output (a wrong-test"
    echo "filter-typo scenario, not just a wrong-count scenario):"
    printf '  missing: %s\n' "${missing_tests[@]}"
    echo "--- actual test names in output ---"
    echo "$output" | grep -E "^test .* \.\.\. (ok|FAILED)" || true
    false
  fi
}
