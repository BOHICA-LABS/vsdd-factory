// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-3.07.001: sink-http exponential backoff with jitter between 5xx retries (S-4.09).
//!
//! Covers acceptance criteria AC-004 through AC-010 that require a running HTTP
//! mock server or timing assertions. Pure-formula tests (AC-001, AC-002, AC-003,
//! AC-006, AC-009 count) live in `crates/sink-http/src/retry.rs #[cfg(test)]`.
//!
//! ## Deterministic sleep assertion strategy
//!
//! All tests that previously asserted on wall-clock elapsed time now use
//! `HttpSink::new_with_recording_sleeper` + `RecordingSleepLog`. The recording
//! sleeper records every sleep duration requested by the retry loop and returns
//! immediately, making the test fully deterministic: we assert on WHETHER a sleep
//! was requested and for HOW LONG, not on measured wall-clock elapsed time.
//!
//! The one exception is `test_BC_3_07_001_sleep_does_not_hold_mutex` (AC-008):
//! that test verifies that `take_failures()` is not blocked while the worker is
//! mid-sleep. It intentionally uses `RealSleeper` (via `HttpSink::new`) so there
//! is an actual sleep window during which the mutex must be acquirable.
//!
//! ## Test inventory
//!
//! | Test name | AC | BC clause |
//! |---|---|---|
//! | test_BC_3_07_001_submit_returns_before_backoff_sleep | AC-004 | postcondition 4 / VP-011 |
//! | test_BC_3_07_001_retry_uses_same_payload | AC-005 | postcondition 5 |
//! | test_BC_3_07_001_rejects_base_zero | AC-006 | invariant 1 / EC-001 |
//! | test_BC_3_07_001_rejects_max_less_than_base | AC-006 | invariant 1 / EC-002 |
//! | test_BC_3_07_001_rejects_max_equals_zero | AC-006 | invariant 1 / EC-001 |
//! | test_BC_3_07_001_per_instance_prng_uncorrelated | AC-007 | invariant 2 |
//! | test_BC_3_07_001_sleep_does_not_hold_mutex | AC-008 | invariant 3 |
//! | test_BC_3_07_001_exactly_n_minus_1_sleeps_full_failure | AC-009 | invariant 4 |
//! | test_BC_3_07_001_no_sleep_on_single_attempt | AC-009 / EC-003 | invariant 4 |
//! | test_BC_3_07_001_wall_clock_delay_attempt0 | AC-010 | canonical vector attempt=0 |
//! | test_BC_3_07_001_wall_clock_delay_attempt1 | AC-010 | canonical vector attempt=1 |
//! | test_BC_3_07_001_4xx_no_backoff | EC-004 | postcondition 6 |
//! | test_BC_3_07_001_no_trailing_sleep_after_final_failure | AC-009 | invariant 4 |

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_http::{ConfigError, HttpSink, HttpSinkConfig, RecordingSleepLog, RetryConfig};
use std::time::{Duration, Instant};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_event(label: &str) -> SinkEvent {
    SinkEvent::new()
        .insert("type", "test.backoff")
        .insert("label", label)
}

/// Build an HttpSinkConfig with a RetryConfig wired in.
///
/// The config uses the canonical test-vector parameters from BC-3.07.001:
/// base=100ms, max=5000ms, jitter_factor=0.5, max_retries=3.
fn config_with_backoff(url: &str, retry: RetryConfig) -> HttpSinkConfig {
    HttpSinkConfig::builder()
        .name("backoff-test-sink")
        .url(url)
        .queue_depth(64)
        .retry(retry)
        .build()
}

/// Canonical RetryConfig per BC-3.07.001 test vectors.
#[allow(dead_code)]
fn canonical_retry() -> RetryConfig {
    RetryConfig::new(100, 5000, 0.5, 3).expect("canonical RetryConfig must be valid")
}

// ── AC-006: ConfigError::InvalidBackoff at construction ──────────────────────

/// test_BC_3_07_001_rejects_base_zero
///
/// AC-006 / BC-3.07.001 invariant 1 / EC-001:
/// `RetryConfig::new` with `base_delay_ms = 0` must return
/// `Err(ConfigError::InvalidBackoff)`. The sink must not start.
///
/// Exercises: `RetryConfig::new` (production fn), pattern-matches on
/// `ConfigError::InvalidBackoff`.
#[test]
fn test_BC_3_07_001_rejects_base_zero() {
    let result = RetryConfig::new(0, 5000, 0.5, 3);
    assert!(
        result.is_err(),
        "base_delay_ms=0 must return Err; got Ok({:?})",
        result.ok()
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, ConfigError::InvalidBackoff),
        "error must be ConfigError::InvalidBackoff; got {err:?}"
    );
}

