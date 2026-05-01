// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_calls_streams.py.
//
// Each Rust test:
//   1. Calls the SDK method (no transport mocking).
//   2. Asserts on the response struct shape returned by the mock.
//   3. Re-asserts the wire request via mocktest::journal_last() — method,
//      path, and body.
//
// Run with: `cargo test --test rest_mock_compat_calls_streams -- --test-threads=1`.

#[path = "common/mod.rs"]
mod common;

use serde_json::json;

// ---------------------------------------------------------------------------
// CompatCalls::start_stream
// ---------------------------------------------------------------------------

#[test]
fn test_compat_calls_start_stream_returns_stream_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .calls()
        .start_stream(
            "CA_TEST",
            &json!({"Url": "wss://example.com/stream", "Name": "my-stream"}),
        )
        .expect("start_stream");
    assert!(result.is_object(), "expected JSON object, got {result:?}");
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("name"),
        "expected stream sid or name in body, got keys {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_TEST/Streams"
    );
}

#[test]
fn test_compat_calls_start_stream_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .start_stream("CA_JX1", &json!({"Url": "wss://a.b/s", "Name": "strm-x"}))
        .expect("start_stream");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_JX1/Streams"
    );
    let body = entry.body_object().expect("body should be object");
    assert_eq!(body.get("Url").and_then(|v| v.as_str()), Some("wss://a.b/s"));
    assert_eq!(body.get("Name").and_then(|v| v.as_str()), Some("strm-x"));
}

// ---------------------------------------------------------------------------
// CompatCalls::stop_stream
// ---------------------------------------------------------------------------

#[test]
fn test_compat_calls_stop_stream_returns_stream_resource_with_status() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .calls()
        .stop_stream("CA_T1", "ST_T1", &json!({"Status": "stopped"}))
        .expect("stop_stream");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("status"),
        "expected sid or status, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_T1/Streams/ST_T1"
    );
}

#[test]
fn test_compat_calls_stop_stream_journal_records_post_to_specific_stream() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .stop_stream("CA_S1", "ST_S1", &json!({"Status": "stopped"}))
        .expect("stop_stream");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_S1/Streams/ST_S1"
    );
    let body = entry.body_object().expect("body should be object");
    assert_eq!(body.get("Status").and_then(|v| v.as_str()), Some("stopped"));
}

// ---------------------------------------------------------------------------
// CompatCalls::update_recording
// ---------------------------------------------------------------------------

#[test]
fn test_compat_calls_update_recording_returns_recording_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .calls()
        .update_recording("CA_T2", "RE_T2", &json!({"Status": "paused"}))
        .expect("update_recording");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("status"),
        "expected sid or status, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_T2/Recordings/RE_T2"
    );
}

#[test]
fn test_compat_calls_update_recording_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .update_recording("CA_R1", "RE_R1", &json!({"Status": "paused"}))
        .expect("update_recording");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_R1/Recordings/RE_R1"
    );
    let body = entry.body_object().expect("body should be object");
    assert_eq!(body.get("Status").and_then(|v| v.as_str()), Some("paused"));
}
