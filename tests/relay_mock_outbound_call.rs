// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_outbound_call_mock.py.
//
// `calling.dial` returns a plain 200 with no call_id; the call info
// arrives via subsequent `calling.call.state` (per leg) and
// `calling.call.dial` (with the winner) events keyed by `tag`. We use the
// mock's `/__mock__/scenarios/dial` endpoint to script the dance.

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use common::relay_mocktest;

fn phone_device(to: &str, frm: &str) -> Value {
    json!({"type": "phone", "params": {"to_number": to, "from_number": frm}})
}

fn default_device() -> Value {
    phone_device("+15551112222", "+15553334444")
}

// ---------------------------------------------------------------------------
// Happy-path dial
// ---------------------------------------------------------------------------

#[test]
fn test_dial_resolves_to_call_with_winner_id() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-happy",
        "winner_call_id": "winner-1",
        "states": ["created", "ringing", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
        "delay_ms": 1,
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-happy"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial should resolve");
    assert_eq!(call.call_id.as_deref(), Some("winner-1"));
    assert_eq!(call.tag.as_deref(), Some("t-happy"));
    // The dial event sets state to "answered" by handle_dial_event's
    // dial_state coercion.
    assert_eq!(call.current_state(), "answered");
    client.disconnect();
}

#[test]
fn test_dial_journal_records_calling_dial_frame() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-frame",
        "winner_call_id": "winner-frame",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let _call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-frame"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial should resolve");
    let entry = relay_mocktest::journal_recv(Some("calling.dial"))
        .into_iter()
        .next()
        .expect("expected one calling.dial frame");
    let p = entry.inner_params();
    assert_eq!(p.get("tag").and_then(Value::as_str), Some("t-frame"));
    let devices = p.get("devices").and_then(Value::as_array);
    assert!(devices.is_some(), "devices should be array");
    let inner_first = &devices.unwrap()[0][0];
    assert_eq!(inner_first.get("type").and_then(Value::as_str), Some("phone"));
    client.disconnect();
}

#[test]
fn test_dial_with_max_duration_in_frame() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-md",
        "winner_call_id": "winner-md",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let _ = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-md"),
            Some(300),
            Duration::from_secs(5),
        )
        .expect("dial");
    let entry = relay_mocktest::journal_recv(Some("calling.dial"))
        .into_iter()
        .next()
        .expect("expected calling.dial frame");
    assert_eq!(
        entry.inner_params().get("max_duration").and_then(Value::as_u64),
        Some(300)
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Failure paths
// ---------------------------------------------------------------------------

#[test]
fn test_dial_failed_raises_relay_error() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    // We can't easily script a failed dial via the mock's arm_dial
    // (which only emits success). Push the failed event from a thread
    // after a small delay.
    let h = relay_mocktest::harness();
    let pusher_url = format!("{}/__mock__/push", h.http_url);
    let join = std::thread::spawn(move || {
        // Wait briefly so the SDK's pending dial future is registered.
        std::thread::sleep(Duration::from_millis(150));
        let frame = json!({
            "frame": {
                "jsonrpc": "2.0",
                "id": "fail-evt-1",
                "method": "signalwire.event",
                "params": {
                    "event_type": "calling.call.dial",
                    "params": {
                        "tag": "t-fail",
                        "node_id": "node-mock-1",
                        "dial_state": "failed",
                        "call": {},
                    },
                },
            },
        });
        let _ = ureq::post(&pusher_url).send_json(&frame);
    });

    // Use a short dial timeout so we don't wait forever on test failure.
    let result = client.dial_blocking(
        json!([[default_device()]]),
        Some("t-fail"),
        None,
        Duration::from_secs(2),
    );
    let _ = join.join();
    // `Arc<Call>` is not Debug, so match the result directly rather than
    // unwrapping the Ok side.
    match result {
        Ok(_) => panic!("dial with failed event should error"),
        Err(err) => {
            assert!(
                matches!(err, signalwire::relay::RelayError::DialFailed { .. }),
                "expected DialFailed, got {err:?}"
            );
            let msg = err.to_string();
            assert!(
                msg.contains("dial failed")
                    && (msg.contains("timed out") || msg.contains("failed")),
                "unexpected dial error: {msg}"
            );
        }
    }
    client.disconnect();
}

#[test]
fn test_dial_timeout_when_no_dial_event() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    // Don't arm any dial scenario — the SDK should time out cleanly.
    let result = client.dial_blocking(
        json!([[default_device()]]),
        Some("t-timeout"),
        None,
        Duration::from_millis(500),
    );
    match result {
        Ok(_) => panic!("dial with no event should time out"),
        Err(err) => {
            assert!(
                matches!(err, signalwire::relay::RelayError::DialFailed { .. }),
                "expected DialFailed, got {err:?}"
            );
            assert!(err.to_string().contains("timed out"));
        }
    }
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Parallel dial — winner + losers
// ---------------------------------------------------------------------------

