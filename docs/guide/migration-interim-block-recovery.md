# Recovering the `migrate-bc-index` interim block

Operator runbook for ADR-052 item 11(e) and BC-1.18.011 ("Operator recovery of the interim
block", EC-058). It describes what the code at S-25.09 does today. Checked against branch
`feature/S-25.09` at 0b8f08a7 (`shard_manager.rs`).

> **This procedure is temporary.** S-25.06 AC-031 replaces it with a shared verify-then-finalize
> step inside `migrate-bc-index` that runs exactly steps 2-5 below under `flock(exclusive.lock)`.
> When AC-031 merges, the interim line below is retired and so is this page.

> **Version note.** Step 3 reads the intent log in the layout the S-25.09 code writes
> (`INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD`). ADR-054's hardened intent-log
> format arrives in S-25.10, and S-25.12 updates this procedure to it (including the record
> checksum recipe and the `canonical_move_plan` field rename from S-25.11). Do not use this page
> against a build that has those stories; use the page as updated by S-25.12.

## Who may run this

- **A human, in a terminal outside any agent session.** Agents MUST NOT perform or script this
  procedure. The guard layer blocks agent writes to `.factory/migration-state/`, and the
  procedure writes there on purpose.
- **With no migration coordinator running.** `flock(exclusive.lock)` must be free (step 0).
- It applies **only** to the state described next. Any other `migrate-bc-index` failure has its
  own code; see the error taxonomy (`COMPLETION_RECORD_MISMATCH_ABORT` with a different message,
  `FOREIGN_MIGRATION_REFUSED`, `MIGRATION_LOCK_CONTENTION`, `MIGRATION_STATE_INTEGRITY_FAILURE`).

## When you hit this

`completed.json` exists beside a live transaction record of the `migrate-bc-index` migration
(state `COMMITTING`, or `STAGING`), and one or both of these happen:

1. `factory-dispatcher migrate-bc-index` exits **2** and prints exactly this one line to stderr:

   ```
   migrate-bc-index: BC-INDEX migration: the terminal record completed.json cannot be proven to describe the live txn (COMPLETION_RECORD_MISMATCH_ABORT, exit 2); no verification was performed in this build; txn and gate unchanged; operator investigation required
   ```

   The build performs no verification, no finalization and no gate flip: the txn and the gate are
   left as they were.
2. Admission blocks writes to `.factory/specs/behavioral-contracts/` (BC-INDEX) with an
   `E-MAINTENANCE-001` message ending in
   `(completion-record mismatch — operator investigation required)`.

If the line differs ("no verification was performed in this build" absent), this page does not
apply. In particular, a line of the form
`migrate-bc-index: BC-INDEX migration: I/O error at <path>/completed.json: <os error>` means
`completed.json` itself is unusable (a dangling or looping symlink, a permission or I/O error, a
directory, or content that is not a valid record). That is a different failure mode: do not use
this page; stop and escalate (step 0 re-checks this).

Exit codes you may see while working through this page: `0` (done, `ALREADY_MIGRATED`; nothing is
printed on success), `1` (`MIGRATION_LOCK_CONTENTION`: another coordinator holds the lock; no harm
done, retry after it exits), `2` (blocked or failed).

## Files involved

All under `.factory/migration-state/` (relative to the project root):

| File | Meaning |
|------|---------|
| `completed.json` | Terminal record: `generation_id`, `txn_id`, `completed_at`, `canonical_paths_count`. |
| `txn-<activation_id>.json` | The live txn record: `txn_id`, `activation_id`, `generation_id`, `state`, `pending_canonical_moves`, `intent_log_path`, `updated_at`, and others. `txn_id` equals `activation_id` (a bare UUID, no prefix). `migration_id`, if present, is `migrate-bc-index`. |
| `gate-state.json` | Admission gate: the JSON string `"OPEN"`, `"DRAINING"` or `"LOCKED"`. |
| `exclusive.lock` | The advisory lock file. A running coordinator holds `flock` on it. |
| `intent-<generation_id>.log` | The intent log: plain text, one block per record. Each block is `INTENT_LOG_RECORD_V1`, then the lines `txn_id=`, `fencing_generation=`, `record_type=` (`INTENT`, `DONE` or `ABORTED`), `target_canonical=`, `staging_path=`, `expected_post_hash=`, `expected_pre_state=`, `timestamp_utc=`, `record_checksum=`, then `END_INTENT_LOG_RECORD`. |

The record checksum in this build is the SHA-256 of these eight values joined by `|`, in this
order: `txn_id|fencing_generation|record_type|target_canonical|staging_path|expected_post_hash|expected_pre_state|timestamp_utc`
(`expected_pre_state` is the literal `MISSING` when there was none).

## Procedure

Run every command from the project root, in a plain `bash` or `zsh` terminal. **Stop at the first
failed check**: a block that prints a line beginning `STOP`, that exits non-zero, or that does not
print its `... OK` line has failed. On a failure leave all files as they are (apart from the
snapshot you took) and escalate (see "When any check fails").

### 0. Preconditions

Confirm you are in a plain terminal, not an agent session, and set variables:

```bash
MS=.factory/migration-state
ls "$MS"                      # expect completed.json, txn-*.json, gate-state.json, exclusive.lock, intent-*.log
cat "$MS/gate-state.json"     # expect "LOCKED" or "DRAINING" (the gate is blocking)
sha() { if command -v shasum >/dev/null 2>&1; then shasum -a 256; else sha256sum; fi | awk '{print $1}'; }
```

Check that no coordinator holds the lock.

Linux (util-linux `flock`):

```bash
flock -n "$MS/exclusive.lock" true && echo "lock free" || echo "STOP: LOCK HELD"
```

macOS (no `flock(1)` by default; use Python, or `brew install flock` and use the Linux command):

```bash
python3 -c 'import fcntl,sys; f=open(sys.argv[1],"a"); fcntl.flock(f, fcntl.LOCK_EX|fcntl.LOCK_NB); print("lock free")' "$MS/exclusive.lock" \
  || echo "STOP: LOCK HELD"
```

If the lock is held, a coordinator is running. Wait for it to exit; do not continue.

Confirm `completed.json` is a readable regular file (or a symlink to one) holding a valid record.
A dangling or looping symlink, a directory, an unreadable file or malformed content is a different
failure mode:

```bash
( [ -f "$MS/completed.json" ] && [ -r "$MS/completed.json" ] \
  && jq -e 'type == "object"
            and (.generation_id | type == "string")
            and (.txn_id | type == "string")
            and (.completed_at | type == "string")
            and (.canonical_paths_count | type == "number" and . >= 0 and . == floor)' \
        "$MS/completed.json" >/dev/null \
  && echo "completed.json OK" ) || echo "STOP: completed.json is not a readable, valid record - a different failure mode; escalate"
```

Find the live txn file. There must be exactly one non-terminal one:

```bash
LIVE=$(grep -l -E '"state": *"(STAGING|COMMITTING)"' "$MS"/txn-*.json 2>/dev/null || true)
if [ "$(printf '%s\n' "$LIVE" | grep -c .)" -eq 1 ]; then TXN=$LIVE; echo "TXN=$TXN"; else echo "STOP: expected exactly one live txn file, found: ${LIVE:-none}"; fi
```

If it printed `STOP`, escalate. Otherwise read it:

```bash
ACT=$(jq -r '.activation_id' "$TXN"); GEN=$(jq -r '.generation_id' "$TXN")
jq -r '.activation_id, .state, .generation_id' "$TXN"
```

### 1. Snapshot

Copy the whole directory and the affected canonical paths somewhere safe. The canonical paths are
the `canonical_path` values in the txn's `pending_canonical_moves`.

```bash
SNAP=$(mktemp -d "${TMPDIR:-/tmp}/migration-snapshot.XXXXXX")
cp -Rp "$MS" "$SNAP/migration-state"
jq -r '.pending_canonical_moves[].canonical_path' "$TXN" | while IFS= read -r p; do
  mkdir -p "$SNAP/canonical/$(dirname "$p")" && cp -p "$p" "$SNAP/canonical/$p"
done
echo "snapshot: $SNAP"
```

(If a `canonical_path` is absolute, the `dirname` under `$SNAP/canonical` simply mirrors it.)

### 2. Identity binding

Read the two records:

```bash
jq '{generation_id, txn_id, canonical_paths_count}' "$MS/completed.json"
jq '{txn_id, activation_id, generation_id, state, migration_id, pending_canonical_moves, intent_log_path}' "$TXN"
```

Require all of the following. Any failure is a real mismatch: stop.

- `completed.json.txn_id` equals the txn's `activation_id`, and the txn's own `txn_id` equals its
  `activation_id` too. Both are the bare activation id: a value such as `txn-<activation_id>` is
  NOT equal, it is a mismatch.
- The txn file name is `txn-<activation_id>.json` (the `txn-` prefix is part of the file name only).
- `completed.json.generation_id` equals the txn's `generation_id`, and that is a string (not
  `null`, not absent).
