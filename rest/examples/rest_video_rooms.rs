// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Manage video rooms via the REST API.
//!
//! The REST client is synchronous. `create` takes a `&serde_json::Value`;
//! `list` takes a `&HashMap<String, String>`. Room recordings live under
//! `video().room_recordings()`. Every method returns
//! `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use serde_json::json;
use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // Create a video room.
    println!("Creating video room ...");
    let room = client.video().rooms().create(&json!({
        "name": "team-standup",
        "display_name": "Daily Standup",
        "max_members": 20,
        "layout": "grid-responsive",
        "record_on_start": false
    }))?;

    let room_id = room["id"].as_str().unwrap_or("unknown").to_string();
    println!("Room created: {room_id}");
    println!("  Name: {}", room["name"]);

    // List all rooms.
    let rooms = client.video().rooms().list(&HashMap::new())?;
    if let Some(arr) = rooms.as_array() {
        println!("\nAll video rooms ({}):", arr.len());
        for r in arr {
            println!(
                "  {} - {} (max: {} members)",
                r["id"], r["display_name"], r["max_members"]
            );
        }
    }

    // Get room details.
    let details = client.video().rooms().get(&room_id)?;
    println!("\nRoom details:");
    println!("  Name: {}", details["display_name"]);
    println!("  Layout: {}", details["layout"]);
    println!("  Max members: {}", details["max_members"]);

    // List room recordings.
    let recordings = client.video().room_recordings().list(&HashMap::new())?;
    if let Some(arr) = recordings.as_array() {
        println!("  Recordings: {}", arr.len());
    }

    Ok(())
}
