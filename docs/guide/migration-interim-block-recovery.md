# Recovering the `migrate-bc-index` interim block

Operator runbook for ADR-052 v1.23 item 11(e) and BC-1.18.011 v1.20 ("Operator recovery of the
interim block", EC-058). It describes what the code at S-25.09 does today.

> **This procedure is temporary.** S-25.06 AC-031 replaces it with a shared verify-then-finalize
> step inside `migrate-bc-index` that runs exactly steps 2-5 below under `flock(exclusive.lock)`.
> When AC-031 merges, the interim line below is retired and so is this page.

## Who may run this

- **A human, in a terminal outside any agent session.** Agents MUST NOT perform or script this
  procedure. The guard layer blocks agent writes to `.factory/migration-state/`, and the
  procedure writes there on purpose.
- **With no migration coordinator running.** `flock(exclusive.lock)` must be free (step 0).
- It applies **only** to the state described next. Any other `migrate-bc-index` failure has its
  own code; see the error taxonomy (`COMPLETION_RECORD_MISMATCH_ABORT`, `FOREIGN_MIGRATION_REFUSED`,
  `MIGRATION_LOCK_CONTENTION`, `MIGRATION_STATE_INTEGRITY_FAILURE`).

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
apply.

Exit codes you may see while working through this page: `0` (done, `ALREADY_MIGRATED`; nothing is
printed on success), `1` (`MIGRATION_LOCK_CONTENTION`: another coordinator holds the lock; no harm
done, retry after it exits), `2` (blocked or failed).

## Files involved

All under `.factory/migration-state/` (relative to the project root):

| File | Meaning |
|------|---------|
| `completed.json` | Terminal record: `generation_id`, `txn_id`, `completed_at`, `canonical_paths_count`. |
| `txn-<activation_id>.json` | The live txn record: `txn_id`, `activation_id`, `generation_id`, `state`, `pending_canonical_moves`, `intent_log_path`, `updated_at`, and others. `migration_id`, if present, is `migrate-bc-index`. |
| `gate-state.json` | Admission gate: the JSON string `"OPEN"`, `"DRAINING"` or `"LOCKED"`. |
| `exclusive.lock` | The advisory lock file. A running coordinator holds `flock` on it. |
| `intent-<generation_id>.log` | The intent log: a text file of `key=value` blocks between `INTENT_LOG_RECORD_V1` and `END_INTENT_LOG_RECORD`. Record types are `INTENT`, `DONE` and `ABORTED`. A `DONE` block carries `target_canonical` and `expected_post_hash`. |

## Procedure

Run every command from the project root. Stop at the first failed check, leave all files as they
are (apart from the snapshot you took), and escalate.

### 0. Preconditions

Confirm you are in a plain terminal, not an agent session, and set variables:

```bash
MS=.factory/migration-state
ls "$MS"                      # expect completed.json, txn-*.json, gate-state.json, exclusive.lock, intent-*.log
cat "$MS/gate-state.json"     # expect "LOCKED" or "DRAINING" (the gate is blocking)
```

Check that no coordinator holds the lock.

Linux (util-linux `flock`):

```bash
flock -n "$MS/exclusive.lock" true && echo "lock free" || echo "LOCK HELD - stop"
```

macOS (no `flock(1)` by default; use Python, or `brew install flock` and use the Linux command):

```bash
python3 -c 'import fcntl,sys; f=open(sys.argv[1],"a"); fcntl.flock(f, fcntl.LOCK_EX|fcntl.LOCK_NB); print("lock free")' "$MS/exclusive.lock" \
  || echo "LOCK HELD - stop"
```

If the lock is held, a coordinator is running. Wait for it to exit; do not continue. Find the
live txn file (there must be exactly one non-terminal one):

```bash
grep -l -E '"state": *"(STAGING|COMMITTING)"' "$MS"/txn-*.json
```

Set `TXN` to that file and `ACT` to its `activation_id`:

```bash
TXN=$MS/txn-<activation_id>.json       # fill in from the grep output
jq -r '.activation_id, .state, .generation_id' "$TXN"
```

If the grep returns zero files or more than one, stop and escalate.

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

Require all of the following. Any failure is a real mismatch.

- `completed.json.txn_id` equals the txn's recorded transaction id. For records this build writes,
  that is `txn-<activation_id>` (see "Where the spec and the code differ" below), and it must also
  agree with the txn file name `txn-<activation_id>.json` and the txn's `activation_id`.
- `completed.json.generation_id` equals the txn's `generation_id`, and that is a string (not
  `null`, not absent).