- The txn's `migration_id`, if present, is `migrate-bc-index`.
- `completed.json.canonical_paths_count` equals N, the number of `pending_canonical_moves`
  entries, and N is at least 1. An empty list fails this step; N is not a fixed number for
  `migrate-bc-index` (one entry per staged shard, sub-shard and manifest file plus `BC-INDEX.md`).

Scripted check (prints `IDENTITY OK`; any failure prints a `FAIL:` message from `jq` and `STOP`):

```bash
{ jq -n -r -e --slurpfile c "$MS/completed.json" --slurpfile t "$TXN" '
  ($c[0]) as $c | ($t[0]) as $t |
  if ($t.activation_id|type) != "string" then error("FAIL: txn activation_id is not a string")
  elif ($t.generation_id|type) != "string" then error("FAIL: txn generation_id is not a string")
  elif $c.generation_id != $t.generation_id then error("FAIL: generation_id differs")
  elif $c.txn_id != $t.activation_id then error("FAIL: completed.json txn_id != txn activation_id (a txn- prefix is a mismatch)")
  elif $t.txn_id != $t.activation_id then error("FAIL: txn txn_id != txn activation_id")
  elif (($t|has("migration_id")) and $t.migration_id != "migrate-bc-index") then error("FAIL: migration_id is not migrate-bc-index")
  elif ($t.pending_canonical_moves|type) != "array" or ($t.pending_canonical_moves|length) < 1 then error("FAIL: pending_canonical_moves is empty or not a list")
  elif $c.canonical_paths_count != ($t.pending_canonical_moves|length) then error("FAIL: canonical_paths_count != number of pending_canonical_moves")
  else "IDENTITY OK" end' \
  && [ "$TXN" = "$MS/txn-$ACT.json" ] && echo "file name OK"; } || echo "STOP: identity binding failed"
```

