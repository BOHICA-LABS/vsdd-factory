//! The ONE intent-log implementation (ADR-054 Decision 1 and Decision 4;
//! BC-1.18.011 Postcondition 15; BC-1.18.013 Precondition 5).
//!
//! This module owns the byte-exact wire format of the migration intent log
//! (`intent-<generation_uuid>.log`): the 11-line record frame, the checksum
//! over the exact on-disk bytes of the first nine lines, the Decision 1.2
//! value rules, the byte-level reader with its torn-tail / mid-log-corruption
//! discrimination (Decision 1.7) and the L1-L4 invariants (Decision 1.8), and
//! the durable writer (Decision 1.9). It is generic over the migration's
//! [`Fs`] seam and names no migration-specific type: the plan and completion
//! pieces of Decision 4 (`decide_recovery`, `plan_matches_intents`,
//! `verify_plan_completion`, the plan validator) are added to this file by
//! S-25.11.
//!
//! # Purity
//!
//! `encode_record`, `checksum_hex`, `validate_path`, `read_log` and the
//! invariant checks are pure: the reader's only input is the byte buffer the
//! shell read, and it never decodes the file as one UTF-8 string. Every
//! effect of [`IntentLogWriter`] goes through [`Fs`], so OBL-1 fault injection
//! can fail each step independently.

use std::path::{Path, PathBuf};

use super::BcIndexMigrationError;
use super::migration_fs::Fs;
use super::sha256_hex;

/// Start marker line (without its LF).
const START_LINE: &[u8] = b"INTENT_LOG_RECORD_V1";
/// End marker line (without its LF).
const END_LINE: &[u8] = b"END_INTENT_LOG_RECORD";
/// The 9 keyed lines between the markers, in their fixed order; `record_checksum`
/// is the ninth keyed line and is NOT covered by the checksum.
const KEYS: [&str; 9] = [
    "txn_id",
    "fencing_generation",
    "record_type",
    "target_canonical",
    "staging_path",
    "expected_post_hash",
    "expected_pre_state",
    "timestamp_utc",
    "record_checksum",
];
/// A record is exactly this many LF-terminated lines.
const RECORD_LINES: usize = 11;
/// The checksum covers the first nine lines (start marker + eight keyed lines).
const CHECKSUMMED_LINES: usize = 9;
/// Longest legal line, LF excluded: `len("target_canonical=") + 4096`. A longer
/// line is invalid without reading further (bounds the probe cost).
const MAX_LINE_BYTES: usize = 17 + MAX_PATH_BYTES;
/// Decision 1.2 rule 2: a path is 1 to 4096 BYTES.
const MAX_PATH_BYTES: usize = 4096;
/// Decision 1.2 rule 3: a `txn_id` is 1 to 128 token characters.
const MAX_TXN_ID_CHARS: usize = 128;
const LF: u8 = b'\n';

/// `record_type` (Decision 1.2 rule 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// Written before any rename.
    Intent,
    /// Completion evidence.
    Done,
    /// Reserved; written by no flow, accepted by the reader, never counts
    /// toward completion.
    Aborted,
}

impl RecordType {
    fn as_str(self) -> &'static str {
        match self {
            RecordType::Intent => "INTENT",
            RecordType::Done => "DONE",
            RecordType::Aborted => "ABORTED",
        }
    }

    fn parse(value: &[u8]) -> Option<Self> {
        match value {
            b"INTENT" => Some(RecordType::Intent),
            b"DONE" => Some(RecordType::Done),
            b"ABORTED" => Some(RecordType::Aborted),
            _ => None,
        }
    }
}

/// One validated record of the intent log. The checksum is computed, never
/// carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRecord {
    pub txn_id: String,
    pub fencing_generation: u64,
    pub record_type: RecordType,
    pub target_canonical: PathBuf,
    pub staging_path: PathBuf,
    pub expected_post_hash: String,
    /// `None` is the `MISSING` sentinel: the canonical path was absent at
    /// INTENT time.
    pub expected_pre_state: Option<String>,
    pub timestamp_utc: String,
}

