// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! List call recordings via the REST API.
//!
//! The REST client is synchronous. `list` takes a `&HashMap<String, String>`
//! of query params and returns `Result<Value, SignalWireRestError>`.
//!
//! Environment: `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, `SIGNALWIRE_SPACE`

use signalwire::rest::RestClient;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    println!("Fetching recordings ...");

    let params = HashMap::from([("limit".to_string(), "20".to_string())]);
    let recordings = client.recordings().list(&params)?;

    if let Some(arr) = recordings.as_array() {
        println!("Recordings ({}):", arr.len());
        for r in arr {
            println!(
                "  {} - {}s ({}) - {}",
                r["id"], r["duration"], r["status"], r["created_at"]
            );
        }
    } else {
        println!("No recordings found.");
    }

    Ok(())
}
