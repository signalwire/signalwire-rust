//! Behavioural proof for RUST-1 — the RELAY verb→`Result` flip.
//!
//! Every `calling.*` verb on [`signalwire::relay::Call`] now returns
//! `Result<_, RelayError>` (was a bare `Value` / `Arc<Action>` that could not
//! report a server rejection). These tests drive the REAL client against the
//! shared `mock_relay` server and prove the flip end-to-end:
//!
//!   * happy path — a verb against the mock returns `Ok(...)` (the typed Err
//!     channel did not break the success path);
//!   * a non-2xx server RESULT code makes the verb return
//!     `Err(RelayError::Rpc)` (mirrors Python's `_handle_message`: any code not
//!     matching `^2\d{2}$` raises) — the failure that the old bare-`Value`
//!     return SILENTLY DROPPED;
//!   * the RELAY "call gone" contract — a 404/410 result is swallowed to a
//!     no-op `Ok({})` (mirrors Python `Call._execute`), NOT surfaced as an
//!     error, so a verb on a vanished call is a harmless no-op.
//!
//! The reject is produced by the real mock (an armed `rpc_code`), never a
//! transport mock — so this is a wire-level behavioural test, not a
//! construction assertion.

#[path = "common/mod.rs"]
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::relay_mocktest;
use serde_json::json;
use signalwire::relay::{Call, Client as RelayClient, RelayError};

fn wait_until<F: Fn() -> bool>(budget_ms: u64, f: F) -> bool {
    use std::time::Instant;
    let deadline = Instant::now() + Duration::from_millis(budget_ms);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Capture the `Call` the mock delivers via an inbound-call sequence, then mark
/// it answered so verbs are meaningful. Mirrors the helper in the actions test.
fn inbound_call(client: &Arc<RelayClient>, call_id: &str) -> Arc<Call> {
    let captured: Arc<Mutex<Option<Arc<Call>>>> = Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    client.on_call(move |call, _ev| {
        *cap2.lock().unwrap() = Some(call);
    });
    relay_mocktest::inbound_call(json!({
        "call_id": call_id,
        "auto_states": ["created"],
    }));
    assert!(
        wait_until(3000, || captured.lock().unwrap().is_some()),
        "on_call did not fire for {call_id}"
    );
    let call = captured.lock().unwrap().clone().unwrap();
    *call.state.lock().unwrap() = "answered".to_string();
    call
}

// ---------------------------------------------------------------------------
// Happy path — a verb against the live mock returns Ok.
// ---------------------------------------------------------------------------

#[test]
fn test_answer_against_mock_returns_ok() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-answer-ok");

    // The mock replies `{"code":"200","message":"Answered"}` — a 2xx result.
    let result = call.answer();
    let body = result.expect("answer against the mock must succeed");
    assert_eq!(
        body.get("code").and_then(|c| c.as_str()),
        Some("200"),
        "the verb returns the server's 2xx result body"
    );
    client.disconnect();
}

#[test]
fn test_play_against_mock_returns_ok_action() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-play-ok");

    // An action-starting verb also returns Ok, carrying the tracked Action.
    let action = call
        .play(json!({"play": [{"type": "silence", "params": {"duration": 1}}]}))
        .expect("play against the mock must succeed");
    assert!(!action.is_done());
    assert_eq!(action.stop_method(), "calling.play.stop");
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Non-2xx server RESULT code → Err(RelayError::Rpc). This is the failure the
// old bare-`Value` return silently dropped.
// ---------------------------------------------------------------------------

#[test]
fn test_answer_with_500_result_yields_rpc_err() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-answer-500");

    // Arm the next `calling.answer` to reply with a 500 RESULT code.
    relay_mocktest::arm_method("calling.answer", json!([{"rpc_code": "500"}]));

    let result = call.answer();
    match result {
        Ok(v) => panic!("a 500 verb result must fail, got Ok({v:?})"),
        Err(RelayError::Rpc { method, message }) => {
            assert_eq!(method, "calling.answer", "the failing verb is carried");
            assert!(!message.is_empty(), "the server message is preserved");
        }
        Err(other) => panic!("expected RelayError::Rpc, got {other:?}"),
    }
    client.disconnect();
}

#[test]
fn test_play_with_500_result_yields_rpc_err_and_drops_action() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-play-500");

    relay_mocktest::arm_method("calling.play", json!([{"rpc_code": "500"}]));

    let result = call.play(json!({"play": [{"type": "silence", "params": {"duration": 1}}]}));
    match result {
        Ok(_) => panic!("a 500 action-start must fail"),
        Err(RelayError::Rpc { method, .. }) => {
            assert_eq!(method, "calling.play");
        }
        Err(other) => panic!("expected RelayError::Rpc, got {other:?}"),
    }
    // The failed action was removed from tracking (no zombie in-flight action).
    assert!(
        call.actions.lock().unwrap().is_empty(),
        "a failed action-start must not leave a tracked action"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RELAY "call gone" contract — a 404/410 is swallowed to a no-op Ok({}), NOT
// an error (mirrors Python Call._execute).
// ---------------------------------------------------------------------------

#[test]
fn test_answer_with_404_result_is_swallowed_to_noop() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-answer-404");

    relay_mocktest::arm_method("calling.answer", json!([{"rpc_code": "404"}]));

    // 404 = call gone → the verb is a no-op returning Ok({}), not an error.
    let body = call
        .answer()
        .expect("a call-gone 404 is swallowed, not surfaced as an error");
    assert!(
        body.as_object().is_some_and(serde_json::Map::is_empty),
        "the swallowed no-op returns an empty object, got {body:?}"
    );
    client.disconnect();
}

#[test]
fn test_play_with_410_result_swallowed_and_resolves_action() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = inbound_call(&client, "call-play-410");

    relay_mocktest::arm_method("calling.play", json!([{"rpc_code": "410"}]));

    // 410 = call gone → the action-start is swallowed; the returned Action is
    // immediately resolved so a later wait() returns instead of hanging.
    let action = call
        .play(json!({"play": [{"type": "silence", "params": {"duration": 1}}]}))
        .expect("a call-gone 410 is swallowed, not an error");
    assert!(
        action.is_done(),
        "a swallowed action-start must be pre-resolved so wait() won't hang"
    );
    assert!(call.actions.lock().unwrap().is_empty());
    client.disconnect();
}
