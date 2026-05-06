// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_calling_mock.py.
//
// Every command in the Calling namespace is exercised here against the
// real SDK + the local mock_signalwire server. Each test:
//
//   1. Calls the SDK method (no transport patching).
//   2. Asserts on the mock's response body shape.
//   3. Asserts on mocktest::journal_last() so we know the SDK sent the
//      right wire request — method, path, command field, the optional
//      top-level `id`, and selected params.

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};

const CALLS_PATH: &str = "/api/calling/calls";

// Helper: fetch params object from the journaled body.
fn params_from_body(body_obj: &serde_json::Map<String, Value>) -> &serde_json::Map<String, Value> {
    body_obj
        .get("params")
        .and_then(Value::as_object)
        .expect("body.params should be object")
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_calling_update() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .update(json!({"id": "call-1", "state": "hold"}))
        .expect("calling.update");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, CALLS_PATH);
    assert!(entry.matched_route.is_some());
    let body_obj = entry.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("update")
    );
    assert!(
        !body_obj.contains_key("id"),
        "top-level body must not contain id"
    );
    let params = params_from_body(body_obj);
    assert_eq!(params.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(params.get("state").and_then(Value::as_str), Some("hold"));
}

#[test]
fn test_calling_dial_forwards_codecs_array() {
    // OpenAPI spec gained an optional `codecs` param on calling/calls dial
    // (porting-sdk PR #1). dial(Value) accepts free-form JSON so codecs
    // flows through without source changes; this test confirms it reaches
    // the wire as an array.
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .dial(json!({
            "url": "https://example.com/swml",
            "to": "+15551234567",
            "codecs": ["OPUS", "G729", "VP8", "PCMA"],
        }))
        .expect("calling.dial");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, CALLS_PATH);
    let body_obj = entry.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("dial")
    );
    assert!(
        !body_obj.contains_key("id"),
        "top-level body must not contain id"
    );
    let params = params_from_body(body_obj);
    assert_eq!(
        params.get("to").and_then(Value::as_str),
        Some("+15551234567")
    );
    let codecs = params
        .get("codecs")
        .and_then(Value::as_array)
        .expect("codecs should be array");
    let codecs_str: Vec<&str> = codecs.iter().filter_map(Value::as_str).collect();
    assert_eq!(codecs_str, vec!["OPUS", "G729", "VP8", "PCMA"]);
}

#[test]
fn test_calling_dial_forwards_codecs_string() {
    // Comma-separated-string form of the same param is also valid per the
    // OpenAPI spec; confirm it round-trips unchanged through the wire.
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .dial(json!({
            "url": "https://example.com/swml",
            "to": "+15551234567",
            "codecs": "OPUS,G729,VP8,PCMA",
        }))
        .expect("calling.dial");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("dial")
    );
    let params = params_from_body(body_obj);
    assert_eq!(
        params.get("codecs").and_then(Value::as_str),
        Some("OPUS,G729,VP8,PCMA")
    );
}

#[test]
fn test_calling_transfer() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .transfer(
            "call-123",
            json!({"destination": "+15551234567", "from_number": "+15559876543"}),
        )
        .expect("calling.transfer");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, CALLS_PATH);
    let body_obj = entry.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.transfer")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-123"));
    let params = params_from_body(body_obj);
    assert_eq!(
        params.get("destination").and_then(Value::as_str),
        Some("+15551234567")
    );
    assert_eq!(
        params.get("from_number").and_then(Value::as_str),
        Some("+15559876543")
    );
}

#[test]
fn test_calling_disconnect() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .disconnect("call-456", json!({"reason": "busy"}))
        .expect("calling.disconnect");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, CALLS_PATH);
    let body_obj = entry.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.disconnect")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-456"));
    assert_eq!(
        params_from_body(body_obj)
            .get("reason")
            .and_then(Value::as_str),
        Some("busy")
    );
}

// ---------------------------------------------------------------------------
// Play
// ---------------------------------------------------------------------------

