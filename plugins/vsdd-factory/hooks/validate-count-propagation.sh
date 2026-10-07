#!/bin/bash
# validate-count-propagation.sh — PostToolUse lint hook for cross-document count drift
#
# Triggered: PostToolUse Write/Edit on ARCH-INDEX.md, BC-INDEX.md, VP-INDEX.md,
#            STATE.md, STORY-INDEX.md, PRD.md, or SS-NN-*.md architecture files.
#
# Behavior:
#   1. Extract count-bearing patterns from the modified file using anchored regexes.
#   2. Grep corpus index files for the same count-keyword pairs.
#   3. If the same keyword appears in a sibling document at a DIFFERENT numeric value,
#      emit a structured warning to stderr and exit 2.
#   4. Exit 0 on no drift OR if count is absent from a sibling file (absence != drift).
#
# Scope limit: reports drift, does not modify files, does not interpret semantics.
# Performance: O(n) two-pass pre-processing (awk length filter + sed ID-strip) before
# the per-line loop; <1s on real corpus files including BC-INDEX.md with 2595 lines
# and a single ~195KB last_amended: blob (S-25.01 CPU-runaway fix).
#
# S-7.02 / BC-7.05.001, BC-7.05.002

set -euo pipefail
# extglob is no longer needed: the ID-token strip moved to a whole-file sed pass
# (S-25.01 fix/count-propagation-cpu-runaway). Kept disabled to prevent accidental
# re-introduction of extglob patterns that could backtrack on long lines.

# Source canonical block-message helper if available (provides block_pre).
_SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_BLOCK_SH="${CLAUDE_PLUGIN_ROOT:+${CLAUDE_PLUGIN_ROOT}/hooks/lib/block.sh}"
_BLOCK_SH="${_BLOCK_SH:-${_SELF_DIR}/lib/block.sh}"
# shellcheck source=lib/block.sh disable=SC1091
if [ -f "$_BLOCK_SH" ]; then source "$_BLOCK_SH"; fi

if ! command -v jq &>/dev/null; then
  exit 0
fi

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Skip if no file path provided or file does not exist
if [[ -z "$FILE_PATH" ]] || [[ ! -f "$FILE_PATH" ]]; then
  exit 0
fi

# Only trigger for index/state/architecture files
BASENAME=$(basename "$FILE_PATH")
case "$BASENAME" in
  ARCH-INDEX.md|BC-INDEX.md|VP-INDEX.md|STATE.md|PRD.md|prd.md|STORY-INDEX.md) ;;
  SS-[0-9][0-9]-*.md) ;;
  *) exit 0 ;;
esac

# Resolve corpus root: walk up from file path to find known anchor
CORPUS_ROOT=""
DIR=$(dirname "$FILE_PATH")
while [[ "$DIR" != "/" ]]; do
  if [[ -d "$DIR/.factory" ]] || [[ -f "$DIR/STATE.md" ]] || [[ "$(basename "$DIR")" == ".factory" ]]; then
    CORPUS_ROOT="$DIR"
    break
  fi
  DIR=$(dirname "$DIR")
done
if [[ -z "$CORPUS_ROOT" ]]; then
  CORPUS_ROOT=$(dirname "$FILE_PATH")
fi

# Build list of sibling index files to check (only those that exist, excluding source file)
SIBLING_FILES=()
for candidate in \
  "$CORPUS_ROOT/.factory/STATE.md" \
  "$CORPUS_ROOT/.factory/specs/architecture/ARCH-INDEX.md" \
  "$CORPUS_ROOT/.factory/specs/behavioral-contracts/BC-INDEX.md" \
  "$CORPUS_ROOT/.factory/specs/verification-properties/VP-INDEX.md" \
  "$CORPUS_ROOT/.factory/stories/STORY-INDEX.md" \
  "$CORPUS_ROOT/STATE.md" \
  "$CORPUS_ROOT/ARCH-INDEX.md" \
  "$CORPUS_ROOT/BC-INDEX.md" \
  "$CORPUS_ROOT/VP-INDEX.md" \
  "$CORPUS_ROOT/STORY-INDEX.md"; do
  if [[ -f "$candidate" ]] && [[ "$candidate" != "$FILE_PATH" ]]; then
    SIBLING_FILES+=("$candidate")
  fi
