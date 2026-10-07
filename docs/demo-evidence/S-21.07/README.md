# Demo Evidence — S-21.07: validate-cross-site-correspondence

Story: `S-21.07-validate-cross-site-correspondence`
Governing BC: `BC-5.39.010`
Gate type: PostToolUse WASM governance hook run under the factory-dispatcher (CLI/terminal
product — no browser UI). Recorded with VHS as a terminal-cast, per the demo-recorder
CLI-product convention.

This directory holds a single, concise (≈23s) VHS recording that illustrates the gate's
three core dispositions against **real** dispatcher + WASM output — not a mock or a
narrated transcript. It is illustrative evidence, not exhaustive AC coverage: the
exhaustive per-AC coverage mapping (28 ACs across bats integration tests and Rust unit
tests) lives in the story's Test Plan
(`.factory/stories/S-21.07-validate-cross-site-correspondence.md`, "## Test Plan"
section) and in `plugins/vsdd-factory/tests/validate-cross-site-correspondence.bats`.

## Files

| File | Description |
|------|-------------|
| `AC-DISPOSITIONS-block-indeterminate-continue.gif` | Recording, GIF (for PR embed) |
| `AC-DISPOSITIONS-block-indeterminate-continue.webm` | Recording, WebM (for archival) |
| `AC-DISPOSITIONS-block-indeterminate-continue.tape` | VHS script source |
| `scenarios/indeterminate-index-corrupt/` | Checked-in fixture (well-formed UTF-8) for the INDETERMINATE scenario; the reproduction commands below append an invalid UTF-8 byte sequence to a *copy* of `BC-INDEX.md` at run time (see "Reproduction" below) — the committed fixture itself is not corrupted |

No shell script is committed in this directory (POLICY 21 — no new `.sh` files outside
`plugins/vsdd-factory/tests/fixtures/`). The exact commands that produced the recording
are inlined below as copy-pasteable fenced blocks instead of a checked-in harness script.

## Scenarios demonstrated

### 1. BLOCK — AC-001 / BC-5.39.010 §PC2b

**Fixture:** `plugins/vsdd-factory/tests/fixtures/validate-cross-site-correspondence/a1-index-ahead-of-primary/`
(the same fixture used by bats test `T-P6B (PC2b): BC-5.39.010 index-newer-than-primary
blocks with prescribed text`).

BC-INDEX.md's body-table row cites `v1.11` for BC-5.39.010, but the BC file's own
frontmatter `version:` is `"1.10"` — the index is *newer* than the primary artifact it
describes. Per §PC2b this is anomalous (no legitimate burst-ordering explains the index
advancing ahead of the file it cites), so the gate BLOCKs (exit code 2) with the
POLICY 14/18 prescribed message, surfaced via the dispatcher's `block_reason=` stderr
field.

```
exit code: 2
block_reason="BLOCKED by validate-cross-site-correspondence: [1] validate-cross-site-correspondence
[Class A Arm1]: BC-INDEX.md body-table row for BC-5.39.010 cites v1.11 but frontmatter
version: is "1.10" — index is newer than primary. This is anomalous: the index cannot
legitimately advance ahead of the BC it cites. Verify no index row was updated
out-of-burst or under the wrong BC path. Update per POLICY 14 leg 5. Fix: review and fix
all cross-site correspondence issues listed above, then retry the write.
Code: POLICY 14/18."
```

### 2. INDETERMINATE advisory — AC-028 / BC-5.39.010 §PC15b / §PC26 (v1.22 / ADV-RECON11-001)

**Fixture:** `scenarios/indeterminate-index-corrupt/` (checked in as well-formed UTF-8;
the reproduction commands below corrupt a copy of `BC-INDEX.md` at run time with an
invalid UTF-8 byte sequence, `\xff\xfe`, appended after the otherwise-valid content —
simulating a mis-encoded save of the index file).

