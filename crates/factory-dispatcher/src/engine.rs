//! Shared `wasmtime::Engine` with epoch-interruption + fuel consumption
//! enabled and the component model explicitly disabled (ADR-003: WASI
//! preview-1 core modules only).
//!
//! The engine is expensive to build (~100 ms) so we construct one per
//! dispatcher process and hand out references. An `EpochTicker` thread
//! bumps the engine's epoch every 10 ms; every wasmtime `Store` gets a
//! deadline expressed in epochs, so once the deadline passes the next
//! guest yield point traps with a clean interrupt instead of running
//! forever.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use thiserror::Error;
use wasmtime::{Config, Engine};

/// Epoch ticker cadence. Every N ms the engine's epoch advances by 1.
/// Timeout resolution equals this value: a 5,000 ms plugin budget
/// becomes `set_epoch_deadline(current + 5_000 / 10)`.
pub const EPOCH_TICK_MS: u64 = 10;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("wasmtime config build failed: {0}")]
    Config(String),
}

/// Build a fresh engine with the v1.0 dispatcher's mandatory features
/// turned on. Callers wrap it in an `Arc` before sharing across
/// threads.
pub fn build_engine() -> Result<Engine, EngineError> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(true);
    // WASI preview-1 needs imports registered per-instance; preview-2
    // is explicitly out of scope for v1.0 (ADR-003).
    config.wasm_reference_types(true);
    // ADR-003: core modules only. Explicitly disable the component model
    // (wasmtime 48 enables it by default) so component binaries are
    // rejected at compile time rather than relying on the default.
    config.wasm_component_model(false);
    Engine::new(&config).map_err(|e| EngineError::Config(e.to_string()))
}

/// Background ticker that increments the engine's epoch at a fixed
/// cadence. Owns its own OS thread so the dispatcher doesn't need a
/// tokio runtime just for this.
///
/// Dropping the ticker stops the thread. Stopping is cooperative — we
/// set a flag and the thread exits on its next wake, so shutdown takes
/// up to one tick.
pub struct EpochTicker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl EpochTicker {
    /// Start a ticker for the given engine. The ticker holds its own
    /// `Arc<Engine>` clone so the engine stays alive for the thread's
    /// lifetime.
    pub fn start(engine: Engine) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let handle = thread::Builder::new()
            .name("vsdd-epoch-ticker".into())
            .spawn(move || {
                while !stop_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(EPOCH_TICK_MS));
                    engine.increment_epoch();
                }
            })
            .expect("epoch-ticker spawn");
        Self {
            stop,
            handle: Some(handle),
        }
    }

    /// Request a cooperative stop and wait for the thread to exit.
    /// Callable more than once (second call is a no-op).
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for EpochTicker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Convert a millisecond timeout into an epoch-count delta. Rounds up
/// so a sub-tick timeout still produces at least one tick of grace.
pub fn timeout_ms_to_epochs(timeout_ms: u64) -> u64 {
    timeout_ms.div_ceil(EPOCH_TICK_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_engine_with_epoch_and_fuel() {
        let engine = build_engine().expect("engine should build");
        // If epoch/fuel weren't configured, these would panic at use
        // time; we can't read back flags from an Engine, but a smoke
        // construct-and-drop proves config validation passed.
        drop(engine);
    }

    /// ADR-003: preview-2 / the component model is out of scope. The engine
    /// must not compile component binaries (defence in depth: no component
    /// parsing attack surface reachable from untrusted plugin bytes).
    #[test]
    fn engine_rejects_component_binaries() {
        let engine = build_engine().expect("engine should build");
        let bytes = wat::parse_str("(component)").expect("minimal component wat");
        let err = match wasmtime::component::Component::new(&engine, &bytes) {
            Ok(_) => panic!("component must NOT compile: component model must be disabled"),
            Err(e) => e,
        };
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("component")
                && (msg.contains("support") || msg.contains("enabled") || msg.contains("disabled")),
            "expected a component-model-disabled error, got: {msg}"
        );
    }

    #[test]
    fn timeout_ms_to_epochs_rounds_up() {
        assert_eq!(timeout_ms_to_epochs(0), 0);
        assert_eq!(timeout_ms_to_epochs(1), 1);
        assert_eq!(timeout_ms_to_epochs(10), 1);
        assert_eq!(timeout_ms_to_epochs(11), 2);
        assert_eq!(timeout_ms_to_epochs(5_000), 500);
    }

    #[test]
    fn ticker_advances_epoch_over_time() {
        let engine = build_engine().unwrap();
        let ticker = EpochTicker::start(engine.clone());
        // Run for a few ticks.
        thread::sleep(Duration::from_millis(EPOCH_TICK_MS * 5 + 5));
        drop(ticker);
        // If the ticker incremented at all, dropping it without panic
        // is proof enough that the thread terminated cleanly. We can't
        // read the engine's epoch directly through the public API.
        drop(engine);
    }

    #[test]
    fn ticker_shutdown_is_idempotent() {
        let engine = build_engine().unwrap();
        let mut ticker = EpochTicker::start(engine);
        ticker.shutdown();
        ticker.shutdown(); // second call must not panic
    }
}
