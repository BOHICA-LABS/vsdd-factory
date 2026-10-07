// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-14: HoneycombSink mock HTTP integration.
//!
//! Traces to:
//! - v1.1 BC candidate: BC-3.NN.NNN-honeycomb-sink-payload-format
//!   (uncontracted in v1.0 per AC-14 BC note)
//!
//! SUT entry point: `Router::submit()` → `SinkRegistry::submit_all()` →
//! `HoneycombSink::send()` (wraps HttpSink internally).
//!
//! Oracle:
//! - Mock HTTP server receives exactly 1 POST with `X-Honeycomb-Team` header
//!   (enforced via httpmock 0.8 `when.header()` matcher)
//!
//! Note: Honeycomb payload schema is uncontracted-by-BC in v1.0. AC-14 verifies
//! behavior shipped via S-4.03 without a formal v1.0 BC anchor.
//!
//! RED gate: will fail until HoneycombSink correctly sends X-Honeycomb-Team header.

use httpmock::prelude::*;
use sink_core::{Sink, SinkConfigCommon, SinkEvent};
use sink_honeycomb::{HoneycombSink, HoneycombSinkConfig};

use factory_dispatcher::sinks::{ObservabilityConfig, Router, SinkRegistry, SinkStanza};

/// v1.1 BC candidate: honeycomb-sink-payload-format — AC-14:
///
/// HoneycombSink POSTs to the mock endpoint with `X-Honeycomb-Team` header.
/// Oracle: mock matcher enforces header presence — if the header is absent,
/// the mock returns 404 and the sink records a failure. Test verifies the
/// X-Honeycomb-Team-constrained mock receives exactly 1 hit.
///
/// RED gate: will fail until HoneycombSink sends the auth header correctly.
#[test]
fn test_BC_v1_1_honeycomb_sink_posts_with_x_honeycomb_team_header() {
    let server = MockServer::start();
    // Require X-Honeycomb-Team header. If absent → 404 → sink records failure.
    let hc_team_mock = server.mock(|when, then| {
        when.method(POST)
            .header("X-Honeycomb-Team", "test-hc-api-key-xyz");
        then.status(200).body("ok");
    });

    let cfg = HoneycombSinkConfig {
        sink_type: "honeycomb".to_string(),
        common: SinkConfigCommon {
            name: "hc-integration".to_string(),
            enabled: true,
            routing_filter: None,
            tags: Default::default(),
        },
        api_key: Some("test-hc-api-key-xyz".to_string()),
        dataset: Some("test-dataset".to_string()),
        base_url: Some(server.url("/1/events")),
        queue_depth: 64,
    };

    let sink = HoneycombSink::new(cfg).expect("build HoneycombSink");

    sink.submit(
        SinkEvent::new()
            .insert("type", "plugin.invoked")
            .insert("ts_epoch", serde_json::json!(1_777_003_425_u64))
            .insert("dispatcher_trace_id", "trace-hc-001"),
    );
    sink.flush().expect("flush");
    sink.shutdown();

    // Oracle: mock with X-Honeycomb-Team constraint received exactly 1 hit.
    hc_team_mock.assert_calls(1);
}

/// v1.1 BC candidate: honeycomb-sink-payload-format — posts to correct endpoint URL.
///
/// HoneycombSink builds URL as `<base_url>/<dataset>`. Oracle: mock at the
/// correct path receives the request.
///
/// RED gate: will fail until HoneycombSink constructs the correct URL.
#[test]
fn test_BC_v1_1_honeycomb_sink_posts_to_correct_dataset_path() {
    let server = MockServer::start();
    // Match on path + header.
    let path_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/1/events/my-dataset")
            .header("X-Honeycomb-Team", "hc-path-key");
        then.status(200).body("ok");
    });

    let cfg = HoneycombSinkConfig {
        sink_type: "honeycomb".to_string(),
        common: SinkConfigCommon {
            name: "hc-path".to_string(),
            enabled: true,
            routing_filter: None,
            tags: Default::default(),
        },
        api_key: Some("hc-path-key".to_string()),
        dataset: Some("my-dataset".to_string()),
        base_url: Some(server.url("/1/events")),
        queue_depth: 64,
    };

    let sink = HoneycombSink::new(cfg).expect("build HoneycombSink");
    sink.submit(SinkEvent::new().insert("type", "commit.made"));
    sink.flush().expect("flush");
    sink.shutdown();

    // Oracle: request hit the `/1/events/my-dataset` path.
    path_mock.assert_calls(1);
}