The primary target (`BC-5.39.010.md`) decodes fine and its `version:` matches the last
known-good index row (`1.6`); the primary-target read path (§PC15a / §PC25) is not
exercised here. The *secondary* read target, `BC-INDEX.md`, succeeds as a raw byte read
(`host::read_file` returns `Ok(bytes)`) but fails `std::str::from_utf8` decode inside
`extract_bc_index_version_state`. This is the `BcIndexVersionState::IndexUnreadable`
state (added v1.22): the row/hash state for the BC is genuinely **indeterminate**, not
confirmed-absent, so the gate MUST NOT fall through to the `RowAbsent` BLOCK path (which
would misreport index corruption as a dropped BC registration). Disposition is a
*distinct* advisory (emitted via `host::log_warn`, visible in the dispatcher's internal
JSONL log as a `plugin.log` record with `"level":"warn"`) plus **Continue** (exit 0).

```
exit code: 0
internal log advisory (host::log_warn) record:
"message":"validate-cross-site-correspondence: BC-INDEX.md failed UTF-8 decode — row/hash
state for 'BC-5.39.010' is INDETERMINATE, not confirmed-absent. Fix: verify the index
file's encoding and re-save as UTF-8."
```

Note this is a genuinely distinct code path from Scenario 3 below — both scenarios exit
0 and produce no `block_reason`, but only this scenario emits the `IndexUnreadable`
advisory record in the internal log. AC-028 is Rust-unit-tested (not bats-integration
tested — see `arm_a1.rs` / `arm_b.rs`); this demo is the first fixture-driven,
real-dispatcher-invoked reproduction of the scenario.

### 3. CONTINUE (pass) — well-formed, consistent artifacts

**Fixture:** `plugins/vsdd-factory/tests/fixtures/validate-cross-site-correspondence/a1-current-index/`
(the A1 control fixture also used by several bats CONTROL tests).

BC-INDEX.md's row cites `v1.6`, matching the BC file's frontmatter `version: "1.6"`
exactly. No violation, no advisory — the gate is silent and the write proceeds normally.

```
exit code: 0
no block, no advisory — write proceeds normally
```

## Reproduction

Every block below is plain, copy-pasteable shell — run it directly in a terminal from
the repo/worktree root. Nothing here is a committed script (POLICY 21).

### 0. Build (once)

```bash
cargo build -p factory-dispatcher
cargo build --release --target wasm32-wasip1 -p validate-cross-site-correspondence
cp target/wasm32-wasip1/release/validate_cross_site_correspondence.wasm \
   plugins/vsdd-factory/hook-plugins/validate-cross-site-correspondence.wasm
```

### 1. Common setup (run once per shell session)

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
DISPATCHER="$REPO_ROOT/target/debug/factory-dispatcher"
[ -x "$DISPATCHER" ] || DISPATCHER="$REPO_ROOT/target/release/factory-dispatcher"
GUARD_WASM="$REPO_ROOT/plugins/vsdd-factory/hook-plugins/validate-cross-site-correspondence.wasm"
FIXTURE_BASE="$REPO_ROOT/plugins/vsdd-factory/tests/fixtures/validate-cross-site-correspondence"
DEMO_DIR="$REPO_ROOT/docs/demo-evidence/S-21.07"

_write_registry() {
  cat > "$1/hooks-registry.toml" <<'TOML'
schema_version = 2

[[hooks]]
name = "validate-cross-site-correspondence"
event = "PostToolUse"
tool = "^(Edit|Write|MultiEdit)$"
plugin = "hook-plugins/validate-cross-site-correspondence.wasm"
priority = 460
timeout_ms = 8000
on_error = "continue"
async = false

[hooks.capabilities.read_file]
path_allow = [
  ".factory/specs/behavioral-contracts/",
  ".factory/specs/verification-properties/",
  ".factory/stories/"
]
TOML
}

_run_dispatcher() {
  local work="$1"
  printf '{"event_name":"PostToolUse","tool_name":"Write","session_id":"demo","dispatcher_trace_id":"demo-trace","tool_input":{"file_path":".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md","content":""},"tool_response":{}}' \
    > "$work/envelope.json"
  VSDD_LOG_DIR="$work/.factory/logs" CLAUDE_PLUGIN_ROOT="$work" CLAUDE_PROJECT_DIR="$work" \
    "$DISPATCHER" < "$work/envelope.json" 1>"$work/stdout.txt" 2>"$work/stderr.txt"
}
```

### 2. Scenario 1 — BLOCK (AC-001 / PC2b)

```bash
WORK="$(mktemp -d)"
mkdir -p "$WORK/.factory/logs" "$WORK/hook-plugins"
cp "$GUARD_WASM" "$WORK/hook-plugins/validate-cross-site-correspondence.wasm"
_write_registry "$WORK"
cp -r "$FIXTURE_BASE/a1-index-ahead-of-primary/factory/." "$WORK/.factory/"

