// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-2 (send to /api/v2/logs), AC-5 (Datadog schema mapping), AC-7 (integration test).
//!
//! BC-3.01.001 postcondition 1: Sink integrates with registry machinery end-to-end.
//! v1.1 BC candidate: BC-3.NN.NNN-datadog-event-schema-mapping.
//! v1.1 BC candidate: BC-3.NN.NNN-datadog-5mb-batch-split.
//!
//! Tests use httpmock to simulate the Datadog Logs Intake endpoint.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_datadog::{DATADOG_MAX_BATCH_BYTES, DatadogSink, DatadogSinkConfig};

fn make_event(msg: &str, source: &str) -> SinkEvent {
    SinkEvent::new()
        .insert("type", "plugin.invoked")
        .insert("message", msg)
        .insert("ddsource", source)
        .insert("service", "vsdd-factory")
}

fn config_for_mock(server: &MockServer, api_key: &str) -> DatadogSinkConfig {
    let toml = format!(
        r#"
schema_version = 1
type = "datadog"
name = "integration-test-sink"
api_key = "{api_key}"
endpoint = "{}/api/v2/logs"
"#,
        server.base_url()
    );
    DatadogSinkConfig::from_toml(&toml)
        .expect("from_toml must succeed")
        .expect("must return Some")
}

/// BC-3.01.001 — events POSTed as JSON array to mock Datadog endpoint.
///
/// Exercises: DatadogSink::new, Sink::submit (x3), Sink::flush.
/// Mock asserts: 1 POST, body is JSON array, all 3 event messages present,
/// DD-API-KEY header present.
#[tokio::test]
async fn test_BC_3_01_001_events_posted_as_json_array_to_datadog_endpoint() {
    let server = MockServer::start();
    let api_key = "batch-post-test-key";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/logs")
            .header("DD-API-KEY", api_key)
            .body_includes(r#""message":"event-a""#)
            .body_includes(r#""message":"event-b""#)
            .body_includes(r#""message":"event-c""#);
        then.status(202).body("{}");
    });

    let config = config_for_mock(&server, api_key);
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    sink.submit(make_event("event-a", "factory"));
    sink.submit(make_event("event-b", "factory"));
    sink.submit(make_event("event-c", "factory"));
    sink.flush().expect("flush must succeed");

    // All 3 events batched into 1 POST with auth header.
    mock.assert_calls(1);
}

/// AC-5 — Datadog schema fields present in POST body.
///
/// Factory events must be mapped so that ddsource, service, message appear
/// in the JSON body (v1.1 BC candidate: BC-3.NN.NNN-datadog-event-schema-mapping).
#[tokio::test]
async fn test_BC_3_01_001_datadog_schema_fields_present_in_post_body() {
    let server = MockServer::start();
    let api_key = "schema-test-key";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/logs")
            .body_includes("ddsource")
            .body_includes("service")
            .body_includes("message");
        then.status(202).body("{}");
    });

    let config = config_for_mock(&server, api_key);
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    sink.submit(
        SinkEvent::new()
            .insert("type", "plugin.invoked")
            .insert("message", "hello datadog")
            .insert("ddsource", "vsdd-factory")
            .insert("service", "factory-dispatcher"),
    );
    sink.flush().expect("flush must succeed");

    mock.assert_calls(1);
}

/// VP-011 — DatadogSink submit is non-blocking (does not block caller thread).
///
/// Exercises: DatadogSink::submit called from a synchronous context.
/// submit must return without blocking even when the worker is busy.
#[test]
fn test_VP_011_datadog_sink_submit_does_not_block() {
    // We need a reachable endpoint for construction but submit() is tested
    // synchronously before flush is called — the worker handles it async.
    let toml = r#"
schema_version = 1
type = "datadog"
name = "non-blocking-test"
api_key = "vp011-key"
endpoint = "http://localhost:19999/api/v2/logs"
"#;

    let config = DatadogSinkConfig::from_toml(toml)
        .expect("from_toml must succeed")
        .expect("must return Some");

    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    // submit must return immediately without blocking.
    let ev = SinkEvent::new().insert("type", "test.non_blocking");
    let start = std::time::Instant::now();
    sink.submit(ev);
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "submit must return in < 100ms (non-blocking), took {}ms",
        elapsed.as_millis()
    );
}

/// VP-012 — DatadogSink failure recorded and does not affect other sinks.
///
/// When the Datadog endpoint returns 5xx, the failure is recorded and
/// isolated — the sink does not panic or propagate the error to the caller.
#[tokio::test]
async fn test_VP_012_datadog_sink_5xx_failure_recorded_and_isolated() {
    let server = MockServer::start();
    let api_key = "vp012-key";

    // Return 500 for all requests.
    let _mock = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(500).body("internal server error");
    });

    let config = config_for_mock(&server, api_key);
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    sink.submit(SinkEvent::new().insert("type", "test.vp012"));
    sink.flush()
        .expect("flush must not propagate 5xx as a Rust error");

    // Failure must be recorded in the sink's failure list (VP-012: isolated, not propagated).
    let failures = sink.take_failures();
    assert!(
        !failures.is_empty(),
        "5xx response must be recorded in take_failures(), got empty list"
    );
}

/// AC-6 — 5MB payload limit enforced: DATADOG_MAX_BATCH_BYTES constant is 5MB
/// AND the sink is constructed successfully (implementation exists).
///
/// v1.1 BC candidate: BC-3.NN.NNN-datadog-5mb-batch-split.
/// RED gate: DatadogSink::new panics on unimplemented!() until GREEN.
#[tokio::test]
async fn test_BC_3_01_001_max_batch_bytes_constant_is_5mb_and_sink_constructs() {
    assert_eq!(
        DATADOG_MAX_BATCH_BYTES,
        5 * 1024 * 1024,
        "DATADOG_MAX_BATCH_BYTES must be exactly 5MB (5 * 1024 * 1024 bytes)"
    );

    // Also verify the sink can be constructed — panics on unimplemented!() until GREEN.
    let toml = r#"
schema_version = 1
type = "datadog"
name = "batch-limit-check"
api_key = "limit-test-key"
endpoint = "http://localhost:49999/api/v2/logs"
"#;
    let config = DatadogSinkConfig::from_toml(toml)
        .expect("from_toml must succeed")
        .expect("must return Some");
    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");
    sink.shutdown();
}
