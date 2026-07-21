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

use serde_json::{Value, json};
use signalwire::rest::namespaces::generated::calling_resources_generated as calling_gen;

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
        .update(
            calling_gen::CallingUpdateRequest::new("call-1").extra("state", json!("hold")),
            None,
        )
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
        .dial(
            calling_gen::CallingDialRequest::new("", "+15551234567")
                .url("https://example.com/swml")
                .codecs(json!(["OPUS", "G729", "VP8", "PCMA"])),
            None,
        )
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
        .dial(
            calling_gen::CallingDialRequest::new("", "+15551234567")
                .url("https://example.com/swml")
                .codecs(json!("OPUS,G729,VP8,PCMA")),
            None,
        )
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
            calling_gen::CallingTransferRequest::new(
                json!({"destination": "+15551234567", "from_number": "+15559876543"}),
            ),
            None,
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
    // New generated API wraps the transfer payload under the required `dest`
    // param; the destination/from_number values are preserved inside it.
    let dest = params_from_body(body_obj)
        .get("dest")
        .and_then(Value::as_object)
        .expect("dest sub-object");
    assert_eq!(
        dest.get("destination").and_then(Value::as_str),
        Some("+15551234567")
    );
    assert_eq!(
        dest.get("from_number").and_then(Value::as_str),
        Some("+15559876543")
    );
}

#[test]
fn test_calling_disconnect() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .disconnect(
            "call-456",
            calling_gen::CallingDisconnectRequest::new().extra("reason", json!("busy")),
            None,
        )
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
        .play_pause(
            "call-1",
            calling_gen::CallingPlayPauseRequest::new("ctrl-1"),
            None,
        )
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
        .play_resume(
            "call-1",
            calling_gen::CallingPlayResumeRequest::new("ctrl-1"),
            None,
        )
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
        .play_stop(
            "call-1",
            calling_gen::CallingPlayStopRequest::new("ctrl-1"),
            None,
        )
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
        .play_volume(
            "call-1",
            calling_gen::CallingPlayVolumeRequest::new("ctrl-1", 2.5),
            None,
        )
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
        .record(
            "call-1",
            calling_gen::CallingRecordRequest::new().extra("record", json!({"format": "mp3"})),
            None,
        )
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
        .record_pause(
            "call-1",
            calling_gen::CallingRecordPauseRequest::new("rec-1"),
            None,
        )
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
        .record_resume(
            "call-1",
            calling_gen::CallingRecordResumeRequest::new("rec-1"),
            None,
        )
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
        .collect(
            "call-1",
            // The recorded wire value for initial_timeout is the integer 5; the
            // generated setter is f64, so pass it through `.extra` to keep the
            // JSON number an integer (matching the `as_i64` assertion below).
            calling_gen::CallingCollectRequest::new()
                .extra("initial_timeout", json!(5))
                .digits(json!({"max": 4})),
            None,
        )
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
        .collect_stop(
            "call-1",
            calling_gen::CallingCollectStopRequest::new("col-1"),
            None,
        )
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
        .collect_start_input_timers(
            "call-1",
            calling_gen::CallingCollectStartInputTimersRequest::new("col-1"),
            None,
        )
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
        .detect(
            "call-1",
            calling_gen::CallingDetectRequest::new(json!({"type": "machine", "params": {}})),
            None,
        )
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
    assert_eq!(detect.get("type").and_then(Value::as_str), Some("machine"));
}