#[test]
fn test_dial_winner_carries_dial_winner_true() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-winner",
        "winner_call_id": "WIN-ID",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
        "losers": [
            {"call_id": "LOSE-A", "states": ["created", "ended"]},
            {"call_id": "LOSE-B", "states": ["created", "ended"]},
        ],
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-winner"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    assert_eq!(call.call_id.as_deref(), Some("WIN-ID"));

    let sends = relay_mocktest::journal_send(Some("calling.call.dial"));
    assert!(!sends.is_empty(), "no calling.call.dial event was pushed");
    let answered: Vec<_> = sends
        .iter()
        .filter(|e| {
            e.event_params()
                .get("dial_state")
                .and_then(Value::as_str)
                == Some("answered")
        })
        .collect();
    assert_eq!(answered.len(), 1, "expected exactly one answered dial event");
    let final_evt = answered[0];
    let inner = final_evt.event_params();
    assert_eq!(
        inner.get("call").and_then(|c| c.get("dial_winner")).and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        inner.get("call").and_then(|c| c.get("call_id")).and_then(Value::as_str),
        Some("WIN-ID")
    );
    client.disconnect();
}

#[test]
fn test_dial_losers_get_state_events() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-losers",
        "winner_call_id": "WIN-2",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
        "losers": [
            {"call_id": "L1", "states": ["created", "ended"]},
        ],
    }));
    let _ = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-losers"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    let state_events = relay_mocktest::journal_send(Some("calling.call.state"));
    let l1_states: Vec<&str> = state_events
        .iter()
        .filter(|e| {
            e.event_params().get("call_id").and_then(Value::as_str) == Some("L1")
        })
        .filter_map(|e| {
            e.event_params()
                .get("call_state")
                .and_then(Value::as_str)
        })
        .collect();
    assert!(
        l1_states.contains(&"ended"),
        "loser L1 never reached 'ended'; saw: {l1_states:?}"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Devices shape on the wire
// ---------------------------------------------------------------------------

#[test]
fn test_dial_devices_serial_two_legs_on_wire() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-serial",
        "winner_call_id": "WIN-SER",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let devs = json!([[
        phone_device("+15551110001", "+15553334444"),
        phone_device("+15551110002", "+15553334444"),
    ]]);
    let _ = client
        .dial_blocking(devs, Some("t-serial"), None, Duration::from_secs(5))
        .expect("dial");
    let entry = relay_mocktest::journal_recv(Some("calling.dial"))
        .into_iter()
        .next()
        .expect("calling.dial frame");
    let devs_arr = entry.inner_params().get("devices").and_then(Value::as_array).unwrap();
    assert_eq!(devs_arr.len(), 1);
    assert_eq!(devs_arr[0].as_array().unwrap().len(), 2);
    let first_to = devs_arr[0][0]
        .get("params")
        .and_then(|p| p.get("to_number"))
        .and_then(Value::as_str);
    assert_eq!(first_to, Some("+15551110001"));
    client.disconnect();
}

#[test]
fn test_dial_devices_parallel_two_legs_on_wire() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-par",
        "winner_call_id": "WIN-PAR",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let devs = json!([
        [phone_device("+15551110001", "+15553334444")],
        [phone_device("+15551110002", "+15553334444")],
    ]);
    let _ = client
        .dial_blocking(devs, Some("t-par"), None, Duration::from_secs(5))
        .expect("dial");
    let entry = relay_mocktest::journal_recv(Some("calling.dial"))
        .into_iter()
        .next()
        .expect("calling.dial frame");
    let devs_arr = entry.inner_params().get("devices").and_then(Value::as_array).unwrap();
    assert_eq!(devs_arr.len(), 2);
    client.disconnect();
}

// ---------------------------------------------------------------------------
// State transitions during dial
// ---------------------------------------------------------------------------

#[test]
fn test_dial_records_call_state_progression_on_winner() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-prog",
        "winner_call_id": "WIN-PROG",
        "states": ["created", "ringing", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-prog"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    let state_events = relay_mocktest::journal_send(Some("calling.call.state"));
    let winner_states: Vec<&str> = state_events
        .iter()
        .filter(|e| {
            e.event_params().get("call_id").and_then(Value::as_str) == Some("WIN-PROG")
        })
        .filter_map(|e| e.event_params().get("call_state").and_then(Value::as_str))
        .collect();
    assert!(winner_states.contains(&"created"));
    assert!(winner_states.contains(&"ringing"));
    assert!(winner_states.contains(&"answered"));
    assert_eq!(call.current_state(), "answered");
    client.disconnect();
}

// ---------------------------------------------------------------------------
// After dial — call object is usable
// ---------------------------------------------------------------------------

#[test]
fn test_dialed_call_can_send_subsequent_command() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-after",
        "winner_call_id": "WIN-AFTER",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-after"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");

    // Send a hangup directly through the wire (Call::hangup() is a
    // pure in-memory recorder, not yet wired through the live client).
    let frame = json!({
        "jsonrpc": "2.0",
        "id": "hangup-1",
        "method": "calling.end",
        "params": {
            "call_id": call.call_id.clone().unwrap_or_default(),
            "node_id": call.node_id.clone().unwrap_or_default(),
        },
    });
    client.send(&frame);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let ends = relay_mocktest::journal_recv(Some("calling.end"));
    assert!(!ends.is_empty(), "no calling.end frame in journal");
    assert_eq!(
        ends.last()
            .unwrap()
            .inner_params()
            .get("call_id")
            .and_then(Value::as_str),
        Some("WIN-AFTER")
    );
    client.disconnect();
}

