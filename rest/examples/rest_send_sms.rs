// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Send an SMS message.
//!
//! NOTE ON SURFACE: the SignalWire Rust SDK's **REST** client has no
//! message-send endpoint — the REST message namespace is read-only log
//! retrieval (`client.logs().messages()`). Outbound SMS/MMS is sent over the
//! **RELAY** client via `Client::send_message`, which this example uses. It is
//! synchronous (blocks on the RELAY WebSocket) and returns a tracked `Message`
//! whose state advances as `messaging.state` events arrive.
//!
//! Usage:
//!   `FROM_NUMBER=+15559876543 TO_NUMBER=+15551234567 cargo run --example rest_send_sms`
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use signalwire::relay::Client;
use std::env;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(Client::from_env()?);

    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());
    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());

    // The reader thread must be up before we can send over RELAY.
    client.connect()?;

    println!("Sending SMS from {from} to {to} ...");

    let message = client.send_message(
        &to,
        &from,
        Some("Hello from the SignalWire Rust SDK!"),
        None, // media
        None, // tags
        None, // context (defaults to the connected protocol / "default")
    )?;

    println!("Message queued: {}", message.repr());

    Ok(())
}
