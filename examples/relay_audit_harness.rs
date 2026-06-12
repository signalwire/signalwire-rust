// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `relay_audit_harness` — runtime probe for the RELAY transport.
//!
//! This binary is what the porting-sdk's `audit_relay_handshake.py`
//! drives to prove the Rust SDK's `relay::Client` opens a real
//! WebSocket connection, runs the `signalwire.connect` handshake,
//! subscribes to a context, and dispatches an inbound
//! `signalwire.event` to the registered callback. A green run from
//! the audit means: socket actually opened (no stub transport),
//! JSON-RPC actually serialized, real bytes on the wire.
//!
//! Environment variables (set by the audit fixture):
//!   - SIGNALWIRE_RELAY_HOST     `127.0.0.1:NNNN` (the fixture's bind port)
//!   - SIGNALWIRE_RELAY_SCHEME   `ws` (audit) or `wss` (production)
//!   - SIGNALWIRE_PROJECT_ID     `audit`
//!   - SIGNALWIRE_API_TOKEN      `audit`
//!   - SIGNALWIRE_CONTEXTS       `audit_ctx` (comma-separated)
//!
//! Exit codes:
//!   - 0  on a clean handshake + subscribe + event dispatch
//!   - 1  on any error (socket failure, handshake timeout, no event in 5s)

use signalwire::relay::Client;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    // Disable normal logging so our stdout is clean for the audit.
    if env::var("SIGNALWIRE_LOG_MODE").is_err() {
        // SAFETY: Single-threaded init; no other threads are spawned yet.
        unsafe {
            env::set_var("SIGNALWIRE_LOG_MODE", "off");
        }
    }

    let project = env::var("SIGNALWIRE_PROJECT_ID").unwrap_or_else(|_| "audit".to_string());
    let token = env::var("SIGNALWIRE_API_TOKEN").unwrap_or_else(|_| "audit".to_string());
    // The "host" arg fed to Client::new is overridden inside connect()
    // by SIGNALWIRE_RELAY_HOST when set; pass a placeholder here.
    let host = env::var("SIGNALWIRE_RELAY_HOST").unwrap_or_else(|_| "audit.host".to_string());
    let contexts: Vec<String> = env::var("SIGNALWIRE_CONTEXTS")
        .unwrap_or_else(|_| "audit_ctx".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let client = Arc::new(Client::new(&project, &token, &host));

    // Pre-load the contexts so they go in the connect handshake's
    // `params.contexts` (mirrors Python). The audit fixture's
    // subscribe-counter is keyed off the explicit `signalwire.subscribe`
    // frame we send below.
    {
        let mut ctxs = client.contexts.lock().unwrap();
        for c in &contexts {
            if !ctxs.contains(c) {
                ctxs.push(c.clone());
            }
        }
    }

    // Register a generic event handler so the audit's `signalwire.event`
    // push reaches user code. We flip an AtomicBool AND fire a
    // `signalwire.event`-method frame back over the socket — that's the
    // hook the porting-sdk fixture watches for to confirm dispatch
    // happened (see audit_relay_handshake.py's
    // `state.event_dispatched = True` branch).
    let saw_event = Arc::new(AtomicBool::new(false));
    {
        let saw = saw_event.clone();
        let cli = client.clone();
        client.on_event(move |event, params| {
            saw.store(true, Ordering::SeqCst);
            cli.send(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": format!("dispatched-{}", event.event_type()),
                "method": "signalwire.event",
                "params": {
                    "dispatched": true,
                    "event_type": event.event_type(),
                    "echoed": params,
                }
            }));
        });
    }

    if let Err(e) = client.connect() {
        eprintln!("relay_audit_harness: connect failed: {e}");
        std::process::exit(1);
    }

    // Subscribe explicitly so the audit fixture's `signalwire.subscribe`
    // method-name watch fires. The fixture replies with a no-op success
    // (audit_relay_handshake.py SKILL_PROBES dispatcher) so the call
    // returns immediately.
    client.send_request(
        "signalwire.subscribe",
        serde_json::json!({ "contexts": contexts }),
    );

    // Wait up to 5 seconds for an inbound event to be dispatched.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if saw_event.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let got_event = saw_event.load(Ordering::SeqCst);

    // Give the reader thread a moment to flush the event-ACK frame to
    // the socket. `handle_message` calls `send_ack` synchronously, which
    // enqueues the frame on the write channel; the reader thread
    // alternates reads with channel drains, so the ack lands on the
    // wire on the next iteration. A short sleep ensures the audit
    // fixture sees the ack before we close.
    std::thread::sleep(Duration::from_millis(300));
    client.disconnect();

    if !got_event {
        eprintln!("relay_audit_harness: no event arrived within 5s");
        std::process::exit(1);
    }

    println!("relay_audit_harness: event dispatched");
    std::process::exit(0);
}
