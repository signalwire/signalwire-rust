// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `wire_relay_dump` — the Rust port's WIRE-RELAY dump program for the
//! cross-port relay differ (porting-sdk/scripts/diff_port_wire_relay.py).
//!
//! It captures, for each `wire_relay_corpus` case, the observable RELAY artifact:
//!   - verb   : the {method, params} JSON-RPC frame a Call verb (or an Action
//!     control-op) hands to the wire.
//!   - client : the {method, params} frame a `RelayClient` call (execute /
//!     dial / `send_message`) sends.
//!   - event  : the decoded fields a typed event decoder extracts from a
//!     payload.
//!
//! It prints ONE JSON object mapping case-id -> artifact to stdout; the differ
//! canonicalizes both sides (normalizing the random `control_id` to a sentinel)
//! and byte-compares against the Python oracle. Only stdout carries the JSON
//! object.
//!
//! Frame capture (compiled port): the Rust `Call`/`Action`/`RelayClient`
//! record every emitted frame in-memory (`Call::sent_commands`,
//! `Action::sent_commands`, `RelayClient::sent_messages`) — even with NO live
//! socket attached — so no mock WS server is needed. The three client-level
//! calls block waiting for a wire response; they are driven on a background
//! thread and the frame (recorded synchronously before the wait) is read back.
//! Event decoding is pure (no wire).
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example wire_relay_dump

use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use signalwire::relay::Client as RelayClient;
use signalwire::relay::call::Call;
use signalwire::relay::event::{
    CallStateEvent, CollectEvent, QueueEvent, RecordEvent, parse_event,
};

const NODE: &str = "node-abc";
const CALL: &str = "call-xyz";
const CID: &str = "ctl-123";

/// A `{method, params}` frame object.
fn frame(method: &str, params: Value) -> Value {
    json!({ "method": method, "params": params })
}

/// A recording client to observe frames at the CLIENT-SEND boundary.
///
/// The relay differ upgraded to observe transmission at the client boundary:
/// a Call verb is only credited a frame if it actually reaches
/// `Client::send_request` (the wire). We therefore build every Call/Action
/// wired to this real `Client` (no live socket — `send()` is pure in-memory,
/// recording into `sent_messages`) and read the frame back from
/// `client.sent_messages`. A verb that builds but never transmits records
/// NOTHING here and the case fails — which is the whole point.
fn recording_client() -> Arc<RelayClient> {
    Arc::new(RelayClient::new("proj-1", "tok-1", "mock"))
}

/// Build a fresh Call wired to `client` so its verbs transmit to the wire.
fn make_call(client: &Arc<RelayClient>) -> Call {
    let call = Call::new(&json!({
        "call_id": CALL,
        "node_id": NODE,
        "state": "answered",
    }));
    call.set_client(client);
    call
}

/// The last `calling.*` frame the client transmitted, reduced to
/// `{method, params}`. Reads at the CLIENT-SEND boundary (`sent_messages`),
/// NOT the Call's in-memory `sent_commands` — so a non-transmitting verb
/// yields no frame and the differ fails the case.
fn last_client_frame(client: &Arc<RelayClient>) -> Value {
    let msgs = client.sent_messages.lock().expect("sent_messages lock");
    let msg = msgs
        .last()
        .cloned()
        .expect("a frame reached the client-send boundary");
    let method = msg
        .get("method")
        .and_then(Value::as_str)
        .expect("frame has method")
        .to_string();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    frame(&method, params)
}

fn main() {
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();

    capture_verbs(&mut out);
    capture_client_frames(&mut out);
    decode_events(&mut out);

    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize relay dump")
    );
}

