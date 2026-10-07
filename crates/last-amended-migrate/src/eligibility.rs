//! Eligibility classification for BC-10.13.001 v1.1 Precondition 2 / EC-003
//! / PC7.

/// Whether a file's `last_amended` value is eligible for this tool's
/// migration subcommand, and — as of the v1.1 full-recovery-split amendment
/// — which of the two ELIGIBLE shapes it is in (Precondition 2 now names
/// both shapes eligible; the only remaining `NotEligible` outcome is the
/// EC-008 malformed-frontmatter case, which is not representable by this
/// enum at all since it is detected before a `last_amended` string exists to
/// classify — see `MigrateError::NotEligible`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eligibility {
    /// Already a single dated entry with no nested `[Prior: ...]` chain —
    /// migration for this field is a verified no-op (PC2, PC4). This is also
    /// the shape a file settles into immediately after a `PriorChainSplit`
    /// has been resolved (PC7 step 8 / Invariant 2) — a re-run against the
    /// freshly-split file reclassifies as `CurrentEntryOnly`, never
    /// re-detects a chain.
    CurrentEntryOnly,
    /// Contains one or more nested `[Prior: <date> (vX.Y) — ...]` bracket
    /// entries (Precondition 2(b)) — ELIGIBLE (v1.1 amendment; supersedes
    /// the v1.0 `NotEligiblePriorChain` framing, which wrongly treated chain
    /// presence as out-of-scope). The tool performs the PC7 full-recovery
    /// split: the current entry is re-emitted as the new current-entry-only
    /// `last_amended`, and every chained entry is relocated into
    /// `changelog:`, newest-first.
    PriorChainSplit,
}

/// The literal marker this crate splits on: a space followed by `[Prior:`
/// (colon immediately after `Prior`). Deliberately distinct from the
/// non-growing `[Prior history → ...]` pointer note, whose text after
/// `[Prior` is ` history`, not `:` — so a plain substring search for this
/// exact marker naturally never matches the pointer note. Shared with
/// `crate::migrate`'s PC7 split orchestration so both modules agree on
/// exactly one definition of "the marker".
pub(crate) const CHAIN_MARKER: &str = " [Prior:";

/// Classify `last_amended_raw` per BC-10.13.001 v1.1 Precondition 2 / EC-003
/// / PC7.
///
/// This function only classifies which of the two ELIGIBLE shapes the value
/// is in — it MUST NOT itself perform the PC7 bracket-splitting surgery;
/// that orchestration lives in `crate::migrate::migrate_file`, which invokes
/// this classifier and then dispatches to the split path when the result is
/// `Eligibility::PriorChainSplit`.
///
/// A single `str::contains` call — Rust's standard library substring search
/// (Two-Way algorithm) runs in linear time in both the haystack and needle
/// length with no quadratic-backtracking worst case, and this is a SINGLE
/// call (not looped/re-scanned), so this stays bounded even against the
/// D-1149 323,499-char (and up to ~350K-char) calibration ceiling (Invariant
/// 3).
pub fn check_eligibility(last_amended_raw: &str) -> Eligibility {
    if last_amended_raw.contains(CHAIN_MARKER) {
        Eligibility::PriorChainSplit
    } else {
        Eligibility::CurrentEntryOnly
    }
}
