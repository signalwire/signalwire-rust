// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Answer an inbound call and say "Welcome to SignalWire!"
//!
//! The RELAY client is synchronous: it runs its WebSocket/Blade event loop on a
//! background thread and invokes the `on_call` handler on that thread. There is
//! no `async`/`await` and no `tokio` runtime to set up.
//!
//! Set these env vars (or pass them directly to `Client::new`):
//!   `SIGNALWIRE_PROJECT_ID`   - your SignalWire project ID
//!   `SIGNALWIRE_API_TOKEN`    - your SignalWire API token
//!   `SIGNALWIRE_SPACE`        - your SignalWire space (e.g. example.signalwire.com)
//!
//! For full WebSocket / JSON-RPC debug output:
//!   `SIGNALWIRE_LOG_LEVEL=debug`

use signalwire::relay::Client;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = Arc::new(Client::from_env()?);

    client.on_call(|call, _event| {
        println!("Incoming call: {}", call.repr());
        let _ = call.answer();

        // Media verbs return an `Arc<Action>`; block on `wait()` for completion.
        let action = call.play(serde_json::json!({
            "play": [{
                "type": "tts",
                "params": {"text": "Welcome to SignalWire!"}
            }]
        }));
        let _ = action.wait(Some(Duration::from_secs(30)));

        let _ = call.hangup();
        println!("Call ended: {}", call.repr());
    });

    println!("Waiting for inbound calls on context 'default' ...");
    client.connect()?;
    client.receive(&["default".to_string()]);

    // Block while the relay loop runs on its background thread.
    client.run();
    Ok(())
}
