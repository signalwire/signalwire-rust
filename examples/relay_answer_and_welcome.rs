// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! RELAY: Answer an inbound call and say "Welcome to SignalWire!"
//!
//! Environment:
//!   SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::relay::RelayClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
