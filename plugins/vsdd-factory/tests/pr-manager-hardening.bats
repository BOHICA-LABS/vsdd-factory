#!/usr/bin/env bats
# pr-manager-hardening.bats — S-19.01 integration tests.
#
# Covers AC-001..AC-004 per BC-5.42.001 / VP-094 / ADR-030 §Decision 2 / §Decision 3.
#
# Test plan (T-001..T-033):
#   T-001 AC-001 — READY verdict without covered_sha triggers READY_SHA_MISSING advisory
#   T-002 AC-001 — gh failure on covered_sha fetch → READY_SHA_FETCH_FAILED on stderr
#   T-003 AC-002 — check-stale-verdict.sh: stale SHA → exit 1 + STALE_READY_VERDICT
#   T-004 AC-002 — check-stale-verdict.sh: matching SHA → exit 0 (fresh verdict)
#   T-005 AC-003 — enforce-merge-strategy.sh: release/v* + --squash → exit 1 + RELEASE_PR_SQUASH_FORBIDDEN
#   T-006 AC-003 — enforce-merge-strategy.sh: release/v* + --merge → exit 0 (allowed)
#   T-007 AC-003 — enforce-merge-strategy.sh: non-release + --squash → exit 0 (delegated)
#   T-008 AC-004 — darwin-leg preflight: wrong interpreter exits 1 + DARWIN_LEG_WRONG_INTERPRETER
#   T-009 AC-004 — darwin-leg preflight: Linux runners skip gracefully (exit 0)
#   T-010 AC-002 — check-stale-verdict.sh: malformed covered_sha → READY_SHA_MISSING (TV4)
#   T-011 AC-003 — enforce-merge-strategy.sh: release/v* + --rebase → RELEASE_PR_SQUASH_FORBIDDEN (TV3)
#   T-012 AC-003 — enforce-merge-strategy.sh: release/v* + no flag → defaults --merge (EC-005/TV5)
#   T-013 AC-002 — check-stale-verdict.sh: STALE_READY_VERDICT exact canonical format (BC-5.42.001 §b)
#   T-014 AC-001 — check-stale-verdict.sh: READY_SHA_FETCH_FAILED exact canonical format (EC-001)
#   T-015 AC-003 — enforce-merge-strategy.sh: RELEASE_PR_SQUASH_FORBIDDEN exact canonical format (§c)
#   T-016 AC-002 — check-stale-verdict.sh: 40-char uppercase-hex → READY_SHA_MISSING (EC-002)
#   T-017 AC-004 — darwin-leg fragment: while IFS= read -r bash-3.2 compat (regression pin rc.22)
#   T-018 AC-002 — check-stale-verdict.sh: closed PR (matching SHA) → CHECK_STALE_VERDICT_ERROR (EC-003)
#   T-019 AC-002 — check-stale-verdict.sh: malformed gh JSON → CHECK_STALE_VERDICT_ERROR (arm 4)
#   T-020 AC-002 — check-stale-verdict.sh: headRefOid null value → CHECK_STALE_VERDICT_ERROR (arm-4 bypass)
#   T-021 AC-003 — enforce-merge-strategy.sh: headRefName null value → fail-open delegate (Decision 3)
#   T-022 AC-003 — pr-manager.md Step 8 must route through wrappers not direct gh pr merge (F-P7-001 wiring)
#   T-023 AC-003 — enforce-merge-strategy.sh forwards --delete-branch residual arg to gh (F-P7-001 pass-through)
#   T-024 AC-003 — enforce-merge-strategy.sh: --merge --squash (two strategies) → exit 1 (F-P7-001/F-P8-002)
#   T-025 AC-003 — enforce-merge-strategy.sh: --merge --admin → exit 1 (F-P7-001/F-P8-002 deny-list)
#   T-026 AC-003 — enforce-merge-strategy.sh: --merge -sd (combined short, s=squash) → exit 1 (F-P7-001/F-P8-002)
#   T-027 AC-003 — enforce-merge-strategy.sh: --merge --delete-branch allowed + forwarded (F-P7-001 positive)
#   T-028 AC-003 — enforce-merge-strategy.sh: release + --merge --delete-branch → delegates both (F-P7-001)
#   T-029 AC-003 — enforce-merge-strategy.sh: --admin as $2 (primary strategy slot) → exit 1 (F-P8-003)
#   T-030 AC-003 — enforce-merge-strategy.sh: -A as $2 (short form of --admin) → exit 1 (F-P8-003)
#   T-031 AC-003 — enforce-merge-strategy.sh: --merge/$2 still works (F-P8-003 positive regression guard)
#   T-032 AC-001 — pr-manager.md Step 8-pre-A must not contain re-fetch-covered_sha fallback (F-S1901-P12-001)
#   T-033 AC-002 — check-stale-verdict.sh: matching SHA + null state → fail-closed (F-P15-001)
#
# Green status: T-001..T-016 pass after implementation; T-017 green (positive + neg-control);
#   T-018/T-019 GREEN — positive verification of ADR-030 §Decision 2 arms 3+4 (post-implementation);
#   T-020/T-021 GREEN (F-P5-001 fixed); T-031 GREEN (positive regression guard);
#   T-022..T-030 RED gates (F-P7-001/F-P8-001/F-P8-002/F-P8-003); T-032 RED gate (F-S1901-P12-001);
#   T-033 RED gate (F-P15-001: null state + SHA match must fail-closed).
#   Note: F-P8-001 RED gates are Rust cargo tests (lib.rs), not bats.
#
# BC trace: BC-5.42.001 PC-1 (T-001/T-002/T-014/T-032), PC-2 (T-003/T-004/T-010/T-013/T-016/T-018/T-019/T-020/T-033),
#           PC-3 (T-005/T-006/T-007/T-011/T-012/T-015/T-021/T-022/T-023/T-024/T-025/T-026/T-027/T-028/T-029/T-030/T-031),
#           AC-004 (T-008/T-009/T-017)

PLUGIN_ROOT=""
BIN_DIR=""
FIXTURES_DIR=""
MOCK_BIN=""

# Darwin-leg suite preflight (AC-004 mechanism gate, F-S1901-P1-003).
# On macOS: verifies /bin/bash --version contains "version 3.2" before any test runs.
# On Linux: no-op (darwin-leg tests individually skip via 'skip' bats directive).
# Exit 1 here aborts the whole file — intentional for the bats-darwin-leg-macos CI job.
#
# Also builds pr-manager-completion-guard.wasm on first use if absent (mirrors the
# S-19.03 on-demand build pattern from warn-pending-wave-gate.bats commit 5c3dbae9).
# In CI the WASM is not tracked in the repo; in local dev it may be absent after
# git rm --cached. The build step requires the wasm32-wasip1 target to be installed.
setup_file() {
    local repo_root
    repo_root="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"
    local wasm_dir="${repo_root}/plugins/vsdd-factory/hook-plugins"
    local wasm="${wasm_dir}/pr-manager-completion-guard.wasm"
    if [ ! -f "$wasm" ]; then
        echo "# setup_file: pr-manager-completion-guard.wasm absent; building..." >&3
        cargo build --release --target wasm32-wasip1 -p pr-manager-completion-guard 2>&1 | tail -5 >&2
        mkdir -p "$wasm_dir"
        cp "${repo_root}/target/wasm32-wasip1/release/pr-manager-completion-guard.wasm" "$wasm"
    fi
    # Dispatcher provenance: record which binary T-001 and similar tests will use.
    # D-693 / F-S2107-P6-017: auditable path + sha256 + mtime via TAP comments.
    load "${BATS_TEST_DIRNAME}/helpers/dispatcher-provenance.bash"
    emit_dispatcher_provenance
    # Darwin-leg /bin/bash 3.2 preflight — macOS only.
    if [[ "$(uname)" != "Darwin" ]]; then
        return 0
    fi
    local bash_version
    bash_version="$(/bin/bash --version 2>/dev/null | head -1)"
    if ! printf '%s' "${bash_version}" | grep -q 'version 3\.2'; then
        printf 'DARWIN_LEG_WRONG_INTERPRETER: expected /bin/bash 3.2.x, got %s\n' \
            "${bash_version}" >&2
        return 1
    fi
}

setup() {
    PLUGIN_ROOT="$(cd "${BATS_TEST_DIRNAME}/.." && pwd)"
    BIN_DIR="${PLUGIN_ROOT}/bin"
    FIXTURES_DIR="${BATS_TEST_DIRNAME}/fixtures/pr-manager"
    MOCK_BIN="$(mktemp -d "${BATS_TMPDIR}/mock-bin-XXXXXX")"

    # Locate factory-dispatcher binary (needed for T-001 WASM hook test)
    REPO_ROOT="$(cd "${PLUGIN_ROOT}/../.." && pwd)"
    DISPATCHER="${REPO_ROOT}/target/debug/factory-dispatcher"
    if [[ ! -x "${DISPATCHER}" ]]; then
        DISPATCHER="${REPO_ROOT}/target/release/factory-dispatcher"
    fi
    export PLUGIN_ROOT BIN_DIR FIXTURES_DIR MOCK_BIN REPO_ROOT DISPATCHER
}

teardown() {
    rm -rf "${MOCK_BIN}"
}

# ─────────────────────────────────────────────────────────────────────────────
# AC-001: covered_sha enforcement (BC-5.42.001 PC-1, Invariant 1, Invariant 5)
# ─────────────────────────────────────────────────────────────────────────────

