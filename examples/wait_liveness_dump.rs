// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! WAIT-LIVENESS dump program for the cross-port behavioral differ
//! (porting-sdk `scripts/diff_port_wait_liveness.py`).
//!
//! The differ runs the `wait_liveness_corpus` against signalwire-python to build
//! the golden LIVENESS classification, then runs THIS program (which embeds the
//! same corpus) and structurally compares our per-case classification. The
//! artifact is a CLASSIFICATION (not raw ms), so the golden is deterministic
//! while the timing that produces it is real and unfakeable: a `wait()` that is
//! a no-op returns at t~=0 (`blocked_until_event=false` -> RED); a `wait()` that
//! hangs blows the deadline (`timed_out=true` -> RED); a correct `wait()` blocks
//! until the deferred completing event arrives, then returns with the finished
//! state (the golden -> GREEN).
//!
//! Unlike `wire_relay_dump` (which RECORDS the send-side frame), this gate MUST
//! exercise real liveness — so we drive the SDK against a REAL porting-sdk
//! `mock_relay` server (via the shared `relay_mocktest` harness) and deliver the
//! completing event as a DEFERRED push through the SAME socket-read ->
//! event-dispatch path the real server drives. A `wait()` that never pumps the
//! read loop cannot observe it.
//!
//! Protocol: stdout = ONE JSON object mapping `case_id` -> classification. Only
//! stdout carries JSON; all setup/logging/diagnostics go to stderr.
//!
//! Run from the repo root: `cargo run --quiet --example wait_liveness_dump`.

#[path = "../tests/common/mod.rs"]
mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use signalwire::relay::{Action, Call, Client};

use common::relay_mocktest;

// Classification tolerances — MUST match diff_port_wait_liveness.py.
const DEADLINE_S: f64 = 5.0; // a wait() outliving this is HUNG (timed_out)
const BLOCK_TOL_MS: f64 = 40.0; // how much earlier than delay_ms a return may be and still "blocked"
const DELAY_MS: u32 = 150; // MUST match wait_liveness_corpus.DELAY_MS

fn wait_until<F: Fn() -> bool>(budget_ms: u64, f: F) -> bool {
    let deadline = Instant::now() + Duration::from_millis(budget_ms);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Bring up an answered inbound call wired to the client (so events route to it).
fn answered_inbound_call(client: &Arc<Client>, call_id: &str) -> Arc<Call> {
    let captured: Arc<Mutex<Option<Arc<Call>>>> = Arc::new(Mutex::new(None));
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
    relay_mocktest::inbound_call(json!({ "call_id": call_id, "auto_states": ["created"] }));
    wait_until(3000, || captured.lock().unwrap().is_some());
    let call = captured.lock().unwrap().clone().expect("on_call fired");
    *call.state.lock().unwrap() = "answered".to_string();
    call
}

/// A `signalwire.event` frame carrying a scripted RELAY event.
fn event_frame(event_type: &str, call_id: &str, control_id: &str, state: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": format!("evt-{event_type}-{call_id}"),
        "method": "signalwire.event",
        "params": {
            "event_type": event_type,
            "params": {"call_id": call_id, "control_id": control_id, "state": state},
        },
    })
}

/// Arm the completing event to be delivered `DELAY_MS` after now (deferred),
/// through the mock's socket -> read-loop -> dispatch path. The mock session
/// scope is thread-local, so the spawned pusher thread must re-establish this
/// test's scope before pushing or the frame broadcasts to the wrong session and
/// never reaches our client. The event targets the action's real `control_id`
/// (the SDK generates its own uuid; we read it back).
fn arm_deferred_completion(event_type: &'static str, call_id: String, control_id: String) {
    let scope = relay_mocktest::scope();
    std::thread::spawn(move || {
        relay_mocktest::set_scope(scope);
        std::thread::sleep(Duration::from_millis(u64::from(DELAY_MS)));
        relay_mocktest::push(event_frame(event_type, &call_id, &control_id, "finished"));
    });
}

/// The comparable liveness classification the differ byte-compares.
fn classify(
    t_wait_start: Instant,
    t_return: Option<Instant>,
    completed_state: String,
    timed_out: bool,
) -> Value {
    let Some(t_return) = t_return.filter(|_| !timed_out) else {
        return json!({
            "blocked_until_event": false,
            "returned_after_event": false,
            "completed_state": "",
            "timed_out": true,
        });
    };
    let elapsed_ms = t_return.duration_since(t_wait_start).as_secs_f64() * 1000.0;
    let blocked = elapsed_ms >= (f64::from(DELAY_MS) - BLOCK_TOL_MS);
    json!({
        "blocked_until_event": blocked,
        "returned_after_event": true,
        "completed_state": completed_state,
        "timed_out": false,
    })
}

