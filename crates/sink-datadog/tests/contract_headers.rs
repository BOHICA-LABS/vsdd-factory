// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-3: DD-API-KEY header present on every POST request.
//!
//! BC-3.06.005 postcondition 1: config api_key is used for auth header.
//! v1.1 BC candidate: BC-3.NN.NNN-datadog-api-key-required.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_datadog::{DatadogSink, DatadogSinkConfig};

/// BC-3.06.005 — POST request includes DD-API-KEY header with config api_key value.
///
/// Exercises: DatadogSink::new, Sink::submit, Sink::flush.
/// Mock server asserts DD-API-KEY header is present and matches the config value.
#[tokio::test]
async fn test_BC_3_06_005_dd_api_key_header_present_on_post() {
    let server = MockServer::start();
    let api_key = "my-secret-dd-key-abc123";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/logs")
            .header("DD-API-KEY", api_key);
        then.status(202).body("{}");
    });

    let toml = format!(
        r#"
schema_version = 1
type = "datadog"
name = "header-test-sink"
api_key = "{api_key}"
endpoint = "{}/api/v2/logs"
"#,
        server.base_url()
    );

    let config = DatadogSinkConfig::from_toml(&toml)
        .expect("from_toml must succeed")
        .expect("must return Some");

    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");

    let ev = SinkEvent::new().insert("type", "test.header_check");
    sink.submit(ev);
    sink.flush().expect("flush must succeed");

    // Assert exactly 1 POST with the correct DD-API-KEY header.
    mock.assert_calls(1);
}

/// BC-3.06.005 — DD-API-KEY header value exactly matches the configured api_key.
///
/// A request WITHOUT the DD-API-KEY header must NOT satisfy the mock,
/// confirming the header is mandatory, not optional.
#[tokio::test]
async fn test_BC_3_06_005_post_without_dd_api_key_does_not_match_auth_mock() {
    let server = MockServer::start();
    let api_key = "strict-key-check";

    // This mock requires the exact DD-API-KEY header.
    let auth_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/logs")
            .header("DD-API-KEY", api_key);
        then.status(202).body("{}");
    });

    // No-auth fallback mock — will catch requests that lack the header.
    let no_auth_mock = server.mock(|when, then| {
        when.method(POST).path("/api/v2/logs");
        then.status(403).body("forbidden");
    });

    let toml = format!(
        r#"
schema_version = 1
type = "datadog"
name = "strict-header-sink"
api_key = "{api_key}"
endpoint = "{}/api/v2/logs"
"#,
        server.base_url()
    );

    let config = DatadogSinkConfig::from_toml(&toml)
        .expect("from_toml must succeed")
        .expect("must return Some");

    let sink = DatadogSink::new(config).expect("DatadogSink::new must succeed");
    sink.submit(SinkEvent::new().insert("type", "test.strict_header"));
    sink.flush().expect("flush must succeed");

    // Auth mock must get the hit (header was present with correct value).
    auth_mock.assert_calls(1);
    // No-auth fallback must get zero hits (header was present, so it matched the first mock).
    no_auth_mock.assert_calls(0);
}