# T-001: READY verdict without covered_sha must trigger READY_SHA_MISSING advisory
# from the pr-manager-completion-guard SubagentStop WASM hook.
#
# Red Gate: WASM hook does not yet inspect READY verdict for covered_sha field.
# After implementation: dispatcher emits READY_SHA_MISSING in VSDD_SINK_FILE or
# in stdout (advisory-block-mode canonical pattern: println! stdout JSON).
@test "T-001: READY verdict without covered_sha triggers READY_SHA_MISSING advisory" {
    if [[ ! -x "${DISPATCHER}" ]]; then
        skip "factory-dispatcher binary not found at ${DISPATCHER}; run cargo build first"
    fi

    local sink_file
    sink_file="${MOCK_BIN}/sink.jsonl"

    # SubagentStop payload: pr-manager agent emits READY verdict WITHOUT covered_sha field
    local payload
    payload=$(printf '%s' '{"event_name":"SubagentStop","session_id":"T001","dispatcher_trace_id":"T001-trace","agent_type":"vsdd-factory:pr-manager","last_assistant_message":"READY: PR #42 has been reviewed. All checks pass. Steps 1-9 complete. STEP_COMPLETE: step=1 name=populate status=ok note= STEP_COMPLETE: step=2 name=demo status=ok note= STEP_COMPLETE: step=3 name=create-pr status=ok note= STEP_COMPLETE: step=4 name=security status=ok note= STEP_COMPLETE: step=5 name=review status=ok note= STEP_COMPLETE: step=6 name=checks status=ok note= STEP_COMPLETE: step=7 name=deps status=ok note= STEP_COMPLETE: step=8 name=merge status=ok note= STEP_COMPLETE: step=9 name=cleanup status=ok note="}')

    run bash -c "cd '${PLUGIN_ROOT}' && printf '%s' '${payload}' | VSDD_SINK_FILE='${sink_file}' CLAUDE_PLUGIN_ROOT='${PLUGIN_ROOT}' '${DISPATCHER}'"

    # Assertion: READY_SHA_MISSING must appear in sink events OR dispatcher output.
    # (BC-5.42.001 PC-1; ADR-030 §Decision 1 advisory-block-mode)
    # The WASM plugin emits via both paths:
    #   (1) host::emit_event → VSDD_SINK_FILE (captured by newer dispatcher builds)
    #   (2) println! → stdout JSON {"outcome":"block","reason":"READY_SHA_MISSING"}
    #       (canonical advisory-block-mode pattern; always present in ${output})
    # On macOS CI using a pre-built bundled binary, the sink file may be empty while
    # the stdout JSON path reliably captures the advisory. Accept either evidence.
    if ! grep -q "READY_SHA_MISSING" "${sink_file}" 2>/dev/null && \
       ! printf '%s' "${output}" | grep -q "READY_SHA_MISSING"; then
        echo "FAIL: READY_SHA_MISSING not found in sink events or dispatcher output"
        echo "Sink contents:"
        cat "${sink_file}" 2>/dev/null || echo "(empty or missing)"
        echo "Dispatcher output: ${output}"
        return 1
    fi

    rm -f "${sink_file}"
}

