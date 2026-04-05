// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! List call recordings via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    println!("Fetching recordings ...");

    let recordings = client.recordings().list(&[
        ("limit", "20"),
    ]).await?;

    if let Some(arr) = recordings.as_array() {
        println!("Recordings ({}):", arr.len());
        for r in arr {
            println!(
                "  {} - {}s ({}) - {}",
                r["sid"],
                r["duration"],
                r["status"],
                r["date_created"]
            );
        }
    } else {
        println!("No recordings found.");
    }

    Ok(())
}
