// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_event_dispatch_mock.py.
//
// Edge cases in the SDK's recv loop and event router that don't fit
// neatly into per-action / per-call test files: sub-command journaling,
// unknown event types, bad call IDs, multi-action concurrency, event
// ACK round-trips, ping handling, and authorization-state events.

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

fn bare_event_frame(event_type: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": format!("evt-{}-{}", event_type, params.get("call_id").and_then(Value::as_str).unwrap_or("x")),
        "method": "signalwire.event",
        "params": {"event_type": event_type, "params": params},
    })
}

fn send_action_frame(
    client: &Arc<signalwire::relay::Client>,
    call: &Arc<signalwire::relay::Call>,
    method: &str,
    control_id: &str,
    extra: Value,
) {
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
    let frame = json!({
        "jsonrpc": "2.0",
        "id": format!("rpc-{}-{}", method, control_id),
        "method": method,
        "params": params,
    });
    client.send(&frame);
}

// ---------------------------------------------------------------------------
// Sub-command journaling
// ---------------------------------------------------------------------------

#[test]
fn test_record_pause_journals_record_pause() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-rec-pa");
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "ec-rec-pa-1",
        json!({"record": {"audio": {"format": "wav"}}}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.record.pause",
        "ec-rec-pa-1",
        json!({"behavior": "continuous"}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let pauses = relay_mocktest::journal_recv(Some("calling.record.pause"));
    assert!(!pauses.is_empty());
    let p = pauses.last().unwrap().inner_params();
    assert_eq!(p.get("control_id").and_then(Value::as_str), Some("ec-rec-pa-1"));
    assert_eq!(p.get("behavior").and_then(Value::as_str), Some("continuous"));
    client.disconnect();
}

#[test]
fn test_record_resume_journals_record_resume() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-rec-re");
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "ec-rec-re-1",
        json!({"record": {"audio": {"format": "wav"}}}),
    );
    send_action_frame(&client, &call, "calling.record.resume", "ec-rec-re-1", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(150));
    let resumes = relay_mocktest::journal_recv(Some("calling.record.resume"));
    assert!(!resumes.is_empty());
    assert_eq!(
        resumes
            .last()
            .unwrap()
            .inner_params()
            .get("control_id")
            .and_then(Value::as_str),
        Some("ec-rec-re-1")
    );
    client.disconnect();
}

#[test]
fn test_collect_start_input_timers_journals_correctly() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-col-sit");
    send_action_frame(
        &client,
        &call,
        "calling.collect",
        "ec-col-sit-1",
        json!({"digits": {"max": 4}, "start_input_timers": false}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.collect.start_input_timers",
        "ec-col-sit-1",
        json!({}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let starts = relay_mocktest::journal_recv(Some("calling.collect.start_input_timers"));
    assert!(!starts.is_empty());
    assert_eq!(
        starts
            .last()
            .unwrap()
            .inner_params()
            .get("control_id")
            .and_then(Value::as_str),
        Some("ec-col-sit-1")
    );
    client.disconnect();
}

#[test]
fn test_play_volume_carries_negative_value() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-pvol");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "ec-pvol-1",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.play.volume",
        "ec-pvol-1",
        json!({"volume": -5.5}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));
    let vol = relay_mocktest::journal_recv(Some("calling.play.volume"));
    assert!(!vol.is_empty());
    let v = vol.last().unwrap().inner_params().get("volume").and_then(Value::as_f64);
    assert_eq!(v, Some(-5.5));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Unknown event types — recv loop survives
// ---------------------------------------------------------------------------

#[test]
fn test_unknown_event_type_does_not_crash() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::push(bare_event_frame("nonsense.unknown", json!({"foo": "bar"})));
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(client.is_connected());
    client.disconnect();
}

#[test]
fn test_event_with_bad_call_id_is_dropped() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::push(bare_event_frame(
        "calling.call.play",
        json!({
            "call_id": "no-such-call-bogus",
            "control_id": "stranger",
            "state": "playing",
        }),
    ));
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(client.is_connected());
    client.disconnect();
}

#[test]
fn test_event_with_empty_event_type_is_dropped() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::push(bare_event_frame("", json!({"call_id": "x"})));
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(client.is_connected());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Multi-action concurrency: 3 actions on one call (assert journaling + state)
// ---------------------------------------------------------------------------