# T-002: gh failure during covered_sha fetch → check-stale-verdict.sh exits non-zero
# with READY_SHA_FETCH_FAILED on stderr (BC-5.42.001 EC-001 verbatim wording).
#
# Red Gate: check-stale-verdict.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 1 with READY_SHA_FETCH_FAILED on stderr.
@test "T-002: gh failure → READY_SHA_FETCH_FAILED on stderr (BC-5.42.001 EC-001)" {
    local pr_number="42"
    local covered_sha="aaaa000000000000000000000000000000000000"

    # Mock gh: exits non-zero to simulate network/auth failure
    cat > "${MOCK_BIN}/gh" <<'GHEOF'
#!/usr/bin/env bash
printf 'Error: HTTP 503: server temporarily unavailable\n' >&2
exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit on gh failure; got exit ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "READY_SHA_FETCH_FAILED" || {
        echo "FAIL: READY_SHA_FETCH_FAILED not found in output"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "PR #${pr_number}" || {
        echo "FAIL: PR number not included in READY_SHA_FETCH_FAILED message"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# AC-002: stale-verdict detection (BC-5.42.001 PC-2, Invariant 2)
# ─────────────────────────────────────────────────────────────────────────────

# T-003: check-stale-verdict.sh exits 1 with STALE_READY_VERDICT when
# live headRefOid differs from covered_sha.
#
# Red Gate: check-stale-verdict.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 1 with STALE_READY_VERDICT diagnostic on stderr.
@test "T-003: check-stale-verdict.sh: stale SHA → exit 1 + STALE_READY_VERDICT" {
    local pr_number="42"
    local covered_sha="aaaa000000000000000000000000000000000000"
    local live_sha="bbbb111111111111111111111111111111111111"

    # Mock gh: returns a DIFFERENT live SHA (covered_sha != headRefOid)
    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefOid"* ]]; then
    printf '{"headRefOid":"${live_sha}"}\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 (fail-closed) on stale verdict; got ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "STALE_READY_VERDICT" || {
        echo "FAIL: STALE_READY_VERDICT sentinel not present in output"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "PR #${pr_number}" || {
        echo "FAIL: PR number not in STALE_READY_VERDICT message"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "${live_sha}" || {
        echo "FAIL: live SHA not in STALE_READY_VERDICT message"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "${covered_sha}" || {
        echo "FAIL: covered_sha not in STALE_READY_VERDICT message"
        echo "Output: ${output}"
        return 1
    }
}

# T-004: check-stale-verdict.sh exits 0 when covered_sha matches live headRefOid.
# Positive control: fresh verdict; safe to proceed with merge.
#
# Red Gate: check-stale-verdict.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 0 (silent success).
@test "T-004: check-stale-verdict.sh: matching SHA → exit 0 (fresh verdict)" {
    local pr_number="42"
    local sha="cccc222222222222222222222222222222222222"

    # Mock gh: returns THE SAME SHA as covered_sha (fresh verdict)
    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefOid"* ]]; then
    printf '{"headRefOid":"${sha}"}\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${sha}"

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 on matching SHA; got ${status}"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# AC-003: merge-strategy enforcement (BC-5.42.001 PC-3, Invariant 3, Invariant 4)
# ─────────────────────────────────────────────────────────────────────────────

# T-005: enforce-merge-strategy.sh exits 1 with RELEASE_PR_SQUASH_FORBIDDEN
# when --squash is requested on a release/v* branch (no gh API call made).
#
# Red Gate: enforce-merge-strategy.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 1 before any gh pr merge invocation.
@test "T-005: enforce-merge-strategy.sh: release/v* + --squash → exit 1 + RELEASE_PR_SQUASH_FORBIDDEN" {
    local pr_number="10"
    local release_branch="release/v1.0.0-rc.23"

    # Mock gh: returns release branch name; gh pr merge must NOT be called
    local gh_merge_called_file
    gh_merge_called_file="$(mktemp "${BATS_TMPDIR}/gh-merge-called-XXXXXX")"
    rm -f "${gh_merge_called_file}"

    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${release_branch}"}\n'
    exit 0
fi
if [[ "\$1" == "pr" && "\$2" == "merge" ]]; then
    touch "${gh_merge_called_file}"
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" --squash 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for --squash on release branch; got ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "RELEASE_PR_SQUASH_FORBIDDEN" || {
        echo "FAIL: RELEASE_PR_SQUASH_FORBIDDEN message absent"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "${release_branch}" || {
        echo "FAIL: branch name not in RELEASE_PR_SQUASH_FORBIDDEN message"
        echo "Output: ${output}"
        return 1
    }
    # Verify gh pr merge was NOT called (BC-5.42.001 Invariant 3: no gh call before exit)
    [[ ! -f "${gh_merge_called_file}" ]] || {
        echo "FAIL: gh pr merge was called despite RELEASE_PR_SQUASH_FORBIDDEN (no API call permitted)"
        return 1
    }

    rm -f "${gh_merge_called_file}"
}

# T-006: enforce-merge-strategy.sh exits 0 when --merge is used on a release/v* branch.
# Positive control: --merge is the required strategy; no blocking.
#
# Red Gate: enforce-merge-strategy.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 0 (merge proceeds).
@test "T-006: enforce-merge-strategy.sh: release/v* + --merge → exit 0 (allowed)" {
    local pr_number="10"
    local release_branch="release/v1.0.0-rc.23"

    # Mock gh: returns release branch; gh pr merge exits 0
    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${release_branch}"}\n'
    exit 0
fi
if [[ "\$1" == "pr" && "\$2" == "merge" ]]; then exit 0; fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" --merge

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for --merge on release branch; got ${status}"
        echo "Output: ${output}"
        return 1
    }
}

# T-007: enforce-merge-strategy.sh delegates --squash unchanged on non-release branches.
# Non-release branches pass the caller-supplied flag through (BC-5.42.001 Invariant 4).
#
# Red Gate: enforce-merge-strategy.sh stub exits 99 with UNIMPLEMENTED.
# After implementation: exits 0 (delegates to gh pr merge --squash).
@test "T-007: enforce-merge-strategy.sh: non-release + --squash → exit 0 (delegated)" {
    local pr_number="10"
    local feature_branch="feature/S-19.01"

    # Mock gh: returns non-release branch; --squash passes through unchanged
    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${feature_branch}"}\n'
    exit 0
fi
if [[ "\$1" == "pr" && "\$2" == "merge" ]]; then exit 0; fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" --squash

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for non-release branch with --squash; got ${status}"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# AC-004: darwin-leg shell-dialect simulation discipline (S-19.01 D-g note)
# ─────────────────────────────────────────────────────────────────────────────

# T-008: darwin-leg preflight exits 1 + DARWIN_LEG_WRONG_INTERPRETER when
# /bin/bash does not report "version 3.2" (e.g., Homebrew bash 5.x).
#
# Gate (AC-004): preflight assertion in setup_file exits 1 on wrong interpreter.
# Literal CI-config gate: grep -qE '^  bats-darwin-leg-macos:$' .github/workflows/ci.yml
#   exits 0 after the job is added (O-P15-04; anchored exact-YAML-key form).
#
# Red Gate: darwin-leg-preflight.sh stub exits 99 with UNIMPLEMENTED;
#   CI-config gate will be added in implementation step.
# After implementation: exits 1 + DARWIN_LEG_WRONG_INTERPRETER when wrong interpreter.
@test "T-008: darwin-leg preflight: wrong interpreter exits 1 + DARWIN_LEG_WRONG_INTERPRETER" {
    if [[ "$(uname)" != "Darwin" ]]; then
        skip "T-008 is macOS-only: darwin-leg-preflight.sh exits 0 on non-Darwin (platform guard at line 23); wrong-interpreter path requires Darwin semantics"
    fi
    # PREFLIGHT_BASH_BIN seam (commit 9e0a5453): production script uses
    #   PREFLIGHT_BASH_BIN="${PREFLIGHT_BASH_BIN:-/bin/bash}"
    # Point it at stub-bash-version.sh which reports bash 5.1.x by default (wrong).
    # No PATH manipulation needed; no recursion risk — absolute-path seam injection.
    #
    # POLICY 11: drives darwin-leg-preflight.sh (production script) via the seam;
    # seam was explicitly added for testability (not a test-only workaround).
    run env PREFLIGHT_BASH_BIN="${FIXTURES_DIR}/stub-bash-version.sh" \
        bash "${FIXTURES_DIR}/darwin-leg-preflight.sh" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: darwin-leg preflight should exit non-zero when stub reports bash 5.1"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "DARWIN_LEG_WRONG_INTERPRETER" || {
        echo "FAIL: DARWIN_LEG_WRONG_INTERPRETER not found in output"
        echo "Output: ${output}"
        return 1
    }

    # Literal CI-config gate (O-P15-04, AC-004 drift-proof anchor):
    # bats-darwin-leg-macos job must be present in ci.yml.
    grep -qE '^  bats-darwin-leg-macos:$' "${REPO_ROOT}/.github/workflows/ci.yml" || {
        echo "FAIL (AC-004 CI-config gate): bats-darwin-leg-macos job not found in ci.yml"
        echo "Expected: job key '  bats-darwin-leg-macos:' (two-space indented, exact)"
        return 1
    }
}

# T-009: darwin-leg preflight: on Linux runners, darwin-leg tests skip gracefully.
# The bats-darwin-leg-macos CI job does not run on Linux; the preflight exits 0
# on non-Darwin platforms (EC-003).
#
# Red Gate: darwin-leg-preflight.sh stub exits 99 with UNIMPLEMENTED;
#   on macOS this test is skipped (darwin-leg is a macOS-only concern).
# After implementation: exits 0 (graceful skip) on non-Darwin platforms.
@test "T-009: darwin-leg preflight: Linux runners skip gracefully (exit 0)" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "T-009 is a Linux-runner test; darwin-leg runs on macOS, not Linux (EC-003)"
    fi

    # On Linux: darwin-leg preflight should exit 0 (graceful platform skip)
    run bash "${FIXTURES_DIR}/darwin-leg-preflight.sh" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: darwin-leg preflight should exit 0 on Linux (graceful skip); got ${status}"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# Missing BC Canonical Test Vectors + exact-text sentinel assertions (gap fill)
# Sources: BC-5.42.001 §Canonical Test Vectors TV3/TV4/TV5; §Description b/c;
#          EC-002/EC-005; VP-094 §Postcondition B/C; ADR-030 §Decision 2/3.
# All tests below FAIL at Red Gate (stubs exit 99 UNIMPLEMENTED / todo!() panics).
# ─────────────────────────────────────────────────────────────────────────────

# T-010: check-stale-verdict.sh: malformed covered_sha (< 40 chars, non-hex)
# → exit 1 + READY_SHA_MISSING: covered_sha is malformed
# (BC-5.42.001 §Canonical Test Vectors TV4: covered_sha=ZZZZZZ invalid hex, <40 chars)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exits 1 with READY_SHA_MISSING: covered_sha is malformed on stderr.
@test "T-010: check-stale-verdict.sh: malformed covered_sha → exit 1 + READY_SHA_MISSING" {
    local pr_number="42"
    # BC-5.42.001 §Canonical Test Vectors TV4: non-hex, too short — same as ZZZZZZ in the spec
    local malformed_sha="ZZZZZZ"

    # gh must NOT be called — malformed SHA is rejected before any network I/O.
    cat > "${MOCK_BIN}/gh" <<'GHEOF'
#!/usr/bin/env bash
printf 'MOCK_GH_SHOULD_NOT_BE_CALLED_FOR_MALFORMED_SHA\n' >&2
exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${malformed_sha}" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 on malformed covered_sha (TV4); got ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "READY_SHA_MISSING" || {
        echo "FAIL: READY_SHA_MISSING sentinel not found for malformed covered_sha (BC-5.42.001 TV4)"
        echo "Expected: READY_SHA_MISSING: covered_sha is malformed"
        echo "Output: ${output}"
        return 1
    }
}

# T-011: enforce-merge-strategy.sh: release/v* + --rebase → exit 1 + RELEASE_PR_SQUASH_FORBIDDEN
# (BC-5.42.001 §Canonical Test Vectors TV3: --rebase is forbidden on release/v* same as --squash)
# (BC-5.42.001 Invariant 3: release-branch squash/rebase is mechanically impossible via wrapper)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exits 1 with RELEASE_PR_SQUASH_FORBIDDEN; no gh pr merge called.
@test "T-011: enforce-merge-strategy.sh: release/v* + --rebase → exit 1 + RELEASE_PR_SQUASH_FORBIDDEN" {
    local pr_number="10"
    local release_branch="release/v1.0.0-rc.23"
    local gh_merge_called_file
    gh_merge_called_file="$(mktemp "${BATS_TMPDIR}/gh-merge-T011-XXXXXX")"
    rm -f "${gh_merge_called_file}"

    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${release_branch}"}\n'
    exit 0
fi
if [[ "\$1" == "pr" && "\$2" == "merge" ]]; then
    touch "${gh_merge_called_file}"
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" --rebase 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for --rebase on release branch (TV3); got ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "RELEASE_PR_SQUASH_FORBIDDEN" || {
        echo "FAIL: RELEASE_PR_SQUASH_FORBIDDEN message absent for --rebase on release branch"
        echo "Output: ${output}"
        return 1
    }
    [[ ! -f "${gh_merge_called_file}" ]] || {
        echo "FAIL: gh pr merge was called despite RELEASE_PR_SQUASH_FORBIDDEN (BC-5.42.001 Invariant 3: no API call permitted)"
        return 1
    }
    rm -f "${gh_merge_called_file}"
}

# T-012: enforce-merge-strategy.sh: release/v* + no merge flag → defaults to --merge, exit 0
# (BC-5.42.001 EC-005: missing flag on release branch → script injects --merge)
# (BC-5.42.001 §Canonical Test Vectors TV5: no flag supplied → gh pr merge <n> --merge)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exits 0; gh pr merge receives --merge flag even though caller supplied none.
@test "T-012: enforce-merge-strategy.sh: release/v* + no flag → defaults to --merge (EC-005)" {
    local pr_number="10"
    local release_branch="release/v1.0.0-rc.23"
    local merge_flag_file
    merge_flag_file="${MOCK_BIN}/merge-flag.txt"
    rm -f "${merge_flag_file}"

    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${release_branch}"}\n'
    exit 0
fi
if [[ "\$1" == "pr" && "\$2" == "merge" ]]; then
    # Record all positional args passed to gh pr merge to verify --merge was injected
    printf '%s\n' "\$@" > "${merge_flag_file}"
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}"

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for release branch with no explicit flag (EC-005); got ${status}"
        echo "Output: ${output}"
        return 1
    }
    # EC-005: --merge must be injected even when caller supplied no merge-strategy flag
    { [[ -f "${merge_flag_file}" ]] && grep -q -- "--merge" "${merge_flag_file}"; } || {
        echo "FAIL: --merge not injected for release branch with no flag (BC-5.42.001 EC-005 / TV5)"
        echo "gh pr merge args recorded: $(cat "${merge_flag_file}" 2>/dev/null || echo '(gh not called)')"
        return 1
    }
    rm -f "${merge_flag_file}"
}

# T-013: check-stale-verdict.sh: STALE_READY_VERDICT exact canonical message format
# BC-5.42.001 §Description b canonical form:
# "STALE_READY_VERDICT: PR #<n> HEAD <current_sha> != covered_sha <covered_sha>"
# (VP-094 §Postcondition B; ADR-030 §Decision 2 stderr routing)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exact canonical diagnostic appears on stderr.
@test "T-013: check-stale-verdict.sh: STALE_READY_VERDICT exact canonical message format" {
    local pr_number="42"
    local covered_sha="aaaa000000000000000000000000000000000000"
    local live_sha="bbbb111111111111111111111111111111111111"

    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefOid"* ]]; then
    printf '{"headRefOid":"${live_sha}"}\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit for stale verdict; got ${status}"
        return 1
    }
    # Exact canonical format (BC-5.42.001 §Description b + TV2 verbatim connector text):
    echo "${output}" | grep -q "STALE_READY_VERDICT: PR #${pr_number} HEAD ${live_sha} != covered_sha ${covered_sha}" || {
        echo "FAIL: exact canonical STALE_READY_VERDICT format not found in stderr"
        printf 'Expected substring: STALE_READY_VERDICT: PR #%s HEAD %s != covered_sha %s\n' \
            "${pr_number}" "${live_sha}" "${covered_sha}"
        echo "Output: ${output}"
        return 1
    }
}

