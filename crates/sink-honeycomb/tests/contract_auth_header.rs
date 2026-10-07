// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-3.06.005 — X-Honeycomb-Team auth header is sent on every POST.
//!
//! AC: Auth via `X-Honeycomb-Team` header from config.
//! Traces to: BC-3.06.005 postcondition 1 (API key is a required field).

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_honeycomb::{HoneycombSink, HoneycombSinkConfig};

/// Build a HoneycombSinkConfig pointing at a mock server URL.
fn config_for_mock(server: &MockServer, dataset: &str, api_key: &str) -> HoneycombSinkConfig {
    // We override the base URL to point at the mock server by constructing
    // the TOML with an explicit url field. Since HoneycombSinkConfig wraps
    // HttpSink, we rely on the implementation accepting a base_url override
    // for tests. For the RED gate, from_toml will unimplemented!(), so this
    // test fails at that call.
    let toml_src = format!(
        r#"
        type = "honeycomb"
        name = "auth-header-test"
        api_key = "{api_key}"
        dataset = "{dataset}"
        base_url = "http://127.0.0.1:{port}/1/events"
        "#,
        port = server.port()
    );
    HoneycombSinkConfig::from_toml(&toml_src)
        .expect("no error")
        .expect("Some")
}

#[test]
fn test_BC_3_06_005_post_includes_x_honeycomb_team_header() {
    // Every POST to Honeycomb must carry X-Honeycomb-Team: <api_key>.
    let server = MockServer::start();
    let api_key = "hcaik_test_secret_key_abc123";
    let dataset = "auth-test-dataset";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/1/events/{dataset}"))
            .header("X-Honeycomb-Team", api_key);
        then.status(200);
    });

    let config = config_for_mock(&server, dataset, api_key);
    let sink = HoneycombSink::new(config).expect("new succeeds");
    sink.submit(SinkEvent::new().insert("type", "plugin.invoked"));
    sink.flush().expect("flush succeeds");
    sink.shutdown();

    mock.assert();
}

#[test]
fn test_BC_3_06_005_auth_header_value_matches_configured_api_key() {
    // The header value must be exactly the configured api_key, unmodified.
    let server = MockServer::start();
    let api_key = "Bearer-style-key-XYZ";
    let dataset = "exact-key-test";

    // Single mock with chained matchers (AND condition): header must exist
    // AND its value must exactly match the configured api_key.
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/1/events/{dataset}"))
            .header_exists("X-Honeycomb-Team")
            .header("X-Honeycomb-Team", api_key);
        then.status(200);
    });

    let config = config_for_mock(&server, dataset, api_key);
    let sink = HoneycombSink::new(config).expect("new succeeds");
    sink.submit(SinkEvent::new().insert("type", "commit.made"));
    sink.flush().expect("flush");
    sink.shutdown();

    mock.assert_calls(1);
}

#[test]
fn test_BC_3_06_005_content_type_is_json() {
    // Every POST must also set Content-Type: application/json.
    let server = MockServer::start();
    let dataset = "ct-test";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/1/events/{dataset}"))
            .header("Content-Type", "application/json");
        then.status(200);
    });

    let config = config_for_mock(&server, dataset, "ct-key");
    let sink = HoneycombSink::new(config).expect("new");
    sink.submit(SinkEvent::new().insert("type", "pr.merged"));
    sink.flush().expect("flush");
    sink.shutdown();

    mock.assert();
}
