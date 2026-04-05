// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Create a SIP endpoint via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    println!("Creating SIP endpoint ...");

    let endpoint = client.sip().endpoints().create(serde_json::json!({
        "username": "alice",
        "password": "secure-password-123",
        "caller_id": "+15551234567",
        "friendly_name": "Alice's Desk Phone",
    })).await?;

    println!("Endpoint created:");
    println!("  SID: {}", endpoint["sid"]);
    println!("  Username: {}", endpoint["username"]);
    println!("  Friendly name: {}", endpoint["friendly_name"]);

    // List all endpoints
    let endpoints = client.sip().endpoints().list(&[]).await?;
    if let Some(arr) = endpoints.as_array() {
        println!("\nAll SIP endpoints ({}):", arr.len());
        for ep in arr {
            println!("  {} - {}", ep["username"], ep["friendly_name"]);
        }
    }

    Ok(())
}