Expect `IDENTITY OK` and `file name OK` and no `STOP`.

### 3. Content binding

3a. **Locate the log.** `intent_log_path` in the txn MUST be a non-null string exactly equal to
`.factory/migration-state/intent-<generation_id>.log`. A null, absent, non-string or any other
value fails this step. Never substitute the derived path for a bad or missing value:

```bash
jq -e --arg want ".factory/migration-state/intent-$GEN.log" '.intent_log_path | type == "string" and . == $want' "$TXN" >/dev/null \
  && LOG=$(jq -r '.intent_log_path' "$TXN") && [ -f "$LOG" ] && [ -r "$LOG" ] && echo "intent log OK: $LOG" \
  || echo "STOP: intent_log_path is null, absent or not the recorded path for this generation, or the log is not a readable file"
```

If it printed `STOP`, escalate. (`LOG` is only set when the recorded value passed the check.)

3b. **Verify every record.** Walk the log and recompute each record's checksum. This writes one
line per block: `V|<the eight checksummed values>` for a complete, verifying block and `X|` for a
block that is unterminated or whose checksum does not match.

```bash
awk '
  /^INTENT_LOG_RECORD_V1$/ { inb=1; done=0; tx=fg=rt=tc=sp=ph=pre=ts=ck=""; next }
  inb && /^END_INTENT_LOG_RECORD$/ { done=1; print done, ck, tx "|" fg "|" rt "|" tc "|" sp "|" ph "|" pre "|" ts; inb=0; next }
  inb { i=index($0,"="); k=substr($0,1,i-1); v=substr($0,i+1)
        if(k=="txn_id")tx=v; else if(k=="fencing_generation")fg=v; else if(k=="record_type")rt=v
        else if(k=="target_canonical")tc=v; else if(k=="staging_path")sp=v; else if(k=="expected_post_hash")ph=v
        else if(k=="expected_pre_state")pre=v; else if(k=="timestamp_utc")ts=v; else if(k=="record_checksum")ck=v }
  END { if(inb) print 0, "-", "unterminated" }' "$LOG" \
| while read -r d ck input; do
    if [ "$d" = 1 ] && [ "$(printf '%s' "$input" | sha)" = "$ck" ]; then echo "V|$input"; else echo "X|"; fi
  done > "$SNAP/records.txt"
echo "verifying: $(grep -c '^V|' "$SNAP/records.txt")  failing: $(grep -c '^X|' "$SNAP/records.txt")"
```