#[test]
fn test_calling_detect_stop() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .detect_stop(
            "call-1",
            calling_gen::CallingDetectStopRequest::new("det-1"),
            None,
        )
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
            calling_gen::CallingTapRequest::new(json!({"type": "audio"}), json!({"type": "rtp"})),
            None,
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
        .tap_stop(
            "call-1",
            calling_gen::CallingTapStopRequest::new("tap-1"),
            None,
        )
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
        .stream(
            "call-1",
            calling_gen::CallingStreamRequest::new("wss://example.com/audio"),
            None,
        )
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
        .stream_stop(
            "call-1",
            calling_gen::CallingStreamStopRequest::new("stream-1"),
            None,
        )
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
    let body = c
        .calling()
        .denoise("call-1", calling_gen::CallingDenoiseRequest::new(), None)
        .expect("denoise");
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
        .denoise_stop(
            "call-1",
            calling_gen::CallingDenoiseStopRequest::new(),
            None,
        )
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
    // denoise_stop takes NO command params (spec CallDenoiseStopRequest.params is
    // empty; the Python oracle's denoise_stop is `(call_id, *, extras)` with an
    // empty `params: {}` — see calling_resources_generated.py:662). The prior
    // assertion of `params.control_id == "dn-1"` was a hand-test defect: Python's
    // own test_denoise_stop asserts only command + id and sends no control_id
    // (#33). control_id is required only on the *pause/resume/stop/volume* media
    // commands (play_stop/record_pause/…), never on denoise_stop.
    assert!(
        params_from_body(body_obj).get("control_id").is_none(),
        "denoise_stop must not send a control_id (parity with the Python oracle)"
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
            calling_gen::CallingTranscribeRequest::new()
                .extra("language", json!("en-US"))
                .extra("transcribe", json!({"engine": "google"})),
            None,
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
        .transcribe_stop(
            "call-1",
            calling_gen::CallingTranscribeStopRequest::new("tr-1"),
            None,
        )
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
    let body = c
        .calling()
        .ai_hold("call-1", calling_gen::CallingAiHoldRequest::new(), None)
        .expect("ai_hold");
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
    let body = c
        .calling()
        .ai_unhold("call-1", calling_gen::CallingAiUnholdRequest::new(), None)
        .expect("ai_unhold");
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
    // ai_stop now requires a control_id in the generated request; the original
    // test sent an empty body and asserts only command + id, so pass "".
    let body = c
        .calling()
        .ai_stop("call-1", calling_gen::CallingAiStopRequest::new(""), None)
        .expect("ai_stop");
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
        .live_transcribe(
            "call-1",
            calling_gen::CallingLiveTranscribeRequest::new(json!({"language": "en-US"})),
            None,
        )
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
    // New generated API wraps the payload under the required `action` param.
    let action = params_from_body(body_obj)
        .get("action")
        .and_then(Value::as_object)
        .expect("action sub-object");
    assert_eq!(
        action.get("language").and_then(Value::as_str),
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
            calling_gen::CallingLiveTranslateRequest::new(
                json!({"source_language": "en", "target_language": "es"}),
            ),
            None,
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
    // New generated API wraps the payload under the required `action` param.
    let action = params_from_body(body_obj)
        .get("action")
        .and_then(Value::as_object)
        .expect("action sub-object");
    assert_eq!(
        action.get("source_language").and_then(Value::as_str),
        Some("en")
    );
    assert_eq!(
        action.get("target_language").and_then(Value::as_str),
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
        .send_fax_stop(
            "call-1",
            calling_gen::CallingSendFaxStopRequest::new(""),
            None,
        )
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
        .receive_fax_stop(
            "call-1",
            calling_gen::CallingReceiveFaxStopRequest::new(""),
            None,
        )
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
        .refer(
            "call-1",
            calling_gen::CallingReferRequest::new(json!({"to": "sip:other@example.com"})),
            None,
        )
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
    // New generated API wraps the payload under the required `device` param.
    let device = params_from_body(body_obj)
        .get("device")
        .and_then(Value::as_object)
        .expect("device sub-object");
    assert_eq!(
        device.get("to").and_then(Value::as_str),
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
            calling_gen::CallingUserEventRequest::new(
                json!({"event_name": "my-event", "payload": {"foo": "bar"}}),
            ),
            None,
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
    // New generated API wraps the payload under the required `event` param.
    let event = params_from_body(body_obj)
        .get("event")
        .and_then(Value::as_object)
        .expect("event sub-object");
    assert_eq!(
        event.get("event_name").and_then(Value::as_str),
        Some("my-event")
    );
    let payload = event
        .get("payload")
        .and_then(Value::as_object)
        .expect("payload object");
    assert_eq!(payload.get("foo").and_then(Value::as_str), Some("bar"));
}
