// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! README quickstart — REST client.
//!
//! The `client` region below is the exact code shown in the "REST Client"
//! section of the crate README, included there via a `<!-- include: -->` marker
//! and asserted byte-identical at gate time. The REST client is blocking
//! (synchronous) — no async runtime, no `.await`.

// region: client
use serde_json::json;
use signalwire::rest::RestClient;
use signalwire::rest::namespaces::generated::calling_resources_generated::CallingDialRequest;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads SIGNALWIRE_PROJECT_ID / SIGNALWIRE_API_TOKEN / SIGNALWIRE_SPACE.
    let client = RestClient::from_env().expect("missing SIGNALWIRE_* env vars");

    client.fabric().ai_agents().create(&json!({
        "name": "Support Bot",
        "prompt": {"text": "You are helpful."}
    }), None)?;

    client.calling().dial(
        CallingDialRequest::new("+15559876543", "+15551234567")
            .url("https://example.com/call-handler"), None
    )?;

    let query = HashMap::from([("areacode".to_string(), "512".to_string())]);
    let results = client.phone_numbers().search(&query, None)?;
    println!("{results:#?}");

    Ok(())
}
// endregion: client