Bytes after the last verifying record are a torn tail (treated as absent) ONLY IF no later block
verifies. If a failing block is followed by a verifying one, the log is corrupt mid-stream. At
least one record must verify, every verifying record must carry `txn_id` equal to `$ACT`, and no
record may be split on a `|` inside a path (each line must have exactly 9 `|`-separated fields
including the `V`):

```bash
{ awk '/^X\|/{x=1} /^V\|/&&x{bad=1} END{exit bad}' "$SNAP/records.txt" \
  && [ "$(grep -c '^V|' "$SNAP/records.txt")" -ge 1 ] \
  && ! awk -F'|' -v a="$ACT" '/^V\|/ && ($2 != a || NF != 9) {bad=1} END{exit !bad}' "$SNAP/records.txt" \
  && echo "LOG RECORDS OK"; } || echo "STOP: intent log is corrupt mid-stream, has no verifying record, or a record's txn_id is not the activation_id"
```

3c. **Bind the plan to the log, then the log to the files.** First, the set of
`canonical_path` values of `pending_canonical_moves` must equal the set of distinct
`target_canonical` values among the verifying `INTENT` records:

```bash
{ diff <(jq -r '.pending_canonical_moves[].canonical_path' "$TXN" | sort -u) \
       <(awk -F'|' '$1=="V" && $4=="INTENT" {print $5}' "$SNAP/records.txt" | sort -u) \
  && echo "PLAN = INTENT SET OK"; } || echo "STOP: pending_canonical_moves and the INTENT records name different canonical paths"
```

Then, for **every** `canonical_path` in `pending_canonical_moves`: the `INTENT` record for that
target must name the same `staging_path` as the plan entry; there must be exactly one distinct
`expected_post_hash` among its `INTENT` records and exactly one among its **`DONE`** records
(an `INTENT` alone does not count); and `DONE.expected_post_hash` == `INTENT.expected_post_hash`
== `sha256(<the canonical file>)`. A DONE hash that equals the file but not the INTENT proves
nothing.