# T-014: check-stale-verdict.sh: READY_SHA_FETCH_FAILED exact canonical message format
# BC-5.42.001 EC-001 verbatim wording:
# "READY_SHA_FETCH_FAILED: gh pr view failed for PR #<pr_number>"
# (S-19.01 AC-001 Gate; v1.9 F-P11-001 corrected literal wording)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exact BC-5.42.001 EC-001 wording on stderr.
@test "T-014: check-stale-verdict.sh: READY_SHA_FETCH_FAILED exact canonical message format (EC-001)" {
    local pr_number="42"
    local covered_sha="aaaa000000000000000000000000000000000000"

    # Mock gh: exits non-zero to simulate network/auth failure (EC-001 trigger)
    cat > "${MOCK_BIN}/gh" <<'GHEOF'
#!/usr/bin/env bash
printf 'Error: HTTP 503: server temporarily unavailable\n' >&2
exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit on gh failure (EC-001); got ${status}"
        return 1
    }
    # Exact BC-5.42.001 EC-001 wording (§Canonical Test Vectors TV3 verbatim):
    echo "${output}" | grep -q "READY_SHA_FETCH_FAILED: gh pr view failed for PR #${pr_number}" || {
        echo "FAIL: exact canonical READY_SHA_FETCH_FAILED format not found in stderr"
        printf 'Expected substring: READY_SHA_FETCH_FAILED: gh pr view failed for PR #%s\n' \
            "${pr_number}"
        echo "Output: ${output}"
        return 1
    }
}

# T-015: enforce-merge-strategy.sh: RELEASE_PR_SQUASH_FORBIDDEN exact canonical message format
# BC-5.42.001 §Description c canonical form:
# "RELEASE_PR_SQUASH_FORBIDDEN: branch <name> requires --merge per RELEASING.md"
# (VP-094 §Postcondition C; ADR-030 §Decision 3 verbatim; "per RELEASING.md" anchor required)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exact canonical diagnostic on stderr including "per RELEASING.md" anchor.
@test "T-015: enforce-merge-strategy.sh: RELEASE_PR_SQUASH_FORBIDDEN exact canonical message format" {
    local pr_number="10"
    local release_branch="release/v1.0.0-rc.23"

    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" && "\$*" == *"headRefName"* ]]; then
    printf '{"headRefName":"${release_branch}"}\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" --squash 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for --squash on release branch; got ${status}"
        echo "Output: ${output}"
        return 1
    }
    # Exact canonical format (BC-5.42.001 §Description c + VP-094 §Postcondition C verbatim);
    # "per RELEASING.md" is a mandatory anchor per ADR-030 §Decision 3.
    echo "${output}" | grep -q "RELEASE_PR_SQUASH_FORBIDDEN: branch ${release_branch} requires --merge per RELEASING.md" || {
        echo "FAIL: exact canonical RELEASE_PR_SQUASH_FORBIDDEN format not found in stderr"
        printf 'Expected substring: RELEASE_PR_SQUASH_FORBIDDEN: branch %s requires --merge per RELEASING.md\n' \
            "${release_branch}"
        echo "Output: ${output}"
        return 1
    }
}