_run_dispatcher "$WORK"; rc=$?
echo "exit code: $rc"
grep -o 'block_reason=".*' "$WORK/stderr.txt" || echo "(no block_reason found)"

rm -rf "$WORK"
```

### 3. Scenario 2 — INDETERMINATE advisory (AC-028 / PC15b / PC26)

```bash
WORK="$(mktemp -d)"
mkdir -p "$WORK/.factory/logs" "$WORK/hook-plugins"
cp "$GUARD_WASM" "$WORK/hook-plugins/validate-cross-site-correspondence.wasm"
_write_registry "$WORK"
cp -r "$DEMO_DIR/scenarios/indeterminate-index-corrupt/factory/." "$WORK/.factory/"

# Corrupt BC-INDEX.md with an invalid UTF-8 continuation byte sequence, simulating a
# mis-encoded save. host::read_file succeeds as raw bytes; the decode failure happens
# inside extract_bc_index_version_state().
printf '\n<!-- CORRUPTION MARKER -->\xff\xfe\n' >> "$WORK/.factory/specs/behavioral-contracts/BC-INDEX.md"

_run_dispatcher "$WORK"; rc=$?
echo "exit code: $rc"
log="$WORK/.factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl"
grep '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
  | grep '"level":"warn"' \
  | grep -o '"message":"[^"]*"' \
  || echo "(no advisory record found)"

rm -rf "$WORK"
```

### 4. Scenario 3 — CONTINUE (clean pass)

```bash
WORK="$(mktemp -d)"
mkdir -p "$WORK/.factory/logs" "$WORK/hook-plugins"
cp "$GUARD_WASM" "$WORK/hook-plugins/validate-cross-site-correspondence.wasm"
_write_registry "$WORK"
cp -r "$FIXTURE_BASE/a1-current-index/factory/." "$WORK/.factory/"

_run_dispatcher "$WORK"; rc=$?
echo "exit code: $rc"
log="$WORK/.factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl"
if grep -q '"plugin_name":"validate-cross-site-correspondence"' "$log" 2>/dev/null \
   && grep '"plugin_name":"validate-cross-site-correspondence"' "$log" | grep -q '"level":"warn"'; then
  echo "UNEXPECTED: advisory present"
else
  echo "no block, no advisory — write proceeds normally"
fi

rm -rf "$WORK"
```

### Re-recording the tape

The committed `.tape` file's `Type` commands invoke `./docs/demo-evidence/S-21.07/run-scenario.sh`
— a harness script that was used to author this recording but is **not committed**
(POLICY 21 forbids new `.sh` files outside `plugins/vsdd-factory/tests/fixtures/`). To
re-record the exact tape as authored, first save the four blocks above (0–4) into a
local, uncommitted script at `docs/demo-evidence/S-21.07/run-scenario.sh` — wrapping
blocks 2/3/4 in `run_block` / `run_indeterminate` / `run_continue` shell functions
dispatched from `"$1"`, matching the `Common setup` + per-scenario blocks verbatim —
make it executable, then run:

```bash
vhs docs/demo-evidence/S-21.07/AC-DISPOSITIONS-block-indeterminate-continue.tape
```

Do **not** commit that local helper script; delete it after recording.

**Tooling note:** the VHS release installed in this environment (0.11.0) does not
reliably support the `Wait` / `Wait+Line` tape commands — the wait would hang and abort
the recording even though the awaited text was already visible on screen (verified via
`Screenshot` probes during authoring). The tape therefore uses fixed `Sleep` durations
instead of `Wait+Line`; each scenario's dispatcher invocation completes in well under
0.5s (measured with `/usr/bin/time`), so the sleeps are a generous safety margin, not a
guess. If re-recording on a newer VHS release, `Wait+Line` may be substituted back in
for tighter timing.
