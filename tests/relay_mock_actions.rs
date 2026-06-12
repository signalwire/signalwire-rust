// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_actions_mock.py.
//
// We drive each calling action over the wire (rather than relying on
// Call::play() which only records to memory), then assert on the
// journaled `calling.<verb>` and follow-up sub-command frames.

// Test helpers take `Value` by value to match the mock-test helper style
// (payloads flow in by value, as in the Python sibling tests).
#![allow(clippy::needless_pass_by_value)]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use common::relay_mocktest;

fn wait_until<F: Fn() -> bool>(budget_ms: u64, f: F) -> bool {
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_millis(budget_ms);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn answered_inbound_call(
    client: &Arc<signalwire::relay::Client>,
    call_id: &str,
) -> Arc<signalwire::relay::Call> {
    let captured: Arc<Mutex<Option<Arc<signalwire::relay::Call>>>> =
        Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    let client2 = client.clone();
    client.on_call(move |call, _ev| {
        let id = call.call_id.clone().unwrap_or_default();
        let frame = json!({
            "jsonrpc": "2.0",
            "id": format!("ans-{}", id),
            "method": "calling.answer",
            "params": {"call_id": id, "node_id": call.node_id.clone().unwrap_or_default()},
        });
        client2.send(&frame);
        *cap2.lock().unwrap() = Some(call);
    });
    relay_mocktest::inbound_call(json!({
        "call_id": call_id,
        "auto_states": ["created"],
    }));
    let cid = call_id.to_string();
    assert!(
        wait_until(3000, || captured.lock().unwrap().is_some()),
        "on_call did not fire for {cid}"
    );
    let call = captured.lock().unwrap().clone().unwrap();
    *call.state.lock().unwrap() = "answered".to_string();
    call
}

fn send_action_frame(
    client: &Arc<signalwire::relay::Client>,
    call: &Arc<signalwire::relay::Call>,
    method: &str,
    control_id: &str,
    extra: Value,
) -> String {
    let mut params = json!({
        "call_id": call.call_id.clone().unwrap_or_default(),
        "node_id": call.node_id.clone().unwrap_or_default(),
        "control_id": control_id,
    });
    if let (Some(obj), Some(extra_obj)) = (params.as_object_mut(), extra.as_object()) {
        for (k, v) in extra_obj {
            obj.insert(k.clone(), v.clone());
        }
    }
    let id = format!("rpc-{method}-{control_id}");
    let frame = json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "method": method,
        "params": params,
    });
    client.send(&frame);
    id
}

fn send_subcommand(
    client: &Arc<signalwire::relay::Client>,
    call: &Arc<signalwire::relay::Call>,
    method: &str,
    control_id: &str,
    extra: Value,
) {
    send_action_frame(client, call, method, control_id, extra);
}

// ---------------------------------------------------------------------------
// PlayAction
// ---------------------------------------------------------------------------

#[test]
fn test_play_journals_calling_play() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-play");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "play-ctl-1",
        json!({"play": [{"type": "tts", "params": {"text": "hi"}}]}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.play"))
        .into_iter()
        .next()
        .expect("expected one calling.play frame");
    let p = entry.inner_params();
    assert_eq!(p.get("call_id").and_then(Value::as_str), Some("call-play"));
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("play-ctl-1"));
    let play = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(play[0].get("type").and_then(Value::as_str), Some("tts"));
    client.disconnect();
}

#[test]
fn test_play_stop_journals_play_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-play-stop");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "play-ctl-stop",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_subcommand(&client, &call, "calling.play.stop", "play-ctl-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.play.stop"));
    assert!(!stops.is_empty(), "no calling.play.stop frame");
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("play-ctl-stop")
    );
    client.disconnect();
}