# T-016: check-stale-verdict.sh: 40-char uppercase-hex covered_sha → READY_SHA_MISSING (EC-002)
# BC-5.42.001 Invariant 5: exactly 40 LOWERCASE hex characters required.
# EC-002: 40-char value with non-lowercase hex chars is treated the same as absent.
# (pr-manager-completion-guard READY_SHA_MISSING same code; check-stale-verdict.sh rejects too)
#
# Red Gate: stub exits 99 UNIMPLEMENTED.
# After implementation: exits 1 with READY_SHA_MISSING on stderr (Invariant 5 / EC-002).
@test "T-016: check-stale-verdict.sh: 40-char uppercase-hex covered_sha → READY_SHA_MISSING (EC-002)" {
    local pr_number="42"
    # BC-5.42.001 Invariant 5 + EC-002: 40-char string but uppercase hex = malformed
    local uppercase_sha="AAAA000000000000000000000000000000000000"

    # gh must NOT be called — malformed SHA rejected before network I/O.
    cat > "${MOCK_BIN}/gh" <<'GHEOF'
#!/usr/bin/env bash
printf 'MOCK_GH_SHOULD_NOT_BE_CALLED_FOR_UPPERCASE_SHA\n' >&2
exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${uppercase_sha}" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for uppercase (malformed) covered_sha (EC-002 / Invariant 5); got ${status}"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "READY_SHA_MISSING" || {
        echo "FAIL: READY_SHA_MISSING not found for uppercase covered_sha (BC-5.42.001 EC-002 / Invariant 5)"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# F-S1901-P1-003 (HIGH) — AC-004 mandatory mechanism test (pass-1 adversary gap)
# T-017: darwin-leg fragment execution — while IFS= read -r bash-3.2 compat regression pin
# ─────────────────────────────────────────────────────────────────────────────

# T-017: Regression pin for release.yml rc.22 failure site (commit c10dc6ca).
# The "Verify registry-declared WASM plugins are staged" step previously used
# mapfile (bash 4.0+ only), failing on macOS /bin/bash 3.2. This test extracts
# the while IFS= read -r loop fragment from the fixed release.yml, runs it
# under /bin/bash with a fixture registry, and asserts exit 0 + staging verdict.
#
# GREEN on Darwin (fix is already in release.yml at c10dc6ca) — acceptable per
# F-S1901-P1-003: this is a regression-pinning mechanism test.
# Negative control: mapfile variant FAILS under /bin/bash 3.2 → confirms the
# test is load-bearing (reverts to mapfile = suite turns red again).
#
# Skipped on Linux: /bin/bash there is bash 5.x so mapfile would pass, making
# the negative control meaningless; the darwin-leg CI job runs on macOS only.
@test "T-017: darwin-leg fragment: while IFS= read -r bash-3.2 compat (regression pin rc.22)" {
    if [[ "$(uname)" != "Darwin" ]]; then
        skip "T-017 is macOS-only: negative control (mapfile) requires /bin/bash 3.2.x"
    fi

    # Build fixture work directory with:
    #   plugins/vsdd-factory/hooks-registry.toml  (copied from repo — 73 entries, ≥ 30 guard)
    #   artifact/<name>.wasm                       (stub empty files for each declared plugin)
    local work_dir
    work_dir="$(mktemp -d "${BATS_TMPDIR}/T017-work-XXXXXX")"
    mkdir -p "${work_dir}/plugins/vsdd-factory" "${work_dir}/artifact"
    cp "${REPO_ROOT}/plugins/vsdd-factory/hooks-registry.toml" \
        "${work_dir}/plugins/vsdd-factory/hooks-registry.toml"

    # Create stub WASMs for every declared plugin (mirrors the real artifact/ from cargo build).
    while IFS= read -r name; do
        [ -n "$name" ] && touch "${work_dir}/artifact/${name}"
    done < <(
        grep -E '^\s*plugin\s*=\s*"hook-plugins/' \
            "${work_dir}/plugins/vsdd-factory/hooks-registry.toml" \
            | grep -oE 'hook-plugins/[^"]+' \
            | sed 's|hook-plugins/||' \
            | sort -u
    )

    # ── POSITIVE TEST ────────────────────────────────────────────────────────
    # Extract the "Verify registry-declared WASM plugins are staged" step shell
    # body from .github/workflows/release.yml at test time (awk strips the
    # 10-space YAML indent). If mapfile is ever reintroduced into release.yml
    # this extraction picks it up → fragment fails under /bin/bash 3.2 → suite RED.
    local pos_script="${work_dir}/fragment-positive.sh"
    {
        printf '#!/bin/bash\n'
        printf 'set -euo pipefail\n'
        printf 'cd %q\n' "${work_dir}"
        awk '
            /- name: Verify registry-declared WASM plugins are staged/ && !found { found=1 }
            found && /^        run: [|]/ { in_run=1; next }
            in_run && /^      - / { exit }
            in_run { sub(/^          /, ""); print }
        ' "${REPO_ROOT}/.github/workflows/release.yml"
    } > "${pos_script}"
    chmod +x "${pos_script}"

    # Pre-flight: confirm extraction captured the while-IFS-read-r fix, not mapfile.
    grep -q "while IFS= read -r" "${pos_script}" || {
        echo "FAIL: extracted fragment missing 'while IFS= read -r' — release.yml may have lost the rc.22 fix"
        cat "${pos_script}"
        return 1
    }
    if grep -qE '^\s*mapfile\b' "${pos_script}"; then
        echo "FAIL: extracted fragment invokes mapfile — rc.22 regression reintroduced in release.yml"
        cat "${pos_script}"
        return 1
    fi

    run /bin/bash "${pos_script}" 2>&1
    [ "${status}" -eq 0 ] || {
        echo "FAIL: while-IFS-read-r fragment should exit 0 under /bin/bash 3.2 (c10dc6ca fix)"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "Registry-vs-staged: all" || {
        echo "FAIL: expected 'Registry-vs-staged: all' staging verdict in output"
        echo "Output: ${output}"
        return 1
    }

    # ── NEGATIVE CONTROL ─────────────────────────────────────────────────────
    # mapfile is bash 4.0+; must fail under /bin/bash 3.2. If this passes,
    # the test is not load-bearing (we are not running under bash 3.2).
    local neg_script="${work_dir}/fragment-mapfile.sh"
    {
        printf '#!/bin/bash\n'
        printf 'set -euo pipefail\n'
        printf 'cd %q\n' "${work_dir}"
        cat << 'MAPFILE_FRAGMENT'
# mapfile requires bash 4.0+; this MUST fail under /bin/bash 3.2 on macOS.
mapfile -t declared < <(
  grep -E '^\s*plugin\s*=\s*"hook-plugins/' \
    plugins/vsdd-factory/hooks-registry.toml \
    | grep -oE 'hook-plugins/[^"]+' \
    | sed 's|hook-plugins/||' \
    | sort -u
)
echo "mapfile-form: ${#declared[@]} entries (should not reach here under bash 3.2)"
MAPFILE_FRAGMENT
    } > "${neg_script}"
    chmod +x "${neg_script}"

    run /bin/bash "${neg_script}" 2>&1
    [ "${status}" -ne 0 ] || {
        echo "FAIL (negative control): mapfile form should fail under /bin/bash 3.2"
        echo "If this passes, /bin/bash is not 3.2 — check interpreter version:"
        /bin/bash --version 2>&1 | head -1
        echo "Output: ${output}"
        return 1
    }
    # Documented: negative control confirms mapfile not portable to bash 3.2;
    # reverting c10dc6ca's while-IFS-read-r fix would cause this test to fail.
}

# ─────────────────────────────────────────────────────────────────────────────
# F-S1901-P1-004 (MEDIUM, closed) — ADR-030 §Decision 2 arms 3+4 now covered
# T-018: EC-003 (closed/merged PR) — GREEN: arm 3 verified post-implementation
# T-019: arm 4 (malformed/unparseable gh JSON) — GREEN: arm 4 verified post-implementation
# ─────────────────────────────────────────────────────────────────────────────

# T-018: check-stale-verdict.sh: closed PR with matching headRefOid → exit 1 + CHECK_STALE_VERDICT_ERROR
# ADR-030 §Decision 2 arm 3: PR closed/merged between READY verdict and check invocation.
# BC-5.42.001 EC-003: gh returns non-open state; script exits non-zero with diagnostic.
# Sentinel (ADR-030 §Decision 2 arm 3):
#   CHECK_STALE_VERDICT_ERROR: PR #<n> is <state> (expected: open)
#
# Verification: implementation fetches state alongside headRefOid; even with matching
# SHAs, non-open state triggers arm 3 — exits non-zero with:
# CHECK_STALE_VERDICT_ERROR: PR #<n> is <state> (expected: open).
@test "T-018: check-stale-verdict.sh: closed PR (matching SHA) → exit 1 + CHECK_STALE_VERDICT_ERROR (EC-003)" {
    local pr_number="42"
    local sha="dddd333333333333333333333333333333333333"

    # Mock gh: returns state=closed + headRefOid MATCHING covered_sha.
    # The SHAs match; the implementation correctly checks state regardless and emits
    # CHECK_STALE_VERDICT_ERROR when state != "open" (ADR-030 §Decision 2 arm 3).
    cat > "${MOCK_BIN}/gh" <<GHEOF
#!/usr/bin/env bash
if [[ "\$1" == "pr" && "\$2" == "view" ]]; then
    printf '{"headRefOid":"${sha}","state":"closed"}\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "\$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit when PR is closed (EC-003); got exit 0"
        echo "arm 3 is implemented: expected non-zero exit; script must fetch state and emit"
        echo "ADR-030 §Decision 2 arm 3: CHECK_STALE_VERDICT_ERROR: PR #<n> is closed (expected: open)"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "CHECK_STALE_VERDICT_ERROR" || {
        echo "FAIL: CHECK_STALE_VERDICT_ERROR not found in output (ADR-030 §Decision 2 arm 3)"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "PR #${pr_number}" || {
        echo "FAIL: PR number not in CHECK_STALE_VERDICT_ERROR message"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "closed" || {
        echo "FAIL: PR state 'closed' not in CHECK_STALE_VERDICT_ERROR message"
        echo "Expected substring: CHECK_STALE_VERDICT_ERROR: PR #${pr_number} is closed (expected: open)"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "expected: open" || {
        echo "FAIL: '(expected: open)' not in CHECK_STALE_VERDICT_ERROR message (ADR-030 §Decision 2 arm 3 form)"
        echo "Expected: CHECK_STALE_VERDICT_ERROR: PR #${pr_number} is closed (expected: open)"
        echo "Output: ${output}"
        return 1
    }
}

# T-019: check-stale-verdict.sh: malformed/unparseable gh JSON → exit 1 + CHECK_STALE_VERDICT_ERROR
# ADR-030 §Decision 2 arm 4: gh exits 0 but returns non-JSON that cannot be parsed.
# BC-5.42.001 EC-004 catch-all: CHECK_STALE_VERDICT_ERROR: <description> on stderr.
#
# Verification: implementation distinguishes gh-failure (arm 1: READY_SHA_FETCH_FAILED)
# from parse-failure (arm 4: CHECK_STALE_VERDICT_ERROR). gh exits 0 but with
# non-JSON output triggers the arm 4 catch-all: CHECK_STALE_VERDICT_ERROR: <description>.
@test "T-019: check-stale-verdict.sh: malformed gh JSON → exit 1 + CHECK_STALE_VERDICT_ERROR (arm 4)" {
    local pr_number="99"
    local covered_sha="eeee444444444444444444444444444444444444"

    # Mock gh: exits 0 (no network failure) but returns non-JSON garbage.
    # Implementation: gh succeeds but JSON parse fails → emits CHECK_STALE_VERDICT_ERROR: <description>
    # (arm 4 catch-all), distinguishing parse failure from gh failure (arm 1: READY_SHA_FETCH_FAILED).
    cat > "${MOCK_BIN}/gh" <<'GHEOF'
#!/usr/bin/env bash
if [[ "$1" == "pr" && "$2" == "view" ]]; then
    printf 'not-json-garbage\n'
    exit 0
fi
printf 'MOCK_GH_UNHANDLED: %s\n' "$*" >&2; exit 1
GHEOF
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit on malformed gh JSON; got exit 0"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "CHECK_STALE_VERDICT_ERROR" || {
        echo "FAIL: CHECK_STALE_VERDICT_ERROR not found in output (ADR-030 §Decision 2 arm 4)"
        echo "arm 4 is implemented: expected CHECK_STALE_VERDICT_ERROR (parse failure)"
        echo "  distinct from arm 1 (READY_SHA_FETCH_FAILED = gh failure)"
        echo "Output: ${output}"
        return 1
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# F-S1901-P5-001 (MEDIUM) — arm-4 catch-all bypassed by set -euo pipefail
#   on non-string JSON values (key present, value is not a quoted string)
# T-020: check-stale-verdict.sh: headRefOid null value — RED gate
# T-021: enforce-merge-strategy.sh: headRefName null value — RED gate (fail-open)
# ─────────────────────────────────────────────────────────────────────────────

# T-020: check-stale-verdict.sh: headRefOid is null (not a quoted string) → arm-4 bypass.
# ADR-030 §Decision 2 arm 4: any unparseable JSON → CHECK_STALE_VERDICT_ERROR on stderr.
# The key "headRefOid" IS present, so the outer `grep -q '"headRefOid"'` matches, entering
# the extraction. But grep -oE '"headRefOid":"[^"]*"' finds no quoted-string value → exits 1.
# With set -euo pipefail, the script aborts BEFORE reaching arm-4 (lines ~80-83).
# Current behavior: bare exit 1, empty stderr. Spec-correct: CHECK_STALE_VERDICT_ERROR.
@test "T-020: check-stale-verdict.sh: headRefOid null value → exit 1 + CHECK_STALE_VERDICT_ERROR (F-P5-001)" {
    local pr_number="77"
    local covered_sha="ffff555555555555555555555555555555555555"

    # stub-gh: exits 0 but returns null for headRefOid (not a quoted string).
    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" STUB_GH_NULL_OID="1" \
        bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL: expected non-zero exit when headRefOid value is null"
        echo "Output: ${output}"
        return 1
    }
    echo "${output}" | grep -q "CHECK_STALE_VERDICT_ERROR" || {
        echo "FAIL: CHECK_STALE_VERDICT_ERROR not found in output (arm-4 bypass under set -euo pipefail)"
        echo "Current behavior: set -e aborts at LIVE_SHA assignment → bare exit 1, empty stderr"
        echo "Spec (ADR-030 §Decision 2 arm 4): CHECK_STALE_VERDICT_ERROR: unable to parse gh JSON response for PR #${pr_number}"
        echo "Output: ${output}"
        return 1
    }
}

# T-021: enforce-merge-strategy.sh: headRefName null value → fail-open delegate.
# ADR-030 §Decision 3: branch resolution failure is fail-open — treat as non-release,
# delegate to gh pr merge with the caller-supplied flag (no error sentinel emitted).
# The key "headRefName" IS present, so the outer grep matches, entering the extraction.
# But grep -oE '"headRefName":"[^"]*"' finds no quoted-string value → exits 1.
# With set -euo pipefail, the script aborts at BRANCH_NAME assignment (BEFORE delegating).
# Current behavior: bare exit 1. Spec-correct: exit 0 (gh pr merge delegated).
@test "T-021: enforce-merge-strategy.sh: headRefName null value → fail-open delegate (F-P5-001 sibling)" {
    local pr_number="88"

    # stub-gh: exits 0 but returns null for headRefName; gh pr merge exits 0.
    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" STUB_GH_NULL_HEAD_REF_NAME="1" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 (fail-open delegate) when headRefName value is null"
        echo "Current behavior: set -e aborts at BRANCH_NAME assignment → bare exit 1"
        echo "Spec (ADR-030 §Decision 3): branch resolution failure is fail-open;"
        echo "  treat as non-release and delegate to gh pr merge (exit 0)"
        echo "Output: ${output}"
        return 1
    }
    if echo "${output}" | grep -q "RELEASE_PR_SQUASH_FORBIDDEN"; then
        echo "FAIL: unexpected RELEASE_PR_SQUASH_FORBIDDEN — null headRefName must not match ^release/v"
        echo "Output: ${output}"
        return 1
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# F-P7-001 (HIGH) — wrapper scripts (check-stale-verdict.sh, enforce-merge-strategy.sh)
#   are correct but NOT wired into pr-manager.md's operative Step 8.  Step 8 still
#   dispatches a direct 'gh pr merge <PR> --squash --delete-branch' via github-ops,
#   recreating D-750 on release PRs. Additionally, enforce-merge-strategy.sh drops
#   residual args ($3+) and has no deny-list for injected second strategy flags.
#
# ADOPTED CONTRACT (research-backed; architect/PO will document in ADR-030/BC-5.42.001):
#   enforce-merge-strategy.sh is a GOVERNED PASS-THROUGH:
#   - Signature stays: <pr_number> [--merge|--squash|--rebase] [residual-args...]
#   - Residual args "${@:3}" are forwarded to gh pr merge (allows --delete-branch etc.)
#   - DENY-LIST rejects (exit 2) any residual arg matching a second strategy flag or --admin:
#     long (--squash/--merge/--rebase/--admin), =-fused, bare short (-s/-m/-r/-A),
#     combined short clusters containing s/m/r/A (e.g. -sd)
#   - --delete-branch/-d MUST be allowed through (not in deny-list)
#   - Release rule unchanged: ^release/v head → force --merge; reject --squash/--rebase
#   - Merge success is NOT gated on branch deletion (best-effort; see note below)
#
# T-022: pr-manager.md Step 8 wiring — RED gate
# T-023: enforce-merge-strategy.sh arg pass-through — RED gate
# T-024: deny-list --squash as second strategy — RED gate
# T-025: deny-list --admin — RED gate
# T-026: deny-list -sd combined short cluster — RED gate
# T-027: deny-list positive (--delete-branch allowed + forwarded) — RED gate
# T-028: release + --merge --delete-branch → delegates both — RED gate
#
# Best-effort delete note (no separate RED test):
#   enforce-merge-strategy.sh delegates to 'gh pr merge "${PR_NUMBER}" "${MERGE_FLAG}" "${@:3}"'
#   and propagates gh's exit code directly. The wrapper has NO separate branch-deletion step;
#   it does not independently verify deletion or gate on it. When gh pr merge --delete-branch
#   exits 0, the wrapper exits 0 regardless of whether GitHub's async deletion completed
#   (see cli/cli #13380, cli/cli #12980). A separate RED test for this property would be
#   trivially GREEN today (wrapper already propagates exit codes). Any future regression
#   where the wrapper independently gates on deletion would be caught by T-023/T-027/T-028.
# ─────────────────────────────────────────────────────────────────────────────

# T-022: pr-manager.md operative Step 8 must NOT dispatch a direct 'gh pr merge' and
# MUST reference both enforce-merge-strategy.sh and check-stale-verdict.sh in the step body.
# BC-5.42.001 PC-3: "Direct gh pr merge calls outside this wrapper are a protocol violation."
# BC-5.42.001 PC-2: "orchestrator MUST invoke check-stale-verdict.sh before every gh pr merge."
#
# RED now: Step 8 contains:
#   Agent(subagent_type="vsdd-factory:github-ops", prompt="cd <project-path> && gh pr merge <PR_NUMBER> --squash --delete-branch")
# and neither enforce-merge-strategy.sh nor check-stale-verdict.sh appear in the Step 8 body.
@test "T-022: pr-manager.md Step 8 must route through wrappers not direct gh pr merge (F-P7-001 wiring)" {
    local pm_md="${PLUGIN_ROOT}/agents/pr-manager.md"

    # Assertion 1: zero direct 'gh pr merge' in any Agent dispatch prompt.
    # The only permitted pattern is routing through enforce-merge-strategy.sh.
    # Current: Step 8 has prompt="cd <project-path> && gh pr merge <PR_NUMBER> --squash --delete-branch"
    if grep -qE '"[^"]*gh pr merge' "${pm_md}"; then
        echo "FAIL: direct 'gh pr merge' found in agent dispatch prompt in pr-manager.md"
        echo "Step 8 must route ALL merges through plugins/vsdd-factory/bin/enforce-merge-strategy.sh"
        echo "Per BC-5.42.001 PC-3: direct gh pr merge outside the wrapper is a protocol violation"
        grep -nE '"[^"]*gh pr merge' "${pm_md}" | head -5
        return 1
    fi

    # Assertion 2: enforce-merge-strategy.sh must appear in the Step 8 operative body.
    # Current: enforce-merge-strategy.sh is only in the separate gate-docs section (after Step 9),
    # not in the operative Step 8 dispatch block.
    if ! awk '/^### Step 8:/,/^### Step 9:/' "${pm_md}" | grep -q 'enforce-merge-strategy.sh'; then
        echo "FAIL: enforce-merge-strategy.sh not referenced in Step 8 body"
        echo "Step 8 must dispatch via: plugins/vsdd-factory/bin/enforce-merge-strategy.sh"
        echo "Required by BC-5.42.001 PC-3 (governed pass-through wrapper)"
        return 1
    fi

    # Assertion 3: check-stale-verdict.sh must appear in the Step 8 operative body.
    # Current: check-stale-verdict.sh is only in the stale-verdict docs section (after Step 9),
    # not in the operative Step 8 dispatch block.
    if ! awk '/^### Step 8:/,/^### Step 9:/' "${pm_md}" | grep -q 'check-stale-verdict.sh'; then
        echo "FAIL: check-stale-verdict.sh not referenced in Step 8 body"
        echo "Step 8 must invoke check-stale-verdict.sh BEFORE delegating to enforce-merge-strategy.sh"
        echo "Required by BC-5.42.001 PC-2 (stale-verdict detection)"
        return 1
    fi
}

# T-023: enforce-merge-strategy.sh must forward residual args (${@:3}) to gh pr merge.
# The governed pass-through contract (F-P7-001) requires --delete-branch and other
# non-denied residual args to be passed through to gh pr merge unchanged.
#
# RED now: wrapper ends with 'gh pr merge "${PR_NUMBER}" "${MERGE_FLAG}"' — drops $3+.
# After implementation: 'gh pr merge "${PR_NUMBER}" "${MERGE_FLAG}" "${@:3}"'.
@test "T-023: enforce-merge-strategy.sh forwards --delete-branch residual arg to gh (F-P7-001 pass-through)" {
    local pr_number="55"
    local argv_log
    argv_log="${MOCK_BIN}/gh-argv.txt"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        STUB_GH_ARGV_LOG="${argv_log}" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "--delete-branch" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for non-release branch --merge --delete-branch; got ${status}"
        echo "Output: ${output}"
        return 1
    }

    # Assert --delete-branch was forwarded to stub-gh's gh pr merge handler.
    { [[ -f "${argv_log}" ]] && grep -q -- "--delete-branch" "${argv_log}"; } || {
        echo "FAIL: --delete-branch not forwarded to gh pr merge"
        echo "Current behavior: wrapper delegates 'gh pr merge \${PR_NUMBER} \${MERGE_FLAG}' — drops \$3+"
        echo "Required (F-P7-001): wrapper must forward residual args '\${@:3}' to gh pr merge"
        echo "argv_log contents: $(cat "${argv_log}" 2>/dev/null || echo '(gh pr merge not called or log empty)')"
        return 1
    }

    rm -f "${argv_log}"
}

# T-024: enforce-merge-strategy.sh deny-list: passing --squash as a residual arg after the
# primary strategy flag → exit 1 (second strategy injection blocked).
# A caller passing '--merge --squash' is attempting to override the release-branch enforcement
# by smuggling a second strategy token; the deny-list must reject this.
#
# F-P8-002 correction: exit code is 1 (not 2) per BC-5.42.001 §Description(c)/Invariant 7 +
# ADR-030 §Decision 3 which mandate exit 1 for STRATEGY_SMUGGLING_FORBIDDEN. Spec wins.
#
# RED now: no deny-list exists; --squash is silently dropped (wrapper uses only $2).
# After implementation: residual strategy flags → exit 1 + STRATEGY_SMUGGLING_FORBIDDEN.
@test "T-024: enforce-merge-strategy.sh: --merge --squash (two strategies) → exit 1 (F-P7-001/F-P8-002 deny-list)" {
    local pr_number="60"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "--squash" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for second strategy flag --squash; got ${status}"
        echo "Current behavior: no deny-list — --squash silently dropped (only \$2 used)"
        echo "Required (F-P8-002 + BC-5.42.001 Invariant 7): residual strategy flags → exit 1"
        echo "  STRATEGY_SMUGGLING_FORBIDDEN per BC-5.42.001 §Description(c) + ADR-030 §Decision 3"
        echo "Output: ${output}"
        return 1
    }
}

# T-025: enforce-merge-strategy.sh deny-list: --admin as residual arg → exit 1.
# --admin bypasses branch-protection rules on GitHub; it must never be forwarded.
#
# F-P8-002 correction: exit code is 1 per BC-5.42.001 §Description(c)/Invariant 7.
#
# RED now: no deny-list; --admin silently dropped.
# After implementation: --admin in residual args → exit 1 + STRATEGY_SMUGGLING_FORBIDDEN.
@test "T-025: enforce-merge-strategy.sh: --merge --admin → exit 1 (F-P7-001/F-P8-002 deny-list --admin)" {
    local pr_number="61"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "--admin" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for --admin residual arg; got ${status}"
        echo "Current behavior: no deny-list — --admin silently dropped"
        echo "Required (F-P8-002 + BC-5.42.001 Invariant 7): --admin → exit 1 (branch-protection bypass blocked)"
        echo "Output: ${output}"
        return 1
    }
}

# T-026: enforce-merge-strategy.sh deny-list: combined short flag cluster -sd → exit 1.
# -s is the short form of --squash; -sd combines squash (-s) with delete-branch (-d).
# The deny-list must reject any combined short-flag cluster containing s, m, r, or A —
# it cannot forward -sd even though -d alone is allowed.
#
# F-P8-002 correction: exit code is 1 per BC-5.42.001 §Description(c)/Invariant 7.
#
# RED now: no deny-list; -sd silently dropped.
# After implementation: combined short clusters containing s/m/r/A → exit 1 + STRATEGY_SMUGGLING_FORBIDDEN.
@test "T-026: enforce-merge-strategy.sh: --merge -sd (combined short, s=squash) → exit 1 (F-P7-001/F-P8-002 deny-list)" {
    local pr_number="62"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "-sd" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for combined short cluster -sd (s=squash); got ${status}"
        echo "Current behavior: no deny-list — -sd silently dropped"
        echo "Required (F-P8-002 + BC-5.42.001 Invariant 7): combined short flags containing s/m/r/A → exit 1"
        echo "  -sd contains s (squash) → denied, even though -d (delete-branch) alone is allowed"
        echo "Output: ${output}"
        return 1
    }
}

# T-027: enforce-merge-strategy.sh deny-list positive: --delete-branch MUST be allowed through
# and forwarded to gh pr merge (--delete-branch is NOT in the deny-list).
# This test guards against an overly-broad deny-list that blocks -d/--delete-branch.
#
# RED now on forwarding: --delete-branch reaches stub-gh only after arg-forwarding is implemented.
# Currently, wrapper drops $3+, so argv_log won't contain --delete-branch.
@test "T-027: enforce-merge-strategy.sh: --merge --delete-branch allowed + forwarded (F-P7-001 deny-list positive)" {
    local pr_number="63"
    local argv_log
    argv_log="${MOCK_BIN}/gh-argv.txt"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        STUB_GH_ARGV_LOG="${argv_log}" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "--delete-branch" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 (--delete-branch must be allowed through deny-list); got ${status}"
        echo "Deny-list must NOT reject --delete-branch (only second-strategy flags and --admin are denied)"
        echo "Output: ${output}"
        return 1
    }

    # --delete-branch must be forwarded to gh pr merge (deny-list allows it).
    { [[ -f "${argv_log}" ]] && grep -q -- "--delete-branch" "${argv_log}"; } || {
        echo "FAIL: --delete-branch not forwarded to gh pr merge"
        echo "Current behavior: wrapper drops \$3+ — arg forwarding not yet implemented"
        echo "Required: deny-list allows --delete-branch; wrapper forwards it via '\${@:3}'"
        echo "argv_log: $(cat "${argv_log}" 2>/dev/null || echo '(gh pr merge not called or log empty)')"
        return 1
    }

    rm -f "${argv_log}"
}

# T-028: enforce-merge-strategy.sh: release/v* branch + --merge --delete-branch → exit 0,
# both --merge and --delete-branch forwarded to gh pr merge.
# The release rule (^release/v → force --merge, reject --squash/--rebase) must coexist with
# arg pass-through: --delete-branch must survive the release-branch path.
#
# RED now: wrapper drops $3+ regardless of branch type; --delete-branch not forwarded.
# After implementation: release branch forces --merge (already passed) and forwards --delete-branch.
@test "T-028: enforce-merge-strategy.sh: release + --merge --delete-branch → delegates both (F-P7-001)" {
    local pr_number="64"
    local release_branch="release/v1.0.0-rc.23"
    local argv_log
    argv_log="${MOCK_BIN}/gh-argv.txt"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="${release_branch}" \
        STUB_GH_ARGV_LOG="${argv_log}" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" "--delete-branch" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for release branch with --merge --delete-branch; got ${status}"
        echo "Output: ${output}"
        return 1
    }

    # Both --merge and --delete-branch must reach gh pr merge on the release path.
    { [[ -f "${argv_log}" ]] && grep -q -- "--merge" "${argv_log}"; } || {
        echo "FAIL: --merge not forwarded to gh pr merge for release branch"
        echo "argv_log: $(cat "${argv_log}" 2>/dev/null || echo '(gh pr merge not called or log empty)')"
        return 1
    }
    { [[ -f "${argv_log}" ]] && grep -q -- "--delete-branch" "${argv_log}"; } || {
        echo "FAIL: --delete-branch not forwarded to gh pr merge (release path must also forward \${@:3})"
        echo "Current behavior: wrapper drops \$3+ — only MERGE_FLAG=\$2 is forwarded"
        echo "argv_log: $(cat "${argv_log}" 2>/dev/null || echo '(gh pr merge not called or log empty)')"
        return 1
    }

    rm -f "${argv_log}"
}

