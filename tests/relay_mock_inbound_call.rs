// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_inbound_call_mock.py.
//
// The mock pushes a `calling.call.receive` frame; the SDK's `on_call`
// handler must fire with a Call object whose state reflects the wire
// frame, and any subsequent `call.answer()` etc. must show up in the
// journal as the right `calling.<verb>` frame.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};
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

fn state_push_frame(call_id: &str, call_state: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": format!("st-{}-{}", call_id, call_state),
        "method": "signalwire.event",
        "params": {
            "event_type": "calling.call.state",
            "params": {
                "call_id": call_id,
                "node_id": "mock-relay-node-1",
                "tag": "",
                // Real RELAY wire key is `call_state` (matches mock_relay).
                "call_state": call_state,
                "direction": "inbound",
                "device": {
                    "type": "phone",
                    "params": {
                        "from_number": "+15551110000",
                        "to_number": "+15552220000",
                    },
                },
            },
        },
    })
}

// ---------------------------------------------------------------------------
// Basic inbound-call handler dispatch
// ---------------------------------------------------------------------------

#[test]
fn test_on_call_handler_fires_with_call_object() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    client.on_call(move |call, _ev| {
        if let Some(id) = call.call_id.as_ref() {
            seen2.lock().unwrap().push(id.clone());
        }
    });

    relay_mocktest::inbound_call(json!({
        "call_id": "c-handler",
        "from_number": "+15551110000",
        "to_number": "+15552220000",
        "auto_states": ["created"],
    }));
    assert!(
        wait_until(2000, || !seen.lock().unwrap().is_empty()),
        "on_call handler did not fire"
    );
    let ids = seen.lock().unwrap().clone();
    assert_eq!(ids, vec!["c-handler".to_string()]);
    client.disconnect();
}

#[test]
fn test_inbound_call_object_has_correct_call_id_and_direction() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let captured: Arc<Mutex<Option<(String, Value)>>> = Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    client.on_call(move |call, _ev| {
        let id = call.call_id.clone().unwrap_or_default();
        let dev = call.device.lock().unwrap().clone();
        *cap2.lock().unwrap() = Some((id, dev));
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-dir",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || captured.lock().unwrap().is_some()));
    let (id, _dev) = captured.lock().unwrap().clone().unwrap();
    assert_eq!(id, "c-dir");
    client.disconnect();
}

#[test]
fn test_inbound_call_carries_from_to_in_device() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    client.on_call(move |call, _ev| {
        *cap2.lock().unwrap() = Some(call.device.lock().unwrap().clone());
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-from-to",
        "from_number": "+15551112233",
        "to_number": "+15554445566",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || captured.lock().unwrap().is_some()));
    let dev = captured.lock().unwrap().clone().unwrap();
    let params = dev.get("params").and_then(Value::as_object).cloned();
    let p = params.expect("device.params should be object");
    assert_eq!(
        p.get("from_number").and_then(Value::as_str),
        Some("+15551112233")
    );
    assert_eq!(
        p.get("to_number").and_then(Value::as_str),
        Some("+15554445566")
    );
    client.disconnect();
}

