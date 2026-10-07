#!/usr/bin/env bash
# factory-lock-write.sh — write/renew/clear the factory_lock frontmatter block in STATE.md
#
# Implements BC-5.40.001 PC1 (schema correctness), PC2 (unlock clears block),
# PC4 (mid-burst TTL renewal), and Invariants 2/3 (TTL = 2700s, expires_at = now + 2700s).
#
# Usage:
#   factory-lock-write.sh acquire <state_md_path>
#   factory-lock-write.sh renew  <state_md_path>
#   factory-lock-write.sh clear  <state_md_path>
#
# Modes:
#   acquire — Write a new factory_lock block with:
#               holder    = $(git config user.email | tr -d '\n')
#               locked_at = ISO-8601 UTC timestamp of acquisition (YYYY-MM-DDTHH:MM:SSZ)
#               expires_at = locked_at + TTL_SECONDS (= now + 2700s)
#             Overwrites any existing factory_lock block.
#
#   renew   — Refresh factory_lock.expires_at = now + 2700s while preserving
#             locked_at and holder unchanged. No-op (exit 0) if factory_lock key is absent.
#
#   clear   — Remove the factory_lock key ENTIRELY from STATE.md frontmatter.
#             Key deletion, NOT null assignment (BC-5.40.001 PC2 / Invariant "not null").
#             After clear, grep for 'factory_lock' in the frontmatter MUST return non-zero.
#
# Arguments:
#   <state_md_path>  — absolute or relative path to the STATE.md file to modify (required)
#
# TTL constant (non-configurable per BC-5.40.001 Invariant 2 and AC-007):
#   TTL_SECONDS=2700 (45 minutes). MUST NOT be overridden via environment or arguments.
#
# Outputs:
#   stdout — human-readable status message on success
#   stderr — error message on failure
#
# Exit codes:
#   0 — operation succeeded
#   1 — error (invalid args, missing file, git config failure, write failure)

set -euo pipefail

# ---------------------------------------------------------------------------
# Temp-file registry + EXIT trap (F-P1-011).
# All mktemp calls in this script MUST go through _make_tmpfile so they are
# registered here and cleaned up on any exit (normal, set -e abort, or signal).
# ---------------------------------------------------------------------------
_TMPFILES=()
_make_tmpfile() {
  local template="$1"
  local path
  path="$(mktemp "$template")"
  _TMPFILES+=("$path")
  printf '%s' "$path"
}
_cleanup_tmpfiles() {
  local f
  for f in "${_TMPFILES[@]+"${_TMPFILES[@]}"}"; do
    [[ -e "$f" ]] && rm -f "$f"
  done
}
trap '_cleanup_tmpfiles' EXIT

# ---------------------------------------------------------------------------
# TTL constant — non-configurable (BC-5.40.001 Invariant 2)
# Canonical TTL: see crates/factory-lock-parse/src/lib.rs pub const TTL_SECONDS: u32 = 2700 (AC-012/ADR-046 F-006).
# ---------------------------------------------------------------------------
TTL_SECONDS=2700

# ---------------------------------------------------------------------------
# Input validation
# ---------------------------------------------------------------------------

usage() {
  printf 'usage: factory-lock-write.sh <acquire|renew|clear> <state_md_path>\n' >&2
  exit 1
}