done

# If no siblings found, nothing to compare against — exit clean
if [[ ${#SIBLING_FILES[@]} -eq 0 ]]; then
  exit 0
fi

# H2 sections whose count mentions are historical/frozen by project convention:
# changelog rows, phase-history rows, and decision-log rows. A count inside one
# of these ("Lists all 36 BCs" in a changelog entry, "PRD (38 BCs)" in a Phase
# Progress row) is an immutable record and MUST be allowed to disagree with the
# current count — comparing it as drift is the #567 false positive. Boundaries
# use the same "^## opens, next ^## closes" idiom as
# validate-changelog-monotonicity.sh.
#
# "## Drift Items" is included as defence-in-depth consistent with the historical
# heading convention: if a drift-items section ever appears earlier in a file than
# the authoritative live count, table cells in that section (e.g., stale snapshots
# of BCs/VPs counts at the time the item was recorded) must not shadow the live
# count. This does not change the verdict for any file in the current corpus —
# all live authoritative counts appear outside drift-items sections today.
_is_historical_heading() {
  case "${1,,}" in
    "## changelog"* | "## change log"* | "## historical content"* | \
    "## phase progress"* | "## decisions log"* | "## drift items"*)
      return 0 ;;
  esac
  return 1
}

# Extract count-bearing pairs from a file.
# Outputs lines of format: KEYWORD:COUNT:RANK
#   RANK 0 — authoritative (YAML frontmatter keys like total_bcs:, total_vps:)
#   RANK 1 — prose (pattern-matched text like "42 BCs")
# Rank-precedence is forward-looking hardening: if a corpus file ever has a
# rank-0 frontmatter key that appears after a rank-1 prose mention in file
# order, the rank-0 value still wins. No current corpus file exercises this
# today — VP-INDEX.md and BC-INDEX.md have total_vps: / total_bcs: frontmatter
# keys (rank-0) but they appear before any prose count mentions in file order,
# so rank precedence is not decisive in practice. STATE.md has no total_vps:
# or total_bcs: frontmatter keys at all.
#
# Supported patterns:
#   "NNN BCs" / "NNN,NNN BCs" — count before keyword (rank 1)
#   "BCs | NNN" / "BCs: NNN" — keyword before count (table or YAML) (rank 1)
#   "total_bcs: NNN" / "total_vps: NNN" — YAML frontmatter keys (rank 0)
# Count mentions inside historical H2 sections (see _is_historical_heading) are
# skipped so frozen records are never compared against the current count (#567).
_extract_counts() {
  local path="$1"
  local in_historical=0
  # Pre-process the file in two O(n) passes BEFORE the per-line loop so no
  # per-line operation ever sees a mega-line or a raw ID token:
  #
  #   Pass 1 — awk length filter: drop lines > 8192 chars.
  #     A legitimate count keyword ("42 BCs", "total_bcs: 42") is typically a
  #     short token. Additionally, lines > 8192 chars are frozen historical
  #     changelog records (e.g., last_amended: blobs) whose embedded counts are
  #     stale and must not shadow real totals — the explicit frontmatter skip
  #     below handles the named fields, but the length guard provides
  #     defence-in-depth for any other oversized record.
  #     Skipping (not truncating) avoids token-split artefacts: a
  #     mid-ID truncation could produce a phantom digit sequence.
  #     Semantic delta: pre-fix, the hook hung on >8192-char lines and never
  #     emitted a verdict; post-fix, it completes. The length filter removes
  #     oversized lines (typically frozen changelog blobs) from extraction
  #     scope, so stale counts embedded in those blobs no longer shadow
  #     authoritative frontmatter values. This is an intentional improvement,
  #     not a no-op.
  #     S-25.01 / fix/count-propagation-cpu-runaway.
  #
  #   Pass 2 — sed -E ID-strip: remove <letters>-<digits-and-dots> tokens
  #     (BC-1.18.003, VP-105, E-11, S-3, TD-001) so their embedded digits are
  #     not parsed as quantities (#690).  sed -E uses a provably-linear NFA;
  #     no backtracking is possible.  `sed -E` is portable to BSD sed (macOS)
  #     and GNU sed (Linux).
  #
  #     Contrast with the prior extglob form "${line//+([A-Za-z])-+([0-9.])/}":
  #     that form ran per-line inside the loop. On a single ~195KB line, bash's
  #     global-replace did O(n*k) string copies (n = line length,
  #     k = match count ≈ 8000), spinning a full core for >12s in PostToolUse
  #     and orphaning the process at PPID 1.
  #     A single whole-file sed pass has O(1) spawn cost and O(n) scan cost.
  #
  #   Error propagation: a process substitution `done < <(awk | sed)` silently
  #     discards awk/sed failures under set -e (the pipe runs in a subshell,
  #     so set -e does not propagate to the parent loop). Explicit temp file +
  #     || error path guarantees the pipeline failure is detected and the
  #     function returns 1 rather than silently producing zero counts
  #     (false-pass: "no drift found").
  #
  #   Heading lines (## …) are inspected AFTER both passes; canonical heading
  #   names used by _is_historical_heading contain no ID tokens, so the strip
  #   does not affect historical-section boundary detection.
  local _preproc_tmp
  # S-1 fail-closed hardening: only exit 2 is blocking under this project's
  # hook semantics (see below); mktemp failure must not degrade to a silent
  # non-blocking "no verdict" on a TMPDIR-exhausted / read-only-tmp host.
  # exit here (not return 1) so a mktemp failure inside this function cannot
  # be swallowed by a caller that forgets to gate on the return code.
  _preproc_tmp="$(mktemp)" || {
    echo "validate-count-propagation: mktemp failed in _extract_counts" >&2
    exit 2
  }
  trap 'rm -f "$_preproc_tmp"' RETURN
  awk 'length <= 8192' "$path" | sed -E 's/[A-Za-z]+-[0-9.]+//g' > "$_preproc_tmp" || {
    echo "validate-count-propagation: preprocessing pipeline failed for $path" >&2
    rm -f "$_preproc_tmp"
    return 1
  }
  while IFS= read -r line; do
    local count keyword
    # Skip YAML frontmatter changelog scalars — frozen historical records,
    # same rationale as _is_historical_heading (#567), which only reaches H2 sections.
    # The last_amended: and change: scalars embed stale counts that shadow current
    # totals; they should never be treated as the source-of-truth quantity.
    # S-25.01 review cycle 2 fix (BLOCKING-1).
    [[ "$line" =~ ^[[:space:]]*(last_amended|change): ]] && continue
    # Skip ONLY genuinely documentary/historical blockquote forms — not every
    # ">"-prefixed line (cycle-4 BLOCKING-C fix). A blanket "^>" skip is wrong:
    # BC-INDEX.md's SOLE carrier of the live "subsystems" count is a blockquote
    # banner line ("> Master index of all N behavioral contracts across M
    # subsystems."), so a blanket skip silently kills subsystems drift
    # detection for that file. Two blockquote FORMS are, by corpus convention,
    # reliably historical/documentary narrative rather than a live count:
    #
    #   1. Date-stamped changelog notes: "> Updated YYYY-MM-DD: ..." or the
    #      "> Updated in <label> (YYYY-MM-DD): ..." variant (STORY-INDEX.md
    #      changelog blockquotes). These record what happened on a past date;
    #      any count embedded in them (e.g., a per-burst point tally) is a
    #      frozen historical snapshot, not the current corpus total.
    #   2. Bold-prefixed narrative prose: "> **...**" (e.g., STORY-INDEX.md's
    #      "> **E-8 Tier 1 batch authoring (2026-04-30):** ... 22 BCs
    #      anchored ...", or wave/pass re-anchor summaries). These are
    #      per-story/per-wave/per-pass local tallies, not corpus totals, and
    #      were the original #567-class false-positive source this guard
    #      exists to suppress.
    #
    # A plain (non-bold, non-"Updated") blockquote line — such as the
    # BC-INDEX.md master-index banner — is NOT skipped: it falls through to
    # the normal Pattern A-D extraction below like any other line, so a live
    # count that happens to be blockquote-formatted is still checked for
    # drift.
    if [[ "$line" =~ ^[[:space:]]*'>' ]]; then
      if [[ "$line" =~ ^[[:space:]]*'>'[[:space:]]*Updated[[:space:]] ]] \
        || [[ "$line" =~ ^[[:space:]]*'>'[[:space:]]*\*\* ]]; then
        continue
      fi
    fi
    # Track historical-section boundaries; skip counts while inside one.
    if [[ "$line" =~ ^##[[:space:]] ]]; then
      if _is_historical_heading "$line"; then in_historical=1; else in_historical=0; fi
    fi
    [[ "$in_historical" -eq 1 ]] && continue
    # Pattern A: count before keyword (rank 1 — prose)
    if [[ "$line" =~ ([0-9][0-9,]+)[[:space:]]+(BCs|VPs|stories|capabilities|subsystems) ]]; then
      keyword="${BASH_REMATCH[2]}"
      count="${BASH_REMATCH[1]//,/}"
      echo "${keyword}:${count}:1"
    fi
    # Pattern B: keyword before count (table cell or colon-value) (rank 1 — prose)
    if [[ "$line" =~ (BCs|VPs|stories|capabilities)[[:space:]]*[|:][[:space:]]*([0-9][0-9,]+) ]]; then
      keyword="${BASH_REMATCH[1]}"
      count="${BASH_REMATCH[2]//,/}"
      echo "${keyword}:${count}:1"
    fi
    # Pattern C: YAML "total_bcs: NNN" (rank 0 — authoritative frontmatter)
    if [[ "$line" =~ total_bcs:[[:space:]]*([0-9][0-9,]+) ]]; then
      count="${BASH_REMATCH[1]//,/}"
      echo "BCs:${count}:0"
    fi
    # Pattern D: YAML "total_vps: NNN" (rank 0 — authoritative frontmatter)
    if [[ "$line" =~ total_vps:[[:space:]]*([0-9][0-9,]+) ]]; then
      count="${BASH_REMATCH[1]//,/}"
      echo "VPs:${count}:0"
    fi
  done < "$_preproc_tmp"
  rm -f "$_preproc_tmp"
}

# Extract counts from modified file using rank-based precedence.
# Frontmatter keys (rank 0) beat prose patterns (rank 1) for the same keyword.
# Rank-precedence is forward-looking hardening: should a file ever have a
# rank-0 entry appearing after a rank-1 entry in file order, the rank-0 value
# still wins. No current corpus file exercises this for STATE.md, but
# VP-INDEX.md and BC-INDEX.md both have total_vps: / total_bcs: frontmatter
# keys at rank 0.
declare -A SOURCE_COUNTS
declare -A COUNT_RANK
# S-1 fail-closed hardening: gate both mktemp calls to exit 2 explicitly.
# Under `set -e` a bare `var="$(mktemp)"` failure already terminates the
# script, but with mktemp's own exit status (1) rather than 2 — and this
# project's hook semantics treat ONLY exit 2 as blocking; any other non-zero
# code is silently non-blocking. Without this explicit gate, a
# TMPDIR-exhausted / read-only-tmp host would degrade the hook to
# "no verdict, not blocking" instead of failing closed.
_src_tmp="$(mktemp)" || { echo "validate-count-propagation: mktemp failed for source temp file" >&2; exit 2; }
_sib_tmp="$(mktemp)" || { echo "validate-count-propagation: mktemp failed for sibling temp file" >&2; exit 2; }
trap 'rm -f "$_src_tmp" "$_sib_tmp"' EXIT
if ! _extract_counts "$FILE_PATH" > "$_src_tmp"; then
  echo "validate-count-propagation: count extraction failed for $FILE_PATH" >&2
  exit 2
fi
while IFS=: read -r kw cnt rnk; do
  [[ -z "$kw" || -z "$cnt" ]] && continue
  _new_rank="${rnk:-9}"
  _stored_rank="${COUNT_RANK[$kw]:-9}"
  if [[ "$_new_rank" -lt "$_stored_rank" ]]; then
    SOURCE_COUNTS["$kw"]="$cnt"
    COUNT_RANK["$kw"]="$_new_rank"
  fi
done < "$_src_tmp"

# Nothing to compare if no count patterns found.
# Guard via ${arr[*]:-} rather than ${#arr[@]}: under `set -u`, expanding the
# element count of an *empty associative array* is itself an unbound-variable
# error in bash 4+. Two paths above can leave this array empty: the ID-token
# drop (an all-phantom line like "5 E-11 stories" with no genuine count) and
# the historical-section skip (a file whose only counts were changelog/
# phase-history rows). Either way this
# no-count path is now reachable and must exit 0 cleanly, not crash.
if [[ -z "${SOURCE_COUNTS[*]:-}" ]]; then
  exit 0
fi

DRIFT_DETECTED=0
DRIFT_MESSAGES=()

for keyword in "${!SOURCE_COUNTS[@]}"; do
  source_count="${SOURCE_COUNTS[$keyword]}"

  for sibling in "${SIBLING_FILES[@]}"; do
    # Extract best-ranked count for this keyword from sibling using rank precedence.
    # Read all entries (no early break) so a rank-0 entry appearing after a rank-1
    # entry in the same file is not missed.
    sib_count=""
    _sib_rank=9
    if ! _extract_counts "$sibling" > "$_sib_tmp"; then
      echo "validate-count-propagation: count extraction failed for $sibling" >&2
      exit 2
    fi
    while IFS=: read -r kw cnt rnk; do
      if [[ "$kw" == "$keyword" ]]; then
        _entry_rank="${rnk:-9}"
        if [[ "$_entry_rank" -lt "$_sib_rank" ]]; then
          sib_count="$cnt"
          _sib_rank="$_entry_rank"
        fi
      fi
    done < "$_sib_tmp"

    # Absence of keyword in sibling is NOT drift — only report mismatch
    if [[ -n "$sib_count" ]] && [[ "$sib_count" != "$source_count" ]]; then
      DRIFT_DETECTED=1
      DRIFT_MESSAGES+=("COUNT DRIFT DETECTED: '${source_count} ${keyword}' in $(basename "$FILE_PATH") but '${sib_count} ${keyword}' in $(basename "$sibling").")
      DRIFT_MESSAGES+=("  Run: grep -r \"${keyword}\" .factory/specs/ .factory/STATE.md to reconcile.")
    fi
  done
done

if [[ "$DRIFT_DETECTED" -eq 1 ]]; then
  # S2 fix: join ALL drift messages so every violation is visible at once, not just the first.
  _ALL_DRIFT=$(printf '%s; ' "${DRIFT_MESSAGES[@]}" | sed 's/; $//')
  _REASON="Count propagation drift detected in $(basename "$FILE_PATH"): $_ALL_DRIFT"
  _REC="Update the lagging index to match the source-of-truth count: BC-INDEX total_bcs, ARCH-INDEX subsystem counts, or STORY-INDEX bcs count — see the specific drift cited above"

  if declare -f block_pre >/dev/null 2>&1; then
    block_pre "validate-count-propagation" \
      "$_REASON" \
      "$_REC" \
      "count_propagation_drift"
    # block_pre exits 2; unreachable
  else
    echo "BLOCKED by validate-count-propagation: ${_REASON}. Fix: ${_REC}. Code: count_propagation_drift." >&2
    exit 2
  fi
fi

exit 0