```bash
jq -r '.pending_canonical_moves[] | [.canonical_path, .staging_path] | @tsv' "$TXN" \
| while IFS="$(printf '\t')" read -r p s; do
    ih=$(awk -F'|' -v p="$p" -v s="$s" '$1=="V" && $4=="INTENT" && $5==p && $6==s {print $7}' "$SNAP/records.txt" | sort -u)
    ia=$(awk -F'|' -v p="$p" '$1=="V" && $4=="INTENT" && $5==p {print $6}' "$SNAP/records.txt" | sort -u)
    dh=$(awk -F'|' -v p="$p" '$1=="V" && $4=="DONE" && $5==p {print $7}' "$SNAP/records.txt" | sort -u)
    actual=$(sha < "$p" 2>/dev/null)
    if [ "$(printf '%s\n' "$ih" | grep -c .)" -eq 1 ] && [ "$ia" = "$s" ] \
       && [ "$(printf '%s\n' "$dh" | grep -c .)" -eq 1 ] \
       && [ -n "$actual" ] && [ "$actual" = "$ih" ] && [ "$actual" = "$dh" ]; then
      echo "OK   $p"
    else
      echo "FAIL $p (file=${actual:-<unreadable>} intent=${ih:-<none>} done=${dh:-<none>} staging_in_log=${ia:-<none>} staging_in_plan=$s)"
    fi
  done | tee "$SNAP/content-check.txt"
N=$(jq '.pending_canonical_moves | length' "$TXN")
{ [ "$(grep -c '^OK ' "$SNAP/content-check.txt")" -eq "$N" ] && ! grep -q '^FAIL' "$SNAP/content-check.txt" \
  && echo "CONTENT OK"; } || echo "STOP: content binding failed"
```

Expect `CONTENT OK`. A path with no `DONE` record, a missing intent log, a differing hash or a
differing staging path is a failure. If `target_canonical` is written differently from
`canonical_path` (for example absolute versus project-relative) the plan/INTENT comparison above
fails; do not skip the path and do not edit the records. Stop and escalate.

### 4. Finalize the txn (COMPLETED)

Only if steps 2 and 3 all passed. Rewrite the txn file with `state` set to `COMPLETED` and
`updated_at` set to the current UTC time (`YYYY-MM-DDTHH:MM:SSZ`). Every other key must keep its
value, including `schema_version`, `migration_id` and `pending_canonical_moves`. Write a temp file
in the **same directory**, then `mv` it over the original (atomic rename); never edit in place.
The `mv` happens only if the temp file differs from the original in exactly `state` and
`updated_at`:

```bash
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
TMP=$(mktemp "$MS/.txn-finalize.XXXXXX")
if jq --indent 2 --arg now "$NOW" '.state = "COMPLETED" | .updated_at = $now' "$TXN" > "$TMP" \
   && [ "$(jq -r '.state' "$TMP")" = "COMPLETED" ] \
   && diff <(jq -S 'del(.state, .updated_at)' "$TXN") <(jq -S 'del(.state, .updated_at)' "$TMP") >/dev/null \
   && sync && mv "$TMP" "$TXN" && sync; then
  echo "TXN FINALIZED: $(jq -r '.state' "$TXN")"
else
  rm -f "$TMP"; echo "STOP: txn rewrite failed or changed more than state/updated_at; the txn file is unchanged"
fi
```

(`sync` is the operator-level stand-in for the fsync-file and fsync-directory the code uses.)

### 5. Open the gate (only after step 4 is durable)

The order is mandatory: the gate must never be OPEN while a live txn exists. Do this only if step 4
printed `TXN FINALIZED: COMPLETED`. `gate-state.json` holds one JSON string; write it atomically:

```bash
cat "$MS/gate-state.json"                            # e.g. "LOCKED"
if [ "$(jq -r '.state' "$TXN")" = "COMPLETED" ]; then
  GTMP=$(mktemp "$MS/.gate-finalize.XXXXXX")
  if printf '"OPEN"' > "$GTMP" && sync && mv "$GTMP" "$MS/gate-state.json" && sync; then
    cat "$MS/gate-state.json"; echo                  # expect "OPEN"
  else rm -f "$GTMP"; echo "STOP: gate write failed"; fi
else echo "STOP: txn is not COMPLETED; do not open the gate"; fi
```

### 6. Confirm

Re-run the migration binary by its pinned absolute path. It must exit **0** and perform zero
writes (`ALREADY_MIGRATED`). It prints nothing on a clean exit 0 (the exit status is the signal;
the token is not emitted):

