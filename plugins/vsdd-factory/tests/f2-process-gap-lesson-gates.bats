#!/usr/bin/env bats
# f2-process-gap-lesson-gates.bats — S-18.09 F2 process-gap lesson gate suite.
#
# Story:   S-18.09 — F2 process-gap lesson gate checks — machine-stable lesson assertions,
#          stale-term detector, BC-precondition registry-block-shape validator, AC↔PC parity gate
# Governs:  S-18.09 — F2 process-gap lesson gate checks (ADR-026 / D-576 / O-P4-004)
# Enforces:
#   AC-001 — L-F2-machine-stable-count-assertion (bats tests use plugin.log structured code: signals)
#   AC-002 — L-F2-fix-at-correct-layer (VP source_bc files exist and are reachable)
#   AC-003 — L-F2-no-bypass-on-edit-failure (no || true on load-bearing writes in E-18 hooks)
#   AC-004 — L-F2-exhaustive-sweep-enumerate-and-count (S-18.08 discovery scan counts before loop)
#   AC-005 — cross-reference title-cite parity (story behavioral_contracts BC IDs resolve to files)
#   AC-006 — stale-term detector (no current_wave: in normative E-18 BC/VP sections)
#   AC-007 — BC-precondition registry-block-shape validator (D-576 name+plugin fields present)
#   AC-008 — AC↔PC parity gate (O-P4-004 process gap: every AC trace resolves to real BC clause)
#
# @test fatal-path contract (O-P7-001):
#   Every @test MUST use:
#     run bash -c '<snippet>'
#     assert_success
#     refute_output --partial "FAIL"
#   so that echo "FAIL: ..." lines emitted by the gate snippets become real test
#   failures. assert_success and refute_output are defined as helpers below
#   (bats-assert is not installed system-wide; helpers are inlined here per
#   project convention of no external bats library dependencies).
#
# .factory/ resolution (worktree topology):
#   .factory/ is an orphan-branch (factory-artifacts) worktree mounted ONLY at
#   the main checkout root. It is NOT present in feature worktrees.
#   Resolution: BATS_TEST_DIRNAME/../../.. gives the feature worktree root.
#   If $ROOT/.factory/specs does NOT exist, fall back to the primary worktree root
#   derived from `git -C $ROOT rev-parse --git-common-dir` (parent of .git).
#   This makes the suite pass both locally and in CI.
#
# Pipefail note: grep exits 1 when no lines match (correct 0-hit outcome). Using
# `|| true` on HITS=... assignments prevents set -e from aborting when the scan
# correctly finds 0 hits. Explicit `[ "$HITS" -eq 0 ] || echo "FAIL: ..."` provides
# the load-bearing assertion.

# ---------------------------------------------------------------------------
# Inline assert_success / refute_output helpers
# (bats-assert API surface; implemented without the bats-assert package)
# ---------------------------------------------------------------------------

assert_success() {
  if [ "$status" -ne 0 ]; then
    echo "assert_success: expected exit status 0, got $status" >&2
    echo "output: $output" >&2
    return 1
  fi
}

refute_output() {
  local mode=""
  local substring=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --partial) mode="partial"; shift ;;
      *) substring="$1"; shift ;;
    esac
  done
  if [ "$mode" = "partial" ]; then
    if echo "$output" | grep -qF "$substring"; then
      echo "refute_output --partial: output contains forbidden substring '$substring'" >&2
      echo "output: $output" >&2
      return 1
    fi
  fi
}

# ---------------------------------------------------------------------------
# setup — resolve FACTORY_ROOT with worktree-topology fallback
# ---------------------------------------------------------------------------

setup() {
  # Candidate root: BATS_TEST_DIRNAME is .../plugins/vsdd-factory/tests — 3 levels up
  local candidate_root
  candidate_root="$(cd "${BATS_TEST_DIRNAME}/../../.." && pwd)"

  if [ -d "${candidate_root}/.factory/specs" ]; then
    FACTORY_ROOT="${candidate_root}"
  else
    # Feature-worktree case: .factory/ is not mounted here.
    # git --git-common-dir gives the common .git dir (in the main worktree).
    # Its parent is the main worktree root where .factory/ is mounted.
    local git_common_dir
    git_common_dir="$(git -C "${candidate_root}" rev-parse --git-common-dir 2>/dev/null || true)"
    FACTORY_ROOT="$(dirname "${git_common_dir}")"
  fi

  TEST_TMP="$(mktemp -d "${BATS_TMPDIR}/f2-process-gap-XXXXXX")"

  export FACTORY_ROOT TEST_TMP
}

