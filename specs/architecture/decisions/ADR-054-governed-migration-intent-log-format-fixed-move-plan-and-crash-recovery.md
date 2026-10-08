---
document_type: architecture-decision-record
adr_id: ADR-054
level: L3
version: "1.1"
status: accepted
date: 2026-10-08
producer: architect
timestamp: 2026-10-08T00:00:00Z
phase: F1
supersedes: null
superseded_by: null
companion_of: ADR-052   # relation tracked here only; ADR-052 lists ADR-054 as an input, so ADR-052 is deliberately NOT an input of this file (breaks an input-hash cycle)
subsystems_affected:
  - SS-01
traces_to: .factory/specs/architecture/ARCH-INDEX.md
inputs:
  - .factory/research/adr-052-intent-log-format-and-move-list-semantics.md
  - CLAUDE.md
input-hash: "6d3e8d5"
# input-hash: run compute-input-hash --update at state-manager registration burst
---

# ADR-054: Governed Migration On-Disk Formats and Crash Recovery — Hardened Intent Log, Fixed Move Plan, Completion Evidence

## Status

ACCEPTED on human authorization dated 2026-10-08 (CLAUDE.md "Architectural Authority" rule 12: the
human authorized two amendments of ADR-052 §Decision 7b and §Decision 7a/7c, after research). The basis
is `.factory/research/adr-052-intent-log-format-and-move-list-semantics.md` (verdicts A = CHANGE-BOTH,
B = AMEND-SPEC-TO-CODE with three code fixes). The human decisions recorded there and applied here:

1. **APPROVED** the HARDENED intent-log format (keep the code's tokens and the `V1` label; fixed key
   order; strict value grammar enforced at plan-build and write time; SHA-256 checksum over the exact
   record bytes; byte-level line-by-line reader where a torn tail is absent but corruption followed by
   a valid record fails closed; torn-tail truncation under the flock before appending; `F_FULLFSYNC` on
   macOS plus a directory sync on creation; ONE shared module for `migrate-bc-index` and
   `backfill-append-logs`).
2. **APPROVED** the FIXED-PLAN model: the move list is the immutable plan and intent-log `DONE` records
   are the completion record. The field is RENAMED `pending_canonical_moves` → `canonical_move_plan`
   (no txn record exists anywhere). Bugs B-1, B-2, B-3 are fixed normatively (Decision 3).
3. All of it is delivered by a chain of THREE new stories stacked on S-25.09 (human decision of 2026-10-08,
   superseding the earlier one-story decision): **S-25.10** = intent-log FORMAT (AC-001..AC-008, AC-018; the
   `INTENT_LOG_CORRUPT` and `INTENT_LOG_VALUE_REJECTED` codes), **S-25.11** = fixed PLAN and COMPLETION
   (AC-009..AC-014; `CANONICAL_MOVE_HALTED`; the rename; `decide_recovery`), **S-25.12** = CLOSURE (AC-016
   Kani h1b and `kani.yml` 10 → 11, AC-017 runbook, AC-019 sweep evidence). Chain: S-25.09 → S-25.10 →
   S-25.11 → S-25.12 → S-25.06; S-25.12 blocks S-25.06 and any `migrate-bc-index` release. S-25.09 keeps the
   `txn_id` (= `activation_id`) / `intent_log_path` (persisted with `generation_id`, READ and never derived,
   enforced by the pairing check `check_generation_intent_log_pair`, S-25.09 AC-019, commit `5632964d`) /
   DONE `txn_id`+`fencing_generation` plumbing work. The pairing rule is S-25.09's and is NOT delivered by
   any story of the ADR-054 chain.

ADR-054 is a **companion** of ADR-052. It is the normative home of the on-disk formats and crash-recovery
mechanics listed in "Anchor map" below. ADR-052 keeps the stable `§Decision 7b` heading as a pointer, and
keeps the rest of Decision 7 (7a lock and txn record, 7c publication sequence, 7d platform barriers, 7e
shared namespace) with in-place amendments that cross-reference this ADR.

## Context

ADR-052 v1.23 §Decision 7b specified the intent log as `--- INTENT_LOG_RECORD v1 ---` / `key: value` /
`--- END_RECORD ---` with a checksum "of all above fields concatenated", and §Decision 7a/7c step 7d
specified `pending_canonical_moves` as a list "not yet completed" that is rewritten after every move. The
merged `migrate-bc-index` code (and the S-25.06 copy) diverged on both: it writes
`INTENT_LOG_RECORD_V1` / `key=value` / `END_INTENT_LOG_RECORD` / `MISSING`, and it keeps the full plan for
the life of the txn, recording completion only through intent-log `DONE` records. v1.23 recorded both as
open items. The independent research (see Status) found that the token choice decides nothing, but that
the reader and writer have real defects against 7b's own "a torn record is absent" rule:

- **A-1 (HIGH):** a valid record followed by non-record garbage and then a later append is discarded
  (unanchored `split` + `strip_suffix`), because the writer never repairs a torn tail.
- **A-2 (HIGH):** one invalid UTF-8 byte in a tail makes the whole log unreadable.
- **A-3 (MEDIUM):** the writer accepts values the reader cannot parse (LF, CR) and non-UTF-8 paths that
  are silently rewritten lossily.
- **A-4 (MEDIUM):** the checksum covers `|`-joined parsed values, not the on-disk bytes: ambiguous,
  blind to duplicate/unknown keys, and not operator-verifiable.
- **A-5 (MEDIUM):** the WAL-boundary append uses plain `fsync` (macOS: not durable to media) and no
  directory sync on creation.
- **A-6 (MEDIUM):** the format is shared by copy between two migrations, and the copies have drifted.
- **B-1 (HIGH):** `TreatDone` counts a move without appending the `DONE` record that 7b requires, so a
  crash between rename and `DONE` yields a migration that completes and then fails every verifier forever.
- **B-2 (HIGH):** `DONE` carries no txn binding (`txn_id` empty or scraped, `fencing_generation` 0), its
  `expected_post_hash` is the hash observed after the rename and never compared with the INTENT's
  (circular verification), and `expected_pre_state` is hard-coded `MISSING`.
- **B-3 (MEDIUM-HIGH):** an empty or tampered plan in a COMMITTING record completes vacuously with
  `canonical_paths_count: 0`.

### Split decision (why a companion ADR, and why exactly this cut)

ADR-052 is 4,795 lines / ~540 KB and already exhausts the PostToolUse hook WASM fuel budget on every
edit (CLAUDE.md "WASM fuel exhaustion"). The v1.24 content is normative text of ~600 lines (grammar,
reader algorithm, recovery table, S-25.10 downstream). The architect DECIDED:

- **Split: YES, but only §Decision 7b and the new fixed-plan/completion material.** Those are one cohesive
  concern (the intent-log wire format, the plan/completion model, and the recovery table over them) that
  two migrations and the operator runbook consume, and that is rewritten wholesale by this amendment.
  They are MOVED, not duplicated: ADR-052's `#### 7b` heading is retained (exact text, so the stable
  anchor and every `§Decision 7b` citation in BCs, VPs, stories and code comments resolve) and its body is
  replaced by a pointer table into this ADR.
- **Not split: 7a, 7c, 7d, 7e.** They remain in ADR-052 with minimal in-place amendments (7a field line,
  7c steps 3/6/7/8, 7d append-barrier bullet). Moving them would copy ~410 unchanged lines (the PC1/PC2
  census gate, the reader protocol, the platform code listings) by hand with transcription risk, would
  separate the gate/txn state machine from the admission sections (5a/5c) that cite it constantly, and
  would buy only ~9% of ADR-052's size. The honest fuel position is to STOP GROWING ADR-052: all new
  normative text lands here (a ~700-line file), and ADR-052 grows only by pointers and in-place
  amendments (about +80 lines, versus the ~600 lines of normative text it would otherwise have taken). The
  split therefore does NOT cure ADR-052's existing hook-fuel exhaustion; it prevents making it worse. A further split of ADR-052
  (e.g. §Decision 5a, §Error Code Semantics, §Downstream history) is a separate architect decision that
  needs its own anchor-map; it is recorded in the v1.24 changelog as a recommendation to the orchestrator,
  not silently deferred.
- **ID allocation (create-adr conventions):** the filesystem holds `ADR-001..ADR-053`; ARCH-INDEX holds
  the same set; the next collision-free ID is **ADR-054** (ADR-053 is the PROPOSED STORY-INDEX sharding
  ADR). The ARCH-INDEX row is inserted after ADR-053.

### Anchor map (stable citations)

| Citation that exists today | Resolves to |
|---|---|
| ADR-052 §Decision 7b (any sub-claim: framing, WAL boundary, torn record, recovery decision table, fault-injection mandate) | ADR-054 Decision 1 (format), Decision 3 (recovery table), Decision 5 (fault-injection mandate) |
| ADR-052 §Decision 7a field `pending_canonical_moves` | ADR-054 Decision 2 (`canonical_move_plan`) |
| ADR-052 §Decision 7c step 7 sub-step d, step 8 "verified" | ADR-054 Decision 2 (no per-move txn rewrite) and Decision 3 (verification) |
| ADR-052 "Branch C hash source", item 11(e) steps 2-3 | ADR-054 Decision 3 (B-2/B-3) and Decision 1.5 (operator recipe); the admission/verdict classification stays in ADR-052 §Error Code Semantics |
| `INTENT_LOG_RECORD_V1`, `END_INTENT_LOG_RECORD`, `record_checksum` | ADR-054 Decision 1.2 / 1.4 |

## Decision

### Decision 1 — Intent-log wire format (formerly ADR-052 §Decision 7b; HARDENED)

#### 1.1 File, scope, authority

One file per generation: `.factory/migration-state/intent-<generation_uuid>.log` (shared namespace, ADR-052
§Decision 7e). The format below is the ONLY intent-log format; both `migrate-bc-index` and
`backfill-append-logs` read and write it through the ONE module of Decision 4. The log is append-only
under the held `exclusive.lock` flock, with the single exception of tail repair (1.9 step 2).

The checksum detects accidental corruption and tearing. It is NOT authentication: an actor who can write
`migration-state/` can recompute it. Tamper-resistance is the job of the Write/Bash guards (ADR-052 §5b/5c),
not of this format.

#### 1.2 Grammar (RFC 5234 ABNF with RFC 7405 case-sensitive `%s` literals; byte-exact)

```
intent-log    = *record [ torn-tail ]          ; torn-tail never produced by a writer; see 1.7
torn-tail     = *OCTET                         ; any bytes after the valid prefix, with NO later valid record
record        = start-line txn-line fence-line type-line target-line staging-line
                post-line pre-line ts-line sum-line end-line   ; EXACTLY these 11 lines, EXACTLY this order

start-line    = %s"INTENT_LOG_RECORD_V1" LF
txn-line      = %s"txn_id=" txn-id LF
fence-line    = %s"fencing_generation=" u64-dec LF
type-line     = %s"record_type=" ( %s"INTENT" / %s"DONE" / %s"ABORTED" ) LF
target-line   = %s"target_canonical=" path LF
staging-line  = %s"staging_path=" path LF
post-line     = %s"expected_post_hash=" hex64 LF
pre-line      = %s"expected_pre_state=" ( %s"MISSING" / hex64 ) LF
ts-line       = %s"timestamp_utc=" utc-ts LF
sum-line      = %s"record_checksum=" hex64 LF
end-line      = %s"END_INTENT_LOG_RECORD" LF

LF            = %x0A
hex64         = 64HEXLC
HEXLC         = DIGIT / %x61-66                 ; 0-9 a-f, lowercase only
txn-id        = 1*128txn-char
txn-char      = ALPHA / DIGIT / "-" / "_" / "."
u64-dec       = "0" / ( %x31-39 0*19DIGIT )     ; semantic: value <= 18446744073709551615
path          = 1*4096OCTET                      ; semantic: see value rules
utc-ts        = 4DIGIT "-" 2DIGIT "-" 2DIGIT "T" 2DIGIT ":" 2DIGIT ":" 2DIGIT [ "." 1*9DIGIT ] "Z"
```

Value rules (apply to every record at write time AND read time; a violation makes the record INVALID):

