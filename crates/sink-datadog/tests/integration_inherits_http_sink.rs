// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-2, VP-011, VP-012: DatadogSink inherits HttpSink retry + non-blocking semantics.
//!
//! BC-3.01.001 postcondition 1: Sink integrates with registry machinery.
//! Verifies that the Datadog sink retries on 5xx (inheriting HttpSink behavior)
//! and that flush() delivers synchronously.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_datadog::{DatadogSink, DatadogSinkConfig};

fn config_for_mock_url(url: &str, api_key: &str) -> DatadogSinkConfig {
    let toml = format!(
        r#"
schema_version = 1
type = "datadog"
name = "retry-test-sink"
api_key = "{api_key}"
endpoint = "{url}"
"#
    );
    DatadogSinkConfig::from_toml(&toml)
        .expect("from_toml must succeed")
        .expect("must return Some")
}

/// VP-011 — DatadogSink wraps HttpSink: submit is non-blocking.
///
/// DatadogSink::submit must enqueue to the internal worker channel and return
/// immediately — it must NOT block waiting for the HTTP round-trip.
#[test]
fn test_VP_011_datadog_inherits_http_sink_non_blocking_submit() {
    let toml = r#"
schema_version = 1
type = "datadog"
name = "non-blocking-inherit"
api_key = "vp011-inherit-key"
endpoint = "http://localhost:29999/api/v2/logs"
"#;

    let config = DatadogSinkConfig::from_toml(toml)
        .expect("from_toml must succeed")
        .expect("must return Some");

    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    // submit 100 events — all must return without blocking even if the worker
    // is busy or the endpoint is unreachable.
    let start = std::time::Instant::now();
    for i in 0..100 {
        sink.submit(SinkEvent::new().insert("type", "test.bulk").insert("i", i));
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 200,
        "100 submits must complete in < 200ms total (non-blocking), took {}ms",
        elapsed.as_millis()
    );
}

/// VP-012 + HttpSink retry — 5xx then 200: sink retries and succeeds.
///
/// Exercises: DatadogSink::new, Sink::submit, Sink::flush.
/// Mock server returns 500 twice, then 200. HttpSink retries up to 3 total
/// attempts (MAX_5XX_ATTEMPTS=3). Verifies that retry behavior is inherited.
#[tokio::test]
async fn test_VP_012_datadog_inherits_http_sink_5xx_retry_then_success() {
    let server = MockServer::start();
    let api_key = "retry-inherit-key";

    // First two requests return 500; third returns 202.
    let fail_mock = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(500).body("transient error");
    });
    let fail_mock2 = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(500).body("transient error 2");
    });
    let success_mock = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(202).body("{}");
    });

    let url = format!("{}/api/v2/logs", server.base_url());
    let config = config_for_mock_url(&url, api_key);
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    sink.submit(SinkEvent::new().insert("type", "test.retry_inherit"));
    // flush should not return Err — the retry logic handles the 5xx.
    sink.flush().expect("flush must succeed after retry");

    // Total request count must be > 1 (at least one retry occurred).
    // Sum hits across all three mocks to get total request count.
    let total_hits = fail_mock.calls() + fail_mock2.calls() + success_mock.calls();
    assert!(
        total_hits >= 2,
        "sink must have retried at least once (inherited from HttpSink), got {total_hits} total requests"
    );
}

/// VP-012 — 5xx exhausts all retries: failure recorded, not propagated as Err.
///
/// When all MAX_5XX_ATTEMPTS are exhausted, flush must return Ok (failure
/// is isolated to the sink — VP-012) and take_failures must be non-empty.
#[tokio::test]
async fn test_VP_012_datadog_5xx_exhausted_retries_recorded_not_propagated() {
    let server = MockServer::start();
    let api_key = "exhausted-retry-key";

    // All requests return 500.
    let _mock = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(500).body("always failing");
    });

    let url = format!("{}/api/v2/logs", server.base_url());
    let config = config_for_mock_url(&url, api_key);
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    sink.submit(SinkEvent::new().insert("type", "test.exhaust"));
    // flush must return Ok even when all retries fail (VP-012: failure isolated).
    sink.flush()
        .expect("flush must not return Err when retries are exhausted");

    let failures = sink.take_failures();
    assert!(
        !failures.is_empty(),
        "exhausted retries must be recorded in take_failures()"
    );

    let failure = &failures[0];
    assert!(
        failure.attempts >= 1,
        "failure must record at least 1 attempt, got {}",
        failure.attempts
    );
}