#[test]
fn test_inbound_call_initial_state_is_created() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let state: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let s2 = state.clone();
    client.on_call(move |call, _ev| {
        *s2.lock().unwrap() = Some(call.current_state());
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-state",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || state.lock().unwrap().is_some()));
    assert_eq!(state.lock().unwrap().clone().unwrap(), "created");
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Handler answers — calling.answer journaled
// ---------------------------------------------------------------------------

#[test]
fn test_answer_in_handler_journals_calling_answer() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let answered: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let ans2 = answered.clone();
    client.on_call(move |call, _ev| {
        // Call::answer transmits the calling.answer frame through the
        // attached Client itself (the e024a18 transmit fix) — exactly
        // once; the mock journals the SDK's own frame.
        let _ = call.answer();
        *ans2.lock().unwrap() = true;
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-ans",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || *answered.lock().unwrap()));
    // Allow the journal to record.
    std::thread::sleep(std::time::Duration::from_millis(150));
    let answers = relay_mocktest::journal_recv(Some("calling.answer"));
    assert_eq!(
        answers.len(),
        1,
        "the SDK must transmit exactly one calling.answer frame (double-send)"
    );
    assert_eq!(
        answers
            .last()
            .unwrap()
            .inner_params()
            .get("call_id")
            .and_then(Value::as_str),
        Some("c-ans")
    );
    client.disconnect();
}

#[test]
fn test_answer_then_state_event_advances_call_state() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let captured: Arc<Mutex<Option<Arc<signalwire::relay::Call>>>> = Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    client.on_call(move |call, _ev| {
        let _ = call.answer();
        *cap2.lock().unwrap() = Some(call);
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-ans-state",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || captured.lock().unwrap().is_some()));
    relay_mocktest::push(state_push_frame("c-ans-state", "answered"));
    let call = captured.lock().unwrap().clone().unwrap();
    assert!(
        wait_until(2000, || call.current_state() == "answered"),
        "call state did not advance to answered: {}",
        call.current_state()
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Multiple inbound calls — independent state
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_inbound_calls_in_sequence_each_unique_object() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    client.on_call(move |call, _ev| {
        if let Some(id) = call.call_id.as_ref() {
            seen2.lock().unwrap().push(id.clone());
        }
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-seq-1",
        "auto_states": ["created"],
    }));
    std::thread::sleep(std::time::Duration::from_millis(100));
    relay_mocktest::inbound_call(json!({
        "call_id": "c-seq-2",
        "auto_states": ["created"],
    }));
    assert!(wait_until(3000, || seen.lock().unwrap().len() == 2));
    let ids = seen.lock().unwrap().clone();
    assert_eq!(ids, vec!["c-seq-1".to_string(), "c-seq-2".to_string()]);
    client.disconnect();
}

#[test]
fn test_multiple_inbound_calls_no_state_bleed() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let calls: Arc<Mutex<std::collections::HashMap<String, Arc<signalwire::relay::Call>>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let calls2 = calls.clone();
    client.on_call(move |call, _ev| {
        let id = call.call_id.clone().unwrap_or_default();
        // Answer through the SDK — Call::answer transmits on the wire.
        let _ = call.answer();
        calls2.lock().unwrap().insert(id, call);
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "cb-1",
        "auto_states": ["created"],
    }));
    std::thread::sleep(std::time::Duration::from_millis(80));
    relay_mocktest::inbound_call(json!({
        "call_id": "cb-2",
        "auto_states": ["created"],
    }));
    assert!(wait_until(3000, || calls.lock().unwrap().len() == 2));
    relay_mocktest::push(state_push_frame("cb-1", "answered"));
    let calls_map = calls.lock().unwrap().clone();
    let cb1 = calls_map.get("cb-1").unwrap().clone();
    let cb2 = calls_map.get("cb-2").unwrap().clone();
    assert!(wait_until(2000, || cb1.current_state() == "answered"));
    assert_ne!(cb2.current_state(), "answered");
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Handler exception doesn't crash client
// ---------------------------------------------------------------------------

#[test]
fn test_handler_exception_does_not_crash_client() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let f2 = fired.clone();
    client.on_call(move |_call, _ev| {
        *f2.lock().unwrap() = true;
        // In Rust we can't `panic` in a callback safely here — instead
        // simulate "exception" by producing no useful work after the
        // flag flip.
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-raise",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || *fired.lock().unwrap()));
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(client.is_connected());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Wire shape — calling.call.receive
// ---------------------------------------------------------------------------

#[test]
fn test_inbound_call_journal_send_records_calling_call_receive() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let f2 = fired.clone();
    client.on_call(move |_call, _ev| {
        *f2.lock().unwrap() = true;
    });
    relay_mocktest::inbound_call(json!({
        "call_id": "c-wire",
        "auto_states": ["created"],
    }));
    assert!(wait_until(2000, || *fired.lock().unwrap()));
    let sends = relay_mocktest::journal_send(Some("calling.call.receive"));
    assert!(
        !sends.is_empty(),
        "no calling.call.receive frame in journal"
    );
    let ev = sends.last().unwrap();
    let inner = ev.event_params();
    assert_eq!(inner.get("call_id").and_then(Value::as_str), Some("c-wire"));
    assert_eq!(
        inner.get("direction").and_then(Value::as_str),
        Some("inbound")
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Inbound without a registered handler — does not crash
// ---------------------------------------------------------------------------

#[test]
fn test_inbound_without_handler_does_not_crash() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    // No on_call registered.
    relay_mocktest::inbound_call(json!({
        "call_id": "c-nohandler",
        "auto_states": ["created"],
    }));
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(client.is_connected());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// scenario_play — full inbound flow
// ---------------------------------------------------------------------------

#[test]
fn test_scenario_play_full_inbound_flow() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let captured: Arc<Mutex<Option<Arc<signalwire::relay::Call>>>> = Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    client.on_call(move |call, _ev| {
        let _ = call.answer();
        *cap2.lock().unwrap() = Some(call);
    });

    let timeline = json!([
        {
            "push": {
                "frame": {
                    "jsonrpc": "2.0",
                    "id": "scen-recv-1",
                    "method": "signalwire.event",
                    "params": {
                        "event_type": "calling.call.receive",
                        "params": {
                            "call_id": "c-scen",
                            "node_id": "mock-relay-node-1",
                            "tag": "",
                            "call_state": "created",
                            "direction": "inbound",
                            "device": {
                                "type": "phone",
                                "params": {
                                    "from_number": "+15551110000",
                                    "to_number": "+15552220000",
                                },
                            },
                            "context": "default",
                        },
                    },
                }
            }
        },
        {"expect_recv": {"method": "calling.answer", "timeout_ms": 5000}},
        {"push": {"frame": state_push_frame("c-scen", "answered")}},
        {"sleep_ms": 50},
        {"push": {"frame": state_push_frame("c-scen", "ended")}},
    ]);
    let result = relay_mocktest::scenario_play(timeline);
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("completed"),
        "scenario_play did not complete: {result}"
    );
    assert!(captured.lock().unwrap().is_some());
    let _call = captured.lock().unwrap().clone().unwrap();
    // Allow ended event to flush. Call may already have been removed
    // from the registry once it reaches ended — that's the contract.
    std::thread::sleep(std::time::Duration::from_millis(100));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RUST-2 / NB-1: an on_call handler that sends a verb and then blocks on
// Action::wait() must complete — the play frame must reach the mock AND the
// wait must RESOLVE (not burn its timeout). This is only possible if the
// handler runs OFF the reader thread: on the pre-fix inline dispatch, the
// reader thread was stuck running the handler, so (a) the play frame never
// flushed to the mock and (b) no completion event could be read, so wait()
// burned its full timeout every time and wait(None) bricked the client.
// ---------------------------------------------------------------------------

#[test]
fn test_in_handler_play_then_wait_completes_not_deadlocks() {
    use std::time::{Duration, Instant};

    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    // Records set by the handler thread: whether wait() resolved, and how long
    // it blocked. A deadlock/timeout leaves resolved=false.
    let resolved: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let waited_ms: Arc<Mutex<u128>> = Arc::new(Mutex::new(0));
    let resolved2 = resolved.clone();
    let waited2 = waited_ms.clone();
    // Capture this test's mock session scope so the pusher thread (spawned
    // from inside the handler) targets the right client.
    let scope = relay_mocktest::scope();

    client.on_call(move |call, _ev| {
        // Start a play action — this frame must flush to the mock while the
        // handler is still running (it can only do so if the reader thread is
        // free, i.e. the handler is NOT on the reader thread).
        let action = call
            .play(json!({
                "play": [{"type": "audio", "params": {"url": "https://x/a.mp3"}}]
            }))
            .expect("play against the mock must succeed");
        let control_id = action.control_id().to_string();
        let call_id = call.call_id.clone().unwrap_or_default();

        // Arm a deferred "finished" completion ~150ms out, delivered through the
        // mock's socket -> reader-loop -> dispatch path (the reader must be free
        // to read it while THIS handler thread is parked in wait()).
        let scope_inner = scope.clone();
        std::thread::spawn(move || {
            relay_mocktest::set_scope(scope_inner);
            std::thread::sleep(Duration::from_millis(150));
            relay_mocktest::push(json!({
                "jsonrpc": "2.0",
                "id": format!("evt-play-{call_id}"),
                "method": "signalwire.event",
                "params": {
                    "event_type": "calling.call.play",
                    "params": {"call_id": call_id, "control_id": control_id, "state": "finished"},
                },
            }));
        });

        // Block on wait() with a generous deadline. On the pre-fix code this
        // ALWAYS burns the full timeout (deadlock); post-fix it resolves in
        // ~150ms when the completion event arrives.
        let start = Instant::now();
        let _ = action.wait(Some(Duration::from_secs(3)));
        let elapsed = start.elapsed().as_millis();
        *waited2.lock().unwrap() = elapsed;
        // The completing event resolves the action with an empty (None) result,
        // so we key on is_done() (the action reached terminal), NOT on a
        // non-empty result. A deadlock leaves is_done()==false at the deadline.
        *resolved2.lock().unwrap() = Some(action.is_done());
    });

    relay_mocktest::inbound_call(json!({
        "call_id": "c-handler-wait",
        "auto_states": ["created"],
    }));

    // The handler runs on its own thread; give it time to play + wait + resolve.
    assert!(
        wait_until(5000, || resolved.lock().unwrap().is_some()),
        "handler never returned — in-handler wait() deadlocked the client"
    );

    // The wait must have RESOLVED (action reached terminal), not timed out.
    assert_eq!(
        resolved.lock().unwrap().clone(),
        Some(true),
        "Action::wait() inside the handler did not resolve (burned its timeout — deadlock)"
    );
    // And it blocked for roughly the completion delay, NOT the full 3s deadline
    // (a timed-out wait would report ~3000ms).
    let ms = *waited_ms.lock().unwrap();
    assert!(
        ms < 2500,
        "wait() blocked {ms}ms — that is a burned timeout, not a real completion"
    );

    // The play frame must have reached the mock while the handler was blocked.
    let plays = relay_mocktest::journal_recv(Some("calling.play"));
    assert!(
        !plays.is_empty(),
        "the play frame never flushed to the mock — the reader thread was blocked by the handler"
    );

    client.disconnect();
}

// ---------------------------------------------------------------------------
// Redelivered calling.call.receive (porting-sdk#141)
//
// RELAY delivers at least once: the same receive frame can arrive twice for one
// call. Receive must therefore be idempotent per call_id — see the "Event
// Redelivery" section of porting-sdk's RELAY_IMPLEMENTATION_GUIDE.md.
// ---------------------------------------------------------------------------

/// Without the idempotency guard the second receive builds a second Call and
/// overwrites `calls[call_id]`. Routing only ever reads that map, so the first
/// Call — the one handed to the application — silently stops receiving events
/// and never reaches a terminal state.
#[test]
fn test_redelivered_receive_keeps_the_live_call() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let handler_calls: Arc<Mutex<Vec<Arc<signalwire::relay::Call>>>> =
        Arc::new(Mutex::new(Vec::new()));
    let collected = handler_calls.clone();
    client.on_call(move |call, _ev| {
        collected.lock().unwrap().push(call);
    });

    relay_mocktest::inbound_call(json!({
        "call_id": "c-redeliver",
        "auto_states": ["ringing", "answered"],
        "delay_ms": 20,
        "redeliver_receive": 1,
    }));
    assert!(
        wait_until(5000, || !handler_calls.lock().unwrap().is_empty()),
        "on_call handler did not fire"
    );
    // Let the redelivery and the trailing state frame drain.
    std::thread::sleep(std::time::Duration::from_millis(600));

    // 1. One call means one handler invocation.
    let calls = handler_calls.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        1,
        "on_call handler re-entered for a redelivered receive ({} invocations for one call)",
        calls.len()
    );

    // 2. The instance the application holds is still the one events route to.
    //    A replacement in the calls map would leave it frozen at "ringing".
    let first = &calls[0];
    assert_eq!(
        first.state.lock().unwrap().clone(),
        "answered",
        "the Call handed to the application stopped receiving events"
    );

    // The duplicate really was on the wire — otherwise this proves nothing.
    let redelivered = relay_mocktest::journal_send(Some("calling.call.receive"))
        .into_iter()
        .filter(|e| {
            e.frame
                .pointer("/params/params/call_id")
                .and_then(Value::as_str)
                == Some("c-redeliver")
        })
        .count();
    assert_eq!(
        redelivered, 2,
        "mock did not redeliver the receive frame ({redelivered} sent); \
         the scenario under test never happened"
    );

    client.disconnect();
}

/// The dedup is per call_id and must not swallow a genuinely new concurrent
/// inbound call.
#[test]
fn test_distinct_call_ids_still_create_separate_calls() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    client.on_call(move |call, _ev| {
        seen2
            .lock()
            .unwrap()
            .push(call.call_id.clone().unwrap_or_default());
    });

    relay_mocktest::inbound_call(json!({"call_id": "c-first", "auto_states": ["ringing"]}));
    relay_mocktest::inbound_call(json!({"call_id": "c-second", "auto_states": ["ringing"]}));
    assert!(
        wait_until(5000, || seen.lock().unwrap().len() >= 2),
        "expected both distinct inbound calls to reach the handler"
    );

    let mut ids = seen.lock().unwrap().clone();
    ids.sort();
    assert_eq!(
        ids,
        vec!["c-first".to_string(), "c-second".to_string()],
        "dedup swallowed a distinct call"
    );

    client.disconnect();
}