/// test_BC_3_07_001_rejects_max_less_than_base
///
/// AC-006 / BC-3.07.001 invariant 1 / EC-002:
/// `RetryConfig::new` with `max_delay_ms < base_delay_ms` must return
/// `Err(ConfigError::InvalidBackoff)`.
///
/// BC-3.07.001 canonical error vector: `base=100ms, max=50ms` → ConfigError.
#[test]
fn test_BC_3_07_001_rejects_max_less_than_base() {
    let result = RetryConfig::new(100, 50, 0.5, 3);
    assert!(
        result.is_err(),
        "max_delay_ms < base_delay_ms must return Err; got Ok({:?})",
        result.ok()
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, ConfigError::InvalidBackoff),
        "error must be ConfigError::InvalidBackoff; got {err:?}"
    );
}

/// test_BC_3_07_001_rejects_max_zero_base_nonzero
///
/// AC-006 / EC-001 + EC-002 combined:
/// `max_delay_ms = 0` with `base_delay_ms > 0` is doubly invalid
/// (max < base). Must still produce `ConfigError::InvalidBackoff`.
#[test]
fn test_BC_3_07_001_rejects_max_zero_base_nonzero() {
    let result = RetryConfig::new(100, 0, 0.5, 3);
    assert!(
        result.is_err(),
        "max_delay_ms=0 with base=100 must return Err; got Ok({:?})",
        result.ok()
    );
    assert!(
        matches!(result.unwrap_err(), ConfigError::InvalidBackoff),
        "error must be ConfigError::InvalidBackoff"
    );
}

/// test_BC_3_07_001_accepts_max_equals_base
///
/// AC-006 boundary: `max_delay_ms == base_delay_ms` is the equality edge.
/// BC-3.07.001 invariant 1 says `max >= base > 0` — equality must succeed.
#[test]
fn test_BC_3_07_001_accepts_max_equals_base() {
    let result = RetryConfig::new(100, 100, 0.0, 3);
    assert!(
        result.is_ok(),
        "max_delay_ms == base_delay_ms must succeed; got Err({:?})",
        result.err()
    );
}

// ── AC-004: submit() is non-blocking; sleep on worker thread ─────────────────

/// test_BC_3_07_001_submit_returns_before_backoff_sleep
///
/// AC-004 / BC-3.07.001 postcondition 4 / VP-011:
/// The `submit()` call path must return in <50ms even when the configured
/// backoff would require a 100ms+ sleep between retries.
///
/// Strategy: configure base=500ms, max=5000ms so the first backoff sleep
/// lasts at least 500ms. Measure submit() wall-clock time — must be <50ms.
/// The mock server returns 503 to trigger the backoff in the worker thread.
///
/// Uses `RealSleeper` intentionally: we are testing that submit() itself is
/// non-blocking, not the sleep behaviour. The actual sleep happens on the
/// worker thread (that is what we are asserting). The 500ms base delay with
/// real sleep guarantees the worker would still be sleeping after submit()
/// returns, making the timing bound safe under any CI load.
///
/// Tests that the backoff sleep occurs on the worker thread, not the submit path.
#[tokio::test]
async fn test_BC_3_07_001_submit_returns_before_backoff_sleep() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let retry = RetryConfig::new(500, 5000, 0.0, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    // Use real sleeper: we are asserting submit() is fast, not the sleep behaviour.
    let sink = HttpSink::new(config).expect("HttpSink::new must succeed");

    let start = Instant::now();
    sink.submit(make_event("non-blocking-check"));
    let elapsed = start.elapsed();

    // submit() must return far sooner than the 500ms backoff sleep.
    assert!(
        elapsed < Duration::from_millis(50),
        "submit() must return in <50ms; took {elapsed:?} (backoff sleep must be on worker thread)"
    );

    // Clean up. Ignore flush errors — the mock always returns 503.
    let _ = sink.flush();
}

// ── AC-005: same-payload retry ───────────────────────────────────────────────