if [[ $# -ne 2 ]]; then
  usage
fi

MODE="$1"
STATE_MD="$2"

if [[ "$MODE" != "acquire" && "$MODE" != "renew" && "$MODE" != "clear" ]]; then
  printf 'factory-lock-write: unknown mode %s — expected acquire, renew, or clear\n' "$MODE" >&2
  exit 1
fi

if [[ -z "$STATE_MD" ]]; then
  printf 'factory-lock-write: <state_md_path> is required\n' >&2
  exit 1
fi

if [[ ! -f "$STATE_MD" ]]; then
  printf 'factory-lock-write: STATE.md path not found: %s\n' "$STATE_MD" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Helper: format an epoch (seconds since 1970-01-01T00:00:00Z) as ISO-8601 UTC.
# Supports both BSD date (macOS) and GNU date (Linux).
# ---------------------------------------------------------------------------
_epoch_to_iso() {
  local epoch="$1"
  if date --version >/dev/null 2>&1; then
    # GNU date
    date -u -d "@${epoch}" +%Y-%m-%dT%H:%M:%SZ
  else
    # BSD date (macOS)
    date -u -r "${epoch}" +%Y-%m-%dT%H:%M:%SZ
  fi
}

# ---------------------------------------------------------------------------
# Helper: capture the current epoch ONCE and set two variables:
#   NOW_LOCKED_AT  — ISO-8601 UTC string for this moment
#   NOW_EXPIRES_AT — ISO-8601 UTC string for this moment + TTL_SECONDS
# Both are derived from a SINGLE clock read (BC-5.40.001 Invariant 3).
# ---------------------------------------------------------------------------
_capture_now_epoch() {
  local now_epoch
  now_epoch="$(date -u +%s)"
  NOW_LOCKED_AT="$(_epoch_to_iso "$now_epoch")"
  NOW_EXPIRES_AT="$(_epoch_to_iso "$(( now_epoch + TTL_SECONDS ))")"
}

# ---------------------------------------------------------------------------
# Helper: compute ISO-8601 UTC timestamp for the current moment + N seconds.
# Used by renew mode only (does not need to share epoch with locked_at).
# ---------------------------------------------------------------------------
_now_plus_seconds() {
  local delta="$1"
  local epoch
  epoch="$(date -u +%s)"
  _epoch_to_iso "$(( epoch + delta ))"
}

# ---------------------------------------------------------------------------
# Helper: capture the octal permission bits of a file portably.
# Returns a 3-digit octal string such as "644" or "600".
# Mirrors the BSD/GNU branch pattern used by _epoch_to_iso above:
#   GNU stat (Linux):   stat -c '%a' <file>
#   BSD stat (macOS):   stat -f '%Lp' <file>
# ---------------------------------------------------------------------------
_get_file_mode() {
  local file="$1"
  if stat --version >/dev/null 2>&1; then
    # GNU stat (Linux)
    stat -c '%a' "$file"
  else
    # BSD stat (macOS)
    stat -f '%Lp' "$file"
  fi
}

# ---------------------------------------------------------------------------
# Helper: remove factory_lock block (key + all 2-space-indented sub-fields)
# from STATE.md in-place using awk.
# FRONTMATTER-BOUNDARY-AWARE: only removes the key inside the frontmatter
# region (between the first and second --- fences). Body lines that begin
# with `factory_lock:` are preserved unchanged.
# This produces a clean key-deletion (not null assignment).
# ---------------------------------------------------------------------------
_remove_factory_lock() {
  local file="$1"
  local orig_mode
  orig_mode="$(_get_file_mode "$file")"
  local tmpfile
  tmpfile="$(_make_tmpfile "${file}.XXXXXX")"
  awk '
    BEGIN { fence=0; skip=0 }
    /^---$/ {
      fence++
      skip=0
      print
      next
    }
    fence == 1 && /^factory_lock:/ { skip=1; next }
    fence == 1 && skip && /^  /    { next }
    { skip=0; print }
  ' "$file" > "$tmpfile"
  chmod "$orig_mode" "$tmpfile"
  mv "$tmpfile" "$file"
}

# ---------------------------------------------------------------------------
# Helper: write/replace the factory_lock block inside YAML frontmatter.
# Strategy: strip any existing factory_lock block, then insert the new one
# immediately after the timestamp: line in the frontmatter, above last_amended:.
# Precondition: the frontmatter MUST contain a `timestamp:` line; if absent,
# the awk insertion trigger never fires, the factory_lock block is not written,
# and the acquire post-write assertion exits 1 with SchemaViolation (loud failure).
# ---------------------------------------------------------------------------
_write_factory_lock_block() {
  local file="$1"
  local holder="$2"
  local locked_at="$3"
  local expires_at="$4"

  # First, remove any existing factory_lock block
  _remove_factory_lock "$file"

  # Insert the new block immediately after the timestamp: line in the frontmatter
  # (ADR-032 §Third Deliverable placement mandate: factory_lock: must appear
  # directly after timestamp:, above last_amended:).
  # Frontmatter-region gate (front == 1), once-only insertion guard (inserted),
  # and print-$0-first/next pattern are consistent with _remove_factory_lock and
  # _update_expires_at sibling awk patterns.
  local orig_mode
  orig_mode="$(_get_file_mode "$file")"
  local tmpfile
  tmpfile="$(_make_tmpfile "${file}.XXXXXX")"
  awk -v holder="$holder" -v locked_at="$locked_at" -v expires_at="$expires_at" '
    BEGIN { front=0; inserted=0 }
    /^---$/ { front++ }
    front == 1 && /^timestamp:/ && !inserted {
      print $0
      print "factory_lock:"
      print "  holder: \"" holder "\""
      print "  locked_at: \"" locked_at "\""
      print "  expires_at: \"" expires_at "\""
      inserted=1
      next
    }
    { print }
  ' "$file" > "$tmpfile"
  chmod "$orig_mode" "$tmpfile"
  mv "$tmpfile" "$file"
}

# ---------------------------------------------------------------------------
# Helper: update factory_lock.expires_at in-place (for renew mode).
# FRONTMATTER-BOUNDARY-AWARE: only modifies the expires_at sub-key inside
# the frontmatter region (between the first and second --- fences).
# Replaces the existing expires_at line under factory_lock: with the new value.
# ---------------------------------------------------------------------------
_update_expires_at() {
  local file="$1"
  local new_expires_at="$2"
  local orig_mode
  orig_mode="$(_get_file_mode "$file")"
  local tmpfile
  tmpfile="$(_make_tmpfile "${file}.XXXXXX")"
  awk -v new_exp="$new_expires_at" '
    BEGIN { fence=0; in_lock=0 }
    /^---$/ {
      fence++
      in_lock=0
      print
      next
    }
    fence == 1 && /^factory_lock:/ { in_lock=1; print; next }
    fence == 1 && in_lock && /^  expires_at:/ {
      print "  expires_at: \"" new_exp "\""
      next
    }
    fence == 1 && in_lock && !/^  / { in_lock=0 }
    { print }
  ' "$file" > "$tmpfile"
  chmod "$orig_mode" "$tmpfile"
  mv "$tmpfile" "$file"
}

# ---------------------------------------------------------------------------
# Helper: normalize CRLF line endings to LF in-place.
# CRLF causes awk patterns like /^---$/ to fail (the line is `---\r`).
# Production-grade path: detect and normalize before any awk processing.
# ---------------------------------------------------------------------------
_normalize_crlf() {
  local file="$1"
  # Only normalize if CR bytes are actually present (avoid rewriting clean files).
  if tr -cd '\r' < "$file" | grep -q .; then
    local orig_mode
    orig_mode="$(_get_file_mode "$file")"
    local tmpfile
    tmpfile="$(_make_tmpfile "${file}.XXXXXX")"
    tr -d '\r' < "$file" > "$tmpfile"
    # Preserve original file mode on replacement (portable: no --reference).
    chmod "$orig_mode" "$tmpfile"
    mv "$tmpfile" "$file"
  fi
}

# ---------------------------------------------------------------------------
# Helper: validate that STATE.md has well-formed YAML frontmatter (opening
# and closing --- fences). Exits non-zero with a SchemaViolation message if
# the frontmatter is absent, single-fence, or otherwise malformed.
# ---------------------------------------------------------------------------
_validate_frontmatter() {
  local file="$1"
  local fence_count
  fence_count="$(grep -c '^---$' "$file" 2>/dev/null || true)"
  if [[ "$fence_count" -lt 2 ]]; then
    printf 'factory-lock-write: SchemaViolation — %s has malformed frontmatter (need two --- fences, found %s). Fix the frontmatter before acquiring the lock.\n' \
      "$file" "$fence_count" >&2
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Mode dispatch
# ---------------------------------------------------------------------------

case "$MODE" in

  acquire)
    # Normalize CRLF → LF first (F-P1-010): CRLF breaks awk /^---$/ pattern.
    _normalize_crlf "$STATE_MD"

    # Validate frontmatter structure before any write (PC1 SchemaViolation).
    _validate_frontmatter "$STATE_MD"

    # Validate git user.email is configured and non-empty.
    # (PC1: holder must be non-empty; git config failure exits 1 here.)
    if ! HOLDER="$(git config user.email 2>/dev/null | tr -d '\n')" || [[ -z "$HOLDER" ]]; then
      printf 'factory-lock-write: SchemaViolation — git config user.email is unset or empty. Run: git config user.email <your-email> to configure the lock holder.\n' >&2
      exit 1
    fi

    # Capture clock ONCE; derive both locked_at and expires_at from same epoch
    # (BC-5.40.001 Invariant 3: expires_at = locked_at + TTL_SECONDS exactly).
    _capture_now_epoch
    LOCKED_AT="$NOW_LOCKED_AT"
    EXPIRES_AT="$NOW_EXPIRES_AT"

    _write_factory_lock_block "$STATE_MD" "$HOLDER" "$LOCKED_AT" "$EXPIRES_AT"

    # Post-write assertion: factory_lock block MUST now exist in frontmatter.
    if ! awk '/^---$/{f++} f==1 && /^factory_lock:/{found=1} f>=2{exit} END{exit !found}' "$STATE_MD"; then
      printf 'factory-lock-write: SchemaViolation — factory_lock block was not written to frontmatter of %s. File may have malformed structure.\n' "$STATE_MD" >&2
      exit 1
    fi

    printf 'factory-lock-write: acquired lock for %s (expires %s)\n' "$HOLDER" "$EXPIRES_AT"
    ;;

  renew)
    # Normalize CRLF → LF first (F-R1-001): CRLF breaks awk /^---$/ pattern.
    _normalize_crlf "$STATE_MD"

    # No-op if factory_lock key is absent (frontmatter-scoped check).
    if ! awk '/^---$/{f++} f==1 && /^factory_lock:/{found=1} f>=2{exit} END{exit !found}' "$STATE_MD" 2>/dev/null; then
      printf 'factory-lock-write: no factory_lock block present — renew is a no-op\n'
      exit 0
    fi

    # The block must contain an expires_at sub-field to renew; a malformed block
    # (missing expires_at) is a RenewalMissed condition (BC-5.40.001 PC4).
    if ! awk '/^---$/{f++} f==1 && /^  expires_at:/{found=1} f>=2{exit} END{exit !found}' "$STATE_MD" 2>/dev/null; then
      printf 'factory-lock-write: RenewalMissed — factory_lock block is missing expires_at sub-field in frontmatter of %s. Cannot renew a malformed block.\n' "$STATE_MD" >&2
      exit 1
    fi

    NEW_EXPIRES_AT="$(_now_plus_seconds "$TTL_SECONDS")"
    _update_expires_at "$STATE_MD" "$NEW_EXPIRES_AT"

    # Post-renew assertion: expires_at must now reflect the new value.
    ACTUAL_EXPIRES="$(awk '/^---$/{f++} f==1 && /^  expires_at:/{gsub(/^  expires_at: *"?/,""); gsub(/"$/,""); print; exit} f>=2{exit}' "$STATE_MD")"
    if [[ "$ACTUAL_EXPIRES" != "$NEW_EXPIRES_AT" ]]; then
      printf 'factory-lock-write: RenewalMissed — expires_at was not updated in %s (expected %s, got %s).\n' \
        "$STATE_MD" "$NEW_EXPIRES_AT" "$ACTUAL_EXPIRES" >&2
      exit 1
    fi

    printf 'factory-lock-write: renewed lock expires_at to %s\n' "$NEW_EXPIRES_AT"
    ;;

  clear)
    # Normalize CRLF → LF first (F-R1-001): CRLF breaks awk /^---$/ pattern.
    _normalize_crlf "$STATE_MD"

    _remove_factory_lock "$STATE_MD"

    # Post-clear assertion: factory_lock key MUST now be absent from frontmatter.
    # Symmetry with acquire's post-write assert (F-R1-001 StaleNullBlock guard).
    if awk '/^---$/{f++} f==1 && /^factory_lock:/{found=1} f>=2{exit} END{exit !found}' "$STATE_MD" 2>/dev/null; then
      printf 'factory-lock-write: StaleNullBlock — factory_lock key is still present in frontmatter of %s after clear. File may have malformed structure.\n' "$STATE_MD" >&2
      exit 1
    fi

    printf 'factory-lock-write: factory_lock block removed (unlocked)\n'
    ;;

esac