1. **Lines.** LF only. CR (`%x0D`) is never valid anywhere. No blank lines, no unknown keys, no duplicate
   keys, no reordered keys. A line splits at its FIRST `=` (no key contains `=`; a path value MAY contain
   `=`, `|`, spaces or marker text). **Whitespace (v1.1 ruling, closes the rule-1/rule-2 ambiguity):**
   "whitespace" in this rule means exactly U+0020 (every other ASCII whitespace byte is a C0 control and
   already invalid; non-ASCII whitespace such as U+00A0 is ordinary text). A line MUST NOT begin with
   U+0020 (the key starts at byte 0), MUST NOT end with U+0020 (the byte before LF is never a space),
   and the byte immediately after the first `=` is part of the value. Because the value is the rest of
   the line, a line-final U+0020 would be a value-final U+0020, so a `path` value MUST NOT begin or end
   with U+0020 (rule 2); INTERIOR spaces, including a space adjacent to `/` such as `/a /b`, are valid.
   Every other value production (rules 3-7) excludes U+0020 by its own grammar.
2. **`path`** (`target_canonical`, `staging_path`): well-formed UTF-8 (RFC 3629; overlongs, surrogates and
   truncated sequences are invalid); no scalar value in U+0000..U+001F and not U+007F; 1 to 4096 BYTES;
   first and last scalar value not U+0020. A path that is not valid UTF-8 (Rust `OsStr::to_str() ==
   None`) is NOT representable: it is rejected, never lossily converted. A path with a leading or trailing
   U+0020 is likewise NOT representable (a line-final space is invisible to operators and to any tool that
   strips trailing whitespace, and `=` splitting must stay unambiguous): it is rejected with the reason
   token `leading_or_trailing_space`, never trimmed. **Reason precedence when several defects co-occur
   (the first listed wins; `validate_path` and the writer agree):** `empty`, `over_4096_bytes`, `not_utf8`,
   `contains_control_character`, `leading_or_trailing_space`. The reader applies the same rules to the
   bytes of a value: a record whose path value begins or ends with U+0020 is INVALID.
3. **`txn_id`**: 1 to 128 token characters. In production it is the activation UUID (ADR-052 §7a: `txn_id`
   == `activation_id`), lowercase hyphenated; the grammar is the broader token so the reader does not
   depend on the UUID textual form.
4. **`fencing_generation`**: canonical decimal, no leading zeros, fits `u64`.
5. **`expected_post_hash`** and a non-`MISSING` **`expected_pre_state`**: 64 lowercase hex digits
   (`sha256_hex` output; uppercase is invalid). `MISSING` means "the canonical path was absent at INTENT
   time". Neither can collide with a hash.
6. **`timestamp_utc`**: `utc-ts` with month 01-12, day 01-31, hour 00-23, minute 00-59, second 00-59, UTC
   `Z` only (no offsets). The writer emits second precision.
7. **`record_type`**: `INTENT` (written before any rename), `DONE` (completion evidence), `ABORTED`
   (reserved; written by no flow in this ADR; a reader accepts it as a valid record that never counts
   toward completion).

Because no value may contain LF, no value can contain a marker LINE; the markers are line-anchored by
construction.

#### 1.3 Marker versioning

The start marker stays `INTENT_LOG_RECORD_V1`. Basis: no released build has written an intent log (the
human states no migration has run; CHANGELOG has no `migrate-bc-index` entry). If a tagged release is
later found to contain the pre-ADR-054 writer, the marker becomes `INTENT_LOG_RECORD_V2` in the same
change and the V1 reader path is not retained (a V1-old-format log is then treated as mid-log corruption
and the migration is re-run from a clean state under human supervision).

#### 1.4 Checksum — byte-exact definition

`record_checksum` = lowercase-hex SHA-256 of the **exact on-disk bytes** of the first nine lines of the
record: from the first byte of the `INTENT_LOG_RECORD_V1` line through and including the LF that ends the
`timestamp_utc=` line. Formally the input is `start-line txn-line fence-line type-line target-line
staging-line post-line pre-line ts-line` (9 lines). It excludes the `record_checksum=` line and the
`END_INTENT_LOG_RECORD` line. The writer computes it over the same byte buffer it then writes; the reader
computes it over the bytes it read; nothing is reconstructed from parsed values.

**Golden record (normative test vector).** `expected_post_hash` = `sha256("lean-body")`,
`expected_pre_state` = `sha256("monolith")`. Lines are LF-terminated.

```
INTENT_LOG_RECORD_V1
txn_id=0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90
fencing_generation=1
record_type=INTENT
target_canonical=/proj/.factory/specs/behavioral-contracts/BC-INDEX.md
staging_path=/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f/BC-INDEX.md
expected_post_hash=dd2ce8a1000fd7b33e333408ed7dc137d3c8358b5602dcc14bf9919ea00fe2bc
expected_pre_state=405dc7565658c749a2775f62875bc67d7892c97795a43438ea5936ef078b72a4
timestamp_utc=2026-10-08T12:00:00Z
record_checksum=2767fd1de24bdaf614a8eb417d3d5db14b5ff165940a32579944944d6ce9e02b
END_INTENT_LOG_RECORD
```

The checksum above is the SHA-256 of the first nine lines. The matching `DONE` record is the same bytes
except `fencing_generation=2`, `record_type=DONE`, `timestamp_utc=2026-10-08T12:00:07Z` and
`record_checksum=3382c172c4b08aef623fcc04469e6cdca6eea1281c6b90415ba18919f3aac5c1`.

#### 1.5 Operator verification recipe (sed + shasum; no other tool)

A record whose `INTENT_LOG_RECORD_V1` line is line `S` of the file occupies lines `S..S+10`; its stored
checksum is line `S+9`. The operator verifies it with:

```
sed -n "${S},$((S+8))p" intent-<generation_uuid>.log | shasum -a 256     # compare with: sed -n "$((S+9))p" … | cut -d= -f2
```

(`sha256sum` on Linux). The procedure of ADR-052 item 11(e) step 3 walks records from `S=1` in steps of 11
and stops at the first failing record. Every record verifying and the file ending exactly at a record
boundary (`wc -l` is a multiple of 11 and the last line is `END_INTENT_LOG_RECORD`) is a clean log.
Bytes after the last verifying record are a torn tail ONLY IF no later occurrence of the byte string
`INTENT_LOG_RECORD_V1` followed by LF, AT ANY BYTE POSITION (including in the middle of a line, i.e. after
garbage that has no trailing LF), starts a record that verifies; if one does, the log is corrupt mid-stream
and the operator MUST stop and escalate. To find such occurrences the operator lists byte offsets with
`grep -a -b -o 'INTENT_LOG_RECORD_V1' <log>` (offsets are 0-based; a marker offset past the end of the last
verifying record is a candidate) and verifies a candidate at byte offset `O` with
`tail -c +$((O+1)) <log> | head -n 9 | shasum -a 256` against line 10 of the same slice.

#### 1.6 Operator-visible semantics of `expected_pre_state`

`expected_pre_state` is recorded truthfully at INTENT time (`sha256(canonical)` if the canonical file
exists, else `MISSING`) and is COPIED VERBATIM into the matching `DONE` (Decision 3, B-2(c)). No flow
writes a literal `MISSING` that was not observed.

#### 1.7 Reader algorithm (byte-level, line-anchored; normative)

Input: the file's bytes `B` (read through the `Fs` seam; NEVER decoded as one UTF-8 string) and the
expected `txn_id` of the live txn. Output: `LogRead { records, valid_prefix_len, tail }` or a typed
failure. The reader performs no I/O except the single read.

```
parse_record_at(B, off) -> Ok(record, next_off) | Err
    PREFIX mode (the scan from offset 0): a record starts ONLY at off == 0 or when B[off-1] == LF
    PROBE mode (the mid-log search below): no line-start requirement on `off`
    read exactly 11 LF-terminated lines; a line longer than 4113 bytes (len("target_canonical=") +
        4096) is invalid without reading further (bounds the probe cost); match each against its
        production in 1.2
    validate value rules (1.2 rules 1-7) on the bytes of that record (UTF-8 checked on the path bytes only)
    recompute SHA-256 over bytes of lines 1..9; require equality with line 10's value
    require line 11 == "END_INTENT_LOG_RECORD\n"

read_log(B, txn_id):
    off = 0; records = []
    loop:
        if off == len(B): return Clean(records)                       # exact record boundary
        match parse_record_at(B, off): Ok(r, nx) => records.push(r); off = nx ; Err => break
    valid_prefix_len = off
    for q in every byte offset strictly after valid_prefix_len          # NOT only line starts
        if B[q..] begins with "INTENT_LOG_RECORD_V1\n" and parse_record_at(B, q, PROBE) is Ok:
            return Err(MidLogCorruption { first_bad_offset: valid_prefix_len, later_valid_offset: q })
        # q is the SMALLEST such offset; `later_valid_offset` is a BYTE offset, which is a line
        # start only when the garbage before it ended in LF
    return Ok(records, valid_prefix_len, tail = Torn { offset: valid_prefix_len, len: len(B) - valid_prefix_len })
    then apply log-level invariants (1.8) to `records`
```

Properties: (i) a record is returned only if every one of its 11 lines parsed, its checksum matched and
its END line is present — a "partial record" is unrepresentable; (ii) the valid prefix is the longest run
of valid records from offset 0; (iii) a torn tail is ABSENT: not an error for the reader; (iv) bytes
after the valid prefix FOLLOWED by any valid record are corruption in the middle of the log and the reader
FAILS CLOSED (RocksDB `kTolerateCorruptedTailRecords` semantics: tail tolerated, middle not). "Followed
by a valid record" is decided at ANY byte position, not only at line starts (v1.1 ruling): garbage with
no trailing LF immediately followed by a complete, checksum-valid record (`zzzz` + R3, or a record torn
inside its END line + R3) puts R3's marker mid-line, and silently dropping R3 as part of a "torn tail"
would lose a durable record. The writer's tail repair (1.9 step 2) makes this unreachable in normal
operation; the probe exists for external damage. A false positive needs a complete valid record
(11 lines, matching SHA-256) inside the post-prefix bytes, which is itself the evidence of corruption;
a path value ending in the marker text cannot forge it because the next line must be `txn_id=`;
(v) tearing
that produces zeros, garbage, a partial start marker, a partial END or invalid UTF-8 in the tail is
handled identically to a short prefix; (vi) the scan is O(n × 11 lines), bounded by the log size.

Surface mapping (the SAME `LogRead`, different verdict delivery):

| Surface | Torn tail | MidLogCorruption / invariant violation (1.8) | Read error other than ENOENT |
|---|---|---|---|
| Coordinator (`migrate-bc-index`, `backfill-append-logs`: recovery, finish, verifier) | absent; repaired before append (1.9) | exit 2 `INTENT_LOG_CORRUPT`, no further move or append (when the read happens before the move loop, nothing at all was moved or appended; the post-loop re-read of Decision 3 step 5 can find corruption after this run's moves, which stay in place and no `completed.json` is written) | `Io`, exit 2 |
| S-25.06 verifier / finalize seam (AC-031) | absent | `canonical_hash_mismatch` (verification-unprovable class) | `Io` |
| Admission Branch C (ADR-052 §5a) and §5c Branch 2 | absent | `canonical_hash_mismatch` | `E-MAINTENANCE-002` `io` |
| Operator (ADR-052 item 11(e)) | absent | STOP, escalate | n/a |

ENOENT (missing log) keeps its ADR-052 classification (`canonical_hash_mismatch` on the verification
surfaces; "no INTENT" on the coordinator, which B-3 turns into `TxnRecordMalformed`).

#### 1.8 Log-level invariants (checked by the reader after record parsing)

Violation ⇒ same verdict as MidLogCorruption (table above). Let a *target* be a distinct `target_canonical`.

- **L1** Every record's `txn_id` equals the live txn's `txn_id`. (A generation has exactly one txn; a new
  activation gets a new generation and a new log file.)
- **L2** All `INTENT` records for one target agree on `staging_path`, `expected_post_hash` and
  `expected_pre_state`. (A STAGING re-run may append duplicate INTENTs; the staged generation is
  content-immutable, so they are identical; the most recent one is canonical.)
- **L3** Every `DONE` for a target is preceded in file order by an `INTENT` for that target and agrees with
  it on `staging_path`, `expected_post_hash` and `expected_pre_state`.
- **L4** `fencing_generation` is non-decreasing along the file, and a `DONE` is not lower than the
  `INTENT` it completes.

**"First violating record" (v1.1, normative; the reported offset).** The checker makes ONE in-order pass
over the parsed records, keeping per-target state (the first-seen `INTENT`'s `staging_path`,
`expected_post_hash`, `expected_pre_state`, and whether an `INTENT` has been seen) and the running maximum
`fencing_generation`. The *first violating record* is the first record in FILE ORDER for which any check
below fails; the reported `offset` is the byte offset of THAT record's `INTENT_LOG_RECORD_V1` line; when
several checks fail on the same record, the invariant reported is the first in the order L1, L2, L3, L4.
All four are judged against the PRECEDING records only, so a violation is always attributed to the record
that introduces the conflict, never to the earlier record it conflicts with:

- **L1** the first record (any type) whose `txn_id` differs from the live `txn_id`.
- **L2** the first `INTENT` whose `staging_path`, `expected_post_hash` or `expected_pre_state` differs from
  the first-seen `INTENT` for the same target: the LATER conflicting `INTENT` (the earlier one is the
  reference, because an in-order scan cannot know it is the wrong one). `DONE` and `ABORTED` records are
  not L2 subjects.
- **L3** the first `DONE` for a target with no preceding `INTENT` for that target, or whose
  `staging_path`, `expected_post_hash` or `expected_pre_state` differs from the target's `INTENT` (all
  INTENTs for a target agree once L2 holds). `ABORTED` records are not L3 subjects.