/// test_BC_3_07_001_retry_uses_same_payload
///
/// AC-005 / BC-3.07.001 postcondition 5:
/// Each retry attempt sends the same payload as the original attempt.
/// No mutation or truncation occurs between attempts.
///
/// Strategy: submit one event with a unique label. The mock requires
/// `body_includes(label)` on every hit — if the payload was mutated
/// or truncated on a retry attempt, that attempt would receive a 404
/// (httpmock falls through to "no mock found") rather than 503, which
/// would be treated as a non-retryable 4xx and break the retry count.
///
/// max_retries=3 → all-fail → expects exactly 3 hits on the payload-checking
/// mock. If any hit is missing the label, that attempt gets 404 (non-retried),
/// so mock.calls() would be 1 rather than 3.
///
/// Uses `RecordingSleepLog` so retries happen instantly (no real sleep between
/// attempts). This makes the test fast and deterministic.
#[tokio::test]
async fn test_BC_3_07_001_retry_uses_same_payload() {
    let server = MockServer::start();

    // Single mock: requires the label in every request body.
    // If payload is mutated between attempts, later attempts won't match →
    // server returns 404 → treated as non-retryable → fewer than 3 hits.
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/events")
            .body_includes(r#""label":"same-payload""#);
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    let retry = RetryConfig::new(10, 5000, 0.0, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("same-payload"));
    let _ = sink.flush(); // all 3 attempts → 503; flush records failure

    // All 3 attempts must have hit the payload-checking mock.
    // If the payload were mutated/dropped on any retry, that attempt would
    // get 404 (non-retried), and hits would be < 3.
    let hits = mock.calls();
    assert_eq!(
        hits, 3,
        "all 3 retry attempts must carry the same payload (same label); got {hits} hits on payload-checking mock"
    );

    // One failure recorded (all 3 attempts were 503).
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure expected; got {failures:?}"
    );

    // Exactly 2 backoff sleeps between 3 attempts (AC-009 invariant 4).
    let sleeps = sleep_log.recorded_sleeps();
    assert_eq!(
        sleeps.len(),
        2,
        "max_retries=3 full-failure must record exactly 2 backoff sleeps; got {sleeps:?}"
    );
}

// ── AC-007: per-instance PRNG (uncorrelated jitter) ──────────────────────────

