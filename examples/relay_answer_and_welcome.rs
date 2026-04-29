// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! RELAY: Answer an inbound call and say "Welcome to SignalWire!"
//!
//! Environment:
//!   SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::relay::Client;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("SIGNALWIRE_LOG_LEVEL").is_err() {
        // SAFETY: Single-threaded init; no other threads are spawned yet.
        unsafe { env::set_var("SIGNALWIRE_LOG_LEVEL", "debug"); }
    }

    let client = Client::from_env()?;

    client.on_call(|call, _event| {
        let id = call.call_id.clone().unwrap_or_default();
        println!("Incoming call: {}", id);
        let _ = call.answer();

        let _ = call.play(serde_json::json!({
            "play": [{
                "type": "tts",
                "params": {"text": "Welcome to SignalWire!"}
            }]
        }));

        let _ = call.hangup();
        println!("Call ended: {}", id);
    });

    println!("Waiting for inbound calls on context 'default' ...");
    client.connect();
    client.receive(&["default".to_string()]);

    // Block forever (relay loop runs in a background thread).
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