- **L4** the first record (any type) whose `fencing_generation` is lower than the maximum
  `fencing_generation` of all preceding records. A `DONE` lower than its own `INTENT` is a special case
  of this (the `INTENT` precedes it), so no separate clause is needed.

A record that is INVALID (fails 1.2 or its checksum) is never an invariant subject: it ends the valid
prefix and is handled by 1.7.

#### 1.9 Writer algorithm (normative; under the held `exclusive.lock` flock)

1. **Validate before any byte.** Every field of every record to be appended is checked against 1.2. A
   violation returns `INTENT_LOG_VALUE_REJECTED` (names field and reason, value rendered on operator stderr
   per Decision 3.1: control characters escaped, each data-derived substring capped at 256 characters); NOTHING is appended. The same check runs at **plan-build time** on every
   `staging_path` and `canonical_path` of the plan (Decision 2) so a bad path is rejected with nothing
   staged; a rejection once the generation directory exists takes the normal pre-commit abort path
   (ADR-052 §7c step 3c shape: txn ABORTED, gate OPEN).
2. **Open and repair the tail (once per run, before the first append).** Read the file, run `read_log`.
   `MidLogCorruption` or an invariant violation ⇒ `INTENT_LOG_CORRUPT`, nothing mutated. A torn tail ⇒
   truncate the file to `valid_prefix_len` and make the truncation durable (file barrier below). Torn bytes
   are therefore never sandwiched by a later append (closes A-1 structurally; PostgreSQL overwrites from
   the end of valid WAL, systemd never appends after corruption). The truncation destroys only bytes that
   are not a valid record.
3. **Serialize.** Build the full bytes of each record (11 lines, LF), checksum over the first nine lines.
4. **Append.** One `write_all` of the batch on an `O_APPEND` handle. (ADR-052 §7c step 3 appends every
   INTENT for the plan in one batch, then the single barrier; step 7 appends one `DONE` per move.)
5. **Barrier.** The platform file barrier of ADR-052 §7d on the log handle: `F_FULLFSYNC` on macOS,
   `fsync` elsewhere, NO silent downgrade (a failing `F_FULLFSYNC` is an `Io` error and the WAL boundary
   is NOT reached). If this call created the file, the parent directory is synced with the same directory
   primitive §7d assigns to rename durability, BEFORE the WAL boundary counts as reached.

   **Who syncs the directory (v1.1 ruling): the WRITER, not `StdFs::append_durable`.**
   `Fs::append_durable` appends, applies the FILE barrier and reports `created: bool` (true iff THIS call
   created the file; `StdFs` decides it atomically with `OpenOptions::create_new(true)` falling back to
   an append-open on `AlreadyExists`, never with a prior `exists()` probe). `IntentLogWriter::append_batch`
   then calls `Fs::fsync_dir(parent)` iff `created == true`, and a non-creating call performs NO
   directory sync. Rationale: (1) the directory sync is a separate failure and crash point that Decision
   5 requires the tests to inject independently of the data barrier; through the `Fs` seam it is a
   distinct, separately failable, separately recorded operation (`migration_fs::fsync_dir`), whereas folded
   into `StdFs::append_durable` it would share that method's failpoint and no test double (which
   re-implements the file effect) could observe or fail it, so the "log-creation directory sync" fault
   point of Decision 5 would be untestable in doubles; (2) the same primitive (`Fs::fsync_dir`) then
   serves rename durability and log-creation durability, so there is one place to make it strict; (3)
   Kani: no harness reaches `Fs` (`decide_recovery`, `recover` and the txn-record functions are pure
   and `Fs`-free), so neither choice changes any Kani model; the ruling has no proof impact.
   **Retry closure.** A failed creation-time `fsync_dir` is an `Io` error and the WAL boundary is not
   reached, but the log file now exists, so a RE-RUN's first append reports `created == false` and would
   never sync the entry. Therefore the coordinator, on the path where the generation's log is found
   already existing and holding no `DONE` record (the pre-rename phase) and before it appends the INTENT
   batch, calls `Fs::fsync_dir(parent_of_log)` once (a coordinator step, not a writer-reopen side effect:
   `IntentLogWriter::open` on a clean existing log performs NO mutating op). An extra directory sync is
   harmless; a missing one lets a power loss erase the log after renames began.
   **Strict barrier primitive.** The file barrier is `last_amended_migrate::atomic_write::
   sync_file_strict_durable(&File) -> Result<(), MigrateError>`: a NEW `pub` wrapper over that module's
   existing private `sync_file_durable` (`fcntl(F_FULLFSYNC)` on macOS, `sync_all` elsewhere, the platform
   `cfg` lives INSIDE the primitive, error propagated, no fallback to `fsync` when `F_FULLFSYNC` fails;
   it is the file-handle sibling of the existing `sync_dir_strict_durable`). `factory-dispatcher` has
   no libc/FFI of its own (`shard_manager.rs` comment on `F_FULLFSYNC`), so `StdFs::append_durable` and
   `StdFs::truncate_durable` each call exactly this primitive and their bodies contain neither
   `.sync_all(` nor `.sync_data(`; the macOS source gate's token `strict_durable` is satisfied by the
   primitive's name.
6. **WAL boundary.** After step 5 of the batch carrying every INTENT, every rename is recoverable.

`Fs` seam additions (OBL-1 fault injection must be able to fail each): `truncate_durable(path, len)` and
`append_durable(path, bytes) -> created: bool` replacing the current `append` (`sync_all`, no dir sync).

### Decision 2 — The fixed move plan (formerly ADR-052 §Decision 7a field and §7c step 7d; RENAMED)

**Definition.** The txn record field **`canonical_move_plan`** is the complete, ordered, immutable list of
`{staging_path, canonical_path}` moves of the migration. It replaces `pending_canonical_moves`, which was a
misnomer for a list that never shrinks in the merged code.

1. **Persisted in full before the commit point.** The plan is written in the txn-record write that
   precedes the `CURRENT.json` pointer swap (ADR-052 §7c step 6), after every INTENT is durable (WAL
   boundary). On the STAGING-resume path, if `CURRENT.json` already names this generation and txn the
   persisted plan is used as-is (files in `gen-<id>/` may already have moved); otherwise it is recomputed
   from the immutable `gen-<id>/` contents before any rename (the existing OBL-1 FINDING 2 recompute).
2. **Never modified afterwards.** There is NO per-move rewrite of the txn record: ADR-052 §7c step 7
   sub-step d is DELETED. The txn record changes only by its state transitions (`COMMITTING` →
   `COMPLETED`/`ABORTED`), `updated_at` and recovery-owner `fencing_generation` claims.
3. **Progress lives only in the log.** Per-move completion is recorded exclusively by intent-log `DONE`
   records (Decision 1). What remains = `plan` minus {targets whose latest valid record is a `DONE` that
   satisfies Decision 3 and whose canonical file hashes to its `expected_post_hash`}. "Remaining" is derived,
   never stored.
4. **Shape.** A JSON array, never `null`, never empty for a txn in `COMMITTING`; every element an object
   with EXACTLY the keys `{staging_path, canonical_path}`, both non-null JSON strings; `canonical_path`
   values pairwise distinct; every path satisfies the 1.2 `path` value rule. Struct name:
   `PlannedCanonicalMove` (was `PendingCanonicalMove`).
5. **Strict-presence decode** (ADR-052 §Error Code Semantics item 10): the key `canonical_move_plan` is
   REQUIRED PRESENT with type array on every live known record at a rewriting arm; the required key set is
   still TWELVE (the eleven named plus `schema_version`) with the renamed field. Nested strictness
   (item 10(a)) now names `canonical_move_plan[<index>]`. `TXN_RECORD_SCHEMA_VERSION` stays **1**: no
   build that wrote a record under the old name was ever released (ADR-052 §Decision 5: no activation has
   run), so no legacy record exists and no migration or version bump is needed. A record carrying the
   old key `pending_canonical_moves` is an unknown top-level key (item 10) and fails `txn_record_malformed`.
6. **N.** For `migrate-bc-index` N = `len(canonical_move_plan)` = the number of distinct `target_canonical`
   among the INTENT records (txn-specific: one per staged shard, sub-shard, manifest and `BC-INDEX.md`). For
   `backfill-append-logs` N is the fixed 4 and equals the plan length.

### Decision 3 — Completion semantics: B-1, B-2, B-3 (normative)

**B-1 — `TreatDone` appends `DONE`.** When recovery finds a target whose canonical file already hashes to
the INTENT's `expected_post_hash` and whose latest valid record is an `INTENT`, it MUST append the `DONE`
record (then barrier, Decision 1.9) BEFORE counting the move complete. This is the existing 7b recovery
table row 1 ("Treat DONE; append DONE record"), now mandatory in code. It closes the crash window between
rename+directory-sync and the `DONE` append.

**B-2 — `DONE` is bound and re-confirmed.** A `DONE` record is written only if all of the following hold,
otherwise the move HALTS (`CANONICAL_MOVE_HALTED`, below) and no `DONE` is appended:
(a) `txn_id` is the live txn's `txn_id` and `fencing_generation` is the txn's CURRENT fencing generation
(S-25.09 delivers the plumbing into the move executor; no scraping of the log, no empty string, no `0`);
(b) `sha256(canonical_path)` observed after the rename (or at `TreatDone`) is COMPARED with the latest
INTENT's `expected_post_hash` and is equal — the `DONE`'s `expected_post_hash` is COPIED from the INTENT,
never computed from the file it will be verified against (a self-hash makes verification circular);
(c) `target_canonical`, `staging_path` and `expected_pre_state` are copied verbatim from the INTENT.
Verifiers (S-25.06 AC-031, admission Branch C, ADR-052 item 11(e) step 3) require
`DONE.expected_post_hash == INTENT.expected_post_hash == sha256(file)`.

**B-3 — The plan is integrity-checked before any move.** `finish_committing_migration` (shared by every
COMMITTING recovery arm) and its mechanism-A counterpart MUST, before the first rename, require:
(a) `canonical_move_plan` is a non-empty array of well-formed, pairwise-distinct moves (Decision 2.4);
(b) the log reads without corruption (Decision 1.7) and the plan's set of `canonical_path` values equals the
set of distinct `target_canonical` values over the log's `INTENT` records, and for each pair the plan's
`staging_path` equals the INTENT's. Otherwise exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` kind
`txn_record_malformed` (empty or malformed plan, or set/pair disagreement; `detail` names the offending
field and the symmetric difference, sanitized), NOTHING moved, no `completed.json`. On the verification
surfaces (Branch C, S-25.06 verifier) a plan that is absent/ill-typed/empty is `state_integrity`
`TxnRecordMalformed` at the `canonical_hash_mismatch` check's position (Tier 1), and a plan that disagrees
with the INTENT set is `canonical_hash_mismatch` (completion unprovable). Rationale for the two classes:
the coordinator is about to ACT on the record, so an untrustworthy record is an integrity failure; the
verifier is deciding whether a completion CLAIM is provable, which is the mismatch vocabulary.