```bash
"$(pwd)/target/release/factory-dispatcher" migrate-bc-index; echo "exit=$?"     # expect exit=0
jq -r '.state' "$TXN"; cat "$MS/gate-state.json"; echo                           # expect COMPLETED and "OPEN"
git -C .factory status --porcelain                  # unchanged by this run
```

Then confirm a normal `.factory/` write is admitted (for example, a trivial edit and revert of a
BC-INDEX-family file by a human, or let the next agent write proceed). If `migrate-bc-index` exits
`1`, a coordinator holds the lock: wait and retry. If it exits `2` again, a check was missed;
restore from the snapshot and escalate.

## Advisory lines you may see

A run that succeeds (exit status unchanged, normally 0) but is degraded prints one stderr line per
condition, shaped `migrate-bc-index: <TOKEN> (advisory): <subject>: <cause>; <what is true now>`.
They are not errors and not part of this procedure; they tell you what to look at:

| Token | Meaning |
|-------|---------|
| `GATE_OPEN_RESET_FAILED` | The migration completed (txn COMPLETED) but writing the gate OPEN failed. The gate stays as it was until the next admission check or `migrate-bc-index` run reconciles it. |
| `TERMINAL_TXN_ARCHIVE_FAILED` | Archiving a stale terminal txn record failed. The record stays in place and is skipped as non-live. |
| `STAGING_DIR_REMOVE_FAILED` | Removing an abandoned generation directory failed. The directory is inert; the txn is ABORTED. |
| `OVERSIZED_ROW_SUBSHARD` | A single BC row exceeded the shard cap and was emitted as its own over-cap sub-shard (`<bc_id>: sub-shard <id> body is <n> bytes, exceeding shard_cap_bytes <cap>`). The migration completed. |

## When any check fails

1. **Stop.** Do not advance the txn and do not open the gate.
2. All files are unchanged except your snapshot. If anything was altered, restore:
   ```bash
   rm -rf "$MS" && cp -Rp "$SNAP/migration-state" "$MS"
   ```
   and restore the canonical paths from `$SNAP/canonical/` (or from pre-migration git history).
3. **Escalate.** A failed identity or content check means the on-disk migration is in an
   unproven state. Do not force it. Hand the snapshot, the output of steps 0 and 2-3 and the
   stderr line to the engine maintainers.

## Where the spec and the code differ

Found while checking ADR-052 v1.25 item 11(e) against `shard_manager.rs` at 3c62c64e. The identity
and intent-log-path rules now agree (the code writes `txn_id = activation_id`, and refuses a live
record whose `intent_log_path` is not exactly the recorded generation path), so the runbook follows
the spec there. The remaining differences:

1. **Field name `pending_canonical_moves`.** The spec (item 11(e) step 2) says `canonical_move_plan`
   (the v1.24 rename). The S-25.09 code, and therefore the txn files this page reads, still use
   `pending_canonical_moves`; the rename is S-25.11's. This page uses the code's name.
2. **Intent-log format and checksum.** The spec (step 3, ADR-054 Decision 1.2/1.5) describes
   11-line record blocks verified with `sed -n "S,S+8p" ... | shasum -a 256`. The S-25.09 code
   writes `INTENT_LOG_RECORD_V1` ... `END_INTENT_LOG_RECORD` blocks whose `record_checksum` is the
   SHA-256 of the `|`-joined values (see "Files involved"), not of the raw lines, so the ADR-054
   recipe does not verify this build's logs. Step 3b uses the build's own formula. S-25.10 moves
   the code to the ADR-054 format; S-25.12 updates this page.
3. **Corruption handling.** The spec says a failing record followed by a verifying one is a corrupt
   log (stop). The code's `read_intent_log` silently skips any torn or mismatching block wherever
   it sits. Step 3b follows the spec.
4. **Intent-log field name.** The spec speaks of each `canonical_path`'s `DONE` record; the log
   stores the path as `target_canonical`. Step 3c matches `canonical_path` to `target_canonical`.
5. **Durability.** The spec asks for write-temp, fsync, rename and directory fsync. A shell has no
   portable per-file fsync, so `sync` is used before and after the `mv`.
