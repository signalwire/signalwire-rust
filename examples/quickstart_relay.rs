// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! README quickstart — RELAY client.
//!
//! The `client` region below is the exact code shown in the "RELAY Client"
//! section of the crate README, included there via a `<!-- include: -->` marker
//! and asserted byte-identical at gate time.

// region: client
use signalwire::relay::Client;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = Arc::new(Client::from_env()?);

    client.on_call(|call, _event| {
        let _ = call.answer();
        let action = call
            .play(serde_json::json!({
                "play": [{
                    "type": "tts",
                    "params": {"text": "Welcome to SignalWire!"}
                }]
            }))
            .expect("relay verb must start against the server");
        let _ = action.is_done();
        let _ = call.hangup();
    });

    println!("Waiting for inbound calls ...");
    client.connect()?;
    client.receive(&["default".to_string()]);

    // Block while the relay loop runs in the background.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
// endregion: client