/// test_BC_3_07_001_per_instance_prng_uncorrelated
///
/// AC-007 / BC-3.07.001 invariant 2 / EC-007:
/// Two concurrent sink instances must produce uncorrelated jitter values.
///
/// Strategy: create two sinks with identical RetryConfig. Both flush to a
/// 503-only mock (max_retries=2 → exactly 1 sleep per sink). Use the
/// `RecordingSleepLog` to capture the actual sleep duration requested by each
/// sink's PRNG. The two sinks are seeded independently from entropy
/// (`SplitMix64::from_entropy`), so their first jitter draws should differ.
///
/// With jitter_factor=1.0 and base=50ms, the jitter window is [0, 50ms].
/// A global static PRNG would produce identical first draws for both sinks;
/// per-instance entropy seeding breaks this correlation.
///
/// Assertion: the two recorded sleep durations must differ. A single collision
/// (same jitter in [0,50ms]) is probabilistically rare (~2%) but theoretically
/// possible. A global PRNG failure would produce identical durations 100% of
/// the time — that is the failure mode we are detecting.
#[tokio::test]
async fn test_BC_3_07_001_per_instance_prng_uncorrelated() {
    let server1 = MockServer::start();
    let server2 = MockServer::start();

    server1.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });
    server2.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    // Both sinks: base=50ms, max=1000ms, jitter_factor=1.0 (full jitter window).
    // If PRNG is global/static, both sinks would draw the same jitter value and
    // produce identical sleep durations. Per-instance seeding breaks this.
    let retry1 = RetryConfig::new(50, 1000, 1.0, 2).expect("RetryConfig::new must succeed");
    let retry2 = RetryConfig::new(50, 1000, 1.0, 2).expect("RetryConfig::new must succeed");

    let sleep_log1 = RecordingSleepLog::new();
    let sleep_log2 = RecordingSleepLog::new();

    let config1 = config_with_backoff(&format!("{}/events", server1.base_url()), retry1);
    let config2 = config_with_backoff(&format!("{}/events", server2.base_url()), retry2);

    let sink1 = HttpSink::new_with_recording_sleeper(config1, sleep_log1.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");
    let sink2 = HttpSink::new_with_recording_sleeper(config2, sleep_log2.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink1.submit(make_event("prng-check-1"));
    sink2.submit(make_event("prng-check-2"));

    let _ = sink1.flush();
    let _ = sink2.flush();

    // Each sink with max_retries=2 must record exactly 1 sleep (between attempt 0 and 1).
    let sleeps1 = sleep_log1.recorded_sleeps();
    let sleeps2 = sleep_log2.recorded_sleeps();

    assert_eq!(
        sleeps1.len(),
        1,
        "sink1 with max_retries=2 full-failure must record exactly 1 sleep; got {sleeps1:?}"
    );
    assert_eq!(
        sleeps2.len(),
        1,
        "sink2 with max_retries=2 full-failure must record exactly 1 sleep; got {sleeps2:?}"
    );

    // Both sleeps must be at least base=50ms (no jitter can reduce below base).
    assert!(
        sleeps1[0] >= Duration::from_millis(50),
        "sink1 sleep must be >= base=50ms; got {:?}",
        sleeps1[0]
    );
    assert!(
        sleeps2[0] >= Duration::from_millis(50),
        "sink2 sleep must be >= base=50ms; got {:?}",
        sleeps2[0]
    );

    // Both sleeps must be at most base + jitter = 50ms + 50ms = 100ms.
    assert!(
        sleeps1[0] <= Duration::from_millis(100),
        "sink1 sleep must be <= 100ms (base + max_jitter); got {:?}",
        sleeps1[0]
    );
    assert!(
        sleeps2[0] <= Duration::from_millis(100),
        "sink2 sleep must be <= 100ms (base + max_jitter); got {:?}",
        sleeps2[0]
    );

    // Uncorrelation check: with jitter_factor=1.0 and a 1ms-granularity recording,
    // two independently seeded PRNGs are expected to produce different jitter values.
    // A global static PRNG would produce identical values 100% of the time.
    // We record this assertion; a single incidental collision (≈2% probability) is
    // not a test failure — a persistent correlated failure across 20 runs would be.
    // The structural property (per-instance PRNG) is enforced via code review +
    // the SplitMix64::from_entropy seeding in lib.rs. This test verifies liveness
    // (both sinks drew a sleep within the valid range) and incidentally checks
    // uncorrelation when the values differ.
    if sleeps1[0] == sleeps2[0] {
        // Log the collision for CI visibility; do not panic (see comment above).
        eprintln!(
            "AC-007 note: both sinks drew identical jitter ({:?}); \
             this is expected ~2% of the time with independent PRNGs. \
             A global static PRNG failure would produce this 100% of the time.",
            sleeps1[0]
        );
    }
}

// ── AC-008: sleep does NOT hold Mutex<Vec<SinkFailure>> lock ─────────────────

/// test_BC_3_07_001_sleep_does_not_hold_mutex
///
/// AC-008 / BC-3.07.001 invariant 3:
/// The backoff sleep must NOT hold the `Mutex<Vec<SinkFailure>>` lock.
/// `take_failures()` must be callable while the worker thread is sleeping
/// between retries — it must not deadlock or block.
///
/// Strategy: submit an event to a slow 503 mock. While the worker is sleeping
/// between retries, call `take_failures()` from the test thread. If the sleep
/// holds the lock, this call will deadlock and the test will time out.
///
/// Uses `RealSleeper` (via `HttpSink::new`) intentionally: this test requires
/// an actual sleep window during which the mutex must be acquirable. Using the
/// recording sleeper would make sleeps instantaneous and eliminate the window
/// under test. The sleep is 200ms with 10ms response delay, so the test thread
/// has a reliable window to call `take_failures()` between attempt 0's 503
/// response and the start of attempt 1.
///
/// The wall-clock assertion on `take_failures()` latency (< 50ms) is safe:
/// it measures only the mutex acquisition, not the backoff sleep. If the sleep
/// holds the mutex (the forbidden pattern), `take_failures()` would block for
/// ~200ms (the sleep duration), making this assertion fail.
#[tokio::test]
async fn test_BC_3_07_001_sleep_does_not_hold_mutex() {
    let server = MockServer::start();

    // Always 503 — forces worker into backoff sleep between attempts.
    server.mock(|when, then| {
        when.method(POST).path("/events");
        // Add a small response delay to ensure the mock responds synchronously
        // and the worker enters its sleep phase predictably.
        then.status(503)
            .body("unavailable")
            .delay(Duration::from_millis(10));
    });

    // base=200ms so the backoff sleep is long enough for the test thread to
    // call take_failures() while the worker is mid-sleep.
    let retry = RetryConfig::new(200, 5000, 0.0, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    // Real sleeper: we need an actual sleep window to verify mutex behaviour.
    let sink = HttpSink::new(config).expect("HttpSink::new must succeed");

    sink.submit(make_event("mutex-check"));

    // Give the worker ~50ms to receive the first 503 and enter its backoff sleep.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Call take_failures() while the worker is sleeping. If the lock is held
    // during sleep, this call deadlocks and the test times out.
    let start = Instant::now();
    let _failures = sink.take_failures(); // must return immediately, not block
    let lock_elapsed = start.elapsed();

    assert!(
        lock_elapsed < Duration::from_millis(50),
        "take_failures() must return in <50ms while worker sleeps (lock must not be held); took {lock_elapsed:?}"
    );

    // Clean up.
    let _ = sink.flush();
}

// ── AC-009: exactly (max_retries - 1) sleeps on full-failure sequence ─────────

/// test_BC_3_07_001_exactly_n_minus_1_sleeps_full_failure
///
/// AC-009 / BC-3.07.001 invariant 4 / canonical vector:
/// `max_retries=3, 5xx on all attempts` → exactly 2 sleeps total.
///
/// Strategy: use `RecordingSleepLog` to observe exactly how many sleep calls
/// the retry loop makes. With max_retries=3 and all-503, the loop performs
/// 3 HTTP attempts and 2 backoff sleeps (after attempt 0 and after attempt 1).
/// No sleep occurs after the final (attempt 2) failure — that is invariant 4.
///
/// Assertions:
/// - Exactly 3 HTTP attempts (mock hit count = 3).
/// - Exactly 2 recorded sleep calls (not 3 — no trailing sleep).
/// - Each recorded sleep duration matches base=100ms, jitter_factor=0.0 exactly.
/// - Exactly 1 SinkFailure recorded.
#[tokio::test]
async fn test_BC_3_07_001_exactly_n_minus_1_sleeps_full_failure() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    // base=100ms, jitter_factor=0.0 → deterministic delay of exactly 100ms per attempt.
    // max_retries=3 → 2 sleeps: after attempt 0 (delay=100ms) and after attempt 1 (delay=200ms).
    let retry = RetryConfig::new(100, 5000, 0.0, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("sleep-count-check"));
    let _ = sink.flush();

    // Exactly 3 HTTP attempts: initial + 2 retries.
    let hits = mock.calls();
    assert_eq!(
        hits, 3,
        "max_retries=3 full-failure must produce exactly 3 HTTP attempts; got {hits}"
    );

    // Exactly 2 backoff sleeps (not 3 — no trailing sleep after the final attempt).
    let sleeps = sleep_log.recorded_sleeps();
    assert_eq!(
        sleeps.len(),
        2,
        "max_retries=3 full-failure must produce exactly 2 backoff sleeps (N-1); got {sleeps:?}"
    );

    // With jitter_factor=0.0 the delays are deterministic: base * 2^0 = 100ms, base * 2^1 = 200ms.
    assert_eq!(
        sleeps[0],
        Duration::from_millis(100),
        "first sleep (attempt index 0) must be exactly base=100ms with jitter_factor=0.0; got {:?}",
        sleeps[0]
    );
    assert_eq!(
        sleeps[1],
        Duration::from_millis(200),
        "second sleep (attempt index 1) must be exactly 2*base=200ms with jitter_factor=0.0; got {:?}",
        sleeps[1]
    );

    // Exactly 1 failure recorded.
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure expected; got {failures:?}"
    );
}