# T-029: enforce-merge-strategy.sh: --admin as $2 (primary strategy slot) → exit 1.
# The signature is <pr_number> [--merge|--squash|--rebase] [residual-args...].
# $2 is the *strategy* slot; only --merge/--squash/--rebase are valid values.
# --admin is a GitHub branch-protection override flag, not a merge strategy.
# Current behavior (RED): no $2 validation; --admin stored in MERGE_FLAG and forwarded
# as "gh pr merge 10 --admin", which is a privilege-escalation bypass.
# After implementation (F-P8-003): invalid $2 → exit 1 + non-empty stderr + gh pr merge NOT called.
@test "T-029: enforce-merge-strategy.sh: --admin as \$2 (primary strategy slot) → exit 1 + non-empty stderr + gh pr merge NOT called (F-P8-003)" {
    local pr_number="10"
    local argv_log
    argv_log="${MOCK_BIN}/gh-argv.txt"
    # Remove so we can detect whether gh pr merge was called at all.
    rm -f "${argv_log}"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        STUB_GH_ARGV_LOG="${argv_log}" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--admin" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for --admin as \$2 (invalid strategy slot); got ${status}"
        echo "Required (F-P8-003): \$2 must be one of --merge/--squash/--rebase"
        echo "  --admin is a privilege-escalation flag, not a merge strategy"
        echo "  INVALID_STRATEGY or equivalent sentinel required on stderr; exit 1; no gh pr merge call"
        echo "Output: ${output}"
        return 1
    }

    [[ -n "${output}" ]] || {
        echo "FAIL: expected non-empty stderr/stdout for --admin as \$2 rejection; got empty output"
        echo "Required (F-P8-003): diagnostic sentinel must appear on stderr before exit 1"
        return 1
    }

    # gh pr merge must NOT have been called — the wrapper must reject before delegating.
    { [[ ! -f "${argv_log}" ]] || [[ ! -s "${argv_log}" ]]; } || {
        echo "FAIL: gh pr merge was called despite --admin as \$2 rejection (argv_log present + non-empty)"
        echo "argv_log content: $(cat "${argv_log}")"
        echo "Required (F-P8-003): script must exit 1 before invoking gh pr merge"
        rm -f "${argv_log}"
        return 1
    }

    rm -f "${argv_log}"
}