#[test]
fn test_three_concurrent_actions_resolve_independently() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-3acts");
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "3a-p1",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.play",
        "3a-p2",
        json!({"play": [{"type": "silence", "params": {"duration": 60}}]}),
    );
    send_action_frame(
        &client,
        &call,
        "calling.record",
        "3a-r1",
        json!({"record": {"audio": {"format": "wav"}}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(150));

    // All three frames must be in the journal under their own control_id.
    let plays = relay_mocktest::journal_recv(Some("calling.play"));
    let recs = relay_mocktest::journal_recv(Some("calling.record"));
    assert_eq!(plays.len(), 2);
    assert_eq!(recs.len(), 1);
    let play_control_ids: Vec<&str> = plays
        .iter()
        .filter_map(|e| e.inner_params().get("control_id").and_then(Value::as_str))
        .collect();
    assert!(play_control_ids.contains(&"3a-p1"));
    assert!(play_control_ids.contains(&"3a-p2"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Event ACK round-trip — server-pushed events get ack frames back
// ---------------------------------------------------------------------------

#[test]
fn test_event_ack_sent_back_to_server() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let evt_id = "evt-ack-test-1";
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": evt_id,
        "method": "signalwire.event",
        "params": {
            "event_type": "calling.call.play",
            "params": {
                "call_id": "anything",
                "control_id": "x",
                "state": "playing",
            },
        },
    }));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let j = relay_mocktest::journal_all();
    let acks: Vec<_> = j
        .iter()
        .filter(|e| {
            e.direction == "recv"
                && e.frame.get("id").and_then(Value::as_str) == Some(evt_id)
                && e.frame.get("result").is_some()
        })
        .collect();
    assert!(
        !acks.is_empty(),
        "no event ACK with id={evt_id:?} found in journal"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Server ping handling
// ---------------------------------------------------------------------------

#[test]
fn test_server_ping_acked_by_sdk() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let ping_id = "ping-test-1";
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": ping_id,
        "method": "signalwire.ping",
        "params": {},
    }));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let j = relay_mocktest::journal_all();
    let pongs: Vec<_> = j
        .iter()
        .filter(|e| {
            e.direction == "recv"
                && e.frame.get("id").and_then(Value::as_str) == Some(ping_id)
                && e.frame.get("result").is_some()
        })
        .collect();
    assert!(
        !pongs.is_empty(),
        "SDK did not respond to ping with id={ping_id:?}"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Authorization state — captured for reconnect
// ---------------------------------------------------------------------------

#[test]
fn test_authorization_state_event_captured() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::push(bare_event_frame(
        "signalwire.authorization.state",
        json!({"authorization_state": "test-auth-state-blob"}),
    ));
    assert!(wait_until(2000, || client
        .authorization_state
        .lock()
        .unwrap()
        .as_deref()
        == Some("test-auth-state-blob")));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Calling.error event — does not raise into the SDK
// ---------------------------------------------------------------------------

#[test]
fn test_calling_error_event_does_not_crash() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::push(bare_event_frame(
        "calling.error",
        json!({"code": "5001", "message": "synthetic error"}),
    ));
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(client.is_connected());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// State event for an answered call updates Call.state
// ---------------------------------------------------------------------------

#[test]
fn test_call_state_event_updates_state() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-stt");
    relay_mocktest::push(bare_event_frame(
        "calling.call.state",
        json!({"call_id": "ec-stt", "state": "ending", "direction": "inbound"}),
    ));
    assert!(wait_until(2000, || call.current_state() == "ending"));
    client.disconnect();
}

#[test]
fn test_call_listener_fires_on_event() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-list");
    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let f2 = fired.clone();
    call.on(move |event, _call| {
        if event.event_type() == "calling.call.play" {
            *f2.lock().unwrap() = true;
        }
    });
    relay_mocktest::push(bare_event_frame(
        "calling.call.play",
        json!({"call_id": "ec-list", "control_id": "x", "state": "playing"}),
    ));
    assert!(wait_until(2000, || *fired.lock().unwrap()));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Tag-based dial routing — call.call_id nested
// ---------------------------------------------------------------------------

