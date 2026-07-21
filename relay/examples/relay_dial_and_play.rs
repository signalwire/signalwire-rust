// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Dial an outbound call and play a message.
//!
//! The RELAY client is synchronous — no `async`/`await`. `dial_blocking`
//! initiates an outbound call and blocks until it is answered (or the dial
//! times out), returning the resolved `Call`.
//!
//! Usage:
//!   `TO_NUMBER=+15551234567 FROM_NUMBER=+15559876543 cargo run --example relay_dial_and_play`
//!
//! Environment:
//!   `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`
//!   `TO_NUMBER`   - destination phone number
//!   `FROM_NUMBER` - caller ID (must be a verified SignalWire number)

use signalwire::relay::Client;
use std::env;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(Client::from_env()?);

    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());
    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());

    // The reader thread must be running before we can dial.
    client.connect()?;

    println!("Dialing {to} from {from} ...");

    // `devices` is the standard serial/parallel device matrix: the outer array
    // is the serial list, each inner array is a set of parallel devices.
    let call = client.dial_blocking(
        serde_json::json!([[
            {"type": "phone", "params": {"to_number": to, "from_number": from}}
        ]]),
        None,                    // tag (auto-generated when None)
        None,                    // max_duration (seconds)
        Duration::from_secs(30), // dial timeout
    )?;
    println!("Call connected: {}", call.repr());

    // Play a greeting (TTS).
    let action = call
        .play(serde_json::json!({
            "play": [{
                "type": "tts",
                "params": {"text": "Hello! This is an automated message from SignalWire."}
            }]
        }))
        .expect("relay verb must start against the server");
    let _ = action.wait(Some(Duration::from_secs(30)));

    // Play an audio file.
    let action = call
        .play(serde_json::json!({
            "play": [{
                "type": "audio",
                "params": {"url": "https://cdn.signalwire.com/default-music/welcome.mp3"}
            }]
        }))
        .expect("relay verb must start against the server");
    let _ = action.wait(Some(Duration::from_secs(30)));

    // Hang up.
    let _ = call.hangup();
    println!("Call ended.");

    Ok(())
}