# T-030: enforce-merge-strategy.sh: -A as $2 (short form of --admin in strategy slot) → exit 1.
# -A is the short form of --admin (GitHub CLI). Same semantics as T-029 but using the
# abbreviated flag. Must be rejected identically — short forms of admin flags are not
# merge strategies.
# Current behavior (RED): no $2 validation; -A stored in MERGE_FLAG and forwarded as
# "gh pr merge 10 -A", bypassing branch protection.
@test "T-030: enforce-merge-strategy.sh: -A as \$2 (short admin flag in strategy slot) → exit 1 + non-empty stderr + gh pr merge NOT called (F-P8-003)" {
    local pr_number="10"
    local argv_log
    argv_log="${MOCK_BIN}/gh-argv.txt"
    rm -f "${argv_log}"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        STUB_GH_ARGV_LOG="${argv_log}" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "-A" 2>&1

    [ "${status}" -eq 1 ] || {
        echo "FAIL: expected exit 1 for -A as \$2 (short admin flag in strategy slot); got ${status}"
        echo "Required (F-P8-003): \$2 must be one of --merge/--squash/--rebase"
        echo "  -A is the short form of --admin — not a valid merge strategy"
        echo "  INVALID_STRATEGY or equivalent sentinel required on stderr; exit 1; no gh pr merge call"
        echo "Output: ${output}"
        return 1
    }

    [[ -n "${output}" ]] || {
        echo "FAIL: expected non-empty stderr/stdout for -A as \$2 rejection; got empty output"
        echo "Required (F-P8-003): diagnostic sentinel must appear on stderr before exit 1"
        return 1
    }

    { [[ ! -f "${argv_log}" ]] || [[ ! -s "${argv_log}" ]]; } || {
        echo "FAIL: gh pr merge was called despite -A as \$2 rejection (argv_log present + non-empty)"
        echo "argv_log content: $(cat "${argv_log}")"
        echo "Required (F-P8-003): script must exit 1 before invoking gh pr merge"
        rm -f "${argv_log}"
        return 1
    }

    rm -f "${argv_log}"
}

