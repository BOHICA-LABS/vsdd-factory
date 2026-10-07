//! Pure partition function for sync/async plugin group split.
//!
//! S-15.01 T-3b — BC-1.14.001 postconditions 1, 5, 6.
//!
//! `partition_plugins` splits a list of matched registry entries into:
//! - `sync_group`: entries where `async_flag == false` (default)
//! - `async_group`: entries where `async_flag == true`
//!
//! The function is pure, deterministic, and Kani-friendly:
//! - No I/O
//! - No async execution
//! - No global state
//! - No side effects
//!
//! VP-077 Kani proof harnesses validate the four key properties:
//! 1. Totality + disjointness: every matched plugin appears in exactly one group
//! 2. Async-field respect: async=true lands in async_group; async=false in sync_group
//! 3. Exit-code independence: async_group results never affect dispatcher_exit
//! 4. Aggregation correctness: exit=2 iff any sync plugin has block_intent=true
//!
//! ASYNC_DRAIN_WINDOW_MS for async-group drain is defined in DI-019 — cite by
//! reference only; do NOT hardcode the value (Decision 4 / DI-019 canonical home).

// Allow `#[cfg(kani)]` without triggering unexpected_cfgs warning.
#![cfg_attr(not(kani), allow(unexpected_cfgs))]

use crate::registry::RegistryEntry;

// ---------------------------------------------------------------------------
// Partition output type
// ---------------------------------------------------------------------------

/// Output of `partition_plugins`: a disjoint, exhaustive split of matched plugins.
///
/// BC-1.14.001 postcondition 1: `sync_group ∩ async_group = ∅` and
/// `sync_group ∪ async_group = matched_plugins`.
#[derive(Debug, Clone)]
pub struct PluginPartition {
    /// Plugins with `async_flag == false` (includes absent field via serde-default).
    /// These are run in parallel within each priority tier; the dispatcher awaits
    /// all completions before computing a verdict. Any block verdict exits 2.
    ///
    /// BC-1.14.001 postcondition 2: parallel-within-tier, sequential-between-tier.
    pub sync_group: Vec<RegistryEntry>,

    /// Plugins with `async_flag == true`.
    /// These are spawned fire-and-forget AFTER sync_group completes.
    /// Their verdicts never influence dispatcher exit code.
    ///
    /// BC-1.14.001 postcondition 4 + postcondition 6.
    pub async_group: Vec<RegistryEntry>,
}

// ---------------------------------------------------------------------------
// Pure partition function (Kani-friendly)
// ---------------------------------------------------------------------------

/// Partition matched registry entries into sync_group and async_group.
///
/// # Contract
///
/// Given `matched` (a slice of registry entries that matched the current event):
/// - Every entry with `async_flag == false` (or absent, via serde-default) → `sync_group`.
/// - Every entry with `async_flag == true` → `async_group`.
/// - No entry appears in both groups (disjoint partition).
/// - Every entry appears in exactly one group (exhaustive / total partition).
/// - The result is deterministic for identical inputs (pure function, no side effects).
///
/// # Kani proof properties (VP-077)
///
/// 1. Totality: `sync_group.len() + async_group.len() == matched.len()`
/// 2. Disjointness: no entry name appears in both groups
/// 3. Async-field respect: `async_flag == true` ↔ entry in async_group
/// 4. Exit-code independence: changes to async_group entries never affect
///    a sync-group verdict computation (verified at dispatch layer)
///
/// # ASYNC_DRAIN_WINDOW_MS
///
/// After sync_group completes, the dispatcher waits up to `ASYNC_DRAIN_WINDOW_MS`
/// (DI-019) for async tasks to emit terminal events. This function does NOT
/// implement the drain — it only performs the pure partition. The drain is
/// implemented in the dispatch loop (T-3c). Cite DI-019 for the value.
///
/// # BC traces
///
/// - BC-1.14.001 postconditions 1, 5, 6 (partition purity + disjointness + spawn ordering)
/// - BC-1.14.001 invariants 1, 3 (partition purity; async group excluded from tier model)
/// - BC-7.06.001 postcondition 2 (async_flag field drives partition)
/// - VP-077 (Kani proof: purity + correctness of this function)
pub fn partition_plugins(matched: &[RegistryEntry]) -> PluginPartition {
    let mut sync_group = Vec::new();
    let mut async_group = Vec::new();
    for entry in matched {
        if entry.async_flag {
            async_group.push(entry.clone());
        } else {
            sync_group.push(entry.clone());
        }
    }
    PluginPartition {
        sync_group,
        async_group,
    }
}