- The txn's `migration_id`, if present, is `migrate-bc-index`.
- `completed.json.canonical_paths_count` equals the number of `pending_canonical_moves`.

Scripted check (prints `IDENTITY OK` or the reason):

```bash
jq -n -r --slurpfile c "$MS/completed.json" --slurpfile t "$TXN" '
  ($c[0]) as $c | ($t[0]) as $t |
  if ($t.generation_id|type) != "string" then "FAIL: txn generation_id is not a string"
  elif $c.generation_id != $t.generation_id then "FAIL: generation_id differs"
  elif $c.txn_id != $t.txn_id then "FAIL: completed.json txn_id != txn txn_id"
  elif $t.txn_id != ("txn-" + $t.activation_id) then "FAIL: txn_id is not txn-<activation_id>"
  elif (($t|has("migration_id")) and $t.migration_id != "migrate-bc-index") then "FAIL: migration_id is not migrate-bc-index"
  elif $c.canonical_paths_count != ($t.pending_canonical_moves|length) then "FAIL: canonical_paths_count != number of pending_canonical_moves"
  else "IDENTITY OK" end'
```

Also confirm the file name matches: `[ "$TXN" = "$MS/txn-$ACT.json" ]`.

### 3. Content binding

For **every** `canonical_path` in `pending_canonical_moves`, find its **`DONE`** record in the
intent log and require `sha256(<that file>)` equal to the record's `expected_post_hash`. An
`INTENT` record alone does not count.