# T-031: enforce-merge-strategy.sh: --merge as $2 on feature branch → exit 0 (regression guard).
# Confirms that adding $2 strategy validation in F-P8-003 does not break the valid-strategy
# path. --merge is a canonical strategy flag; feature branches must continue to work after
# the $2 validation gate is introduced.
# This test is a GREEN positive regression guard — it should pass before and after F-P8-003.
@test "T-031: enforce-merge-strategy.sh: --merge as \$2 (feature branch) → exit 0 (F-P8-003 positive regression guard)" {
    local pr_number="10"

    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_NAME="feature/S-19.01" \
        STUB_GH_MERGE_EXIT="0" \
        bash "${BIN_DIR}/enforce-merge-strategy.sh" \
        "${pr_number}" "--merge" 2>&1

    [ "${status}" -eq 0 ] || {
        echo "FAIL: expected exit 0 for --merge as \$2 on feature branch; got ${status}"
        echo "Required (F-P8-003 regression guard): valid strategy flags must continue to work"
        echo "  --merge is a canonical strategy; \$2 validation must allow it through"
        echo "Output: ${output}"
        return 1
    }
}

# T-032: pr-manager.md Step 8-pre-A must NOT contain a re-fetch-covered_sha fallback (F-S1901-P12-001).
#
# The defect: Step 8-pre-A currently reads "Use `gh pr view <PR_NUMBER> --json headRefOid` to
# obtain the SHA if not already recorded". This makes the stale-verdict guard vacuous:
# if covered_sha is re-fetched as the live HEAD at merge time, check-stale-verdict.sh sees
# live-vs-live → always exit 0 → merges code that was never reviewed (D-749 recurrence).
#
# BC-5.42.001 authoritative requirements:
#   - Part a: covered_sha is recorded AT THE MOMENT OF ASSESSMENT (review time, not merge time)
#   - Precondition 4: covered_sha is "recorded in the most recent READY verdict"
#   - PC-1/Invariant 1+2: a READY verdict without covered_sha is NOT ACTIONABLE → HALT +
#     re-dispatch pr-reviewer (never re-fetch)
#
# The sibling "Stale-Verdict Detection (BC-5.42.001 PC2)" section already states this correctly
# (bare <covered_sha> arg, no fetch-fallback). Step 8-pre-A must align with it.
#
# Two assertions (both RED now):
#   Assertion 1 (negative — RED): "obtain the SHA if not already recorded" IS present in Step 8-pre-A.
#   Assertion 2 (positive — RED): "PC-1" is NOT cited in Step 8-pre-A (absent-covered_sha HALT path missing).
# After implementation: fallback line removed; absent-covered_sha path cites PC-1 → both assertions GREEN.
@test "T-032: pr-manager.md Step 8-pre-A must not contain a re-fetch-covered_sha fallback (F-S1901-P12-001)" {
    local pm_md="${PLUGIN_ROOT}/agents/pr-manager.md"

    # Assertion 1 (negative): Step 8-pre-A MUST NOT contain the re-fetch fallback instruction.
    # The awk range extracts from the Step 8-pre-A header to the Step 8-pre-B header (inclusive).
    # Current behavior (RED): "obtain the SHA if not already recorded" is present in that range.
    if awk '/Step 8-pre-A/,/Step 8-pre-B/' "${pm_md}" | grep -qF "obtain the SHA if not already recorded"; then
        echo "FAIL (F-S1901-P12-001): Step 8-pre-A contains a re-fetch-covered_sha fallback"
        echo "Found: 'obtain the SHA if not already recorded' in Step 8-pre-A body"
        echo "This makes the stale-verdict guard vacuous:"
        echo "  Re-fetching covered_sha at merge time → check-stale-verdict.sh compares live-vs-live"
        echo "  → always exit 0 → unreviewed code merged (D-749 merge-race recurrence)"
        echo "Required (BC-5.42.001 PC-1/Part a/Precondition 4):"
        echo "  covered_sha MUST be the value recorded in the READY verdict at review time"
        echo "  If absent: HALT — do NOT re-fetch; re-dispatch pr-reviewer (BC-5.42.001 PC-1/Invariant 2)"
        echo "Offending lines in Step 8-pre-A:"
        awk '/Step 8-pre-A/,/Step 8-pre-B/' "${pm_md}" | grep -n "obtain\|if not already" | head -5
        return 1
    fi

    # Assertion 2 (positive): Step 8-pre-A MUST cite BC-5.42.001 PC-1 for the absent-covered_sha HALT path.
    # BC-5.42.001 PC-1: a READY verdict without covered_sha is NOT ACTIONABLE → HALT + re-dispatch pr-reviewer.
    # Current behavior (RED): Step 8-pre-A header cites PC-2/Invariant 2 only; PC-1 is absent.
    # The absent-covered_sha HALT instruction (citing PC-1) must be added when the fetch-fallback is removed.
    if ! awk '/Step 8-pre-A/,/Step 8-pre-B/' "${pm_md}" | grep -q "PC-1"; then
        echo "FAIL (F-S1901-P12-001): Step 8-pre-A does not cite BC-5.42.001 PC-1"
        echo "Required: the absent-covered_sha HALT path must cite BC-5.42.001 PC-1/Invariant 2"
        echo "  Example: 'If covered_sha is absent: HALT — re-dispatch pr-reviewer (BC-5.42.001 PC-1/Invariant 2)'"
        echo "Current BC-5.42.001 citations in Step 8-pre-A:"
        awk '/Step 8-pre-A/,/Step 8-pre-B/' "${pm_md}" | grep "BC-5\.42" | head -5
        return 1
    fi
}

# T-033: check-stale-verdict.sh: headRefOid matches covered_sha but state field is JSON null
# (absent/unparseable) → must fail-closed with exit 1 + CHECK_STALE_VERDICT_ERROR.
#
# The defect path (F-P15-001):
#   gh returns: {"headRefOid":"<matching-sha>","state":null}
#   Parser: grep -oE '"state":"[^"]*"' does NOT match "state":null (null is unquoted JSON)
#   → PR_STATE="" (empty string)
#   → Arm 4 first check: LIVE_SHA non-empty → skipped
#   → Arm 3 (state check): if [[ -n "${PR_STATE}" ]] → false, skipped
#   → SHA match: LIVE_SHA == COVERED_SHA → exit 0 ← BUG (silent success despite unknown state)
#
# Required (ADR-030 §Decision 2 arm 4 catch-all; F-P15-001 hardening):
#   PR_STATE must be present AND non-empty AND == OPEN before SHA-match exits 0.
#   If PR_STATE is absent/null/unparseable: fail-closed via arm 4 CHECK_STALE_VERDICT_ERROR,
#   even when headRefOid exactly matches covered_sha.
#
# The test uses STUB_GH_NULL_STATE=1 which makes stub-gh emit:
#   {"headRefOid":"<covered_sha>","state":null}
# ensuring the SHA would match so any exit 1 is due to the null-state path, not SHA mismatch.
#
# RED now: exits 0 (arm 3 skipped; SHA match succeeds despite null state).
# After implementation: exits 1 + CHECK_STALE_VERDICT_ERROR.
@test "T-033: check-stale-verdict.sh: matching headRefOid but null state → exit 1 + CHECK_STALE_VERDICT_ERROR (F-P15-001)" {
    local pr_number="42"
    local covered_sha="dddd333333333333333333333333333333333333"

    # stub-gh: headRefOid = covered_sha (would match), but state = JSON null (unquoted).
    # The state-null path is distinct from: null headRefOid (T-020), closed PR (T-018),
    # malformed JSON (T-019). This specifically tests the matching-SHA + absent-state arm.
    cp "${FIXTURES_DIR}/stub-gh.sh" "${MOCK_BIN}/gh"
    chmod +x "${MOCK_BIN}/gh"

    run env PATH="${MOCK_BIN}:${PATH}" \
        STUB_GH_HEAD_REF_OID="${covered_sha}" \
        STUB_GH_NULL_STATE="1" \
        bash "${BIN_DIR}/check-stale-verdict.sh" \
        "${pr_number}" "${covered_sha}" 2>&1

    [ "${status}" -ne 0 ] || {
        echo "FAIL (F-P15-001): expected non-zero exit for matching SHA + null state; got exit 0"
        echo "Current behavior: arm 3 (state check) is skipped because PR_STATE is empty"
        echo "  (null JSON value does not match grep -oE '\"state\":\"[^\"]*\"')"
        echo "  → SHA match at Step 4 returns exit 0 despite unknown/absent PR state"
        echo "Required (ADR-030 §Decision 2 arm 4; F-P15-001 hardening):"
        echo "  PR_STATE must be present AND == OPEN before SHA-match exits 0"
        echo "  null/absent state → fail-closed via arm 4 CHECK_STALE_VERDICT_ERROR"
        echo "Output: ${output}"
        return 1
    }

    echo "${output}" | grep -q "CHECK_STALE_VERDICT_ERROR" || {
        echo "FAIL (F-P15-001): CHECK_STALE_VERDICT_ERROR not found in output"
        echo "Required: CHECK_STALE_VERDICT_ERROR sentinel on stderr for null/absent state"
        echo "  (arm 4 catch-all: state unparseable → fail-closed, same class as T-020)"
        echo "Output: ${output}"
        return 1
    }
}
