// Bridge between the reference oracle (`s2510_support/mod.rs`) and the not-yet-existing
// `factory_dispatcher::shard_manager::intent_log` module (ADR-054 Decision 4). Only the
// module-level test files include this; the black-box files must not (see mod.rs).
#![allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! # Expected public API of `shard_manager::intent_log` (the contract these tests pin)
//!
//! `shard_manager.rs` must declare the module `pub mod intent_log;` (integration tests live
//! in `tests/`, so `pub(crate)` is not enough). Items, all `pub`:
//!
//! ```text
//! pub enum RecordType { Intent, Done, Aborted }
//! pub struct IntentRecord {                       // derives Debug, Clone, PartialEq, Eq
//!     pub txn_id: String, pub fencing_generation: u64, pub record_type: RecordType,
//!     pub target_canonical: PathBuf, pub staging_path: PathBuf,
//!     pub expected_post_hash: String,
//!     pub expected_pre_state: Option<String>,     // None == the MISSING sentinel
//!     pub timestamp_utc: String,
//! }                                               // no checksum field: it is computed
//! pub fn encode_record(&IntentRecord) -> Result<Vec<u8>, BcIndexMigrationError>
//!     // validates (Decision 1.2) then emits the 11 LF-terminated lines;
//!     // Err(IntentLogValueRejected { field, reason }) with the closed-domain tokens
//! pub fn checksum_hex(first_nine_lines: &[u8]) -> String   // lowercase-hex SHA-256
//! pub fn validate_path(&Path) -> Result<(), &'static str>  // Err(reason token) per rule 2
//! pub struct LogRead { pub records: Vec<IntentRecord>, pub valid_prefix_len: usize, .. }
//! pub enum Invariant { L1, L2, L3, L4 }                    // derives Debug, Clone, Copy, PartialEq, Eq
//! pub enum LogReadError {                                  // derives Debug, Clone, PartialEq, Eq
//!     MidLogCorruption { first_bad_offset: usize, later_valid_offset: usize },
//!     InvariantViolation { invariant: Invariant, offset: usize },
//! }
//! pub fn read_log(bytes: &[u8], txn_id: &str) -> Result<LogRead, LogReadError>   // pure
//! pub struct IntentLogWriter<'a, F: Fs> { .. }
//! impl<'a, F: Fs> IntentLogWriter<'a, F> {
//!     pub fn open(fs: &'a F, log_path: &Path, txn_id: &str)
//!         -> Result<Self, BcIndexMigrationError>
//!         // Decision 1.9 step 2: read through fs, read_log; MidLogCorruption / invariant
//!         // violation => Err(IntentLogCorrupt{..}) with NOTHING mutated; a torn tail =>
//!         // fs.truncate_durable(log_path, valid_prefix_len) BEFORE returning; an absent
//!         // file => Ok (the file is created by the first append, not by open)
//!     pub fn append_batch(&mut self, records: &[IntentRecord])
//!         -> Result<(), BcIndexMigrationError>
//!         // validate EVERY record first (zero bytes on any rejection), serialize, ONE
//!         // fs.append_durable call for the whole batch, then fs.fsync_dir(parent) iff
//!         // that call returned created == true; any Err means the WAL boundary is NOT reached
//! }
//! ```
//!
//! `Fs` (in `shard_manager::migration_fs`): the existing nine methods minus `append`, plus
//!
//! ```text
//! fn append_durable(&self, path: &Path, content: &[u8]) -> Result<bool, BcIndexMigrationError>
//!     // O_APPEND + write_all + platform barrier (F_FULLFSYNC on macOS); Ok(created)
//! fn truncate_durable(&self, path: &Path, len: u64) -> Result<(), BcIndexMigrationError>
//!     // set_len + platform barrier
//! ```
//!
//! `StdFs` carries `migration_failpoint!("migration_fs::append_durable", path)` and
//! `migration_failpoint!("migration_fs::truncate_durable", path)` (the `migration_fs::<op>`
//! naming of the existing seams).
//!
//! `BcIndexMigrationError` gains `IntentLogCorrupt { path: PathBuf, kind: String, offset: u64 }`
//! and `IntentLogValueRejected { field: String, reason: String }` (String payloads holding the
//! closed-domain tokens, rendered through the shared sanitizer).

use std::path::PathBuf;

#[allow(unused_imports)]
pub use factory_dispatcher::shard_manager::intent_log::{
    IntentLogWriter, IntentRecord, Invariant, LogRead, LogReadError, RecordType, checksum_hex,
    encode_record, read_log, validate_path,
};

use crate::support::Rec;

pub fn rtype(s: &str) -> RecordType {
    match s {
        "INTENT" => RecordType::Intent,
        "DONE" => RecordType::Done,
        "ABORTED" => RecordType::Aborted,
        other => panic!("test fixture error: unknown record type {other}"),
    }
}

pub fn to_intent(r: &Rec) -> IntentRecord {
    IntentRecord {
        txn_id: r.txn_id.clone(),
        fencing_generation: r.fencing,
        record_type: rtype(&r.rtype),
        target_canonical: PathBuf::from(&r.target),
        staging_path: PathBuf::from(&r.staging),
        expected_post_hash: r.post.clone(),
        expected_pre_state: if r.pre == "MISSING" {
            None
        } else {
            Some(r.pre.clone())
        },
        timestamp_utc: r.ts.clone(),
    }
}

pub fn to_intents(rs: &[Rec]) -> Vec<IntentRecord> {
    rs.iter().map(to_intent).collect()
}
