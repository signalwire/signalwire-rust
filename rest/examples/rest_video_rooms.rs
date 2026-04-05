// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage video rooms via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a video room
    println!("Creating video room ...");
    let room = client.video().rooms().create(serde_json::json!({
        "name": "team-standup",
        "display_name": "Daily Standup",
        "max_participants": 20,
        "layout": "grid-responsive",
        "record_on_start": false,
    })).await?;

    let room_id = room["id"].as_str().unwrap_or("unknown");
    println!("Room created: {room_id}");
    println!("  Name: {}", room["name"]);

    // List all rooms
    let rooms = client.video().rooms().list(&[]).await?;
    if let Some(arr) = rooms.as_array() {
        println!("\nAll video rooms ({}):", arr.len());
        for r in arr {
            println!(
                "  {} - {} (max: {} participants)",
                r["id"], r["display_name"], r["max_participants"]
            );
        }
    }

    // Get room details
    let details = client.video().rooms().get(room_id).await?;
    println!("\nRoom details:");
    println!("  Name: {}", details["display_name"]);
    println!("  Layout: {}", details["layout"]);
    println!("  Max participants: {}", details["max_participants"]);

    // List recordings for this room
    let recordings = client.video().recordings().list(&[
        ("room_id", room_id),
    ]).await?;
    if let Some(arr) = recordings.as_array() {
        println!("  Recordings: {}", arr.len());
    }

    Ok(())
}
