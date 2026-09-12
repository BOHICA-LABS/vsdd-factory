#!/usr/bin/env bats
# read-prefix-wasm.bats — T-009 integration gates for hook-sdk read_prefix
# wrapper callable from a WASM plugin (S-19.06 AC-007).
#
# AC-007 requires three gates for the hook-sdk to correctly expose read_prefix:
#   Gate 1: safe Rust wrapper signature in crates/hook-sdk/src/host.rs
#   Gate 2: raw wire-ABI extern in crates/hook-sdk/src/ffi.rs (3 clauses)
#   Gate 3: read_prefix registered in the dispatcher host dispatch table
#
# This bats suite implements the subset of AC-007 gates that the existing
# bats harness supports: static file analysis via grep/awk on source files.
#
# FIXTURE WASM COMPILE/LINK (Gate 4 — integration):
#   AC-007 requires "a fixture WASM plugin that imports and calls read_prefix
#   compiles and links successfully."  T-009f is this compile/link harness:
#   it invokes `cargo build -p read-prefix-fixture --target wasm32-wasip1`
#   and asserts exit 0.  The fixture crate lives at
#   crates/hook-plugins/read-prefix-fixture/.
#
#   What remains out of scope for this bats suite: RUNTIME execution of the
#   fixture under a live dispatcher.  No wasm-execution harness exists in
#   this suite; compile/link (T-009f) is the extent of what is exercised.
#   Runtime dispatch exercise is a future integration concern beyond S-19.06.
#
# RED GATE STATUS:
#   T-009a..T-009e (static file checks): PASS at Red Gate because the stubs
#   committed at e422a30e already carry the correct signatures and registrations.
#   These tests provide regression protection post-implementation but do NOT
#   constitute a meaningful Red Gate test for the behavioral contract.
#
#   T-009f (fixture compile/link, AC-007 Gate 4): PASSES — fixture crate
#   created at crates/hook-plugins/read-prefix-fixture/ and registered in the
#   workspace.  This IS the Red Gate for the FFI boundary.
#
#   The load-bearing behavioral gate for AC-007 (host-side logic) is the unit
#   test suite in read_prefix.rs: T-001..T-008 + T-010 (original 9, Red Gate
#   at stub phase) plus T-012, T-012_MUTANT_VERIFY, T-013a, T-013b, and
#   T-013_MUTANT_VERIFY (5 cascade-remediation regression locks, written green;
#   14 tests total).  T-009f closes the compile/link gap; runtime dispatch
#   exercise is out of scope.
#
# VP Trace: VP-101
# Story: S-19.06
# BC: BC-1.17.001 v1.6 §(a) layering parenthetical + §Architecture Anchors
#     + Invariant 2

setup() {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  HOST_RS="$REPO_ROOT/crates/hook-sdk/src/host.rs"
  FFI_RS="$REPO_ROOT/crates/hook-sdk/src/ffi.rs"
  MOD_RS="$REPO_ROOT/crates/factory-dispatcher/src/host/mod.rs"
  READ_PREFIX_RS="$REPO_ROOT/crates/factory-dispatcher/src/host/read_prefix.rs"
  CI_YML="$REPO_ROOT/.github/workflows/ci.yml"
  RELEASE_YML="$REPO_ROOT/.github/workflows/release.yml"
  TEST_TMP="$(mktemp -d "${BATS_TMPDIR}/read-prefix-wasm-XXXXXX")"
}

teardown() {
  rm -rf "${TEST_TMP}"
}