// ---------------------------------------------------------------------------
// Value rules (Decision 1.2) -- shared by the writer and the reader
// ---------------------------------------------------------------------------

fn is_lower_hex64(value: &[u8]) -> bool {
    value.len() == 64
        && value
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}

fn is_token_txn_id(value: &[u8]) -> bool {
    (1..=MAX_TXN_ID_CHARS).contains(&value.len())
        && value
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Canonical decimal `u64`: `"0"` or a non-zero-led digit string that fits.
fn parse_canonical_u64(value: &[u8]) -> Option<u64> {
    let canonical = match value {
        [b'0'] => true,
        [first, rest @ ..] => (b'1'..=b'9').contains(first) && rest.iter().all(u8::is_ascii_digit),
        [] => false,
    };
    if !canonical {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse::<u64>().ok()
}

/// `utc-ts` of Decision 1.2 rule 6: `YYYY-MM-DDTHH:MM:SS[.f{1,9}]Z`, month
/// 01-12, day 01-31, hour 00-23, minute 00-59, second 00-59, `Z` only.
fn is_utc_timestamp(value: &[u8]) -> bool {
    let two = |i: usize| -> Option<u32> {
        let (a, b) = (*value.get(i)?, *value.get(i + 1)?);
        (a.is_ascii_digit() && b.is_ascii_digit())
            .then(|| u32::from(a - b'0') * 10 + u32::from(b - b'0'))
    };
    if value.len() < 20 || value.last() != Some(&b'Z') {
        return false;
    }
    let year_ok = value[..4].iter().all(u8::is_ascii_digit);
    let shape_ok = value[4] == b'-'
        && value[7] == b'-'
        && value[10] == b'T'
        && value[13] == b':'
        && value[16] == b':';
    let range_ok = matches!(two(5), Some(1..=12))
        && matches!(two(8), Some(1..=31))
        && matches!(two(11), Some(0..=23))
        && matches!(two(14), Some(0..=59))
        && matches!(two(17), Some(0..=59));
    let fraction = &value[19..value.len() - 1];
    let fraction_ok = fraction.is_empty()
        || (fraction[0] == b'.'
            && (2..=10).contains(&fraction.len())
            && fraction[1..].iter().all(u8::is_ascii_digit));
    year_ok && shape_ok && range_ok && fraction_ok
}

/// Decision 1.2 rule 2 over the raw bytes of a path value. `Err` carries the
/// closed-domain reason token; precedence (first listed wins): `empty`,
/// `over_4096_bytes`, `not_utf8`, `contains_control_character`,
/// `leading_or_trailing_space`.
fn check_path_bytes(bytes: &[u8]) -> Result<&str, &'static str> {
    if bytes.is_empty() {
        return Err("empty");
    }
    if bytes.len() > MAX_PATH_BYTES {
        return Err("over_4096_bytes");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "not_utf8")?;
    if text.chars().any(|c| c <= '\u{1f}' || c == '\u{7f}') {
        return Err("contains_control_character");
    }
    if text.starts_with(' ') || text.ends_with(' ') {
        return Err("leading_or_trailing_space");
    }
    Ok(text)
}

/// Decision 1.2 rule 2 for a path as the migration holds it. A path that is
/// not valid UTF-8 (`OsStr::to_str() == None`) is NOT representable: it is
/// rejected as `not_utf8`, never converted lossily. The one function shared by
/// the writer and the plan builder, so both apply the same reason precedence.
pub fn validate_path(path: &Path) -> Result<(), &'static str> {
    checked_path_str(path).map(|_| ())
}

fn checked_path_str(path: &Path) -> Result<&str, &'static str> {
    let os = path.as_os_str();
    // Length and emptiness come first in the precedence, whatever the encoding.
    if os.is_empty() {
        return Err("empty");
    }
    if os.len() > MAX_PATH_BYTES {
        return Err("over_4096_bytes");
    }
    let text = os.to_str().ok_or("not_utf8")?;
    check_path_bytes(text.as_bytes())
}