/// Call command verbs — capture the `{method, params}` frame each emits. The
/// Rust raw verbs take pre-built wire params (the typed verbs `play_tts` /
/// `detect_answering_machine` / `prompt_tts` build the params themselves).
fn capture_verbs(out: &mut BTreeMap<&str, Value>) {
    // relay_play
    {
        let client = recording_client();
        let call = make_call(&client);
        call.play(json!({
            "play": [{"type": "audio", "params": {"url": "https://x/a.mp3"}}],
            "volume": 5.0,
            "control_id": CID,
        }));
        out.insert("relay_play", last_client_frame(&client));
    }
    // relay_play_tts (typed convenience)
    {
        let client = recording_client();
        let call = make_call(&client);
        call.play_tts("Hello world", json!({"voice": "en-US-Neural"}));
        out.insert("relay_play_tts", last_client_frame(&client));
    }
    // relay_record
    {
        let client = recording_client();
        let call = make_call(&client);
        call.record(json!({
            "record": {"audio": {"format": "mp3", "beep": true}},
            "control_id": CID,
        }));
        out.insert("relay_record", last_client_frame(&client));
    }
    // relay_connect
    {
        let client = recording_client();
        let call = make_call(&client);
        call.connect(json!({
            "devices": [[{"type": "phone", "params": {"to_number": "+15551112222"}}]],
            "ringback": [{"type": "ringtone", "params": {"name": "us"}}],
            "tag": "leg-1",
            "max_duration": 3600,
        }));
        out.insert("relay_connect", last_client_frame(&client));
    }
    // relay_collect
    {
        let client = recording_client();
        let call = make_call(&client);
        call.collect(json!({
            "digits": {"max": 4, "terminators": "#"},
            "speech": {"language": "en-US"},
            "initial_timeout": 5.0,
            "partial_results": true,
            "control_id": CID,
        }));
        out.insert("relay_collect", last_client_frame(&client));
    }
    // relay_prompt (play_and_collect via typed prompt_tts)
    {
        let client = recording_client();
        let call = make_call(&client);
        call.prompt_tts(
            "Enter your PIN",
            json!({"digits": {"max": 4}}),
            json!({"voice": "en-US-Neural"}),
        );
        out.insert("relay_prompt", last_client_frame(&client));
    }
    // relay_detect
    {
        let client = recording_client();
        let call = make_call(&client);
        call.detect(json!({
            "detect": {"type": "machine", "params": {"initial_timeout": 4.0}},
            "timeout": 30.0,
            "control_id": CID,
        }));
        out.insert("relay_detect", last_client_frame(&client));
    }
    // relay_detect_amd (typed convenience)
    {
        let client = recording_client();
        let call = make_call(&client);
        call.detect_answering_machine(json!({
            "initial_timeout": 4.0,
            "machine_words_threshold": 6,
            "timeout": 30.0,
        }));
        out.insert("relay_detect_amd", last_client_frame(&client));
    }
    // relay_tap
    {
        let client = recording_client();
        let call = make_call(&client);
        call.tap(json!({
            "tap": {"type": "audio", "params": {"direction": "both"}},
            "device": {"type": "ws", "params": {"uri": "wss://x/tap"}},
            "control_id": CID,
        }));
        out.insert("relay_tap", last_client_frame(&client));
    }
    // relay_send_fax
    {
        let client = recording_client();
        let call = make_call(&client);
        call.send_fax(json!({
            "document": "https://x/doc.pdf",
            "identity": "+15550001111",
            "header_info": "Hdr",
            "control_id": CID,
        }));
        out.insert("relay_send_fax", last_client_frame(&client));
    }

    // ---- control-ops: Action methods ----
    // Each control-op transmits a sub-command frame through the SAME client;
    // `last_client_frame` reads that (the last transmitted) frame, so a
    // control-op that never reaches the client would be caught here too.
    // relay_play_stop
    {
        let client = recording_client();
        let call = make_call(&client);
        let action = call.play(json!({
            "play": [{"type": "audio", "params": {"url": "https://x/a.mp3"}}],
            "control_id": CID,
        }));
        action.stop();
        out.insert("relay_play_stop", last_client_frame(&client));
    }
    // relay_play_pause — the pause sub-command (control-op) on the play action.
    // `call.play` returns the base `Action`; its control-ops (pause/resume/
    // volume) are the typed-subclass wrappers over `execute_subcommand`, so we
    // emit the same sub-command frame directly.
    {
        let client = recording_client();
        let call = make_call(&client);
        let action = call.play(json!({
            "play": [{"type": "audio", "params": {"url": "https://x/a.mp3"}}],
            "control_id": CID,
        }));
        let mut extra = std::collections::HashMap::new();
        extra.insert("behavior".to_string(), json!("silence"));
        action.execute_subcommand("calling.play.pause", extra);
        out.insert("relay_play_pause", last_client_frame(&client));
    }
    // relay_record_resume
    {
        let client = recording_client();
        let call = make_call(&client);
        let action = call.record(json!({
            "record": {"audio": {"format": "mp3"}},
            "control_id": CID,
        }));
        action.execute_subcommand("calling.record.resume", std::collections::HashMap::new());
        out.insert("relay_record_resume", last_client_frame(&client));
    }
    // relay_play_volume
    {
        let client = recording_client();
        let call = make_call(&client);
        let action = call.play(json!({
            "play": [{"type": "audio", "params": {"url": "https://x/a.mp3"}}],
            "control_id": CID,
        }));
        let mut extra = std::collections::HashMap::new();
        extra.insert("volume".to_string(), json!(3.5));
        action.execute_subcommand("calling.play.volume", extra);
        out.insert("relay_play_volume", last_client_frame(&client));
    }
}