/// Drive one action verb: arm the deferred finished event, start the action,
/// wait, and measure the wait-start / wait-return instants, completed state,
/// and whether it timed out.
fn drive(
    call: &Arc<Call>,
    action: &Arc<Action>,
    event_type: &'static str,
) -> (Instant, Option<Instant>, String, bool) {
    let call_id = call.call_id.clone().unwrap_or_default();
    arm_deferred_completion(event_type, call_id, action.control_id().to_string());
    let t_wait_start = Instant::now();
    let result = action.wait(Some(Duration::from_secs_f64(DEADLINE_S)));
    let t_return = Instant::now();
    let timed_out = !action.is_done() && result.is_none();
    let completed_state = action.state().unwrap_or_default();
    (
        t_wait_start,
        if timed_out { None } else { Some(t_return) },
        completed_state,
        timed_out,
    )
}

fn play_media() -> Value {
    json!([{"type": "audio", "params": {"url": "https://x/a.mp3"}}])
}

fn main() {
    let mut out = serde_json::Map::new();

    // ---- live_play_wait -----------------------------------------------------
    {
        let _g = relay_mocktest::begin();
        let client = relay_mocktest::connected_client(&["default"]);
        let call = answered_inbound_call(&client, "call-xyz");
        let action = call
            .play(json!({"play": play_media()}))
            .expect("play must start");
        let (s, r, st, to) = drive(&call, &action, "calling.call.play");
        out.insert("live_play_wait".to_string(), classify(s, r, st, to));
        client.disconnect();
    }

    // ---- live_record_wait ---------------------------------------------------
    {
        let _g = relay_mocktest::begin();
        let client = relay_mocktest::connected_client(&["default"]);
        let call = answered_inbound_call(&client, "call-xyz");
        let action = call
            .record(json!({"record": {"audio": {"format": "mp3"}}}))
            .expect("record must start");
        let (s, r, st, to) = drive(&call, &action, "calling.call.record");
        out.insert("live_record_wait".to_string(), classify(s, r, st, to));
        client.disconnect();
    }

    // ---- live_nested_wait ---------------------------------------------------
    // The "wait inside on_completed" re-entrancy pattern. Like the python oracle
    // (diff_port_wait_liveness.py::_drive_nested), the inner wait is driven right
    // AFTER the outer wait returns — not synchronously inside the completion
    // callback (which fires on the read-loop thread and would deadlock a
    // thread-blocking wait). This still exercises re-entrancy of the receive
    // path: the inner wait pumps the same connection the outer just used.
    // FOLD: timed_out if EITHER hung, blocked only if BOTH blocked,
    // completed_state from the inner (last) completion.
    {
        let _g = relay_mocktest::begin();
        let client = relay_mocktest::connected_client(&["default"]);
        let call = answered_inbound_call(&client, "call-xyz");

        let outer = call
            .play(json!({"play": play_media()}))
            .expect("play must start");
        let (os, or, _ost, oto) = drive(&call, &outer, "calling.call.play");

        let outer_cls = classify(os, or, "finished".to_string(), oto);
        let folded = if outer_cls["timed_out"].as_bool().unwrap_or(false) {
            json!({
                "blocked_until_event": false,
                "returned_after_event": false,
                "completed_state": "",
                "timed_out": true,
            })
        } else {
            let inner = call
                .record(json!({"record": {"audio": {"format": "mp3"}}}))
                .expect("record must start");
            let (is, ir, ist, ito) = drive(&call, &inner, "calling.call.record");
            let inner_cls = classify(is, ir, ist, ito);
            if inner_cls["timed_out"].as_bool().unwrap_or(false) {
                json!({
                    "blocked_until_event": false,
                    "returned_after_event": false,
                    "completed_state": "",
                    "timed_out": true,
                })
            } else {
                let both_blocked = outer_cls["blocked_until_event"].as_bool().unwrap_or(false)
                    && inner_cls["blocked_until_event"].as_bool().unwrap_or(false);
                json!({
                    "blocked_until_event": both_blocked,
                    "returned_after_event": true,
                    "completed_state": inner_cls["completed_state"].clone(),
                    "timed_out": false,
                })
            }
        };
        out.insert("live_nested_wait".to_string(), folded);
        client.disconnect();
    }

    println!("{}", Value::Object(out));
}
