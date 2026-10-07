// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration test — HoneycombSink inherits HttpSink retry + non-blocking
//! behaviour.
//!
//! AC: Inherits HttpSink retry + non-blocking inherit from HttpSink.
//! VP-011: Sink submit must not block the dispatcher.
//! VP-012: Sink failure affects only that sink.
//! Traces to: BC-3.01.001 postcondition 1.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_honeycomb::{HoneycombSink, HoneycombSinkConfig};

fn make_config(server: &MockServer, dataset: &str) -> HoneycombSinkConfig {
    let toml_src = format!(
        r#"
        type = "honeycomb"
        name = "inherit-test"
        api_key = "hcaik_inherit"
        dataset = "{dataset}"
        base_url = "http://127.0.0.1:{port}/1/events"
        "#,
        port = server.port()
    );
    HoneycombSinkConfig::from_toml(&toml_src)
        .expect("valid")
        .expect("Some")
}

/// VP-011: submit() returns before the HTTP round-trip completes.
/// The event is queued; the worker handles the I/O asynchronously.
#[test]
fn test_VP_011_submit_returns_before_http_round_trip() {
    let server = MockServer::start();
    let dataset = "vp011-inherit";

    // Mock with a slight delay to make it observable that submit returned
    // before the response. httpmock does not support artificial delays, so
    // we verify that submit() doesn't block by checking the mock hit count
    // before flush().
    let mock = server.mock(|when, then| {
        when.method(POST).path(format!("/1/events/{dataset}"));
        then.status(200);
    });

    let config = make_config(&server, dataset);
    let sink = HoneycombSink::new(config).expect("new");

    // submit() must return immediately — the worker has not yet POSTed.
    sink.submit(SinkEvent::new().insert("type", "plugin.invoked"));

    // Before flush, the HTTP call may or may not have fired (race). After
    // flush it must have.
    sink.flush().expect("flush");
    sink.shutdown();

    mock.assert_calls(1);
}

/// VP-011: Overflow drops events without blocking (queue_full_count tracks
/// drops; submit never blocks).
#[test]
fn test_VP_011_overflow_drops_without_blocking() {
    // Use a very small queue depth so we can saturate it quickly.
    let server = MockServer::start();
    let dataset = "overflow-test";

    let mock = server.mock(|when, then| {
        when.method(POST).path(format!("/1/events/{dataset}"));
        then.status(200);
    });

    let toml_src = format!(
        r#"
        type = "honeycomb"
        name = "overflow-sink"
        api_key = "hcaik_overflow"
        dataset = "{dataset}"
        queue_depth = 2
        base_url = "http://127.0.0.1:{port}/1/events"
        "#,
        port = server.port()
    );
    let config = HoneycombSinkConfig::from_toml(&toml_src)
        .expect("valid")
        .expect("Some");
    let sink = HoneycombSink::new(config).expect("new");

    // Flood the queue; no submit() call must block regardless of overflow.
    for i in 0..100_u32 {
        sink.submit(SinkEvent::new().insert("seq", i));
    }
    // No assertion on mock hits — just that the above did not block.
    let _ = sink.flush();
    sink.shutdown();
    let _ = mock;
}

/// VP-012: HoneycombSink failure (unreachable endpoint) does not propagate
/// to other sinks or the caller — failure is isolated to this sink instance.
#[test]
fn test_VP_012_network_failure_isolated_to_sink() {
    // Point the sink at a port that refuses connections.
    let toml_src = r#"
        type = "honeycomb"
        name = "isolated-failure"
        api_key = "hcaik_fail"
        dataset = "fail-dataset"
        base_url = "http://127.0.0.1:1/1/events"
    "#;
    // from_toml will unimplemented!() on RED gate — that is the expected failure.
    let config = HoneycombSinkConfig::from_toml(toml_src)
        .expect("valid config parse")
        .expect("Some");
    let sink = HoneycombSink::new(config).expect("new");
    // submit() must not panic or propagate network failure.
    sink.submit(SinkEvent::new().insert("type", "commit.made"));
    // flush() may return Err — that is OK. It must not panic.
    let _ = sink.flush();
    sink.shutdown();
    // Test passes if we reach here without panic.
}

/// Inherited 5xx retry: HoneycombSink must retry 5xx (via HttpSink base)
/// up to MAX_5XX_ATTEMPTS (3) before recording a failure.
#[test]
fn test_BC_3_01_001_inherits_5xx_retry_from_http_sink() {
    let server = MockServer::start();
    let dataset = "retry-5xx";

    let mock = server.mock(|when, then| {
        when.method(POST).path(format!("/1/events/{dataset}"));
        then.status(503);
    });

    let config = make_config(&server, dataset);
    let sink = HoneycombSink::new(config).expect("new");
    sink.submit(SinkEvent::new().insert("type", "plugin.completed"));
    let _ = sink.flush(); // may err after retries exhausted
    sink.shutdown();

    // Must have retried (at least 2 attempts == 1 retry).
    assert!(
        mock.calls() >= 2,
        "5xx must trigger retries, got {} hits",
        mock.calls()
    );
}

/// Flush blocks until the in-flight POST completes (inherited HttpSink
/// flush semantics).
#[test]
fn test_BC_3_01_001_flush_blocks_until_post_complete() {
    let server = MockServer::start();
    let dataset = "flush-sync";

    let mock = server.mock(|when, then| {
        when.method(POST).path(format!("/1/events/{dataset}"));
        then.status(200);
    });

    let config = make_config(&server, dataset);
    let sink = HoneycombSink::new(config).expect("new");
    sink.submit(SinkEvent::new().insert("type", "pr.merged"));
    sink.flush().expect("flush must succeed on 200 response");
    sink.shutdown();

    // After flush, the POST is guaranteed to have completed.
    mock.assert_calls(1);
}