#[test]
fn test_dialed_call_can_play() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-play",
        "winner_call_id": "WIN-PLAY",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-play"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    // Send a play through the wire (the mock will synthesize a 200).
    let frame = json!({
        "jsonrpc": "2.0",
        "id": "play-1",
        "method": "calling.play",
        "params": {
            "call_id": call.call_id.clone().unwrap_or_default(),
            "node_id": call.node_id.clone().unwrap_or_default(),
            "control_id": "play-after-dial",
            "play": [{"type": "tts", "params": {"text": "hi"}}],
        },
    });
    client.send(&frame);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let plays = relay_mocktest::journal_recv(Some("calling.play"));
    assert!(!plays.is_empty(), "no calling.play frame in journal");
    let p = plays.last().unwrap().inner_params();
    assert_eq!(
        p.get("call_id").and_then(Value::as_str),
        Some("WIN-PLAY")
    );
    let play_arr = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(play_arr[0].get("type").and_then(Value::as_str), Some("tts"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Tag preservation
// ---------------------------------------------------------------------------

#[test]
fn test_dial_preserves_explicit_tag() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "my-very-explicit-tag-99",
        "winner_call_id": "WIN-T",
        "states": ["created", "answered"],
        "node_id": "node-mock-1",
        "device": default_device(),
    }));
    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("my-very-explicit-tag-99"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    assert_eq!(call.tag.as_deref(), Some("my-very-explicit-tag-99"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// JSON-RPC envelope
// ---------------------------------------------------------------------------

#[test]
fn test_dial_uses_jsonrpc_2_0() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    relay_mocktest::arm_dial(json!({
        "tag": "t-rpc",
        "winner_call_id": "W",
        "states": ["created", "answered"],
        "node_id": "n",
        "device": default_device(),
    }));
    let _ = client
        .dial_blocking(
            json!([[default_device()]]),
            Some("t-rpc"),
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    let entry = relay_mocktest::journal_recv(Some("calling.dial"))
        .into_iter()
        .next()
        .expect("calling.dial frame");
    assert_eq!(
        entry.frame.get("jsonrpc").and_then(Value::as_str),
        Some("2.0")
    );
    assert_eq!(
        entry.frame.get("method").and_then(Value::as_str),
        Some("calling.dial")
    );
    assert!(entry.frame.get("id").is_some());
    assert!(entry.frame.get("params").is_some());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Auto-tag generation
// ---------------------------------------------------------------------------

#[test]
fn test_dial_auto_generates_uuid_tag_when_omitted() {
    let _g = relay_mocktest::begin();
    let client: Arc<signalwire::relay::Client> = relay_mocktest::connected_client(&["default"]);
    // We need to learn the auto-generated tag, then push the answered
    // event for it manually.
    let h = relay_mocktest::harness();
    let push_url = format!("{}/__mock__/push", h.http_url);

    let join = std::thread::spawn(move || {
        // Poll the journal until calling.dial appears, then read its tag.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut tag = None;
        while std::time::Instant::now() < deadline {
            let entries = relay_mocktest::journal_recv(Some("calling.dial"));
            if let Some(e) = entries.into_iter().next() {
                tag = e
                    .inner_params()
                    .get("tag")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if tag.is_some() {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let t = tag.expect("expected calling.dial tag in journal");
        // Push a dial-answered event keyed on that tag.
        let frame = json!({
            "frame": {
                "jsonrpc": "2.0",
                "id": "auto-evt-1",
                "method": "signalwire.event",
                "params": {
                    "event_type": "calling.call.dial",
                    "params": {
                        "tag": t.clone(),
                        "node_id": "node-mock-1",
                        "dial_state": "answered",
                        "call": {
                            "call_id": "auto-tag-winner",
                            "node_id": "node-mock-1",
                            "tag": t.clone(),
                            "device": {"type": "phone", "params": {}},
                            "dial_winner": true,
                        },
                    },
                },
            },
        });
        let _ = ureq::post(&push_url).send_json(&frame);
        t
    });

    let call = client
        .dial_blocking(
            json!([[default_device()]]),
            None,
            None,
            Duration::from_secs(5),
        )
        .expect("dial");
    let auto_tag = join.join().unwrap();
    assert_eq!(call.call_id.as_deref(), Some("auto-tag-winner"));
    let uuid_re = regex::Regex::new(
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
    )
    .unwrap();
    assert!(
        uuid_re.is_match(&auto_tag),
        "expected UUID-shaped tag, got {auto_tag:?}"
    );
    assert_eq!(call.tag.as_deref(), Some(auto_tag.as_str()));
    client.disconnect();
}