fn rejected(field: &str, reason: &str) -> BcIndexMigrationError {
    BcIndexMigrationError::IntentLogValueRejected {
        field: field.to_string(),
        reason: reason.to_string(),
    }
}

/// Validate every field of `record` against Decision 1.2 and return the two
/// path texts. Nothing is written by a caller on `Err`.
fn validate_record(record: &IntentRecord) -> Result<(&str, &str), BcIndexMigrationError> {
    if !is_token_txn_id(record.txn_id.as_bytes()) {
        return Err(rejected("txn_id", "not_1_to_128_token_characters"));
    }
    let target = checked_path_str(&record.target_canonical)
        .map_err(|reason| rejected("target_canonical", reason))?;
    let staging = checked_path_str(&record.staging_path)
        .map_err(|reason| rejected("staging_path", reason))?;
    if !is_lower_hex64(record.expected_post_hash.as_bytes()) {
        return Err(rejected("expected_post_hash", "not_64_lowercase_hex"));
    }
    if let Some(pre) = &record.expected_pre_state
        && !is_lower_hex64(pre.as_bytes())
    {
        return Err(rejected("expected_pre_state", "not_64_lowercase_hex"));
    }
    if !is_utc_timestamp(record.timestamp_utc.as_bytes()) {
        return Err(rejected("timestamp_utc", "malformed_timestamp"));
    }
    Ok((target, staging))
}

// ---------------------------------------------------------------------------
// Encoder and checksum (Decision 1.4)
// ---------------------------------------------------------------------------

/// Lowercase-hex SHA-256 of the exact on-disk bytes of a record's first nine
/// lines (Decision 1.4); nothing is reconstructed from parsed values.
#[must_use]
pub fn checksum_hex(first_nine_lines: &[u8]) -> String {
    sha256_hex(first_nine_lines)
}

/// Validate `record` (Decision 1.2) and emit its 11 LF-terminated lines in the
/// fixed key order, the checksum computed over the same buffer that is
/// written. `Err(IntentLogValueRejected { field, reason })` with the closed
/// domain tokens on any violation; no bytes are produced on `Err`.
pub fn encode_record(record: &IntentRecord) -> Result<Vec<u8>, BcIndexMigrationError> {
    let (target, staging) = validate_record(record)?;
    let mut out = Vec::with_capacity(512 + target.len() + staging.len());
    let mut push_line = |key: Option<&str>, value: &str| {
        if let Some(key) = key {
            out.extend_from_slice(key.as_bytes());
            out.push(b'=');
        }
        out.extend_from_slice(value.as_bytes());
        out.push(LF);
    };
    push_line(None, "INTENT_LOG_RECORD_V1");
    push_line(Some(KEYS[0]), &record.txn_id);
    push_line(Some(KEYS[1]), &record.fencing_generation.to_string());
    push_line(Some(KEYS[2]), record.record_type.as_str());
    push_line(Some(KEYS[3]), target);
    push_line(Some(KEYS[4]), staging);
    push_line(Some(KEYS[5]), &record.expected_post_hash);
    push_line(
        Some(KEYS[6]),
        record.expected_pre_state.as_deref().unwrap_or("MISSING"),
    );
    push_line(Some(KEYS[7]), &record.timestamp_utc);
    let checksum = checksum_hex(&out);
    out.extend_from_slice(KEYS[8].as_bytes());
    out.push(b'=');
    out.extend_from_slice(checksum.as_bytes());
    out.push(LF);
    out.extend_from_slice(END_LINE);
    out.push(LF);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Reader (Decision 1.7) and invariants (Decision 1.8)
// ---------------------------------------------------------------------------

/// The result of reading a log: the longest run of valid records from offset
/// 0, the byte offset each record starts at, and the length of that valid
/// prefix. Bytes after `valid_prefix_len` are a torn tail (absent, repaired by
/// the writer before the first append).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRead {
    pub records: Vec<IntentRecord>,
    /// Start offset of each record in `records` (same order).
    pub record_offsets: Vec<usize>,
    pub valid_prefix_len: usize,
}

