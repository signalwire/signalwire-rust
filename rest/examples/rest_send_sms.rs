// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Send an SMS message via the REST API.
//!
//! Usage:
//!   FROM_NUMBER=+15559876543 TO_NUMBER=+15551234567 \
//!     cargo run --example rest_send_sms
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    let from = env::var("FROM_NUMBER").unwrap_or_else(|_| "+15559876543".into());
    let to = env::var("TO_NUMBER").unwrap_or_else(|_| "+15551234567".into());

    println!("Sending SMS from {from} to {to} ...");

    let result = client.messaging().send(serde_json::json!({
        "from": from,
        "to": to,
        "body": "Hello from the SignalWire Rust SDK!",
    })).await?;

    println!("Message SID: {}", result["sid"]);
    println!("Status: {}", result["status"]);

    Ok(())
}