**Durable ordering (WAL boundary; unchanged in substance from ADR-052 v1.23 §7b).** (1) Write and sync all
staging generation files (ADR-052 §7d). (2) Append every INTENT record in one batch and apply the log
barrier (Decision 1.9): **after this barrier every rename is recoverable.** (3) `rename(staging_path,
canonical_path)` (same filesystem). (4) `sync_dir(parent_dir_of_canonical_path)` (ADR-052 §7d). (5) Append the
`DONE` record for the target and apply the log barrier. Steps 3-5 repeat per plan entry; the txn record is
not touched between them (Decision 2.2).

**Per-move procedure (the body of `execute_canonical_path_moves` and its S-25.06 twin; for each plan entry
in array order, under the flock):**

1. `intent` = the latest valid `INTENT` for the target; `latest` = the latest valid record for the target.
2. Compute `canonical_hash` (absent ⇒ none) and `staging_hash` (absent ⇒ none) through the `Fs` seam.
3. Apply the pure decision `decide_recovery(canonical_hash, staging_hash, intent, done_present)`:

| # | Latest valid record | Canonical | Staging | Decision and action |
|---|---|---|---|---|
| 1 | INTENT | `== expected_post_hash` | any | `TreatDone`: append `DONE` (B-1/B-2), count complete |
| 2 | DONE (satisfies L3) | `== expected_post_hash` | any | `AlreadyDone`: count complete, append NOTHING |
| 3 | DONE | `!= expected_post_hash` | any | `FailClosed` (diverged after completion) |
| 4 | INTENT | `== expected_pre_state` (or absent and pre is `MISSING`) | `== expected_post_hash` | `RedoRename`: rename, directory sync, verify `sha256(canonical) == expected_post_hash` (else halt), append `DONE`, count complete |
| 5 | INTENT | pre-state matches | missing or `!= expected_post_hash` | `FailClosed` (no recovery copy) |
| 6 | INTENT | matches neither post nor pre | any | `FailClosed` |
| 7 | none | any | any | `FailClosed` (unreachable once B-3 holds; kept for totality) |

   `decide_recovery` is pure and total over its inputs (no I/O, no panic): it is proved by the NEW B2-suite
   harness `proof_obl1_h1_decide_recovery_totality` (VP-147 h1b) and mirrored by VP-146 a1 (with the consumers
   a2, a5, a6) once S-25.06 rebases onto S-25.12. The existing VP-147 h1 `proof_obl1_h1_recover_totality`
   proves the txn-record `recover` function, NOT this per-target decision, and is not re-pointed.
   It adds the `AlreadyDone` outcome and the `done_present` input to the previous three-outcome function.

   **A `DONE` is only ever derived from an `INTENT` (v1.1 ruling).** A `DONE` is built by copying the
   latest valid `INTENT` of the target (Decision 3 B-2 (c)); the shared module exposes it as a
   constructor taking that `INTENT` (`txn_id`, `fencing_generation`, timestamp are the only other inputs),
   so "a `DONE` with nothing to copy" is unrepresentable in the module API and S-25.10's writer can never
   emit one. A call of the per-move procedure for a target with NO valid `INTENT` (a direct
   `execute_canonical_path_moves` call, or a plan entry the log does not cover) is table row 7:
   `decide_recovery` returns `FailClosed` and the move halts `CANONICAL_MOVE_HALTED` with `<reason>`
   `fail_closed_no_intent`; no rename and no `DONE` append happens for that target. The row is
   unreachable through the coordinator once B-3 holds (the plan equals the INTENT set before the first
   rename) and reachable only by a direct call or a defect, which is why it is kept for totality. OWNER:
   S-25.11 (the code, the `CanonicalMoveHalted` variant and the token test are S-25.11's). In S-25.10,
   whose converted `execute_canonical_path_moves` still precedes the variant, the call site MUST
   fail closed without any rename or `DONE` append and with a non-success result carried by the
   pre-existing error for that site; S-25.10 tests assert only "no `DONE` appended, no rename, not Ok",
   never the carrier, which S-25.11 replaces.
4. Any of the SIX halt sites ⇒ halt further moves, leave the txn `COMMITTING` (forward recovery re-runs
   idempotently), exit 2 `CANONICAL_MOVE_HALTED` with the site's `<reason>` token (§3.1): (1) recovery decision
   `FailClosed` (`fail_closed_*`); (2) the parent-directory create before the rename fails
   (`parent_dir_create_failed`); (3) the rename fails (`rename_failed`); (4) the directory sync fails
   (`dir_sync_failed`); (5) the post-rename verification read of the canonical file fails, or the file is
   missing (`post_rename_read_failed`), or its hash diverges from the INTENT's `expected_post_hash`
   (`post_rename_hash_diverged`); (6) the `DONE` append or its barrier fails (`done_append_failed`). No halt
   site may surface as `BinaryIntegrityFailure`, `Io` or any other carrier.
5. After the loop, BEFORE writing `completed.json`: re-read the log and call
   `verify_plan_completion(plan, log)` (the SAME function the S-25.06 verifier and Branch C use): every plan
   target has a `DONE` satisfying B-2. Only then write `completed.json` with
   `canonical_paths_count = len(plan)`, transition the txn to `COMPLETED` (checked) and open the gate.

**Idempotence.** A second recovery pass over a fully recovered migration finds row 2 for every target and
appends nothing; the log bytes and the txn record bytes are unchanged (ADR-052 §7b "identical results"
mandate, now true because B-1 holds).

**New named errors (binary exit codes; all exit 2; none is a `HookResult`):**

| Code | Variant | Trigger | Mutation |
|---|---|---|---|
| `INTENT_LOG_CORRUPT` | `IntentLogCorrupt { path, kind, offset }` | `MidLogCorruption`, invariant L1-L4 violation, or a non-V1 record followed by a valid record | none |
| `INTENT_LOG_VALUE_REJECTED` | `IntentLogValueRejected { field, reason }` | a plan path or record field fails the 1.2 value rules at plan-build or write time | none appended; nothing staged |
| `CANONICAL_MOVE_HALTED` | `CanonicalMoveHalted { target, reason }` | any of the six halt sites during COMMITTING (v1.1): decision `FailClosed`, parent-directory create failure, rename failure, directory-sync failure, post-rename read failure/missing file or post-hash divergence from the INTENT, `DONE` append failure (replaces the generic `BinaryIntegrityFailure` count-shortfall carrier, which mislabels the digest code) | txn stays `COMMITTING`; log keeps its valid records |

#### 3.1 Normative stderr text of the three new codes (ratified v1.0; error-taxonomy v1.41 reviewed)

The product-owner's three derived one-line formats (error-taxonomy v1.41) were reviewed against the
exact-line convention of `EXPIRY_ABORT`, `FOREIGN_MIGRATION_REFUSED` and `MIGRATION_LOCK_CONTENTION`
(ADR-052 Error Code Semantics items 8, 9, 9-sibling): stderr = `<subcommand>: ` + the variant `Display`;
single line, no newline inside; the code token and the literal `exit N` appear in the line; a
past-tense claim is printed only if it is TRUE at print time (ADR-052 item 8 rule (b)); a hostile value
must not forge terminal output. All three share ONE module (Decision 4), so, unlike the `migrate-bc-index`
rows, no `BC-INDEX migration: ` label is printed; `<subcommand>` is `migrate-bc-index` or
`backfill-append-logs`. Prefix, code token and `exit 2` were ACCEPTED; four defects were CORRECTED:

- **C-1 (false claim, `INTENT_LOG_CORRUPT`).** "nothing was moved or appended" is untrue when the shared
  reader runs at Decision 3 step 5 (post-loop re-read): this run's moves already happened. Replaced by
  "no further move or append was made and no completion was recorded", true on every read site (before
  the loop it is vacuously stronger; after it, `completed.json` is not written and the txn is untouched).
- **C-2 (false claim, `INTENT_LOG_VALUE_REJECTED`).** "nothing was staged" is untrue for a rejection
  AFTER the generation directory exists (Decision 1.9 step 1: the pre-commit abort path, whose staged
  files are discarded by the abort). The line now claims only what is always true: "no record of this
  batch was appended". The abort path's writes (txn ABORTED, then gate OPEN) happen BEFORE the line is
  printed; if either fails, that write's own `Io` error is the result (exit 2), never this line (the
  `EXPIRY_ABORT` item 8 rule (b) pattern).
- **C-3 (unusable value, `INTENT_LOG_CORRUPT`).** `<path>` truncated to 64 chars would drop the
  distinguishing tail of `.factory/migration-state/intent-<uuid>.log` (25-char directory prefix plus
  47-char file name = 72). `<path>` is rendered as the log FILE NAME only, `intent-<generation_id>.log`
  (47 chars for a UUID, passed through the sanitizer anyway because the log path is read from a record).
  (v1.1 extension 2: the stderr cap is now 256, see "Sanitization cap" below, so truncation no longer
  threatens the tail; the file-name-only rendering is KEPT because the directory prefix is constant noise on
  a one-line message and the file name is the only distinguishing part. The original 64-char reasoning is
  historical.)
- **C-4 (ambiguous action, `CANONICAL_MOVE_HALTED`).** "re-run recovery or escalate" is wrong for the
  deterministic `FailClosed` rows, where a re-run reproduces the halt. Replaced by the sibling exit-2
  vocabulary ("operator investigation required") plus the true idempotence statement.

**Final text** (each is `<subcommand>: ` + exactly this, one line):

```
INTENT_LOG_CORRUPT (exit 2): intent log <log_file> is corrupt (<kind>) at byte offset <offset>; no further move or append was made and no completion was recorded; operator investigation required
INTENT_LOG_VALUE_REJECTED (exit 2): intent-log field <field> rejected: <reason>; no record of this batch was appended
CANONICAL_MOVE_HALTED (exit 2): move to <target> halted: <reason>; the transaction remains COMMITTING and no completion was recorded; operator investigation required, and after the cause is fixed a re-run resumes idempotently
```

Placeholder rules (all rendered by the SAME sanitizer as the ADR-052 v1.21 admission diagnostic and
`FOREIGN_MIGRATION_REFUSED`'s `<id>`: escape control characters and cap EACH data-derived substring at
**256** characters; raw record content is never echoed beyond that, and no serde/OS error string reaches the
line). **Sanitization cap (v1.1 extension 2; ADR-052 v1.25 item 11(g)).** The 64-character cap is the
`InternalLog`-event bound ONLY; operator stderr uses 256 per data-derived substring, for every code in this
section, so the one `sanitize_diagnostic(s, cap)` is called with 256 here. This is deliberate consistency, not a
per-code choice: `<target>` is a validated canonical path of up to 4096 bytes (Decision 1.2) and a 64-character
cap would cut it mid-name, and `<log_file>` is read from a record. Placeholders from closed token domains
(`<kind>`, `<offset>`, `<field>`, both `<reason>` sets) are far below either cap and are never truncated. A
value longer than 256 is cut after escaping, so the escaped output never exceeds 256 characters plus the
fixed text. Tests: a 200-character `<target>` appears in full on the `CANONICAL_MOVE_HALTED` line; a hostile
`<target>` with control characters is escaped and cannot add a second line. Stories: S-25.10 (`INTENT_LOG_CORRUPT`,
`INTENT_LOG_VALUE_REJECTED`) and S-25.11 (`CANONICAL_MOVE_HALTED`), neither implemented yet, so no rework;
S-25.09 carries the sibling `FOREIGN_MIGRATION_REFUSED` `<id>` change (ADR-052 item 9). Product-owner mirrors:
BC-1.18.011 (~1466, ~1487, ~1806), BC-1.18.013 (~1549, ~1559, ~1832), error-taxonomy lines 113-116.