/// A log-level invariant of Decision 1.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invariant {
    L1,
    L2,
    L3,
    L4,
}

/// A fail-closed verdict of the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogReadError {
    /// Bytes after the valid prefix are followed, at ANY byte position, by a
    /// complete checksum-valid record.
    MidLogCorruption {
        first_bad_offset: usize,
        later_valid_offset: usize,
    },
    /// The first record (in file order) that violates `invariant`; `offset` is
    /// its start offset.
    InvariantViolation { invariant: Invariant, offset: usize },
}

impl LogReadError {
    /// The coordinator-surface verdict: `INTENT_LOG_CORRUPT` with the closed
    /// `<kind>` token and the offset ADR-054 Decision 3.1 assigns (the reader's
    /// `first_bad_offset`, NOT `later_valid_offset`, for mid-log corruption).
    #[must_use]
    pub fn into_error(self, log_path: &Path) -> BcIndexMigrationError {
        let (kind, offset) = match self {
            LogReadError::MidLogCorruption {
                first_bad_offset, ..
            } => ("mid_log_corruption", first_bad_offset),
            LogReadError::InvariantViolation { offset, .. } => ("log_invariant_violation", offset),
        };
        BcIndexMigrationError::IntentLogCorrupt {
            path: log_path.to_path_buf(),
            kind: kind.to_string(),
            offset: offset as u64,
        }
    }
}

/// Parse the record starting at `off`, or `None` if any of its 11 lines, a
/// value rule, its checksum or its END line fails. PREFIX mode (`probe ==
/// false`, the scan from offset 0) requires `off == 0` or a preceding LF; PROBE
/// mode (the mid-log search) has no line-start requirement.
fn parse_record_at(bytes: &[u8], off: usize, probe: bool) -> Option<(IntentRecord, usize)> {
    if !probe && off != 0 && bytes.get(off - 1) != Some(&LF) {
        return None;
    }
    let mut lines: [&[u8]; RECORD_LINES] = [&[]; RECORD_LINES];
    let mut pos = off;
    for line in &mut lines {
        let window_end = bytes.len().min(pos + MAX_LINE_BYTES + 1);
        let rel = bytes.get(pos..window_end)?.iter().position(|b| *b == LF)?;
        *line = &bytes[pos..pos + rel];
        pos += rel + 1;
    }
    if lines[0] != START_LINE || lines[RECORD_LINES - 1] != END_LINE {
        return None;
    }
    let mut values: [&[u8]; 9] = [&[]; 9];
    for (i, key) in KEYS.iter().enumerate() {
        let line = lines[i + 1];
        let rest = line.strip_prefix(key.as_bytes())?;
        values[i] = rest.strip_prefix(b"=")?;
    }
    let [
        txn_id,
        fencing,
        record_type,
        target,
        staging,
        post_hash,
        pre_state,
        timestamp,
        checksum,
    ] = values;

    if !is_token_txn_id(txn_id) {
        return None;
    }
    let fencing_generation = parse_canonical_u64(fencing)?;
    let record_type = RecordType::parse(record_type)?;
    let target_canonical = check_path_bytes(target).ok()?;
    let staging_path = check_path_bytes(staging).ok()?;
    if !is_lower_hex64(post_hash) {
        return None;
    }
    let expected_pre_state = if pre_state == b"MISSING" {
        None
    } else if is_lower_hex64(pre_state) {
        Some(String::from_utf8_lossy(pre_state).into_owned())
    } else {
        return None;
    };
    if !is_utc_timestamp(timestamp) || !is_lower_hex64(checksum) {
        return None;
    }
    // The checksum is over the exact on-disk bytes of lines 1..=9.
    let covered: usize = lines[..CHECKSUMMED_LINES].iter().map(|l| l.len() + 1).sum();
    if checksum_hex(&bytes[off..off + covered]).as_bytes() != checksum {
        return None;
    }
    Some((
        IntentRecord {
            // All of these were validated as ASCII / UTF-8 above.
            txn_id: String::from_utf8_lossy(txn_id).into_owned(),
            fencing_generation,
            record_type,
            target_canonical: PathBuf::from(target_canonical),
            staging_path: PathBuf::from(staging_path),
            expected_post_hash: String::from_utf8_lossy(post_hash).into_owned(),
            expected_pre_state,
            timestamp_utc: String::from_utf8_lossy(timestamp).into_owned(),
        },
        pos,
    ))
}

