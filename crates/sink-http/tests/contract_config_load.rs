// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! AC-2, AC-3, AC-4: Config load validation tests.
//!
//! BC-3.06.005 — disabled config: no events accepted, no HTTP calls.
//! BC-3.01.002 — unknown sink type warns but does not fail load.
//! BC-3.01.003 — schema_version != 1 is a hard error.

use httpmock::prelude::*;
use sink_core::{Sink, SinkEvent};
use sink_http::{HttpSink, HttpSinkConfig};

/// BC-3.06.005 — disabled sink accepts no events and makes no HTTP calls.
///
/// Exercises: HttpSinkConfig::from_toml (enabled=false), HttpSink::new,
/// Sink::accepts, Sink::submit, Sink::flush.
/// Mock server uses expect(0) to assert zero HTTP calls.
#[tokio::test]
async fn test_BC_3_06_005_disabled_config_no_events_accepted() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/events");
        then.status(200).body("{}");
    });

    let toml = format!(
        r#"
schema_version = 1
type = "http"
name = "disabled-sink"
enabled = false
url = "{}/events"
"#,
        server.base_url()
    );

    let config = HttpSinkConfig::from_toml(&toml)
        .expect("from_toml must not error for disabled sink")
        .expect("must return Some — disabled is not unknown type");

    let sink = HttpSink::new(config).expect("HttpSink::new must succeed");

    // A disabled sink must refuse every event.
    let ev = SinkEvent::new().insert("type", "test.event");
    assert!(
        !sink.accepts(&ev),
        "disabled sink must return false from accepts()"
    );

    // Even if submit is called directly, flush must not POST anything.
    sink.submit(ev);
    sink.flush().expect("flush must not error");

    // Zero calls to the mock server — the core assertion for AC-2.
    mock.assert_calls(0);
}

/// BC-3.01.002 — unknown sink type logs warning and returns Ok(None).
///
/// Exercises: HttpSinkConfig::from_toml with type="unknownfoo".
/// The load must succeed (Ok) and return None (sink skipped).
#[test]
fn test_BC_3_01_002_unknown_sink_type_warns_no_fail() {
    let toml = r#"
schema_version = 1
type = "unknownfoo"
name = "mystery-sink"
url = "http://localhost:9999/events"
"#;

    // BC postcondition: load returns Ok (not Err).
    let result =
        HttpSinkConfig::from_toml(toml).expect("from_toml must not return Err for unknown type");

    // BC postcondition: unknown type means sink is skipped (None).
    assert!(
        result.is_none(),
        "unknown sink type must return Ok(None) so the rest of the config loads"
    );
    // Note: the warning-to-stderr assertion is inherently observational.
    // The implementer must write to stderr (eprintln! or tracing::warn!) for
    // this BC. The test verifies the non-failure path; manual inspection or
    // a stderr-capture harness verifies the warning side.
}

/// BC-3.01.003 — schema_version != 1 is a hard error at load time.
///
/// Exercises: HttpSinkConfig::from_toml with schema_version=2.
/// Returns Err containing "schema_version".
#[test]
fn test_BC_3_01_003_schema_version_mismatch_is_hard_error() {
    let toml = r#"
schema_version = 2
type = "http"
name = "future-sink"
url = "http://localhost:9999/events"
"#;

    let result = HttpSinkConfig::from_toml(toml);

    assert!(
        result.is_err(),
        "schema_version != 1 must return Err — got Ok instead"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("schema_version"),
        "error message must mention 'schema_version', got: {err_msg}"
    );
}
