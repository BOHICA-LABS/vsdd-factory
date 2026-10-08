# ADR-052 Intent-Log Text Encoding and `pending_canonical_moves` Semantics: Independent Validation

**Date:** 2026-10-08
**Project:** drbothen/vsdd-factory (self-referential)
**Researcher:** vsdd-factory:research-agent (adversarial mandate: evidence against each option sought as hard as evidence for it)
**Scope:** Two spec-vs-code divergences in `migrate-bc-index`, recorded as open items in ADR-052 §Downstream ("Code fixes owed to the implementer" item (3), and "Open item for the human"):
- **A.** Intent-log byte encoding: Decision 7b's `--- INTENT_LOG_RECORD v1 ---` / `key: value` / `--- END_RECORD ---` / `missing`, against the code's `INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD` / `MISSING`.
- **B.** `pending_canonical_moves` semantics: Decision 7a ("not yet completed") and 7c step 7d (rewrite the list per move), against the code (fixed plan; completion recorded only by intent-log `DONE` records).

**Artifacts read:**
- `.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`: §Decision 7a, 7b (record format, torn rule, WAL ordering, recovery decision table, fault-injection mandate), 7c steps 1-9; §Error Code Semantics "Branch C hash source" + rulings (i)-(v); item 11(e) manual verify-then-finalize procedure; §Downstream open items.
- `.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager.rs` and `shard_manager/migration_fs.rs` (feature/S-25.09 working tree; **it was being edited during this research**, see the snapshot note below). Cross-checked against `crates/factory-dispatcher/src/shard_manager.rs` on `develop` (main checkout).
- `.worktrees/S-25.06/crates/factory-dispatcher/src/shard_manager.rs` (the `backfill-append-logs` intent-log copy).
- `.worktrees/S-25.09/crates/factory-dispatcher/tests/bc_1_18_011_b2_migration_test.rs`, `.worktrees/S-25.06/crates/factory-dispatcher/tests/s2506_append_log_backfill_test.rs`.
- `.worktrees/S-25.09/docs/guide/migration-interim-block-recovery.md` (operator runbook).
- `.factory/stories/S-25.06-append-log-backfill-split-executor.md` (AC-031 verifier hash source).

**Snapshot note.** While this research was running, an uncommitted edit appeared in the S-25.09 worktree's `execute_canonical_path_moves`: the DONE record's `txn_id` is now taken from a `hack_txn` value scraped from the first `txn_id=` line of the log file through raw `std::fs::read_to_string`. On `develop`, the same field is `String::new()`. Both forms are analysed below (finding B-2). Every other anchor below is identical on `develop` and the worktree snapshot.

---

## Summary of Verdicts

| # | Question | Verdict | Short form |
|---|---|---|---|
| A | Intent-log text encoding | **CHANGE-BOTH (hardened format)** | The two encodings differ only in cosmetic tokens. Both have the same structural weaknesses, and the code's reader has **two real torn-record defects** that violate 7b's own "torn ⇒ absent" rule. Changing the code to the spec's tokens fixes nothing. Amending the spec to the code's tokens would make the defects normative. Recommended: amend 7b to the code's token vocabulary (it is already in two code copies, the tests and the runbook, and `=` has no whitespace-trimming ambiguity), **and** make the framing normative and fix it in the code: a line-anchored state-machine reader, a checksum over the raw record bytes, a strict value grammar that rejects LF/CR/control characters/non-UTF-8 paths at write time, torn-tail truncation before append, fail-closed on mid-log corruption, F_FULLFSYNC plus directory fsync on the WAL boundary, and **one shared implementation** for both migrations. Amending 7b requires the human's authorization under CLAUDE.md rule 12. |
| B | `pending_canonical_moves` semantics | **AMEND-SPEC-TO-CODE (fixed plan + DONE records), with three code fixes owed under 7b as already written** | A fixed plan plus an append-only completion log is the correct design: one source of truth, no extra crash windows, and no empty-list vacuous verification. A shrinking list adds N durable rewrites and a new disagreement window, and it carries no information the log does not already have. **However, the code is not internally consistent with the fixed-plan model on three paths:** (B-1) `TreatDone` skips writing a DONE record, so a crash between a rename and its DONE append produces a migration that completes and then fails its own verifier forever; (B-2) DONE records do not bind to the txn and do not re-confirm the INTENT hash; (B-3) an empty or tampered plan in a COMMITTING record completes vacuously with `canonical_paths_count: 0`. Field naming (`pending_` for a list that never shrinks) is a sub-decision for the human (rename recommended). |

---

## Codebase Grounding (anchors are function names, per TD-VSDD-091)

