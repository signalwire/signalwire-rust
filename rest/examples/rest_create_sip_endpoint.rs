// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Create a SIP endpoint via the REST API.
//!
//! SIP endpoints live under the Fabric namespace
//! (`client.fabric().sip_endpoints()`). The REST client is synchronous:
//! `create` takes a `&serde_json::Value`, `list` a `&HashMap<String, String>`,
//! and both return `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    println!("Creating SIP endpoint ...");

    let endpoint = client.fabric().sip_endpoints().create(&json!({
        "username": "alice",
        "password": "secure-password-123",
        "caller_id": "+15551234567",
        "name": "Alice's Desk Phone"
    }), None)?;

    println!("Endpoint created:");
    println!("  ID: {}", endpoint["id"]);
    println!("  Username: {}", endpoint["username"]);
    println!("  Name: {}", endpoint["name"]);

    // List all endpoints.
    let endpoints = client.fabric().sip_endpoints().list(&HashMap::new(), None)?;
    if let Some(arr) = endpoints.as_array() {
        println!("\nAll SIP endpoints ({}):", arr.len());
        for ep in arr {
            println!("  {} - {}", ep["username"], ep["name"]);
        }
    }

    Ok(())
}