#[test]
fn test_play_pause_resume_volume_journal() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-play-prv");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "play-ctl-prv",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_subcommand(&client, &call, "calling.play.pause", "play-ctl-prv", json!({}));
    send_subcommand(&client, &call, "calling.play.resume", "play-ctl-prv", json!({}));
    send_subcommand(
        &client,
        &call,
        "calling.play.volume",
        "play-ctl-prv",
        json!({"volume": -3.0}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(!relay_mocktest::journal_recv(Some("calling.play.pause")).is_empty());
    assert!(!relay_mocktest::journal_recv(Some("calling.play.resume")).is_empty());
    let vol = relay_mocktest::journal_recv(Some("calling.play.volume"));
    assert!(!vol.is_empty());
    let v = vol.last().unwrap().inner_params().get("volume").and_then(Value::as_f64);
    assert_eq!(v, Some(-3.0));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RecordAction
// ---------------------------------------------------------------------------

#[test]
fn test_record_journals_calling_record() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-rec");
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "rec-ctl-1",
        json!({"record": {"audio": {"format": "mp3"}}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.record"))
        .into_iter()
        .next()
        .expect("expected calling.record frame");
    let p = entry.inner_params();
    assert_eq!(p.get("call_id").and_then(Value::as_str), Some("call-rec"));
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("rec-ctl-1"));
    assert_eq!(
        p.get("record")
            .and_then(|r| r.get("audio"))
            .and_then(|a| a.get("format"))
            .and_then(Value::as_str),
        Some("mp3")
    );
    client.disconnect();
}

#[test]
fn test_record_stop_journals_record_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-rec-stop");
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "rec-ctl-stop",
        json!({"record": {"audio": {"format": "wav"}}}),
    );
    send_subcommand(&client, &call, "calling.record.stop", "rec-ctl-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.record.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("rec-ctl-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// DetectAction
// ---------------------------------------------------------------------------

#[test]
fn test_detect_stop_journals_detect_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-det-stop");
    send_action_frame(
        &client,
        &call,
        "calling.detect",
        "det-stop",
        json!({"detect": {"type": "fax", "params": {}}}),
    );
    send_subcommand(&client, &call, "calling.detect.stop", "det-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.detect.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("det-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// CollectAction (play_and_collect)
// ---------------------------------------------------------------------------

#[test]
fn test_play_and_collect_journals_play_and_collect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-pac");
    send_action_frame(
        &client,
        &call,
        "calling.play_and_collect",
        "pac-ctl-1",
        json!({
            "play": [{"type": "tts", "params": {"text": "Press 1"}}],
            "collect": {"digits": {"max": 1}},
        }),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.play_and_collect"))
        .into_iter()
        .next()
        .expect("expected calling.play_and_collect frame");
    let p = entry.inner_params();
    assert_eq!(p.get("call_id").and_then(Value::as_str), Some("call-pac"));
    let play = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(play[0].get("type").and_then(Value::as_str), Some("tts"));
    assert_eq!(
        p.get("collect")
            .and_then(|c| c.get("digits"))
            .and_then(|d| d.get("max"))
            .and_then(Value::as_u64),
        Some(1)
    );
    client.disconnect();
}

#[test]
fn test_play_and_collect_stop_journals_pac_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-pac-stop");
    send_action_frame(
        &client,
        &call,
        "calling.play_and_collect",
        "pac-stop",
        json!({
            "play": [{"type": "silence", "params": {"duration": 1}}],
            "collect": {"digits": {"max": 1}},
        }),
    );
    send_subcommand(
        &client,
        &call,
        "calling.play_and_collect.stop",
        "pac-stop",
        json!({}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.play_and_collect.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("pac-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// StandaloneCollectAction
// ---------------------------------------------------------------------------

#[test]
fn test_collect_journals_calling_collect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-col");
    send_action_frame(
        &client,
        &call,
        "calling.collect",
        "col-ctl",
        json!({"digits": {"max": 4}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.collect"))
        .into_iter()
        .next()
        .expect("calling.collect frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("digits").and_then(|d| d.get("max")).and_then(Value::as_u64),
        Some(4)
    );
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("col-ctl"));
    client.disconnect();
}

#[test]
fn test_collect_stop_journals_collect_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-col-stop");
    send_action_frame(
        &client,
        &call,
        "calling.collect",
        "col-stop",
        json!({"digits": {"max": 4}}),
    );
    send_subcommand(&client, &call, "calling.collect.stop", "col-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.collect.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("col-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// PayAction
// ---------------------------------------------------------------------------

#[test]
fn test_pay_journals_calling_pay() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-pay");
    send_action_frame(
        &client,
        &call,
        "calling.pay",
        "pay-ctl",
        json!({
            "payment_connector_url": "https://pay.example/connect",
            "charge_amount": "9.99",
        }),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.pay"))
        .into_iter()
        .next()
        .expect("calling.pay frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("payment_connector_url").and_then(Value::as_str),
        Some("https://pay.example/connect")
    );
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("pay-ctl"));
    assert_eq!(p.get("charge_amount").and_then(Value::as_str), Some("9.99"));
    client.disconnect();
}

#[test]
fn test_pay_stop_journals_pay_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-pay-stop");
    send_action_frame(
        &client,
        &call,
        "calling.pay",
        "pay-stop",
        json!({"payment_connector_url": "https://pay.example/connect"}),
    );
    send_subcommand(&client, &call, "calling.pay.stop", "pay-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.pay.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("pay-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// FaxAction
// ---------------------------------------------------------------------------

#[test]
fn test_send_fax_journals_calling_send_fax() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-sfax");
    send_action_frame(
        &client,
        &call,
        "calling.send_fax",
        "sfax-ctl",
        json!({
            "document": "https://docs.example/test.pdf",
            "identity": "+15551112222",
        }),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.send_fax"))
        .into_iter()
        .next()
        .expect("calling.send_fax frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("document").and_then(Value::as_str),
        Some("https://docs.example/test.pdf")
    );
    assert_eq!(
        p.get("identity").and_then(Value::as_str),
        Some("+15551112222")
    );
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("sfax-ctl"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// TapAction
// ---------------------------------------------------------------------------

#[test]
fn test_tap_journals_calling_tap() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-tap");
    send_action_frame(
        &client,
        &call,
        "calling.tap",
        "tap-ctl",
        json!({
            "tap": {"type": "audio"},
            "device": {"type": "rtp", "params": {"addr": "203.0.113.1", "port": 4000}},
        }),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.tap"))
        .into_iter()
        .next()
        .expect("calling.tap frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("tap").and_then(|t| t.get("type")).and_then(Value::as_str),
        Some("audio")
    );
    assert_eq!(
        p.get("device").and_then(|d| d.get("params")).and_then(|p| p.get("port")).and_then(Value::as_u64),
        Some(4000)
    );
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("tap-ctl"));
    client.disconnect();
}

#[test]
fn test_tap_stop_journals_tap_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-tap-stop");
    send_action_frame(
        &client,
        &call,
        "calling.tap",
        "tap-stop",
        json!({
            "tap": {"type": "audio"},
            "device": {"type": "rtp", "params": {"addr": "203.0.113.1", "port": 4000}},
        }),
    );
    send_subcommand(&client, &call, "calling.tap.stop", "tap-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.tap.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("tap-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// StreamAction
// ---------------------------------------------------------------------------

#[test]
fn test_stream_journals_calling_stream() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-strm");
    send_action_frame(
        &client,
        &call,
        "calling.stream",
        "strm-ctl",
        json!({
            "url": "wss://stream.example/audio",
            "codec": "OPUS@48000h",
        }),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.stream"))
        .into_iter()
        .next()
        .expect("calling.stream frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("url").and_then(Value::as_str),
        Some("wss://stream.example/audio")
    );
    assert_eq!(
        p.get("codec").and_then(Value::as_str),
        Some("OPUS@48000h")
    );
    assert_eq!(
        p.get("control_id").and_then(Value::as_str),
        Some("strm-ctl")
    );
    client.disconnect();
}

#[test]
fn test_stream_stop_journals_stream_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-strm-stop");
    send_action_frame(
        &client,
        &call,
        "calling.stream",
        "strm-stop",
        json!({"url": "wss://stream.example/audio"}),
    );
    send_subcommand(&client, &call, "calling.stream.stop", "strm-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.stream.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("strm-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// TranscribeAction
// ---------------------------------------------------------------------------

#[test]
fn test_transcribe_journals_calling_transcribe() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-tr");
    send_action_frame(&client, &call, "calling.transcribe", "tr-ctl", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.transcribe"))
        .into_iter()
        .next()
        .expect("calling.transcribe frame");
    assert_eq!(
        entry.inner_params().get("control_id").and_then(Value::as_str),
        Some("tr-ctl")
    );
    client.disconnect();
}

#[test]
fn test_transcribe_stop_journals_transcribe_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-tr-stop");
    send_action_frame(&client, &call, "calling.transcribe", "tr-stop", json!({}));
    send_subcommand(
        &client,
        &call,
        "calling.transcribe.stop",
        "tr-stop",
        json!({}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.transcribe.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("tr-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// AIAction
// ---------------------------------------------------------------------------

#[test]
fn test_ai_journals_calling_ai() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ai");
    send_action_frame(
        &client,
        &call,
        "calling.ai",
        "ai-ctl",
        json!({"prompt": {"text": "You are helpful."}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some("calling.ai"))
        .into_iter()
        .next()
        .expect("calling.ai frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("prompt")
            .and_then(|pr| pr.get("text"))
            .and_then(Value::as_str),
        Some("You are helpful.")
    );
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("ai-ctl"));
    client.disconnect();
}

#[test]
fn test_ai_stop_journals_ai_stop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ai-stop");
    send_action_frame(
        &client,
        &call,
        "calling.ai",
        "ai-stop",
        json!({"prompt": {"text": "You are helpful."}}),
    );
    send_subcommand(&client, &call, "calling.ai.stop", "ai-stop", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stops = relay_mocktest::journal_recv(Some("calling.ai.stop"));
    assert!(!stops.is_empty());
    assert_eq!(
        stops.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("ai-stop")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Multi-action concurrency
// ---------------------------------------------------------------------------

#[test]
fn test_concurrent_play_and_record_route_independently() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-multi");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "ctl-play-x",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "ctl-rec-y",
        json!({"record": {"audio": {"format": "wav"}}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let plays = relay_mocktest::journal_recv(Some("calling.play"));
    let recs = relay_mocktest::journal_recv(Some("calling.record"));
    assert!(!plays.is_empty());
    assert!(!recs.is_empty());
    assert_eq!(
        plays.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("ctl-play-x")
    );
    assert_eq!(
        recs.last().unwrap().inner_params().get("control_id").and_then(Value::as_str),
        Some("ctl-rec-y")
    );
    client.disconnect();
}