### Intent-log writer / reader (B2, `shard_manager.rs`)
- Constants `INTENT_LOG_RECORD_START = "INTENT_LOG_RECORD_V1\n"` and `INTENT_LOG_RECORD_END = "END_INTENT_LOG_RECORD\n"`.
- `append_intent_log_record` builds the block: START, then nine `key=value\n` lines (`txn_id`, `fencing_generation`, `record_type`, `target_canonical`, `staging_path`, `expected_post_hash`, `expected_pre_state` with `None` written as `MISSING`, `timestamp_utc`, `record_checksum`), then END. Paths are written via `Path::display()` (lossy for non-UTF-8 paths). **No value validation or escaping.** It then calls `Fs::append`.
- `intent_log_checksum_input`: `sha256` over `format!("{}|{}|{}|{}|{}|{}|{}|{}", ...)` of the **parsed field values** joined by `|`, **not over the on-disk bytes**.
- `read_intent_log`: `fs.read` → **`String::from_utf8(bytes)` on the whole file** (error ⇒ `BcIndexMigrationError::Io`) → `content.split(INTENT_LOG_RECORD_START).skip(1)` (an **unanchored substring split**, not line-anchored) → `raw_block.strip_suffix(INTENT_LOG_RECORD_END)` (no suffix ⇒ silently `continue`) → `parse_intent_log_block`.
- `parse_intent_log_block`: `body.lines()` → `split_once('=')` into a `HashMap` (**duplicate keys: the last one wins; unknown keys and lines with no `=` are silently ignored**) → the required keys are fetched → the checksum is recomputed from the parsed values → a mismatch returns `None` (treated as torn).
- `decide_intent_log_recovery`: `canonical == expected_post_hash` ⇒ `TreatDone`; `staging == post && canonical == expected_pre` ⇒ `RedoRename`; otherwise `FailClosed`.
- `migration_fs.rs` `Fs::append` (StdFs): `OpenOptions::create(true).append(true)` → `write_all` → **`sync_all()`** (plain `fsync`; **not** `F_FULLFSYNC` on macOS); **no parent-directory fsync** when the call creates the file.

### `backfill-append-logs` (S-25.06 worktree)
- `write_append_log_intent_record`, `parse_append_log_intent_log_block`, `read_append_log_intent_log` and `append_log_intent_log_checksum_input` are **copies** of the B2 functions ("mirrors ... EXACTLY in shape, retyped"). They reuse the same `INTENT_LOG_RECORD_START/END` constants. **The writer and format are shared by copy, not by code.** The two copies have already drifted: S-25.06's `finish_append_log_migration` writes DONE with `txn.txn_id` and `txn.fencing_generation`; B2's writes an empty/scraped `txn_id` and `0`. Both migrations write `migration_state_dir/intent-<generation_id>.log`, so the S-25.06 verifier (AC-031) reads files written by both writers.

