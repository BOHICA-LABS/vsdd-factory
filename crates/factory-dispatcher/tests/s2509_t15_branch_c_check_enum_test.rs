// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.09 T-15 -- BC-1.18.013 v1.11 Postcondition 10(a) "Closed `check` value
//! domain" (ADR-052 v1.22 section 5a): the nine Branch C `check` tokens must be
//! compile-time constants of an exhaustive `enum` with
//! `fn token(self) -> &'static str`, never string literals at the emitting sites.
//!
//! Expected production API (no name is mandated by the BC / ADR-052 / story, so
//! `BranchCCheck`, mirroring `BlockBranch` / `AdvisoryReason`):
//! `pub enum BranchCCheck` in `shard_manager/admission.rs`, re-exported from
//! `factory_dispatcher::shard_manager` (`pub`, `Copy`, `Debug`, `PartialEq`),
//! exactly nine variants in BC order, `pub const ALL: [Self; 9]` in BC order,
//! `pub fn token(self) -> &'static str`.

use std::collections::BTreeSet;

use factory_dispatcher::shard_manager::BranchCCheck;

/// BC-1.18.013 v1.11 Postcondition 10(a), checks (1)-(9), in the BC's fixed order.
const BC_NINE_TOKENS: [&str; 9] = [
    "staging_with_terminal_record",
    "terminal_record_unparseable",
    "terminal_record_schema_mismatch",
    "txn_id_mismatch",
    "generation_id_mismatch",
    "canonical_paths_count_mismatch",
    "canonical_hash_mismatch",
    "terminal_record_unverified",
    "finalize_unwired",
];

#[test]
fn test_BC_1_18_013_10a_branch_c_check_tokens_equal_bc_nine_in_order() {
    let tokens: Vec<&'static str> = BranchCCheck::ALL.iter().map(|c| c.token()).collect();
    assert_eq!(tokens, BC_NINE_TOKENS);
}

#[test]
fn test_BC_1_18_013_10a_branch_c_check_tokens_are_unique() {
    let set: BTreeSet<&'static str> = BranchCCheck::ALL.iter().map(|c| c.token()).collect();
    assert_eq!(set.len(), BranchCCheck::ALL.len());
    assert_eq!(set.len(), 9);
}

#[test]
fn test_BC_1_18_013_10a_branch_c_check_tokens_are_snake_case_and_at_most_64_chars() {
    for c in BranchCCheck::ALL {
        let t = c.token();
        assert!(!t.is_empty() && t.chars().count() <= 64, "{t:?}");
        assert!(
            t.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{t:?} is not snake_case"
        );
    }
}

/// Exhaustive-match compile pin: NO wildcard arm. Adding a tenth variant makes
/// this fail to compile (E0004), forcing the BC / this test to be revisited.
/// Also pins the BC order -> variant mapping via `ALL`.
fn bc_ordinal(c: BranchCCheck) -> usize {
    match c {
        BranchCCheck::StagingWithTerminalRecord => 1,
        BranchCCheck::TerminalRecordUnparseable => 2,
        BranchCCheck::TerminalRecordSchemaMismatch => 3,
        BranchCCheck::TxnIdMismatch => 4,
        BranchCCheck::GenerationIdMismatch => 5,
        BranchCCheck::CanonicalPathsCountMismatch => 6,
        BranchCCheck::CanonicalHashMismatch => 7,
        BranchCCheck::TerminalRecordUnverified => 8,
        BranchCCheck::FinalizeUnwired => 9,
    }
}

#[test]
fn test_BC_1_18_013_10a_branch_c_check_exhaustive_match_pins_variant_set_and_order() {
    for (i, c) in BranchCCheck::ALL.iter().enumerate() {
        assert_eq!(bc_ordinal(*c), i + 1, "ALL[{i}] out of BC order");
    }
    assert_eq!(BranchCCheck::ALL.len(), 9);
}
