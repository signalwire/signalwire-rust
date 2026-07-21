// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage call queues via the REST API.
//!
//! The REST client is synchronous. `create` takes a `&serde_json::Value`;
//! `list` / `list_members` take a `&HashMap<String, String>`. Every method
//! returns `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a queue.
    println!("Creating queue ...");
    let queue = client.queues().create(
        &json!({
            "name": "Support Queue",
            "max_size": 100
        }),
        None,
    )?;
    let queue_id = queue["id"].as_str().unwrap_or("unknown").to_string();
    println!("Queue created: {queue_id}");

    // List all queues.
    let queues = client.queues().list(&HashMap::new(), None)?;
    if let Some(arr) = queues.as_array() {
        println!("\nAll queues ({}):", arr.len());
        for q in arr {
            println!(
                "  {} - {} (size: {})",
                q["id"], q["name"], q["current_size"]
            );
        }
    }

    // Get queue details.
    let details = client.queues().get(&queue_id, None)?;
    println!("\nQueue details:");
    println!("  Name: {}", details["name"]);
    println!("  Max size: {}", details["max_size"]);
    println!("  Current size: {}", details["current_size"]);

    // List members in the queue.
    let members = client
        .queues()
        .list_members(&queue_id, &HashMap::new(), None)?;
    if let Some(arr) = members.as_array() {
        println!("  Members: {}", arr.len());
    }

    Ok(())
}