teardown() {
  rm -rf "${TEST_TMP}"
}

# ---------------------------------------------------------------------------
# AC-001 / test_e18_bats_tests_use_machine_stable_assertions_not_presentation_regex
#
# Gate assertion — L-F2-machine-stable-count-assertion:
#   Verifies that E-18 bats test files assert against plugin.log structured code:
#   fields, not presentation-coupled grep -c "^  - " regex patterns.
#
# Expected: ANTI_PATTERN_HITS=0, STABLE_ASSERTIONS>0.
# ---------------------------------------------------------------------------

@test "test_e18_bats_tests_use_machine_stable_assertions_not_presentation_regex" {
  local tests_dir="${BATS_TEST_DIRNAME}"
  # Write the script to a temp file to avoid quoting complexity for double-quote patterns.
  # Note: || true on grep calls prevents set -e from aborting on 0-match (grep exits 1).
  local tmpscript
  tmpscript="${TEST_TMP}/gate.sh"
  cat > "$tmpscript" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
TESTS_DIR="${tests_dir}"
# Scan E-18 bats files for presentation-coupled assertion anti-pattern
ANTI_PATTERN_HITS=\$(grep -rE "grep -c '^\\\\s*-'" \\
  "\${TESTS_DIR}/validate-heavy-op-delegation.bats" \\
  "\${TESTS_DIR}/pure-parse-invariant-gate.bats" \\
  2>/dev/null | wc -l) || true
[ "\$ANTI_PATTERN_HITS" -eq 0 ] || echo "FAIL: presentation-coupled assertion pattern found"

# Scan for machine-stable assertion pattern: plugin.log structured record presence
STABLE_ASSERTIONS=\$(grep -rE '"code":"?DelegationRecommended"?' \\
  "\${TESTS_DIR}/validate-heavy-op-delegation.bats" \\
  2>/dev/null | wc -l) || true
[ "\$STABLE_ASSERTIONS" -gt 0 ] || echo "FAIL: no machine-stable DelegationRecommended assertions found in bats"
SCRIPT
  chmod +x "$tmpscript"
  run bash "$tmpscript"
  rm -f "$tmpscript"
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-002 / test_e18_vp_source_bc_files_exist_and_are_reachable
#
# Gate assertion — L-F2-fix-at-correct-layer:
#   For each E-18 VP (VP-088..VP-091), verify source_bc frontmatter field
#   matches an existing BC file — proxy for "VP does not assert beyond its BC".
#
# Expected: all 4 VP source_bc files found; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_vp_source_bc_files_exist_and_are_reachable" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
    # For each E-18 VP, verify source_bc frontmatter field matches an existing BC file
    for VP_FILE in \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-088.md" \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-089.md" \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-090.md" \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-091.md"; do
      SOURCE_BC=$(grep "^source_bc:" "$VP_FILE" | cut -d'"'"'"'"'"' -f2 | cut -d'"'"' '"'"' -f1)
      BC_ID=$(echo "$SOURCE_BC" | grep -oE "BC-[0-9]+\.[0-9]+\.[0-9]+")
      BC_SS=$(echo "$BC_ID" | grep -oE "^BC-([0-9]+)" | grep -oE "[0-9]+")
      BC_SS_DIR=$(printf "ss-%02d" "$BC_SS")
      BC_FILE="'"${factory_root}"'/.factory/specs/behavioral-contracts/${BC_SS_DIR}/${BC_ID}.md"
      [ -f "$BC_FILE" ] || echo "FAIL: $VP_FILE source_bc $BC_ID not found at $BC_FILE"
    done
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-003 / test_e18_hook_scripts_no_bypass_on_load_bearing_writes
#
# Gate assertion — L-F2-no-bypass-on-edit-failure:
#   No load-bearing write operation in E-18 hook scripts uses || true to suppress errors.
#   Load-bearing write = git commit, git push, git add, >> .jsonl, tee .jsonl.
#
# Note: || true on non-load-bearing reads (e.g. git show ... 2>/dev/null || { ... })
# is acceptable and is NOT flagged by this gate.
#
# Expected: BYPASS_HITS=0; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_hook_scripts_no_bypass_on_load_bearing_writes" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
    # Positive-coverage pre-assertion (F-P3-001 / F-P4-002 class — absence-only vacuity guard):
    # Assert the scan target exists and is non-empty BEFORE the BYPASS_HITS check.
    # If the file were renamed or deleted, BYPASS_HITS=0 would vacuously pass (zero items scanned).
    SCAN_TARGET="'"${factory_root}"'/plugins/vsdd-factory/hooks/postcompact-reanchor.sh"
    [ -s "$SCAN_TARGET" ] \
      || echo "FAIL: AC-003 scan target $SCAN_TARGET absent or empty — gate cannot verify no-bypass (positive-coverage guard)"

    # Scan for || true on git commit / git push / git add / >> .jsonl / tee .jsonl patterns
    BYPASS_HITS=$(grep -En "(git commit|git push|git add|>> .+\.jsonl|tee .+\.jsonl).*\|\| true" \
      "$SCAN_TARGET" \
      2>/dev/null | wc -l)
    [ "$BYPASS_HITS" -eq 0 ] || echo "FAIL: load-bearing write with || true bypass found in E-18 hook scripts"
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-004 / test_s18_08_discovery_scan_enumerates_and_counts_before_loop
#
# Gate assertion — L-F2-exhaustive-sweep-enumerate-and-count:
#   The pure-parse-invariant-gate.bats AC-005 implementation enumerates the
#   discovered BC set and counts the members BEFORE running the scan loop.
#   A discovery returning empty set must be a gate failure, not a vacuous success.
#
# Expected: discovered_count enumeration variable present; discovered_count -eq 0
# empty-discovery guard present. Validated: passes against real file (discovered_count
# at lines 204/209/229); fails if guard removed.
# ---------------------------------------------------------------------------

@test "test_s18_08_discovery_scan_enumerates_and_counts_before_loop" {
  local tests_dir="${BATS_TEST_DIRNAME}"
  run bash -c '
    # (a) Verify pure-parse-invariant-gate.bats uses the real enumeration variable:
    #     discovered_count is incremented for each found BC and checked for zero.
    grep -q "discovered_count" "'"${tests_dir}"'/pure-parse-invariant-gate.bats" \
      || echo "FAIL: pure-parse-invariant-gate.bats does not contain discovered_count enumeration variable"

    # (b) Verify the empty-discovery guard is present — the gate must fail (exit 1)
    #     when discovered_count is 0, not silently pass over an empty set.
    grep -qE "discovered_count.*-eq.*0" \
      "'"${tests_dir}"'/pure-parse-invariant-gate.bats" \
      || echo "FAIL: pure-parse-invariant-gate.bats does not contain discovered_count -eq 0 empty-discovery guard"
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-005 / test_e18_story_behavioral_contracts_bc_ids_resolve_to_existing_bc_files
#
# Gate assertion — cross-reference title-cite parity:
#   For each E-18 story with a non-empty behavioral_contracts: array, verify:
#   1. Each BC ID matches pattern BC-\d+\.\d{2}\.\d{3}
#   2. A BC file for that ID exists in the spec tree
#   3. The BC file's H1 heading (line starting with # BC-) is present
#
# Expected: all BC files found; all H1 headings present; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_story_behavioral_contracts_bc_ids_resolve_to_existing_bc_files" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
    # For each E-18 story, extract behavioral_contracts IDs and verify BC files exist.
    #
    # Extraction is frontmatter-array-scoped — handles BOTH forms:
    #   Multi-line:  behavioral_contracts:\n  - BC-X.XX.XXX
    #   Inline list: behavioral_contracts: [BC-1.13.001]
    # The awk prints the key line itself (for inline [...] content) then sets f=1 for
    # continuation lines, stopping at the next top-level YAML key (^[a-z_]+:).
    # The [] empty-array guard prevents comment lines immediately following
    # `behavioral_contracts: []` from being scanned as continuation content.
    # Body prose BC mentions (e.g. BC-1.01.004 appearing in body text of S-18.14)
    # are NEVER extracted.
    #
    # H1 check accepts BOTH corpus forms:
    #   '"'"'# BC-NNN: Title'"'"'                     (103 files)
    #   '"'"'# Behavioral Contract BC-NNN: Title'"'"' (1870 files)
    # Both are valid per POLICY 7; the check verifies the H1 cites the right BC ID.
    for STORY_FILE in "'"${factory_root}"'/.factory/stories/S-18."*.md; do
      BC_IDS=$(awk '"'"'
        /^behavioral_contracts:/{
          if (/\[\]/) { next }
          print; f=1; next
        }
        /^[a-z_]+:/{if(f) exit}
        f{print}
      '"'"' "$STORY_FILE" | grep -oE '"'"'BC-[0-9]+\.[0-9]+\.[0-9]+'"'"' || true)
      for BC_ID in $BC_IDS; do
        BC_SS=$(echo "$BC_ID" | grep -oE "^BC-([0-9]+)" | grep -oE "[0-9]+")
        BC_SS_DIR=$(printf "ss-%02d" "$BC_SS")
        BC_FILE="'"${factory_root}"'/.factory/specs/behavioral-contracts/${BC_SS_DIR}/${BC_ID}.md"
        [ -f "$BC_FILE" ] || { echo "FAIL: $STORY_FILE references $BC_ID but $BC_FILE not found"; continue; }
        # Verify H1 heading is present in BC file — accept either corpus H1 form
        grep -qE "^# (Behavioral Contract )?${BC_ID}:" "$BC_FILE" \
          || { echo "FAIL: $BC_ID file exists but H1 does not contain '"'"'# BC-NNN:'"'"' or '"'"'# Behavioral Contract BC-NNN:'"'"' (either form is valid per POLICY 7)"; exit 1; }
      done
    done
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-006 / test_e18_spec_set_no_stale_current_wave_term_in_normative_sections
#
# Gate assertion — stale-term detector:
#   No normative behavioral claim in the E-18 spec set uses the retired
#   current_wave: field. Exclusion filter (S-18.09 extended, case-insensitive -Eiv)
#   covers two false-positive classes:
#   1. Historical/annotation mentions: Changelog, ADR cite, prior version,
#      was removed, phantom, retired, removed from
#   2. Negation/prohibition in normative sections: there is no, does not exist,
#      does NOT, MUST NOT, non-existent, no `current_wave, it does not
#
# Expected: STALE_HITS=0; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_spec_set_no_stale_current_wave_term_in_normative_sections" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
    STALE_HITS=$(grep -rEn "current_wave:" \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-04/" \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-05/" \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-06/" \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-07/" \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-08"[89]".md" \
      "'"${factory_root}"'/.factory/specs/verification-properties/VP-09"[01]".md" \
      2>/dev/null \
      | grep -Eiv "^.*(Changelog|changelog|ADR cite|prior version|was removed|phantom|retired|removed from|there is no|does not exist|does NOT|MUST NOT|non-existent|no \`current_wave|it does not|not stored as)" \
      | wc -l)
    [ "$STALE_HITS" -eq 0 ] || echo "FAIL: $STALE_HITS stale '"'"'current_wave:'"'"' references found in E-18 spec normative sections"
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-007 / test_e18_bc_preconditions_toml_blocks_have_canonical_name_and_plugin_fields
#
# Gate assertion — BC-precondition registry-block-shape validator (D-576):
#   Every [[hooks]] TOML block in §Preconditions of E-18 SS-04 BCs
#   (BC-4.14.001, BC-4.15.001) MUST contain BOTH:
#     name = "..."
#     plugin = "hook-plugins/....wasm"
#
# Flag-form awk avoids macOS BSD awk range-collapse when START matches END /^## /.
#
# Expected: both fields present in all TOML blocks; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_bc_preconditions_toml_blocks_have_canonical_name_and_plugin_fields" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
    for BC_FILE in \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-04/BC-4.14.001.md" \
      "'"${factory_root}"'/.factory/specs/behavioral-contracts/ss-04/BC-4.15.001.md"; do
      # Find the [[hooks]] block in the §Preconditions section
      # Flag form avoids macOS BSD awk range-collapse when START also matches END /^## /
      TOML_BLOCK=$(awk '"'"'/^## Preconditions/{f=1; next} /^## /{f=0} f'"'"' "$BC_FILE" \
        | grep -A 10 '"'"'\[\[hooks\]\]'"'"')
      # Assert both name and plugin fields present
      echo "$TOML_BLOCK" | grep -q '"'"'name = "'"'"' \
        || echo "FAIL: $BC_FILE §Preconditions [[hooks]] block missing name field (D-576)"
      echo "$TOML_BLOCK" | grep -q '"'"'plugin = "hook-plugins/'"'"' \
        || echo "FAIL: $BC_FILE §Preconditions [[hooks]] block missing plugin field (D-576)"
    done
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}

# ---------------------------------------------------------------------------
# AC-008 / test_e18_ac_traces_resolve_to_real_bc_clause_numbers
#
# Gate assertion — AC↔PC parity gate (O-P4-004 process gap):
#   For every E-18 story with non-empty behavioral_contracts:, every
#   (traces to BC-X.XX.XXX PC-N / INV-N) parenthetical in its AC section
#   MUST resolve to a real numbered clause in the cited BC file.
#
# Compound-cite-aware (F-P9-002): splits on +, ;, , (O-P12-001) with BC-ID carry-forward.
# AC-section-scoped (F-P9-003): only scans ## Acceptance Criteria section.
# Fenced-code-block-stripped (F-P10-003): strips content between ``` delimiters.
# Flag-form awk (F-P8-001): prevents macOS BSD awk range-collapse.
# printf path (F-P8-005; O-P8-A): ss-%02d for SS-10+ correctness; printf for non-vacuity.
# Multi-separator split +;, (O-P12-001): all three delimiters treated as segment separators.
# RAW_LABEL regex [^ )]+ (F-P11-001): captures hyphenated labels like PC-B-B1.
# BC_ARRAY extraction unquoted-frontmatter-tolerant (F-P1-001): mirrors AC-005 awk.
# TRACES_KEYLESS non-vacuity guard (F-P1-001): counts keyword-less PC-N/INV-N cites.
# Keyword-less CLAUSE_TYPE inference (F-P1-001): PC...→postcondition, INV...→invariant.
#
# Four label forms supported:
#   - Numeric:           "postcondition 1", "postcondition 2a", "invariant 3"
#   - PC-prefix numeric: "postcondition PC1" (strip "PC")
#   - PC-letter:         "postcondition PC-A", "postcondition PC-B-B1" (BC-4.15.001)
#   - Keyword-less:      "PC10", "PC6", "PC4", "INV-1" (F-P1-001 — CLAUSE_TYPE inferred)
#
# Expected: all clause cites resolve; TRACES_CHECKED > 0 for stories with BCs; no FAIL lines.
# ---------------------------------------------------------------------------

@test "test_e18_ac_traces_resolve_to_real_bc_clause_numbers" {
  local factory_root="$FACTORY_ROOT"
  run bash -c '
_resolve_clause() {
  # Args: BC_ID CLAUSE_TYPE RAW_LABEL STORY_FILE
  local BC_ID="$1" CLAUSE_TYPE="$2" RAW_LABEL="$3" STORY_FILE="$4"
  # Normalize label
  local NORM_LABEL LABEL_FORM
  if echo "$RAW_LABEL" | grep -qiE "^PC-[A-Z]"; then
    NORM_LABEL="$RAW_LABEL"; LABEL_FORM="letter"
  elif echo "$RAW_LABEL" | grep -qiE "^PC[0-9]"; then
    NORM_LABEL=$(echo "$RAW_LABEL" | sed '"'"'s/^[Pp][Cc]//'"'"'); LABEL_FORM="numeric"
  elif echo "$RAW_LABEL" | grep -qiE "^INV-?[0-9]"; then
    NORM_LABEL=$(echo "$RAW_LABEL" | sed '"'"'s/^[Ii][Nn][Vv]-\{0,1\}//'"'"'); LABEL_FORM="numeric"
  else
    NORM_LABEL="$RAW_LABEL"; LABEL_FORM="numeric"
  fi
  # Build BC file path (F-P8-005)
  local BC_SS BC_SS_DIR BC_FILE
  BC_SS=$(echo "$BC_ID" | grep -oE "^BC-([0-9]+)" | grep -oE "[0-9]+")
  BC_SS_DIR=$(printf "ss-%02d" "$BC_SS")
  BC_FILE="'"${factory_root}"'/.factory/specs/behavioral-contracts/${BC_SS_DIR}/${BC_ID}.md"
  [ -f "$BC_FILE" ] \
    || { echo "FAIL: $STORY_FILE cites $BC_ID but file not found at $BC_FILE"; return; }
  # Determine section heading
  local SECTION_HEAD="## Postconditions"
  [ "$CLAUSE_TYPE" = "invariant" ] && SECTION_HEAD="## Invariants"
  [ "$CLAUSE_TYPE" = "precondition" ] && SECTION_HEAD="## Preconditions"
  # Clause existence check — flag form (F-P8-001)
  local FOUND
  if [ "$LABEL_FORM" = "letter" ]; then
    FOUND=$(awk "/^${SECTION_HEAD}/{f=1; next} /^## /{f=0} f" "$BC_FILE" \
      | awk '"'"'/^```/{fence=!fence;next}!fence'"'"' \
      | grep -cE "(^|\*\*)${NORM_LABEL}(\*\*|[: ])")
  else
    FOUND=$(awk "/^${SECTION_HEAD}/{f=1; next} /^## /{f=0} f" "$BC_FILE" \
      | awk '"'"'/^```/{fence=!fence;next}!fence'"'"' \
      | grep -cE "^${NORM_LABEL}\. ")
  fi
  [ "$FOUND" -gt 0 ] \
    || echo "FAIL: $STORY_FILE cites $BC_ID ${CLAUSE_TYPE} ${RAW_LABEL} (normalized: ${NORM_LABEL}) but clause not found in BC ${SECTION_HEAD} section"
}

for STORY_FILE in "'"${factory_root}"'/.factory/stories/S-18."*.md; do
  # --- Non-vacuity check (POLICY 11 / TD-VSDD-059 anti-silent-inert) ---
  # BC_ARRAY extraction is unquoted-frontmatter-tolerant (F-P1-001): mirrors AC-005 awk,
  # handles quoted, unquoted, inline [...], and multiline YAML forms.
  HAS_BC=$(grep -oE "^behavioral_contracts:" "$STORY_FILE" | head -1 || true)
  if [ -n "$HAS_BC" ]; then
    BC_ARRAY=$(awk '"'"'
      /^behavioral_contracts:/{
        if (/\[\]/) { next }
        print; f=1; next
      }
      /^[a-z_]+:/{if(f) exit}
      f{print}
    '"'"' "$STORY_FILE" | grep -oE '"'"'BC-[0-9]+\.[0-9]+\.[0-9]+'"'"' || true)
    if [ -n "$BC_ARRAY" ]; then
      # Scope to AC section only (F-P9-003), then strip fenced code blocks (F-P10-003)
      AC_SECTION=$(awk '"'"'/^## Acceptance Criteria/{f=1; next} /^## /{f=0} f'"'"' "$STORY_FILE" \
        | awk '"'"'/^```/{fence=!fence;next}!fence'"'"')
      RAW_PARENS=$(printf '"'"'%s'"'"' "$AC_SECTION" \
        | grep -oiE '"'"'\(traces to [^)]+\)'"'"' 2>/dev/null || true)
      # Count parentheticals with a recognized clause reference — BOTH forms:
      #   (a) keyword form: contains precondition/postcondition/invariant
      #   (b) keyword-less form (F-P1-001): contains a PC-N or INV-N token after a BC-ID
      TRACES_KEYWORD=$(printf '"'"'%s'"'"' "$RAW_PARENS" \
        | grep -ciE '"'"'(precondition|postcondition|invariant)'"'"' || true)
      TRACES_KEYLESS=$(printf '"'"'%s'"'"' "$RAW_PARENS" \
        | grep -ciE '"'"'BC-[0-9]+\.[0-9]+\.[0-9]+[[:space:]]+(PC|INV)-?[0-9]'"'"' || true)
      TRACES_CHECKED=$(( TRACES_KEYWORD + TRACES_KEYLESS ))
      [ "$TRACES_CHECKED" -gt 0 ] \
        || { echo "FAIL: $STORY_FILE has non-empty behavioral_contracts but TRACES_CHECKED=0 — gate is vacuously passing (notation not recognized)"; exit 1; }
    fi
  fi

  # --- Per-parenthetical compound-cite resolution (F-P9-002) ---
  # Scope to AC section only (F-P9-003), then strip fenced code blocks (F-P10-003)
  AC_SECTION=$(awk '"'"'/^## Acceptance Criteria/{f=1; next} /^## /{f=0} f'"'"' "$STORY_FILE" \
    | awk '"'"'/^```/{fence=!fence;next}!fence'"'"')
  # Extract each full (traces to ...) parenthetical
  while IFS= read -r PAREN; do
    [ -z "$PAREN" ] && continue
    # Strip outer parens and "traces to " prefix (case-insensitive)
    CONTENT=$(echo "$PAREN" | sed '"'"'s/^(//;s/)$//;s/^[Tt][Rr][Aa][Cc][Ee][Ss] [Tt][Oo] //'"'"')
    # Carry-forward BC-ID: the first segment always carries a BC-ID;
    # subsequent +/;/, segments may omit the BC-ID and inherit the prior one.
    CARRY_BC=""
    # Split on '"'"'+'"'"', '"'"';'"'"', or '"'"','"'"' (O-P12-001: all three are valid segment separators)
    # Each segment may or may not carry a BC-ID
    while IFS= read -r SEG; do
      SEG=$(echo "$SEG" | sed '"'"'s/^ *//;s/ *$//'"'"')
      [ -z "$SEG" ] && continue
      # Check if this segment introduces a new BC-ID
      NEW_BC=$(echo "$SEG" | grep -oiE "BC-[0-9]+\.[0-9]+\.[0-9]+" | head -1)
      [ -n "$NEW_BC" ] && CARRY_BC="$NEW_BC"
      [ -z "$CARRY_BC" ] && continue
      # Extract clause type (precondition|postcondition|invariant) from this segment (keyword form)
      CLAUSE_TYPE=$(echo "$SEG" | grep -oiE "(precondition|postcondition|invariant)" \
        | head -1 | tr '"'"'[:upper:]'"'"' '"'"'[:lower:]'"'"')
      if [ -z "$CLAUSE_TYPE" ]; then
        # Keyword-less form (F-P1-001): infer CLAUSE_TYPE from PC-N / INV-N token after BC-ID.
        # Match a PCN / PC-N / INVN / INV-N token that appears directly in the segment
        # (possibly after the BC-ID or a space). PC... maps to postconditions; INV... to invariants.
        KEYLESS_TOKEN=$(echo "$SEG" | grep -oiE '"'"'(PC|INV)-?[0-9][0-9A-Za-z-]*'"'"' | head -1)
        if [ -n "$KEYLESS_TOKEN" ]; then
          case "$(echo "$KEYLESS_TOKEN" | tr '"'"'[:lower:]'"'"' '"'"'[:upper:]'"'"' | cut -c1-2)" in
            PC) CLAUSE_TYPE="postcondition" ;;
            IN) CLAUSE_TYPE="invariant" ;;
            *)  CLAUSE_TYPE="" ;;
          esac
          RAW_LABEL="$KEYLESS_TOKEN"
        fi
        [ -z "$CLAUSE_TYPE" ] && continue
      else
        # Keyword form: extract raw label as token immediately after the clause type keyword
        RAW_LABEL=$(echo "$SEG" | grep -oiE \
          '"'"'(precondition|postcondition|invariant) [^ )]+'"'"' \
          | grep -oE '"'"' [^ )]+$'"'"' | tr -d '"'"' '"'"')
      fi
      [ -z "$RAW_LABEL" ] && continue
      _resolve_clause "$CARRY_BC" "$CLAUSE_TYPE" "$RAW_LABEL" "$STORY_FILE"
    done <<< "$(echo "$CONTENT" | tr '"'"'+;,'"'"' '"'"'\n'"'"')"
  done <<< "$(printf '"'"'%s'"'"' "$AC_SECTION" | grep -oiE '"'"'\(traces to [^)]+\)'"'"' 2>/dev/null || true)"
done
  '
  assert_success
  refute_output --partial "FAIL"
  refute_output --partial "WARN"
}