#[test]
fn test_dial_event_routes_via_tag_when_no_top_level_call_id() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "ec-tag-route",
        "winner_call_id": "WINTAG",
        "states": ["created", "answered"],
        "node_id": "n",
        "device": {"type": "phone", "params": {}},
    }));
    let call = client
        .dial_blocking(
            json!([[{"type": "phone", "params": {"to_number": "+1", "from_number": "+2"}}]]),
            Some("ec-tag-route"),
            None,
            std::time::Duration::from_secs(5),
        )
        .expect("dial");
    assert_eq!(call.call_id.as_deref(), Some("WINTAG"));
    let sends = relay_mocktest::journal_send(Some("calling.call.dial"));
    assert!(!sends.is_empty(), "no calling.call.dial event in journal");
    let inner = sends.last().unwrap().event_params();
    // Top-level: tag, dial_state, call. NO call_id.
    assert!(inner.get("call_id").is_none());
    assert_eq!(
        inner
            .get("call")
            .and_then(|c| c.get("call_id"))
            .and_then(Value::as_str),
        Some("WINTAG")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Tier-3 typed state enum: Call::call_state() on a real dispatched
// calling.call.state event, and parity with the string accessor.
// ---------------------------------------------------------------------------

#[test]
fn test_call_state_typed_accessor_tracks_real_state_event() {
    use signalwire::relay::CallState;

    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "ec-stt-typed");

    // After answered_inbound_call forced state to "answered", the typed view
    // must agree with the string view.
    assert_eq!(call.current_state(), "answered");
    assert_eq!(call.call_state(), CallState::Answered);
    assert_eq!(call.call_state().as_str(), call.current_state());
    assert!(!call.call_state().is_terminal());

    // Drive a REAL calling.call.state event through the mock → SDK recv loop.
    relay_mocktest::push(bare_event_frame(
        "calling.call.state",
        json!({"call_id": "ec-stt-typed", "state": "ending", "direction": "inbound"}),
    ));
    assert!(wait_until(2000, || call.call_state() == CallState::Ending));
    // Typed and string accessors stay in lock-step, still non-terminal.
    assert_eq!(call.current_state(), "ending");
    assert_eq!(call.call_state().as_str(), "ending");
    assert!(!call.call_state().is_terminal());

    // Terminal transition: the enum reports terminal exactly when the call ends.
    relay_mocktest::push(bare_event_frame(
        "calling.call.state",
        json!({"call_id": "ec-stt-typed", "state": "ended", "direction": "inbound"}),
    ));
    assert!(wait_until(2000, || call.call_state().is_terminal()));
    assert_eq!(call.call_state(), CallState::Ended);
    assert_eq!(call.call_state().as_str(), call.current_state());

    client.disconnect();
}

// ---------------------------------------------------------------------------
// Tier-3 typed Device: build the dial matrix with the typed struct and prove
// (a) a real mock dial answers and (b) the journaled wire `devices` is
// byte-identical to the hand-written json! matrix.
// ---------------------------------------------------------------------------

#[test]
fn test_dial_with_typed_device_matches_handwritten_wire() {
    use signalwire::relay::Device;

    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "ec-typed-dev",
        "winner_call_id": "WINDEV",
        "states": ["created", "answered"],
        "node_id": "n",
        "device": {"type": "phone", "params": {}},
    }));

    // Typed device matrix — one parallel leg, one phone device.
    let devices = Device::matrix(&[&[Device::phone("+1", "+2")]]);
    // It must be byte-identical to the hand-written matrix the other dial
    // tests use, BEFORE we ever send it.
    let handwritten =
        json!([[{"type": "phone", "params": {"to_number": "+1", "from_number": "+2"}}]]);
    assert_eq!(
        serde_json::to_string(&devices).unwrap(),
        serde_json::to_string(&handwritten).unwrap(),
    );

    let call = client
        .dial_blocking(
            devices,
            Some("ec-typed-dev"),
            None,
            std::time::Duration::from_secs(5),
        )
        .expect("typed-device dial should answer");
    assert_eq!(call.call_id.as_deref(), Some("WINDEV"));

    // The dial RPC the SDK actually sent must carry exactly that device wire.
    let sent = relay_mocktest::journal_recv(Some("calling.dial"));
    assert!(!sent.is_empty(), "no calling.dial RPC in journal");
    let sent_devices = sent.last().unwrap().inner_params().get("devices").cloned();
    assert_eq!(
        sent_devices.map(|d| serde_json::to_string(&d).unwrap()),
        Some(serde_json::to_string(&handwritten).unwrap()),
    );

    client.disconnect();
}