/// v1.1 BC candidate: honeycomb-time-field-rfc3339 — sink enriches with time field.
///
/// HoneycombSink enriches events with a `time` RFC3339 field. This test verifies
/// the POST is made (request reaches mock). The time field enrichment is verified
/// by the existing unit tests in sink-honeycomb; here we confirm the enriched
/// events are successfully POSTed.
///
/// RED gate: will fail until HoneycombSink sends the POST.
#[test]
fn test_BC_v1_1_honeycomb_sink_sends_events_to_endpoint() {
    let server = MockServer::start();
    let any_mock = server.mock(|when, then| {
        when.method(POST).header("X-Honeycomb-Team", "hc-time-key");
        then.status(200).body("ok");
    });

    let cfg = HoneycombSinkConfig {
        sink_type: "honeycomb".to_string(),
        common: SinkConfigCommon {
            name: "hc-time-field".to_string(),
            enabled: true,
            routing_filter: None,
            tags: Default::default(),
        },
        api_key: Some("hc-time-key".to_string()),
        dataset: Some("time-test".to_string()),
        base_url: Some(server.url("/1/events")),
        queue_depth: 64,
    };

    let sink = HoneycombSink::new(cfg).expect("build HoneycombSink");
    sink.submit(
        SinkEvent::new()
            .insert("type", "commit.made")
            .insert("ts_epoch", serde_json::json!(1_700_000_000_i64)),
    );
    sink.flush().expect("flush");
    sink.shutdown();

    assert!(
        any_mock.calls() >= 1,
        "AC-14 v1.1 honeycomb: HoneycombSink must POST to endpoint; got {} hits",
        any_mock.calls()
    );
}

/// AC-14 via Router::submit() — HoneycombSink wired through full Router path.
///
/// RED gate: will fail if `SinkRegistry::from_config` does not wire `type='honeycomb'`.
#[test]
fn test_BC_v1_1_honeycomb_sink_reachable_through_router_submit() {
    let server = MockServer::start();
    let hc_mock = server.mock(|when, then| {
        when.method(POST)
            .header("X-Honeycomb-Team", "hc-router-key");
        then.status(200).body("ok");
    });

    let mut extra = toml::value::Table::new();
    extra.insert(
        "api_key".into(),
        toml::Value::String("hc-router-key".into()),
    );
    extra.insert(
        "dataset".into(),
        toml::Value::String("router-dataset".into()),
    );
    extra.insert(
        "base_url".into(),
        toml::Value::String(server.url("/1/events")),
    );

    let cfg = ObservabilityConfig {
        schema_version: 1,
        sinks: vec![SinkStanza {
            type_: "honeycomb".into(),
            name: "hc-router".into(),
            dlq_enabled: false,
            extra,
        }],
    };

    // RED gate: SinkRegistry::from_config currently skips 'honeycomb' as unknown.
    let registry = SinkRegistry::from_config(cfg).expect("config load must succeed");
    assert_eq!(
        registry.sinks().len(),
        1,
        "AC-14 Router path RED gate: registry must have 1 HoneycombSink; \
         got {} (honeycomb not yet wired into from_config)",
        registry.sinks().len()
    );

    let router = Router::new(registry);
    router.submit(SinkEvent::new().insert("type", "plugin.invoked"));
    router.flush().expect("flush");
    router.shutdown();

    assert!(
        hc_mock.calls() >= 1,
        "AC-14 Router path: HoneycombSink must POST with X-Honeycomb-Team via Router::submit()"
    );
}