| Placeholder | Closed domain / source |
|---|---|
| `<log_file>` | `intent-<generation_id>.log` (file name only, C-3) |
| `<kind>` | `mid_log_corruption` for `MidLogCorruption` and for a non-V1 record followed by a valid record; `log_invariant_violation` for L1-L4 (Decision 1.8) |
| `<offset>` | decimal byte offset: for `mid_log_corruption` the reader's `first_bad_offset` (= `valid_prefix_len`, NOT `later_valid_offset`); for `log_invariant_violation` the start offset of the FIRST violating record as defined in Decision 1.8 (the reader keeps each record's start offset; for L2 it is the LATER conflicting `INTENT`) |
| `<field>` | plan time: `staging_path`, `canonical_path`; write time: `txn_id`, `fencing_generation`, `record_type`, `target_canonical`, `staging_path`, `expected_post_hash`, `expected_pre_state`, `timestamp_utc` |
| `<reason>` (VALUE_REJECTED) | `contains_control_character`, `not_utf8`, `empty`, `over_4096_bytes`, `leading_or_trailing_space` (v1.1; paths only), `not_64_lowercase_hex`, `malformed_timestamp`, `not_1_to_128_token_characters`, `not_canonical_u64`, `unknown_record_type` (rules 1-7 of Decision 1.2) |
| `<target>` | the plan entry's `canonical_path` as stored |
| `<reason>` (MOVE_HALTED) | `fail_closed_diverged_after_completion` (table row 3), `fail_closed_no_recovery_copy` (row 5), `fail_closed_matches_neither_state` (row 6), `fail_closed_no_intent` (row 7), `rename_failed`, `dir_sync_failed`, `post_rename_hash_diverged`; v1.1 adds `parent_dir_create_failed`, `post_rename_read_failed`, `done_append_failed` (the seven pre-existing tokens are unchanged; ten in all: four `fail_closed_*` for site 1, one each for sites 2, 3, 4 and 6, and `post_rename_read_failed` + `post_rename_hash_diverged` for site 5) |

Variant payloads are unchanged (`IntentLogCorrupt { path, kind, offset }`, `IntentLogValueRejected { field,
reason }`, `CanonicalMoveHalted { target, reason }`); `path` is the full path, the `Display` renders its file
name. The `Display` is the contract; tests assert it with `starts_with`/`contains` on the code token and on
the full normative suffix for fixed inputs, and assert exit status 2.

### Decision 4 — One shared module

A single module `crates/factory-dispatcher/src/shard_manager/intent_log.rs` owns, for BOTH migrations:
`IntentRecord`/`RecordType`, `encode_record`, `checksum_hex`, `read_log` (1.7) and the 1.8 invariants, the
value-rule validators (1.2), `IntentLogWriter` (1.9), `decide_recovery` (Decision 3), `plan_matches_intents`
and `verify_plan_completion`, and the `PlannedCanonicalMove` plan validator (Decision 2.4). It is generic
over the migration's `Fs` seam type and the clock seam; it contains no migration-specific names. The
S-25.06 copies (`write_append_log_intent_record`, `parse_append_log_intent_log_block`,
`read_append_log_intent_log`, `append_log_intent_log_checksum_input`, `AppendLogIntentLogRecoveryDecision`,
`decide_append_log_recovery`) and the B2 originals (`append_intent_log_record`, `intent_log_checksum_input`,
`read_intent_log`, `parse_intent_log_block`, `decide_intent_log_recovery`) are DELETED, not wrapped.
TD-VSDD-060 sibling sweep: no other crate source or test may spell `INTENT_LOG_RECORD_V1` or
`END_INTENT_LOG_RECORD` outside this module and its tests (a literal-shell grep gate in S-25.10).
Sequencing across the S-25.10 / S-25.11 / S-25.12 chain (architect determination, 2026-10-08): S-25.10
introduces the module with the FORMAT pieces (`IntentRecord`/`RecordType`, `encode_record`, `checksum_hex`,
`read_log`, the value-rule validators, `IntentLogWriter`) and deletes the B2 format functions
(`append_intent_log_record`, `intent_log_checksum_input`, `read_intent_log`, `parse_intent_log_block`),
converting their call sites; S-25.11 adds the PLAN/COMPLETION pieces (`decide_recovery`,
`plan_matches_intents`, `verify_plan_completion`, the `PlannedCanonicalMove` validator), converts
`execute_canonical_path_moves` / `finish_committing_migration`, and deletes `decide_intent_log_recovery`
(its only caller is the per-move loop that S-25.11 converts, so it cannot be deleted earlier); S-25.12
proves `decide_recovery` total (Kani h1b) and runs the final sibling sweep. S-25.06 (rebased after S-25.12)
converts `backfill-append-logs` and deletes its copy.

### Decision 5 — Verification obligations (carried from ADR-052 §7b "Fault injection test mandate")

The mandate is retained and extended: tests MUST inject a fault between every step — staging-sync,
INTENT append, barrier, rename, directory sync, `DONE` append, `DONE` barrier, tail truncation,
log-creation directory sync — and each fault point MUST verify that recovery converges to the correct
terminal state with a txn-bound `DONE` for every plan target; a second pass MUST be a no-op. The reader
MUST additionally be tested by truncation at EVERY byte offset of a multi-record log (property test), by
garbage/NUL/invalid-UTF-8 tails followed by a later append, by mid-log bit flips in each field, and
against the golden vector of 1.4 with the operator recipe of 1.5. Tooling: `Fs`-seam fault injection
(OBL-1, ADR-052 §Decision 12) plus proptest; Kani proves `decide_recovery` totality (VP-147 h1b, the
new harness `proof_obl1_h1_decide_recovery_totality`; mirrored by VP-146 a1 after the S-25.06 rebase). `dm-log-writes` remains the Linux power-loss option.

## Rationale

**Why harden instead of choosing between the two encodings.** The two encodings differ only in tokens; the
defects are structural (A-1..A-6) and exist under either. Changing the code to the old spec tokens renames
`=` to `: ` and fixes nothing; amending the spec to the code as-is makes the defects normative. The code's
tokens are kept because they already appear in two writer/reader copies, four tests and the operator
runbook, and `key=value` split at the first `=` has no whitespace-trimming ambiguity (`key: value` invites
YAML-style parsing of `: ` and `#` inside paths). Cost decides the tokens; correctness decides the rest.

**Why a line-oriented text format with a raw-byte checksum, not JSON Lines or a binary frame.** Binary
length-prefixed CRC frames (LevelDB) are the most robust but end the operator's ability to verify with
`sed`/`shasum`, which ADR-052 item 11(e) and the runbook depend on. JSON Lines solves escaping but would
reverse ratified ruling (iv) ("plain text, never JSON"), needs canonicalization (RFC 8785) for a stable
checksum, and still must reject non-UTF-8 paths. Once values are validated, a fixed 11-line frame is as
safe and verifiable with standard tools. Prior art checksums raw record bytes (SQLite WAL, PostgreSQL
CRC-32C, LevelDB) and stops at the first invalid record; RocksDB's tolerate-tail / fail-on-middle mode is
exactly the policy needed here.

**Why a fixed plan with an append-only completion log.** One source of truth for progress, no extra
crash windows, no empty-list vacuous verification. A shrinking list adds N durable txn rewrites (each a new
fault-injection point) and a new disagreement window between the `DONE` append and the shrink that
recovery must resolve by trusting the log — at which point the list carries no information the log lacks.
ARIES, dpkg and Kubernetes keep the plan/desired state stable and derive progress; git's sequencer shrinks
its todo list but moves a command to `done` BEFORE running it, so even there "done" means "attempted" and
a human resolves the crash window. The merged code already chose the fixed plan; this ADR makes the choice
normative and repairs its three incomplete paths.

**Why rename now.** A field called `pending_…` that never shrinks misled the ADR's own author. No txn record
exists anywhere, so the rename costs code, tests, a Kani struct literal, the runbook and the taxonomy now;
later it would need a `schema_version` bump and a migration.

**Why tail repair by truncation.** A reader that tolerates a torn tail is not enough if the writer then
appends after it: the next record is sandwiched behind garbage, and a durable valid record before the
garbage can be lost (A-1). Truncating the torn tail under the flock before the first append keeps the file
a pure sequence of records plus at most one repaired-away tail.

## Consequences

### Positive

- A crash at any fault point converges to a log with a txn-bound, hash-confirmed `DONE` per target; the
  verifier/runbook can no longer fail forever on a correctly finished migration.
- Verification is non-circular and covers the plan: an empty or tampered plan cannot complete.
- One implementation of the format; the S-25.06 drift class (A-6) is closed by deletion.
- The checksum is operator-verifiable with `sed` and `shasum`; the golden vector pins the bytes.

### Negative / Trade-offs

- Three new exit-2 codes and a stricter reader: a log that the old reader tolerated (e.g. with a CR or a
  duplicate key) now fails closed. Acceptable: no log has ever been written by a release.
- `decide_recovery` gains an input and an outcome, so the mechanism-A harnesses that call the old function
  (VP-146 a1, a2, a5, a6; exhaustive matches need an `AlreadyDone` arm) change in S-25.06 after its rebase, and
  the B2 suite gains one harness (VP-147 h1b, S-25.12), so the OBL-1 CI job's `EXPECTED_PROOFS` rises 10 → 11 (no VP
  count change). The mechanism-A job (`kani-mechanism-a`) derives its count from its pinned list and this ADR
  adds NO mechanism-A harness (a1, a2, a5, a6 are retyped in place). CORRECTION (2026-10-08, verified against
  the S-25.06 worktree `feature/S-25.06` tip `9886cbc1`): the job pins SEVEN harnesses today (it exists only in
  that worktree, not on develop); S-25.06 T-8a adds the three VP-146 v1.1 harnesses
  (`proof_obl_a1_terminal_reconcile_totality`, `proof_obl_a4_reservation_quiescence_and_selfheal`,
  `proof_obl_a6_terminal_reconcile_idempotence`), making it TEN; the count after S-25.06 AND this ADR is TEN.
  An earlier wording "stays 7" conflated "this ADR does not change the count" with "the count is 7" and is
  withdrawn.
- `Fs` gains `truncate_durable` and `append_durable`, widening the OBL-1 fault-injection surface.
- ADR-052 and ADR-054 must be read together for Decision 7; the anchor map and the pointer table in
  ADR-052 §7b mitigate this.

### Status as of v1.0 (2026-10-08)

Normative spec accepted; implementation delivered by the chain S-25.10 (format: A-1..A-6) → S-25.11 (plan and
completion: B-1..B-3 and the name) → S-25.12 (closure: Kani h1b, runbook, sweep evidence), none started. The
pre-ADR-054 code is non-conforming on A-1..A-6, B-1..B-3 and the name; CLAUDE.md rule 12 ("the spec wins")
makes that chain the fix, and S-25.12 blocks S-25.06 and any `migrate-bc-index` release.

## Alternatives Considered

- **Option 1 — Change the code to ADR-052 v1.23's tokens only.** Rejected: renames tokens, keeps A-1..A-5.
- **Option 2 — Amend the spec to the code verbatim.** Rejected: makes the torn-record defects normative.
- **Option 3 — JSON Lines + per-record checksum.** Rejected: reverses ruling (iv), needs RFC 8785
  canonicalization, no safety gain over a validated line format.
- **Option 4 — Binary length-prefixed CRC frames.** Rejected: ends operator `sed`/`shasum` verification.
- **Option 5 — Keep the shrinking `pending_canonical_moves` (change code to spec).** Rejected: N extra
  durable rewrites, a new disagreement window, vacuous verification when the list reaches empty.
- **Option 6 — Keep the fixed list under the old name with a normative definition.** Rejected: the name is
  a standing trap and rename is free today.
- **Option 7 — Keep two copies of the format (one per migration).** Rejected: already drifted (A-6).
- **Option 8 — Move all of ADR-052 §Decision 7 into this ADR.** Rejected: ~410 unchanged lines copied by
  hand, separates the state machine from §5a/5c, ~9% size gain. Recorded in "Split decision".

## Source / Origin

