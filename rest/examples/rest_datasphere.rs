// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Datasphere document search via the REST API.
//!
//! Environment: SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE

use signalwire::rest::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RestClient::from_env()?;

    // List existing documents
    println!("Listing documents ...");
    let docs = client.datasphere().documents().list(&[]).await?;
    if let Some(arr) = docs.as_array() {
        println!("Documents ({}):", arr.len());
        for d in arr {
            println!("  {} - {}", d["id"], d["name"]);
        }
    }

    // Search documents
    println!("\nSearching for 'pricing' ...");
    let results = client.datasphere().documents().search(serde_json::json!({
        "query": "pricing",
        "limit": 5,
    })).await?;

    if let Some(arr) = results["results"].as_array() {
        println!("Search results ({}):", arr.len());
        for r in arr {
            println!("  Score: {} - {}", r["score"], r["text"]);
        }
    } else {
        println!("No results found.");
    }

    Ok(())
}