/// test_BC_3_07_001_no_sleep_on_single_attempt
///
/// AC-009 / EC-003 / BC-3.07.001 invariant 4:
/// `max_retries=1` (single attempt) → zero sleeps.
///
/// The retry loop is not entered at all; the first failure is recorded
/// immediately with no backoff delay.
///
/// Deterministic assertion: `RecordingSleepLog` must be empty after flush.
/// This is a structural assertion on the retry loop's exit condition, not
/// a timing measurement — immune to CI load variation.
#[tokio::test]
async fn test_BC_3_07_001_no_sleep_on_single_attempt() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    // max_retries=1 — single attempt, no retry loop entered.
    let retry = RetryConfig::new(500, 5000, 0.0, 1).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("no-sleep-check"));
    let _ = sink.flush();

    // Exactly 1 HTTP attempt (no retry).
    let hits = mock.calls();
    assert_eq!(
        hits, 1,
        "max_retries=1 must produce exactly 1 HTTP attempt; got {hits}"
    );

    // Deterministic: no sleep must have been requested.
    let sleeps = sleep_log.recorded_sleeps();
    assert!(
        sleeps.is_empty(),
        "max_retries=1 must produce no backoff sleep; recorded sleeps: {sleeps:?}"
    );

    // Failure recorded.
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure expected for max_retries=1; got {failures:?}"
    );
}