#[test]
fn test_calling_play_pause() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .play_pause("call-1", json!({"control_id": "ctrl-1"}))
        .expect("play_pause");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, CALLS_PATH);
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.play.pause")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("ctrl-1")
    );
}

#[test]
fn test_calling_play_resume() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .play_resume("call-1", json!({"control_id": "ctrl-1"}))
        .expect("play_resume");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.play.resume")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("ctrl-1")
    );
}

#[test]
fn test_calling_play_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .play_stop("call-1", json!({"control_id": "ctrl-1"}))
        .expect("play_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.play.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

#[test]
fn test_calling_play_volume() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .play_volume("call-1", json!({"control_id": "ctrl-1", "volume": 2.5}))
        .expect("play_volume");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.play.volume")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("volume")
            .and_then(Value::as_f64),
        Some(2.5)
    );
}

// ---------------------------------------------------------------------------
// Record
// ---------------------------------------------------------------------------

#[test]
fn test_calling_record() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .record("call-1", json!({"record": {"format": "mp3"}}))
        .expect("record");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.record")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    let params = params_from_body(body_obj);
    let record_obj = params
        .get("record")
        .and_then(Value::as_object)
        .expect("record sub-object");
    assert_eq!(
        record_obj.get("format").and_then(Value::as_str),
        Some("mp3")
    );
}

#[test]
fn test_calling_record_pause() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .record_pause("call-1", json!({"control_id": "rec-1"}))
        .expect("record_pause");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.record.pause")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("rec-1")
    );
}

#[test]
fn test_calling_record_resume() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .record_resume("call-1", json!({"control_id": "rec-1"}))
        .expect("record_resume");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.record.resume")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("rec-1")
    );
}

// ---------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------

#[test]
fn test_calling_collect() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .collect("call-1", json!({"initial_timeout": 5, "digits": {"max": 4}}))
        .expect("collect");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.collect")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("initial_timeout")
            .and_then(Value::as_i64),
        Some(5)
    );
}

#[test]
fn test_calling_collect_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .collect_stop("call-1", json!({"control_id": "col-1"}))
        .expect("collect_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.collect.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("col-1")
    );
}

#[test]
fn test_calling_collect_start_input_timers() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .collect_start_input_timers("call-1", json!({"control_id": "col-1"}))
        .expect("collect_start_input_timers");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.collect.start_input_timers")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("col-1")
    );
}

// ---------------------------------------------------------------------------
// Detect
// ---------------------------------------------------------------------------

#[test]
fn test_calling_detect() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .detect("call-1", json!({"detect": {"type": "machine", "params": {}}}))
        .expect("detect");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.detect")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    let detect = params_from_body(body_obj)
        .get("detect")
        .and_then(Value::as_object)
        .expect("detect sub-object");
    assert_eq!(
        detect.get("type").and_then(Value::as_str),
        Some("machine")
    );
}

#[test]
fn test_calling_detect_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .detect_stop("call-1", json!({"control_id": "det-1"}))
        .expect("detect_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.detect.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("det-1")
    );
}

// ---------------------------------------------------------------------------
// Tap
// ---------------------------------------------------------------------------

#[test]
fn test_calling_tap() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .tap(
            "call-1",
            json!({"tap": {"type": "audio"}, "device": {"type": "rtp"}}),
        )
        .expect("tap");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.tap")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    let tap_obj = params_from_body(body_obj)
        .get("tap")
        .and_then(Value::as_object)
        .expect("tap sub-object");
    assert_eq!(tap_obj.get("type").and_then(Value::as_str), Some("audio"));
}

#[test]
fn test_calling_tap_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .tap_stop("call-1", json!({"control_id": "tap-1"}))
        .expect("tap_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.tap.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("tap-1")
    );
}

// ---------------------------------------------------------------------------
// Stream
// ---------------------------------------------------------------------------

#[test]
fn test_calling_stream() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .stream("call-1", json!({"url": "wss://example.com/audio"}))
        .expect("stream");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.stream")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("url")
            .and_then(Value::as_str),
        Some("wss://example.com/audio")
    );
}

