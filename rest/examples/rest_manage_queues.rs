// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage call queues via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a queue
    println!("Creating queue ...");
    let queue = client.queues().create(serde_json::json!({
        "friendly_name": "Support Queue",
        "max_size": 100,
    })).await?;
    let queue_sid = queue["sid"].as_str().unwrap_or("unknown");
    println!("Queue created: {queue_sid}");

    // List all queues
    let queues = client.queues().list(&[]).await?;
    if let Some(arr) = queues.as_array() {
        println!("\nAll queues ({}):", arr.len());
        for q in arr {
            println!(
                "  {} - {} (size: {})",
                q["sid"], q["friendly_name"], q["current_size"]
            );
        }
    }

    // Get queue details
    let details = client.queues().get(queue_sid).await?;
    println!("\nQueue details:");
    println!("  Name: {}", details["friendly_name"]);
    println!("  Max size: {}", details["max_size"]);
    println!("  Current size: {}", details["current_size"]);

    // List members in the queue
    let members = client.queues().members(queue_sid).await?;
    if let Some(arr) = members.as_array() {
        println!("  Members: {}", arr.len());
    }

    Ok(())
}