### Move plan and recovery
- Fresh run (`run_bc_index_migration`): build `pending_moves` → `append_intent_records_for_pending_moves` (INTENT per target, before the swap) → `txn.pending_canonical_moves = pending_moves; write_txn_record` → `swap_current_generation_pointer_with_precommit_recheck` → `finish_committing_migration`.
- STAGING-resume (`recover()` `ResumeFromStaging` arm): if `CURRENT.json` already names this generation and txn, the persisted plan is used as-is. Otherwise `recompute_pending_canonical_moves_from_staged_generation` derives the plan from the immutable `gen-<id>/` contents, persists it, and re-appends INTENTs (duplicates are harmless; the lookup takes the most recent record per target).
- COMMITTING (`plan_recovery` → `RecoveryDecision::ForwardRecovery { pending: live.pending_canonical_moves.clone() }`; `recover()` arm): `decode_txn_record_strict` → `finish_committing_migration`.
- `finish_committing_migration`: `execute_canonical_path_moves(fs, &txn.pending_canonical_moves, ..)` → if `completed_count < plan.len()` ⇒ `BinaryIntegrityFailure` → otherwise `completed.json` with `canonical_paths_count: completed_count` → txn COMPLETED (checked) → gate OPEN (best-effort).
- `execute_canonical_path_moves`: reads the log once. For each move it takes the most recent record whose `target_canonical == canonical` (`.rev().find`): `TreatDone` ⇒ **`completed_count += 1; continue;` (no DONE appended)**; `RedoRename` ⇒ fall through; `FailClosed` ⇒ `break`. Then rename → `fsync_dir(parent)` → read canonical → `post_hash = sha256(canonical)` → append DONE with **`expected_post_hash: post_hash` (observed, not compared to the INTENT's)**, `expected_pre_state: None`, `txn_id: String::new()` (develop) / `hack_txn` (worktree snapshot), `fencing_generation: 0`.
- **Nothing in either migration ever shrinks `pending_canonical_moves`.** The only assignments are full-plan writes before the swap. S-25.06's `finish_append_log_migration` likewise iterates the full `txn.pending_canonical_moves` and has the same `TreatDone ⇒ continue` without a DONE.

### Consumers of the plan and the DONE records
- ADR-052 item 11(e) steps 2-3 and the runbook §2-§3 compute `canonical_paths_count == len(pending_canonical_moves)` and require, "for EVERY `canonical_path` in `pending_canonical_moves`", a **DONE** block with `sha256(file) == expected_post_hash` ("an INTENT record alone does not count").
- §Error Code Semantics "Branch C hash source": per canonical path, the DONE record; "a path with no DONE record ... is `canonical_hash_mismatch`". S-25.06 AC-031 uses the same rule.
- Runbook awk (`migration-interim-block-recovery.md` §3) extracts `record_type` / `target_canonical` / `expected_post_hash` between the markers. It **does not verify `record_checksum`**, although the runbook's own text and 11(e) step 3 say a bad checksum counts as absent.

### Test coverage for torn records
- B2 (`bc_1_18_011_b2_migration_test.rs`): `test_BC_1_18_011_intent_log_append_and_read_round_trip`, and `test_BC_1_18_011_intent_log_torn_trailing_record_treated_as_absent_never_partial` (one record, last 5 bytes cut, i.e. inside the END marker). That is all.
- S-25.06: `test_BC_1_18_013_PC5_intent_log_round_trips_and_torn_trailing_record_treated_as_absent`, `test_BC_1_18_013_PC5_intent_log_tampered_checksum_treated_as_absent`.
- **Not covered by either:** a torn record followed by a later valid append; garbage or NUL bytes after a valid record; invalid UTF-8 in the tail; truncation at every byte offset; values containing LF / CR / `=` / `|` / marker text; duplicate or unknown keys; a crash between rename and DONE append followed by verifier/runbook step 3; DONE hash differing from the INTENT hash.

---

## Question A: Intent-log text encoding

### External findings (verified; URLs in Top Sources)

| Format | Framing / torn-tail rule | Relevance |
|---|---|---|
| SQLite WAL | 32-byte header with salts; per-frame header with salt copies and a **cumulative checksum over header + content**. Recovery scans until EOF or the **first invalid frame** and keeps the last valid commit frame. Later bytes are ignored. | Valid prefix then stop. The checksum covers raw bytes, not a reconstructed string. |
| PostgreSQL WAL | Record header `xl_tot_len` + **CRC-32C over the record bytes**. A zero-filled header yields "invalid record length ... got 0", which marks end-of-WAL, not corruption. Replay stops at the first invalid record. | Length plus raw-bytes CRC; zero tails are expected and handled. |
| LevelDB / RocksDB log | 7-byte header: CRC-32C(type+data), length, type; FULL/FIRST/MIDDLE/LAST fragments. RocksDB `WALRecoveryMode`: `kTolerateCorruptedTailRecords` (an incomplete final record or trailing preallocation zeros is fine; corruption in the middle is an error), `kAbsoluteConsistency`, `kPointInTimeRecovery` (default: stop before the first inconsistency), `kSkipAnyCorruptedRecords`. RocksDB itself calls the tail-vs-middle distinction a heuristic. | Precedent for **"tail tolerated, middle fails closed"**, which is what the intent log needs. |
| git packed-refs / reflog | One LF-terminated line per record. The packed-refs parser **rejects an unterminated or malformed last record**. Git normalizes reflog messages so they contain no embedded newlines. Updates go through lock files, not atomic appends. | Line formats stay safe by **forbidding LF in values at write time**. |
| systemd journal | Binary objects with type/flags/size headers. When a writer finds corruption or an unclean file, it rotates to a new file and leaves the old one. | Never appends after a corrupted tail. |
| JSON Lines / RFC 7464 | JSONL: one JSON value per LF line; JSON escapes newlines; no standard recovery policy for an incomplete last line. RFC 7464: `RS json LF`; malformed elements are skipped to the next RS. | Escaping solves the value-delimiter problem. A truncated last line is detectable only as "invalid JSON". |
| Pillai et al., OSDI 2014 ("All File Systems Are Not Created Equal") | **No tested configuration gave an atomic multi-block append.** Prefix appends failed in ext2 and in writeback-mode ext3/ext4/reiserfs. File size can reach disk before content, producing **zeros or garbage, not just a short prefix**. ext4 delalloc and historical XFS NUL-file issues are documented. | Readers must expect **non-prefix tears and zero/garbage tails**, not only truncation. |
| Apple `fsync(2)` / SQLite `fullfsync` | macOS `fsync` does **not** flush the drive's volatile cache; `F_FULLFSYNC` does. | The WAL-boundary append uses plain `fsync` while the renames use `F_FULLFSYNC` (D-1232-OBL-2(a)). |
| POSIX / Linux `fsync(2)` | `fsync` on a file does not necessarily persist its **directory entry**; the parent directory must be fsynced. | The first `Fs::append` creates the log with no directory fsync. |

### Adversarial analysis

**The token choice does not decide anything.** `--- INTENT_LOG_RECORD v1 ---` / `key: value` / `--- END_RECORD ---` and `INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD` have the same structure: in-band text markers, a one-character separator, no escaping, and a checksum that is only as good as its input. Against the spec's tokens specifically: `key: value` invites YAML-style parsing (YAML treats `: ` and `#` in a path as syntax) and leaves "trim the space after the colon?" undefined. `key=value` split on the first `=` has neither problem, because no key contains `=`. `MISSING` and `missing` are equally safe: neither is 64-character lowercase hex (`sha256_hex` emits `{:02x}`), so neither can collide with a hash. For the runbook, both are equally easy to read and to awk. The deciding factor is cost: the code tokens already appear in two writer/reader copies, four tests and the runbook's awk. So **if** the spec is amended, use the code's tokens. That is a small preference, not the verdict.

**Real defects in the code's reader (they exist regardless of which tokens are used):**

1. **A-1 (HIGH): a valid record is lost when garbage follows it.** `read_intent_log` splits on the START substring and requires each piece to end exactly with END. Take an fsynced, valid record R1, followed by a torn append whose surviving bytes do **not** include a complete START line: a partial START (`INTENT_LOG_RE`), NUL bytes from a size-before-data tear (Pillai et al.), or any garbage. The next run then appends R3. The file is `START R1 END <garbage> START R3 END`. `split` yields `"R1 END <garbage>"` and `"R3 END"`. **`strip_suffix(END)` fails on the R1 piece, so a durable, valid R1 is discarded.** In this design the writer never truncates a torn tail before appending, so torn bytes are always sandwiched by the next append. Consequences: if R1 was the DONE for target T, the most recent surviving record for T is its INTENT, `TreatDone` fires, and with B-1 below T never gets a DONE again. Completion then succeeds and the verifier, admission Branch C and runbook step 3 fail closed **permanently** (`canonical_hash_mismatch`). If R1 was an INTENT whose rename had not happened, `decide_intent_log_recovery(.., None)` ⇒ `FailClosed`, a permanent halt in COMMITTING. This is safe (nothing is wrongly accepted, thanks to the checksum) but it is a liveness and verification defect, and it directly contradicts 7b ("Recovery reads from the last valid record").
2. **A-2 (HIGH): one invalid UTF-8 byte makes the whole log unreadable.** `String::from_utf8` is applied to the entire file. A tail that is not valid UTF-8 (random garbage, or a tear in the middle of a multi-byte character in a non-ASCII path) makes `read_intent_log` return `Io`. Every later `finish_committing_migration` then fails with exit 2. 7b says a torn record is **absent**; here a torn record makes **every** record absent and the run errors. The code has no repair path.
3. **A-3 (MEDIUM): the writer accepts values the reader cannot parse.** A `target_canonical` or `staging_path` containing LF (legal in Unix paths) is written raw. The reader splits it into two lines and the checksum fails, so the record silently disappears on read **after the writer reported a durable success**. A trailing CR before the LF is stripped by `str::lines` (it treats `\r\n` as a line ending), with the same result. A non-UTF-8 path goes through `display()` / `to_string_lossy` as U+FFFD on both sides. The checksum still matches, so the record **validates** while naming a different path from the real one. The plan itself is also built with `to_string_lossy` (`recompute_pending_canonical_moves_from_staged_generation`), so the rename would target the wrong path. These paths derive from the project root plus fixed shard filenames, so likelihood is low, but nothing rejects them. The production-grade fix is to **reject at plan-build time and at write time**, with a named error and nothing appended.
4. **A-4 (MEDIUM): the checksum does not cover the bytes.** `intent_log_checksum_input` hashes `|`-joined parsed values. As a result (a) inserted lines, duplicated keys (HashMap last-wins) and unknown keys are **not detected** as long as the surviving values reproduce the checksum; (b) the `|` join is ambiguous (`a|b`+`c` = `a`+`b|c`), a canonicalization flaw; (c) **an operator cannot verify the checksum with standard tools**, and the runbook does not try. Every prior-art format above (SQLite, PostgreSQL, LevelDB) checksums the raw record bytes.
5. **A-5 (MEDIUM): the WAL boundary is weaker than the moves it protects.** `Fs::append` uses `sync_all` (macOS: host-to-drive only) and does not fsync the parent directory when it creates the log. After a power loss on macOS, a later rename made durable with `F_FULLFSYNC(dir)` can survive while its INTENT record does not. Recovery then fails closed (safe, but the WAL guarantee "after this fsync, every rename is recoverable" does not hold). D-1232-OBL-2(a) covers canonical-path mutations. The WAL boundary is the precondition for those mutations and should meet the same standard.
6. **A-6 (MEDIUM, governance): the format is shared by copy.** Every fix above has to be made twice (TD-VSDD-060). The DONE-`txn_id` drift (B-2) shows the copies have already diverged.
7. **Unanchored START split (LOW).** A START line is recognized anywhere, not only at a record boundary. Today this needs an LF inside a value (blocked by the fix to A-3), but the reader should be a line-anchored state machine anyway.

**Values equal to markers.** A path ending in `END_INTENT_LOG_RECORD` is harmless, because the suffix check applies to the whole block and the last line is always the real END. A path containing `INTENT_LOG_RECORD_V1` followed by LF needs an LF, which is A-3. Once values are restricted to no control characters, no value can contain a marker **line**.

**Is a different format warranted (JSON Lines, length-prefixed, binary)?**
- *Binary length-prefixed with CRC (LevelDB style):* the most robust, but it ends operator readability. The runbook and the 11(e) manual procedure depend on plain text and awk. Rejected.
- *JSON Lines + per-record checksum:* escaping is solved. But ruling (iv) says "plain text, never JSON". A checksum over canonical JSON needs RFC 8785 canonicalization or a `<hash> <json>` line form, and non-UTF-8 paths still have to be rejected because JSON strings are Unicode. It gives no safety gain over a line format once values are validated, and it would reverse a ratified ruling. Rejected as the default; recorded as an option.
- *Hardened line format (recommended):* keep the code's tokens and the human-readable `key=value` lines. Add (i) a strict value grammar enforced at write time, (ii) a checksum over raw bytes, (iii) a line-anchored reader that tolerates a torn tail and fails closed on mid-log corruption (the RocksDB `kTolerateCorruptedTailRecords` semantics), and (iv) truncation of the torn tail before append (PostgreSQL overwrite-from-end, systemd "never append after corruption").

### Verdict: **CHANGE-BOTH (hardened format)**

Neither option as posed is production-grade. "Change code to spec" only renames tokens and keeps A-1..A-5. "Amend spec to code" makes A-1..A-5 normative. The human must authorize the 7b amendment (CLAUDE.md rule 12). The code changes are owed regardless, because A-1/A-2 already violate 7b's existing "torn ⇒ absent" rule.

---

## Question B: `pending_canonical_moves` semantics

### External findings (verified)

| Prior art | What it does with plan vs progress |
|---|---|
| ARIES (Mohan et al. 1992) | The log is authoritative. Analysis **rebuilds** the transaction and dirty-page tables from the last fuzzy checkpoint **plus** the log; the checkpoint tables are derived and never trusted alone. Redo is guarded by `pageLSN < LSN`, so it is idempotent. CLRs record undo progress so interrupted rollback is not repeated. |
| git sequencer (`sequencer.c` `pick_commits` → `save_todo`) | The textbook shrinking list. `save_todo` rewrites `git-rebase-todo` **without** the current command and appends it to `done` **before** executing it. So `done` means *attempted*, not *completed*. Git does not update the two files atomically, and resolves a crashed or conflicted step interactively (`--continue`, `--edit-todo`, `--reschedule-failed-exec`). **Even git's shrinking list is not a completion ledger.** |
| dpkg (`lib/dpkg/dbmodify.c`) | Each package stanza change is written as a numbered file in `/var/lib/dpkg/updates/` (temp, sync, rename, dir sync), then **merged into `status` at checkpoint**, and only then are the update files removed. Progress is a per-item state machine kept in a journal and compacted later. It does not mutate a work list. |
| rpm `rpmts` | An ordered transaction set. **No documented resume-from-element**. `--rebuilddb` rebuilds the database and does not resume a plan. Not useful as precedent either way. |
| Kubernetes API conventions | `spec` = desired state, `status` = observed state. Controllers are level-triggered and should not rewrite `spec` to report progress. Status should be reconstructable from observation. |
| Helland "Immutability Changes Everything"; Kleppmann "Turning the database inside-out"; Fowler "Event Sourcing" | The log is the truth and state is a rebuildable view. Two independently authoritative stores (database + cache) race. *Inference, not a quotation:* "remaining = plan − completed" should be derived, not stored. |

### Adversarial analysis

**Arguments for the shrinking list (CHANGE-CODE-TO-SPEC), steel-manned:**
1. The txn record alone tells an operator what remains, without parsing the log.
2. The spec says so (rule 12: the spec wins by default).
3. git rebase uses a todo list, which shows the pattern works.

**Why each fails here:**
1. *Rewrite count and atomicity.* Each shrink is a full durable txn rewrite (write-temp, `F_FULLFSYNC`, rename, `F_FULLFSYNC(dir)`): **N extra durable rewrites**, where N = shards + sub-shards + manifests + `BC-INDEX.md`. Each one is a new crash point that the 7b fault-injection mandate requires tests to cover. The DONE append is already one `O_APPEND` write plus fsync.
2. *A new disagreement window.* 7c step 7 orders c (DONE) before d (shrink). A crash between them leaves the DONE present and the target still "pending". Recovery then needs a rule saying the log wins. Once that rule exists, the list is a cache of the DONE set and adds no information. In the other window (rename done, DONE not yet appended) the list and the log agree, and recovery must use on-disk hashes either way. **The shrinking list never decides a case the log cannot.**
3. *The plan is lost from the txn.* After shrinking, N and "every canonical path" can only come from the INTENT records. Every v1.23 consumer (11(e) steps 2-3, the runbook, Branch C, S-25.06 AC-031) enumerates from the list. **A list that has shrunk to empty passes verification vacuously.** `canonical_paths_count` (written from `completed_count` after the moves) would equal nothing it could be checked against. The architect's open item names exactly this risk, and the code confirms it: `recover()` → `ForwardRecovery` → `finish_committing_migration` with an empty plan writes `completed.json` with count 0 and returns `Completed` (finding B-3 below, already reachable today through a tampered or corrupt record).
4. *The precedent is weak.* git moves a command to `done` **before** running it, so its shrinking list cannot tell "done" from "attempted". It relies on a human at `--continue`. The precedents that run unattended (ARIES, dpkg, Kubernetes) keep the plan or desired state stable and record progress separately, deriving what remains.
5. *The operator view.* The runbook already reads the log for content binding (step 3), so a shrinking list saves the operator nothing.

**Arguments against the fixed plan (looking for paths where it is wrong):**

- **B-1 (HIGH, code wrong on a fixed-plan path; violates 7b as written): `TreatDone` does not write a DONE record.** 7b's recovery table row 1 says "Treat DONE; **append DONE record**". `execute_canonical_path_moves` (and S-25.06's `finish_append_log_migration`) only count and `continue`. Scenario: rename T succeeds and the dir fsync succeeds, then the process crashes before the DONE append (exactly the window the intent log exists for). Next run: the most recent record for T is its INTENT, canonical hash = post ⇒ `TreatDone` ⇒ counted ⇒ all moves counted ⇒ `completed.json` written ⇒ txn COMPLETED ⇒ exit 0. Then every verifier (S-25.06 AC-031, admission Branch C, runbook step 3, ruling "a path with no DONE record ⇒ `canonical_hash_mismatch`") finds no DONE for T ⇒ **fail-closed block on a correctly finished migration, with no automated repair**. Finding A-1 reaches the same state by losing a valid DONE. Under the fixed-plan model DONE records are the **only** completion evidence, so this path is fatal to that model unless it is fixed. The fix is mechanical: `TreatDone` appends a DONE (txn-bound, re-confirmed per B-2) before counting.
- **B-2 (HIGH): DONE neither binds to the txn nor re-confirms the INTENT.** (a) On `develop`, B2's DONE has `txn_id: String::new()` and `fencing_generation: 0`. 7b requires both; S-25.06 writes them; ruling (iii) binds `txn_id` = `activation_id`. The worktree's in-progress `hack_txn` replaces this with the first `txn_id=` line read by raw `std::fs::read_to_string`. That bypasses the `Fs` seam (so it is invisible to the OBL-1 fault injection), swallows read errors through `.ok()`, ignores record validity (it can pick up a torn or checksum-failed record's value), and reads the whole log on every move. The correct fix is plumbing: pass `txn.txn_id` and `txn.fencing_generation` into `execute_canonical_path_moves` (Standing Rule 3 §4: add the missing plumbing). (b) DONE's `expected_post_hash` is the hash **observed after the rename**, not compared to the INTENT's `expected_post_hash`. The "Branch C hash source" ruling says the hash is "written at INTENT time and **re-confirmed** by the DONE record". As written, verification compares `sha256(file)` with a hash taken from that same file, which is circular. A staged file altered after the step-3b gate would be recorded and then "verified". Fix: append DONE only if `post_hash == INTENT.expected_post_hash`, otherwise halt fail-closed. Verifiers should require DONE.hash == INTENT.hash == `sha256(file)`. (c) DONE's `expected_pre_state` is hard-coded to `MISSING`, which claims "canonical was absent" for targets that existed. Copy it from the INTENT.
- **B-3 (MEDIUM-HIGH): the plan's integrity is not checked.** The fixed-plan model needs the plan to be trustworthy, and nothing checks it on the COMMITTING path. An empty `pending_canonical_moves` (hand edit, a corrupted rewrite, or a future regression) ⇒ `execute_canonical_path_moves` iterates zero moves ⇒ `0 < 0` is false ⇒ `completed.json {canonical_paths_count: 0}` ⇒ `Completed`. That is the same vacuous success that `recompute_pending_canonical_moves_from_staged_generation` was written to prevent on the STAGING path (OBL-1 FINDING 2), left open on the COMMITTING path. Fix: in `finish_committing_migration` (shared by all three call sites), require a non-empty plan **and** that the plan's canonical paths equal the set of distinct `target_canonical` values in the valid INTENT records; otherwise `AdmissionStateIntegrity { TxnRecordMalformed }`, exit 2, nothing moved. 11(e) step 2's manual cross-check already requires this. The code should enforce what the human procedure checks.
- **Checked and consistent:** both pre-swap writers persist the full plan **before** the swap (`run_bc_index_migration`; `ResumeFromStaging` before `append_intent_records_for_pending_moves`). The `CURRENT.json`-already-committed sub-path relies on the persisted plan, which is correct because `gen-<id>/` files may already have moved. The STAGING recompute from `gen-<id>/` is correct only **because** it runs before any rename, so a shrinking list would gain nothing there. Duplicate INTENTs from a STAGING re-run are harmless (same hashes; most recent wins). Fixed-plan iteration with per-target hash decisions is idempotent: a second recovery pass gives `TreatDone` for every target. That meets 7b's "identical results" mandate **once B-1 is fixed** (today the second pass also writes no DONE).
- **Naming hazard (sub-decision):** a field named `pending_canonical_moves` that never shrinks misled the ADR author and will mislead later readers. No txn record exists anywhere, so renaming it now (e.g. `canonical_move_plan`) costs only code, test, Kani-harness, runbook and taxonomy edits across both migrations. Renaming later would need a `schema_version` bump and migration. Keeping the name with a normative definition is the cheaper alternative.

### Verdict: **AMEND-SPEC-TO-CODE (fixed plan + DONE records)**, conditional on B-1, B-2 and B-3

The design choice the code made is right. Its implementation of that choice is incomplete on the exact crash window the intent log exists for. B-1 and B-2(a)/(c) are already required by 7b as written; B-2(b) and B-3 are required by the v1.23 "Branch C hash source" and 11(e) rulings. All three are code fixes, not spec amendments. Field naming is NEEDS-HUMAN as a sub-decision; recommendation: rename.

---

## Recommended Approach

### A. Spec amendment (architect; human authorization required under rule 12)

Replace the 7b "Record format" block and the torn-record paragraph with normative text equivalent to:

```
INTENT_LOG_RECORD_V1
txn_id=<txn_id, == activation_id>
fencing_generation=<u64 decimal>
record_type=INTENT|DONE|ABORTED
target_canonical=<path>
staging_path=<path>
expected_post_hash=<64 lowercase hex>
expected_pre_state=MISSING|<64 lowercase hex>
timestamp_utc=<RFC 3339, UTC, 'Z'>
record_checksum=<64 lowercase hex>
END_INTENT_LOG_RECORD
```
1. **Grammar.** LF line endings only. Exactly these ten lines in this order between the markers, each key once, no blank lines. A line splits at its **first** `=`. Paths must be valid UTF-8 containing no C0 control characters (LF, CR, NUL, ...) and no DEL. The coordinator rejects such a path when it **builds the plan** (named error, nothing staged) and the writer rejects it again (defense in depth, nothing appended). `record_type`, the hashes and `fencing_generation` are validated against their exact grammars. Any deviation means the record is invalid.
2. **Checksum.** `record_checksum` = SHA-256 over the **exact bytes** from the first byte of the `INTENT_LOG_RECORD_V1` line through the LF that ends the `timestamp_utc` line. An operator can verify it with `sed`/`awk` plus `shasum -a 256`.
3. **Reader.** Read the file as bytes and parse it as a line-anchored state machine. A record starts only at offset 0 or right after a previous valid record's END line. The **valid prefix** is the longest run of valid records from offset 0. Bytes after the valid prefix that contain no further valid record form a **torn tail**: treated as absent. Bytes after the valid prefix followed by any valid record are **mid-log corruption**: fail closed, as the integrity class of the reading surface (coordinator: exit 2, `MIGRATION_STATE_INTEGRITY_FAILURE`; verifier / Branch C: `canonical_hash_mismatch` per the existing ruling). This is RocksDB's `kTolerateCorruptedTailRecords`.
4. **Tail repair before append.** Before its first append in a run (under the held flock), the coordinator truncates the log to its valid-prefix length and durably syncs it. This guarantees torn bytes are never sandwiched by a later append (closes A-1 structurally, as PostgreSQL and systemd do).
5. **Durability.** Every append is `write` then `F_FULLFSYNC` on macOS (strict, the same primitive as D-1232-OBL-2(a)) or `fsync` elsewhere. When an append creates the file, the parent directory gets the durable directory sync before the WAL boundary counts as reached.
6. **DONE semantics.** A DONE record carries the txn's `txn_id` and `fencing_generation`, copies the INTENT's `expected_pre_state`, and is written only if `sha256(canonical) == INTENT.expected_post_hash`. Recovery row 1 ("Treat DONE; append DONE") is mandatory.
7. **Scope.** One format, one implementation: both `migrate-bc-index` and `backfill-append-logs` use the same writer and reader.

The marker stays `V1`: no released build has written a log (CHANGELOG has no `migrate-bc-index` entry; the human states no migration has run). If a tagged release is later found to contain the current writer, bump to `V2` instead.

### B. Spec amendment (architect; human authorization required)
- 7a: `pending_canonical_moves` (or the renamed `canonical_move_plan`) := "the complete, immutable move plan `{staging_path, canonical_path}`, persisted in full before the pointer swap and never modified afterwards. Per-move completion is recorded only by intent-log DONE records (7b). What remains = plan − {targets with a valid, txn-bound, re-confirmed DONE whose hash matches the file}."
- 7c step 7: delete sub-step d. Add: "The txn record is not rewritten per move."
- 7c step 8 / item 11(e) / Branch C: N = |plan| = |distinct INTENT `target_canonical`|; a COMMITTING record whose plan is empty or differs from the INTENT target set is `TxnRecordMalformed`.
- Rename decision (human): `canonical_move_plan` (recommended) or keep `pending_canonical_moves` with the definition above.

### Code changes (implementer; both migrations; TD-VSDD-060 sibling sweep)
1. Extract one generic intent-log module (writer, reader, checksum, `decide_*_recovery`) used by B2 and S-25.06. Delete the S-25.06 copy.
2. Implement A.1-A.6. Replace `String::from_utf8(whole file)` and `split(START)` with the byte-level state machine plus tail truncation. Route `Fs::append` through the strict `F_FULLFSYNC` primitive, with a directory sync on create.
3. `execute_canonical_path_moves` / `finish_append_log_migration`: take `txn_id` and `fencing_generation` as parameters (remove `String::new()` / `hack_txn`). `TreatDone` appends a DONE. DONE is written only when the post-hash equals the INTENT hash. Copy `expected_pre_state`.
4. `finish_committing_migration` (and the S-25.06 equivalent): check that the plan is non-empty and equals the INTENT target set before any move.
5. Runbook (devops-engineer): verify `record_checksum` in step 3, and require DONE.hash == INTENT.hash == `sha256(file)`. Update the awk to the hardened grammar.

### Test strategy (test-writer; red first)
| # | Test | Asserts |
|---|---|---|
| T1 | Truncate a 3-record log at **every byte offset** (property test) | The reader returns exactly the fully-written prefix records; never an error; never a partial record. |
| T2 | Valid R1 + garbage tail (partial START, 512 NUL bytes, random non-UTF-8 bytes) + **appended** R3 | R1 and R3 are both returned (after the tail truncation in A.4); without tail repair, mid-log corruption ⇒ fail closed (red today: R1 is lost). |
| T3 | Non-UTF-8 byte inside the tail of an otherwise valid log | Earlier records are returned; no `Io` (red today). |
| T4 | Bit-flip in each field of a middle record, followed by a valid record | Mid-log corruption ⇒ fail closed with the named integrity error. |
| T5 | Duplicate key, unknown key, blank line, CRLF endings, keys out of order | Record invalid (red today: accepted). |
| T6 | Writer given a path containing LF, CR, NUL or a non-UTF-8 byte; plan builder given the same | Named error, zero bytes appended, nothing staged. Paths containing `=`, the pipe character or the marker text round-trip exactly. |
| T7 | Failpoint after rename + dir fsync, before the DONE append (`migration_fs::append` on the DONE) → re-run | Final log has a txn-bound DONE for **every** plan target. The shared verifier / runbook step 3 passes. Second recovery pass makes zero changes (red today: DONE missing). |
| T8 | Mutate the staged file after step 3b, before its rename | No DONE; halt fail-closed; never `Completed`. |
| T9 | COMMITTING txn with empty plan; with a plan that differs from the INTENT set by one entry | `TxnRecordMalformed`, exit 2, nothing moved, no `completed.json` (red today: count-0 `Completed`). |
| T10 | Golden-bytes test of one record + operator checksum recomputation via the documented `sed` + `sha256` recipe | Byte-identical; checksum matches. |
| T11 | Both migrations run against the shared module (S-25.06 parity) | Same reader and writer bytes; DONE fields populated identically. |
| T12 | `Fs::append` creating the log | Directory durable-sync is invoked (fault-injection seam observes it); macOS path calls the strict full-sync primitive. |

---

## Top Sources

| Topic | URL |
|---|---|
| SQLite WAL format / recovery | https://www.sqlite.org/fileformat.html#the_write_ahead_log ; https://sqlite.org/walformat.html |
| PostgreSQL WAL record header / CRC / end-of-WAL | https://www.postgresql.org/docs/current/wal-internals.html ; https://github.com/postgres/postgres/blob/master/src/include/access/xlogrecord.h ; https://github.com/postgres/postgres/blob/master/src/backend/access/transam/xlogreader.c |
| LevelDB log format | https://github.com/google/leveldb/blob/main/doc/log_format.md |
| RocksDB WAL format / recovery modes | https://github.com/facebook/rocksdb/wiki/Write-Ahead-Log-File-Format ; https://github.com/facebook/rocksdb/wiki/WAL-Recovery-Modes ; https://github.com/facebook/rocksdb/blob/main/include/rocksdb/options.h |
| git ref/reflog format, packed-refs parser | https://github.com/git/git/blob/master/Documentation/git-update-ref.adoc ; https://github.com/git/git/blob/master/refs/packed-backend.c |
| systemd journal format | https://github.com/systemd/systemd/blob/main/docs/JOURNAL_FILE_FORMAT.md |
| JSON Lines / RFC 7464 | https://jsonlines.org/ ; https://www.rfc-editor.org/rfc/rfc7464.txt |
| Torn / non-prefix appends | https://research.cs.wisc.edu/wind/Publications/alice-osdi14.pdf ; https://docs.kernel.org/admin-guide/ext4.html ; https://xfs.org/index.php/XFS_FAQ |
| macOS fsync vs F_FULLFSYNC; SQLite fullfsync | https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html ; https://www.sqlite.org/pragma.html |
| Directory fsync | https://pubs.opengroup.org/onlinepubs/9799919799/functions/fsync.html ; https://man7.org/linux/man-pages/man2/fsync.2.html |
| Rust `str::lines` (`\r\n` handling) | https://doc.rust-lang.org/std/primitive.str.html#method.lines |
| ARIES | https://www.cs.cmu.edu/~15849g/readings/mohan92.pdf |
| git sequencer todo/done ordering | https://github.com/git/git/blob/master/sequencer.c ; https://git-scm.com/docs/git-rebase |
| dpkg status + updates journal | https://manpages.debian.org/unstable/dpkg/dpkg.1.en.html ; https://git.dpkg.org/cgit/dpkg/dpkg.git/tree/lib/dpkg/dbmodify.c |
| rpm transaction set / db recovery | https://rpm.org/docs/4.19.x/api/group__rpmts.html ; https://rpm.org/user_doc/db_recovery.html |
| Kubernetes spec/status | https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md ; https://github.com/kubernetes/design-proposals-archive/blob/main/architecture/resource-management.md |
| Log-as-truth / derived state | https://ceres.cs.umd.edu/818/papers/immutability.pdf ; https://martin.kleppmann.com/2015/03/04/turning-the-database-inside-out.html ; https://martinfowler.com/eaaDev/EventSourcing.html |

**Not verified / inconclusive:**
- Whether APFS can leave zero-filled or garbage tails after a power loss mid-append. The Pillai et al. evidence is for Linux filesystems. The recommendation does not depend on it, because the hardened reader handles any tail.
- Whether any tagged release contains the current intent-log writer (CHANGELOG has no entry; GitHub release assets were not checked). This only affects the V1-vs-V2 marker choice.
- The `hack_txn` edit in the S-25.09 worktree was observed mid-session, uncommitted. Its final form was not seen.
- The Kani harnesses (`obl1_kani_proofs.rs`, `append_log_kani_proofs.rs`) were not audited for dependence on the field name or plan semantics. The rename sub-decision needs that sweep.

---

## Research Methods

| Tool | Queries | Purpose |
|------|---------|---------|
| **Perplexity perplexity_research (PRIMARY)** | 2 | (1) Log framing and torn-record handling: SQLite WAL, PostgreSQL WAL, LevelDB/RocksDB + recovery modes, git reflog/packed-refs, systemd journal, JSONL/RFC 7464, Pillai OSDI'14 non-prefix appends, macOS F_FULLFSYNC, directory fsync. (2) Plan vs progress: ARIES analysis/redo, git sequencer todo/done ordering, dpkg status + updates journal, rpm rpmts, Kubernetes spec/status, Helland/Kleppmann/Fowler. |
| Perplexity perplexity_reason | 0 | — |
| Perplexity perplexity_search | 0 | — |
| Perplexity perplexity_ask | 0 | — |
| Context7 | 0 | No third-party library API in question (the format is in-house; Rust `str::lines` semantics cited from std docs) |
| Tavily | 0 | Not available in this session's toolset |
| WebFetch / WebSearch | 0 | — |
| Codebase reads (Read / Grep) | ~30 | ADR-052 §7a/7b/7c, Branch C hash source + rulings (i)-(v), 11(e), §Downstream; `shard_manager.rs` (`append_intent_log_record`, `intent_log_checksum_input`, `read_intent_log`, `parse_intent_log_block`, `decide_intent_log_recovery`, `append_intent_records_for_pending_moves`, `execute_canonical_path_moves`, `finish_committing_migration`, `recompute_pending_canonical_moves_from_staged_generation`, `plan_recovery` COMMITTING arm, `recover()` ForwardRecovery / ResumeFromStaging arms, `run_bc_index_migration` pre-swap prologue, `sha256_hex`); `migration_fs.rs` `Fs::append`; S-25.06 copies (`write_append_log_intent_record`, `read_append_log_intent_log`, `finish_append_log_migration`); tests in both worktrees; runbook §3; S-25.06 story AC-031; develop vs worktree diff on the DONE `txn_id` |
| Training data | 2 areas | Rust `str::lines` CRLF behaviour (std docs URL cited); general reasoning about checksum canonicalization ambiguity of pipe-joined fields |

**Total MCP tool calls:** 2
**Training data reliance:** low. Every external claim comes from the two research calls with primary-source URLs. Every codebase claim is anchored to a function read directly in the S-25.09 / S-25.06 worktrees and cross-checked on `develop`. Only two `perplexity_research` calls were needed because each was a broad multi-system sweep; the decisive findings (A-1, A-2, B-1, B-2, B-3) come from the code, not from external sources.