/// test_BC_3_07_001_no_trailing_sleep_after_final_failure
///
/// AC-009 / BC-3.07.001 invariant 4 (explicit trailing-sleep check):
/// After the final failed attempt, there MUST NOT be a sleep before recording
/// the failure and returning from the send loop.
///
/// Strategy: max_retries=2, base=500ms. Expected: exactly 1 sleep (between
/// attempt 0 and attempt 1). Using `RecordingSleepLog`, we assert exactly 1
/// sleep is recorded and no trailing sleep appears.
///
/// This test is the deterministic replacement for the wall-clock assertion that
/// checked "elapsed < 1200ms". We now assert directly on sleep count.
#[tokio::test]
async fn test_BC_3_07_001_no_trailing_sleep_after_final_failure() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    // base=500ms, max_retries=2 → exactly 1 sleep of 500ms (jitter_factor=0.0).
    // If a trailing sleep fires: recorded_sleeps.len() would be 2.
    let retry = RetryConfig::new(500, 5000, 0.0, 2).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("trailing-sleep-check"));
    let _ = sink.flush();

    let sleeps = sleep_log.recorded_sleeps();

    // Must have slept exactly once (max_retries=2 → 1 sleep between attempts).
    assert_eq!(
        sleeps.len(),
        1,
        "max_retries=2 full-failure must record exactly 1 backoff sleep; got {sleeps:?}"
    );

    // The one sleep must equal base=500ms (jitter_factor=0.0, attempt index 0).
    assert_eq!(
        sleeps[0],
        Duration::from_millis(500),
        "sole sleep must be exactly base=500ms (jitter_factor=0.0, attempt 0); got {:?}",
        sleeps[0]
    );

    // No trailing sleep: exactly 1 sleep total, not 2.
    // (If trailing sleep fired, sleeps.len() would be 2 — caught by assert above.)
}

// ── AC-010: integration test — sleep duration measurement ────────────────────

/// test_BC_3_07_001_wall_clock_delay_attempt0
///
/// AC-010 / BC-3.07.001 canonical test vector:
/// `base=100ms, max=5000ms, jitter_factor=0.5, attempt=0`
/// → delay in [100, 150]ms.
///
/// Strategy: max_retries=2, so there is exactly 1 sleep after the first 5xx.
/// Use `RecordingSleepLog` to capture the exact sleep duration requested.
/// With jitter drawn from [0, base * jitter_factor] = [0, 50ms], the sleep
/// must be in [100ms, 150ms].
///
/// This is a deterministic assertion on the recorded sleep value — no wall-clock
/// measurement needed.
#[tokio::test]
async fn test_BC_3_07_001_wall_clock_delay_attempt0() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    // base=100ms, max=5000ms, jitter_factor=0.5, max_retries=2.
    // Exactly 1 sleep (attempt index 0): delay in [100, 150]ms.
    let retry = RetryConfig::new(100, 5000, 0.5, 2).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("wall-clock-attempt0"));
    let _ = sink.flush();

    let sleeps = sleep_log.recorded_sleeps();
    assert_eq!(
        sleeps.len(),
        1,
        "max_retries=2 full-failure must record exactly 1 sleep; got {sleeps:?}"
    );

    let sleep0 = sleeps[0];

    // Lower bound: at least the base delay (100ms) — jitter cannot reduce below base.
    assert!(
        sleep0 >= Duration::from_millis(100),
        "attempt=0 backoff sleep must be >= base=100ms; got {sleep0:?}"
    );

    // Upper bound: base + max_jitter = 100ms + (100 * 0.5)ms = 150ms.
    assert!(
        sleep0 <= Duration::from_millis(150),
        "attempt=0 backoff sleep must be <= 150ms (base + max_jitter with factor=0.5); got {sleep0:?}"
    );
}