/// RelayClient-level frames. Each call blocks on a wire response; the frame is
/// recorded synchronously by `send()` before the wait, so we drive the call on
/// a detached thread and read `sent_messages` back. The recorded frame is a
/// full JSON-RPC envelope `{jsonrpc, id, method, params}`; we reduce it to
/// `{method, params}`.
fn capture_client_frames(out: &mut BTreeMap<&str, Value>) {
    // relay_client_execute — passthrough (calling.answer).
    out.insert(
        "relay_client_execute",
        drive_client_frame("calling.answer", |c| {
            let _ = c.execute("calling.answer", json!({"node_id": NODE, "call_id": CALL}));
        }),
    );

    // relay_send_message — messaging.send with default context + body/tags.
    out.insert(
        "relay_send_message",
        drive_client_frame("messaging.send", |c| {
            let _ = c.send_message(
                "+15551112222",
                "+15553334444",
                Some("hi"),
                None,
                Some(&["t1".to_string()]),
                None,
            );
        }),
    );

    // relay_dial — calling.dial with tag + devices.
    out.insert(
        "relay_dial",
        drive_client_frame("calling.dial", |c| {
            let _ = c.dial(
                json!([[{"type": "phone", "params": {"to_number": "+15551112222"}}]]),
                Some("dial-1"),
                Some(600),
                Duration::from_millis(50),
            );
        }),
    );
}

/// Run `f` (a blocking client call) on a detached thread, wait for the frame
/// whose `method` matches to land in `sent_messages`, and reduce it to
/// `{method, params}`.
fn drive_client_frame<F>(want_method: &str, f: F) -> Value
where
    F: FnOnce(&Arc<RelayClient>) + Send + 'static,
{
    let client = Arc::new(RelayClient::new("proj-1", "tok-1", "mock"));
    let driver = Arc::clone(&client);
    // Detach the blocking call; the frame is recorded before it waits.
    thread::spawn(move || f(&driver));

    // Poll sent_messages until the target frame appears (or a short deadline).
    for _ in 0..100 {
        {
            let msgs = client.sent_messages.lock().expect("sent_messages lock");
            if let Some(msg) = msgs
                .iter()
                .rev()
                .find(|m| m.get("method").and_then(Value::as_str) == Some(want_method))
            {
                let params = msg.get("params").cloned().unwrap_or(Value::Null);
                return frame(want_method, params);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("client frame {want_method} was never recorded");
}

/// Typed event decoders (pure — no wire).
fn decode_events(out: &mut BTreeMap<&str, Value>) {
    // relay_evt_queue — queue_id <- id, queue_name <- name.
    {
        let ev = QueueEvent::from_payload(&json!({
            "event_type": "calling.call.queue",
            "params": {
                "call_id": CALL, "control_id": CID, "status": "waiting",
                "id": "q-42", "name": "support", "position": 3, "size": 10,
            },
        }));
        out.insert(
            "relay_evt_queue",
            json!({
                "control_id": ev.control_id(),
                "status": ev.status(),
                "queue_id": ev.queue_id(),
                "queue_name": ev.queue_name(),
                "position": ev.position(),
                "size": ev.size(),
            }),
        );
    }

    // relay_evt_record — url/duration/size FALLBACK from nested record{}.
    {
        let ev = RecordEvent::from_payload(&json!({
            "event_type": "calling.call.record",
            "params": {
                "call_id": CALL, "control_id": CID, "state": "finished",
                "record": {"url": "https://x/rec.mp3", "duration": 12.5, "size": 4096},
            },
        }));
        out.insert(
            "relay_evt_record",
            json!({
                "control_id": ev.control_id(),
                "state": ev.state(),
                "url": ev.url(),
                "duration": ev.duration(),
                "size": ev.size(),
            }),
        );
    }

    // relay_evt_state_dispatch — parse_event -> CallStateEvent class + fields.
    {
        let payload = json!({
            "event_type": "calling.call.state",
            "params": {
                "call_id": CALL, "call_state": "answered",
                "direction": "inbound", "end_reason": "",
            },
        });
        let ev = parse_event(&payload);
        // Also decode via the typed class for the picked fields.
        let typed = CallStateEvent::from_payload(&payload);
        out.insert(
            "relay_evt_state_dispatch",
            json!({
                "_class": ev.class_name(),
                "call_id": typed.call_id(),
                "call_state": typed.call_state(),
                "direction": typed.direction(),
            }),
        );
    }

    // relay_evt_collect — result{} + final tri-state.
    {
        let ev = CollectEvent::from_payload(&json!({
            "event_type": "calling.call.collect",
            "params": {
                "call_id": CALL, "control_id": CID, "state": "finished",
                "result": {"type": "digit", "params": {"digits": "1234"}},
                "final": true,
            },
        }));
        out.insert(
            "relay_evt_collect",
            json!({
                "control_id": ev.control_id(),
                "state": ev.state(),
                "result": ev.result(),
                "final": ev.is_final(),
            }),
        );
    }
}