# ---------------------------------------------------------------------------
# T-009a  AC-007 Gate 1 — safe wrapper signature in hook-sdk/src/host.rs
#
# BC-1.17.001 v1.6 §(a) layering parenthetical: the safe wrapper returns
# Result<Vec<u8>, HostError> (NOT -> i32).  Gate 1 asserts the FULL signature
# including return type, rejecting any -> i32 wrapper that would violate the
# layering parenthetical.
#
# Red Gate note: PASSES at Red Gate — stub committed with correct signature.
# Provides regression protection: if the signature is changed (e.g. return
# type narrowed to -> i32), this gate fires.
# ---------------------------------------------------------------------------
@test "T-009a AC-007 Gate 1: safe wrapper pub fn read_prefix signature in hook-sdk/src/host.rs" {
  [ -f "$HOST_RS" ] || {
    echo "FAIL: hook-sdk/src/host.rs not found at $HOST_RS"
    false
  }
  if ! grep -qE \
    'pub fn read_prefix\(path: &str, max_bytes: u32, timeout_ms: u32\) -> Result<Vec<u8>, HostError>' \
    "$HOST_RS"; then
    echo "FAIL: full safe wrapper signature not found in $HOST_RS"
    echo "Expected: pub fn read_prefix(path: &str, max_bytes: u32, timeout_ms: u32) -> Result<Vec<u8>, HostError>"
    echo "Searched: $HOST_RS"
    grep -n 'read_prefix' "$HOST_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009b  AC-007 Gate 2 clause (i) — raw extern 6-param shape in ffi.rs
#
# BC-1.17.001 v1.6 §(a): the raw wire-ABI extern in ffi.rs has a 6-parameter
# pointer/length shape mirroring ffi::read_file exactly.  Specifically it must
# have both path_len and out_ptr_out in its parameter list.
#
# Red Gate note: PASSES at Red Gate — stub committed with correct shape.
# ---------------------------------------------------------------------------
@test "T-009b AC-007 Gate 2(i): raw extern pub safe fn read_prefix in hook-sdk/src/ffi.rs" {
  [ -f "$FFI_RS" ] || {
    echo "FAIL: hook-sdk/src/ffi.rs not found at $FFI_RS"
    false
  }
  # Clause (i)-a: pub safe fn read_prefix( present
  if ! grep -qE 'pub safe fn read_prefix\(' "$FFI_RS"; then
    echo "FAIL: 'pub safe fn read_prefix(' not found in $FFI_RS"
    grep -n 'read_prefix' "$FFI_RS" || true
    false
  fi
  # Clause (i)-b: 6-param shape — path_len AND out_ptr_out both present in the
  # read_prefix block.  Count must be 2 (one occurrence each).
  count=$(awk '/pub safe fn read_prefix\(/,/-> i32;/' "$FFI_RS" \
    | grep -cE 'path_len|out_ptr_out' || true)
  if [ "$count" -ne 2 ]; then
    echo "FAIL: expected 2 occurrences of (path_len|out_ptr_out) in read_prefix block in ffi.rs; got $count"
    awk '/pub safe fn read_prefix\(/,/-> i32;/' "$FFI_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009c  AC-007 Gate 2 clause (ii) — vsdd import module attribute in ffi.rs
#
# The extern block must be under #[link(wasm_import_module = "vsdd")] so
# read_prefix lands in the vsdd:: namespace (not the default unnamed namespace).
#
# Red Gate note: PASSES at Red Gate — stub in the existing vsdd block.
# ---------------------------------------------------------------------------
@test "T-009c AC-007 Gate 2(ii): #[link(wasm_import_module = \"vsdd\")] attribute in ffi.rs" {
  [ -f "$FFI_RS" ] || {
    echo "FAIL: hook-sdk/src/ffi.rs not found at $FFI_RS"
    false
  }
  if ! grep -qF '#[link(wasm_import_module = "vsdd")]' "$FFI_RS"; then
    echo 'FAIL: #[link(wasm_import_module = "vsdd")] not found in ffi.rs'
    grep -n 'link\|wasm_import' "$FFI_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009d  AC-007 Gate 2 clause (iii) — read_prefix in BOTH cfg blocks
#
# BC-1.17.001 v1.6 §Architecture Anchors: read_prefix must appear in:
#   (a) the #[cfg(target_arch = "wasm32")] extern block (wasm target)
#   (b) the pub mod host_stubs block (non-wasm stub for cargo test / clippy)
#
# Both blocks are required for the SDK to compile on all targets.
#
# Red Gate note: PASSES at Red Gate — both blocks have the stub committed.
# ---------------------------------------------------------------------------
@test "T-009d AC-007 Gate 2(iii): read_prefix in wasm32 extern block AND host_stubs in ffi.rs" {
  [ -f "$FFI_RS" ] || {
    echo "FAIL: hook-sdk/src/ffi.rs not found at $FFI_RS"
    false
  }
  # Check wasm32 cfg block contains read_prefix.
  if ! awk '/^#\[cfg\(target_arch = "wasm32"\)\]/,/^}/' "$FFI_RS" \
    | grep -q 'fn read_prefix'; then
    echo 'FAIL: read_prefix not found inside #[cfg(target_arch = "wasm32")] block in ffi.rs'
    awk '/^#\[cfg\(target_arch = "wasm32"\)\]/,/^}/' "$FFI_RS" || true
    false
  fi
  # Check host_stubs block contains read_prefix.
  if ! awk '/^pub mod host_stubs \{/,/^\}/' "$FFI_RS" \
    | grep -q 'fn read_prefix'; then
    echo 'FAIL: read_prefix not found inside pub mod host_stubs block in ffi.rs'
    awk '/^pub mod host_stubs \{/,/^\}/' "$FFI_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009e  AC-007 Gate 3 — read_prefix registered in dispatcher dispatch table
#
# The dispatcher must call read_prefix::register(&mut linker) in setup_linker
# so the vsdd::read_prefix host import is available to WASM plugins at runtime.
#
# Red Gate note: PASSES at Red Gate — register() call was added to setup_linker
# in the stub commit at e422a30e.
# ---------------------------------------------------------------------------
@test "T-009e AC-007 Gate 3: read_prefix::register in factory-dispatcher/src/host/mod.rs" {
  [ -f "$MOD_RS" ] || {
    echo "FAIL: factory-dispatcher/src/host/mod.rs not found at $MOD_RS"
    false
  }
  if ! grep -qE 'read_prefix::register' "$MOD_RS"; then
    echo "FAIL: read_prefix::register not found in $MOD_RS"
    echo "Expected a call to read_prefix::register(&mut linker) in setup_linker"
    grep -n 'register' "$MOD_RS" || true
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009f  AC-007 Gate 4 — fixture WASM compile/link
#
# Gate 4 asserts that a fixture WASM plugin which imports and calls
# hook_sdk::host::read_prefix compiles and links for the wasm32-wasip1 target.
# This is the FFI boundary integration gate: it exercises the full call path
# from wasm32 extern→safe wrapper in a real WASM binary, catching link errors
# that static grep checks (T-009a..T-009e) cannot detect.
#
# FIXTURE CRATE: crates/hook-plugins/read-prefix-fixture/
#   Package name: read-prefix-fixture
#   Build target: wasm32-wasip1
#   Convention:   mirrors all other hook-plugin crates under crates/hook-plugins/
#                 (e.g., precompact-flush, regression-gate, validate-burst-log)
#   Structure (as implemented):
#     crates/hook-plugins/read-prefix-fixture/Cargo.toml
#       [package] name = "read-prefix-fixture"
#       [dependencies] vsdd-hook-sdk = { path = "../../hook-sdk" }
#       [[bin]] name = "read-prefix-fixture" path = "src/main.rs"
#     crates/hook-plugins/read-prefix-fixture/src/main.rs
#       plain fn main() WASI-command entry point (no #[hook] macro);
#       calls host::read_prefix("", 0, 0) to exercise the wasm32 extern linkage
#     Cargo.toml (workspace root): "crates/hook-plugins/read-prefix-fixture"
#       is registered in the workspace members list
#
# RED GATE STATUS: PASSES — the fixture crate exists at
#   crates/hook-plugins/read-prefix-fixture/ and is registered in the
#   workspace Cargo.toml.  `cargo build -p read-prefix-fixture
#   --target wasm32-wasip1` exits 0.
#
# TIMEOUT: 120 seconds (WASM compile of a minimal crate; network-free).
# ---------------------------------------------------------------------------
@test "T-009f AC-007 Gate 4: fixture WASM read-prefix-fixture builds for wasm32-wasip1" {
  REPO_ROOT="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
  # This test must run from the repo root so Cargo picks up the workspace.
  cd "$REPO_ROOT"

  # Select platform-compatible timeout command.
  # macOS ships without GNU coreutils `timeout`; gtimeout is the homebrew alias.
  # If neither is available (rare CI case), fall through to plain cargo — the
  # package-not-found error fires in <1 s at Red Gate so no hang risk then.
  local timeout_cmd=""
  if command -v timeout &>/dev/null; then
    timeout_cmd="timeout 120"
  elif command -v gtimeout &>/dev/null; then
    timeout_cmd="gtimeout 120"
  fi

  local output
  local exit_code=0
  # shellcheck disable=SC2086  # word-split intentional: timeout_cmd may be empty
  output=$(${timeout_cmd} cargo build -p read-prefix-fixture --target wasm32-wasip1 2>&1) \
    || exit_code=$?

  if [ "$exit_code" -ne 0 ]; then
    echo "FAIL: cargo build -p read-prefix-fixture --target wasm32-wasip1 exited $exit_code"
    echo "--- cargo output (last 30 lines) ---"
    echo "$output" | tail -30
    echo "---"
    echo "ACTION: implement the fixture crate at crates/hook-plugins/read-prefix-fixture/"
    echo "  See the T-009f comment block above for the minimum structure."
    return 1
  fi
}

# ---------------------------------------------------------------------------
# T-009g  AC-003 static gate — closes described-but-unexecuted gap (F-P1-001)
#
# AC-003 (S-19.06 v1.20) requires that OUTPUT_TOO_LARGE does NOT appear in
# non-comment production code of read_prefix.rs.  The four-stage pipeline:
#
#   Stage 1: awk '/^#\[cfg\(test\)\]/{exit} {print}'
#            Scopes output to the production region only — stops at the test-
#            module boundary, excluding T-004's assert_ne!(codes::OUTPUT_TOO_LARGE,...)
#            and its assertion message string (POSIX awk; replaces BSD-
#            incompatible sed label loop).
#
#   Stage 2: stateful awk block-comment stripper (POSIX portable)
#            Strips /* ... */ block comments spanning multiple lines.
#
#   Stage 3: sed 's://.*::'
#            Strips // line comments and //! doc comments.
#
#   Stage 4: grep -oE 'OUTPUT_TOO_LARGE'
#            Emits the literal match only; empty output → gate passes.
#
# Mutation-liveness check (TD-VSDD-059): injects `codes::OUTPUT_TOO_LARGE`
# into the production region of a temp copy and asserts the gate fires.
# This is embedded as a self-verifying fixture within the test so it is
# permanently load-bearing.
#
# Red Gate note: PASSES at Red Gate — production region contains OUTPUT_TOO_LARGE
#   only in //! doc comments and // inline comments, all stripped by stage 3.
# ---------------------------------------------------------------------------
@test "T-009g AC-003 static gate: OUTPUT_TOO_LARGE absent from non-comment production code in read_prefix.rs" {
  [ -f "$READ_PREFIX_RS" ] || {
    echo "FAIL: read_prefix.rs not found at $READ_PREFIX_RS"
    false
  }

  # --- Mutation-liveness check: inject codes::OUTPUT_TOO_LARGE into the
  # production region of a temp copy and assert the gate fires.
  # The injection appears BEFORE // so sed stage 3 does NOT strip it.
  local mut_file
  mut_file="${TEST_TMP}/mutant.rs"

  # Insert the mutation line immediately before the #[cfg(test)] boundary so
  # stage-1 awk includes it in the production region it processes.
  awk '{
    if (/^#\[cfg\(test\)\]/) {
      print "let _m = codes::OUTPUT_TOO_LARGE; // mutation-liveness injection"
    }
    print
  }' "$READ_PREFIX_RS" > "$mut_file"

  local mutant_output
  mutant_output=$(
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$mut_file" \
      | awk 'BEGIN{b=0}{if(b){if(sub(/.*\*\//,""))b=0;else next};while(match($0,/\/\*/)){s=substr($0,1,RSTART-1);r=substr($0,RSTART+2);if(match(r,/\*\//)){$0=s substr(r,RSTART+RLENGTH)}else{$0=s;b=1;break}};print}' \
      | sed 's://.*::' \
      | grep -oE 'OUTPUT_TOO_LARGE' || true
  )
  rm -f "$mut_file"

  if [ -z "$mutant_output" ]; then
    echo "FAIL: mutation-liveness check — gate did NOT fire on mutant containing"
    echo "  'let _m = codes::OUTPUT_TOO_LARGE;' in the production region."
    echo "  The gate is not live: it cannot detect the violation it is designed to catch."
    false
  fi
  echo "PASS mutation-liveness: gate output on mutant = '$mutant_output'"

  # --- Primary check: gate must produce empty output on actual read_prefix.rs ---
  local actual_output
  actual_output=$(
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$READ_PREFIX_RS" \
      | awk 'BEGIN{b=0}{if(b){if(sub(/.*\*\//,""))b=0;else next};while(match($0,/\/\*/)){s=substr($0,1,RSTART-1);r=substr($0,RSTART+2);if(match(r,/\*\//)){$0=s substr(r,RSTART+RLENGTH)}else{$0=s;b=1;break}};print}' \
      | sed 's://.*::' \
      | grep -oE 'OUTPUT_TOO_LARGE' || true
  )

  if [ -n "$actual_output" ]; then
    echo "FAIL: AC-003 violation — OUTPUT_TOO_LARGE found in non-comment production code of read_prefix.rs"
    echo "  Gate output: '$actual_output'"
    echo "  Pipeline: awk test-strip | awk block-comment-strip | sed line-comment-strip | grep OUTPUT_TOO_LARGE"
    echo "  read_prefix MUST NEVER reference OUTPUT_TOO_LARGE in non-comment production code."
    false
  fi
}

# ---------------------------------------------------------------------------
# T-009h  POLICY 20 exclusion presence-gate — closes single-point-defense gap (F-P1-002)
#
# read-prefix-fixture is a test-only fixture crate (S-19.06 AC-007 compile/link
# gate, bats T-009f).  It must NEVER become a release artifact (POLICY 20).
# Defense requires exclusion at TWO layers:
#
#   Layer 1 (--exclude): every wasm32-wasip1 --workspace cargo build must carry
#     `--exclude read-prefix-fixture` so the fixture is never compiled into the
#     release artifact bundle.
#
#   Layer 2 (staging case-skip): every staging loop that iterates over
#     target/wasm32-wasip1/*/\*.wasm must have a `read-prefix-fixture.wasm)`
#     case that continues, so a stale build artifact from a prior caching
#     layer is never promoted to the registry.
#
# Layer 1 assertions (COUPLED counts — fires on both removal AND new-build-without-exclusion):
#   The gate counts wasm32-wasip1 --workspace build invocations and asserts
#   '--exclude read-prefix-fixture' count EQUALS that build count.  This is
#   stronger than a hardcoded exact-count check: adding a new --workspace build
#   WITHOUT the exclusion causes the counts to diverge and the gate fires.
#   A workspace build is detected by: a 'cargo build.*wasm32-wasip1.*\' line
#   immediately followed by a '--workspace' line (the adjacency is reliable
#   because the workspace flag is always the first continuation flag).
#
#   (a) release.yml  --exclude count == wasm32-wasip1 --workspace build count
#   (b) ci.yml       --exclude count == wasm32-wasip1 --workspace build count
#
# Layer 2 assertions (exact counts — staging loops are independent of build count):
#   (c) ci.yml       staging case-skip count == 3
#   (d) release.yml  staging case-skip count == 2
#
# Mutation-liveness fixture (TWO directions, both verified in-test):
#   Direction 1 (deletion): ci.yml temp copy with one --exclude line deleted
#     → exclusion count drops below workspace count → coupled assertion fires.
#   Direction 2 (new-build-without-exclusion): ci.yml temp copy with a synthetic
#     'cargo build --release --target wasm32-wasip1 \ --workspace' appended
#     WITHOUT --exclude read-prefix-fixture → workspace count increases, exclusion
#     count stays same → coupled assertion fires.
#
# Red Gate note: PASSES at Red Gate — both layers present since the first
#   exclusion commit.
# ---------------------------------------------------------------------------
@test "T-009h POLICY 20 exclusion presence-gate: read-prefix-fixture excluded in all wasm32-wasip1 --workspace builds and staging loops" {
  [ -f "$CI_YML" ] || {
    echo "FAIL: ci.yml not found at $CI_YML"
    false
  }
  [ -f "$RELEASE_YML" ] || {
    echo "FAIL: release.yml not found at $RELEASE_YML"
    false
  }

  # --- Count wasm32-wasip1 --workspace build invocations ---
  # A workspace build is identified by: a 'cargo build.*wasm32-wasip1.*\' line
  # immediately followed by a line containing '--workspace'.  This adjacency is
  # the reliable anchor: every workspace build has --workspace as its first
  # continuation flag, and single-package builds (-p <crate>) do not.
  local ci_workspace_count
  ci_workspace_count=$(awk '
    prev ~ /cargo build.*wasm32-wasip1.*\\/ && /--workspace/ { count++ }
    { prev = $0 }
    END { print count+0 }
  ' "$CI_YML")

  local release_workspace_count
  release_workspace_count=$(awk '
    prev ~ /cargo build.*wasm32-wasip1.*\\/ && /--workspace/ { count++ }
    { prev = $0 }
    END { print count+0 }
  ' "$RELEASE_YML")

  # --- (a) release.yml: --exclude count must equal workspace build count ---
  local release_excl_count
  release_excl_count=$(grep -c '\-\-exclude read-prefix-fixture' "$RELEASE_YML" || true)
  if [ "$release_excl_count" -ne "$release_workspace_count" ]; then
    echo "FAIL: release.yml '--exclude read-prefix-fixture' count ($release_excl_count) != wasm32-wasip1 --workspace build count ($release_workspace_count)"
    echo "  Every wasm32-wasip1 --workspace build must carry '--exclude read-prefix-fixture' (POLICY 20)."
    echo "  Adding a new --workspace build without the exclusion causes this gate to fire."
    false
  fi

  # --- (b) ci.yml: --exclude count must equal workspace build count ---
  local ci_excl_count
  ci_excl_count=$(grep -c '\-\-exclude read-prefix-fixture' "$CI_YML" || true)
  if [ "$ci_excl_count" -ne "$ci_workspace_count" ]; then
    echo "FAIL: ci.yml '--exclude read-prefix-fixture' count ($ci_excl_count) != wasm32-wasip1 --workspace build count ($ci_workspace_count)"
    echo "  Every wasm32-wasip1 --workspace build must carry '--exclude read-prefix-fixture' (POLICY 20)."
    echo "  Adding a new --workspace build without the exclusion causes this gate to fire."
    false
  fi

  # --- (c) ci.yml: staging case-skip read-prefix-fixture.wasm) count == 3 ---
  # Staging loops are independent of workspace builds; exact count is the
  # appropriate anchor here.
  local ci_staging_count
  ci_staging_count=$(grep -c 'read-prefix-fixture\.wasm)' "$CI_YML" || true)
  if [ "$ci_staging_count" -ne 3 ]; then
    echo "FAIL: ci.yml staging case-skip 'read-prefix-fixture.wasm)' count: expected 3, got $ci_staging_count"
    echo "  All staging loops in ci.yml that iterate over wasm artifacts must skip the"
    echo "  fixture (POLICY 20). Adding a new staging loop without the case-skip fires this gate."
    false
  fi

  # --- (d) release.yml: staging case-skip read-prefix-fixture.wasm) count == 2 ---
  local release_staging_count
  release_staging_count=$(grep -c 'read-prefix-fixture\.wasm)' "$RELEASE_YML" || true)
  if [ "$release_staging_count" -ne 2 ]; then
    echo "FAIL: release.yml staging case-skip 'read-prefix-fixture.wasm)' count: expected 2, got $release_staging_count"
    echo "  All staging loops in release.yml that iterate over wasm artifacts must skip the"
    echo "  fixture (POLICY 20). Adding a new staging loop without the case-skip fires this gate."
    false
  fi

  # --- Mutation-liveness direction 1 (deletion): delete one --exclude line from ci.yml ---
  # After deletion the exclusion count drops below the workspace count, proving
  # the coupled assertion (b) would fire on any --exclude removal.
  local mut_ci_del
  mut_ci_del="${TEST_TMP}/ci-del.yml"
  awk 'seen==0 && /--exclude read-prefix-fixture/{seen=1;next}{print}' \
    "$CI_YML" > "$mut_ci_del"
  local mut_del_excl_count
  mut_del_excl_count=$(grep -c '\-\-exclude read-prefix-fixture' "$mut_ci_del" || true)
  rm -f "$mut_ci_del"
  if [ "$mut_del_excl_count" -eq "$ci_workspace_count" ]; then
    echo "FAIL: mutation-liveness (deletion) — deleting one '--exclude read-prefix-fixture' line"
    echo "  still gives excl count ($mut_del_excl_count) == workspace count ($ci_workspace_count)."
    echo "  The coupled gate would NOT fire on removal; the gate is not live."
    false
  fi
  echo "PASS mutation-liveness (deletion): after deletion excl=$mut_del_excl_count != workspace=$ci_workspace_count (gate fires)"

  # --- Mutation-liveness direction 2 (new-build-without-exclusion): append a synthetic ---
  # workspace build without --exclude read-prefix-fixture to a temp copy of ci.yml.
  # The workspace count increases by 1 but the exclusion count stays the same,
  # proving assertion (b) fires when a new workspace build lacks the exclusion.
  local mut_ci_add
  mut_ci_add="${TEST_TMP}/ci-add.yml"
  {
    cat "$CI_YML"
    printf '          cargo build --release --target wasm32-wasip1 \\\n'
    printf '            --workspace\n'
  } > "$mut_ci_add"
  local mut_add_workspace_count
  mut_add_workspace_count=$(awk '
    prev ~ /cargo build.*wasm32-wasip1.*\\/ && /--workspace/ { count++ }
    { prev = $0 }
    END { print count+0 }
  ' "$mut_ci_add")
  local mut_add_excl_count
  mut_add_excl_count=$(grep -c '\-\-exclude read-prefix-fixture' "$mut_ci_add" || true)
  rm -f "$mut_ci_add"
  if [ "$mut_add_excl_count" -eq "$mut_add_workspace_count" ]; then
    echo "FAIL: mutation-liveness (new-build-without-exclusion) — appending a workspace build"
    echo "  without '--exclude read-prefix-fixture' still gives excl ($mut_add_excl_count) == workspace ($mut_add_workspace_count)."
    echo "  The coupled gate would NOT fire; adding a build without exclusion goes undetected."
    false
  fi
  echo "PASS mutation-liveness (new-build-without-exclusion): workspace=$mut_add_workspace_count, excl=$mut_add_excl_count (diverge; gate fires)"

  echo "PASS T-009h: release.yml excl=$release_excl_count==workspace=$release_workspace_count; ci.yml excl=$ci_excl_count==workspace=$ci_workspace_count; ci.yml staging=$ci_staging_count; release.yml staging=$release_staging_count"
}

# ---------------------------------------------------------------------------
# T-009 status summary (post T-009g/T-009h addition)
#
# T-009a..T-009e (static file checks): PASS at Red Gate — stubs at e422a30e
#   already carry the correct signatures and registrations.  These tests are
#   regression guards; they do NOT constitute a meaningful Red Gate.
#
# T-009f (fixture compile/link): PASSES — the fixture crate exists at
#   crates/hook-plugins/read-prefix-fixture/ and is registered in the
#   workspace Cargo.toml.  This IS the Red Gate for AC-007 Gate 4 (FFI
#   boundary compile/link verification).
#
# T-009g (AC-003 static gate): PASSES at Red Gate — production region contains
#   OUTPUT_TOO_LARGE only in // and //! comments, all stripped by stage 3 sed.
#   Includes a self-verifying mutation-liveness fixture (injects bare
#   codes::OUTPUT_TOO_LARGE into the production region of a temp copy and
#   asserts the gate fires).  Closes F-P1-001 described-but-unexecuted gap.
#
# T-009h (POLICY 20 exclusion presence-gate): PASSES at Red Gate — both
#   --exclude flags and staging case-skips are present in ci.yml and
#   release.yml.  Coupled-count assertions fire on both removal and un-swept
#   new-build additions.  Includes a self-verifying mutation-liveness check in
#   two directions: (1) deletes one --exclude from a temp ci.yml copy and
#   asserts the exclusion count no longer equals the workspace build count;
#   (2) appends a workspace build without --exclude and asserts the counts
#   diverge.  Closes F-P1-002 single-point-defense gap.
#
# The load-bearing behavioral gate for AC-007 overall is the unit test suite
# in read_prefix.rs (14 tests: T-001..T-008, T-010 (original 9, Red Gate at
# stub phase) + T-012, T-012_MUTANT_VERIFY, T-013a, T-013b, T-013_MUTANT_VERIFY
# (5 cascade-remediation regression locks, written green)), which tests the
# host-side logic.  T-009f closes the FFI-boundary gap: it proves the wasm32
# extern block compiles and links against the real hook-sdk.
# ---------------------------------------------------------------------------