/// Per-target state of the Decision 1.8 in-order pass.
struct FirstIntent<'r> {
    staging_path: &'r Path,
    expected_post_hash: &'r str,
    expected_pre_state: Option<&'r str>,
}

impl FirstIntent<'_> {
    fn agrees_with(&self, record: &IntentRecord) -> bool {
        self.staging_path == record.staging_path
            && self.expected_post_hash == record.expected_post_hash
            && self.expected_pre_state == record.expected_pre_state.as_deref()
    }
}

/// Decision 1.8 "first violating record": ONE in-order pass, every check judged
/// against the PRECEDING records only; the first record failing any check is
/// reported, attributed to the first of L1, L2, L3, L4 it fails.
fn check_invariants(
    records: &[IntentRecord],
    offsets: &[usize],
    txn_id: &str,
) -> Result<(), LogReadError> {
    let mut first_intents: std::collections::HashMap<&Path, FirstIntent<'_>> =
        std::collections::HashMap::new();
    let mut max_fencing: Option<u64> = None;
    for (record, &offset) in records.iter().zip(offsets) {
        let violated = if record.txn_id != txn_id {
            Some(Invariant::L1)
        } else {
            let first = first_intents.get(record.target_canonical.as_path());
            match (record.record_type, first) {
                (RecordType::Intent, Some(first)) if !first.agrees_with(record) => {
                    Some(Invariant::L2)
                }
                (RecordType::Done, None) => Some(Invariant::L3),
                (RecordType::Done, Some(first)) if !first.agrees_with(record) => {
                    Some(Invariant::L3)
                }
                _ if max_fencing.is_some_and(|max| record.fencing_generation < max) => {
                    Some(Invariant::L4)
                }
                _ => None,
            }
        };
        if let Some(invariant) = violated {
            return Err(LogReadError::InvariantViolation { invariant, offset });
        }
        if record.record_type == RecordType::Intent {
            first_intents
                .entry(record.target_canonical.as_path())
                .or_insert_with(|| FirstIntent {
                    staging_path: &record.staging_path,
                    expected_post_hash: &record.expected_post_hash,
                    expected_pre_state: record.expected_pre_state.as_deref(),
                });
        }
        max_fencing = Some(max_fencing.map_or(record.fencing_generation, |max| {
            max.max(record.fencing_generation)
        }));
    }
    Ok(())
}

/// Read a log's bytes (Decision 1.7). Pure: the only input is `bytes`, never
/// decoded as one UTF-8 string. A torn tail is ABSENT (the valid prefix is
/// returned); bytes after the valid prefix that are followed, at ANY byte
/// position, by a complete checksum-valid record are `MidLogCorruption`; the
/// L1-L4 invariants (Decision 1.8) are then applied to the parsed records.
pub fn read_log(bytes: &[u8], txn_id: &str) -> Result<LogRead, LogReadError> {
    let mut off = 0usize;
    let mut records = Vec::new();
    let mut offsets = Vec::new();
    while off != bytes.len() {
        let Some((record, next)) = parse_record_at(bytes, off, false) else {
            break;
        };
        records.push(record);
        offsets.push(off);
        off = next;
    }
    let valid_prefix_len = off;
    // Probe EVERY byte offset after the valid prefix (not only line starts).
    let marker: &[u8] = b"INTENT_LOG_RECORD_V1\n";
    for q in (valid_prefix_len + 1)..bytes.len() {
        if bytes[q..].starts_with(marker) && parse_record_at(bytes, q, true).is_some() {
            return Err(LogReadError::MidLogCorruption {
                first_bad_offset: valid_prefix_len,
                later_valid_offset: q,
            });
        }
    }
    check_invariants(&records, &offsets, txn_id)?;
    Ok(LogRead {
        records,
        record_offsets: offsets,
        valid_prefix_len,
    })
}

