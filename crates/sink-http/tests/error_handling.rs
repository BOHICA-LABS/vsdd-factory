// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-6: HTTP error handling — retry on 5xx, drop on 4xx.
//!
//! VP-012: Sink Failure Affects Only That Sink — failure isolation invariant.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_http::{HttpSink, HttpSinkConfig};

fn make_event() -> SinkEvent {
    SinkEvent::new()
        .insert("type", "test.error_handling")
        .insert("payload", "x")
}

fn config_for_url(url: &str) -> HttpSinkConfig {
    let toml = format!(
        r#"
schema_version = 1
type = "http"
name = "error-test-sink"
url = "{url}"
"#
    );
    HttpSinkConfig::from_toml(&toml)
        .expect("from_toml must succeed")
        .expect("must return Some")
}

/// VP-012 — 5xx response triggers retry; failure recorded; dispatcher unblocked.
///
/// Exercises: HttpSink::new, Sink::submit, Sink::flush, HttpSink::take_failures.
/// Mock returns 503. Verifies: >=2 POST attempts (retry); 1 SinkFailure recorded;
/// flush() returns without panicking (failure isolation — dispatcher unaffected).
#[tokio::test]
async fn test_VP_012_5xx_retries_then_records_failure() {
    let server = MockServer::start();
    // Always respond 503 to force all retries to fail.
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(503).body("service unavailable");
    });

    let config = config_for_url(&format!("{}/events", server.base_url()));
    let sink = HttpSink::new(config).expect("HttpSink::new must succeed");

    sink.submit(make_event());

    // flush() must NOT panic/propagate — failure isolation is the contract.
    // It may return Ok (failure recorded internally) or Err (implementer choice),
    // but the dispatcher thread must remain alive.
    let _ = sink.flush();

    // Retry behavior: at least 2 attempts before giving up.
    let hits = mock.calls();
    assert!(
        hits >= 2,
        "5xx must trigger at least 1 retry (>=2 total attempts), got {hits}"
    );

    // Failure must be recorded for operator inspection.
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure must be recorded for the 503 batch"
    );
    assert!(
        failures[0].attempts >= 2,
        "recorded SinkFailure must report >=2 attempts, got {}",
        failures[0].attempts
    );
}

/// VP-012 — 4xx response drops batch immediately without retry.
///
/// Exercises: HttpSink::new, Sink::submit, Sink::flush, HttpSink::take_failures.
/// Mock returns 400. Verifies: exactly 1 POST attempt (no retry); 1 SinkFailure
/// recorded; flush() returns without panicking.
#[tokio::test]
async fn test_VP_012_4xx_drops_immediately_no_retry() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(400).body("bad request");
    });

    let config = config_for_url(&format!("{}/events", server.base_url()));
    let sink = HttpSink::new(config).expect("HttpSink::new must succeed");

    sink.submit(make_event());

    // flush() must not panic — failure isolation.
    let _ = sink.flush();

    // Exactly 1 attempt — 4xx is a client error, no retry is correct behavior.
    let hits = mock.calls();
    assert_eq!(
        hits, 1,
        "4xx must result in exactly 1 attempt (no retry), got {hits}"
    );

    // Failure must be recorded (batch dropped, not silently lost).
    let failures = sink.take_failures();
    assert_eq!(
        failures.len(),
        1,
        "exactly 1 SinkFailure must be recorded for the 400 batch"
    );
    assert_eq!(
        failures[0].attempts, 1,
        "recorded SinkFailure must report exactly 1 attempt for 4xx"
    );
}
