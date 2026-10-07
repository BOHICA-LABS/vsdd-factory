//! `factory_dispatcher` — library surface for the vsdd-factory v1.0
//! dispatcher. The binary at [`main.rs`](src/main.rs) is a thin
//! orchestrator over these modules; integration tests consume the
//! library directly.
//!
//! S-1.2 owns the I/O boundary, registry loading, and routing
//! decisions. Execution (wasmtime instantiation, tokio scheduling,
//! fuel enforcement) is filled in by S-1.4–S-1.6.

// Allow: factory-dispatcher is a pre-existing crate with intentional usage of
// .expect() on mutex locks (poison guard), static-regex compilation, and tokio
// runtime initialization — all infallible in well-behaved programs. The
// workspace-level warn/deny targets new resolver crates (vsdd-context-resolvers).
// Filed for future cleanup as part of broader dispatcher hardening.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// F-P2-002: deny unsafe code in factory-dispatcher — regression guard.
// The crate operates in a security-critical dispatch path; unsafe is
// never warranted here (InternalLog is Clone; closures use by-value capture).
#![deny(unsafe_code)]

pub mod aggregator;
pub mod engine;
pub mod executor;
pub mod host;
pub mod indeterminate_marker;
pub mod internal_log;
pub mod invoke;
pub mod log_dir;
pub use log_dir::{resolve_log_dir_from, resolve_log_dir_from_params};
pub mod partition;
pub mod payload;
pub mod plugin_loader;
pub mod registry;
pub mod resolver;
pub use resolver::CollisionInfo;
pub mod resolver_classify_trap;
pub mod resolver_loader;
pub use resolver_loader::{ResolverLoadError, ResolverLoader};
pub mod routing;
pub mod shard_manager;
pub mod sinks;
pub mod vsdd_sink;
pub use vsdd_sink::flush_sink_file;

pub use aggregator::{PluginResult as AggregatorPluginResult, aggregate_exit_code};
pub use engine::{EPOCH_TICK_MS, EngineError, EpochTicker, build_engine};
pub use executor::{
    DispatchOutcome, ExecutorInputs, IndeterminateCause, PluginOutcome, TierExecutionSummary,
    classify_outcome, execute_tiers, spawn_async_plugin,
};
pub use host::{HostCallError, HostContext, setup_linker};
pub use indeterminate_marker::{
    MarkerFields, delete_marker_if_pass, should_write_marker, write_indeterminate_marker,
};
pub use internal_log::{
    DEFAULT_RETENTION_DAYS, DISPATCHER_SHUTTING_DOWN, DISPATCHER_STARTED,
    INTERNAL_CAPABILITY_DENIED, INTERNAL_DISPATCHER_ERROR, INTERNAL_EVENT_FILTERED,
    INTERNAL_EVENT_SCHEMA_VERSION, INTERNAL_HOST_FUNCTION_PANIC, INTERNAL_SINK_CIRCUIT_CLOSED,
    INTERNAL_SINK_CIRCUIT_OPENED, INTERNAL_SINK_ERROR, INTERNAL_SINK_QUEUE_FULL, InternalEvent,
    InternalLog, PLUGIN_COMPLETED, PLUGIN_CRASHED, PLUGIN_INVOKED, PLUGIN_LOAD_FAILED,
    PLUGIN_LOADED, PLUGIN_TIMEOUT,
};
pub use invoke::{
    DEFAULT_FUEL_CAP, EventType, InvokeError, InvokeLimits, PluginResult, StoreData, TimeoutCause,
    dispatch_postcompact, dispatch_precompact, invoke_plugin,
};
pub use partition::{PluginPartition, partition_plugins};
pub use payload::{HookPayload, PayloadError};
pub use plugin_loader::{PluginCache, PluginLoadError};
pub use registry::{
    Capabilities, ExecSubprocessCaps, FailurePolicy, OnError, ReadFileCaps, Registry,
    RegistryDefaults, RegistryEntry, RegistryError,
};
pub use routing::{PluginResultStub, group_by_priority, match_plugins};
pub use shard_manager::{
    CapFormulaInputs, EditDelta, ShardConfigError, ShardEntry, ShardRegistry, ShardShape, ToolKind,
    compute_shard_cap_bytes, current_shard_bytes_flat, effective_shard_cap_bytes,
    find_matching_entry, item_count_trigger_fires, net_delta_bytes_for_edit,
    net_delta_bytes_for_multi_edit, projected_size_edit, projected_size_write,
    read_changelog_item_count, resolved_low_water_mark, shard_cap_gate_check, size_trigger_fires,
    validate_entry, validate_low_water_mark,
};

/// ABI version the dispatcher speaks. Kept in lock-step with
/// `vsdd_hook_sdk::HOST_ABI_VERSION`; diverging is a breaking change.
pub const HOST_ABI_VERSION: u32 = 1;

/// Test-only wrapper: calls `host::read_file::prepare` for unit testing
/// capability denial without a live WASM instance.
///
/// F-P2-005: AC-011 path-escape WAT fixture is deferred to S-12.07.
/// This function allows integration tests in `tests/` to exercise
/// the host-function capability gate without a full WASM invocation.
///
/// `#[doc(hidden)]` — not part of the public API; only for test use.
#[doc(hidden)]
pub fn read_file_prepare_for_test(
    ctx: &HostContext,
    path: &str,
    max_bytes: u32,
) -> Result<(Vec<u8>, u32), i32> {
    host::read_file::prepare(ctx, path, max_bytes)
}

/// Drain window for async-group tasks (S-15.01 T-3d — BC-1.14.001 postcondition 4).
///
/// After `sync_group` completes, the dispatcher waits up to `ASYNC_DRAIN_WINDOW_MS`
/// for spawned async tasks to emit terminal events to FileSink. Tasks that do not
/// complete within this window are forcibly terminated; their pending I/O is discarded.
///
/// **DI-019 canonical source.** This constant is the authoritative Rust definition.
/// All other references (BCs, VPs, ADRs, test fixtures) cite DI-019 by name, NOT
/// by hardcoding the value `100`. Tests MUST use `factory_dispatcher::ASYNC_DRAIN_WINDOW_MS`
/// rather than `Duration::from_millis(100)` directly.
///
/// # BC traces
///
/// - DI-019 — `ASYNC_DRAIN_WINDOW_MS = 100ms` (domain invariant, canonical source)
/// - BC-1.14.001 postcondition 4 — async-group drain window contract
/// - VP-079 Scenario 5 — drain-window truncation verification
pub const ASYNC_DRAIN_WINDOW_MS: std::time::Duration = std::time::Duration::from_millis(
    // DI-019: ASYNC_DRAIN_WINDOW_MS = 100ms (canonical).
    // Do NOT change this value without a DI-019 amendment.
    // Do NOT add other inline `100` references — cite this constant instead.
    100,
);

/// Generate a fresh v4 UUID to use as the per-invocation
/// `dispatcher_trace_id`. Extracted so tests can substitute a fixed
/// value.
pub fn new_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
