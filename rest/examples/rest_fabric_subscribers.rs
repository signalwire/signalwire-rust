// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage Fabric subscribers via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a subscriber
    println!("Creating subscriber ...");
    let sub = client.fabric().subscribers().create(serde_json::json!({
        "email": "alice@example.com",
        "first_name": "Alice",
        "last_name": "Smith",
        "display_name": "Alice Smith",
    })).await?;

    let sub_id = sub["id"].as_str().unwrap_or("unknown");
    println!("Subscriber created: {sub_id}");

    // List all subscribers
    let subs = client.fabric().subscribers().list(&[]).await?;
    if let Some(arr) = subs.as_array() {
        println!("\nAll subscribers ({}):", arr.len());
        for s in arr {
            println!(
                "  {} - {} {} ({})",
                s["id"], s["first_name"], s["last_name"], s["email"]
            );
        }
    }

    // Update subscriber
    println!("\nUpdating subscriber ...");
    client.fabric().subscribers().update(sub_id, serde_json::json!({
        "display_name": "Alice S.",
    })).await?;
    println!("Subscriber updated.");

    // Get subscriber details
    let details = client.fabric().subscribers().get(sub_id).await?;
    println!("Display name: {}", details["display_name"]);

    Ok(())
}