Intent log location: use `intent_log_path` from the txn if it is a non-null string. Builds at this
revision leave it `null`; then the log is `"$MS/intent-<generation_id>.log"` (see "Where the spec
and the code differ").

```bash
GEN=$(jq -r '.generation_id' "$TXN")
LOG=$(jq -r '.intent_log_path // empty' "$TXN"); LOG=${LOG:-$MS/intent-$GEN.log}
[ -f "$LOG" ] || { echo "no intent log at $LOG - stop"; }

# DONE records: "<target_canonical> <expected_post_hash>"
awk '/^INTENT_LOG_RECORD_V1$/{t="";h="";r="";next}
     /^record_type=/{r=substr($0,13)} /^target_canonical=/{t=substr($0,18)} /^expected_post_hash=/{h=substr($0,20)}
     /^END_INTENT_LOG_RECORD$/{if(r=="DONE")print t, h}' "$LOG"
```

Hash each canonical file.

macOS:

```bash
jq -r '.pending_canonical_moves[].canonical_path' "$TXN" | while IFS= read -r p; do
  actual=$(shasum -a 256 "$p" | awk '{print $1}')
  want=$(awk -v p="$p" '/^INTENT_LOG_RECORD_V1$/{t="";h="";r="";next}
       /^record_type=/{r=substr($0,13)} /^target_canonical=/{t=substr($0,18)} /^expected_post_hash=/{h=substr($0,20)}
       /^END_INTENT_LOG_RECORD$/{if(r=="DONE" && t==p)print h}' "$LOG" | tail -n 1)
  [ -n "$want" ] && [ "$actual" = "$want" ] && echo "OK   $p" || echo "FAIL $p (actual=$actual expected=${want:-<no DONE record>})"
done
```

Linux: identical, with `actual=$(sha256sum "$p" | awk '{print $1}')`.

Every line must be `OK`. A path with no `DONE` record, a missing intent log, or a differing hash
is a failure. If `target_canonical` is written differently from `canonical_path` (for example
absolute versus project-relative), compare the same file by hand; do not skip the path.

### 4. Finalize the txn (COMPLETED)

Only if steps 2 and 3 all passed. Rewrite the txn file with `state` set to `COMPLETED` and
`updated_at` set to the current UTC time (`YYYY-MM-DDTHH:MM:SSZ`). Every other key must keep its
value, including `schema_version`, `migration_id` and `pending_canonical_moves`. Write a temp file
in the **same directory**, then `mv` it over the original (atomic rename); never edit in place.

```bash
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
TMP=$(mktemp "$MS/.txn-finalize.XXXXXX")
jq --indent 2 --arg now "$NOW" '.state = "COMPLETED" | .updated_at = $now' "$TXN" > "$TMP" \
  && diff <(jq -S . "$TXN") <(jq -S . "$TMP")        # expect ONLY state and updated_at to differ
sync                                                  # flush the temp file before the rename
mv "$TMP" "$TXN" && sync                              # rename, then flush the directory entry
jq -r '.state' "$TXN"                                 # expect COMPLETED
```

Check the `diff` output before running the `mv`: exactly the `state` and `updated_at` lines may
differ. If anything else differs, delete `$TMP` and stop. (`sync` is the operator-level stand-in
for the fsync-file and fsync-directory the code uses.)

### 5. Open the gate (only after step 4 is durable)

The order is mandatory: the gate must never be OPEN while a live txn exists. `gate-state.json`
holds a JSON string; write the same encoding the file has now (`cat` it first), atomically:

```bash
cat "$MS/gate-state.json"                            # e.g. "LOCKED"
GTMP=$(mktemp "$MS/.gate-finalize.XXXXXX")
printf '"OPEN"' > "$GTMP"
sync
mv "$GTMP" "$MS/gate-state.json" && sync
cat "$MS/gate-state.json"                            # expect "OPEN"
```

### 6. Confirm

Re-run the migration binary by its pinned absolute path. It must exit **0** with no output and
perform zero writes (`ALREADY_MIGRATED`):

```bash
"$(pwd)/target/release/factory-dispatcher" migrate-bc-index; echo "exit=$?"     # expect exit=0
git -C .factory status --porcelain                  # unchanged by this run
```

Then confirm a normal `.factory/` write is admitted (for example, a trivial edit and revert of a
BC-INDEX-family file by a human, or let the next agent write proceed). If `migrate-bc-index` exits
`1`, a coordinator holds the lock: wait and retry. If it exits `2` again, a check was missed;
restore from the snapshot and escalate.

## When any check fails

1. **Stop.** Do not advance the txn and do not open the gate.
2. All files are unchanged except your snapshot. If anything was altered, restore:
   ```bash
   rm -rf "$MS" && cp -Rp "$SNAP/migration-state" "$MS"
   ```
   and restore the canonical paths from `$SNAP/canonical/` (or from pre-migration git history).
3. **Escalate.** A failed identity or content check means the on-disk migration is in an
   unproven state. Do not force it. Hand the snapshot, the output of steps 2-3 and the stderr
   line to the engine maintainers.

## Where the spec and the code differ

These were found while checking ADR-052 item 11(e) against `shard_manager.rs` at efd482d2. The
runbook above follows the code where a literal reading of the spec would reject a healthy record.

1. **`completed.json.txn_id`.** The spec says it must equal the txn's `activation_id`. The code
   writes `txn_id = "txn-<activation_id>"` in both the txn record and `completed.json`, so on a
   genuine record the two differ by the `txn-` prefix. Step 2 therefore checks `completed.json.txn_id`
   against the txn's `txn_id` (and that `txn_id` is `txn-<activation_id>`).
2. **`intent_log_path`.** The spec says to open the log at the txn's `intent_log_path`. The
   `migrate-bc-index` code never sets it (it stays `null`), and writes the log at
   `migration-state/intent-<generation_id>.log`. Step 3 uses that path when the field is null.
3. **Intent-log field name.** The spec speaks of each `canonical_path`'s `DONE` record; the log
   stores the path as `target_canonical`. Step 3 matches `canonical_path` to `target_canonical`.
4. **`ALREADY_MIGRATED`.** The success exit prints nothing; the token is not emitted. Success is
   exit 0 only.
5. **Durability.** The spec asks for write-temp, fsync, rename and directory fsync. A shell has no
   portable per-file fsync, so `sync` is used before and after the `mv`.
