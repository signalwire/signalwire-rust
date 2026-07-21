// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage Fabric subscribers via the REST API.
//!
//! The REST client is synchronous. `create`/`update` take a
//! `&serde_json::Value`; `list` takes a `&HashMap<String, String>`. Every
//! method returns `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a subscriber.
    println!("Creating subscriber ...");
    let sub = client.fabric().subscribers().create(&json!({
        "email": "alice@example.com",
        "first_name": "Alice",
        "last_name": "Smith",
        "display_name": "Alice Smith"
    }), None)?;

    let sub_id = sub["id"].as_str().unwrap_or("unknown").to_string();
    println!("Subscriber created: {sub_id}");

    // List all subscribers.
    let subs = client.fabric().subscribers().list(&HashMap::new(), None)?;
    if let Some(arr) = subs.as_array() {
        println!("\nAll subscribers ({}):", arr.len());
        for s in arr {
            println!(
                "  {} - {} {} ({})",
                s["id"], s["first_name"], s["last_name"], s["email"]
            );
        }
    }

    // Update subscriber.
    println!("\nUpdating subscriber ...");
    client.fabric().subscribers().update(
        &sub_id,
        &json!({
            "display_name": "Alice S."
        }), None
    )?;
    println!("Subscriber updated.");

    // Get subscriber details.
    let details = client.fabric().subscribers().get(&sub_id, None)?;
    println!("Display name: {}", details["display_name"]);

    Ok(())
}
