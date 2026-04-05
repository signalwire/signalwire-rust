// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Answer an inbound call and say "Welcome to SignalWire!"
//!
//! Set these env vars (or pass them directly to RelayClient):
//!   SIGNALWIRE_PROJECT_ID   - your SignalWire project ID
//!   SIGNALWIRE_API_TOKEN    - your SignalWire API token
//!   SIGNALWIRE_SPACE        - your SignalWire space (e.g. example.signalwire.com)
//!
//! For full WebSocket / JSON-RPC debug output:
//!   SIGNALWIRE_LOG_LEVEL=debug

use signalwire::relay::RelayClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Optionally enable debug logging
    if env::var("SIGNALWIRE_LOG_LEVEL").is_err() {
        env::set_var("SIGNALWIRE_LOG_LEVEL", "debug");
    }

    let client = RelayClient::builder()
        .project(&env::var("SIGNALWIRE_PROJECT_ID")?)
        .token(&env::var("SIGNALWIRE_API_TOKEN")?)
        .space(&env::var("SIGNALWIRE_SPACE")?)
        .contexts(vec!["default".into()])
        .build()?;

    client.on_call(|call| async move {
        println!("Incoming call: {}", call.call_id);
        call.answer().await?;

        let action = call.play(vec![serde_json::json!({
            "type": "tts",
            "params": {"text": "Welcome to SignalWire!"}
        })]).await?;
        action.wait().await?;

        call.hangup().await?;
        println!("Call ended: {}", call.call_id);
        Ok(())
    });

    println!("Waiting for inbound calls on context 'default' ...");
    client.run().await?;
    Ok(())
}
