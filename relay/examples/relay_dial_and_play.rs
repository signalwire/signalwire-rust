// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Dial an outbound call and play a message.
//!
//! Usage:
//!   TO_NUMBER=+15551234567 FROM_NUMBER=+15559876543 cargo run --example relay_dial_and_play
//!
//! Environment:
//!   SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE
//!   TO_NUMBER   - destination phone number
//!   FROM_NUMBER - caller ID (must be a verified SignalWire number)

use signalwire::relay::RelayClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RelayClient::builder()
        .project(&env::var("SIGNALWIRE_PROJECT_ID")?)
        .token(&env::var("SIGNALWIRE_API_TOKEN")?)
        .space(&env::var("SIGNALWIRE_SPACE")?)
        .contexts(vec!["default".into()])
        .build()?;

    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());
    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());

    println!("Dialing {to} from {from} ...");

    let call = client.calling().dial(&to, &from).await?;
    println!("Call connected: {}", call.call_id);

    // Play a greeting
    let action = call.play(vec![serde_json::json!({
        "type": "tts",
        "params": {"text": "Hello! This is an automated message from SignalWire."}
    })]).await?;
    action.wait().await?;

    // Play an audio file
    let action = call.play(vec![serde_json::json!({
        "type": "audio",
        "params": {"url": "https://cdn.signalwire.com/default-music/welcome.mp3"}
    })]).await?;
    action.wait().await?;

    // Hang up
    call.hangup().await?;
    println!("Call ended.");

    Ok(())
}