// ---------------------------------------------------------------------------
// Writer (Decision 1.9)
// ---------------------------------------------------------------------------

/// The durable intent-log writer. Used under the held `exclusive.lock` flock.
///
/// [`IntentLogWriter::open`] reads the log through the seam, runs
/// [`read_log`] and durably truncates a torn tail (Decision 1.9 step 2);
/// [`IntentLogWriter::append_batch`] validates every record before any byte,
/// appends the whole batch in ONE `append_durable` call and syncs the parent
/// directory iff that call created the file (steps 1, 3-5).
pub struct IntentLogWriter<'a, F: Fs> {
    fs: &'a F,
    log_path: PathBuf,
    /// The records the log holds: those read at `open`, plus every batch this
    /// writer appended successfully.
    records: Vec<IntentRecord>,
    /// Whether the log file existed when this writer was opened.
    existed: bool,
}

impl<'a, F: Fs> IntentLogWriter<'a, F> {
    /// Open and repair (once per run, before the first append). A missing file
    /// is `Ok` (created by the first append, not by `open`). `MidLogCorruption`
    /// or an invariant violation is `IntentLogCorrupt` with NOTHING mutated; a
    /// torn tail is durably truncated to `valid_prefix_len` BEFORE returning. A
    /// clean existing log performs no mutating operation.
    pub fn open(fs: &'a F, log_path: &Path, txn_id: &str) -> Result<Self, BcIndexMigrationError> {
        let (records, existed) = match fs.read(log_path)? {
            None => (Vec::new(), false),
            Some(bytes) => {
                let read = read_log(&bytes, txn_id).map_err(|e| e.into_error(log_path))?;
                if read.valid_prefix_len < bytes.len() {
                    fs.truncate_durable(log_path, read.valid_prefix_len as u64)?;
                }
                (read.records, true)
            }
        };
        Ok(Self {
            fs,
            log_path: log_path.to_path_buf(),
            records,
            existed,
        })
    }

    /// The records the log holds (as read at `open`, plus this writer's own
    /// successful appends).
    #[must_use]
    pub fn records(&self) -> &[IntentRecord] {
        &self.records
    }

    /// Whether the log file existed when this writer was opened.
    #[must_use]
    pub fn log_existed_at_open(&self) -> bool {
        self.existed
    }

    /// Whether the log currently holds a `DONE` record (the rename phase).
    #[must_use]
    pub fn holds_done_record(&self) -> bool {
        self.records
            .iter()
            .any(|r| r.record_type == RecordType::Done)
    }

    /// Append `records` as ONE batch. Every record is validated and serialized
    /// first (zero bytes on any rejection), then ONE `append_durable` call
    /// carries the whole batch, then `fsync_dir(parent)` runs iff that call
    /// created the file. Any `Err` means the WAL boundary was NOT reached. An
    /// empty batch appends nothing.
    pub fn append_batch(&mut self, records: &[IntentRecord]) -> Result<(), BcIndexMigrationError> {
        if records.is_empty() {
            return Ok(());
        }
        let mut batch = Vec::new();
        for record in records {
            batch.extend_from_slice(&encode_record(record)?);
        }
        let created = self.fs.append_durable(&self.log_path, &batch)?;
        if created {
            let parent = self
                .log_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            self.fs.fsync_dir(parent)?;
        }
        self.existed = true;
        self.records.extend_from_slice(records);
        Ok(())
    }
}