#[test]
fn test_calling_stream_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .stream_stop("call-1", json!({"control_id": "stream-1"}))
        .expect("stream_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.stream.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("stream-1")
    );
}

// ---------------------------------------------------------------------------
// Denoise
// ---------------------------------------------------------------------------

#[test]
fn test_calling_denoise() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.calling().denoise("call-1", json!({})).expect("denoise");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.denoise")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

#[test]
fn test_calling_denoise_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .denoise_stop("call-1", json!({"control_id": "dn-1"}))
        .expect("denoise_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.denoise.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("dn-1")
    );
}

// ---------------------------------------------------------------------------
// Transcribe
// ---------------------------------------------------------------------------

#[test]
fn test_calling_transcribe() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .transcribe(
            "call-1",
            json!({"language": "en-US", "transcribe": {"engine": "google"}}),
        )
        .expect("transcribe");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.transcribe")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("language")
            .and_then(Value::as_str),
        Some("en-US")
    );
}

#[test]
fn test_calling_transcribe_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .transcribe_stop("call-1", json!({"control_id": "tr-1"}))
        .expect("transcribe_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.transcribe.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("control_id")
            .and_then(Value::as_str),
        Some("tr-1")
    );
}

// ---------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------

#[test]
fn test_calling_ai_hold() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.calling().ai_hold("call-1", json!({})).expect("ai_hold");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.ai_hold")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

#[test]
fn test_calling_ai_unhold() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.calling().ai_unhold("call-1", json!({})).expect("ai_unhold");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.ai_unhold")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

#[test]
fn test_calling_ai_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.calling().ai_stop("call-1", json!({})).expect("ai_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.ai.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

// ---------------------------------------------------------------------------
// Live transcribe / translate
// ---------------------------------------------------------------------------

#[test]
fn test_calling_live_transcribe() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .live_transcribe("call-1", json!({"language": "en-US"}))
        .expect("live_transcribe");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.live_transcribe")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("language")
            .and_then(Value::as_str),
        Some("en-US")
    );
}

#[test]
fn test_calling_live_translate() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .live_translate(
            "call-1",
            json!({"source_language": "en", "target_language": "es"}),
        )
        .expect("live_translate");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.live_translate")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    let params = params_from_body(body_obj);
    assert_eq!(
        params.get("source_language").and_then(Value::as_str),
        Some("en")
    );
    assert_eq!(
        params.get("target_language").and_then(Value::as_str),
        Some("es")
    );
}

// ---------------------------------------------------------------------------
// Fax stop
// ---------------------------------------------------------------------------

#[test]
fn test_calling_send_fax_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .send_fax_stop("call-1", json!({}))
        .expect("send_fax_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.send_fax.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

#[test]
fn test_calling_receive_fax_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .receive_fax_stop("call-1", json!({}))
        .expect("receive_fax_stop");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.receive_fax.stop")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
}

// ---------------------------------------------------------------------------
// SIP refer + custom user_event
// ---------------------------------------------------------------------------

#[test]
fn test_calling_refer() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .refer("call-1", json!({"to": "sip:other@example.com"}))
        .expect("refer");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.refer")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    assert_eq!(
        params_from_body(body_obj)
            .get("to")
            .and_then(Value::as_str),
        Some("sip:other@example.com")
    );
}

#[test]
fn test_calling_user_event() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .user_event(
            "call-1",
            json!({"event_name": "my-event", "payload": {"foo": "bar"}}),
        )
        .expect("user_event");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    let body_obj = entry.body_object().unwrap();
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("calling.user_event")
    );
    assert_eq!(body_obj.get("id").and_then(Value::as_str), Some("call-1"));
    let params = params_from_body(body_obj);
    assert_eq!(
        params.get("event_name").and_then(Value::as_str),
        Some("my-event")
    );
    let payload = params
        .get("payload")
        .and_then(Value::as_object)
        .expect("payload object");
    assert_eq!(payload.get("foo").and_then(Value::as_str), Some("bar"));
}