// ---------------------------------------------------------------------------
// Kani proof harnesses — VP-077
// These run under `cargo kani` only (not `cargo test`).
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::registry::{OnError, RegistryEntry};

    // Helper: construct a minimal RegistryEntry for Kani use.
    // Kani can't call String::from / PathBuf::from with kani::any()
    // directly, so we use fixed-string variants and vary async_flag.
    fn make_entry(name: &str, async_flag: bool, on_error_block: bool) -> RegistryEntry {
        RegistryEntry {
            name: name.to_string(),
            event: "PostToolUse".to_string(),
            tool: None,
            plugin: std::path::PathBuf::from(format!("hook-plugins/{}.wasm", name)),
            priority: None,
            enabled: true,
            timeout_ms: None,
            fuel_cap: None,
            on_error: if on_error_block {
                Some(OnError::Block)
            } else {
                None
            },
            capabilities: None,
            config: toml::Value::Table(toml::Table::new()),
            async_flag,
            needs_context: vec![],
            failure_policy: Default::default(),
        }
    }

    /// VP-077 H1: proof_vp077_totality
    ///
    /// Properties: 1 (Totality) — cardinality invariant.
    /// sync_group.len() + async_group.len() == input set size.
    ///
    /// Note: this is cardinality-only. Disjointness (mutual exclusion) is proved
    /// separately in H2 so that a buggy partition that duplicates entries into both
    /// groups cannot satisfy H1 by accident.
    ///
    /// Precondition: (name, event, tool) tuple uniqueness is enforced at registry-load
    /// time by BC-7.06.001 v1.7 Invariant 7 via registry.rs::validate(). Bounded to
    /// n<=4 for Kani tractability.
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_totality() {
        let n: usize = kani::any();
        kani::assume(n <= 4);

        let mut entries = Vec::new();
        for i in 0..n {
            let async_flag: bool = kani::any();
            entries.push(make_entry(&format!("plugin-{}", i), async_flag, false));
        }

        let partition = partition_plugins(&entries);

        // Totality (cardinality): every entry assigned to exactly one group.
        kani::assert(
            partition.sync_group.len() + partition.async_group.len() == entries.len(),
            "VP-077 H1: partition must be total (sync + async == matched)",
        );
    }

    /// VP-077 H2: proof_vp077_disjointness
    ///
    /// Properties: 3 (Disjointness) — no plugin appears in both groups.
    /// Uses name-based contains() check to verify mutual exclusion, not just
    /// cardinality. Separated from H1 so disjointness cannot be vacuously
    /// satisfied by a partition that duplicates every plugin into both groups.
    ///
    /// Precondition: (name, event, tool) tuple uniqueness is enforced at registry-load
    /// time by BC-7.06.001 v1.7 Invariant 7 via registry.rs::validate(). Bounded to
    /// n<=4 for Kani tractability; uniqueness ensured by-construction via
    /// format!("plugin-{}", i) rather than explicit kani::assume guards
    /// (equivalent semantics).
    ///
    /// Bounded to n<=4 for tractability; add #[kani::unwind(10)] if needed.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_vp077_disjointness() {
        let n: usize = kani::any();
        kani::assume(n <= 4);

        let mut entries = Vec::new();
        for i in 0..n {
            let async_flag: bool = kani::any();
            entries.push(make_entry(&format!("plugin-{}", i), async_flag, false));
        }

        let partition = partition_plugins(&entries);

        // Disjointness: no plugin name appears in both groups.
        // Each plugin has a unique name (plugin-0 .. plugin-{n-1}) by construction,
        // so contains() by name correctly checks mutual exclusion.
        for sync_entry in &partition.sync_group {
            let in_async = partition
                .async_group
                .iter()
                .any(|a| a.name == sync_entry.name);
            kani::assert(
                !in_async,
                "VP-077 H2: sync_group and async_group must be disjoint (no name in both)",
            );
        }
        for async_entry in &partition.async_group {
            let in_sync = partition
                .sync_group
                .iter()
                .any(|s| s.name == async_entry.name);
            kani::assert(
                !in_sync,
                "VP-077 H2: async_group and sync_group must be disjoint (no name in both)",
            );
        }
    }

    /// VP-077 H3: proof_vp077_async_field_respected
    ///
    /// Properties: 2 (Async-field respect) — post-parse typed bool values only.
    /// async_flag=true entries land in async_group; async_flag=false in sync_group.
    /// Serde-default (absent field → false) is out of scope; verified by VP-078 H4.
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_async_field_respected() {
        let async_flag: bool = kani::any();
        let entry = make_entry("test-plugin", async_flag, false);
        let partition = partition_plugins(std::slice::from_ref(&entry));

        if async_flag {
            kani::assert(
                partition.async_group.len() == 1 && partition.sync_group.is_empty(),
                "VP-077 H3: async_flag=true must land in async_group",
            );
        } else {
            kani::assert(
                partition.sync_group.len() == 1 && partition.async_group.is_empty(),
                "VP-077 H3: async_flag=false must land in sync_group",
            );
        }
    }

    /// VP-077 H4: proof_vp077_union_completeness
    ///
    /// Properties: 4 (Union completeness) — every input plugin appears in exactly
    /// one group. Distinct from H1 (cardinality) and H2 (mutual exclusion): H4
    /// asserts positive coverage — no plugin is silently dropped.
    ///
    /// Precondition: (name, event, tool) tuple uniqueness is enforced at registry-load
    /// time by BC-7.06.001 v1.7 Invariant 7 via registry.rs::validate(). Bounded to
    /// n<=4 for Kani tractability; uniqueness ensured by-construction via
    /// format!("plugin-{}", i) rather than explicit kani::assume guards
    /// (equivalent semantics).
    ///
    /// With H1 (totality by cardinality) + H2 (disjointness), union completeness
    /// is logically implied; this harness provides an explicit positive witness.
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_union_completeness() {
        let n: usize = kani::any();
        kani::assume(n <= 4);

        let mut entries = Vec::new();
        for i in 0..n {
            let async_flag: bool = kani::any();
            entries.push(make_entry(&format!("plugin-{}", i), async_flag, false));
        }

        let partition = partition_plugins(&entries);

        // Union completeness: every input plugin appears in at least one group.
        // Combined with H2 (disjointness), this guarantees exactly-one membership.
        for entry in &entries {
            let in_sync = partition.sync_group.iter().any(|s| s.name == entry.name);
            let in_async = partition.async_group.iter().any(|a| a.name == entry.name);
            kani::assert(
                in_sync || in_async,
                "VP-077 H4: every matched plugin must appear in exactly one group",
            );
        }
    }

    /// VP-077 legacy: determinism proof (kept from original harness set).
    ///
    /// Two calls with identical input always produce identical partition sizes.
    /// This is an additional property not numbered in VP-077 v1.10 H1-H6 but
    /// retained to prevent regression of the original determinism check.
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_determinism() {
        let async_flag: bool = kani::any();
        let entry = make_entry("determinism-plugin", async_flag, false);
        let entries = vec![entry];

        let p1 = partition_plugins(&entries);
        let p2 = partition_plugins(&entries);

        kani::assert(
            p1.sync_group.len() == p2.sync_group.len()
                && p1.async_group.len() == p2.async_group.len(),
            "VP-077 determinism: partition must be deterministic for identical inputs",
        );
    }

    /// VP-077 empty input: empty input → empty output.
    ///
    /// BC-1.14.001 EC-007: `partition_plugins([], registry)` → `([], [])`.
    /// Retained from the original harness set (prior to H1-H4 renaming).
    #[kani::proof]
    #[kani::unwind(4)]
    fn proof_vp077_empty_input_empty_output() {
        let partition = partition_plugins(&[]);
        kani::assert(
            partition.sync_group.is_empty() && partition.async_group.is_empty(),
            "VP-077 empty: empty input must produce empty sync_group and async_group",
        );
    }
}