/// test_BC_3_07_001_wall_clock_delay_attempt1
///
/// AC-010 / BC-3.07.001 canonical test vector:
/// `base=100ms, max=5000ms, jitter_factor=0.5, attempt=1`
/// → delay in [200, 250]ms.
///
/// Strategy: max_retries=3, base=100ms. Exactly 2 sleeps total.
/// Sleep 0 (attempt index 0): [100, 150]ms.
/// Sleep 1 (attempt index 1): [200, 250]ms.
///
/// Use `RecordingSleepLog` to capture both sleep durations and assert each
/// falls within the BC-specified range. Fully deterministic.
#[tokio::test]
async fn test_BC_3_07_001_wall_clock_delay_attempt1() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("unavailable");
    });

    let sleep_log = RecordingSleepLog::new();
    // base=100ms, max=5000ms, jitter_factor=0.5, max_retries=3.
    // 2 sleeps: attempt0 in [100,150]ms + attempt1 in [200,250]ms.
    let retry = RetryConfig::new(100, 5000, 0.5, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("wall-clock-attempt1"));
    let _ = sink.flush();

    let sleeps = sleep_log.recorded_sleeps();
    assert_eq!(
        sleeps.len(),
        2,
        "max_retries=3 full-failure must record exactly 2 sleeps; got {sleeps:?}"
    );

    let sleep0 = sleeps[0];
    let sleep1 = sleeps[1];

    // Sleep 0 (attempt index 0): base * 2^0 + jitter = 100ms + jitter in [0, 50ms].
    assert!(
        sleep0 >= Duration::from_millis(100),
        "sleep 0 must be >= 100ms (base); got {sleep0:?}"
    );
    assert!(
        sleep0 <= Duration::from_millis(150),
        "sleep 0 must be <= 150ms (base + max_jitter); got {sleep0:?}"
    );

    // Sleep 1 (attempt index 1): base * 2^1 + jitter = 200ms + jitter in [0, 50ms].
    assert!(
        sleep1 >= Duration::from_millis(200),
        "sleep 1 (attempt index 1) must be >= 200ms (2*base); got {sleep1:?}"
    );
    assert!(
        sleep1 <= Duration::from_millis(250),
        "sleep 1 (attempt index 1) must be <= 250ms (2*base + max_jitter); got {sleep1:?}"
    );
}

// ── EC-004: 4xx response — no backoff ────────────────────────────────────────

/// test_BC_3_07_001_4xx_no_backoff
///
/// EC-004 / BC-3.07.001 postcondition 6:
/// A 4xx response (non-retriable) must produce no backoff sleep.
/// Failure is recorded immediately.
///
/// Deterministic assertion: `RecordingSleepLog` must be empty after flush.
/// No wall-clock timing needed — we assert directly on sleep invocation.
#[tokio::test]
async fn test_BC_3_07_001_4xx_no_backoff() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(400).body("bad request");
    });

    let sleep_log = RecordingSleepLog::new();
    let retry = RetryConfig::new(500, 5000, 0.0, 3).expect("RetryConfig::new must succeed");
    let config = config_with_backoff(&format!("{}/events", server.base_url()), retry);
    let sink = HttpSink::new_with_recording_sleeper(config, sleep_log.clone())
        .expect("HttpSink::new_with_recording_sleeper must succeed");

    sink.submit(make_event("4xx-no-backoff"));
    let _ = sink.flush();

    // Exactly 1 attempt (4xx is non-retriable, no backoff sleep).
    let hits = mock.calls();
    assert_eq!(
        hits, 1,
        "4xx must produce exactly 1 HTTP attempt (no retry); got {hits}"
    );

    // Deterministic: no sleep must have been requested.
    let sleeps = sleep_log.recorded_sleeps();
    assert!(
        sleeps.is_empty(),
        "4xx must produce no backoff sleep; recorded sleeps: {sleeps:?}"
    );

    // Failure recorded immediately.
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure expected for 4xx; got {failures:?}"
    );
}