- Independent research `.factory/research/adr-052-intent-log-format-and-move-list-semantics.md` (2026-10-08),
  including the external prior art (SQLite WAL, PostgreSQL WAL, LevelDB/RocksDB, git sequencer, dpkg, ARIES,
  Kubernetes spec/status, Pillai et al. OSDI'14, Apple `fsync(2)`).
- Human authorization 2026-10-08 (Status, decisions 1-3).
- Code as-built (function anchors, TD-VSDD-091): `append_intent_log_record`, `intent_log_checksum_input`,
  `read_intent_log`, `parse_intent_log_block`, `decide_intent_log_recovery`, `execute_canonical_path_moves`,
  `finish_committing_migration`, `append_intent_records_for_pending_moves`,
  `recompute_pending_canonical_moves_from_staged_generation`, `plan_recovery` in
  `crates/factory-dispatcher/src/shard_manager.rs`; `Fs::append` in `shard_manager/migration_fs.rs`; the
  S-25.06 copies in the `S-25.06` worktree.
- Behavioral contracts: BC-1.18.011 (B2 migration), BC-1.18.013 (mechanism-A migration).
- ADR-052 §Decision 7a/7b/7c/7d/7e, §Error Code Semantics "Branch C hash source", item 11(e), §Decision 12.

## Downstream to Product-Owner / Story-Writer / Test-Writer — S-25.10 / S-25.11 / S-25.12 block

### Story split — scope and sequencing (human decision 2026-10-08; supersedes the one-story plan)

The work is delivered by THREE new stories, each stacked on the previous one (branch `feature/S-25.10` from
`feature/S-25.09`, `feature/S-25.11` from `feature/S-25.10`, `feature/S-25.12` from `feature/S-25.11`).
Chain: **S-25.09 → S-25.10 → S-25.11 → S-25.12 → S-25.06**. S-25.12 **blocks S-25.06** and any
`migrate-bc-index` release; `depends_on` of S-25.06 gains S-25.12 (and transitively S-25.10, S-25.11),
`blocks:` in STORY-INDEX follows the chain. Subsystem SS-01. Anchor BCs: BC-1.18.011 (primary), BC-1.18.013
(S-25.06 consumption contract only). The AC numbering AC-001..AC-019 below is KEPT across the three stories
(so the BCs, VPs and red-test ids keep resolving); each story's own file may renumber locally but MUST cite the
ADR-054 AC id.

| Story | Title | ADR-054 ACs | Codes and artifacts it introduces | Red tests |
|---|---|---|---|---|
| **S-25.09** (existing, owner of the plumbing and the pairing rule) | Admission v1.21 | its own AC-019 | `txn_id = activation_id`; `intent_log_path` persisted WITH `generation_id`, READ never derived, enforced by `check_generation_intent_log_pair` (`5632964d`, BC-1.18.011 EC-069 / Precondition 6(f)(iii), ADR-052 item 11(e) step 3); `txn_id` / `fencing_generation` parameters into the move executor | its own |
| **S-25.10** | Hardened intent-log FORMAT | AC-001..AC-008, AC-018; AC-015 for the two log codes | `shard_manager/intent_log.rs` (format pieces), `Fs::append_durable` / `truncate_durable`, `INTENT_LOG_CORRUPT`, `INTENT_LOG_VALUE_REJECTED` | T1-T6, T10, T11, T12, T15 |
| **S-25.11** | Fixed PLAN and COMPLETION | AC-009..AC-014; AC-015 for `CANONICAL_MOVE_HALTED` | the rename `canonical_move_plan` / `PlannedCanonicalMove`, B-1/B-2/B-3, `decide_recovery`, `plan_matches_intents`, `verify_plan_completion`, `CANONICAL_MOVE_HALTED` | T7, T8, T9, T13, T14, T16, T17 |
| **S-25.12** | CLOSURE | AC-016 (Kani h1b + `kani.yml`), AC-017 (runbook), AC-019 (sweep evidence) | `proof_obl1_h1_decide_recovery_totality`, OBL-1 `EXPECTED_PROOFS` 10 → 11, runbook, captured sweep | T13 (Kani side) |

Architect determinations that the split leaves open and this ADR therefore settles (no deferral):

1. **AC-015 follows the variants.** The exhaustive exit-code table test is extended by the story that adds the
   variant: S-25.10 adds `IntentLogCorrupt` and `IntentLogValueRejected`, S-25.11 adds `CanonicalMoveHalted`;
   neither uses a wildcard arm. The by-name three-variant assertion is complete once S-25.11 has merged.
2. **AC-016's function is S-25.11's, its proof is S-25.12's.** `decide_recovery` implements the Decision 3
   table that the per-move loop of S-25.11 calls, so S-25.11 introduces it and deletes
   `decide_intent_log_recovery` (see Decision 4 sequencing). S-25.12 `depends_on` S-25.11, adds only the Kani
   harness h1b and the `kani.yml` change, and owns the verification that the three drift guards (`HARNESSES`,
   `EXPECTED_PROOFS`, the `DECLARED` grep) agree.
3. **AC-018 (parity) is S-25.10's.** The mechanism-agnostic fixture proves identical writer bytes and
   identical `DONE` field population for both migrations' parameter sets at the module level; the
   plan-dependent B-2 population checks are re-asserted by S-25.11 over the same fixture.
4. **AC-019 evidence is S-25.12's, but each story keeps its own gate.** S-25.10 runs the `INTENT_LOG_RECORD_V1`
   / `END_INTENT_LOG_RECORD` grep gate (AC-001), S-25.11 runs the rename gate (AC-009, T14); S-25.12 captures
   the combined literal-shell sweep over `crates/`, `plugins/`, `docs/` after both have merged.
5. **The pairing rule is NOT in this chain.** S-25.10/S-25.11 read the log through the recorded
   `intent_log_path` and rely on the S-25.09 pairing check having rejected an unusable path BEFORE any
   mutation; they add no pairing code.

Sizing is the story-writer's call per story against the 13-point cap.

### ADR-054 acceptance criteria (id kept across the three stories; the story table above assigns each AC)

| AC | Statement (each is testable; red test ids in the plan below) |
|---|---|
| AC-001 | `shard_manager/intent_log.rs` exists and is the only implementation of the format; `migrate-bc-index` uses it; the B2 originals (Decision 4) are deleted; a literal-shell grep finds `INTENT_LOG_RECORD_V1` / `END_INTENT_LOG_RECORD` only in that module and its tests. |
| AC-002 | `encode_record` emits exactly the 11-line frame in the fixed key order; the golden record of Decision 1.4 is byte-identical and its checksum equals the vector (T10). |
| AC-003 | The checksum is SHA-256 over the exact first nine lines; the operator recipe (Decision 1.5, `sed` + `shasum`) reproduces it for every record of a multi-record log (T10). |
| AC-004 | Value rules: the writer rejects (named `INTENT_LOG_VALUE_REJECTED`, zero bytes appended) any path containing LF, CR, NUL, other C0, DEL, a non-UTF-8 byte, an empty path, one over 4096 bytes, or one with a leading or trailing U+0020 (`leading_or_trailing_space`), a non-lowercase or wrong-length hash, a bad timestamp, a non-token `txn_id`; paths containing `=`, the pipe character, spaces and marker text round-trip exactly (T6). |
| AC-005 | Plan-build validates every `staging_path`/`canonical_path` under the same rule and aborts pre-commit with nothing staged on rejection (T6). |
| AC-006 | The reader is byte-level and line-anchored: truncation of a multi-record log at EVERY byte offset yields exactly the fully written prefix, never an error, never a partial record (T1); a tail of garbage, 512 NUL bytes, a partial start marker or invalid UTF-8 yields the prefix (T3). |
| AC-007 | Grammar strictness: duplicate key, unknown key, blank line, CRLF, reordered keys, uppercase hex, extra whitespace each make the record invalid (T5); a bit flip in each field of a middle record followed by a valid record fails closed `INTENT_LOG_CORRUPT` (T4); L1-L4 violations fail closed (T15). |
| AC-008 | Tail repair and durability: the coordinator truncates a torn tail to `valid_prefix_len` and syncs before its first append, so valid R1 + garbage + appended R3 returns R1 and R3 (T2); every append uses the strict full-sync primitive on macOS and a parent-directory sync when the call created the file, both observable through the `Fs` seam (T12). |
| AC-009 | `pending_canonical_moves` is renamed `canonical_move_plan` and `PendingCanonicalMove` → `PlannedCanonicalMove` everywhere (struct, serde key, `TXN_RECORD_REQUIRED_KEYS` still 12, nested strictness `detail`, Kani struct literal in `obl1_kani_proofs.rs`, tests, runbook); a record with the old key fails `txn_record_malformed`; a literal-shell gate finds the old name only in historical changelog text (T14). |
| AC-010 | Fixed plan: the plan is persisted in full before the pointer swap and the txn record is NOT rewritten per move; a test asserts the txn file bytes are identical across all moves except the final state transition (T17). |
| AC-011 | B-1: a fault injected after rename+directory-sync and before the `DONE` append, then re-run, leaves a txn-bound `DONE` for every plan target and the shared verifier passes; a second recovery pass changes zero bytes of the log and the txn (T7, T16). |
| AC-012 | B-2: `DONE` carries the live `txn_id` and current `fencing_generation` (parameters, no log scraping), `expected_post_hash` copied from and compared with the INTENT, `expected_pre_state` copied; a staged file mutated after step 3b yields no `DONE`, `CANONICAL_MOVE_HALTED`, never `Completed` (T8). |
| AC-013 | B-3: a COMMITTING txn with an empty plan, or a plan differing from the INTENT target set by one entry (or by one `staging_path`), exits 2 `MIGRATION_STATE_INTEGRITY_FAILURE` `txn_record_malformed`, moves nothing, writes no `completed.json` (T9). |
| AC-014 | `finish_committing_migration` calls `verify_plan_completion` over a fresh log read before writing `completed.json`; `canonical_paths_count == len(plan)`; the count-shortfall `BinaryIntegrityFailure` carrier is replaced by `CANONICAL_MOVE_HALTED`. |
| AC-015 | The exhaustive exit-code table test (S-25.09) gains the three new variants; no wildcard arm. Per story: S-25.10 adds `IntentLogCorrupt` and `IntentLogValueRejected`, S-25.11 adds `CanonicalMoveHalted` (in the S-25.11 story file this is its **AC-008**; the story renumbered locally). **S-25.11 AC-008 (ADR-054 AC-015) owns ALL SIX halt sites** of Decision 3 step 4 and the full ten-token `<reason>` domain of §3.1 (each token exercised); **S-25.09 emits NO interim halt line** (ADR-052 v1.25 item 11(f) ruling). |
| AC-016 | (S-25.11 delivers the pure function; S-25.12 delivers the proof and the CI change.) `decide_recovery` (pure, total) replaces `decide_intent_log_recovery`; NEW harness `proof_obl1_h1_decide_recovery_totality` (VP-147 h1b, in `obl1_kani_proofs.rs`) proves it total with the exact seven-row table of Decision 3 (outcomes `TreatDone`, `AlreadyDone`, `RedoRename`, `FailClosed`; input `done_present`) and its `kani::cover!` witnesses; `.github/workflows/kani.yml` pins it (OBL-1 job `HARNESSES` + `EXPECTED_PROOFS` 10 → 11, header comment "ten" → "eleven"); VP-146 a1/a2/a5/a6 are re-pointed to it by S-25.06 after its rebase. |
| AC-017 | The operator runbook `docs/guide/migration-interim-block-recovery.md` and ADR-052 item 11(e) steps are implemented as specified: checksum verification (Decision 1.5), plan-equals-INTENT-set, `DONE == INTENT == sha256(file)`, renamed field. |
| AC-018 | Parity contract for S-25.06 (T11): a mechanism-agnostic fixture shows the module produces identical writer bytes and identical `DONE` field population for both migrations' parameter sets; S-25.06 consumes it. |
| AC-019 | Sibling-sweep evidence (TD-VSDD-060): a captured literal-shell `grep -rn` run over `crates/`, `plugins/`, `docs/` shows zero live references to the old field name, the old tokens, and the deleted functions. |

### Red-test plan (adapted from the research T1-T12; all fail before the code change)

| # | Test (file: `tests/s2510_*`) | Asserts | Red today because |
|---|---|---|---|
| T1 | `s2510_intent_log_reader_test.rs`: truncate a 3-record log at every byte offset (proptest) | exactly the fully written prefix; no error; no partial record | whole-file UTF-8 decode errors and `strip_suffix` quirks |
| T2 | same file: valid R1 + garbage tail (partial START, 512 NUL, random non-UTF-8) then coordinator open + append R3 | R1 and R3 returned after tail repair; without repair the reader reports `MidLogCorruption` | R1 lost (A-1) |
| T3 | same file: non-UTF-8 byte inside an otherwise valid log's tail | earlier records returned; no `Io` | A-2 |
| T4 | same file: bit-flip in each field of a middle record followed by a valid record | `INTENT_LOG_CORRUPT` | undetected or silently dropped |
| T5 | same file: duplicate key, unknown key, blank line, CRLF, reordered keys, uppercase hex | record invalid | accepted (HashMap last-wins) |
| T6 | `s2510_intent_log_values_test.rs`: writer and plan builder given LF, CR, NUL, non-UTF-8, over-long paths; paths with `=`, pipe, marker text | `INTENT_LOG_VALUE_REJECTED`, zero bytes appended, nothing staged; benign paths round-trip | A-3 |
| T7 | `s2510_fixed_plan_recovery_test.rs`: failpoint after rename+dir-sync before `DONE`, re-run | `DONE` for every target; verifier passes; second pass zero changes | B-1 |
| T8 | same: mutate staged file after step 3b, before its rename | no `DONE`, `CANONICAL_MOVE_HALTED`, not `Completed`; DONE fields bound to txn | B-2 |
| T9 | same: COMMITTING with empty plan; with plan differing by one entry/one `staging_path` | `txn_record_malformed`, exit 2, nothing moved, no `completed.json` | B-3 (count-0 `Completed`) |
| T10 | `s2510_intent_log_golden_test.rs`: golden record bytes; operator `sed`+`shasum` recomputation over a 4-record log | byte-identical; checksum matches | A-4 |
| T11 | `s2510_intent_log_parity_test.rs`: both parameter sets through the shared module | identical bytes and `DONE` population | A-6 |
| T12 | `s2510_intent_log_durability_test.rs`: `append_durable` creating the log; macOS strict path | directory sync invoked; full-sync primitive used; failing barrier ⇒ `Io`, boundary not reached | A-5 |
| T13 | `decide_recovery` totality + the seven table rows | one outcome per row | (Kani + unit) |
| T14 | rename sweep gate (literal shell) | old field/struct name absent outside historical text; old key rejected on decode | rename |
| T15 | L1-L4: foreign `txn_id`; conflicting INTENTs; DONE before INTENT; decreasing fencing | `INTENT_LOG_CORRUPT` | no invariants today |
| T16 | idempotence: run recovery twice | second pass changes zero bytes | B-1 |
| T17 | txn file bytes across all moves | unchanged except final transition | step 7d removed |

### Code changes per story

**S-25.10 / S-25.11 / S-25.12 (the numbered list is the original single-story list; the per-item story is
fixed here).** Item 1 → S-25.10 (format pieces) with the plan/completion pieces added by S-25.11 (Decision 4
sequencing). Item 2 → S-25.10. Item 3 → S-25.10 for the deletion of the B2 format functions, the call-site
conversion of `append_intent_records_for_pending_moves` (rename to `append_intent_records_for_plan`) and the
`run_bc_index_migration` prologue path rule (AC-005, `Path::to_str`, no `to_string_lossy`); S-25.11 for
`execute_canonical_path_moves`, `finish_committing_migration`, the `plan_recovery` / `recover()` arms,
`recompute_…_from_staged_generation` and the deletion of `decide_intent_log_recovery`. Item 4 (rename,
including the `make_txn_record` literal in `obl1_kani_proofs.rs`, which must compile) → S-25.11. Item 5 →
S-25.10 for `IntentLogCorrupt` / `IntentLogValueRejected`, S-25.11 for `CanonicalMoveHalted`. Item 6
(runbook) → S-25.12. Item 7 (CHANGELOG) → each story adds its own Unreleased line. Item 8 (new harness h1b and
the `kani.yml` change) → S-25.12 only. Red-test files follow the story: `tests/s2510_intent_log_*` (T1-T6,
T10-T12, T15) in S-25.10; the fixed-plan recovery tests (T7, T8, T9, T14, T16, T17) in `tests/s2511_*` in
S-25.11 (the file name `s2510_fixed_plan_recovery_test.rs` above reads `s2511_fixed_plan_recovery_test.rs`).

1. NEW `shard_manager/intent_log.rs` (Decision 4) and wiring into `shard_manager.rs` (module declaration).
2. `migration_fs.rs` `Fs`: replace `append` with `append_durable` (strict full-sync on macOS, directory sync on create); add `truncate_durable`; update `StdFs` and every test double (`FaultFs` etc.).
3. `shard_manager.rs`: delete the B2 intent-log functions; convert `append_intent_records_for_pending_moves` (rename to `append_intent_records_for_plan`), `execute_canonical_path_moves` (txn_id and fencing parameters from S-25.09, `TreatDone` appends, INTENT-hash compare, `expected_pre_state` copy, no log scraping), `finish_committing_migration` (B-3 pre-check, `verify_plan_completion`, `CANONICAL_MOVE_HALTED`), `plan_recovery` / `recover()` arms, `recompute_…_from_staged_generation`, `run_bc_index_migration` prologue (plan validated with 1.2 path rule via `Path::to_str`, no `to_string_lossy`).
4. Rename `pending_canonical_moves` → `canonical_move_plan`, `PendingCanonicalMove` → `PlannedCanonicalMove`, `pending_moves` locals, `TXN_RECORD_REQUIRED_KEYS`, nested strictness text, `obl1_kani_proofs.rs` struct literal, ALL tests under `crates/factory-dispatcher/tests/` that spell the name (`bc_1_18_011_b2_migration_test.rs`, `bc_1_18_011_b2_migration_crash_injection_test.rs`, `s2509_*`, `s2508_*`).
5. New error variants and their `Display`/exit mapping; exhaustive table test extended.
6. Runbook `docs/guide/migration-interim-block-recovery.md` (technical-writer or devops-engineer): renamed field, checksum step, plan/INTENT equality, three-way hash equality, hardened awk.
7. CHANGELOG entry under Unreleased (internal format hardening; no released-format change).
8. Kani: in `obl1_kani_proofs.rs` rename the `make_txn_record` literal (`canonical_move_plan: Vec::new()`) and ADD `proof_obl1_h1_decide_recovery_totality` (VP-147 h1b: symbolic `done_present`, hashes from the single-character domain {absent, post, pre, foreign} as in the a1 skeleton, `#[kani::stub(std::fmt::format, ...)]` for the `FailClosed` reason strings, the seven-row table as assertions, one `kani::cover!` per row); in `.github/workflows/kani.yml` OBL-1 job add the harness to `HARNESSES`, set `EXPECTED_PROOFS=11`, and update the "ten"/"SIX logical obligations, TEN harnesses" comments (the job's own `DECLARED` grep guard fails the build if any of the three drifts). The mechanism-A job (`kani-mechanism-a`) is unchanged here: its count derives from its pinned list.

**S-25.09 (scope, restated 2026-10-08).** `txn_id = activation_id`; `intent_log_path` persisted with `generation_id`, READ and never derived, and the pairing rule enforced at the coordinator arm entry by `check_generation_intent_log_pair` (AC-019, commit `5632964d`; BC-1.18.011 EC-069 / Precondition 6(f)(iii); ADR-052 item 11(e) step 3) — the pairing rule is S-25.09's and appears in no story of the ADR-054 chain; and the `txn_id`/`fencing_generation` parameters into the move executor. It MUST NOT introduce the log-scraping `hack_txn`; if present in its worktree it is removed there.

**S-25.06 (consumes).** After S-25.12 merges (the last story of the chain): rebase; delete the five intent-log copies and `AppendLogIntentLogRecoveryDecision`; call the shared module; rename its `pending_canonical_moves` uses; AC-031 verifier = `verify_plan_completion` + Branch C steps of ADR-052 §Error Code Semantics; `finish_append_log_migration` gets the same B-1/B-2/B-3 behavior through the shared per-move procedure; its tests `s2506_append_log_backfill_test.rs` move to the hardened fixtures (torn-tail test keeps its intent, now via the shared reader).

### BC wording owed (product-owner)

**BC-1.18.011 (next version after 1.20; additive amendments):**
- Preconditions 5/6(f)(iii) and "strict-presence" list: `pending_canonical_moves` → `canonical_move_plan`; nested strictness `detail` names `canonical_move_plan[<index>]`; required key set stays TWELVE.
- Precondition 5 (durable txn record + intent log): replace "framed checksummed intent log" with a citation of ADR-054 Decision 1 and add: the plan is persisted before the pointer swap and never rewritten; completion is recorded only by txn-bound, INTENT-confirmed `DONE` records.
- NEW Postconditions: (B-1) recovery of a target already at its post-hash appends `DONE`; (B-2) `DONE` fields; (B-3) a COMMITTING txn needs a non-empty plan equal to the INTENT set else `txn_record_malformed`, nothing moved.
- Branch C hash source Postcondition 9(b) (line near the `intent_log_path` bullet): hash source is the `DONE` located via `intent_log_path`, required equal to the INTENT's and to `sha256(file)`; torn tail absent, mid-log corruption `canonical_hash_mismatch`.
- "Operator recovery of the interim block" steps 2-3 (the section mirrored from ADR-052 item 11(e)): field rename; N = `len(canonical_move_plan)` = distinct INTENT targets and non-empty; step 3 gains the checksum recipe and the three-way hash equality.
- Edge cases: renames in EC-045/EC-053/EC-058; NEW ECs (next free numbers) — crash between rename and `DONE`; `DONE` hash divergence from INTENT; empty plan; plan/INTENT set mismatch; torn tail then append; mid-log corruption; value rejection (LF path, non-UTF-8 path); conflicting INTENTs; foreign `txn_id`. Canonical test vectors: the golden record.
- Invariant: the txn record is not rewritten per canonical move.

**BC-1.18.013 (next version after 1.12):** Precondition 5 / Postcondition 5 (intent log round-trip, torn trailing record) re-worded to the shared module and the Decision 1.7 reader; the Branch C `expected_post_hash` and `intent_log_path` bullets (hash source = DONE confirmed against INTENT); the "NOT a `pending_canonical_moves` field" bullets → `canonical_move_plan`; EC-050/EC-051/EC-052 gain the plan/INTENT-set and corruption vectors; AC-031 verifier steps 2-5 reference `verify_plan_completion`. Mechanism-A N stays fixed 4 = `len(plan)`.

**error-taxonomy.md:** three NEW rows (`INTENT_LOG_CORRUPT`, `INTENT_LOG_VALUE_REJECTED`, `CANONICAL_MOVE_HALTED`; category Migration binary exit codes, blocked, exit 2); `MIGRATION_STATE_INTEGRITY_FAILURE` trigger gains "`canonical_move_plan` empty/ill-typed or differing from the intent-log INTENT target set (COMMITTING)" and its nested-key detail renamed; `COMPLETION_RECORD_MISMATCH_ABORT` trigger (a) N wording renamed and "non-empty, equal to the INTENT set"; changelog row. No change to the E-MAINTENANCE-002 `kind` tokens or to BC-3.08.001 Event 12 (the mid-log case on the admission leg is `canonical_hash_mismatch`, an existing check).

**BC-1.18.008 / BC-1.18.010:** none (they do not name the field or the format). **BC-3.08.001:** none; grep for the old name in `detail` examples only.

**VPs (no VP added or retired; VP-INDEX total unchanged; verification-architecture and coverage-matrix get ANCHOR TEXT ONLY per the VP-INDEX propagation obligation):**
- **VP-133** (B2 migration; lines naming `pending_canonical_moves`, facet 3/9 vectors): field rename; the facet that checks `canonical_paths_count ≠ len(plan)` gains the B-3 vectors; the crash-window facet gains B-1/B-2.
- **VP-143** (mechanism-A, facet (b) four-file fault injection): add the rename+dir-sync→`DONE` fault point and the hardened-log vectors (T1-T5, T7-T9, T12) as facet (b) obligations; anchor story gains S-25.10 (format vectors) and S-25.11 (rename→`DONE` fault point, B-1/B-2/B-3); the (b4) pairing rule is S-25.09 AC-019 with the S-25.06 AC-036 twin.
- **VP-146** (a1 `decide_append_log_recovery`, harness skeleton uses `AppendLogIntentLogRecoveryDecision::TreatDone`): re-point to the shared `decide_recovery`; four outcomes; new input. The same function is called by a2, a5 and a6, which change with it (S-25.06 after rebase onto S-25.12). The harness count is not changed by this ADR: the `kani-mechanism-a` job pins 7 harnesses today (S-25.06 worktree only) and 10 after S-25.06 T-8a; it stays 10 (see Consequences).
- **VP-147** (the B2 suite): `proof_obl1_h1_recover_totality` proves `recover` over txn records and never called the per-target decision, so it is NOT re-pointed; the correct change is a NEW harness h1b `proof_obl1_h1_decide_recovery_totality` over the shared `decide_recovery` (EXPECTED_PROOFS 10 → 11; S-25.12), plus the struct-literal rename (S-25.11) `pending_canonical_moves: Vec::new()` → `canonical_move_plan: Vec::new()` in `make_txn_record` (the only field-name dependency in `obl1_kani_proofs.rs`).
- **VP-INDEX / verification-architecture / verification-coverage-matrix:** record "ADR-054 anchor and S-25.10 / S-25.11 / S-25.12 re-point; no count change" (done in VP-INDEX v3.33 extension 3). Per-VP story anchors: VP-133 `S-25.02, S-25.08, S-25.09, S-25.10, S-25.11, S-25.06` (facet 11(a) → S-25.09 AC-019; facet 10, 11(b), 11(d) → S-25.11; 11(c) → S-25.10); VP-143 `S-25.08, S-25.09, S-25.10, S-25.11, S-25.06`; VP-146 `S-25.06`; VP-147 `S-25.08, S-25.12`.

**Stories:** story-writer creates S-25.10, S-25.11, S-25.12 and their STORY-INDEX rows (`depends_on` chain S-25.09 → S-25.10 → S-25.11 → S-25.12, `blocks` S-25.06 on S-25.12), amends S-25.06 (consumption, `depends_on` S-25.12, AC-031 wording; T-8a Kani count wording: the `kani-mechanism-a` job goes 7 → 10 at T-8a and stays 10 after the ADR-054 re-point, which adds no harness) and S-25.09 (plumbing plus pairing-rule scope note).

### Open items

None for the human. Rename and format decisions are made. Remaining ADR-052 hygiene (further size split) is a recommendation to the orchestrator, recorded in the ADR-052 v1.24 changelog row.

## References

- `.factory/research/adr-052-intent-log-format-and-move-list-semantics.md` — the basis (verdicts A and B)
- `ADR-052` §Decision 7a/7b/7c/7d/7e, §Error Code Semantics "Branch C hash source", item 11(e), §Decision 12
- `ADR-051` §Decision 1 (WASM fuel budget)
- RFC 5234 (ABNF), RFC 7405 (case-sensitive ABNF literals), RFC 3629 (UTF-8)
- SQLite WAL format, PostgreSQL WAL internals, LevelDB log format, RocksDB WAL recovery modes, Pillai et al. OSDI'14, Apple `fsync(2)`/`fcntl(2)`; URLs are in the research file

## Files to Change

| File | Change | Owner |
|---|---|---|
| `crates/factory-dispatcher/src/shard_manager/intent_log.rs` | NEW shared module (Decision 4): format pieces | implementer (S-25.10); plan/completion pieces (`decide_recovery`, `plan_matches_intents`, `verify_plan_completion`, `PlannedCanonicalMove` validator) implementer (S-25.11) |
| `.github/workflows/kani.yml` | OBL-1 job: pin `proof_obl1_h1_decide_recovery_totality`, `EXPECTED_PROOFS` 10 → 11 | implementer / devops-engineer (S-25.12) |
| `crates/factory-dispatcher/src/shard_manager.rs`, `shard_manager/migration_fs.rs`, `shard_manager/obl1_kani_proofs.rs` | delete B2 copies; convert call sites; `Fs` additions; rename; B-1/B-2/B-3 | implementer (S-25.10: format-function deletion, call-site conversion, `Fs` additions; S-25.11: rename, B-1/B-2/B-3, `decide_intent_log_recovery` deletion, `make_txn_record` literal; S-25.12: h1b in `obl1_kani_proofs.rs`) |
| `crates/factory-dispatcher/tests/s2510_*.rs` and renamed-field edits in existing tests | red-test plan | test-writer (S-25.10: `s2510_*`; S-25.11: `s2511_*` and renamed-field edits) |
| `docs/guide/migration-interim-block-recovery.md` | checksum step, plan rename, three-way equality | technical-writer / devops-engineer (S-25.12) |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md`, `BC-1.18.013.md` | wording owed above | product-owner |
| `.factory/specs/prd-supplements/error-taxonomy.md` | three rows + reworded triggers | product-owner |
| `.factory/specs/verification-properties/VP-133.md`, `VP-143.md`, `VP-146.md`, `VP-147.md`, `VP-INDEX.md` (+ the two architecture VP docs, anchor text only) | wording owed above | architect / spec-steward |
| `.factory/stories/S-25.10-*.md`, `S-25.11-*.md`, `S-25.12-*.md`, `STORY-INDEX.md`, `S-25.06-*.md`, `S-25.09-*.md` | three new stories; dependency chain; S-25.06 T-8a Kani count wording | story-writer |
| `.factory/specs/architecture/ARCH-INDEX.md` | ADR-054 row | architect |

## Changelog

| Version | Date | Author | Change |
|---|---|---|---|
| 1.1 (same-version extension 2, file still uncommitted) | 2026-10-08 | architect | Consistency with ADR-052 v1.25 item 11(g): the 64-character cap is InternalLog-event-only; the stderr placeholders of `INTENT_LOG_VALUE_REJECTED`, `CANONICAL_MOVE_HALTED` and `INTENT_LOG_CORRUPT` (and ADR-052 item 9's `<id>`) use 256 per data-derived substring through the one sanitizer. Edited Decision 1.9 step 1 (parenthetical), Decision 3.1 C-3 (kept file-name-only rendering; 64 reasoning now historical) and the placeholder-rules lead-in (new "Sanitization cap" paragraph). No code token, exit code, placeholder domain or VP change. Affected: S-25.10/S-25.11 (unimplemented), S-25.09 for item 9. PO owes BC-1.18.011, BC-1.18.013, error-taxonomy mirrors. |
| 1.1 (same-version extension 1, file still uncommitted) | 2026-10-08 | architect | S-25.10 test-writer ambiguities settled. (1) Decision 1.9 step 5: the WRITER syncs the parent directory iff `append_durable` returned `created == true` (fault-injection observability through the `Fs` seam; no Kani impact); retry closure (coordinator syncs the log's directory when the log pre-exists with no `DONE`); `created` via `create_new`; strict primitive named: new `pub last_amended_migrate::atomic_write::sync_file_strict_durable(&File)`. (2) Decision 3: a `DONE` is only constructed from an `INTENT`; no INTENT is table row 7 `fail_closed_no_intent`, owned by S-25.11; S-25.10 fails closed without rename or `DONE`. (3) Decision 1.8: precise "first violating record" for L1-L4 (L2 = the later conflicting `INTENT`). (4) Decision 1.7/1.5 DEFECT FIX: the mid-log probe now searches every byte offset, not only line starts, so garbage without a trailing LF followed by a valid record is `MidLogCorruption`; probe line-length bound 4113. (5) Decision 1.1/1.2 rules 1-2: paths with a leading or trailing U+0020 are rejected with the NEW token `leading_or_trailing_space` (reason precedence fixed); interior spaces valid; §3.1 and AC-004 updated. No change to Display text, exit code, variants, recovery table or other Decisions. |
| 1.1 | 2026-10-08 | architect | **ADR-054 v1.0 is committed (`.factory` 61709b2d) and immutable; this is a NEW version. Widens the `CANONICAL_MOVE_HALTED` trigger and `<reason>` domain to ALL SIX halt sites of `execute_canonical_path_moves`** (recovery `FailClosed`; parent-directory create; rename; directory sync; post-rename read failure/missing file or post-hash divergence; `DONE` append), found by the S-25.09 local adversary pass 1 ruling (ADR-052 v1.25 item 11(f)). Edited: §Decision 3 per-move procedure step 4 (six sites enumerated, none may surface as `BinaryIntegrityFailure`/`Io`), the "New named errors" Trigger cell, and the §3.1 `<reason>` (MOVE_HALTED) row (the seven existing tokens kept unchanged; three NEW: `parent_dir_create_failed`, `post_rename_read_failed`, `done_append_failed`; ten tokens in all). §Downstream: S-25.11 story AC-008 (= ADR-054 AC-015, the story renumbered locally) owns all six sites and every token; S-25.09 emits NO interim halt line. No change to the Display text (§3.1 final line), the exit code (2), variant payloads, the recovery table or any other Decision. |
| 1.0 | 2026-10-08 | architect | Initial. Companion of ADR-052; created on human authorization 2026-10-08 (research `adr-052-intent-log-format-and-move-list-semantics.md`). Moves ADR-052 §Decision 7b here and rewrites it as the HARDENED format (ABNF, byte-exact checksum, golden vector, operator recipe, byte-level reader, tail repair, durability, shared module); defines the FIXED-PLAN model with the rename `pending_canonical_moves` → `canonical_move_plan`; specifies B-1/B-2/B-3 normatively with the recovery table and three new exit-2 codes; §Downstream S-25.10 block (ACs, red-test plan, code changes, owed BC/taxonomy/VP wording). |
| 1.0 (same-version extension, file still untracked) | 2026-10-08 | architect | Kani-impact correction found while applying the owed VP wording against `obl1_kani_proofs.rs` in the S-25.09 worktree: `proof_obl1_h1_recover_totality` calls `recover` over txn records and no OBL-1 harness calls the per-target decision, so "re-point VP-147 h1" was wrong. Corrected: NEW harness `proof_obl1_h1_decide_recovery_totality` (VP-147 h1b; `EXPECTED_PROOFS` 10 → 11 in the OBL-1 kani job); the only field-name dependency in the B2 file is the `make_txn_record` literal; mechanism-A harnesses a1, a2, a5 and a6 all call the replaced function and change in S-25.06 after its rebase (count unchanged, derived from the pinned list). Edited Decision 3 note, Decision 5, Negative Consequences, AC-016, Code changes item 8, Files to Change and the VP bullets. |
| 1.0 (same-version extension 2, file still untracked) | 2026-10-08 | architect | Ratified the stderr message text of the three new codes (new §Decision 3.1): reviewed error-taxonomy v1.41's derived lines against the exact-line convention of EXPIRY_ABORT / FOREIGN_MIGRATION_REFUSED / MIGRATION_LOCK_CONTENTION (ADR-052 items 8, 9). Prefix, code token and `exit 2` accepted; four corrections C-1..C-4 (false "nothing was moved" on the post-loop re-read; false "nothing was staged" after the generation directory exists; `<path>` rendered as the log file name because 64-char truncation drops the distinguishing tail; "re-run recovery or escalate" replaced because a re-run reproduces a deterministic `FailClosed`). Closed placeholder domains added (`<kind>`, `<offset>`, `<field>`, both `<reason>` sets). Decision 1.7 surface-mapping cell reworded to match C-1. The product-owner mirrors the final text in error-taxonomy and BC-1.18.011 / BC-1.18.013. VP anchoring audit (VP-133 facet 11, VP-143 (b4)-(b8), VP-146/VP-147 boundary text) propagated in VP-INDEX v3.33 same-version extension 2. |
| 1.0 (same-version extension 3, file still untracked) | 2026-10-08 | architect | Human decisions of 2026-10-08. (1) The one-story plan is replaced by a chain of three stories stacked on S-25.09: S-25.10 format (AC-001..AC-008, AC-018, `INTENT_LOG_CORRUPT`, `INTENT_LOG_VALUE_REJECTED`), S-25.11 plan and completion (AC-009..AC-014, `CANONICAL_MOVE_HALTED`, rename, `decide_recovery`), S-25.12 closure (AC-016 h1b + `kani.yml` 10 → 11, AC-017 runbook, AC-019 sweep); chain S-25.09 → S-25.10 → S-25.11 → S-25.12 → S-25.06. Edited Status item 3, Decision 4 (new sequencing paragraph: `decide_intent_log_recovery` is deleted by S-25.11, the only story that converts its caller), Status as of v1.0, the Downstream header (new story table plus five architect determinations: AC-015 follows the variants, AC-016 function S-25.11 / proof S-25.12, AC-018 S-25.10, AC-019 sweep S-25.12 with per-story gates, no pairing code in the chain), Code changes per story, the S-25.06 consumption paragraph (rebase after S-25.12), VP and Stories bullets, Files to Change. (2) The `intent_log_path` pairing rule is S-25.09's (AC-019, `check_generation_intent_log_pair`, `5632964d`), not this chain's; S-25.09 paragraph restated. (3) CORRECTION: Consequences said the mechanism-A Kani job "stays 7"; verified against the S-25.06 worktree (tip `9886cbc1`) the job pins 7 harnesses today, S-25.06 T-8a (VP-146 v1.1) makes it 10, and this ADR adds none, so the count after S-25.06 and this ADR is 10 (VP-146 v1.5 was correct; this ADR was wrong). VP-133 v1.9 / VP-143 v1.8 / VP-146 v1.5 / VP-147 v1.5 / VP-INDEX v3.33 / verification-architecture v1.46 / verification-coverage-matrix v1.44 re-anchored in the same burst (extension 3). |
